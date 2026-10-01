//! Antlions and Centipede larvae. Source clip cues, bounded local movement and attachment.
use crate::{
    ant::Timing,
    collision::World,
    combat::{self, DamageKind, Feedback, Hit, Opponents, Target},
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
#[cfg(test)]
mod tests;
pub const STEP: f32 = 1. / 120.;
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    Antlion,
    Underground,
    Larva,
}
impl Kind {
    pub const ALL: [Self; 3] = [Self::Antlion, Self::Underground, Self::Larva];
    pub fn from_model(m: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.model() == m)
    }
    pub fn model(self) -> &'static str {
        match self {
            Self::Antlion => "c_antlion",
            Self::Underground => "c_antlion-underground",
            Self::Larva => "c_larva",
        }
    }
    pub fn health(self) -> f32 {
        if self == Self::Larva {
            7.
        } else {
            120.
        }
    }
    pub fn clips(self) -> &'static [&'static str] {
        if self == Self::Larva {
            &[
                "idle",
                "run",
                "jump_attack",
                "air_pose",
                "jump_attack_hold",
                "jump_land",
                "unroll",
                "attack_attach",
                "attack_suck",
                "attack_death",
                "death_norm",
                "death_frozen",
            ]
        } else {
            &[
                "idle_base",
                "ready",
                "ready_twitch",
                "alert01",
                "alert02",
                "walk",
                "run",
                "burrow_up",
                "burrow_down",
                "attack_slash",
                "attack_pincers",
                "attack_sting",
                "pain_shakeoff",
                "pain_folding",
                "pain_strong",
                "death01",
                "death02",
                "death_frozen",
            ]
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    Idle,
    Hidden,
    Rise,
    Walk,
    Run,
    Melee,
    Dive,
    Tunnel,
    Pain,
    Leap,
    Attach,
    Suck,
    Detach,
    Land,
    Dead,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Insect {
    pub kind: Kind,
    pub feet: Vec3,
    pub yaw: f32,
    pub scale: f32,
    pub health: f32,
    pub phase: Phase,
    pub time: f32,
    pub variant: usize,
    pub frozen: bool,
    pub opponents: Opponents,
    pub notarget: bool,
    recoil: combat::Recoil,
    velocity: Vec3,
    placed: bool,
    contacts: u8,
    cooldown: f32,
    memory: f32,
    last_seen: Vec3,
    random: u32,
    chase_age: f32,
    drain_clock: f32,
    victim: Option<usize>,
    offset: Vec3,
}
impl Insect {
    pub fn new(kind: Kind, feet: Vec3, yaw: f32, scale: f32, seed: usize) -> Self {
        Self {
            kind,
            feet,
            yaw,
            scale,
            health: kind.health(),
            phase: if kind == Kind::Underground {
                Phase::Hidden
            } else {
                Phase::Idle
            },
            time: 0.,
            variant: 0,
            frozen: false,
            opponents: Default::default(),
            notarget: false,
            recoil: Default::default(),
            velocity: Vec3::ZERO,
            placed: false,
            contacts: 0,
            cooldown: 0.,
            memory: 0.,
            last_seen: feet,
            random: seed as u32,
            chase_age: 0.,
            drain_clock: 0.,
            victim: None,
            offset: Vec3::ZERO,
        }
    }
    pub fn target(&self, id: usize) -> Target {
        // Larva's TIKI bounds are already server-space; the 1.75 model scale is visual.
        let half = if self.kind == Kind::Larva {
            vec3(16., 16., 12.)
        } else {
            vec3(36., 36., 22.)
        } * self.scale;
        Target {
            id,
            center: self.feet + Vec3::Z * half.z,
            half,
        }
    }
    /// A boss-emitted larva begins airborne; do not snap it to the floor on its first step.
    pub fn launch(&mut self, velocity: Vec3) {
        self.placed = true;
        self.velocity = velocity.clamp_length_max(1000.);
        self.set(Phase::Leap);
    }
    pub fn place(&mut self, world: &World) {
        if self.placed {
            return;
        }
        let b = self.target(0);
        if let Some(p) = world.actor_footing(self.feet, b.center - self.feet, b.half, 1024.) {
            self.feet = p;
        }
        self.placed = true;
    }
    pub fn set(&mut self, phase: Phase) {
        self.phase = phase;
        self.time = 0.;
        self.contacts = 0;
        if !matches!(phase, Phase::Attach | Phase::Suck) {
            self.victim = None;
        }
    }
    fn random(&mut self) -> usize {
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.random >> 16) as usize
    }
    pub fn vulnerable(&self) -> bool {
        self.health > 0. && !matches!(self.phase, Phase::Hidden | Phase::Tunnel)
    }
    pub fn loops(&self) -> bool {
        matches!(
            self.phase,
            Phase::Idle | Phase::Hidden | Phase::Walk | Phase::Run | Phase::Tunnel | Phase::Suck
        )
    }
    pub fn clip(&self) -> &'static str {
        if self.frozen {
            return "death_frozen";
        }
        match self.phase {
            Phase::Hidden => "idle_base",
            Phase::Rise => "burrow_up",
            Phase::Dive => "burrow_down",
            Phase::Walk => "walk",
            Phase::Run | Phase::Tunnel => "run",
            Phase::Melee => ["attack_slash", "attack_pincers", "attack_sting"][self.variant % 3],
            Phase::Pain => ["pain_shakeoff", "pain_folding", "pain_strong"][self.variant % 3],
            Phase::Leap => "jump_attack",
            Phase::Attach => "attack_attach",
            Phase::Suck => "attack_suck",
            Phase::Detach => "attack_death",
            Phase::Land => "jump_land",
            Phase::Dead if self.kind == Kind::Larva => "death_norm",
            Phase::Dead => ["death01", "death02"][self.variant % 2],
            _ if self.kind == Kind::Larva => "idle",
            _ => "ready",
        }
    }
    pub fn sample_time(&self, rig: &impl Timing) -> f32 {
        if self.frozen {
            self.time.min(
                rig.frame(self.kind.model(), self.clip())
                    * if self.kind == Kind::Larva { 1. } else { 9. },
            )
        } else {
            self.time
        }
    }
    pub fn visual_scale(&self, rig: &impl Timing) -> f32 {
        if matches!(self.phase, Phase::Hidden | Phase::Tunnel) {
            return 0.;
        }
        self.scale
            * if self.health > 0. {
                1.
            } else if self.frozen {
                (3. - self.time).clamp(0., 1.)
            } else {
                ((rig.duration(self.kind.model(), self.clip())
                    + if self.kind == Kind::Larva { 2. } else { 5. }
                    - self.time)
                    / 1.)
                    .clamp(0., 1.)
            }
    }
    fn sound(&self) -> Option<&'static str> {
        match (self.kind, self.phase) {
            (Kind::Larva, Phase::Leap) => Some("sound/character/larva/jump_attack.wav"),
            (Kind::Larva, Phase::Attach) => Some("sound/character/larva/attack_attach.wav"),
            (Kind::Larva, Phase::Suck) => Some("sound/character/larva/attack_suck.wav"),
            (Kind::Larva, Phase::Detach) => Some("sound/character/larva/attack_death.wav"),
            (Kind::Larva, Phase::Land) => Some("sound/character/larva/jump_land.wav"),
            (Kind::Larva, Phase::Dead) => Some("sound/character/larva/death_norm.wav"),
            (Kind::Larva, _) => None,
            (_, Phase::Rise) => Some("sound/character/antlion/anl_burrowup.wav"),
            (_, Phase::Dive) => Some("sound/character/antlion/anl_burrowdown.wav"),
            (_, Phase::Melee) => Some(
                [
                    "sound/character/antlion/anl_attkslash.wav",
                    "sound/character/antlion/anl_attkpincers.wav",
                    "sound/character/antlion/anl_attksting.wav",
                ][self.variant % 3],
            ),
            (_, Phase::Pain) => Some(
                [
                    "sound/character/antlion/anl_painshakeoff.wav",
                    "sound/character/antlion/anl_painsquirm.wav",
                    "sound/character/antlion/anl_painstrong.wav",
                ][self.variant % 3],
            ),
            (_, Phase::Dead) => Some(
                [
                    "sound/character/antlion/anl_death1.wav",
                    "sound/character/antlion/anl_death2.wav",
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
        if !self.vulnerable() || !hit.damage.is_finite() || hit.damage <= 0. {
            return None;
        }
        self.health = (self.health - hit.damage).max(0.);
        self.opponents.demon |= hit.kind.is_demon();
        self.recoil.hit(
            hit.knockback,
            if self.kind == Kind::Larva { 50. } else { 200. },
        );
        self.memory = 6.;
        self.variant = self.random() % 6;
        // The source larva pain branch plays death_norm then suicides, even after a small hit.
        if self.kind == Kind::Larva {
            self.health = 0.;
        }
        if self.health == 0. {
            self.frozen = hit.kind.means() == DamageKind::Ice;
            self.set(Phase::Dead);
            self.velocity = Vec3::ZERO;
        } else if self.phase != Phase::Pain {
            self.set(Phase::Pain);
            self.cooldown = 0.4;
        } else {
            return None;
        }
        self.sound()
    }
    fn settle(&mut self, world: &World) {
        let body = self.target(0);
        self.feet += self.recoil.step(STEP, world, body);
        let body = self.target(0);
        self.velocity.z = (self.velocity.z - 800. * STEP).max(-1000.);
        let trace = world.sweep(
            body.center,
            body.center + Vec3::Z * self.velocity.z * STEP,
            body.half,
        );
        if !trace.start_solid {
            self.feet.z += self.velocity.z * STEP * trace.fraction;
        }
        if trace.start_solid || trace.fraction < 1. {
            self.velocity.z = 0.;
        }
    }
    fn surface_clear(&self, world: &World) -> bool {
        let b = self.target(0);
        !world.sweep(b.center, b.center, b.half).start_solid
            && world.liquid_at(self.feet + Vec3::Z * 2.) == 0
            && world
                .actor_footing(self.feet, b.center - self.feet, b.half, 20.)
                .is_some()
    }
    fn walk(&mut self, world: &World, delta: Vec3, speed: f32) {
        let dir = delta.with_z(0.).normalize_or_zero();
        for angle in [0., 0.65, -0.65, 1.1, -1.1] {
            let next = combat::walk_body(
                world,
                self.feet,
                Quat::from_rotation_z(angle) * dir * speed.min(600.) * STEP * self.scale,
                self.target(0).half,
            );
            if world.liquid_at(next + Vec3::Z * 2.) == 0
                && (next - self.feet).truncate().length_squared() > 0.0001
            {
                self.feet = next;
                break;
            }
        }
    }
    pub fn step(&mut self, world: &World, eye: Vec3, rig: &impl Timing, out: &mut Feedback) {
        self.place(world);
        if self.health == 0. {
            if self.time < 20. {
                self.time = (self.time + STEP).min(20.);
                self.settle(world);
            }
            return;
        }
        self.time = (self.time + STEP).min(3600.);
        self.cooldown = (self.cooldown - STEP).max(0.);
        self.memory = (self.memory - STEP).max(0.);
        if !matches!(self.phase, Phase::Leap | Phase::Attach | Phase::Suck) {
            self.settle(world);
        }
        let from = self.target(0).center;
        let (aim, notarget, victim) =
            self.opponents
                .aim(world, from, eye, self.notarget, !self.loops());
        let delta = aim - from;
        let distance = delta.truncate().length();
        let sight = world.sweep(from, aim, Vec3::splat(0.5));
        let visible =
            !notarget && delta.length() < 1000. && !sight.start_solid && sight.fraction >= 1.;
        if visible {
            self.memory = 6.;
            self.last_seen = aim;
        }
        let facing = vec2(self.yaw.cos(), self.yaw.sin()).dot(delta.truncate().normalize_or_zero());
        if self.memory > 0. && !matches!(self.phase, Phase::Attach | Phase::Suck) {
            let d = self.last_seen - from;
            let angle = (d.y.atan2(d.x) - self.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw += angle.clamp(-5. * STEP, 5. * STEP);
        }
        if self.kind == Kind::Larva {
            self.larva(world, eye, rig, out, visible, aim, victim);
            return;
        }
        let duration = rig.duration(self.kind.model(), self.clip());
        match self.phase {
            Phase::Hidden => {
                if visible && self.surface_clear(world) {
                    self.enter(Phase::Rise, out);
                }
            }
            Phase::Idle => {
                if visible {
                    self.set(Phase::Walk);
                }
            }
            Phase::Rise | Phase::Pain => {
                if self.time >= duration {
                    self.set(Phase::Walk);
                }
            }
            Phase::Walk | Phase::Run => {
                if self.memory == 0. || notarget {
                    self.set(Phase::Idle);
                    return;
                }
                if visible
                    && distance < 100. * self.scale
                    && delta.z.abs() < 90. * self.scale
                    && facing > 0.6
                    && self.cooldown == 0.
                {
                    self.variant = self.random() % 6;
                    self.enter(Phase::Melee, out);
                    return;
                }
                let phase = if distance > 150. {
                    Phase::Run
                } else {
                    Phase::Walk
                };
                if self.phase != phase {
                    self.set(phase);
                }
                if distance > 70. * self.scale {
                    self.walk(
                        world,
                        self.last_seen - from,
                        rig.speed(self.kind.model(), self.clip()),
                    );
                }
            }
            Phase::Melee => {
                for (i, &(frame, damage)) in self.melee_events().iter().enumerate() {
                    if self.contacts & (1 << i) == 0
                        && self.time >= frame * rig.frame(self.kind.model(), self.clip())
                    {
                        self.contacts |= 1 << i;
                        if visible
                            && facing > 0.5
                            && distance < 115. * self.scale
                            && delta.z.abs() < 90. * self.scale
                        {
                            out.strike(
                                victim,
                                damage,
                                delta.normalize_or_zero() * 45.,
                                DamageKind::Other,
                            );
                        }
                    }
                }
                if self.time >= duration {
                    if self.random() % 4 != 0 && self.surface_clear(world) {
                        self.enter(Phase::Dive, out);
                    } else {
                        self.set(Phase::Walk);
                        self.cooldown = 0.6;
                    }
                }
            }
            Phase::Dive => {
                if self.time >= duration {
                    self.set(Phase::Tunnel);
                }
            }
            Phase::Tunnel => {
                if self.memory > 0. && !notarget && distance > 125. * self.scale && self.time < 4. {
                    self.walk(
                        world,
                        self.last_seen - from,
                        rig.speed(self.kind.model(), "run"),
                    );
                }
                if (self.time >= 2. && distance < 250. || self.time >= 4. || notarget)
                    && self.surface_clear(world)
                {
                    self.enter(Phase::Rise, out);
                }
                self.time = self.time.min(4.);
            }
            _ => {}
        }
    }
    pub fn melee_events(&self) -> &'static [(f32, f32)] {
        match (self.kind, self.variant % 3) {
            (Kind::Underground, 0) => &[(9., 8.), (17., 8.)],
            (_, 0) => &[(5., 8.), (10., 8.), (18., 8.)],
            (_, 1) => &[(8., 5.), (14., 5.), (19., 5.), (25., 5.), (30., 5.)],
            (Kind::Underground, _) => &[(9., 15.)],
            _ => &[(4., 15.)],
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn larva(
        &mut self,
        world: &World,
        eye: Vec3,
        rig: &impl Timing,
        out: &mut Feedback,
        visible: bool,
        aim: Vec3,
        victim: usize,
    ) {
        let duration = rig.duration(self.kind.model(), self.clip()).max(STEP);
        match self.phase {
            Phase::Idle => {
                if visible {
                    self.set(Phase::Run);
                    self.chase_age = 0.;
                }
            }
            Phase::Run => {
                self.chase_age += STEP;
                if self.chase_age >= 16. {
                    self.die(out);
                    return;
                }
                if self.memory == 0. || (self.notarget && victim == crate::dice::ALICE) {
                    self.set(Phase::Idle);
                    return;
                }
                let delta = aim - self.target(0).center;
                if visible && delta.length() < 110. * self.scale && self.cooldown == 0. {
                    self.velocity = delta.with_z(0.).normalize_or_zero() * 300.;
                    self.velocity.z = 200.;
                    self.enter(Phase::Leap, out);
                } else {
                    self.walk(
                        world,
                        self.last_seen - self.feet,
                        rig.speed(self.kind.model(), "run"),
                    );
                }
            }
            Phase::Leap => {
                self.velocity.z = (self.velocity.z - 800. * STEP).max(-1000.);
                let body = self.target(0);
                let end = body.center + self.velocity * STEP;
                let targets = self
                    .opponents
                    .bodies(eye)
                    .into_iter()
                    .filter(|t| t.id == victim && (t.id == crate::dice::SUMMON || !self.notarget))
                    .collect::<Vec<_>>();
                let contact = combat::contact_box(
                    &combat::Context {
                        world,
                        targets: &targets,
                    },
                    body.center,
                    end,
                    body.half,
                );
                let trace = world.sweep(body.center, end, body.half);
                if !trace.start_solid {
                    self.feet += self.velocity * STEP * contact.map_or(trace.fraction, |(_, f)| f);
                }
                if let Some((id, _)) = contact {
                    let target = targets.iter().find(|t| t.id == id).unwrap();
                    self.enter(Phase::Attach, out);
                    self.victim = Some(id);
                    self.offset = self.target(0).center - target.center;
                    self.velocity = Vec3::ZERO;
                } else if trace.start_solid || trace.fraction < 1. || self.time >= 1.2 {
                    self.velocity = Vec3::ZERO;
                    self.enter(Phase::Land, out);
                }
            }
            Phase::Attach | Phase::Suck => {
                let target = self.opponents.bodies(eye).into_iter().find(|t| {
                    Some(t.id) == self.victim && !(t.id == crate::dice::ALICE && self.notarget)
                });
                let Some(target) = target else {
                    self.enter(Phase::Detach, out);
                    return;
                };
                let body = self.target(0);
                let desired = target.center + self.offset;
                let trace = world.sweep(body.center, desired, body.half);
                let sight = world.sweep(desired, target.center, Vec3::splat(0.5));
                // Losing the host, crossing a wall or a teleport releases instead of dragging either body.
                if desired.distance(body.center) > 96.
                    || trace.start_solid
                    || trace.fraction < 1.
                    || sight.start_solid
                    || sight.fraction < 1.
                {
                    self.enter(Phase::Detach, out);
                    return;
                }
                self.feet += desired - body.center;
                if self.phase == Phase::Attach {
                    if self.time >= duration {
                        self.enter(Phase::Suck, out);
                        self.drain_clock = 0.;
                    }
                } else {
                    if self.time >= 3. {
                        self.enter(Phase::Detach, out);
                        return;
                    }
                    if self.time >= self.drain_clock {
                        self.drain_clock += duration;
                        out.strike(target.id, 2., Vec3::ZERO, DamageKind::Other);
                    }
                }
            }
            Phase::Detach => {
                if self.time >= duration {
                    self.die(out);
                }
            }
            Phase::Land if self.time >= duration => {
                self.set(Phase::Run);
                self.cooldown = 0.5;
            }
            _ => {}
        }
    }
    fn die(&mut self, out: &mut Feedback) {
        self.health = 0.;
        self.velocity = Vec3::ZERO;
        self.enter(Phase::Dead, out);
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.feet.is_finite()
                && self.yaw.is_finite()
                && (0.01..=10.).contains(&self.scale)
                && (0. ..=self.kind.health()).contains(&self.health)
                && (0. ..=3600.).contains(&self.time)
                && self.variant < 6
                && self.contacts < 32
                && (0. ..=0.6).contains(&self.cooldown)
                && (0. ..=6.).contains(&self.memory)
                && self.last_seen.is_finite()
                && self.recoil.valid()
                && self.velocity.is_finite()
                && self.velocity.length() <= 1100.
                && (0. ..=16.1).contains(&self.chase_age)
                && (0. ..=60.).contains(&self.drain_clock)
                && self.offset.is_finite()
                && self.offset.length() <= 200. * self.scale,
            "Invalid insect state"
        );
        ensure!(
            (self.health == 0.) == (self.phase == Phase::Dead)
                && (!self.frozen || self.health == 0.),
            "Invalid insect death"
        );
        let attached = matches!(self.phase, Phase::Attach | Phase::Suck);
        ensure!(
            self.victim.is_some() == attached
                && self
                    .victim
                    .is_none_or(|v| [crate::dice::ALICE, crate::dice::SUMMON].contains(&v)),
            "Invalid larva host"
        );
        ensure!(
            if self.kind == Kind::Larva {
                matches!(
                    self.phase,
                    Phase::Idle
                        | Phase::Run
                        | Phase::Leap
                        | Phase::Attach
                        | Phase::Suck
                        | Phase::Detach
                        | Phase::Land
                        | Phase::Dead
                )
            } else {
                !matches!(
                    self.phase,
                    Phase::Leap | Phase::Attach | Phase::Suck | Phase::Detach | Phase::Land
                )
            },
            "Insect phase/family mismatch"
        );
        Ok(())
    }
}
