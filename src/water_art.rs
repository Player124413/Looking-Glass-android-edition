//! Alice's swimming shell and low-air warning, derived from saved player state.
use crate::{
    assets::Assets,
    environment::Atmosphere,
    movement::Player,
    particles::Attached,
    skeletal::{Skeleton, Transform},
    texture,
    weapons::Prop,
};
use anyhow::{Context, Result};
use macroquad::prelude::*;
use std::{cell::RefCell, collections::BTreeMap};

pub struct Art {
    shell: Prop,
    bubbles: Attached,
    events: crate::animation_events::Model,
    warning_duration: f32,
    warning_frame: f32,
    back: usize,
    mouth: usize,
    visible: bool,
    warning: Option<f32>,
    view: RefCell<(Atmosphere, Vec3)>,
}

/// Exercise the actual Character draw path, including immediate paused restores.
pub async fn check(a: &mut Assets) -> Result<()> {
    use crate::{
        character::Character,
        movement::Controls,
        water::{Immersion, Liquid},
    };
    use anyhow::ensure;
    let root = std::path::Path::new("private/turtle-shell");
    std::fs::create_dir_all(root)?;
    let mut scene = crate::render::Scene::load(a, "garden1")?;
    // The authored entrance current carries an idle player out of this pool.
    // Isolate breathing here, as --swim-check does; normal gameplay keeps it.
    for current in &mut scene.world.traversal.pushes {
        current.enabled = false;
    }
    let mut player =
        Player::spawn(&scene.world, scene.map.spawn().0).context("Garden swimming spawn")?;
    let stats = crate::inventory::Stats::for_level("garden1", None);
    stats.prepare_player(&mut player);
    for _ in 0..120 * 6 {
        player.tick(&scene.world, Controls::default());
    }
    ensure!(
        player.immersion.level == 3 && player.breath.shell && player.breath.hits == 0,
        "Shell did not protect actual Garden swimmer beyond five seconds: level={}, shell={}, hits={}",
        player.immersion.level, player.breath.shell, player.breath.hits
    );
    let mut alice = Character::load(a)?;
    alice.reset(&player, 0.);
    for _ in 0..120 {
        alice.update(
            1. / 60.,
            &player,
            false,
            crate::weapons::WeaponInput {
                aim: Vec3::X,
                ..Default::default()
            },
            &crate::combat::Context {
                world: &scene.world,
                targets: &[],
            },
        );
    }
    let saved = alice.snapshot();
    let mut restored = Character::load(a)?;
    restored.restore(&serde_json::from_str(&serde_json::to_string(&saved)?)?)?;
    let saved_player: Player = serde_json::from_str(&serde_json::to_string(&player)?)?;
    let target = player.feet + Vec3::Z * 32.;
    let camera = Camera3D {
        position: target + vec3(-115., -80., 35.),
        target,
        up: Vec3::Z,
        fovy: 55_f32.to_radians(),
        z_near: 2.,
        z_far: 30000.,
        ..Default::default()
    };
    let mut pictures = Vec::new();
    for case in [
        "unowned",
        "shell",
        "paused",
        "restored",
        "dry",
        "lava",
        "low-air",
        "low-air-paused",
        "refilled",
    ] {
        player = serde_json::from_str(&serde_json::to_string(&saved_player)?)?;
        player.breath.submerged = 1.;
        if case == "unowned" {
            player.breath.shell = false;
        }
        if case == "dry" {
            player.immersion = Immersion::default();
        }
        if case == "lava" {
            player.immersion.kind = Liquid::Lava;
        }
        if case.starts_with("low-air") {
            player.breath.submerged = 15.12;
        }
        if case == "refilled" {
            player.breath.refill();
        }
        let actor = if case == "restored" {
            &mut restored
        } else {
            &mut alice
        };
        actor.water_appearance(&player);
        clear_background(Color::new(0.12, 0.2, 0.27, 1.));
        set_camera(&camera);
        crate::lighting::select(vec![], camera.position, &scene.world);
        let atmosphere = Atmosphere::default();
        crate::render_fx::begin_view(&camera, 2., &atmosphere, true);
        actor.atmosphere(&atmosphere, camera.position);
        actor.draw(player.feet, true);
        crate::render_fx::finish();
        set_default_camera();
        pictures.push(get_screen_data());
        crate::viewer::save_capture(&root.join(format!("{case}.png")))?;
        next_frame().await;
    }
    let changed = |a: usize, b: usize| {
        pictures[a]
            .bytes
            .chunks_exact(4)
            .zip(pictures[b].bytes.chunks_exact(4))
            .filter(|(x, y)| x != y)
            .count()
    };
    ensure!(changed(0, 1) > 100, "Swimming shell did not render");
    for i in [2, 3, 8] {
        ensure!(
            changed(1, i) == 0,
            "Shell pause/restore/refill differs: {i}"
        );
    }
    for i in [4, 5] {
        ensure!(changed(0, i) == 0, "Shell shown on land or in lava: {i}");
    }
    // Blended GPU channels can round one byte differently between batches;
    // permit that only at a handful of pixels, never moving particle geometry.
    let warning_rounding_only = pictures[6]
        .bytes
        .iter()
        .zip(&pictures[7].bytes)
        .all(|(a, b)| a.abs_diff(*b) <= 1);
    ensure!(
        changed(1, 6) > 20 && changed(6, 7) <= 4 && warning_rounding_only,
        "Low-air bubbles missing or advancing while paused"
    );
    // Also retain an ordinary scene with the normal collision-aware camera and liquid fog.
    player = saved_player;
    alice.water_appearance(&player);
    let (eye, target, show) =
        crate::camera::Follow::default().update(&scene.world, player.feet, Vec3::X, 128., 1. / 60.);
    ensure!(show, "Garden swimming follow camera hides Alice");
    let view = Camera3D {
        position: eye,
        target,
        up: Vec3::Z,
        z_near: 2.,
        z_far: 30000.,
        fovy: 75_f32.to_radians(),
        ..Default::default()
    };
    scene.atmosphere.liquid = Liquid::Water.tint();
    clear_background(BLACK);
    set_camera(&view);
    crate::render_fx::begin_view(&view, 2., &scene.atmosphere, false);
    scene.draw(eye, 2., false, false, &[]);
    alice.atmosphere(&scene.atmosphere, eye);
    alice.draw(player.feet, false);
    crate::render_fx::finish();
    set_default_camera();
    crate::viewer::save_capture(&root.join("garden-swimming.png"))?;
    next_frame().await;
    println!("PASS swimming shell: {} visible pixels; paused/restore/refill identical; absent without reward/on land/in lava; low-air bubbles visible and paused; actual Garden breathing", changed(0,1));
    Ok(())
}
impl Art {
    pub fn load(
        a: &mut Assets,
        rig: &Skeleton,
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let (mut def, model) =
            crate::weapons::read_model_clip(a, "mock_shell_alice", Some("shell_alice"))?;
        // Gameplay selects the original second skin; the first is the story's
        // entity-alpha fade and would leave an opaque shell during swimming.
        for skin in def.skins.values_mut() {
            *skin = "turtleshell".into();
        }
        // This timer TAN intentionally has no render geometry or tags.
        let warning = crate::skeletal::Definition::load(a, "models/fx_alicebreath.tik")?;
        let file = warning
            .animations
            .get("bubble_on")
            .context("Missing breath warning clip")?;
        let bytes = a.read(&format!("{}/{file}", warning.path))?;
        let header = crate::skeletal::Bytes(&bytes);
        anyhow::ensure!(
            header.span(0, 4)? == b"TAN " && header.int(4)? == 2,
            "Invalid breath timer"
        );
        let frames = header.count(72, 10000)?;
        let warning_duration = header.float(84)?;
        let warning_frame = warning_duration / frames.max(1) as f32;
        anyhow::ensure!(
            frames > 0 && (0.001..=1.).contains(&warning_frame),
            "Invalid breath timing"
        );
        let tag = |name: &str| {
            rig.bones
                .iter()
                .position(|b| b.name == name)
                .with_context(|| format!("Missing swimming attachment {name}"))
        };
        Ok(Self {
            shell: Prop::build(a, def, model, specs)?,
            bubbles: Attached::load(a, "fx_alicebreath", specs)?
                .context("Missing breath bubbles")?,
            events: crate::animation_events::Model::load(a, "models/fx_alicebreath.tik")?,
            warning_duration,
            warning_frame,
            back: tag("tag_back")?,
            mouth: tag("tag_mouth")?,
            visible: false,
            warning: None,
            view: RefCell::new((Atmosphere::default(), Vec3::ZERO)),
        })
    }
    pub fn reset(&mut self) {
        self.visible = false;
        self.warning = None;
        self.bubbles = self.bubbles.fork();
    }
    pub fn sync(&mut self, player: &Player) {
        self.visible = player.breath.shell
            && player.immersion.level >= 2
            && player.immersion.kind != crate::water::Liquid::Lava;
        let warning = (player.immersion.level == 3)
            .then(|| player.breath.warning_age())
            .flatten();
        if self.warning.is_some() && warning.is_none() {
            self.bubbles = self.bubbles.fork();
        }
        self.warning = warning;
    }
    pub fn atmosphere(&self, atmosphere: &Atmosphere, camera: Vec3) {
        *self.view.borrow_mut() = (atmosphere.clone(), camera);
    }
    pub fn draw(
        &mut self,
        pose: &[Transform],
        body: Transform,
        scale: f32,
        time: f32,
        fullbright: bool,
    ) {
        let tag = |index: usize| Transform {
            translation: body.point(pose[index].translation * scale),
            rotation: body.rotation * pose[index].rotation,
        };
        if self.visible {
            self.shell
                .draw_frame(tag(self.back), scale, fullbright, time, true);
        }
        if let Some(age) = self.warning {
            let mouth = tag(self.mouth);
            let (atmosphere, camera) = &*self.view.borrow();
            self.bubbles.draw(
                age,
                scale,
                |_, _| mouth,
                |name, birth, default| {
                    self.events
                        .visual(
                            "bubble_on",
                            birth,
                            self.warning_duration,
                            self.warning_frame,
                            true,
                        )
                        .emitters
                        .get(name)
                        .copied()
                        .unwrap_or(default)
                },
                *camera,
                atmosphere,
            );
        }
    }
}
