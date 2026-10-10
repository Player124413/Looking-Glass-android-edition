//! Multi-touch virtual joystick, relative touch-look trackpad and context-aware HUD controls.
use crate::{preferences::Preferences, ui::Ui};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
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
///
/// A global set of "draining" touch IDs filters out fingers that were already held
/// when a modal (menu/chapters/etc.) opened. Without this, a still-held finger on
/// the MENU/MAP/... button is reported as `pointer_pressed=true` on the modal's
/// first frames (Stationary phase, but `touches()` is read raw), which
/// hit-tests whatever button lies under it and instantly dismisses the modal.
///
/// Call [`drain_active_touches`] on entry to a modal (after the prior `suppress()`
/// + `next_frame().await`) to snapshot currently-live fingers; those fingers are
/// ignored for click purposes until they leave the screen.
///
/// Presses are detected by touch-ID edge AND include *fast taps*. macroquad keeps
/// only the LAST event per finger, so a tap that goes down AND up between two
/// frames reaches us as a single Ended/Cancelled point (its Started was
/// overwritten, and the entry is purged at the end of the frame). Such a point is
/// a complete click and must fire — dropping it is why buttons felt broken on
/// slow devices where frames are longer than a tap.
pub fn pointer_state() -> (Vec2, bool, bool) {
    let ts = touches();
    let mut pressed = false;
    let mut press_pos: Option<Vec2> = None;
    let mut live_pos: Option<Vec2> = None;
    let mut down = false;
    let mut now: BTreeSet<u64> = BTreeSet::new();
    {
        let prev = PREV_POINTER_IDS.with(|s| s.borrow().clone());
        for t in &ts {
            if is_draining(t.id) {
                continue;
            }
            now.insert(t.id);
            let live = !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled);
            if live {
                down = true;
                if live_pos.is_none() {
                    live_pos = Some(t.position);
                }
            }
            if !prev.contains(&t.id) {
                // New pointer id: a fresh finger (any phase — Android is free to
                // skip the Started) or a complete fast tap in one batch.
                pressed = true;
                if press_pos.is_none() {
                    press_pos = Some(t.position);
                }
            }
        }
    }
    PREV_POINTER_IDS.with(|s| *s.borrow_mut() = now);
    if let Some(pos) = press_pos.or(live_pos) {
        (pos, pressed, down)
    } else {
        (
            Vec2::from(mouse_position()),
            is_mouse_button_pressed(MouseButton::Left),
            is_mouse_button_down(MouseButton::Left),
        )
    }
}

thread_local! {
    static DRAINING_IDS: RefCell<BTreeSet<u64>> = RefCell::new(BTreeSet::new());
    static PREV_POINTER_IDS: RefCell<BTreeSet<u64>> = RefCell::new(BTreeSet::new());
}

fn is_draining(id: u64) -> bool {
    DRAINING_IDS.with(|s| s.borrow().contains(&id))
}

/// Mark every currently-held touch finger as "draining": it will be ignored by
/// [`pointer_state`] for click/press purposes until the finger lifts. Use this
/// when entering a modal menu/chapters/inventory so the finger that opened the
/// modal cannot accidentally hit a button and immediately close it.
pub fn drain_active_touches() {
    let ts: Vec<TouchPoint> = touches().into_iter().map(Into::into).collect();
    DRAINING_IDS.with(|s| {
        let mut s = s.borrow_mut();
        for t in &ts {
            if matches!(t.phase, TouchPhase::Started | TouchPhase::Moved | TouchPhase::Stationary) {
                s.insert(t.id);
            }
        }
        let live: BTreeSet<u64> = ts
            .iter()
            .filter(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled))
            .map(|t| t.id)
            .collect();
        s.retain(|id| live.contains(id));
    });
    // Also seed PREV_POINTER_IDS with every id currently present (ANY phase):
    // a finger already held — or a fast tap that just completed and only exists
    // as an Ended point — must not fire as a fresh press on the modal's first
    // frame.
    PREV_POINTER_IDS.with(|s| {
        *s.borrow_mut() = ts.iter().map(|t| t.id).collect();
    });
}

/// Clear all drain state. Called implicitly when the drained fingers lift,
/// or explicitly when returning to a state where leftover fingers should be
/// treated as fresh (e.g. entering gameplay from the launcher).
pub fn clear_drain() {
    DRAINING_IDS.with(|s| s.borrow_mut().clear());
    PREV_POINTER_IDS.with(|s| s.borrow_mut().clear());
}

/// Seed the per-frame touch-id tracker with the fingers currently on screen.
/// Used when entering a fresh screen (e.g. the launcher) so that already-held
/// fingers aren't falsely reported as a new "press" on the very first frame.
pub fn seed_pointer_state() {
    drain_active_touches();
    // drain_active_touches both drains AND seeds PREV_POINTER_IDS — but we
    // actually don't want to DRAIN those fingers, only seed the tracker.
    // So drop the draining set it populated.
    DRAINING_IDS.with(|s| s.borrow_mut().clear());
}

// Keep DRAINING_IDS trimmed using the per-frame `points` slice (caller already
// has it from `touches()`). Does NOT re-read `touches()` so unit tests that
// drive TouchState::update with synthetic events don't need a GL context.
pub fn tick_drain_from(points: &[TouchPoint]) {
    DRAINING_IDS.with(|s| {
        let live: BTreeSet<u64> = points
            .iter()
            .filter(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled))
            .map(|t| t.id)
            .collect();
        s.borrow_mut().retain(|id| live.contains(id));
    });
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

