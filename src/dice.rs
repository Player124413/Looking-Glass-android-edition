//! Independently authored Demon Dice simulation. Original assets supply poses and event frames.
use crate::{
    assets::Assets,
    collision::World,
    combat::{self, Context, Hit, Target},
    skeletal::{Animation, Definition, Skeleton, Transform},
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

// Stable reserved combat IDs also fit the save format's bounded numeric domain.
pub const ALICE: usize = 5_000_000;
// Separate from Duchess::ID: a player shot must reach the boss damage handler.
pub const SUMMON: usize = 4_500_000;
const STEP: f32 = 1. / 120.;
pub const MODELS: [&str; 3] = ["c_demon_lesser", "c_demon_normal", "c_demon_king"];
const IDLE: [&str; 3] = ["idle_fly1", "idle1", "idle"];
const MOVE: [&str; 3] = ["fly_fast", "walk_fast", "walk"];
const MELEE: [&str; 3] = ["melee", "attack1", "melee1a"];
const RANGED: [&str; 3] = ["ranged", "attack3", "attack2"];
const SIZE: [f32; 3] = [1.25, 1.5, 2.8];
const HEALTH: [f32; 3] = [50., 100., 200.];
const TOSS: &str = "sound/weapon/dice/dice_toss.wav";
const OPEN: &str = "sound/weapon/dice/dice_rift_open.wav";
const CLOSE: &str = "sound/weapon/dice/dice_rift_close.wav";
pub const SOUNDS: [&str; 42] = [
    TOSS,
    OPEN,
    CLOSE,
    "sound/weapon/shared/small_drop.wav",
    "sound/character/demon/lesser/dmnlsr_attkfist.wav",
    "sound/character/demon/lesser/dmnlsr_attkltng.wav",
    "sound/character/demon/normal/dmnnrm_attk1.wav",
    "sound/character/demon/normal/dmnnrm_attk3.wav",
    "sound/character/demon/king/king_melee1.wav",
    "sound/character/demon/king/king_attack2.wav",
    "sound/character/demon/king/king_attack2.wav",
    "sound/character/demon/king/king_breath1.wav",
    "sound/character/demon/king/king_death.wav",
    "sound/character/demon/king/king_death2.wav",
    "sound/character/demon/king/king_fly.wav",
    "sound/character/demon/king/king_idle1.wav",
    "sound/character/demon/king/king_melee1.wav",
    "sound/character/demon/king/king_melee2.wav",
    "sound/character/demon/king/king_pain1.wav",
    "sound/character/demon/king/king_pain2.wav",
    "sound/character/demon/king/king_pain3.wav",
    "sound/character/demon/king/king_runattack.wav",
    "sound/character/demon/king/king_runattack_start.wav",
    "sound/character/demon/king/step1.wav",
    "sound/character/demon/king/step2.wav",
    "sound/character/demon/lesser/dmnlsr_attkfist.wav",
    "sound/character/demon/lesser/dmnlsr_attkltng.wav",
    "sound/character/demon/lesser/dmnlsr_death2.wav",
    "sound/character/demon/lesser/dmnlsr_pain1.wav",
    "sound/character/demon/lesser/dmnlsr_pain2.wav",
    "sound/character/demon/lesser/dmnlsr_pain3.wav",
    "sound/character/demon/normal/dmnnrm_attk1.wav",
    "sound/character/demon/normal/dmnnrm_attk2.wav",
    "sound/character/demon/normal/dmnnrm_attk3.wav",
    "sound/character/demon/normal/dmnnrm_death1.wav",
    "sound/character/demon/normal/dmnnrm_idle1.wav",
    "sound/character/demon/normal/dmnnrm_idle2.wav",
    "sound/character/demon/normal/dmnnrm_idle3.wav",
    "sound/character/demon/normal/dmnnrm_pain1.wav",
    "sound/character/demon/normal/dmnnrm_pain2.wav",
    "sound/character/demon/normal/dmnnrm_pain3.wav",
    "sound/character/demon/normal/dmnnrm_shield.wav",
];

#[derive(Clone, Copy)]
pub struct Timing {
    pub duration: f32,
    pub frame: f32,
}
pub struct Data {
    pub attacks: [[Timing; 2]; 3],
    clips: [std::collections::BTreeMap<String, Animation>; 3],
    skeletons: Vec<Skeleton>,
    events: Vec<crate::animation_events::Model>,
}
impl Data {
    fn timing(&self, d: &Demon) -> Timing {
        self.clips[d.kind].get(d.clip().0).map_or(
            Timing {
                duration: 1.4,
                frame: 0.05,
            },
            |c| Timing {
                duration: c.duration(),
                frame: c.frame_time,
            },
        )
    }
    fn tag(&self, d: &Demon, tag: &str) -> Transform {
        if let (Some(s), Some(c)) = (
            self.skeletons.get(d.kind),
            self.clips[d.kind].get(d.clip().0),
        ) {
            if let Some(i) = s
                .bones
                .iter()
                .position(|b| b.name.eq_ignore_ascii_case(tag))
            {
                let pose = s.global_pose(&c.sample(d.time, d.clip().1));
                let rotation = Quat::from_rotation_z(d.yaw);
                return Transform {
                    translation: d.feet + rotation * pose[i].translation * SIZE[d.kind],
                    rotation: rotation * pose[i].rotation,
                };
            }
        }
        Transform {
            translation: d.center(),
            rotation: Quat::from_rotation_z(d.yaw),
        }
    }
    fn muzzle(&self, d: &Demon, tag: &str) -> Vec3 {
        self.tag(d, tag).translation
    }
    fn speed(&self, d: &Demon) -> f32 {
        self.clips[d.kind].get(d.clip().0).map_or(110., |c| {
            (c.distance / c.duration() * SIZE[d.kind]).max(60.)
        })
    }
}
impl Data {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let mut attacks = [[Timing {
            duration: 1.,
            frame: 0.05,
        }; 2]; 3];
        let mut clips = std::array::from_fn(|_| std::collections::BTreeMap::new());
        let mut skeletons = Vec::new();
        let mut events = Vec::new();
        for k in 0..3 {
            let def = Definition::load(assets, &format!("models/{}.tik", MODELS[k]))?;
            let skeleton = Skeleton::parse(&assets.read(&format!("{}/{}", def.path, def.model))?)?;
            for (name, file) in &def.animations {
                let clip = Animation::parse(
                    &assets.read(&format!("{}/{file}", def.path))?,
                    skeleton.bones.len(),
                )?;
                if let Some(i) = [MELEE[k], RANGED[k]].iter().position(|n| *n == name) {
                    attacks[k][i] = Timing {
                        duration: clip.duration(),
                        frame: clip.frame_time,
                    };
                }
                clips[k].insert(name.clone(), clip);
            }
            skeletons.push(skeleton);
            events.push(crate::animation_events::Model::load(
                assets,
                &format!("models/{}.tik", MODELS[k]),
            )?);
        }
        Ok(Self {
            attacks,
            clips,
            skeletons,
            events,
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Die {
    pub position: Vec3,
    velocity: Vec3,
    pub age: f32,
    resting: bool,
    pub pip: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Phase {
    Appear,
    Idle,
    Move,
    Melee,
    Ranged,
    Claw,
    Ice,
    ChargeStart,
    Charge,
    ChargeHit,
    Follow,
    Recover,
    Pain,
    Dead,
    Vanish,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Demon {
    pub kind: usize,
    pub feet: Vec3,
    pub yaw: f32,
    pub health: f32,
    pub phase: Phase,
    pub time: f32,
    life: f32,
    idle: f32,
    cooldown: f32,
    target: Option<usize>,
    hostile: bool,
    fired: u8,
    #[serde(default)]
    frozen: bool,
    #[serde(default)]
    recoil: combat::Recoil,
    #[serde(default)]
    shield: f32,
}
impl Demon {
    fn scale(&self) -> f32 {
        SIZE[self.kind]
    }
    pub fn center(&self) -> Vec3 {
        self.feet + Vec3::Z * 40. * self.scale()
    }
    pub fn target(&self) -> Target {
        Target {
            id: SUMMON,
            center: self.center(),
            half: vec3(32., 32., 40.) * self.scale(),
        }
    }
    fn set(&mut self, phase: Phase) {
        self.phase = phase;
        self.time = 0.;
        self.fired = 0;
    }
    pub fn clip(&self) -> (&'static str, bool) {
        match self.phase {
            Phase::Appear | Phase::Idle => (IDLE[self.kind], true),
            Phase::Move => (MOVE[self.kind], true),
            Phase::Melee => (MELEE[self.kind], false),
            Phase::Ranged => (RANGED[self.kind], false),
            Phase::Claw => ("attack2", false),
            Phase::Ice => ("attack1", false),
            Phase::ChargeStart => ("run_attack1a", false),
            Phase::Charge => ("run_attack1b", true),
            Phase::ChargeHit => ("run_attack1c", false),
            Phase::Follow => ("melee1b", false),
            Phase::Recover => ("melee1a2", false),
            Phase::Pain => ("pain1", false),
            Phase::Dead if self.frozen => ("death_frozen", false),
            Phase::Dead => (["death2", "death", "death2"][self.kind], false),
            Phase::Vanish => ("poof", false),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Bolt {
    position: Vec3,
    velocity: Vec3,
    age: f32,
    hostile: bool,
    #[serde(default)]
    ice: bool,
    #[serde(default)]
    id: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Beam {
    pub a: Vec3,
    pub b: Vec3,
    pub life: f32,
    pub kind: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    /// Derived from the player's current targeting flags, not a separate save setting.
    #[serde(skip)]
    pub notarget: bool,
    #[serde(skip)]
    pub rage: bool,
    #[serde(skip)]
    pub difficulty: crate::powerups::Difficulty,
    #[serde(default)]
    forced_king: bool,
    #[serde(default)]
    next_id: u32,
    pub dice: Vec<Die>,
    pub demon: Option<Demon>,
    bolts: Vec<Bolt>,
    #[serde(default)]
    pub beams: Vec<Beam>,
    seed: u32,
    pub cooldown: f32,
    time: f32,
    accumulator: f64,
    #[serde(default)]
    world_accumulator: f64,
}
impl Default for State {
    fn default() -> Self {
        Self {
            notarget: false,
            rage: false,
            difficulty: Default::default(),
            forced_king: false,
            next_id: 0,
            dice: vec![],
            demon: None,
            bolts: vec![],
            beams: vec![],
            seed: 0x6d2b79f5,
            cooldown: 0.,
            time: 0.,
            accumulator: 0.,
            world_accumulator: 0.,
        }
    }
}
#[derive(Default)]
pub struct Feedback {
    pub hits: Vec<Hit>,
    pub sounds: Vec<&'static str>,
    pub spatial_sounds: Vec<(&'static str, Vec3)>,
    pub refund: f32,
}
fn eligible(t: &Target) -> bool {
    t.id != SUMMON
        && t.id != ALICE
        && !(crate::interaction::SHOT_BASE..crate::encounters::BASE).contains(&t.id)
}
fn visible(world: &World, a: Vec3, b: Vec3) -> bool {
    let h = world.sweep(a, b, Vec3::splat(0.5));
    !h.start_solid && h.fraction >= 1.
}
pub fn tier(pips: &[u8]) -> usize {
    let total: u8 = pips.iter().sum();
    if pips.len() == 3 && total >= 13 {
        2
    } else if pips.len() >= 2 && total >= 9 {
        1
    } else {
        0
    }
}
impl State {
    pub fn ready(&self) -> bool {
        self.cooldown <= 0. && self.dice.is_empty() && self.demon.is_none()
    }
    pub fn throw(&mut self, count: u8, origin: Vec3, direction: Vec3) -> bool {
        if !self.ready()
            || !(1..=3).contains(&count)
            || !origin.is_finite()
            || !direction.is_finite()
        {
            return false;
        }
        self.time = 0.;
        self.forced_king = count == 3 && self.rage;
        self.cooldown = 6.;
        for i in 0..count {
            // Private deterministic generator: saving mid-throw cannot reroll a summon.
            self.seed ^= self.seed << 13;
            self.seed ^= self.seed >> 17;
            self.seed ^= self.seed << 5;
            let pip = (self.seed % 6 + 1) as u8;
            let spread = (i as f32 - (count - 1) as f32 * 0.5) * 0.16;
            self.dice.push(Die {
                position: origin,
                velocity: Quat::from_rotation_z(spread) * direction.normalize_or_zero() * 200.
                    + Vec3::Z * 110.,
                age: 0.,
                resting: false,
                pip,
            });
        }
        true
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.dice.len() <= 3
                && self.bolts.len() <= 32
                && self.seed != 0
                && (0.0..=6.).contains(&self.cooldown)
                && (0.0..=9.).contains(&self.time)
                && self.accumulator.is_finite()
                && self.accumulator.abs() < 0.01
                && self.world_accumulator.is_finite()
                && self.world_accumulator.abs() < 0.01,
            "Invalid saved Demon Dice clock"
        );
        ensure!(
            self.dice.iter().all(|d| d.position.is_finite()
                && d.velocity.is_finite()
                && (0.0..=8.1).contains(&d.age)
                && (1..=6).contains(&d.pip)),
            "Invalid saved thrown dice"
        );
        if let Some(d) = &self.demon {
            ensure!(
                self.dice.is_empty()
                    && d.kind < 3
                    && (d.kind == 2
                        || !matches!(
                            d.phase,
                            Phase::Ice
                                | Phase::ChargeStart
                                | Phase::Charge
                                | Phase::ChargeHit
                                | Phase::Follow
                                | Phase::Recover
                        ))
                    && (d.kind == 1 || d.phase != Phase::Claw)
                    && d.feet.is_finite()
                    && d.yaw.is_finite()
                    && (0.0..=HEALTH[d.kind]).contains(&d.health)
                    && (d.health > 0. || matches!(d.phase, Phase::Dead | Phase::Vanish))
                    && (0.0..=1e9).contains(&d.life)
                    && d.recoil.valid()
                    && (0.0..=1.).contains(&d.shield)
                    && (0.0..=1e9).contains(&d.time)
                    && (0.0..=3.).contains(&d.cooldown)
                    && (0.0..=3.).contains(&d.idle)
                    && d.fired <= 7,
                "Invalid saved demon"
            );
        }
        ensure!(
            self.bolts.iter().all(|b| b.position.is_finite()
                && b.velocity.is_finite()
                && (0.0..=3.).contains(&b.age)),
            "Invalid saved demon projectile"
        );
        ensure!(
            self.beams.len() <= 128
                && self.beams.iter().all(|b| b.a.is_finite()
                    && b.b.is_finite()
                    && (0.0..=0.2).contains(&b.life)
                    && b.kind < 3),
            "Invalid saved demon beam"
        );
        Ok(())
    }
    #[cfg(test)]
    pub fn hurt(&mut self, damage: f32) {
        self.hit(Hit {
            id: SUMMON,
            damage,
            kind: combat::DamageKind::Other,
            knockback: Vec3::ZERO,
        });
    }
    pub fn hit(&mut self, hit: Hit) {
        if !hit.damage.is_finite() || hit.damage <= 0. {
            return;
        }
        if let Some(d) = &mut self.demon {
            if d.health <= 0. || matches!(d.phase, Phase::Appear | Phase::Vanish) {
                return;
            }
            // Sentient::Damage emits the normal demon's shield impact but still
            // applies health damage; its powerup is not blanket invulnerability.
            if d.kind == 1 {
                d.shield = 0.35;
            }
            d.recoil.hit(hit.knockback, [150., 200., 400.][d.kind]);
            d.health = (d.health - hit.damage).max(0.);
            if d.health == 0. {
                d.frozen = hit.kind.means() == combat::DamageKind::Ice;
                d.set(Phase::Dead);
            } else if hit.damage >= [35., 40., 75.][d.kind] {
                d.set(Phase::Pain);
            }
        }
    }
    pub fn dismiss(&mut self) {
        if let Some(d) = &mut self.demon {
            if !matches!(d.phase, Phase::Dead | Phase::Vanish) {
                d.set(Phase::Vanish);
            }
        }
    }
    pub fn target(&self) -> Option<Target> {
        self.demon
            .as_ref()
            .filter(|d| {
                d.health > 0. && !matches!(d.phase, Phase::Appear | Phase::Dead | Phase::Vanish)
            })
            .map(Demon::target)
    }
    pub fn ally_target(&self) -> Option<Target> {
        self.target()
            .filter(|_| self.demon.as_ref().is_some_and(|d| !d.hostile))
    }
    pub fn advance(&mut self, dt: f32, ctx: &Context<'_>, eye: Vec3, data: &Data) -> Feedback {
        self.advance_timed(dt, true, ctx, eye, data)
    }
    pub fn advance_timed(
        &mut self,
        dt: f32,
        world_active: bool,
        ctx: &Context<'_>,
        eye: Vec3,
        data: &Data,
    ) -> Feedback {
        self.advance_clocks(dt, if world_active { dt } else { 0. }, ctx, eye, data)
    }
    pub fn advance_clocks(
        &mut self,
        dt: f32,
        world_dt: f32,
        ctx: &Context<'_>,
        eye: Vec3,
        data: &Data,
    ) -> Feedback {
        let mut out = Feedback::default();
        if dt <= 0. || !dt.is_finite() {
            return out;
        }
        self.accumulator += dt.min(0.1) as f64;
        self.world_accumulator += world_dt.clamp(0., dt.min(0.1)) as f64;
        while self.accumulator + 1e-9 >= 1. / 120. || self.world_accumulator + 1e-9 >= 1. / 120. {
            let real = self.accumulator + 1e-9 >= 1. / 120.;
            let world = self.world_accumulator + 1e-9 >= 1. / 120.;
            if real {
                self.accumulator -= 1. / 120.;
            }
            if world {
                self.world_accumulator -= 1. / 120.;
            }
            self.step(real, world, ctx, eye, data, &mut out);
        }
        out
    }
    fn step(
        &mut self,
        real_active: bool,
        world_active: bool,
        ctx: &Context<'_>,
        eye: Vec3,
        data: &Data,
        out: &mut Feedback,
    ) {
        if real_active {
            self.cooldown = (self.cooldown - STEP).max(0.);
        }
        if world_active {
            self.beams.retain_mut(|b| {
                b.life -= STEP;
                b.life > 0.
            });
        }
        let enemies = ctx
            .targets
            .iter()
            .copied()
            .filter(eligible)
            .collect::<Vec<_>>();
        let alice = Target {
            id: ALICE,
            center: eye - Vec3::Z * 22.,
            half: vec3(15., 15., 28.),
        };
        if world_active {
            self.bolts.retain_mut(|b| {
                b.age += STEP;
                let end = b.position + b.velocity * STEP;
                let targets = if b.hostile {
                    std::slice::from_ref(&alice)
                } else {
                    &enemies
                };
                let contact = combat::contact(
                    &Context {
                        world: ctx.world,
                        targets,
                    },
                    b.position,
                    end,
                    4.,
                );
                let wall = ctx.world.sweep(b.position, end, Vec3::splat(4.));
                let after = b
                    .position
                    .lerp(end, contact.map(|(_, f)| f).unwrap_or(wall.fraction));
                b.position = after;
                if let Some((id, _)) = contact {
                    out.hits.push(Hit {
                        knockback: Vec3::ZERO,
                        kind: if b.ice {
                            combat::DamageKind::DemonIce
                        } else {
                            combat::DamageKind::DemonFire
                        },
                        id,
                        damage: if b.ice { 15. } else { 25. },
                    });
                }
                contact.is_none() && !wall.start_solid && wall.fraction >= 1. && b.age < 2.
            });
        }
        if real_active && !self.dice.is_empty() {
            self.time += STEP;
            for die in &mut self.dice {
                die.age += STEP;
                if die.resting {
                    continue;
                }
                die.velocity.z -= 800. * STEP;
                let end = die.position + die.velocity * STEP;
                let hit = ctx.world.sweep(die.position, end, Vec3::splat(8.));
                die.position = die.position.lerp(end, hit.fraction);
                if hit.start_solid {
                    die.resting = true;
                } else if hit.fraction < 1. {
                    if die.velocity.length() > 40. {
                        out.sounds.push(SOUNDS[3]);
                    }
                    die.velocity =
                        (die.velocity - hit.normal * 2. * die.velocity.dot(hit.normal)) * 0.4;
                    die.position += hit.normal * 0.1;
                    if hit.normal.z > 0.7 && die.velocity.length() < 50. {
                        die.resting = true;
                    }
                }
            }
            if self.time >= 2. && self.dice.iter().all(|d| d.resting) || self.time >= 8. {
                let kind = if self.forced_king {
                    2
                } else {
                    tier(&self.dice.iter().map(|d| d.pip).collect::<Vec<_>>())
                };
                let at = self.dice[0].position;
                self.demon = spawn(ctx.world, at, kind).map(|feet| {
                    let center = feet + Vec3::Z * 40. * SIZE[kind];
                    let hostile = !enemies.iter().any(|t| {
                        t.center.distance(center) < 1200. && visible(ctx.world, center, t.center)
                    });
                    Demon {
                        kind,
                        feet,
                        yaw: 0.,
                        health: HEALTH[kind],
                        phase: Phase::Appear,
                        time: 0.,
                        life: 0.,
                        idle: 0.,
                        cooldown: 0.,
                        target: None,
                        hostile,
                        fired: 0,
                        frozen: false,
                        recoil: Default::default(),
                        shield: 0.,
                    }
                });
                if self.demon.is_none() {
                    out.refund += combat::will_cost(6, false);
                }
                self.dice.clear();
                self.time = 0.;
                out.sounds
                    .push(if self.demon.is_some() { OPEN } else { CLOSE });
            }
        }
        if !world_active {
            return;
        }
        if let Some(d) = &mut self.demon {
            d.life = (d.life + STEP).min(1e9);
            d.shield = (d.shield - STEP).max(0.);
            d.feet += d.recoil.step(STEP, ctx.world, d.target());
            if d.time == 0. && d.phase == Phase::Vanish {
                out.sounds.push(CLOSE);
            }
            d.time += STEP;
            d.cooldown = (d.cooldown - STEP).max(0.);
            let timing = data.timing(d);
            if let Some(model) = data.events.get(d.kind) {
                for event in model.between(
                    d.clip().0,
                    crate::animation_events::Span {
                        start: d.time - STEP,
                        end: d.time,
                        duration: timing.duration,
                        frame_time: timing.frame,
                        looping: d.clip().1,
                        entered: d.time <= STEP,
                    },
                ) {
                    if let crate::animation_events::Command::Sound { path, .. } = event {
                        if let Some(&path) = SOUNDS.iter().find(|&&p| p == path) {
                            out.spatial_sounds.push((path, d.center()));
                        }
                    }
                }
            }
            if matches!(d.phase, Phase::Dead | Phase::Vanish) {
                if d.time >= timing.duration {
                    self.demon = None;
                }
                return;
            }
            if d.phase == Phase::Appear {
                if d.time >= timing.duration {
                    d.set(Phase::Idle);
                }
                return;
            }
            if d.phase == Phase::Pain {
                if d.time >= timing.duration {
                    d.set(Phase::Idle);
                }
                return;
            }
            let targets = if d.hostile && self.notarget {
                &[][..]
            } else if d.hostile {
                std::slice::from_ref(&alice)
            } else {
                &enemies
            };
            let center = d.center();
            let attacking = !matches!(d.phase, Phase::Idle | Phase::Move);
            let target = targets.iter().find(|t| Some(t.id) == d.target).or_else(|| {
                targets
                    .iter()
                    .filter(|t| {
                        t.center.distance(center) < 1200. && visible(ctx.world, center, t.center)
                    })
                    .min_by(|a, b| {
                        a.center
                            .distance_squared(center)
                            .total_cmp(&b.center.distance_squared(center))
                    })
            });
            if let Some(t) = target {
                d.idle = 0.;
                d.target = Some(t.id);
                let delta = t.center - center;
                let gap = (delta.abs() - d.target().half - t.half)
                    .max(Vec3::ZERO)
                    .length();
                let sees = visible(ctx.world, center, t.center);
                let angle = delta.y.atan2(delta.x);
                let turn = (angle - d.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                let turn_step = [15_f32, 50., 7.][d.kind].to_radians() * STEP / 0.05;
                d.yaw += turn.clamp(-turn_step, turn_step);
                if d.phase == Phase::ChargeStart {
                    if !sees {
                        d.set(Phase::Move);
                    } else if d.time >= timing.duration {
                        d.set(Phase::Charge);
                    }
                } else if d.phase == Phase::Charge {
                    if gap < 8. {
                        d.set(Phase::ChargeHit);
                    } else if !sees || !move_demon(d, ctx.world, t.center, data.speed(d).max(250.))
                    {
                        d.set(Phase::Move);
                    }
                } else if attacking {
                    let frames: &[u8] = match (d.kind, d.phase) {
                        (0, Phase::Melee) => &[8],
                        (0, Phase::Ranged) => &[9, 10, 11, 12, 13, 14, 15],
                        (1, Phase::Melee) => &[6, 7, 8, 9],
                        (1, Phase::Claw) => &[5, 6, 7, 8, 9],
                        (1, Phase::Ranged) => &[20],
                        (2, Phase::Melee) => &[12],
                        (2, Phase::Follow) => &[3],
                        (2, Phase::ChargeHit) => &[1],
                        (2, Phase::Ice) => &[11, 12, 15, 18, 21],
                        (2, Phase::Ranged) => &[10, 16, 18, 20],
                        _ => &[],
                    };
                    while (d.fired as usize) < frames.len()
                        && d.time >= frames[d.fired as usize] as f32 * timing.frame
                    {
                        d.fired += 1;
                        if !sees {
                            continue;
                        }
                        if d.kind == 1 && d.phase == Phase::Ranged || d.phase == Phase::Ice {
                            let ice = d.phase == Phase::Ice;
                            let muzzle =
                                data.muzzle(d, if ice { "tag_breath" } else { "tag_righhand" });
                            // Start at the swept tag; a wall between the body and tag cannot be bypassed.
                            let trace = ctx.world.sweep(center, muzzle, Vec3::splat(4.));
                            if !trace.start_solid && trace.fraction >= 1. {
                                self.next_id = self.next_id.wrapping_add(1).max(1);
                                self.bolts.push(Bolt {
                                    id: self.next_id,
                                    position: muzzle,
                                    velocity: (t.center - muzzle).normalize_or_zero()
                                        * if ice { 800. } else { 700. },
                                    age: 0.,
                                    hostile: d.hostile,
                                    ice,
                                });
                            }
                        } else if d.phase == Phase::Ranged {
                            let tag = data.tag(
                                d,
                                if d.fired % 2 == 1 {
                                    "tag_lefthand"
                                } else {
                                    "tag_righthand"
                                },
                            );
                            let muzzle = tag.translation;
                            let delta = t.center - muzzle;
                            let end = if d.kind == 2 {
                                t.center
                            } else {
                                // FireBeam mode 0 uses animated hand heading, target elevation,
                                // and difficulty-dependent scatter; mode 1 tracks the enemy.
                                let forward = tag.rotation * Vec3::X;
                                let mut direction = (forward * delta.length())
                                    .with_z(delta.z)
                                    .normalize_or_zero();
                                let right = direction.cross(Vec3::Z).normalize_or_zero();
                                let up = right.cross(direction).normalize_or_zero();
                                let spread = if self.difficulty as usize >= 2 {
                                    0.01
                                } else {
                                    0.05
                                };
                                for axis in [right, up] {
                                    self.seed =
                                        self.seed.wrapping_mul(214013).wrapping_add(2531011);
                                    direction += axis
                                        * ((((self.seed >> 16) & 0x7fff) as f32 / 32767.) * 2.
                                            - 1.)
                                        * spread;
                                }
                                muzzle + direction.normalize_or_zero() * 500.
                            };
                            let contact = combat::contact(
                                &Context {
                                    world: ctx.world,
                                    targets,
                                },
                                muzzle,
                                end,
                                0.,
                            );
                            let wall = ctx.world.sweep(muzzle, end, Vec3::ZERO);
                            if let Some((id, _)) = contact {
                                out.hits.push(Hit {
                                    id,
                                    damage: 2.,
                                    kind: combat::DamageKind::DemonElectric,
                                    knockback: Vec3::ZERO,
                                });
                            }
                            self.beams.push(Beam {
                                a: muzzle,
                                b: muzzle.lerp(end, contact.map_or(wall.fraction, |(_, f)| f)),
                                life: 0.1,
                                kind: d.kind,
                            });
                        } else if gap <= if d.kind == 2 { 135. } else { 125. } && turn.abs() < 1.2 {
                            let (damage, kick) = match (d.kind, d.phase) {
                                (0, _) => (10., 20.),
                                (1, _) => (5., 30.),
                                (_, Phase::Follow) => (20., 100.),
                                (_, Phase::ChargeHit) => (35., 150.),
                                _ => (20., 2.),
                            };
                            out.hits.push(Hit {
                                id: t.id,
                                damage,
                                kind: if d.kind == 0 {
                                    combat::DamageKind::DemonElectric
                                } else {
                                    combat::DamageKind::DemonMelee
                                },
                                knockback: delta.normalize_or_zero() * kick,
                            });
                        }
                    }
                    if d.time >= timing.duration {
                        let next = if d.kind == 2 && d.phase == Phase::Melee {
                            if sees && gap < 135. {
                                Phase::Follow
                            } else {
                                Phase::Recover
                            }
                        } else if d.phase == Phase::ChargeHit && gap < 120. {
                            Phase::Melee
                        } else {
                            Phase::Move
                        };
                        d.set(next);
                    }
                } else if d.cooldown <= 0. && sees && gap < 600. {
                    self.seed = self.seed.wrapping_mul(214013).wrapping_add(2531011);
                    let roll = (self.seed >> 16) & 0x7fff;
                    let phase = if gap < [120., 125., 120.][d.kind] {
                        if d.kind == 1 && roll % 2 == 0 {
                            Phase::Claw
                        } else {
                            Phase::Melee
                        }
                    } else if d.kind == 2 {
                        if gap > 450. && roll % 5 != 0 || roll % 3 == 0 {
                            Phase::ChargeStart
                        } else if roll % 2 == 0 {
                            Phase::Ice
                        } else {
                            Phase::Ranged
                        }
                    } else {
                        Phase::Ranged
                    };
                    d.set(phase);
                    if data.events.is_empty() {
                        out.spatial_sounds.push((attack_sound(d), d.center()));
                    }
                } else {
                    if d.phase != Phase::Move {
                        d.set(Phase::Move);
                    }
                    move_demon(d, ctx.world, t.center, data.speed(d));
                }
            } else {
                d.target = None;
                d.idle += STEP;
                if d.phase != Phase::Idle {
                    d.set(Phase::Idle);
                }
                if d.idle >= 2. && d.time >= timing.duration {
                    d.set(Phase::Vanish);
                }
            }
        }
        if self.beams.len() > 128 {
            self.beams.drain(..self.beams.len() - 128);
        }
    }
}
fn spawn(world: &World, at: Vec3, kind: usize) -> Option<Vec3> {
    let half = vec3(32., 32., 40.) * SIZE[kind];
    for offset in [
        Vec3::ZERO,
        Vec3::X * 72.,
        -Vec3::X * 72.,
        Vec3::Y * 72.,
        -Vec3::Y * 72.,
    ] {
        let start = at + offset + Vec3::Z * (half.z + 16.);
        if !visible(world, at, start) {
            continue;
        }
        let floor = world.sweep(start, start - Vec3::Z * 192., half);
        if !floor.start_solid && floor.fraction < 1. && floor.normal.z > 0.7 {
            let feet = start - Vec3::Z * (192. * floor.fraction + half.z);
            if !world
                .sweep(feet + Vec3::Z * half.z, feet + Vec3::Z * half.z, half)
                .start_solid
            {
                return Some(feet);
            }
        }
    }
    None
}
fn move_demon(d: &mut Demon, world: &World, target: Vec3, speed: f32) -> bool {
    let half = d.target().half;
    let center = d.center();
    let delta = if d.kind == 0 {
        target - center
    } else {
        (target - center).with_z(0.)
    };
    let before = d.feet;
    let desired = delta.normalize_or_zero();
    // Probe ahead to steer around a local obstruction before the hull is wedged.
    // A charging king commits to its line instead of swerving through obstacles.
    let angles: &[f32] = if d.phase == Phase::Charge {
        &[0.]
    } else {
        &[0., 30., -30., 60., -60., 90., -90.]
    };
    for angle in angles {
        let direction = Quat::from_rotation_z(angle.to_radians()) * desired;
        let lookahead = speed.mul_add(STEP, 24.);
        let next = if d.kind == 0 {
            let end = center + direction * lookahead;
            let trace = world.sweep(center, end, half);
            if trace.start_solid || trace.fraction < 1. {
                continue;
            }
            before + direction * speed * STEP
        } else {
            let probe = combat::walk_body(world, before, direction * lookahead, half);
            if probe.distance_squared(before) < 1. {
                continue;
            }
            combat::walk_body(world, before, direction * speed * STEP, half)
        };
        if next.distance_squared(before) > 0.000001 {
            d.feet = next;
            break;
        }
    }
    d.feet.distance_squared(before) > 0.000001
}

fn attack_sound(d: &Demon) -> &'static str {
    match (d.kind, d.phase) {
        (1, Phase::Claw) => "sound/character/demon/normal/dmnnrm_attk2.wav",
        (2, Phase::Ice) => "sound/character/demon/king/king_breath1.wav",
        (2, Phase::ChargeStart) => "sound/character/demon/king/king_runattack_start.wav",
        _ => SOUNDS[4 + d.kind * 2 + usize::from(d.phase == Phase::Ranged)],
    }
}

pub struct Art {
    puppets: Vec<crate::npc::Puppet>,
    shield: crate::weapons::Prop,
    ice: crate::weapons::Prop,
    trails: Vec<crate::particles::Attached>,
    emitters: std::collections::BTreeMap<u32, crate::particles::Attached>,
    poof: Vec<crate::particles::Attached>,
    spawn: crate::particles::Attached,
    pub atmosphere: std::cell::RefCell<crate::environment::Atmosphere>,
}
impl Art {
    pub fn load(
        assets: &mut Assets,
        specs: &std::collections::BTreeMap<String, crate::texture::MaterialSpec>,
    ) -> Result<Self> {
        let mut puppets = Vec::new();
        for model in MODELS {
            let def = Definition::load(assets, &format!("models/{model}.tik"))?;
            let clips = def
                .animations
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>();
            puppets.push(crate::npc::Puppet::load(assets, model, &clips, specs)?);
        }
        let mut trails = Vec::new();
        for name in ["prj_fireball", "fx_demonking_icebreath"] {
            trails.push(
                crate::particles::Attached::load(assets, name, specs)?
                    .ok_or_else(|| anyhow::anyhow!("Missing demon particles: {name}"))?,
            );
        }
        let mut poof = Vec::new();
        for name in MODELS {
            poof.push(
                crate::particles::Attached::load_bursts(assets, name, 0.05, specs)?
                    .ok_or_else(|| anyhow::anyhow!("Missing demon poof: {name}"))?,
            );
        }
        Ok(Self {
            puppets,
            shield: crate::weapons::Prop::load_animation(
                assets,
                "fx_shield_demon_normal",
                "idle",
                specs,
            )?,
            ice: crate::weapons::Prop::load_animation(
                assets,
                "fx_demonking_icebreath",
                "idle",
                specs,
            )?,
            trails,
            emitters: Default::default(),
            poof,
            spawn: crate::particles::Attached::load(assets, "fx_demonspawn", specs)?
                .ok_or_else(|| anyhow::anyhow!("Missing demon spawn effect"))?,
            atmosphere: Default::default(),
        })
    }
    pub fn draw(&mut self, state: &State, fullbright: bool) {
        if let Some(d) = &state.demon {
            let growth = match d.phase {
                Phase::Appear => (d.time / 0.7).clamp(0.01, 1.),
                Phase::Vanish => (1. - d.time / 0.5).max(0.),
                _ => 1.,
            };
            let (clip, looping) = d.clip();
            self.puppets[d.kind].draw(
                clip,
                d.time,
                looping,
                Transform {
                    translation: d.feet,
                    rotation: Quat::from_rotation_z(d.yaw),
                },
                [1.25, 1.5, 2.][d.kind] * growth,
                fullbright,
            );
        }
    }
    pub fn effects(&mut self, state: &State, camera: Vec3) {
        let atmosphere = self.atmosphere.borrow();
        if let Some(d) = &state.demon {
            let pose = Transform {
                translation: d.center(),
                rotation: Quat::from_rotation_z(d.yaw),
            };
            if d.shield > 0. {
                for mesh in self
                    .shield
                    .meshes_at(pose, d.scale(), true, 0.35 - d.shield, false)
                {
                    for v in &mut mesh.vertices {
                        v.color[3] = (255. * (d.shield / 0.35)) as u8;
                    }
                    crate::render_fx::effect(mesh, crate::materials::Blend::AlphaAdd);
                }
            }
            if d.phase == Phase::Vanish {
                self.poof[d.kind].draw(
                    d.time,
                    d.scale(),
                    |_, _| Transform {
                        translation: d.feet,
                        ..pose
                    },
                    |_, _, _| true,
                    camera,
                    &atmosphere,
                );
            } else if d.phase == Phase::Appear {
                self.spawn.draw(
                    d.time,
                    d.scale(),
                    |_, _| Transform {
                        translation: d.feet,
                        ..pose
                    },
                    |_, _, on| on,
                    camera,
                    &atmosphere,
                );
            }
        }
        self.emitters
            .retain(|id, _| state.bolts.iter().any(|p| p.id == *id));
        for p in &state.bolts {
            let pose = Transform {
                translation: p.position,
                rotation: Quat::from_rotation_arc(Vec3::X, p.velocity.normalize_or_zero()),
            };
            if p.ice {
                for mesh in self.ice.meshes_at(pose, 1., true, p.age, true) {
                    crate::render_fx::effect(mesh, crate::materials::Blend::AlphaAdd);
                }
            }
            self.emitters
                .entry(p.id)
                .or_insert_with(|| self.trails[usize::from(p.ice)].fork())
                .draw(
                    p.age,
                    1.,
                    |t, _| Transform {
                        translation: p.position + p.velocity * (t - p.age),
                        ..pose
                    },
                    |_, _, on| on,
                    camera,
                    &atmosphere,
                );
        }
    }
}

pub fn check(assets: &mut Assets) -> Result<()> {
    let data = Data::load(assets)?;
    for (kind, phase, frames, damage) in [
        (0, Phase::Melee, 8., 10.),
        (0, Phase::Ranged, 15., 14.),
        (1, Phase::Melee, 9., 20.),
        (1, Phase::Claw, 9., 25.),
        (1, Phase::Ranged, 20., 25.),
        (2, Phase::Melee, 12., 20.),
        (2, Phase::Follow, 3., 20.),
        (2, Phase::ChargeHit, 1., 35.),
        (2, Phase::Ice, 21., 75.),
        (2, Phase::Ranged, 20., 8.),
    ] {
        let world = World::fixture(&[(vec3(-4000., -4000., -30.), vec3(4000., 4000., 0.))]);
        let mut state = State {
            demon: Some(Demon {
                kind,
                feet: Vec3::Z * 0.1,
                yaw: 0.,
                health: HEALTH[kind],
                phase,
                time: 0.,
                life: 0.,
                idle: 0.,
                cooldown: 0.,
                target: Some(42),
                hostile: false,
                fired: 0,
                frozen: false,
                recoil: Default::default(),
                shield: 0.,
            }),
            ..Default::default()
        };
        let d = state.demon.as_ref().unwrap();
        let target = Target {
            id: 42,
            center: d.center() + Vec3::X * 100.,
            half: vec3(30., 60., 80.),
        };
        let time = data.timing(d).frame * frames + 0.3;
        let ctx = Context {
            world: &world,
            targets: &[target],
        };
        let mut total = 0.;
        for _ in 0..(time * 120.).ceil() as usize {
            total += state
                .advance(STEP, &ctx, Vec3::new(-800., 0., 60.), &data)
                .hits
                .iter()
                .map(|h| h.damage)
                .sum::<f32>();
        }
        ensure!(
            total >= damage,
            "Archive-backed demon {kind}/{phase:?}: expected at least {damage}, got {total}"
        );
        state.validate()?;
        println!(
            "PASS original demon {kind}/{phase:?}: {total} damage, original animation frame events"
        );
    }
    let catalog = crate::inventory::Catalog::load(assets)?;
    ensure!(
        catalog.weapons[6].primary == combat::will_cost(6, false)
            && catalog.weapons[6].alternate.is_none(),
        "Demon Dice resource definition changed"
    );
    let map = crate::bsp::Bsp::parse(&assets.read("maps/skool1.bsp")?)?;
    let world = World::from_bsp(&map)?;
    for hz in [30, 60, 144] {
        let mut state = State::default();
        let eye = vec3(-1888., 1752., -452.);
        let mut guard = combat::Guard::new(vec3(-1888., 1990., -504.), 0., 1.);
        ensure!(state.throw(1, eye, Vec3::Y), "Dice throw rejected");
        let mut damage = 0.;
        let mut summoned = false;
        for _ in 0..hz * 22 {
            let targets = if guard.health > 0. {
                vec![guard.target(0)]
            } else {
                vec![]
            };
            let out = state.advance(
                1. / hz as f32,
                &Context {
                    world: &world,
                    targets: &targets,
                },
                eye,
                &data,
            );
            summoned |= state.demon.is_some();
            for h in out.hits {
                ensure!(h.id == 0, "Combat summon unexpectedly attacked Alice");
                damage += h.damage;
                guard.hurt(h.damage);
            }
        }
        ensure!(
            summoned && guard.health == 0.,
            "Dice did not defeat school guard at {hz} Hz: damage={damage}, state={state:?}"
        );
        println!("PASS Demon Dice school combat at {hz} Hz: {damage} damage, guard defeated");
    }
    println!("PASS original Dice cost, three demon skeletons/attack clips, collision-aware summon and enemy damage");
    Ok(())
}

/// Explicitly staged art checks, separate from the live combat and traversal checks.
pub async fn render_check(assets: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(assets, "skool2")?;
    let mut i = crate::interaction::Interactions::load(&scene.map)?;
    i.set_entry(assets, &scene.map, "skool2", None)?;
    let mut visuals = crate::weapons::Visuals::load(assets)?;
    let material = crate::character::skin_material()?;
    let data = Data::load(assets)?;
    // Stage on the gym floor, below its rafters, with room for the largest summon.
    let start = vec3(2280., -2830., 100.);
    let end = start - Vec3::Z * 1100.;
    let floor = scene.world.sweep(start, end, Vec3::splat(2.));
    ensure!(
        !floor.start_solid && floor.fraction < 1. && floor.normal.z > 0.7,
        "Demon visual fixture needs clear supported ground"
    );
    let feet = start.lerp(end, floor.fraction) - Vec3::Z * 1.9;
    let camera = vec3(2100., -3050., feet.z + 140.);
    ensure!(
        !scene
            .world
            .sweep(camera, camera, Vec3::splat(2.))
            .start_solid,
        "Demon visual fixture camera is obstructed"
    );
    println!("Demon visual fixture: floor {feet:?}, camera {camera:?}");
    for kind in 0..3 {
        let phases = match kind {
            0 => vec![
                Phase::Appear,
                Phase::Melee,
                Phase::Ranged,
                Phase::Pain,
                Phase::Dead,
            ],
            1 => vec![
                Phase::Appear,
                Phase::Melee,
                Phase::Claw,
                Phase::Ranged,
                Phase::Pain,
                Phase::Dead,
            ],
            _ => vec![
                Phase::Appear,
                Phase::Melee,
                Phase::Follow,
                Phase::Ranged,
                Phase::Ice,
                Phase::ChargeStart,
                Phase::Charge,
                Phase::ChargeHit,
                Phase::Pain,
                Phase::Dead,
            ],
        };
        for phase in phases {
            visuals.clear();
            let delta = if matches!(
                phase,
                Phase::Melee | Phase::Claw | Phase::Follow | Phase::ChargeHit
            ) {
                vec3(-20., -65., 0.)
            } else {
                vec3(-70., -220., 0.)
            };
            visuals.dice.demon = Some(Demon {
                kind,
                feet,
                yaw: delta.y.atan2(delta.x),
                health: HEALTH[kind],
                phase,
                time: 0.,
                life: 0.,
                idle: 0.,
                cooldown: 0.,
                target: Some(0),
                hostile: false,
                fired: 0,
                frozen: false,
                recoil: Default::default(),
                shield: 0.,
            });
            let center = visuals.dice.demon.as_ref().unwrap().center();
            ensure!(
                visible(&scene.world, camera, center),
                "Demon visual fixture is occluded"
            );
            let target = Target {
                id: 0,
                center: center + delta,
                half: Vec3::splat(24.),
            };
            ensure!(
                visible(&scene.world, center, target.center),
                "Demon visual attack is obstructed"
            );
            let time = if phase == Phase::Appear {
                0.35
            } else if phase == Phase::Melee {
                data.attacks[kind][0].frame * [8., 6., 12.][kind] + 0.035
            } else if phase == Phase::Ranged {
                data.attacks[kind][1].frame * [9., 20., 10.][kind]
                    + if kind == 1 { 0.18 } else { 0.035 }
            } else {
                data.timing(visuals.dice.demon.as_ref().unwrap()).frame
                    * match phase {
                        Phase::Claw => 6.,
                        Phase::Ice => 12.,
                        Phase::ChargeHit => 1.,
                        Phase::Follow => 3.,
                        _ => 5.,
                    }
                    + 0.035
            };
            let frames = (time * 60.).ceil() as usize;
            for frame in 0..frames {
                visuals.dice.advance(
                    1. / 60.,
                    &Context {
                        world: &scene.world,
                        targets: &[target],
                    },
                    camera,
                    &data,
                );
                clear_background(BLACK);
                set_camera(&Camera3D {
                    position: camera,
                    target: center,
                    up: Vec3::Z,
                    fovy: 75_f32.to_radians(),
                    z_near: 2.,
                    z_far: 30000.,
                    ..Default::default()
                });
                scene.draw(camera, 0., false, false, &i.transforms());
                material.atmosphere(&scene.atmosphere, camera);
                visuals.atmosphere(&scene.atmosphere, camera);
                visuals.draw_effects(&material, camera, false, false);
                crate::render::depth_read_only(|| {
                    scene.draw(camera, 0., false, true, &i.transforms())
                });
                set_default_camera();
                draw_text(
                    &format!("STAGED DEMON DICE / {} / {phase:?}", MODELS[kind]),
                    24.,
                    35.,
                    24.,
                    WHITE,
                );
                if frame + 1 == frames {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/dice-{kind}-{phase:?}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
    }
    println!("PASS staged native Demon Dice models, appearance, melee and ranged poses/effects for all three tiers");
    Ok(())
}

#[cfg(test)]
mod tests {
    include!("dice_tests.rs");
    use super::*;
    fn data() -> Data {
        Data {
            clips: std::array::from_fn(|_| Default::default()),
            skeletons: Vec::new(),
            events: Vec::new(),
            attacks: [[Timing {
                duration: 1.4,
                frame: 0.05,
            }; 2]; 3],
        }
    }
    fn floor() -> World {
        World::fixture(&[(vec3(-2000., -2000., -40.), vec3(2000., 2000., 0.))])
    }
    fn demon(kind: usize) -> State {
        State {
            demon: Some(Demon {
                kind,
                feet: Vec3::Z * 0.1,
                yaw: 0.,
                health: HEALTH[kind],
                phase: Phase::Idle,
                time: 0.,
                life: 0.,
                idle: 0.,
                cooldown: 0.,
                target: None,
                hostile: false,
                fired: 0,
                frozen: false,
                recoil: Default::default(),
                shield: 0.,
            }),
            ..State::default()
        }
    }
    fn encoded(s: &State) -> serde_json::Value {
        serde_json::to_value(s).unwrap()
    }
    #[test]
    fn watch_freezes_summons_and_their_bolts_but_thrown_dice_keep_rolling() {
        let world = floor();
        let targets = [Target {
            id: 2,
            center: vec3(400., 0., 80.),
            half: Vec3::splat(20.),
        }];
        let ctx = Context {
            world: &world,
            targets: &targets,
        };
        let eye = Vec3::Z * 60.;
        let mut s = demon(1);
        s.bolts.push(Bolt {
            position: vec3(50., 0., 80.),
            velocity: Vec3::X * 100.,
            age: 0.,
            hostile: false,
            ice: false,
            id: 1,
        });
        let before = encoded(&s);
        for _ in 0..240 {
            assert!(s
                .advance_timed(STEP, false, &ctx, eye, &data())
                .hits
                .is_empty());
        }
        assert_eq!(before["demon"], encoded(&s)["demon"]);
        assert_eq!(before["bolts"], encoded(&s)["bolts"]);
        let mut restored: State = serde_json::from_value(encoded(&s)).unwrap();
        for _ in 0..120 {
            s.advance_timed(STEP, true, &ctx, eye, &data());
            restored.advance_timed(STEP, true, &ctx, eye, &data());
        }
        assert_eq!(encoded(&s), encoded(&restored));
        assert_ne!(before["demon"], encoded(&s)["demon"]);
        assert_ne!(before["bolts"], encoded(&s)["bolts"]);
        let mut rolling = State::default();
        assert!(rolling.throw(1, eye, Vec3::X));
        let start = rolling.dice[0].position;
        rolling.advance_timed(0.1, false, &ctx, eye, &data());
        assert_ne!(start, rolling.dice[0].position);
        assert!(rolling.dice[0].age > 0.);
    }
    #[test]
    fn rolls_respect_collection_counts_and_original_tier_thresholds() {
        assert_eq!(tier(&[6]), 0);
        assert_eq!(tier(&[4, 4]), 0);
        assert_eq!(tier(&[4, 5]), 1);
        assert_eq!(tier(&[4, 4, 4]), 1);
        assert_eq!(tier(&[4, 4, 5]), 2);
        for count in 1..=3 {
            let mut s = State::default();
            assert!(s.throw(count, Vec3::Z * 60., Vec3::X));
            assert_eq!(s.dice.len(), count as usize);
            assert!(s.dice.iter().all(|d| (1..=6).contains(&d.pip)));
            assert!(!s.throw(count, Vec3::ZERO, Vec3::X));
        }
    }
    #[test]
    fn mid_roll_restart_is_deterministic_and_pause_freezes_every_timer() {
        let world = floor();
        let targets = [Target {
            id: 1,
            center: vec3(400., 0., 80.),
            half: Vec3::splat(20.),
        }];
        let ctx = Context {
            world: &world,
            targets: &targets,
        };
        let eye = Vec3::Z * 60.;
        let mut a = State::default();
        a.throw(3, eye, Vec3::X);
        a.advance(0.1, &ctx, eye, &data());
        let saved = encoded(&a);
        let mut b: State = serde_json::from_value(saved.clone()).unwrap();
        b.validate().unwrap();
        b.advance(0., &ctx, eye, &data());
        assert_eq!(saved, encoded(&b));
        for _ in 0..500 {
            let x = a.advance(STEP, &ctx, eye, &data());
            let y = b.advance(STEP, &ctx, eye, &data());
            assert_eq!(encoded(&a), encoded(&b));
            assert_eq!(
                x.hits.iter().map(|h| (h.id, h.damage)).collect::<Vec<_>>(),
                y.hits.iter().map(|h| (h.id, h.damage)).collect::<Vec<_>>()
            );
        }
        assert!(a.demon.is_some());
        let mut bad = a.clone();
        bad.demon.as_mut().unwrap().kind = 9;
        assert!(bad.validate().is_err());
        let mut bad = a;
        bad.cooldown = f32::NAN;
        assert!(bad.validate().is_err());
    }
    #[test]
    fn saves_from_the_first_dice_system_load_validate_and_keep_their_state() {
        // Written before the shield, recoil, freeze, beam, bolt-id, forced-king and world-clock
        // fields existed: a mid-air roll's clocks and a live summon must load unchanged.
        let old = serde_json::json!({
            "dice": [{"position": [10., 20., 30.], "velocity": [40., 0., -20.], "age": 0.25,
                      "resting": false, "pip": 4}],
            "demon": null,
            "bolts": [],
            "seed": 123456,
            "cooldown": 5.75,
            "time": 0.25,
            "accumulator": 0.004
        });
        let rolling: State = serde_json::from_value(old.clone()).unwrap();
        rolling.validate().unwrap();
        let saved = encoded(&rolling);
        for key in ["dice", "demon", "bolts", "seed", "accumulator"] {
            assert_eq!(saved[key], old[key], "{key}");
        }
        assert!(
            (saved["cooldown"].as_f64().unwrap() - 5.75).abs() < 1e-6
                && (saved["time"].as_f64().unwrap() - 0.25).abs() < 1e-6
        );
        assert_eq!(saved["world_accumulator"], 0.0);
        assert_eq!(saved["forced_king"], false);
        for (kind, phase) in [(0, "Idle"), (1, "Melee"), (2, "Ranged"), (2, "Vanish")] {
            let old = serde_json::json!({
                "dice": [],
                "demon": {"kind": kind, "feet": [0., 0., 0.], "yaw": 1.5, "health": 20.,
                          "phase": phase, "time": 1.25, "life": 30., "idle": 0.5,
                          "cooldown": 1., "target": 7, "hostile": kind == 1, "fired": 2},
                "bolts": [{"position": [1., 2., 3.], "velocity": [100., 0., 0.], "age": 0.5,
                           "hostile": true}],
                "seed": 9, "cooldown": 0., "time": 0., "accumulator": 0.
            });
            let s: State = serde_json::from_value(old).unwrap();
            s.validate().unwrap();
            let d = s.demon.as_ref().unwrap();
            assert_eq!((d.kind, d.health, d.time), (kind, 20., 1.25));
            assert!(!d.frozen && d.shield == 0. && d.recoil.valid());
            assert_eq!(encoded(&s)["bolts"][0]["ice"], false);
            let again: State = serde_json::from_value(encoded(&s)).unwrap();
            assert_eq!(encoded(&again), encoded(&s));
        }
    }
    #[test]
    fn all_demons_have_melee_and_ranged_contacts_at_consistent_rates() {
        let world = floor();
        for kind in 0..3 {
            for distance in [70., 400.] {
                let mut results = Vec::new();
                for hz in [30, 60, 144] {
                    let mut s = demon(kind);
                    let center = s.demon.as_ref().unwrap().center();
                    let target = Target {
                        id: 2,
                        center: center + Vec3::X * distance,
                        half: Vec3::splat(20.),
                    };
                    let mut damage = 0.;
                    for _ in 0..hz * 5 {
                        for hit in s
                            .advance(
                                1. / hz as f32,
                                &Context {
                                    world: &world,
                                    targets: &[target],
                                },
                                Vec3::Z * 60.,
                                &data(),
                            )
                            .hits
                        {
                            assert_eq!(hit.id, 2);
                            damage += hit.damage;
                        }
                    }
                    assert!(damage > 0., "kind {kind} distance {distance}");
                    results.push(damage);
                }
                assert_eq!(results[0], results[1]);
                assert_eq!(results[0], results[2]);
            }
        }
    }
    #[test]
    fn walls_stop_attack_damage_and_blocked_spawn_never_places_a_body_in_solid() {
        let world = World::fixture(&[
            (vec3(-2000., -2000., -40.), vec3(2000., 2000., 0.)),
            (vec3(150., -300., 0.), vec3(160., 300., 500.)),
        ]);
        let target = Target {
            id: 2,
            center: vec3(400., 0., 80.),
            half: Vec3::splat(20.),
        };
        for kind in 0..3 {
            let mut s = demon(kind);
            s.demon.as_mut().unwrap().set(Phase::Ranged);
            s.demon.as_mut().unwrap().target = Some(2);
            for _ in 0..600 {
                assert!(s
                    .advance(
                        STEP,
                        &Context {
                            world: &world,
                            targets: &[target]
                        },
                        Vec3::Z * 60.,
                        &data()
                    )
                    .hits
                    .is_empty());
            }
        }
        let sealed = World::fixture(&[
            (vec3(-500., -500., -40.), vec3(500., 500., 0.)),
            (vec3(-500., -500., 65.), vec3(500., 500., 90.)),
        ]);
        for kind in 0..3 {
            assert!(spawn(&sealed, Vec3::Z * 8., kind).is_none());
        }
    }
    #[test]
    fn empty_room_summon_turns_hostile_and_death_stops_attacks() {
        let world = floor();
        let ctx = Context {
            world: &world,
            targets: &[],
        };
        let eye = Vec3::Z * 60.;
        let mut s = State::default();
        s.throw(1, eye, Vec3::X);
        let mut damage = 0.;
        for _ in 0..1200 {
            for h in s.advance(STEP, &ctx, eye, &data()).hits {
                assert_eq!(h.id, ALICE);
                damage += h.damage;
            }
        }
        assert!(damage > 0.);
        s.hurt(1000.);
        assert!(s.target().is_none());
        let mut closes = 0;
        for _ in 0..300 {
            let feedback = s.advance(STEP, &ctx, eye, &data());
            assert!(feedback.hits.is_empty());
            closes += feedback.sounds.iter().filter(|&&s| s == CLOSE).count();
        }
        assert_eq!(closes, 0); // Killed actors use death, not the idle dismissal rift.
        assert!(s.demon.is_none() && s.ready());
    }
    #[test]
    fn notarget_removes_alice_from_hostile_demon_targets() {
        let world = floor();
        let ctx = Context {
            world: &world,
            targets: &[],
        };
        let eye = vec3(80., 0., 60.);
        let mut s = demon(0);
        s.demon.as_mut().unwrap().hostile = true;
        s.notarget = true;
        let before = s.demon.as_ref().unwrap().feet;
        for _ in 0..60 {
            assert!(s.advance(STEP, &ctx, eye, &data()).hits.is_empty());
        }
        assert_eq!(s.demon.as_ref().unwrap().feet, before);
        s.notarget = false;
        let mut hits = 0;
        for _ in 0..240 {
            hits += s.advance(STEP, &ctx, eye, &data()).hits.len();
        }
        assert!(hits > 0);
    }
    #[test]
    fn cleared_enemies_are_not_resurrected_or_replaced_with_alice() {
        let world = floor();
        let mut s = demon(0);
        let target = Target {
            id: 7,
            center: vec3(80., 0., 50.),
            half: Vec3::splat(20.),
        };
        s.advance(
            0.1,
            &Context {
                world: &world,
                targets: &[target],
            },
            Vec3::Z * 60.,
            &data(),
        );
        for _ in 0..500 {
            assert!(s
                .advance(
                    STEP,
                    &Context {
                        world: &world,
                        targets: &[]
                    },
                    Vec3::Z * 60.,
                    &data()
                )
                .hits
                .is_empty());
        }
        assert!(s.demon.is_none());
    }
}
