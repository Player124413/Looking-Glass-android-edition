//! Water Logged: three weighted flushers, rising water, lids, drain fans and the introduction.
mod cast_check;
mod check;
mod motion;
mod route;
mod scene;
use super::{state, Check, Registration, Run};
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, Liquid, World},
    event::{Condition, Facts},
    interaction::{self, Events, Interactions},
    inventory::Stats,
    level::{LevelArt, LevelController, Respawn, TriggerInfo, Upgrade},
    movement::{Controls, Player, FIXED_DT},
    skeletal::Transform,
    story::{BeatSpec, Story},
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;
const INTRO: &str = "Tower2_Start";
const FLUSH: [&str; 3] = ["MoveWater1", "MoveWater2", "MoveWater3"];
fn arrived_legacy() -> bool {
    true
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct Machinery {
    sink: [f32; 3],
    lids: [f32; 3],
    sway: [f32; 3],
    fans: [f32; 5],
    crush: bool,
    #[serde(default)]
    sink_velocity: [f32; 3],
    #[serde(default)]
    currents: [f32; 4],
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    stages: [Option<f32>; 3],
    #[serde(default)]
    machine: Option<Machinery>,
    #[serde(default)]
    scene: Option<scene::Intro>,
    #[serde(default = "arrived_legacy")]
    arrived: bool,
    #[serde(default)]
    fade: f32,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        for (i, t) in self.stages.iter().enumerate() {
            if let Some(t) = t {
                state::clock("flush", *t, 8.)?;
                ensure!(
                    i == 0 || self.stages[i - 1].is_some(),
                    "Skipped water stage"
                );
            }
        }
        state::clock("tower fade", self.fade, 0.5)?;
        ensure!(
            !self.arrived || self.scene.is_none(),
            "Repeated Tower introduction"
        );
        if let Some(s) = &self.scene {
            s.validate()?;
        }
        if let Some(m) = &self.machine {
            for t in m.sink {
                state::clock("flusher depth", t, 64.)?;
            }
            ensure!(
                m.sink_velocity
                    .iter()
                    .all(|v| v.is_finite() && v.abs() < 1000.),
                "Invalid flusher velocity"
            );
            for t in m.currents {
                state::clock("Tower current cooldown", t, 0.2)?;
            }
            for t in m.sway {
                state::clock("lid sway", t, 10.)?;
            }
            for t in m.fans {
                state::clock("fan angle", t, 360.)?;
            }
            for (i, t) in m.lids.iter().enumerate() {
                ensure!(
                    t.is_finite()
                        && if i == 2 {
                            (-38. ..=0.).contains(t)
                        } else {
                            (0. ..=90.).contains(t)
                        },
                    "Invalid tank lid angle"
                );
                ensure!(
                    self.stages[i].is_some() || *t == 0.,
                    "Opened lid before flushing"
                );
            }
        }
        Ok(())
    }
}
struct Object {
    name: String,
    model: usize,
    base: Vec3,
    pose: Transform,
    collider: Collider,
}
struct Tower {
    objects: Vec<Object>,
    saved: Saved,
    water: Liquid,
    data: scene::Data,
    currents: Vec<crate::traversal::Push>,
    stopped: bool,
}
fn owned(e: &super::Entity) -> bool {
    e.get("targetname").is_some_and(|s| {
        s == "water" || s.starts_with("fliptop") || s.starts_with("flusher") || s.starts_with("fan")
    }) && e.get("model").is_some_and(|s| s.starts_with('*'))
}
impl Tower {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let objects = map
            .entities
            .iter()
            .filter(|e| owned(e))
            .map(|e| {
                let model = e["model"].trim_start_matches('*').parse()?;
                let base = interaction::vector(&e["origin"]).context("Tank object lacks origin")?;
                Ok(Object {
                    name: e["targetname"].clone(),
                    model,
                    base,
                    pose: Transform {
                        translation: base,
                        rotation: Quat::IDENTITY,
                    },
                    collider: Collider::model(map, model, base, Quat::IDENTITY, true)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let water = objects
            .iter()
            .find(|o| o.name == "water")
            .context("Missing tank water")?;
        let contents = map.models[water.model]
            .brushes
            .clone()
            .fold(0, |n, i| n | map.shaders[map.brushes[i].shader].contents)
            & crate::collision::LIQUID_MASK;
        ensure!(contents != 0, "Tank brush lacks liquid contents");
        let liquid = Liquid {
            contents,
            volume: Collider::model(map, water.model, water.base, Quat::IDENTITY, false)?,
        };
        let intro = scene::Intro::new();
        let currents = crate::traversal::Traversal::load(map)?
            .pushes
            .into_iter()
            .filter(|p| [30, 48, 49, 50].contains(&p.id.0))
            .collect();
        let mut out = Self {
            objects,
            water: liquid,
            data: scene::Data::load(a, map)?,
            currents,
            stopped: false,
            saved: Saved {
                version: 1,
                stages: [None; 3],
                machine: Some(Machinery::default()),
                scene: Some(intro),
                arrived: false,
                fade: 0.,
            },
        };
        out.rebuild(map)?;
        Ok(out)
    }
    fn machine(&self) -> &Machinery {
        self.saved.machine.as_ref().unwrap()
    }
    fn height(&self) -> f32 {
        624. + self
            .saved
            .stages
            .iter()
            .flatten()
            .map(|t| 256. * (t / 5.).min(1.))
            .sum::<f32>()
    }
    fn ready(&self, i: usize) -> bool {
        self.saved.arrived
            && self.saved.stages[i].is_none()
            && (i == 0 || self.saved.stages[i - 1].is_some_and(|t| t >= 5.))
    }
    fn exit_ready(&self) -> bool {
        self.saved.stages[2].is_some_and(|t| t >= 5.) && self.machine().lids[2] <= -30.
    }
}
impl LevelController for Tower {
    fn id(&self) -> &'static str {
        "tower2"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        for i in 0..3 {
            f.flag(&format!("tower2.flush{}", i + 1), self.ready(i));
        }
        f.flag("tower2.exit", self.exit_ready());
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        if let Some(i) = FLUSH.iter().position(|s| *s == t.thread) {
            return Some(Condition::flag(&format!("tower2.flush{}", i + 1)));
        }
        t.exit
            .filter(|s| s.starts_with("hedge3"))
            .map(|_| Condition::flag("tower2.exit"))
    }
    fn event(&mut self, thread: &str) -> Option<Events> {
        if matches!(thread, "Tower2_Start" | "MoveFan") {
            return Some(Events::default());
        }
        let i = FLUSH.iter().position(|s| *s == thread)?;
        if self.ready(i) {
            self.saved.stages[i] = Some(0.);
        }
        Some(Events::default())
    }
    fn prepare_player(&mut self, s: &mut Stats, p: &mut Player) {
        s.turtle_air = true;
        p.breath.shell = true;
        self.stopped = s.powers.stopped > 0.;
        // Four stationary air emitters launch overlapping four-second breath
        // objects at 0, 2 and 4 seconds: their refresh volume stays occupied.
        let c = p.feet + crate::collision::PLAYER_CENTER;
        let h = crate::collision::PLAYER_HALF;
        let (lo, hi) = self.data.bubble_bounds;
        if self
            .data
            .bubbles
            .iter()
            .any(|b| (*b + lo).cmple(c + h).all() && (*b + hi).cmpge(c - h).all())
        {
            p.breath.refill();
        }
    }
    fn ignores_watch(&self) -> bool {
        true
    }
    fn traversal(&self, t: &mut crate::traversal::Traversal) {
        t.pushes.retain(|p| ![30, 48, 49, 50].contains(&p.id.0));
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.objects
            .iter()
            .map(|o| (o.model, o.pose.translation, o.pose.rotation))
            .collect()
    }
    fn colliders(&self) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| o.name != "water")
            .map(|o| o.collider.clone())
            .collect()
    }
    fn liquids(&self) -> Vec<Liquid> {
        vec![self.water.clone()]
    }
    fn settled_supports(&self) -> Vec<Collider> {
        self.supports()
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
        self.advance_intro(dt, w, p)?;
        self.move_machinery(dt, map, w, p, fixed)?;
        self.move_currents(dt, p);
        Ok(())
    }
    fn combat(&mut self, c: &mut crate::level::Combat<'_>) -> crate::combat::Feedback {
        let mut f = crate::combat::Feedback::default();
        if c.dt > 0. && std::mem::take(&mut self.saved.machine.as_mut().unwrap().crush) {
            f.damage = 1000.;
        }
        f
    }
    fn scripted(&self) -> bool {
        self.saved.scene.is_some()
    }
    fn controlled(&self) -> bool {
        self.scripted()
    }
    fn scene_id(&self) -> Option<&'static str> {
        self.saved
            .scene
            .as_ref()
            .filter(|s| s.skip.is_none())
            .map(|_| INTRO)
    }
    fn allow_cheshire(&self) -> bool {
        !self.scripted()
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        self.saved
            .scene
            .as_ref()
            .map(|s| self.data.camera.camera(s.clock.time))
    }
    fn fade(&self) -> Option<(Color, f32)> {
        self.intro_fade()
    }
    fn entry_story(&mut self, _: &mut Story) -> bool {
        true
    }
    fn prepare_story(&self, story: &mut Story) -> bool {
        let Some(s) = &self.saved.scene else {
            return true;
        };
        if s.clock.time < 2.5 || s.skip.is_some() {
            return false;
        }
        if !story.busy() && !story.has_seen(INTRO) {
            story.trigger(INTRO);
        }
        true
    }
    fn sync_story(&mut self, story: &Story) {
        if let Some(s) = &mut self.saved.scene {
            s.dialogue_done |= story.has_seen(INTRO);
            if let Some((line, time)) = story.progress(INTRO) {
                s.clock.line = line;
                s.clock.line_time = time;
            }
        }
    }
    fn skip(&mut self, _: &Bsp, _: &mut World, p: &mut Player, story: &mut Story) -> Result<bool> {
        let Some(s) = &mut self.saved.scene else {
            return Ok(false);
        };
        if s.skip.is_some() {
            return Ok(false);
        }
        if s.clock.home.is_none() {
            p.script_facing = self.data.yaw;
        }
        crate::level::scene::SceneRunner {
            spec: &scene::spec(),
            state: &mut s.clock,
        }
        .capture(p);
        if !story.has_seen(INTRO) {
            story.trigger(INTRO);
        }
        story.finish_sequence(INTRO);
        s.dialogue_done = true;
        s.skip = Some(0.);
        Ok(true)
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
        if s.machine.is_none() {
            let mut m = Machinery::default();
            for i in 0..3 {
                if let Some(t) = s.stages[i] {
                    m.lids[i] = if i < 2 {
                        s.stages[i + 1]
                            .map_or(35. * (t / 6.).min(1.), |n| 35. + 55. * (n / 8.).min(1.))
                    } else {
                        -35. * (t / 5.).min(1.)
                    };
                }
            }
            s.machine = Some(m);
        }
        self.saved = s;
        self.rebuild(map)
    }
    fn upgraded(&mut self) {
        self.saved.arrived = true;
        self.saved.scene = None;
    }
    fn upgrade(&self) -> Upgrade {
        Upgrade {
            respawn: Respawn::Always,
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
    id: "tower2",
    applies: |m, e| super::first_visit(m, e, "tower2"),
    load: |a, m, _, _| Ok(Box::new(Tower::load(a, m)?)),
    art: Some(|a, _, _| Ok(Box::new(scene::Art::load(a)?))),
    owns_submodel: |m, e| m == "tower2" && owned(e),
    owns_npc: |name, model| name == "cat_actor1" && model == "c_cheshire",
    target_base: None,
    story_beats: &[BeatSpec::linear(
        "tower2",
        INTRO,
        "../tower2",
        "Cat1_Dialog",
        1,
    )],
    checks: &[
        Check {
            flag: "--tower2-cast-check",
            help: "Verify every native Snark wave and its saved continuation.",
            run: Run::Windowed(cast_check::check),
        },
        Check {
            flag: "--tower2-check",
            help: "Verify all water stages, machinery, introduction and saved continuation.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--tower2-render-check",
            help: "Capture tank water, machinery and the introduction.",
            run: Run::Windowed(|a| Box::pin(check::render_check(a))),
        },
        Check {
            flag: "--tower2-route-check",
            help: "Traverse three flushers and dive to hedge3 with live encounters.",
            run: Run::Windowed(route::check),
        },
    ],
    save_cases: &[
        super::SaveCase {
            name: "tower2-intro",
            visit: "tower2$first",
            stage: Some(|i, _| {
                let t = owner(i)?;
                t.saved.arrived = false;
                t.saved.scene = Some(scene::Intro::new());
                t.saved.scene.as_mut().unwrap().clock.time = 3.;
                Ok(())
            }),
            behavior: None,
        },
        super::SaveCase {
            name: "tower2-water",
            visit: "tower2$first",
            stage: Some(|i, map| {
                let t = owner(i)?;
                t.saved.scene = None;
                t.saved.arrived = true;
                t.saved.stages[0] = Some(2.5);
                t.saved.machine.as_mut().unwrap().lids[0] = 35. * 2.5 / 6.;
                t.rebuild(map)
            }),
            behavior: None,
        },
        super::SaveCase {
            name: "tower2-second",
            visit: "tower2$first",
            stage: Some(|i, map| {
                let t = owner(i)?;
                t.upgraded();
                t.saved.stages = [Some(8.), Some(3.), None];
                let m = t.saved.machine.as_mut().unwrap();
                m.lids = [55.625, 17.5, 0.];
                m.sink[1] = 32.;
                m.sink_velocity[1] = 40.;
                t.rebuild(map)
            }),
            behavior: None,
        },
        super::SaveCase {
            name: "tower2-dive-ready",
            visit: "tower2$first",
            stage: Some(|i, map| {
                let t = owner(i)?;
                t.upgraded();
                t.saved.stages = [Some(8.); 3];
                t.saved.machine.as_mut().unwrap().lids = [90., 90., -35.];
                t.rebuild(map)
            }),
            behavior: None,
        },
    ],
    visibility: &[],
};
fn owner(i: &mut Interactions) -> Result<&mut Tower> {
    i.levels
        .iter_mut()
        .find(|l| l.reg.id == "tower2")
        .and_then(|l| l.ctl.downcast_mut::<Tower>())
        .context("Missing tank controller")
}

pub(crate) use route::drive as drive_route;
