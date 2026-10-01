//! Compact HUD using the user's original meter and weapon artwork.
use crate::{
    assets::Assets,
    inventory::{Catalog, Pickup, PickupKind, Stats, WEAPONS},
    texture,
};
use anyhow::Result;
use macroquad::prelude::*;

const METER_SCALE: f32 = 0.78;

/// Encounter progress, drawn after returning to the screen-space camera.
pub fn boss_meter(ui: &crate::ui::Ui, label: &str, remaining: f32) {
    let s = crate::ui::overlay_scale();
    let w = (280. * s).min((screen_width() - 64. * s).max(1.));
    let x = (screen_width() - w) * 0.5;
    ui.dialog(Rect::new(x - 22. * s, 12. * s, w + 44. * s, 48. * s));
    ui.fit_label(label, x, 30. * s, 18. * s, w, WHITE);
    draw_rectangle(x, 39. * s, w, 7. * s, Color::new(0.12, 0.04, 0.05, 0.9));
    draw_rectangle(
        x,
        39. * s,
        w * remaining.clamp(0., 1.),
        7. * s,
        Color::new(0.65, 0.16, 0.21, 0.9),
    );
}

pub fn air(breath: &crate::water::Breath, ui: &crate::ui::Ui) {
    let text = format!("Air  {:.0}", breath.remaining().ceil());
    ui.toast(&text, screen_height() - 76.);
}

