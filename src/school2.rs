//! Explicit school-two movers and quest state. Original assets are read, not executed.
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    interaction::vector,
    movement::Player,
    skeletal::Transform,
    weapons::{read_model, Prop},
};
use crate::{
    boojum::{Boojum, Timing as BoojumTiming},
    combat::{Feedback, Guard, Target, Timing},
    interaction::Events,
    school2_quest::{Quest, Stage},
};
use anyhow::{Context, Result};
pub(crate) mod cinema;
pub(crate) mod cinema_check;
pub(crate) mod encounters;
pub(crate) mod potion_check;
pub(crate) mod rage_hint;
pub const ENEMY_BASE: usize = 1_000_000;
use macroquad::prelude::*;
use std::collections::BTreeMap;

const INLINE: &[&str] = &[
    "floating_plat1",
    "t2",
    "gym_pend1",
    "gym_pend2",
    "gym_pend3",
    "slamming_door1",
    "slamming_door2",
    "jumbo_door1",
    "jumbo_door2",
    "boojum_clip",
    "exit_portal",
];
const PROPS: &[&str] = &[
    "floating_books1",
    "floating_cabinet1",
    "gym_light1",
    "gym_light2",
    "gym_light3",
];
pub fn supported(e: &BTreeMap<String, String>) -> bool {
    e.get("targetname")
        .is_some_and(|n| INLINE.contains(&n.as_str()))
}
fn entity<'a>(map: &'a Bsp, name: &str) -> Result<&'a BTreeMap<String, String>> {
    map.entities
        .iter()
        .find(|e| e.get("targetname").is_some_and(|s| s == name))
        .with_context(|| format!("Missing school-two object {name}"))
}
fn origin(map: &Bsp, name: &str) -> Result<Vec3> {
    entity(map, name)?
        .get("origin")
        .and_then(|s| vector(s))
        .context("Invalid school-two origin")
}
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Pose {
    pub origin: Vec3,
    pub rotation: Quat,
}
#[derive(Clone)]
enum Shape {
    Brush(usize),
    Mesh(Vec<[Vec3; 3]>),
}
impl Shape {
    fn collider(&self, map: &Bsp, pose: Pose) -> Result<Collider> {
        match self {
            Self::Brush(i) => Collider::model(map, *i, pose.origin, pose.rotation, true),
            Self::Mesh(t) => Ok(Collider::triangles(t, pose.origin, pose.rotation)),
        }
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
enum Motion {
    Still,
    Pendulum {
        pivot: Vec3,
        roll: bool,
        period: f32,
    },
    Path {
        points: Vec<Vec3>,
        period: f32,
        spin: bool,
    },
    Open {
        angle: f32,
        duration: f32,
    },
    Slam {
        angle: f32,
        down: f32,
        up: f32,
    },
}
#[derive(Clone)]
struct Object {
    name: String,
    prop: Option<String>,
    scale: f32,
    shape: Shape,
    base: Pose,
    pose: Pose,
    motion: Motion,
    time: f32,
    collider: Collider,
    enabled: bool,
}
impl Object {
    fn sample(&self, time: f32) -> Pose {
        match &self.motion {
            Motion::Still => self.base,
            Motion::Open { angle, duration } => Pose {
                rotation: Quat::from_rotation_z((angle * (time / duration).min(1.)).to_radians()),
                ..self.base
            },
            Motion::Pendulum {
                pivot,
                roll,
                period,
            } => {
                let a = (time * std::f32::consts::TAU / period).sin() * 25_f32.to_radians();
                let q = if *roll {
                    Quat::from_rotation_x(a)
                } else {
                    Quat::from_rotation_y(-a)
                };
                Pose {
                    origin: *pivot + q * (self.base.origin - *pivot),
                    rotation: q * self.base.rotation,
                }
            }
            Motion::Path {
                points,
                period,
                spin,
            } => {
                let t = (time / period).rem_euclid(1.) * points.len() as f32;
                let i = t.floor() as usize;
                let f = t.fract();
                let n = points.len();
                let [a, b, c, d] = [
                    points[(i + n - 1) % n],
                    points[i],
                    points[(i + 1) % n],
                    points[(i + 2) % n],
                ];
                let p = 0.5
                    * ((2. * b)
                        + (-a + c) * f
                        + (2. * a - 5. * b + 4. * c - d) * f * f
                        + (-a + 3. * b - 3. * c + d) * f * f * f);
                let q = if *spin {
                    Quat::from_rotation_z((15. * time).to_radians())
                        * Quat::from_rotation_y((-8. * time).to_radians())
                        * Quat::from_rotation_x((22. * time).to_radians())
                } else {
                    Quat::IDENTITY
                };
                Pose {
                    origin: p,
                    rotation: q * self.base.rotation,
                }
            }
            Motion::Slam { angle, down, up } => {
                let t = time % (down + up);
                let a = if t < *down {
                    t / down
                } else {
                    1. - (t - down) / up
                } * angle;
                Pose {
                    rotation: Quat::from_rotation_z(a.to_radians()),
                    ..self.base
                }
            }
        }
    }
}
fn path(map: &Bsp, start: &str) -> Result<Vec<Vec3>> {
    let mut name = start.to_owned();
    let mut seen = std::collections::BTreeSet::new();
    let mut points = Vec::new();
    while seen.insert(name.clone()) && points.len() < 128 {
        points.push(origin(map, &name)?);
        let Some(next) = entity(map, &name)?.get("target") else {
            break;
        };
        name = next.clone();
    }
    anyhow::ensure!(points.len() >= 3, "Incomplete school-two path {start}");
    Ok(points)
}
#[derive(Clone)]
pub struct School2 {
    cinema_data: std::sync::Arc<cinema::Data>,
    cinema: Option<cinema::State>,
    rage_hint: Option<rage_hint::State>,
    rage_pose: Transform,
    objects: Vec<Object>,
    pub quest: Quest,
    pub boojums: Vec<Boojum>,
    pub guards: Vec<Guard>,
    guard_angry: [bool; 2],
    reinforcements: Vec<Guard>,
    dice_guards: bool,
    boojum_timing: BoojumTiming,
    guard_timing: Timing,
    gnome: Vec3,
    lab: Vec3,
    gnome_yaw: f32,
    placed: bool,
    items: BTreeMap<String, Vec3>,
    pub age: f32,
}
impl School2 {
    pub fn notarget(&mut self, value: bool) {
        for b in &mut self.boojums {
            b.notarget = value;
        }
        for (i, g) in self.guards.iter_mut().enumerate() {
            // The lab guards can still see their captive before Alice draws their fire.
            g.notarget = value && self.guard_angry[i];
        }
        for g in &mut self.reinforcements {
            g.notarget = value;
        }
    }
    pub fn load(assets: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut objects = Vec::new();
        for &name in INLINE.iter().chain(PROPS) {
            let e = entity(map, name)?;
            let base = Pose {
                origin: origin(map, name)?,
                rotation: Quat::from_rotation_z(
                    e.get("angle")
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(0.)
                        .to_radians(),
                ),
            };
            let scale = e
                .get("scale")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(1.);
            let (shape, prop) = if let Some(m) = e["model"].strip_prefix('*') {
                (Shape::Brush(m.parse()?), None)
            } else {
                let model_name = e["model"]
                    .trim_start_matches("models/")
                    .trim_end_matches(".tik");
                let (def, model) = read_model(assets, model_name)?;
                let triangles = model
                    .surfaces
                    .iter()
                    .flat_map(|s| {
                        s.indices.chunks_exact(3).map(|t| {
                            [
                                s.frames[0][t[0] as usize],
                                s.frames[0][t[1] as usize],
                                s.frames[0][t[2] as usize],
                            ]
                            .map(|p| p * scale * def.scale)
                        })
                    })
                    .collect();
                (Shape::Mesh(triangles), Some(model_name.to_owned()))
            };
            let motion = if name.starts_with("gym_pend") || name.starts_with("gym_light") {
                let i = name.chars().last().unwrap().to_digit(10).unwrap() as usize;
                Motion::Pendulum {
                    pivot: origin(map, &format!("gym_pend{i}"))?,
                    roll: i == 2,
                    period: [5., 3., 4.][i - 1],
                }
            } else {
                match name {
                    "floating_books1" => Motion::Path {
                        points: path(map, "fb_path1")?,
                        period: 7.,
                        spin: true,
                    },
                    "floating_cabinet1" => Motion::Path {
                        points: path(map, "fc_path1")?,
                        period: 4. / 0.3,
                        spin: false,
                    },
                    "slamming_door1" => Motion::Slam {
                        angle: -32.,
                        down: 0.4,
                        up: 0.3,
                    },
                    "slamming_door2" => Motion::Slam {
                        angle: 26.,
                        down: 0.2,
                        up: 0.4,
                    },
                    _ => Motion::Still,
                }
            };
            objects.push(Object {
                name: name.into(),
                prop,
                scale,
                collider: shape.collider(map, base)?,
                shape,
                base,
                pose: base,
                motion,
                time: 0.,
                enabled: !["boojum_clip", "exit_portal"].contains(&name),
            });
        }
        for o in &mut objects {
            o.pose = o.sample(0.);
            o.collider = o.shape.collider(map, o.pose)?;
        }
        let mut boojums = Vec::new();
        for (i, n) in [
            "spawn_newbooj1",
            "spawn_newbooj2",
            "spawn_newbooj3",
            "flying_boojum1",
        ]
        .iter()
        .enumerate()
        {
            boojums.push(Boojum::new(origin(map, n)?, i as f32 * 0.6));
        }
        let mut garden = Boojum::new(vec3(-2848., -2560., 632.), 0.);
        garden.active = true;
        boojums.push(garden);
        boojums.push(Boojum::new(vec3(-824., -2512., -144.), 0.));
        for n in ["b_spawn1", "b_spawn2", "b_spawn3", "b_spawn4"] {
            boojums.push(Boojum::new(origin(map, n)?, 0.6));
        }
        let reinforcements = ["club_spawn1", "club_spawn2", "club_spawn3", "club_spawn4"]
            .iter()
            .map(|n| Ok(Guard::new(origin(map, n)?, 0., 1.)))
            .collect::<Result<Vec<_>>>()?;
        let guards = ["gnome_killer1", "gnome_killer2"]
            .iter()
            .map(|n| Ok(Guard::new(origin(map, n)?, std::f32::consts::PI, 1.)))
            .collect::<Result<Vec<_>>>()?;
        let mut items = BTreeMap::new();
        for n in [
            "jumbo_shelf1",
            "ig_lollypop",
            "shrink_potion",
            "lolly_jg_beaker",
            "gnome_condenser",
        ] {
            items.insert(n.into(), origin(map, n)?);
        }
        items.insert(
            "lucky_star".into(),
            origin(map, "gnome_star_pos1")? + Vec3::Z * 38.,
        );
        Ok(Self {
            cinema_data: std::sync::Arc::new(cinema::Data::load(assets, map)?),
            cinema: None,
            rage_hint: None,
            rage_pose: Transform {
                translation: origin(map, "rage_cat")?,
                rotation: Quat::from_rotation_z(entity(map, "rage_cat")?.get("angle")
                    .and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.).to_radians()),
            },
            objects,
            quest: Quest::default(),
            boojums,
            guards,
            guard_angry: [false; 2],
            reinforcements,
            dice_guards: false,
            boojum_timing: BoojumTiming::load(assets)?,
            guard_timing: crate::npc::guard_timing(assets)?,
            gnome: origin(map, "old_gnome_1")?,
            lab: origin(map, "gnome_pos1")?,
            gnome_yaw: 0.,
            placed: false,
            items,
            age: 0.,
        })
    }
    pub fn event_facts(&self) -> crate::event::Facts {
        let mut f = crate::event::Facts::default();
        for (key, stage) in [
            ("quest.explore", Stage::Explore),
            ("quest.mushroom_dialogue", Stage::MushroomDialogue),
            ("quest.laboratory", Stage::Laboratory),
            ("quest.jumbogrow", Stage::Jumbogrow),
            ("quest.spice_dialogue", Stage::SpiceDialogue),
            ("quest.final_dialogue", Stage::FinalDialogue),
            ("quest.lollipop", Stage::Lollipop),
            ("quest.complete", Stage::Complete),
        ] {
            f.flag(key, self.quest.stage == stage);
        }
        f.flag("item.jumbogrow", self.quest.items.jumbogrow);
        f.flag("item.lollipop", self.quest.items.lollipop);
        f
    }
    pub fn trigger_condition(name: &str, thread: &str, exit: bool) -> crate::event::Condition {
        use crate::event::Condition as C;
        if exit {
            return if name == "exit_trigger" {
                C::flag("quest.complete")
            } else {
                C::Any(vec![])
            };
        }
        match thread {
            "Old_Gnome_Mushroom" => C::flag("quest.explore"),
            "Kill_The_Gnome" => C::flag("quest.laboratory"),
            "Skool2_GrowLollypop" => {
                C::All(vec![C::flag("quest.jumbogrow"), C::flag("item.jumbogrow")])
            }
            "Skool2_LastGnome_Cinema" => {
                C::All(vec![C::flag("quest.lollipop"), C::flag("item.lollipop")])
            }
            _ => C::Always,
        }
    }
    /// Only the two idle conversation opportunities are selectable. Rescue and
    /// Spice Drops remain combat-owned; no talk action advances their gates.
    pub fn conversation_target(&self, thread: &str) -> Option<Vec3> {
        match (thread, self.quest.stage) {
            (cinema::MUSHROOM, Stage::Explore) => Some(self.gnome + Vec3::Z * 32.),
            (cinema::FINAL, Stage::Lollipop)
                if self.quest.items.mushroom
                    && self.quest.items.spice
                    && self.quest.items.lollipop =>
            {
                Some(self.lab + Vec3::Z * 32.)
            }
            _ => None,
        }
    }
    pub fn trigger_enabled(&self, name: &str, thread: &str, exit: bool) -> bool {
        Self::trigger_condition(name, thread, exit).test(&self.event_facts())
    }
    pub fn event(&mut self, event: &str) -> Option<Events> {
        if event == rage_hint::EVENT { return Some(self.start_rage_hint()); }
        let mut result = Events::default();
        if self.quest.begin(event) {
            match event {
                cinema::MUSHROOM => self.start_scene(cinema::Beat::Mushroom),
                cinema::GROW => self.start_scene(cinema::Beat::Growth),
                cinema::FINAL => self.start_scene(cinema::Beat::Final),
                _ => (),
            }
            println!("School quest: {:?}", self.quest.stage);
            if crate::story::supports("skool2", event) {
                result.story.push(event.into());
            }
            result.message = Some(self.quest.objective());
        } else {
            match event {
                "flying_boojum_attack" => self.boojums[3].active = true,
                "dice_cat" => {
                    self.boojums[5].active = true;
                    self.dice_guards = true;
                }
                "Skool2_SetupCinema2" => {}
                "Old_Gnome_Mushroom"
                | "Kill_The_Gnome"
                | "Skool2_GrowLollypop"
                | "Skool2_LastGnome_Cinema" => {}
                _ => return None,
            }
        }
        Some(result)
    }
    pub fn completed_dialogue(&mut self, event: &str) -> bool {
        if self.scene_dialogue(event) {
            return false;
        }
        let changed = self.quest.dialogue_finished(event);
        if changed {
            println!("School quest: {:?}", self.quest.stage);
            if self.quest.stage == Stage::Battle {
                for b in &mut self.boojums[..3] {
                    b.active = true;
                }
            }
            if self.quest.stage == Stage::Jumbogrow {
                if self.cinema.as_ref().is_some_and(|s| s.cabinet.is_some()) {
                    return changed;
                }
                for o in &mut self.objects {
                    match o.name.as_str() {
                        "jumbo_door1" => {
                            o.motion = Motion::Open {
                                angle: -75.,
                                duration: 3.,
                            };
                            o.time = 0.;
                        }
                        "jumbo_door2" => {
                            o.motion = Motion::Open {
                                angle: 70.,
                                duration: 3.2,
                            };
                            o.time = 0.;
                        }
                        _ => {}
                    }
                }
            }
        }
        changed
    }
    pub fn loot_sources(&self) -> Vec<crate::loot::Source> {
        use crate::loot::{Grade, Source};
        let mut out: Vec<_> = self
            .boojums
            .iter()
            .enumerate()
            .map(|(i, b)| Source {
                id: ENEMY_BASE + i,
                feet: b.feet,
                grade: Grade::Large,
                dead: b.health <= 0.,
            })
            .collect();
        for (offset, guards) in [(100, &self.guards), (200, &self.reinforcements)] {
            out.extend(guards.iter().enumerate().map(|(i, g)| Source {
                id: ENEMY_BASE + offset + i,
                feet: g.feet,
                grade: if g.ranged {
                    Grade::Medium
                } else {
                    Grade::Small
                },
                dead: g.health <= 0.,
            }));
        }
        out
    }
    pub fn targets(&self) -> Vec<Target> {
        if self.cinematic() {
            return Vec::new();
        }
        let mut result = self
            .boojums
            .iter()
            .enumerate()
            .filter(|(_, b)| b.active && b.health > 0.)
            .map(|(i, b)| b.target(ENEMY_BASE + i))
            .collect::<Vec<_>>();
        if self.quest.stage == Stage::Rescue {
            result.extend(
                self.guards
                    .iter()
                    .enumerate()
                    .filter(|(_, g)| g.health > 0.)
                    .map(|(i, g)| g.target(ENEMY_BASE + 100 + i)),
            );
        }
        if self.dice_guards {
            result.extend(
                self.reinforcements
                    .iter()
                    .enumerate()
                    .filter(|(_, g)| g.health > 0.)
                    .map(|(i, g)| g.target(ENEMY_BASE + 200 + i)),
            );
        }
        result
    }
    pub fn hit(&mut self, id: usize, damage: f32) -> Option<&'static str> {
        self.hit_kind(id, damage, crate::combat::DamageKind::Other)
    }
    pub fn hit_kind(
        &mut self,
        id: usize,
        damage: f32,
        kind: crate::combat::DamageKind,
    ) -> Option<&'static str> {
        self.hit_attack(crate::combat::Hit {
            id,
            damage,
            kind,
            knockback: Vec3::ZERO,
        })
    }
    pub fn hit_attack(&mut self, hit: crate::combat::Hit) -> Option<&'static str> {
        if self.cinematic() {
            return None;
        }
        let id = hit.id;
        let index = id.checked_sub(ENEMY_BASE)?;
        if index < self.boojums.len() {
            return self.boojums[index].hit_attack(hit);
        }
        if self.quest.stage == Stage::Rescue && (100..102).contains(&index) {
            self.guard_angry[index - 100] = true;
            return self.guards[index - 100].hit(hit);
        }
        if self.dice_guards && (200..204).contains(&index) {
            return self.reinforcements[index - 200].hit(hit);
        }
        None
    }
    pub fn summon(&mut self, target: Option<Target>) {
        for b in &mut self.boojums {
            b.opponents.summon = target;
        }
        for g in self.guards.iter_mut().chain(self.reinforcements.iter_mut()) {
            g.opponents.summon = target;
        }
    }
    pub fn provoke_summon(&mut self, id: usize) {
        let Some(i) = id.checked_sub(ENEMY_BASE) else {
            return;
        };
        if i < self.boojums.len() {
            self.boojums[i].opponents.demon = true;
        } else if (100..100 + self.guards.len()).contains(&i) {
            self.guards[i - 100].opponents.demon = true;
        } else if (200..200 + self.reinforcements.len()).contains(&i) {
            self.reinforcements[i - 200].opponents.demon = true;
        }
    }
    pub fn update(
        &mut self,
        dt: f32,
        world: &World,
        feet: Vec3,
        paused: bool,
    ) -> (Events, Feedback) {
        let mut events = Events::default();
        let mut feedback = Feedback::default();
        if !self.placed {
            let center = Vec3::Z * 32.;
            let half = vec3(24., 24., 32.);
            self.gnome = world
                .actor_footing(self.gnome, center, half, 1024.)
                .unwrap_or(self.gnome);
            self.lab = world
                .actor_footing(self.lab, center, half, 1024.)
                .unwrap_or(self.lab);
            self.placed = true;
        }
        // Dialogue pauses quest/combat updates, but its actor still addresses
        // Alice. The render camera can orbit independently in third person.
        if dt <= 0. || !dt.is_finite() {
            return (events, feedback);
        }
        self.advance_rage_hint(dt, feet, &mut feedback);
        if let Some(s) = &mut self.cinema {
            if !s.sounds.is_empty() {
                events.sound = Some(cinema::SOUNDS[s.sounds.remove(0)].into());
            }
        }
        let position = if matches!(
            self.quest.stage,
            Stage::Explore | Stage::MushroomDialogue | Stage::Battle
        ) {
            self.gnome
        } else {
            self.lab
        };
        let target = if self.quest.stage == Stage::Rescue {
            self.guards
                .iter()
                .find(|g| g.health > 0.)
                .map_or(feet, |g| g.feet)
        } else if self.quest.stage == Stage::Mixing {
            self.items["gnome_condenser"]
        } else {
            feet
        };
        let delta = target - position;
        if delta.truncate().length_squared() > 0.01 {
            let desired = delta.y.atan2(delta.x);
            let turn = (desired - self.gnome_yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.gnome_yaw += turn.clamp(-dt * 2.8, dt * 2.8);
        }
        self.age += dt;
        // Death presentation continues while dialogue suspends live combat.
        // Otherwise the last rescued guard freezes upright across the exchange.
        for g in &mut self.guards {
            if g.health <= 0. {
                g.advance(dt, world, feet + Vec3::Z * 48., self.guard_timing);
            }
        }
        if paused || self.cinematic() {
            return (events, feedback);
        }
        for b in &mut self.boojums {
            let f = b.advance(
                dt,
                world,
                feet + Vec3::Z * crate::movement::EYE_HEIGHT,
                self.boojum_timing,
            );
            feedback.summon_hits.extend(f.summon_hits);
            feedback.damage += f.damage;
            feedback.impulse += f.impulse;
            feedback
                .spatial_sounds
                .extend(f.sounds.into_iter().map(|s| (s, b.target(0).center)));
        }
        if self.quest.stage == Stage::Rescue {
            for (i, g) in self.guards.iter_mut().enumerate() {
                if g.health <= 0. {
                    continue;
                }
                let target = if self.guard_angry[i] {
                    feet + Vec3::Z * 48.
                } else {
                    self.lab + Vec3::Z * 48.
                };
                let f = g.advance(dt, world, target, self.guard_timing);
                feedback.summon_hits.extend(f.summon_hits);
                if self.guard_angry[i] {
                    feedback.damage += f.damage;
                    feedback.impulse += f.impulse;
                }
                feedback
                    .spatial_sounds
                    .extend(f.sounds.into_iter().map(|s| (s, g.target(0).center)));
            }
        }
        if self.dice_guards {
            for g in &mut self.reinforcements {
                let f = g.advance(dt, world, feet + Vec3::Z * 48., self.guard_timing);
                feedback.summon_hits.extend(f.summon_hits);
                feedback.damage += f.damage;
                feedback.impulse += f.impulse;
                feedback
                    .spatial_sounds
                    .extend(f.sounds.into_iter().map(|s| (s, g.target(0).center)));
            }
        }
        let before = self.quest.stage;
        if let Some(event) = self.quest.tick(
            dt,
            self.boojums[..3].iter().filter(|b| b.health <= 0.).count(),
            self.guards.iter().filter(|g| g.health <= 0.).count(),
        ) {
            if event == cinema::SPICE {
                self.start_scene(cinema::Beat::Spice);
            }
            events.story.push(event.into());
        }
        if before == Stage::Battle && self.quest.stage == Stage::Laboratory {
            self.start_scene(cinema::Beat::Warp);
        }
        if before == Stage::Growing && self.quest.stage == Stage::Lollipop {
            for b in &mut self.boojums[8..10] {
                b.active = true;
            }
        }
        // Each pickup requires proximity and a clear line through the actual world.
        for name in ["jumbo_shelf1", "ig_lollypop", "shrink_potion", "lucky_star"] {
            let point = self.items[name] + Vec3::Z * 16.;
            let eye = feet + Vec3::Z * 48.;
            let delta = point - eye;
            if delta.truncate().length() > 46. || delta.z.abs() > 58. {
                continue;
            }
            let t = world.sweep(eye, point, Vec3::splat(0.5));
            if !t.start_solid && t.fraction >= 1. && self.quest.collect(name) {
                println!("School pickup: {name}");
                events.message = Some(format!(
                    "Collected {}",
                    match name {
                        "jumbo_shelf1" => "Jumbogrow",
                        "ig_lollypop" => "Lollipop",
                        "shrink_potion" => "Drink Me potion",
                        _ => "Lucky Star",
                    }
                ));
                events.sound = Some("sound/item/pickup.wav".into());
            }
        }
        if self.quest.stage == Stage::Complete && before != Stage::Complete {
            events.sound = Some("sound/ambience/special/door_flip.wav".into());
        }
        if before != self.quest.stage {
            println!("School quest: {:?}", self.quest.stage);
            events.message = Some(self.quest.objective());
        }
        // The battle floor is introduced only when it cannot intersect Alice.
        for o in &mut self.objects {
            if o.name == "boojum_clip"
                && self.quest.stage != Stage::Explore
                && self.quest.stage != Stage::MushroomDialogue
                && !o
                    .collider
                    .touches(feet + PLAYER_CENTER, feet + PLAYER_CENTER, PLAYER_HALF)
            {
                o.enabled = true;
            }
            if o.name == "exit_portal" {
                o.enabled = self.quest.stage == Stage::Complete;
            }
        }
        (events, feedback)
    }
    fn props(&self) -> Vec<(&'static str, Transform, f32)> {
        if self.cinema.as_ref().is_some_and(|s| {
            s.active && matches!(s.beat, cinema::Beat::Growth | cinema::Beat::Final)
        }) {
            return Vec::new();
        }
        let s = self.quest.stage;
        let mut p = Vec::new();
        let mut add = |model, name, scale| {
            p.push((
                model,
                Transform {
                    translation: self.items[name],
                    rotation: Quat::IDENTITY,
                },
                scale,
            ))
        };
        if (s == Stage::Jumbogrow
            || self
                .cinema
                .as_ref()
                .is_some_and(|s| s.active && s.cabinet.is_some()))
            && !self.quest.items.jumbogrow
        {
            add("beaker01", "jumbo_shelf1", 1.);
        }
        if !self.quest.items.lollipop
            && !matches!(
                s,
                Stage::FinalDialogue | Stage::Mixing | Stage::Rewards | Stage::Complete
            )
        {
            add(
                "lollypop",
                "ig_lollypop",
                if s == Stage::Growing {
                    0.07 + 3.93 * (self.quest.time / 5.5).min(1.)
                } else if s == Stage::Lollipop {
                    4.
                } else {
                    0.1
                },
            );
        }
        if s == Stage::Growing {
            let t = (self.quest.time / 4.).min(1.);
            p.push((
                "beaker01",
                Transform {
                    translation: self.items["lolly_jg_beaker"] + vec3(0., 8. * t, 64. * t),
                    rotation: Quat::from_rotation_x(135_f32.to_radians() * t),
                },
                1. - (self.quest.time - 4.).max(0.) / 1.5,
            ));
        }
        if s == Stage::Mixing
            || (matches!(s, Stage::Rewards | Stage::Complete) && !self.quest.items.potion)
        {
            p.push((
                "beaker02",
                Transform {
                    translation: self.items["shrink_potion"],
                    rotation: Quat::IDENTITY,
                },
                (if self
                    .cinema
                    .as_ref()
                    .is_some_and(|s| s.beat == cinema::Beat::Final)
                {
                    1.
                } else {
                    0.7
                }) * if s == Stage::Mixing {
                    (self.quest.time / 5.).min(1.)
                } else {
                    1.
                },
            ));
        }
        if matches!(s, Stage::Rewards | Stage::Complete) && !self.quest.items.star {
            p.push((
                "star",
                Transform {
                    translation: self.items["lucky_star"],
                    rotation: Quat::from_rotation_z(self.age),
                },
                1.,
            ));
        }
        p
    }
    pub fn colliders(&self) -> impl Iterator<Item = Collider> + '_ {
        self.objects
            .iter()
            .filter(|o| o.enabled && o.name != "exit_portal")
            .map(|o| o.collider.clone())
    }
    pub fn transforms(&self) -> impl Iterator<Item = (usize, Vec3, Quat)> + '_ {
        self.objects
            .iter()
            .filter(|o| o.enabled)
            .filter_map(|o| match o.shape {
                Shape::Brush(i) => Some((i, o.pose.origin, o.pose.rotation)),
                _ => None,
            })
    }
    pub fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        let mut remaining = dt.clamp(0., 0.1);
        while remaining > 0.000001 {
            let step = remaining.min(1. / 120.);
            remaining -= step;
            let mut done = std::collections::BTreeSet::new();
            for i in 0..self.objects.len() {
                if !done.insert(i) {
                    continue;
                }
                let o = &self.objects[i];
                if !o.enabled || matches!(o.motion, Motion::Still) {
                    continue;
                }
                // A pendulum and its bound lamp are one rigid body, including when blocked.
                let mut group = vec![i];
                if let Some(number) = o.name.strip_prefix("gym_pend") {
                    if let Some(j) = self
                        .objects
                        .iter()
                        .position(|o| o.name == format!("gym_light{number}"))
                    {
                        group.push(j);
                        done.insert(j);
                    }
                }
                let time = o.time + step;
                let proposed = group
                    .iter()
                    .map(|&j| {
                        let o = &self.objects[j];
                        let pose = o.sample(time);
                        Ok((j, pose, o.shape.collider(map, pose)?))
                    })
                    .collect::<Result<Vec<_>>>()?;
                world.set_dynamic(
                    fixed
                        .iter()
                        .cloned()
                        .chain(
                            self.objects
                                .iter()
                                .enumerate()
                                .filter(|(j, o)| {
                                    !group.contains(j) && o.enabled && o.name != "exit_portal"
                                })
                                .map(|(_, o)| o.collider.clone()),
                        )
                        .collect(),
                );
                let mut support = None;
                let mut blocked = false;
                for (j, pose, collider) in &proposed {
                    let o = &self.objects[*j];
                    let ground = o.collider.trace(
                        player.feet + PLAYER_CENTER,
                        player.feet + PLAYER_CENTER - Vec3::Z * 3.,
                        PLAYER_HALF,
                    );
                    let riding = player.velocity.z <= 1.
                        && !ground.start_solid
                        && ground.fraction < 1.
                        && ground.normal.z >= 0.65;
                    if riding {
                        let carried = pose.origin
                            + pose.rotation
                                * o.pose.rotation.inverse()
                                * (player.feet - o.pose.origin);
                        if let Some((feet, normal)) = collider.rider_feet(carried) {
                            support = Some((feet, normal));
                        } else {
                            blocked = true;
                        }
                    }
                }
                let feet = support.map_or(player.feet, |s| s.0);
                if let Some((feet, _)) = support {
                    let sweep = world.body_trace(player.feet, feet);
                    blocked |= sweep.start_solid || sweep.fraction < 1. || !world.body_clear(feet);
                }
                blocked |= proposed.iter().any(|(_, _, c)| {
                    c.touches(feet + PLAYER_CENTER, feet + PLAYER_CENTER, PLAYER_HALF)
                });
                if blocked {
                    continue;
                }
                for (j, pose, collider) in proposed {
                    let o = &mut self.objects[j];
                    o.pose = pose;
                    o.time = time;
                    o.collider = collider;
                }
                if let Some((feet, normal)) = support {
                    player.feet = feet;
                    player.grounded = true;
                    player.ground_normal = normal;
                }
            }
        }
        world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        self.advance_scene(dt, world, player)?;
        Ok(())
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rage_hint: Option<rage_hint::State>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cinema: Option<cinema::State>,
    objects: Vec<ObjectSave>,
    quest: Quest,
    boojums: Vec<Boojum>,
    guards: Vec<Guard>,
    guard_angry: [bool; 2],
    reinforcements: Vec<Guard>,
    dice_guards: bool,
    age: f32,
    #[serde(default)]
    gnome_yaw: f32,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct ObjectSave {
    name: String,
    pose: Pose,
    motion: Motion,
    time: f32,
    enabled: bool,
}
impl School2 {
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            rage_hint: self.rage_hint.clone(),
            cinema: self.cinema.clone(),
            objects: self
                .objects
                .iter()
                .map(|o| ObjectSave {
                    name: o.name.clone(),
                    pose: o.pose,
                    motion: o.motion.clone(),
                    time: o.time,
                    enabled: o.enabled,
                })
                .collect(),
            quest: self.quest.clone(),
            boojums: self.boojums.clone(),
            guards: self.guards.clone(),
            guard_angry: self.guard_angry,
            reinforcements: self.reinforcements.clone(),
            dice_guards: self.dice_guards,
            age: self.age,
            gnome_yaw: self.gnome_yaw,
        }
    }
    pub fn restore(&mut self, s: &Snapshot, map: &Bsp) -> Result<()> {
        if let Some(hint) = &s.rage_hint { hint.validate()?; }
        if let Some(c) = &s.cinema {
            c.validate(s.quest.stage)?;
        }
        anyhow::ensure!(
            self.objects.len() == s.objects.len()
                && self.boojums.len() == s.boojums.len()
                && self.guards.len() == s.guards.len()
                && self.reinforcements.len() == s.reinforcements.len(),
            "Saved second school does not match map"
        );
        anyhow::ensure!(
            s.quest.time >= 0. && s.age >= 0. && s.gnome_yaw.is_finite(),
            "Invalid saved quest timer"
        );
        for b in &s.boojums {
            b.validate_save()?;
        }
        for g in s.guards.iter().chain(&s.reinforcements) {
            g.validate_save()?;
        }
        for (o, v) in self.objects.iter_mut().zip(&s.objects) {
            anyhow::ensure!(
                o.name == v.name && v.time >= 0. && v.pose.rotation.is_normalized(),
                "Invalid saved school mover"
            );
            match &v.motion {
                Motion::Path { points, period, .. } => anyhow::ensure!(
                    !points.is_empty() && points.len() <= 128 && *period > 0.,
                    "Invalid saved mover path"
                ),
                Motion::Open { duration, .. } => {
                    anyhow::ensure!(*duration > 0., "Invalid saved door duration")
                }
                Motion::Pendulum { period, .. } => {
                    anyhow::ensure!(*period > 0., "Invalid saved pendulum period")
                }
                Motion::Slam { down, up, .. } => {
                    anyhow::ensure!(*down > 0. && *up > 0., "Invalid saved slamming door")
                }
                Motion::Still => (),
            }
            o.pose = v.pose;
            o.motion = v.motion.clone();
            o.time = v.time;
            o.enabled = v.enabled;
            o.collider = o.shape.collider(map, o.pose)?;
        }
        self.quest = s.quest.clone();
        self.boojums = s.boojums.clone();
        self.guards = s.guards.clone();
        self.guard_angry = s.guard_angry;
        self.reinforcements = s.reinforcements.clone();
        self.dice_guards = s.dice_guards;
        self.age = s.age;
        self.gnome_yaw = s.gnome_yaw;
        self.cinema = s.cinema.clone();
        self.rage_hint = s.rage_hint.clone();
        Ok(())
    }
}
pub struct Art {
    cinema: cinema::Art,
    models: BTreeMap<String, Prop>,
    material: crate::character::SkinMaterial,
    boojum: crate::npc::Puppet,
    gnome: crate::npc::Puppet,
    guard: crate::npc::Puppet,
    condenser: Prop,
}
impl Art {
    pub fn load(assets: &mut Assets, s: &School2) -> Result<Self> {
        let specs = crate::texture::read_materials(assets)?;
        let mut models = BTreeMap::new();
        for name in s.objects.iter().filter_map(|o| o.prop.as_ref()) {
            if !models.contains_key(name) {
                models.insert(name.clone(), Prop::load(assets, name, &specs)?);
            }
        }
        for name in ["beaker01", "beaker02", "lollypop", "star"] {
            models.insert(name.into(), Prop::load(assets, name, &specs)?);
        }
        Ok(Self {
            cinema: cinema::Art::load(assets, &specs)?,
            models,
            material: crate::character::skin_material()?,
            boojum: crate::npc::Puppet::load(
                assets,
                "c_boojum",
                &[
                    "fly",
                    "attack_scream",
                    "pain1",
                    "death_part01",
                    "death_part02",
                ],
                &specs,
            )?,
            gnome: crate::npc::Puppet::load(
                assets,
                "c_gnomeold",
                &["talk", "mixing", "vanish01", "punch", "handout"],
                &specs,
            )?,
            guard: crate::npc::Puppet::load(assets, "cardguard_club", &[], &specs)?,
            condenser: Prop::load_animation(assets, "condenser", "change", &specs)?,
        })
    }
    pub fn story_pose(&mut self, story: &crate::story::Story) {
        self.cinema.story_pose(story);
        self.gnome
            .mouth(story.mouth(&["old_gnome_1", "old_gnome_2"]));
    }
    pub fn draw(
        &mut self,
        s: &School2,
        fullbright: bool,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
    ) {
        self.material.atmosphere(atmosphere, camera);
        self.material.bind();
        for o in &s.objects {
            if o.enabled {
                if let Some(name) = &o.prop {
                    self.models.get_mut(name).unwrap().draw(
                        Transform {
                            translation: o.pose.origin,
                            rotation: o.pose.rotation,
                        },
                        o.scale,
                        fullbright,
                    );
                }
            }
        }
        for (name, pose, scale) in s.props() {
            self.models
                .get_mut(name)
                .unwrap()
                .draw(pose, scale, fullbright);
        }
        self.condenser.draw_frame(
            Transform {
                translation: s.items["gnome_condenser"],
                rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            },
            1.,
            fullbright,
            s.condenser_time(),
            false,
        );
        for (i, b) in s.boojums.iter().enumerate() {
            if !b.active && i != 3 {
                continue;
            }
            let (clip, time, looping) = b.clip(s.boojum_timing);
            let scale = if b.frozen {
                (2.5 - b.time).clamp(0., 1.)
            } else if b.health <= 0. {
                1. - ((b.time - s.boojum_timing.death) / 1.4).clamp(0., 1.)
            } else {
                1.
            };
            self.boojum.draw(
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
            self.boojum.draw_electric(b.electric);
        }
        let (position, clip, scale) = match s.quest.stage {
            Stage::Explore => (s.gnome, "idle", 1.),
            Stage::MushroomDialogue => (s.gnome, "talk", 1.),
            Stage::Battle => (s.gnome, "vanish01", 1. - (s.quest.time / 1.5).min(1.)),
            Stage::Laboratory => (s.lab, "idle", (s.quest.time / 1.5).min(1.)),
            Stage::Rescue => (s.lab, "punch", 1.),
            Stage::SpiceDialogue | Stage::FinalDialogue => (s.lab, "talk", 1.),
            Stage::Mixing => (s.lab, "mixing", 1.),
            Stage::Jumbogrow | Stage::Rewards => {
                (s.lab, "vanish01", 1. - (s.quest.time / 1.5).min(1.))
            }
            Stage::Lollipop => (s.lab, "idle", 1.),
            _ => (s.lab, "idle", 0.),
        };
        if !self.cinema.draw(s, fullbright, atmosphere, camera) {
            self.gnome.draw(
                clip,
                if clip == "talk" { s.age } else { s.quest.time },
                true,
                Transform {
                    translation: position,
                    rotation: Quat::from_rotation_z(s.gnome_yaw),
                },
                scale,
                fullbright,
            );
            self.gnome.draw_effects(
                clip,
                if clip == "talk" { s.age } else { s.quest.time },
                true,
                Transform {
                    translation: position,
                    rotation: Quat::from_rotation_z(s.gnome_yaw),
                },
                scale,
                camera,
                atmosphere,
            );
        }
        if !matches!(
            s.quest.stage,
            Stage::Explore | Stage::MushroomDialogue | Stage::Battle | Stage::Laboratory
        ) {
            for g in &s.guards {
                self.guard.draw_guard(g, fullbright, camera, atmosphere);
            }
        }
        if s.dice_guards {
            for g in &s.reinforcements {
                self.guard.draw_guard(g, fullbright, camera, atmosphere);
            }
        }
        gl_use_default_material();
    }
}
/// Real-asset component checks complement the continuous entrance route.
pub fn check(assets: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&assets.read("maps/skool2.bsp")?)?;
    crate::interaction::school2_check::check(assets)?;
    cinema_check::check(assets, &map)?;
    potion_check::check(assets, &map)?;
    let initial = School2::load(assets, &map)?;
    let mut actor = initial.clone();
    let mut actor_world = World::from_bsp(&map)?;
    actor_world.set_dynamic(actor.colliders().collect());
    let alice_feet = actor.gnome + vec3(-120., 75., 0.);
    actor.quest.enter(Stage::MushroomDialogue);
    for _ in 0..180 {
        actor.update(1. / 60., &actor_world, alice_feet, true);
    }
    let forward = |yaw: f32| vec2(yaw.cos(), yaw.sin());
    anyhow::ensure!(
        forward(actor.gnome_yaw).dot((alice_feet - actor.gnome).truncate().normalize()) > 0.999,
        "Gnome does not face Alice during dialogue"
    );
    let saved = actor.snapshot();
    actor.restore(&saved, &map)?;
    actor.update(0., &actor_world, alice_feet + Vec3::X * 1000., true);
    anyhow::ensure!(
        actor.gnome_yaw == saved.gnome_yaw,
        "Pause/save changes Gnome facing"
    );
    actor.quest.enter(Stage::Mixing);
    for _ in 0..180 {
        actor.update(1. / 60., &actor_world, alice_feet, true);
    }
    anyhow::ensure!(
        forward(actor.gnome_yaw).dot(
            (actor.items["gnome_condenser"] - actor.lab)
                .truncate()
                .normalize()
        ) > 0.999,
        "Gnome does not face the apparatus while mixing"
    );
    println!("PASS Gnome support, dialogue facing, mixing direction, paused/save-restored facing");
    let mut poses = Vec::new();
    for fps in [30, 60, 144] {
        let mut s = initial.clone();
        let mut world = World::from_bsp(&map)?;
        let mut p = Player::new(vec3(-64., -3440., -32.));
        let before = s.objects.iter().map(|o| o.pose.origin).collect::<Vec<_>>();
        s.advance(0., &map, &mut world, &mut p, &[])?;
        anyhow::ensure!(
            s.objects
                .iter()
                .zip(before)
                .all(|(a, b)| a.pose.origin == b),
            "Paused movers advanced"
        );
        for _ in 0..fps * 3 {
            s.advance(1. / fps as f32, &map, &mut world, &mut p, &[])?;
        }
        for n in 1..=3 {
            let rod = s
                .objects
                .iter()
                .find(|o| o.name == format!("gym_pend{n}"))
                .unwrap();
            let lamp = s
                .objects
                .iter()
                .find(|o| o.name == format!("gym_light{n}"))
                .unwrap();
            anyhow::ensure!(
                (rod.time - lamp.time).abs() < 0.00001,
                "Bound pendulum separated"
            );
        }
        poses.push(s.objects.iter().map(|o| o.pose).collect::<Vec<_>>());
    }
    for p in &poses[1..] {
        for (a, b) in p.iter().zip(&poses[0]) {
            anyhow::ensure!(
                a.origin.distance(b.origin) < 0.05 && a.rotation.dot(b.rotation).abs() > 0.99999,
                "Display-rate-dependent mover"
            );
        }
    }
    for name in ["floating_cabinet1", "floating_books1"] {
        let mut s = initial.clone();
        let o = s.objects.iter().find(|o| o.name == name).unwrap();
        let c = o.pose.origin;
        let t = o
            .collider
            .trace(c + Vec3::Z * 600., c - Vec3::Z * 600., PLAYER_HALF);
        anyhow::ensure!(!t.start_solid && t.fraction < 1., "No prop support {name}");
        let feet = (c + Vec3::Z * 600.).lerp(c - Vec3::Z * 600., t.fraction) - PLAYER_CENTER;
        let mut p = Player::new(feet);
        p.grounded = true;
        let mut world = World::from_bsp(&map)?;
        world.set_dynamic(s.colliders().collect());
        anyhow::ensure!(
            world.body_clear(feet),
            "Prop test begins embedded {name}: {feet:?}"
        );
        for _ in 0..120 {
            s.advance(1. / 120., &map, &mut world, &mut p, &[])?;
            p.tick(&world, Default::default());
            anyhow::ensure!(world.body_clear(p.feet), "Prop rider embedded {name}");
        }
        anyhow::ensure!(
            p.feet.distance(feet) > 1.,
            "Prop did not carry Alice {name}"
        );
        println!(
            "PASS {name} visible mesh collision and rider movement: {:?}",
            p.feet - feet
        );
    }
    // Check the actual trigger volumes without pretending these are a traversal test.
    let mut i = crate::interaction::Interactions::load(&map)?;
    i.set_entry(assets, &map, "skool2", None)?;
    let arrival = vec3(-64., -3440., -24.);
    let exit = vec3(-64., -1236., 272.);
    anyhow::ensure!(
        i.triggers(0.1, arrival, arrival).transition.is_none()
            && i.triggers(0.1, exit, exit).transition.is_none(),
        "Early school exit"
    );
    let q = &mut i.school2.as_mut().unwrap().quest;
    q.enter(Stage::Rewards);
    q.collect("shrink_potion");
    q.collect("shrink_potion");
    i.reset_contacts();
    anyhow::ensure!(
        i.triggers(0.1, exit, exit).transition.is_none(),
        "Duplicate reward bypassed exit"
    );
    i.school2.as_mut().unwrap().quest.collect("lucky_star");
    i.reset_contacts();
    anyhow::ensure!(
        i.triggers(0.1, exit, exit).transition
            == Some(("skool1".into(), Some("skool1_start2".into()))),
        "Completed potion quest exit does not return to school one"
    );
    i.reset_contacts();
    anyhow::ensure!(
        i.triggers(0.1, arrival, arrival).transition.is_none(),
        "Arrival backdoor opened"
    );
    println!(
        "PASS school-two mover timing, rigid pendulums, paused motion and two-reward exit gating"
    );
    Ok(())
}
/// Staged visual fixtures; deliberately separate from the no-cheats entrance route.
pub async fn check_render(assets: &mut Assets) -> Result<()> {
    crate::interaction::school2_check::render(assets).await?;
    cinema_check::render(assets).await?;
    let mut scene = crate::render::Scene::load(assets, "skool2")?;
    let mut i = crate::interaction::Interactions::load(&scene.map)?;
    i.set_entry(assets, &scene.map, "skool2", None)?;
    let mut art = Art::load(assets, i.school2.as_ref().unwrap())?;
    let mut player = Player::new(vec3(2077., -3090., 440.));
    for (name, eye, target, stage) in [
        (
            "movers",
            vec3(-710., -3100., 160.),
            vec3(-1100., -3120., 20.),
            Stage::Explore,
        ),
        (
            "gym",
            vec3(1980., -2950., 520.),
            vec3(1430., -2850., 448.),
            Stage::Explore,
        ),
        (
            "battle",
            vec3(2077., -3090., 488.),
            vec3(1860., -2870., 460.),
            Stage::Battle,
        ),
        (
            "rescue",
            vec3(-20., -990., 340.),
            vec3(20., -780., 300.),
            Stage::Rescue,
        ),
        (
            "cabinet",
            vec3(70., -700., 350.),
            vec3(70., -430., 340.),
            Stage::Jumbogrow,
        ),
        (
            "growth",
            vec3(-2760., -2570., 570.),
            vec3(-3020., -2550., 560.),
            Stage::Growing,
        ),
        (
            "potion",
            vec3(100., -900., 370.),
            vec3(-60., -760., 325.),
            Stage::Mixing,
        ),
        (
            "rewards",
            vec3(110., -850., 370.),
            vec3(-60., -725., 300.),
            Stage::Rewards,
        ),
    ] {
        let s = i.school2.as_mut().unwrap();
        s.quest.enter(stage);
        if stage == Stage::Battle {
            for b in &mut s.boojums[..3] {
                b.active = true;
            }
        }
        if stage == Stage::Jumbogrow {
            s.quest.enter(Stage::SpiceDialogue);
            s.completed_dialogue("Old_Gnome_SpiceDrops");
        }
        for frame in 0..360 {
            i.advance_school(1. / 60., &scene.map, &mut scene.world, &mut player)?;
            let s = i.school2.as_mut().unwrap();
            if stage == Stage::Battle {
                s.update(1. / 60., &scene.world, player.feet, false);
            } else {
                s.quest.time = frame as f32 / 60.;
                s.age += 1. / 60.;
            }
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: eye,
                target,
                up: Vec3::Z,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 30000.,
                ..Default::default()
            });
            let transforms = i.transforms();
            scene.draw(eye, frame as f32 / 60., false, false, &transforms);
            let s = i.school2.as_ref().unwrap();
            art.draw(s, false, &scene.atmosphere, eye);
            crate::render::depth_read_only(|| {
                scene.draw(eye, frame as f32 / 60., false, true, &transforms);
                for b in &s.boojums {
                    b.draw_waves();
                }
            });
            set_default_camera();
            draw_text(
                &format!("SCHOOL TWO VISUAL FIXTURE / {name}"),
                24.,
                35.,
                22.,
                WHITE,
            );
            if frame == 179 || frame == 359 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/school2-{name}-{frame}.png"
                )))?;
            }
            next_frame().await;
        }
    }
    println!("PASS school-two visual fixtures rendered; inspect captures separately from traversal proof");
    Ok(())
}
