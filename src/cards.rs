//! Heart and Spade guards: authored clips/contacts with bounded local steering.
use crate::{
    collision::World,
    combat::{self, DamageKind, Feedback, Hit, Opponents, Target},
    dismember,
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
#[cfg(test)]
mod tests;
pub const STEP: f32 = 1. / 120.;
pub trait Rig: crate::clockwork::Rig {
    fn sever(&self) -> Option<&dismember::Recipe>;
}
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Kind {
    Heart,
    Spade,
}
impl Kind {
    pub fn from_model(model: &str) -> Option<Self> {
        match model {
            "cardguard_heart" => Some(Self::Heart),
            "cardguard_spade" => Some(Self::Spade),
            _ => None,
        }
    }
    pub fn model(self) -> &'static str {
        match self {
            Self::Heart => "cardguard_heart",
            Self::Spade => "cardguard_spade",
        }
    }
    pub fn projectile(self) -> &'static str {
        match self {
            Self::Heart => "prj_heart",
            Self::Spade => "prj_spade",
        }
    }
    pub fn health(self) -> f32 {
        if self == Self::Heart {
            200.
        } else {
            160.
        }
    }
    pub fn clips(self) -> &'static [&'static str] {
        match self {
            Self::Heart => &[
                "idle1",
                "twitch",
                "alert1",
                "walk",
                "run",
                "attack1",
                "attack2",
                "attack3",
                "attack4",
                "attack_5_begin",
                "attack_5_mid",
                "attack_5_end",
                "fire_1_begin",
                "fire1",
                "fire2",
                "pain1",
                "pain2",
                "pain3",
                "death1",
                "death2",
                "death3",
                "death_frozen",
            ],
            Self::Spade => &[
                "idle",
                "idle_shift",
                "alert01",
                "alert02",
                "walk",
                "run",
                "pain01",
                "pain02",
                "attack_ranged",
                "attack_basic",
                "attack_spin",
                "death01",
                "death02",
                "death_top",
                "death_frozen",
                "death_gib",
                "death_gib2",
            ],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    Idle,
    Alert,
    Chase,
    Run,
    Flee,
    Melee,
    ComboStart,
    Combo,
    ComboEnd,
    Charge,
    Fire,
    Slam,
    Pain,
    Dead,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Shot {
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
pub struct Guard {
    pub kind: Kind,
    pub feet: Vec3,
    pub yaw: f32,
    pub scale: f32,
    pub health: f32,
    pub phase: Phase,
    pub time: f32,
    pub variant: usize,
    pub frozen: bool,
    pub cut: bool,
    pub dismember: dismember::State,
    pub opponents: Opponents,
    pub notarget: bool,
    pub shots: Vec<Shot>,
    pub impacts: Vec<Impact>,
    pub vision: f32,
    recoil: combat::Recoil,
    falling: f32,
    placed: bool,
    contacts: u8,
    cooldown: f32,
    pain: f32,
    memory: f32,
    last_seen: Vec3,
    random: u32,
    serial: u32,
}
impl Guard {
    pub fn new(kind: Kind, feet: Vec3, yaw: f32, scale: f32, seed: usize) -> Self {
        Self {
            kind,
            feet,
            yaw,
            scale,
            health: kind.health(),
            phase: Phase::Idle,
            time: 0.,
            variant: 0,
            frozen: false,
            cut: false,
            dismember: Default::default(),
            opponents: Default::default(),
            notarget: false,
            shots: vec![],
            impacts: vec![],
            vision: if kind == Kind::Heart { 1200. } else { 1000. },
            recoil: Default::default(),
            falling: 0.,
            placed: false,
            contacts: 0,
            cooldown: 0.,
            pain: 0.,
            memory: 0.,
            last_seen: feet,
            random: seed as u32,
            serial: 0,
        }
    }
    pub fn target(&self, id: usize) -> Target {
        let half = if self.kind == Kind::Heart {
            vec3(38., 38., 48.)
        } else {
            vec3(32., 32., 44.)
        } * self.scale;
        Target {
            id,
            center: self.feet + Vec3::Z * half.z,
            half,
        }
    }
    pub fn place(&mut self, world: &World) {
        if self.placed {
            return;
        }
        let b = self.target(0);
        if let Some(feet) = world.actor_footing(self.feet, b.center - self.feet, b.half, 1024.) {
            self.feet = feet;
        }
        self.placed = true;
    }
    /// Explicit script attack orders supply a destination even before first sight.
    /// Shooting and movement still obey the normal collision and visibility checks.
    pub(crate) fn attack_player(&mut self, eye: Vec3) {
        if self.health <= 0. || self.notarget {
            return;
        }
        self.last_seen = eye;
        self.memory = 6.;
        if self.phase == Phase::Idle {
            self.set(Phase::Chase);
        }
    }
    pub fn set(&mut self, phase: Phase) {
        self.phase = phase;
        self.time = 0.;
        self.contacts = 0;
    }
    fn random(&mut self) -> usize {
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.random >> 16) as usize
    }
    pub fn loops(&self) -> bool {
        matches!(
            self.phase,
            Phase::Idle | Phase::Chase | Phase::Run | Phase::Flee
        )
    }
    pub fn clip(&self) -> &'static str {
        if self.frozen {
            return "death_frozen";
        }
        if self.cut {
            return "death_gib";
        }
        match (self.kind, self.phase) {
            (_, Phase::Chase) => "walk",
            (_, Phase::Run | Phase::Flee) => "run",
            (Kind::Heart, Phase::Alert) => "alert1",
            (Kind::Spade, Phase::Alert) => ["alert01", "alert02"][self.variant % 2],
            (Kind::Heart, Phase::Melee) => {
                ["attack1", "attack2", "attack3", "attack4"][self.variant % 4]
            }
            (Kind::Spade, Phase::Melee) => {
                if self.variant % 3 == 0 {
                    "attack_spin"
                } else {
                    "attack_basic"
                }
            }
            (_, Phase::ComboStart) => "attack_5_begin",
            (_, Phase::Combo) => "attack_5_mid",
            (_, Phase::ComboEnd) => "attack_5_end",
            (_, Phase::Charge) => "fire_1_begin",
            (Kind::Heart, Phase::Fire) => "fire1",
            (_, Phase::Slam) => "fire2",
            (Kind::Spade, Phase::Fire) => "attack_ranged",
            (Kind::Heart, Phase::Pain) => ["pain1", "pain2", "pain3"][self.variant % 3],
            (Kind::Spade, Phase::Pain) => ["pain01", "pain02"][self.variant % 2],
            (Kind::Heart, Phase::Dead) => ["death1", "death2", "death3"][self.variant % 3],
            (Kind::Spade, Phase::Dead) => ["death01", "death02"][self.variant % 2],
            (Kind::Heart, _) => "idle1",
            (Kind::Spade, _) => "idle",
        }
    }
    pub fn sample_time(&self, rig: &impl Rig) -> f32 {
        if self.frozen {
            self.time.min(
                rig.frame(self.kind.model(), self.clip())
                    * if self.kind == Kind::Heart { 9. } else { 6. },
            )
        } else {
            self.time
        }
    }
    pub fn fade(&self, rig: &impl Rig) -> f32 {
        if self.health > 0. {
            1.
        } else if self.frozen {
            (3. - self.time).clamp(0., 1.)
        } else {
            (1. - (self.time - rig.duration(self.kind.model(), self.clip())) / 2.).clamp(0., 1.)
        }
    }
    fn sound(&self) -> Option<&'static str> {
        match (self.kind, self.phase) {
            (Kind::Heart, Phase::Alert) => Some("sound/character/cardguard/heart/alert1.wav"),
            (Kind::Heart, Phase::Melee) => Some(
                [
                    "sound/character/cardguard/heart/attack1.wav",
                    "sound/character/cardguard/heart/attack2.wav",
                    "sound/character/cardguard/heart/attack3.wav",
                    "sound/character/cardguard/heart/attack4.wav",
                ][self.variant % 4],
            ),
            (Kind::Heart, Phase::ComboStart) => {
                Some("sound/character/cardguard/heart/attack_5_begin.wav")
            }
            (Kind::Heart, Phase::Combo) => Some("sound/character/cardguard/heart/attack_5_mid.wav"),
            (Kind::Heart, Phase::ComboEnd) => {
                Some("sound/character/cardguard/heart/attack_5_end.wav")
            }
            (Kind::Heart, Phase::Charge) => {
                Some("sound/character/cardguard/heart/fire_1_begin.wav")
            }
            (Kind::Heart, Phase::Fire) => Some("sound/character/cardguard/heart/fire1.wav"),
            (Kind::Heart, Phase::Slam) => Some("sound/character/cardguard/heart/fire2.wav"),
            (Kind::Heart, Phase::Pain) => Some(
                [
                    "sound/character/cardguard/heart/pain1.wav",
                    "sound/character/cardguard/heart/pain2.wav",
                    "sound/character/cardguard/heart/pain3.wav",
                ][self.variant % 3],
            ),
            (Kind::Heart, Phase::Dead) => Some(
                [
                    "sound/character/cardguard/heart/death1.wav",
                    "sound/character/cardguard/heart/death2.wav",
                    "sound/character/cardguard/heart/death3.wav",
                ][self.variant % 3],
            ),
            (Kind::Spade, Phase::Alert) => Some("sound/character/cardguard/spade/alert01.wav"),
            (Kind::Spade, Phase::Melee) => Some(if self.variant % 3 == 0 {
                "sound/character/cardguard/spade/attack_spin.wav"
            } else {
                "sound/character/cardguard/spade/attack_basic.wav"
            }),
            (Kind::Spade, Phase::Fire) => Some("sound/character/cardguard/spade/attack_ranged.wav"),
            (Kind::Spade, Phase::Pain) => Some(
                [
                    "sound/character/cardguard/spade/pain1.wav",
                    "sound/character/cardguard/spade/pain2.wav",
                ][self.variant % 2],
            ),
            (Kind::Spade, Phase::Dead) => Some(
                [
                    "sound/character/cardguard/spade/death1.wav",
                    "sound/character/cardguard/spade/death2.wav",
                ][self.variant % 2],
            ),
            _ => None,
        }
    }
    fn enter(&mut self, phase: Phase, out: &mut Feedback) {
        self.set(phase);
        if let Some(s) = self.sound() {
            out.sounds.push(s);
        }
    }
    pub fn hit(&mut self, hit: Hit) -> Option<&'static str> {
        if self.health <= 0. || !hit.damage.is_finite() || hit.damage <= 0. {
            return None;
        }
        self.health = (self.health - hit.damage).max(0.);
        self.opponents.demon |= hit.kind.is_demon();
        self.recoil.hit(hit.knockback, 200.);
        self.memory = 6.;
        if self.health == 0. {
            self.variant = self.random() % 12;
            self.frozen = hit.kind.means() == DamageKind::Ice;
            self.cut = self.kind == Kind::Spade && hit.kind.means() == DamageKind::Knife;
            self.set(Phase::Dead);
            return self.sound();
        }
        self.pain += hit.damage;
        if self.phase != Phase::Pain && (self.kind == Kind::Spade || self.pain >= 35.) {
            self.pain = 0.;
            self.variant = self.random() % 12;
            self.set(Phase::Pain);
            self.cooldown = 0.35;
            return self.sound();
        }
        None
    }
    fn fire(&mut self, rig: &impl Rig, aim: Vec3, frame: f32, victim: usize) {
        if self.shots.len() >= 12 {
            return;
        }
        let local = rig.tag(
            self.clip(),
            frame * rig.frame(self.kind.model(), self.clip()),
            "tag_barrel",
        );
        let position = self.feet + Quat::from_rotation_z(self.yaw) * local.translation * self.scale;
        let direction = (aim - position).normalize_or_zero();
        if direction.length_squared() < 0.5 {
            return;
        }
        self.serial = self.serial.wrapping_add(1);
        self.shots.push(Shot {
            serial: self.serial,
            position,
            direction,
            age: 0.,
            victim,
        });
    }
    fn projectiles(&mut self, world: &World, eye: Vec3, out: &mut Feedback) {
        self.impacts.retain_mut(|i| {
            i.age += STEP;
            i.age < 2.
        });
        let targets = self.opponents.bodies(eye);
        self.shots.retain_mut(|s| {
            if self.kind == Kind::Heart {
                if let Some(t) = targets.iter().find(|t| t.id == s.victim) {
                    let desired = (t.center - s.position).normalize_or_zero();
                    if desired.length_squared() > 0.5 {
                        let angle = s.direction.dot(desired).clamp(-1., 1.).acos();
                        let turn = Quat::IDENTITY.slerp(
                            Quat::from_rotation_arc(s.direction, desired),
                            (2. * STEP / angle.max(0.0001)).min(1.),
                        );
                        s.direction = (turn * s.direction).normalize_or_zero();
                    }
                }
            }
            let end = s.position + s.direction * 850. * STEP;
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
            s.age += STEP;
            let impact = hit.is_some() || wall.start_solid || wall.fraction < 1. || s.age >= 5.;
            if impact {
                if let Some((id, _)) = hit {
                    out.strike(
                        id,
                        if self.kind == Kind::Heart { 20. } else { 13. },
                        s.direction * 150.,
                        DamageKind::Other,
                    );
                }
                for t in &targets {
                    if hit.is_some_and(|(id, _)| id == t.id) {
                        continue;
                    }
                    let delta = t.center - s.position;
                    let falloff = (1. - delta.length() / 110.).max(0.);
                    let trace = world.sweep(s.position, t.center, Vec3::splat(0.5));
                    if falloff > 0. && !trace.start_solid && trace.fraction >= 1. {
                        out.strike(
                            t.id,
                            50. * falloff,
                            delta.normalize_or_zero() * 200. * falloff,
                            DamageKind::Other,
                        );
                    }
                }
                if self.impacts.len() < 12 {
                    self.impacts.push(Impact {
                        position: s.position,
                        age: 0.,
                    });
                }
                out.spatial_sounds
                    .push(("sound/weapon/shared/explode1.wav", s.position));
            }
            !impact
        });
    }
    pub fn step(&mut self, world: &World, eye: Vec3, rig: &impl Rig, out: &mut Feedback) {
        self.projectiles(world, eye, out);
        self.place(world);
        if self.health <= 0. {
            self.time = (self.time + STEP).min(rig.duration(self.kind.model(), self.clip()) + 6.);
            if self.cut {
                let lift = rig.sever().map_or(0., dismember::card::birth_lift);
                self.dismember.update(
                    STEP,
                    self.time,
                    self.feet + Vec3::Z * lift * self.scale,
                    self.yaw,
                    self.scale,
                    world,
                    rig.sever(),
                );
            }
            return;
        }
        let body = self.target(0);
        self.feet += self.recoil.step(STEP, world, body);
        let body = self.target(0);
        self.falling = (self.falling - 800. * STEP).max(-1000.);
        let trace = world.sweep(
            body.center,
            body.center + Vec3::Z * self.falling * STEP,
            body.half,
        );
        if !trace.start_solid {
            self.feet.z += self.falling * STEP * trace.fraction;
        }
        if trace.fraction < 1. || trace.start_solid {
            self.falling = 0.;
        }
        self.time += STEP;
        if self.loops() && self.time > 3600. {
            self.time = 0.;
        }
        self.cooldown = (self.cooldown - STEP).max(0.);
        self.memory = (self.memory - STEP).max(0.);
        let from = self.feet
            + Vec3::Z
                * if self.kind == Kind::Heart {
                    88. * self.scale
                } else {
                    80. * self.scale
                };
        let (aim, notarget, victim) = self.opponents.aim(
            world,
            from,
            eye,
            self.notarget,
            !self.loops() && self.phase != Phase::Alert,
        );
        let delta = aim - from;
        let distance = delta.truncate().length();
        let facing = vec2(self.yaw.cos(), self.yaw.sin()).dot(delta.truncate().normalize_or_zero());
        let trace = world.sweep(from, aim, Vec3::splat(0.5));
        let visible = !notarget
            && delta.length() < self.vision
            && !trace.start_solid
            && trace.fraction >= 1.
            && (self.memory > 0.
                || facing
                    > if self.kind == Kind::Heart {
                        -0.866
                    } else {
                        -0.766
                    });
        if visible {
            self.memory = 6.;
            self.last_seen = aim;
        }
        let movement = if self.phase == Phase::Flee {
            self.feet - self.last_seen
        } else {
            self.last_seen - self.feet
        };
        if (visible || self.memory > 0.) && self.phase != Phase::Pain {
            let d = if self.loops() { movement } else { delta };
            let angle = (d.y.atan2(d.x) - self.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw += angle.clamp(-5. * STEP, 5. * STEP);
        }
        let duration = rig.duration(self.kind.model(), self.clip());
        match self.phase {
            Phase::Idle => {
                if visible {
                    self.variant = self.random() % 12;
                    self.enter(Phase::Alert, out);
                }
            }
            Phase::Alert => {
                if self.time >= duration {
                    self.set(Phase::Chase);
                }
            }
            Phase::Chase | Phase::Run | Phase::Flee => {
                if self.memory == 0. || notarget {
                    self.set(Phase::Idle);
                    return;
                }
                if visible && facing > 0.65 && self.cooldown == 0. && self.phase != Phase::Flee {
                    self.variant = self.random() % 12;
                    if distance < 120. * self.scale {
                        self.enter(
                            if self.kind == Kind::Heart && self.variant % 5 == 4 {
                                Phase::ComboStart
                            } else {
                                Phase::Melee
                            },
                            out,
                        );
                        return;
                    } else if distance < self.vision
                        && self.random() % 100 < if self.kind == Kind::Heart { 67 } else { 40 }
                    {
                        let phase = if self.kind == Kind::Spade {
                            Phase::Fire
                        } else if self.random() % 2 == 0 {
                            Phase::Charge
                        } else {
                            Phase::Slam
                        };
                        self.enter(phase, out);
                        return;
                    }
                    self.cooldown = 0.7;
                }
                let phase = if self.kind == Kind::Spade && self.health < 6. && distance < 400. {
                    Phase::Flee
                } else if distance > 250. {
                    Phase::Run
                } else {
                    Phase::Chase
                };
                if self.phase != phase {
                    self.set(phase);
                }
                if movement.truncate().length() > 50. {
                    let stride = rig.speed(self.kind.model(), self.clip()) * STEP * self.scale;
                    let dir = movement.with_z(0.).normalize_or_zero();
                    for angle in [0., 0.65, -0.65, 1.1, -1.1] {
                        let next = combat::walk_body(
                            world,
                            self.feet,
                            Quat::from_rotation_z(angle) * dir * stride,
                            self.target(0).half,
                        );
                        if (next - self.feet).truncate().length_squared() > 0.0001 {
                            self.feet = next;
                            break;
                        }
                    }
                }
            }
            Phase::Melee | Phase::Combo => {
                let events: &[(f32, f32)] = match self.clip() {
                    "attack1" => &[(8., 15.)],
                    "attack2" => &[(11., 15.)],
                    "attack3" => &[(6., 15.), (9., 15.)],
                    "attack4" => &[(25., 15.), (38., 15.)],
                    "attack_5_mid" => &[(2., 15.), (5., 15.), (9., 15.), (12., 15.)],
                    "attack_spin" => &[(9., 5.), (13., 15.)],
                    _ => &[(11., 15.)],
                };
                for (i, &(frame, damage)) in events.iter().enumerate() {
                    if self.contacts & (1 << i) == 0
                        && self.time >= frame * rig.frame(self.kind.model(), self.clip())
                    {
                        self.contacts |= 1 << i;
                        if visible
                            && facing > 0.5
                            && distance < 130. * self.scale
                            && delta.z.abs() < 96. * self.scale
                        {
                            out.strike(
                                victim,
                                damage,
                                delta.normalize_or_zero() * 50.,
                                DamageKind::Other,
                            );
                        }
                    }
                }
                if self.time >= duration {
                    if self.phase == Phase::Combo {
                        self.enter(Phase::ComboEnd, out);
                    } else {
                        self.set(Phase::Chase);
                        self.cooldown = 0.5;
                    }
                }
            }
            Phase::ComboStart => {
                if self.time >= duration {
                    self.enter(Phase::Combo, out);
                }
            }
            Phase::ComboEnd | Phase::Pain => {
                if self.time >= duration {
                    self.set(Phase::Chase);
                    self.cooldown = 0.5;
                }
            }
            Phase::Charge => {
                if self.time >= duration.max(0.5) {
                    self.enter(Phase::Fire, out);
                }
            }
            Phase::Fire | Phase::Slam => {
                let events: &[f32] = if self.kind == Kind::Spade {
                    &[16., 30.]
                } else if self.phase == Phase::Slam {
                    &[38.]
                } else {
                    &[0.]
                };
                for (i, &frame) in events.iter().enumerate() {
                    if self.contacts & (1 << i) == 0
                        && self.time >= frame * rig.frame(self.kind.model(), self.clip())
                    {
                        self.contacts |= 1 << i;
                        if visible {
                            self.fire(rig, aim, frame, victim);
                        }
                    }
                }
                if self.time >= duration {
                    self.set(Phase::Chase);
                    self.cooldown = 1.;
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
                && self.variant < 12
                && (0. ..=1.).contains(&self.cooldown)
                && (0. ..=6.).contains(&self.memory)
                && self.last_seen.is_finite()
                && self.pain.is_finite()
                && self.pain >= 0.
                && self.recoil.valid()
                && (-1000. ..=0.).contains(&self.falling)
                && [1000., 1200., 4000.].contains(&self.vision),
            "Invalid card guard state"
        );
        ensure!(
            (self.health == 0.) == (self.phase == Phase::Dead)
                && (!self.frozen || self.health == 0.)
                && (!self.cut || (self.kind == Kind::Spade && self.health == 0. && !self.frozen))
                && (self.cut || !self.dismember.severed),
            "Invalid card death"
        );
        ensure!(
            self.kind == Kind::Heart
                || !matches!(
                    self.phase,
                    Phase::Charge
                        | Phase::Slam
                        | Phase::ComboStart
                        | Phase::Combo
                        | Phase::ComboEnd
                ),
            "Card phase/family mismatch"
        );
        self.dismember.validate(self.cut)?;
        let mut ids = std::collections::BTreeSet::new();
        ensure!(
            self.shots.len() <= 12 && self.impacts.len() <= 12,
            "Unbounded card projectiles"
        );
        for s in &self.shots {
            ensure!(
                ids.insert(s.serial)
                    && s.position.is_finite()
                    && s.direction.is_finite()
                    && (s.direction.length() - 1.).abs() < 0.01
                    && (0. ..=5.).contains(&s.age)
                    && [crate::dice::ALICE, crate::dice::SUMMON].contains(&s.victim),
                "Invalid card shot"
            );
        }
        for i in &self.impacts {
            ensure!(
                i.position.is_finite() && (0. ..=2.).contains(&i.age),
                "Invalid card impact"
            );
        }
        Ok(())
    }
}
