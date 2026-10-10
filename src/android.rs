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

/// Call `Activity.finish()` from Rust so the app actually returns to the launcher
/// on a clean Quit, instead of leaving a frozen/black GL surface behind.
pub fn finish_activity() {
    #[cfg(target_os = "android")]
    unsafe {
        use macroquad::miniquad::native::android::{attach_jni_env, ACTIVITY};
        let env = attach_jni_env();
        if env.is_null() || ACTIVITY.is_null() {
            return;
        }
        let Some(get_object_class) = (**env).GetObjectClass else { return };
        let Some(get_method_id) = (**env).GetMethodID else { return };
        let Some(call_void_method) = (**env).CallVoidMethod else { return };
        let class = get_object_class(env, ACTIVITY);
        if class.is_null() {
            return;
        }
        // android.app.Activity.finish()V
        let Ok(name) = std::ffi::CString::new("finish") else { return };
        let Ok(sig) = std::ffi::CString::new("()V") else { return };
        let mid = get_method_id(env, class, name.as_ptr() as _, sig.as_ptr() as _);
        if !mid.is_null() {
            call_void_method(env, ACTIVITY, mid);
        }
        if let Some(exc) = (**env).ExceptionCheck {
            if exc(env) != 0 {
                if let Some(clear) = (**env).ExceptionClear { clear(env); }
            }
        }
        if let Some(del) = (**env).DeleteLocalRef { del(env, class); }
    }
}

/// Target frame rate cap selectable in the Android Launcher and Video Settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FpsLimit {
    #[serde(alias = "30", alias = "30fps")]
    Fps30,
    #[default]
    #[serde(alias = "60", alias = "60fps")]
    Fps60,
    #[serde(alias = "uncapped", alias = "max")]
    Unlimited,
}

static ACTIVE_FPS_LIMIT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(1);

pub fn set_active_fps_limit(limit: FpsLimit) {
    let code = match limit {
        FpsLimit::Fps30 => 0,
        FpsLimit::Fps60 => 1,
        FpsLimit::Unlimited => 2,
    };
    ACTIVE_FPS_LIMIT.store(code, std::sync::atomic::Ordering::Relaxed);
}

pub fn active_fps_limit() -> FpsLimit {
    match ACTIVE_FPS_LIMIT.load(std::sync::atomic::Ordering::Relaxed) {
        0 => FpsLimit::Fps30,
        2 => FpsLimit::Unlimited,
        _ => FpsLimit::Fps60,
    }
}

#[no_mangle]
pub extern "C" fn looking_glass_target_frame_us() -> u64 {
    active_fps_limit().target_frame_us()
}

static ANDROID_PAD_CONNECTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
static ANDROID_PAD_BUTTONS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static ANDROID_PAD_AXES_L: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static ANDROID_PAD_AXES_R: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

#[no_mangle]
pub extern "C" fn Java_com_lookingglass_alice_MainActivity_nativeOnGamepad(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    connected: i32,
    buttons: i32,
    lt: i32,
    rt: i32,
    lx: i32,
    ly: i32,
    rx: i32,
    ry: i32,
) {
    use std::sync::atomic::Ordering::Relaxed;
    ANDROID_PAD_CONNECTED.store(connected != 0, Relaxed);
    let btn16 = (buttons as u32) & 0xFFFF;
    let lt8 = (lt.clamp(0, 255) as u32) & 0xFF;
    let rt8 = (rt.clamp(0, 255) as u32) & 0xFF;
    ANDROID_PAD_BUTTONS.store(btn16 | (lt8 << 16) | (rt8 << 24), Relaxed);
    let lx16 = (lx.clamp(-32767, 32767) as i16 as u16) as u32;
    let ly16 = (ly.clamp(-32767, 32767) as i16 as u16) as u32;
    ANDROID_PAD_AXES_L.store(lx16 | (ly16 << 16), Relaxed);
    let rx16 = (rx.clamp(-32767, 32767) as i16 as u16) as u32;
    let ry16 = (ry.clamp(-32767, 32767) as i16 as u16) as u32;
    ANDROID_PAD_AXES_R.store(rx16 | (ry16 << 16), Relaxed);
}

pub fn read_gamepad() -> Option<(u16, u8, u8, i16, i16, i16, i16)> {
    use std::sync::atomic::Ordering::Relaxed;
    if !ANDROID_PAD_CONNECTED.load(Relaxed) {
        return None;
    }
    let packed_btn = ANDROID_PAD_BUTTONS.load(Relaxed);
    let packed_l = ANDROID_PAD_AXES_L.load(Relaxed);
    let packed_r = ANDROID_PAD_AXES_R.load(Relaxed);
    let buttons = (packed_btn & 0xFFFF) as u16;
    let lt = ((packed_btn >> 16) & 0xFF) as u8;
    let rt = ((packed_btn >> 24) & 0xFF) as u8;
    let lx = (packed_l & 0xFFFF) as u16 as i16;
    let ly = ((packed_l >> 16) & 0xFFFF) as u16 as i16;
    let rx = (packed_r & 0xFFFF) as u16 as i16;
    let ry = ((packed_r >> 16) & 0xFFFF) as u16 as i16;
    Some((buttons, lt, rt, lx, ly, rx, ry))
}

