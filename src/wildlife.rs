//! Remaining shared creatures. Clip timing and damage cues come from supplied data.
//! Native PhantasmAttack/SlingWeb internals are absent; see docs/wildlife.md.
use crate::{
    clockwork::Rig,
    collision::World,
    combat::{self, DamageKind, Feedback, Hit, Opponents, Target},
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
pub const DT: f32 = 1. / 120.;
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    Phantom,
    Spider,
    WallSpider,
    Jabber,
    Jabber2,
    Jabber3,
    Sleeping,
    Sleeping2,
    Sleeping3,
    Rock,
    SmallRock,
}
impl Kind {
    pub const ALL: [Self; 11] = [
        Self::Phantom,
        Self::Spider,
        Self::WallSpider,
        Self::Jabber,
        Self::Jabber2,
        Self::Jabber3,
        Self::Sleeping,
        Self::Sleeping2,
        Self::Sleeping3,
        Self::Rock,
        Self::SmallRock,
    ];
    pub fn from_model(model: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.model() == model)
    }
    pub fn model(self) -> &'static str {
        match self {
            Self::Phantom => "c_phantasm",
            Self::Spider => "c_spider",
            Self::WallSpider => "c_spider_wall",
            Self::Jabber => "c_jabberspawn",
            Self::Jabber2 => "c_jabberspawn2",
            Self::Jabber3 => "c_jabberspawn3",
            Self::Sleeping => "c_jabberspawn-asleep",
            Self::Sleeping2 => "c_jabberspawn-asleep2",
            Self::Sleeping3 => "c_jabberspawn-asleep3",
            Self::Rock => "c_walkrock_large",
            Self::SmallRock => "c_walkrock_small",
        }
    }
    pub fn rock(self) -> bool {
        matches!(self, Self::Rock | Self::SmallRock)
    }
    pub fn spider(self) -> bool {
        matches!(self, Self::Spider | Self::WallSpider)
    }
    pub fn sleeping(self) -> bool {
        matches!(self, Self::Sleeping | Self::Sleeping2 | Self::Sleeping3)
    }
    pub fn jabber(self) -> bool {
        !self.rock() && !self.spider() && self != Self::Phantom
    }
    pub fn health(self) -> f32 {
        if self.rock() {
            10000.
        } else if self.spider() {
            150.
        } else if matches!(self, Self::Sleeping2 | Self::Sleeping3) {
            250.
        } else {
            200.
        }
    }
    pub fn clips(self) -> &'static [&'static str] {
        if self.rock() {
            &["idle", "getting_up", "walk", "run", "getting_down"]
        } else if self == Self::Phantom {
            &[
                "idle",
                "fly",
                "fly_fast",
                "alert1",
                "attack_1",
                "attack_2",
                "attack_3",
                "pain1",
                "pain2",
                "pain3",
                "death_loop",
                "death2_enda",
                "death2_endb",
            ]
        } else if self.spider() {
            &[
                "idle",
                "idle_onwall",
                "alert_right",
                "walk",
                "run",
                "jump_offwall_jump",
                "jump_offwall_air",
                "jump_offwall_land",
                "attack_impale",
                "attack_poison_spray",
                "jump",
                "fall",
                "land",
                "attack_pounce_land",
                "web_spinning_start",
                "web_spinning_middle",
                "web_spinning_swing_leap",
                "pain_normal",
                "pain_strong",
                "pain_shakeoff",
                "death_regular",
                "death_stagger",
                "death_frozen",
            ]
        } else {
            &[
                "idle_base",
                "idle_sleep",
                "idle_base_2_ready",
                "idle_sleep_2_ready",
                "ready",
                "walk",
                "run",
                "attack_tail",
                "attack_claw",
                "attack_snap",
                "attack_head_charge",
                "attack_head_fire",
                "jump",
                "fall",
                "land",
                "attack_pounce_land",
                "pain_left",
                "pain_strong",
                "pain_right",
                "death01",
                "death_flip",
                "death_decap",
                "death_frozen",
            ]
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    Idle,
    Wake,
    Walk,
    Run,
    Windup,
    Melee,
    Ranged,
    Recover,
    Leap,
    Air,
    Land,
    Pain,
    DeathLoop,
    Dead,
    Wall,
    WallJump,
    WallAir,
    WallLand,
    Settle,
    WebStart,
    Web,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Creature {
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
    /// A clipped beam endpoint; rebuilt only by simulation, never a render clock.
    pub beam: Option<Vec3>,
    pub web: Option<Vec3>,
    velocity: Vec3,
    placed: bool,
    contacts: u8,
    cues: u8,
    cooldown: f32,
    pain: f32,
    charge: f32,
    random: u32,
    airborne: f32,
    recoil: combat::Recoil,
    #[serde(default)]
    detour: combat::navigation::Detour,
    #[serde(default)]
    last_seen: Option<Vec3>,
    #[serde(default)]
    memory: f32,
}
impl Creature {
    pub fn new(kind: Kind, feet: Vec3, yaw: f32, scale: f32, seed: usize) -> Self {
        Self {
            kind,
            feet,
            yaw,
            scale,
            health: kind.health(),
            phase: if kind == Kind::WallSpider {
                Phase::Wall
            } else {
                Phase::Idle
            },
            time: 0.,
            variant: 0,
            frozen: false,
            opponents: Default::default(),
            notarget: false,
            beam: None,
            web: None,
            velocity: Vec3::ZERO,
            placed: false,
            contacts: 0,
            cues: 0,
            cooldown: 0.,
            pain: 0.,
            charge: 0.6,
            random: seed as u32,
            airborne: 0.,
            recoil: Default::default(),
            detour: Default::default(),
            last_seen: None,
            memory: 0.,
        }
    }
    pub fn target(&self, id: usize) -> Target {
        let half = if self.kind == Kind::Phantom {
            vec3(32., 32., 64.)
        } else if self.kind.spider() {
            vec3(44., 44., 23.)
        } else if self.kind == Kind::Rock {
            vec3(18., 18., 16.)
        } else if self.kind == Kind::SmallRock {
            vec3(16., 16., 12.)
        } else {
            vec3(52., 52., 37.)
        } * self.scale;
        Target {
            id,
            center: self.feet + Vec3::Z * half.z,
            half,
        }
    }
    pub fn vulnerable(&self) -> bool {
        self.health > 0. && !self.kind.rock() && self.phase != Phase::Wall
    }
    pub fn place(&mut self, world: &World) {
        if self.placed {
            return;
        }
        self.placed = true;
        if self.kind == Kind::Phantom || self.kind == Kind::WallSpider {
            return;
        }
        let b = self.target(0);
        if let Some(p) = world.actor_footing(self.feet, Vec3::Z * b.half.z, b.half, 1024.) {
            self.feet = p;
        }
    }
    pub fn loops(&self) -> bool {
        matches!(
            self.phase,
            Phase::Idle | Phase::Walk | Phase::Run | Phase::Wall | Phase::Air | Phase::WallAir
        )
    }
    pub fn clip(&self) -> &'static str {
        if self.frozen && self.kind != Kind::Phantom {
            return "death_frozen";
        }
        if self.kind.rock() {
            return match self.phase {
                Phase::Wake => "getting_up",
                Phase::Walk => "walk",
                Phase::Run => "run",
                Phase::Settle => "getting_down",
                _ => "idle",
            };
        }
        if self.kind == Kind::Phantom {
            return match self.phase {
                Phase::Walk => "fly",
                Phase::Run => "fly_fast",
                Phase::Wake => "alert1",
                Phase::Windup => "attack_1",
                Phase::Melee | Phase::Ranged => "attack_2",
                Phase::Recover => "attack_3",
                Phase::Pain => ["pain1", "pain2", "pain3"][self.variant],
                Phase::DeathLoop => "death_loop",
                Phase::Dead => ["death2_enda", "death2_endb"][self.variant % 2],
                _ => "idle",
            };
        }
        if self.kind.spider() {
            return match self.phase {
                Phase::WebStart => "web_spinning_start",
                Phase::Web => "web_spinning_middle",
                Phase::Wall => "idle_onwall",
                Phase::WallJump => "jump_offwall_jump",
                Phase::WallAir => "jump_offwall_air",
                Phase::WallLand => "jump_offwall_land",
                Phase::Wake => "alert_right",
                Phase::Walk => "walk",
                Phase::Run => "run",
                Phase::Melee => "attack_impale",
                Phase::Ranged => "attack_poison_spray",
                Phase::Leap => "jump",
                Phase::Air => "fall",
                Phase::Land => "land",
                Phase::Recover => "attack_pounce_land",
                Phase::Pain => ["pain_normal", "pain_strong", "pain_shakeoff"][self.variant],
                Phase::Dead => ["death_regular", "death_stagger"][self.variant % 2],
                _ => "idle",
            };
        }
        match self.phase {
            Phase::Idle => {
                if self.kind.sleeping() {
                    "idle_sleep"
                } else {
                    "idle_base"
                }
            }
            Phase::Wake => {
                if self.kind.sleeping() {
                    "idle_sleep_2_ready"
                } else {
                    "idle_base_2_ready"
                }
            }
            Phase::Walk => "walk",
            Phase::Run => "run",
            Phase::Windup => "attack_head_charge",
            Phase::Ranged => "attack_head_fire",
            Phase::Melee => ["attack_tail", "attack_claw", "attack_snap"][self.variant],
            Phase::Leap => "jump",
            Phase::Air => "fall",
            Phase::Land => "land",
            Phase::Recover => "attack_pounce_land",
            Phase::Pain => ["pain_left", "pain_strong", "pain_right"][self.variant],
            Phase::Dead => ["death01", "death_flip", "death_decap"][self.variant],
            _ => "ready",
        }
    }
    pub fn sample_time(&self, data: &impl Rig) -> f32 {
        if self.frozen {
            self.time.min(if self.kind.jabber() {
                5. * data.frame(self.kind.model(), self.clip())
            } else {
                0.
            })
        } else {
            self.time
        }
    }
    pub fn visual_scale(&self, data: &impl Rig) -> f32 {
        self.scale
            * if self.phase == Phase::Dead || self.frozen {
                (1. - (self.time
                    - if self.frozen {
                        1.5
                    } else {
                        data.duration(self.kind.model(), self.clip())
                    })
                    / 1.5)
                    .clamp(0., 1.)
            } else {
                1.
            }
    }
    pub fn alpha(&self, data: &impl Rig) -> f32 {
        if self.kind != Kind::Phantom {
            return 1.;
        }
        let frame = (self.time / data.frame(self.kind.model(), self.clip())).floor() as usize;
        match self.phase {
            Phase::Windup => [0.12, 0.2, 0.3, 0.4, 0.5, 1., 0.7, 0.75][frame.min(7)],
            Phase::Melee | Phase::Ranged => 0.75,
            Phase::Recover => {
                if frame < 12 {
                    0.75
                } else {
                    [0.7, 1., 0.5, 0.4, 0.3, 0.2, 0.12][(frame - 12).min(6)]
                }
            }
            Phase::DeathLoop | Phase::Dead => 0.75,
            _ => 0.12,
        }
    }
    pub fn set(&mut self, phase: Phase) {
        self.phase = phase;
        self.time = 0.;
        self.contacts = 0;
        self.cues = 0;
        self.beam = None;
        if !matches!(phase, Phase::WebStart | Phase::Web) {
            self.web = None;
        }
    }
    fn random(&mut self) -> usize {
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.random >> 16) as usize
    }
    pub fn hit(&mut self, hit: Hit) -> Option<&'static str> {
        if !self.vulnerable() || !hit.damage.is_finite() || hit.damage <= 0. {
            return None;
        }
        self.opponents.demon |= hit.kind.is_demon();
        self.health = (self.health - hit.damage).max(0.);
        self.recoil.hit(hit.knockback, 200.);
        self.variant = self.random() % 3;
        if self.health == 0. {
            self.frozen = hit.kind.means() == DamageKind::Ice;
            self.velocity = Vec3::ZERO;
            self.set(if self.kind == Kind::Phantom && !self.frozen {
                Phase::DeathLoop
            } else {
                Phase::Dead
            });
        } else if self.pain == 0.
            && hit.damage
                >= if self.kind == Kind::Phantom {
                    50.
                } else if self.kind.spider() {
                    25.
                } else {
                    30.
                }
        {
            self.pain = 0.4;
            self.set(Phase::Pain);
        } else {
            return None;
        }
        // All clip sounds are dispatched through the per-actor cue reader in npc::wildlife_art.
        None
    }
    fn move_ground(&mut self, world: &World, delta: Vec3) {
        self.feet = combat::walk_body(world, self.feet, delta, self.target(0).half);
    }
    fn move_fly(&mut self, world: &World, delta: Vec3) {
        let b = self.target(0);
        let tr = world.sweep(b.center, b.center + delta, b.half);
        if !tr.start_solid {
            self.feet += delta * tr.fraction;
        }
    }
    fn leap(&mut self, delta: Vec3, wall: bool) {
        let speed = if self.kind.spider() { 450. } else { 550. };
        self.velocity =
            delta.with_z(0.).normalize_or_zero() * speed + Vec3::Z * if wall { 80. } else { 280. };
        self.airborne = 0.;
        self.set(if wall { Phase::WallJump } else { Phase::Leap });
    }
    fn flight(&mut self, world: &World) {
        self.airborne = (self.airborne + DT).min(10.);
        self.velocity.z = (self.velocity.z - 800. * DT).max(-800.);
        let b = self.target(0);
        let delta = self.velocity * DT;
        let tr = world.sweep(b.center, b.center + delta, b.half);
        if !tr.start_solid {
            self.feet += delta * tr.fraction;
        }
        if tr.start_solid || (tr.fraction < 1. && tr.normal.z > 0.65) {
            self.velocity = Vec3::ZERO;
            self.set(if matches!(self.phase, Phase::WallAir | Phase::WallJump) {
                Phase::WallLand
            } else {
                Phase::Land
            });
        } else if tr.fraction < 1. {
            self.velocity -= tr.normal * self.velocity.dot(tr.normal).min(0.);
        }
        if self.airborne >= 10. {
            self.velocity = Vec3::ZERO;
            self.set(Phase::Walk);
            self.cooldown = 2.;
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn strike(
        &mut self,
        world: &World,
        eye: Vec3,
        range: f32,
        width: f32,
        height: f32,
        damage: f32,
        once: bool,
        out: &mut Feedback,
    ) {
        let start = self.target(0).center;
        let forward = vec3(self.yaw.cos(), self.yaw.sin(), 0.);
        let targets = self.opponents.bodies(eye);
        if let Some((id, _)) = combat::contact_box(
            &combat::Context {
                world,
                targets: &targets,
            },
            start,
            start + forward * range * self.scale,
            vec3(width, width, height) * self.scale,
        ) {
            let bit = if id == crate::dice::SUMMON { 2 } else { 1 };
            if !once || self.contacts & bit == 0 {
                self.contacts |= bit;
                out.strike(id, damage, forward * 35., DamageKind::Other);
            }
        }
    }
    pub fn step(&mut self, world: &World, eye: Vec3, data: &impl Rig, out: &mut Feedback) {
        let model = self.kind.model();
        if self.health == 0. {
            self.time = (self.time + DT).min(data.duration(model, self.clip()) + 3.);
            if self.phase == Phase::DeathLoop && self.time >= data.duration(model, self.clip()) {
                self.set(Phase::Dead);
            }
            return;
        }
        self.time = (self.time + DT).min(3600.);
        if self.loops() && self.time >= 3600. {
            self.time = self.time.rem_euclid(data.duration(model, self.clip()));
        }
        self.cooldown = (self.cooldown - DT).max(0.);
        self.pain = (self.pain - DT).max(0.);
        let b = self.target(0);
        let recoil = self.recoil.step(DT, world, b);
        if self.kind == Kind::Phantom {
            self.move_fly(world, recoil);
        } else if !matches!(
            self.phase,
            Phase::Wall
                | Phase::Leap
                | Phase::Air
                | Phase::WallJump
                | Phase::WallAir
                | Phase::WebStart
                | Phase::Web
        ) {
            self.move_ground(world, recoil);
        }
        let (aim, notarget, victim) =
            self.opponents
                .aim(world, b.center, eye, self.notarget, !self.loops());
        let mut delta = aim - b.center;
        let distance = delta.length();
        let tr = world.sweep(b.center, aim, Vec3::splat(0.5));
        let visible = !notarget
            && distance
                < if self.kind == Kind::Phantom {
                    700.
                } else if self.kind.spider() {
                    1200.
                } else {
                    1000.
                }
            && !tr.start_solid
            && tr.fraction >= 1.;
        if self.kind.spider() {
            if visible { self.last_seen = Some(aim); self.memory = 3.; }
            else { self.memory = (self.memory - DT).max(0.); }
            if notarget { self.memory = 0.; self.detour.clear(); }
            if !visible && self.memory > 0. { delta = self.last_seen.unwrap_or(aim) - b.center; }
        }
        let pursuing = visible || (self.kind.spider() && self.memory > 0.);
        if self.kind.rock() {
            self.rock_step(world, delta, visible, data);
            return;
        }
        if pursuing
            && matches!(
                self.phase,
                Phase::Idle | Phase::Wake | Phase::Walk | Phase::Run | Phase::Windup
            )
        {
            let angle = (delta.y.atan2(delta.x) - self.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw =
                (self.yaw + angle.clamp(-2.5 * DT, 2.5 * DT)).rem_euclid(std::f32::consts::TAU);
        }
        let facing = delta.truncate().length_squared() < 1.
            || vec2(self.yaw.cos(), self.yaw.sin()).dot(delta.truncate().normalize_or_zero()) > 0.8;
        let duration = data.duration(model, self.clip());
        let frame = data.frame(model, self.clip());
        match self.phase {
            Phase::Idle => {
                if visible {
                    self.set(Phase::Wake);
                }
            }
            Phase::Wall => {
                if visible {
                    // Authored facing controls departure. A blocked hull stays hanging, rather than teleporting out.
                    let forward = vec3(self.yaw.cos(), self.yaw.sin(), 0.);
                    let tr = world.sweep(b.center, b.center + forward * 12., b.half);
                    if !tr.start_solid && tr.fraction >= 1. {
                        self.leap(forward, true);
                    }
                }
            }
            Phase::Wake | Phase::Pain | Phase::WallLand => {
                if self.time >= duration {
                    self.set(Phase::Walk);
                    self.cooldown = 0.35;
                }
            }
            Phase::Walk | Phase::Run => {
                if !pursuing {
                    if self.kind.sleeping() {
                        self.time = self.time.min(duration);
                    } else {
                        self.set(Phase::Idle);
                    }
                    return;
                }
                if visible && facing && self.cooldown == 0. {
                    if self.kind == Kind::Phantom
                        && (distance < 110. || (distance > 150. && self.time >= duration))
                    {
                        self.variant = usize::from(distance >= 110.);
                        self.set(Phase::Windup);
                        return;
                    }
                    if self.kind != Kind::Phantom {
                        if distance < if self.kind.spider() { 110. } else { 130. } * self.scale {
                            self.variant = self.random() % 3;
                            self.set(Phase::Melee);
                            return;
                        }
                        if self.kind.spider() && distance < 275. {
                            self.set(Phase::Ranged);
                            return;
                        }
                        if self.kind.jabber() && self.time >= duration && self.random() % 2 == 0 {
                            self.charge = 0.6 + (self.random() % 901) as f32 / 1000.;
                            self.set(Phase::Windup);
                            return;
                        }
                        if distance > 200. && distance < 600. && self.time >= duration {
                            if self.kind.spider() && distance > 300. && self.random() % 2 == 0 {
                                let top = b.center + Vec3::Z * 400.;
                                let ceiling = world.sweep(b.center, top, Vec3::splat(1.));
                                if !ceiling.start_solid
                                    && ceiling.fraction < 1.
                                    && ceiling.fraction > 0.15
                                {
                                    self.web = Some(b.center.lerp(top, ceiling.fraction));
                                    self.set(Phase::WebStart);
                                    return;
                                }
                            }
                            self.leap(delta, false);
                            return;
                        }
                    }
                }
                let run = distance > 500.;
                if self.kind != Kind::Phantom && run != (self.phase == Phase::Run) {
                    self.set(if run { Phase::Run } else { Phase::Walk });
                }
                let motion = if self.kind == Kind::Phantom {
                    delta.normalize_or_zero()
                } else {
                    delta.with_z(0.).normalize_or_zero()
                } * data.speed(model, self.clip()).clamp(20., 320.)
                    * self.scale
                    * DT;
                if self.kind == Kind::Phantom {
                    self.move_fly(world, motion);
                } else if self.kind.spider() {
                    self.feet = self.detour.walk(world, self.feet, self.feet + delta.with_z(0.),
                        self.target(0).half, motion.length(), DT);
                    if !visible && delta.truncate().length() < 12. { self.memory = 0.; }
                } else {
                    self.move_ground(world, motion);
                }
            }
            Phase::Windup => {
                if !visible || (self.kind == Kind::Phantom && self.variant == 0 && distance > 125.)
                {
                    self.set(Phase::Recover);
                } else if self.time
                    >= if self.kind == Kind::Phantom {
                        duration
                    } else {
                        self.charge
                    }
                {
                    self.set(if self.kind == Kind::Phantom && self.variant == 0 {
                        Phase::Melee
                    } else {
                        Phase::Ranged
                    });
                }
            }
            Phase::Melee => {
                if self.kind == Kind::Phantom {
                    if self.cues == 0 && self.time >= duration * 0.5 {
                        self.cues = 1;
                        if visible && distance <= 125. {
                            if victim == crate::dice::ALICE {
                                out.will_drain += 10.;
                            } else {
                                out.strike(victim, 10., Vec3::ZERO, DamageKind::Other);
                            }
                            self.beam = Some(aim);
                        }
                    }
                } else {
                    let (cue, damage) = if self.kind.spider() {
                        (8., 10.)
                    } else {
                        [(10., 10.), (7., 20.), (9., 10.)][self.variant]
                    };
                    if self.cues == 0 && self.time >= cue * frame {
                        self.cues = 1;
                        self.strike(
                            world,
                            eye,
                            if self.kind.spider() {
                                if self.kind == Kind::WallSpider {
                                    110.
                                } else {
                                    100.
                                }
                            } else {
                                130.
                            },
                            if self.kind.spider() { 4. } else { 24. },
                            if self.kind.spider() { 2. } else { 32. },
                            damage,
                            true,
                            out,
                        );
                    }
                }
                if self.time >= duration {
                    self.set(if self.kind == Kind::Phantom {
                        Phase::Recover
                    } else {
                        Phase::Walk
                    });
                    self.cooldown = 0.6;
                }
            }
            Phase::Ranged => {
                if self.kind.spider() {
                    let cues: &[usize] = if self.kind == Kind::WallSpider {
                        &[9, 11, 13, 16, 19, 22]
                    } else {
                        &[9, 12, 15, 18, 21, 24, 27]
                    };
                    for (i, cue) in cues.iter().enumerate() {
                        if self.cues & (1 << i) == 0 && self.time >= *cue as f32 * frame {
                            self.cues |= 1 << i;
                            self.strike(world, eye, 270., 8., 4., 10., true, out);
                        }
                    }
                } else if self.kind == Kind::Phantom {
                    if visible && self.time <= duration.min(0.75) {
                        self.beam = Some(aim);
                        out.strike(
                            victim,
                            0.,
                            -delta.normalize_or_zero() * 160. * DT,
                            DamageKind::Other,
                        );
                    } else {
                        self.beam = None;
                    }
                } else if self.time >= frame && self.time < frame + 0.5 && visible {
                    let from = self.feet
                        + Quat::from_rotation_z(self.yaw)
                            * data.tag(self.clip(), self.time, "tag_beam").translation
                            * self.scale;
                    let direction = (aim - from).normalize_or_zero();
                    let muzzle = world.sweep(b.center, from, Vec3::splat(0.5));
                    if !muzzle.start_solid && muzzle.fraction >= 1. {
                        let end = from + direction * 1000.;
                        let ray = world.sweep(from, end, Vec3::splat(0.5));
                        self.beam = Some(from.lerp(end, ray.fraction));
                        if self.cues == 0 {
                            self.cues = 1;
                            let targets = self.opponents.bodies(eye);
                            if let Some((id, f)) = combat::contact(
                                &combat::Context {
                                    world,
                                    targets: &targets,
                                },
                                from,
                                end,
                                0.5,
                            ) {
                                self.beam = Some(from.lerp(end, f));
                                out.strike(id, 5., direction * 20., DamageKind::Other);
                            }
                        }
                    }
                } else {
                    self.beam = None;
                }
                if self.time >= duration {
                    self.set(if self.kind == Kind::Phantom {
                        Phase::Recover
                    } else {
                        Phase::Walk
                    });
                    self.cooldown = 0.8;
                }
            }
            Phase::WebStart | Phase::Web => {
                if !visible {
                    self.leap(Vec3::ZERO, false);
                } else if self.phase == Phase::WebStart && self.time >= duration {
                    self.set(Phase::Web);
                } else if self.phase == Phase::Web {
                    if let Some(anchor) = self.web {
                        if anchor.z - self.target(0).center.z > self.target(0).half.z + 12. {
                            self.move_fly(world, Vec3::Z * 80. * DT);
                        }
                    }
                    if self.time >= duration.min(0.75) {
                        self.leap(delta, false);
                    }
                }
            }
            Phase::Leap | Phase::Air | Phase::WallJump | Phase::WallAir => {
                self.flight(world);
                if matches!(self.phase, Phase::Leap | Phase::WallJump) && self.time >= duration {
                    self.set(if self.phase == Phase::Leap {
                        Phase::Air
                    } else {
                        Phase::WallAir
                    });
                }
            }
            Phase::Land => {
                if self.kind.spider() {
                    self.strike(world, eye, 110., 4., 2., 10., true, out);
                } else if self.cues == 0 && self.time >= 2. * frame {
                    self.cues = 1;
                    self.strike(world, eye, 130., 24., 32., 20., true, out);
                }
                if self.time >= duration {
                    self.set(Phase::Recover);
                }
            }
            Phase::Recover if self.time >= duration => {
                self.set(Phase::Walk);
                self.cooldown = 0.8;
            }
            _ => {}
        }
    }
    fn rock_step(&mut self, world: &World, delta: Vec3, visible: bool, data: &impl Rig) {
        let distance = delta.length();
        let duration = data.duration(self.kind.model(), self.clip());
        match self.phase {
            Phase::Idle => {
                if visible && distance < 150. {
                    self.set(Phase::Wake);
                }
            }
            Phase::Wake => {
                if self.time >= duration {
                    self.set(Phase::Walk);
                }
            }
            Phase::Walk | Phase::Run => {
                if !visible || distance > 180. {
                    self.set(Phase::Settle);
                    return;
                }
                let run = distance < 100.;
                if run != (self.phase == Phase::Run) {
                    self.set(if run { Phase::Run } else { Phase::Walk });
                }
                let dir = -delta.with_z(0.).normalize_or_zero();
                self.yaw = dir.y.atan2(dir.x);
                self.move_ground(
                    world,
                    dir * data.speed(self.kind.model(), self.clip()).clamp(10., 240.)
                        * self.scale
                        * DT,
                );
            }
            Phase::Settle if self.time >= duration => {
                self.set(Phase::Idle);
            }
            _ => {}
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.detour.valid()
                && self.memory.is_finite() && (0. ..=3.).contains(&self.memory)
                && self.last_seen.is_none_or(|p| p.is_finite() && p.abs().max_element() < 100_000.)
                && self.feet.is_finite()
                && self.yaw.is_finite()
                && (0.01..=10.).contains(&self.scale)
                && (0. ..=self.kind.health()).contains(&self.health)
                && (0. ..=3600.).contains(&self.time)
                && self.variant < 3
                && self.velocity.is_finite()
                && self.velocity.length() < 2000.
                && (0. ..=2.).contains(&self.cooldown)
                && (0. ..=0.4).contains(&self.pain)
                && (0.6..=1.5).contains(&self.charge)
                && (0. ..=10.).contains(&self.airborne)
                && self.contacts < 4
                && self.recoil.valid()
                && self.beam.is_none_or(|b| b.is_finite()),
            "Invalid creature state"
        );
        ensure!(
            (self.health == 0.) == matches!(self.phase, Phase::Dead | Phase::DeathLoop)
                && (!self.frozen || self.health == 0.),
            "Invalid creature death"
        );
        ensure!(
            !self.kind.rock()
                || (self.health == self.kind.health()
                    && matches!(
                        self.phase,
                        Phase::Idle | Phase::Wake | Phase::Walk | Phase::Run | Phase::Settle
                    )),
            "Invalid Walkrock state"
        );
        ensure!(
            !matches!(
                self.phase,
                Phase::Wall | Phase::WallJump | Phase::WallAir | Phase::WallLand
            ) || self.kind == Kind::WallSpider,
            "Invalid wall state"
        );
        ensure!(
            !matches!(self.phase, Phase::DeathLoop) || self.kind == Kind::Phantom,
            "Invalid ghost death"
        );
        ensure!(
            self.web.is_none_or(|p| p.is_finite()
                && self.kind.spider()
                && matches!(self.phase, Phase::WebStart | Phase::Web)),
            "Invalid web anchor"
        );
        ensure!(
            self.beam.is_none() || matches!(self.phase, Phase::Melee | Phase::Ranged),
            "Stale beam"
        );
        Ok(())
    }
}
#[cfg(test)]
mod tests;
