//! Water-bound Snarks using source clips/cues and bounded local swimming.
use crate::{
    clockwork::Rig,
    collision::World,
    combat::{self, DamageKind, Feedback, Hit, Opponents, Recoil, Target},
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
#[cfg(test)]
mod tests;
const STEP: f32 = 1. / 120.;
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Kind {
    Water,
    BiteOnly,
    Fire,
}
impl Kind {
    pub const ALL: [Self; 3] = [Self::Water, Self::BiteOnly, Self::Fire];
    pub fn from_model(model: &str) -> Option<Self> {
        match model {
            "c_snark" => Some(Self::Water),
            "c_snark_biteonly" => Some(Self::BiteOnly),
            "c_firesnark" => Some(Self::Fire),
            _ => None,
        }
    }
    pub fn model(self) -> &'static str {
        match self {
            Self::Water => "c_snark",
            Self::BiteOnly => "c_snark_biteonly",
            Self::Fire => "c_firesnark",
        }
    }
    pub fn health(self) -> f32 {
        if self == Self::Fire {
            100.
        } else {
            25.
        }
    }
    pub fn clips(self) -> Vec<&'static str> {
        let mut clips = vec!["swim_norm", "bite", "death01", "death02"];
        if self == Self::BiteOnly {
            clips.extend(["idle1", "pain1", "pain2", "pain3"]);
        } else {
            clips.extend([
                "idle",
                "pain01",
                "pain02",
                "pain03",
                "jump_up",
                "jump_down",
                "spit",
                "tongue",
            ]);
        }
        if self != Self::Fire {
            clips.push("death_frozen");
        }
        clips
    }
    pub fn projectile(self) -> &'static str {
        if self == Self::Fire {
            "prj_fireball"
        } else {
            "prj_acid_glob"
        }
    }
    pub fn speed(self) -> f32 {
        if self == Self::Fire {
            700.
        } else {
            600.
        }
    }
    pub fn wet(self, world: &World, point: Vec3) -> bool {
        let mask = world.liquid_at(point);
        if self == Self::Fire {
            mask & (8 | 32) != 0
        } else {
            mask & 32 != 0 && mask & 8 == 0
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    Idle,
    Swim,
    Rise,
    Spit,
    Tongue,
    Dive,
    Bite,
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
struct Acid {
    victim: usize,
    time: f32,
    ticks: u8,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Snark {
    pub kind: Kind,
    /// Authored entity origin (water Snarks have a centred collision hull).
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
    pub tongue: Option<Vec3>,
    acid: Vec<Acid>,
    cooldown: f32,
    pain: f32,
    contact: bool,
    random: u32,
    serial: u32,
    recoil: Recoil,
    fall: f32,
    rise_to: f32,
    ranged_tongue: bool,
}
impl Snark {
    pub fn new(kind: Kind, feet: Vec3, yaw: f32, scale: f32, seed: usize) -> Self {
        Self {
            kind,
            feet,
            yaw,
            scale,
            health: kind.health(),
            phase: Phase::Idle,
            time: 0.,
            frozen: false,
            variant: 0,
            opponents: Default::default(),
            notarget: false,
            shots: vec![],
            tongue: None,
            acid: vec![],
            cooldown: 0.,
            pain: 0.,
            contact: false,
            random: seed as u32,
            serial: 0,
            recoil: Default::default(),
            fall: 0.,
            rise_to: feet.z,
            ranged_tongue: false,
        }
    }
    pub fn target(&self, id: usize) -> Target {
        let fire = self.kind == Kind::Fire;
        Target {
            id,
            center: self.feet + Vec3::Z * if fire { 40. * self.scale } else { 0. },
            half: vec3(24., 24., if fire { 32. } else { 24. }) * self.scale,
        }
    }
    pub fn loops(&self) -> bool {
        matches!(self.phase, Phase::Idle | Phase::Swim)
    }
    pub fn clip(&self) -> &'static str {
        if self.frozen {
            return if self.kind == Kind::Fire {
                "death01"
            } else {
                "death_frozen"
            };
        }
        match self.phase {
            Phase::Idle => {
                if self.kind == Kind::BiteOnly {
                    "idle1"
                } else {
                    "idle"
                }
            }
            Phase::Swim => "swim_norm",
            Phase::Rise => "jump_up",
            Phase::Dive => "jump_down",
            Phase::Bite => "bite",
            Phase::Spit => "spit",
            Phase::Tongue => "tongue",
            Phase::Pain => {
                if self.kind == Kind::BiteOnly {
                    ["pain1", "pain2", "pain3"][self.variant]
                } else {
                    ["pain01", "pain02", "pain03"][self.variant]
                }
            }
            Phase::Dead => ["death01", "death02"][self.variant % 2],
        }
    }
    pub fn sample_time(&self) -> f32 {
        if self.frozen {
            0.
        } else {
            self.time
        }
    }
    pub fn visual_scale(&self, data: &impl Rig) -> f32 {
        self.scale
            * if self.health > 0. {
                1.
            } else if self.frozen {
                (2.5 - self.time).clamp(0., 1.)
            } else {
                1. - ((self.time - data.duration(self.kind.model(), self.clip())) / 1.4)
                    .clamp(0., 1.)
            }
    }
    pub fn set(&mut self, phase: Phase) {
        self.phase = phase;
        self.time = 0.;
        self.contact = false;
        self.tongue = None;
    }
    fn sound(&self, cue: &str) -> &'static str {
        match (self.kind == Kind::Fire, cue) {
            (true, "bite") => "sound/character/snark/fire/bite.wav",
            (true, "spit") => "sound/character/snark/fire/spit.wav",
            (true, "tongue") => "sound/character/snark/fire/tongue.wav",
            (true, "pain") => "sound/character/snark/fire/pain01.wav",
            (true, _) => "sound/character/snark/fire/death01.wav",
            (false, "bite") => "sound/character/snark/water/bite.wav",
            (false, "spit") => "sound/character/snark/water/spit.wav",
            (false, "tongue") => "sound/character/snark/water/tongue.wav",
            (false, "pain") => "sound/character/snark/water/pain1.wav",
            (false, _) => "sound/character/snark/water/death1.wav",
        }
    }
    pub fn hit(&mut self, hit: Hit) -> Option<&'static str> {
        if self.health <= 0. || !hit.damage.is_finite() || hit.damage <= 0. {
            return None;
        }
        self.health = (self.health - hit.damage).max(0.);
        self.opponents.demon |= hit.kind.is_demon();
        self.recoil.hit(hit.knockback, 100.);
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        self.variant = (self.random >> 16) as usize % 3;
        if self.health == 0. {
            self.frozen = hit.kind.means() == DamageKind::Ice;
            self.set(Phase::Dead);
            return Some(self.sound("death"));
        }
        // Nonlethal feedback is useful even though the water variant's source
        // pain threshold exceeds its entire health pool. Debounce rapid hits.
        if self.pain == 0. {
            self.pain = 0.4;
            self.set(Phase::Pain);
            return Some(self.sound("pain"));
        }
        None
    }
    fn movement(&mut self, world: &World, delta: Vec3, wet_only: bool) {
        // Small substeps also constrain large knockback to the connected liquid.
        let steps = (delta.length() / 2.).ceil().clamp(1., 40.) as usize;
        let delta = delta / steps as f32;
        for _ in 0..steps {
            let body = self.target(0);
            let trace = world.sweep(body.center, body.center + delta, body.half);
            if trace.start_solid {
                break;
            }
            let advance = delta * trace.fraction;
            if !wet_only || self.kind.wet(world, self.feet + advance) {
                self.feet += advance;
            }
            if trace.fraction < 1. {
                break;
            }
        }
    }
    fn projectiles(&mut self, world: &World, eye: Vec3, out: &mut Feedback) {
        let targets = self.opponents.bodies(eye);
        for acid in &mut self.acid {
            acid.time += STEP;
            if acid.time >= 1. {
                acid.time -= 1.;
                acid.ticks += 1;
                if targets.iter().any(|t| t.id == acid.victim) {
                    out.strike(acid.victim, 5., Vec3::ZERO, DamageKind::Other);
                }
            }
        }
        self.acid
            .retain(|a| a.ticks < 3 && targets.iter().any(|t| t.id == a.victim));
        let kind = self.kind;
        self.shots.retain_mut(|s| {
            let end = s.position + s.direction * kind.speed() * STEP;
            let radius = if kind == Kind::Fire { 4. } else { 16. };
            let hit = combat::contact(
                &combat::Context {
                    world,
                    targets: &targets,
                },
                s.position,
                end,
                radius,
            );
            let wall = world.sweep(s.position, end, Vec3::splat(radius));
            s.position = s.position.lerp(end, hit.map_or(wall.fraction, |(_, f)| f));
            s.age += STEP;
            if let Some((id, _)) = hit {
                out.strike(
                    id,
                    if kind == Kind::Fire { 25. } else { 5. },
                    s.direction * 30.,
                    if kind == Kind::Fire {
                        DamageKind::FireSword
                    } else {
                        DamageKind::Other
                    },
                );
                if kind != Kind::Fire {
                    self.acid.retain(|a| a.victim != id);
                    self.acid.push(Acid {
                        victim: id,
                        time: 0.,
                        ticks: 0,
                    });
                }
            }
            hit.is_none()
                && !wall.start_solid
                && wall.fraction >= 1.
                && s.age < if kind == Kind::Fire { 2. } else { 5. }
        });
    }
    pub fn mouth(&self, data: &impl Rig, time: f32) -> Vec3 {
        self.feet
            + Quat::from_rotation_z(self.yaw)
                * data.tag(self.clip(), time, "tag_mouth").translation
                * self.scale
    }
    pub fn step(&mut self, world: &World, eye: Vec3, data: &impl Rig, out: &mut Feedback) {
        self.projectiles(world, eye, out);
        if self.health <= 0. {
            self.time = (self.time + STEP).min(data.duration(self.kind.model(), self.clip()) + 3.);
            return;
        }
        self.time += STEP;
        if self.loops() && self.time > 3600. {
            self.time = 0.;
        }
        self.cooldown = (self.cooldown - STEP).max(0.);
        self.pain = (self.pain - STEP).max(0.);
        let wet = self.kind.wet(world, self.feet);
        let recoil = self.recoil.step(STEP, world, self.target(0));
        self.movement(world, recoil, wet);
        let body = self.target(0);
        let (aim, notarget, victim) = self.opponents.aim(
            world,
            body.center,
            eye,
            self.notarget,
            !matches!(self.phase, Phase::Idle | Phase::Swim | Phase::Pain),
        );
        let delta = aim - body.center;
        let distance = delta.length();
        let trace = world.sweep(body.center, aim, Vec3::splat(0.5));
        let visible = !notarget && distance < 1000. && !trace.start_solid && trace.fraction >= 1.;
        if visible && self.phase != Phase::Pain {
            let angle = (delta.y.atan2(delta.x) - self.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw =
                (self.yaw + angle.clamp(-3. * STEP, 3. * STEP)).rem_euclid(std::f32::consts::TAU);
        }
        let facing = delta.truncate().length_squared() < 1.
            || vec2(self.yaw.cos(), self.yaw.sin()).dot(delta.truncate().normalize_or_zero()) > 0.6;
        let duration = data.duration(self.kind.model(), self.clip());
        let airborne_attack = matches!(self.phase, Phase::Rise | Phase::Spit | Phase::Tongue);
        if !wet && !airborne_attack {
            self.fall = (self.fall + 600. * STEP).min(600.);
            self.movement(world, -Vec3::Z * self.fall * STEP, false);
        } else {
            self.fall = 0.;
        }
        self.tongue = None;
        match self.phase {
            Phase::Idle | Phase::Swim => {
                if !visible || !wet {
                    if self.phase != Phase::Idle {
                        self.set(Phase::Idle);
                    }
                    return;
                }
                if facing && self.cooldown == 0. && distance < 120. * self.scale {
                    self.set(Phase::Bite);
                    out.sounds.push(self.sound("bite"));
                } else if self.kind != Kind::BiteOnly
                    && facing
                    && self.cooldown == 0.
                    && self.time >= 1.
                    && distance < 550.
                    && !self.kind.wet(world, aim)
                {
                    self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
                    self.ranged_tongue = distance < 250. && self.random & 1 == 0;
                    self.rise_to = (aim.z - body.center.z).clamp(32., 192.) + self.feet.z;
                    self.set(Phase::Rise);
                } else {
                    if self.phase == Phase::Idle {
                        self.set(Phase::Swim);
                    }
                    let desired = aim - Vec3::Z * 40. - body.center;
                    let travel = desired.normalize_or_zero() * 240. * STEP;
                    let before = self.feet;
                    self.movement(world, travel, true);
                    if self.feet == before {
                        // Local axis slide follows banks/walls without crossing dry land.
                        for axis in [
                            vec3(travel.x, 0., 0.),
                            vec3(0., travel.y, 0.),
                            vec3(0., 0., travel.z),
                        ] {
                            self.movement(world, axis, true);
                        }
                    }
                }
            }
            Phase::Rise => {
                self.movement(
                    world,
                    Vec3::Z * (self.rise_to - self.feet.z).clamp(0., 320. * STEP),
                    false,
                );
                if self.time >= duration {
                    let attack = if self.ranged_tongue {
                        Phase::Tongue
                    } else {
                        Phase::Spit
                    };
                    self.set(attack);
                    if attack == Phase::Tongue {
                        out.sounds.push(self.sound("tongue"));
                    }
                }
            }
            Phase::Bite => {
                if !self.contact && self.time >= 8. * data.frame(self.kind.model(), self.clip()) {
                    self.contact = true;
                    if visible && facing && distance < 120. * self.scale {
                        out.strike(
                            victim,
                            10.,
                            delta.normalize_or_zero() * 60.,
                            DamageKind::Other,
                        );
                    }
                }
                if self.time >= duration {
                    self.set(Phase::Swim);
                    self.cooldown = 0.35;
                }
            }
            Phase::Spit => {
                if !self.contact && self.time >= 8. * data.frame(self.kind.model(), self.clip()) {
                    self.contact = true;
                    if visible && facing && self.shots.len() < 4 {
                        let position =
                            self.mouth(data, 8. * data.frame(self.kind.model(), self.clip()));
                        let direction = (aim - position).normalize_or_zero();
                        // A long animated jaw must not place its projectile through a wall.
                        let muzzle = world.sweep(
                            body.center,
                            position,
                            Vec3::splat(if self.kind == Kind::Fire { 4. } else { 16. }),
                        );
                        if direction.length_squared() > 0.5
                            && !muzzle.start_solid
                            && muzzle.fraction >= 1.
                        {
                            self.serial = self.serial.wrapping_add(1);
                            self.shots.push(Shot {
                                serial: self.serial,
                                position,
                                direction,
                                age: 0.,
                            });
                            out.sounds.push(self.sound("spit"));
                        }
                    }
                }
                if self.time >= duration {
                    self.set(Phase::Dive);
                    self.cooldown = 1.;
                }
            }
            Phase::Tongue => {
                if visible
                    && facing
                    && distance < 250.
                    && (0.2..duration.min(0.85)).contains(&self.time)
                {
                    let mouth = self.mouth(data, self.time);
                    let trace = world.sweep(mouth, aim, Vec3::splat(2.));
                    if !trace.start_solid && trace.fraction >= 1. {
                        self.tongue = Some(aim);
                        out.strike(
                            victim,
                            0.,
                            -delta.normalize_or_zero() * 160. * STEP,
                            DamageKind::Other,
                        );
                    }
                }
                if self.time >= duration {
                    self.set(Phase::Dive);
                    self.cooldown = 1.;
                }
            }
            Phase::Dive => {
                if wet || self.time >= 3. {
                    self.set(Phase::Swim);
                }
            }
            Phase::Pain => {
                if self.time >= duration {
                    self.set(Phase::Swim);
                    self.cooldown = 0.4;
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
                && (0. ..=86400.).contains(&self.time)
                && self.variant < 3
                && (0. ..=1.).contains(&self.cooldown)
                && (0. ..=0.4).contains(&self.pain)
                && self.recoil.valid()
                && (0. ..=600.).contains(&self.fall)
                && self.rise_to.is_finite()
                && self.tongue.is_none_or(|p| p.is_finite())
                && self.shots.len() <= 4
                && self.acid.len() <= 2,
            "Invalid Snark state"
        );
        ensure!(
            (self.health == 0.) == (self.phase == Phase::Dead)
                && (!self.frozen || self.health == 0.)
                && (self.tongue.is_none() || self.phase == Phase::Tongue),
            "Invalid Snark death/attack"
        );
        ensure!(
            self.kind != Kind::BiteOnly
                || (!matches!(
                    self.phase,
                    Phase::Rise | Phase::Dive | Phase::Spit | Phase::Tongue
                ) && self.shots.is_empty()
                    && self.acid.is_empty()),
            "Bite-only Snark gained a ranged attack"
        );
        let mut ids = std::collections::BTreeSet::new();
        for s in &self.shots {
            ensure!(
                ids.insert(s.serial)
                    && s.position.is_finite()
                    && s.direction.is_finite()
                    && (s.direction.length() - 1.).abs() < 0.01
                    && (0. ..=5.).contains(&s.age),
                "Invalid Snark projectile"
            );
        }
        ids.clear();
        for a in &self.acid {
            ensure!(
                self.kind == Kind::Water
                    && matches!(a.victim, crate::dice::ALICE | crate::dice::SUMMON)
                    && ids.insert(a.victim as u32)
                    && (0. ..1.).contains(&a.time)
                    && a.ticks < 3,
                "Invalid Snark acid"
            );
        }
        Ok(())
    }
}
