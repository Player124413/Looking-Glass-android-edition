//! Beyond the Wall's authored machinery, with shared rendering/collision poses.
use crate::{
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    event::{Condition, Facts},
    interaction::{vector, Events},
    movement::Player,
    skeletal::Transform,
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
use std::collections::BTreeMap;
pub mod cinema;
pub mod cinema_check;

pub fn supported(e: &BTreeMap<String, String>) -> bool {
    e.get("classname").is_some_and(|c| c == "script_object")
        && e.get("model").is_some_and(|m| m.starts_with('*'))
}
fn rot(pitch: f32, yaw: f32, roll: f32) -> Quat {
    Quat::from_rotation_z(yaw.to_radians())
        * Quat::from_rotation_y(pitch.to_radians())
        * Quat::from_rotation_x(roll.to_radians())
}
/// One turn of the flipping corridor's four platforms, in seconds.
pub const LONG_CYCLE: f32 = 88.5;
fn ramp(t: f32, start: f32, duration: f32) -> f32 {
    ((t - start) / duration).clamp(0., 1.)
}
fn index(n: &str, prefix: &str) -> Option<usize> {
    n.strip_prefix(prefix)?.parse().ok()
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum RageLift {
    Waiting,
    Lowering(f32),
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cinema: Option<cinema::State>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rage_started: Option<f32>,
    // Missing in older saves: keep their existing descent, without lifting Alice
    // back up or replaying the pickup. New scenes release the lift at completion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rage_lift: Option<RageLift>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub rage_wave: bool,
    pub age: f32,
    pub long_started: Option<f32>,
    pub puzzle_started: bool,
    pub solved: bool,
    pub lever_used: [bool; 3],
    pub notes: u8,
    note_replay: Option<f32>,
    next_note: usize,
    pub walkway: u8,
    pub step_slots: [u8; 3],
    slide: Option<Slide>,
    pub raised: Option<f32>,
    pub last_started: Option<f32>,
    last_cycle: u8,
    last_from: Option<[Vec3; 3]>,
    pub last_open: bool,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Slide {
    start: f32,
    step: usize,
    from: u8,
    to: u8,
    direction: i8,
}
impl Default for State {
    fn default() -> Self {
        Self {
            cinema: None,
            rage_started: None,
            rage_lift: None,
            rage_wave: false,
            age: 0.,
            long_started: None,
            puzzle_started: false,
            solved: false,
            lever_used: [false; 3],
            notes: 0,
            note_replay: None,
            next_note: 0,
            walkway: 2,
            step_slots: [1, 2, 3],
            slide: None,
            raised: None,
            last_started: None,
            last_cycle: 0,
            last_from: None,
            last_open: false,
        }
    }
}
struct Object {
    name: String,
    model: usize,
    base: Vec3,
    collider: Collider,
    pose: (Vec3, Quat),
}
pub struct Beyond {
    cinema_data: Option<cinema::Data>,
    pub state: State,
    objects: Vec<Object>,
    points: BTreeMap<String, Vec3>,
    pub levers: Vec<(String, Transform)>,
}
impl Beyond {
    pub fn load(map: &Bsp) -> Result<Self> {
        let mut objects = Vec::new();
        let mut points = BTreeMap::new();
        let mut levers = Vec::new();
        for e in &map.entities {
            let Some(base) = e.get("origin").and_then(|s| vector(s)) else {
                continue;
            };
            let name = e.get("targetname").cloned().unwrap_or_default();
            if !name.is_empty() {
                points.insert(name.clone(), base);
            }
            if !map.difficulty.allows(
                e.get("spawnflags")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            ) {
                continue;
            }
            if supported(e) {
                let model = e["model"].trim_start_matches('*').parse()?;
                objects.push(Object {
                    name: name.clone(),
                    model,
                    base,
                    collider: Collider::model(map, model, base, Quat::IDENTITY, true)?,
                    pose: (base, Quat::IDENTITY),
                });
            }
            if e.get("classname").is_some_and(|s| s == "Objects_Lever") {
                levers.push((
                    name,
                    Transform {
                        translation: base,
                        rotation: rot(
                            0.,
                            e.get("angle").and_then(|s| s.parse().ok()).unwrap_or(0.),
                            0.,
                        ),
                    },
                ));
            }
        }
        ensure!(
            levers.len() == 4 && points.contains_key("gs_waypoint10"),
            "Missing Beyond the Wall machinery"
        );
        let mut s = Self {
            cinema_data: None,
            state: State::default(),
            objects,
            points,
            levers,
        };
        s.rebuild(map)?;
        Ok(s)
    }
    fn point(&self, n: &str) -> Vec3 {
        self.points[n]
    }
    fn gs(&self, slot: u8) -> Vec3 {
        self.point(&format!("gs_waypoint{slot}"))
    }
    pub fn step_goal(&self, i: usize) -> Vec3 {
        let n = format!("archstep{i}");
        self.pose(&n, self.point(&n)).0 + Vec3::Z * 8.
    }
    /// Seconds since the room was raised, or -100 before it is (the arches' shared clock).
    pub fn arch_time(&self) -> f32 {
        self.state.raised.map_or(-100., |t| self.state.age - t)
    }
    /// Is a walkway step in flight? The walkway triggers stay shut until it lands.
    pub fn sliding(&self) -> bool {
        self.state.slide.is_some()
    }
    /// Seconds into the flipping corridor's cycle, once its trigger has run.
    pub fn long_phase(&self) -> Option<f32> {
        self.state
            .long_started
            .map(|t| (self.state.age - t).rem_euclid(LONG_CYCLE))
    }
    /// Does this visit's map carry the named machinery? The flipping corridor is not on Easy.
    pub fn has_object(&self, name: &str) -> bool {
        self.objects.iter().any(|o| o.name == name)
    }
    fn arch_height(&self, n: usize) -> f32 {
        let (start, duration, height) = match n {
            1 => (9., 8., 192.),
            2 => (5., 10., 384.),
            _ => (0., 12., 576.),
        };
        height * ramp(self.arch_time(), start, duration)
    }
    fn last_pose(&self, i: usize) -> (Vec3, Quat) {
        let s = &self.state;
        let source = ["lasta_org", "lastb_org", "lastc_org"][i];
        let initial = ["c2", "b2", "a2"][i];
        let mut p = s
            .last_from
            .map_or_else(|| self.point(initial), |from| from[i]);
        let Some(start) = s.last_started else {
            return (p - self.point(source), Quat::IDENTITY);
        };
        let t = s.age - start;
        // Original shuffled destinations. Each identity keeps its door and trigger.
        let prefix = ["a", "b", "c"][i];
        let seq: Vec<(f32, u8)> = match s.last_cycle {
            2 => vec![
                (3., 1),
                (5., if i == 1 { 1 } else { 2 }),
                (
                    7.,
                    if i == 2 {
                        3
                    } else if i == 1 {
                        1
                    } else {
                        2
                    },
                ),
            ],
            _ => {
                let offset = if s.last_cycle == 0 { 5.2 } else { 3. };
                let mut seq = if s.last_cycle == 0 {
                    vec![(3.2, [2, 1, 3][i])]
                } else {
                    Vec::new()
                };
                for (j, slots) in [[2, 2, 2], [1, 2, 3], [2, 3, 1], [3, 1, 2]]
                    .iter()
                    .enumerate()
                {
                    seq.push((offset + j as f32 * 2.25 + i as f32 * 0.75, slots[i]));
                }
                seq
            }
        };
        for (begin, slot) in seq {
            let dest = self.point(&format!("{prefix}{slot}"));
            if t < begin {
                break;
            }
            let a = ramp(t, begin, 1.5);
            if a < 1. {
                return (
                    p.lerp(dest, a) + Vec3::Z * (512. * 4. * a * (1. - a)) - self.point(source),
                    rot(360. * a, 0., 0.),
                );
            }
            p = dest;
        }
        (p - self.point(source), Quat::IDENTITY)
    }
    pub fn last_ready(&self) -> bool {
        self.state.last_started.is_some_and(|t| {
            self.state.age - t
                >= match self.state.last_cycle {
                    0 => 15.,
                    1 => 12.8,
                    _ => 8.5,
                }
        })
    }
    pub fn pose(&self, n: &str, base: Vec3) -> (Vec3, Quat) {
        let s = &self.state;
        let mut p = base;
        let mut r = Quat::IDENTITY;
        if let Some(i) = index(n, "gs_step").filter(|i| (1..=3).contains(i)) {
            p = self.gs(s.step_slots[i - 1]);
            if let Some(a) = s.slide.as_ref().filter(|a| a.step == i - 1) {
                let t = ramp(s.age, a.start, 0.65);
                p = self.gs(a.from).lerp(self.gs(a.to), t) - Vec3::Z * (160. * 4. * t * (1. - t));
                r = rot(0., 360. * t, 0.);
            }
        } else if matches!(n, "gs_forward" | "gs_backwards") {
            let mut slot = self.gs(s.walkway);
            if let Some(a) = &s.slide {
                slot = slot.lerp(
                    self.gs((s.walkway as i8 + a.direction) as u8),
                    ramp(s.age, a.start, 0.65),
                );
            }
            p += slot - self.gs(2);
        } else if let Some(i) = index(n, "gs_door") {
            let open = if (1..=3).contains(&i) {
                self.door_open(i)
            } else {
                1.
            };
            r = rot(0., if i % 2 == 1 { 75. * open } else { -75. * open }, 0.);
        } else if let Some(i) = index(n, "longwalk").filter(|i| (1..=4).contains(i)) {
            if let Some(start) = s.long_started {
                let t = (s.age - start).rem_euclid(LONG_CYCLE);
                let offset = (i - 1) as f32 * 0.5;
                let mut from = base;
                let stages = [
                    ("", offset, 1.5, 384.),
                    ("1", 2.5 + offset, 1.5, 384.),
                    ("2", 4.5 + offset, 1.5, 384.),
                    ("3", 6.5 + offset, 1.5, 384.),
                    ("2", 15.5 + (4 - i) as f32 * 3., 3., -384.),
                    ("1", 33.5 + (4 - i) as f32 * 3., 3., -384.),
                    ("", 51.5 + (4 - i) as f32 * 3., 3., -384.),
                    ("x", 69.5 + (4 - i) as f32 * 3., 3., -384.),
                ];
                for (j, (group, begin, duration, height)) in stages.iter().enumerate() {
                    if t < *begin {
                        break;
                    }
                    let to = self.point(&format!("movehere{group}{i}"));
                    let a = ramp(t, *begin, *duration);
                    p = from.lerp(to, a) + Vec3::Z * (height * 4. * a * (1. - a));
                    r = rot(-360. * a, if j % 2 == 0 { 540. } else { 360. } * a, 0.);
                    if a < 1. {
                        break;
                    }
                    // Each flip begins from the preceding final angle.
                    r = rot(0., if j % 2 == 0 { 180. } else { 0. }, 0.);
                    from = to;
                }
            }
        } else if let Some(i) = index(n, "archstep").filter(|i| (1..=15).contains(i)) {
            let t = self.arch_time() - 2.;
            let pivot = self.point("archorigin");
            let initial = rot(-180. * ramp(t, 0., 3.), 0., 0.);
            p = pivot + initial * (base - pivot);
            r = initial;
            let begin = 3.5 + (i - 1) as f32 * 0.5;
            if t >= begin {
                p = pivot + rot(-180., 0., 0.) * (base - pivot);
                let a = ramp(t, begin, 2.5);
                p.z += (384. - (i - 1) as f32 * 25.6) * a;
                r = rot(-180. * (1. - a), -180. * a, 0.);
                let wave = t - 6. - (i - 1) as f32;
                if wave > 0. {
                    p.z += 128. * (1. - ((wave.rem_euclid(4.) - 2.) / 2.).abs());
                }
            }
        } else if matches!(n, "archroof1" | "flippydoor2" | "toskool" | "ra_facelamp1") {
            p.z += self.arch_height(3);
            if n == "flippydoor2" {
                r = rot(0., 75. * ramp(self.arch_time(), 17., 1.4), 0.);
            }
        } else if n.starts_with("archpillar") {
            p.y -= 776.;
        } else if n.starts_with("archscope") {
            p.x -= 1504.;
            let i = n.as_bytes()[9] - b'0';
            let t = self.arch_time();
            let h = match i {
                1 => 128. * ramp(t, 12., 5.),
                2 => 256. * ramp(t, 9., 6.),
                _ => 384. * ramp(t, 4.75, 7.),
            };
            p.z += if n.ends_with("part2") {
                if i == 2 {
                    128. + 128. * ramp(t, 12., 3.)
                } else if i == 3 {
                    192. + 192. * ramp(t, 8.25, 3.5)
                } else {
                    0.
                }
            } else {
                h
            };
        } else if n.starts_with("archpost") {
            p.x += 1440.;
            p.z += self.arch_height(index(n, "archpost").unwrap());
        } else if matches!(n, "arch1" | "arch2" | "arch3") {
            p.z += self.arch_height(index(n, "arch").unwrap());
        } else if n == "ragelift" {
            p.z -= 320.;
            if let Some(start) = s.rage_started {
                p.z -= 48.
                    * match s.rage_lift {
                        Some(RageLift::Waiting) => 0.,
                        Some(RageLift::Lowering(start)) => ramp(s.age - start, 0., 4.),
                        None => ramp(s.age - start, 1., 4.),
                    };
            }
            if let Some(start) = s.last_started {
                let t = s.age - start;
                let delay = if s.rage_started.is_some() { 0. } else { 4. };
                if s.rage_started.is_none() {
                    p.z -= 48. * ramp(t, 0., 4.);
                }
                if t > delay {
                    p.z += 368. * (1. - (((t - delay).rem_euclid(8.) - 4.) / 4.).abs());
                }
            }
        } else if n.starts_with("lasta")
            || n.starts_with("lastb")
            || n.starts_with("lastc")
            || n == "last_changelevel"
        {
            let i = if n == "last_changelevel" {
                0
            } else {
                (n.as_bytes()[4] - b'a') as usize
            };
            let (offset, q) = self.last_pose(i);
            let pivot = self.point(["lasta_org", "lastb_org", "lastc_org"][i]);
            p = pivot + offset + q * (base - pivot);
            r = q;
            if n.ends_with("_door")
                && i == 0
                && (s.last_open || s.last_started.is_some_and(|t| s.age - t < 2.2))
            {
                r *= rot(0., -90., 0.);
            }
        }
        (p, r)
    }
    fn visible(&self, n: &str) -> bool {
        match n {
            "skymojo" => false,
            "flippydoorportal" => self.arch_time() >= 17.,
            "doorframe1" => self.state.last_started.is_none(),
            "doorframe2" | "lift_bridge" => self.state.last_started.is_some(),
            "lasta" | "lastb" | "lastc" | "lasta_door" | "lastb_door" | "lastc_door" => {
                self.state.raised.is_some()
            }
            "archscope1part2" => false,
            "archscope2part2" => self.arch_time() >= 12.,
            "archscope3part2" => self.arch_time() >= 8.25,
            "archpillar1part2" | "archpillar2part2" => self.arch_time() >= 3.5,
            _ => true,
        }
    }
    fn solid(&self, n: &str) -> bool {
        self.visible(n) && n != "flippydoorportal" && n != "skymojo"
    }
    pub fn colliders(&self) -> impl Iterator<Item = Collider> + '_ {
        self.objects
            .iter()
            .filter(|o| self.solid(&o.name))
            .map(|o| o.collider.clone())
    }
    pub fn transforms(&self) -> impl Iterator<Item = (usize, Vec3, Quat)> + '_ {
        self.objects
            .iter()
            .filter(|o| self.visible(&o.name))
            .map(|o| {
                let (p, r) = self.pose(&o.name, o.base);
                (o.model, p, r)
            })
    }
    pub fn trigger_pose(&self, n: &str, base: Vec3) -> Option<(Vec3, Quat)> {
        matches!(
            n,
            "gs_forward"
                | "gs_backwards"
                | "toskool"
                | "lasta_trg"
                | "lastb_trg"
                | "lastc_trg"
                | "last_changelevel"
        )
        .then(|| self.pose(n, base))
    }
    pub fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("beyond.walkway_ready", self.state.slide.is_none());
        f.flag("beyond.raised", self.arch_time() >= 18.4);
        f.flag(
            "beyond.last_ready",
            self.last_ready() && !self.state.last_open,
        );
        f.flag("beyond.last_open", self.state.last_open);
        f.flag("beyond.last_started", self.state.last_started.is_some());
        f.flag("beyond.solved", self.state.solved);
        f
    }
    pub fn gate(name: &str) -> Condition {
        match name {
            "gs_forward" | "gs_backwards" => Condition::flag("beyond.walkway_ready"),
            "toskool" => Condition::flag("beyond.raised"),
            "last_changelevel" => Condition::flag("beyond.last_open"),
            "lasta_trg" | "lastb_trg" | "lastc_trg" => Condition::flag("beyond.last_ready"),
            "bottom_door_trigger" => Condition::flag("beyond.last_started").not(),
            "cat_easy_trigger" | "cat_hard_trigger" => Condition::flag("beyond.solved").not(),
            _ => Condition::Always,
        }
    }
    pub fn event(&mut self, n: &str) -> Option<Events> {
        let last_from = [0, 1, 2]
            .map(|i| self.last_pose(i).0 + self.point(["lasta_org", "lastb_org", "lastc_org"][i]));
        let s = &mut self.state;
        let mut message = None;
        match n {
            "Long_Walk_Flip" => {
                s.long_started.get_or_insert(s.age);
            }
            "Start_GetSmart" => {
                if s.puzzle_started {
                    return Some(Events::default());
                }
                s.cinema = Some(cinema::State::new(cinema::Beat::Demonstration));
                s.puzzle_started = true;
                s.note_replay = Some(s.age);
                s.next_note = 0;
                message = Some(
                    "Listen to the three notes, then match their order using the levers below.",
                );
            }
            "GS_Movers_Reset" => {
                s.slide = None;
                s.walkway = 2;
                s.step_slots = [1, 2, 3];
            }
            "GS_Move" | "GS_MoveBack" => {
                let direction = if n == "GS_Move" { 1 } else { -1 };
                if s.slide.is_none()
                    && ((direction == 1 && s.walkway < 9) || (direction == -1 && s.walkway > 2))
                {
                    let from = (s.walkway as i8 - direction) as u8;
                    let step = s.step_slots.iter().position(|v| *v == from).unwrap();
                    s.slide = Some(Slide {
                        start: s.age,
                        step,
                        from,
                        to: (s.walkway as i8 + direction * 2) as u8,
                        direction,
                    });
                }
            }
            "RAISE_ROOM" => {
                if s.raised.is_none() {
                    s.raised = Some(s.age);
                    s.cinema = Some(cinema::State::new(cinema::Beat::Arches));
                }
                message = Some("The room is rising. Climb the moving steps to the upper door.");
            }
            "LastStart" => {
                s.last_started.get_or_insert(s.age);
                message = Some("Watch the open door, then follow it through the shuffle.");
            }
            "LastOpenA" => {
                if self.last_ready() {
                    self.state.last_open = true;
                }
            }
            "LastOpenB" | "LastOpenC" => {
                s.last_from = Some(last_from);
                s.last_cycle = if n == "LastOpenB" { 1 } else { 2 };
                s.last_started = Some(s.age);
                message = Some("Watch the correct door and try again.");
            }
            "Get_Diamond1" | "HubAmbush" | "boo3" | "Arch_Spawn_Cards2" => (),
            "cat_dialog_easy" | "cat_dialog_hard" => {
                return Some(Events {
                    story: vec![n.into()],
                    ..Default::default()
                });
            }
            _ => return None,
        }
        Some(Events {
            message: message.map(str::to_owned),
            ..Default::default()
        })
    }
    fn nearby(&self, world: &World, eye: Vec3, aim: Vec3) -> Option<usize> {
        if self.scene_id().is_some() || self.state.solved || !self.state.puzzle_started {
            return None;
        }
        self.levers
            .iter()
            .enumerate()
            .filter_map(|(i, (_, t))| {
                let point = t.point(vec3(36., 0., 26.));
                let delta = point - eye;
                let trace = world.sweep(eye, point, Vec3::splat(0.5));
                (delta.length() < 125.
                    && delta.normalize_or_zero().dot(aim) > 0.45
                    && !trace.start_solid
                    && trace.fraction >= 1.)
                    .then_some((i, delta.length()))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|p| p.0)
    }
    pub fn prompt(&self, world: &World, eye: Vec3, aim: Vec3) -> Option<&'static str> {
        let i = self.nearby(world, eye, aim)?;
        Some(if self.levers[i].0 == "gs_reset_lever" {
            "E  reset and replay the three notes"
        } else {
            "E  play lever note"
        })
    }
    pub fn activate(&mut self, world: &World, eye: Vec3, aim: Vec3) -> Option<Events> {
        let i = self.nearby(world, eye, aim)?;
        let n = &self.levers[i].0;
        if n == "gs_reset_lever" {
            self.state.cinema = Some(cinema::State::new(cinema::Beat::Reset));
            self.state.notes = 0;
            self.state.lever_used = [false; 3];
            self.state.note_replay = Some(self.state.age);
            self.state.next_note = 0;
            return Some(Events {
                message: Some("Lever sequence reset. Listen and try again.".into()),
                ..Default::default()
            });
        }
        let lever = index(n, "gs_lever")?;
        if self.state.lever_used[lever - 1] {
            return Some(Events {
                message: Some("Already used / reset the sequence at the upper lever".into()),
                ..Default::default()
            });
        }
        self.state.lever_used[lever - 1] = true;
        if [3, 1, 2].get(self.state.notes as usize) == Some(&lever) {
            self.state.notes += 1;
            self.state.solved = self.state.notes == 3;
            if self.state.solved {
                self.state.cinema = Some(cinema::State::new(cinema::Beat::Solved));
            }
        }
        Some(Events {
            message: Some(
                if self.state.solved {
                    "The three doors are opening."
                } else {
                    "A note echoes through the hall."
                }
                .into(),
            ),
            sound: Some(
                SOUNDS[match lever {
                    3 => 0,
                    1 => 1,
                    _ => 2,
                }]
                .into(),
            ),
            ..Default::default()
        })
    }
    pub fn update(&mut self) -> Events {
        if let Some(start) = self.state.note_replay {
            if self.state.next_note < 3 && self.state.age - start >= self.note_time() {
                let sound = SOUNDS[self.state.next_note].into();
                self.state.next_note += 1;
                if self.state.next_note == 3 {
                    self.state.note_replay = None;
                }
                return Events {
                    sound: Some(sound),
                    ..Default::default()
                };
            }
        }
        Events::default()
    }
    fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        let poses = self
            .objects
            .iter()
            .map(|o| self.pose(&o.name, o.base))
            .collect::<Vec<_>>();
        for (o, (p, r)) in self.objects.iter_mut().zip(poses) {
            if o.pose != (p, r) {
                o.collider = Collider::model(map, o.model, p, r, true)?;
                o.pose = (p, r);
            }
        }
        Ok(())
    }
    pub fn snapshot(&self) -> State {
        self.state.clone()
    }
    pub fn restore(&mut self, s: &State, map: &Bsp) -> Result<()> {
        ensure!(
            s.age.is_finite()
                && s.age >= 0.
                && (2..=9).contains(&s.walkway)
                && s.notes <= 3
                && s.next_note <= 3
                && s.solved == (s.notes == 3)
                && (!s.rage_wave || s.rage_started.is_some()),
            "Invalid Beyond the Wall save"
        );
        for t in [
            s.long_started,
            s.raised,
            s.last_started,
            s.note_replay,
            s.rage_started,
        ]
        .into_iter()
        .flatten()
        {
            ensure!(
                t.is_finite() && t >= 0. && t <= s.age,
                "Invalid machinery timer"
            );
        }
        ensure!(
            s.step_slots.iter().all(|v| (1..=10).contains(v)) && s.last_cycle <= 2,
            "Invalid machinery state"
        );
        let mut slots = s.step_slots;
        slots.sort();
        ensure!(
            slots == [s.walkway - 1, s.walkway, s.walkway + 1],
            "Disconnected walkway state"
        );
        if let Some(a) = &s.slide {
            ensure!(
                a.start.is_finite()
                    && a.start <= s.age
                    && a.start >= 0.
                    && a.step < 3
                    && [1, -1].contains(&a.direction)
                    && (1..=10).contains(&a.from)
                    && (1..=10).contains(&a.to),
                "Invalid walkway motion"
            );
            ensure!(
                s.step_slots[a.step] == a.from
                    && a.from as i8 == s.walkway as i8 - a.direction
                    && a.to as i8 == s.walkway as i8 + 2 * a.direction
                    && s.age - a.start <= 0.66,
                "Inconsistent walkway motion"
            );
        }
        ensure!(
            s.last_from.is_none_or(|v| v.iter().all(|p| p.is_finite())),
            "Invalid door positions"
        );
        ensure!(
            (0..3).all(|i| (i + 1..3).all(|j| s.step_slots[i] != s.step_slots[j])),
            "Duplicate walkway slots"
        );
        if let Some(c) = &s.cinema {
            c.validate(s)?;
        }
        if let Some(lift) = &s.rage_lift {
            ensure!(s.rage_started.is_some(), "Rage lift without pickup");
            ensure!(
                match lift {
                    RageLift::Waiting => s
                        .cinema
                        .as_ref()
                        .is_some_and(|c| c.beat == cinema::Beat::Rage && !c.finished),
                    RageLift::Lowering(t) =>
                        t.is_finite()
                            && *t >= s.rage_started.unwrap()
                            && *t <= s.age
                            && !s
                                .cinema
                                .as_ref()
                                .is_some_and(|c| c.beat == cinema::Beat::Rage && !c.finished),
                },
                "Invalid Rage lift phase"
            );
        }
        self.state = s.clone();
        self.rebuild(map)
    }
    pub fn pickup(&mut self, stats: &crate::inventory::Stats) -> bool {
        if self.state.rage_started.is_none() && stats.collected.contains("fortress2:37") {
            self.state.rage_started = Some(self.state.age);
            // Only a newly accepted pickup starts the performance. Existing saves
            // with a recorded callback must never replay it just for active Rage.
            if stats.powers.rage > 0. && self.scene_id().is_none() {
                self.state.cinema = Some(cinema::State::new(cinema::Beat::Rage));
                self.state.rage_lift = Some(RageLift::Waiting);
            }
            return true;
        }
        false
    }
    pub fn rage_wave(&mut self) -> bool {
        if !self.state.rage_wave
            && self
                .state
                .rage_started
                .is_some_and(|t| self.state.age - t >= 1.)
        {
            self.state.rage_wave = true;
            return true;
        }
        false
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
        self.scene_hold(player);
        let old = self.state.clone();
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
                    vec3(1., 1., PLAYER_HALF.z),
                );
                player.velocity.z <= 1.
                    && !hit.start_solid
                    && hit.fraction < 1.
                    && hit.normal.z > 0.65
            })
            .map(|o| (o.model, o.name.clone(), o.base, self.pose(&o.name, o.base)));
        self.state.age += dt.min(0.1);
        if let Some(c) = &mut self.state.cinema {
            c.time = (c.time + dt.min(0.1)).min(c.beat.duration() + 0.5);
        }
        if let Some(a) = self
            .state
            .slide
            .clone()
            .filter(|a| self.state.age - a.start >= 0.65)
        {
            self.state.step_slots[a.step] = a.to;
            self.state.walkway = (self.state.walkway as i8 + a.direction) as u8;
            self.state.slide = None;
        }
        self.rebuild(map)?;
        let mut feet = player.feet;
        if let Some((model, n, base, (p0, r0))) = rider {
            let (p1, r1) = self.pose(&n, base);
            feet = p1 + r1 * r0.inverse() * (feet - p0);
            if let Some((f, _)) = self
                .objects
                .iter()
                .find(|o| o.model == model)
                .and_then(|o| o.collider.rider_feet(feet))
            {
                feet = f;
            }
        }
        world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        // At adjoining tiles, the upright body can touch both supports. Let a
        // departing support move away; resolve a small lip as a normal step.
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
            self.state = old;
            self.rebuild(map)?;
        }
        world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        self.scene_finish_step(world, player)?;
        Ok(())
    }
}