/// Per-button / per-joystick layout customization persisted in `Preferences::touch_layout`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TouchControlCustom {
    /// Horizontal offset as a fraction of screen width (-1.0 .. +1.0).
    pub offset_x: f32,
    /// Vertical offset as a fraction of screen height (-1.0 .. +1.0).
    pub offset_y: f32,
    /// Individual size multiplier for this control (0.4 .. 2.2).
    pub scale: f32,
    /// Individual opacity multiplier for this control (0.1 .. 1.0).
    pub opacity: f32,
    /// Whether this control is visible and active on the touch HUD.
    pub visible: bool,
}

impl Default for TouchControlCustom {
    fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            scale: 1.0,
            opacity: 1.0,
            visible: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchElement {
    Stick,
    Button(TouchButton),
}

impl TouchElement {
    pub const ALL: [Self; 18] = [
        Self::Stick,
        Self::Button(TouchButton::PrimaryAttack),
        Self::Button(TouchButton::AlternateAttack),
        Self::Button(TouchButton::Jump),
        Self::Button(TouchButton::Interact),
        Self::Button(TouchButton::Dive),
        Self::Button(TouchButton::RunLock),
        Self::Button(TouchButton::Hint),
        Self::Button(TouchButton::PrevToy),
        Self::Button(TouchButton::Inventory),
        Self::Button(TouchButton::NextToy),
        Self::Button(TouchButton::Menu),
        Self::Button(TouchButton::Chapters),
        Self::Button(TouchButton::ViewToggle),
        Self::Button(TouchButton::Help),
        Self::Button(TouchButton::Recover),
        Self::Button(TouchButton::QuickSave),
        Self::Button(TouchButton::QuickLoad),
    ];

    pub const fn config_key(self) -> &'static str {
        match self {
            Self::Stick => "stick",
            Self::Button(b) => b.config_key(),
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::Stick => "Movement Joystick (STICK)",
            Self::Button(TouchButton::PrimaryAttack) => "Primary Attack (ATK 1)",
            Self::Button(TouchButton::AlternateAttack) => "Alternate Attack (ATK 2)",
            Self::Button(TouchButton::Jump) => "Jump / Climb / Rise (JUMP)",
            Self::Button(TouchButton::Interact) => "Interact / Use (USE)",
            Self::Button(TouchButton::Dive) => "Crouch / Dive (CROUCH)",
            Self::Button(TouchButton::RunLock) => "Run / Sprint Lock (RUN)",
            Self::Button(TouchButton::Hint) => "Cheshire Cat Hint (CAT)",
            Self::Button(TouchButton::PrevToy) => "Previous Toy (< TOY)",
            Self::Button(TouchButton::Inventory) => "Toy Inventory (TOYS)",
            Self::Button(TouchButton::NextToy) => "Next Toy (TOY >)",
            Self::Button(TouchButton::Menu) => "Pause / Main Menu (MENU)",
            Self::Button(TouchButton::Chapters) => "Chapters / Map (MAP)",
            Self::Button(TouchButton::ViewToggle) => "Camera View Toggle (VIEW)",
            Self::Button(TouchButton::Help) => "Help Overlay (HELP)",
            Self::Button(TouchButton::Recover) => "Safe Footing (FOOT)",
            Self::Button(TouchButton::QuickSave) => "Quick Save (SAVE)",
            Self::Button(TouchButton::QuickLoad) => "Quick Load (LOAD)",
            Self::Button(TouchButton::SkipScene) => "Skip Cutscene (SKIP)",
        }
    }
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

