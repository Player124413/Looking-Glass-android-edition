//! Bounded Clockwork Automaton combat using supplied animation and projectile facts.
use crate::{
    ant::Timing,
    collision::World,
    combat::{self, DamageKind, Feedback, Hit, Opponents, Recoil, Target},
    skeletal::Transform,
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
#[cfg(test)]
mod tests;
pub const MODEL: &str = "c_clockwork";
pub const HALF: Vec3 = vec3(32., 32., 43.);
pub const CLIPS: &[&str] = &[
    "idle",
    "twitch",
    "alert01",
    "alert02",
    "idle_base_2_ready",
    "ready",
    "walk_slow",
    "walk_norm",
    "walk_fast",
    "attack_punch",
    "range_fist_ready",
    "range_fist_fire",
    "range_fist_back",
    "range_steam_ready",
    "range_steam_fire",
    "range_steam_back",
    "pain01",
    "pain02",
    "pain03",
    "death01",
    "death02",
    "death_frozen",
];
const STEP: f32 = 1. / 120.;
pub trait Rig: Timing {
    /// Animated model-local tag, including the definition's setup scale.
    fn tag(&self, clip: &str, time: f32, tag: &str) -> Transform;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    Idle,
    Twitch,
    Wake,
    Ready,
    Walk,
    Fast,
    Punch,
    FistReady,
    FistFire,
    FistBack,
    SteamReady,
    SteamFire,
    SteamBack,
    Pain,
    Dead,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Fist {
    pub serial: u32,
    pub position: Vec3,
    pub direction: Vec3,
    pub age: f32,
    pub victim: usize,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Impact {
    pub position: Vec3,
    pub age: f32,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Automaton {
    pub active: bool,
    pub spawn_delay: Option<f32>,
    pub feet: Vec3,
    pub yaw: f32,
    pub scale: f32,
    pub health: f32,
    pub phase: Phase,
    pub time: f32,
    pub variant: usize,
    pub frozen: bool,
    pub fists: Vec<Fist>,
    pub impacts: Vec<Impact>,
    pub opponents: Opponents,
    #[serde(skip)]
    pub notarget: bool,
    contacts: u8,
    steam_hits: u8,
    repeat: bool,
    recoil: Recoil,
    accumulator: f32,
    falling: f32,
    memory: f32,
    last_seen: Vec3,
    pain: f32,
    cooldown: f32,
    random: u32,
    serial: u32,
}
impl Automaton {
    pub fn new(feet: Vec3, yaw: f32, scale: f32, seed: usize, active: bool) -> Self {
        Self {
            active,
            spawn_delay: None,
            feet,
            yaw,
            scale,
            health: 400.,
            phase: Phase::Idle,
            time: 0.,
            variant: 0,
            frozen: false,
            fists: Vec::new(),
            impacts: Vec::new(),
            opponents: Default::default(),
            notarget: false,
            contacts: 0,
            steam_hits: 0,
            repeat: false,
            recoil: Default::default(),
            accumulator: 0.,
            falling: 0.,
            memory: 0.,
            last_seen: feet,
            pain: 0.,
            cooldown: 0.,
            random: seed as u32,
            serial: 0,
        }
    }
    pub fn target(&self, id: usize) -> Target {
        Target {
            id,
            center: self.feet + Vec3::Z * HALF.z * self.scale,
            half: HALF * self.scale,
        }
    }
    pub fn clip(&self) -> &'static str {
        match self.phase {
            Phase::Idle => "idle",
            Phase::Twitch => "twitch",
            Phase::Wake => "idle_base_2_ready",
            Phase::Ready => "ready",
            Phase::Walk => "walk_slow",
            Phase::Fast => "walk_norm",
            Phase::Punch => "attack_punch",
            Phase::FistReady => "range_fist_ready",
            Phase::FistFire => "range_fist_fire",
            Phase::FistBack => "range_fist_back",
            Phase::SteamReady => "range_steam_ready",
            Phase::SteamFire => "range_steam_fire",
            Phase::SteamBack => "range_steam_back",
            Phase::Pain => ["pain01", "pain02", "pain03"][self.variant],
            Phase::Dead if self.frozen => "death_frozen",
            Phase::Dead => ["death01", "death02"][self.variant % 2],
        }
    }
    pub fn loops(&self) -> bool {
        matches!(
            self.phase,
            Phase::Idle | Phase::Ready | Phase::Walk | Phase::Fast
        )
    }
    pub fn visual(&self, data: &impl Timing) -> Option<Transform> {
        if !self.active {
            return None;
        }
        let sink = if self.health > 0. {
            0.
        } else {
            (self.time - data.duration(MODEL, self.clip()) - 5.).max(0.) / 2.
        };
        (sink < 1.).then_some(Transform {
            translation: self.feet - Vec3::Z * sink * 110. * self.scale,
            rotation: Quat::from_rotation_z(self.yaw),
        })
    }
    fn set(&mut self, phase: Phase) {
        self.phase = phase;
        self.time = 0.;
        self.contacts = 0;
        self.steam_hits = 0;
    }
    fn random(&mut self) -> u32 {
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        self.random
    }
    pub fn hit(&mut self, hit: Hit) -> Option<&'static str> {
        if !self.active || self.health <= 0. || !hit.damage.is_finite() || hit.damage <= 0. {
            return None;
        }
        self.health = (self.health - hit.damage).max(0.);
        self.opponents.demon |= hit.kind.is_demon();
        self.recoil.hit(hit.knockback, 500.);
        self.pain = (self.pain + hit.damage).min(400.);
        self.memory = 3.;
        self.last_seen = self.feet - hit.knockback.normalize_or_zero() * 100.;
        if self.health == 0. {
            let side = vec2(-self.yaw.sin(), self.yaw.cos()).dot(hit.knockback.truncate());
            self.variant = usize::from(side > 0.);
            self.frozen = hit.kind.means() == DamageKind::Ice;
            self.set(Phase::Dead);
            return Some(
                [
                    "sound/character/clockwork/clk_death01.wav",
                    "sound/character/clockwork/clk_death02.wav",
                ][self.variant],
            );
        }
        if self.pain >= 65. && self.phase != Phase::Pain {
            self.pain = 0.;
            self.variant = (self.random() >> 16) as usize % 3;
            self.set(Phase::Pain);
            return Some(
                [
                    "sound/character/clockwork/clk_pain01.wav",
                    "sound/character/clockwork/clk_pain02.wav",
                    "sound/character/clockwork/clk_pain03.wav",
                ][self.variant],
            );
        }
        None
    }
    pub fn update(
        &mut self,
        dt: f32,
        world: &World,
        eye: Vec3,
        data: &impl Rig,
        out: &mut Feedback,
    ) {
        if !dt.is_finite() || dt <= 0. || (!self.active && self.spawn_delay.is_none()) {
            return;
        }
        self.accumulator += dt.min(0.1);
        while self.accumulator + 0.000001 >= STEP {
            self.accumulator = (self.accumulator - STEP).max(0.);
            if let Some(delay) = &mut self.spawn_delay {
                *delay = (*delay - STEP).max(0.);
                if *delay == 0. {
                    self.active = true;
                    self.spawn_delay = None;
                }
                continue;
            }
            self.step(world, eye, data, out);
        }
    }
    fn projectiles(&mut self, world: &World, eye: Vec3, out: &mut Feedback) {
        for impact in &mut self.impacts {
            impact.age += STEP;
        }
        self.impacts.retain(|i| i.age < 0.25);
        let bodies = self.opponents.bodies(eye);
        self.fists.retain_mut(|f| {
            if let Some(target) = bodies.iter().find(|t| t.id == f.victim) {
                let desired = (target.center - f.position).normalize_or_zero();
                if desired.length_squared() > 0.5 {
                    // A bounded turn rate keeps the missile dodgeable; seeker units are approximated.
                    let angle = f.direction.dot(desired).clamp(-1., 1.).acos();
                    if angle > 0.00001 {
                        let q = Quat::from_rotation_arc(f.direction, desired);
                        f.direction = (Quat::IDENTITY
                            .slerp(q, (120_f32.to_radians() * STEP / angle).min(1.))
                            * f.direction)
                            .normalize();
                    }
                }
            }
            let end = f.position + f.direction * 500. * STEP;
            let hit = combat::contact(
                &combat::Context {
                    world,
                    targets: &bodies,
                },
                f.position,
                end,
                8.,
            );
            let wall = world.sweep(f.position, end, Vec3::splat(8.));
            f.position = f.position.lerp(end, hit.map_or(wall.fraction, |(_, t)| t));
            f.age += STEP;
            if let Some((id, _)) = hit {
                out.strike(id, 10., f.direction * 150., DamageKind::Other);
            }
            if hit.is_some() || wall.start_solid || wall.fraction < 1. {
                if self.impacts.len() < 8 {
                    self.impacts.push(Impact {
                        position: f.position,
                        age: 0.,
                    });
                }
                return false;
            }
            f.age < 5.
        });
    }
    fn tag(&self, data: &impl Rig, tag: &str, at: f32) -> Transform {
        let local = data.tag(self.clip(), at, tag);
        let rotation = Quat::from_rotation_z(self.yaw);
        Transform {
            translation: self.feet + rotation * local.translation * self.scale,
            rotation: rotation * local.rotation,
        }
    }
    fn steam(&mut self, world: &World, eye: Vec3, data: &impl Rig, out: &mut Feedback) {
        let origin = self.tag(data, "tag_steam", self.time).translation;
        let forward = vec3(self.yaw.cos(), self.yaw.sin(), 0.);
        for victim in self.opponents.bodies(eye) {
            let bit = if victim.id == crate::dice::SUMMON {
                2
            } else {
                1
            };
            if self.steam_hits & bit != 0 || (victim.id == crate::dice::ALICE && self.notarget) {
                continue;
            }
            let delta = victim.center - origin;
            let along = delta.dot(forward);
            let side = (delta - forward * along).truncate().length();
            let clear = along >= 0.
                && along <= 320. * self.scale
                && side <= 16. * self.scale + along * 0.18 + victim.half.x
                && delta.z.abs() < 24. * self.scale + victim.half.z
                && {
                    let t = world.sweep(origin, victim.center, Vec3::splat(0.5));
                    !t.start_solid && t.fraction >= 1.
                };
            if clear {
                self.steam_hits |= bit;
                out.strike(victim.id, 12., forward * 25., DamageKind::Other);
            }
        }
    }
    fn step(&mut self, world: &World, player_eye: Vec3, data: &impl Rig, out: &mut Feedback) {
        self.projectiles(world, player_eye, out);
        let duration = data.duration(MODEL, self.clip());
        if self.phase == Phase::Dead && self.time >= duration + 7. {
            return;
        }
        self.time += STEP;
        self.cooldown = (self.cooldown - STEP).max(0.);
        self.feet += self.recoil.step(STEP, world, self.target(0));
        self.falling = (self.falling + 800. * STEP).min(800.);
        let body = self.target(0);
        let down = world.sweep(
            body.center,
            body.center - Vec3::Z * self.falling * STEP,
            body.half,
        );
        if !down.start_solid {
            self.feet.z -= self.falling * STEP * down.fraction;
        }
        if down.start_solid || down.fraction < 1. {
            self.falling = 0.;
        }
        if self.phase == Phase::Dead {
            self.time = self.time.min(duration + 7.);
            return;
        }
        let committed = matches!(
            self.phase,
            Phase::Punch
                | Phase::FistReady
                | Phase::FistFire
                | Phase::SteamReady
                | Phase::SteamFire
        );
        let (eye, notarget, victim) =
            self.opponents
                .aim(world, body.center, player_eye, self.notarget, committed);
        let delta = eye - body.center;
        let distance = delta.truncate().length();
        let visible = !notarget && delta.length() < 1000. && {
            let t = world.sweep(body.center, eye, Vec3::splat(0.5));
            !t.start_solid && t.fraction >= 1.
        };
        if visible {
            self.memory = 3.;
            self.last_seen = eye;
        } else {
            self.memory = (self.memory - STEP).max(0.);
        }
        if notarget {
            self.memory = 0.;
        }
        let pursuing = visible || self.memory > 0.;
        let toward = if visible {
            delta
        } else {
            self.last_seen - body.center
        };
        if pursuing && self.phase != Phase::Pain {
            let angle = (toward.y.atan2(toward.x) - self.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw += angle.clamp(-6. * STEP, 6. * STEP);
        }
        let facing =
            vec2(self.yaw.cos(), self.yaw.sin()).dot(delta.truncate().normalize_or_zero()) > 0.65;
        let close =
            visible && facing && distance <= 130. * self.scale && delta.z.abs() < 75. * self.scale;
        match self.phase {
            Phase::Punch => {
                for (i, frame) in [9., 12., 16.].into_iter().enumerate() {
                    if self.contacts & (1 << i) == 0
                        && self.time >= frame * data.frame(MODEL, self.clip())
                    {
                        self.contacts |= 1 << i;
                        if close {
                            out.strike(
                                victim,
                                5.,
                                delta.normalize_or_zero() * 30.,
                                DamageKind::Other,
                            );
                        }
                    }
                }
                if self.time >= duration {
                    self.set(Phase::Ready);
                    self.cooldown = 0.35;
                }
            }
            Phase::FistReady if self.time >= duration => {
                self.set(Phase::FistFire);
                out.spatial_sounds
                    .push(("sound/character/clockwork/clk_projectile.wav", self.feet));
            }
            Phase::FistFire => {
                for (i, (frame, tag)) in [(2., "tag_left_hand"), (12., "tag_right_hand")]
                    .into_iter()
                    .enumerate()
                {
                    if self.contacts & (1 << i) == 0
                        && self.time >= frame * data.frame(MODEL, self.clip())
                    {
                        self.contacts |= 1 << i;
                        let from = self
                            .tag(data, tag, frame * data.frame(MODEL, self.clip()))
                            .translation;
                        let direction = (eye - Vec3::Z * 10. - from).normalize_or_zero();
                        let clear = world.sweep(body.center, from, Vec3::splat(8.));
                        if visible
                            && facing
                            && self.fists.len() < 8
                            && !clear.start_solid
                            && clear.fraction >= 1.
                            && direction.length_squared() > 0.5
                        {
                            self.serial = self.serial.wrapping_add(1);
                            self.fists.push(Fist {
                                serial: self.serial,
                                position: from,
                                direction,
                                age: 0.,
                                victim,
                            });
                        }
                    }
                }
                if self.time >= duration {
                    if !self.repeat && visible && self.random() % 100 < 15 {
                        self.repeat = true;
                        self.set(Phase::FistFire);
                    } else {
                        self.set(Phase::FistBack);
                    }
                }
            }
            Phase::SteamReady if self.time >= duration => {
                self.set(Phase::SteamFire);
                out.spatial_sounds
                    .push(("sound/character/clockwork/clk_steam_start.wav", self.feet));
            }
            Phase::SteamFire => {
                self.steam(world, player_eye, data, out);
                if self.time >= duration {
                    if !self.repeat && visible && self.random() % 100 < 15 {
                        self.repeat = true;
                        self.set(Phase::SteamFire);
                    } else {
                        self.set(Phase::SteamBack);
                    }
                }
            }
            Phase::FistBack | Phase::SteamBack if self.time >= duration => {
                self.set(Phase::Ready);
                self.cooldown = 1.;
            }
            Phase::Wake | Phase::Pain if self.time >= duration => self.set(Phase::Ready),
            Phase::Twitch if self.time >= duration => self.set(Phase::Idle),
            Phase::Idle | Phase::Twitch if pursuing => {
                self.set(Phase::Wake);
                out.spatial_sounds
                    .push(("sound/character/clockwork/clk_ready.wav", self.feet));
            }
            Phase::Ready | Phase::Walk | Phase::Fast if pursuing => {
                if close && self.cooldown == 0. {
                    self.set(Phase::Punch);
                    out.spatial_sounds
                        .push(("sound/character/clockwork/clk_attack_punch.wav", self.feet));
                } else if visible
                    && facing
                    && distance > 225. * self.scale
                    && self.cooldown == 0.
                    && self.time >= duration
                {
                    self.repeat = false;
                    let steam = distance < 325. * self.scale && self.random() % 100 < 75;
                    self.set(if steam {
                        Phase::SteamReady
                    } else {
                        Phase::FistReady
                    });
                    out.spatial_sounds
                        .push(("sound/character/clockwork/clk_steam_ready.wav", self.feet));
                } else if !close {
                    let phase = if distance > 250. * self.scale {
                        Phase::Fast
                    } else {
                        Phase::Walk
                    };
                    if !matches!(self.phase, Phase::Walk | Phase::Fast) {
                        self.set(phase);
                    } else {
                        self.phase = phase;
                    }
                    if toward.truncate().length() > 24. {
                        self.walk(world, toward, data.speed(MODEL, self.clip()) * STEP);
                    } else if !visible {
                        self.memory = 0.;
                    }
                    self.time = self.time.min(10.);
                } else {
                    self.time = self.time.min(10.);
                }
            }
            Phase::Ready | Phase::Walk | Phase::Fast => self.set(Phase::Idle),
            Phase::Idle if self.time >= 16. => {
                if self.random() % 5 == 0 {
                    self.set(Phase::Twitch);
                } else {
                    self.time = 0.;
                }
            }
            _ => {}
        }
    }
    fn walk(&mut self, world: &World, delta: Vec3, step: f32) {
        for angle in [0., 0.65, -0.65, 1.1, -1.1] {
            let next = combat::walk_body(
                world,
                self.feet,
                Quat::from_rotation_z(angle)
                    * delta.truncate().extend(0.).normalize_or_zero()
                    * step
                    * self.scale,
                HALF * self.scale,
            );
            if next.truncate().distance_squared(self.feet.truncate()) > 0.0001 {
                self.feet = next;
                break;
            }
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.variant < 3
                && self.feet.is_finite()
                && self.feet.abs().max_element() < 100000.
                && self.yaw.is_finite()
                && (0.01..=10.).contains(&self.scale)
                && (0. ..=400.).contains(&self.health)
                && (self.health == 0.) == (self.phase == Phase::Dead)
                && (!self.frozen || self.health == 0.)
                && self
                    .spawn_delay
                    .is_none_or(|d| !self.active && (0. ..=30.).contains(&d))
                && (self.active
                    || (self.health == 400.
                        && self.phase == Phase::Idle
                        && self.fists.is_empty()
                        && self.impacts.is_empty()))
                && self.contacts < 8
                && self.steam_hits < 4
                && self.recoil.valid()
                && self.last_seen.is_finite()
                && (0. ..=3.).contains(&self.memory)
                && (0. ..=400.).contains(&self.pain)
                && (0. ..=1.).contains(&self.cooldown)
                && (0. ..=800.).contains(&self.falling)
                && (0. ..=0.01).contains(&self.accumulator)
                && (0. ..=30.).contains(&self.time)
                && self.fists.len() <= 8
                && self.impacts.len() <= 8,
            "Invalid saved Clockwork Automaton"
        );
        let mut ids = std::collections::BTreeSet::new();
        ensure!(
            self.fists.iter().all(|f| ids.insert(f.serial)
                && f.position.is_finite()
                && f.position.abs().max_element() < 100000.
                && f.direction.is_finite()
                && (f.direction.length_squared() - 1.).abs() < 0.01
                && (0. ..=5.).contains(&f.age)
                && matches!(f.victim, crate::dice::ALICE | crate::dice::SUMMON))
                && self
                    .impacts
                    .iter()
                    .all(|i| i.position.is_finite() && (0. ..=0.25).contains(&i.age)),
            "Invalid saved Clockwork effects"
        );
        Ok(())
    }
}
