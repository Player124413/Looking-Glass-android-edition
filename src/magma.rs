//! Magma Man combat from supplied clip/cue data. Cooling time is an approximation:
//! the native MagmaMan class that drove STAGE is not present in the supplied scripts.
use crate::{
    clockwork::Rig,
    collision::World,
    combat::{self, DamageKind, Feedback, Hit, Opponents, Recoil, Target},
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
pub const MODEL: &str = "c_magmamen";
const DT: f32 = 1. / 120.;
pub const CLIPS: &[&str] = &[
    "idle1",
    "idle_2_ready",
    "ready",
    "rockready",
    "walk_1",
    "walk_2",
    "walk_3",
    "attack_rock",
    "attack_rock2",
    "attack_firespit",
    "attack_magmapunch",
    "attack_2b",
    "pain_rock1",
    "pain_rock2",
    "pain_rock3",
    "pain_mag1",
    "pain_mag2",
    "pain_mag3",
    "death",
    "death2",
    "death_rock",
    "death_frozen",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    Idle,
    Alert,
    Walk,
    Punch,
    Rock,
    Bash,
    Spit,
    Recover,
    Pain,
    Dead,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Magma {
    pub feet: Vec3,
    pub yaw: f32,
    pub scale: f32,
    pub health: f32,
    pub phase: Phase,
    pub time: f32,
    pub form: u8,
    pub cooling: f32,
    pub frozen: bool,
    pub variant: usize,
    pub opponents: Opponents,
    pub notarget: bool,
    pub shots: Vec<crate::snark::Shot>,
    /// A one-use source launch pad, resolved at placement creation, never on restore.
    pub launch: Option<Vec3>,
    velocity: Vec3,
    grounded: bool,
    launched: bool,
    cues: u8,
    cooldown: f32,
    pain: f32,
    random: u32,
    serial: u32,
    recoil: Recoil,
}
impl Magma {
    pub fn new(feet: Vec3, yaw: f32, scale: f32, seed: usize) -> Self {
        Self {
            feet,
            yaw,
            scale,
            health: 200.,
            phase: Phase::Idle,
            time: 0.,
            form: 0,
            cooling: 0.,
            frozen: false,
            variant: 0,
            opponents: Default::default(),
            notarget: false,
            shots: vec![],
            launch: None,
            velocity: Vec3::ZERO,
            grounded: false,
            launched: false,
            cues: 0,
            cooldown: 0.,
            pain: 0.,
            random: seed as u32,
            serial: 0,
            recoil: Default::default(),
        }
    }
    pub fn target(&self, id: usize) -> Target {
        Target {
            id,
            center: self.feet + Vec3::Z * 32. * self.scale,
            half: vec3(44., 44., 32.) * self.scale,
        }
    }
    pub fn place(&mut self, world: &World) {
        if !self.grounded && self.launch.is_none() {
            let b = self.target(0);
            if let Some(p) = world.actor_footing(self.feet, Vec3::Z * b.half.z, b.half, 1024.) {
                self.feet = p;
            }
            self.grounded = true;
        }
    }
    pub fn loops(&self) -> bool {
        matches!(self.phase, Phase::Idle | Phase::Walk)
    }
    pub fn clip(&self) -> &'static str {
        if self.frozen {
            return "death_frozen";
        }
        match self.phase {
            Phase::Idle => {
                if self.form == 2 {
                    "rockready"
                } else {
                    "idle1"
                }
            }
            Phase::Alert => "idle_2_ready",
            Phase::Walk => ["walk_1", "walk_2", "walk_3"][self.form as usize],
            Phase::Punch => "attack_magmapunch",
            Phase::Rock => "attack_rock",
            Phase::Bash => "attack_rock2",
            Phase::Spit => "attack_firespit",
            Phase::Recover => "attack_2b",
            Phase::Pain => {
                if self.form == 2 {
                    ["pain_rock1", "pain_rock2", "pain_rock3"][self.variant]
                } else {
                    ["pain_mag1", "pain_mag2", "pain_mag3"][self.variant]
                }
            }
            Phase::Dead => {
                if self.form == 2 {
                    "death_rock"
                } else {
                    ["death", "death2"][self.variant % 2]
                }
            }
        }
    }
    pub fn sample_time(&self, data: &impl Rig) -> f32 {
        if self.frozen {
            self.time.min(32. * data.frame(MODEL, self.clip()))
        } else {
            self.time
        }
    }
    pub fn visual_scale(&self, data: &impl Rig) -> f32 {
        self.scale
            * if self.health > 0. {
                1.
            } else {
                (1. - (self.time - data.duration(MODEL, self.clip())) / 1.4).clamp(0., 1.)
            }
    }
    pub fn set(&mut self, phase: Phase) {
        self.phase = phase;
        self.time = 0.;
        self.cues = 0;
    }
    pub fn hit(&mut self, hit: Hit) -> Option<&'static str> {
        if self.health <= 0.
            || !hit.damage.is_finite()
            || hit.damage <= 0.
            || hit.kind.means() == DamageKind::FireSword
        {
            return None;
        }
        self.opponents.demon |= hit.kind.is_demon();
        self.health = (self.health - hit.damage).max(0.);
        self.recoil.hit(hit.knockback, 200.);
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        self.variant = (self.random >> 16) as usize % 3;
        if self.health == 0. {
            self.frozen = hit.kind.means() == DamageKind::Ice;
            self.set(Phase::Dead);
            return Some(if self.frozen || (self.form < 2 && self.variant % 2 == 1) {
                "sound/character/magma_man/death2.wav"
            } else if self.form == 2 {
                "sound/character/magma_man/death_rock.wav"
            } else {
                "sound/character/magma_man/death.wav"
            });
        } else if hit.damage >= 25. && self.pain == 0. {
            self.pain = 0.4;
            self.set(Phase::Pain);
            return Some(if self.form == 2 {
                [
                    "sound/character/magma_man/twitcha.wav",
                    "sound/character/magma_man/pain_rock2.wav",
                    "sound/character/magma_man/pain_rock3.wav",
                ][self.variant]
            } else {
                [
                    "sound/character/magma_man/pain_mag1.wav",
                    "sound/character/magma_man/pain_mag2.wav",
                    "sound/character/magma_man/pain_mag3.wav",
                ][self.variant]
            });
        }
        None
    }
    fn shots(&mut self, world: &World, eye: Vec3, out: &mut Feedback) {
        let targets = self.opponents.bodies(eye);
        self.shots.retain_mut(|s| {
            let end = s.position + s.direction * 700. * DT;
            let hit = combat::contact(
                &combat::Context {
                    world,
                    targets: &targets,
                },
                s.position,
                end,
                4.,
            );
            let wall = world.sweep(s.position, end, Vec3::splat(4.));
            s.position = s.position.lerp(end, hit.map_or(wall.fraction, |(_, f)| f));
            s.age += DT;
            if let Some((id, _)) = hit {
                out.strike(id, 25., s.direction * 30., DamageKind::FireSword);
            }
            hit.is_none() && !wall.start_solid && wall.fraction >= 1. && s.age < 2.
        });
    }
    fn fall(&mut self, world: &World) {
        if !self.launched {
            if let Some(v) = self.launch {
                self.velocity = v;
            }
            self.launched = true;
        }
        self.velocity.z = (self.velocity.z - 800. * DT).max(-800.);
        let b = self.target(0);
        let delta = self.velocity * DT;
        let t = world.sweep(b.center, b.center + delta, b.half);
        if !t.start_solid {
            self.feet += delta * t.fraction;
        }
        if t.start_solid {
            self.velocity = Vec3::ZERO;
        } else if t.fraction < 1. {
            self.velocity -= t.normal * self.velocity.dot(t.normal).min(0.);
            if t.normal.z > 0.65 {
                self.velocity = Vec3::ZERO;
            }
        }
    }
    pub fn step(&mut self, world: &World, eye: Vec3, data: &impl Rig, out: &mut Feedback) {
        self.shots(world, eye, out);
        if self.health <= 0. {
            self.time = (self.time + DT).min(data.duration(MODEL, self.clip()) + 3.);
            return;
        }
        self.fall(world);
        let old_time = self.time;
        self.time += DT;
        if self.loops() && self.time > 3600. {
            self.time = 0.;
        }
        self.cooldown = (self.cooldown - DT).max(0.);
        self.pain = (self.pain - DT).max(0.);
        self.cooling = (self.cooling
            + if world.liquid_at(self.feet + Vec3::Z * 4.) & 8 != 0 {
                -4. * DT
            } else {
                DT
            })
        .clamp(0., 12.);
        if self.loops() {
            let form = (self.cooling / 6.).floor().min(2.) as u8;
            if form != self.form {
                self.form = form;
                self.time = 0.;
            }
        }
        let recoil = self.recoil.step(DT, world, self.target(0));
        self.feet = combat::walk_body_liquids(world, self.feet, recoil, self.target(0).half, 8);
        let b = self.target(0);
        let (aim, notarget, _) = self.opponents.aim(
            world,
            b.center,
            eye,
            self.notarget,
            !self.loops() && self.phase != Phase::Pain,
        );
        let delta = aim - b.center;
        let distance = delta.length();
        let trace = world.sweep(b.center, aim, Vec3::splat(0.5));
        let visible = !notarget && distance < 800. && !trace.start_solid && trace.fraction >= 1.;
        if visible && matches!(self.phase, Phase::Idle | Phase::Walk | Phase::Alert) {
            let angle = (delta.y.atan2(delta.x) - self.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw =
                (self.yaw + angle.clamp(-2.5 * DT, 2.5 * DT)).rem_euclid(std::f32::consts::TAU);
        }
        let facing = delta.truncate().length_squared() < 1.
            || vec2(self.yaw.cos(), self.yaw.sin()).dot(delta.truncate().normalize_or_zero()) > 0.8;
        let duration = data.duration(MODEL, self.clip());
        if old_time == 0. {
            if let Some(sound) = match self.phase {
                Phase::Rock => Some("sound/character/magma_man/attack_rock.wav"),
                Phase::Bash => Some("sound/character/magma_man/attack_rock2.wav"),
                Phase::Recover => Some("sound/character/magma_man/attack_2b.wav"),
                _ => None,
            } {
                out.sounds.push(sound);
            }
        }
        if self.phase == Phase::Spit {
            let cue = 2. * data.frame(MODEL, self.clip());
            if old_time < cue && self.time >= cue {
                out.sounds
                    .push("sound/character/magma_man/attack_firespit.wav");
            }
        }
        match self.phase {
            Phase::Idle => {
                if visible {
                    self.set(Phase::Alert);
                }
            }
            Phase::Alert | Phase::Pain | Phase::Recover => {
                if self.time >= duration {
                    self.set(Phase::Walk);
                    self.cooldown = 0.25;
                }
            }
            Phase::Walk => {
                if !visible {
                    self.set(Phase::Idle);
                    return;
                }
                if facing
                    && self.cooldown == 0.
                    && distance < if self.form == 2 { 120. } else { 150. } * self.scale
                {
                    self.set(if self.form == 2 {
                        Phase::Rock
                    } else {
                        Phase::Punch
                    });
                } else if self.form < 2 && facing && self.cooldown == 0. && self.time >= duration {
                    self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
                    self.set(if self.random % 10 < 3 || distance < 250. {
                        Phase::Punch
                    } else {
                        Phase::Spit
                    });
                } else {
                    let motion = delta.with_z(0.).normalize_or_zero()
                        * data.speed(MODEL, self.clip()).min(320.)
                        * self.scale
                        * DT;
                    self.feet =
                        combat::walk_body_liquids(world, self.feet, motion, self.target(0).half, 8);
                }
            }
            Phase::Punch | Phase::Rock | Phase::Bash => {
                let phase = self.phase;
                let events: &[(f32, f32, f32, f32)] = match phase {
                    Phase::Rock => &[(11., 112., 10., 15.)],
                    Phase::Bash => &[(13., 145., 145., 40.)],
                    _ => &[
                        (10., 64., 24., 10.),
                        (11., 128., 24., 10.),
                        (12., 256., 24., 10.),
                        (13., 384., 24., 10.),
                        (14., 512., 24., 10.),
                        (15., 768., 24., 10.),
                        (16., 800., 24., 10.),
                    ],
                };
                for (i, &(frame, range, width, damage)) in events.iter().enumerate() {
                    if self.cues & (1 << i) != 0
                        || self.time < frame * data.frame(MODEL, self.clip())
                    {
                        continue;
                    }
                    self.cues |= 1 << i;
                    if i == 0 && phase == Phase::Punch {
                        out.sounds
                            .push("sound/character/magma_man/attack_magmapunch.wav");
                    }
                    let direction = vec3(self.yaw.cos(), self.yaw.sin(), 0.);
                    let targets = self.opponents.bodies(eye);
                    let hits = if phase == Phase::Bash {
                        targets
                            .iter()
                            .filter(|t| {
                                t.center.distance(b.center) < range * self.scale && {
                                    let tr = world.sweep(b.center, t.center, Vec3::splat(0.5));
                                    !tr.start_solid && tr.fraction >= 1.
                                }
                            })
                            .map(|t| t.id)
                            .collect::<Vec<_>>()
                    } else {
                        combat::contact_box(
                            &combat::Context {
                                world,
                                targets: &targets,
                            },
                            b.center,
                            b.center + direction * range * self.scale,
                            vec3(width, width, if phase == Phase::Punch { 32. } else { 10. })
                                * self.scale,
                        )
                        .map(|(id, _)| id)
                        .into_iter()
                        .collect()
                    };
                    for id in hits {
                        out.strike(
                            id,
                            damage,
                            direction * 60.,
                            if phase == Phase::Punch {
                                DamageKind::Fire
                            } else {
                                DamageKind::Other
                            },
                        );
                        if phase == Phase::Punch {
                            self.set(Phase::Recover);
                            break;
                        }
                    }
                    if self.phase != phase {
                        break;
                    }
                }
                if self.phase == phase && self.time >= duration {
                    self.set(if phase == Phase::Punch {
                        Phase::Recover
                    } else {
                        Phase::Walk
                    });
                    self.cooldown = 0.25;
                }
            }
            Phase::Spit => {
                let frame = 10. * data.frame(MODEL, self.clip());
                if self.cues == 0 && self.time >= frame {
                    self.cues = 1;
                    let position = self.feet
                        + Quat::from_rotation_z(self.yaw)
                            * data.tag(self.clip(), frame, "tag_mouth").translation
                            * self.scale;
                    let direction = (aim - position).normalize_or_zero();
                    let muzzle = world.sweep(b.center, position, Vec3::splat(4.));
                    if !notarget
                        && direction.length_squared() > 0.5
                        && !muzzle.start_solid
                        && muzzle.fraction >= 1.
                        && self.shots.len() < 4
                    {
                        self.serial = self.serial.wrapping_add(1);
                        self.shots.push(crate::snark::Shot {
                            serial: self.serial,
                            position,
                            direction,
                            age: 0.,
                        });
                    }
                }
                if self.time >= duration {
                    self.set(Phase::Walk);
                    self.cooldown = 0.35;
                }
            }
            Phase::Dead => {}
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.feet.is_finite()
                && self.yaw.is_finite()
                && (0.01..=10.).contains(&self.scale)
                && (0. ..=200.).contains(&self.health)
                && (0. ..=86400.).contains(&self.time)
                && self.form < 3
                && (0. ..=12.).contains(&self.cooling)
                && self.variant < 3
                && (0. ..=0.4).contains(&self.cooldown)
                && (0. ..=0.4).contains(&self.pain)
                && self.recoil.valid()
                && self.velocity.is_finite()
                && self.velocity.length() <= 4000.
                && self
                    .launch
                    .is_none_or(|v| v.is_finite() && v.length() <= 4000.)
                && self.shots.len() <= 4,
            "Invalid Magma Man state"
        );
        ensure!(
            (self.health == 0.) == (self.phase == Phase::Dead)
                && (!self.frozen || self.health == 0.),
            "Invalid Magma death"
        );
        let mut ids = std::collections::BTreeSet::new();
        for s in &self.shots {
            ensure!(
                ids.insert(s.serial)
                    && s.position.is_finite()
                    && s.direction.is_finite()
                    && (s.direction.length() - 1.).abs() < 0.01
                    && (0. ..=2.).contains(&s.age),
                "Invalid Magma fireball"
            );
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests;
