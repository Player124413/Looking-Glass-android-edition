//! Android platform paths, PK3 data discovery, guided setup screen and mobile performance presets.
use crate::assets::Assets;
use anyhow::Result;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
};

pub const PACKAGE_ID: &str = "com.lookingglass.alice";

/// Initialize the Android NDK context (`JavaVM` + `Activity` handle) required by
/// `cpal` / `oboe` / `rodio` before any audio output stream is opened.
pub fn init_runtime() {
    #[cfg(target_os = "android")]
    {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| unsafe {
            std::env::set_var("RUST_MIN_STACK", "16777216");
            use macroquad::miniquad::native::android::{attach_jni_env, ndk_sys, ACTIVITY};
            let env = attach_jni_env();
            if !env.is_null() && !ACTIVITY.is_null() {
                let mut vm: *mut ndk_sys::JavaVM = std::ptr::null_mut();
                if let Some(get_java_vm) = (**env).GetJavaVM {
                    get_java_vm(env, &mut vm);
                }
                if !vm.is_null() {
                    ndk_context::initialize_android_context(vm.cast(), ACTIVITY.cast());
                }
            }
        });
        install_panic_hook();
    }
}

/// Install an Android panic hook that logs to logcat, writes `crash.log`, and displays
/// the error in `MainActivity` instead of silently terminating the background GL thread.
pub fn install_panic_hook() {
    #[cfg(target_os = "android")]
    {
        std::panic::set_hook(Box::new(|info| {
            let thread = std::thread::current();
            let thread_name = thread.name().unwrap_or("unnamed");
            let msg = format!("Rust panic on thread '{thread_name}':\n{info}\n");
            let crash_path = storage_root().join("crash.log");
            let _ = fs::write(&crash_path, &msg);
            if let Ok(c_msg) = std::ffi::CString::new(msg.replace('\0', " ")) {
                unsafe {
                    macroquad::miniquad::native::android::console_error(c_msg.as_ptr());
                }
            }
            report_native_crash_to_java(&msg);
            if thread_name != "main" {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(3600));
                }
            }
        }));
    }
}

#[cfg(target_os = "android")]
fn report_native_crash_to_java(message: &str) {
    unsafe {
        use macroquad::miniquad::native::android::{attach_jni_env, ACTIVITY};
        let env = attach_jni_env();
        if env.is_null() || ACTIVITY.is_null() {
            return;
        }
        let Some(get_object_class) = (**env).GetObjectClass else {
            return;
        };
        let Some(get_method_id) = (**env).GetMethodID else {
            return;
        };
        let Some(call_void_method) = (**env).CallVoidMethod else {
            return;
        };
        let Some(new_string_utf) = (**env).NewStringUTF else {
            return;
        };
        let sanitized = message.replace('\0', " ");
        let Ok(c_msg) = std::ffi::CString::new(sanitized) else {
            return;
        };
        let Ok(method) = std::ffi::CString::new("reportNativeCrash") else {
            return;
        };
        let Ok(sig) = std::ffi::CString::new("(Ljava/lang/String;)V") else {
            return;
        };
        let class = get_object_class(env, ACTIVITY);
        if class.is_null() {
            return;
        }
        let mid = get_method_id(env, class, method.as_ptr() as _, sig.as_ptr() as _);
        if !mid.is_null() {
            let jstr = new_string_utf(env, c_msg.as_ptr());
            if !jstr.is_null() {
                call_void_method(env, ACTIVITY, mid, jstr);
                if let Some(del) = (**env).DeleteLocalRef {
                    del(env, jstr);
                }
            }
        }
        if let Some(exc) = (**env).ExceptionCheck {
            if exc(env) != 0 {
                if let Some(clear) = (**env).ExceptionClear {
                    clear(env);
                }
            }
        }
        if let Some(del) = (**env).DeleteLocalRef {
            del(env, class);
        }
    }
}

/// Runtime rendering and frame-budget preset for desktop and mobile GPUs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PerformancePreset {
    #[default]
    Auto,
    Quality,
    Balanced,
    Performance,
}

thread_local! {
    static ACTIVE_PRESET: Cell<PerformancePreset> = const { Cell::new(PerformancePreset::Quality) };
}

pub fn set_active_preset(preset: PerformancePreset) {
    ACTIVE_PRESET.with(|p| p.set(preset.effective()));
}

pub fn active_preset() -> PerformancePreset {
    ACTIVE_PRESET.with(Cell::get)
}

