//! Centipede arena: animation-driven weak point, three combat stages and mushroom departure.
mod art;
mod battle;
mod check;
pub(crate) use check::drive as drive_route;
mod data;
mod saves;
mod scene;
use super::state;
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World},
    combat::{Feedback, Hit, Target},
    interaction::Events,
    inventory::Stats,
    level::{Combat, LevelArt, LevelController, Respawn, TriggerClass, TriggerInfo, Upgrade},
    movement::Player,
    skeletal::Transform,
    story::{registry::BeatSpec, Story},
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;
const BASE: usize = 7_700_000;
const INTRO: &str = "Centipede2_Start";
const GROW: &str = "Centipede2_Grow_Alice";
const TALK1: &str = "Grow_Dialog1";
const TALK2: &str = "Grow_Dialog2";
const EXIT: crate::level::spec::ExitSpec = crate::level::spec::ExitSpec {
    map: "wforest",
    entrance: "wforest_start1",
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    Intro,
    Slide,
    Fight,
    Drop,
    Climb,
    Greeting,
    Eat,
    Grow,
    Done,
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    phase: Phase,
    time: f32,
    clock: f32,
    battle: battle::Battle,
    essence: usize,
    essence_wait: f32,
    exit: crate::level::exit::ExitState,
    checkpoint: bool,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        state::clock("Centipede scene", self.time, 3600.)?;
        state::clock("Centipede clock", self.clock, 86400.)?;
        state::clock("Centipede essence", self.essence_wait, 10.)?;
        state::clock("Centipede exit retry", self.exit.retry_time, 1.)?;
        ensure!(self.essence < 3, "Invalid essence station");
        self.battle.validate()?;
        let defeated = matches!(
            self.phase,
            Phase::Drop | Phase::Climb | Phase::Greeting | Phase::Eat | Phase::Grow | Phase::Done
        );
        ensure!(
            !defeated || self.battle.hits == 6 && self.battle.action == battle::Action::Dead,
            "Premature boss defeat"
        );
        ensure!(
            self.exit.committed == (self.phase == Phase::Done),
            "Premature mushroom departure"
        );
        ensure!(
            !matches!(self.phase, Phase::Intro | Phase::Slide) || self.battle.hits == 0,
            "Injured boss before arena"
        );
        Ok(())
    }
}
struct Centipede {
    data: data::Data,
    saved: Saved,
    shapes: Vec<Collider>,
    poses: Vec<(usize, Vec3, Quat)>,
    alive: bool,
}
impl Centipede {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let data = data::Data::load(a, map)?;
        let mut pose = data.points["centipede_actor1"];
        pose.translation = World::from_bsp(map)?
            .actor_footing(
                pose.translation,
                vec3(0., 0., 226.),
                vec3(64., 64., 226.),
                128.,
            )
            .context("Centipede has no arena floor")?;
        let battle = battle::Battle::new(pose);
        let mut o = Self {
            data,
            saved: Saved {
                version: 1,
                phase: Phase::Intro,
                time: 0.,
                clock: 0.,
                battle,
                essence: 2,
                essence_wait: 0.,
                exit: Default::default(),
                checkpoint: false,
            },
            shapes: vec![],
            poses: vec![],
            alive: true,
        };
        o.rebuild(map)?;
        Ok(o)
    }
    fn phase(&mut self, p: Phase) {
        self.saved.phase = p;
        self.saved.time = 0.;
    }
    fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        let poses = self.brush_poses();
        if poses == self.poses {
            return Ok(());
        }
        self.shapes = poses
            .iter()
            .filter(|(_, p, _)| p.z > -90000.)
            .map(|(m, p, q)| Collider::model(map, *m, *p, *q, true))
            .collect::<Result<_>>()?;
        self.poses = poses;
        Ok(())
    }
    fn essence_position(&self) -> Vec3 {
        self.data.points[["get_me1", "get_me2", "get_me3"][self.saved.essence]].translation
    }
}
impl LevelController for Centipede {
    fn id(&self) -> &'static str {
        "centipede2"
    }
    fn facts(&self) -> crate::event::Facts {
        let mut f = crate::event::Facts::default();
        f.flag("centipede2.climb", self.saved.phase == Phase::Climb);
        f.flag("centipede2.arena", self.saved.phase == Phase::Slide);
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<crate::event::Condition> {
        use crate::event::Condition;
        if t.class == TriggerClass::Exit {
            Some(Condition::Always.not())
        } else if t.thread == GROW || t.name == "catmessage" {
            Some(Condition::flag("centipede2.climb"))
        } else if t.thread == "Centipede2_MinHealth_Reset" {
            Some(Condition::flag("centipede2.arena"))
        } else {
            None
        }
    }
    fn event(&mut self, t: &str) -> Option<Events> {
        match t {
            INTRO
            | "C2_End"
            | "Centipede2_DropSpike"
            | "Cent2_Get_Tea"
            | "Centipede2_ME1"
            | "Centipede2_ME2"
            | "Centipede2_ME3" => {}
            "Centipede2_MinHealth_Reset" => {
                if self.saved.phase == Phase::Slide {
                    self.phase(Phase::Fight);
                    self.saved.checkpoint = true;
                }
            }
            GROW => {
                if self.saved.phase == Phase::Climb && self.alive {
                    self.phase(Phase::Greeting);
                }
            }
            _ => return None,
        }
        Some(Events::default())
    }
    fn dialogue_complete(&mut self, n: &str) -> Events {
        if n == TALK1 && self.saved.phase == Phase::Greeting {
            self.phase(Phase::Eat);
        }
        Events::default()
    }
    fn prepare_player(&mut self, s: &mut Stats, p: &mut Player) {
        self.alive = s.alive();
        s.minimum_sanity = if self.saved.phase == Phase::Slide {
            30.
        } else {
            0.
        };
        if self.saved.phase == Phase::Fight
            && self.saved.essence_wait == 0.
            && s.alive()
            && (s.sanity() < 100. || s.will() < 100.)
        {
            let d = (p.feet + self::vec3(0., 0., 32.) - self.essence_position()).abs();
            if d.x < 36. && d.y < 36. && d.z < 64. {
                s.essence(50.);
                self.saved.essence = (self.saved.essence + 1) % 3;
                self.saved.essence_wait = 10.;
            }
        }
    }
    fn dismiss_summons(&self) -> bool {
        true
    }
    fn quake_offset(&self) -> Vec3 {
        let b = &self.saved.battle;
        if self.saved.phase == Phase::Fight {
            vec3(
                (self.saved.clock * 53.).sin(),
                (self.saved.clock * 67.).sin(),
                (self.saved.clock * 73.).cos(),
            ) * b.quake
                * 2.
        } else {
            Vec3::ZERO
        }
    }
    fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        _: &[Collider],
    ) -> Result<()> {
        if dt <= 0. || !self.alive {
            return Ok(());
        }
        let dt = dt.min(0.1);
        self.saved.clock = (self.saved.clock + dt).min(86400.);
        self.saved.time = (self.saved.time + dt).min(3600.);
        self.saved.essence_wait = (self.saved.essence_wait - dt).max(0.);
        self.saved.exit.advance(dt);
        self.advance_scene(map, w, p)?;
        self.rebuild(map)
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
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.poses.clone()
    }
    fn colliders(&self) -> Vec<Collider> {
        self.shapes.clone()
    }
    fn settled_supports(&self) -> Vec<Collider> {
        self.poses
            .iter()
            .position(|(m, _, _)| *m == 2)
            .map(|i| vec![self.shapes[i].clone()])
            .unwrap_or_default()
    }
    fn traversal(&self, t: &mut crate::traversal::Traversal) {
        for p in &mut t.pushes {
            if p.id.0 == 21 {
                p.enabled =
                    self.saved.phase != Phase::Intro || self.saved.time >= self.slide_start();
            }
        }
    }
    fn scripted(&self) -> bool {
        matches!(
            self.saved.phase,
            Phase::Intro | Phase::Drop | Phase::Greeting | Phase::Eat | Phase::Grow | Phase::Done
        )
    }
    fn controlled(&self) -> bool {
        self.scripted() || self.saved.battle.grabbed
    }
    fn scene_id(&self) -> Option<&'static str> {
        (self.saved.phase == Phase::Intro).then_some(INTRO)
    }
    fn camera(&self, w: &World) -> Option<crate::cinematic::Camera> {
        self.scene_camera(w)
    }
    fn fade(&self) -> Option<(Color, f32)> {
        self.scene_fade()
    }
    fn entry_story(&mut self, _: &mut Story) -> bool {
        true
    }
    fn prepare_story(&self, s: &mut Story) -> bool {
        let talk = if self.saved.phase == Phase::Greeting && self.saved.time >= 3. {
            Some(TALK1)
        } else if self.saved.phase == Phase::Grow {
            Some(TALK2)
        } else {
            None
        };
        if let Some(n) = talk {
            if !s.busy() && !s.has_seen(n) {
                s.trigger(n);
            }
        }
        true
    }
    fn skip(&mut self, map: &Bsp, w: &mut World, p: &mut Player, _: &mut Story) -> Result<bool> {
        if self.saved.phase != Phase::Intro {
            return Ok(false);
        }
        self.finish_intro(w, p)?;
        self.rebuild(map)?;
        Ok(true)
    }
    fn allow_cheshire(&self) -> bool {
        self.saved.phase == Phase::Climb
    }
    fn music_mood(&self) -> Option<&'static str> {
        Some(if self.saved.phase == Phase::Fight {
            "action"
        } else {
            "normal"
        })
    }
    fn objective(&self) -> Option<String> {
        Some(
            match self.saved.phase {
                Phase::Intro | Phase::Slide => "Follow the slide into the arena.",
                Phase::Fight => {
                    "Dodge the Centipede. Strike the glowing weak spot as it rears up to crush you."
                }
                Phase::Drop => "The ceiling is collapsing.",
                _ => "Climb the fallen spikes to the mushroom.",
            }
            .into(),
        )
    }
    fn recovery_entry(&self, normal: (Vec3, f32)) -> (Vec3, f32) {
        let point = match self.saved.phase {
            Phase::Intro => return normal,
            Phase::Slide => "alice_slide_pos1",
            _ => "alice_pos3",
        };
        let p = self.data.points[point];
        (p.translation, p.rotation.to_euler(EulerRot::ZYX).0)
    }
    fn targets(&self) -> Vec<Target> {
        if self.saved.phase == Phase::Fight {
            self.saved.battle.targets(&self.data)
        } else {
            vec![]
        }
    }
    fn hit(&mut self, h: Hit) -> Option<&'static str> {
        if self.saved.phase == Phase::Fight {
            self.saved.battle.hit(h, &self.data)
        } else {
            None
        }
    }
    fn combat(&mut self, c: &mut Combat<'_>) -> Feedback {
        if self.saved.phase != Phase::Fight || c.dt <= 0. || !c.stats.alive() {
            return Feedback::default();
        }
        self.saved.battle.advance(c, &self.data)
    }
    fn lights(&self) -> Vec<crate::lighting::Light> {
        self.boss_lights()
    }
    fn sound_state(
        &self,
        l: &mut Vec<crate::audio::LoopCue>,
        c: &mut Vec<crate::audio::world::Clock>,
    ) {
        self.scene_sounds(l, c)
    }
    fn checkpoint_requested(&self) -> bool {
        self.saved.checkpoint && self.saved.phase == Phase::Fight
    }
    fn checkpoint_written(&mut self) {
        self.saved.checkpoint = false;
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        self.saved = state::load(v, state::Visit { returning: false })?;
        self.rebuild(map)
    }
    fn upgrade(&self) -> Upgrade {
        Upgrade {
            respawn: Respawn::Always,
            rearm: vec![
                INTRO.into(),
                GROW.into(),
                "Centipede2_MinHealth_Reset".into(),
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
pub static REGISTRATION: super::Registration = super::Registration {
    id: "centipede2",
    applies: |m, e| super::first_visit(m, e, "centipede2"),
    load: |a, m, _, _| Ok(Box::new(Centipede::load(a, m)?)),
    art: Some(|a, _, l| {
        Ok(Box::new(art::Art::load(
            a,
            l.downcast_ref::<Centipede>()
                .context("Centipede owner missing")?,
        )?))
    }),
    owns_submodel: |m, e| {
        m == "centipede2" && e.get("classname").is_some_and(|s| s == "script_object")
    },
    owns_npc: |n, m| {
        m == "c_centipede"
            || matches!(
                n,
                "ant_guard1"
                    | "ant_guard2"
                    | "ant_guard3"
                    | "ant_guard4"
                    | "ant1"
                    | "ant2"
                    | "cat_actor1"
            )
    },
    target_base: Some(BASE),
    story_beats: &[
        BeatSpec::linear("centipede2", TALK1, "centipede2_cinematics", TALK1, 2),
        BeatSpec::linear("centipede2", TALK2, "centipede2_cinematics", TALK2, 1),
    ],
    checks: &[
        super::Check {
            flag: "--centipede2-check",
            help: "Verify Centipede weak points, phases, spikes and mushroom exit.",
            run: super::Run::Headless(check::check),
        },
        super::Check {
            flag: "--centipede2-route-check",
            help: "Play the Centipede arena through the real movement and weapon route.",
            run: super::Run::Headless(check::route),
        },
        super::Check {
            flag: "--centipede2-render-check",
            help: "Capture Centipede combat and story scenes.",
            run: super::Run::Windowed(check::render),
        },
        super::Check {
            flag: "--centipede2-save-write",
            help: "Write Centipede native save fixtures.",
            run: super::Run::Windowed(saves::write),
        },
        super::Check {
            flag: "--centipede2-save-read",
            help: "Restore Centipede saves in a fresh process.",
            run: super::Run::Windowed(saves::read),
        },
    ],
    save_cases: &[],
    visibility: &[],
};
