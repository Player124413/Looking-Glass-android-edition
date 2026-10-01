//! Fire Imp combat derived from the user's supplied model and state declarations.
use crate::{
    ant::Timing,
    collision::World,
    combat::{DamageKind, Feedback, Hit, Opponents, Recoil, Target},
    skeletal::Transform,
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
#[cfg(test)]
mod tests;

pub const MODEL: &str = "c_fireimp";
pub const CLIPS: &[&str] = &[
    "idle01",
    "twitch",
    "alert01",
    "alert02",
    "walk01",
    "run",
    "attack01",
    "pain01",
    "pain02",
    "death01",
    "death02",
    "death_frozen",
    "gib01",
];
pub const HALF: Vec3 = vec3(16., 16., 25.);
const STEP: f32 = 1. / 120.;
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    Idle,
    Twitch,
    Alert,
    Walk,
    Run,
    Attack,
    Pain,
    Dead,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Fragment {
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Quat,
    pub spin: Vec3,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Imp {
    pub active: bool,
    pub spawn_delay: Option<f32>,
    /// One-use authored launch, retained while the ambush is dormant.
    #[serde(default)]
    pub launch: Option<Vec3>,
    #[serde(default)]
    flight: Option<Vec3>,
    pub feet: Vec3,
    pub yaw: f32,
    pub scale: f32,
    pub health: f32,
    pub phase: Phase,
    pub time: f32,
    pub variant: usize,
    pub frozen: bool,
    pub gibbed: bool,
    /// Exactly four chunks and the fork, retired together after five seconds.
    pub fragments: Option<[Fragment; 5]>,
    pub opponents: Opponents,
    #[serde(skip)]
    pub notarget: bool,
    fired: bool,
    sounded: bool,
    recoil: Recoil,
    accumulator: f32,
    falling: f32,
    memory: f32,
    last_seen: Vec3,
    pain: f32,
    random: u32,
}
impl Imp {
    pub fn new(feet: Vec3, yaw: f32, scale: f32, seed: usize, active: bool) -> Self {
        Self {
            active,
            spawn_delay: None,
            launch: None,
            flight: None,
            feet,
            yaw,
            scale,
            health: 35.,
            phase: Phase::Idle,
            time: 0.,
            variant: 0,
            frozen: false,
            gibbed: false,
            fragments: None,
            opponents: Opponents::default(),
            notarget: false,
            fired: false,
            sounded: false,
            recoil: Recoil::default(),
            accumulator: 0.,
            falling: 0.,
            memory: 0.,
            last_seen: feet,
            pain: 0.,
            random: seed as u32,
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
            Phase::Idle => "idle01",
            Phase::Twitch => "twitch",
            Phase::Alert => ["alert01", "alert02"][self.variant],
            Phase::Walk => "walk01",
            Phase::Run => "run",
            Phase::Attack => "attack01",
            Phase::Pain => ["pain01", "pain02"][self.variant],
            Phase::Dead if self.frozen => "death_frozen",
            Phase::Dead if self.gibbed => "gib01",
            Phase::Dead => ["death01", "death02"][self.variant],
        }
    }
    pub fn loops(&self) -> bool {
        matches!(self.phase, Phase::Idle | Phase::Walk | Phase::Run)
    }
    pub fn visual_scale(&self, data: &impl Timing) -> f32 {
        if !self.active || self.gibbed {
            return 0.;
        }
        self.scale
            * if self.health > 0. {
                1.
            } else {
                (1. - (self.time - data.duration(MODEL, self.clip()) - 5.).max(0.) / 2.)
                    .clamp(0., 1.)
            }
    }
    fn set(&mut self, phase: Phase) {
        self.phase = phase;
        self.time = 0.;
        self.fired = false;
        self.sounded = false;
    }
    fn random(&mut self) -> u32 {
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        self.random
    }
    pub fn hit(&mut self, hit: Hit) -> Option<&'static str> {
        if !self.active || self.health <= 0. || !hit.damage.is_finite() || hit.damage <= 0. {
            return None;
        }
        // The source immunity is firesword, not every fire/explosion weapon.
        if hit.kind.means() == DamageKind::FireSword {
            return None;
        }
        self.opponents.demon |= hit.kind.is_demon();
        self.health = (self.health - hit.damage).max(0.);
        self.recoil.hit(hit.knockback, 100.);
        self.memory = 3.;
        self.last_seen = self.feet - hit.knockback.normalize_or_zero() * 100.;
        self.pain = (self.pain + hit.damage).min(35.);
        if self.health == 0. {
            self.variant = (self.random() >> 16) as usize % 2;
            self.frozen = hit.kind.means() == DamageKind::Ice;
            self.gibbed = !matches!(hit.kind.means(), DamageKind::Ice | DamageKind::Knife)
                && self.random() % 100 < 15;
            self.set(Phase::Dead);
            if self.gibbed {
                self.fragments = Some(std::array::from_fn(|i| {
                    let angle = (self.random() % 6283) as f32 / 1000.;
                    Fragment {
                        position: self.feet + Vec3::Z * (20. + i as f32 * 3.) * self.scale,
                        velocity: vec3(
                            angle.cos() * 100.,
                            angle.sin() * 100.,
                            160. + (self.random() % 220) as f32,
                        ),
                        rotation: Quat::from_rotation_z(angle),
                        spin: vec3(1.2, 2.1, angle - 3.),
                    }
                }));
            }
            return Some(
                [
                    "sound/character/fireimp/death01.wav",
                    "sound/character/fireimp/death02.wav",
                ][self.variant],
            );
        }
        if self.pain >= 25. && self.phase != Phase::Pain {
            self.pain = 0.;
            self.variant = (self.random() >> 16) as usize % 2;
            self.set(Phase::Pain);
            return Some(
                [
                    "sound/character/fireimp/pain01.wav",
                    "sound/character/fireimp/pain02.wav",
                ][self.variant],
            );
        }
        None
    }
    /// Start the detached fork at the actual skeletal attachment, once at the lethal hit.
    pub fn fork_origin(&mut self, transform: Transform) {
        if let Some(parts) = &mut self.fragments {
            parts[4].position = transform.translation;
            parts[4].rotation = transform.rotation;
        }
    }
    pub fn update(
        &mut self,
        dt: f32,
        world: &World,
        eye: Vec3,
        data: &impl Timing,
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
    fn step(&mut self, world: &World, eye: Vec3, data: &impl Timing, out: &mut Feedback) {
        // Retired corpses must not keep falling through the void indefinitely.
        if self.phase == Phase::Dead && self.time >= data.duration(MODEL, self.clip()) + 7. {
            return;
        }
        self.time += STEP;
        if let Some(v) = self.launch.take() { self.flight = Some(v); }
        if let Some(mut velocity) = self.flight {
            velocity.z -= 800. * STEP;
            let body = self.target(0);
            let tr = world.sweep(body.center, body.center + velocity * STEP, body.half);
            if !tr.start_solid { self.feet += velocity * STEP * tr.fraction; }
            if tr.start_solid || (tr.fraction < 1. && tr.normal.z > 0.65) {
                self.flight = None;
                self.falling = 0.;
            } else {
                if tr.fraction < 1. { velocity -= tr.normal * velocity.dot(tr.normal).min(0.); }
                self.flight = Some(velocity);
                self.time = self.time.min(1.);
                return;
            }
        }
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
        let duration = data.duration(MODEL, self.clip());
        if self.phase == Phase::Dead {
            if self.time >= 5. {
                self.fragments = None;
            }
            if let Some(parts) = &mut self.fragments {
                for part in parts {
                    part.velocity.z -= 650. * STEP;
                    let end = part.position + part.velocity * STEP;
                    let trace = world.sweep(part.position, end, Vec3::splat(2. * self.scale));
                    if trace.start_solid {
                        part.velocity = Vec3::ZERO;
                        continue;
                    }
                    part.position = part.position.lerp(end, trace.fraction);
                    if trace.fraction < 1. {
                        part.velocity -=
                            trace.normal * 1.6 * part.velocity.dot(trace.normal).min(0.);
                        part.velocity *= 0.8;
                    }
                    part.rotation = (part.rotation
                        * Quat::from_euler(
                            EulerRot::XYZ,
                            part.spin.x * STEP,
                            part.spin.y * STEP,
                            part.spin.z * STEP,
                        ))
                    .normalize();
                }
            }
            self.time = self.time.min(duration + 7.);
            return;
        }
        let (eye, notarget, victim) = self.opponents.aim(
            world,
            body.center,
            eye,
            self.notarget,
            self.phase == Phase::Attack,
        );
        let delta = eye - body.center;
        let distance = delta.truncate().length();
        let sight = world.sweep(body.center, eye, Vec3::splat(0.5));
        let visible =
            !notarget && delta.length() < 800. && !sight.start_solid && sight.fraction >= 1.;
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
            let turn = (toward.y.atan2(toward.x) - self.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw += turn.clamp(-10. * STEP, 10. * STEP);
        }
        let facing =
            vec2(self.yaw.cos(), self.yaw.sin()).dot(delta.truncate().normalize_or_zero()) > 0.6;
        let in_reach =
            visible && facing && distance <= 90. * self.scale && delta.z.abs() < 65. * self.scale;
        match self.phase {
            Phase::Attack => {
                if !self.sounded && self.time >= data.frame(MODEL, self.clip()) * 10. {
                    self.sounded = true;
                    out.spatial_sounds
                        .push(("sound/character/fireimp/attack01.wav", self.feet));
                }
                if !self.fired && self.time >= data.frame(MODEL, self.clip()) * 12. {
                    self.fired = true;
                    if in_reach {
                        out.strike(
                            victim,
                            10.,
                            delta.normalize_or_zero() * 30.,
                            DamageKind::Other,
                        );
                    }
                }
                if self.time >= duration {
                    self.set(Phase::Walk);
                }
            }
            Phase::Pain | Phase::Alert if self.time >= duration => self.set(Phase::Walk),
            Phase::Twitch if self.time >= duration => self.set(Phase::Idle),
            Phase::Idle | Phase::Twitch if pursuing => {
                self.variant = (self.random() >> 16) as usize % 2;
                self.set(Phase::Alert);
                out.spatial_sounds.push((
                    [
                        "sound/character/fireimp/alert01.wav",
                        "sound/character/fireimp/alert02.wav",
                    ][self.variant],
                    self.feet,
                ));
            }
            Phase::Walk | Phase::Run if pursuing => {
                if in_reach {
                    self.set(Phase::Attack);
                } else {
                    let phase = if toward.truncate().length() > 160. * self.scale {
                        Phase::Run
                    } else {
                        Phase::Walk
                    };
                    if self.phase != phase {
                        self.set(phase);
                    }
                    if toward.truncate().length() > 24. {
                        let step = data.speed(MODEL, self.clip()) * STEP * self.scale;
                        for angle in [0., 0.65, -0.65, 1.1, -1.1] {
                            let next = crate::combat::walk_body(
                                world,
                                self.feet,
                                Quat::from_rotation_z(angle)
                                    * toward.truncate().extend(0.).normalize_or_zero()
                                    * step,
                                HALF * self.scale,
                            );
                            if next.truncate().distance_squared(self.feet.truncate()) > 0.0001 {
                                self.feet = next;
                                break;
                            }
                        }
                    } else if !visible {
                        self.memory = 0.;
                    }
                    if self.time >= duration {
                        self.time %= duration;
                    }
                }
            }
            Phase::Walk | Phase::Run => self.set(Phase::Idle),
            Phase::Idle if self.time >= 13. => {
                if self.random() % 3 == 0 {
                    self.set(Phase::Twitch);
                    out.spatial_sounds
                        .push(("sound/character/fireimp/twitch.wav", self.feet));
                } else {
                    self.time = 0.;
                }
            }
            _ => {}
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.variant < 2
                && self.feet.is_finite()
                && self.feet.abs().max_element() < 100000.
                && self.yaw.is_finite()
                && (0.01..=10.).contains(&self.scale)
                && (0. ..=35.).contains(&self.health)
                && (self.health == 0.) == (self.phase == Phase::Dead)
                && (!(self.frozen || self.gibbed) || self.health == 0.)
                && !(self.frozen && self.gibbed)
                && (self.active || (self.health == 35. && self.phase == Phase::Idle))
                && self
                    .spawn_delay
                    .is_none_or(|d| !self.active && (0. ..=30.).contains(&d))
                && self.recoil.valid()
                && self.launch.into_iter().chain(self.flight).all(|v| v.is_finite() && v.length() < 4000.)
                && self.last_seen.is_finite()
                && (0. ..=3.).contains(&self.memory)
                && (0. ..=35.).contains(&self.pain)
                && (0. ..=800.).contains(&self.falling)
                && (0. ..=0.01).contains(&self.accumulator)
                && (0. ..=30.).contains(&self.time),
            "Invalid saved Fire Imp"
        );
        if let Some(parts) = &self.fragments {
            ensure!(
                self.gibbed
                    && self.time < 5.
                    && parts.iter().all(|p| p.position.is_finite()
                        && p.position.abs().max_element() < 100000.
                        && p.velocity.is_finite()
                        && p.velocity.length() < 4000.
                        && p.rotation.is_finite()
                        && (p.rotation.length_squared() - 1.).abs() < 0.01
                        && p.spin.is_finite()
                        && p.spin.length() < 10.),
                "Invalid Fire Imp debris"
            );
        }
        Ok(())
    }
}
