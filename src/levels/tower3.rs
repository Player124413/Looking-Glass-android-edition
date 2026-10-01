//! Machinations: bound lifts and ride machinery, room fall returns and arrival scene.
mod check;
mod motion;
mod route;
mod scene;
use super::{state, Check, Registration, Run};
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    event::{Condition, Facts},
    interaction::{self, Events, Interactions},
    level::{LevelArt, LevelController, Respawn, TriggerInfo, Upgrade},
    movement::{Controls, Player, FIXED_DT},
    skeletal::Transform,
    story::{BeatSpec, Story},
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;
const INTRO: &str = "Tower3_Start";
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    clocks: Vec<f64>,
    room: u8,
    scene: Option<scene::Intro>,
    arrived: bool,
    fade: f32,
    damage: f32,
    hurt_wait: f32,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        ensure!((1..=3).contains(&self.room), "Invalid Tower checkpoint");
        ensure!(
            self.clocks.len() <= 64
                && self
                    .clocks
                    .iter()
                    .all(|t| t.is_finite() && (0. ..=1e8).contains(t)),
            "Invalid Tower clocks"
        );
        state::clock("Tower fade", self.fade, 0.5)?;
        state::clock("Tower crush wait", self.hurt_wait, 0.5)?;
        ensure!(
            [0., 15., 1000.].contains(&self.damage),
            "Invalid Tower damage"
        );
        ensure!(
            self.arrived == self.scene.is_none(),
            "Inconsistent Tower introduction"
        );
        if let Some(s) = &self.scene {
            s.validate()?;
        }
        Ok(())
    }
}
struct Tower {
    objects: Vec<motion::Object>,
    roots: Vec<usize>,
    saved: Saved,
    data: scene::Data,
}
impl Tower {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let objects = motion::load(map)?;
        let roots = objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.parent.is_none())
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        let saved = Saved {
            version: 1,
            clocks: vec![0.; roots.len()],
            room: 1,
            scene: Some(scene::Intro::new()),
            arrived: false,
            fade: 0.,
            damage: 0.,
            hurt_wait: 0.,
        };
        let mut out = Self {
            objects,
            roots,
            saved,
            data: scene::Data::load(a, map)?,
        };
        out.rebuild();
        Ok(out)
    }
    fn rebuild(&mut self) {
        for (g, root) in self.roots.iter().copied().enumerate() {
            let poses = motion::poses(&self.objects, root, self.saved.clocks[g]);
            for (i, pose) in poses {
                self.objects[i].set(pose);
            }
        }
    }
}
impl LevelController for Tower {
    fn id(&self) -> &'static str {
        "tower3"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        for n in 1..=3 {
            f.flag(&format!("tower3.room{n}"), self.saved.room == n);
        }
        f.flag("tower3.play", self.saved.arrived);
        f.flag("tower3.intro_hint_disabled", false);
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        match t.name {
            "room1_tele" => Some(Condition::flag("tower3.room1")),
            "room2_tele" => Some(Condition::flag("tower3.room2")),
            "room3_tele" => Some(Condition::flag("tower3.room3")),
            _ if t.id.0 == 16 => Some(Condition::flag("tower3.intro_hint_disabled")),
            _ => None,
        }
    }
    fn event(&mut self, thread: &str) -> Option<Events> {
        match thread {
            "Room1Tele" => self.saved.room = 1,
            "Room2Tele" => self.saved.room = 2,
            "Room3Tele" => self.saved.room = 3,
            "Tower3_Start"
            | "Setup"
            | "Tower3_Cinematics_Init"
            | "Crank_Move"
            | "Pedal_Move"
            | "Arm_Move"
            | "Pendulum_Move"
            | "Gear_Move"
            | "Gear03_MoveUp"
            | "Gear04_MoveUp"
            | "Gear05_MoveUp"
            | "Gear06_MoveUp"
            | "GearPlat_MoveUp"
            | "GearUp_01_MoveUp"
            | "GearUp_02_MoveUp"
            | "CageWheel_Move"
            | "MedGear_Move"
            | "BigGear_Move"
            | "BigLatch_Move"
            | "Grinder_Move"
            | "ladder_drop" => {}
            _ => return None,
        }
        Some(Events::default())
    }
    fn advance(
        &mut self,
        dt: f32,
        _: &Bsp,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. {
            return Ok(());
        }
        let dt = dt.min(0.1);
        self.saved.fade = (self.saved.fade - dt).max(0.);
        self.saved.hurt_wait = (self.saved.hurt_wait - dt).max(0.);
        self.advance_intro(dt, w, p)?;
        self.move_all(dt, w, p, fixed)
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.objects
            .iter()
            .filter_map(|o| o.model.map(|m| (m, o.pose.translation, o.pose.rotation)))
            .collect()
    }
    fn colliders(&self) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| o.solid)
            .filter_map(|o| o.collider.clone())
            .collect()
    }
    fn settled_supports(&self) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| matches!(o.id, 21 | 22 | 28 | 29))
            .filter_map(|o| o.collider.clone())
            .collect()
    }
    fn combat(&mut self, c: &mut crate::level::Combat<'_>) -> crate::combat::Feedback {
        let mut f = crate::combat::Feedback::default();
        if c.dt > 0. {
            f.damage = std::mem::take(&mut self.saved.damage);
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
    fn recovery_entry(&self, normal: (Vec3, f32)) -> (Vec3, f32) {
        match self.saved.room {
            2 => (vec3(450., 704., 1120. + crate::movement::EYE_HEIGHT), 0.),
            3 => (
                vec3(2432., 960., 1632. + crate::movement::EYE_HEIGHT),
                135_f32.to_radians(),
            ),
            _ => (normal.0, normal.1),
        }
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, _: &Bsp) -> Result<()> {
        let s: Saved = state::load(v, state::Visit { returning: false })?;
        ensure!(
            s.clocks.len() == self.roots.len(),
            "Tower machine count changed"
        );
        self.saved = s;
        self.rebuild();
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
    id: "tower3",
    applies: |m, e| super::first_visit(m, e, "tower3"),
    load: |a, m, _, _| Ok(Box::new(Tower::load(a, m)?)),
    art: Some(|a, _, _| Ok(Box::new(scene::Art::load(a)?))),
    owns_submodel: |m, e| m == "tower3" && motion::owned(e),
    owns_npc: |n, m| n == "cat_actor1" && m == "c_cheshire",
    target_base: None,
    story_beats: &[BeatSpec::linear(
        "tower3",
        INTRO,
        "../tower3",
        "Cat1_Dialog",
        1,
    )],
    checks: &[
        Check {
            flag: "--tower3-check",
            help: "Verify bound machines, checkpoints and introduction.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--tower3-route-check",
            help: "Traverse Machinations and its fall returns into Royal Rage.",
            run: Run::Headless(route::check),
        },
        Check {
            flag: "--tower3-skip-route-check",
            help: "Traverse Machinations after skipping the introduction.",
            run: Run::Headless(route::skip_check),
        },
        Check {
            flag: "--tower3-render-check",
            help: "Capture Tower machinery and the Cheshire introduction.",
            run: Run::Windowed(|a| Box::pin(check::render(a))),
        },
    ],
    save_cases: check::SAVES,
    visibility: &[],
};
fn owner(i: &mut Interactions) -> Result<&mut Tower> {
    i.levels
        .iter_mut()
        .find(|s| s.reg.id == "tower3")
        .and_then(|s| s.ctl.downcast_mut::<Tower>())
        .context("Missing Tower3 controller")
}

pub(crate) use route::drive as drive_route;
