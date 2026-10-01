//! Herbaceous Border's scenes, bridges and scripted encounters.
mod art;
mod check;
mod data;
mod encounters;
mod motion;
mod route;
pub(crate) use route::drive as drive_route;
mod scene;
use super::{state, Check, Registration, Run, SaveCase};
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World},
    event::{Condition, Facts},
    interaction::{Events, Interactions},
    level::scene::{SceneRunner, SceneState},
    level::spec::{EndSpec, SceneSpec, ShotSpec},
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
const BASE: usize = 7_300_000;
const INTRO: &str = "Garden2_Start_Cinema2";
const KNEEL: &str = "Garden2_Squish_Kneel";
const FINAL: &str = "Garden2_Cat_End";
const BEATS: &[BeatSpec] = &[
    BeatSpec::linear("garden2", INTRO, "garden2_cinematics", INTRO, 1),
    BeatSpec::linear("garden2", KNEEL, "garden2_cinematics", KNEEL, 2),
    BeatSpec::linear("garden2", FINAL, "garden2_cinematics", FINAL, 1),
];
const ALICE: &[&str] = &[
    "idle",
    "run",
    "idle_base_02",
    "idle_base_02_kneel",
    "kneel_idle",
    "kneel_shakeno",
    "kneel_2_weep",
    "weep_sobbing",
    "weep_2_kneel",
    "kneel_2_base_02",
];
const RABBIT: &[&str] = &[
    "run",
    "i_calm_l",
    "i_calm_front_2_right",
    "i_calm_eartwitch_l",
];
const HATTER: &[&str] = &["ready", "walk", "jump", "stomp_yo_ass"];
const CAT: &[&str] = &[
    "idle",
    "sit_idle1",
    "sit_talk1",
    "sit_talk2",
    "sit_smile_open",
    "sit_smile_shut",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Kind {
    Arrival,
    North,
    South,
    Second,
    Cat,
}
impl Kind {
    fn id(self) -> &'static str {
        match self {
            Self::Arrival => "Garden2_Start",
            Self::North => "Collapse_Bridge1",
            Self::South => "Collapse_Bridge2",
            Self::Second => "Collapse_Second_Bridge1",
            Self::Cat => FINAL,
        }
    }
    fn bridge(self) -> bool {
        matches!(self, Self::North | Self::South | Self::Second)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    Approach,
    Talk,
    Run,
    Squish,
    Kneel,
    Outro,
}
#[derive(Clone, Serialize, Deserialize)]
struct Scene {
    kind: Kind,
    phase: Phase,
    start: f32,
    clock: SceneState,
    skipping: bool,
    kneel_at: Option<f32>,
}
impl Scene {
    fn elapsed(&self) -> f32 {
        self.clock.time - self.start
    }
    fn phase(&mut self, p: Phase) {
        self.phase = p;
        self.start = self.clock.time;
        if p == Phase::Kneel {
            self.kneel_at = Some(self.clock.time);
        }
    }
    fn dialogue(&self) -> Option<&'static str> {
        match (self.kind, self.phase) {
            (Kind::Arrival, Phase::Talk) => Some(INTRO),
            (Kind::Arrival, Phase::Kneel) => Some(KNEEL),
            (Kind::Cat, Phase::Talk) => Some(FINAL),
            _ => None,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    arrived: bool,
    bridge1: bool,
    bridge2: bool,
    cat: bool,
    legacy: bool,
    fog_pending: bool,
    scene: Option<Scene>,
    fade: f32,
    ants: Vec<crate::ant::Ant>,
    ladies: Vec<encounters::Lady>,
    #[serde(default)]
    under_ambush: Option<f32>,
    #[serde(default)]
    fulcrum: Vec2,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        state::clock("garden2 fade", self.fade, 1.5)?;
        ensure!(
            self.fulcrum.is_finite() && self.fulcrum.abs().max_element() <= 12.,
            "Invalid fulcrum angle"
        );
        if let Some(t) = self.under_ambush {
            state::clock("garden2 underworld ambush", t, 8.)?;
        }
        for a in &self.ants {
            a.validate()?;
        }
        for l in &self.ladies {
            if let Some(t) = l.delay {
                state::clock("garden2 spawn delay", t, 2.1)?;
            }
        }
        if let Some(s) = &self.scene {
            s.clock.validate(&spec(s.kind, None))?;
            state::clock("garden2 phase", s.start, s.clock.time)?;
            ensure!(
                !s.clock.finished && s.clock.line < 2,
                "Invalid Herbaceous scene clock"
            );
            ensure!(
                !(s.kind == Kind::Arrival && self.arrived)
                    && !(s.kind == Kind::Cat && self.cat)
                    && !(s.kind == Kind::Second && self.bridge2)
                    && !(matches!(s.kind, Kind::North | Kind::South) && self.bridge1),
                "Completed scene replayed"
            );
            ensure!(!s.skipping || s.phase == Phase::Outro, "Invalid skip phase");
            ensure!(
                s.kind == Kind::Arrival
                    || s.kind == Kind::Cat
                        && matches!(s.phase, Phase::Approach | Phase::Talk | Phase::Outro)
                    || s.kind.bridge() && matches!(s.phase, Phase::Approach | Phase::Outro),
                "Invalid scene phase for kind"
            );
            if let Some(t) = s.kneel_at {
                state::clock("garden2 kneeling", t, s.clock.time)?;
                ensure!(
                    s.kind == Kind::Arrival && matches!(s.phase, Phase::Kneel | Phase::Outro),
                    "Invalid kneeling clock"
                );
            }
            ensure!(
                s.phase != Phase::Kneel || s.kneel_at.is_some(),
                "Missing kneeling clock"
            );
        }
        Ok(())
    }
}
fn spec(kind: Kind, landing: Option<Transform>) -> SceneSpec {
    SceneSpec {
        id: kind.id(),
        version: 1,
        duration: 3600.,
        shots: &[ShotSpec {
            start: 0.,
            track: "garden2",
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
    objects: Vec<motion::Object>,
}
impl Garden {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let (ants, ladies) = encounters::load(map)?;
        let mut o = Self {
            saved: Saved {
                version: 1,
                arrived: false,
                bridge1: false,
                bridge2: false,
                cat: false,
                legacy: false,
                fog_pending: false,
                scene: None,
                fade: 0.,
                ants,
                ladies,
                under_ambush: None,
                fulcrum: Vec2::ZERO,
            },
            data: data::Data::load(a, map)?,
            objects: motion::load(map)?,
        };
        o.data.world.set_dynamic(o.colliders());
        o.begin(Kind::Arrival);
        Ok(o)
    }
    fn begin(&mut self, kind: Kind) {
        if self.saved.scene.is_none() {
            self.saved.scene = Some(Scene {
                kind,
                phase: Phase::Approach,
                start: 0.,
                clock: SceneState::new(&spec(kind, None)),
                skipping: false,
                kneel_at: None,
            });
        }
    }
    fn finish(
        &mut self,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        let Some(s) = self.saved.scene.as_ref() else {
            return Ok(());
        };
        let kind = s.kind;
        // Validate against the completed geometry before committing its removal.
        let mut next = self.saved.clone();
        match kind {
            Kind::Arrival => next.arrived = true,
            Kind::North | Kind::South => next.bridge1 = true,
            Kind::Second => next.bridge2 = true,
            Kind::Cat => next.cat = true,
        }
        let solids = self
            .objects
            .iter()
            .filter_map(|o| motion::desired(o, &next, self.data.collapse()).map(|p| (o, p)))
            .map(|(o, p)| Collider::model(map, o.model, p.translation, p.rotation, true))
            .collect::<Result<Vec<_>>>()?;
        w.set_dynamic(fixed.iter().cloned().chain(solids).collect());
        let landing = match kind {
            Kind::Arrival => Some(self.data.points["alice_pos_squish2"]),
            Kind::Second => Some(self.data.points["fakeplayer_bridge_pos2"]),
            _ => None,
        };
        let landing = landing
            .map(|mut at| -> Result<Transform> {
                at.translation = w
                    .actor_footing(
                        at.translation,
                        crate::collision::PLAYER_CENTER,
                        crate::collision::PLAYER_HALF,
                        96.,
                    )
                    .context("Herbaceous endpoint has no support")?;
                Ok(at)
            })
            .transpose()?;
        let scene = self.saved.scene.as_mut().unwrap();
        let result = if landing.is_some() {
            SceneRunner {
                spec: &spec(kind, landing),
                state: &mut scene.clock,
            }
            .finish(w, p)
        } else {
            SceneRunner {
                spec: &spec(kind, None),
                state: &mut scene.clock,
            }
            .finish_at_home(w, p)
        };
        if let Err(e) = result {
            w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
            return Err(e);
        }
        self.saved.fog_pending = kind.bridge();
        self.saved.arrived = next.arrived;
        self.saved.bridge1 = next.bridge1;
        self.saved.bridge2 = next.bridge2;
        self.saved.cat = next.cat;
        match kind {
            Kind::Arrival => self.spawn_ladies(1),
            Kind::Second => self.activate_named(map, "t131"),
            _ => (),
        }
        self.saved.scene = None;
        self.saved.fade = if kind == Kind::Arrival { 1.5 } else { 0.5 };
        self.rebuild(map)?;
        p.script_motion = 0;
        Ok(())
    }
}
impl LevelController for Garden {
    fn id(&self) -> &'static str {
        "garden2"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        for (n, b) in [
            ("bridge1", !self.saved.bridge1),
            ("bridge2", !self.saved.bridge2),
            ("cat", !self.saved.cat),
        ] {
            f.flag(
                &format!("garden2.{n}"),
                b && self.saved.arrived && !self.scripted(),
            );
        }
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        if t.id.0 == 673 {
            return Some(Condition::flag("garden2.bridge2"));
        }
        let n = match t.thread {
            "Collapse_Bridge1" | "Collapse_Bridge2" => "garden2.bridge1",
            "Collapse_Second_Bridge1" => "garden2.bridge2",
            FINAL => "garden2.cat",
            _ => return None,
        };
        Some(Condition::flag(n))
    }
    fn event(&mut self, n: &str) -> Option<Events> {
        let k = match n {
            "Ant_Ambush2" => {
                if self.saved.under_ambush.is_none() {
                    self.saved.under_ambush = Some(0.);
                    if let Some(a) = self.saved.ants.iter_mut().find(|a| a.id == 139) {
                        a.enabled = true;
                        a.script_wait = true;
                    }
                }
                None
            }
            "Collapse_Bridge1" if !self.saved.bridge1 => Some(Kind::North),
            "Collapse_Bridge2" if !self.saved.bridge1 => Some(Kind::South),
            "Collapse_Second_Bridge1" if !self.saved.bridge2 => Some(Kind::Second),
            FINAL if !self.saved.cat => Some(Kind::Cat),
            "Spawn_Lady1" => {
                self.spawn_ladies(1);
                None
            }
            "Spawn_Lady2" => {
                self.spawn_ladies(2);
                None
            }
            "Spawn_Lady3" => {
                self.spawn_ladies(3);
                None
            }
            "Spawn_Lady4" => {
                self.spawn_ladies(4);
                None
            }
            "Garden2_Start"
            | "Collapse_Bridge1"
            | "Collapse_Bridge2"
            | "Collapse_Second_Bridge1"
            | FINAL => None,
            _ => return None,
        };
        if let Some(k) = k {
            if self.saved.arrived {
                self.begin(k);
            }
        }
        Some(Events::default())
    }
    fn upgraded(&mut self) {
        self.saved.arrived = true;
        self.saved.scene = None;
        self.saved.legacy = true;
    }
    fn upgrade_triggers(&mut self, ids: &[crate::entity::Id]) {
        self.saved.bridge1 = ids.iter().any(|id| matches!(id.0, 129 | 130));
        self.saved.bridge2 = ids.iter().any(|id| matches!(id.0, 114 | 673));
        self.saved.cat = ids.iter().any(|id| matches!(id.0, 52 | 53));
        for o in &mut self.objects {
            if (o.group == 1 && self.saved.bridge1) || (o.group == 2 && self.saved.bridge2) {
                o.pose = None;
            }
        }
        if self.saved.bridge2 {
            for a in &mut self.saved.ants {
                if matches!(a.id, 84 | 85) {
                    a.enabled = true;
                }
            }
        }
    }
    fn restore_position(&mut self, p: &Player, map: &Bsp) -> Result<()> {
        if self.saved.legacy {
            // Only old controller-less visits use this conservative route classification.
            self.saved.bridge1 |= p.feet.z < -900. && p.feet.x < 2400.;
            self.saved.bridge2 |= p.feet.x < 1000. && p.feet.z < -900.;
            self.saved.legacy = false;
            self.rebuild(map)?;
        }
        Ok(())
    }
    fn scripted(&self) -> bool {
        self.saved.scene.is_some()
    }
    fn controlled(&self) -> bool {
        self.scripted()
    }
    fn allow_cheshire(&self) -> bool {
        !self.scripted()
    }
    fn scene_id(&self) -> Option<&'static str> {
        self.saved
            .scene
            .as_ref()
            .filter(|s| !s.skipping)
            .map(|s| s.kind.id())
    }
    fn entry_story(&mut self, _: &mut Story) -> bool {
        true
    }
    fn prepare_story(&self, story: &mut Story) -> bool {
        let Some(s) = &self.saved.scene else {
            return true;
        };
        let Some(n) = s.dialogue() else { return false };
        if s.phase == Phase::Kneel && s.elapsed() < 3. {
            return false;
        }
        if !story.busy() && !story.has_seen(n) {
            story.trigger(n);
        }
        true
    }
    fn sync_story(&mut self, story: &Story) {
        if let Some(s) = &mut self.saved.scene {
            if let Some((line, time)) = s.dialogue().and_then(|n| story.progress(n)) {
                s.clock.line = line;
                s.clock.line_time = time;
            }
        }
    }
    fn dialogue_complete(&mut self, n: &str) -> Events {
        if let Some(s) = &mut self.saved.scene {
            if s.dialogue() == Some(n) {
                s.phase(if n == INTRO { Phase::Run } else { Phase::Outro });
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
        if dt <= 0. {
            return Ok(());
        }
        let dt = dt.min(0.1);
        self.saved.fade = (self.saved.fade - dt).max(0.);
        let mut finish = false;
        if let Some(s) = &mut self.saved.scene {
            let local = scene::local_delta(s, dt, &self.data);
            let mut r = SceneRunner {
                spec: &spec(s.kind, None),
                state: &mut s.clock,
            };
            r.capture(p);
            r.advance(local);
            p.velocity = Vec3::ZERO;
            p.script_motion = 1;
            p.cancel_climb();
            p.release_rope();
            let t = s.elapsed();
            if s.skipping {
                finish = t >= 0.5;
            } else if s.kind.bridge() {
                finish = s.clock.time
                    >= self.data.collapse() + if s.kind == Kind::Second { 4. } else { 3.5 };
            } else {
                match (s.kind, s.phase) {
                    (Kind::Arrival, Phase::Approach)
                        if t >= self.data.travel(
                            "c_whiterabbit",
                            "run",
                            "rabbit_pos1",
                            "rabbit_pos2",
                        ) + 1. =>
                    {
                        s.phase(Phase::Talk)
                    }
                    (Kind::Arrival, Phase::Run) if t >= 4.5 => s.phase(Phase::Squish),
                    (Kind::Arrival, Phase::Squish)
                        if t >= 15.
                            + self.data.travel(
                                "alice",
                                "run",
                                "alice_pos_squish3",
                                "alice_pos_squish2",
                            ) =>
                    {
                        s.phase(Phase::Kneel)
                    }
                    (Kind::Arrival, Phase::Outro) if t >= 5. => finish = true,
                    (Kind::Cat, Phase::Approach) if t >= 2.5 => s.phase(Phase::Talk),
                    (Kind::Cat, Phase::Outro) if t >= 4.5 => finish = true,
                    _ => (),
                }
            }
        }
        if finish {
            self.finish(map, w, p, fixed)?;
        } else {
            self.rebuild(map)?;
            w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        }
        if !self.scripted() {
            self.advance_fulcrum(dt, map, w, p, fixed)?;
            for l in &mut self.saved.ladies {
                if let Some(t) = &mut l.delay {
                    *t = (*t - dt).max(0.);
                    if *t == 0. {
                        l.enabled = true;
                        l.actor.patrol_started = true;
                        l.delay = None;
                    }
                }
            }
        }
        Ok(())
    }
    fn skip(&mut self, _: &Bsp, _: &mut World, p: &mut Player, story: &mut Story) -> Result<bool> {
        let Some(s) = &mut self.saved.scene else {
            return Ok(false);
        };
        if s.skipping {
            return Ok(false);
        }
        for n in match s.kind {
            Kind::Arrival => &[INTRO, KNEEL][..],
            Kind::Cat => &[FINAL][..],
            _ => &[][..],
        } {
            if !story.has_seen(n) {
                story.trigger(n);
            }
            story.finish_sequence(n);
        }
        SceneRunner {
            spec: &spec(s.kind, None),
            state: &mut s.clock,
        }
        .capture(p);
        s.skipping = true;
        s.phase(Phase::Outro);
        Ok(true)
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        self.scene_camera()
    }
    fn fade(&self) -> Option<(Color, f32)> {
        self.scene_fade()
    }
    fn scene_fog(&self) -> Option<Vec4> {
        self.saved
            .scene
            .as_ref()
            .filter(|s| s.kind.bridge())
            .map(|s| {
                if s.clock.time < self.data.collapse() - 1.3 {
                    vec4(0.33, 0.23, 0.1, 2000.)
                } else {
                    vec4(0.25, 0.25, 0.15, 2400.)
                }
            })
    }
    fn take_presentation_event(&mut self) -> Option<&'static str> {
        if std::mem::take(&mut self.saved.fog_pending) {
            Some("Garden2_Fog1")
        } else {
            None
        }
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.objects
            .iter()
            .filter_map(|o| o.pose.map(|p| (o.model, p.translation, p.rotation)))
            .collect()
    }
    fn colliders(&self) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| o.pose.is_some())
            .map(|o| o.collider.clone())
            .collect()
    }
    fn settled_supports(&self) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| {
                o.pose.is_some_and(|p| {
                    p.translation == o.base.translation && p.rotation == o.base.rotation
                })
            })
            .map(|o| o.collider.clone())
            .collect()
    }
    fn receivers(&self, _: &crate::entity::Registry) -> Vec<crate::entity::Id> {
        self.saved
            .ants
            .iter()
            .map(|a| crate::entity::Id(a.id))
            .collect()
    }
    fn rules(&self, _: &crate::level::RuleContext<'_>) -> Vec<crate::event::Rule> {
        self.saved
            .ants
            .iter()
            .map(|a| {
                let id = crate::entity::Id(a.id);
                crate::event::Rule {
                    key: format!("garden2/ant/{}", a.id),
                    event: crate::event::Event::Entity(id, crate::event::Input::Activate),
                    condition: Condition::Always,
                    once: false,
                    cooldown: 0.,
                    actions: vec![crate::event::Action::Output(
                        crate::event::Effect::Activate(id),
                    )],
                }
            })
            .collect()
    }
    fn output(&mut self, e: &crate::event::Effect) -> Option<Events> {
        if let crate::event::Effect::Activate(id) = e {
            if let Some(a) = self.saved.ants.iter_mut().find(|a| a.id == id.0) {
                a.enabled = true;
                return Some(Events::default());
            }
        }
        None
    }
    fn targets(&self) -> Vec<crate::combat::Target> {
        self.encounter_targets()
    }
    fn hit(&mut self, h: crate::combat::Hit) -> Option<&'static str> {
        self.encounter_hit(h)
    }
    fn combat(&mut self, c: &mut crate::level::Combat<'_>) -> crate::combat::Feedback {
        self.encounter_combat(c)
    }
    fn loot_sources(&self) -> Vec<crate::loot::Source> {
        self.saved
            .ants
            .iter()
            .filter(|a| a.enabled)
            .map(|a| crate::loot::Source {
                id: BASE + a.id,
                feet: a.feet,
                dead: a.health == 0.,
                grade: a.grade(),
            })
            .chain(
                self.saved
                    .ladies
                    .iter()
                    .filter(|l| l.enabled)
                    .map(|l| crate::loot::Source {
                        id: BASE + l.id,
                        feet: l.actor.feet,
                        dead: l.actor.health <= 0.,
                        grade: crate::loot::Grade::Medium,
                    }),
            )
            .collect()
    }
    fn sound_state(
        &self,
        _: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        self.sounds(clocks);
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        let mut s: Saved = state::load(v, state::Visit { returning: false })?;
        ensure!(
            s.ants.len() == self.saved.ants.len()
                && s.ants
                    .iter()
                    .zip(&self.saved.ants)
                    .all(|(a, b)| a.id == b.id && a.corporal == b.corporal),
            "Garden Ant identity mismatch"
        );
        ensure!(
            s.ladies.len() == self.saved.ladies.len(),
            "Garden Ladybug identity mismatch"
        );
        for (a, b) in s.ladies.iter().zip(&self.saved.ladies) {
            ensure!(a.id == b.id, "Garden Ladybug identity mismatch");
            a.actor.validate_save(&b.actor)?;
        }
        // Earlier saves could contain active Ladybugs whose patrol was never started.
        for l in &mut s.ladies {
            l.actor.patrol_started |= l.enabled;
        }
        self.saved = s;
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
    id: "garden2",
    applies: |m, e| super::first_visit(m, e, "garden2"),
    load: |a, m, _, _| Ok(Box::new(Garden::load(a, m)?)),
    art: Some(|a, _, _| Ok(Box::new(art::Art::load(a)?))),
    owns_submodel: |m, e| m == "garden2" && motion::owns(e),
    owns_npc: |n, m| {
        matches!(
            n,
            "rabbit_actor"
                | "rabbit_actor_tiny"
                | "rabbit_actor_dead"
                | "hatter_actor_tiny"
                | "cat_actor1"
                | "last_cat"
        ) || matches!(m, "c_armyant" | "c_armyantcorp")
    },
    target_base: Some(BASE),
    story_beats: BEATS,
    checks: &[
        Check {
            flag: "--garden2-route-check",
            help: "Walk the underworld with the native enemy cast and saved continuation.",
            run: Run::Windowed(route::check),
        },
        Check {
            flag: "--garden2-check",
            help: "Verify Herbaceous scenes, physical bridges and safe saved handoffs.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--garden2-render-check",
            help: "Render Herbaceous shots and scene-owned cast.",
            run: Run::Windowed(check::render),
        },
    ],
    save_cases: &[
        SaveCase {
            name: "garden2-arrival-rabbit",
            visit: "garden2$first",
            stage: None,
            behavior: None,
        },
        SaveCase {
            name: "garden2-arrival-hatter",
            visit: "garden2$first",
            stage: None,
            behavior: None,
        },
        SaveCase {
            name: "garden2-arrival-kneel",
            visit: "garden2$first",
            stage: None,
            behavior: None,
        },
        SaveCase {
            name: "garden2-bridge1-fall",
            visit: "garden2$first",
            stage: None,
            behavior: None,
        },
        SaveCase {
            name: "garden2-bridge2-fall",
            visit: "garden2$first",
            stage: None,
            behavior: None,
        },
        SaveCase {
            name: "garden2-cat-talk",
            visit: "garden2$first",
            stage: None,
            behavior: None,
        },
    ],
    visibility: &[],
};
fn owner(i: &mut Interactions) -> Result<&mut Garden> {
    i.levels
        .iter_mut()
        .find_map(|s| s.ctl.downcast_mut())
        .context("Herbaceous owner missing")
}
