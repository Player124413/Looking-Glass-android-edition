//! Gameplay bindings and Windows XInput. Menu controls remain fixed and usable
//! even when every gameplay action has been rebound. No original DLL is loaded.
use crate::preferences::Preferences;
use macroquad::prelude::*;
use std::collections::HashSet;

pub const KEYS: &[(&str, KeyCode)] = &[
    ("W", KeyCode::W),
    ("S", KeyCode::S),
    ("A", KeyCode::A),
    ("D", KeyCode::D),
    ("Space", KeyCode::Space),
    ("E", KeyCode::E),
    ("Shift", KeyCode::LeftShift),
    ("C", KeyCode::C),
    ("B", KeyCode::B),
    ("G", KeyCode::G),
    ("J", KeyCode::J),
    ("K", KeyCode::K),
    ("L", KeyCode::L),
    ("N", KeyCode::N),
    ("O", KeyCode::O),
    ("T", KeyCode::T),
    ("U", KeyCode::U),
    ("Y", KeyCode::Y),
    ("Z", KeyCode::Z),
    ("Q", KeyCode::Q),
    ("R", KeyCode::R),
    ("F", KeyCode::F),
    ("H", KeyCode::H),
    ("I", KeyCode::I),
    ("M", KeyCode::M),
    ("P", KeyCode::P),
    ("V", KeyCode::V),
    ("X", KeyCode::X),
    ("Ctrl", KeyCode::LeftControl),
    ("Alt", KeyCode::LeftAlt),
    ("R Shift", KeyCode::RightShift),
    ("R Ctrl", KeyCode::RightControl),
    ("R Alt", KeyCode::RightAlt),
    ("Tab", KeyCode::Tab),
    ("Enter", KeyCode::Enter),
    ("Backspace", KeyCode::Backspace),
    ("Delete", KeyCode::Delete),
    ("Insert", KeyCode::Insert),
    ("Home", KeyCode::Home),
    ("End", KeyCode::End),
    ("Page Up", KeyCode::PageUp),
    ("Page Down", KeyCode::PageDown),
    ("Up", KeyCode::Up),
    ("Down", KeyCode::Down),
    ("Left", KeyCode::Left),
    ("Right", KeyCode::Right),
    ("[", KeyCode::LeftBracket),
    ("]", KeyCode::RightBracket),
    ("-", KeyCode::Minus),
    ("=", KeyCode::Equal),
    (",", KeyCode::Comma),
    (".", KeyCode::Period),
    ("/", KeyCode::Slash),
    (";", KeyCode::Semicolon),
    ("'", KeyCode::Apostrophe),
    ("\\", KeyCode::Backslash),
    ("1", KeyCode::Key1),
    ("2", KeyCode::Key2),
    ("3", KeyCode::Key3),
    ("4", KeyCode::Key4),
    ("5", KeyCode::Key5),
    ("6", KeyCode::Key6),
    ("7", KeyCode::Key7),
    ("8", KeyCode::Key8),
    ("9", KeyCode::Key9),
    ("0", KeyCode::Key0),
    ("F1", KeyCode::F1),
    ("F2", KeyCode::F2),
    ("F3", KeyCode::F3),
    ("F4", KeyCode::F4),
    ("F5", KeyCode::F5),
    ("F6", KeyCode::F6),
    ("F7", KeyCode::F7),
    ("F8", KeyCode::F8),
    ("F9", KeyCode::F9),
    ("F10", KeyCode::F10),
    ("F11", KeyCode::F11),
    ("Num 0", KeyCode::Kp0),
    ("Num 1", KeyCode::Kp1),
    ("Num 2", KeyCode::Kp2),
    ("Num 3", KeyCode::Kp3),
    ("Num 4", KeyCode::Kp4),
    ("Num 5", KeyCode::Kp5),
    ("Num 6", KeyCode::Kp6),
    ("Num 7", KeyCode::Kp7),
    ("Num 8", KeyCode::Kp8),
    ("Num 9", KeyCode::Kp9),
    ("Num +", KeyCode::KpAdd),
    ("Num -", KeyCode::KpSubtract),
    ("Num *", KeyCode::KpMultiply),
    ("Num /", KeyCode::KpDivide),
    ("Num .", KeyCode::KpDecimal),
    ("Num Enter", KeyCode::KpEnter),
];
pub fn key_name(key: KeyCode) -> Option<&'static str> {
    KEYS.iter().find(|(_, k)| *k == key).map(|(s, _)| *s)
}
pub fn key_code(name: &str) -> Option<KeyCode> {
    KEYS.iter().find(|(s, _)| *s == name).map(|(_, k)| *k)
}
pub fn valid_binding(name: &str) -> bool {
    !matches!(name, "F1" | "F2" | "F3" | "F4")
        && (key_code(name).is_some()
            || matches!(
                name,
                "None" | "Mouse 1" | "Mouse 2" | "Mouse 3" | "Wheel Up" | "Wheel Down"
            ))
}
fn mouse_button(name: &str) -> Option<MouseButton> {
    match name {
        "Mouse 1" => Some(MouseButton::Left),
        "Mouse 2" => Some(MouseButton::Right),
        "Mouse 3" => Some(MouseButton::Middle),
        _ => None,
    }
}
fn physical(name: &str, edge: bool) -> bool {
    if let Some(k) = key_code(name) {
        return if edge {
            is_key_pressed(k)
        } else {
            is_key_down(k)
        };
    }
    if let Some(b) = mouse_button(name) {
        return if edge {
            is_mouse_button_pressed(b)
        } else {
            is_mouse_button_down(b)
        };
    }
    match name {
        "Wheel Up" => mouse_wheel().1 > 0.,
        "Wheel Down" => mouse_wheel().1 < 0.,
        _ => false,
    }
}
pub fn capture_binding() -> Option<String> {
    KEYS.iter()
        .find(|(_, k)| is_key_pressed(*k))
        .map(|(s, _)| (*s).to_owned())
        .or_else(|| {
            ["Mouse 1", "Mouse 2", "Mouse 3", "Wheel Up", "Wheel Down"]
                .into_iter()
                .find(|s| physical(s, true))
                .map(str::to_owned)
        })
}