impl PerformancePreset {
    pub const ALL: [Self; 4] = [Self::Auto, Self::Quality, Self::Balanced, Self::Performance];

    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Quality => "Quality",
            Self::Balanced => "Balanced",
            Self::Performance => "Performance",
        }
    }

    pub fn next(self, delta: f32) -> Self {
        let index = Self::ALL.iter().position(|&p| p == self).unwrap_or(0) as i32;
        let step = if delta < 0. { -1 } else { 1 };
        Self::ALL[(index + step).rem_euclid(Self::ALL.len() as i32) as usize]
    }

    pub fn effective(self) -> Self {
        match self {
            Self::Auto => {
                if is_android() {
                    Self::Balanced
                } else {
                    Self::Quality
                }
            }
            other => other,
        }
    }

    /// Screen-space geometric error threshold (pixels) for static prop LOD selection.
    pub fn detail_error_threshold(self) -> f32 {
        match self.effective() {
            Self::Quality | Self::Auto => 0.35,
            Self::Balanced => 0.65,
            Self::Performance => 1.15,
        }
    }

    pub fn lod_error_limit(self) -> f32 {
        self.detail_error_threshold()
    }

    pub fn lod_distance_scale(self) -> f32 {
        match self.effective() {
            Self::Quality | Self::Auto => 1.0,
            Self::Balanced => 0.8,
            Self::Performance => 0.6,
        }
    }

    /// Maximum dynamic point lights evaluated per draw pass after visibility culling.
    pub fn max_dynamic_lights(self) -> usize {
        match self.effective() {
            Self::Quality | Self::Auto => 8,
            Self::Balanced => 4,
            Self::Performance => 2,
        }
    }

    /// Skip the 24-ray rim sweep for player contact shadows on low-power mobile GPUs.
    pub fn fast_contact_shadow(self) -> bool {
        matches!(self.effective(), Self::Balanced | Self::Performance)
    }

    pub fn fast_contact_shadows(self) -> bool {
        self.fast_contact_shadow()
    }

    /// Maximum distance at which environmental steam/particle emitters are simulated and drawn.
    pub fn particle_cull_distance(self) -> f32 {
        match self.effective() {
            Self::Quality | Self::Auto => 3000.,
            Self::Balanced => 2200.,
            Self::Performance => 1500.,
        }
    }

    pub fn particle_cull_distance_sq(self) -> f32 {
        let d = self.particle_cull_distance();
        d * d
    }

    /// Avoid per-frame full-screen framebuffer copy on tile-based mobile GPUs; capture on demand.
    pub fn throttle_save_preview(self) -> bool {
        is_android() || matches!(self.effective(), Self::Balanced | Self::Performance)
    }

    pub fn save_preview_interval(self) -> u32 {
        match self.effective() {
            Self::Quality | Self::Auto if !is_android() => 1,
            Self::Balanced | Self::Quality | Self::Auto => 15,
            Self::Performance => 30,
        }
    }

    /// Maximum dimension for 3D world/actor textures on mobile GPUs before ETC2 compression.
    pub fn max_texture_size(self) -> u16 {
        if !is_android() {
            return 4096;
        }
        match self.effective() {
            Self::Quality | Self::Auto => 1024,
            Self::Balanced => 512,
            Self::Performance => 256,
        }
    }
}

pub fn etc2_cache_dir() -> Option<PathBuf> {
    if !is_android() {
        return None;
    }
    let dir = storage_root().join("cache").join("etc2_v1");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir)
}

pub fn is_android() -> bool {
    cfg!(target_os = "android")
        || std::env::var_os("LOOKING_GLASS_ANDROID_MODE")
            .is_some_and(|v| !v.is_empty() && v != "0")
}

/// Resolve the writable root directory for settings, logs and local saves.
pub fn storage_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("LOOKING_GLASS_SETTINGS_DIR")
        .or_else(|| std::env::var_os("LOOKING_GLASS_ANDROID_STORAGE"))
        .or_else(|| std::env::var_os("LOOKING_GLASS_ANDROID_DIR"))
        .filter(|s| !s.is_empty())
    {
        return PathBuf::from(dir);
    }
    if cfg!(target_os = "android") {
        for candidate in [
            format!("/storage/emulated/0/Android/data/{PACKAGE_ID}/files"),
            format!("/sdcard/Android/data/{PACKAGE_ID}/files"),
            "/storage/emulated/0/LookingGlass/private".into(),
            "/sdcard/LookingGlass/private".into(),
            format!("/data/user/0/{PACKAGE_ID}/files"),
            format!("/data/data/{PACKAGE_ID}/files"),
        ] {
            let path = PathBuf::from(candidate);
            if fs::create_dir_all(&path).is_ok() {
                return path;
            }
        }
        let fallback = std::env::temp_dir().join("looking-glass");
        let _ = fs::create_dir_all(&fallback);
        return fallback;
    }
    PathBuf::from("private")
}