pub struct Hud {
    pub ui: std::rc::Rc<crate::ui::Ui>,
    meters: [Texture2D; 2],
    icons: Vec<Texture2D>,
    power_icons: [Texture2D; 3],
    pickup_models: std::cell::RefCell<Vec<crate::weapons::Prop>>,
    foldout: crate::tan::Model,
    bar: crate::tan::Model,
    riser: crate::tan::Model,
    back: crate::tan::Model,
    back_texture: Texture2D,
    liquids: [Texture2D; 2],
    pub message: String,
    remaining: f32,
}
fn load(assets: &mut Assets, path: &str) -> Result<Texture2D> {
    let image = texture::decode(assets, path)?;
    let t = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
    t.set_filter(FilterMode::Linear);
    Ok(t)
}
impl Hud {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let specs = texture::read_materials(assets)?;
        let pickup_models = WEAPONS
            .iter()
            .map(|(id, _)| crate::weapons::Prop::load(assets, &format!("w_{id}"), &specs))
            .collect::<Result<Vec<_>>>()?;
        let meters = [
            load(assets, "models/ui/pieces/main/health.ftx")?,
            load(assets, "models/ui/pieces/main/skin01.ftx")?,
        ];
        let mut icons = Vec::new();
        for (id, _) in WEAPONS {
            icons.push(load(
                assets,
                &format!(
                    "models/ui/{}.ftx",
                    if id == "demondice" { "dice1" } else { id }
                ),
            )?);
        }
        Ok(Self {
            pickup_models: std::cell::RefCell::new(pickup_models),
            ui: crate::ui::Ui::load(assets)?,
            meters,
            icons,
            power_icons: [
                load(assets, "models/ui/ragebox.ftx")?,
                load(assets, "models/ui/tea.ftx")?,
                load(assets, "models/ui/glass.ftx")?,
            ],
            foldout: crate::tan::Model::parse(&assets.read("models/ui/hud/foldout/folding.tan")?)?,
            bar: crate::tan::Model::parse(&assets.read("models/ui/hud/bar/notmoving.tan")?)?,
            riser: crate::tan::Model::parse(&assets.read("models/ui/hud/riser/rising.tan")?)?,
            back: crate::tan::Model::parse(&assets.read("models/ui/hud/back/notmoving.tan")?)?,
            back_texture: load(assets, "models/ui/pieces/main/skin_back01.ftx")?,
            liquids: [
                load(assets, "models/ui/pieces/main/skin_health02.ftx")?,
                load(assets, "models/ui/pieces/main/skin_mana01.ftx")?,
            ],
            message: String::new(),
            remaining: 0.,
        })
    }
    pub fn announce(&mut self, text: impl Into<String>) {
        self.message = text.into();
        self.remaining = (2.5 + self.message.chars().count() as f32 * 0.035).clamp(3.5, 8.);
    }
    pub fn update(&mut self, dt: f32) {
        self.remaining = (self.remaining - dt.max(0.)).max(0.);
    }
    pub fn selected(&mut self, stats: &Stats) {
        if let Some(selected) = stats.equipped() {
            self.announce(WEAPONS[selected].1);
        }
    }
    pub fn draw_notice(&self) {
        if self.remaining > 0. && !self.message.is_empty() {
            // Leave the top-center encounter meter unobstructed.
            self.ui.toast(&self.message, 80. * crate::ui::overlay_scale());
        }
    }
    fn icon(&self, index: usize, r: Rect, color: Color) {
        self.foldout_icon(&self.icons[index], r, false, color);
    }
    fn foldout_icon(&self, texture: &Texture2D, r: Rect, mirror: bool, color: Color) {
        let points = &self.foldout.surfaces[0].frames[0];
        let (min, max) = bounds(points);
        let scale = (r.w / (max.x - min.x)).min(r.h / (max.y - min.y));
        let base = vec2(
            r.x + (r.w - (max.x - min.x) * scale) * 0.5,
            r.y + (r.h - (max.y - min.y) * scale) * 0.5,
        );
        tan_picture(
            &self.foldout,
            0.,
            texture,
            base,
            min,
            max,
            scale,
            mirror,
            color,
        );
    }
    fn meter(&self, index: usize, value: f32, x: f32, y: f32, scale: f32) {
        let (min, max) = bounds(&self.bar.surfaces[0].frames[0]);
        let base = vec2(x, y);
        let scale = scale * METER_SCALE;
        tan_picture(
            &self.back,
            0.,
            &self.back_texture,
            base,
            min,
            max,
            scale,
            index == 1,
            WHITE,
        );
        tan_picture(
            &self.riser,
            (1. - value.clamp(0., 100.) / 100.) * 100.,
            &self.liquids[index],
            base,
            min,
            max,
            scale,
            index == 1,
            WHITE,
        );
        tan_picture(
            &self.bar,
            0.,
            &self.meters[index],
            base,
            min,
            max,
            scale,
            index == 1,
            WHITE,
        );
    }
    fn meter_icon_rect(&self, x: f32, y: f32, scale: f32, mirror: bool) -> Rect {
        // Bar and foldout use the same authored origin. Keep the hinge attached
        // to the handle instead of independently anchoring the card to the screen.
        let (bar_min, bar_max) = bounds(&self.bar.surfaces[0].frames[0]);
        let (min, max) = bounds(&self.foldout.surfaces[0].frames[0]);
        let scale = scale * METER_SCALE;
        let left = if mirror {
            bar_max.x - max.x
        } else {
            min.x - bar_min.x
        };
        Rect::new(
            x + left * scale,
            y + (bar_max.y - max.y) * scale,
            (max.x - min.x) * scale,
            (max.y - min.y) * scale,
        )
    }
    pub fn draw(
        &self,
        stats: &Stats,
        catalog: &Catalog,
        inventory: bool,
        input: &crate::input::Input,
        prefs: &crate::preferences::Preferences,
    ) {
        let text_scale = crate::ui::overlay_scale();
        if !inventory {
            if let Some((kind, time)) = stats.active_power() {
                self.ui.label(
                    &format!("{}  {:.0}s", kind.name(), time.ceil()),
                    24. * text_scale,
                    76. * text_scale,
                    18. * text_scale,
                    Color::from_hex(0xc2cce8),
                );
            }
            if stats.powers.stopped > 0. {
                self.ui.label(
                    &format!("Time stopped  {:.0}s", stats.powers.stopped.ceil()),
                    24. * text_scale,
                    100. * text_scale,
                    18. * text_scale,
                    Color::from_hex(0xd8c09c),
                );
            } else if stats.selected() == 9 && stats.powers.recharge > 0. {
                self.ui.label(
                    &format!("Watch ready in {:.0}s", stats.powers.recharge.ceil()),
                    24. * text_scale,
                    100. * text_scale,
                    18. * text_scale,
                    Color::from_hex(0xd8c09c),
                );
            }
        }
        if inventory {
            self.draw_inventory(stats, catalog, input, prefs);
            return;
        }
        let (w, h) = (screen_width(), screen_height());
        let s = (h / 900.).clamp(0.55, 4.).min(w / 900.);
        let y = h - 64. * s - 385. * s;
        let weapon_rect = self.meter_icon_rect(24. * s, y, s, false);
        if let Some((kind, _)) = stats.active_power() {
            self.foldout_icon(
                &self.power_icons[kind as usize],
                self.meter_icon_rect(w - 48. * s, y, s, true),
                true,
                WHITE,
            );
        }
        if let Some(selected) = stats.equipped() {
            self.icon(selected, weapon_rect, WHITE);
            self.ui.fit_label(
                WEAPONS[selected].1,
                weapon_rect.right() + 10. * s,
                weapon_rect.y + weapon_rect.h * 0.5 + 11. * s,
                25. * s,
                235. * s,
                Color::from_hex(0xeee4d4),
            );
        }
        // The handles cover the cards' hinges on both sides.
        self.meter(0, stats.sanity(), 24. * s, y, s);
        self.meter(1, stats.will(), w - 48. * s, y, s);
        if !stats.alive() {
            let r = Rect::new(
                (w - 440. * text_scale).max(16. * text_scale) * 0.5,
                h * 0.34,
                (w - 32. * text_scale).min(440. * text_scale),
                145. * text_scale,
            );
            self.ui.panel(r);
            self.ui.center(
                "Sanity lost",
                Rect::new(r.x + 20. * text_scale, r.y + 18. * text_scale, r.w - 40. * text_scale, 32. * text_scale),
                30. * text_scale,
                crate::ui::INK,
            );
            self.ui.center(
                &format!(
                    "{} - retry",
                    if input.using_pad {
                        "A".into()
                    } else {
                        input.label(prefs, "Enter")
                    }
                ),
                Rect::new(r.x + 20. * text_scale, r.y + 64. * text_scale, r.w - 40. * text_scale, 26. * text_scale),
                22. * text_scale,
                crate::ui::INK,
            );
            self.ui.center(
                if input.using_pad {
                    "Start - load a game or change options"
                } else {
                    "Esc - load a game or change options"
                },
                Rect::new(r.x + 20. * text_scale, r.y + 102. * text_scale, r.w - 40. * text_scale, 26. * text_scale),
                22. * text_scale,
                crate::ui::INK,
            );
        }
    }
    pub fn inventory_hit(&self) -> Option<usize> {
        if !is_mouse_button_pressed(MouseButton::Left) {
            return None;
        }
        let p = Vec2::from(mouse_position());
        (0..10).find(|&i| slot(i).contains(p))
    }
    fn draw_inventory(
        &self,
        stats: &Stats,
        catalog: &Catalog,
        input: &crate::input::Input,
        prefs: &crate::preferences::Preferences,
    ) {
        let first = slot(0);
        let last = slot(9);
        let width = last.right() - first.x;
        let panel = Rect::new(
            first.x - 16.,
            first.y - 112.,
            width + 32.,
            last.bottom() - first.y + 214.,
        );
        self.ui.panel(panel);
        let ink = crate::ui::INK;
        self.ui.center(
            "Alice's Toys",
            Rect::new(first.x, first.y - 101., width, 30.),
            30.,
            ink,
        );
        self.ui.center(
            &stats.school_items.summary(),
            Rect::new(first.x, first.y - 68., width, 23.),
            20.,
            ink,
        );
        self.ui.center(
            &if input.using_pad {
                "D-pad: select    A: equip    B: return".into()
            } else {
                format!(
                    "Arrows / click: select    {} / Esc: return",
                    input.label(prefs, "I")
                )
            },
            Rect::new(first.x, first.y - 39., width, 23.),
            20.,
            ink,
        );
        for (i, &(_, name)) in WEAPONS.iter().enumerate() {
            let r = slot(i);
            let owned = stats.copies(i) > 0;
            let current = stats.equipped() == Some(i);
            if current {
                self.ui
                    .marker(Rect::new(r.right() - 18., r.y + 6., 16., 16.));
            }
            let size = (r.h - 52.).min(r.w - 20.);
            self.icon(
                i,
                Rect::new(r.x + (r.w - size) * 0.5, r.y + 5., size, size),
                if owned {
                    WHITE
                } else {
                    Color::new(0.4, 0.4, 0.4, 0.55)
                },
            );
            self.ui.font.left(
                &if input.using_pad {
                    String::new()
                } else {
                    input.label(prefs, &format!("{}", (i + 1) % 10))
                },
                r.x + 7.,
                r.y + 24.,
                16.,
                ink,
            );
            self.ui.center(
                name,
                Rect::new(r.x + 5., r.bottom() - 44., r.w - 10., 22.),
                20.,
                ink,
            );
            let detail = if i == 6 && owned {
                format!("{} dice", stats.copies(i))
            } else if current {
                "Selected".into()
            } else if !owned {
                "Unavailable".into()
            } else {
                String::new()
            };
            self.ui.center(
                &detail,
                Rect::new(r.x + 5., r.bottom() - 22., r.w - 10., 19.),
                17.,
                ink,
            );
        }
        let weapon = &catalog.weapons[stats.selected()];
        let details = if stats.equipped().is_none() {
            "Unarmed".into()
        } else if stats.selected() == 9 {
            "Pocket Watch - stops time".into()
        } else {
            format!(
                "Sanity {:.0}    Will {:.0}    Primary cost: {:.0}",
                stats.sanity(),
                stats.will(),
                weapon.primary
            )
        };
        self.ui.center(
            &details,
            Rect::new(first.x, last.bottom() + 11., width, 25.),
            22.,
            ink,
        );
        self.ui.center(
            &if stats.equipped().is_some() {
                format!(
                    "{} / {}: primary / alternate attack",
                    input.label(prefs, "Mouse 1"),
                    input.label(prefs, "Mouse 2")
                )
            } else {
                "Find a toy to equip".into()
            },
            Rect::new(first.x, last.bottom() + 39., width, 23.),
            20.,
            ink,
        );
        if self.remaining > 0. {
            self.ui.center(
                &self.message,
                Rect::new(first.x, last.bottom() + 65., width, 22.),
                19.,
                ink,
            );
        }
    }
    pub fn draw_pickups(&self, items: &[Pickup], stats: &Stats, camera: Vec3, clock: f32) {
        for item in items {
            if stats.collected.contains(&item.id)
                || camera.distance_squared(item.origin) > 1500. * 1500.
            {
                continue;
            }
            match item.kind {
                PickupKind::Power(_) => {}
                PickupKind::Weapon(i) => {
                    self.pickup_models.borrow_mut()[i].draw_pickup(
                        item.origin + Vec3::Z * (44. + (clock * 2.).sin() * 2.),
                        clock * 0.6,
                    );
                }
                _ => {} // Resource and powerup art shares the world item renderer.
            }
        }
    }
}
fn draw_mesh(mesh: &Mesh) {
    crate::render_fx::effect(mesh, crate::materials::Blend::Alpha);
}

