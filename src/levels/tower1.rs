//! Airborne Terror: saved arrival performance and four contact-driven face gusts.
mod check;
mod faces;
mod route;
mod scene;
use super::{state, Check, Registration, Run};
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World},
    event::{Condition, Facts},
    interaction::{self, Events, Interactions},
    level::{LevelArt, LevelController, TriggerInfo},
    movement::{Player, FIXED_DT},
    skeletal::Transform,
    story::{BeatSpec, Story},
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;
const INTRO: &str = "Tower1_Start";
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    scene: Option<scene::Intro>,
    arrived: bool,
    fade: f32,
    #[serde(default)]
    cat_tail: f32,
    faces: [f64; 4],
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        state::clock("Tower arrival fade", self.fade, 0.5)?;
        state::clock("Tower Cat fade tail", self.cat_tail, 1.)?;
        ensure!(
            self.arrived == self.scene.is_none(),
            "Invalid Tower arrival"
        );
        ensure!(
            self.faces
                .iter()
                .all(|t| t.is_finite() && (0. ..=6.).contains(t)),
            "Invalid face phase"
        );
        if let Some(s) = &self.scene {
            s.validate()?;
        }
        Ok(())
    }
}
struct Tower {
    saved: Saved,
    data: scene::Data,
    faces: Vec<faces::Face>,
    actor_pushes: Vec<crate::traversal::Push>,
}
impl Tower {
    fn load(a: &mut Assets, m: &Bsp) -> Result<Self> {
        Ok(Self {
            saved: Saved {
                version: 1,
                scene: Some(scene::Intro::new()),
                arrived: false,
                fade: 0.,
                cat_tail: 0.,
                faces: [6.; 4],
            },
            data: scene::Data::load(a, m)?,
            faces: faces::load(m)?,
            actor_pushes: faces::actor_pushes(m)?,
        })
    }
    fn gust(&self, n: usize) -> bool {
        (0.55..2.55).contains(&self.saved.faces[n])
    }
}
impl LevelController for Tower {
    fn id(&self) -> &'static str {
        "tower1"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("tower1.arrived", self.saved.arrived);
        for n in 0..4 {
            f.flag(
                &format!("tower1.face{}.ready", n + 1),
                self.saved.faces[n] >= 6.,
            );
        }
        f.flag("tower1.intro_hint_disabled", false);
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        if let Some(n) = [1, 406, 407, 409].iter().position(|id| *id == t.id.0) {
            return Some(Condition::flag(&format!("tower1.face{}.ready", n + 1)));
        }
        if t.id.0 == 20 {
            Some(Condition::flag("tower1.intro_hint_disabled"))
        } else {
            None
        }
    }
    fn event(&mut self, thread: &str) -> Option<Events> {
        if let Some(n) = ["Face1Thread", "Face2Thread", "Face3Thread", "Face4Thread"]
            .iter()
            .position(|s| *s == thread)
        {
            if self.saved.faces[n] >= 6. {
                self.saved.faces[n] = 0.;
            }
        } else if ![INTRO, "Tower1_Cinematics_Init", "T1Start_End"].contains(&thread) {
            return None;
        }
        Some(Events::default())
    }
    fn advance(
        &mut self,
        dt: f32,
        _: &Bsp,
        w: &mut World,
        p: &mut Player,
        _: &[Collider],
    ) -> Result<()> {
        if dt <= 0. {
            return Ok(());
        }
        let dt = dt.min(0.1);
        self.saved.fade = (self.saved.fade - dt).max(0.);
        self.saved.cat_tail = (self.saved.cat_tail - dt).max(0.);
        for t in &mut self.saved.faces {
            *t = (*t + dt as f64).min(6.);
        }
        self.advance_intro(dt, w, p)
    }
    fn traversal(&self, t: &mut crate::traversal::Traversal) {
        t.actor_pushes = self.actor_pushes.clone();
        for p in &mut t.actor_pushes {
            if let Some(n) = self.faces.iter().position(|f| f.push == p.id.0) {
                p.enabled = self.gust(n);
            }
        }
        for p in &mut t.pushes {
            if let Some(n) = self.faces.iter().position(|f| f.push == p.id.0) {
                p.enabled = self.gust(n);
            }
        }
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
        if s.clock.time < 4.5 || s.skip.is_some() {
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

    fn upgraded(&mut self) {
        self.saved.arrived = true;
        self.saved.scene = None;
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, _: &Bsp) -> Result<()> {
        self.saved = state::load(v, state::Visit { returning: false })?;
        Ok(())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
pub static REGISTRATION: Registration = Registration {
    id: "tower1",
    applies: |m, e| super::first_visit(m, e, "tower1"),
    load: |a, m, _, _| Ok(Box::new(Tower::load(a, m)?)),
    art: Some(|a, _, _| Ok(Box::new(scene::Art::load(a)?))),
    owns_submodel: |_, _| false,
    owns_npc: |n, m| n == "cat_actor1" && m == "c_cheshire",
    target_base: None,
    story_beats: &[BeatSpec::linear(
        "tower1",
        INTRO,
        "../tower1",
        "Cat1_Dialog",
        1,
    )],
    checks: &[
        Check {
            flag: "--tower1-check",
            help: "Verify Tower arrival, timed face gusts and saved phases.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--tower1-route-check",
            help: "Traverse Airborne Terror into Hedge2 with the production cast.",
            run: Run::Windowed(route::check),
        },
        Check {
            flag: "--tower1-skip-route-check",
            help: "Traverse Airborne Terror after skipping arrival.",
            run: Run::Windowed(route::skipped),
        },
        Check {
            flag: "--tower1-probe",
            help: "Diagnose Tower navigation without the native cast.",
            run: Run::Headless(route::probe),
        },
        Check {
            flag: "--tower1-actors-check",
            help: "Verify native Tower enemy activation, death and save identity.",
            run: Run::Windowed(|a| Box::pin(check::actors(a))),
        },
        Check {
            flag: "--tower1-render-check",
            help: "Capture Tower faces and Cheshire staging.",
            run: Run::Windowed(|a| Box::pin(check::render(a))),
        },
    ],
    save_cases: check::SAVES,
    visibility: &[],
};
fn owner(i: &mut Interactions) -> Result<&mut Tower> {
    i.levels
        .iter_mut()
        .find(|s| s.reg.id == "tower1")
        .and_then(|s| s.ctl.downcast_mut::<Tower>())
        .context("Missing Tower1 controller")
}

pub(crate) use route::drive as drive_route;
