//! Icy Reception: ice collapse, rolling hazards, wall smash and the Caterpillar portal.
mod art;
mod check;
mod course;
mod course_check;
mod data;
mod route;
pub(crate) use route::drive as drive_route;
mod scene;
use super::{state, Check, Registration, Run};
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World},
    event::{Condition, Facts},
    interaction::{Events, Interactions},
    inventory::Stats,
    level::scene::{SceneRunner, SceneState},
    level::spec::{EndSpec, ExitSpec, SceneSpec, ShotSpec},
    level::{LevelArt, LevelController, TriggerInfo},
    movement::Player,
    skeletal::Transform,
    story::{BeatSpec, Story},
};
use anyhow::{ensure, Context, Result};
pub use check::stage;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;
pub const START: &str = "Garden4_StartCinema1";
pub const TALK: &str = "Garden4_Caterpillar_Cinema1";
pub const END: &str = "G4_CatEnd";
const ALICE: &[&str] = &["idle_stand", "idle_stand_shakeno", "idle_stand_rocktoes"];
const CATER: &[&str] = &[
    "idle_base",
    "idle_smoke",
    "idle_adjust",
    "talk01",
    "talk02",
    "talk03",
    "talk04",
    "talk05",
    "talk06",
    "talk07",
    "portal_smoke",
];
const EXIT: ExitSpec = ExitSpec {
    map: "centipede1",
    entrance: "centipede1_start1",
};
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    Setup,
    Talk,
    SkipSetup,
    LastLine,
    Reveal,
}
#[derive(Clone, Serialize, Deserialize)]
struct Scene {
    clock: SceneState,
    phase: Phase,
    start: f32,
    shot: usize,
    shot_start: f32,
    alice_second: Option<f32>,
    cater_second: Option<f32>,
    skipping: bool,
}
impl Scene {
    fn elapsed(&self) -> f32 {
        self.clock.time - self.start
    }
    fn phase(&mut self, p: Phase) {
        self.phase = p;
        self.start = self.clock.time;
    }
    fn shot(&mut self, k: usize) {
        if self.shot != k {
            self.shot = k;
            self.shot_start = self.clock.time;
        }
    }
    fn dialogue(&self) -> &'static str {
        if self.phase == Phase::LastLine {
            END
        } else {
            TALK
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    scene: Option<Scene>,
    complete: bool,
    enable_delay: f32,
    fade: f32,
    idle: f32,
    outro: Option<f32>,
    exit: crate::level::exit::ExitState,
    legacy_encounter: bool,
    #[serde(default)]
    course: Option<course::Course>,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        if let Some(c) = &self.course {
            c.validate()?;
        }
        state::clock("portal delay", self.enable_delay, 4.)?;
        state::clock("portal fade", self.fade, 0.5)?;
        state::clock("Caterpillar idle", self.idle, 1e6)?;
        if let Some(t) = self.outro {
            state::clock("Caterpillar final acting", t, 600.)?;
        }
        ensure!(
            !self.complete || self.outro.is_some(),
            "Missing committed Caterpillar performance"
        );
        state::clock("portal retry", self.exit.retry_time, 1.)?;
        ensure!(
            self.exit.retry_time == 0. || self.exit.committed,
            "Retry without a portal request"
        );
        ensure!(
            !self.legacy_encounter || (!self.complete && self.scene.is_none()),
            "Invalid pending legacy encounter"
        );
        ensure!(
            !self.complete || self.scene.is_none(),
            "Completed Caterpillar scene replayed"
        );
        ensure!(
            self.enable_delay == 0. || self.complete,
            "Portal delay before commitment"
        );
        ensure!(
            !self.exit.committed || (self.complete && self.enable_delay == 0.),
            "Premature portal exit"
        );
        if let Some(s) = &self.scene {
            s.clock.validate(&spec(None))?;
            state::clock("Caterpillar line cursor", s.clock.line_time, 180.)?;
            ensure!(
                !s.clock.finished && s.clock.line < 10 && s.shot < 8,
                "Invalid Caterpillar cursor"
            );
            state::clock("Caterpillar phase start", s.start, s.clock.time)?;
            state::clock("Caterpillar shot start", s.shot_start, s.clock.time)?;
            for t in [s.alice_second, s.cater_second].into_iter().flatten() {
                state::clock("Caterpillar acting restart", t, s.clock.time)?;
            }
            ensure!(
                s.skipping == matches!(s.phase, Phase::SkipSetup | Phase::LastLine)
                    || s.phase == Phase::Reveal,
                "Invalid skip phase"
            );
            ensure!(
                s.phase != Phase::LastLine || s.clock.line == 0,
                "Invalid closing dialogue index"
            );
        }
        Ok(())
    }
}
fn spec(landing: Option<Transform>) -> SceneSpec {
    SceneSpec {
        id: START,
        version: 1,
        duration: 3600.,
        shots: &[ShotSpec {
            start: 0.,
            track: "garden4",
            offset: 0.,
            hold: 3600.,
        }],
        cues: &[],
        end: EndSpec {
            landing,
            exit: None,
        },
    }
}
struct Garden {
    saved: Saved,
    data: data::Data,
    objects: Vec<course::Object>,
    alive: bool,
    legacy_course: bool,
}
impl Garden {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let data = data::Data::load(a, map)?;
        let course = Some(course::Course::new(&data.rocks));
        let mut out = Self {
            saved: Saved {
                version: 1,
                scene: None,
                complete: false,
                enable_delay: 0.,
                fade: 0.,
                idle: 0.,
                outro: None,
                exit: Default::default(),
                legacy_encounter: false,
                course,
            },
            data,
            objects: course::objects(map)?,
            alive: true,
            legacy_course: false,
        };
        out.rebuild(map)?;
        Ok(out)
    }
    fn begin(&mut self) {
        if self.saved.complete || self.saved.scene.is_some() {
            return;
        }
        self.saved.scene = Some(Scene {
            clock: SceneState::new(&spec(None)),
            phase: Phase::Setup,
            start: 0.,
            shot: 0,
            shot_start: 0.5,
            alice_second: None,
            cater_second: None,
            skipping: false,
        });
        // The camera starts after the initial fade; keep the saved start valid at time zero.
        self.saved.scene.as_mut().unwrap().shot_start = 0.;
    }
    fn ready(&self) -> bool {
        self.saved.complete && self.saved.enable_delay == 0. && !self.saved.exit.committed
    }
    fn finish(&mut self, w: &World, p: &mut Player) -> Result<()> {
        let Some(s) = &mut self.saved.scene else {
            return Ok(());
        };
        let skipped = s.skipping;
        SceneRunner {
            spec: &spec(Some(self.data.alice)),
            state: &mut s.clock,
        }
        .finish(w, p)?;
        self.saved.complete = true;
        self.saved.scene = None;
        self.saved.enable_delay = if skipped { 4. } else { 0. };
        self.saved.fade = 0.5;
        Ok(())
    }
}
impl LevelController for Garden {
    fn id(&self) -> &'static str {
        "garden4"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("garden4.portal", self.ready());
        f.flag(
            "garden4.encounter",
            !self.saved.complete && !self.scripted(),
        );
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        match t.id.0 {
            6 => Some(Condition::flag("garden4.portal")),
            49 => Some(Condition::flag("garden4.encounter")),
            _ => None,
        }
    }
    fn event(&mut self, n: &str) -> Option<Events> {
        if self.course_event(n) {
            return Some(Events::default());
        }
        if n != START {
            return None;
        }
        self.begin();
        Some(Events::default())
    }
    fn output(&mut self, e: &crate::event::Effect) -> Option<Events> {
        // The fog presentation consumes this thread; its world changes share the contact.
        if matches!(e, crate::event::Effect::Trigger(crate::entity::Id(78))) {
            self.course_mut().fog = true;
            return Some(Events::default());
        }
        if !matches!(e, crate::event::Effect::Trigger(crate::entity::Id(6))) {
            return None;
        }
        if self.ready() {
            self.saved.exit.committed = true;
            let _ = self.saved.exit.request(EXIT);
        }
        // The shared trigger dispatch emits this initial transition. Only retries belong here.
        Some(Events::default())
    }
    fn update(&mut self, _: &mut World, _: &Player, _: Vec3, _: bool) -> Events {
        Events {
            transition: self.saved.exit.request(EXIT),
            ..Default::default()
        }
    }
    fn transition_failed(&mut self, e: &(String, Option<String>)) {
        if EXIT.matches(e) {
            self.saved.exit.failed();
        }
    }
    fn scripted(&self) -> bool {
        self.saved.scene.is_some() || self.marble_time().is_some()
    }
    fn controlled(&self) -> bool {
        self.scripted()
    }
    fn allow_cheshire(&self) -> bool {
        !self.scripted()
    }
    fn scene_id(&self) -> Option<&'static str> {
        self.saved.scene.is_some().then_some(START)
    }
    fn prepare_story(&self, story: &mut Story) -> bool {
        if self.marble_time().is_some() {
            story.finish_sequence("Ice_Fall3");
            return false;
        }
        if self.course().altar.is_some() && !story.has_seen("Ice_Fall3") {
            story.trigger("Ice_Fall3");
        }
        let Some(s) = &self.saved.scene else {
            return true;
        };
        if matches!(s.phase, Phase::Talk | Phase::LastLine) {
            story.resume_scene(s.dialogue(), s.clock.line, s.clock.line_time);
            true
        } else {
            false
        }
    }
    fn sync_story(&mut self, story: &Story) {
        let Some(s) = &mut self.saved.scene else {
            return;
        };
        if !matches!(s.phase, Phase::Talk | Phase::LastLine) {
            return;
        }
        if let Some((line, time)) = story.progress(s.dialogue()) {
            s.clock.line = line;
            s.clock.line_time = time;
            if s.phase == Phase::Talk {
                let shot = match line {
                    0 => 0,
                    1 | 2 => 1,
                    3 => 2,
                    4..=6 => 3,
                    7 => 4,
                    _ => 5,
                };
                s.shot(shot);
                if line >= 3 {
                    s.alice_second.get_or_insert(s.clock.time);
                }
                if line >= 6 {
                    s.cater_second.get_or_insert(s.clock.time);
                }
            }
        }
    }
    fn dialogue_complete(&mut self, n: &str) -> Events {
        if let Some(s) = &mut self.saved.scene {
            if (s.phase == Phase::Talk && n == TALK && s.clock.line == 9)
                || (s.phase == Phase::LastLine && n == END)
            {
                s.phase(Phase::Reveal);
                self.saved.outro = Some(0.);
            }
        }
        Events::default()
    }
    fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. || !self.alive {
            return Ok(());
        }
        let dt = dt.min(0.1);
        self.course_step(dt, map, w, p, fixed)?;
        self.saved.fade = (self.saved.fade - dt).max(0.);
        self.saved.idle = (self.saved.idle + dt).min(1e6);
        if let Some(t) = &mut self.saved.outro {
            *t = (*t + dt).min(600.);
        }
        self.saved.exit.advance(dt);
        self.saved.enable_delay = (self.saved.enable_delay - dt).max(0.);
        if let Some(s) = &mut self.saved.scene {
            let line_time = s.clock.line_time;
            let mut run = SceneRunner {
                spec: &spec(None),
                state: &mut s.clock,
            };
            run.capture(p);
            run.advance(dt);
            // Speech owns this cursor. Waiting for another voice must not consume
            // the interrupted line or make it impossible to resume on the next frame.
            s.clock.line_time = line_time;
            p.cancel_climb();
            p.release_rope();
            p.velocity = Vec3::ZERO;
            p.script_motion = 1;
            if s.phase == Phase::Setup && s.elapsed() >= 0.5 {
                s.phase(Phase::Talk);
                s.shot_start = s.clock.time;
                s.clock.line_time = 0.;
            } else if s.phase == Phase::SkipSetup && s.elapsed() >= 0.5 {
                s.phase(Phase::LastLine);
                s.shot(6);
                s.clock.line_time = 0.;
            } else if s.phase == Phase::Reveal {
                if !s.skipping && s.elapsed() >= 5.5 {
                    s.shot(7);
                }
                if s.elapsed() >= if s.skipping { 5.5 } else { 14. } {
                    self.finish(w, p)?;
                }
            }
        }
        Ok(())
    }
    fn skip(&mut self, _: &Bsp, _: &mut World, _: &mut Player, story: &mut Story) -> Result<bool> {
        let Some(s) = &mut self.saved.scene else {
            return Ok(false);
        };
        if s.skipping {
            return Ok(false);
        }
        if s.phase == Phase::Reveal {
            // Closing speech and portal reveal already started. Shorten only the
            // remaining camera hold; never replay the voice or hide/restart the portal.
            s.skipping = true;
            return Ok(true);
        }
        story.finish_sequence(TALK);
        s.skipping = true;
        s.phase(Phase::SkipSetup);
        s.clock.line = 0;
        s.clock.line_time = 0.;
        Ok(true)
    }
    fn trigger_pose(&self, n: &str, base: Vec3) -> Option<(Vec3, Quat)> {
        if n != "portal_trigger" {
            return None;
        }
        let p = self.portal_pose();
        Some((
            p.translation + p.rotation * (base - self.data.portal_origin),
            p.rotation,
        ))
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        let mut poses: Vec<_> = self
            .objects
            .iter()
            .filter_map(|o| o.pose.map(|p| (o.model, p.translation, p.rotation)))
            .collect();
        if !self.portal_visible() {
            return poses;
        }
        let p = self.portal_pose();
        poses.push((2, p.translation, p.rotation));
        poses
    }
    fn colliders(&self) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| o.solid)
            .map(|o| o.collider.clone())
            .chain(
                self.data
                    .rocks
                    .iter()
                    .zip(&self.course().rocks)
                    .filter_map(|(d, s)| d.collider(s)),
            )
            .collect()
    }
    fn settled_supports(&self) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| {
                o.solid
                    && o.pose.is_some_and(|p| {
                        p.translation == o.base.translation && p.rotation == o.base.rotation
                    })
            })
            .map(|o| o.collider.clone())
            .collect()
    }
    fn prepare_player(&mut self, s: &mut Stats, _: &mut Player) {
        self.alive = s.alive();
    }
    fn combat(&mut self, c: &mut crate::level::Combat<'_>) -> crate::combat::Feedback {
        if c.dt <= 0. || self.scripted() {
            return Default::default();
        }
        crate::combat::Feedback {
            damage: std::mem::take(&mut self.course_mut().damage),
            impulse: std::mem::take(&mut self.course_mut().impulse),
            ..Default::default()
        }
    }
    fn objective(&self) -> Option<String> {
        Some(
            if self.ready() {
                "Enter the Caterpillar's portal."
            } else if self.course().fog {
                "Cross the vents and find the Caterpillar."
            } else {
                "Follow the ice path. Keep clear of falling rocks."
            }
            .into(),
        )
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        if let Some(t) = self.marble_time() {
            let (k, t) = if t < 4.5 {
                (0, (t - 0.5).max(0.))
            } else {
                (1, t - 4.5)
            };
            return Some(self.data.marble_cameras[k].camera(t));
        }
        self.scene_camera()
    }
    fn fade(&self) -> Option<(Color, f32)> {
        if let Some(t) = self.marble_time() {
            return Some((
                WHITE,
                if t < 0.5 {
                    t * 2.
                } else if t < 1. {
                    (1. - (t - 0.5) * 2.).max(0.)
                } else {
                    ((t - 6.5) * 2.).clamp(0., 1.)
                },
            ));
        }
        self.scene_fade()
    }
    fn upgraded(&mut self) {
        self.saved.complete = false;
        self.saved.exit = Default::default();
        self.legacy_course = true;
    }
    fn upgrade_triggers(&mut self, ids: &[crate::entity::Id]) {
        self.saved.legacy_encounter = ids.contains(&crate::entity::Id(49));
    }
    fn restore_position(&mut self, p: &Player, map: &Bsp) -> Result<()> {
        if self.legacy_course {
            if p.feet.y > 1600. || self.saved.scene.is_some() || self.saved.complete {
                let s = self.course_mut();
                s.wall = Some(0.);
                s.fog = true;
                s.age = s.age.max(10.);
                s.marble = Some(0.);
                s.marble_done = true;
                for t in &mut s.crushed {
                    *t = Some(0.);
                }
            }
            self.legacy_course = false;
            self.rebuild(map)?;
        }
        if self.saved.legacy_encounter {
            self.begin();
            self.saved.legacy_encounter = false;
        }
        // Portal proximity is not proof of completion in a build with the premature exit bug.
        let _ = p;
        Ok(())
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn sound_state(
        &self,
        _: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        for (k, at) in self.course().crushed.iter().enumerate() {
            if let Some(at) = at {
                clocks.push(crate::audio::world::Clock {
                    key: [
                        "garden4.crush1",
                        "garden4.crush2",
                        "garden4.crush3",
                        "garden4.crush4",
                    ][k],
                    time: (self.course().age - at) as f32,
                    period: None,
                    origin: self.data.rocks[k].nodes[0].point,
                    cues: &[(0., "sound/ambience/special/thronebreak2.wav")],
                });
            }
        }
        if let Some(at) = self.course().wall {
            clocks.push(crate::audio::world::Clock {
                key: "garden4.wall",
                time: (self.course().age - at) as f32,
                period: None,
                origin: vec3(-2624., 1536., -5200.),
                cues: &[
                    (0., "sound/ambience/special/thronebreak2.wav"),
                    (0.5, "sound/ambience/special/thronebreak3.wav"),
                ],
            });
        }
        if let Some(at) = self.course().bounce {
            clocks.push(crate::audio::world::Clock {
                key: "garden4.bounce",
                time: (self.course().age - at) as f32,
                period: None,
                origin: self
                    .course()
                    .rocks
                    .iter()
                    .filter(|r| r.started)
                    .last()
                    .map_or(Vec3::ZERO, |r| r.position),
                cues: &[(0., "sound/ambience/special/quake_step1.wav")],
            });
        }
        let time = self
            .saved
            .scene
            .as_ref()
            .map_or(self.saved.idle, |s| s.clock.time);
        let (clip, time, _) = self.acting_at("c_caterpillar", time);
        let frame = self.data.frame("c_caterpillar", clip);
        let (key, time, cues): (_, _, &'static [(f32, &'static str)]) = match clip {
            "idle_adjust" => (
                "garden4.cater.adjust",
                time,
                &[(0., "sound/character/caterpillar/idle_adjust.wav")],
            ),
            "idle_smoke" => (
                "garden4.cater.smoke",
                time - 14. * frame,
                &[(0., "sound/character/caterpillar/idle_smoke.wav")],
            ),
            "portal_smoke" => {
                clocks.push(crate::audio::world::Clock {
                    key: "garden4.cater.portal.out",
                    time: time - 78. * frame,
                    period: None,
                    origin: self.data.cater.translation,
                    cues: &[(0., "sound/character/caterpillar/portal_smoke_out.wav")],
                });
                (
                    "garden4.cater.portal.in",
                    time - 14. * frame,
                    &[(0., "sound/character/caterpillar/portal_smoke_in.wav")],
                )
            }
            _ => return,
        };
        clocks.push(crate::audio::world::Clock {
            key,
            time,
            period: None,
            origin: self.data.cater.translation,
            cues,
        });
    }
    fn quake_offset(&self) -> Vec3 {
        if self.saved.scene.is_some() {
            return Vec3::ZERO;
        }
        let s = self.course();
        let bounce = s
            .bounce
            .map_or(0., |t| (1. - (s.age - t) as f32 * 2.5).max(0.));
        let cycle = if s.marble.is_none() {
            (1. - (s.age % 20.) as f32).max(0.) * 0.4
        } else {
            0.
        };
        vec3(
            (s.age as f32 * 43.).sin(),
            (s.age as f32 * 57.).sin(),
            (s.age as f32 * 71.).cos(),
        ) * (bounce + cycle)
            * 2.
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        let mut s: Saved = state::load(v, state::Visit { returning: false })?;
        let legacy = s.course.is_none();
        if legacy {
            s.course = Some(course::Course::new(&self.data.rocks));
        }
        for (spec, r) in self
            .data
            .rocks
            .iter()
            .zip(&s.course.as_ref().unwrap().rocks)
        {
            spec.validate(r)?;
        }
        self.saved = s;
        self.legacy_course = legacy;
        self.rebuild(map)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
pub static REGISTRATION: Registration = Registration {
    id: "garden4",
    applies: |m, e| super::first_visit(m, e, "garden4"),
    load: |a, m, _, _| Ok(Box::new(Garden::load(a, m)?)),
    art: Some(|a, _, _| Ok(Box::new(art::Art::load(a)?))),
    owns_submodel: |m, e| m == "garden4" && course::owns(e),
    owns_npc: |n, m| {
        n == "caterpillar_actor1" && m == "c_caterpillar" || course::ROCKS.contains(&n)
    },
    target_base: Some(7_500_000),
    story_beats: &[
        BeatSpec::linear("garden4", "Ice_Fall3", "garden4_cinematics", "Ice_Fall3", 1),
        BeatSpec::linear("garden4", TALK, "garden4_cinematics", TALK, 10),
        BeatSpec::linear("garden4", END, "garden4_cinematics", END, 1),
    ],
    checks: &[
        Check {
            flag: "--garden4-route-check",
            help: "Walk Icy Reception from the entrance into Fungiferous Flora.",
            run: Run::Headless(route::watched),
        },
        Check {
            flag: "--garden4-skip-route-check",
            help: "Walk Icy Reception with the Caterpillar scene skipped.",
            run: Run::Headless(route::skipped),
        },
        Check {
            flag: "--garden4-check",
            help: "Verify Caterpillar dialogue and bound portal gate.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--garden4-render-check",
            help: "Render Caterpillar staging and portal fixtures.",
            run: Run::Windowed(check::render),
        },
    ],
    save_cases: &[
        super::SaveCase {
            name: "garden4-ice-fall",
            visit: "garden4$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden4-ice-collapse",
            visit: "garden4$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden4-marble",
            visit: "garden4$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden4-wall",
            visit: "garden4$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden4-talk",
            visit: "garden4$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden4-smoke",
            visit: "garden4$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden4-portal",
            visit: "garden4$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden4-skip-delay",
            visit: "garden4$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden4-ready",
            visit: "garden4$first",
            stage: None,
            behavior: None,
        },
    ],
    visibility: &[],
};
fn owner(i: &mut Interactions) -> Result<&mut Garden> {
    i.levels
        .iter_mut()
        .find_map(|s| s.ctl.downcast_mut::<Garden>())
        .context("Missing Garden4 owner")
}

/// Older scene-only saves retain their event history. Complete only hazards they
/// already crossed; otherwise a consumed wall trigger would strand that save.
pub(crate) fn restore_course_history(
    levels: &mut [crate::level::Slot],
    map: &Bsp,
    history: &[crate::entity::Id],
) -> Result<()> {
    let Some(g) = levels
        .iter_mut()
        .find_map(|s| s.ctl.downcast_mut::<Garden>())
    else {
        return Ok(());
    };
    if !g.legacy_course {
        return Ok(());
    }
    let crossed = |ids: &[usize]| {
        ids.iter()
            .any(|id| history.contains(&crate::entity::Id(*id)))
    };
    g.course_mut().age = 60.;
    for (k, done) in [
        crossed(&[251, 252]) && !g.data.easy,
        crossed(&[253]) && !g.data.easy,
        crossed(&[254, 110]),
        crossed(&[254, 110]),
        crossed(&[37, 78]),
        crossed(&[36, 78]),
        crossed(&[110, 78]),
    ]
    .into_iter()
    .enumerate()
    {
        if !done {
            continue;
        }
        let s = g.saved.course.as_mut().unwrap();
        g.data.rocks[k].seek(&mut s.rocks[k], 60.);
        s.rocks[k].visible = false;
        s.rocks[k].solid = false;
        if k < 4 {
            s.crushed[k] = Some(0.);
        }
        if k == 2 {
            s.altar = Some(0.);
        }
        if k == 4 {
            s.wall = Some(0.);
        }
        if k == 6 {
            s.marble = Some(0.);
            s.marble_done = true;
        }
    }
    g.course_mut().fog = crossed(&[78]);
    g.rebuild(map)
}
