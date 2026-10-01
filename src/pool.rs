//! Reviewed Pool of Tears transports. Original scripts are read as data only.
use crate::{
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    event::{Condition, Facts},
    interaction::{vector, Events},
    movement::Player,
    skeletal::Transform,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
mod ants;
pub mod arrival;
pub mod arrival_check;
pub mod boulder_check;
pub mod boulders;
pub mod cinema;
pub mod cinema_check;
pub mod pilot_check;
pub mod route;
pub mod save_check;
pub mod transport_check;

pub const TALK: &str = "Tears1_Turtle_Cinema1";
fn rot(a: Vec3) -> Quat {
    Quat::from_rotation_z(a.y.to_radians())
        * Quat::from_rotation_y(a.x.to_radians())
        * Quat::from_rotation_x(a.z.to_radians())
}
struct Node {
    p: Vec3,
    r: Quat,
    time: f32,
}
struct Path {
    nodes: Vec<Node>,
}
impl Path {
    fn load(map: &Bsp, first: &str) -> Result<Self> {
        let mut name = first;
        let mut seen = BTreeSet::new();
        let mut nodes = Vec::new();
        let mut time = 0.;
        let mut speed: f32 = 1.;
        let mut angles = Vec3::ZERO;
        loop {
            ensure!(
                seen.insert(name) && seen.len() <= 512,
                "Invalid leaf path {first}"
            );
            let e = map
                .entities
                .iter()
                .find(|e| e.get("targetname").is_some_and(|n| n == name))
                .with_context(|| format!("Missing leaf path node {name}"))?;
            ensure!(
                e.get("classname").is_some_and(|n| n == "info_splinepath"),
                "Not a path node"
            );
            let p = e
                .get("origin")
                .and_then(|s| vector(s))
                .context("Missing leaf path position")?;
            if let Some(a) = e.get("angles").and_then(|s| vector(s)) {
                angles = a;
            }
            if let Some(s) = e.get("speed") {
                speed = s.parse()?;
            }
            ensure!(
                p.is_finite()
                    && angles.is_finite()
                    && speed.is_finite()
                    && speed > 0.
                    && speed <= 20.,
                "Invalid leaf node"
            );
            nodes.push(Node {
                p,
                r: rot(angles),
                time,
            });
            time += 1. / speed;
            let Some(next) = e.get("target") else { break };
            name = next;
        }
        ensure!(nodes.len() >= 2, "Incomplete leaf path");
        Ok(Self { nodes })
    }
    fn duration(&self) -> f32 {
        self.nodes.last().unwrap().time
    }
    fn pose(&self, time: f32) -> (Vec3, Quat) {
        let j = self
            .nodes
            .partition_point(|n| n.time <= time)
            .clamp(1, self.nodes.len() - 1);
        let (a, b) = (&self.nodes[j - 1], &self.nodes[j]);
        let t = ((time - a.time) / (b.time - a.time)).clamp(0., 1.);
        (a.p.lerp(b.p, t), a.r.slerp(b.r, t))
    }
}
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    #[serde(default)]
    pub cinema: cinema::State,
    #[serde(default)]
    pub boulders: Option<boulders::State>,
    pub age: f32,
    pub talking: bool,
    pub talked: bool,
    /// Only the obstructed mover loses time; dialogue, hazards and other leaves keep running.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    blocked_time: BTreeMap<String, f32>,
    drops: [Option<f32>; 4],
    rides: [Option<f32>; 4],
    trains: [Option<(usize, f32)>; 2],
    ended: [bool; 2],
    turtle: [Option<f32>; 3],
}
struct Object {
    name: String,
    model: usize,
    base: Vec3,
    pose: (Vec3, Quat),
    collider: Collider,
}
struct Prop {
    model: String,
    owner: String,
    base: Vec3,
    owner_base: Vec3,
    scale: f32,
}
pub struct Pool {
    cinema: cinema::Data,
    pub state: State,
    paths: Vec<Path>,
    turtle_path: Path,
    objects: Vec<Object>,
    points: BTreeMap<String, Vec3>,
    props: Vec<Prop>,
    // W1: reviewed, unnamed player-clip posts. Never submitted to rendering.
    clips: Vec<Collider>,
}
pub fn supported(e: &BTreeMap<String, String>) -> bool {
    e.get("classname").is_some_and(|n| n == "script_object")
        && e.get("model").is_some_and(|m| m.starts_with('*'))
        && e.get("targetname").is_some_and(|n| {
            n.starts_with("rideleaf")
                || n.starts_with("leaftrain")
                || n.starts_with("woblily")
                || n == "swingingbranchobj"
        })
}
fn ride(n: &str) -> Option<usize> {
    (0..4).find(|i| n == format!("rideleaf{}obj", i + 1))
}
fn train(n: &str) -> Option<(usize, usize)> {
    for group in 0..2 {
        for slot in 0..4 {
            if n == format!("leaftrain{}{}_obj", group + 1, (b'a' + slot as u8) as char) {
                return Some((group, slot));
            }
        }
    }
    None
}
impl Pool {
    pub fn load(assets: &mut crate::assets::Assets, map: &Bsp) -> Result<Self> {
        let paths = (1..=4)
            .map(|i| Path::load(map, &format!("rideleaf{i}path")))
            .collect::<Result<Vec<_>>>()?;
        let turtle_path = Path::load(map, "turtle1path")?;
        let points = map
            .entities
            .iter()
            .filter_map(|e| Some((e.get("targetname")?.clone(), vector(e.get("origin")?)?)))
            .collect::<BTreeMap<_, _>>();
        let mut objects = Vec::new();
        for e in map.entities.iter().filter(|e| supported(e)) {
            let model = e["model"].trim_start_matches('*').parse()?;
            let base = vector(&e["origin"]).context("Invalid pool mover")?;
            objects.push(Object {
                name: e["targetname"].clone(),
                model,
                base,
                pose: (base, Quat::IDENTITY),
                collider: Collider::model(map, model, base, Quat::IDENTITY, true)?,
            });
        }
        let mut props = Vec::new();
        for e in &map.entities {
            let Some(name) = e.get("targetname") else {
                continue;
            };
            let owner = if let Some(n) = name.strip_suffix("mdl") {
                format!("{n}obj")
            } else {
                continue;
            };
            if let Some(o) = objects.iter().find(|o| o.name == owner) {
                props.push(Prop {
                    model: e["model"].trim_end_matches(".tik").into(),
                    owner,
                    base: vector(&e["origin"]).context("Invalid leaf prop")?,
                    owner_base: o.base,
                    scale: e.get("scale").and_then(|s| s.parse().ok()).unwrap_or(1.),
                });
            }
        }
        ensure!(
            objects.len() == 15 && props.len() == 14,
            "Missing Pool of Tears transports: {} objects, {} props",
            objects.len(),
            props.len()
        );
        let mut result = Self {
            state: State {
                cinema: cinema::State::fresh(),
                ..Default::default()
            },
            cinema: cinema::Data::load(assets, map)?,
            paths,
            turtle_path,
            objects,
            points,
            props,
            clips: (1..=6)
                .filter(|&id| {
                    map.difficulty.allows(
                        map.entities[id]
                            .get("spawnflags")
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0),
                    )
                })
                .map(|id| {
                    let e = &map.entities[id];
                    ensure!(
                        e.get("classname").is_some_and(|c| c == "script_object")
                            && e.get("model") == Some(&format!("*{id}"))
                            && !e.contains_key("targetname"),
                        "Pool clip identity changed"
                    );
                    Collider::model(
                        map,
                        id,
                        vector(&e["origin"]).context("Invalid Pool clip")?,
                        Quat::IDENTITY,
                        true,
                    )
                })
                .collect::<Result<_>>()?,
        };
        result.restore_boulders()?;
        result.rebuild(map)?;
        Ok(result)
    }
    fn ready(&self, i: usize) -> bool {
        i == 2 || self.state.drops[i].is_some_and(|t| self.state.age - t >= 9.)
    }
    fn train_time(&self, group: usize, slot: usize) -> Option<(usize, f32)> {
        let (path, start) = self.state.trains[group]?;
        let interval = [15., 15., 13., 8.][path];
        let name = format!("leaftrain{}{}_obj", group + 1, (b'a' + slot as u8) as char);
        let t = self.state.age
            - self.state.blocked_time.get(&name).copied().unwrap_or(0.)
            - start
            - slot as f32 * interval;
        (t >= 0.).then_some((path, t.rem_euclid(4. * interval)))
    }
    fn active(&self, n: &str) -> bool {
        if let Some((g, s)) = train(n) {
            return self
                .train_time(g, s)
                .is_some_and(|(p, t)| t <= self.paths[p].duration());
        }
        true
    }
    fn solid(&self, n: &str) -> bool {
        if !self.active(n) {
            return false;
        }
        if let Some(i) = ride(n) {
            return i >= 2 || !self.state.ended[i];
        }
        if let Some((g, _)) = train(n) {
            return self.state.trains[g].is_some_and(|(p, _)| p >= 2 || !self.state.ended[g]);
        }
        true
    }
    pub fn pose(&self, n: &str, base: Vec3) -> (Vec3, Quat) {
        let s = &self.state;
        let age = s.age - s.blocked_time.get(n).copied().unwrap_or(0.);
        if let Some(i) = ride(n) {
            if let Some(t) = s.rides[i] {
                return self.paths[i].pose(age - t);
            }
            let height = [2048., 2100., 0., 2048.][i];
            let t = s.drops[i].map_or(0., |t| (age - t) / 8.).clamp(0., 1.);
            let wobble = if t > 0. && t < 1. {
                (age * 3.).sin() * 25. * (1. - t)
            } else {
                0.
            };
            return (
                base + Vec3::Z * height * (1. - t),
                rot(vec3(wobble, [0., 240., -135., 210.][i], wobble)),
            );
        }
        if let Some((g, slot)) = train(n) {
            return self
                .train_time(g, slot)
                .map_or((base, Quat::IDENTITY), |(p, t)| self.paths[p].pose(t));
        }
        let (pivot, angles) = match n {
            "woblily1obj" => (
                self.points["woblily1org"],
                vec3(2., 10., 2.) * (age * std::f32::consts::TAU / 5.).sin(),
            ),
            "woblily2obj" => (
                self.points["woblily2org"],
                vec3(2., 10., 2.) * ((age + 0.3) * std::f32::consts::TAU / 4.).sin(),
            ),
            "swingingbranchobj" => (
                self.points["swingingbranchorigin"],
                vec3(10., 0., 0.) * (age * std::f32::consts::TAU / 5.).sin(),
            ),
            _ => return (base, Quat::IDENTITY),
        };
        let r = rot(angles);
        (pivot + r * (base - pivot), r)
    }
    pub fn facts(&self) -> Facts {
        let mut f = Facts::default();
        for i in 0..4 {
            f.flag(&format!("pool.ready{i}"), self.ready(i));
        }
        f.flag("pool.talked", self.state.talked);
        f
    }
    pub fn gate(name: &str) -> Condition {
        for i in 0..4 {
            if name == format!("rideleaf{}trig", i + 1) {
                return Condition::flag(&format!("pool.ready{i}"));
            }
        }
        Condition::Always
    }
    pub fn event(&mut self, n: &str) -> Option<Events> {
        if let Some(event) = self.scene_event(n) {
            return Some(event);
        }
        for i in 0..4 {
            if n == format!("rideleaf{}start", i + 1) {
                if self.ready(i) {
                    if self.state.rides[i].is_none() {
                        self.state
                            .blocked_time
                            .remove(&format!("rideleaf{}obj", i + 1));
                    }
                    self.state.rides[i].get_or_insert(self.state.age);
                }
                return Some(Events::default());
            }
            if n == format!("leaftrain{}", i + 1) {
                let group = i % 2;
                if self.state.trains[group].is_none_or(|(p, _)| p < i) {
                    self.state
                        .blocked_time
                        .retain(|n, _| train(n).is_none_or(|(g, _)| g != group));
                    self.state.trains[group] = Some((i, self.state.age));
                }
                return Some(Events::default());
            }
        }
        match n {
            "leaftrain1end" => self.state.ended[0] = true,
            "leaftrain2end" => self.state.ended[1] = true,
            "rideleaf4falldown" => {
                self.state.drops[3].get_or_insert(self.state.age);
            }
            "Turtle_Encounter1" | "Turtle_Encounter2" | "Turtle_Encounter3" => {
                let index = n.as_bytes()[n.len() - 1] as usize - b'1' as usize;
                self.state.turtle[index].get_or_insert(self.state.age);
            }
            _ => return None,
        }
        Some(Events::default())
    }
    pub fn dialogue_complete(&mut self, n: &str) {
        if n == TALK && self.state.talking && !self.state.talked {
            self.state.talked = true;
            if self.state.cinema.beat == Some(cinema::Beat::Talk) {
                self.state.cinema.beat = Some(cinema::Beat::Depart);
                self.state.cinema.time = 0.;
            } else {
                self.state.drops[0] = Some(self.state.age);
            }
        }
    }
    pub fn objective(&self) -> String {
        if !self.state.talked { "Climb the riverbank and speak to the Mock Turtle at the top." }
        else if self.state.rides[3].is_some() { "Ride the final leaf toward the tunnel to Hollow Hideaway." }
        else if self.state.turtle[2].is_some() { "Follow the upper bank and board the final fallen leaf." }
        else if self.state.rides[2].is_some() { "Follow the third leaf past the posts. Jump and grab the rope before the waterfall, then climb to the bank." }
        else { "Board the fallen leaves to follow the river. Use the banks between rides; more leaves follow if you miss one." }.into()
    }
    pub fn transforms(&self) -> impl Iterator<Item = (usize, Vec3, Quat)> + '_ {
        self.objects
            .iter()
            .filter(|o| self.active(&o.name))
            .map(|o| (o.model, o.pose.0, o.pose.1))
    }
    pub fn colliders(&self) -> impl Iterator<Item = Collider> + '_ {
        self.objects
            .iter()
            .filter(|o| self.solid(&o.name))
            .map(|o| o.collider.clone())
            .chain(self.clips.iter().cloned())
            .chain(self.rock_colliders())
    }
    fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        let poses = self
            .objects
            .iter()
            .map(|o| self.pose(&o.name, o.base))
            .collect::<Vec<_>>();
        for (o, (p, r)) in self.objects.iter_mut().zip(poses) {
            if o.pose != (p, r) {
                o.pose = (p, r);
                o.collider = Collider::model(map, o.model, p, r, true)?;
            }
        }
        Ok(())
    }
    pub fn snapshot(&self) -> State {
        self.state.clone()
    }
    pub fn restore(&mut self, s: &State, map: &Bsp) -> Result<()> {
        s.cinema.validate(s.age)?;
        ensure!(
            s.blocked_time
                .iter()
                .all(|(name, time)| self.objects.iter().any(|o| &o.name == name)
                    && time.is_finite()
                    && *time >= 0.
                    && *time <= s.age),
            "Invalid blocked Pool mover clock"
        );
        ensure!(
            s.age.is_finite() && s.age >= 0. && (!s.talked || s.talking),
            "Invalid pool save"
        );
        for t in s
            .drops
            .iter()
            .chain(&s.rides)
            .chain(&s.turtle)
            .copied()
            .flatten()
            .chain(s.trains.iter().flatten().map(|(_, t)| *t))
        {
            ensure!(t.is_finite() && t >= 0. && t <= s.age, "Invalid leaf timer");
        }
        for (g, t) in s.trains.iter().enumerate() {
            ensure!(
                t.is_none_or(|(p, _)| p < 4 && p % 2 == g),
                "Invalid leaf train"
            );
        }
        for i in [0, 1, 3] {
            ensure!(
                s.rides[i].is_none_or(|t| s.drops[i].is_some_and(|d| t >= d + 8.99)),
                "Leaf started before falling"
            );
        }
        self.state = s.clone();
        if self.state.cinema.version == 0 {
            self.state.cinema = cinema::State::fresh();
            self.state.cinema.done[0] = true;
            self.state.cinema.done[3] = s.talked;
        }
        self.state.cinema.migrate_ending();
        self.restore_arrival()?;
        self.restore_boulders()?;
        self.rebuild(map)
    }
    pub fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. {
            return Ok(());
        }
        let old_colliders = self
            .objects
            .iter()
            .map(|o| o.collider.clone())
            .collect::<Vec<_>>();
        let rider = self
            .objects
            .iter()
            .find(|o| {
                if !self.solid(&o.name) {
                    return false;
                }
                let hit = o.collider.trace(
                    player.feet + PLAYER_CENTER,
                    player.feet + PLAYER_CENTER - Vec3::Z * 3.,
                    // Grounding uses Alice's whole footprint. A centre-only
                    // probe loses a valid rider at a leaf's edge.
                    PLAYER_HALF,
                );
                player.velocity.z <= 1.
                    && !hit.start_solid
                    && hit.fraction < 1.
                    && hit.normal.z > 0.65
            })
            .map(|o| (o.name.clone(), o.pose));
        self.state.age += dt.min(0.1);
        let turtle_time = self.turtle_path.nodes[self.turtle_path.nodes.len() - 2].time + 0.5;
        if self.state.turtle[0].is_some_and(|t| self.state.age - t >= turtle_time) {
            let drop = self.state.turtle[0].unwrap() + turtle_time;
            self.state.drops[1].get_or_insert(drop);
        }
        self.rebuild(map)?;
        self.advance_cinema(dt.min(0.1), world, player)?;
        if self.scene_id().is_some() {
            self.advance_boulders(dt.min(0.1), player);
            world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
            return Ok(());
        }
        let mut feet = player.feet;
        if let Some((n, (p0, r0))) = rider {
            if let Some(o) = self.objects.iter().find(|o| o.name == n && self.solid(&n)) {
                // A recycled train leaf is a new arrival, never a rider teleport.
                if o.pose.0.distance(p0) < 128. {
                    feet = o.pose.0 + o.pose.1 * r0.inverse() * (feet - p0);
                    if let Some((p, _)) = o.collider.rider_feet(feet) {
                        feet = p;
                    }
                }
            }
        }
        world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        if !world.body_clear(feet) && world.body_clear(player.feet) {
            feet = player.feet;
        }
        if !world.body_clear(feet) {
            for height in [1., 2., 4., 8., 12., 18.] {
                let candidate = feet + Vec3::Z * height;
                if world.body_clear(candidate) {
                    feet = candidate;
                    break;
                }
            }
        }
        if world.body_clear(feet) {
            player.feet = feet;
        } else {
            // A swimmer can meet the underside of a leaf, or a rider a bank. Push only
            // through space clear of the fixed world and all other movers. If there is
            // no escape, hold the colliding mover, never the visit's global clock.
            let overlaps = |c: &Collider, p: Vec3| {
                c.trace(p + PLAYER_CENTER, p + PLAYER_CENTER, PLAYER_HALF)
                    .start_solid
            };
            let blocked = self
                .objects
                .iter()
                .enumerate()
                .filter(|(_, o)| self.solid(&o.name) && overlaps(&o.collider, player.feet))
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            world.set_dynamic(
                fixed
                    .iter()
                    .cloned()
                    .chain(
                        self.objects
                            .iter()
                            .enumerate()
                            .filter(|(i, o)| !blocked.contains(i) && self.solid(&o.name))
                            .map(|(_, o)| o.collider.clone()),
                    )
                    .chain(self.clips.iter().cloned())
                    .chain(self.rock_colliders())
                    .collect(),
            );
            let mut escape = None;
            'search: for distance in [2., 4., 8., 16., 24., 32., 48., 64.] {
                for direction in 0..16 {
                    let angle = direction as f32 * std::f32::consts::TAU / 16.;
                    let candidate = player.feet + vec3(angle.cos(), angle.sin(), 0.) * distance;
                    let swept = world.body_trace(player.feet, candidate);
                    if !swept.start_solid
                        && swept.fraction >= 1.
                        && world.body_clear(candidate)
                        && blocked
                            .iter()
                            .all(|&i| !overlaps(&self.objects[i].collider, candidate))
                    {
                        escape = Some(candidate);
                        break 'search;
                    }
                }
            }
            if let Some(p) = escape {
                player.cancel_climb();
                player.feet = p;
            } else {
                for index in blocked {
                    let o = &self.objects[index];
                    if !overlaps(&old_colliders[index], player.feet) {
                        *self.state.blocked_time.entry(o.name.clone()).or_default() += dt.min(0.1);
                    }
                }
                self.rebuild(map)?;
            }
        }
        // Resolve leaf carrying against last frame's rock hulls before moving
        // hazards. A rock contact must not rewind its own timer or damage cue.
        self.advance_boulders(dt.min(0.1), player);
        world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        Ok(())
    }
}