    pub const fn config_key(self) -> &'static str {
        match self {
            Self::PrimaryAttack => "atk1",
            Self::AlternateAttack => "atk2",
            Self::Jump => "jump",
            Self::Interact => "use",
            Self::Dive => "dive",
            Self::RunLock => "run",
            Self::Hint => "hint",
            Self::PrevToy => "prev_toy",
            Self::NextToy => "next_toy",
            Self::Inventory => "toys",
            Self::ViewToggle => "view",
            Self::Help => "help",
            Self::Recover => "foot",
            Self::QuickSave => "save",
            Self::QuickLoad => "load",
            Self::SkipScene => "skip",
            Self::Menu => "menu",
            Self::Chapters => "map",
        }
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
    opacity: f32,
    visible: bool,
    modal: bool,
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
            opacity: 1.0,
            visible: true,
            modal: false,
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
            opacity: 1.0,
            visible: true,
            modal: false,
        }
    }

    fn pill_modal(button: TouchButton, rect: Rect, label: &'static str, highlight: bool) -> Self {
        let mut s = Self::pill(button, rect, label, highlight);
        s.modal = true;
        s
    }

    fn apply_custom(&mut self, prefs: &Preferences, w: f32, h: f32) {
        if self.modal {
            return;
        }
        let custom = prefs.touch_control(self.button.config_key());
        let scale = custom.scale.clamp(0.4, 2.2);
        self.opacity = custom.opacity.clamp(0.1, 1.0);
        self.visible = custom.visible;
        let shift = vec2(custom.offset_x * w, custom.offset_y * h);
        if let Some(r) = self.rect {
            let nw = r.w * scale;
            let nh = r.h * scale;
            let cx = (r.center().x + shift.x).clamp(nw * 0.5 + 4., (w - nw * 0.5 - 4.).max(nw * 0.5 + 4.));
            let cy = (r.center().y + shift.y).clamp(nh * 0.5 + 4., (h - nh * 0.5 - 4.).max(nh * 0.5 + 4.));
            self.center = vec2(cx, cy);
            self.radius = nw.max(nh) * 0.5;
            self.rect = Some(Rect::new(cx - nw * 0.5, cy - nh * 0.5, nw, nh));
        } else {
            let nr = self.radius * scale;
            let cx = (self.center.x + shift.x).clamp(nr + 4., (w - nr - 4.).max(nr + 4.));
            let cy = (self.center.y + shift.y).clamp(nr + 4., (h - nr - 4.).max(nr + 4.));
            self.center = vec2(cx, cy);
            self.radius = nr;
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
    stick_opacity: f32,
    stick_visible: bool,
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

        let stick_custom = prefs.touch_control("stick");
        let stick_radius = 68. * s * stick_custom.scale.clamp(0.4, 2.2);
        let raw_stick = vec2(mirror_x(132. * s), h - 136. * s)
            + vec2(stick_custom.offset_x * w, stick_custom.offset_y * h);
        let stick_default = vec2(
            raw_stick
                .x
                .clamp(stick_radius + 8., (w - stick_radius - 8.).max(stick_radius + 8.)),
            raw_stick
                .y
                .clamp(stick_radius + 8., (h - stick_radius - 8.).max(stick_radius + 8.)),
        );
        let stick_opacity = stick_custom.opacity.clamp(0.1, 1.0);
        let stick_visible = stick_custom.visible;
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
            buttons.push(ButtonRegion::pill_modal(
                TouchButton::SkipScene,
                Rect::new(w - 196. * s, 58. * s, 178. * s, 38. * s),
                "HOLD TO SKIP",
                true,
            ));
        }

        if !ctx.alive {
            for b in &mut buttons {
                b.apply_custom(prefs, w, h);
            }
            buttons.push(ButtonRegion::pill_modal(
                TouchButton::Recover,
                Rect::new((w - 220. * s) * 0.5, h * 0.34 + 156. * s, 220. * s, 44. * s),
                "TAP TO RETRY",
                true,
            ));
            return Self {
                scale: s,
                stick_default,
                stick_radius,
                stick_opacity,
                stick_visible,
                left_handed,
                screen: vec2(w, h),
                buttons,
            };
        }

        if ctx.inventory_open {
            for b in &mut buttons {
                b.apply_custom(prefs, w, h);
            }
            buttons.push(ButtonRegion::pill_modal(
                TouchButton::Inventory,
                Rect::new((w - 180. * s) * 0.5, h - 56. * s, 180. * s, 40. * s),
                "CLOSE TOYS",
                true,
            ));
            return Self {
                scale: s,
                stick_default,
                stick_radius,
                stick_opacity,
                stick_visible,
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

        for b in &mut buttons {
            b.apply_custom(prefs, w, h);
        }

        Self {
            scale: s,
            stick_default,
            stick_radius,
            stick_opacity,
            stick_visible,
            left_handed,
            screen: vec2(w, h),
            buttons,
        }
    }

    fn hit_button(&self, p: Vec2) -> Option<TouchButton> {
        self.buttons
            .iter()
            .rev()
            .find(|b| b.visible && b.contains(p))
            .map(|b| b.button)
    }

    fn hit_editor_element(&self, p: Vec2) -> Option<TouchElement> {
        if let Some(b) = self.buttons.iter().rev().find(|b| !b.modal && b.contains(p)) {
            return Some(TouchElement::Button(b.button));
        }
        if self.stick_default.distance_squared(p) <= (self.stick_radius * 1.08).powi(2) {
            return Some(TouchElement::Stick);
        }
        None
    }

    fn in_stick_zone(&self, p: Vec2) -> bool {
        if !self.stick_visible {
            return false;
        }
        if self.stick_default.distance_squared(p) <= (self.stick_radius * 1.65).powi(2) {
            return true;
        }
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
    tapped: bool,
    pub sprint_locked: bool,
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

    /// True when a fresh tap landed anywhere this frame — including *fast taps*
    /// that only ever appear as a single Ended point (see [`pointer_state`]).
    /// Computed in [`TouchState::update`] from the same batch the HUD uses.
    pub fn screen_tapped(&self) -> bool {
        self.tapped || is_mouse_button_pressed(MouseButton::Left)
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
    }

    fn assign_new_touch(
        &mut self,
        t: &TouchPoint,
        layout: &Layout,
        _screen: Vec2,
        can_press_buttons: bool,
    ) {
        // Try to claim this touch as a button / stick / look finger.
        // Order: button hit first, then stick zone, then look (camera drag).
        // When `can_press_buttons` is false (i.e. we are picking up an
        // orphaned Moved/Stationary event after a suppress/menu boundary),
        // skip button claims. A real button press always arrives as Started
        // (or a fast Ended tap) in the same batch — re-binding a lingering
        // held finger as a new button press would auto-close menus and
        // re-fire HUD actions the user never re-tapped.
        if can_press_buttons {
            if let Some(button) = layout.hit_button(t.position) {
                if button == TouchButton::RunLock {
                    // Run is a pure toggle: each press flips the lock. The finger does NOT
                    // hold it down — sprint only stays on while `sprint_locked` is true.
                    self.sprint_locked = !self.sprint_locked;
                }
                self.button_touches.insert(t.id, button);
                if button.allows_look_drag() && self.look_touch.is_none() {
                    self.look_touch = Some(t.id);
                    self.look_prev = t.position;
                }
                return;
            }
        }
        if self.stick_touch.is_none() && layout.in_stick_zone(t.position) {
            self.stick_touch = Some(t.id);
            // Fixed virtual stick: the knob's origin stays at the button's
            // configured position regardless of where exactly in the zone the
            // finger first lands. The finger only controls the offset from
            // that fixed center (with deadzone + clamping to stick_radius).
            self.stick_origin = layout.stick_default;
            self.stick_pos = self.stick_origin;
            self.move_axis = Vec2::ZERO;
            return;
        }
        if self.look_touch.is_none() {
            self.look_touch = Some(t.id);
            self.look_prev = t.position;
        }
    }

    pub fn update(
        &mut self,
        points: &[TouchPoint],
        screen: Vec2,
        prefs: &Preferences,
        focused: bool,
    ) {
        self.look_delta = Vec2::ZERO;
        self.tapped = false;
        // Trim global drain list each gameplay frame so lifted fingers stop being filtered.
        tick_drain_from(points);
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

        // First pass: identify rapid taps so their `pressed` edge always fires
        // even if finger-down and finger-up happen within the same event batch.
        let mut started_ids: BTreeSet<u64> = BTreeSet::new();
        let mut ended_ids: BTreeSet<u64> = BTreeSet::new();
        let mut orphan_end_ids: BTreeSet<u64> = BTreeSet::new();
        for t in points {
            if self.suppressed_ids.contains(&t.id) || !t.position.is_finite() {
                continue;
            }
            let known = self.stick_touch == Some(t.id)
                || self.look_touch == Some(t.id)
                || self.button_touches.contains_key(&t.id);
            match t.phase {
                TouchPhase::Started => {
                    started_ids.insert(t.id);
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    ended_ids.insert(t.id);
                    if !known {
                        orphan_end_ids.insert(t.id);
                    }
                }
                _ => {}
            }
        }
        // A fast tap whose Down and Up both arrive this frame, OR whose Up arrives
        // alone (OS coalesced events / dropped Started). Keep the entry alive for
        // one tick so the pressed edge fires, and release it next frame cleanly.
        let paired_taps: BTreeSet<u64> = &started_ids & &ended_ids;
        let same_frame_taps: BTreeSet<u64> = &paired_taps | &orphan_end_ids;
        self.tapped = !started_ids.is_empty() || !same_frame_taps.is_empty();

        for t in points {
            if self.suppressed_ids.contains(&t.id) || !t.position.is_finite() {
                continue;
            }
            let known = self.stick_touch == Some(t.id)
                || self.look_touch == Some(t.id)
                || self.button_touches.contains_key(&t.id);
            match t.phase {
                TouchPhase::Started => {
                    // A genuine new finger — can press any button/stick/look.
                    self.assign_new_touch(t, &layout, screen, true);
                }
                TouchPhase::Moved | TouchPhase::Stationary => {
                    if !known {
                        // Orphaned motion: Android/miniquad can skip the Started event
                        // (e.g. when multiple fingers land/change within the same frame,
                        // or during a brief app/input hiccup). Claim stick/look drags on
                        // the fly so camera/joystick stay responsive, but DO NOT bind
                        // this to a HUD button — a real button press always arrives as
                        // a Started (or fast Ended tap) event; otherwise a still-held
                        // finger crossing the suppress/menu boundary would re-fire the
                        // Menu button and instantly close the menu we just opened.
                        self.assign_new_touch(t, &layout, screen, false);
                    }
                    if self.stick_touch == Some(t.id) {
                        let offset = t.position - self.stick_origin;
                        let max_r = layout.stick_radius.max(1.);
                        let clamped = offset.clamp_length_max(max_r);
                        self.stick_pos = self.stick_origin + clamped;
                        let norm = vec2(offset.x / max_r, -offset.y / max_r);
                        let len = norm.length();
                        if len <= STICK_DEADZONE {
                            self.move_axis = Vec2::ZERO;
                        } else {
                            let scaled = ((len.min(1.) - STICK_DEADZONE)
                                / (1. - STICK_DEADZONE))
                            .clamp(0., 1.);
                            self.move_axis = norm / len * scaled;
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
                    if !known {
                        // A tap whose Started was never seen (dropped by OS/driver)
                        // — this can still be a valid quick tap on a button, so allow
                        // button claims here so the pressed edge fires before we
                        // release it below. Moved/Stationary orphans (above) stay
                        // non-button so lingering held fingers don't re-fire HUD taps.
                        self.assign_new_touch(t, &layout, screen, true);
                    }
                    if self.stick_touch == Some(t.id) {
                        self.stick_touch = None;
                        self.move_axis = Vec2::ZERO;
                    }
                    if self.look_touch == Some(t.id) {
                        self.look_touch = None;
                    }
                    if !same_frame_taps.contains(&t.id) {
                        self.button_touches.remove(&t.id);
                    }
                }
            }
        }

        if self.stick_touch.is_some_and(|id| !live_ids.contains(&id) && !same_frame_taps.contains(&id))
        {
            self.stick_touch = None;
            self.move_axis = Vec2::ZERO;
        }
        if self.look_touch.is_some_and(|id| !live_ids.contains(&id) && !same_frame_taps.contains(&id)) {
            self.look_touch = None;
        }
        self.button_touches
            .retain(|id, _| live_ids.contains(id) || same_frame_taps.contains(id));

        // Compute held-button mask. RunLock is NEVER held by finger pressure —
        // sprint only activates via the toggle (sprint_locked) while moving.
        let mut next_down = 0u32;
        for &button in self.button_touches.values() {
            if button != TouchButton::RunLock {
                next_down |= button.bit();
            }
        }
        if self.sprint_locked && self.move_axis != Vec2::ZERO {
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

        // Draw virtual movement stick when alive, visible, and inventory is closed.
        if layout.stick_visible
            && self.context.alive
            && !self.context.inventory_open
            && !self.context.paused
        {
            let sa = alpha * layout.stick_opacity;
            let stick_base = Color::new(0.11, 0.08, 0.13, 0.58 * sa);
            let stick_active = Color::new(0.58, 0.16, 0.15, 0.82 * sa);
            let stick_border = Color::new(0.78, 0.64, 0.44, 0.78 * sa);
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
            draw_circle(center.x, center.y, r, stick_base);
            draw_circle_lines(center.x, center.y, r, 2. * s, stick_border);
            draw_circle_lines(
                center.x,
                center.y,
                r * STICK_SPRINT_RING,
                1. * s,
                Color::new(0.78, 0.64, 0.44, 0.28 * sa),
            );
            let knob_r = (r * 0.38).max(16. * s);
            draw_circle(
                knob.x,
                knob.y,
                knob_r,
                if self.stick_touch.is_some() {
                    stick_active
                } else {
                    Color::new(0.26, 0.19, 0.24, 0.72 * sa)
                },
            );
            draw_circle_lines(knob.x, knob.y, knob_r, 2. * s, stick_border);
        }

        // Draw buttons.
        for b in &layout.buttons {
            if !b.visible {
                continue;
            }
            let ba = alpha * b.opacity;
            let held = self.down & b.button.bit() != 0
                || (b.button == TouchButton::RunLock && self.sprint_locked);
            let fill = if held {
                Color::new(0.58, 0.16, 0.15, 0.82 * ba)
            } else if b.highlight {
                Color::new(0.36, 0.24, 0.12, 0.72 * ba)
            } else {
                Color::new(0.11, 0.08, 0.13, 0.58 * ba)
            };
            let ring = if held || b.highlight {
                Color::new(0.96, 0.82, 0.46, 0.95 * ba)
            } else {
                Color::new(0.78, 0.64, 0.44, 0.78 * ba)
            };
            let btn_text = Color::new(0.95, 0.90, 0.80, 0.92 * ba);
            if let Some(rect) = b.rect {
                draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
                draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2. * s, ring);
                ui.center(
                    b.label,
                    Rect::new(rect.x + 4. * s, rect.y + 4. * s, rect.w - 8. * s, rect.h - 8. * s),
                    (16. * s).clamp(11., 28.),
                    btn_text,
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
                    btn_text,
                );
            }
        }
    }
}

/// Interactive visual drag-and-drop Touch Controls Studio used in the Android Launcher.
/// Allows moving buttons and the virtual joystick, resizing individual controls,
/// toggling per-control visibility and opacity, and disabling touch controls for gamepad play.
pub struct TouchEditor {
    pub selected: TouchElement,
    dragging: bool,
    drag_last: Vec2,
    pub status: String,
}

impl Default for TouchEditor {
    fn default() -> Self {
        Self {
            selected: TouchElement::Button(TouchButton::PrimaryAttack),
            dragging: false,
            drag_last: Vec2::ZERO,
            status: "Drag any button or joystick to move. Tap to select & adjust size/visibility."
                .into(),
        }
    }
}

impl TouchEditor {
    /// Draw and update the full-screen Touch Controls Studio.
    /// Returns `true` when the user taps `Done & Save` to return to the Launcher.
    pub fn update_and_draw(&mut self, prefs: &mut Preferences) -> bool {
        let (w, h) = (screen_width().max(320.), screen_height().max(240.));
        let screen = vec2(w, h);
        let s = (h / 540.).clamp(0.65, 2.4).min(w / 900.);
        clear_background(Color::from_hex(0x110e15));

        // Draw subtle alignment grid & center axes.
        let grid_col = Color::new(0.45, 0.36, 0.52, 0.14);
        let axis_col = Color::new(0.78, 0.62, 0.42, 0.26);
        let step = (64. * s).max(24.);
        let mut gx = step;
        while gx < w {
            draw_line(gx, 0., gx, h, 1., grid_col);
            gx += step;
        }
        let mut gy = step;
        while gy < h {
            draw_line(0., gy, w, gy, 1., grid_col);
            gy += step;
        }
        draw_line(w * 0.5, 0., w * 0.5, h, 1.5, axis_col);
        draw_line(0., h * 0.5, w, h * 0.5, 1.5, axis_col);

        let ctx = TouchContext {
            in_gameplay: true,
            alive: true,
            ..Default::default()
        };
        let layout = Layout::build(screen, prefs, ctx);

        // Inspector panel positioned in center-upper area so top bar, bottom bar, left stick,
        // and right action cluster remain completely unobstructed.
        let panel_w = (580. * s).min(w - 32.);
        let panel_h = (318. * s).min(h - 110.);
        let panel = Rect::new((w - panel_w) * 0.5, (h - panel_h) * 0.42, panel_w, panel_h);

        let (pointer_pos, pointer_pressed, pointer_down) = pointer_state();
        let on_panel = panel.contains(pointer_pos) && !self.dragging;

        // Handle direct canvas drag-and-drop when touching outside the inspector panel.
        if pointer_pressed && !on_panel {
            if let Some(elem) = layout.hit_editor_element(pointer_pos) {
                self.selected = elem;
                self.dragging = true;
                self.drag_last = pointer_pos;
                self.status = format!("Selected: {}", elem.title());
            }
        } else if pointer_down && self.dragging {
            let delta = pointer_pos - self.drag_last;
            self.drag_last = pointer_pos;
            if delta.length_squared() > 0.01 && delta.is_finite() {
                let key = self.selected.config_key();
                let custom = prefs.touch_control_mut(key);
                custom.offset_x = (custom.offset_x + delta.x / w).clamp(-0.92, 0.92);
                custom.offset_y = (custom.offset_y + delta.y / h).clamp(-0.92, 0.92);
            }
        } else if !pointer_down && self.dragging {
            self.dragging = false;
            let _ = prefs.save();
        }

        // Rebuild layout after any drag update so visuals are 100% immediate.
        let layout = Layout::build(screen, prefs, ctx);
        let global_alpha = prefs.touch_opacity.clamp(0.2, 1.0);

        // 1. Draw the Virtual Joystick on the editor canvas.
        {
            let is_sel = self.selected == TouchElement::Stick;
            let vis = layout.stick_visible && prefs.touch_mode != TouchMode::Off;
            let sa = if vis {
                global_alpha * layout.stick_opacity
            } else {
                0.28
            };
            let fill = if vis {
                Color::new(0.14, 0.10, 0.18, 0.68 * sa)
            } else {
                Color::new(0.28, 0.10, 0.12, 0.35)
            };
            let ring = if is_sel {
                Color::from_hex(0x5dade2)
            } else if vis {
                Color::new(0.78, 0.64, 0.44, 0.85 * sa)
            } else {
                Color::from_hex(0xc0392b)
            };
            let c = layout.stick_default;
            let r = layout.stick_radius;
            draw_circle(c.x, c.y, r, fill);
            draw_circle_lines(c.x, c.y, r, if is_sel { 3.5 * s } else { 2. * s }, ring);
            let knob_r = (r * 0.38).max(16. * s);
            draw_circle(c.x, c.y, knob_r, Color::new(0.32, 0.24, 0.34, 0.78 * sa));
            draw_circle_lines(c.x, c.y, knob_r, 2. * s, ring);
            let tag = if !layout.stick_visible {
                "STICK [HIDDEN]"
            } else {
                "STICK"
            };
            let fs = (16. * s).round();
            let tw = measure_text(tag, None, fs as u16, 1.0).width;
            draw_text(tag, c.x - tw * 0.5, c.y + 6. * s, fs, WHITE);
        }

        // 2. Draw all Touch Buttons on the editor canvas.
        for b in &layout.buttons {
            if b.modal {
                continue;
            }
            let is_sel = self.selected == TouchElement::Button(b.button);
            let vis = b.visible && prefs.touch_mode != TouchMode::Off;
            let ba = if vis { global_alpha * b.opacity } else { 0.30 };
            let fill = if vis {
                Color::new(0.14, 0.10, 0.18, 0.72 * ba)
            } else {
                Color::new(0.30, 0.10, 0.12, 0.36)
            };
            let ring = if is_sel {
                Color::from_hex(0x5dade2)
            } else if vis {
                Color::new(0.84, 0.68, 0.46, 0.88 * ba)
            } else {
                Color::from_hex(0xc0392b)
            };
            let label = if b.visible {
                b.label.to_string()
            } else {
                format!("{} [OFF]", b.label)
            };
            let fs = (15. * s).clamp(11., 24.).round();
            let tw = measure_text(&label, None, fs as u16, 1.0).width;
            if let Some(rect) = b.rect {
                draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
                draw_rectangle_lines(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    if is_sel { 3.2 * s } else { 2. * s },
                    ring,
                );
                draw_text(
                    &label,
                    rect.center().x - tw * 0.5,
                    rect.center().y + fs * 0.34,
                    fs,
                    WHITE,
                );
            } else {
                draw_circle(b.center.x, b.center.y, b.radius, fill);
                draw_circle_lines(
                    b.center.x,
                    b.center.y,
                    b.radius,
                    if is_sel { 3.5 * s } else { 2.2 * s },
                    ring,
                );
                draw_text(
                    &label,
                    b.center.x - tw * 0.5,
                    b.center.y + fs * 0.34,
                    fs,
                    WHITE,
                );
            }
        }

        // 3. Draw Floating Inspector Panel (dimmed while actively dragging a button).
        let panel_alpha = if self.dragging { 0.38 } else { 0.95 };
        draw_rectangle(
            panel.x,
            panel.y,
            panel.w,
            panel.h,
            Color::new(0.13, 0.10, 0.16, panel_alpha),
        );
        draw_rectangle_lines(
            panel.x,
            panel.y,
            panel.w,
            panel.h,
            2. * s,
            Color::new(0.79, 0.66, 0.49, panel_alpha),
        );

        let px = panel.x + 16. * s;
        let mut py = panel.y + 26. * s;
        draw_text(
            "Touch Controls Studio (Drag buttons on screen)",
            px,
            py,
            (20. * s).round(),
            Color::new(0.95, 0.90, 0.78, panel_alpha),
        );
        py += 10. * s;

        let click = (pointer_pressed && !self.dragging).then_some(pointer_pos);
        let draw_btn = |rect: Rect, text: &str, fill_hex: u32, active: bool| {
            let mut c = Color::from_hex(fill_hex);
            c.a = panel_alpha;
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, c);
            let mut border_c = Color::from_hex(if active { 0xf5cba7 } else { 0x8c6d46 });
            border_c.a = panel_alpha;
            draw_rectangle_lines(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                if active { 2.6 * s } else { 1.4 * s },
                border_c,
            );
            let mut fs = (16. * s).round();
            let mut tw = measure_text(text, None, fs as u16, 1.0).width;
            let max_w = (rect.w - 8. * s).max(8.);
            if tw > max_w {
                fs = (fs * max_w / tw).floor().max(9.);
                tw = measure_text(text, None, fs as u16, 1.0).width;
            }
            draw_text(
                text,
                rect.center().x - tw * 0.5,
                rect.center().y + fs * 0.34,
                fs,
                Color::new(1., 1., 1., panel_alpha),
            );
        };

        // Row 1: Touch Mode (Auto / Always On / OFF for Gamepad)
        let row_h = 34. * s;
        let mode_w = (panel.w - 32. * s - 16. * s) / 3.;
        for (i, (mode, label)) in [
            (TouchMode::Auto, "Touch: Auto"),
            (TouchMode::On, "Touch: Always On"),
            (TouchMode::Off, "Touch: OFF (Gamepad)"),
        ]
        .into_iter()
        .enumerate()
        {
            let r = Rect::new(px + i as f32 * (mode_w + 8. * s), py, mode_w, row_h);
            let active = prefs.touch_mode == mode;
            let fill = if active {
                if mode == TouchMode::Off {
                    0x7b241c
                } else {
                    0x276e36
                }
            } else {
                0x2b2233
            };
            draw_btn(r, label, fill, active);
            if click.is_some_and(|p| r.contains(p)) {
                prefs.touch_mode = mode;
                let _ = prefs.save();
                self.status = if mode == TouchMode::Off {
                    "Touch controls DISABLED (Gamepad mode active).".into()
                } else {
                    format!("Touch mode set to {}.", mode.name())
                };
            }
        }
        py += row_h + 10. * s;

        // Row 2: Selected Control Switcher (< Prev | Selected Name | Next >)
        let prev_btn = Rect::new(px, py, 76. * s, row_h);
        let next_btn = Rect::new(panel.right() - 16. * s - 76. * s, py, 76. * s, row_h);
        let name_box = Rect::new(
            prev_btn.right() + 8. * s,
            py,
            next_btn.x - prev_btn.right() - 16. * s,
            row_h,
        );
        draw_btn(prev_btn, "< Prev", 0x354552, false);
        draw_btn(next_btn, "Next >", 0x354552, false);
        draw_btn(name_box, self.selected.title(), 0x241c2c, true);
        if click.is_some_and(|p| prev_btn.contains(p)) {
            let idx = TouchElement::ALL
                .iter()
                .position(|&e| e == self.selected)
                .unwrap_or(0);
            self.selected = TouchElement::ALL
                [(idx + TouchElement::ALL.len() - 1) % TouchElement::ALL.len()];
        } else if click.is_some_and(|p| next_btn.contains(p)) {
            let idx = TouchElement::ALL
                .iter()
                .position(|&e| e == self.selected)
                .unwrap_or(0);
            self.selected = TouchElement::ALL[(idx + 1) % TouchElement::ALL.len()];
        }
        py += row_h + 10. * s;

        // Row 3: Selected Control Visibility, Size, and Opacity
        let sel_key = self.selected.config_key();
        let sel_cfg = prefs.touch_control(sel_key);
        let vis_btn = Rect::new(px, py, 148. * s, row_h);
        draw_btn(
            vis_btn,
            if sel_cfg.visible {
                "Visible: ON"
            } else {
                "Visible: HIDDEN"
            },
            if sel_cfg.visible { 0x276e36 } else { 0x7b241c },
            true,
        );
        if click.is_some_and(|p| vis_btn.contains(p)) {
            let c = prefs.touch_control_mut(sel_key);
            c.visible = !c.visible;
            let _ = prefs.save();
        }

        let size_minus = Rect::new(vis_btn.right() + 10. * s, py, 42. * s, row_h);
        let size_label = Rect::new(size_minus.right() + 4. * s, py, 102. * s, row_h);
        let size_plus = Rect::new(size_label.right() + 4. * s, py, 42. * s, row_h);
        draw_btn(size_minus, "-", 0x4a354f, false);
        draw_btn(
            size_label,
            &format!("Size {:.0}%", sel_cfg.scale * 100.),
            0x241c2c,
            false,
        );
        draw_btn(size_plus, "+", 0x4a354f, false);
        if click.is_some_and(|p| size_minus.contains(p)) {
            let c = prefs.touch_control_mut(sel_key);
            c.scale = (c.scale - 0.1).clamp(0.4, 2.2);
            let _ = prefs.save();
        } else if click.is_some_and(|p| size_plus.contains(p)) {
            let c = prefs.touch_control_mut(sel_key);
            c.scale = (c.scale + 0.1).clamp(0.4, 2.2);
            let _ = prefs.save();
        }

        let op_minus = Rect::new(size_plus.right() + 10. * s, py, 38. * s, row_h);
        let op_label = Rect::new(op_minus.right() + 4. * s, py, 96. * s, row_h);
        let op_plus = Rect::new(op_label.right() + 4. * s, py, 38. * s, row_h);
        draw_btn(op_minus, "-", 0x354552, false);
        draw_btn(
            op_label,
            &format!("Alpha {:.0}%", sel_cfg.opacity * 100.),
            0x241c2c,
            false,
        );
        draw_btn(op_plus, "+", 0x354552, false);
        if click.is_some_and(|p| op_minus.contains(p)) {
            let c = prefs.touch_control_mut(sel_key);
            c.opacity = (c.opacity - 0.1).clamp(0.1, 1.0);
            let _ = prefs.save();
        } else if click.is_some_and(|p| op_plus.contains(p)) {
            let c = prefs.touch_control_mut(sel_key);
            c.opacity = (c.opacity + 0.1).clamp(0.1, 1.0);
            let _ = prefs.save();
        }
        py += row_h + 10. * s;

        // Row 4: Global Scale, Global Opacity, Left-Handed, Show All
        let g_minus = Rect::new(px, py, 38. * s, row_h);
        let g_label = Rect::new(g_minus.right() + 4. * s, py, 114. * s, row_h);
        let g_plus = Rect::new(g_label.right() + 4. * s, py, 38. * s, row_h);
        draw_btn(g_minus, "-", 0x4a354f, false);
        draw_btn(
            g_label,
            &format!("All Size {:.0}%", prefs.touch_scale * 100.),
            0x241c2c,
            false,
        );
        draw_btn(g_plus, "+", 0x4a354f, false);
        if click.is_some_and(|p| g_minus.contains(p)) {
            prefs.touch_scale = (prefs.touch_scale - 0.1).clamp(0.6, 1.6);
            let _ = prefs.save();
        } else if click.is_some_and(|p| g_plus.contains(p)) {
            prefs.touch_scale = (prefs.touch_scale + 0.1).clamp(0.6, 1.6);
            let _ = prefs.save();
        }

        let lh_btn = Rect::new(g_plus.right() + 10. * s, py, 165. * s, row_h);
        draw_btn(
            lh_btn,
            if prefs.touch_left_handed {
                "Left-Handed: ON"
            } else {
                "Left-Handed: OFF"
            },
            if prefs.touch_left_handed {
                0x5c3d6e
            } else {
                0x2b2233
            },
            prefs.touch_left_handed,
        );
        if click.is_some_and(|p| lh_btn.contains(p)) {
            prefs.touch_left_handed = !prefs.touch_left_handed;
            let _ = prefs.save();
        }

        let show_all_btn = Rect::new(
            lh_btn.right() + 10. * s,
            py,
            panel.right() - 16. * s - (lh_btn.right() + 10. * s),
            row_h,
        );
        draw_btn(show_all_btn, "Show All", 0x354552, false);
        if click.is_some_and(|p| show_all_btn.contains(p)) {
            for c in prefs.touch_layout.values_mut() {
                c.visible = true;
            }
            let _ = prefs.save();
            self.status = "All touch buttons set to Visible.".into();
        }
        py += row_h + 12. * s;

        // Row 5: Reset Selected, Reset All, Save & Back
        let reset_one = Rect::new(px, py, 155. * s, 38. * s);
        let reset_all = Rect::new(reset_one.right() + 10. * s, py, 165. * s, 38. * s);
        let done_btn = Rect::new(
            reset_all.right() + 10. * s,
            py,
            panel.right() - 16. * s - (reset_all.right() + 10. * s),
            38. * s,
        );
        draw_btn(reset_one, "Reset Selected", 0x5b2c24, false);
        draw_btn(reset_all, "Reset All Default", 0x6e352c, false);
        draw_btn(done_btn, "Save & Done", 0x276e36, true);

        if click.is_some_and(|p| reset_one.contains(p)) {
            prefs.reset_touch_control(sel_key);
            let _ = prefs.save();
            self.status = format!("Reset {} to default.", self.selected.title());
        } else if click.is_some_and(|p| reset_all.contains(p)) {
            prefs.reset_all_touch_controls();
            prefs.touch_scale = 1.0;
            prefs.touch_opacity = 0.78;
            prefs.touch_left_handed = false;
            let _ = prefs.save();
            self.status = "All touch controls reset to defaults.".into();
        } else if click.is_some_and(|p| done_btn.contains(p))
            || is_key_pressed(KeyCode::Escape)
            || is_key_pressed(KeyCode::Back)
        {
            let _ = prefs.save();
            return true;
        }

        false
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
        // Sprint is no longer triggered by pushing the joystick past an outer ring —
        // it is exclusively controlled by the RUN toggle button.
        assert!(!touch.action("Shift", false));
        let look = touch.look_radians(screen.y, &prefs);
        assert!(look.x > 0.1 && look.y < -0.05);

        // Tapping RUN toggles sprint lock; sprint becomes active while moving.
        let run_pos = Layout::build(screen, &prefs, touch.context)
            .buttons
            .iter()
            .find(|b| b.button == TouchButton::RunLock)
            .unwrap()
            .center;
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
                TouchPoint {
                    id: 3,
                    phase: TouchPhase::Started,
                    position: run_pos,
                },
            ],
            screen,
            &prefs,
            true,
        );
        assert!(touch.action("Shift", false));

        // Release Finger 1; Finger 2 remains active. Sprint drops without movement.
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
                TouchPoint {
                    id: 3,
                    phase: TouchPhase::Ended,
                    position: run_pos,
                },
            ],
            screen,
            &prefs,
            true,
        );
        assert_eq!(touch.move_axis(), Vec2::ZERO);
        assert!(!touch.action("Shift", false));
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

    #[test]
    fn custom_button_position_size_visibility_and_gamepad_off_mode() {
        let mut touch = TouchState::default();
        let ctx = TouchContext {
            in_gameplay: true,
            alive: true,
            ..Default::default()
        };
        touch.set_context(ctx);
        let screen = vec2(1280., 720.);
        let mut prefs = Preferences::default();

        let default_layout = Layout::build(screen, &prefs, ctx);
        let default_jump = default_layout
            .buttons
            .iter()
            .find(|b| b.button == TouchButton::Jump)
            .unwrap()
            .center;

        // Move Jump left by 20% screen width, double its size, and hide Hint.
        {
            let j = prefs.touch_control_mut("jump");
            j.offset_x = -0.20;
            j.scale = 1.5;
            let h = prefs.touch_control_mut("hint");
            h.visible = false;
        }
        let custom_layout = Layout::build(screen, &prefs, ctx);
        let moved_jump = custom_layout
            .buttons
            .iter()
            .find(|b| b.button == TouchButton::Jump)
            .unwrap();
        assert!((moved_jump.center.x - (default_jump.x - 1280. * 0.20)).abs() < 1.0);
        assert!(moved_jump.visible);

        // Hidden Hint button does not register hits in gameplay, but remains selectable in the editor.
        let hint_pos = custom_layout
            .buttons
            .iter()
            .find(|b| b.button == TouchButton::Hint)
            .unwrap()
            .center;
        assert_eq!(custom_layout.hit_button(hint_pos), None);
        assert_eq!(
            custom_layout.hit_editor_element(hint_pos),
            Some(TouchElement::Button(TouchButton::Hint))
        );

        // Disabling touch controls (TouchMode::Off for gamepad play) ignores all touch inputs.
        prefs.touch_mode = TouchMode::Off;
        touch.update(
            &[TouchPoint {
                id: 1,
                phase: TouchPhase::Started,
                position: moved_jump.center,
            }],
            screen,
            &prefs,
            true,
        );
        assert!(!touch.action("Space", true));
    }
}
