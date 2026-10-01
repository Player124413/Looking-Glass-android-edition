//! Runtime preferences, independent of campaign saves and original engine cvars.
use anyhow::{ensure, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn path(name: &str) -> PathBuf {
    std::env::var_os("LOOKING_GLASS_SETTINGS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("private"))
        .join(name)
}

pub const SIZES: [(u32, u32); 8] = [
    (800, 600),
    (1024, 768),
    (1200, 680),
    (1280, 720),
    (1600, 900),
    (1920, 1080),
    (2560, 1440),
    (3840, 2160),
];
// Keep the first eight indices stable when migrating preferences from 0.31.
pub const BINDINGS: &[(&str, &str)] = &[
    ("Forward", "W"),
    ("Backward", "S"),
    ("Step left", "A"),
    ("Step right", "D"),
    ("Jump / climb", "Space"),
    ("Interact", "E"),
    ("Run / walk", "Shift"),
    ("Cheshire hint", "C"),
    ("Dive / descend", "Ctrl"),
    ("Primary attack", "Mouse 1"),
    ("Alternate attack", "Mouse 2"),
    ("Previous toy", "Wheel Up"),
    ("Next toy", "Wheel Down"),
    ("Change view", "V"),
    ("Inventory", "I"),
    ("Choose level", "Tab"),
    ("Help", "H"),
    ("Pause", "P"),
    ("Safe footing", "R"),
    ("Retry / skip scene", "Enter"),
    ("Quick save", "F5"),
    ("Quick load", "F9"),
    ("Restart level", "Home"),
    ("Mute sound", "M"),
    ("Look left", "Left"),
    ("Look right", "Right"),
    ("Look up", "Up"),
    ("Look down", "Down"),
    ("Vorpal Blade", "1"),
    ("Cards", "2"),
    ("Mallet", "3"),
    ("Jackbomb", "4"),
    ("Ice Wand", "5"),
    ("Jacks", "6"),
    ("Demon Dice", "7"),
    ("Eye Staff", "8"),
    ("Blunderbuss", "9"),
    ("Pocket Watch", "0"),
];
fn pad_defaults() -> Vec<String> {
    BINDINGS
        .iter()
        .map(|(_, key)| {
            match *key {
                "Space" => "A",
                "E" => "X",
                "Shift" => "L Stick",
                "C" => "D Up",
                "Ctrl" => "B",
                "Mouse 1" => "RT",
                "Mouse 2" => "LT",
                "Wheel Up" => "LB",
                "Wheel Down" => "RB",
                "V" => "R Stick",
                "I" => "Y",
                "Tab" => "Back",
                "H" => "D Down",
                "P" => "D Right",
                "R" => "D Left",
                _ => "None",
            }
            .to_owned()
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub resolution: usize,
    pub fullscreen: bool,
    pub sensitivity: f32,
    pub invert_mouse: bool,
    pub camera_distance: f32,
    pub always_run: bool,
    pub subtitles: bool,
    pub bindings: Vec<String>,
    pub pad_bindings: Vec<String>,
    pub pad_sensitivity: f32,
    pub pad_deadzone: f32,
    pub invert_pad: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            resolution: 2,
            fullscreen: false,
            sensitivity: 1.,
            invert_mouse: false,
            camera_distance: 132.,
            always_run: false,
            subtitles: true,
            bindings: BINDINGS.iter().map(|(_, key)| (*key).to_owned()).collect(),
            pad_bindings: pad_defaults(),
            pad_sensitivity: 1.,
            pad_deadzone: 0.24,
            invert_pad: false,
        }
    }
}
impl Preferences {
    fn validate(&self) -> Result<()> {
        ensure!(self.resolution < SIZES.len(), "Invalid display size");
        ensure!(
            self.sensitivity.is_finite() && (0.2..=3.).contains(&self.sensitivity),
            "Invalid sensitivity"
        );
        ensure!(
            self.camera_distance.is_finite() && (0. ..=1024.).contains(&self.camera_distance),
            "Invalid camera distance"
        );
        ensure!(
            self.bindings.len() == BINDINGS.len() && self.pad_bindings.len() == BINDINGS.len(),
            "Invalid bindings length"
        );
        ensure!(
            self.pad_sensitivity.is_finite() && (0.2..=3.).contains(&self.pad_sensitivity),
            "Invalid controller sensitivity"
        );
        ensure!(
            self.pad_deadzone.is_finite() && (0.1..=0.4).contains(&self.pad_deadzone),
            "Invalid controller deadzone"
        );
        for (i, key) in self.bindings.iter().enumerate() {
            ensure!(
                crate::input::valid_binding(key)
                    && (key == "None" || !self.bindings[..i].contains(key)),
                "Invalid or duplicate key binding"
            );
        }
        for (i, key) in self.pad_bindings.iter().enumerate() {
            ensure!(
                key == "None"
                    || (crate::input::PAD_BUTTONS.iter().any(|(s, _)| s == key)
                        && !self.pad_bindings[..i].contains(key)),
                "Invalid or duplicate controller binding"
            );
        }
        Ok(())
    }
    pub fn load() -> Self {
        std::fs::read(path("preferences.json"))
            .ok()
            .and_then(|b| Self::parse(&b).ok())
            .unwrap_or_default()
    }
    fn parse(b: &[u8]) -> Result<Self> {
        let mut p: Self = serde_json::from_slice(b)?;
        // Preserve every older user binding. New actions that collide start unbound.
        for (_, default) in BINDINGS.iter().skip(p.bindings.len()) {
            p.bindings.push(
                if p.bindings.iter().any(|key| key == default) {
                    "None"
                } else {
                    default
                }
                .to_string(),
            );
        }
        for default in pad_defaults().into_iter().skip(p.pad_bindings.len()) {
            p.pad_bindings.push(if p.pad_bindings.contains(&default) {
                "None".into()
            } else {
                default
            });
        }
        p.validate()?;
        Ok(p)
    }
    pub fn save(&self) -> Result<()> {
        self.validate()?;
        let path = path("preferences.json");
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
    pub fn display(&self) {
        set_fullscreen(self.fullscreen);
        if !self.fullscreen {
            let (w, h) = SIZES[self.resolution];
            // Menu resolutions are framebuffer pixels, independent of desktop DPI.
            // Fullscreen remains borderless at the desktop's native resolution.
            miniquad::window::set_window_size(w, h);
        }
    }
    pub fn bind(&mut self, row: usize, name: &str, pad: bool) -> bool {
        if row >= BINDINGS.len()
            || if pad {
                name != "None" && !crate::input::PAD_BUTTONS.iter().any(|(s, _)| *s == name)
            } else {
                !crate::input::valid_binding(name)
            }
        {
            return false;
        }
        let bindings = if pad {
            &mut self.pad_bindings
        } else {
            &mut self.bindings
        };
        if let Some(other) = bindings.iter().position(|k| k == name && name != "None") {
            bindings.swap(row, other);
        } else {
            bindings[row] = name.to_owned();
        }
        true
    }
    pub fn run(&self, shift: bool) -> bool {
        self.always_run != shift
    }
    pub fn controls_text(&self, text: &str) -> String {
        text.split(' ')
            .map(|word| {
                BINDINGS
                    .iter()
                    .position(|(_, key)| *key == word)
                    .map_or(word, |i| &self.bindings[i])
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_roundtrip_and_reject_invalid_values() {
        let p = Preferences {
            sensitivity: 2.4,
            invert_mouse: true,
            fullscreen: true,
            ..Default::default()
        };
        assert_eq!(
            Preferences::parse(&serde_json::to_vec(&p).unwrap()).unwrap(),
            p
        );
        assert!(Preferences::parse(br#"{"sensitivity":0}"#).is_err());
        assert!(Preferences::parse(br#"{"resolution":100}"#).is_err());
        assert!(Preferences::parse(br#"{"camera_distance":-1}"#).is_err());
        assert_eq!(Preferences::parse(b"{}").unwrap(), Preferences::default());
    }
    #[test]
    fn rebindings_swap_duplicates_and_preserve_reserved_menu_keys() {
        let mut p = Preferences::default();
        assert!(!p.bind(0, "Escape", false));
        assert!(!p.bind(0, "`", false));
        assert!(p.bind(0, "E", false));
        assert_eq!(p.bindings[0], "E");
        assert_eq!(p.bindings[5], "W");
        p.validate().unwrap();
        assert!(!p.run(false));
        p.always_run = true;
        assert!(p.run(false));
        assert!(!p.run(true));
    }
    #[test]
    fn old_controls_migrate_without_losing_custom_keys_or_other_settings() {
        let p = Preferences::parse(
            br#"{"resolution":5,"bindings":["H","S","A","D","Space","E","Shift","C"]}"#,
        )
        .unwrap();
        assert_eq!(p.resolution, 5);
        assert_eq!(p.bindings[0], "H");
        assert_eq!(p.bindings[16], "None");
        assert_eq!(p.bindings[9], "Mouse 1");
        assert_eq!(p.pad_bindings[9], "RT");
        p.validate().unwrap();
    }
    #[test]
    fn mouse_wheel_and_controller_rebinds_swap_and_roundtrip() {
        let mut p = Preferences::default();
        assert!(p.bind(4, "Mouse 2", false));
        assert_eq!(p.bindings[10], "Space");
        assert!(p.bind(5, "Wheel Up", false));
        assert_eq!(p.bindings[11], "E");
        assert!(p.bind(4, "X", true));
        assert_eq!(p.pad_bindings[5], "A");
        assert!(!p.bind(4, "Start", true));
        assert!(!p.bind(999, "A", true));
        assert!(p.bind(4, "None", true));
        assert_eq!(
            Preferences::parse(&serde_json::to_vec(&p).unwrap()).unwrap(),
            p
        );
    }
}
