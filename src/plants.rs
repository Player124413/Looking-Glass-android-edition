//! Stationary plant enemies. Source clips/contact frames; bounded replacement for Digest.
use crate::{
    clockwork::Rig,
    collision::World,
    combat::{self, DamageKind, Feedback, Hit, Opponents, Target},
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
#[cfg(test)]
mod tests;
pub const ROSE_CLIPS: &[&str] = &[
    "idle_1",
    "idle_2",
    "idle_3",
    "alert_1",
    "alert_2",
    "attack_1",
    "attack_2",
    "attack_3",
    "pain1",
    "pain2",
    "pain3",
    "death2",
    "death3",
    "death_frozen",
];
pub const MUSHROOM_CLIPS: &[&str] = &[
    "idle_base",
    "twitch",
    "ready",
    "alert01",
    "alert02",
    "attack_spit",
    "attack_suffocate_close",
    "digest",
    "spit",
    "pain_center",
    "pain_left",
    "pain_right",
    "death01",
    "death02",
    "death_frozen",
];
const STEP: f32 = 1. / 120.;
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    Rose,
    Mushroom,
}
impl Kind {
    pub fn from_model(model: &str) -> Option<Self> {
        match model {
            "c_bloodrose" => Some(Self::Rose),
            "c_evilmushroom" => Some(Self::Mushroom),
            _ => None,
        }
    }
    pub fn model(self) -> &'static str {
        match self {
            Self::Rose => "c_bloodrose",
            Self::Mushroom => "c_evilmushroom",
        }
    }
    pub fn clips(self) -> &'static [&'static str] {
        match self {
            Self::Rose => ROSE_CLIPS,
            Self::Mushroom => MUSHROOM_CLIPS,
        }
    }
    pub fn health(self) -> f32 {
        match self {
            Self::Rose => 70.,
            Self::Mushroom => 125.,
        }
    }
    pub fn tag(self) -> &'static str {
        match self {
            Self::Rose => "tag_barrel",
            Self::Mushroom => "tag_skull",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    Dormant,
    Grow,
    Ready,
    Melee,
    Spit,
    Fan,
    Suck,
    Grab,
    Digest,
    Release,
    Pain,
    Dead,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Shot {
    pub serial: u32,
    pub position: Vec3,
    pub direction: Vec3,
    pub age: f32,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Plant {
    pub kind: Kind,
    pub feet: Vec3,
    pub yaw: f32,
    pub scale: f32,
    pub health: f32,
    pub phase: Phase,
    pub time: f32,
    pub frozen: bool,
    pub variant: usize,
    pub opponents: Opponents,
    pub notarget: bool,
    pub shots: Vec<Shot>,
    growth: f32,
    contacts: u8,
    cooldown: f32,
    pain: f32,
    random: u32,
    serial: u32,
}
impl Plant {
    pub fn new(kind: Kind, feet: Vec3, yaw: f32, scale: f32, seed: usize) -> Self {
        Self {
            kind,
            feet,
            yaw,
            scale,
            health: kind.health(),
            phase: Phase::Dormant,
            time: 0.,
            frozen: false,
            variant: 0,
            opponents: Default::default(),
            notarget: false,
            shots: Vec::new(),
            growth: 0.,
            contacts: 0,
            cooldown: 0.,
            pain: 0.,
            random: seed as u32,
            serial: 0,
        }
    }
    pub fn grow_for_check(&mut self) {
        self.growth = 0.7;
    }
    pub fn vulnerable(&self) -> bool {
        self.phase != Phase::Dormant && self.health > 0.
    }
    pub fn target(&self, id: usize) -> Target {
        let (lo, hi) = if self.kind == Kind::Rose {
            (4., 300.)
        } else {
            (8., 178.)
        };
        let scale = self.scale * self.growth_scale();
        Target {
            id,
            center: self.feet + Vec3::Z * (lo + hi) * 0.5 * scale,
            half: vec3(48., 48., (hi - lo) * 0.5) * scale,
        }
    }
    fn growth_scale(&self) -> f32 {
        if self.kind == Kind::Rose {
            0.2 + 0.8 * (self.growth / 0.7).min(1.)
        } else {
            1.
        }
    }
    pub fn visual_scale(&self, data: &impl Rig) -> f32 {
        let fade = if self.health > 0. {
            1.
        } else if self.frozen {
            (3. - self.time).clamp(0., 1.)
        } else {
            1. - ((self.time - data.duration(self.kind.model(), self.clip())) / 1.4).clamp(0., 1.)
        };
        self.scale * self.growth_scale() * fade
    }
    pub fn sample_time(&self, data: &impl Rig) -> f32 {
        if self.frozen {
            self.time.min(
                data.frame(self.kind.model(), self.clip())
                    * if self.kind == Kind::Rose { 28. } else { 5. },
            )
        } else {
            self.time
        }
    }
    pub fn loops(&self) -> bool {
        matches!(
            self.phase,
            Phase::Dormant | Phase::Ready | Phase::Grow | Phase::Suck | Phase::Digest
        )
    }
    pub fn clip(&self) -> &'static str {
        if self.frozen {
            return "death_frozen";
        }
        match (self.kind, self.phase) {
            (Kind::Rose, Phase::Melee) => "attack_1",
            (Kind::Rose, Phase::Spit) => "attack_2",
            (Kind::Rose, Phase::Fan) => "attack_3",
            (Kind::Rose, Phase::Pain) => ["pain1", "pain2", "pain3"][self.variant],
            (Kind::Rose, Phase::Dead) => ["death2", "death3"][self.variant % 2],
            (Kind::Rose, _) => "idle_1",
            (Kind::Mushroom, Phase::Dormant) => "idle_base",
            (Kind::Mushroom, Phase::Spit) => "attack_spit",
            (Kind::Mushroom, Phase::Suck | Phase::Grab) => "attack_suffocate_close",
            (Kind::Mushroom, Phase::Digest) => "digest",
            (Kind::Mushroom, Phase::Release) => "spit",
            (Kind::Mushroom, Phase::Pain) => {
                ["pain_center", "pain_left", "pain_right"][self.variant]
            }
            (Kind::Mushroom, Phase::Dead) => ["death01", "death02"][self.variant % 2],
            (Kind::Mushroom, _) => "ready",
        }
    }
    pub fn set(&mut self, phase: Phase) {
        self.phase = phase;
        self.time = 0.;
        self.contacts = 0;
    }
    fn random(&mut self) -> u32 {
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        self.random
    }
    pub fn hit(&mut self, hit: Hit) -> Option<&'static str> {
        if !self.vulnerable() || !hit.damage.is_finite() || hit.damage <= 0. {
            return None;
        }
        self.health = (self.health - hit.damage).max(0.);
        self.opponents.demon |= hit.kind.is_demon();
        self.variant = (self.random() >> 16) as usize % 3;
        if self.health == 0. {
            self.frozen = hit.kind.means() == DamageKind::Ice;
            self.set(Phase::Dead);
            return Some(match self.kind {
                Kind::Rose => [
                    "sound/character/blood_rose/blr_death02.wav",
                    "sound/character/blood_rose/blr_death03.wav",
                ][self.variant % 2],
                Kind::Mushroom => [
                    "sound/character/evil_mushroom/death01.wav",
                    "sound/character/evil_mushroom/death02.wav",
                ][self.variant % 2],
            });
        }
        self.pain += hit.damage;
        if self.phase != Phase::Pain && (self.kind == Kind::Rose || self.pain >= 25.) {
            self.pain = 0.;
            self.set(Phase::Pain);
            self.cooldown = 0.5;
            return Some(match self.kind {
                Kind::Rose => [
                    "sound/character/blood_rose/blr_pain-01.wav",
                    "sound/character/blood_rose/blr_pain-02.wav",
                    "sound/character/blood_rose/blr_pain-03.wav",
                ][self.variant],
                Kind::Mushroom => [
                    "sound/character/evil_mushroom/pain_center.wav",
                    "sound/character/evil_mushroom/pain_left.wav",
                    "sound/character/evil_mushroom/pain_right.wav",
                ][self.variant],
            });
        }
        None
    }
    fn fire(&mut self, data: &impl Rig, aim: Vec3, frame: f32, yaw: f32) {
        if self.shots.len() >= 12 {
            return;
        }
        let local = data.tag(
            self.clip(),
            data.frame(self.kind.model(), self.clip()) * frame,
            self.kind.tag(),
        );
        let position = self.feet
            + Quat::from_rotation_z(self.yaw)
                * local.translation
                * self.scale
                * self.growth_scale();
        let direction =
            Quat::from_rotation_z(yaw.to_radians()) * (aim - position).normalize_or_zero();
        if direction.length_squared() < 0.5 {
            return;
        }
        self.serial = self.serial.wrapping_add(1);
        self.shots.push(Shot {
            serial: self.serial,
            position,
            direction,
            age: 0.,
        });
    }
    fn projectiles(&mut self, world: &World, eye: Vec3, out: &mut Feedback) {
        let targets = self.opponents.bodies(eye);
        self.shots.retain_mut(|s| {
            let end = s.position + s.direction * 600. * STEP;
            let hit = combat::contact(
                &combat::Context {
                    world,
                    targets: &targets,
                },
                s.position,
                end,
                16.,
            );
            let wall = world.sweep(s.position, end, Vec3::splat(16.));
            s.position = s.position.lerp(end, hit.map_or(wall.fraction, |(_, f)| f));
            s.age += STEP;
            if let Some((id, _)) = hit {
                out.strike(id, 10., s.direction * 30., DamageKind::Other);
            }
            hit.is_none() && !wall.start_solid && wall.fraction >= 1. && s.age < 5.
        });
    }
    pub fn step(&mut self, world: &World, eye: Vec3, data: &impl Rig, out: &mut Feedback) {
        self.projectiles(world, eye, out);
        if self.health <= 0. {
            self.time = (self.time + STEP).min(data.duration(self.kind.model(), self.clip()) + 3.);
            return;
        }
        self.time += STEP;
        if matches!(self.phase, Phase::Dormant | Phase::Ready) && self.time > 3600. {
            self.time = 0.;
        }
        self.cooldown = (self.cooldown - STEP).max(0.);
        if self.phase != Phase::Dormant {
            self.growth = (self.growth + STEP).min(0.7);
        }
        let body = self.target(0);
        let (aim, notarget, victim) = self.opponents.aim(
            world,
            body.center,
            eye,
            self.notarget,
            !matches!(
                self.phase,
                Phase::Dormant | Phase::Grow | Phase::Ready | Phase::Pain
            ),
        );
        let delta = aim - body.center;
        let distance = delta.truncate().length();
        let visible = !notarget && delta.length_squared() < 1000_f32.powi(2) && {
            let trace = world.sweep(body.center, aim, Vec3::splat(0.5));
            !trace.start_solid && trace.fraction >= 1.
        };
        if visible && self.phase != Phase::Pain {
            let angle = (delta.y.atan2(delta.x) - self.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw += angle.clamp(-5. * STEP, 5. * STEP);
        }
        let facing =
            vec2(self.yaw.cos(), self.yaw.sin()).dot(delta.truncate().normalize_or_zero()) > 0.6;
        let duration = data.duration(self.kind.model(), self.clip());
        match self.phase {
            Phase::Dormant => {
                if visible
                    && match self.kind {
                        Kind::Rose => (100. ..768.).contains(&distance),
                        Kind::Mushroom => distance < 256.,
                    }
                {
                    self.set(if self.kind == Kind::Rose {
                        Phase::Grow
                    } else {
                        Phase::Ready
                    });
                }
            }
            Phase::Grow => {
                if self.growth >= 0.7 {
                    self.set(Phase::Ready);
                }
            }
            Phase::Ready => {
                if visible && facing && self.cooldown == 0. {
                    let phase = if self.kind == Kind::Rose {
                        if distance < 225. {
                            Phase::Melee
                        } else if distance < 650. {
                            Phase::Spit
                        } else {
                            Phase::Fan
                        }
                    } else if distance < 100. {
                        Phase::Grab
                    } else if distance < 416. && self.random() & 1 == 0 {
                        Phase::Suck
                    } else {
                        Phase::Spit
                    };
                    self.set(phase);
                    out.sounds.push(match (self.kind, phase) {
                        (Kind::Rose, Phase::Melee) => {
                            "sound/character/blood_rose/blr_attack1-01.wav"
                        }
                        (Kind::Rose, Phase::Spit) => "sound/character/blood_rose/blr_attack2-1.wav",
                        (Kind::Rose, _) => "sound/character/blood_rose/blr_attack3-1.wav",
                        (Kind::Mushroom, Phase::Spit) => {
                            "sound/character/evil_mushroom/attack_spit.wav"
                        }
                        _ => "sound/character/evil_mushroom/attack_suffocate_close.wav",
                    });
                }
            }
            Phase::Melee => {
                if self.contacts == 0
                    && self.time >= 14. * data.frame(self.kind.model(), self.clip())
                {
                    self.contacts = 1;
                    if visible
                        && facing
                        && distance < 225. * self.scale
                        && delta.z.abs() < 150. * self.scale
                    {
                        out.strike(
                            victim,
                            25.,
                            delta.normalize_or_zero() * 60.,
                            DamageKind::Other,
                        );
                    }
                }
                if self.time >= duration {
                    self.set(Phase::Ready);
                    self.cooldown = 0.4;
                }
            }
            Phase::Spit | Phase::Fan => {
                let events: &[(f32, f32)] = if self.phase == Phase::Fan {
                    &[(11., -80.), (14., 80.), (17., -40.), (20., 40.), (23., 0.)]
                } else if self.kind == Kind::Rose {
                    &[(10., 0.)]
                } else {
                    &[(14., 0.)]
                };
                for (i, &(frame, yaw)) in events.iter().enumerate() {
                    if self.contacts & (1 << i) == 0
                        && self.time >= frame * data.frame(self.kind.model(), self.clip())
                    {
                        self.contacts |= 1 << i;
                        if visible {
                            self.fire(data, aim, frame, yaw);
                        }
                    }
                }
                if self.time >= duration {
                    self.set(Phase::Ready);
                    self.cooldown = 0.4;
                }
            }
            Phase::Suck => {
                if !visible || distance >= 416. || self.time >= 2. {
                    self.set(Phase::Release);
                } else if distance < 100. {
                    self.set(Phase::Grab);
                } else {
                    out.strike(
                        victim,
                        0.,
                        -delta.normalize_or_zero() * 1200. * STEP,
                        DamageKind::Other,
                    );
                }
            }
            Phase::Grab => {
                if !visible || distance >= 120. {
                    self.set(Phase::Release);
                } else if self.time >= duration {
                    self.set(Phase::Digest);
                    out.sounds.push("sound/character/evil_mushroom/digest.wav");
                }
            }
            Phase::Digest => {
                if !visible || distance >= 120. {
                    self.set(Phase::Release);
                } else {
                    for (i, at) in [0.25, 0.75, 1.25].into_iter().enumerate() {
                        if self.contacts & (1 << i) == 0 && self.time >= at {
                            self.contacts |= 1 << i;
                            out.strike(victim, 5., Vec3::ZERO, DamageKind::Other);
                        }
                    }
                    if self.time >= 1.5 {
                        self.set(Phase::Release);
                        out.strike(
                            victim,
                            0.,
                            delta.normalize_or_zero() * 200.,
                            DamageKind::Other,
                        );
                    }
                }
            }
            Phase::Release | Phase::Pain => {
                if self.time >= duration {
                    self.set(Phase::Ready);
                    self.cooldown = 0.75;
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
                && (0. ..=self.kind.health()).contains(&self.health)
                && self.time.is_finite()
                && (0. ..=86400.).contains(&self.time)
                && (0. ..=0.7).contains(&self.growth)
                && self.variant < 3
                && (0. ..=1.).contains(&self.cooldown)
                && self.pain.is_finite()
                && self.pain >= 0.
                && self.shots.len() <= 12,
            "Invalid plant state"
        );
        ensure!(
            (self.health == 0.) == (self.phase == Phase::Dead)
                && (!self.frozen || self.health == 0.),
            "Invalid plant death"
        );
        ensure!(
            match self.kind {
                Kind::Rose => !matches!(
                    self.phase,
                    Phase::Suck | Phase::Grab | Phase::Digest | Phase::Release
                ),
                Kind::Mushroom => !matches!(self.phase, Phase::Grow | Phase::Melee | Phase::Fan),
            },
            "Plant family phase mismatch"
        );
        let mut ids = std::collections::BTreeSet::new();
        for s in &self.shots {
            ensure!(
                ids.insert(s.serial)
                    && s.position.is_finite()
                    && s.direction.is_finite()
                    && (s.direction.length() - 1.).abs() < 0.01
                    && (0. ..=5.).contains(&s.age),
                "Invalid plant projectile"
            );
        }
        Ok(())
    }
}
