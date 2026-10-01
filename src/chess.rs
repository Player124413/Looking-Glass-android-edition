//! Red chess combat reconstructed from the supplied model declarations and animation data.
//! Scripted royalty and white scene actors keep their existing owners.
use crate::{
    ant::Timing,
    collision::World,
    combat::{DamageKind, Feedback, Hit, Opponents, Recoil, Target},
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    Pawn,
    Knight,
    Bishop,
    Rook,
}
impl Kind {
    pub const ALL: [Self; 4] = [Self::Pawn, Self::Knight, Self::Bishop, Self::Rook];
    pub fn from_model(model: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.model() == model)
    }
    pub fn model(self) -> &'static str {
        match self {
            Self::Pawn => "c_chess_red_pawn",
            Self::Knight => "c_chess_red_knight",
            Self::Bishop => "c_chess_red_bishop",
            Self::Rook => "c_chess_red_rook",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Pawn => "Red pawn",
            Self::Knight => "Red knight",
            Self::Bishop => "Red bishop",
            Self::Rook => "Red rook",
        }
    }
    pub fn health(self) -> f32 {
        match self {
            Self::Pawn => 75.,
            Self::Knight => 150.,
            Self::Bishop => 125.,
            Self::Rook => 200.,
        }
    }
    pub fn half(self) -> Vec3 {
        match self {
            Self::Pawn => vec3(24., 24., 32.),
            Self::Knight | Self::Rook => vec3(32., 32., 40.),
            Self::Bishop => vec3(30., 30., 54.),
        }
    }
    pub fn grade(self) -> crate::loot::Grade {
        match self {
            Self::Pawn => crate::loot::Grade::Small,
            Self::Bishop => crate::loot::Grade::Medium,
            _ => crate::loot::Grade::Large,
        }
    }
    pub fn clips(self) -> &'static [&'static str] {
        match self {
            Self::Pawn => &[
                "idle",
                "walk",
                "attack_1",
                "pain1",
                "pain2",
                "pain3",
                "death1",
                "death_back",
                "death_frozen",
            ],
            Self::Knight => &[
                "idle",
                "stand_2_ready",
                "walk_1",
                "walk_2",
                "attack_1",
                "block_1",
                "block_2",
                "pain_front",
                "pain_right",
                "pain_left",
                "death1",
                "death2",
                "death_frozen",
            ],
            Self::Bishop => &[
                "idlea",
                "walk_1",
                "walk_2",
                "attack_1",
                "attack_range",
                "pain1",
                "pain2",
                "pain_back",
                "death",
                "death_back",
                "death_frozen",
            ],
            Self::Rook => &[
                "idle",
                "walk_1",
                "walk_2",
                "attack_1_a",
                "attack_1_b",
                "attack_2_a",
                "attack_2_b",
                "attack_3",
                "pain1",
                "pain_back",
                "pain3",
                "death1",
                "death_back",
                "death_frozen",
            ],
        }
    }
}
pub fn script_wait(map: &str, name: &str) -> bool {
    match map {
        "wchess1" => name == "red_knight_bully",
        "wchess2" => matches!(name, "r_knight_queen1" | "r_knight_queen2"),
        "rchess1" => name.starts_with("spec_") || name.starts_with("king_"),
        _ => false,
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    Idle,
    Ready,
    Chase,
    Melee,
    Beam,
    Charge,
    ChargeHit,
    Block,
    BlockEnd,
    Pain,
    Dead,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Beam {
    pub from: Vec3,
    pub to: Vec3,
    pub age: f32,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Piece {
    pub kind: Kind,
    pub active: bool,
    pub suppressed: bool,
    pub spawn_delay: Option<f32>,
    pub script_wait: bool,
    pub feet: Vec3,
    pub yaw: f32,
    pub scale: f32,
    pub health: f32,
    pub phase: Phase,
    pub time: f32,
    pub variant: usize,
    pub frozen: bool,
    pub fired: bool,
    pub beam: Option<Beam>,
    pub opponents: Opponents,
    #[serde(skip)]
    pub notarget: bool,
    recoil: Recoil,
    accumulator: f32,
    pub(crate) falling: f32,
    pub(crate) memory: f32,
    last_seen: Vec3,
    block_cooldown: f32,
    random: u32,
}
impl Piece {
    pub fn new(kind: Kind, feet: Vec3, yaw: f32, scale: f32, seed: usize) -> Self {
        Self {
            kind,
            active: true,
            suppressed: false,
            spawn_delay: None,
            script_wait: false,
            feet,
            yaw,
            scale,
            health: kind.health(),
            phase: Phase::Idle,
            time: 0.,
            variant: 0,
            frozen: false,
            fired: false,
            beam: None,
            opponents: Opponents::default(),
            notarget: false,
            recoil: Recoil::default(),
            accumulator: 0.,
            falling: 0.,
            memory: 0.,
            last_seen: feet,
            block_cooldown: 0.,
            random: seed as u32,
        }
    }
    pub fn target(&self, id: usize) -> Target {
        let half = self.kind.half() * self.scale;
        Target {
            id,
            center: self.feet + Vec3::Z * half.z,
            half,
        }
    }
    pub fn loops(&self) -> bool {
        matches!(
            self.phase,
            Phase::Idle | Phase::Chase | Phase::Charge | Phase::Block
        )
    }
    pub fn clip(&self) -> &'static str {
        match self.phase {
            Phase::Idle => {
                if self.kind == Kind::Bishop {
                    "idlea"
                } else {
                    "idle"
                }
            }
            Phase::Ready => "stand_2_ready",
            Phase::Chase => {
                if self.kind == Kind::Pawn {
                    "walk"
                } else {
                    "walk_1"
                }
            }
            Phase::Melee => {
                if self.kind == Kind::Rook {
                    "attack_3"
                } else {
                    "attack_1"
                }
            }
            Phase::Beam => "attack_range",
            Phase::Charge => {
                if self.variant % 2 == 0 {
                    "attack_1_a"
                } else {
                    "attack_2_a"
                }
            }
            Phase::ChargeHit => {
                if self.variant % 2 == 0 {
                    "attack_1_b"
                } else {
                    "attack_2_b"
                }
            }
            Phase::Block => "block_1",
            Phase::BlockEnd => "block_2",
            Phase::Pain => match self.kind {
                Kind::Pawn => ["pain1", "pain2", "pain3"][self.variant],
                Kind::Knight => ["pain_front", "pain_right", "pain_left"][self.variant],
                Kind::Bishop => ["pain1", "pain2", "pain_back"][self.variant],
                Kind::Rook => ["pain1", "pain_back", "pain3"][self.variant],
            },
            Phase::Dead if self.frozen => "death_frozen",
            Phase::Dead => match self.kind {
                Kind::Knight => ["death1", "death2"][self.variant % 2],
                Kind::Bishop => ["death", "death_back"][self.variant % 2],
                _ => ["death1", "death_back"][self.variant % 2],
            },
        }
    }
    pub fn visual_scale(&self, data: &impl Timing) -> f32 {
        if !self.active {
            return 0.;
        }
        if self.health > 0. {
            return self.scale;
        }
        self.scale
            * (1. - (self.time - data.duration(self.kind.model(), self.clip()) - 5.).max(0.) / 2.)
                .clamp(0., 1.)
    }
    fn set(&mut self, phase: Phase) {
        self.phase = phase;
        self.time = 0.;
        self.fired = false;
    }
    fn roll(&mut self) {
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        self.variant = self.random as usize % 3;
    }
    pub fn threatened(&mut self, incoming: bool) {
        if incoming
            && self.active
            && !self.script_wait
            && self.kind == Kind::Knight
            && self.block_cooldown == 0.
            && matches!(self.phase, Phase::Idle | Phase::Chase)
        {
            self.set(Phase::Block);
            self.block_cooldown = 2.;
        }
    }
    pub fn hit(&mut self, hit: Hit) -> Option<&'static str> {
        if !self.active || self.health <= 0. || !hit.damage.is_finite() || hit.damage <= 0. {
            return None;
        }
        self.opponents.demon |= hit.kind.is_demon();
        // Directional shield: the impact impulse travels toward the actor. Rear hits,
        // energy, splash and impacts lacking direction remain effective.
        let front = hit
            .knockback
            .truncate()
            .normalize_or_zero()
            .dot(vec2(self.yaw.cos(), self.yaw.sin()))
            < -0.5;
        let blockable = matches!(hit.kind.means(), DamageKind::Knife | DamageKind::Cards);
        if front
            && blockable
            && self.kind == Kind::Knight
            && !self.script_wait
            && (self.phase == Phase::Block
                || (self.block_cooldown == 0. && matches!(self.phase, Phase::Idle | Phase::Chase)))
        {
            self.set(Phase::BlockEnd);
            self.block_cooldown = 2.;
            return Some("sound/character/chess_piece/knight/block_2.wav");
        }
        self.health = (self.health - hit.damage).max(0.);
        self.recoil.hit(hit.knockback, 100.);
        if self.phase != Phase::Pain || self.health == 0. {
            self.roll();
        }
        if self.health == 0. {
            self.frozen = hit.kind.means() == DamageKind::Ice;
            self.set(Phase::Dead);
        } else if self.phase != Phase::Pain {
            self.set(Phase::Pain);
        }
        Some(self.sound())
    }
    fn sound(&self) -> &'static str {
        match (self.kind, self.health <= 0.) {
            (Kind::Pawn, false) => "sound/character/chess_piece/pawn/pain1.wav",
            (Kind::Pawn, true) => "sound/character/chess_piece/pawn/death1.wav",
            (Kind::Knight, false) => "sound/character/chess_piece/knight/pain_front.wav",
            (Kind::Knight, true) => "sound/character/chess_piece/knight/death1.wav",
            (Kind::Bishop, false) => "sound/character/chess_piece/bishop/pain1.wav",
            (Kind::Bishop, true) => "sound/character/chess_piece/bishop/death.wav",
            (Kind::Rook, false) => "sound/character/chess_piece/rook/pain1.wav",
            (Kind::Rook, true) => "sound/character/chess_piece/rook/death1.wav",
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
        if !dt.is_finite() || dt <= 0. || self.suppressed {
            return;
        }
        if !self.active && self.spawn_delay.is_none() {
            return;
        }
        self.accumulator += dt.min(0.1);
        while self.accumulator + 0.000001 >= 1. / 120. {
            self.accumulator = (self.accumulator - 1. / 120.).max(0.);
            if let Some(delay) = &mut self.spawn_delay {
                *delay = (*delay - 1. / 120.).max(0.);
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
        let dt = 1. / 120.;
        self.time += dt;
        self.block_cooldown = (self.block_cooldown - dt).max(0.);
        self.feet += self.recoil.step(dt, world, self.target(0));
        self.falling = (self.falling + 800. * dt).min(800.);
        let body = self.target(0);
        let down = world.sweep(
            body.center,
            body.center - Vec3::Z * self.falling * dt,
            body.half,
        );
        if !down.start_solid {
            self.feet.z -= self.falling * dt * down.fraction;
        }
        if down.start_solid || down.fraction < 1. {
            self.falling = 0.;
        }
        if let Some(b) = &mut self.beam {
            b.age += dt;
            if b.age >= 0.125 {
                self.beam = None;
            }
        }
        let duration = data.duration(self.kind.model(), self.clip());
        if self.phase == Phase::Dead {
            self.time = self.time.min(duration + 7.);
            return;
        }
        if self.script_wait {
            if self.phase == Phase::Pain && self.time >= duration {
                self.set(Phase::Idle);
            }
            return;
        }
        let player_eye = eye;
        let (eye, notarget, victim) = self.opponents.aim(
            world,
            self.target(0).center,
            eye,
            self.notarget,
            matches!(
                self.phase,
                Phase::Melee | Phase::Beam | Phase::Charge | Phase::ChargeHit
            ),
        );
        let delta = eye - self.target(0).center;
        let distance = delta.truncate().length();
        let sight = world.sweep(self.target(0).center, eye, Vec3::splat(0.5));
        let visible =
            !notarget && delta.length() < 800. && !sight.start_solid && sight.fraction >= 1.;
        if visible {
            self.memory = 3.;
            self.last_seen = eye;
        } else {
            self.memory = (self.memory - dt).max(0.);
        }
        if notarget {
            self.memory = 0.;
        }
        let pursuing = visible || self.memory > 0.;
        let toward = if visible {
            delta
        } else {
            self.last_seen - self.target(0).center
        };
        if pursuing && !matches!(self.phase, Phase::Pain | Phase::Block | Phase::BlockEnd) {
            let turn = (toward.y.atan2(toward.x) - self.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw += turn.clamp(-10. * dt, 10. * dt);
        }
        let facing =
            vec2(self.yaw.cos(), self.yaw.sin()).dot(delta.truncate().normalize_or_zero()) > 0.6;
        let reach = if self.kind == Kind::Pawn { 85. } else { 100. } * self.scale;
        match self.phase {
            Phase::Melee | Phase::ChargeHit | Phase::Beam => {
                let frame = match self.phase {
                    Phase::ChargeHit => 2.,
                    Phase::Beam => 10.,
                    _ => match self.kind {
                        Kind::Pawn | Kind::Knight => 8.,
                        Kind::Bishop => 10.,
                        Kind::Rook => 9.,
                    },
                };
                if !self.fired && self.time >= frame * data.frame(self.kind.model(), self.clip()) {
                    self.fired = true;
                    if self.phase == Phase::Beam {
                        // Source beam lifetime/range/damage. Clip the visual to the same sweep used for contact.
                        let from = self.feet + Vec3::Z * 72. * self.scale;
                        let end = from + (eye - from).normalize_or_zero() * 500.;
                        let wall = world.sweep(from, end, Vec3::splat(1.));
                        if visible && facing && !wall.start_solid {
                            let bodies = self.opponents.bodies(player_eye);
                            let contact = crate::combat::contact(
                                &crate::combat::Context {
                                    world,
                                    targets: &bodies,
                                },
                                from,
                                end,
                                1.,
                            );
                            self.beam = Some(Beam {
                                from,
                                to: from.lerp(end, contact.map_or(wall.fraction, |c| c.1)),
                                age: 0.,
                            });
                            if let Some((id, _)) = contact {
                                out.strike(
                                    id,
                                    8.,
                                    (eye - from).normalize_or_zero() * 12.,
                                    DamageKind::Electric,
                                );
                            }
                            out.spatial_sounds
                                .push(("sound/character/chess_piece/bishop/attack_2.wav", from));
                        }
                    } else if visible
                        && facing
                        && distance < reach + 12.
                        && delta.z.abs() < 72. * self.scale
                    {
                        out.strike(
                            victim,
                            match self.kind {
                                Kind::Pawn => 20.,
                                Kind::Knight => 10.,
                                _ => 15.,
                            },
                            delta.normalize_or_zero() * 45.,
                            DamageKind::Other,
                        );
                        out.spatial_sounds.push((
                            match self.kind {
                                Kind::Pawn => "sound/character/chess_piece/pawn/attack_1.wav",
                                Kind::Knight => "sound/character/chess_piece/knight/attack_1.wav",
                                Kind::Bishop => "sound/character/chess_piece/bishop/attack_1.wav",
                                Kind::Rook => "sound/character/chess_piece/rook/attack_3.wav",
                            },
                            self.feet,
                        ));
                    }
                }
                if self.time >= duration {
                    self.set(Phase::Chase);
                }
            }
            Phase::Charge => {
                if visible && distance <= reach {
                    self.set(Phase::ChargeHit);
                } else if !visible
                    || self.time > 3.
                    || !self.walk(
                        world,
                        toward,
                        data.speed(self.kind.model(), self.clip()) * dt,
                        false,
                    )
                {
                    self.set(Phase::Chase);
                }
            }
            Phase::Block if self.time >= 0.6 => self.set(Phase::BlockEnd),
            Phase::Ready | Phase::Pain | Phase::BlockEnd if self.time >= duration => {
                self.set(Phase::Chase)
            }
            Phase::Idle | Phase::Chase if pursuing => {
                if self.phase == Phase::Idle && self.kind == Kind::Knight {
                    self.set(Phase::Ready);
                } else if visible && facing && distance <= reach && delta.z.abs() < 72. * self.scale
                {
                    self.set(Phase::Melee);
                } else if visible
                    && facing
                    && self.kind == Kind::Bishop
                    && distance <= 500.
                    && self.time >= 0.35
                {
                    self.set(Phase::Beam);
                } else if visible
                    && facing
                    && self.kind == Kind::Rook
                    && distance < 750.
                    && self.time >= 1.
                {
                    self.roll();
                    self.set(Phase::Charge);
                } else {
                    if self.phase == Phase::Idle {
                        self.set(Phase::Chase);
                    }
                    if toward.truncate().length() > 24. {
                        self.walk(
                            world,
                            toward,
                            data.speed(self.kind.model(), self.clip()) * dt,
                            true,
                        );
                    } else if !visible {
                        self.memory = 0.;
                    }
                    // Loop the rendered clip, but retain the short attack decision timer.
                    self.time = self.time.min(10.);
                }
            }
            Phase::Chase if !pursuing => self.set(Phase::Idle),
            _ => {}
        }
    }
    fn walk(&mut self, world: &World, delta: Vec3, step: f32, sidestep: bool) -> bool {
        let direction = delta.truncate().extend(0.).normalize_or_zero();
        for angle in if sidestep {
            &[0., 0.65, -0.65, 1.1, -1.1][..]
        } else {
            &[0.][..]
        } {
            let next = crate::combat::walk_body(
                world,
                self.feet,
                Quat::from_rotation_z(*angle) * direction * step * self.scale,
                self.kind.half() * self.scale,
            );
            if next.truncate().distance_squared(self.feet.truncate()) > 0.0001 {
                self.feet = next;
                return true;
            }
        }
        false
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (!self.suppressed || (!self.active && self.spawn_delay.is_none()))
                && self
                    .spawn_delay
                    .is_none_or(|t| !self.active && (0. ..=30.).contains(&t))
                && self.variant < 3
                && self.feet.is_finite()
                && self.feet.abs().max_element() < 100_000.
                && self.yaw.is_finite()
                && (0.01..=10.).contains(&self.scale)
                && (0. ..=self.kind.health()).contains(&self.health)
                && (self.health == 0.) == (self.phase == Phase::Dead)
                && (!self.frozen || self.health == 0.)
                && self.recoil.valid()
                && self.last_seen.is_finite()
                && (0. ..=3.).contains(&self.memory)
                && (0. ..=2.).contains(&self.block_cooldown)
                && (0. ..=800.).contains(&self.falling)
                && (0. ..=0.01).contains(&self.accumulator)
                && self.time.is_finite()
                && (0. ..=1e6).contains(&self.time)
                && self.beam.as_ref().is_none_or(|b| self.kind == Kind::Bishop
                    && b.from.is_finite()
                    && b.to.is_finite()
                    && b.from.distance(b.to) <= 501.
                    && (0. ..=0.125).contains(&b.age))
                && (!matches!(self.phase, Phase::Ready | Phase::Block | Phase::BlockEnd)
                    || self.kind == Kind::Knight)
                && (self.phase != Phase::Beam || self.kind == Kind::Bishop)
                && (!matches!(self.phase, Phase::Charge | Phase::ChargeHit)
                    || self.kind == Kind::Rook),
            "Invalid saved chess piece"
        );
        Ok(())
    }
}
