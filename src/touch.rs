//! Multi-touch virtual joystick, relative touch-look trackpad and context-aware HUD controls.
use crate::{preferences::Preferences, ui::Ui};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const STICK_DEADZONE: f32 = 0.14;
const STICK_SPRINT_RING: f32 = 0.82;
const LOOK_RADIANS_PER_HEIGHT: f32 = 3.2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TouchMode {
    #[default]
    Auto,
    On,
    Off,
}

impl TouchMode {
    pub const ALL: [Self; 3] = [Self::Auto, Self::On, Self::Off];

    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::On => "Always On",
            Self::Off => "Off",
        }
    }

    pub fn next(self, delta: f32) -> Self {
        let index = Self::ALL.iter().position(|&m| m == self).unwrap_or(0) as i32;
        let step = if delta < 0. { -1 } else { 1 };
        Self::ALL[(index + step).rem_euclid(Self::ALL.len() as i32) as usize]
    }
}

/// Unifies mouse cursor state and primary touch point for menus, chapters and inventory UI.
pub fn pointer_state() -> (Vec2, bool, bool) {
    let ts = touches();
    if let Some(t) = ts.iter().find(|t| {
        matches!(
            t.phase,
            TouchPhase::Started | TouchPhase::Moved | TouchPhase::Stationary
        )
    }) {
        let started = ts.iter().any(|t| t.phase == TouchPhase::Started);
        (
            t.position,
            started || is_mouse_button_pressed(MouseButton::Left),
            true,
        )
    } else {
        (
            Vec2::from(mouse_position()),
            is_mouse_button_pressed(MouseButton::Left),
            is_mouse_button_down(MouseButton::Left),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchPoint {
    pub id: u64,
    pub phase: TouchPhase,
    pub position: Vec2,
}

impl From<Touch> for TouchPoint {
    fn from(t: Touch) -> Self {
        Self {
            id: t.id,
            phase: t.phase,
            position: t.position,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TouchContext {
    pub in_gameplay: bool,
    pub paused: bool,
    pub inventory_open: bool,
    pub help_open: bool,
    pub alive: bool,
    pub swimming: bool,
    pub climbing_rope: bool,
    pub flying: bool,
    pub dialogue_active: bool,
    pub cinematic_active: bool,
    pub can_interact: bool,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchButton {
    PrimaryAttack = 0,
    AlternateAttack = 1,
    Jump = 2,
    Interact = 3,
    Dive = 4,
    RunLock = 5,
    Hint = 6,
    PrevToy = 7,
    NextToy = 8,
    Inventory = 9,
    ViewToggle = 10,
    Help = 11,
    Recover = 12,
    QuickSave = 13,
    QuickLoad = 14,
    SkipScene = 15,
    Menu = 16,
    Chapters = 17,
}

impl TouchButton {
    const fn bit(self) -> u32 {
        1 << (self as u8)
    }

    fn allows_look_drag(self) -> bool {
        matches!(
            self,
            Self::PrimaryAttack | Self::AlternateAttack | Self::Jump | Self::Dive
        )
    }

    fn binding_name(self, alive: bool) -> Option<&'static str> {
        match self {
            Self::PrimaryAttack => Some("Mouse 1"),
            Self::AlternateAttack => Some("Mouse 2"),
            Self::Jump => Some("Space"),
            Self::Interact => Some("E"),
            Self::Dive => Some("Ctrl"),
            Self::RunLock => Some("Shift"),
            Self::Hint => Some("C"),
            Self::PrevToy => Some("Wheel Up"),
            Self::NextToy => Some("Wheel Down"),
            Self::Inventory => Some("I"),
            Self::ViewToggle => Some("V"),
            Self::Help => Some("H"),
            Self::Recover => Some(if alive { "R" } else { "Enter" }),
            Self::QuickSave => Some("F5"),
            Self::QuickLoad => Some("F9"),
            Self::SkipScene => Some("Enter"),
            Self::Chapters => Some("Tab"),
            Self::Menu => None,
        }
    }
}

#[derive(Clone, Copy)]
struct ButtonRegion {
    button: TouchButton,
    center: Vec2,
    radius: f32,
    rect: Option<Rect>,
    label: &'static str,
    highlight: bool,
}

impl ButtonRegion {
    fn circle(
        button: TouchButton,
        center: Vec2,
        radius: f32,
        label: &'static str,
        highlight: bool,
    ) -> Self {
        Self {
            button,
            center,
            radius,
            rect: None,
            label,
            highlight,
        }
    }

    fn pill(button: TouchButton, rect: Rect, label: &'static str, highlight: bool) -> Self {
        Self {
            button,
            center: rect.center(),
            radius: rect.w.max(rect.h) * 0.5,
            rect: Some(rect),
            label,
            highlight,
        }
    }

    fn contains(&self, p: Vec2) -> bool {
        if let Some(r) = self.rect {
            let pad = 4.;
            Rect::new(r.x - pad, r.y - pad, r.w + pad * 2., r.h + pad * 2.).contains(p)
        } else {
            self.center.distance_squared(p) <= (self.radius * 1.12).powi(2)
        }
    }
}

struct Layout {
    scale: f32,
    stick_default: Vec2,
    stick_radius: f32,
    left_handed: bool,
    screen: Vec2,
    buttons: Vec<ButtonRegion>,
}

impl Layout {
    fn build(screen: Vec2, prefs: &Preferences, ctx: TouchContext) -> Self {
        let (w, h) = (screen.x.max(320.), screen.y.max(240.));
        let base_scale = (h / 720.).clamp(0.65, 2.2).min(w / 1080.).max(0.55);
        let s = base_scale * prefs.touch_scale.clamp(0.6, 1.6);
        let left_handed = prefs.touch_left_handed;
        let mirror_x = |x: f32| if left_handed { w - x } else { x };
        let mirror_rect = |r: Rect| {
            if left_handed {
                Rect::new(w - r.x - r.w, r.y, r.w, r.h)
            } else {
                r
            }
        };

        let stick_radius = 68. * s;
        let stick_default = vec2(mirror_x(132. * s), h - 136. * s);
        let mut buttons = Vec::with_capacity(18);

        // Top utility bar (leaves top-center clear for boss health meters).
        let top_y = 14. * s;
        let pill_h = 34. * s;
        let pill_w = 64. * s;
        let gap = 8. * s;
        for (i, (btn, label, highlight)) in [
            (TouchButton::Menu, "MENU", false),
            (TouchButton::Chapters, "MAP", false),
            (TouchButton::ViewToggle, "VIEW", false),
            (TouchButton::Help, "HELP", ctx.help_open),
        ]
        .into_iter()
        .enumerate()
        {
            let x = 18. * s + i as f32 * (pill_w + gap);
            buttons.push(ButtonRegion::pill(
                btn,
                Rect::new(x, top_y, pill_w, pill_h),
                label,
                highlight,
            ));
        }

        for (i, (btn, label, highlight)) in [
            (
                TouchButton::Recover,
                if ctx.alive { "FOOT" } else { "RETRY" },
                !ctx.alive,
            ),
            (TouchButton::QuickSave, "SAVE", false),
            (TouchButton::QuickLoad, "LOAD", false),
        ]
        .into_iter()
        .enumerate()
        {
            let right_edge = w - 18. * s - (2 - i) as f32 * (pill_w + gap) - pill_w;
            buttons.push(ButtonRegion::pill(
                btn,
                Rect::new(right_edge, top_y, pill_w, pill_h),
                label,
                highlight,
            ));
        }

        if ctx.cinematic_active {
            buttons.push(ButtonRegion::pill(
                TouchButton::SkipScene,
                Rect::new(w - 196. * s, 58. * s, 178. * s, 38. * s),
                "HOLD TO SKIP",
                true,
            ));
        }

        if !ctx.alive {
            buttons.push(ButtonRegion::pill(
                TouchButton::Recover,
                Rect::new((w - 220. * s) * 0.5, h * 0.34 + 156. * s, 220. * s, 44. * s),
                "TAP TO RETRY",
                true,
            ));
            return Self {
                scale: s,
                stick_default,
                stick_radius,
                left_handed,
                screen: vec2(w, h),
                buttons,
            };
        }

        if ctx.inventory_open {
            buttons.push(ButtonRegion::pill(
                TouchButton::Inventory,
                Rect::new((w - 180. * s) * 0.5, h - 56. * s, 180. * s, 40. * s),
                "CLOSE TOYS",
                true,
            ));
            return Self {
                scale: s,
                stick_default,
                stick_radius,
                left_handed,
                screen: vec2(w, h),
                buttons,
            };
        }

        // Bottom-center toy selector strip.
        let toy_y = h - 46. * s;
        let toy_w = 66. * s;
        let toy_h = 34. * s;
        buttons.push(ButtonRegion::pill(
            TouchButton::PrevToy,
            Rect::new(w * 0.5 - toy_w * 1.5 - 10. * s, toy_y, toy_w, toy_h),
            "< TOY",
            false,
        ));
        buttons.push(ButtonRegion::pill(
            TouchButton::Inventory,
            Rect::new(w * 0.5 - toy_w * 0.5, toy_y, toy_w, toy_h),
            "TOYS",
            false,
        ));
        buttons.push(ButtonRegion::pill(
            TouchButton::NextToy,
            Rect::new(w * 0.5 + toy_w * 0.5 + 10. * s, toy_y, toy_w, toy_h),
            "TOY >",
            false,
        ));

        // Sprint lock button above the movement stick.
        buttons.push(ButtonRegion::pill(
            TouchButton::RunLock,
            mirror_rect(Rect::new(98. * s, h - 246. * s, 68. * s, 30. * s)),
            "RUN",
            false,
        ));

        // Action cluster on the opposite thumb side.
        let jump_label = if ctx.swimming {
            "RISE"
        } else if ctx.climbing_rope {
            "CLIMB"
        } else if ctx.flying {
            "UP"
        } else {
            "JUMP"
        };
        let use_label = if ctx.dialogue_active {
            "NEXT"
        } else if ctx.climbing_rope {
            "DROP"
        } else {
            "USE"
        };
        let dive_label = if ctx.swimming {
            "DIVE"
        } else if ctx.climbing_rope || ctx.flying {
            "DOWN"
        } else {
            "CROUCH"
        };

        buttons.push(ButtonRegion::circle(
            TouchButton::PrimaryAttack,
            vec2(mirror_x(w - 110. * s), h - 122. * s),
            44. * s,
            "ATK 1",
            false,
        ));
        buttons.push(ButtonRegion::circle(
            TouchButton::AlternateAttack,
            vec2(mirror_x(w - 204. * s), h - 92. * s),
            34. * s,
            "ATK 2",
            false,
        ));
        buttons.push(ButtonRegion::circle(
            TouchButton::Jump,
            vec2(mirror_x(w - 86. * s), h - 222. * s),
            36. * s,
            jump_label,
            ctx.swimming || ctx.climbing_rope,
        ));
        buttons.push(ButtonRegion::circle(
            TouchButton::Interact,
            vec2(mirror_x(w - 182. * s), h - 186. * s),
            32. * s,
            use_label,
            ctx.can_interact || ctx.dialogue_active,
        ));
        buttons.push(ButtonRegion::circle(
            TouchButton::Dive,
            vec2(mirror_x(w - 86. * s), h - 306. * s),
            29. * s,
            dive_label,
            ctx.swimming || ctx.climbing_rope || ctx.flying,
        ));
        buttons.push(ButtonRegion::circle(
            TouchButton::Hint,
            vec2(mirror_x(w - 254. * s), h - 168. * s),
            25. * s,
            "CAT",
            false,
        ));

        Self {
            scale: s,
            stick_default,
            stick_radius,
            left_handed,
            screen: vec2(w, h),
            buttons,
        }
    }

    fn hit_button(&self, p: Vec2) -> Option<TouchButton> {
        self.buttons
            .iter()
            .rev()
            .find(|b| b.contains(p))
            .map(|b| b.button)
    }

    fn in_stick_zone(&self, p: Vec2) -> bool {
        let on_move_side = if self.left_handed {
            p.x > self.screen.x * 0.56
        } else {
            p.x < self.screen.x * 0.44
        };
        on_move_side && p.y > self.screen.y * 0.32
    }
}

#[derive(Default)]
pub struct TouchState {
    pub context: TouchContext,
    stick_touch: Option<u64>,
    stick_origin: Vec2,
    stick_pos: Vec2,
    move_axis: Vec2,
    look_touch: Option<u64>,
    look_prev: Vec2,
    look_delta: Vec2,
    button_touches: BTreeMap<u64, TouchButton>,
    suppressed_ids: BTreeSet<u64>,
    down: u32,
    pressed: u32,
    pub sprint_locked: bool,
    outer_ring_sprint: bool,
    had_touch_input: bool,
}

impl TouchState {
    pub fn set_context(&mut self, context: TouchContext) {
        self.context = context;
    }

    pub fn had_touch(&self) -> bool {
        self.had_touch_input
    }

    pub fn active_touches(&self) -> bool {
        self.stick_touch.is_some()
            || self.look_touch.is_some()
            || !self.button_touches.is_empty()
            || self.down != 0
    }

    pub fn any_touch_down(&self) -> bool {
        self.active_touches()
    }

    pub fn menu_pressed(&self) -> bool {
        self.ui_pressed(KeyCode::Escape)
    }

    pub fn screen_tapped(&self) -> bool {
        touches().iter().any(|t| t.phase == TouchPhase::Started)
            || is_mouse_button_pressed(MouseButton::Left)
    }

    pub fn suppress(&mut self) {
        if let Some(id) = self.stick_touch.take() {
            self.suppressed_ids.insert(id);
        }
        if let Some(id) = self.look_touch.take() {
            self.suppressed_ids.insert(id);
        }
        for id in self.button_touches.keys().copied().collect::<Vec<_>>() {
            self.suppressed_ids.insert(id);
        }
        self.button_touches.clear();
        self.move_axis = Vec2::ZERO;
        self.look_delta = Vec2::ZERO;
        self.down = 0;
        self.pressed = 0;
        self.outer_ring_sprint = false;
    }

    pub fn update(
        &mut self,
        points: &[TouchPoint],
        screen: Vec2,
        prefs: &Preferences,
        focused: bool,
    ) {
        self.look_delta = Vec2::ZERO;
        if !focused || prefs.touch_mode == TouchMode::Off {
            self.suppress();
            return;
        }
        if !points.is_empty() {
            self.had_touch_input = true;
        }

        let live_ids: BTreeSet<u64> = points
            .iter()
            .filter(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled))
            .map(|t| t.id)
            .collect();
        self.suppressed_ids.retain(|id| live_ids.contains(id));

        let layout = Layout::build(screen, prefs, self.context);

        for t in points {
            if self.suppressed_ids.contains(&t.id) || !t.position.is_finite() {
                continue;
            }
            match t.phase {
                TouchPhase::Started => {
                    if let Some(button) = layout.hit_button(t.position) {
                        if button == TouchButton::RunLock {
                            self.sprint_locked = !self.sprint_locked;
                        }
                        self.button_touches.insert(t.id, button);
                        if button.allows_look_drag() && self.look_touch.is_none() {
                            self.look_touch = Some(t.id);
                            self.look_prev = t.position;
                        }
                    } else if self.stick_touch.is_none() && layout.in_stick_zone(t.position) {
                        self.stick_touch = Some(t.id);
                        let margin = layout.stick_radius + 16. * layout.scale;
                        self.stick_origin = vec2(
                            t.position.x.clamp(margin, screen.x - margin),
                            t.position.y.clamp(margin, screen.y - margin),
                        );
                        self.stick_pos = self.stick_origin;
                        self.move_axis = Vec2::ZERO;
                        self.outer_ring_sprint = false;
                    } else if self.look_touch.is_none() {
                        self.look_touch = Some(t.id);
                        self.look_prev = t.position;
                    }
                }
                TouchPhase::Moved | TouchPhase::Stationary => {
                    if self.stick_touch == Some(t.id) {
                        let offset = t.position - self.stick_origin;
                        let max_r = layout.stick_radius.max(1.);
                        let clamped = offset.clamp_length_max(max_r);
                        self.stick_pos = self.stick_origin + clamped;
                        let norm = vec2(offset.x / max_r, -offset.y / max_r);
                        let len = norm.length();
                        if len <= STICK_DEADZONE {
                            self.move_axis = Vec2::ZERO;
                            self.outer_ring_sprint = false;
                        } else {
                            let scaled =
                                ((len.min(1.) - STICK_DEADZONE) / (1. - STICK_DEADZONE)).clamp(0., 1.);
                            self.move_axis = norm / len * scaled;
                            self.outer_ring_sprint = len >= STICK_SPRINT_RING;
                        }
                    }
                    if self.look_touch == Some(t.id) {
                        let delta = t.position - self.look_prev;
                        self.look_prev = t.position;
                        if delta.is_finite() {
                            self.look_delta += delta;
                        }
                    }
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    if self.stick_touch == Some(t.id) {
                        self.stick_touch = None;
                        self.move_axis = Vec2::ZERO;
                        self.outer_ring_sprint = false;
                    }
                    if self.look_touch == Some(t.id) {
                        self.look_touch = None;
                    }
                    self.button_touches.remove(&t.id);
                }
            }
        }

        if self.stick_touch.is_some_and(|id| !live_ids.contains(&id)) {
            self.stick_touch = None;
            self.move_axis = Vec2::ZERO;
            self.outer_ring_sprint = false;
        }
        if self.look_touch.is_some_and(|id| !live_ids.contains(&id)) {
            self.look_touch = None;
        }
        self.button_touches.retain(|id, _| live_ids.contains(id));

        let mut next_down = 0u32;
        for &button in self.button_touches.values() {
            next_down |= button.bit();
        }
        if (self.sprint_locked || self.outer_ring_sprint) && self.move_axis != Vec2::ZERO {
            next_down |= TouchButton::RunLock.bit();
        }
        self.pressed = next_down & !self.down;
        self.down = next_down;
    }

    pub fn move_axis(&self) -> Vec2 {
        self.move_axis
    }

    /// Relative camera rotation in radians for this frame, independent of frame rate.
    pub fn look_radians(&self, screen_height: f32, prefs: &Preferences) -> Vec2 {
        if self.look_delta == Vec2::ZERO {
            return Vec2::ZERO;
        }
        let h = screen_height.max(240.);
        let sens = if prefs.touch_sensitivity.is_finite() && prefs.touch_sensitivity > 0. {
            prefs.touch_sensitivity
        } else {
            1.
        };
        let invert = if prefs.invert_touch { -1. } else { 1. };
        vec2(
            self.look_delta.x / h * LOOK_RADIANS_PER_HEIGHT * sens,
            self.look_delta.y / h * LOOK_RADIANS_PER_HEIGHT * sens * invert,
        )
    }

    pub fn action(&self, original: &str, edge: bool) -> bool {
        let mask = if edge { self.pressed } else { self.down };
        if mask == 0 {
            return false;
        }
        [
            TouchButton::PrimaryAttack,
            TouchButton::AlternateAttack,
            TouchButton::Jump,
            TouchButton::Interact,
            TouchButton::Dive,
            TouchButton::RunLock,
            TouchButton::Hint,
            TouchButton::PrevToy,
            TouchButton::NextToy,
            TouchButton::Inventory,
            TouchButton::ViewToggle,
            TouchButton::Help,
            TouchButton::Recover,
            TouchButton::QuickSave,
            TouchButton::QuickLoad,
            TouchButton::SkipScene,
            TouchButton::Chapters,
        ]
        .into_iter()
        .any(|btn| {
            mask & btn.bit() != 0 && btn.binding_name(self.context.alive) == Some(original)
        })
    }

    pub fn ui_pressed(&self, key: KeyCode) -> bool {
        match key {
            KeyCode::Escape => self.pressed & TouchButton::Menu.bit() != 0,
            KeyCode::Tab => self.pressed & TouchButton::Chapters.bit() != 0,
            KeyCode::Enter => {
                self.pressed & (TouchButton::SkipScene.bit() | TouchButton::Recover.bit()) != 0
            }
            _ => false,
        }
    }

    pub fn draw(&self, ui: &Ui, prefs: &Preferences) {
        if prefs.touch_mode == TouchMode::Off {
            return;
        }
        let screen = vec2(screen_width(), screen_height());
        let layout = Layout::build(screen, prefs, self.context);
        let alpha = prefs.touch_opacity.clamp(0.2, 1.0);
        let s = layout.scale;

        let base_fill = Color::new(0.11, 0.08, 0.13, 0.58 * alpha);
        let active_fill = Color::new(0.58, 0.16, 0.15, 0.82 * alpha);
        let highlight_fill = Color::new(0.36, 0.24, 0.12, 0.72 * alpha);
        let border = Color::new(0.78, 0.64, 0.44, 0.78 * alpha);
        let highlight_border = Color::new(0.96, 0.82, 0.46, 0.95 * alpha);
        let text_color = Color::new(0.95, 0.90, 0.80, 0.92 * alpha);

        // Draw virtual movement stick when alive and inventory is closed.
        if self.context.alive && !self.context.inventory_open && !self.context.paused {
            let center = if self.stick_touch.is_some() {
                self.stick_origin
            } else {
                layout.stick_default
            };
            let knob = if self.stick_touch.is_some() {
                self.stick_pos
            } else {
                center
            };
            let r = layout.stick_radius;
            draw_circle(center.x, center.y, r, base_fill);
            draw_circle_lines(center.x, center.y, r, 2. * s, border);
            draw_circle_lines(
                center.x,
                center.y,
                r * STICK_SPRINT_RING,
                1. * s,
                Color::new(0.78, 0.64, 0.44, 0.28 * alpha),
            );
            let knob_r = 26. * s;
            draw_circle(
                knob.x,
                knob.y,
                knob_r,
                if self.stick_touch.is_some() {
                    active_fill
                } else {
                    Color::new(0.26, 0.19, 0.24, 0.72 * alpha)
                },
            );
            draw_circle_lines(knob.x, knob.y, knob_r, 2. * s, border);
        }

        // Draw buttons.
        for b in &layout.buttons {
            let held = self.down & b.button.bit() != 0
                || (b.button == TouchButton::RunLock && self.sprint_locked);
            let fill = if held {
                active_fill
            } else if b.highlight {
                highlight_fill
            } else {
                base_fill
            };
            let ring = if held || b.highlight {
                highlight_border
            } else {
                border
            };
            if let Some(rect) = b.rect {
                draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
                draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2. * s, ring);
                ui.center(
                    b.label,
                    Rect::new(rect.x + 4. * s, rect.y + 4. * s, rect.w - 8. * s, rect.h - 8. * s),
                    (16. * s).clamp(11., 28.),
                    text_color,
                );
            } else {
                draw_circle(b.center.x, b.center.y, b.radius, fill);
                draw_circle_lines(b.center.x, b.center.y, b.radius, 2.2 * s, ring);
                let w = b.radius * 1.65;
                let h = 20. * s;
                ui.center(
                    b.label,
                    Rect::new(b.center.x - w * 0.5, b.center.y - h * 0.5, w, h),
                    (16.5 * s).clamp(11., 28.),
                    text_color,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multitouch_stick_and_look_operate_independently() {
        let mut touch = TouchState::default();
        touch.set_context(TouchContext {
            in_gameplay: true,
            alive: true,
            ..Default::default()
        });
        let prefs = Preferences::default();
        let screen = vec2(1920., 1080.);

        // Finger 1 starts on left stick zone; Finger 2 starts on right look area.
        touch.update(
            &[
                TouchPoint {
                    id: 1,
                    phase: TouchPhase::Started,
                    position: vec2(220., 880.),
                },
                TouchPoint {
                    id: 2,
                    phase: TouchPhase::Started,
                    position: vec2(1200., 500.),
                },
            ],
            screen,
            &prefs,
            true,
        );
        assert_eq!(touch.move_axis(), Vec2::ZERO);
        assert_eq!(touch.look_radians(screen.y, &prefs), Vec2::ZERO);

        // Move Finger 1 forward-right and drag Finger 2 to look right/up.
        touch.update(
            &[
                TouchPoint {
                    id: 1,
                    phase: TouchPhase::Moved,
                    position: vec2(260., 760.),
                },
                TouchPoint {
                    id: 2,
                    phase: TouchPhase::Moved,
                    position: vec2(1320., 440.),
                },
            ],
            screen,
            &prefs,
            true,
        );
        assert!(touch.move_axis().x > 0.1 && touch.move_axis().y > 0.5);
        assert!(touch.action("Shift", false));
        let look = touch.look_radians(screen.y, &prefs);
        assert!(look.x > 0.1 && look.y < -0.05);

        // Release Finger 1; Finger 2 remains active.
        touch.update(
            &[
                TouchPoint {
                    id: 1,
                    phase: TouchPhase::Ended,
                    position: vec2(260., 760.),
                },
                TouchPoint {
                    id: 2,
                    phase: TouchPhase::Stationary,
                    position: vec2(1320., 440.),
                },
            ],
            screen,
            &prefs,
            true,
        );
        assert_eq!(touch.move_axis(), Vec2::ZERO);
        assert_eq!(touch.look_radians(screen.y, &prefs), Vec2::ZERO);
    }

    #[test]
    fn attack_button_allows_simultaneous_drag_aim_and_respects_suppression() {
        let mut touch = TouchState::default();
        let ctx = TouchContext {
            in_gameplay: true,
            alive: true,
            ..Default::default()
        };
        touch.set_context(ctx);
        let prefs = Preferences::default();
        let screen = vec2(1280., 720.);
        let layout = Layout::build(screen, &prefs, ctx);
        let atk = layout
            .buttons
            .iter()
            .find(|b| b.button == TouchButton::PrimaryAttack)
            .unwrap()
            .center;

        touch.update(
            &[TouchPoint {
                id: 7,
                phase: TouchPhase::Started,
                position: atk,
            }],
            screen,
            &prefs,
            true,
        );
        assert!(touch.action("Mouse 1", true));
        assert!(touch.action("Mouse 1", false));

        // Dragging the same thumb while holding attack aims the camera without dropping the button.
        touch.update(
            &[TouchPoint {
                id: 7,
                phase: TouchPhase::Moved,
                position: atk + vec2(-45., 20.),
            }],
            screen,
            &prefs,
            true,
        );
        assert!(!touch.action("Mouse 1", true));
        assert!(touch.action("Mouse 1", false));
        assert!(touch.look_radians(screen.y, &prefs).x < -0.05);

        // Suppressing on a modal boundary ignores finger 7 until lifted.
        touch.suppress();
        touch.update(
            &[TouchPoint {
                id: 7,
                phase: TouchPhase::Moved,
                position: atk,
            }],
            screen,
            &prefs,
            true,
        );
        assert!(!touch.action("Mouse 1", false));
        assert_eq!(touch.look_radians(screen.y, &prefs), Vec2::ZERO);
    }
}
