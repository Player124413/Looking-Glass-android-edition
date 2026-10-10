//! Original archive menu art/layouts, with a small, explicit Rust action router.
//! URC commands are data only: no original console/script commands are executed.
use crate::{
    assets::Assets,
    audio, bsp,
    powerups::Difficulty,
    preferences::{Preferences, BINDINGS, SIZES},
    save::{Slot, Store},
    texture,
    ui::{Canvas, Ui},
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::collections::BTreeMap;
mod save_screen;
mod chapters;
pub use chapters::ChapterChoice;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Main,
    Settings,
    Video,
    Audio,
    Game,
    Controls,
    LoadSave,
    NewGame,
    Quit,
    Credits,
    ConfirmNew(Difficulty),
    ConfirmLoad(Slot),
    ConfirmSave(Slot),
}
impl Page {
    fn parent(self) -> Option<Self> {
        match self {
            Self::Main => None,
            Self::Video | Self::Audio | Self::Game | Self::Controls => Some(Self::Settings),
            Self::Credits => Some(Self::Quit),
            Self::ConfirmNew(_) => Some(Self::NewGame),
            Self::ConfirmLoad(_) | Self::ConfirmSave(_) => Some(Self::LoadSave),
            _ => Some(Self::Main),
        }
    }
    fn layout(self) -> &'static str {
        match self {
            Self::Settings | Self::Video | Self::Audio | Self::Game | Self::Controls => "controls",
            Self::LoadSave | Self::ConfirmLoad(_) | Self::ConfirmSave(_) => "loadsave",
            Self::NewGame | Self::ConfirmNew(_) => "newgame",
            Self::Quit => "quit",
            Self::Credits => "credits",
            _ => "main",
        }
    }
    fn settings(self) -> bool {
        matches!(
            self,
            Self::Video | Self::Audio | Self::Game | Self::Controls
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Resume,
    Chapters,
    Save(Slot),
    Load(Slot),
    NewGame(Difficulty),
    Quit,
}
#[derive(Clone, Copy)]
enum Click {
    Page(Page),
    Back,
    Action(Action),
    Row(usize),
    Apply,
    Cancel,
    Reset,
    CreditScroll(i32),
    ControlsPage(i32),
    Device,
    Slot(usize),
    SaveSelection,
    LoadSelection,
}
struct Button {
    rect: Rect,
    text: String,
    art: String,
    hover: String,
    click: Click,
}
impl Button {
    fn text(rect: Rect, text: impl Into<String>, click: Click) -> Self {
        Self {
            rect,
            text: text.into(),
            art: "ui/buttons/apply".into(),
            hover: "ui/buttons/apply_hover".into(),
            click,
        }
    }
}

#[derive(Clone, Debug)]
struct Widget {
    kind: String,
    fields: BTreeMap<String, Vec<String>>,
    rect: Rect,
}
impl Widget {
    fn value(&self, name: &str) -> &str {
        self.fields
            .get(name)
            .and_then(|v| v.first())
            .map_or("", String::as_str)
    }
    fn art(&self) -> &str {
        self.fields
            .get("staticshader")
            .and_then(|v| v.get(1))
            .map_or_else(|| self.value("shader"), String::as_str)
    }
    fn button(&self, click: Click) -> Button {
        Button {
            rect: self.rect,
            text: self.value("title").into(),
            art: self.art().into(),
            hover: self.value("hovershader").into(),
            click,
        }
    }
}
fn layout(source: &str) -> Result<Vec<Widget>> {
    ensure!(source.len() < 256 * 1024, "Menu definition too large");
    let mut widgets = Vec::new();
    let mut current: Option<Widget> = None;
    let mut resource = false;
    for line in source.lines() {
        let tokens = bsp::tokens(line)?;
        let Some(key) = tokens.first() else {
            continue;
        };
        if key == "resource" {
            resource = true;
            continue;
        }
        if resource {
            current = Some(Widget {
                kind: key.clone(),
                fields: BTreeMap::new(),
                rect: Rect::default(),
            });
            resource = false;
        } else if key == "}" {
            if let Some(mut w) = current.take() {
                let r = w
                    .fields
                    .get("rect")
                    .context("Menu widget has no rectangle")?;
                ensure!(r.len() == 4, "Invalid menu rectangle");
                let v = r
                    .iter()
                    .map(|n| n.parse::<f32>())
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                ensure!(
                    v.iter().all(|n| n.is_finite() && n.abs() <= 4096.) && v[2] >= 0. && v[3] >= 0.,
                    "Invalid menu bounds"
                );
                w.rect = Rect::new(v[0], v[1], v[2], v[3]);
                widgets.push(w);
            }
        } else if key != "{" {
            if let Some(w) = &mut current {
                w.fields
                    .insert(key.to_ascii_lowercase(), tokens[1..].to_vec());
            }
        }
    }
    ensure!(
        current.is_none() && !resource && !widgets.is_empty(),
        "Incomplete menu layout"
    );
    Ok(widgets)
}

fn load_texture(assets: &mut Assets, path: &str, additive: bool) -> Result<Texture2D> {
    let mut im = texture::decode(assets, path)?;
    // Black-backed additive menu highlights can use the stock alpha pipeline.
    if additive {
        for p in im.pixels.chunks_exact_mut(4) {
            let a = p[0].max(p[1]).max(p[2]);
            p[3] = ((p[3] as u16 * a as u16) / 255) as u8;
            if a > 0 {
                for c in &mut p[..3] {
                    *c = ((*c as u16 * 255) / a as u16) as u8;
                }
            }
        }
    }
    let tex = Texture2D::from_rgba8(im.width, im.height, &im.pixels);
    tex.set_filter(FilterMode::Linear);
    Ok(tex)
}

pub struct Menu {
    pub page: Option<Page>,
    pub message: String,
    pub post_game: bool,
    pub frontend: bool,
    layouts: BTreeMap<String, Vec<Widget>>,
    textures: BTreeMap<String, Texture2D>,
    ui: std::rc::Rc<Ui>,
    selected: usize,
    previous_pointer: Vec2,
    draft: Preferences,
    draft_audio: audio::Settings,
    binding: Option<usize>,
    save_labels: [String; 6],
    save_dates: [String; 6],
    save_usable: [bool; 6],
    save_previews: [Option<Texture2D>; 6],
    save_selected: usize,
    save_film: save_screen::Film,
    controls_pad: bool,
    controls_touch: bool,
    controls_page: usize,
    using_pad: bool,
    using_touch: bool,
    credits: Vec<String>,
    credit_offset: usize,
    mirror: crate::npc::Puppet,
    mirror_target: RenderTarget,
    material: crate::character::SkinMaterial,
}
impl Menu {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let specs = texture::read_materials(assets)?;
        let mut layouts = BTreeMap::new();
        let mut credits = Vec::new();
        for name in ["main", "controls", "loadsave", "newgame", "quit", "credits"] {
            let data = assets.read(&format!("ui/{name}.urc"))?;
            let source = String::from_utf8_lossy(&data);
            if name == "credits" {
                for line in source.lines() {
                    let t = bsp::tokens(line)?;
                    if t.first().is_some_and(|s| s == "addline") {
                        credits.push(t.get(1).cloned().unwrap_or_default());
                    }
                }
            }
            layouts.insert(name.into(), layout(&source)?);
        }
        // The original scroller starts off-page; paging should begin at its text.
        let first = credits
            .iter()
            .position(|line| !line.trim().is_empty())
            .unwrap_or(credits.len());
        credits.drain(..first);
        for name in ["newgame", "loadgame", "settings", "quit", "backtogame"] {
            ensure!(
                layouts["main"]
                    .iter()
                    .any(|w| w.kind == "Button" && w.value("name") == name),
                "Missing original menu button: {name}"
            );
        }
        for name in [
            "BigSaveShot",
            "bigcover_left",
            "bigcover_right",
            "MapName",
            "DateTime",
            "PageNum",
            "PrevButton",
            "NextButton",
            "DeleteButton",
            "LoadingButton",
            "SaveButton",
        ]
        .into_iter()
        .map(str::to_owned)
        .chain((1..=6).flat_map(|i| [format!("LoadSaveShot{i}"), format!("LoadButton{i}")]))
        {
            ensure!(
                layouts["loadsave"].iter().any(|w| w.value("name") == name),
                "Missing original save widget {name}"
            );
        }
        let mut names = vec![
            "ui/control/head_2_video",
            "ui/control/head_2_audio",
            "ui/control/head_2_control",
            "ui/control/head_2_game",
            "ui/buttons/apply",
            "ui/buttons/apply_hover",
            "ui/control/hslider_bar",
            "ui/control/hslider_indicator",
            "ui/control/hslider_prev",
            "ui/control/hslider_next",
            "ui/control/hslider_prev_pressed",
            "ui/control/hslider_next_pressed",
            "ui/control/checkbox_checked",
            "ui/control/checkbox_unchecked",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        for w in layouts.values().flatten() {
            names.extend(
                [w.art(), w.value("hovershader")]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned),
            );
        }
        let mut textures = BTreeMap::new();
        for name in names {
            let key = name.to_ascii_lowercase();
            if textures.contains_key(&key) {
                continue;
            }
            let spec = specs.get(&key);
            let candidate = spec
                .and_then(|s| s.candidates.iter().find(|p| assets.contains(p)))
                .cloned()
                .unwrap_or_else(|| {
                    if assets.contains(&key) {
                        key.clone()
                    } else {
                        format!("{key}.tga")
                    }
                });
            if assets.contains(&candidate) {
                textures.insert(
                    key,
                    load_texture(
                        assets,
                        &candidate,
                        name.ends_with("_glow") && name.contains("/button_")
                            || name.ends_with("game_glow")
                            || name.ends_with("settings_glow")
                            || name.ends_with("quit_glow"),
                    )?,
                );
            }
        }
        let mut mirror =
            crate::npc::Puppet::load(assets, "alice", &["idle_stand_rocktoes"], &specs)?;
        mirror.show_attachments(false);
        Ok(Self {
            page: None,
            message: String::new(),
            post_game: false,
            frontend: false,
            layouts,
            textures,
            ui: Ui::load(assets)?,
            selected: 0,
            previous_pointer: Vec2::splat(-10000.),
            draft: Preferences::default(),
            draft_audio: audio::Settings::default(),
            binding: None,
            save_labels: Default::default(),
            save_dates: Default::default(),
            save_usable: [false; 6],
            save_previews: Default::default(),
            save_selected: 0,
            save_film: save_screen::Film::load(assets)?,
            controls_pad: false,
            controls_touch: crate::android::is_android(),
            controls_page: 0,
            using_pad: false,
            using_touch: crate::android::is_android(),
            credits,
            credit_offset: 0,
            mirror,
            mirror_target: render_target_ex(
                384,
                480,
                RenderTargetParams {
                    depth: true,
                    ..Default::default()
                },
            ),
            material: crate::character::skin_material()?,
        })
    }
    pub fn open(&mut self, page: Page, preferences: &Preferences, audio: audio::Settings) {
        self.page = Some(page);
        self.selected = 0;
        self.binding = None;
        self.draft = preferences.clone();
        self.draft_audio = audio;
        self.previous_pointer = Vec2::from(mouse_position());
        self.message.clear();
        if page == Page::LoadSave {
            self.selected = self.save_selected;
            self.save_film.open(self.save_selected, get_time());
        }
    }
    pub fn close(&mut self) {
        self.page = None;
        self.save_film.stop();
    }
    fn art(&self, canvas: Canvas, name: &str, rect: Rect, alpha: f32) {
        if let Some(t) = self.textures.get(&name.to_ascii_lowercase()) {
            let r = canvas.rect(rect);
            draw_texture_ex(
                t,
                r.x,
                r.y,
                Color::new(1., 1., 1., alpha),
                DrawTextureParams {
                    dest_size: Some(r.size()),
                    ..Default::default()
                },
            );
        }
    }
    fn button_named(&self, layout: &str, name: &str, click: Click) -> Button {
        self.layouts[layout]
            .iter()
            .find(|w| w.kind == "Button" && w.value("name") == name)
            .expect("validated original menu button")
            .button(click)
    }
    fn buttons(&self, page: Page) -> Vec<Button> {
        let back = || Button {
            rect: Rect::new(0., 456., 80., 24.),
            text: String::new(),
            art: "ui/back_button".into(),
            hover: "ui/back_button_hover".into(),
            click: Click::Back,
        };
        match page {
            Page::Main => {
                let mut buttons = vec![
                    self.button_named("main", "newgame", Click::Page(Page::NewGame)),
                    self.button_named("main", "loadgame", Click::Page(Page::LoadSave)),
                    self.button_named("main", "settings", Click::Page(Page::Settings)),
                    self.button_named("main", "quit", Click::Page(Page::Quit)),
                    self.button_named("main", "backtogame", Click::Action(Action::Resume)),
                ];
                if self.post_game || self.frontend {
                    buttons.pop();
                }
                if self.using_touch || crate::android::is_android() {
                    buttons.push(Button::text(
                        Rect::new(470., 440., 150., 32.),
                        "Chapters",
                        Click::Action(Action::Chapters),
                    ));
                }
                buttons
            }
            Page::Settings => {
                let mut buttons = vec![back()];
                let pages = [Page::Video, Page::Audio, Page::Controls, Page::Game];
                for (w, page) in self.layouts["controls"]
                    .iter()
                    .filter(|w| {
                        w.kind == "Button"
                            && w.value("groupid") == "group_main"
                            && w.value("stuffcommand").contains("activategroup")
                    })
                    .zip(pages)
                {
                    buttons.push(w.button(Click::Page(page)));
                }
                buttons
            }
            Page::NewGame => {
                let mut buttons = self.layouts["newgame"]
                    .iter()
                    .filter(|w| {
                        w.kind == "Button" && w.value("stuffcommand").contains("seta skill")
                    })
                    .zip(Difficulty::ALL)
                    .map(|(w, d)| w.button(if self.frontend {
                        Click::Action(Action::NewGame(d))
                    } else {
                        Click::Page(Page::ConfirmNew(d))
                    }))
                    .collect::<Vec<_>>();
                buttons.push(back());
                buttons
            }
            Page::Quit => self.layouts["quit"]
                .iter()
                .filter(|w| w.kind == "Button")
                .zip([
                    Click::Action(Action::Quit),
                    Click::Back,
                    Click::Page(Page::Credits),
                    Click::Back,
                ])
                .map(|(w, c)| w.button(c))
                .collect(),
            Page::ConfirmNew(_) | Page::ConfirmLoad(_) | Page::ConfirmSave(_) => {
                let action = match page {
                    Page::ConfirmNew(d) => Action::NewGame(d),
                    Page::ConfirmLoad(s) => Action::Load(s),
                    Page::ConfirmSave(s) => Action::Save(s),
                    _ => unreachable!(),
                };
                vec![
                    Button::text(
                        Rect::new(205., 264., 100., 35.),
                        "Yes",
                        Click::Action(action),
                    ),
                    Button::text(Rect::new(335., 264., 100., 35.), "No", Click::Back),
                ]
            }
            Page::LoadSave => {
                let mut buttons = (0..6)
                    .map(|i| Button {
                        rect: self.slot_rect(i),
                        text: String::new(),
                        art: String::new(),
                        hover: String::new(),
                        click: Click::Slot(i),
                    })
                    .collect::<Vec<_>>();
                buttons.push(self.button_named("loadsave", "LoadingButton", Click::LoadSelection));
                if !self.frontend {
                    buttons.push(self.button_named("loadsave", "SaveButton", Click::SaveSelection));
                }
                buttons.push(back());
                buttons
            }
            Page::Controls => {
                let mut buttons = vec![Button {
                    rect: Rect::new(65., 62., 226., 28.),
                    text: if self.controls_touch {
                        "Touch Controls"
                    } else if self.controls_pad {
                        "Controller"
                    } else {
                        "Keyboard / Mouse"
                    }
                    .into(),
                    art: String::new(),
                    hover: String::new(),
                    click: Click::Device,
                }];
                if self.controls_touch {
                    let touch_base = BINDINGS.len() + 10;
                    for (i, (label, y, h)) in [
                        (format!("Overlay: {}", self.draft.touch_mode.name()), 96., 26.),
                        ("Touch Sensitivity".into(), 124., 42.),
                        ("Invert Touch Look".into(), 174., 26.),
                        ("Button Size".into(), 202., 42.),
                        ("HUD Opacity".into(), 252., 42.),
                        ("Left-Handed Layout".into(), 302., 26.),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        buttons.push(Button {
                            rect: Rect::new(65., y, 226., h),
                            text: label,
                            art: String::new(),
                            hover: String::new(),
                            click: Click::Row(touch_base + i),
                        });
                    }
                } else {
                    let start = self.controls_page * 8;
                    for (row, (label, _)) in BINDINGS.iter().enumerate().skip(start).take(8) {
                        buttons.push(Button {
                            rect: Rect::new(65., 96. + (row - start) as f32 * 26., 226., 26.),
                            text: (*label).into(),
                            art: String::new(),
                            hover: String::new(),
                            click: Click::Row(row),
                        });
                    }
                    if self.controls_pad && self.controls_page == BINDINGS.len().div_ceil(8) {
                        for (i, label) in ["Look Sensitivity", "Invert Look", "Stick Deadzone"]
                            .into_iter()
                            .enumerate()
                        {
                            buttons.push(Button {
                                rect: Rect::new(65., 110. + i as f32 * 66., 226., 44.),
                                text: label.into(),
                                art: String::new(),
                                hover: String::new(),
                                click: Click::Row(BINDINGS.len() + i),
                            });
                        }
                    }
                    for (x, label, delta) in [(50., "Previous", -1), (206., "Next", 1)] {
                        buttons.push(Button::text(
                            Rect::new(x, 311., 100., 32.),
                            label,
                            Click::ControlsPage(delta),
                        ));
                    }
                }
                buttons.push(Button::text(
                    Rect::new(125., 352., 100., 32.),
                    "Reset",
                    Click::Reset,
                ));
                buttons.push(Button::text(
                    Rect::new(50., 400., 100., 35.),
                    "Apply",
                    Click::Apply,
                ));
                buttons.push(Button::text(
                    Rect::new(200., 400., 100., 35.),
                    "Cancel",
                    Click::Cancel,
                ));
                buttons
            }
            Page::Credits => vec![
                back(),
                Button::text(
                    Rect::new(320., 410., 100., 35.),
                    "Previous",
                    Click::CreditScroll(-1),
                ),
                Button::text(
                    Rect::new(430., 410., 100., 35.),
                    "Next",
                    Click::CreditScroll(1),
                ),
            ],
            _ => {
                let rows: Vec<(String, f32)> = match page {
                    Page::Video => vec![
                        (
                            format!(
                                "{} x {}",
                                SIZES[self.draft.resolution].0, SIZES[self.draft.resolution].1
                            ),
                            104.,
                        ),
                        (
                            if self.draft.fullscreen {
                                "Fullscreen"
                            } else {
                                "Windowed"
                            }
                            .into(),
                            184.,
                        ),
                        (
                            format!("Preset: {}", self.draft.performance_preset.name()),
                            252.,
                        ),
                        (
                            format!("FPS Limit: {}", self.draft.fps_limit.name()),
                            304.,
                        ),
                    ],
                    Page::Audio => vec![
                        ("Music Volume".into(), 168.),
                        ("Effects Volume".into(), 208.),
                        ("Mute".into(), 298.),
                    ],
                    Page::Game => vec![
                        ("Sensitivity".into(), 80.),
                        ("Invert Mouse".into(), 130.),
                        ("Camera Distance".into(), 168.),
                        ("Always Run".into(), 222.),
                        ("Subtitles".into(), 270.),
                    ],
                    _ => Vec::new(),
                };
                let mut buttons = rows
                    .into_iter()
                    .enumerate()
                    .map(|(i, (text, y))| Button {
                        rect: Rect::new(
                            65.,
                            y,
                            226.,
                            if self.slider(page, i).is_some() {
                                40.
                            } else {
                                26.
                            },
                        ),
                        text,
                        art: String::new(),
                        hover: String::new(),
                        click: Click::Row(i),
                    })
                    .collect::<Vec<_>>();
                buttons.push(Button::text(
                    Rect::new(50., 400., 100., 35.),
                    "Apply",
                    Click::Apply,
                ));
                buttons.push(Button::text(
                    Rect::new(200., 400., 100., 35.),
                    "Cancel",
                    Click::Cancel,
                ));
                if page == Page::Controls {
                    buttons.push(Button::text(
                        Rect::new(125., 344., 100., 35.),
                        "Reset",
                        Click::Reset,
                    ));
                }
                buttons
            }
        }
    }
    fn adjust(&mut self, page: Page, row: usize, delta: f32) {
        match (page, row) {
            (Page::Video, 0) => {
                self.draft.resolution = (self.draft.resolution as i32
                    + if delta < 0. { -1 } else { 1 })
                .rem_euclid(SIZES.len() as i32) as usize
            }
            (Page::Video, 1) => self.draft.fullscreen = !self.draft.fullscreen,
            (Page::Video, 2) => {
                self.draft.performance_preset = self.draft.performance_preset.next(delta)
            }
            (Page::Video, 3) => {
                self.draft.fps_limit = self.draft.fps_limit.next(delta)
            }
            (Page::Audio, 0) => {
                self.draft_audio.music = (self.draft_audio.music + delta * 0.05).clamp(0., 1.)
            }
            (Page::Audio, 1) => {
                self.draft_audio.effects = (self.draft_audio.effects + delta * 0.05).clamp(0., 1.)
            }
            (Page::Audio, 2) => self.draft_audio.muted = !self.draft_audio.muted,
            (Page::Game, 0) => {
                self.draft.sensitivity = (self.draft.sensitivity + delta * 0.1).clamp(0.2, 3.)
            }
            (Page::Game, 1) => self.draft.invert_mouse = !self.draft.invert_mouse,
            (Page::Game, 2) => {
                self.draft.camera_distance =
                    (self.draft.camera_distance + delta * 10.).clamp(80., 220.)
            }
            (Page::Game, 3) => self.draft.always_run = !self.draft.always_run,
            (Page::Game, 4) => self.draft.subtitles = !self.draft.subtitles,
            (Page::Controls, row) if row < BINDINGS.len() => self.binding = Some(row),
            (Page::Controls, row) if row == BINDINGS.len() => {
                self.draft.pad_sensitivity =
                    (self.draft.pad_sensitivity + delta * 0.1).clamp(0.2, 3.)
            }
            (Page::Controls, row) if row == BINDINGS.len() + 1 => {
                self.draft.invert_pad = !self.draft.invert_pad
            }
            (Page::Controls, row) if row == BINDINGS.len() + 2 => {
                self.draft.pad_deadzone = (self.draft.pad_deadzone + delta * 0.02).clamp(0.1, 0.4)
            }
            (Page::Controls, row) if row == BINDINGS.len() + 10 => {
                self.draft.touch_mode = self.draft.touch_mode.next(delta)
            }
            (Page::Controls, row) if row == BINDINGS.len() + 11 => {
                self.draft.touch_sensitivity =
                    (self.draft.touch_sensitivity + delta * 0.1).clamp(0.2, 3.)
            }
            (Page::Controls, row) if row == BINDINGS.len() + 12 => {
                self.draft.invert_touch = !self.draft.invert_touch
            }
            (Page::Controls, row) if row == BINDINGS.len() + 13 => {
                self.draft.touch_scale = (self.draft.touch_scale + delta * 0.05).clamp(0.6, 1.6)
            }
            (Page::Controls, row) if row == BINDINGS.len() + 14 => {
                self.draft.touch_opacity = (self.draft.touch_opacity + delta * 0.05).clamp(0.2, 1.0)
            }
            (Page::Controls, row) if row == BINDINGS.len() + 15 => {
                self.draft.touch_left_handed = !self.draft.touch_left_handed
            }
            _ => (),
        }
    }
    fn toggle(&self, page: Page, row: usize) -> Option<bool> {
        match (page, row) {
            (Page::Audio, 2) => Some(self.draft_audio.muted),
            (Page::Game, 1) => Some(self.draft.invert_mouse),
            (Page::Game, 3) => Some(self.draft.always_run),
            (Page::Game, 4) => Some(self.draft.subtitles),
            (Page::Controls, row) if row == BINDINGS.len() + 1 => Some(self.draft.invert_pad),
            (Page::Controls, row) if row == BINDINGS.len() + 12 => Some(self.draft.invert_touch),
            (Page::Controls, row) if row == BINDINGS.len() + 15 => {
                Some(self.draft.touch_left_handed)
            }
            _ => None,
        }
    }
    /// Original main artwork behind loading and chapter selection.
    pub fn backdrop(&mut self) {
        self.draw(Page::Main, &[]);
    }
    fn slider(&self, page: Page, row: usize) -> Option<f32> {
        match (page, row) {
            (Page::Audio, 0) => Some(self.draft_audio.music),
            (Page::Audio, 1) => Some(self.draft_audio.effects),
            (Page::Game, 0) => Some((self.draft.sensitivity - 0.2) / 2.8),
            (Page::Game, 2) => Some(((self.draft.camera_distance - 80.) / 140.).clamp(0., 1.)),
            (Page::Controls, row) if row == BINDINGS.len() => {
                Some((self.draft.pad_sensitivity - 0.2) / 2.8)
            }
            (Page::Controls, row) if row == BINDINGS.len() + 2 => {
                Some((self.draft.pad_deadzone - 0.1) / 0.3)
            }
            (Page::Controls, row) if row == BINDINGS.len() + 11 => {
                Some(((self.draft.touch_sensitivity - 0.2) / 2.8).clamp(0., 1.))
            }
            (Page::Controls, row) if row == BINDINGS.len() + 13 => {
                Some(((self.draft.touch_scale - 0.6) / 1.0).clamp(0., 1.))
            }
            (Page::Controls, row) if row == BINDINGS.len() + 14 => {
                Some(((self.draft.touch_opacity - 0.2) / 0.8).clamp(0., 1.))
            }
            _ => None,
        }
    }
    fn drag_slider(&mut self, page: Page, row: usize, value: f32) {
        let v = value.clamp(0., 1.);
        match (page, row) {
            (Page::Audio, 0) => self.draft_audio.music = v,
            (Page::Audio, 1) => self.draft_audio.effects = v,
            (Page::Game, 0) => self.draft.sensitivity = 0.2 + v * 2.8,
            (Page::Game, 2) => self.draft.camera_distance = 80. + v * 140.,
            (Page::Controls, row) if row == BINDINGS.len() => {
                self.draft.pad_sensitivity = 0.2 + v * 2.8
            }
            (Page::Controls, row) if row == BINDINGS.len() + 2 => {
                self.draft.pad_deadzone = 0.1 + v * 0.3
            }
            (Page::Controls, row) if row == BINDINGS.len() + 11 => {
                self.draft.touch_sensitivity = 0.2 + v * 2.8
            }
            (Page::Controls, row) if row == BINDINGS.len() + 13 => {
                self.draft.touch_scale = 0.6 + v * 1.0
            }
            (Page::Controls, row) if row == BINDINGS.len() + 14 => {
                self.draft.touch_opacity = 0.2 + v * 0.8
            }
            _ => (),
        }
    }
    fn save_preview(&self, canvas: Canvas, index: usize, rect: Rect) {
        let r = canvas.rect(rect);
        if let Some(texture) = &self.save_previews[index] {
            // Crop to the frame without stretching a widescreen game view.
            let size = vec2(texture.width(), texture.height());
            let scale = (r.w / size.x).max(r.h / size.y);
            let source_size = r.size() / scale;
            let source = (size - source_size) * 0.5;
            draw_texture_ex(
                texture,
                r.x,
                r.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(r.size()),
                    source: Some(Rect::new(source.x, source.y, source_size.x, source_size.y)),
                    ..Default::default()
                },
            );
        } else {
            draw_rectangle(r.x, r.y, r.w, r.h, BLACK);
            self.ui.font.draw(
                canvas,
                if self.save_usable[index] {
                    "No preview"
                } else {
                    &self.save_labels[index]
                },
                rect,
                17.,
                crate::ui::PAPER_TEXT,
            );
        }
    }
    fn refresh_saves(&mut self, store: &Store) {
        for (index, slot) in Slot::ALL.into_iter().enumerate() {
            let loaded = store.read(slot);
            self.save_usable[index] = loaded.is_ok();
            self.save_previews[index] = loaded
                .as_ref()
                .ok()
                .and_then(|s| s.preview.as_ref())
                .and_then(|p| p.texture());
            self.save_dates[index] = loaded
                .as_ref()
                .map(|s| {
                    format!(
                        "{}{}",
                        save_screen::timestamp(s.saved_at),
                        if s.backup { " (backup)" } else { "" }
                    )
                })
                .unwrap_or_else(|_| slot.title().into());
            self.save_labels[index] = loaded
                .map(|s| {
                    format!(
                        "{}{}",
                        s.game
                            .level()
                            .map_or("Unknown", |l| crate::campaign::level_title(&l.map)),
                        if s.backup { " (backup)" } else { "" }
                    )
                })
                .unwrap_or_else(|_| {
                    if store.exists(slot) {
                        "Unreadable save"
                    } else {
                        "Empty slot"
                    }
                    .into()
                });
        }
    }
    pub async fn check_save_previews(&mut self, store: &Store) -> Result<()> {
        self.refresh_saves(store);
        ensure!(
            self.save_previews[0].is_some() && self.save_previews[1].is_some(),
            "Saved previews missing from menu"
        );
        ensure!(
            self.save_previews[2].is_none() && self.save_previews[3].is_none(),
            "Legacy/damaged image did not fall back"
        );
        for index in [0, 1, 2] {
            self.save_selected = index;
            self.selected = index;
            self.save_film.open(index, get_time() - 1.);
            let buttons = self.buttons(Page::LoadSave);
            self.draw(Page::LoadSave, &buttons);
            crate::viewer::save_capture(std::path::Path::new(&format!(
                "private/save-preview-menu-{index}.png"
            )))?;
            next_frame().await;
        }
        self.save_selected = 1;
        self.selected = 1;
        self.save_film.open(0, get_time() - 1.);
        self.save_film.select(1, get_time() - 0.23);
        self.draw(Page::LoadSave, &self.buttons(Page::LoadSave));
        crate::viewer::save_capture(std::path::Path::new("private/save-menu-shutters.png"))?;
        next_frame().await;
        self.save_film.open(1, get_time() - 1.);
        let confirm = Page::ConfirmSave(Slot::Two);
        self.draw(confirm, &self.buttons(confirm));
        crate::viewer::save_capture(std::path::Path::new("private/save-menu-confirm.png"))?;
        next_frame().await;
        Ok(())
    }
    fn controls_page(&mut self, delta: i32) {
        let count = BINDINGS.len().div_ceil(8) + usize::from(self.controls_pad);
        self.controls_page = (self.controls_page as i32 + delta).rem_euclid(count as i32) as usize;
        self.selected = 0;
    }
    fn back(&mut self, page: Page, prefs: &Preferences, audio: audio::Settings) -> Option<Action> {
        if self.post_game || self.frontend {
            if page == Page::Main {
                return None;
            }
            if page == Page::Credits {
                self.open(Page::Main, prefs, audio);
                return None;
            }
        }
        if let Some(parent) = page.parent() {
            self.open(parent, prefs, audio);
            None
        } else {
            self.close();
            Some(Action::Resume)
        }
    }
    fn draw_mirror(&mut self) {
        let eye = vec3(125., 0., 38.);
        set_camera(&Camera3D {
            position: eye,
            target: vec3(0., 0., 33.),
            up: Vec3::Z,
            fovy: 40f32.to_radians(),
            z_near: 1.,
            render_target: Some(self.mirror_target.clone()),
            aspect: Some(384. / 480.),
            ..Default::default()
        });
        clear_background(Color::new(0., 0., 0., 0.));
        self.material
            .atmosphere(&crate::environment::Atmosphere::default(), eye);
        self.material.bind();
        self.mirror.draw_filtered(
            "idle_stand_rocktoes",
            get_time() as f32,
            true,
            crate::skeletal::Transform {
                translation: Vec3::ZERO,
                rotation: Quat::from_rotation_z(-0.2),
            },
            1.,
            false,
            &["material15", "material16", "material17"],
        );
        gl_use_default_material();
        set_default_camera();
    }
    fn draw(&mut self, page: Page, buttons: &[Button]) {
        let canvas = Canvas::new(screen_width(), screen_height());
        if page.layout() == "controls" {
            self.draw_mirror();
        }
        clear_background(BLACK);
        if page.layout() == "loadsave" {
            self.draw_save_underlay(canvas, get_time());
        }
        for widget in &self.layouts[page.layout()] {
            if widget.kind == "Alice3D" {
                let r = canvas.rect(Rect::new(356., 105., 231., 289.));
                draw_texture_ex(
                    &self.mirror_target.texture,
                    r.x,
                    r.y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(r.size()),
                        flip_y: true,
                        ..Default::default()
                    },
                );
            }
            if widget.kind != "Label" {
                continue;
            }
            if page.layout() == "loadsave"
                && (widget.value("name").starts_with("LoadSaveShot")
                    || [
                        "BigSaveShot",
                        "sepia1",
                        "bigcover_black",
                        "bigcover_left",
                        "bigcover_right",
                    ]
                    .contains(&widget.value("name")))
            {
                continue;
            }
            // These original widgets are activated only on their error path.
            if ["DiskFull", "MsgBackground"].contains(&widget.value("name")) {
                continue;
            }
            let group = widget.value("groupid");
            if !(group.is_empty() || page == Page::Settings && group == "group_main") {
                continue;
            }
            let art = widget.art();
            let alpha = if art.contains("/glow") || art.contains("wick") {
                0.15 + (get_time() as f32 * 3.).sin().abs() * 0.15
            } else {
                1.
            };
            self.art(canvas, art, widget.rect, alpha);
        }
        if page == Page::Video {
            let ink = Color::from_hex(0x330b0b);
            self.art(
                canvas,
                "ui/control/head_2_video",
                Rect::new(48., 319., 256., 64.),
                1.,
            );
            for (title, y) in [
                ("Window Resolution", 76.),
                ("Display Mode", 156.),
                ("Performance Profile", 224.),
            ] {
                self.ui
                    .font
                    .draw(canvas, title, Rect::new(65., y, 226., 26.), 24., ink);
            }
            let preset = match SIZES[self.draft.resolution] {
                (1920, 1080) => "Full HD / 1080p",
                (2560, 1440) => "QHD / 1440p (2K)",
                (3840, 2160) => "Ultra HD / 4K",
                _ => "",
            };
            self.ui
                .font
                .draw(canvas, preset, Rect::new(65., 131., 226., 21.), 18., ink);
            self.ui.font.draw(
                canvas,
                "Balances detail, lights & shadows.",
                Rect::new(65., 282., 226., 21.),
                16.,
                ink,
            );
        }
        if matches!(page, Page::Audio | Page::Game) {
            let name = match page {
                Page::Audio => "audio",
                Page::Game => "game",
                _ => "control",
            };
            self.art(
                canvas,
                &format!("ui/control/head_2_{name}"),
                Rect::new(48., 319., 256., 64.),
                1.,
            );
        }
        if page == Page::Settings {
            if let Some(b) = buttons.get(self.selected) {
                let art = match b.click {
                    Click::Page(Page::Video) => "video",
                    Click::Page(Page::Audio) => "audio",
                    Click::Page(Page::Game) => "game",
                    _ => "control",
                };
                self.art(
                    canvas,
                    &format!("ui/control/head_2_{art}"),
                    Rect::new(48., 319., 256., 64.),
                    1.,
                );
            }
        }
        if page.layout() == "loadsave" {
            self.draw_save_details(canvas, page);
        }
        if matches!(
            page,
            Page::ConfirmNew(_) | Page::ConfirmLoad(_) | Page::ConfirmSave(_)
        ) {
            let r = canvas.rect(Rect::new(100., 165., 440., 153.));
            self.ui.panel(r);
            let title = match page {
                Page::ConfirmNew(d) => format!("Start a new {} game?", d.name()),
                Page::ConfirmLoad(_) => "Replace this session with the saved game?".into(),
                Page::ConfirmSave(s) => format!("Replace {} with this game?", s.title()),
                _ => unreachable!(),
            };
            self.ui.font.draw(
                canvas,
                &title,
                Rect::new(112., 182., 416., 36.),
                24.,
                crate::ui::INK,
            );
            self.ui.font.draw(
                canvas,
                if matches!(page, Page::ConfirmNew(_)) {
                    "Your current game will be saved automatically."
                } else {
                    "Choose No to return."
                },
                Rect::new(114., 222., 412., 25.),
                18.,
                crate::ui::INK,
            );
        }
        if page == Page::Credits {
            for (i, line) in self
                .credits
                .iter()
                .skip(self.credit_offset)
                .take(15)
                .enumerate()
            {
                self.ui.font.draw(
                    canvas,
                    line,
                    Rect::new(32., 83. + i as f32 * 20., 260., 22.),
                    20.,
                    BLACK,
                );
            }
        }
        for (i, b) in buttons.iter().enumerate() {
            let selected = i == self.selected;
            self.art(
                canvas,
                if selected && !b.hover.is_empty() {
                    &b.hover
                } else {
                    &b.art
                },
                b.rect,
                1.,
            );
            let dark = Color::from_hex(if selected { 0x831c12 } else { 0x330b0b });
            if !b.text.is_empty() {
                let r = if page == Page::Video && matches!(b.click, Click::Row(_)) {
                    for (name, x) in [("prev", b.rect.x), ("next", b.rect.right() - 18.)] {
                        self.art(
                            canvas,
                            &format!(
                                "ui/control/hslider_{name}{}",
                                if selected { "_pressed" } else { "" }
                            ),
                            Rect::new(x, b.rect.y + 5., 16., 16.),
                            1.,
                        );
                    }
                    Rect::new(b.rect.x + 24., b.rect.y, b.rect.w - 48., 26.)
                } else if matches!(b.click, Click::Row(_)) {
                    Rect::new(b.rect.x, b.rect.y, b.rect.w, 26.)
                } else if b.art == "ui/buttons/apply" {
                    Rect::new(b.rect.x + 24., b.rect.y, b.rect.w - 30., b.rect.h)
                } else {
                    b.rect
                };
                if let Click::Row(row) = b.click {
                    if page == Page::Controls && row < BINDINGS.len() {
                        self.ui.font.draw_left(
                            canvas,
                            BINDINGS[row].0,
                            Rect::new(r.x, r.y, r.w - 80., r.h),
                            20.,
                            dark,
                        );
                        self.ui.font.draw(
                            canvas,
                            if self.controls_pad {
                                &self.draft.pad_bindings[row]
                            } else {
                                &self.draft.bindings[row]
                            },
                            Rect::new(r.right() - 78., r.y, 78., r.h),
                            19.,
                            dark,
                        );
                    } else if page != Page::Video {
                        let width = if self.toggle(page, row).is_some() {
                            r.w - 38.
                        } else {
                            r.w
                        };
                        self.ui.font.draw_left(
                            canvas,
                            &b.text,
                            Rect::new(r.x, r.y, width, r.h),
                            24.,
                            dark,
                        );
                    } else {
                        self.ui.font.draw(canvas, &b.text, r, 24., dark);
                    }
                } else {
                    self.ui.font.draw(canvas, &b.text, r, 24., dark);
                }
            }
            if let Click::Row(row) = b.click {
                if let Some(value) = self.toggle(page, row) {
                    self.art(
                        canvas,
                        if value {
                            "ui/control/checkbox_checked"
                        } else {
                            "ui/control/checkbox_unchecked"
                        },
                        Rect::new(260., b.rect.y + 5., 16., 16.),
                        1.,
                    );
                }
                if let Some(value) = self.slider(page, row) {
                    self.art(
                        canvas,
                        "ui/control/hslider_bar",
                        Rect::new(88., b.rect.y + 30., 200., 8.),
                        1.,
                    );
                    self.art(
                        canvas,
                        "ui/control/hslider_indicator",
                        Rect::new(84. + value * 200., b.rect.y + 25., 12., 20.),
                        1.,
                    );
                }
            }
        }
        if page == Page::Settings {
            self.ui.font.draw(
                canvas,
                "1 Video   2 Audio   3 Controls   4 Game",
                Rect::new(85., 441., 455., 24.),
                20.,
                WHITE,
            );
        }
        if page.settings() {
            self.ui.font.draw(
                canvas,
                if self.binding.is_some() {
                    if self.controls_pad {
                        "Press a button   Start / Esc cancel   Delete clear"
                    } else {
                        "Press a key, mouse button or wheel   Esc cancels"
                    }
                } else if self.using_pad {
                    "D-pad / stick: browse   A: select   B: back"
                } else {
                    "Arrows adjust   Enter select   Esc cancel"
                },
                Rect::new(80., 442., 478., 24.),
                19.,
                WHITE,
            );
            if page == Page::Controls && !self.controls_touch {
                self.ui.font.draw(
                    canvas,
                    &format!(
                        "{} / {}",
                        self.controls_page + 1,
                        BINDINGS.len().div_ceil(8) + usize::from(self.controls_pad)
                    ),
                    Rect::new(155., 312., 45., 20.),
                    18.,
                    BLACK,
                );
            }
        }
        if page == Page::Main && !buttons.is_empty() && !self.using_touch {
            self.ui.font.draw(
                canvas,
                if self.frontend || self.post_game {
                    "Tab: chapters"
                } else if self.using_pad {
                    "Tab: chapters   Start / B: back to game"
                } else {
                    "Tab: chapters   Esc: back to game"
                },
                Rect::new(285., 451., 345., 25.),
                18.,
                WHITE,
            );
        }
        if !self.message.is_empty() {
            let r = canvas.rect(Rect::new(86., 10., 470., 48.));
            self.ui.panel(r);
            self.ui.font.draw(
                canvas,
                &self.message,
                Rect::new(102., 14., 438., 40.),
                22.,
                crate::ui::INK,
            );
        }
    }
    /// The world loop does not advance while this modal loop is running.
    pub async fn run(
        &mut self,
        prefs: &mut Preferences,
        audio: &mut audio::Audio,
        store: &Store,
        listener: Vec3,
        yaw: f32,
        input: &mut crate::input::Input,
    ) -> Action {
        show_mouse(false);
        self.refresh_saves(store);
        // Entry grace period (TIME-BASED, 400 ms). On Android, the finger that
        // tapped MENU can be re-delivered by the OS (new ID, re-Started event,
        // synthesized mouse-left click, or focus-change Escape) for several
        // frames after we enter the modal loop, and miniquad does not always
        // return Stationary touches while the finger is held — so we cannot
        // rely on finger-lift to end grace. During this window we ignore:
        //   - any pointer clicks / drags
        //   - Escape / Back / Tab / Y / N keys
        //   - any clicks landed on buttons (Resume/Quit/Back/...)
        // After 400 ms the opening finger is guaranteed to have either lifted
        // or settled, and real player input resumes.
        let grace_deadline = get_time() + 0.40;
        loop {
            let page = self.page.unwrap_or(Page::Main);
            let mut action = None;
            let focused = crate::look::window_focused();
            input.update(prefs, focused);
            self.using_pad = input.using_pad;
            self.using_touch = input.using_touch;
            self.save_film.update_audio(audio.settings, focused);
            let previous_selected = self.selected;
            let buttons = self.buttons(page);
            self.selected = self.selected.min(buttons.len().saturating_sub(1));
            audio.update(0., listener, yaw, true, true);
            let canvas = Canvas::new(screen_width(), screen_height());
            let (mouse, pointer_pressed_raw, pointer_down_raw) = crate::touch::pointer_state();
            // Grace ends purely on wall-clock time — no "finger lifted" short-
            // circuit, because miniquad/Android sometimes skips Stationary
            // events and touches() appears empty even while a finger is held.
            let grace = get_time() < grace_deadline;
            // During grace, swallow every source of click/close input.
            let pointer_pressed = pointer_pressed_raw && !grace;
            let pointer_down = if grace { false } else { pointer_down_raw };
            // Also synthesize a "virtual mouse released" so if Android has
            // already set mouse_left_down=true from the opening finger, we
            // don't see a spurious drag-click mid-grace.
            let hit = buttons
                .iter()
                .position(|b| b.rect.contains(canvas.pointer(mouse)));
            if focused {
                if let Some(row) = self.binding {
                    if is_key_pressed(KeyCode::Escape) || input.pad_pressed("Start") {
                        self.binding = None;
                        self.message.clear();
                    } else if let Some(name) = if self.controls_pad {
                        if is_key_pressed(KeyCode::Delete) {
                            Some("None".into())
                        } else {
                            input.capture_pad().map(str::to_owned)
                        }
                    } else {
                        crate::input::capture_binding()
                    } {
                        if self.draft.bind(row, &name, self.controls_pad) {
                            self.binding = None;
                            self.message.clear();
                            input.suppress();
                        } else {
                            self.message = "That control is reserved. Choose another.".into();
                        }
                    }
                } else {
                    if mouse.distance_squared(self.previous_pointer) > 1. || pointer_pressed {
                        if let Some(i) = hit {
                            self.selected = i;
                        }
                    }
                    if pointer_down {
                        if let Some(i) = hit {
                            if let Click::Row(row) = buttons[i].click {
                                let p = canvas.pointer(mouse);
                                if self.slider(page, row).is_some()
                                    && p.y >= buttons[i].rect.y + 26.
                                {
                                    self.drag_slider(page, row, (p.x - 88.) / 200.);
                                }
                            }
                        }
                    }
                    if input.ui(KeyCode::Down) {
                        self.selected = (self.selected + 1) % buttons.len();
                    }
                    if input.ui(KeyCode::Up) {
                        self.selected = (self.selected + buttons.len() - 1) % buttons.len();
                    }
                    // Escape/Back/Tab/Y/N are ALL suppressed during grace so the
                    // opening finger (or any synthetic Android Back it triggers)
                    // cannot dismiss us. Pad B/Start are also routed through
                    // input.ui(Escape), so they're covered automatically.
                    let hard_block = grace;
                    let click = if !hard_block && page == Page::Main && input.ui(KeyCode::Tab) {
                        Some(Click::Action(Action::Chapters))
                    } else if !hard_block && (input.ui(KeyCode::Escape)
                        || is_key_pressed(KeyCode::Back))
                    {
                        Some(Click::Back)
                    } else if !hard_block && page == Page::Quit && is_key_pressed(KeyCode::Y) {
                        Some(Click::Action(Action::Quit))
                    } else if !hard_block && page == Page::Quit && is_key_pressed(KeyCode::N) {
                        Some(Click::Back)
                    } else if page == Page::Settings && is_key_pressed(KeyCode::Key1) {
                        Some(Click::Page(Page::Video))
                    } else if page == Page::Settings && is_key_pressed(KeyCode::Key2) {
                        Some(Click::Page(Page::Audio))
                    } else if page == Page::Settings && is_key_pressed(KeyCode::Key3) {
                        Some(Click::Page(Page::Controls))
                    } else if page == Page::Settings && is_key_pressed(KeyCode::Key4) {
                        Some(Click::Page(Page::Game))
                    } else if pointer_pressed {
                        hit.map(|i| buttons[i].click)
                    } else if input.ui(KeyCode::Enter) {
                        Some(buttons[self.selected].click)
                    } else if page == Page::Controls
                        && (input.pad_pressed("LB")
                            || is_key_pressed(KeyCode::PageUp)
                            || mouse_wheel().1 > 0.)
                    {
                        Some(Click::ControlsPage(-1))
                    } else if page == Page::Controls
                        && (input.pad_pressed("RB")
                            || is_key_pressed(KeyCode::PageDown)
                            || mouse_wheel().1 < 0.)
                    {
                        Some(Click::ControlsPage(1))
                    } else {
                        None
                    };
                    if input.ui(KeyCode::Left) || input.ui(KeyCode::Right) {
                        if let Click::Row(row) = buttons[self.selected].click {
                            self.adjust(page, row, if input.ui(KeyCode::Left) { -1. } else { 1. });
                        }
                    }
                    if page == Page::LoadSave && self.selected != previous_selected {
                        self.save_film.sound(audio, 1);
                    }
                    if let Some(click) = click {
                        if page.layout() == "loadsave" && !matches!(click, Click::Slot(_)) {
                            self.save_film.sound(audio, 2);
                        }
                        match click {
                            Click::Back | Click::Cancel => {
                                action = self.back(page, prefs, audio.settings)
                            }
                            Click::Page(p) => self.open(p, prefs, audio.settings),
                            Click::Action(a) => {
                                if a == Action::Chapters {
                                    // Keep the main menu underneath the modal chapter chooser.
                                } else if matches!(a, Action::Save(_) | Action::Load(_)) {
                                    self.open(Page::LoadSave, prefs, audio.settings);
                                } else {
                                    self.close();
                                }
                                action = Some(a);
                            }
                            Click::Row(row) => {
                                if !(pointer_pressed
                                    && self.slider(page, row).is_some()
                                    && canvas.pointer(mouse).y
                                        >= buttons[hit.unwrap_or(self.selected)].rect.y + 26.)
                                {
                                    let previous = page == Page::Video
                                        && pointer_pressed
                                        && hit.is_some_and(|i| {
                                            canvas.pointer(mouse).x < buttons[i].rect.x + 24.
                                        });
                                    self.adjust(page, row, if previous { -1. } else { 1. });
                                }
                            }
                            Click::Reset => {
                                let defaults = Preferences::default();
                                if self.controls_touch {
                                    self.draft.touch_mode = defaults.touch_mode;
                                    self.draft.touch_sensitivity = defaults.touch_sensitivity;
                                    self.draft.invert_touch = defaults.invert_touch;
                                    self.draft.touch_scale = defaults.touch_scale;
                                    self.draft.touch_opacity = defaults.touch_opacity;
                                    self.draft.touch_left_handed = defaults.touch_left_handed;
                                } else if self.controls_pad {
                                    self.draft.pad_bindings = defaults.pad_bindings;
                                    self.draft.pad_sensitivity = defaults.pad_sensitivity;
                                    self.draft.pad_deadzone = defaults.pad_deadzone;
                                    self.draft.invert_pad = false;
                                } else {
                                    self.draft.bindings = defaults.bindings;
                                }
                            }
                            Click::ControlsPage(delta) => self.controls_page(delta),
                            Click::Device => {
                                if !self.controls_pad && !self.controls_touch {
                                    self.controls_pad = true;
                                } else if self.controls_pad {
                                    self.controls_pad = false;
                                    self.controls_touch = true;
                                } else {
                                    self.controls_touch = false;
                                }
                                self.controls_page = 0;
                                self.selected = 0;
                            }
                            Click::Slot(i) => {
                                self.save_selected = i;
                                self.save_film.select(i, get_time());
                                self.save_film.sound(audio, 0);
                                self.message.clear();
                            }
                            Click::LoadSelection => {
                                if self.save_usable[self.save_selected] {
                                    let slot = Slot::ALL[self.save_selected];
                                    if self.frontend {
                                        action = Some(Action::Load(slot));
                                    } else {
                                        self.open(Page::ConfirmLoad(slot), prefs, audio.settings);
                                    }
                                } else {
                                    self.message =
                                        if self.frontend { "This slot has no usable save." } else { "This slot has no usable save. Choose Save to create one." }
                                            .into();
                                }
                            }
                            Click::SaveSelection => {
                                if self.post_game {
                                    self.message =
                                        "Campaign complete. Load a checkpoint or start a new game."
                                            .into();
                                    continue;
                                }
                                let slot = Slot::ALL[self.save_selected];
                                if slot == Slot::Auto {
                                    self.message = "Choose a manual slot or the quick save.".into();
                                } else if store.exists(slot) {
                                    self.open(Page::ConfirmSave(slot), prefs, audio.settings);
                                } else {
                                    action = Some(Action::Save(slot));
                                }
                            }
                            Click::CreditScroll(delta) => {
                                self.credit_offset = (self.credit_offset as i32 + delta * 14)
                                    .clamp(0, self.credits.len().saturating_sub(15) as i32)
                                    as usize
                            }
                            Click::Apply => {
                                let display_changed = prefs.resolution != self.draft.resolution
                                    || prefs.fullscreen != self.draft.fullscreen
                                    || prefs.performance_preset != self.draft.performance_preset
                                    || prefs.fps_limit != self.draft.fps_limit;
                                match self.draft.save().and_then(|_| self.draft_audio.save()) {
                                    Ok(()) => {
                                        *prefs = self.draft.clone();
                                        audio.settings = self.draft_audio;
                                        if display_changed {
                                            prefs.display();
                                        }
                                        self.open(Page::Settings, prefs, audio.settings);
                                        self.message = "Settings saved".into();
                                    }
                                    Err(e) => {
                                        self.message = format!("Could not save settings: {e}")
                                    }
                                }
                            }
                        }
                    }
                }
            }
            self.previous_pointer = mouse;
            if is_quit_requested() {
                self.close();
                action = Some(Action::Quit);
            }
            // Consume the activating click/key before handing control back to gameplay.
            if let Some(action) = action {
                input.suppress();
                show_mouse(true);
                next_frame().await;
                return action;
            }
            let page = self.page.unwrap_or(Page::Main);
            let buttons = self.buttons(page);
            self.selected = self.selected.min(buttons.len().saturating_sub(1));
            self.draw(page, &buttons);
            if focused && !self.using_touch && !crate::android::is_android() {
                self.ui.cursor();
            }
            if focused && is_key_pressed(KeyCode::F12) {
                let path = std::path::PathBuf::from(format!(
                    "private/screenshots/menu-{}.png",
                    (get_time() * 1000.) as u64
                ));
                if let Err(e) = crate::viewer::save_capture(&path) {
                    self.message = format!("Capture failed: {e}");
                }
            }
            next_frame().await;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layouts_are_data_and_bounds_are_checked() {
        let w=layout("menu test\nresource\nButton\n{\nrect 12 16 80 24\ntitle \"Back to game\"\nstuffcommand \"quit\"\n}\n").unwrap();
        assert_eq!(w[0].rect, Rect::new(12., 16., 80., 24.));
        assert_eq!(w[0].value("title"), "Back to game");
        assert!(layout("resource\nLabel\n{\nrect NaN 0 1 2\n}\n").is_err());
        assert!(layout("resource\nLabel\n{\nrect 0 0 -1 2\n}\n").is_err());
        assert!(layout("resource\nLabel\n{").is_err());
    }
    #[test]
    fn letterboxing_preserves_original_hit_regions() {
        let c = Canvas::new(1200., 680.);
        let original = Rect::new(61., 159., 256., 64.);
        let screen = c.rect(original);
        assert!(original.contains(c.pointer(screen.center())));
        assert!(!original.contains(c.pointer(vec2(0., 0.))));
        assert!((c.rect(Rect::new(0., 0., 640., 480.)).w / 680. - 4. / 3.).abs() < 0.001);
    }
    #[test]
    fn escape_returns_through_menu_hierarchy_never_quits() {
        assert_eq!(Page::Audio.parent(), Some(Page::Settings));
        assert_eq!(Page::Settings.parent(), Some(Page::Main));
        assert_eq!(Page::Quit.parent(), Some(Page::Main));
        assert_eq!(Page::Main.parent(), None);
        assert_eq!(
            Page::ConfirmLoad(Slot::Quick).parent(),
            Some(Page::LoadSave)
        );
    }
}