pub fn default_save_dir() -> PathBuf {
    storage_root().join("saves")
}

pub fn default_data_dir() -> PathBuf {
    let fallback = std::fs::read_to_string("private/data-path.txt")
        .ok()
        .filter(|path| !path.trim().is_empty())
        .map(|path| PathBuf::from(path.trim()))
        .unwrap_or_else(|| PathBuf::from("alice_202106/Alice1/bin/base"));
    resolve_data_dir(&fallback)
}

pub fn has_pk3_archives(dir: &Path) -> bool {
    fs::read_dir(dir).is_ok_and(|entries| {
        entries.filter_map(|e| e.ok()).any(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pk3"))
        })
    })
}

/// Inspect a directory or its standard `base` / `Alice1/bin/base` children for PK3 archives.
pub fn find_pk3_dir(root: &Path) -> Option<PathBuf> {
    for suffix in ["", "base", "bin/base", "Alice1/bin/base", "game-data/Alice1/bin/base"] {
        let candidate = if suffix.is_empty() {
            root.to_path_buf()
        } else {
            root.join(suffix)
        };
        if has_pk3_archives(&candidate) {
            return Some(candidate);
        }
    }
    None
}

pub fn candidate_data_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(env_dir) = std::env::var_os("LOOKING_GLASS_DATA")
        .or_else(|| std::env::var_os("LOOKING_GLASS_DATA_DIR"))
        .filter(|s| !s.is_empty())
    {
        dirs.push(PathBuf::from(env_dir));
    }
    for pointer in [root.join("data-path.txt"), PathBuf::from("private/data-path.txt")] {
        if let Ok(saved) = fs::read_to_string(pointer) {
            let trimmed = saved.trim();
            if !trimmed.is_empty() {
                dirs.push(PathBuf::from(trimmed));
            }
        }
    }
    dirs.push(root.join("base"));
    dirs.push(root.join("game-data/Alice1/bin/base"));
    dirs.push(root.join("Alice1/bin/base"));
    if is_android() {
        for path in [
            format!("/storage/emulated/0/Android/data/{PACKAGE_ID}/files/base"),
            format!("/sdcard/Android/data/{PACKAGE_ID}/files/base"),
            "/storage/emulated/0/LookingGlass/base".into(),
            "/storage/emulated/0/LookingGlass".into(),
            "/storage/emulated/0/Alice1/bin/base".into(),
            "/storage/emulated/0/Download/LookingGlass/base".into(),
            "/storage/emulated/0/Download/Alice1/bin/base".into(),
            "/sdcard/LookingGlass/base".into(),
            "/sdcard/LookingGlass".into(),
            "/sdcard/Alice1/bin/base".into(),
            format!("/data/user/0/{PACKAGE_ID}/files/base"),
            format!("/data/data/{PACKAGE_ID}/files/base"),
        ] {
            dirs.push(PathBuf::from(path));
        }
    }
    dirs.push(PathBuf::from("alice_202106/Alice1/bin/base"));
    dirs.push(PathBuf::from("base"));
    dirs
}

pub fn resolve_data_dir(configured: &Path) -> PathBuf {
    if let Some(found) = find_pk3_dir(configured) {
        return found;
    }
    let root = storage_root();
    for candidate in candidate_data_dirs(&root) {
        if let Some(found) = find_pk3_dir(&candidate) {
            return found;
        }
    }
    if is_android() {
        prepare_directories(&root);
        return root.join("base");
    }
    configured.to_path_buf()
}

pub fn remember_data_dir(root: &Path, data_dir: &Path) {
    if fs::create_dir_all(root).is_ok() {
        let _ = fs::write(
            root.join("data-path.txt"),
            format!("{}\n", data_dir.display()),
        );
    }
}

