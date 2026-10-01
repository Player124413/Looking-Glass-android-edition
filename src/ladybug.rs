//! Pool of Tears acorn bombers. Independent AI; original models and event timing.
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::World,
    combat::{self, Feedback, Target},
    flight::Navigator,
    skeletal::{Animation, Definition, Skeleton, Transform},
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

const STEP: f32 = 1. / 120.;
pub const HEALTH: f32 = 39.; // Source health 1039, death sequence starts at 1000.
pub const SOUNDS: [&str; 6] = [
    "sound/character/ladybug/attack_bomb.wav",
    "sound/character/ladybug/pain_center.wav",
    "sound/character/ladybug/death_hit.wav",
    "sound/character/ladybug/death_air.wav",
    "sound/character/ladybug/death_land.wav",
    "sound/character/ladybug/acorn_explode.wav",
];
pub const CLIPS: [&str; 9] = [
    "idle",
    "fly_normal_acorn",
    "fly_normal",
    "attack_bomb",
    "pain_center_acorn",
    "pain_center",
    "death_hit",
    "death_air",
    "death_land",
];
pub fn activation(thread: &str) -> Option<[(&'static str, f64); 2]> {
    Some(match thread {
        "Spawn_LadyX1" => [("x_lady1", 0.), ("x_lady2", 2.1)],
        "Spawn_LadyX2" => [("x_lady3", 0.), ("x_lady4", 2.1)],
        "Spawn_LadyX3" => [("x_lady5", 0.), ("x_lady6", 2.1)],
        "Spawn_LadyX4" => [("x_lady7", 0.), ("x_lady8", 2.1)],
        "ladies1and2" => [("lady1", 0.), ("lady2", 0.)],
        "ladies3and4" => [("lady3", 0.), ("lady4", 0.)],
        _ => return None,
    })
}
#[derive(Clone, Copy)]
pub struct Timing {
    attack: f32,
    release: f32,
    muzzle: Vec3,
    death_muzzle: Vec3,
    pain: f32,
    death: f32,
}
impl Timing {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let d = Definition::load(assets, "models/c_ladybug.tik")?;
        let skeleton = Skeleton::parse(&assets.read(&format!("{}/{}", d.path, d.model))?)?;
        let tag = skeleton
            .bones
            .iter()
            .position(|b| b.name == "tag_weapon")
            .context("Ladybug weapon tag")?;
        let mut read = |n: &str| {
            Animation::parse(
                &assets.read(&format!("{}/{}", d.path, d.animations[n]))?,
                skeleton.bones.len(),
            )
        };
        for name in CLIPS {
            read(name)?;
        }
        let attack = read("attack_bomb")?;
        let release = attack.frame_time * 10.;
        let death = read("death_hit")?;
        ensure!(
            release < attack.duration(),
            "Ladybug release exceeds animation"
        );
        Ok(Self {
            attack: attack.duration(),
            release,
            muzzle: skeleton.global_pose(&attack.sample(release, false))[tag].translation * d.scale,
            death_muzzle: skeleton.global_pose(&death.sample(0., false))[tag].translation * d.scale,
            pain: read("pain_center")?.duration(),
            death: death.duration(),
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum State {
    Patrol,
    Chase,
    Attack,
    Return,
    Pain,
    Dead,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Acorn {
    pub position: Vec3,
    velocity: Vec3,
    pub age: f32,
}
#[derive(Clone, Default)]
pub struct Burst {
    pub position: Vec3,
    pub age: f32,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Ladybug {
    #[serde(default, skip_serializing_if = "crate::electric::inactive")]
    pub electric: f32,
    #[serde(default)]
    pub opponents: combat::Opponents,
    #[serde(default)]
    pub frozen: bool,
    #[serde(default)]
    pub recoil: combat::Recoil,
    pub feet: Vec3,
    pub yaw: f32,
    pub scale: f32,
    pub health: f32,
    pub state: State,
    pub time: f32,
    pub carrying: bool,
    pub notarget: bool,
    pub acorns: Vec<Acorn>,
    #[serde(skip)]
    pub bursts: Vec<Burst>,
    pub drops: u32,
    pub patrol_started: bool,
    home: Vec3,
    route: Vec<Vec3>,
    waypoint: usize,
    navigation: Navigator,
    reload: f32,
    released: bool,
    memory: f32,
    last_seen: Vec3,
    falling: f32,
    landed: Option<f32>,
    accumulator: f64,
}
impl Ladybug {
    pub fn new(feet: Vec3, yaw: f32, scale: f32, route: Vec<Vec3>) -> Self {
        Self {
            opponents: Default::default(),
            frozen: false,
            recoil: combat::Recoil::default(),
            electric: 0.,
            feet,
            yaw,
            scale,
            health: HEALTH,
            state: State::Patrol,
            time: 0.,
            carrying: true,
            notarget: false,
            acorns: vec![],
            bursts: vec![],
            drops: 0,
            patrol_started: false,
            home: feet,
            route,
            waypoint: 0,
            navigation: Navigator::default(),
            reload: 0.,
            released: false,
            memory: 0.,
            last_seen: feet,
            falling: 0.,
            landed: None,
            accumulator: 0.,
        }
    }
    pub fn validate_save(&self, original: &Self) -> Result<()> {
        ensure!(
            self.home == original.home
                && self.route == original.route
                && self.scale == original.scale
                && (0.0..=HEALTH).contains(&self.health)
                && self.feet.is_finite()
                && self.recoil.valid()
                && (0. ..=crate::electric::LIFE).contains(&self.electric)
                && (!self.frozen || self.health == 0.)
                && self.yaw.is_finite()
                && (0.0..=1e8).contains(&self.time)
                && (0.0..=3.).contains(&self.reload)
                && (0.0..=5.).contains(&self.memory)
                && self.last_seen.is_finite()
                && self.waypoint < self.route.len().max(1)
                && self.accumulator.is_finite()
                && self.accumulator.abs() < 0.01
                && (0.0..=500.).contains(&self.falling)
                && self
                    .landed
                    .is_none_or(|t| t.is_finite() && t >= 0. && t <= self.time)
                && ((self.health == 0.) == (self.state == State::Dead))
                && self.acorns.len() <= 16
                && self.acorns.iter().all(|p| p.position.is_finite()
                    && p.velocity.is_finite()
                    && p.velocity.length() <= 2500.
                    && (0.0..=3.).contains(&p.age)),
            "Invalid saved Ladybug"
        );
        self.navigation.validate()
    }
    pub fn visual_scale(&self) -> f32 {
        let fade = if self.frozen {
            (2.5 - self.time).clamp(0., 1.)
        } else {
            self.landed.map_or(
                if self.state == State::Dead {
                    (12. - self.time).clamp(0., 1.)
                } else {
                    1.
                },
                |t| (3. - (self.time - t)).clamp(0., 1.),
            )
        };
        self.scale * fade
    }
    pub fn target(&self, id: usize) -> Target {
        Target {
            id,
            center: self.feet + Vec3::Z * 32. * self.scale,
            half: Vec3::splat(32. * self.scale),
        }
    }
    fn enter(&mut self, state: State) {
        self.state = state;
        self.time = 0.;
        self.released = false;
        self.navigation.reset();
    }
    pub fn hit_attack(&mut self, hit: combat::Hit) -> Option<&'static str> {
        if self.health > 0. && hit.damage.is_finite() && hit.damage > 0. {
            self.recoil.hit(hit.knockback, 150.);
            crate::electric::hit(&mut self.electric, hit);
        }
        let was_alive = self.health > 0.;
        let sound = self.hit(hit.damage);
        if was_alive && self.health == 0. && hit.kind == combat::DamageKind::Ice {
            self.frozen = true;
            Some("sound/character/shared/freeze_death.wav")
        } else {
            sound
        }
    }
    pub fn hit(&mut self, damage: f32) -> Option<&'static str> {
        if self.health <= 0. || !damage.is_finite() || damage <= 0. {
            return None;
        }
        self.health = (self.health - damage).max(0.);
        if self.health == 0. {
            self.enter(State::Dead);
            Some(SOUNDS[2])
        } else {
            if self.state != State::Pain {
                self.enter(State::Pain);
            }
            Some(SOUNDS[1])
        }
    }
    pub fn clip(&self, timing: Timing) -> (&'static str, f32, bool) {
        if self.frozen {
            return ("death_frozen", self.time.min(0.05), false);
        }
        match self.state {
            State::Attack => ("attack_bomb", self.time, false),
            State::Pain => (
                if self.carrying {
                    "pain_center_acorn"
                } else {
                    "pain_center"
                },
                self.time,
                false,
            ),
            State::Dead if self.landed.is_some() => {
                ("death_land", self.time - self.landed.unwrap(), false)
            }
            State::Dead if self.time < timing.death => ("death_hit", self.time, false),
            State::Dead => ("death_air", self.time - timing.death, true),
            _ => (
                if self.carrying {
                    "fly_normal_acorn"
                } else {
                    "fly_normal"
                },
                self.time,
                true,
            ),
        }
    }
    pub fn advance(&mut self, dt: f32, world: &World, eye: Vec3, timing: Timing) -> Feedback {
        let mut out = Feedback::default();
        if dt <= 0. || !dt.is_finite() {
            return out;
        }
        self.accumulator += dt.min(0.1) as f64;
        while self.accumulator + 1e-9 >= 1. / 120. {
            self.accumulator -= 1. / 120.;
            self.step(world, eye, timing, &mut out);
        }
        out
    }
    fn step(&mut self, world: &World, eye: Vec3, timing: Timing, out: &mut Feedback) {
        self.electric = (self.electric - 1. / 120.).max(0.);
        self.feet += self.recoil.step(STEP, world, self.target(0));
        self.bursts.retain_mut(|b| {
            b.age += STEP;
            b.age < 0.6
        });
        let targets = self.opponents.bodies(eye);
        self.acorns.retain_mut(|p| {
            p.age += STEP;
            p.velocity.z -= 800. * STEP;
            let end = p.position + p.velocity * STEP;
            let ctx = combat::Context {
                world,
                targets: &targets,
            };
            let contact = combat::contact(&ctx, p.position, end, 8.);
            let wall = world.sweep(p.position, end, Vec3::splat(8.));
            p.position = p
                .position
                .lerp(end, contact.map_or(wall.fraction, |(_, f)| f));
            let impact = contact.is_some()
                || wall.start_solid
                || wall.fraction < 1.
                || p.age >= 3.
                || world.liquid_at(p.position) != 0;
            if impact {
                if let Some((id, _)) = contact {
                    out.strike(id, 20., Vec3::ZERO, combat::DamageKind::Other);
                }
                for player in &targets {
                    let distance = player.center.distance(p.position);
                    if !wall.start_solid
                        && distance < 192.
                        && visible(world, p.position, player.center)
                    {
                        let strength = 1. - distance / 192.;
                        out.strike(
                            player.id,
                            25. * strength,
                            (player.center - p.position).normalize_or_zero() * 200. * strength,
                            combat::DamageKind::Other,
                        );
                    }
                }
                self.bursts.push(Burst {
                    position: p.position,
                    age: 0.,
                });
                out.sounds.push(SOUNDS[5]);
            }
            !impact
        });
        let previous = self.time;
        self.time += STEP;
        self.reload = (self.reload - STEP).max(0.);
        self.memory = (self.memory - STEP).max(0.);
        let body = self.target(0);
        if self.state == State::Dead {
            if self.carrying {
                self.drop_acorn(world, timing.death_muzzle);
            }
            if previous < timing.death && self.time >= timing.death && self.landed.is_none() {
                out.sounds.push(SOUNDS[3]);
            }
            if self.landed.is_none() {
                self.falling = (self.falling + 500. * STEP).min(500.);
                let end = body.center - Vec3::Z * self.falling * STEP;
                let hit = world.sweep(body.center, end, body.half);
                if !hit.start_solid {
                    self.feet += body.center.lerp(end, hit.fraction) - body.center;
                }
                if hit.start_solid
                    || (hit.fraction < 1. && hit.normal.z > 0.5)
                    || world.liquid_at(self.feet) != 0
                {
                    self.landed = Some(self.time);
                    out.sounds.push(SOUNDS[4]);
                }
            }
            return;
        }
        let (eye, notarget, _) = self.opponents.aim(
            world,
            body.center,
            eye,
            self.notarget,
            self.state == State::Attack,
        );
        let sees =
            !notarget && body.center.distance(eye) < 2048. && visible(world, body.center, eye);
        if sees {
            self.last_seen = eye;
            self.memory = 5.;
        }
        if notarget {
            self.memory = 0.;
        }
        match self.state {
            State::Pain => {
                if self.time >= timing.pain {
                    self.enter(if self.carrying {
                        State::Patrol
                    } else {
                        State::Return
                    });
                }
            }
            State::Attack => {
                if notarget && !self.released {
                    self.enter(State::Patrol);
                    return;
                }
                if !self.released && self.time >= timing.release {
                    self.released = true;
                    self.reload = 2.;
                    self.drop_acorn(world, timing.muzzle);
                }
                if self.time >= timing.attack {
                    self.enter(State::Return);
                }
            }
            _ => {
                let pursue = self.carrying && self.memory > 0.;
                let patrol = if self.patrol_started {
                    self.route.get(self.waypoint).copied().unwrap_or(self.home)
                } else {
                    self.home
                } + Vec3::Z * 32. * self.scale;
                let goal = if pursue {
                    self.last_seen + Vec3::Z * 192.
                } else {
                    patrol
                };
                let next_state = if pursue {
                    State::Chase
                } else if self.carrying {
                    State::Patrol
                } else {
                    State::Return
                };
                if self.state != next_state {
                    self.enter(next_state);
                }
                let next = self.navigation.advance(
                    world,
                    body.center,
                    goal,
                    body.half,
                    if pursue { 240. } else { 180. },
                    STEP,
                );
                let delta = if next.distance(body.center) > 0.01 {
                    next - body.center
                } else {
                    eye - body.center
                };
                if delta.truncate().length_squared() > 0.01 {
                    let angle = delta.y.atan2(delta.x);
                    let turn = (angle - self.yaw + std::f32::consts::PI)
                        .rem_euclid(std::f32::consts::TAU)
                        - std::f32::consts::PI;
                    self.yaw += turn.clamp(-4. * STEP, 4. * STEP);
                }
                self.feet += next - body.center;
                if !pursue && next.distance(patrol) < 24. {
                    if !self.carrying && self.reload == 0. {
                        self.carrying = true;
                        self.enter(State::Patrol);
                    }
                    if self.patrol_started && !self.route.is_empty() {
                        self.waypoint = (self.waypoint + 1) % self.route.len();
                    }
                }
                let horizontal = (self.feet - eye).truncate().length();
                let height = self.target(0).center.z - eye.z;
                if sees
                    && self.carrying
                    && self.reload == 0.
                    && horizontal < 80.
                    && (120.0..=260.).contains(&height)
                {
                    self.enter(State::Attack);
                    out.sounds.push(SOUNDS[0]);
                }
            }
        }
    }
    fn drop_acorn(&mut self, world: &World, offset: Vec3) {
        self.carrying = false;
        let center = self.target(0).center;
        let muzzle = self.feet + Quat::from_rotation_z(self.yaw) * offset * self.scale;
        let trace = world.sweep(center, muzzle, Vec3::splat(8.));
        if !trace.start_solid {
            self.acorns.push(Acorn {
                position: center.lerp(muzzle, trace.fraction),
                velocity: Vec3::ZERO,
                age: 0.,
            });
            self.drops += 1;
        }
    }
}
fn visible(world: &World, a: Vec3, b: Vec3) -> bool {
    let t = world.sweep(a, b, Vec3::splat(0.5));
    !t.start_solid && t.fraction >= 1.
}
/// Narrow reviewed bindings, not interpretation of arbitrary script threads.
pub fn route_for(map: &Bsp, name: &str) -> Result<Vec<Vec3>> {
    let path = if let Some(n) = name
        .strip_prefix("lady")
        .and_then(|n| n.parse::<usize>().ok())
    {
        format!("lady{n}path")
    } else if let Some(n) = name
        .strip_prefix("x_lady")
        .and_then(|n| n.parse::<usize>().ok())
    {
        ensure!((1..=8).contains(&n), "Unknown Ladybug ambush");
        format!("xladypath{}", n.div_ceil(2))
    } else {
        anyhow::bail!("Unknown Ladybug patrol {name}");
    };
    route_from(map, &path)
}
pub(crate) fn route_from(map: &Bsp, path: &str) -> Result<Vec<Vec3>> {
    let mut current = path;
    let mut visited = std::collections::BTreeSet::new();
    let mut result = Vec::new();
    while visited.insert(current) {
        let e = map
            .entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|n| n == current))
            .with_context(|| format!("Missing Ladybug path {current}"))?;
        ensure!(
            e.get("classname").is_some_and(|s| s == "info_splinepath"),
            "Invalid Ladybug path node"
        );
        result.push(
            e.get("origin")
                .and_then(|s| crate::interaction::vector(s))
                .context("Invalid flight path position")?,
        );
        ensure!(result.len() <= 256, "Flight path too long");
        current = e.get("target").context("Open Ladybug patrol")?;
    }
    ensure!(
        current == path && result.len() >= 2,
        "Ladybug patrol does not loop"
    );
    Ok(result)
}
pub fn check(assets: &mut Assets) -> Result<()> {
    use crate::{
        encounters::{Encounters, Enemy},
        interaction::Interactions,
    };
    let map = Bsp::parse(&assets.read("maps/potears1.bsp")?)?;
    let world = World::from_bsp(&map)?;
    let timing = Timing::load(assets)?;
    let cast = Encounters::load(assets, &map, "potears1", None)?;
    ensure!(
        cast.actors.len() == 12 && cast.actors.iter().filter(|a| a.active).count() == 4,
        "Wrong Pool of Tears Ladybug cast"
    );
    for a in &cast.actors {
        if let Enemy::Ladybug(b) = &a.enemy {
            let t = b.target(0);
            ensure!(
                !world.sweep(t.center, t.center, t.half).start_solid,
                "Obstructed Ladybug spawn {}",
                a.name
            );
            println!(
                "Ladybug {}: {} patrol nodes, active={}",
                a.name,
                b.route.len(),
                a.active
            );
            let mut patrolling = b.clone();
            patrolling.patrol_started = true;
            patrolling.notarget = true;
            let mut progressed = false;
            for _ in 0..900 {
                patrolling.advance(1. / 30., &world, Vec3::ZERO, timing);
                progressed |= patrolling.waypoint != 0;
                let t = patrolling.target(0);
                ensure!(
                    !world.sweep(t.center, t.center, t.half).start_solid,
                    "Ladybug {} clipped during its patrol",
                    a.name
                );
            }
            ensure!(
                progressed && patrolling.feet.distance(b.feet) > 24.,
                "Ladybug {} failed to progress on its authored patrol: {:?}",
                a.name,
                patrolling.feet
            );
            println!(
                "PASS {} 30-second authored patrol: waypoint {}",
                a.name, patrolling.waypoint
            );
        }
    }
    for hz in [30, 60, 144] {
        let mut i = Interactions::load(&map)?;
        i.set_entry(assets, &map, "potears1", None)?;
        for e in &map.entities {
            let Some(thread) = e.get("thread") else {
                continue;
            };
            let Some(groups) = activation(thread) else {
                continue;
            };
            let center = e
                .get("origin")
                .and_then(|s| crate::interaction::vector(s))
                .context("Missing ambush volume")?;
            let feet = center - Vec3::Z * 28.;
            let result = i.triggers(1. / hz as f32, feet - Vec3::X * 100., feet);
            ensure!(
                result.message.is_none(),
                "Supported Ladybug trigger still pending"
            );
            let is_ready = |i: &Interactions, name: &str| {
                i.encounters.as_ref().unwrap().actors.iter().any(|a| {
                    a.name == name
                        && a.active
                        && matches!(&a.enemy, Enemy::Ladybug(b) if b.patrol_started)
                })
            };
            ensure!(
                is_ready(&i, groups[0].0),
                "First Ladybug did not activate: {thread}"
            );
            if groups[1].1 > 0. {
                ensure!(!is_ready(&i, groups[1].0), "Second Ladybug activated early");
            }
            let frozen = serde_json::to_value(i.snapshot())?;
            i.triggers(0., feet, feet);
            ensure!(
                serde_json::to_value(i.snapshot())? == frozen,
                "Paused activation advanced"
            );
            // Reload between the pair, then let the original pending event finish.
            let saved = i.snapshot();
            let mut restored = Interactions::load(&map)?;
            restored.set_entry(assets, &map, "potears1", None)?;
            restored.restore(&saved, &map)?;
            i = restored;
            for _ in 0..hz * 3 {
                i.triggers(1. / hz as f32, Vec3::splat(-90000.), Vec3::splat(-90000.));
            }
            ensure!(
                is_ready(&i, groups[1].0),
                "Delayed Ladybug did not activate"
            );
            let encounter = i.encounters.as_mut().unwrap();
            let index = encounter
                .actors
                .iter()
                .position(|a| a.name == groups[0].0)
                .unwrap();
            encounter.hit(combat::Hit {
                knockback: Vec3::ZERO,
                kind: crate::combat::DamageKind::Other,
                id: crate::encounters::BASE + index,
                damage: 100.,
            });
            let history = i.event_world.snapshot();
            i.reset_contacts();
            i.triggers(1. / hz as f32, feet, feet);
            ensure!(
                history
                    .usage
                    .iter()
                    .filter(|(k, _)| k.starts_with("ladybug/") || k.starts_with("actor/"))
                    .all(|(k, v)| i.event_world.snapshot().usage.get(k) == Some(v)),
                "Ambush repeated"
            );
            ensure!(
                matches!(&i.encounters.as_ref().unwrap().actors[index].enemy, Enemy::Ladybug(b) if b.health == 0.),
                "Ambush revived dead enemy"
            );
        }
        println!("PASS Ladybug patrol/ambush triggers, delayed restart, pause and no resurrection at {hz} Hz");
    }
    // Staged combat at a real placed actor; Alice's stationary target isolates AI/damage.
    let original = cast
        .actors
        .iter()
        .find_map(|a| match &a.enemy {
            Enemy::Ladybug(b) if a.name == "lady3" => Some(b.clone()),
            _ => None,
        })
        .unwrap();
    let eye = original.target(0).center - Vec3::Z * 192.;
    let mut results = vec![];
    for hz in [30, 60, 144] {
        let mut bug = original.clone();
        let mut damage = 0.;
        for _ in 0..hz * 8 {
            damage += bug.advance(1. / hz as f32, &world, eye, timing).damage;
        }
        ensure!(
            bug.drops > 0 && damage > 0.,
            "Ladybug never hit the school-following area target: drops={} damage={damage}",
            bug.drops
        );
        bug.hit(combat::weapon_damage(0, false));
        ensure!(bug.state == State::Pain, "Blade did not cause pain");
        bug.hit(combat::weapon_damage(0, true));
        ensure!(
            bug.state == State::Dead,
            "Thrown Blade did not defeat Ladybug"
        );
        results.push((bug.drops, damage));
        println!("PASS real-map Ladybug combat at {hz} Hz: {} drops, {damage:.2} damage, pain and defeat", bug.drops);
    }
    ensure!(
        results.windows(2).all(|w| w[0] == w[1]),
        "Frame-dependent Ladybug combat"
    );
    Ok(())
}
/// Explicitly staged snapshots; combat and trigger checks run separately.
pub async fn render_check(assets: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(assets, "potears1")?;
    let mut logic = crate::interaction::Interactions::load(&scene.map)?;
    logic.set_entry(assets, &scene.map, "potears1", None)?;
    let timing = Timing::load(assets)?;
    let specs = crate::texture::read_materials(assets)?;
    let mut art = Art::load(assets, &specs)?;
    let material = crate::character::skin_material()?;
    let original = logic
        .encounters
        .as_ref()
        .unwrap()
        .actors
        .iter()
        .find_map(|a| match &a.enemy {
            crate::encounters::Enemy::Ladybug(b) if a.name == "lady3" => Some(b.clone()),
            _ => None,
        })
        .context("Resident Ladybug fixture")?;
    let eye = original.target(0).center - Vec3::Z * 192.;
    for stage in [
        "patrol", "attack", "drop", "blast", "pain", "death", "landed",
    ] {
        let mut b = original.clone();
        if matches!(stage, "pain" | "death" | "landed") {
            b.hit(if stage == "pain" { 10. } else { 100. });
        }
        b.notarget = stage == "patrol";
        let mut reached = false;
        for step in 0..2400 {
            b.advance(STEP, &scene.world, eye, timing);
            reached = match stage {
                "patrol" => step > 24,
                "attack" => b.state == State::Attack && b.time > timing.release * 0.5,
                "drop" => b.acorns.first().is_some_and(|p| p.age > 0.15),
                "blast" => b.bursts.first().is_some_and(|p| p.age > 0.1),
                "pain" | "death" => b.time > 0.15,
                "landed" => b.landed.is_some_and(|t| b.time - t > 0.2),
                _ => false,
            };
            if reached {
                break;
            }
        }
        ensure!(reached, "Ladybug visual stage never reached: {stage}");
        let mut center = b.target(0).center;
        if stage == "blast" {
            center = center.lerp(b.bursts[0].position, 0.5);
        }
        let camera = (0..24)
            .find_map(|n| {
                let angle = n as f32 * std::f32::consts::TAU / 24.;
                let p = center + vec3(angle.cos() * 280., angle.sin() * 280., 80.);
                (!scene.world.sweep(p, p, Vec3::splat(4.)).start_solid
                    && visible(&scene.world, p, center))
                .then_some(p)
            })
            .context("Clear Ladybug fixture camera")?;
        println!(
            "Ladybug {stage}: feet={:?}, camera={camera:?}, carrying={}, bombs={}, bursts={}",
            b.feet,
            b.carrying,
            b.acorns.len(),
            b.bursts.len()
        );
        for frame in 0..3 {
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: camera,
                target: center,
                up: Vec3::Z,
                fovy: 65_f32.to_radians(),
                z_near: 2.,
                z_far: 30000.,
                ..Default::default()
            });
            scene.draw(camera, 0., false, false, &logic.transforms());
            material.atmosphere(&scene.atmosphere, camera);
            material.bind();
            art.draw(&b, timing, false);
            gl_use_default_material();
            crate::render::depth_read_only(|| {
                scene.draw(camera, 0., false, true, &logic.transforms());
                Art::effects(&b);
            });
            set_default_camera();
            draw_text(&format!("STAGED LADYBUG / {stage}"), 24., 35., 24., WHITE);
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/ladybug-{stage}.png"
                )))?;
            }
            next_frame().await;
        }
    }
    println!(
        "PASS staged native Ladybug patrol, bomb release/blast, pain, falling death and landing"
    );
    Ok(())
}
pub struct Art {
    actor: crate::npc::Puppet,
    acorn: crate::weapons::Prop,
}
impl Art {
    pub fn load(
        assets: &mut Assets,
        specs: &std::collections::BTreeMap<String, crate::texture::MaterialSpec>,
    ) -> Result<Self> {
        Ok(Self {
            actor: crate::npc::Puppet::load(assets, "c_ladybug", &CLIPS, specs)?,
            acorn: crate::weapons::Prop::load_animation(assets, "prj_acorn", "acorn", specs)?,
        })
    }
    pub fn draw(&mut self, b: &Ladybug, timing: Timing, fullbright: bool) {
        let (clip, time, looping) = b.clip(timing);
        let scale = b.visual_scale();
        self.actor.show_attachments(b.carrying);
        self.actor.draw(
            clip,
            time,
            looping,
            Transform {
                translation: b.feet,
                rotation: Quat::from_rotation_z(b.yaw),
            },
            scale,
            fullbright,
        );
        self.actor.draw_electric(b.electric);
        for p in &b.acorns {
            self.acorn.draw_frame(
                Transform {
                    translation: p.position,
                    rotation: Quat::from_euler(
                        EulerRot::XYZ,
                        p.age * 1.75,
                        p.age * 3.5,
                        p.age * 5.25,
                    ),
                },
                1.,
                fullbright,
                p.age,
                true,
            );
        }
    }
    pub fn effects(b: &Ladybug) {
        for burst in &b.bursts {
            let r = 12. + burst.age * 250.;
            let color = Color::new(1., 0.65, 0.15, (1. - burst.age / 0.6) * 0.8);
            for i in 0..24 {
                let a = i as f32 * std::f32::consts::TAU / 24.;
                let c = (i + 1) as f32 * std::f32::consts::TAU / 24.;
                draw_line_3d(
                    burst.position + vec3(a.cos(), a.sin(), 0.) * r,
                    burst.position + vec3(c.cos(), c.sin(), 0.) * r,
                    color,
                );
                draw_line_3d(
                    burst.position,
                    burst.position + vec3(a.cos(), a.sin(), ((i % 5) as f32 - 2.) * 0.4) * r * 0.5,
                    color,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acorns_and_blasts_hit_demons_separately_from_alice() {
        let world = floor();
        let mut b = bug();
        b.notarget = true;
        b.opponents.summon = Some(Target {
            id: crate::dice::SUMMON,
            center: vec3(0., 0., 40.),
            half: vec3(40., 40., 40.),
        });
        let mut damage = 0.;
        for _ in 0..120 * 8 {
            let f = b.advance(STEP, &world, vec3(-800., 0., 60.), timing());
            assert_eq!(f.damage, 0.);
            damage += f.summon_hits.iter().map(|h| h.damage).sum::<f32>();
        }
        assert!(b.drops > 0 && damage > 0.);
    }
    fn timing() -> Timing {
        Timing {
            attack: 0.8,
            release: 0.4,
            muzzle: Vec3::ZERO,
            death_muzzle: Vec3::ZERO,
            pain: 0.4,
            death: 0.35,
        }
    }
    fn floor() -> World {
        World::fixture(&[(vec3(-2000., -2000., -64.), vec3(2000., 2000., 0.))])
    }
    fn bug() -> Ladybug {
        Ladybug::new(vec3(0., 0., 240.), 0., 1., vec![])
    }
    fn json(b: &Ladybug) -> serde_json::Value {
        serde_json::to_value(b).unwrap()
    }
    #[test]
    fn bomb_drop_damage_and_reload_are_fixed_step_and_dodgeable() {
        let world = floor();
        let eye = Vec3::Z * 60.;
        let mut results = vec![];
        for hz in [30, 60, 144] {
            let mut b = bug();
            let mut damage = 0.;
            for _ in 0..hz * 8 {
                damage += b.advance(1. / hz as f32, &world, eye, timing()).damage;
            }
            assert!(b.drops >= 2 && damage > 40.);
            results.push((b.drops, damage));
        }
        assert_eq!(results[0], results[1]);
        assert_eq!(results[0], results[2]);
        let mut b = bug();
        let mut damage = 0.;
        for _ in 0..200 {
            let target = if b.drops == 0 {
                eye
            } else {
                eye + Vec3::X * 700.
            };
            damage += b.advance(STEP, &world, target, timing()).damage;
        }
        assert!(b.drops > 0);
        assert_eq!(damage, 0.);
    }
    #[test]
    fn pain_interrupts_attack_and_death_drops_only_the_carried_bomb() {
        let world = floor();
        let mut b = bug();
        for _ in 0..20 {
            b.advance(STEP, &world, Vec3::Z * 60., timing());
        }
        assert_eq!(b.state, State::Attack);
        b.hit(10.);
        assert_eq!(b.state, State::Pain);
        let frozen = json(&b);
        b.advance(0., &world, Vec3::ZERO, timing());
        assert_eq!(frozen, json(&b));
        for _ in 0..30 {
            b.advance(STEP, &world, Vec3::Z * 60., timing());
        }
        assert_eq!(b.drops, 0);
        b.hit(29.);
        assert_eq!(b.health, 0.);
        let mut landed = 0;
        for _ in 0..600 {
            let f = b.advance(STEP, &world, Vec3::Z * 60., timing());
            landed += f.sounds.iter().filter(|&&s| s == SOUNDS[4]).count();
        }
        assert_eq!(b.drops, 1);
        assert!(!b.carrying);
        assert_eq!(landed, 1);
        assert!(b.landed.is_some() && b.feet.z >= -0.1);
        assert!(b.hit(10.).is_none());
    }
    #[test]
    fn bombs_and_blasts_cannot_damage_through_walls() {
        let world = World::fixture(&[
            (vec3(-1000., -1000., -40.), vec3(1000., 1000., 0.)),
            (vec3(30., -300., 0.), vec3(45., 300., 400.)),
        ]);
        let mut b = bug();
        b.notarget = true;
        b.acorns.push(Acorn {
            position: vec3(0., 0., 100.),
            velocity: Vec3::ZERO,
            age: 0.,
        });
        for _ in 0..600 {
            assert_eq!(
                b.advance(STEP, &world, vec3(80., 0., 55.), timing()).damage,
                0.
            );
        }
        assert_eq!(b.drops, 0);
        assert!(b.acorns.is_empty());
    }
    #[test]
    fn save_resumes_in_flight_bomb_and_rejects_changed_patrol() {
        let world = floor();
        let original = bug();
        let mut a = original.clone();
        while a.acorns.is_empty() {
            a.advance(STEP, &world, Vec3::Z * 60., timing());
        }
        let mut b: Ladybug = serde_json::from_value(json(&a)).unwrap();
        b.validate_save(&original).unwrap();
        for _ in 0..500 {
            let fa = a.advance(STEP, &world, Vec3::Z * 60., timing());
            let fb = b.advance(STEP, &world, Vec3::Z * 60., timing());
            assert_eq!(fa.damage, fb.damage);
            assert_eq!(fa.sounds, fb.sounds);
            assert_eq!(json(&a), json(&b));
        }
        b.route.push(Vec3::ZERO);
        assert!(b.validate_save(&original).is_err());
        b = a;
        b.reload = f32::NAN;
        assert!(b.validate_save(&original).is_err());
    }
    #[test]
    fn saving_before_an_armed_death_drop_does_not_lose_or_duplicate_it() {
        let world = floor();
        let original = bug();
        let mut a = original.clone();
        a.hit(HEALTH);
        let mut b: Ladybug = serde_json::from_value(json(&a)).unwrap();
        b.validate_save(&original).unwrap();
        for _ in 0..600 {
            let fa = a.advance(STEP, &world, Vec3::Z * 60., timing());
            let fb = b.advance(STEP, &world, Vec3::Z * 60., timing());
            assert_eq!(fa.damage, fb.damage);
            assert_eq!(json(&a), json(&b));
        }
        assert_eq!(b.drops, 1);
        assert!(b.landed.is_some() && b.acorns.is_empty());
    }
}
