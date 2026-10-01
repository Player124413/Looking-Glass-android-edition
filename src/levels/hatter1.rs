//! Crazed Clockwork: persistent machinery and reviewed scene handoffs.
mod art;
mod lever;
mod route;
pub(crate) use route::drive as drive_route;
mod check;
mod motion;
mod saves;
mod scene;
use super::{state, Beat, Check, Registration, Run};
use crate::{
    assets::Assets,
    bsp::Bsp,
    cinematic::Camera,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    event::{Condition, Facts},
    interaction::{self, Events},
    inventory::Stats,
    level::{LevelArt, LevelController, TriggerInfo},
    movement::Player,
    skeletal::Transform,
    story::Story,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use scene::{Kind, Scene};
use serde::{Deserialize, Serialize};
use std::{any::Any, collections::BTreeMap};
const LEVERS: [usize; 5] = [376, 378, 289, 694, 44];
const THREADS: [&str; 5] = [
    "Gear_Bridge_One",
    "Gear_Bridge_Two",
    "Open_ClockRoom_Door",
    "Open_Port",
    "stop_clock",
];
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    initialized: bool,
    age: f32,
    levers: [Option<f32>; 5],
    no_return: Option<f32>,
    clockroom_closed: Option<f32>,
    lift: f32,
    lift_up: bool,
    floater: Option<f32>,
    hare: Option<f32>,
    port: Option<f32>,
    gryphon: Option<f32>,
    extendo: Option<f32>,
    cubes: [bool; 4],
    clock: Option<f32>,
    stopped: bool,
    doors: Vec<f32>,
    holds: Vec<f32>,
    sinks: Vec<f32>,
    hints: [bool; 2],
    scene: Option<Scene>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pull: Option<lever::Pull>,
}
impl Saved {
    fn new(n: usize) -> Self {
        Self {
            version: 1,
            initialized: false,
            age: 0.,
            levers: [None; 5],
            no_return: None,
            clockroom_closed: None,
            lift: 0.,
            lift_up: true,
            floater: None,
            hare: None,
            port: None,
            gryphon: None,
            extendo: None,
            cubes: [false; 4],
            clock: None,
            stopped: false,
            doors: vec![0.; n],
            holds: vec![0.; n],
            sinks: vec![0.; n],
            hints: [false; 2],
            scene: None,
            pull: None,
        }
    }
    fn both(&self) -> Option<f32> {
        Some(self.levers[0]?.max(self.levers[1]?) + 2.)
    }
    fn elapsed(&self, t: Option<f32>, length: f32) -> f32 {
        t.map_or(0., |t| ((self.age - t) / length).clamp(0., 1.))
    }
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        if let Some(pull) = &self.pull { pull.validate(self)?; }
        state::clock("machinery", self.age, 1e7)?;
        for t in self
            .levers
            .into_iter()
            .chain([
                self.no_return,
                self.clockroom_closed,
                self.floater,
                self.hare,
                self.port,
                self.gryphon,
                self.extendo,
                self.clock,
            ])
            .flatten()
        {
            state::clock("event", t, self.age)?;
        }
        ensure!(
            self.doors.len() <= 512
                && self.doors.len() == self.sinks.len()
                && self.holds.len() == self.doors.len(),
            "Invalid machinery count"
        );
        for v in &self.doors {
            state::fraction("door", *v)?;
        }
        for v in &self.holds {
            state::clock("door hold", *v, 10.)?;
        }
        for v in &self.sinks {
            state::clock("sink", *v, 2048.)?;
        }
        state::clock("lift", self.lift, 170.)?;
        ensure!(
            self.clock.is_some() == self.cubes.iter().all(|b| *b),
            "Chair and clock state disagree"
        );
        ensure!(
            !self.stopped || self.levers[4].is_some(),
            "Stopped clock without lever"
        );
        ensure!(
            self.port.is_none() || self.levers[3].is_some(),
            "Port without lever"
        );
        ensure!(
            self.gryphon.is_none() || self.port.is_some(),
            "Gryphon before port"
        );
        ensure!(
            self.extendo.is_none() || self.gryphon.is_some(),
            "Bridge before Gryphon"
        );
        ensure!(
            !self.cubes.iter().any(|b| *b) || self.extendo.is_some(),
            "Chairs before bridge"
        );
        ensure!(
            self.levers[2].is_none() || self.both().is_some(),
            "Clockroom before gears"
        );
        ensure!(
            self.levers[3].is_none() || self.hare.is_some(),
            "Port lever before laboratory"
        );
        ensure!(
            self.levers[4].is_none() || self.clock.is_some(),
            "Stop lever before chairs"
        );
        ensure!(
            self.no_return.is_none() || self.both().is_some(),
            "Barrier before gears"
        );
        ensure!(
            self.clockroom_closed.is_none() || self.levers[2].is_some(),
            "Clockroom closed before opened"
        );
        if let Some(s) = &self.scene {
            s.validate()?;
        }
        Ok(())
    }
}
struct Clockwork {
    saved: Saved,
    objects: Vec<motion::Object>,
    points: BTreeMap<String, Transform>,
    levers: Vec<Transform>,
    pull_duration: f32,
    cameras: BTreeMap<String, crate::fortress::spline::Spline>,
    floater: crate::fortress::spline::Spline,
}
fn at(e: &super::Entity) -> Transform {
    Transform {
        translation: e
            .get("origin")
            .and_then(|s| interaction::vector(s))
            .unwrap_or(Vec3::ZERO),
        rotation: Quat::from_rotation_z(
            e.get("angle")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.)
                .to_radians(),
        ),
    }
}
impl Clockwork {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        ensure!(
            map.entities[44]
                .get("move_thread")
                .is_some_and(|s| s == "stop_clock"),
            "Unexpected Clockwork layout"
        );
        let objects = motion::load(map)?;
        let mut cameras = BTreeMap::new();
        for n in [
            "hatter1_path2",
            "hatter1_path3",
            "hatter1_path4",
            "hatter1_jpath1",
            "hatter1_jpath2",
            "hatter1_jpath3",
            "hatter1_jpath4",
            "hatter1_jpath5",
            "hatter1_jpath6",
            "hatter1_jpath7",
            "hatter1_jpath8",
            "hatter_jpath9",
            "hatter1_jpath10",
            "hatter1_jpath11",
            "hatter1_jdm1",
            "hatter1_jdm2",
            "hatter1_jdm3",
            "hatter1_end1",
            "hatter1_end2",
            "hatter1_end3",
        ] {
            cameras.insert(
                n.into(),
                crate::fortress::spline::Spline::camera_track(
                    crate::cinematic::Track::load(a, n)?.controls().collect(),
                ),
            );
        }
        let points: BTreeMap<_, _> = map
            .entities
            .iter()
            .filter_map(|e| Some((e.get("targetname")?.clone(), at(e))))
            .collect();
        let floater = crate::fortress::spline::Spline::new(
            [4, 1, 2, 3]
                .into_iter()
                .map(|i| {
                    let t = points[&format!("f_pth{i}")];
                    (t.translation, t.rotation, 0.3)
                })
                .collect(),
            true,
        );
        let mut o = Self {
            saved: Saved::new(objects.len()),
            objects,
            points,
            cameras,
            floater,
            levers: LEVERS.into_iter().map(|i| at(&map.entities[i])).collect(),
            pull_duration: lever::duration(a)?,
        };
        o.rebuild(map)?;
        Ok(o)
    }
    fn point(&self, n: &str) -> Transform {
        self.points[n]
    }
    fn can_lever(&self, k: usize) -> bool {
        self.saved.scene.is_none()
            && self.saved.pull.is_none()
            && self.saved.levers[k].is_none()
            && match k {
                0 | 1 => true,
                2 => self.saved.both().is_some_and(|t| self.saved.age >= t),
                3 => self.saved.hare.is_some(),
                4 => self.saved.elapsed(self.saved.clock, 5.) == 1.,
                _ => false,
            }
    }
    fn press(&mut self, k: usize) {
        if !self.can_lever(k) {
            return;
        }
        self.saved.levers[k] = Some(self.saved.age);
        if k == 4 {
            self.begin(Kind::Stop);
        }
    }
    fn use_lever(&self, w: &World, eye: Vec3, aim: Vec3) -> Option<usize> {
        self.levers.iter().enumerate().find_map(|(k, p)| {
            let target = p.translation + Vec3::Z * 20.;
            let d = target - eye;
            (self.can_lever(k)
                && d.length() < 130.
                && d.normalize_or_zero().dot(aim.normalize_or_zero()) > 0.35
                && w.sweep(eye, target, Vec3::splat(0.5)).fraction > 0.96)
                .then_some(k)
        })
    }
}
impl LevelController for Clockwork {
    fn id(&self) -> &'static str {
        "hatter1"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        for (n, b) in [
            ("free", self.saved.scene.is_none() && self.saved.pull.is_none()),
            (
                "gear",
                self.saved.both().is_some_and(|t| self.saved.age >= t + 1.),
            ),
            ("room", self.saved.levers[2].is_some()),
            ("port", self.saved.port.is_some()),
            ("gryphon", self.saved.gryphon.is_some()),
            ("extendo", self.saved.extendo.is_some()),
            ("chairs", self.saved.clock.is_some()),
            ("clock", self.saved.elapsed(self.saved.clock, 5.) == 1.),
            ("exit", self.saved.stopped),
            ("mirror", self.saved.floater.is_none()),
        ] {
            f.flag(&format!("hatter1.{n}"), b);
        }
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        let n = match t.id.0 {
            121 => "exit",
            10 => "chairs",
            125 => "clock",
            35 | 45 | 128 => "extendo",
            37 => "gryphon",
            158 => "port",
            199 => "room",
            21 => "gear",
            46 => "port",
            48 => "mirror",
            67 | 761 | 762 | 763 => "extendo",
            _ => return None,
        };
        let condition = Condition::flag(&format!("hatter1.{n}"));
        Some(if matches!(t.id.0, 158 | 199 | 46 | 48) {
            Condition::All(vec![condition, Condition::flag("hatter1.free")])
        } else {
            condition
        })
    }
    fn event(&mut self, n: &str) -> Option<Events> {
        if let Some(k) = THREADS.iter().position(|t| *t == n) {
            self.press(k);
            return Some(Events::default());
        }
        match n {
            "NORETURN" => {
                if self.saved.both().is_some() {
                    self.saved.no_return.get_or_insert(self.saved.age);
                }
            }
            "Close_ClockRoom_Door" => {
                if self.saved.levers[2].is_some() {
                    self.saved.clockroom_closed.get_or_insert(self.saved.age);
                }
            }
            "Lift_Up" => self.saved.lift_up = true,
            "Lift_Down" => self.saved.lift_up = false,
            "Start_Floater1" => {
                self.saved.floater.get_or_insert(self.saved.age);
            }
            "Hatter1_Cinema1" => {
                if self.saved.levers[2].is_some() && self.saved.hare.is_none() {
                    self.begin(Kind::Hare);
                }
            }
            "Hatter1_Cinema2" => {
                if self.saved.port.is_some() && self.saved.gryphon.is_none() {
                    self.begin(Kind::Gryphon);
                }
            }
            "extendo" => {
                if self.saved.gryphon.is_some() {
                    self.saved.extendo.get_or_insert(self.saved.age);
                }
            }
            "cube_1" | "cube_2" | "cube_3" | "cube_4" => {
                if self.saved.extendo.is_some() {
                    let k = n.as_bytes()[5] as usize - b'1' as usize;
                    self.saved.cubes[k] = true;
                    if self.saved.cubes.iter().all(|b| *b) {
                        self.saved.clock.get_or_insert(self.saved.age);
                    }
                }
            }
            "mirror_cat" => {
                if !self.saved.hints[0] && self.saved.floater.is_none() {
                    self.begin(Kind::Mirror);
                }
            }
            "hatter_cat" => {
                if !self.saved.hints[1] && self.saved.port.is_some() {
                    self.begin(Kind::Hint);
                }
            }
            "chairspiders" | "spider_wall12" | "spider_wall67" | "spider_wall5"
            | "spider_wall8" | "end_spider" => {}
            _ => return None,
        }
        Some(Events::default())
    }
    fn prepare_player(&mut self, s: &mut Stats, _: &mut Player) {
        if !self.saved.initialized {
            s.full_stats();
            self.saved.initialized = true;
        }
        if s.sanity() <= 0. {
            if let Some(pull) = self.saved.pull.take() {
                // The final scene has not committed yet; keep its lever usable after recovery.
                if pull.started && pull.lever == 4 { self.saved.levers[4] = None; }
            }
        }
    }
    fn update(&mut self, w: &mut World, p: &Player, aim: Vec3, use_pressed: bool) -> Events {
        if use_pressed && self.saved.scene.is_none() {
            if let Some(k) = self.use_lever(w, p.eye(), aim) {
                self.start_pull(k, w, p);
            } else {
                self.use_door(w, p, aim);
            }
        }
        Events::default()
    }
    fn prompt(&self, w: &World, eye: Vec3, aim: Vec3) -> Option<&'static str> {
        self.use_lever(w, eye, aim)
            .map(|_| "E  Pull lever")
            .or_else(|| self.door_pick(w, eye, aim).map(|_| "E  open door"))
    }
    fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. {
            return Ok(());
        }
        let dt = dt.min(0.1);
        self.advance_motion(dt, map, w, p, fixed)?;
        self.advance_pull(dt, w, p);
        if self.saved.port.is_none()
            && self.saved.scene.is_none()
            && self.saved.levers[3].is_some_and(|t| self.saved.age >= t + 4.)
        {
            self.begin(Kind::Port);
        }
        self.advance_scene(dt, w, p)?;
        self.rebuild(map)?;
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        Ok(())
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.objects
            .iter()
            .filter(|o| o.draw)
            .map(|o| (o.model, o.pose.translation, o.pose.rotation))
            .collect()
    }
    fn colliders(&self) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| o.solid)
            .map(|o| o.collider.clone())
            .collect()
    }
    fn reflection(&self) -> (Vec<(usize, Vec3, Quat)>, Vec<usize>) {
        (
            self.objects
                .iter()
                .filter(|o| motion::fake(&o.name))
                .map(|o| (o.model, o.pose.translation, o.pose.rotation))
                .collect(),
            self.objects
                .iter()
                .filter(|o| o.name.starts_with("sink"))
                .map(|o| o.model)
                .collect(),
        )
    }
    fn reflection_actor(&self) -> Option<crate::level::ReflectionActor> {
        let s = self
            .saved
            .scene
            .as_ref()
            .filter(|s| s.kind == Kind::Mirror)?;
        Some(crate::level::ReflectionActor {
            pose: self.point("mirror_cat"),
            time: s.time,
            alpha: (s.time / 2.).min(1.)
                * s.ending
                    .map_or(1., |t| (1. - (t - 4.).max(0.) / 2.).max(0.)),
        })
    }
    fn trigger_pose(&self, n: &str, base: Vec3) -> Option<(Vec3, Quat)> {
        (n == "clock_teleport").then(|| {
            (
                base + vec3(0., 70., -360.) * self.saved.elapsed(self.saved.clock, 5.),
                Quat::IDENTITY,
            )
        })
    }
    fn scripted(&self) -> bool {
        self.saved
            .scene
            .as_ref()
            .is_some_and(|s| !matches!(s.kind, Kind::Mirror | Kind::Hint))
    }
    fn controlled(&self) -> bool { self.saved.pull.is_some() }
    fn hides_player(&self) -> bool { self.saved.pull.is_some() }
    fn blocks_weapons(&self) -> bool { self.saved.pull.is_some() }
    fn scene_id(&self) -> Option<&'static str> {
        self.saved.scene.as_ref().map(|s| s.kind.id())
    }
    fn camera(&self, world: &World) -> Option<Camera> {
        self.pull_camera(world).or_else(|| self.scene_camera())
    }
    fn fade(&self) -> Option<(Color, f32)> {
        self.scene_fade()
    }
    fn prepare_story(&self, s: &mut Story) -> bool {
        self.prepare_scene_story(s)
    }
    fn sync_story(&mut self, s: &Story) {
        self.sync_scene_story(s)
    }
    fn dialogue_complete(&mut self, n: &str) -> Events {
        if let Some(s) = &mut self.saved.scene {
            if s.kind.dialogue() == Some(n) {
                s.ending.get_or_insert(0.);
            }
        }
        Events::default()
    }
    fn skip(&mut self, _: &Bsp, _: &mut World, _: &mut Player, story: &mut Story) -> Result<bool> {
        let Some(s) = &mut self.saved.scene else {
            return Ok(false);
        };
        if let Some(n) = s.kind.dialogue() {
            story.finish_sequence(n);
        }
        s.skip.get_or_insert(0.);
        Ok(true)
    }
    fn objective(&self) -> Option<String> {
        Some(
            if self.saved.stopped {
                "Enter the Hatter's arena."
            } else if self.saved.clock.is_some() {
                "Reach the lowered clock and stop the machinery."
            } else if self.saved.extendo.is_some() {
                "Land on all four chair cushions."
            } else if self.saved.gryphon.is_some() {
                "Cross the extending bridge."
            } else if self.saved.port.is_some() {
                "Find the imprisoned Gryphon."
            } else if self.saved.hare.is_some() {
                "Pull the laboratory lever."
            } else if self.saved.levers[2].is_some() {
                "Investigate the laboratory."
            } else if self.saved.both().is_some() {
                "Use the mirror to find safe stepping stones."
            } else {
                "Pull both levers beside the great gear."
            }
            .into(),
        )
    }
    fn sound_state(
        &self,
        _: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        for (k, start) in self.saved.levers.iter().enumerate() {
            if let Some(start) = start {
                clocks.push(crate::audio::world::Clock {
                    key: ["hatter1/lever0", "hatter1/lever1", "hatter1/lever2", "hatter1/lever3", "hatter1/lever4"][k],
                    time: self.saved.age - start, period: None, origin: self.levers[k].translation,
                    cues: &[(1.05, "sound/world/machine/lever1.wav")],
                });
            }
        }
        if let Some(s) = self
            .saved
            .scene
            .as_ref()
            .filter(|s| s.kind == Kind::Stop && s.skip.is_none())
        {
            clocks.push(crate::audio::world::Clock {
                key: "hatter1/clock-stop",
                time: s.time,
                period: None,
                origin: self.point("clockmin").translation,
                cues: &[
                    (7.5, "sound/ui/controls_c.wav"),
                    (10., "sound/ui/controls_c.wav"),
                    (12.5, "sound/ui/controls_c.wav"),
                    (15., "sound/ui/controls_c.wav"),
                    (17.5, "sound/ui/controls_c.wav"),
                    (20., "sound/ui/controls_c.wav"),
                ],
            });
        }
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        let s = state::load::<Saved>(v, state::Visit { returning: false })?;
        ensure!(
            s.doors.len() == self.objects.len(),
            "Changed machinery layout"
        );
        self.saved = s;
        self.rebuild(map)
    }
    fn upgraded(&mut self) {
        self.saved.initialized = true;
    }
    fn upgrade(&self) -> crate::level::Upgrade {
        crate::level::Upgrade {
            respawn: crate::level::Respawn::Always,
            rearm: vec![
                "Hatter1_Cinema1".into(),
                "Hatter1_Cinema2".into(),
                "mirror_cat".into(),
                "hatter_cat".into(),
            ],
            ..Default::default()
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
pub static REGISTRATION: Registration = Registration {
    id: "hatter1",
    applies: |m, e| super::first_visit(m, e, "hatter1"),
    load: |a, m, _, _| Ok(Box::new(Clockwork::load(a, m)?)),
    art: Some(|a, _, _| Ok(Box::new(art::Art::load(a)?))),
    owns_submodel: |m, e| m == "hatter1" && motion::owns(e),
    owns_npc: |n, m| {
        matches!(
            n,
            "hare_actor1"
                | "fake_hare"
                | "mouse_actor1"
                | "gryphon_actor1"
                | "loco_hatter"
                | "mirror_cat"
                | "hatter_cat"
                | "da_machine"
        ) || m == "lever"
    },
    target_base: Some(8_300_000),
    story_beats: &[
        Beat::linear(
            "hatter1",
            "Hatter1_Cinema1",
            "hatter1_cinematics",
            "Hatter1_Cinema1",
            12,
        ),
        Beat::linear(
            "hatter1",
            "Hatter1_Cinema2",
            "hatter1_cinematics",
            "Hatter1_Cinema2",
            7,
        ),
        Beat::linear(
            "hatter1",
            "mirror_cat",
            "hatter1_cinematics",
            "mirror_cat",
            1,
        ),
        Beat::linear(
            "hatter1",
            "hatter_cat",
            "hatter1_cinematics",
            "hatter_cat",
            1,
        ),
    ],
    checks: &[
        Check {flag:"--hatter1-route-native-check",help:"Verify continuous Clockwork traversal with its native cast.",run:Run::Windowed(route::check)},
        Check {
            flag: "--hatter1-traversal-check",
            help: "Check lever inputs, sinks, lift riders and chair contacts.",
            run: Run::Headless(check::traversal),
        },
        Check {
            flag: "--hatter1-check",
            help: "Check Clockwork machinery, gates, saves and scene outcomes.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--hatter1-render-check",
            help: "Capture Clockwork machinery and scenes.",
            run: Run::Windowed(check::render),
        },
    ],
    save_cases: &[],
    visibility: &[],
};