pub const PAD_BUTTONS: &[(&str, u32)] = &[
    ("A", 0x1000),
    ("B", 0x2000),
    ("X", 0x4000),
    ("Y", 0x8000),
    ("LB", 0x100),
    ("RB", 0x200),
    ("Back", 0x20),
    ("L Stick", 0x40),
    ("R Stick", 0x80),
    ("D Up", 1),
    ("D Down", 2),
    ("D Left", 4),
    ("D Right", 8),
    ("LT", 0x10000),
    ("RT", 0x20000),
];
fn pad_mask(name: &str) -> u32 {
    if name == "Start" {
        0x10
    } else {
        PAD_BUTTONS
            .iter()
            .find(|(s, _)| *s == name)
            .map_or(0, |(_, m)| *m)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct RawPad {
    buttons: u16,
    lt: u8,
    rt: u8,
    lx: i16,
    ly: i16,
    rx: i16,
    ry: i16,
}
#[repr(C)]
#[derive(Default)]
struct RawState {
    packet: u32,
    pad: RawPad,
}
#[cfg(windows)]
#[link(name = "xinput")]
extern "system" {
    fn XInputGetState(index: u32, state: *mut RawState) -> u32;
}
fn read_pad(index: u32) -> Option<RawPad> {
    #[cfg(windows)]
    {
        let mut state = RawState::default();
        // Fixed C layout, writable state, index checked by XInput; Windows 10+ system API.
        (unsafe { XInputGetState(index, &mut state) } == 0).then_some(state.pad)
    }
    #[cfg(not(windows))]
    {
        if index == 0 {
            if let Some((buttons, lt, rt, lx, ly, rx, ry)) = crate::android::read_gamepad() {
                return Some(RawPad {
                    buttons,
                    lt,
                    rt,
                    lx,
                    ly,
                    rx,
                    ry,
                });
            }
        }
        None
    }
}
fn stick(x: i16, y: i16, deadzone: f32) -> Vec2 {
    let v = vec2(x as f32 / 32767., y as f32 / 32767.);
    let length = v.length();
    if length <= deadzone {
        Vec2::ZERO
    } else {
        v / length * ((length.min(1.) - deadzone) / (1. - deadzone))
    }
}

#[derive(Default)]
struct Pad {
    connected: bool,
    down: u32,
    pressed: u32,
    blocked: u32,
    move_axis: Vec2,
    look_axis: Vec2,
    neutral_sticks: bool,
    nav_down: u32,
    nav_pressed: u32,
    nav_delay: f32,
}
impl Pad {
    fn update(&mut self, raw: Option<RawPad>, focused: bool, deadzone: f32, dt: f32) {
        let connected = raw.is_some();
        let raw = raw.unwrap_or_default();
        let down = raw.buttons as u32
            | if raw.lt > 30 { 0x10000 } else { 0 }
            | if raw.rt > 30 { 0x20000 } else { 0 };
        if !focused || !self.connected {
            self.blocked |= down;
            self.neutral_sticks = true;
        }
        self.blocked &= down;
        let available = if focused { down & !self.blocked } else { 0 };
        self.pressed = available & !self.down;
        self.down = available;
        self.connected = connected;
        let movement = stick(raw.lx, raw.ly, deadzone);
        let look = stick(raw.rx, raw.ry, deadzone);
        if movement == Vec2::ZERO && look == Vec2::ZERO {
            self.neutral_sticks = false;
        }
        (self.move_axis, self.look_axis) = if focused && !self.neutral_sticks {
            (movement, look)
        } else {
            (Vec2::ZERO, Vec2::ZERO)
        };
        let nav = self.down & 15
            | if self.move_axis.y > 0.55 { 1 } else { 0 }
            | if self.move_axis.y < -0.55 { 2 } else { 0 }
            | if self.move_axis.x < -0.55 { 4 } else { 0 }
            | if self.move_axis.x > 0.55 { 8 } else { 0 };
        self.nav_pressed = nav & !self.nav_down;
        if nav != self.nav_down {
            self.nav_delay = 0.38;
        } else if nav != 0 {
            self.nav_delay -= dt;
            if self.nav_delay <= 0. {
                self.nav_pressed |= nav;
                self.nav_delay = 0.12;
            }
        }
        self.nav_down = nav;
    }
    fn suppress(&mut self) {
        self.blocked |= self.down;
        self.down = 0;
        self.pressed = 0;
        self.nav_down = 0;
        self.nav_pressed = 0;
        self.neutral_sticks = true;
        self.move_axis = Vec2::ZERO;
        self.look_axis = Vec2::ZERO;
    }
}

#[derive(Default)]
pub struct Input {
    pad: Pad,
    pub touch: crate::touch::TouchState,
    slot: Option<u32>,
    retry: f64,
    blocked: HashSet<String>,
    pub using_pad: bool,
    pub using_touch: bool,
    pub disconnected: bool,
    focused: bool,
}
impl Input {
    pub fn update(&mut self, prefs: &Preferences, focused: bool) {
        let mut raw = self.slot.and_then(read_pad);
        if raw.is_none() && get_time() >= self.retry {
            self.slot = (0..4).find(|i| {
                if let Some(p) = read_pad(*i) {
                    raw = Some(p);
                    true
                } else {
                    false
                }
            });
            self.retry = get_time() + 1.;
        }
        self.disconnected = self.pad.connected && raw.is_none() && self.using_pad;
        self.pad
            .update(raw, focused, prefs.pad_deadzone, get_frame_time().min(0.1));
        let raw_touches: Vec<crate::touch::TouchPoint> =
            touches().into_iter().map(Into::into).collect();
        self.touch.update(
            &raw_touches,
            vec2(screen_width(), screen_height()),
            prefs,
            focused,
        );
        if !focused {
            self.suppress();
        }
        self.blocked
            .retain(|s| physical(s, false) && !physical(s, true));
        let pad_active = focused
            && (self.pad.pressed != 0
                || self.pad.move_axis != Vec2::ZERO
                || self.pad.look_axis != Vec2::ZERO);
        let key_active = focused && KEYS.iter().any(|(_, k)| is_key_pressed(*k));
        let mouse_active = focused
            && raw_touches.is_empty()
            && !self.touch.active_touches()
            && ([MouseButton::Left, MouseButton::Right, MouseButton::Middle]
                .iter()
                .any(|b| is_mouse_button_pressed(*b))
                || mouse_wheel().1 != 0.);
        if pad_active {
            self.using_pad = true;
            self.using_touch = false;
        }
        if key_active || mouse_active {
            self.using_pad = false;
            if key_active {
                self.using_touch = false;
            }
        }
        match prefs.touch_mode {
            crate::touch::TouchMode::Off => {
                self.using_touch = false;
                if crate::android::is_android() && !key_active && !mouse_active {
                    self.using_pad = true;
                }
            }
            crate::touch::TouchMode::On => {
                self.using_touch = true;
                self.using_pad = false;
            }
            crate::touch::TouchMode::Auto => {
                if !raw_touches.is_empty() {
                    self.using_touch = true;
                    self.using_pad = false;
                } else if crate::android::is_android() && !self.using_pad && !key_active {
                    self.using_touch = true;
                }
            }
        }
        self.focused = focused;
    }
    pub fn set_touch_context(&mut self, ctx: crate::touch::TouchContext) {
        self.touch.set_context(ctx);
    }
    /// On menus/load/focus boundaries, held inputs must be released before reuse.
    pub fn suppress(&mut self) {
        self.pad.suppress();
        self.touch.suppress();
        for &(s, _) in KEYS {
            if physical(s, false) {
                self.blocked.insert(s.into());
            }
        }
        for s in ["Mouse 1", "Mouse 2", "Mouse 3"] {
            if physical(s, false) {
                self.blocked.insert(s.into());
            }
        }
    }
    pub fn capture_pad(&self) -> Option<&'static str> {
        PAD_BUTTONS
            .iter()
            .find(|(_, m)| self.pad.pressed & m != 0)
            .map(|(s, _)| *s)
    }
    pub fn pad_pressed(&self, name: &str) -> bool {
        self.focused && self.pad.pressed & pad_mask(name) != 0
    }
    pub fn pad_held(&self, name: &str) -> bool {
        self.focused && self.pad.down & pad_mask(name) != 0
    }
    pub fn ui(&self, key: KeyCode) -> bool {
        let mask = match key {
            KeyCode::Up => 1,
            KeyCode::Down => 2,
            KeyCode::Left => 4,
            KeyCode::Right => 8,
            _ => 0,
        };
        self.focused
            && (is_key_pressed(key)
                || (key == KeyCode::Escape && is_key_pressed(KeyCode::Back))
                || self.touch.ui_pressed(key)
                || self.pad.nav_pressed & mask != 0
                || match key {
                    KeyCode::Enter => self.pad_pressed("A"),
                    KeyCode::Escape => self.pad_pressed("B") || self.pad_pressed("Start"),
                    _ => false,
                })
    }
    pub fn action(&self, prefs: &Preferences, original: &str, edge: bool) -> bool {
        if !self.focused {
            return false;
        }
        if self.touch.action(original, edge) {
            return true;
        }
        let row = crate::preferences::BINDINGS
            .iter()
            .position(|(_, key)| *key == original);
        let name = row.map_or(original, |i| prefs.bindings[i].as_str());
        let ignore_emulated_mouse =
            self.using_touch && matches!(name, "Mouse 1" | "Mouse 2" | "Mouse 3");
        (!ignore_emulated_mouse && !self.blocked.contains(name) && physical(name, edge))
            || row.is_some_and(|i| {
                let mask = pad_mask(&prefs.pad_bindings[i]);
                (if edge {
                    self.pad.pressed
                } else {
                    self.pad.down
                }) & mask
                    != 0
            })
    }
    pub fn key(&self, prefs: &Preferences, key: KeyCode, edge: bool) -> bool {
        key_name(key).map_or_else(
            || {
                self.focused
                    && if edge {
                        is_key_pressed(key)
                    } else {
                        is_key_down(key)
                    }
            },
            |name| self.action(prefs, name, edge),
        )
    }
    pub fn movement(&self) -> Vec2 {
        (self.pad.move_axis + self.touch.move_axis()).clamp_length_max(1.)
    }
    pub fn look(&self, prefs: &Preferences) -> Vec2 {
        self.pad.look_axis
            * vec2(1., if prefs.invert_pad { -1. } else { 1. })
            * prefs.pad_sensitivity
    }
    pub fn touch_look(&self, prefs: &Preferences) -> Vec2 {
        self.touch.look_radians(screen_height(), prefs)
    }
    pub fn label(&self, prefs: &Preferences, original: &str) -> String {
        if self.using_touch {
            return match original {
                "Mouse 1" => "ATK 1",
                "Mouse 2" => "ATK 2",
                "Space" => "JUMP",
                "E" => "USE",
                "Ctrl" => "DIVE",
                "Shift" => "RUN",
                "C" => "CAT",
                "I" => "TOYS",
                "V" => "VIEW",
                "Tab" => "MAP",
                "H" => "HELP",
                "R" => "FOOT",
                "Enter" => "SKIP",
                "Wheel Up" => "< TOY",
                "Wheel Down" => "TOY >",
                other => other,
            }
            .to_owned();
        }
        crate::preferences::BINDINGS
            .iter()
            .position(|(_, key)| *key == original)
            .map_or_else(
                || original.to_owned(),
                |i| {
                    if self.using_pad && prefs.pad_bindings[i] != "None" {
                        prefs.pad_bindings[i].clone()
                    } else {
                        prefs.bindings[i].clone()
                    }
                },
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn radial_sticks_are_quiet_at_centre_and_preserve_analog_speed() {
        assert_eq!(stick(3000, -4000, 0.24), Vec2::ZERO);
        let half = stick(0, 20000, 0.24);
        assert!(half.y > 0.4 && half.y < 0.6);
        assert!((stick(i16::MAX, i16::MIN, 0.24).length() - 1.).abs() < 0.0001);
    }
    #[test]
    fn reconnect_focus_and_modal_boundaries_require_release() {
        let mut p = Pad::default();
        let held = RawPad {
            buttons: 0x1000,
            rt: 255,
            lx: 20000,
            ..Default::default()
        };
        p.update(Some(held), true, 0.24, 0.016);
        assert_eq!(p.pressed, 0);
        assert_eq!(p.move_axis, Vec2::ZERO);
        p.update(Some(RawPad::default()), true, 0.24, 0.016);
        p.update(Some(held), true, 0.24, 0.016);
        assert_eq!(p.pressed, 0x21000);
        assert!(p.move_axis.x > 0.);
        p.update(Some(held), true, 0.24, 0.016);
        assert_eq!(p.pressed, 0);
        p.suppress();
        p.update(Some(held), true, 0.24, 0.016);
        assert_eq!(p.down, 0);
        p.update(None, true, 0.24, 0.016);
        assert!(!p.connected);
        p.update(Some(held), true, 0.24, 0.016);
        assert_eq!(p.down, 0);
        p.update(Some(held), false, 0.24, 0.016);
        assert_eq!(p.move_axis, Vec2::ZERO);
    }
    #[test]
    fn menu_stick_repeats_without_repeating_accept_or_trigger() {
        let mut p = Pad::default();
        p.update(Some(RawPad::default()), true, 0.24, 0.016);
        let raw = RawPad {
            ly: 32767,
            buttons: 0x1000,
            rt: 31,
            ..Default::default()
        };
        p.update(Some(raw), true, 0.24, 0.016);
        assert_eq!(p.nav_pressed, 1);
        assert_eq!(p.pressed, 0x21000);
        p.update(Some(raw), true, 0.24, 0.2);
        assert_eq!(p.nav_pressed, 0);
        assert_eq!(p.pressed, 0);
        p.update(Some(raw), true, 0.24, 0.2);
        assert_eq!(p.nav_pressed, 1);
        assert_eq!(p.pressed, 0);
    }
}