pub fn prepare_directories(root: &Path) {
    let base = root.join("base");
    let saves = root.join("saves");
    let _ = fs::create_dir_all(&base);
    let _ = fs::create_dir_all(&saves);
    let readme = base.join("PLACE_PK3_FILES_HERE.txt");
    if !readme.exists() {
        let _ = fs::write(
            readme,
            "Looking Glass (Android)\n\
             Copy your compatible American McGee's Alice (2011) PK3 files into this folder:\n\
             - pak0.pk3\n\
             - pak1_large.pk3\n\
             - pak2_small.pk3\n\
             - pak3.pk3\n\
             - pak4_english.pk3\n\
             - pak5_mod.pk3 (if present)\n",
        );
    }
}

#[cfg(target_os = "android")]
fn call_activity_void(method_name: &str) -> bool {
    unsafe {
        use macroquad::miniquad::native::android::{attach_jni_env, ACTIVITY};
        let env = attach_jni_env();
        if env.is_null() || ACTIVITY.is_null() {
            return false;
        }
        let Some(get_object_class) = (**env).GetObjectClass else {
            return false;
        };
        let Some(get_method_id) = (**env).GetMethodID else {
            return false;
        };
        let Some(call_void_method) = (**env).CallVoidMethod else {
            return false;
        };
        let Ok(method) = std::ffi::CString::new(method_name) else {
            return false;
        };
        let Ok(sig) = std::ffi::CString::new("()V") else {
            return false;
        };
        let class = get_object_class(env, ACTIVITY);
        if class.is_null() {
            return false;
        }
        let mid = get_method_id(env, class, method.as_ptr() as _, sig.as_ptr() as _);
        if mid.is_null() {
            if let Some(clear) = (**env).ExceptionClear {
                clear(env);
            }
            if let Some(del) = (**env).DeleteLocalRef {
                del(env, class);
            }
            return false;
        }
        call_void_method(env, ACTIVITY, mid);
        let mut ok = true;
        if let Some(exc) = (**env).ExceptionCheck {
            if exc(env) != 0 {
                if let Some(clear) = (**env).ExceptionClear {
                    clear(env);
                }
                ok = false;
            }
        }
        if let Some(del) = (**env).DeleteLocalRef {
            del(env, class);
        }
        ok
    }
}

#[cfg(not(target_os = "android"))]
fn call_activity_void(_method_name: &str) -> bool {
    false
}

/// Trigger Android's native system folder chooser (`Intent.ACTION_OPEN_DOCUMENT_TREE`),
/// which copies all `.pk3` files from the chosen game folder into `<storage>/base/`.
pub fn open_system_folder_picker() -> bool {
    call_activity_void("openFolderPicker")
}

/// Trigger Android's native system file chooser (`Intent.ACTION_OPEN_DOCUMENT`),
/// allowing the user to select `.pk3` files or a `.zip` archive to copy into `<storage>/base/`.
pub fn open_system_file_picker() -> bool {
    call_activity_void("openFilePicker")
}

fn collect_pk3_and_zip_files(
    dir: &Path,
    depth: usize,
    pk3_out: &mut Vec<PathBuf>,
    zip_out: &mut Vec<PathBuf>,
) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.eq_ignore_ascii_case("pk3") {
                pk3_out.push(path);
            } else if ext.eq_ignore_ascii_case("zip") {
                zip_out.push(path);
            }
        }
    }
    subdirs.sort();
    for sub in subdirs {
        collect_pk3_and_zip_files(&sub, depth + 1, pk3_out, zip_out);
    }
}

fn extract_pk3_from_zip(zip_path: &Path, dest_base: &Path) -> Result<usize> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    fs::create_dir_all(dest_base)?;
    let mut extracted = 0usize;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let Some(name) = Path::new(entry.name()).file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.to_ascii_lowercase().ends_with(".pk3") {
            continue;
        }
        let dest = dest_base.join(name);
        let tmp = dest_base.join(format!("{name}.part"));
        {
            let mut out = fs::File::create(&tmp)?;
            std::io::copy(&mut entry, &mut out)?;
        }
        let _ = fs::remove_file(&dest);
        fs::rename(&tmp, &dest)?;
        extracted += 1;
    }
    Ok(extracted)
}