pub struct Art {
    pub cinema: cinema::Art,
    props: BTreeMap<String, crate::weapons::Prop>,
    material: crate::character::SkinMaterial,
}
impl Art {
    pub fn load(assets: &mut crate::assets::Assets, pool: &Pool) -> Result<Self> {
        let specs = crate::texture::read_materials(assets)?;
        let mut props = BTreeMap::new();
        for p in &pool.props {
            if !props.contains_key(&p.model) {
                props.insert(
                    p.model.clone(),
                    crate::weapons::Prop::load_animation(assets, &p.model, "idle", &specs)?,
                );
            }
        }
        Ok(Self {
            cinema: cinema::Art::load(assets)?,
            props,
            material: crate::character::skin_material()?,
        })
    }
    pub fn draw(
        &mut self,
        pool: &Pool,
        fullbright: bool,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
    ) {
        self.material.atmosphere(atmosphere, camera);
        self.material.bind();
        for p in &pool.props {
            if !pool.active(&p.owner) {
                continue;
            }
            let (pos, r) = pool.pose(&p.owner, p.owner_base);
            self.props.get_mut(&p.model).unwrap().draw_frame(
                Transform {
                    translation: pos + r * (p.base - p.owner_base),
                    rotation: r,
                },
                p.scale,
                fullbright,
                pool.state.age,
                true,
            );
        }
        gl_use_default_material();
        self.cinema.draw(pool, atmosphere, camera, fullbright);
    }
}