pub struct Art {
    rage_pickup: crate::power_pose::effects::Pickup,
    rest: crate::weapons::Prop,
    pull: crate::weapons::Prop,
    machine: crate::weapons::Prop,
    reset_machine: crate::weapons::Prop,
    material: crate::character::SkinMaterial,
}
impl Art {
    pub fn load(assets: &mut crate::assets::Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(assets)?;
        Ok(Self {
            rage_pickup: crate::power_pose::effects::Pickup::load(assets, &specs)?,
            rest: crate::weapons::Prop::load_animation(assets, "lever", "start", &specs)?,
            pull: crate::weapons::Prop::load_animation(assets, "lever", "move", &specs)?,
            machine: crate::weapons::Prop::load_animation(
                assets,
                "obj_door_machine01",
                "run",
                &specs,
            )?,
            reset_machine: crate::weapons::Prop::load_animation(
                assets,
                "obj_door_machine02",
                "spin",
                &specs,
            )?,
            material: crate::character::skin_material()?,
        })
    }
    pub fn draw(
        &mut self,
        s: &Beyond,
        fullbright: bool,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
    ) {
        self.material.atmosphere(atmosphere, camera);
        self.material.bind();
        self.machine.draw_frame(
            Transform {
                translation: s.point("doormachine1"),
                rotation: rot(0., 90., 0.),
            },
            1.,
            fullbright,
            if s.state.puzzle_started {
                s.state.age
            } else {
                0.
            },
            true,
        );
        self.reset_machine.draw_frame(
            Transform {
                translation: s.point("doormachine2"),
                rotation: rot(0., 90., 0.),
            },
            1.,
            fullbright,
            s.state.note_replay.map_or(0., |t| s.state.age - t),
            false,
        );
        for (n, t) in s.levers.iter().filter(|_| s.levers_visible()) {
            if index(n, "gs_lever").is_some_and(|i| s.state.lever_used[i - 1]) {
                self.pull.draw_frame(*t, 1., fullbright, 10., false);
            } else {
                self.rest.draw(*t, 1., fullbright);
            }
        }
        if let Some((time, false)) = s.rage_scene() {
            self.rage_pickup.draw(
                time,
                Transform {
                    translation: s.point("spawn_ragebox"),
                    rotation: Quat::IDENTITY,
                },
                fullbright,
                camera,
                atmosphere,
            );
        }
        gl_use_default_material();
    }
}

pub const SOUNDS: [&str; 3] = [
    "sound/ambience/special/door_machine_tone3.wav",
    "sound/ambience/special/door_machine_tone4.wav",
    "sound/ambience/special/door_machine_tone5.wav",
];