impl FpsLimit {
    pub const ALL: [Self; 3] = [Self::Fps30, Self::Fps60, Self::Unlimited];

    pub fn name(self) -> &'static str {
        match self {
            Self::Fps30 => "30 FPS",
            Self::Fps60 => "60 FPS",
            Self::Unlimited => "Unlimited",
        }
    }

    pub fn next(self, delta: f32) -> Self {
        let index = Self::ALL.iter().position(|&p| p == self).unwrap_or(1) as i32;
        let step = if delta < 0. { -1 } else { 1 };
        Self::ALL[(index + step).rem_euclid(Self::ALL.len() as i32) as usize]
    }

    pub fn target_frame_us(self) -> u64 {
        match self {
            Self::Fps30 => 33_333,
            Self::Fps60 => 16_666,
            Self::Unlimited => 0,
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
/// Priority order (first existing/writable wins):
///   1. $LOOKING_GLASS_SETTINGS_DIR     (set by Java on every launch — canonical)
///   2. $LOOKING_GLASS_ANDROID_STORAGE  (legacy name, same value)
///   3. $LOOKING_GLASS_ANDROID_DIR      (older legacy name)
///   4. Android canonical external-files dir (getExternalFilesDir-equivalent paths)
///   5. app-internal data dir (getFilesDir equivalent)
///   6. ./private (desktop only; never used on Android)
pub fn storage_root() -> PathBuf {
    for key in [
        "LOOKING_GLASS_SETTINGS_DIR",
        "LOOKING_GLASS_ANDROID_STORAGE",
        "LOOKING_GLASS_ANDROID_DIR",
    ] {
        if let Some(dir) = std::env::var_os(key)
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
        {
            if fs::create_dir_all(&dir).is_ok() {
                return dir;
            }
        }
    }
    if cfg!(target_os = "android") {
        let package = PACKAGE_ID;
        let candidates: [PathBuf; 6] = [
            PathBuf::from(format!("/storage/emulated/0/Android/data/{package}/files")),
            PathBuf::from(format!("/sdcard/Android/data/{package}/files")),
            PathBuf::from(format!("/data/user/0/{package}/files")),
            PathBuf::from(format!("/data/data/{package}/files")),
            PathBuf::from("/storage/emulated/0/LookingGlass/private"),
            PathBuf::from("/sdcard/LookingGlass/private"),
        ];
        for path in candidates {
            if fs::create_dir_all(&path).is_ok() {
                return path;
            }
        }
        // Last-resort: app's internal storage (always writable on Android).
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
    let root = storage_root();
    // Prefer the remembered path written next to storage_root (set by remember_data_dir).
    let saved = fs::read_to_string(root.join("data-path.txt"))
        .or_else(|_| fs::read_to_string("private/data-path.txt"))
        .ok()
        .filter(|path| !path.trim().is_empty())
        .map(|path| PathBuf::from(path.trim()));
    let fallback = saved.unwrap_or_else(|| {
        if is_android() {
            // On Android the canonical game-data folder is <storage>/base.
            root.join("base")
        } else {
            PathBuf::from("alice_202106/Alice1/bin/base")
        }
    });
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

/// Retail PK3 archives the game needs (same set the Windows setup validates).
/// `pak4_english.pk3` is the language pack; any `pak4*.pk3` satisfies that slot.
pub const REQUIRED_PACKS: [&str; 6] = [
    "pak0.pk3",
    "pak1_large.pk3",
    "pak2.pk3",
    "pak3.pk3",
    "pak4_english.pk3",
    "pak5_mod.pk3",
];

/// Names from [`REQUIRED_PACKS`] that are not present in `dir` (case-insensitive).
pub fn missing_required_packs(dir: &Path) -> Vec<&'static str> {
    let present: Vec<String> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| entry.file_name().to_str().map(|n| n.to_ascii_lowercase()))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    REQUIRED_PACKS
        .iter()
        .copied()
        .filter(|required| {
            if required.starts_with("pak4_") {
                !present
                    .iter()
                    .any(|name| name.starts_with("pak4") && name.ends_with(".pk3"))
            } else {
                !present.iter().any(|name| name.as_str() == *required)
            }
        })
        .collect()
}

fn missing_packs_message(missing: &[&str]) -> String {
    format!(
        "Missing game files: {}. Use Choose Game Folder or Select PK3 / ZIP.",
        missing.join(", ")
    )
}

/// Resolve the game folder, require the complete PK3 set and mount it.
/// Returns a user-facing message instead of starting with an incomplete install.
fn open_ready_assets(initial: &Path, root: &Path) -> std::result::Result<Assets, String> {
    let resolved = resolve_data_dir(initial);
    let missing = missing_required_packs(&resolved);
    if !missing.is_empty() {
        return Err(missing_packs_message(&missing));
    }
    // Create <storage>/mods/ if missing and layer any pk3s there on top of base.
    let mods_dir = root.join("mods");
    let _ = std::fs::create_dir_all(&mods_dir);
    match crate::assets::Assets::open_with_mods(&resolved, Some(&mods_dir)) {
        Ok(assets) => {
            remember_data_dir(root, &resolved);
            Ok(assets)
        }
        Err(e) => Err(format!(
            "Cannot open game files in {}: {e:#}",
            resolved.display()
        )),
    }
}

/// Greedy word wrap for the launcher status line (no font metrics needed).
fn wrap_status(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > max_chars {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
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
        let legacy_android: [PathBuf; 12] = [
            PathBuf::from(format!("/storage/emulated/0/Android/data/{PACKAGE_ID}/files/base")),
            PathBuf::from(format!("/sdcard/Android/data/{PACKAGE_ID}/files/base")),
            PathBuf::from("/storage/emulated/0/LookingGlass/base"),
            PathBuf::from("/storage/emulated/0/LookingGlass"),
            PathBuf::from("/storage/emulated/0/Alice1/bin/base"),
            PathBuf::from("/storage/emulated/0/Download/LookingGlass/base"),
            PathBuf::from("/storage/emulated/0/Download/Alice1/bin/base"),
            PathBuf::from("/sdcard/LookingGlass/base"),
            PathBuf::from("/sdcard/LookingGlass"),
            PathBuf::from("/sdcard/Alice1/bin/base"),
            PathBuf::from(format!("/data/user/0/{PACKAGE_ID}/files/base")),
            PathBuf::from(format!("/data/data/{PACKAGE_ID}/files/base")),
        ];
        for path in legacy_android {
            dirs.push(path);
        }
    }
    dirs.push(PathBuf::from("alice_202106/Alice1/bin/base"));
    dirs.push(PathBuf::from("base"));
    dirs
}

pub fn resolve_data_dir(configured: &Path) -> PathBuf {
    // Prefer a folder that holds the complete PK3 set; otherwise remember the first
    // partial one so the launcher can report exactly which packs are missing.
    let mut first_partial = None;
    if let Some(found) = find_pk3_dir(configured) {
        if missing_required_packs(&found).is_empty() {
            return found;
        }
        first_partial = Some(found);
    }
    let root = storage_root();
    for candidate in candidate_data_dirs(&root) {
        if let Some(found) = find_pk3_dir(&candidate) {
            if missing_required_packs(&found).is_empty() {
                return found;
            }
            if first_partial.is_none() {
                first_partial = Some(found);
            }
        }
    }
    if let Some(found) = first_partial {
        return found;
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
    let mods = root.join("mods");
    let _ = fs::create_dir_all(&base);
    let _ = fs::create_dir_all(&saves);
    let _ = fs::create_dir_all(&mods);

    // Best-effort migration of saves / pk3s / config files from legacy storage
    // locations into the canonical root. Runs every launch but never overwrites
    // existing files and never deletes anything from the source — it only fills
    // in what is missing, so it is safe across upgrades.
    if is_android() {
        migrate_legacy_user_data(root);
    }

    let readme = base.join("PLACE_PK3_FILES_HERE.txt");
    if !readme.exists() {
        let _ = fs::write(
            readme,
            "Looking Glass (Android)\n\
             Copy your compatible American McGee's Alice (2011) PK3 files into this folder:\n\
             - pak0.pk3\n\
             - pak1_large.pk3\n\
             - pak2.pk3\n\
             - pak3.pk3\n\
             - pak4_english.pk3\n\
             - pak5_mod.pk3\n",
        );
    }
}

/// Copy any saves/, *.pk3 and top-level config files from legacy locations into
/// `root`. Safe to call repeatedly.
fn migrate_legacy_user_data(root: &Path) {
    let canon_base = root.join("base");
    let canon_saves = root.join("saves");
    let _ = fs::create_dir_all(&canon_base);
    let _ = fs::create_dir_all(&canon_saves);

    let legacy_roots: &[&str] = &[
        "/storage/emulated/0/LookingGlass/private",
        "/sdcard/LookingGlass/private",
        "/storage/emulated/0/LookingGlass",
        "/sdcard/LookingGlass",
    ];
    let package = PACKAGE_ID;
    let internal_paths = [
        format!("/data/user/0/{package}/files"),
        format!("/data/data/{package}/files"),
    ];

    let mut all_paths: Vec<PathBuf> = legacy_roots.iter().map(PathBuf::from).collect();
    for p in internal_paths {
        let pb = PathBuf::from(p);
        if pb != root {
            all_paths.push(pb);
        }
    }
    // The Rust-side old fallback was `./private` when CWD was the app's home.
    let cwd_private = std::env::current_dir().ok().map(|c| c.join("private"));
    if let Some(p) = cwd_private {
        if p != root {
            all_paths.push(p);
        }
    }

    for legacy in all_paths {
        if !legacy.is_dir() {
            continue;
        }
        // PK3 candidates: <legacy>/, <legacy>/base/, <legacy>/Alice1/bin/base/
        for sub in ["", "base", "Alice1/bin/base"] {
            let dir = if sub.is_empty() {
                legacy.clone()
            } else {
                legacy.join(sub)
            };
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if !p.is_file() {
                        continue;
                    }
                    let name_lower = p
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_ascii_lowercase())
                        .unwrap_or_default();
                    if name_lower.ends_with(".pk3") {
                        let dest = canon_base.join(p.file_name().unwrap());
                        if !dest.exists() {
                            let _ = fs::copy(&p, &dest);
                        }
                    }
                }
            }
        }
        // Migrate saves/ tree
        let legacy_saves = legacy.join("saves");
        if legacy_saves.is_dir() {
            copy_dir_missing(&legacy_saves, &canon_saves);
        }
        // Top-level config files
        if let Ok(entries) = fs::read_dir(&legacy) {
            for entry in entries.flatten() {
                let p = entry.path();
                if !p.is_file() {
                    continue;
                }
                let name = match p.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };
                if name == "data-path.txt"
                    || name.ends_with(".cfg")
                    || name.ends_with(".json")
                    || name == "crash.log"
                {
                    let dest = root.join(&name);
                    if !dest.exists() {
                        let _ = fs::copy(&p, &dest);
                    }
                }
            }
        }
    }
}

