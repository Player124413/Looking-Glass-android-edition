//! School-two's one-use gym lever and authored extending bleachers.
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    interaction::vector,
    movement::Player,
    skeletal::Transform,
    weapons::Prop,
};
use anyhow::{Context, Result};
use macroquad::prelude::*;
use std::collections::BTreeMap;

pub fn supported(e: &BTreeMap<String, String>) -> bool {
    e.get("classname").is_some_and(|c| c == "script_object")
        && e.get("targetname")
            .and_then(|n| n.strip_prefix("bleach"))
            .and_then(|n| n.parse::<usize>().ok())
            .is_some_and(|n| (1..=12).contains(&n))
}
struct Step {
    model: usize,
    base: Vec3,
    origin: Vec3,
    distance: f32,
    duration: f32,
    time: f32,
    collider: Collider,
}
pub struct Gym {
    steps: Vec<Step>,
    lever: Transform,
    pub used: bool,
    time: f32,
    sound_pending: bool,
}
impl Gym {
    pub fn load(map: &Bsp) -> Result<Self> {
        let mut steps = Vec::new();
        for e in map.entities.iter().filter(|e| supported(e)) {
            let n = e["targetname"]
                .trim_start_matches("bleach")
                .parse::<usize>()?;
            let model = e["model"].trim_start_matches('*').parse()?;
            let base = vector(&e["origin"]).context("Invalid gym bleacher")?;
            steps.push(Step {
                model,
                base,
                origin: base,
                distance: (n - 1) as f32 * 20.,
                duration: (n - 1) as f32 * 0.5,
                time: 0.,
                collider: Collider::model(map, model, base, Quat::IDENTITY, true)?,
            });
        }
        anyhow::ensure!(steps.len() == 12, "Missing gym bleachers");
        let e = map
            .entities
            .iter()
            .find(|e| e.get("move_thread").is_some_and(|s| s == "extendBleachers"))
            .context("Missing gym lever")?;
        Ok(Self {
            steps,
            lever: Transform {
                translation: vector(&e["origin"]).context("Invalid gym lever")?,
                rotation: Quat::from_rotation_z(
                    e.get("angle")
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(0.)
                        .to_radians(),
                ),
            },
            used: false,
            time: 0.,
            sound_pending: false,
        })
    }
    fn reachable(&self, world: &World, eye: Vec3, aim: Vec3) -> bool {
        let point = self.lever.point(vec3(36., 0., 26.));
        let delta = point - eye;
        let trace = world.sweep(eye, point, Vec3::splat(0.5));
        delta.length_squared() < 125. * 125.
            && delta.normalize_or_zero().dot(aim) > 0.6
            && !trace.start_solid
            && trace.fraction >= 1.
    }
    pub fn prompt(&self, world: &World, eye: Vec3, aim: Vec3) -> Option<&'static str> {
        self.reachable(world, eye, aim).then_some(if self.used {
            "Gym lever activated"
        } else {
            "E  extend gym bleachers"
        })
    }
    pub fn activate(&mut self, world: &World, eye: Vec3, aim: Vec3) -> bool {
        if self.used || !self.reachable(world, eye, aim) {
            return false;
        }
        self.used = true;
        true
    }
    pub fn colliders(&self) -> impl Iterator<Item = Collider> + '_ {
        self.steps.iter().map(|s| s.collider.clone())
    }
    pub fn transforms(&self) -> impl Iterator<Item = (usize, Vec3, Quat)> + '_ {
        self.steps
            .iter()
            .map(|s| (s.model, s.origin, Quat::IDENTITY))
    }
    pub fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. || !self.used {
            return Ok(());
        }
        let mut remaining = dt.min(0.1);
        while remaining > 0.000001 {
            let dt = remaining.min(1. / 120.);
            remaining -= dt;
            if self.time < 1. && self.time + dt >= 1. { self.sound_pending = true; }
            self.time += dt;
            for i in 0..self.steps.len() {
                let s = &self.steps[i];
                if s.duration == 0. || s.time >= s.duration + 1. {
                    continue;
                }
                let time = (s.time + dt).min(s.duration + 1.);
                let origin =
                    s.base - Vec3::X * s.distance * ((time - 1.) / s.duration).clamp(0., 1.);
                let next = Collider::model(map, s.model, origin, Quat::IDENTITY, true)?;
                let ground = s.collider.trace(
                    player.feet + PLAYER_CENTER,
                    player.feet + PLAYER_CENTER - Vec3::Z * 3.,
                    PLAYER_HALF,
                );
                let riding = player.velocity.z <= 1.
                    && !ground.start_solid
                    && ground.fraction < 1.
                    && ground.normal.z > 0.65;
                world.set_dynamic(
                    fixed
                        .iter()
                        .cloned()
                        .chain(
                            self.steps
                                .iter()
                                .enumerate()
                                .filter(|(j, _)| *j != i)
                                .map(|(_, s)| s.collider.clone()),
                        )
                        .collect(),
                );
                let carried = player.feet + origin - s.origin;
                if riding {
                    let trace = world.body_trace(player.feet, carried);
                    if trace.start_solid || trace.fraction < 1. || !world.body_clear(carried) {
                        continue;
                    }
                } else if next.touches(
                    player.feet + PLAYER_CENTER,
                    player.feet + PLAYER_CENTER,
                    PLAYER_HALF,
                ) {
                    continue;
                }
                let s = &mut self.steps[i];
                s.time = time;
                s.origin = origin;
                s.collider = next;
                if riding {
                    player.feet = carried;
                    player.grounded = true;
                }
            }
        }
        world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        Ok(())
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    used: bool,
    time: f32,
    steps: Vec<(usize, Vec3, f32)>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    sound_pending: bool,
}
impl Gym {
    pub fn take_sound(&mut self) -> Option<&'static str> {
        std::mem::take(&mut self.sound_pending).then_some("sound/world/mover/bleacher.wav")
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            used: self.used,
            time: self.time,
            sound_pending: self.sound_pending,
            steps: self
                .steps
                .iter()
                .map(|s| (s.model, s.origin, s.time))
                .collect(),
        }
    }
    pub fn restore(&mut self, s: &Snapshot, map: &Bsp) -> Result<()> {
        anyhow::ensure!(
            self.steps.len() == s.steps.len(),
            "Saved gym does not match map"
        );
        for (step, (model, origin, time)) in self.steps.iter_mut().zip(&s.steps) {
            anyhow::ensure!(
                step.model == *model && *time >= 0.,
                "Invalid saved bleacher"
            );
            step.origin = *origin;
            step.time = *time;
            step.collider = Collider::model(map, step.model, *origin, Quat::IDENTITY, true)?;
        }
        self.used = s.used;
        self.time = s.time;
        self.sound_pending = s.sound_pending;
        Ok(())
    }
}
pub struct Art {
    rest: Prop,
    pull: Prop,
    material: crate::character::SkinMaterial,
}
impl Art {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(assets)?;
        Ok(Self {
            rest: Prop::load_animation(assets, "lever", "start", &specs)?,
            pull: Prop::load_animation(assets, "lever", "move", &specs)?,
            material: crate::character::skin_material()?,
        })
    }
    pub fn draw(
        &mut self,
        gym: &Gym,
        fullbright: bool,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
    ) {
        self.material.atmosphere(atmosphere, camera);
        self.material.bind();
        if gym.used {
            self.pull
                .draw_frame(gym.lever, 1., fullbright, gym.time, false);
        } else {
            self.rest.draw(gym.lever, 1., fullbright);
        }
        gl_use_default_material();
    }
}
pub fn check(assets: &mut Assets) -> Result<()> {
    use crate::movement::{Controls, FixedClock};
    let map = Bsp::parse(&assets.read("maps/skool2.bsp")?)?;
    for fps in [30, 60, 144] {
        let mut world = World::from_bsp(&map)?;
        let mut gym = Gym::load(&map)?;
        world.set_dynamic(gym.colliders().collect());
        let mut approach = Player::spawn(&world, gym.lever.translation + vec3(-45., 0., 48.))
            .context("Gym lever approach is obstructed")?;
        for _ in 0..240 {
            approach.tick(&world, Controls::default());
        }
        anyhow::ensure!(approach.grounded, "Lever approach has no footing");
        let eye = approach.eye();
        println!("Gym lever standing approach {:?}", approach.feet);
        anyhow::ensure!(
            !gym.activate(&world, eye, -Vec3::X),
            "Lever activated while facing away"
        );
        anyhow::ensure!(
            gym.activate(
                &world,
                eye,
                (gym.lever.point(vec3(36., 0., 26.)) - eye).normalize()
            ),
            "Lever unreachable"
        );
        let mut player = Player::new(vec3(2100., -2560., 0.));
        gym.advance(0., &map, &mut world, &mut player, &[])?;
        anyhow::ensure!(
            gym.steps.iter().all(|s| s.origin == s.base),
            "Paused bleachers moved"
        );
        for _ in 0..fps * 8 {
            gym.advance(1. / fps as f32, &map, &mut world, &mut player, &[])?;
        }
        let pending = gym.snapshot();
        gym.restore(&pending, &map)?;
        anyhow::ensure!(gym.take_sound() == Some("sound/world/mover/bleacher.wav") && gym.take_sound().is_none(), "Bleacher movement cue lost or duplicated after save");
        let drained = gym.snapshot();
        gym.restore(&drained, &map)?;
        gym.advance(0.1, &map, &mut world, &mut player, &[])?;
        anyhow::ensure!(gym.take_sound().is_none(), "Completed bleachers replayed movement cue");
        anyhow::ensure!(
            gym.steps
                .iter()
                .all(|s| (s.origin - (s.base - Vec3::X * s.distance)).length() < 0.01),
            "Bleachers failed to extend"
        );
        // Real geometry: approach the fully extended lowest step and walk up the tiers.
        player = Player::new(vec3(2180., -2560., 16.1));
        let mut clock = FixedClock::default();
        for _ in 0..fps * 2 {
            clock.advance(
                1. / fps as f64,
                &world,
                &mut player,
                Controls {
                    wish: Vec2::X,
                    ..Default::default()
                },
            );
            anyhow::ensure!(world.body_clear(player.feet), "Bleachers embedded player");
        }
        anyhow::ensure!(
            player.feet.z >= 190.,
            "Cannot climb bleachers: {:?}",
            player.feet
        );
        println!(
            "PASS gym lever, pause, extension and physical staircase climb at {fps} Hz: {:?}",
            player.feet
        );
    }
    let mut world = World::from_bsp(&map)?;
    let mut gym = Gym::load(&map)?;
    gym.used = true;
    world.set_dynamic(gym.colliders().collect());
    let mut blocker = Player::new(vec3(2240., -2560., 0.1));
    anyhow::ensure!(world.body_clear(blocker.feet), "Blocked test start");
    for _ in 0..600 {
        gym.advance(1. / 60., &map, &mut world, &mut blocker, &[])?;
        anyhow::ensure!(
            world.body_clear(blocker.feet),
            "Bleachers crushed stationary Alice"
        );
    }
    anyhow::ensure!(
        gym.steps
            .iter()
            .any(|s| s.time < s.duration + 1. && s.duration > 0.),
        "Bleachers did not stop for obstruction"
    );
    blocker.feet = vec3(2100., -2560., 0.);
    for _ in 0..600 {
        gym.advance(1. / 60., &map, &mut world, &mut blocker, &[])?;
    }
    anyhow::ensure!(
        gym.steps
            .iter()
            .all(|s| (s.origin - (s.base - Vec3::X * s.distance)).length() < 0.01),
        "Bleachers did not resume"
    );
    println!("PASS gym obstruction stops and resumes without crushing Alice");
    Ok(())
}