/// Native layout fixtures use the same HUD, notices and subtitle drawing as play.
pub(crate) async fn check_presentation(assets: &mut Assets) -> Result<()> {
    let output = std::path::PathBuf::from(std::env::var_os("LOOKING_GLASS_PRESENTATION_CHECK").unwrap());
    std::fs::create_dir_all(&output)?;
    let mut hud = Hud::load(assets)?;
    let catalog = Catalog::load(assets)?;
    let prefs = crate::preferences::Preferences::default();
    let input = crate::input::Input::default();
    for (width, height) in [(640, 480), (1200, 680), (1920, 1080), (3840, 2160)] {
        request_new_screen_size(width as f32, height as f32);
        for _ in 0..8 { next_frame().await; }
        anyhow::ensure!(screen_width() as u32 == width && screen_height() as u32 == height,
            "Presentation viewport was constrained to {} x {}", screen_width(), screen_height());
        for page in ["play", "pause", "death"] {
            let mut stats = Stats::preview();
            if page == "death" { stats.damage(10000.); }
            hud.announce("A new chapter is ready. Your toys and progress have been carried into this visit. Explore the room at your own pace; you can open the main menu whenever you need to save, load, or adjust your controls.");
            for frame in 0..3 {
                set_default_camera();
                clear_background(Color::from_hex(0x242228));
                hud.draw(&stats, &catalog, false, &input, &prefs);
                if page == "pause" {
                    hud.ui.paused("P or click to resume");
                } else if page == "play" {
                    hud.draw_notice();
                    boss_meter(&hud.ui, "Encounter", 0.6);
                    crate::story::draw_subtitle(
                        "The path winds through the room. Look around carefully, then choose where to go next.",
                        "Alice", "E", 1., 8., &hud.ui);
                }
                if frame == 2 {
                    crate::viewer::save_capture(&output.join(format!("{width}x{height}-{page}.png")))?;
                }
                next_frame().await;
            }
        }
        println!("PASS presentation layout {width}x{height}");
    }
    Ok(())
}
fn slot(index: usize) -> Rect {
    let width = (screen_width() - 48.).clamp(400., 900.);
    let gap = 10.;
    let cell = (width - gap * 4.) / 5.;
    let height = (screen_height() * 0.23).clamp(100., 160.);
    let top = (screen_height() - height * 2. - gap) / 2. + 15.;
    Rect::new(
        (screen_width() - width) / 2. + (index % 5) as f32 * (cell + gap),
        top + (index / 5) as f32 * (height + gap),
        cell,
        height,
    )
}

