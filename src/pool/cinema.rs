//! Reviewed Pool staging. Asset identifiers and independently expressed timelines only.
use super::*;
use crate::level::{
    scene::{SceneRunner, SceneState},
    spec::{EndSpec, ExitSpec, SceneSpec, ShotSpec},
};
use crate::{assets::Assets, cinematic::Camera, npc::Puppet, story::Story};

pub const ENTRY: &str = "Tears1_Start_Cinematic";
pub const EXIT: &str = "Tears1_End_Cinematic";
pub const ENDING: SceneSpec = SceneSpec {
    id: EXIT,
    version: 1,
    duration: 5.,
    shots: &[ShotSpec {
        start: 0.,
        track: "tears1_path3",
        offset: -0.5,
        hold: 4.5,
    }],
    cues: &[2.],
    end: EndSpec {
        landing: None,
        exit: Some(ExitSpec {
            map: "potears2",
            entrance: "potears2_start1",
        }),
    },
};
const ALICE: &[&str] = &[
    "idle_stand",
    "walk",
    "jump",
    "float",
    "idle_shrug",
    "idle_shrug_headtilt",
    "idle_shrug_shakeno",
    "idle_shrug_nodyes",
    "idle_shrug_tap",
];
const TURTLE: &[&str] = &[
    "idle",
    "crying01",
    "crying02",
    "walk",
    "run",
    "jump",
    "idle_nosewipe",
    "talk_shrug",
];
const CAST: &[&str] = &[
    "rabbit_actor1",
    "turtle_talk",
    "turtle1",
    "turtle2",
    "ant_pusher1",
    "ant_pusher2",
];
pub fn owns(name: &str) -> bool {
    CAST.contains(&name)
}
pub fn newly_supported(thread: &str) -> bool {
    matches!(
        thread,
        EXIT | "Tears1_Boulder1"
            | "Tears1_Boulder2"
            | "Tears1_Boulder3"
            | "Turtle_Encounter2"
            | "Turtle_Encounter3"
    )
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Beat {
    Entry,
    Boulder1,
    Boulder2,
    Talk,
    Depart,
    Exit,
}
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    #[serde(default)]
    pub ending: Option<SceneState>,
    #[serde(default)]
    pub arrival: Option<SceneState>,
    pub version: u8,
    pub beat: Option<Beat>,
    pub time: f32,
    pub shot: usize,
    pub shot_time: f32,
    pub line: usize,
    pub line_time: f32,
    pub done: [bool; 5],
    pub home: Option<Transform>,
    pub rocks: [Option<f32>; 4],
    pub exit_sent: bool,
    pub exit_sound: bool,
}
impl State {
    pub fn fresh() -> Self {
        Self {
            version: 3,
            ..Default::default()
        }
    }
    pub(super) fn start(&mut self, beat: Beat) {
        self.beat = Some(beat);
        self.time = 0.;
        self.shot = 0;
        self.shot_time = 0.;
        self.line = 0;
        self.line_time = 0.;
        self.home = None;
        if beat == Beat::Exit {
            self.ending = Some(SceneState::new(&ENDING));
        }
    }
    fn sync_ending(&mut self) {
        if let Some(s) = &self.ending {
            self.time = s.time;
            self.shot = s.shot;
            self.shot_time = s.shot_time;
            self.line = s.line;
            self.line_time = s.line_time;
            self.home = s.home;
            self.done[4] = s.finished;
        }
    }
    pub(super) fn migrate_ending(&mut self) {
        if self.version < 2 {
            if self.beat == Some(Beat::Exit) || self.done[4] {
                let mut s = SceneState::new(&ENDING);
                s.time = if self.done[4] { 5. } else { self.time.min(5.) };
                s.cast_time = s.time;
                s.shot_time = s.time;
                s.home = self.home;
                s.finished = self.done[4];
                s.fired = u64::from(self.exit_sound || self.done[4]);
                s.exit.committed = self.done[4];
                self.ending = Some(s);
                self.sync_ending();
            }
            self.version = 2;
        }
    }
    pub fn validate(&self, age: f32) -> Result<()> {
        ensure!(
            self.version <= 3
                && self.shot <= 4
                && self.line < 14
                && [self.time, self.shot_time, self.line_time]
                    .iter()
                    .all(|t| t.is_finite() && (0. ..=1e6).contains(t))
                && self
                    .rocks
                    .iter()
                    .flatten()
                    .all(|t| t.is_finite() && *t >= 0. && *t <= age)
                && self.home.is_none_or(|h| h.translation.is_finite()
                    && h.translation.abs().max_element() < 100_000.
                    && h.rotation.is_finite()
                    && (h.rotation.length_squared() - 1.).abs() < 0.01)
                && (!self.exit_sent || self.done[4]),
            "Invalid Pool scene save"
        );
        if self.version >= 2 {
            ensure!(
                self.beat != Some(Beat::Exit) || self.ending.is_some(),
                "Missing Pool ending state"
            );
            if let Some(s) = &self.ending {
                s.validate(&ENDING)?;
                ensure!(
                    s.finished == self.done[4] && self.beat == Some(Beat::Exit),
                    "Inconsistent Pool ending"
                );
            }
        }
        Ok(())
    }
}
pub struct Data {
    pub(super) camera_world: World,
    pub(super) arrival: super::arrival::Data,
    pub(super) tracks: BTreeMap<String, crate::fortress::spline::Spline>,
    durations: BTreeMap<String, f32>,
    speeds: BTreeMap<String, f32>,
    turtle_leaf: Path,
    last_leaf: Path,
    pub(super) rocks: Vec<crate::falling_rock::Spec>,
    frames: BTreeMap<String, f32>,
}
impl Data {
    pub fn load(assets: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut tracks = BTreeMap::new();
        for name in (1..=7)
            .map(|n| format!("tears1_path{n}"))
            .chain((1..=3).map(|n| format!("potears1_kmpx{n}")))
        {
            let track = crate::cinematic::Track::load(assets, &name)?;
            tracks.insert(
                name,
                crate::fortress::spline::Spline::camera_track(track.controls().collect()),
            );
        }
        let mut durations = BTreeMap::new();
        let mut speeds = BTreeMap::new();
        let mut frames = BTreeMap::new();
        for (model, clips) in [
            ("alice", ALICE),
            ("c_mockturtle", TURTLE),
            ("c_whiterabbit", &["idle", "run"][..]),
            ("c_armyant", super::ants::CLIPS),
        ] {
            let d = crate::skeletal::Definition::load(assets, &format!("models/{model}.tik"))?;
            let rig = crate::skeletal::Skeleton::parse(
                &assets.read(&format!("{}/{}", d.path, d.model))?,
            )?;
            for clip in clips {
                let file = d
                    .animations
                    .get(*clip)
                    .with_context(|| format!("Missing Pool clip {model}/{clip}"))?;
                let a = crate::skeletal::Animation::parse(
                    &assets.read(&format!("{}/{file}", d.path))?,
                    rig.bones.len(),
                )?;
                durations.insert(format!("{model}/{clip}"), a.duration());
                frames.insert(format!("{model}/{clip}"), a.frame_time);
                speeds.insert(
                    format!("{model}/{clip}"),
                    (a.distance * d.scale / a.duration()).max(1.),
                );
            }
        }
        let rocks = (1..=4)
            .map(|n| crate::falling_rock::Spec::load(map, &format!("dk_boulder{n}")))
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            rocks
                .iter()
                .all(|r| r.nodes.iter().all(|n| n.thread.is_none())),
            "Unreviewed Pool waypoint callback"
        );
        Ok(Self {
            arrival: super::arrival::Data::load(map)?,
            camera_world: World::from_bsp(map)?,
            tracks,
            durations,
            speeds,
            frames,
            rocks,
            turtle_leaf: Path::load(map, "turtleleafpath")?,
            last_leaf: Path::load(map, "last_turtleleafpath")?,
        })
    }
    pub(super) fn speed(&self, model: &str, clip: &str) -> f32 {
        self.speeds[&format!("{model}/{clip}")]
    }
    pub(super) fn duration(&self, model: &str, clip: &str) -> f32 {
        self.durations[&format!("{model}/{clip}")]
    }
    pub(super) fn frame_time(&self, model: &str, clip: &str) -> f32 {
        self.frames[&format!("{model}/{clip}")]
    }
    pub(super) fn push_time(&self) -> f32 {
        self.durations["c_armyant/push"]
    }
}
fn pose(p: Vec3, yaw: f32) -> Transform {
    Transform {
        translation: p,
        rotation: Quat::from_rotation_z(yaw.to_radians()),
    }
}
fn travel(a: Vec3, b: Vec3, t: f32, duration: f32, arc: f32) -> Transform {
    let f = (t / duration.max(0.001)).clamp(0., 1.);
    let d = b - a;
    let mut at = pose(a.lerp(b, f), d.y.atan2(d.x).to_degrees());
    at.translation.z += 4. * arc * f * (1. - f);
    at
}
impl Pool {
    fn walk_time(&self, model: &str, a: &str, b: &str, clip: &str) -> f32 {
        self.points[a].distance(self.points[b]) / self.cinema.speed(model, clip)
    }
    fn depart_walk(&self) -> f32 {
        self.walk_time(
            "c_mockturtle",
            "turtle_talk_stand",
            "turtle_talk_run",
            "walk",
        )
        .max(2.)
    }
    pub(super) fn depart_ride(&self) -> f32 {
        6. + self.depart_walk() + 1.5
    }
    pub fn scene_id(&self) -> Option<&'static str> {
        Some(match self.state.cinema.beat? {
            Beat::Entry => ENTRY,
            Beat::Boulder1 => "Tears1_Boulder1",
            Beat::Boulder2 => "Tears1_Boulder2",
            Beat::Talk | Beat::Depart => TALK,
            Beat::Exit => EXIT,
        })
    }
    pub fn begin(&mut self) {
        self.begin_arrival();
    }
    pub(super) fn scene_event(&mut self, n: &str) -> Option<Events> {
        if n == ENTRY {
            self.begin_arrival();
            return Some(Events::default());
        }
        let s = &mut self.state.cinema;
        let (index, beat) = match n {
            ENTRY => (0, Beat::Entry),
            "Tears1_Boulder1" => (1, Beat::Boulder1),
            "Tears1_Boulder2" => (2, Beat::Boulder2),
            TALK => (3, Beat::Talk),
            EXIT => (4, Beat::Exit),
            "Tears1_Boulder3" => {
                s.rocks[2].get_or_insert(self.state.age);
                return Some(Events::default());
            }
            _ => return None,
        };
        if s.done[index] || s.beat.is_some() {
            return Some(Events::default());
        }
        s.start(beat);
        if beat == Beat::Talk {
            self.state.talking = true;
            Some(Events {
                story: vec![TALK.into()],
                ..Default::default()
            })
        } else {
            Some(Events::default())
        }
    }
    pub fn conversation_target(&self) -> Option<Vec3> {
        if self.state.talking || self.state.talked || self.state.cinema.beat.is_some() {
            return None;
        }
        self.actor("turtle_talk")
            .map(|(_, _, pose)| pose.translation + Vec3::Z * 32.)
    }
    pub fn sync_story(&mut self, story: &Story) {
        let s = &mut self.state.cinema;
        if s.beat != Some(Beat::Talk) {
            return;
        }
        if let Some((line, time)) = story.progress(TALK) {
            s.line = line;
            s.line_time = time;
            let shot = match line {
                0..=2 => 0,
                3..=7 => 1,
                8 => 2,
                9 => 3,
                _ => 4,
            };
            if shot != s.shot {
                s.shot = shot;
                s.shot_time = 0.;
            }
        }
    }
    fn finish_scene(&mut self, world: &World, player: &mut Player) -> Result<()> {
        let Some(beat) = self.state.cinema.beat else {
            return Ok(());
        };
        if beat == Beat::Entry {
            return self.finish_arrival(world, player);
        }
        if beat == Beat::Exit {
            let s = &mut self.state.cinema;
            SceneRunner {
                spec: &ENDING,
                state: s.ending.as_mut().unwrap(),
            }
            .finish(world, player)?;
            s.sync_ending();
            player.velocity = Vec3::ZERO;
            player.cancel_climb();
            player.release_rope();
            player.script_motion = 1;
            return Ok(());
        }
        let index = match beat {
            Beat::Entry => 0,
            Beat::Boulder1 => 1,
            Beat::Boulder2 => 2,
            Beat::Talk | Beat::Depart => 3,
            Beat::Exit => 4,
        };
        let landing = match beat {
            Beat::Entry => pose(self.points["alice_posx1"], 0.),
            Beat::Talk | Beat::Depart => pose(self.points["alice_talk1_spot"], 270.),
            Beat::Boulder2 => self.boulder_alice(),
            _ => self
                .state
                .cinema
                .home
                .unwrap_or(pose(player.feet, player.script_facing.to_degrees())),
        };
        // The jump exit remains held until update emits its one transition.
        if beat != Beat::Exit {
            crate::cinematic::land_player(player, world, landing)?;
        }
        if index == 3 {
            self.state.talking = true;
            self.state.talked = true;
            self.state.drops[0].get_or_insert(self.state.age);
        }
        if index == 0 {
            self.state.cinema.rocks[3].get_or_insert(self.state.age);
        }
        if (1..=2).contains(&index) {
            self.finish_boulder(index - 1);
        }
        self.state.cinema.done[index] = true;
        if beat != Beat::Exit {
            self.state.cinema.beat = None;
        }
        Ok(())
    }
    pub fn skip(&mut self, world: &World, player: &mut Player, story: &mut Story) -> Result<bool> {
        let Some(id) = self.scene_id() else {
            return Ok(false);
        };
        if self.state.cinema.done[4] {
            return Ok(false);
        }
        self.finish_scene(world, player)?;
        story.finish_sequence(id);
        Ok(true)
    }
    pub(super) fn advance_cinema(
        &mut self,
        dt: f32,
        world: &World,
        player: &mut Player,
    ) -> Result<()> {
        if self.state.cinema.beat == Some(Beat::Entry) {
            return self.advance_arrival(dt, world, player);
        }
        let walk = self.depart_ride();
        let s = &mut self.state.cinema;
        let Some(beat) = s.beat else {
            return Ok(());
        };
        if beat == Beat::Exit {
            let mut runner = SceneRunner {
                spec: &ENDING,
                state: s.ending.as_mut().unwrap(),
            };
            runner.capture(player);
            if runner.advance(dt) {
                runner.finish(world, player)?;
            }
            s.sync_ending();
            player.feet = s.home.unwrap().translation;
            player.velocity = Vec3::ZERO;
            player.cancel_climb();
            player.release_rope();
            player.script_motion = 1;
            return Ok(());
        }
        let home = *s
            .home
            .get_or_insert(pose(player.feet, player.script_facing.to_degrees()));
        s.time += dt;
        s.shot_time += dt;
        player.velocity = Vec3::ZERO;
        player.cancel_climb();
        player.release_rope();
        player.feet = home.translation;
        player.script_motion = 1;
        match beat {
            Beat::Entry if s.time >= 4. => {
                s.rocks[3].get_or_insert(self.state.age);
            }
            Beat::Boulder1 | Beat::Boulder2 if s.time >= 1. => {
                s.rocks[usize::from(beat == Beat::Boulder2)]
                    .get_or_insert((self.state.age - (s.time - 1.)).max(0.));
            }
            Beat::Depart if s.time >= walk => {
                self.state.drops[0].get_or_insert(self.state.age);
            }
            _ => {}
        }
        let duration = match beat {
            Beat::Entry => {
                8. + self.points["alice_posx1"].distance(home.translation)
                    / self.cinema.speed("alice", "walk")
            }
            Beat::Boulder1 => 0.5 + self.cinema.push_time() + 3.5,
            Beat::Boulder2 => 0.5 + self.cinema.push_time() + 2.5,
            Beat::Depart => walk + 7.5,
            Beat::Exit => 5.,
            Beat::Talk => f32::MAX,
        };
        if s.time >= duration {
            self.finish_scene(world, player)?;
        }
        Ok(())
    }
    pub(super) fn rock_pose(&self, index: usize) -> Option<(Transform, f32)> {
        let rock = &self.state.boulders.as_ref()?.rocks[index];
        rock.visible.then_some((
            Transform {
                translation: rock.position,
                rotation: rock.rotation,
            },
            rock.elapsed,
        ))
    }
    pub fn scene_update(&mut self) -> Events {
        let s = &mut self.state.cinema;
        let mut e = Events::default();
        if let Some(b) = &mut self.state.boulders {
            e.damage = std::mem::take(&mut b.damage);
            e.sound = b.sounds.pop();
        }
        if let Some(ending) = &mut s.ending {
            if (SceneRunner {
                spec: &ENDING,
                state: ending,
            })
            .cue(0)
            {
                s.exit_sound = true;
                e.sound = Some("sound/character/alice/death_fall.wav".into());
            }
            e.transition = ending.exit.request(ENDING.end.exit.unwrap());
            s.exit_sent |= e.transition.is_some();
        }
        e
    }
    pub fn camera(&self) -> Option<Camera> {
        let s = &self.state.cinema;
        let (track, time) = match s.beat? {
            Beat::Entry => ("tears1_path4", s.time),
            Beat::Boulder1 => ("tears1_path5", (s.time - 0.5).max(0.)),
            Beat::Boulder2 => ("tears1_path7", (s.time - 0.5).max(0.)),
            Beat::Talk => (
                [
                    "tears1_path1",
                    "tears1_path2",
                    "potears1_kmpx2",
                    "potears1_kmpx1",
                    "potears1_kmpx3",
                ][s.shot],
                s.shot_time,
            ),
            Beat::Depart if (2. ..6.).contains(&s.time) => {
                return Some(Camera::look(
                    self.points["dialog_watchx"],
                    self.talk_leaf().translation,
                ));
            }
            Beat::Depart if s.time >= 6. => ("tears1_path6", s.time - 6.),
            Beat::Depart => ("potears1_kmpx3", s.shot_time),
            Beat::Exit => SceneRunner::track_time(&ENDING, s.ending.as_ref().unwrap()),
        };
        let camera = self.cinema.tracks[track].camera(time);
        // The first close-up's authored eye intersects the port's tree/terrain
        // hull. Keep the shot on the visible side using a swept near-plane box.
        // This is a port clearance correction, not a native camera offset.
        if s.beat == Some(Beat::Talk) {
            let focus = (self.points["turtle_talk_stand"] + self.points["alice_talk1_spot"]) * 0.5
                + Vec3::Z * 104.;
            let trace = self
                .cinema
                .camera_world
                .sweep_geometry(focus, camera.eye, Vec3::splat(6.));
            if !trace.start_solid && trace.fraction < 1. {
                return Some(Camera::look(
                    focus.lerp(camera.eye, trace.fraction) + trace.normal * 2.,
                    focus,
                ));
            }
        }
        Some(camera)
    }
    pub fn fade(&self) -> (Color, f32) {
        let s = &self.state.cinema;
        let Some(b) = s.beat else {
            return (
                WHITE,
                self.state
                    .boulders
                    .as_ref()
                    .map_or(0., |b| ((b.fade_until - self.state.age) * 2.).clamp(0., 1.)),
            );
        };
        if b == Beat::Entry {
            return (BLACK, (1. - s.time / 2.).clamp(0., 1.));
        }
        if b == Beat::Depart {
            return (WHITE, 0.);
        }
        let mut alpha = (1. - (s.time - 0.5).abs() * 2.).clamp(0., 1.);
        if matches!(b, Beat::Boulder1 | Beat::Boulder2) {
            let duration = self.cinema.push_time() + if b == Beat::Boulder1 { 4. } else { 3. };
            alpha = alpha.max(((s.time - duration + 0.5) * 2.).clamp(0., 1.));
        }
        (WHITE, alpha)
    }
    fn talk_leaf(&self) -> Transform {
        let t = if self.state.cinema.beat == Some(Beat::Depart) {
            self.state.cinema.time
        } else {
            0.
        };
        if t >= self.depart_ride() {
            let (p, r) = self.cinema.turtle_leaf.pose(t - self.depart_ride());
            Transform {
                translation: p + vec3(0.2, -0.78, -8.85),
                rotation: r,
            }
        } else {
            pose(
                self.points["turtleleafmdl"] + Vec3::Z * (2048. * (1. - t / 8.).clamp(0., 1.)),
                -85.,
            )
        }
    }
    pub(super) fn actor(&self, name: &str) -> Option<(&'static str, f32, Transform)> {
        let s = &self.state.cinema;
        let t = s
            .ending
            .as_ref()
            .filter(|_| s.beat == Some(Beat::Exit))
            .map_or(s.time, |e| e.cast_time);
        match name {
            "alice" => {
                let home = s.home?;
                Some(match s.beat? {
                    Beat::Entry => return Some(self.arrival_alice()),
                    Beat::Boulder1 => ("idle_stand", t, home),
                    Beat::Boulder2 => (
                        "idle_stand",
                        t,
                        if t < 0.5 { home } else { self.boulder_alice() },
                    ),
                    Beat::Talk | Beat::Depart => (
                        [
                            "idle_shrug_headtilt",
                            "idle_shrug",
                            "idle_shrug_shakeno",
                            "idle_shrug_nodyes",
                            "idle_shrug_tap",
                        ][s.line % 5],
                        s.line_time,
                        pose(self.points["alice_talk1_spot"], 270.),
                    ),
                    Beat::Exit => (
                        if t < 1. { "float" } else { "jump" },
                        (t - 1.).max(0.),
                        travel(
                            self.points["alice_jumptoend"],
                            self.points["alice_end_jump"]
                                - Vec3::Z * (350. * (t - 2.).max(0.).powi(2)),
                            (t - 1.).max(0.),
                            1.,
                            64.,
                        ),
                    ),
                })
            }
            "rabbit_actor1" => (s.beat == Some(Beat::Entry)).then(|| self.arrival_rabbit()),
            "turtle_talk" => {
                if s.done[3] {
                    return None;
                }
                if s.beat == Some(Beat::Depart) && t >= 6. {
                    let walk = self.depart_walk();
                    if t < 6. + walk {
                        return Some((
                            "walk",
                            t - 6.,
                            travel(
                                self.points["turtle_talk_stand"],
                                self.points["turtle_talk_run"],
                                t - 6.,
                                walk,
                                0.,
                            ),
                        ));
                    }
                    if t < 7. + walk {
                        return Some((
                            "jump",
                            t - 6. - walk,
                            travel(
                                self.points["turtle_talk_run"],
                                self.points["turtleleafjump"],
                                t - 6. - walk,
                                1.,
                                80.,
                            ),
                        ));
                    }
                    return None; // attached to the animated leaf tag by Art
                }
                Some((
                    if s.beat == Some(Beat::Talk) {
                        ["crying01", "idle_nosewipe", "talk_shrug", "idle"][s.line % 4]
                    } else {
                        "crying02"
                    },
                    if s.beat == Some(Beat::Talk) {
                        s.line_time
                    } else {
                        self.state.age
                    },
                    pose(
                        self.points[if matches!(s.beat, Some(Beat::Talk | Beat::Depart)) {
                            "turtle_talk_stand"
                        } else {
                            "turtle_talk"
                        }],
                        90.,
                    ),
                ))
            }
            "turtle1" if self.state.turtle[0].is_none() => {
                Some(("idle", self.state.age, pose(self.points["turtle1"], 180.)))
            }
            "turtle2" => {
                if let Some(start) = self.state.turtle[2] {
                    let dt = self.state.age - start;
                    let walk = self.walk_time(
                        "c_mockturtle",
                        "turtle2_startjump",
                        "turtle_lastpos1",
                        "walk",
                    );
                    if dt < walk {
                        return Some((
                            "walk",
                            dt,
                            travel(
                                self.points["turtle2_startjump"],
                                self.points["turtle_lastpos1"],
                                dt,
                                walk,
                                0.,
                            ),
                        ));
                    }
                    if dt < walk + 1. {
                        return Some((
                            "jump",
                            dt - walk,
                            travel(
                                self.points["turtle_lastpos1"],
                                self.points["last_turtlejump_pos"],
                                dt - walk,
                                1.,
                                80.,
                            ),
                        ));
                    }
                    return None;
                }
                if let Some(start) = self.state.turtle[1] {
                    let dt = self.state.age - start - 2.;
                    let duration =
                        self.walk_time("c_mockturtle", "turtle2_doit", "turtle2run", "run");
                    if dt >= duration {
                        return Some((
                            "idle",
                            dt - duration,
                            pose(self.points["turtle2_startjump"], 225.),
                        ));
                    }
                    return Some((
                        if dt < 0. { "idle" } else { "run" },
                        dt.max(0.),
                        travel(
                            self.points["turtle2_doit"],
                            self.points["turtle2run"],
                            dt,
                            duration,
                            0.,
                        ),
                    ));
                }
                Some((
                    "idle",
                    self.state.age,
                    pose(self.points["turtle2_doit"], 225.),
                ))
            }
            "ant_pusher1" | "ant_pusher2" => {
                let index = usize::from(name == "ant_pusher2");
                if let Some(ant) = self
                    .state
                    .boulders
                    .as_ref()
                    .map(|b| &b.ants[index])
                    .filter(|a| a.enabled)
                {
                    return Some((ant.clip(), ant.time, pose(ant.feet, ant.yaw.to_degrees())));
                }
                let pushing = s.beat
                    == Some(if index == 0 {
                        Beat::Boulder1
                    } else {
                        Beat::Boulder2
                    })
                    && t >= 0.5
                    && t < 0.5 + self.cinema.push_time();
                Some((
                    if pushing { "push" } else { "idle" },
                    if pushing { t - 0.5 } else { self.state.age },
                    pose(
                        self.points[if index == 0 { "ant_pos1" } else { name }],
                        if index == 0 { 90. } else { 180. },
                    ),
                ))
            }
            _ => None,
        }
    }
}
pub struct Art {
    pub alice: Puppet,
    turtle: Puppet,
    rabbit: Puppet,
    ant: Puppet,
    leaf: crate::weapons::Prop,
    rock: crate::weapons::Prop,
    material: crate::character::SkinMaterial,
}
impl Art {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(assets)?;
        let mut turtle = Puppet::load(assets, "c_mockturtle", TURTLE, &specs)?;
        turtle.show_attachments(false);
        Ok(Self {
            alice: Puppet::load(assets, "alice", ALICE, &specs)?,
            turtle,
            rabbit: Puppet::load(assets, "c_whiterabbit", &["idle", "run"], &specs)?,
            ant: Puppet::load(assets, "c_armyant", super::ants::CLIPS, &specs)?,
            leaf: crate::weapons::Prop::load_animation(assets, "leaf_ride", "idle", &specs)?,
            rock: crate::weapons::Prop::load_animation(assets, "boulder", "idle", &specs)?,
            material: crate::character::skin_material()?,
        })
    }
    pub fn story_pose(&mut self, story: &Story) {
        self.alice
            .mouth(story.mouth(&["fakeplayer", "alice", "player"]));
        self.turtle.mouth(story.mouth(&["turtle_talk"]));
    }
    fn leaf(&mut self, at: Transform, turtle: bool, age: f32, bright: bool) {
        self.material.bind();
        self.leaf.draw_frame(at, 1.3, bright, age, true);
        gl_use_default_material();
        if turtle {
            let at = Transform {
                translation: self.leaf.point(at, "tag_turtle", 1.3),
                rotation: at.rotation,
            };
            self.turtle.draw("idle", age, true, at, 1., bright);
        }
    }
    pub fn draw(
        &mut self,
        pool: &Pool,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        for p in [
            &mut self.alice,
            &mut self.turtle,
            &mut self.rabbit,
            &mut self.ant,
        ] {
            p.atmosphere(atmosphere, camera);
        }
        self.material.atmosphere(atmosphere, camera);
        for name in [
            "alice",
            "rabbit_actor1",
            "turtle_talk",
            "turtle1",
            "turtle2",
            "ant_pusher1",
            "ant_pusher2",
        ] {
            if let Some((clip, time, at)) = pool.actor(name) {
                let p = match name {
                    "alice" => &mut self.alice,
                    "rabbit_actor1" => &mut self.rabbit,
                    "ant_pusher1" | "ant_pusher2" => &mut self.ant,
                    _ => &mut self.turtle,
                };
                p.draw(
                    clip,
                    if clip == "death_frozen" {
                        time.min(pool.cinema.frame_time("c_armyant", clip) * 5.)
                    } else {
                        time
                    },
                    !matches!(clip, "push" | "jump")
                        && !clip.starts_with("death")
                        && !clip.starts_with("attack")
                        && !clip.starts_with("pain"),
                    at,
                    if matches!(name, "ant_pusher1" | "ant_pusher2") {
                        pool.state
                            .boulders
                            .as_ref()
                            .map(|b| {
                                b.ants[usize::from(name == "ant_pusher2")]
                                    .visual_scale(&pool.cinema)
                            })
                            .unwrap_or(1.)
                    } else {
                        1.
                    },
                    bright,
                );
            }
        }
        let s = &pool.state;
        let c = &s.cinema;
        if !c.done[3] {
            self.leaf(
                pool.talk_leaf(),
                c.beat == Some(Beat::Depart) && c.time >= pool.depart_ride() - 0.5,
                s.age,
                bright,
            );
        }
        if let Some(start) = s.turtle[0] {
            let dt = (s.age - start - 0.5).max(0.);
            if dt < pool.turtle_path.nodes[pool.turtle_path.nodes.len() - 2].time {
                let (p, r) = pool.turtle_path.pose(dt);
                self.leaf(
                    Transform {
                        translation: p,
                        rotation: r,
                    },
                    true,
                    s.age,
                    bright,
                );
            }
        }
        let attach = pool.walk_time(
            "c_mockturtle",
            "turtle2_startjump",
            "turtle_lastpos1",
            "walk",
        ) + 1.;
        let time = s.turtle[2].map(|start| s.age - start);
        let (p, r) = pool
            .cinema
            .last_leaf
            .pose(time.map_or(0., |t| (t - attach - 0.5).max(0.)));
        self.leaf(
            Transform {
                translation: p,
                rotation: r,
            },
            time.is_some_and(|t| t >= attach),
            s.age,
            bright,
        );
        self.material.bind();
        for (index, rock) in pool.cinema.rocks.iter().enumerate() {
            let Some((at, _)) = pool.rock_pose(index) else {
                continue;
            };
            self.rock.draw_frame(at, rock.scale, bright, s.age, true);
        }
        gl_use_default_material();
        if let Some(b) = &pool.state.boulders {
            for ant in &b.ants {
                for bullet in &ant.shots {
                    draw_line_3d(
                        bullet.at,
                        bullet.at - Vec3::Z * 1.5,
                        Color::new(0.6, 0.45, 0.2, 1.),
                    );
                }
            }
        }
    }
}

impl crate::ant::Timing for Data {
    fn duration(&self, model: &str, clip: &str) -> f32 {
        self.duration(model, clip)
    }
    fn frame(&self, model: &str, clip: &str) -> f32 {
        self.frame_time(model, clip)
    }
    fn speed(&self, model: &str, clip: &str) -> f32 {
        self.speed(model, clip)
    }
}