/// Copy `.pk3` archives from a user-selected game folder, `.pk3` file, or `.zip` archive
/// into `dest_base` (typically `<storage>/base`). Returns the number of `.pk3` files copied.
pub fn import_game_dir(source: &Path, dest_base: &Path) -> Result<usize> {
    fs::create_dir_all(dest_base)?;
    let canonical_dest = dest_base.canonicalize().ok();
    if source.is_file() {
        let Some(ext) = source.extension().and_then(|e| e.to_str()) else {
            anyhow::bail!("Unsupported file: {}", source.display());
        };
        if ext.eq_ignore_ascii_case("pk3") {
            let Some(name) = source.file_name() else {
                anyhow::bail!("Invalid PK3 file name");
            };
            let target = dest_base.join(name);
            if source.canonicalize().ok() != target.canonicalize().ok() {
                let tmp = dest_base.join(format!("{}.part", name.to_string_lossy()));
                fs::copy(source, &tmp)?;
                let _ = fs::remove_file(&target);
                fs::rename(&tmp, &target)?;
            }
            return Ok(1);
        } else if ext.eq_ignore_ascii_case("zip") {
            let count = extract_pk3_from_zip(source, dest_base)?;
            if count == 0 {
                anyhow::bail!("Archive {} does not contain any .pk3 files", source.display());
            }
            return Ok(count);
        }
        anyhow::bail!("Expected a folder, .pk3 file, or .zip archive");
    }

    if !source.is_dir() {
        anyhow::bail!("Folder does not exist: {}", source.display());
    }

    let mut pk3_files = Vec::new();
    let mut zip_files = Vec::new();
    if let Some(pk3_dir) = find_pk3_dir(source) {
        collect_pk3_and_zip_files(&pk3_dir, 0, &mut pk3_files, &mut zip_files);
    } else {
        collect_pk3_and_zip_files(source, 0, &mut pk3_files, &mut zip_files);
    }
    pk3_files.sort();
    pk3_files.dedup();

    let mut copied = 0usize;
    for pk3 in &pk3_files {
        let Some(name) = pk3.file_name() else {
            continue;
        };
        if let (Some(src_parent), Some(dst)) = (
            pk3.parent().and_then(|p| p.canonicalize().ok()),
            canonical_dest.as_ref(),
        ) {
            if &src_parent == dst {
                copied += 1;
                continue;
            }
        }
        let target = dest_base.join(name);
        let tmp = dest_base.join(format!("{}.part", name.to_string_lossy()));
        fs::copy(pk3, &tmp)?;
        let _ = fs::remove_file(&target);
        fs::rename(&tmp, &target)?;
        copied += 1;
    }

    if copied == 0 {
        for zip_path in &zip_files {
            if let Ok(n) = extract_pk3_from_zip(zip_path, dest_base) {
                copied += n;
            }
        }
    }

    if copied == 0 {
        anyhow::bail!("No .pk3 archives found in {}", source.display());
    }
    Ok(copied)
}

/// Automatically scan common device storage locations (`Download`, `Documents`, `LookingGlass`)
/// and copy any discovered `.pk3` archives into `<root>/base`.
pub fn auto_import_from_storage(root: &Path) -> Result<usize> {
    let dest_base = root.join("base");
    let mut search_roots = vec![
        PathBuf::from("/storage/emulated/0/Download"),
        PathBuf::from("/storage/emulated/0/Documents"),
        PathBuf::from("/storage/emulated/0/LookingGlass"),
        PathBuf::from("/storage/emulated/0/Games"),
        PathBuf::from("/sdcard/Download"),
        PathBuf::from("/sdcard/LookingGlass"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        search_roots.push(PathBuf::from(&home).join("Downloads"));
    }
    for candidate in search_roots {
        if !candidate.is_dir() {
            continue;
        }
        if let Ok(count) = import_game_dir(&candidate, &dest_base) {
            if count > 0 && has_pk3_archives(&dest_base) {
                return Ok(count);
            }
        }
    }
    anyhow::bail!("No .pk3 archives found in Download / Documents / LookingGlass folders")
}

fn read_import_status(root: &Path) -> Option<(String, String)> {
    let text = fs::read_to_string(root.join("import-status.txt")).ok()?;
    let mut lines = text.lines();
    let first = lines.next()?.trim();
    let state = first.strip_prefix("STATE:")?.trim().to_string();
    let msg = lines.collect::<Vec<_>>().join(" ").trim().to_string();
    Some((state, msg))
}

fn clear_import_status(root: &Path) {
    let _ = fs::remove_file(root.join("import-status.txt"));
}

fn list_browsable_subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    out.sort();
    out
}