// Project the original HUD meshes in their authored Y/Z plane. Using their UVs
// avoids drawing unused opaque atlas areas around the frames and glass.
fn bounds(points: &[Vec3]) -> (Vec2, Vec2) {
    points.iter().fold(
        (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
        |(lo, hi), p| {
            let q = vec2(p.y, p.z);
            (lo.min(q), hi.max(q))
        },
    )
}
#[allow(clippy::too_many_arguments)]
fn tan_picture(
    model: &crate::tan::Model,
    frame: f32,
    texture: &Texture2D,
    base: Vec2,
    min: Vec2,
    max: Vec2,
    scale: f32,
    mirror: bool,
    color: Color,
) {
    for surface in &model.surfaces {
        let f = frame.clamp(0., (surface.frames.len() - 1) as f32);
        let lo = f.floor() as usize;
        let hi = (lo + 1).min(surface.frames.len() - 1);
        let positions = surface.frames[lo]
            .iter()
            .zip(&surface.frames[hi])
            .map(|(a, b)| a.lerp(*b, f.fract()))
            .collect::<Vec<_>>();
        let mut triangles = surface.indices.chunks_exact(3).collect::<Vec<_>>();
        triangles.sort_by(|a, b| {
            let depth = |t: &&[u16]| t.iter().map(|&i| positions[i as usize].x).sum::<f32>();
            depth(a).total_cmp(&depth(b))
        });
        let indices = triangles.into_iter().flatten().copied().collect();
        let vertices = positions
            .iter()
            .zip(&surface.uv)
            .map(|(p, &uv)| {
                let x = if mirror { max.x - p.y } else { p.y - min.x };
                Vertex {
                    position: vec3(base.x + x * scale, base.y + (max.y - p.z) * scale, 0.),
                    uv,
                    color: color.into(),
                    normal: Vec4::ZERO,
                }
            })
            .collect();
        draw_mesh(&Mesh {
            vertices,
            indices,
            texture: Some(texture.clone()),
        });
    }
}
