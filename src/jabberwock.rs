//! The two Jabberwock encounters share an actor, but never their win conditions.
mod art;
mod altar_check;
mod battle;
pub(crate) mod check;
mod data;
mod fire_check;
mod saves;
mod scene;
use crate::levels::state;
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
    story::Story,
};
use anyhow::{ensure, Context, Result};
use data::Data;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Kind {
    Lair,
    Grounds,
}
impl Kind {
    fn map(self) -> &'static str {
        if self == Self::Lair {
            "jlair2"
        } else {
            "grounds1"
        }
    }
    fn base(self) -> usize {
        if self == Self::Lair {
            8_600_000
        } else {
            9_400_000
        }
    }
    fn intro(self) -> &'static str {
        if self == Self::Lair {
            "JLair2_Cinema1"
        } else {
            "grounds1_StartCine"
        }
    }
    fn outro(self) -> &'static str {
        if self == Self::Lair {
            "JLair2_Dead_Jabber"
        } else {
            "grounds1_EndCine"
        }
    }
    fn exit(self) -> crate::level::spec::ExitSpec {
        if self == Self::Lair {
            crate::level::spec::ExitSpec {
                map: "wforest",
                entrance: "wforest_start2",
            }
        } else {
            crate::level::spec::ExitSpec {
                map: "grounds2",
                entrance: "grounds2_start1",
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    Intro,
    Fight,
    Death,
    Outro,
    Reward,
    Bridge,
    Done,
}
#[derive(Clone, Serialize, Deserialize)]
struct Wave {
    actor: crate::wildlife::Creature,
    launch: f32,
    from: Vec3,
    to: Vec3,
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    kind: Kind,
    phase: Phase,
    time: f32,
    clock: f32,
    ticks: u32,
    remainder: f64,
    boss: battle::Boss,
    waves: Vec<Wave>,
    wave_count: u8,
    essence: usize,
    essence_wait: f32,
    dialogue_done: bool,
    dialogue_line: usize,
    dialogue_time: f32,
    after_talk: f32,
    eye: bool,
    checkpoint: bool,
    exit: crate::level::exit::ExitState,
    sky: u8,
    door: f32,
    door_requested: bool,
    walk_started: Option<f32>,
    cat: Option<f32>,
    cat_ending: bool,
    gnome: Option<f32>,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        for (n, t, m) in [
            ("scene", self.time, 3600.),
            ("level", self.clock, 86400.),
            ("essence", self.essence_wait, 8.),
            ("speech", self.dialogue_time, 300.),
            ("aftermath", self.after_talk, 3600.),
            ("exit retry", self.exit.retry_time, 1.),
        ] {
            state::clock(n, t, m)?;
        }
        ensure!(
            self.remainder.is_finite()
                && (0. ..0.009).contains(&self.remainder)
                && self.ticks <= 10800
                && self.wave_count <= 2
                && self.dialogue_line <= 8,
            "Invalid encounter clock"
        );
        ensure!(
            self.sky <= 2 && self.essence < if self.kind == Kind::Lair { 2 } else { 3 },
            "Invalid essence cycle"
        );
        ensure!(
            self.walk_started
                .is_none_or(|t| t.is_finite() && (0. ..=3600.).contains(&t)),
            "Invalid intro walk"
        );
        ensure!(
            self.cat
                .is_none_or(|t| t.is_finite() && (0. ..=300.).contains(&t))
                && self
                    .gnome
                    .is_none_or(|t| t.is_finite() && (0. ..=30.).contains(&t)),
            "Invalid epilogue actor clock"
        );
        ensure!(
            (0. ..=2.).contains(&self.door)
                && (!self.door_requested
                    || (self.kind == Kind::Grounds && self.phase == Phase::Done)),
            "Invalid exit door"
        );
        self.boss.validate(self.kind)?;
        ensure!(
            self.waves.len() <= 6 && (self.kind == Kind::Lair || self.waves.is_empty()),
            "Invalid waves"
        );
        for w in &self.waves {
            w.actor.validate()?;
            ensure!(
                w.actor.kind == crate::wildlife::Kind::Jabber
                    && w.from.is_finite()
                    && w.to.is_finite()
                    && w.launch.is_finite()
                    && (0. ..=1.5).contains(&w.launch),
                "Invalid wave launch"
            );
        }
        let after = matches!(
            self.phase,
            Phase::Outro | Phase::Reward | Phase::Bridge | Phase::Done
        );
        ensure!(
            !after
                || if self.kind == Kind::Lair {
                    self.ticks == 10800 && self.wave_count == 2
                } else {
                    self.boss.health == 0.
                },
            "Encounter completed before victory"
        );
        ensure!(
            self.phase != Phase::Death || (self.kind == Kind::Grounds && self.boss.health == 0.),
            "Death scene has a living boss"
        );
        if self.phase == Phase::Intro {
            ensure!(
                self.ticks == 0
                    && self.wave_count == 0
                    && self.waves.is_empty()
                    && self.boss.health
                        == if self.kind == Kind::Lair {
                            99999.
                        } else {
                            2000.
                        },
                "Battle modified before intro"
            );
        }
        if self.kind == Kind::Lair && self.phase == Phase::Fight {
            ensure!(
                self.wave_count
                    == if self.ticks >= 7200 {
                        2
                    } else {
                        u8::from(self.ticks >= 3600)
                    }
                    && self.waves.len() == usize::from(self.wave_count) * 3,
                "Wave schedule differs from survival clock"
            );
        }
        ensure!(
            !self.eye
                || (self.kind == Kind::Lair && matches!(self.phase, Phase::Reward | Phase::Done)),
            "Premature Eye Staff"
        );
        ensure!(
            self.kind == Kind::Lair || self.phase != Phase::Reward,
            "Wrong reward"
        );
        ensure!(
            !self.exit.committed
                || (self.kind == Kind::Lair && self.eye && self.phase == Phase::Done),
            "Premature departure"
        );
        Ok(())
    }
}
pub(crate) struct Encounter {
    data: Data,
    saved: Saved,
    poses: Vec<(usize, Vec3, Quat)>,
    shapes: Vec<Collider>,
    alive: bool,
}
impl Encounter {
    pub(crate) fn load(a: &mut Assets, map: &Bsp, kind: Kind) -> Result<Self> {
        let data = Data::load(a, map, kind)?;
        let mut pose = data.points[if kind == Kind::Lair {
            "jabber_pos3"
        } else {
            "jabberwock"
        }];
        pose.translation = World::from_bsp(map)?
            .actor_footing(pose.translation, Vec3::Z * 114., vec3(72., 72., 114.), 256.)
            .context("Jabberwock arena floor unavailable")?;
        let boss = battle::Boss::new(pose, kind);
        let mut o = Self {
            data,
            saved: Saved {
                version: 1,
                kind,
                phase: Phase::Intro,
                time: 0.,
                clock: 0.,
                ticks: 0,
                remainder: 0.,
                boss,
                waves: vec![],
                wave_count: 0,
                essence: 0,
                essence_wait: 0.,
                dialogue_done: false,
                dialogue_line: 0,
                dialogue_time: 0.,
                after_talk: 0.,
                eye: false,
                checkpoint: false,
                exit: Default::default(),
                sky: 0,
                door: 0.,
                door_requested: false,
                walk_started: None,
                cat: None,
                cat_ending: false,
                gnome: None,
            },
            poses: vec![],
            shapes: vec![],
            alive: true,
        };
        o.rebuild(map)?;
        Ok(o)
    }
    fn phase(&mut self, p: Phase) {
        self.saved.phase = p;
        self.saved.time = 0.;
        self.saved.dialogue_done = false;
        self.saved.dialogue_line = 0;
        self.saved.dialogue_time = 0.;
        self.saved.after_talk = 0.;
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
        self.data.points[if self.saved.kind == Kind::Lair {
            ["help_me1", "help_me2"][self.saved.essence]
        } else {
            ["get_me3", "get_me1", "get_me2"][self.saved.essence]
        }]
        .translation
    }
    fn eye_visible(&self) -> bool {
        self.saved.kind == Kind::Lair
            && !self.saved.eye
            && (self.saved.phase == Phase::Reward
                || (self.saved.phase == Phase::Outro && self.saved.time >= 4.55))
    }
    fn finish_intro(&mut self, w: &World, p: &mut Player) -> Result<()> {
        let pose = if self.saved.kind == Kind::Lair {
            self.data
                .points
                .get("alice_pos1")
                .copied()
                .unwrap_or(Transform {
                    translation: vec3(-848., 416., 96.),
                    rotation: Quat::IDENTITY,
                })
        } else {
            self.data.points["gdeath"]
        };
        crate::cinematic::land_player(p, w, pose)?;
        self.phase(Phase::Fight);
        self.saved.checkpoint = true;
        Ok(())
    }
    fn finish_outro(&mut self, w: &World, p: &mut Player) -> Result<()> {
        let pose = self.data.points[if self.saved.kind == Kind::Lair {
            "alice_dead_pos1"
        } else {
            "alice_end1"
        }];
        crate::cinematic::land_player(p, w, pose)?;
        self.saved.waves.clear();
        self.phase(if self.saved.kind == Kind::Lair {
            Phase::Reward
        } else {
            Phase::Bridge
        });
        Ok(())
    }
    fn wave(&mut self) {
        for (from, to) in &self.data.waves {
            let i = self.saved.waves.len();
            let actor =
                crate::wildlife::Creature::new(crate::wildlife::Kind::Jabber, *from, 0., 1., i + 1);
            self.saved.waves.push(Wave {
                actor,
                launch: 0.,
                from: *from,
                to: *to,
            });
        }
        self.saved.wave_count += 1;
    }
}
impl LevelController for Encounter {
    fn particles(&self, steam: &mut crate::particles::Steam) {
        // The reward scene draws this altar itself. Its placed copy must never
        // emit as ambient scenery, including before the reward and after pickup.
        if let Some(id) = self.data.eye_altar {
            steam.gate(&[(id, false)]);
        }
    }
    fn id(&self) -> &'static str {
        self.saved.kind.map()
    }
    fn facts(&self) -> crate::event::Facts {
        let mut f = crate::event::Facts::default();
        f.flag(
            &format!("{}.won", self.id()),
            self.saved.phase == Phase::Done,
        );
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<crate::event::Condition> {
        use crate::event::Condition;
        if t.class == TriggerClass::Exit {
            return Some(Condition::flag(&format!("{}.won", self.id())));
        }
        if matches!(
            t.thread,
            "JLair2_Dead_Jabber" | "pickup_eye" | "grounds1_EndCine"
        ) {
            return Some(Condition::Always.not());
        }
        if matches!(t.thread, "CatThread" | "GnomeGuard") {
            return Some(Condition::flag("grounds1.won"));
        }
        None
    }
    fn event(&mut self, t: &str) -> Option<Events> {
        if self.saved.kind == Kind::Grounds && matches!(t, "CatThread" | "GnomeGuard") {
            if self.saved.phase == Phase::Done {
                if t == "CatThread" {
                    self.saved.cat.get_or_insert(0.);
                } else {
                    self.saved.gnome.get_or_insert(0.);
                }
            }
            return Some(Events::default());
        }
        if matches!(
            t,
            "JLair2_Dead_Jabber"
                | "pickup_eye"
                | "JL1a_End"
                | "JL2a_End"
                | "grounds1_EndCine"
                | "grounds1_StartCine"
                | "JLair2_Cinema1"
                | "Get_ME1"
                | "Get_ME2"
                | "Get_ME3"
                | "Jabber_End"
        ) {
            Some(Events::default())
        } else {
            None
        }
    }
    fn dialogue_complete(&mut self, n: &str) -> Events {
        if n == "CatThread" && self.saved.cat.is_some() && !self.saved.cat_ending {
            self.saved.cat = Some(0.);
            self.saved.cat_ending = true;
        }
        if (self.saved.phase == Phase::Intro && n == self.saved.kind.intro())
            || (self.saved.phase == Phase::Outro && n == self.saved.kind.outro())
        {
            self.saved.dialogue_done = true;
        }
        Events::default()
    }
    fn prepare_player(&mut self, s: &mut Stats, p: &mut Player) {
        self.alive = s.alive();
        if !self.alive {
            return;
        }
        if self.saved.kind == Kind::Grounds
            && self.saved.phase == Phase::Done
            && p.feet.x > 1250.
            && (p.feet.y - 192.).abs() < 200.
        {
            self.saved.door_requested = true;
        }
        if self.saved.phase == Phase::Fight
            && self.saved.essence_wait == 0.
            && (s.sanity() < 100. || s.will() < 100.)
        {
            let d = (p.feet + Vec3::Z * 32. - self.essence_position()).abs();
            if d.x < 36. && d.y < 36. && d.z < 64. {
                s.essence(50.);
                self.saved.essence =
                    (self.saved.essence + 1) % if self.saved.kind == Kind::Lair { 2 } else { 3 };
                self.saved.essence_wait = 8.;
            }
        }
        if self.saved.phase == Phase::Reward
            && !self.saved.eye
            && self.data.volumes["pickup_eye_trigger"].contains(p.feet + Vec3::Z * 20.)
        {
            s.apply(crate::inventory::PickupKind::Weapon(7), 100.);
            self.saved.eye = true;
            self.saved.time = 0.;
        }
    }
    fn dismiss_summons(&self) -> bool {
        true
    }
    fn ignores_watch(&self) -> bool {
        true
    }
    fn lights(&self) -> Vec<crate::lighting::Light> {
        self.saved
            .boss
            .shots
            .iter()
            .filter(|s| s.ended.is_none())
            .take(12)
            .map(|s| crate::lighting::Light {
                position: s.at,
                color: if s.spiral {
                    vec3(1., 1., 0.2)
                } else {
                    vec3(1., 0.3, 0.1)
                },
                radius: if s.spiral { 200. } else { 100. },
                only_models: false,
                flare: false,
            })
            .collect()
    }
    fn sound_state(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        if self.saved.phase == Phase::Fight {
            loops.push(crate::audio::LoopCue {
                id: self.saved.kind.base() + 900,
                path: if matches!(
                    self.saved.boss.action,
                    battle::Action::Fire | battle::Action::FlyFire
                ) {
                    "sound/character/jabberwock/attack4b.wav"
                } else {
                    "sound/character/jabberwock/idle.wav"
                },
                origin: self.saved.boss.at,
                clock: Some(self.saved.boss.time),
            });
        }
        if self.saved.phase == Phase::Death {
            clocks.push(crate::audio::world::Clock {
                key: "grounds1.boss-death",
                time: self.saved.time,
                period: None,
                origin: self.saved.boss.at,
                cues: &[(0., "sound/character/jabberwock/death.wav")],
            });
        }
    }
    fn take_presentation_event(&mut self) -> Option<&'static str> {
        if self.saved.kind != Kind::Grounds {
            return None;
        }
        let sky = if self.saved.phase == Phase::Intro && (23.4..27.45).contains(&self.saved.time) {
            1
        } else if self.saved.phase == Phase::Intro && (31.45..34.85).contains(&self.saved.time) {
            2
        } else {
            0
        };
        if self.saved.sky == sky {
            return None;
        }
        self.saved.sky = sky;
        Some(["JabberSkyLava", "JabberSkyManga", "JabberSkySide"][sky as usize])
    }
    fn checkpoint_requested(&self) -> bool {
        self.saved.checkpoint && self.saved.phase == Phase::Fight
    }
    fn checkpoint_written(&mut self) {
        self.saved.checkpoint = false;
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
        let old_phase = self.saved.phase;
        let old_time = self.saved.time;
        let old_bridge = self
            .poses
            .iter()
            .find(|(m, _, _)| {
                self.data
                    .brushes
                    .iter()
                    .any(|b| b.index == *m && b.name == "drawbridge")
            })
            .copied();
        let rider = old_bridge.is_some_and(|(m, at, q)| {
            Collider::model(map, m, at, q, true).is_ok_and(|b| {
                let tr = b.trace(
                    p.feet + crate::collision::PLAYER_CENTER,
                    p.feet + crate::collision::PLAYER_CENTER - Vec3::Z * 3.,
                    vec3(1., 1., crate::collision::PLAYER_HALF.z),
                );
                p.velocity.z <= 1. && !tr.start_solid && tr.fraction < 1. && tr.normal.z > 0.65
            })
        });
        let dt = dt.min(0.1);
        self.saved.clock = (self.saved.clock + dt).min(86400.);
        self.saved.time = (self.saved.time + dt).min(3600.);
        self.saved.essence_wait = (self.saved.essence_wait - dt).max(0.);
        self.saved.exit.advance(dt);
        if self.saved.door_requested {
            self.saved.door = (self.saved.door + dt).min(2.);
        }
        if let Some(t) = &mut self.saved.cat {
            *t = (*t + dt).min(300.);
        }
        if let Some(t) = &mut self.saved.gnome {
            *t = (*t + dt).min(30.);
        }
        if self.saved.dialogue_done {
            self.saved.after_talk += dt;
        }
        match self.saved.phase {
            Phase::Intro if self.saved.dialogue_done => self.finish_intro(w, p)?,
            Phase::Fight
                if self.saved.kind == Kind::Lair
                    && self.saved.ticks >= 10800
                    && self.data.volumes["end_trigger"].contains(p.feet + Vec3::Z * 20.) =>
            {
                self.saved.boss.shots.clear();
                self.saved.boss.beam = None;
                self.saved.waves.clear();
                self.phase(Phase::Outro);
            }
            Phase::Fight if self.saved.boss.health == 0. => self.phase(Phase::Death),
            Phase::Death => {
                if self.saved.boss.flying {
                    let b = &mut self.saved.boss;
                    let body = b.target(0);
                    let delta = Vec3::Z * (-600. * dt);
                    let tr = w.sweep(body.center, body.center + delta, body.half);
                    if !tr.start_solid {
                        b.at += delta * tr.fraction;
                    }
                    if tr.fraction < 1. && tr.normal.z > 0.65 {
                        b.at.z -= 147.;
                        b.flying = false;
                        b.landed = true;
                    }
                }
                self.saved.boss.time =
                    (self.saved.boss.time + dt).min(self.data.boss().duration("death") + 1.);
                if self.saved.time >= 10. {
                    self.phase(Phase::Outro);
                }
            }
            Phase::Outro
                if self.saved.dialogue_done
                    && self.saved.after_talk
                        >= if self.saved.kind == Kind::Lair {
                            3.
                        } else {
                            12.85
                        } =>
            {
                self.finish_outro(w, p)?
            }
            Phase::Reward if self.saved.eye && self.saved.time >= 2. => {
                self.phase(Phase::Done);
                self.saved.exit.committed = true;
            }
            Phase::Bridge if self.saved.time >= 13. => self.phase(Phase::Done),
            _ => {}
        }
        self.rebuild(map)?;
        if old_phase == Phase::Bridge {
            let mut feet = p.feet;
            if rider {
                if let Some((m, at, q)) = old_bridge {
                    if let Some((_, next, rotation)) = self.poses.iter().find(|(n, _, _)| *n == m) {
                        feet = *next + *rotation * (q.inverse() * (feet - at));
                    }
                }
            }
            w.set_dynamic(fixed.to_vec());
            let sweep = w.body_trace(p.feet, feet);
            w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
            if sweep.start_solid || sweep.fraction < 1. || !w.body_clear(feet) {
                self.saved.phase = old_phase;
                self.saved.time = old_time;
                self.rebuild(map)?;
            } else {
                p.feet = feet;
            }
        }
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        Ok(())
    }
    fn update(&mut self, _: &mut World, _: &Player, _: Vec3, _: bool) -> Events {
        Events {
            transition: self.saved.exit.request(self.saved.kind.exit()),
            ..Default::default()
        }
    }
    fn transition_failed(&mut self, e: &(String, Option<String>)) {
        if self.saved.kind.exit().matches(e) {
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
        if self.saved.phase == Phase::Done {
            self.shapes.clone()
        } else {
            vec![]
        }
    }
    fn scripted(&self) -> bool {
        matches!(self.saved.phase, Phase::Intro | Phase::Outro)
    }
    fn hides_player(&self) -> bool {
        self.scripted()
    }
    fn controlled(&self) -> bool {
        self.scripted()
    }
    fn scene_id(&self) -> Option<&'static str> {
        match self.saved.phase {
            Phase::Intro => Some(self.saved.kind.intro()),
            Phase::Outro => Some(self.saved.kind.outro()),
            _ => None,
        }
    }
    fn entry_story(&mut self, _: &mut Story) -> bool {
        true
    }
    fn prepare_story(&self, s: &mut Story) -> bool {
        if self.saved.cat.is_some_and(|t| t >= 2.)
            && !self.saved.cat_ending
            && !s.has_seen("CatThread")
            && !s.busy()
        {
            s.trigger("CatThread");
        }
        let n = match self.saved.phase {
            Phase::Intro if self.saved.time >= self.intro_talk_start() => {
                Some(self.saved.kind.intro())
            }
            Phase::Outro
                if self.saved.time
                    >= if self.saved.kind == Kind::Lair {
                        10.55
                    } else {
                        0.5
                    } =>
            {
                Some(self.saved.kind.outro())
            }
            _ => None,
        };
        if let Some(n) = n {
            if !self.saved.dialogue_done && !s.busy() {
                if !s.has_seen(n) {
                    if n == "grounds1_EndCine" {
                        s.trigger_gated(n, "grounds1.survived", self.alive);
                    } else {
                        s.trigger(n);
                    }
                } else if !s.sequence_pending(n) {
                    s.resume_scene(n, self.saved.dialogue_line, self.saved.dialogue_time);
                }
            }
        }
        true
    }
    fn sync_story(&mut self, s: &Story) {
        if let Some(n) = self.scene_id() {
            if let Some((i, t)) = s.progress(n) {
                self.saved.dialogue_line = i;
                self.saved.dialogue_time = t;
                if self.saved.kind == Kind::Lair
                    && self.saved.phase == Phase::Intro
                    && i >= 2
                    && self.saved.walk_started.is_none()
                {
                    self.saved.walk_started = Some(self.saved.time);
                }
            }
        }
    }
    fn skip(&mut self, map: &Bsp, w: &mut World, p: &mut Player, s: &mut Story) -> Result<bool> {
        let Some(n) = self.scene_id() else {
            return Ok(false);
        };
        if self.saved.phase == Phase::Intro {
            self.finish_intro(w, p)?;
        } else {
            self.finish_outro(w, p)?;
        }
        s.finish_sequence(n);
        self.rebuild(map)?;
        Ok(true)
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        self.scene_camera()
    }
    fn fade(&self) -> Option<(Color, f32)> {
        let t = self.saved.time;
        let alpha = match self.saved.phase {
            Phase::Intro => (1. - t / 3.).clamp(0., 1.),
            Phase::Death => ((t - 9.) / 1.).clamp(0., 1.),
            Phase::Outro if self.saved.dialogue_done => ((self.saved.after_talk
                - if self.saved.kind == Kind::Lair {
                    1.
                } else {
                    10.85
                })
                / 2.)
                .clamp(0., 1.),
            _ => 0.,
        };
        (alpha > 0.).then_some((BLACK, alpha))
    }
    fn music_mood(&self) -> Option<&'static str> {
        Some(if self.saved.phase == Phase::Fight {
            "action"
        } else {
            "normal"
        })
    }
    fn objective(&self) -> Option<String> {
        Some(match self.saved.phase{Phase::Intro=>"The Jabberwock approaches.",Phase::Fight if self.saved.kind==Kind::Lair=>"Survive the Jabberwock until help arrives. Dodge its fire and defeat the Jabberspawn.",Phase::Fight if self.saved.boss.flying=>"Strike the Jabberwock in the air. Keep moving when it dives.",Phase::Fight=>"Finish the wounded Jabberwock. Dodge its breath and stay clear of its claws.",Phase::Reward=>"Collect the fallen eye to obtain the Eye Staff.",Phase::Bridge=>"The drawbridge is lowering.",Phase::Done=>"Continue through the castle gate.",_=>"The battle is over."}.into())
    }
    fn recovery_entry(&self, normal: (Vec3, f32)) -> (Vec3, f32) {
        if self.saved.phase == Phase::Intro {
            return normal;
        }
        let pose = if self.saved.kind == Kind::Grounds {
            self.data.points["gdeath"]
        } else {
            self.data.points["alice_dead_pos1"]
        };
        (pose.translation, pose.rotation.to_euler(EulerRot::ZYX).0)
    }
    fn targets(&self) -> Vec<Target> {
        if self.saved.phase != Phase::Fight {
            return vec![];
        }
        let mut t = if self.saved.boss.health > 0. {
            vec![self.saved.boss.target(self.saved.kind.base())]
        } else {
            vec![]
        };
        t.extend(
            self.saved
                .waves
                .iter()
                .enumerate()
                .filter(|(_, w)| w.launch >= 1.5 && w.actor.vulnerable())
                .map(|(i, w)| w.actor.target(self.saved.kind.base() + 100 + i)),
        );
        t
    }
    fn hit(&mut self, h: Hit) -> Option<&'static str> {
        if self.saved.phase != Phase::Fight {
            return None;
        }
        if h.id == self.saved.kind.base() {
            return self.saved.boss.hit(h, self.saved.kind);
        }
        let index = h.id.checked_sub(self.saved.kind.base() + 100)?;
        self.saved
            .waves
            .get_mut(index)
            .filter(|w| w.launch >= 1.5)
            .and_then(|w| w.actor.hit(h))
    }
    fn combat(&mut self, c: &mut Combat<'_>) -> Feedback {
        let mut out = Feedback::default();
        if self.saved.phase != Phase::Fight || c.dt <= 0. || !c.stats.alive() {
            return out;
        }
        self.saved.remainder += f64::from(c.dt.min(0.1));
        self.saved.boss.freeze_spirals = c.stats.powers.stopped > 0.;
        while self.saved.remainder + 1e-9 >= 1. / 120. {
            self.saved.remainder = (self.saved.remainder - 1. / 120.).max(0.);
            if self.saved.kind == Kind::Lair {
                self.saved.ticks = (self.saved.ticks + 1).min(10800);
                if (self.saved.wave_count == 0 && self.saved.ticks >= 3600)
                    || (self.saved.wave_count == 1 && self.saved.ticks >= 7200)
                {
                    self.wave();
                }
            }
            self.saved.boss.step(
                self.saved.kind,
                c.world,
                c.player.eye(),
                c.notarget,
                &self.data,
                &mut out,
            );
            for w in &mut self.saved.waves {
                if c.stats.powers.stopped > 0. {
                    continue;
                }
                if w.launch < 1.5 {
                    w.launch = (w.launch + battle::STEP).min(1.5);
                    let t = w.launch / 1.5;
                    w.actor.feet = w.from.lerp(w.to, t) + Vec3::Z * (4. * t * (1. - t) * 120.);
                    if w.launch >= 1.5 {
                        w.actor.place(c.world);
                    }
                } else {
                    w.actor.notarget = c.notarget;
                    let clip = w.actor.clip();
                    let time = w.actor.time;
                    let rig = &self.data.rigs["c_jabberspawn"];
                    for cue in rig.audio.between(
                        clip,
                        crate::audio::events::Span {
                            start: time,
                            end: time + battle::STEP,
                            duration: rig.duration(clip),
                            frame_time: rig.frame(clip),
                            looping: w.actor.loops(),
                            entered: time == 0.,
                        },
                    ) {
                        out.cue_sounds.push((cue.path, w.actor.feet));
                    }
                    w.actor.step(c.world, c.player.eye(), &self.data, &mut out);
                }
            }
        }
        out
    }
    fn loot_sources(&self) -> Vec<crate::loot::Source> {
        self.saved
            .waves
            .iter()
            .enumerate()
            .map(|(i, w)| crate::loot::Source {
                id: self.saved.kind.base() + 100 + i,
                feet: w.actor.feet,
                grade: crate::loot::Grade::Medium,
                dead: w.actor.health <= 0.,
            })
            .collect()
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        let s: Saved = state::load(v, state::Visit { returning: false })?;
        ensure!(s.kind == self.saved.kind, "Wrong Jabberwock battle");
        self.saved = s;
        self.rebuild(map)
    }
    fn upgrade(&self) -> Upgrade {
        Upgrade {
            respawn: Respawn::Always,
            rearm: vec![
                self.saved.kind.intro().into(),
                self.saved.kind.outro().into(),
                "pickup_eye".into(),
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
pub(crate) fn art(a: &mut Assets, o: &dyn LevelController) -> Result<Box<dyn LevelArt>> {
    Ok(Box::new(art::Art::load(
        a,
        o.downcast_ref::<Encounter>()
            .context("Missing Jabberwock owner")?,
    )?))
}