fn initial_browse_dir(root: &Path) -> PathBuf {
    for candidate in [
        PathBuf::from("/storage/emulated/0/Download"),
        PathBuf::from("/storage/emulated/0"),
        PathBuf::from("/sdcard/Download"),
        PathBuf::from("/sdcard"),
    ] {
        if candidate.is_dir() {
            return candidate;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| root.to_path_buf())
}

/// Interactive setup launcher when launched before PK3 data is mounted.
/// Lets the user pick a game folder or PK3/ZIP archive (via Android's native system picker
/// or the built-in folder browser) and automatically copies the archives into `<root>/base`.
pub async fn wait_for_data(initial: PathBuf) -> Result<Assets> {
    let root = storage_root();
    prepare_directories(&root);
    clear_import_status(&root);
    let mut status = format!(
        "Select your game folder or PK3 archive to copy into {}",
        root.join("base").display()
    );
    let mut browsing = false;
    let mut browse_dir = initial_browse_dir(&root);
    let mut browse_scroll = 0usize;

    loop {
        if is_quit_requested() {
            anyhow::bail!("Setup cancelled before game data was mounted");
        }

        if let Some((state, msg)) = read_import_status(&root) {
            if !msg.is_empty() {
                status = msg;
            }
            if state == "DONE" {
                clear_import_status(&root);
                let resolved = resolve_data_dir(&initial);
                if let Ok(assets) = Assets::open(&resolved) {
                    remember_data_dir(&root, &resolved);
                    return Ok(assets);
                }
            } else if state == "ERROR" || state == "IDLE" {
                clear_import_status(&root);
            }
        }

        let (w, h) = (screen_width(), screen_height());
        let s = (h / 720.).clamp(0.65, 2.2).min(w / 960.);
        clear_background(Color::from_hex(0x141118));
        let panel = Rect::new(
            (w - 880. * s).max(20.) * 0.5,
            (h - 560. * s).max(20.) * 0.5,
            (w - 40.).min(880. * s),
            (h - 40.).min(560. * s),
        );
        draw_rectangle(panel.x, panel.y, panel.w, panel.h, Color::from_hex(0x221b29));
        draw_rectangle_lines(
            panel.x,
            panel.y,
            panel.w,
            panel.h,
            2. * s,
            Color::from_hex(0x8c6d46),
        );
        let x = panel.x + 26. * s;
        let mut y = panel.y + 40. * s;
        draw_text(
            "Looking Glass - Game Data Launcher",
            x,
            y,
            (30. * s).round(),
            Color::from_hex(0xf3e5c8),
        );
        y += 32. * s;

        let pointer = touches()
            .iter()
            .find(|t| t.phase == TouchPhase::Started)
            .map(|t| t.position)
            .or_else(|| {
                is_mouse_button_pressed(MouseButton::Left).then(|| Vec2::from(mouse_position()))
            });

        if !browsing {
            for line in [
                "Tap 'Choose Game Folder' or 'Select PK3 / ZIP' to select your Alice game folder,",
                "and Looking Glass will automatically copy pak0.pk3..pak4_english.pk3 into:",
                &format!("  {}", root.join("base").display()),
                "Required archives: pak0.pk3, pak1_large.pk3, pak2_small.pk3, pak3.pk3, pak4_english.pk3",
            ] {
                draw_text(line, x, y, (19. * s).round(), Color::from_hex(0xd8cbb8));
                y += 26. * s;
            }
            y += 10. * s;
            draw_text(
                &status,
                x,
                y,
                (19. * s).round(),
                Color::from_hex(0xe59866),
            );

            let row1_y = panel.bottom() - 140. * s;
            let row2_y = panel.bottom() - 74. * s;
            let folder_btn = Rect::new(x, row1_y, 250. * s, 50. * s);
            let file_btn = Rect::new(x + 266. * s, row1_y, 240. * s, 50. * s);
            let browse_btn = Rect::new(x + 522. * s, row1_y, 260. * s, 50. * s);

            let auto_btn = Rect::new(x, row2_y, 250. * s, 50. * s);
            let scan_btn = Rect::new(x + 266. * s, row2_y, 240. * s, 50. * s);
            let quit_btn = Rect::new(x + 522. * s, row2_y, 160. * s, 50. * s);

            for (rect, label, fill) in [
                (folder_btn, "Choose Game Folder", Color::from_hex(0x6e352c)),
                (file_btn, "Select PK3 / ZIP", Color::from_hex(0x4a354f)),
                (browse_btn, "Browse Folders", Color::from_hex(0x354552)),
                (auto_btn, "Auto-Import Downloads", Color::from_hex(0x3d4f35)),
                (scan_btn, "Scan & Start", Color::from_hex(0x5b2c24)),
                (quit_btn, "Quit", Color::from_hex(0x342a38)),
            ] {
                draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
                draw_rectangle_lines(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    2. * s,
                    Color::from_hex(0xc9a97c),
                );
                draw_text(
                    label,
                    rect.x + 16. * s,
                    rect.y + 32. * s,
                    (21. * s).round(),
                    WHITE,
                );
            }

            if pointer.is_some_and(|p| folder_btn.contains(p)) {
                if open_system_folder_picker() {
                    status = "Opening Android system folder chooser...".into();
                } else {
                    browsing = true;
                    browse_scroll = 0;
                }
            } else if pointer.is_some_and(|p| file_btn.contains(p)) {
                if open_system_file_picker() {
                    status = "Opening Android system file chooser (.pk3 / .zip)...".into();
                } else {
                    browsing = true;
                    browse_scroll = 0;
                }
            } else if pointer.is_some_and(|p| browse_btn.contains(p)) {
                browsing = true;
                browse_scroll = 0;
            } else if pointer.is_some_and(|p| auto_btn.contains(p)) {
                match auto_import_from_storage(&root) {
                    Ok(count) => {
                        status = format!("Copied {count} PK3 archive(s) into base/. Starting...");
                        let resolved = resolve_data_dir(&initial);
                        if let Ok(assets) = Assets::open(&resolved) {
                            remember_data_dir(&root, &resolved);
                            return Ok(assets);
                        }
                    }
                    Err(e) => {
                        status = format!("{e}");
                    }
                }
            }

            let trigger_scan = is_key_pressed(KeyCode::Enter)
                || is_key_pressed(KeyCode::Space)
                || pointer.is_some_and(|p| scan_btn.contains(p));
            let trigger_quit = is_key_pressed(KeyCode::Escape)
                || is_key_pressed(KeyCode::Back)
                || pointer.is_some_and(|p| quit_btn.contains(p));
            if trigger_quit {
                anyhow::bail!("Closed from data setup screen");
            }
            if trigger_scan {
                prepare_directories(&root);
                let resolved = resolve_data_dir(&initial);
                match Assets::open(&resolved) {
                    Ok(assets) => {
                        remember_data_dir(&root, &resolved);
                        return Ok(assets);
                    }
                    Err(e) => {
                        status = format!("Still missing PK3 files: {e}");
                    }
                }
            }
        } else {
            draw_text(
                &format!("Current Folder: {}", browse_dir.display()),
                x,
                y,
                (19. * s).round(),
                Color::from_hex(0xd8cbb8),
            );
            y += 26. * s;
            draw_text(
                &status,
                x,
                y,
                (18. * s).round(),
                Color::from_hex(0xe59866),
            );
            y += 14. * s;

            let subdirs = list_browsable_subdirs(&browse_dir);
            let max_rows = 6usize;
            let row_h = 40. * s;
            let list_w = panel.w - 52. * s;

            for idx in 0..max_rows {
                let Some(dir_path) = subdirs.get(browse_scroll + idx) else {
                    break;
                };
                let row_rect = Rect::new(x, y + idx as f32 * (row_h + 6. * s), list_w, row_h);
                let has_pk3 = has_pk3_archives(dir_path);
                let fill = if has_pk3 {
                    Color::from_hex(0x3b4d34)
                } else {
                    Color::from_hex(0x2d2436)
                };
                draw_rectangle(row_rect.x, row_rect.y, row_rect.w, row_rect.h, fill);
                draw_rectangle_lines(
                    row_rect.x,
                    row_rect.y,
                    row_rect.w,
                    row_rect.h,
                    1.5 * s,
                    Color::from_hex(0x8c6d46),
                );
                let name = dir_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("folder");
                let badge = if has_pk3 { "  [PK3 FOUND]" } else { "" };
                draw_text(
                    &format!("[DIR] {name}{badge}"),
                    row_rect.x + 14. * s,
                    row_rect.y + 26. * s,
                    (20. * s).round(),
                    WHITE,
                );
                if pointer.is_some_and(|p| row_rect.contains(p)) {
                    browse_dir = dir_path.clone();
                    browse_scroll = 0;
                }
            }

            let btn_y = panel.bottom() - 68. * s;
            let up_btn = Rect::new(x, btn_y, 130. * s, 48. * s);
            let prev_btn = Rect::new(x + 142. * s, btn_y, 110. * s, 48. * s);
            let next_btn = Rect::new(x + 264. * s, btn_y, 110. * s, 48. * s);
            let copy_btn = Rect::new(x + 386. * s, btn_y, 260. * s, 48. * s);
            let back_btn = Rect::new(x + 658. * s, btn_y, 140. * s, 48. * s);

            for (rect, label, fill) in [
                (up_btn, ".. Parent", Color::from_hex(0x3c3144)),
                (prev_btn, "< Up", Color::from_hex(0x342a38)),
                (next_btn, "Down >", Color::from_hex(0x342a38)),
                (copy_btn, "Copy From This Folder", Color::from_hex(0x6e352c)),
                (back_btn, "Back", Color::from_hex(0x342a38)),
            ] {
                draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
                draw_rectangle_lines(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    2. * s,
                    Color::from_hex(0xc9a97c),
                );
                draw_text(
                    label,
                    rect.x + 14. * s,
                    rect.y + 30. * s,
                    (19. * s).round(),
                    WHITE,
                );
            }

            if pointer.is_some_and(|p| up_btn.contains(p)) {
                if let Some(parent) = browse_dir.parent() {
                    browse_dir = parent.to_path_buf();
                    browse_scroll = 0;
                }
            } else if pointer.is_some_and(|p| prev_btn.contains(p)) {
                browse_scroll = browse_scroll.saturating_sub(max_rows);
            } else if pointer.is_some_and(|p| next_btn.contains(p)) {
                if browse_scroll + max_rows < subdirs.len() {
                    browse_scroll += max_rows;
                }
            } else if pointer.is_some_and(|p| copy_btn.contains(p)) {
                let dest_base = root.join("base");
                match import_game_dir(&browse_dir, &dest_base) {
                    Ok(count) => {
                        let resolved = resolve_data_dir(&initial);
                        match Assets::open(&resolved) {
                            Ok(assets) => {
                                remember_data_dir(&root, &resolved);
                                return Ok(assets);
                            }
                            Err(e) => {
                                status = format!("Copied {count} archive(s), but mount failed: {e}");
                                browsing = false;
                            }
                        }
                    }
                    Err(e) => {
                        status = format!("{e}");
                    }
                }
            } else if is_key_pressed(KeyCode::Escape)
                || is_key_pressed(KeyCode::Back)
                || pointer.is_some_and(|p| back_btn.contains(p))
            {
                browsing = false;
            }
        }

        next_frame().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn resolves_nested_pk3_folders_and_persists_data_path() -> Result<()> {
        let temp = std::env::temp_dir().join(format!("lg-android-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let nested = temp.join("Alice1/bin/base");
        fs::create_dir_all(&nested)?;
        let pk3 = nested.join("pak0.pk3");
        let mut zip = zip::ZipWriter::new(fs::File::create(&pk3)?);
        zip.start_file("maps/skool1.bsp", zip::write::FileOptions::default())?;
        zip.write_all(b"test")?;
        zip.finish()?;

        assert!(has_pk3_archives(&nested));
        assert!(!has_pk3_archives(&temp));
        assert_eq!(find_pk3_dir(&temp), Some(nested.clone()));

        let settings = temp.join("settings");
        prepare_directories(&settings);
        assert!(settings.join("base/PLACE_PK3_FILES_HERE.txt").is_file());
        assert!(settings.join("saves").is_dir());

        remember_data_dir(&settings, &nested);
        let candidates = candidate_data_dirs(&settings);
        assert!(candidates.contains(&nested));

        let imported_base = settings.join("base");
        let count = import_game_dir(&temp, &imported_base)?;
        assert_eq!(count, 1);
        assert!(imported_base.join("pak0.pk3").is_file());

        fs::remove_dir_all(&temp)?;
        Ok(())
    }

    #[test]
    fn performance_presets_cycle_and_scale_budgets_predictably() {
        assert_eq!(
            PerformancePreset::Quality.next(1.),
            PerformancePreset::Balanced
        );
        assert_eq!(
            PerformancePreset::Quality.next(-1.),
            PerformancePreset::Auto
        );
        assert_eq!(PerformancePreset::Quality.max_dynamic_lights(), 8);
        assert_eq!(PerformancePreset::Balanced.max_dynamic_lights(), 4);
        assert_eq!(PerformancePreset::Performance.max_dynamic_lights(), 2);
        assert!(!PerformancePreset::Quality.fast_contact_shadow());
        assert!(PerformancePreset::Performance.fast_contact_shadow());
        assert!(
            PerformancePreset::Performance.detail_error_threshold()
                > PerformancePreset::Balanced.detail_error_threshold()
        );
        assert!(
            PerformancePreset::Balanced.detail_error_threshold()
                > PerformancePreset::Quality.detail_error_threshold()
        );
    }
}
