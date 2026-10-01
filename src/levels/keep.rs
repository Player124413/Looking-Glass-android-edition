//! Castle Keep: saved mirror/portrait progression and the final Cheshire scene.
mod art;
mod check;
mod data;
mod guards;
mod motion;
mod scene;
mod sound;
use super::{state, Check, Registration, Run};
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    combat::{Guard, Hit, Target},
    event::{Condition, Facts},
    interaction::{self, Events},
    inventory::Stats,
    level::exit::ExitState,
    level::spec::ExitSpec,
    level::{LevelArt, LevelController, TriggerInfo},
    movement::Player,
    skeletal::Transform,
    story::{BeatSpec, Story},
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use scene::{Kind, Scene};
use serde::{Deserialize, Serialize};
use std::any::Any;
const BASE: usize = 9_700_000;
const EXIT: ExitSpec = ExitSpec {
    map: "qlair",
    entrance: "qlair_start1",
};
// Selection order is authored: club, diamond, spade. Correct portraits use the same order.
const SUITS: [&str; 3] = ["club", "diamond", "spade"];
const ANGLES: [f32; 3] = [60., 0., -60.];
#[derive(Clone, Serialize, Deserialize)]
struct Spawned {
    entity: usize,
    guard: guards::Body,
    accumulator: f64,
    electric: f32,
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    elapsed: f32,
    clock: f32,
    initialized: bool,
    arrival: bool,
    hint: bool,
    selected: Option<usize>,
    next: usize,
    mirror_from: f32,
    mirror_time: f32,
    won: [bool; 3],
    room: Option<usize>,
    room_time: f32,
    lost: Option<usize>,
    loss_time: f32,
    win_time: f32,
    smashed: Option<usize>,
    doors: [f32; 5],
    holds: [f32; 3],
    hall: f32,
    #[serde(default)]
    hall_open: f32,
    heart_open: bool,
    heart_crossed: bool,
    heart_scene: bool,
    queen_open: bool,
    death_started: bool,
    death_done: bool,
    sink: f32,
    head: crate::dismember::State,
    scene: Option<Scene>,
    spawned: Vec<Spawned>,
    exit: ExitState,
}
impl Default for Saved {
    fn default() -> Self {
        Self {
            version: 2,
            elapsed: 0.,
            clock: 0.,
            initialized: false,
            arrival: false,
            hint: false,
            selected: None,
            next: 0,
            mirror_from: -45.,
            mirror_time: 5.,
            won: [false; 3],
            room: None,
            room_time: 0.,
            lost: None,
            loss_time: 0.,
            win_time: 0.,
            smashed: None,
            doors: [0.; 5],
            holds: [0.; 3],
            hall: 0.,
            hall_open: 0.,
            heart_open: false,
            heart_crossed: false,
            heart_scene: false,
            queen_open: false,
            death_started: false,
            death_done: false,
            sink: 0.,
            head: Default::default(),
            scene: None,
            spawned: vec![],
            exit: ExitState::default(),
        }
    }
}
impl state::State for Saved {
    const VERSION: u8 = 2;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        for (n, t, max) in [
            ("arrival", self.elapsed, 5.),
            ("world", self.clock, 1e6),
            ("mirror", self.mirror_time, 5.),
            ("room", self.room_time, 1.),
            ("loss", self.loss_time, 1.5),
            ("win", self.win_time, 2.1),
            ("hall", self.hall, 2.),
            ("sink", self.sink, 44.),
            ("exit retry", self.exit.retry_time, 1.),
        ] {
            state::clock(n, t, max)?;
        }
        ensure!(
            self.next < 3
                && self.selected.is_none_or(|v| v < 3)
                && self.room.is_none_or(|v| v < 3 && !self.won[v])
                && self.lost.is_none_or(|v| v < 3 && !self.won[v])
                && self.smashed.is_none_or(|v| v < 3 && self.won[v]),
            "Invalid Keep room identity"
        );
        ensure!(
            self.mirror_from.is_finite() && (-60. ..=60.).contains(&self.mirror_from),
            "Invalid mirror angle"
        );
        for v in self.doors {
            state::clock("door", v, 1.)?;
        }
        state::fraction("hall door", self.hall_open)?;
        for v in self.holds {
            state::clock("door hold", v, 2.)?;
        }
        ensure!(
            self.room.is_none() || (self.room == self.selected && self.lost.is_none()),
            "Unselected portrait room"
        );
        ensure!(
            !self.heart_scene || self.won.iter().all(|v| *v),
            "Premature heart gate"
        );
        ensure!(
            !self.heart_open || self.heart_scene,
            "Heart gate without puzzle"
        );
        ensure!(
            !self.heart_crossed || self.heart_scene,
            "Premature heart contact"
        );
        ensure!(
            !self.death_started || self.heart_scene,
            "Premature final scene"
        );
        ensure!(
            !self.queen_open || self.death_started,
            "Premature Queen doors"
        );
        ensure!(
            !self.death_done || self.death_started,
            "Unstarted death completion"
        );
        ensure!(!self.exit.committed || self.death_done, "Premature exit");
        ensure!(
            self.exit.retry_time == 0. || self.exit.committed,
            "Uncommitted retry"
        );
        self.head.validate(self.death_started)?;
        ensure!(self.spawned.len() <= 5, "Unbounded Keep guard pool");
        let mut ids = std::collections::BTreeSet::new();
        for a in &self.spawned {
            ensure!(ids.insert(a.entity), "Duplicate Keep guard");
            a.guard.validate()?;
            ensure!(
                a.accumulator.is_finite() && (0. ..0.01).contains(&a.accumulator),
                "Invalid guard remainder"
            );
            state::clock("guard electricity", a.electric, crate::electric::LIFE)?;
        }
        if let Some(s) = &self.scene {
            s.validate()?;
            ensure!(
                match s.kind {
                    Kind::Arrival => !self.arrival,
                    Kind::Hint => !self.hint,
                    Kind::Mirror => self.selected.is_some(),
                    Kind::Heart => self.heart_scene,
                    Kind::Death => self.death_started && !self.death_done,
                },
                "Invalid scene progression"
            );
        }
        Ok(())
    }
}
struct Keep {
    saved: Saved,
    data: data::Data,
    motion: motion::Motion,
}
impl Keep {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut k = Self {
            saved: Saved::default(),
            data: data::Data::load(a, map)?,
            motion: motion::Motion::load(map)?,
        };
        k.motion.rebuild(map, &k.saved)?;
        Ok(k)
    }
    fn all_won(&self) -> bool {
        self.saved.won.iter().all(|v| *v)
    }
    fn free(&self) -> bool {
        !self.scripted()
            && !self.saved.death_done
            && !(self.saved.lost.is_some() && self.saved.loss_time < 1.5)
    }
    fn room_ready(&self, i: usize) -> bool {
        self.free()
            && self.saved.room == Some(i)
            && self.saved.room_time == 1.
            && !self.saved.won[i]
    }
    fn mirror_angle(&self) -> f32 {
        motion::mirror_angle(&self.saved)
    }
    fn rotate(&mut self) {
        if !self.free() || self.all_won() {
            return;
        }
        self.saved.mirror_from = self.mirror_angle();
        self.saved.selected = Some(self.saved.next);
        self.saved.next = (self.saved.next + 1) % 3;
        self.saved.mirror_time = 0.;
        self.saved.lost = None;
        self.saved.loss_time = 0.;
        self.saved.room = None;
        self.saved.holds = [0.; 3];
        self.begin(Kind::Mirror);
    }
    fn enter_room(&mut self, i: usize) {
        if self.free()
            && self.saved.selected == Some(i)
            && self.saved.room.is_none()
            && !self.saved.won[i]
            && self.saved.lost.is_none()
        {
            self.saved.room = Some(i);
            self.saved.room_time = 0.;
            self.saved.spawned.clear();
        }
    }
    fn answer(&mut self, i: usize, correct: bool) {
        if !self.room_ready(i) {
            return;
        }
        self.saved.spawned.clear();
        self.saved.room = None;
        if correct {
            self.saved.won[i] = true;
            self.saved.win_time = 0.;
            self.saved.smashed = Some(i);
            self.spawn_group("heart");
        } else {
            self.saved.lost = Some(i);
            self.saved.loss_time = 0.;
            self.saved.holds = [0.; 3];
            self.spawn_group(SUITS[i]);
        }
    }
    fn spawn_group(&mut self, group: &str) {
        self.saved.spawned = self
            .data
            .spawns
            .iter()
            .filter(|(_, g, _)| g == group)
            .map(|(entity, _, guard)| Spawned {
                entity: *entity,
                guard: guard.clone(),
                accumulator: 0.,
                electric: 0.,
            })
            .collect();
    }
    fn lever(&self, w: &World, eye: Vec3, aim: Vec3) -> bool {
        if !self.free() || self.all_won() {
            return false;
        }
        let at = self.data.at("mirror_lever").point(vec3(36., 0., 26.));
        let d = at - eye;
        let t = w.sweep(eye, at, Vec3::splat(0.5));
        d.length() < 125.
            && d.length() > 0.01
            && aim.normalize_or_zero().dot(d.normalize()) > 0.45
            && !t.start_solid
            && t.fraction >= 1.
    }
}
impl LevelController for Keep {
    fn id(&self) -> &'static str {
        "keep"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("keep.never", false);
        f.flag(
            "keep.hint",
            self.free() && !self.saved.hint && !self.saved.death_started,
        );
        f.flag(
            "keep.death",
            self.free() && self.saved.heart_scene && !self.saved.death_started,
        );
        f.flag(
            "keep.heart-contact",
            self.free() && self.saved.heart_scene && !self.saved.heart_crossed,
        );
        f.flag("keep.exit", self.saved.death_done);
        for (i, n) in SUITS.iter().enumerate() {
            f.flag(
                &format!("keep.{n}-entry"),
                self.free()
                    && self.saved.selected == Some(i)
                    && !self.saved.won[i]
                    && self.saved.room.is_none()
                    && self.saved.lost.is_none(),
            );
            f.flag(&format!("keep.{n}-shot"), self.room_ready(i));
            f.flag(
                &format!("keep.{n}-teleport"),
                self.saved.lost == Some(i) && self.saved.loss_time >= 0.5,
            );
            f.flag(
                &format!("keep.{n}-door"),
                self.free()
                    && (self.saved.won[i] || self.saved.selected == Some(i))
                    && self.saved.lost.is_none(),
            );
            f.flag(
                &format!("keep.{n}-return"),
                self.free() && self.saved.won[i],
            );
        }
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        let key = match t.id.0 {
            1 => "hint",
            7 => "heart-contact",
            64 => "death",
            71 => "exit",
            29 => "club-entry",
            30 => "spade-entry",
            31 => "diamond-entry",
            36 => "diamond-teleport",
            466 => "spade-teleport",
            467 => "club-teleport",
            32..=34 => "diamond-shot",
            468..=470 => "spade-shot",
            471..=473 => "club-shot",
            11 => "diamond-door",
            542 => "spade-door",
            543 => "club-door",
            544 => "club-return",
            545 => "diamond-return",
            546 => "spade-return",
            _ => return None,
        };
        Some(Condition::flag(&format!("keep.{key}")))
    }
    fn shootable_thread(&self, n: &str) -> bool {
        data::answer(n).is_some()
    }
    fn repeatable_shot(&self, n: &str) -> bool {
        self.shootable_thread(n)
    }
    fn receivers(&self, _: &crate::entity::Registry) -> Vec<crate::entity::Id> {
        [8, 9, 12, 13, 14, 15, 16, 17]
            .into_iter()
            .map(crate::entity::Id)
            .collect()
    }
    fn rules(&self, c: &crate::level::RuleContext<'_>) -> Vec<crate::event::Rule> {
        self.receivers(c.registry)
            .into_iter()
            .map(|id| crate::event::Rule {
                key: format!("keep/door/{}", id.0),
                event: crate::event::Event::Entity(id, crate::event::Input::Activate),
                condition: Condition::Always,
                once: false,
                cooldown: 0.,
                actions: vec![crate::event::Action::Output(
                    crate::event::Effect::Activate(id),
                )],
            })
            .collect()
    }
    fn output(&mut self, e: &crate::event::Effect) -> Option<Events> {
        if let crate::event::Effect::Activate(id) = e {
            match id.0 {
                14 | 15 => self.saved.holds[0] = 2.,
                16 | 17 => self.saved.holds[1] = 2.,
                12 | 13 => self.saved.holds[2] = 2.,
                8 | 9 => {
                    if !self.saved.heart_crossed && self.saved.heart_scene {
                        self.saved.heart_open = !self.saved.heart_open;
                        self.saved.heart_crossed = true;
                    }
                }
                _ => return None,
            }
            return Some(Events::default());
        }
        None
    }
    fn event(&mut self, n: &str) -> Option<Events> {
        let mut e = Events::default();
        if let Some((i, correct)) = data::answer(n) {
            self.answer(i, correct);
            e.sound = Some(
                if correct {
                    "sound/ambience/special/stone_breaking1.wav"
                } else {
                    "sound/ambience/weather/thunder_boom01.wav"
                }
                .into(),
            );
            return Some(e);
        }
        match n {
            "Start_Keep" => self.begin(Kind::Arrival),
            "Lever_Cat_Dialog" => {
                if self.free() && !self.saved.hint {
                    self.begin(Kind::Hint)
                }
            }
            "Keep_Rotate_Mirror" => self.rotate(),
            "Door_Club_Close" => self.enter_room(0),
            "Door_Diamond_Close" => self.enter_room(1),
            "Door_Spade_Close" => self.enter_room(2),
            "Keep_Cheshire_Dead" => {
                if self.free() && self.saved.heart_scene && !self.saved.death_started {
                    self.saved.death_started = true;
                    self.begin(Kind::Death);
                }
            }
            _ => return None,
        }
        if n.starts_with("Door_") {
            e.sound = Some("sound/ambience/weather/thunder_boom01.wav".into());
        }
        Some(e)
    }
    fn prepare_player(&mut self, _: &mut Stats, _: &mut Player) {
        if !self.saved.initialized {
            // Entering from the departure lift preserves carried resources.
            self.saved.initialized = true;
        }
    }
    fn update(&mut self, w: &mut World, p: &Player, aim: Vec3, use_pressed: bool) -> Events {
        if use_pressed && self.lever(w, p.eye(), aim) {
            self.rotate();
        }
        if p.feet.y > 1180. && p.feet.y < 1550. && (p.feet.x - 512.).abs() < 220. {
            self.saved.hall = 2.;
        }
        Events {
            transition: self.saved.exit.request(EXIT),
            ..Default::default()
        }
    }
    fn prompt(&self, w: &World, eye: Vec3, aim: Vec3) -> Option<&'static str> {
        self.lever(w, eye, aim).then_some("E / Turn the mirror")
    }
    fn exit_contact(&mut self, d: &(String, Option<String>)) -> Option<Events> {
        EXIT.matches(d).then(|| Events {
            transition: self.saved.exit.request(EXIT),
            ..Default::default()
        })
    }
    fn transition_failed(&mut self, d: &(String, Option<String>)) {
        if EXIT.matches(d) {
            self.saved.exit.failed();
        }
    }
    fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. || !dt.is_finite() {
            return Ok(());
        }
        let dt = dt.min(0.1);
        self.saved.clock = (self.saved.clock + dt).min(1e6);
        self.saved.exit.advance(dt);
        if self.saved.room.is_some() {
            self.saved.room_time = (self.saved.room_time + dt).min(1.);
        }
        if self.saved.lost.is_some() {
            self.saved.loss_time = (self.saved.loss_time + dt).min(1.5);
        }
        if self.saved.smashed.is_some() {
            self.saved.win_time = (self.saved.win_time + dt).min(2.1);
        }
        if self.all_won()
            && self.saved.win_time == 2.1
            && !self.saved.heart_scene
            && self.saved.scene.is_none()
        {
            self.saved.heart_scene = true;
            self.saved.heart_open = true;
            self.begin(Kind::Heart);
        }
        self.advance_scene(dt, w, p)?;
        self.motion.advance(dt, map, w, p, fixed, &mut self.saved)?;
        Ok(())
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.motion.transforms(&self.saved)
    }
    fn colliders(&self) -> Vec<Collider> {
        self.motion.colliders()
    }
    fn reflection(&self) -> (Vec<(usize, Vec3, Quat)>, Vec<usize>) {
        self.motion.reflection(&self.saved)
    }
    fn sky_origin(&self) -> Option<Vec3> {
        Some(
            self.data
                .at(
                    if self.saved.room.is_some()
                        || (self.saved.smashed.is_some() && self.saved.win_time < 2.)
                        || (self.saved.lost.is_some() && self.saved.loss_time < 1.)
                    {
                        "grounds_sky"
                    } else {
                        "black_sky"
                    },
                )
                .translation,
        )
    }
    fn targets(&self) -> Vec<Target> {
        if self.scripted() {
            return vec![];
        }
        self.saved
            .spawned
            .iter()
            .filter(|a| a.guard.health() > 0.)
            .map(|a| a.guard.target(BASE + a.entity))
            .collect()
    }
    fn hit(&mut self, h: Hit) -> Option<&'static str> {
        self.saved
            .spawned
            .iter_mut()
            .find(|a| BASE + a.entity == h.id)
            .and_then(|a| {
                if h.kind.means() == crate::combat::DamageKind::Electric {
                    a.electric = crate::electric::LIFE;
                }
                a.guard.hit(h)
            })
    }
    fn combat(&mut self, c: &mut crate::level::Combat<'_>) -> crate::combat::Feedback {
        self.combat_guards(c)
    }
    fn loot_sources(&self) -> Vec<crate::loot::Source> {
        self.saved.spawned.iter().map(|a|crate::loot::Source{id:BASE+a.entity,feet:a.guard.feet(),grade:if matches!(&a.guard,guards::Body::Card(g) if g.kind==crate::cards::Kind::Heart){crate::loot::Grade::Large}else{crate::loot::Grade::Medium},dead:a.guard.health()==0.}).collect()
    }
    fn scripted(&self) -> bool {
        self.saved.scene.is_some()
    }
    fn controlled(&self) -> bool {
        self.scripted()
    }
    fn allow_cheshire(&self) -> bool {
        !self.saved.death_started && !self.scripted()
    }
    fn scene_id(&self) -> Option<&'static str> {
        self.saved.scene.as_ref().map(|s| s.kind.id())
    }
    fn entry_story(&mut self, _: &mut Story) -> bool {
        self.begin(Kind::Arrival);
        true
    }
    fn prepare_story(&self, s: &mut Story) -> bool {
        self.prepare_scene_story(s)
    }
    fn sync_story(&mut self, s: &Story) {
        self.sync_scene_story(s)
    }
    fn dialogue_complete(&mut self, n: &str) -> Events {
        self.complete_dialogue(n);
        Events::default()
    }
    fn skip(&mut self, _: &Bsp, w: &mut World, p: &mut Player, story: &mut Story) -> Result<bool> {
        let Some(s) = &self.saved.scene else {
            return Ok(false);
        };
        let id = s.kind.dialogue();
        if s.kind == Kind::Death {
            self.saved.scene.as_mut().unwrap().skip.get_or_insert(0.);
        } else {
            self.finish_scene(w, p, true)?;
        }
        if let Some(id) = id {
            story.finish_sequence(id);
        }
        Ok(true)
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        self.scene_camera()
    }
    fn fade(&self) -> Option<(Color, f32)> {
        self.scene_fade()
    }
    fn objective(&self) -> Option<String> {
        Some(if self.saved.death_started{"Continue to the Queen's Lair."}else if self.all_won(){"Pass through the opened heart gate."}else{"Turn the mirror. Match each suit to the portrait it replaces, then strike that portrait in its room."}.into())
    }
    fn sound_state(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        self.collect_sound(loops, clocks);
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        let mut saved = if v.get("version").and_then(|v| v.as_u64()) == Some(1) {
            let elapsed = v
                .get("elapsed")
                .and_then(|v| v.as_f64())
                .context("Missing arrival clock")? as f32;
            state::clock("legacy arrival", elapsed, 5.)?;
            Saved {
                elapsed,
                arrival: true,
                initialized: true,
                ..Default::default()
            }
        } else {
            state::load::<Saved>(v, state::Visit { returning: false })?
        };
        if v.get("hall_open").is_none() {
            // Older snapshots derived this pose directly from the hold timer.
            saved.hall_open = (saved.hall / 0.5).min(1.);
        }
        for a in &saved.spawned {
            let (_, _, g) = self
                .data
                .spawns
                .iter()
                .find(|(id, _, _)| *id == a.entity)
                .context("Foreign Keep guard")?;
            ensure!(g.identity() == a.guard.identity(), "Changed guard identity");
        }
        self.motion.rebuild(map, &saved)?;
        self.saved = saved;
        Ok(())
    }
    fn upgraded(&mut self) {
        self.saved.initialized = true;
        self.saved.arrival = true;
    }
    fn upgrade(&self) -> crate::level::Upgrade {
        crate::level::Upgrade {
            rearm: vec!["Lever_Cat_Dialog".into(), "Keep_Cheshire_Dead".into()],
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
    id: "keep",
    applies: |m, e| super::first_visit(m, e, "keep"),
    load: |a, m, _, _| Ok(Box::new(Keep::load(a, m)?)),
    art: Some(|a, _, _| Ok(Box::new(art::Art::load(a)?))),
    owns_submodel: |m, e| m == "keep" && motion::owned(e),
    owns_npc: |n, _| {
        matches!(
            n,
            "lift_cat"
                | "lever_cat"
                | "cheshire_actor1"
                | "cheshire_actor2"
                | "cheshire_actor3"
                | "cat_popup"
                | "cat_head1"
                | "mirror_lever"
        )
    },
    target_base: Some(BASE),
    story_beats: &[
        BeatSpec::linear("keep", "Start_Keep", "keep_cinematics", "Start_Keep", 1),
        BeatSpec::linear(
            "keep",
            "Lever_Cat_Dialog",
            "keep_cinematics",
            "Lever_Cat_Dialog",
            1,
        ),
        BeatSpec::linear(
            "keep",
            "Keep_Cheshire_Dialog",
            "keep_cinematics",
            "Keep_Cheshire_Dialog",
            4,
        ),
    ],
    checks: &[
        Check {
            flag: "--keep-check",
            help: "Verify Keep puzzle, scenes, saves and gated exit.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--keep-route-check",
            help: "Walk the complete Castle Keep route.",
            run: Run::Headless(check::route),
        },
        Check {
            flag: "--keep-skip-route-check",
            help: "Walk Keep with scene skips.",
            run: Run::Headless(check::skip_route),
        },
        Check {
            flag: "--keep-render-check",
            help: "Capture Keep mirror and final scene.",
            run: Run::Windowed(check::render),
        },
    ],
    save_cases: check::SAVES,
    visibility: &[],
};

pub(crate) use check::drive as drive_route;
