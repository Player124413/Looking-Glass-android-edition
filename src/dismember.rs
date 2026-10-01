//! Saved, bounded detached pieces using the Club Guard's authored knife deaths.
use crate::{collision::World, skeletal::Transform};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
pub mod burst;
pub mod card;
pub const CUT_FRAME_TIME: f32 = 0.05;
pub const LIFE: f32 = 5.;
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Variant {
    #[default]
    Torso,
    Fold,
    Head,
}
impl Variant {
    pub const ALL: [Self; 3] = [Self::Torso, Self::Fold, Self::Head];
    pub fn clip(self) -> &'static str {
        match self {
            Self::Torso => "death_3a",
            Self::Fold => "death_3b",
            Self::Head => "death_3c",
        }
    }
    pub fn is_default(&self) -> bool {
        *self == Self::Torso
    }
    /// Stable variation across placements; no render-clock or global RNG state.
    pub fn choose(home: Vec3, attacks: u32) -> Self {
        let mut seed = home.x.to_bits()
            ^ home.y.to_bits().rotate_left(11)
            ^ home.z.to_bits().rotate_left(22)
            ^ attacks.wrapping_mul(0x9e3779b9);
        seed ^= seed >> 16;
        seed = seed.wrapping_mul(0x7feb352d);
        seed ^= seed >> 15;
        seed = seed.wrapping_mul(0x846ca68b);
        seed ^= seed >> 16;
        Self::ALL[(seed % 3) as usize]
    }
}
/// Immutable authored recipe. More actors can supply another recipe while
/// keeping the same saved-piece physics, masking and event timeline.
pub struct Recipe {
    pub events: crate::animation_events::Model,
    pub duration: f32,
    pub frame_time: f32,
    pub split: f32,
    min: Vec3,
    max: Vec3,
}
pub fn club_recipes(assets: &mut crate::assets::Assets) -> Result<&'static [Recipe; 3]> {
    static RECIPES: std::sync::OnceLock<[Recipe; 3]> = std::sync::OnceLock::new();
    if let Some(recipes) = RECIPES.get() {
        return Ok(recipes);
    }
    let recipes = [
        load_recipe(assets, Variant::Torso.clip())?,
        load_recipe(assets, Variant::Fold.clip())?,
        load_recipe(assets, Variant::Head.clip())?,
    ];
    let _ = RECIPES.set(recipes);
    Ok(RECIPES.get().unwrap())
}
fn load_recipe(assets: &mut crate::assets::Assets, clip: &str) -> Result<Recipe> {
    actor_recipe(assets, "cardguard_club", clip)
}
/// Reviewed actor death recipes use the same bounded detached-piece physics.
pub fn actor_recipe(assets: &mut crate::assets::Assets, model: &str, clip: &str) -> Result<Recipe> {
    use crate::{
        animation_events::{Command, When},
        skeletal::{Animation, Definition, Skeleton},
    };
    let path = format!("models/{model}.tik");
    let def = Definition::load(assets, &path)?;
    let skeleton = Skeleton::parse(&assets.read(&format!("{}/{}", def.path, def.model))?)?;
    let events = crate::animation_events::Model::load(assets, &path)?;
    let death = Animation::parse(
        &assets.read(&format!("{}/{}", def.path, def.animations[clip]))?,
        skeleton.bones.len(),
    )?;
    let (frame, cap, name, pattern) = events.clips[clip]
        .iter()
        .find_map(|e| match (&e.when, &e.command) {
            (
                When::Frame(n),
                Command::Gib {
                    cap,
                    animation,
                    surfaces,
                    ..
                },
            ) => Some((*n, cap, animation, surfaces)),
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("Missing Club Guard cut recipe"))?;
    let top = Animation::parse(
        &assets.read(&format!("{}/{}", def.path, def.animations[name]))?,
        skeleton.bones.len(),
    )?;
    let (mut min, mut max) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
    // Include intermediate rotation samples, and padding. The conservative box
    // contains the complete authored fall instead of only the upright torso.
    for i in 0..top.frames.len() * 4 {
        let pose = skeleton.global_pose(&top.sample(i as f32 * top.frame_time / 4., false));
        for v in skeleton
            .surfaces
            .iter()
            .filter(|s| s.name == *cap || crate::animation_events::matches(pattern, &s.name))
            .flat_map(|s| &s.vertices)
        {
            let p = v.position(&pose) * def.scale;
            min = min.min(p);
            max = max.max(p);
        }
    }
    ensure!(min.is_finite() && max.is_finite(), "Empty sever surfaces");
    let recipe = Recipe {
        split: frame as f32 * death.frame_time,
        duration: death.duration(),
        frame_time: death.frame_time,
        events,
        min: min - Vec3::ONE,
        max: max + Vec3::ONE,
    };
    Ok(recipe)
}
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub severed: bool,
    pub age: f32,
    pub fragment: Option<Fragment>,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Fragment {
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub scale: f32,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
}
impl State {
    pub fn validate(&self, dead: bool) -> Result<()> {
        ensure!(
            self.age.is_finite()
                && (0. ..=LIFE).contains(&self.age)
                && (!self.severed || dead)
                && (self.severed || self.fragment.is_none()),
            "Invalid saved dismemberment"
        );
        if let Some(f) = &self.fragment {
            ensure!(
                self.age < LIFE
                    && f.position.is_finite()
                    && f.position.abs().max_element() < 100000.
                    && f.velocity.is_finite()
                    && f.velocity.length() < 2000.
                    && f.yaw.is_finite()
                    && f.bounds_min.is_finite()
                    && f.bounds_max.is_finite()
                    && f.bounds_min.cmplt(f.bounds_max).all()
                    && (f.bounds_max - f.bounds_min).max_element() < 4000.
                    && (0.01..=10.).contains(&f.scale),
                "Invalid saved detached piece"
            );
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        dt: f32,
        death_time: f32,
        feet: Vec3,
        yaw: f32,
        scale: f32,
        world: &World,
        recipe: Option<&Recipe>,
    ) {
        if dt <= 0. {
            return;
        }
        if !self.severed && death_time >= recipe.map_or(CUT_FRAME_TIME, |r| r.split) {
            self.severed = true;
            let (mut min, mut max) = (-vec3(24., 24., 32.) * scale, vec3(24., 24., 32.) * scale);
            if let Some(r) = recipe {
                min = Vec3::splat(f32::INFINITY);
                max = Vec3::splat(f32::NEG_INFINITY);
                for x in [r.min.x, r.max.x] {
                    for y in [r.min.y, r.max.y] {
                        for z in [r.min.z, r.max.z] {
                            let p = Quat::from_rotation_z(yaw) * vec3(x, y, z) * scale
                                - Vec3::Z * 48. * scale;
                            min = min.min(p);
                            max = max.max(p);
                        }
                    }
                }
            }
            let mut position = feet + Vec3::Z * 48. * scale;
            let (offset, half) = ((min + max) * 0.5, (max - min) * 0.5);
            if world
                .sweep(position + offset, position + offset, half)
                .start_solid
            {
                if let Some(clear) = world.actor_footing(position, offset, half, 0.) {
                    position = clear;
                } else {
                    return;
                }
            }
            self.fragment = Some(Fragment {
                position,
                velocity: vec3(yaw.cos() * 80., yaw.sin() * 80., 150.),
                yaw,
                scale,
                bounds_min: min,
                bounds_max: max,
            });
        }
        if !self.severed {
            return;
        }
        self.age = (self.age + dt).min(LIFE);
        if self.age >= LIFE {
            self.fragment = None;
            return;
        }
        if let Some(f) = &mut self.fragment {
            f.velocity.z -= 500. * dt;
            let end = f.position + f.velocity * dt;
            let offset = (f.bounds_min + f.bounds_max) * 0.5;
            let half = (f.bounds_max - f.bounds_min) * 0.5;
            let trace = world.sweep(f.position + offset, end + offset, half);
            if trace.start_solid {
                f.velocity = Vec3::ZERO;
                return;
            }
            f.position = f.position.lerp(end, trace.fraction);
            if trace.fraction < 1. {
                f.velocity -= trace.normal * f.velocity.dot(trace.normal).min(0.);
                f.velocity *= 0.45;
            }
        }
    }
    pub fn fade(&self) -> f32 {
        ((self.age - (LIFE - 1.)) / 1.).clamp(0., 1.)
    }
}
impl Fragment {
    pub fn transform(&self) -> Transform {
        Transform {
            translation: self.position - Vec3::Z * 48. * self.scale,
            rotation: Quat::from_rotation_z(self.yaw),
        }
    }
}
pub fn check(assets: &mut crate::assets::Assets) -> Result<()> {
    use crate::animation_events::{Command, Model, When};
    let events = Model::load(assets, "models/cardguard_club.tik")?;
    ensure!(events.clips["death_3a"].iter().any(|e| e.when==When::Frame(1) && matches!(&e.command,Command::Gib{cap,animation,surfaces,..} if cap=="cap_body" && animation=="death_top" && surfaces=="top*")),"Club Guard sever recipe differs from original");
    let text = String::from_utf8_lossy(&assets.read("ai/cardguard_club.st")?).into_owned();
    ensure!(
        text.contains("MOD \"knife\""),
        "Missing original knife death selection"
    );
    let def = crate::skeletal::Definition::load(assets, "models/cardguard_club.tik")?;
    let skeleton =
        crate::skeletal::Skeleton::parse(&assets.read(&format!("{}/{}", def.path, def.model))?)?;
    ensure!(
        skeleton.surfaces.iter().any(|s| s.name == "cap_body")
            && skeleton
                .surfaces
                .iter()
                .filter(|s| s.name.starts_with("top"))
                .count()
                == 4,
        "Missing damage surfaces"
    );
    for name in ["death_3a", "death_top"] {
        let clip = crate::skeletal::Animation::parse(
            &assets.read(&format!("{}/{}", def.path, def.animations[name]))?,
            skeleton.bones.len(),
        )?;
        ensure!(
            (clip.frame_time - CUT_FRAME_TIME).abs() < 0.0001,
            "Changed cut frame time"
        );
    }
    let cue = events.between(
        "death_3a",
        crate::animation_events::Span {
            start: 0.1,
            end: 0.2,
            duration: 2.,
            frame_time: 0.05,
            looping: false,
            entered: false,
        },
    );
    ensure!(cue.iter().any(|c|matches!(c,Command::Sound{path,..} if path=="sound/character/cardguard/club/death_3a.wav")),"Missing authored death cue");
    let timing = crate::npc::guard_timing(assets)?;
    let world = World::fixture(&[
        (vec3(-1000., -1000., -100.), vec3(1000., 1000., 0.)),
        (vec3(70., -1000., 0.), vec3(80., 1000., 500.)),
    ]);
    let mut g = crate::combat::Guard::new(vec3(0., 0., 0.1), 0., 1.);
    g.hurt_kind(100., crate::combat::DamageKind::Knife);
    g.cut_variant = Variant::Torso;
    ensure!(
        g.cut && g.health == 0. && g.clip() == "death_3a",
        "Knife did not select cut death"
    );
    let mut count = 0;
    for _ in 0..30 {
        count += g
            .advance(1. / 120., &world, vec3(0., 200., 48.), timing)
            .spatial_sounds
            .len();
    }
    ensure!(
        count == 1 && g.dismember.fragment.is_some(),
        "Cut event or sound count incorrect"
    );
    let saved = serde_json::to_vec(&g)?;
    g.advance(0., &world, Vec3::ZERO, timing);
    ensure!(saved == serde_json::to_vec(&g)?, "Paused guard changed");
    let mut restored: crate::combat::Guard = serde_json::from_slice(&saved)?;
    restored.validate_save()?;
    for _ in 0..700 {
        g.advance(1. / 120., &world, Vec3::ZERO, timing);
        restored.advance(1. / 120., &world, Vec3::ZERO, timing);
    }
    ensure!(
        serde_json::to_vec(&g)? == serde_json::to_vec(&restored)? && g.dismember.fragment.is_none(),
        "Restored piece differs or fails to expire"
    );
    let mut legacy = serde_json::to_value(crate::combat::Guard::new(Vec3::ZERO, 0., 1.))?;
    for key in ["cut", "dismember"] {
        legacy.as_object_mut().unwrap().remove(key);
    }
    serde_json::from_value::<crate::combat::Guard>(legacy)?.validate_save()?;
    let mut diamond = crate::combat::Guard::diamond(Vec3::ZERO, 0., 1.);
    diamond.hurt_kind(100., crate::combat::DamageKind::Knife);
    ensure!(!diamond.cut, "Unsupported actor was severed");
    let recipes = club_recipes(assets)?;
    let floor = World::fixture(&[(vec3(-1000., -1000., -100.), vec3(1000., 1000., 0.))]);
    for variant in Variant::ALL {
        let recipe = &recipes[variant as usize];
        let clip = variant.clip();
        let (cap, pattern) = if variant == Variant::Head {
            ("", "top_head")
        } else {
            ("cap_body", "top*")
        };
        ensure!(recipe.events.clips[clip].iter().any(|e| e.when == When::Frame(1)
            && matches!(&e.command, Command::Gib { cap: c, surfaces, .. } if c == cap && surfaces == pattern)), "Wrong cut surfaces for {clip}");
        let visual = recipe
            .events
            .visual(clip, 0.2, recipe.duration, recipe.frame_time, false);
        ensure!(
            visual
                .attachments
                .contains_key(if variant == Variant::Head {
                    "tag_gib_head"
                } else {
                    "tag_gib"
                }),
            "Missing cut effect"
        );
        ensure!(
            visual.removed.contains("tag_weapon") == (variant != Variant::Head),
            "Wrong staff visibility"
        );
        for hz in [30, 60, 144] {
            let mut guard = crate::combat::Guard::new(vec3(0., 0., 0.1), 0., 1.);
            guard.hurt_kind(100., crate::combat::DamageKind::Knife);
            guard.cut_variant = variant;
            let mut sounds = Vec::new();
            for _ in 0..hz / 2 {
                sounds.extend(
                    guard
                        .advance(1. / hz as f32, &floor, Vec3::ZERO, timing)
                        .spatial_sounds,
                );
            }
            ensure!(
                sounds.len() == 1 && sounds[0].0.ends_with(&format!("/{clip}.wav")),
                "Wrong sound timeline for {clip} at {hz} Hz"
            );
            ensure!(guard.dismember.fragment.is_some(), "Missing {clip} piece");
            let saved = serde_json::to_vec(&guard)?;
            guard.advance(0., &floor, Vec3::ZERO, timing);
            ensure!(saved == serde_json::to_vec(&guard)?, "Paused cut changed");
            let mut restored: crate::combat::Guard = serde_json::from_slice(&saved)?;
            restored.validate_save()?;
            ensure!(restored.clip() == clip, "Restored wrong knife death");
            for _ in 0..700 {
                guard.advance(1. / 120., &floor, Vec3::ZERO, timing);
                restored.advance(1. / 120., &floor, Vec3::ZERO, timing);
            }
            ensure!(
                serde_json::to_vec(&guard)? == serde_json::to_vec(&restored)?
                    && guard.dismember.fragment.is_none(),
                "Cut save future or cleanup differs"
            );
        }
        println!("PASS {clip}: original surfaces/effect/staff, single cue at 30/60/144 Hz, pause, saved choice, piece cleanup");
    }
    println!("PASS cut frame, single sound, swept piece, pause, old/new saves, continuation and finite cleanup");
    println!("PASS original Club Guard knife death, frame-one torso separation, four upper surfaces and cap material");
    Ok(())
}

pub async fn render_check(assets: &mut crate::assets::Assets) -> Result<()> {
    check(assets)?;
    for variant in Variant::ALL {
        render_variant(assets, variant).await?;
    }
    Ok(())
}
async fn render_variant(assets: &mut crate::assets::Assets, variant: Variant) -> Result<()> {
    let specs = crate::texture::read_materials(assets)?;
    let mut actor = crate::npc::Puppet::load(assets, "cardguard_club", &[], &specs)?;
    let timing = crate::npc::guard_timing(assets)?;
    let world = World::fixture(&[(vec3(-1000., -1000., -100.), vec3(1000., 1000., 0.))]);
    let atmosphere = crate::environment::Atmosphere::default();
    let eye = vec3(190., -250., 120.);
    let mut g = crate::combat::Guard::new(vec3(0., 0., 0.1), -0.7, 1.);
    g.place(&world);
    let intact = g.clone();
    g.hurt_kind(100., crate::combat::DamageKind::Knife);
    g.cut_variant = variant;
    let draw = |actor: &mut crate::npc::Puppet, g: &crate::combat::Guard| {
        clear_background(Color::new(0.035, 0.045, 0.06, 1.));
        set_camera(&Camera3D {
            position: eye,
            target: vec3(0., 0., 42.),
            up: Vec3::Z,
            fovy: 45_f32.to_radians(),
            z_near: 1.,
            z_far: 1200.,
            ..Default::default()
        });
        draw_cube(
            vec3(0., 0., -1.),
            vec3(600., 600., 2.),
            None,
            Color::new(0.16, 0.19, 0.22, 1.),
        );
        actor.draw_guard(g, true, eye, &atmosphere);
        gl_use_default_material();
    };
    for _ in 0..3 {
        draw(&mut actor, &intact);
        next_frame().await;
    }
    draw(&mut actor, &intact);
    let intact_pixels = get_screen_data().bytes;
    next_frame().await;
    for (label, until) in [
        ("intact", 0_f32),
        ("cut", 0.2),
        ("falling", 0.65),
        ("settled", 2.),
        ("fading", 4.6),
        ("cleaned", 5.2),
    ] {
        while g.time + 0.001 < until.min(LIFE + CUT_FRAME_TIME) {
            g.advance(1. / 120., &world, eye, timing);
        }
        for frame in 0..3 {
            draw(&mut actor, &g);
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/dismember-{}-{label}.png",
                    variant.clip()
                )))?;
            }
            next_frame().await;
        }
        let serialized = serde_json::to_vec(&g)?;
        draw(&mut actor, &g);
        let before = get_screen_data();
        next_frame().await;
        let restored = serde_json::from_slice(&serialized)?;
        draw(&mut actor, &restored);
        let after = get_screen_data();
        next_frame().await;
        ensure!(
            before.bytes == after.bytes,
            "Restored {label} render differs"
        );
        // A shared mesh mask or material must not damage the next intact actor.
        draw(&mut actor, &intact);
        let neighbor = get_screen_data();
        next_frame().await;
        draw(&mut actor, &intact);
        let repeat = get_screen_data();
        next_frame().await;
        ensure!(
            neighbor.bytes == repeat.bytes && neighbor.bytes == intact_pixels,
            "Shared actor rendering leaked"
        );
    }
    ensure!(
        g.dismember.fragment.is_none(),
        "Detached piece survived cleanup"
    );
    println!("PASS native {} intact/cut/falling/settled/fading/cleaned original surfaces, attachments, effects and matching saved render", variant.clip());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn knife_deaths_vary_by_guard_and_persist_without_affecting_other_deaths() {
        use crate::combat::{DamageKind, Guard};
        let mut seen = [false; 3];
        for i in 0..100 {
            let mut g = Guard::new(vec3(i as f32 * 64., 10., 0.), 0., 1.);
            let mut restored: Guard =
                serde_json::from_value(serde_json::to_value(&g).unwrap()).unwrap();
            g.hurt_kind(100., DamageKind::Knife);
            restored.hurt_kind(100., DamageKind::Knife);
            assert_eq!(g.cut_variant, restored.cut_variant);
            seen[g.cut_variant as usize] = true;
            let chosen = g.clip();
            g.hurt_kind(10., DamageKind::Knife);
            assert_eq!(chosen, g.clip());
            g.validate_save().unwrap();
            let corpse: Guard = serde_json::from_value(serde_json::to_value(&g).unwrap()).unwrap();
            assert_eq!(corpse.clip(), chosen);
        }
        assert_eq!(seen, [true; 3]);
        for kind in [DamageKind::Other, DamageKind::Ice, DamageKind::Cards] {
            let mut g = Guard::new(Vec3::ZERO, 0., 1.);
            g.hurt_kind(100., kind);
            assert!(!g.cut && g.cut_variant.is_default());
        }
        let mut legacy = Guard::new(Vec3::ZERO, 0., 1.);
        legacy.hurt_kind(100., DamageKind::Knife);
        legacy.cut_variant = Variant::Torso;
        let old = serde_json::to_value(&legacy).unwrap();
        assert!(old.get("cut_variant").is_none());
        let old: Guard = serde_json::from_value(old).unwrap();
        assert_eq!(old.clip(), "death_3a");
        old.validate_save().unwrap();
    }
    #[test]
    fn piece_obeys_walls_pause_restoration_and_cleanup_without_respawn() {
        let world = World::fixture(&[
            (vec3(-1000., -1000., -100.), vec3(1000., 1000., 0.)),
            (vec3(45., -1000., 0.), vec3(50., 1000., 500.)),
        ]);
        let mut s = State::default();
        s.update(0., 1., Vec3::ZERO, 0., 1., &world, None);
        assert!(!s.severed);
        for i in 0..120 {
            s.update(1. / 120., i as f32 / 120., Vec3::ZERO, 0., 1., &world, None);
        }
        assert!(s.fragment.as_ref().unwrap().position.x < 21.);
        let encoded = serde_json::to_vec(&s).unwrap();
        s.update(0., 2., Vec3::ZERO, 0., 1., &world, None);
        assert_eq!(encoded, serde_json::to_vec(&s).unwrap());
        let mut restored: State = serde_json::from_slice(&encoded).unwrap();
        for _ in 0..650 {
            for state in [&mut s, &mut restored] {
                state.update(1. / 120., 2., Vec3::ZERO, 0., 1., &world, None);
            }
        }
        assert_eq!(
            serde_json::to_vec(&s).unwrap(),
            serde_json::to_vec(&restored).unwrap()
        );
        assert!(s.severed && s.fragment.is_none());
        s.validate(true).unwrap();
        assert!(s.validate(false).is_err());
    }
}
