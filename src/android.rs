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

/// Interactive in-window setup screen when launched on Android before PK3 data is copied.
pub async fn wait_for_data(initial: PathBuf) -> Result<Assets> {
    let root = storage_root();
    prepare_directories(&root);
    let mut status = format!(
        "No PK3 archives found yet in {}",
        initial.display()
    );
    loop {
        if is_quit_requested() {
            anyhow::bail!("Setup cancelled before game data was mounted");
        }
        let (w, h) = (screen_width(), screen_height());
        let s = (h / 720.).clamp(0.65, 2.2).min(w / 960.);
        clear_background(Color::from_hex(0x141118));
        let panel = Rect::new(
            (w - 820. * s).max(24.) * 0.5,
            (h - 500. * s).max(24.) * 0.5,
            (w - 48.).min(820. * s),
            (h - 48.).min(500. * s),
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
        let x = panel.x + 28. * s;
        let mut y = panel.y + 44. * s;
        draw_text(
            "Looking Glass - Game Data Setup",
            x,
            y,
            (32. * s).round(),
            Color::from_hex(0xf3e5c8),
        );
        y += 38. * s;
        for line in [
            "Copy your compatible American McGee's Alice (2011) PK3 files to:",
            &format!("  {}", root.join("base").display()),
            "or place them in /storage/emulated/0/LookingGlass/base",
            "Required archives: pak0.pk3, pak1_large.pk3, pak2_small.pk3, pak3.pk3, pak4_english.pk3",
        ] {
            draw_text(line, x, y, (20. * s).round(), Color::from_hex(0xd8cbb8));
            y += 28. * s;
        }
        y += 12. * s;
        draw_text(
            &status,
            x,
            y,
            (19. * s).round(),
            Color::from_hex(0xe59866),
        );

        let scan_btn = Rect::new(x, panel.bottom() - 78. * s, 240. * s, 50. * s);
        let quit_btn = Rect::new(x + 264. * s, panel.bottom() - 78. * s, 160. * s, 50. * s);
        let pointer = touches()
            .iter()
            .find(|t| t.phase == TouchPhase::Started)
            .map(|t| t.position)
            .or_else(|| {
                is_mouse_button_pressed(MouseButton::Left).then(|| Vec2::from(mouse_position()))
            });
        for (rect, label, fill) in [
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
                rect.x + 24. * s,
                rect.y + 32. * s,
                (23. * s).round(),
                WHITE,
            );
        }
        let trigger_scan = is_key_pressed(KeyCode::Enter)
            || is_key_pressed(KeyCode::Space)
            || pointer.is_some_and(|p| scan_btn.contains(p));
        let trigger_quit = is_key_pressed(KeyCode::Escape)
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