fn copy_dir_missing(src: &Path, dst: &Path) {
    let _ = fs::create_dir_all(dst);
    let Ok(entries) = fs::read_dir(src) else { return };
    for entry in entries.flatten() {
        let p = entry.path();
        let out = dst.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_dir_missing(&p, &out);
        } else if p.is_file() && !out.exists() {
            let _ = fs::copy(&p, &out);
        }
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

/// Open Player1444's Telegram channel (`https://t.me/player1444ports`).
pub fn open_telegram_link() -> bool {
    call_activity_void("openTelegramLink")
}

/// Open the original LookingGlass repository (`https://github.com/skulitom/LookingGlass`).
pub fn open_github_link() -> bool {
    call_activity_void("openGithubLink")
}

/// Show the localized Startup Credits & Links window (`MainActivity.showCreditsDialog()`).
pub fn show_credits_dialog() -> bool {
    call_activity_void("showCreditsDialog")
}

/// Ask the activity to drop the render buffer from the sharp launcher size to the game's
/// 540p size (`MainActivity.enterGameRenderMode()`). Returns `true` when the call was made.
pub fn enter_game_render_mode() -> bool {
    call_activity_void("enterGameRenderMode")
}

/// Trigger an asynchronous GitHub update check (`MainActivity.checkForUpdates()`).
pub fn check_for_updates() -> bool {
    call_activity_void("checkForUpdates")
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
#[allow(dead_code)]
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
    let assets = wait_for_data_inner(initial).await?;
    // The launcher is drawn in a sharp, (almost) native-resolution buffer. The game itself keeps
    // running in the small buffer it was tuned for, so switch before it starts.
    if enter_game_render_mode() {
        let start = std::time::Instant::now();
        while macroquad::prelude::screen_width().min(macroquad::prelude::screen_height()) > 600.
            && start.elapsed() < std::time::Duration::from_millis(2500)
        {
            clear_background(BLACK);
            next_frame().await;
        }
        // Let the new buffer settle for a few frames before the game allocates render targets.
        for _ in 0..3 {
            clear_background(BLACK);
            next_frame().await;
        }
    }
    Ok(assets)
}

/// Draws launcher text, shrinking it (never growing it) so it fits into `max_w` pixels.
fn draw_text_fit(text: &str, x: f32, y: f32, size: f32, max_w: f32, color: Color) {
    let mut fs = size.round().max(8.);
    let width = measure_text(text, None, fs as u16, 1.0).width;
    if width > max_w && width > 0. {
        fs = (fs * max_w / width).floor().max(8.);
    }
    draw_text(text, x, y, fs, color);
}

async fn wait_for_data_inner(initial: PathBuf) -> Result<Assets> {
    let root = storage_root();
    prepare_directories(&root);
    clear_import_status(&root);
    let mut prefs = crate::preferences::Preferences::load();
    prefs.display();
    let initial_dir = resolve_data_dir(&initial);
    let initial_missing = missing_required_packs(&initial_dir);
    let mut status = if has_pk3_archives(&initial_dir) && initial_missing.is_empty() {
        format!(
            "Game data ready in {}. Select FPS / Preset and tap START GAME.",
            initial_dir.display()
        )
    } else if has_pk3_archives(&initial_dir) {
        missing_packs_message(&initial_missing)
    } else {
        format!(
            "Select your game folder or PK3 archive to copy into {}",
            root.join("base").display()
        )
    };
    let mut browsing = false;
    let mut editing_touch = false;
    let mut touch_editor = crate::touch::TouchEditor::default();
    let mut browse_dir = initial_browse_dir(&root);
    let mut browse_scroll = 0usize;
    let mut frame = 0u32;
    let mut data_ready = false;

    loop {
        if is_quit_requested() {
            anyhow::bail!("Setup cancelled before game data was mounted");
        }

        if editing_touch {
            if touch_editor.update_and_draw(&mut prefs) {
                editing_touch = false;
                status = format!(
                    "Touch controls saved (Mode: {}).",
                    prefs.touch_mode.name()
                );
            }
            next_frame().await;
            continue;
        }

        if let Some((state, msg)) = read_import_status(&root) {
            if !msg.is_empty() {
                status = msg;
            }
            if state == "DONE" {
                clear_import_status(&root);
                match open_ready_assets(&initial, &root) {
                    Ok(assets) => return Ok(assets),
                    Err(msg) => status = format!("Import finished. {msg}"),
                }
            } else if state == "ERROR" || state == "IDLE" {
                clear_import_status(&root);
                if state == "ERROR" {
                    let resolved = resolve_data_dir(&initial);
                    let missing = missing_required_packs(&resolved);
                    if has_pk3_archives(&resolved) && !missing.is_empty() {
                        status = format!("{status} {}", missing_packs_message(&missing));
                    }
                }
            }
        }

        // Scanning storage every frame is wasteful; refresh a few times per second.
        if frame % 20 == 0 {
            let resolved = resolve_data_dir(&initial);
            data_ready =
                has_pk3_archives(&resolved) && missing_required_packs(&resolved).is_empty();
        }
        frame = frame.wrapping_add(1);
        let (w, h) = (screen_width(), screen_height());
        // UI scale: the launcher renders at (almost) native resolution; the panel is
        // 960x560 design units and always fits the screen.
        let s = (h / 560.)
            .clamp(0.65, 2.4)
            .min(w / 1000.)
            .min((h - 24.).max(200.) / 560.);
        clear_background(Color::from_hex(0x141118));
        let panel = Rect::new(
            (w - 960. * s).max(20.) * 0.5,
            (h - 560. * s).max(16.) * 0.5,
            (w - 40.).min(960. * s),
            (h - 32.).min(560. * s),
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
        let pointer_pressed = {
            // Rely on touch.rs's pointer_state(), which correctly handles Android
            // touch edge detection (Started phase filtering, draining, etc.).
            // Use the _pressed_ edge only so holding a finger doesn't re-fire
            // selections every frame.
            let (_pos, pressed, _down) = crate::touch::pointer_state();
            pressed
        };
        // Position of the first active touch (or mouse) for hit-testing.
        let pointer_pos = touches()
            .iter()
            .find(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled))
            .map(|t| t.position)
            .or_else(|| is_mouse_button_down(MouseButton::Left).then(|| Vec2::from(mouse_position())));

        let x = panel.x + 26. * s;
        let mut y = panel.y + 84. * s;
        draw_text_fit(
            "Looking Glass - Android Launcher",
            x,
            y,
            30. * s,
            panel.w - 52. * s,
            Color::from_hex(0xf3e5c8),
        );

        let top_btn_y = panel.y + 10. * s;
        let tg_btn = Rect::new(panel.right() - 568. * s, top_btn_y, 250. * s, 38. * s);
        let upd_btn = Rect::new(panel.right() - 308. * s, top_btn_y, 170. * s, 38. * s);
        let info_btn = Rect::new(panel.right() - 128. * s, top_btn_y, 110. * s, 38. * s);
        for (rect, label, fill, border) in [
            (
                tg_btn,
                "Port by Player1444 (TG)",
                Color::from_hex(0x1b4f72),
                Color::from_hex(0x85c1e9),
            ),
            (
                upd_btn,
                "Check Updates",
                Color::from_hex(0x7d5a2b),
                Color::from_hex(0xf5cba7),
            ),
            (
                info_btn,
                "Credits",
                Color::from_hex(0x4a235a),
                Color::from_hex(0xbb8fce),
            ),
        ] {
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.5 * s, border);
            draw_text_fit(
                label,
                rect.x + 10. * s,
                rect.y + 26. * s,
                18. * s,
                rect.w - 16. * s,
                WHITE,
            );
        }
        if pointer_pressed && pointer_pos.is_some_and(|p| tg_btn.contains(p)) {
            let _ = open_telegram_link();
            status = "Opening https://t.me/player1444ports ...".into();
        } else if pointer_pressed && pointer_pos.is_some_and(|p| upd_btn.contains(p)) {
            let _ = check_for_updates();
            status = "Checking for updates on GitHub...".into();
        } else if pointer_pressed && pointer_pos.is_some_and(|p| info_btn.contains(p)) {
            let _ = show_credits_dialog();
        }

        y += 30. * s;

        if !browsing {
            for line in [
                "Tap 'Choose Game Folder' or 'Select PK3 / ZIP' to import your game, then 'START GAME'.",
                "Needs: pak0, pak1_large, pak2, pak3, pak4_english, pak5_mod (.pk3)",
                &format!("Game files folder: {}", root.join("base").display()),
            ] {
                draw_text_fit(
                    line,
                    x,
                    y,
                    20. * s,
                    panel.w - 52. * s,
                    Color::from_hex(0xd8cbb8),
                );
                y += 25. * s;
            }
            y += 4. * s;
            let status_lines = wrap_status(&status, 66);
            for (i, line) in status_lines.iter().take(4).enumerate() {
                draw_text_fit(
                    line,
                    x,
                    y + i as f32 * 24. * s,
                    21. * s,
                    panel.w - 52. * s,
                    Color::from_hex(if data_ready { 0x82e0aa } else { 0xe59866 }),
                );
            }
            y += (status_lines.len().clamp(1, 4) as f32 - 1.) * 24. * s;

            // FPS Limit Selector Row (30 FPS / 60 FPS / Unlimited)
            let fps_y = y + 18. * s;
            draw_text_fit(
                "FPS Limit:",
                x,
                fps_y + 31. * s,
                23. * s,
                140. * s,
                Color::from_hex(0xf3e5c8),
            );
            for (i, mode) in FpsLimit::ALL.into_iter().enumerate() {
                let btn = Rect::new(x + 150. * s + i as f32 * 190. * s, fps_y, 180. * s, 44. * s);
                let active = prefs.fps_limit == mode;
                draw_rectangle(
                    btn.x,
                    btn.y,
                    btn.w,
                    btn.h,
                    Color::from_hex(if active { 0x7d5a2b } else { 0x2f2638 }),
                );
                draw_rectangle_lines(
                    btn.x,
                    btn.y,
                    btn.w,
                    btn.h,
                    if active { 3. * s } else { 1.5 * s },
                    Color::from_hex(if active { 0xf5cba7 } else { 0x7c6750 }),
                );
                draw_text_fit(
                    mode.name(),
                    btn.x + 20. * s,
                    btn.y + 30. * s,
                    22. * s,
                    btn.w - 30. * s,
                    if active { Color::from_hex(0xfff2d6) } else { WHITE },
                );
                if pointer_pressed && pointer_pos.is_some_and(|p| btn.contains(p)) {
                    prefs.fps_limit = mode;
                    set_active_fps_limit(mode);
                    let _ = prefs.save();
                    status = format!("FPS Limit set to {} (saved).", mode.name());
                }
            }

            // Graphics Preset Selector Row (Auto / Quality / Balanced / Performance)
            let preset_y = fps_y + 52. * s;
            draw_text_fit(
                "Graphics:",
                x,
                preset_y + 31. * s,
                23. * s,
                140. * s,
                Color::from_hex(0xf3e5c8),
            );
            for (i, preset) in PerformancePreset::ALL.into_iter().enumerate() {
                let btn = Rect::new(x + 150. * s + i as f32 * 178. * s, preset_y, 168. * s, 44. * s);
                let active = prefs.performance_preset == preset;
                draw_rectangle(
                    btn.x,
                    btn.y,
                    btn.w,
                    btn.h,
                    Color::from_hex(if active { 0x5c3d6e } else { 0x2f2638 }),
                );
                draw_rectangle_lines(
                    btn.x,
                    btn.y,
                    btn.w,
                    btn.h,
                    if active { 3. * s } else { 1.5 * s },
                    Color::from_hex(if active { 0xe8daef } else { 0x7c6750 }),
                );
                draw_text_fit(
                    preset.name(),
                    btn.x + 14. * s,
                    btn.y + 30. * s,
                    21. * s,
                    btn.w - 24. * s,
                    if active { Color::from_hex(0xfff2d6) } else { WHITE },
                );
                if pointer_pressed && pointer_pos.is_some_and(|p| btn.contains(p)) {
                    prefs.performance_preset = preset;
                    set_active_preset(preset);
                    let _ = prefs.save();
                    status = format!("Graphics Preset set to {} (saved).", preset.name());
                }
            }

            // Touch Controls & Gamepad Mode Row (Touch Auto / On / Off + Edit Touch HUD)
            let ctrl_y = preset_y + 52. * s;
            draw_text_fit(
                "Controls:",
                x,
                ctrl_y + 31. * s,
                23. * s,
                140. * s,
                Color::from_hex(0xf3e5c8),
            );
            for (i, (mode, label)) in [
                (crate::touch::TouchMode::Auto, "Touch: Auto"),
                (crate::touch::TouchMode::On, "Touch: ON"),
                (crate::touch::TouchMode::Off, "Touch: OFF (Pad)"),
            ]
            .into_iter()
            .enumerate()
            {
                let btn = Rect::new(x + 150. * s + i as f32 * 178. * s, ctrl_y, 168. * s, 44. * s);
                let active = prefs.touch_mode == mode;
                let fill = if active {
                    if mode == crate::touch::TouchMode::Off {
                        0x7b241c
                    } else {
                        0x276e36
                    }
                } else {
                    0x2f2638
                };
                draw_rectangle(btn.x, btn.y, btn.w, btn.h, Color::from_hex(fill));
                draw_rectangle_lines(
                    btn.x,
                    btn.y,
                    btn.w,
                    btn.h,
                    if active { 3. * s } else { 1.5 * s },
                    Color::from_hex(if active { 0xf5cba7 } else { 0x7c6750 }),
                );
                draw_text_fit(
                    label,
                    btn.x + 12. * s,
                    btn.y + 30. * s,
                    20. * s,
                    btn.w - 20. * s,
                    if active { Color::from_hex(0xfff2d6) } else { WHITE },
                );
                if pointer_pressed && pointer_pos.is_some_and(|p| btn.contains(p)) {
                    prefs.touch_mode = mode;
                    let _ = prefs.save();
                    status = if mode == crate::touch::TouchMode::Off {
                        "Touch controls DISABLED (Gamepad mode active).".into()
                    } else {
                        format!("Controls mode set to {} (saved).", label)
                    };
                }
            }
            let edit_hud_btn = Rect::new(x + 150. * s + 3. * 178. * s, ctrl_y, 200. * s, 44. * s);
            draw_rectangle(
                edit_hud_btn.x,
                edit_hud_btn.y,
                edit_hud_btn.w,
                edit_hud_btn.h,
                Color::from_hex(0x1f618d),
            );
            draw_rectangle_lines(
                edit_hud_btn.x,
                edit_hud_btn.y,
                edit_hud_btn.w,
                edit_hud_btn.h,
                2. * s,
                Color::from_hex(0x85c1e9),
            );
            draw_text_fit(
                "Edit Touch HUD",
                edit_hud_btn.x + 14. * s,
                edit_hud_btn.y + 30. * s,
                20. * s,
                edit_hud_btn.w - 24. * s,
                WHITE,
            );
            if pointer_pressed && pointer_pos.is_some_and(|p| edit_hud_btn.contains(p)) {
                editing_touch = true;
                next_frame().await;
                continue;
            }

            let row1_y = panel.bottom() - 130. * s;
            let row2_y = panel.bottom() - 68. * s;
            let folder_btn = Rect::new(x, row1_y, 446. * s, 50. * s);
            let file_btn = Rect::new(x + 462. * s, row1_y, 446. * s, 50. * s);
            let scan_btn = Rect::new(x, row2_y, 908. * s, 50. * s);

            for (rect, label, fill) in [
                (folder_btn, "Choose Game Folder", Color::from_hex(0x6e352c)),
                (file_btn, "Select PK3 / ZIP", Color::from_hex(0x4a354f)),
                (
                    scan_btn,
                    "START GAME",
                    Color::from_hex(if data_ready { 0x276e36 } else { 0x5b2c24 }),
                ),
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
                draw_text_fit(
                    label,
                    rect.x + 18. * s,
                    rect.y + 35. * s,
                    25. * s,
                    rect.w - 30. * s,
                    WHITE,
                );
            }

            if pointer_pressed && pointer_pos.is_some_and(|p| folder_btn.contains(p)) {
                if open_system_folder_picker() {
                    status = "Opening Android system folder chooser...".into();
                } else {
                    browsing = true;
                    browse_scroll = 0;
                }
            } else if pointer_pressed && pointer_pos.is_some_and(|p| file_btn.contains(p)) {
                if open_system_file_picker() {
                    status = "Opening Android system file chooser (.pk3 / .zip)...".into();
                } else {
                    browsing = true;
                    browse_scroll = 0;
                }
            }

            let pad_start = read_gamepad().is_some_and(|(btns, _, _, _, _, _, _)| {
                btns & (0x0010 | 0x1000) != 0
            });
            let trigger_scan = is_key_pressed(KeyCode::Enter)
                || is_key_pressed(KeyCode::Space)
                || pad_start
                || pointer_pressed && pointer_pos.is_some_and(|p| scan_btn.contains(p));
            let trigger_quit = is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Back);
            if trigger_quit {
                macroquad::miniquad::window::request_quit();
                anyhow::bail!("Closed from data setup screen");
            }
            if trigger_scan {
                prepare_directories(&root);
                match open_ready_assets(&initial, &root) {
                    Ok(assets) => return Ok(assets),
                    Err(msg) => status = msg,
                }
            }
        } else {
            draw_text_fit(
                &format!("Current Folder: {}", browse_dir.display()),
                x,
                y,
                22. * s,
                panel.w - 52. * s,
                Color::from_hex(0xd8cbb8),
            );
            y += 28. * s;
            draw_text_fit(
                &status,
                x,
                y,
                21. * s,
                panel.w - 52. * s,
                Color::from_hex(0xe59866),
            );
            y += 16. * s;

            let subdirs = list_browsable_subdirs(&browse_dir);
            let max_rows = 6usize;
            let row_h = 44. * s;
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
                draw_text_fit(
                    &format!("[DIR] {name}{badge}"),
                    row_rect.x + 14. * s,
                    row_rect.y + 30. * s,
                    22. * s,
                    row_rect.w - 28. * s,
                    WHITE,
                );
                if pointer_pressed && pointer_pos.is_some_and(|p| row_rect.contains(p)) {
                    browse_dir = dir_path.clone();
                    browse_scroll = 0;
                }
            }

            let btn_y = panel.bottom() - 72. * s;
            let up_btn = Rect::new(x, btn_y, 150. * s, 52. * s);
            let prev_btn = Rect::new(x + 162. * s, btn_y, 120. * s, 52. * s);
            let next_btn = Rect::new(x + 294. * s, btn_y, 130. * s, 52. * s);
            let copy_btn = Rect::new(x + 436. * s, btn_y, 300. * s, 52. * s);
            let back_btn = Rect::new(x + 748. * s, btn_y, 150. * s, 52. * s);

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
                draw_text_fit(
                    label,
                    rect.x + 14. * s,
                    rect.y + 34. * s,
                    22. * s,
                    rect.w - 24. * s,
                    WHITE,
                );
            }

            if pointer_pressed && pointer_pos.is_some_and(|p| up_btn.contains(p)) {
                if let Some(parent) = browse_dir.parent() {
                    browse_dir = parent.to_path_buf();
                    browse_scroll = 0;
                }
            } else if pointer_pressed && pointer_pos.is_some_and(|p| prev_btn.contains(p)) {
                browse_scroll = browse_scroll.saturating_sub(max_rows);
            } else if pointer_pressed && pointer_pos.is_some_and(|p| next_btn.contains(p)) {
                if browse_scroll + max_rows < subdirs.len() {
                    browse_scroll += max_rows;
                }
            } else if pointer_pressed && pointer_pos.is_some_and(|p| copy_btn.contains(p)) {
                let dest_base = root.join("base");
                match import_game_dir(&browse_dir, &dest_base) {
                    Ok(count) => match open_ready_assets(&initial, &root) {
                        Ok(assets) => return Ok(assets),
                        Err(msg) => {
                            status = format!("Copied {count} archive(s). {msg}");
                            browsing = false;
                        }
                    },
                    Err(e) => {
                        status = format!("{e}");
                    }
                }
            } else if is_key_pressed(KeyCode::Escape)
                || is_key_pressed(KeyCode::Back)
                || pointer_pressed && pointer_pos.is_some_and(|p| back_btn.contains(p))
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
    fn reports_missing_required_packs() -> Result<()> {
        let temp = std::env::temp_dir().join(format!("lg-android-packs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp)?;
        assert_eq!(missing_required_packs(&temp).len(), REQUIRED_PACKS.len());
        for name in ["pak0.pk3", "PAK1_LARGE.PK3", "pak3.pk3", "pak4_english.pk3"] {
            fs::write(temp.join(name), b"x")?;
        }
        assert_eq!(
            missing_required_packs(&temp),
            vec!["pak2.pk3", "pak5_mod.pk3"]
        );
        for name in ["pak2.pk3", "pak5_mod.pk3"] {
            fs::write(temp.join(name), b"x")?;
        }
        assert!(missing_required_packs(&temp).is_empty());
        assert!(missing_required_packs(&temp.join("does-not-exist")).len() == REQUIRED_PACKS.len());
        fs::remove_dir_all(&temp)?;
        Ok(())
    }

    #[test]
    fn status_text_wraps_on_word_boundaries() {
        let lines = wrap_status("alpha beta gamma delta", 11);
        assert_eq!(lines, vec!["alpha beta", "gamma delta"]);
        assert!(wrap_status("", 10).is_empty());
        assert_eq!(wrap_status("superlongword", 4), vec!["superlongword"]);
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
