//! About Face: the clock arena, Mad Hatter and Gryphon departure.
mod art;
mod battle;
mod check;
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
    story::Story,
};
use anyhow::{ensure, Context, Result};
use data::Data;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;
const BASE: usize = 8_400_000;
const GRYPHON: &str = "Hatter2_Gryphon_Cinema1";
const EXIT: crate::level::spec::ExitSpec = crate::level::spec::ExitSpec {
    map: "jlair1",
    entrance: "jlair1_start1",
};
fn owns_brush(e: &super::Entity) -> bool {
    matches!(
        e.get("classname").map(String::as_str),
        Some("script_object" | "func_rotatingdoor" | "func_smashablewall")
    )
}
pub static REGISTRATION: super::Registration = super::Registration {
    id: "hatter2",
    applies: |m, e| super::first_visit(m, e, "hatter2"),
    load: |a, m, _, _| Ok(Box::new(Encounter::load(a, m)?)),
    art: Some(|a, _, o| {
        Ok(Box::new(art::Art::load(
            a,
            o.downcast_ref::<Encounter>()
                .context("Missing Hatter owner")?,
        )?))
    }),
    owns_submodel: |_, e| owns_brush(e),
    owns_npc: |n, m| {
        matches!(m, "c_madhatter" | "c_gryphon" | "altar_eyestaff_blade")
            || matches!(n, "hatter_cat" | "watch_cat")
    },
    target_base: Some(BASE),
    story_beats: &[
        crate::story::registry::BeatSpec::linear(
            "hatter2",
            "hatter_cat",
            "../hatter2",
            "hatter_cat",
            1,
        ),
        crate::story::registry::BeatSpec {
            map: "hatter2",
            event: "ExitTest",
            script: "../hatter2",
            thread: "ExitTest",
            source_lines: 1,
            calls: crate::story::registry::Calls::Gated {
                key: "hatter2.rewards",
                indices: &[0],
            },
        },
        crate::story::registry::BeatSpec::linear("hatter2", GRYPHON, "../hatter2", GRYPHON, 5),
    ],
    checks: &[
        super::Check {
            flag: "--hatter2-check",
            help: "Verify Hatter attacks, clock phases, lifts, rewards and restoration.",
            run: super::Run::Headless(check::check),
        },
        super::Check {
            flag: "--hatter2-route-check",
            help: "Play About Face through the Hatter and Gryphon exit.",
            run: super::Run::Headless(check::route),
        },
        super::Check {
            flag: "--hatter2-render-check",
            help: "Capture Hatter combat, machinery and Gryphon scenes.",
            run: super::Run::Windowed(check::render),
        },
    ],
    save_cases: &[],
    visibility: &[],
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    Arrival,
    Cat,
    Fight,
    Victory,
    WatchCat,
    ExitReady,
    Gryphon,
    Done,
}
#[derive(Clone, Serialize, Deserialize)]
struct Wave {
    rewarded: bool,
    actor: crate::clockwork::Automaton,
    launch: f32,
    from: Vec3,
    to: Vec3,
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    phase: Phase,
    time: f32,
    clock: f32,
    cycle: u32,
    remainder: f64,
    boss: battle::Boss,
    waves: Vec<Wave>,
    drops: Vec<(Vec3, f32)>,
    bridge: Option<f32>,
    lifts: f32,
    door: f32,
    initialized: bool,
    blade: bool,
    watch: bool,
    checkpoint: bool,
    talk_done: bool,
    line: usize,
    line_time: f32,
    after_talk: f32,
    scene_from: Transform,
    essence: usize,
    essence_wait: f32,
    essence_live: bool,
    exit: crate::level::exit::ExitState,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        for (n, t, max) in [
            ("scene", self.time, 3600.),
            ("world", self.clock, 86400.),
            ("lifts", self.lifts, 8.),
            ("door", self.door, 1.4),
            ("dialogue", self.line_time, 300.),
            ("after dialogue", self.after_talk, 3600.),
            ("healing", self.essence_wait, 10.),
            ("exit retry", self.exit.retry_time, 1.),
        ] {
            state::clock(n, t, max)?;
        }
        ensure!(
            self.cycle < 11520
                && self.remainder.is_finite()
                && (0. ..0.009).contains(&self.remainder)
                && self.line <= 4
                && self.essence < 4
                && self.waves.len() <= 2,
            "Invalid Hatter counters"
        );
        ensure!(
            self.bridge.is_none_or(|t| (0. ..=1.).contains(&t))
                && self.scene_from.translation.is_finite()
                && self.scene_from.rotation.is_finite()
                && (self.scene_from.rotation.length_squared() - 1.).abs() < 0.01,
            "Invalid Hatter transforms"
        );
        self.boss.validate()?;
        ensure!(
            self.drops.len() <= 4
                && self
                    .drops
                    .iter()
                    .all(|(p, t)| p.is_finite() && (0. ..70.).contains(t)),
            "Invalid reinforcement drops"
        );
        for w in &self.waves {
            w.actor.validate()?;
            ensure!(
                (0. ..=1.5).contains(&w.launch) && w.from.is_finite() && w.to.is_finite(),
                "Invalid reinforcement"
            );
        }
        let won = matches!(
            self.phase,
            Phase::Victory | Phase::WatchCat | Phase::ExitReady | Phase::Gryphon | Phase::Done
        );
        ensure!(
            !won || self.boss.action == battle::Action::Gone,
            "Premature Hatter victory"
        );
        ensure!(
            !matches!(
                self.phase,
                Phase::WatchCat | Phase::ExitReady | Phase::Gryphon | Phase::Done
            ) || (self.blade && self.watch),
            "Missing Hatter rewards"
        );
        ensure!(
            self.exit.committed == (self.phase == Phase::Done),
            "Invalid Gryphon departure"
        );
        Ok(())
    }
}
struct Encounter {
    data: Data,
    saved: Saved,
    poses: Vec<(usize, Vec3, Quat)>,
    shapes: Vec<Collider>,
    alive: bool,
    stopped: bool,
    easy: bool,
}
impl Encounter {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let data = Data::load(a, map)?;
        let boss = battle::Boss::new(data.points["tower_up"]);
        let mut o = Self {
            data,
            saved: Saved {
                version: 1,
                phase: Phase::Arrival,
                time: 0.,
                clock: 0.,
                cycle: 0,
                remainder: 0.,
                boss,
                waves: vec![],
                drops: vec![],
                bridge: None,
                lifts: 0.,
                door: 0.,
                initialized: false,
                blade: false,
                watch: false,
                checkpoint: false,
                talk_done: false,
                line: 0,
                line_time: 0.,
                after_talk: 0.,
                scene_from: Transform {
                    translation: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                essence: 0,
                essence_wait: 0.,
                essence_live: false,
                exit: Default::default(),
            },
            poses: vec![],
            shapes: vec![],
            alive: true,
            stopped: false,
            easy: false,
        };
        o.rebuild(map)?;
        Ok(o)
    }
    fn phase(&mut self, p: Phase) {
        self.saved.phase = p;
        self.saved.time = 0.;
        self.saved.talk_done = false;
        self.saved.line = 0;
        self.saved.line_time = 0.;
        self.saved.after_talk = 0.;
    }
    fn start_fight(&mut self) {
        self.phase(Phase::Fight);
        self.saved.cycle = 0;
        if !self.saved.boss.dying() {
            self.saved.boss.set(battle::Action::Idle);
        }
        self.saved.checkpoint = true;
    }
    fn won(&self) -> bool {
        matches!(
            self.saved.phase,
            Phase::Victory | Phase::WatchCat | Phase::ExitReady | Phase::Gryphon | Phase::Done
        )
    }
    fn scale(&self) -> f32 {
        if self.saved.boss.dying() {
            return if self.saved.boss.action == battle::Action::Gone {
                0.
            } else {
                1.
            };
        }
        if self.saved.phase != Phase::Fight {
            return 1.;
        }
        let t = self.saved.cycle as f32 / 120.;
        let up_delay = self.data.boss().duration("ready_2_stand");
        let warp = if t < 2.5 {
            Some(t)
        } else if (48. + up_delay..50.5 + up_delay).contains(&t) {
            Some(t - 48. - up_delay)
        } else {
            None
        };
        warp.map_or(1., |t| {
            if t < 0.5 {
                (1. - t * 2.).max(0.)
            } else if t < 2. {
                0.
            } else {
                ((t - 2.) * 2.).min(1.)
            }
        })
    }
    fn spawn_wave(&mut self) {
        self.saved.waves.clear();
        for (n, to) in [("clockwork_east", "t85"), ("clockwork_west", "t86")] {
            let p = self.data.points[n];
            let mut landing = self.data.points[to].translation;
            landing = self
                .data
                .arena
                .actor_footing(landing, Vec3::Z * 43., crate::clockwork::HALF, 512.)
                .unwrap_or(landing);
            self.saved.waves.push(Wave {
                rewarded: false,
                actor: crate::clockwork::Automaton::new(
                    p.translation,
                    p.rotation.to_euler(EulerRot::ZYX).0,
                    1.,
                    self.saved.waves.len() + 31,
                    true,
                ),
                launch: 0.,
                from: p.translation,
                to: landing,
            });
        }
    }
    fn essence_position(&self) -> Vec3 {
        if self.easy && self.data.healing.len() > 1 {
            self.data.healing[self.saved.essence.min(self.data.healing.len() - 1)]
        } else {
            self.data
                .healing
                .last()
                .copied()
                .unwrap_or(vec3(5664., 768., 48.))
        }
    }
    fn finish_escape(&mut self) {
        self.saved.phase = Phase::Done;
        self.saved.time = self.saved.time.max(11.6);
        self.saved.talk_done = true;
        self.saved.after_talk = 10.85;
        self.saved.exit.committed = true;
    }
    fn finish_cat(&mut self) {
        if self.saved.phase == Phase::Cat {
            self.start_fight()
        } else if self.saved.phase == Phase::WatchCat {
            self.phase(Phase::ExitReady)
        }
    }
}
impl LevelController for Encounter {
    fn id(&self) -> &'static str {
        "hatter2"
    }
    fn facts(&self) -> crate::event::Facts {
        let mut f = crate::event::Facts::default();
        f.flag(
            "hatter2.exit",
            self.won() && self.saved.blade && self.saved.watch && self.saved.door >= 1.4,
        );
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<crate::event::Condition> {
        if t.class == TriggerClass::Teleport || t.thread == GRYPHON {
            Some(crate::event::Condition::flag("hatter2.exit"))
        } else {
            None
        }
    }
    fn event(&mut self, t: &str) -> Option<Events> {
        match t {
            "drawbridgeMove" => {
                self.saved.bridge.get_or_insert(0.);
            }
            "hatter_cat" => {
                if self.saved.phase == Phase::Arrival {
                    self.phase(Phase::Cat);
                }
            }
            GRYPHON => {
                if self.saved.phase == Phase::ExitReady && self.saved.door >= 1.4 {
                    self.phase(Phase::Gryphon);
                    self.saved.waves.clear();
                    self.saved.boss.shots.clear();
                }
            }
            "ExitTest" | "EndPlats" | "Get_ME1" | "Get_ME2" | "Get_ME3" | "Get_ME4" => {}
            _ => return None,
        }
        Some(Events::default())
    }
    fn dialogue_complete(&mut self, n: &str) -> Events {
        if self.scene_id() == Some(n) {
            self.saved.talk_done = true;
        }
        Events::default()
    }
    fn prepare_player(&mut self, s: &mut Stats, p: &mut Player) {
        self.alive = s.alive();
        self.stopped = s.powers.stopped > 0.;
        self.easy = s.difficulty == crate::powerups::Difficulty::Easy;
        if !self.alive {
            return;
        }
        if !self.saved.initialized {
            s.full_stats();
            self.saved.initialized = true;
        }
        self.saved.watch |= s.collected.contains(&self.data.watch_key);
        self.saved.blade |= s.collected.contains("hatter2:11");
        if !self.saved.blade
            && (p.feet + Vec3::Z * 20. - self.data.blade)
                .abs()
                .cmple(vec3(40., 40., 64.))
                .all()
        {
            self.saved.blade = true;
            s.collected.insert("hatter2:11".into());
        }
        if self.saved.essence_live
            && (s.sanity() < 100. || s.will() < 100.)
            && (p.feet + Vec3::Z * 32. - self.essence_position())
                .abs()
                .cmple(vec3(36., 36., 64.))
                .all()
        {
            s.essence(25.);
            self.saved.essence_live = false;
            if self.easy {
                self.saved.essence = (self.saved.essence + 1) % self.data.healing.len().min(4);
                self.saved.essence_wait = 10.;
            }
        }
        self.saved.drops.retain(|(at, age)| {
            if (s.sanity() < 100. || s.will() < 100.)
                && (p.feet + Vec3::Z * 24. - *at)
                    .abs()
                    .cmple(vec3(32., 32., 56.))
                    .all()
            {
                s.essence(if *age < 10. {
                    50.
                } else if *age < 30. {
                    25.
                } else {
                    15.
                });
                false
            } else {
                true
            }
        });
        if self.saved.phase == Phase::Victory && self.saved.blade && self.saved.watch {
            self.phase(Phase::WatchCat);
        }
        if self.saved.phase == Phase::ExitReady {
            self.saved.scene_from = Transform {
                translation: p.feet,
                rotation: Quat::from_rotation_z(p.script_facing),
            };
        }
    }
    fn upgraded(&mut self) {
        self.saved.initialized = true;
    }
    fn particles(&self, steam: &mut crate::particles::Steam) {
        steam.gate(&[
            (crate::entity::Id(11), false),
            (crate::entity::Id(12), false),
        ]);
    }
    fn ignores_watch(&self) -> bool {
        true
    }
    fn dismiss_summons(&self) -> bool {
        true
    }
    fn allow_cheshire(&self) -> bool {
        false
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
        let dt = dt.min(0.1);
        self.saved.exit.advance(dt);
        if self.stopped && !self.scripted() {
            return Ok(());
        }
        let old_bridge = self.saved.bridge;
        let old_lifts = self.saved.lifts;
        let old_door = self.saved.door;
        let old_poses = self.poses.clone();
        let old_shapes = self.shapes.clone();
        self.saved.clock = (self.saved.clock + dt).min(86400.);
        self.saved.time = (self.saved.time + dt).min(3600.);
        if let Some(t) = &mut self.saved.bridge {
            *t = (*t + dt).min(1.);
        }
        if self.won() {
            self.saved.lifts = (self.saved.lifts + dt) % 8.;
        }
        if matches!(
            self.saved.phase,
            Phase::ExitReady | Phase::Gryphon | Phase::Done
        ) {
            self.saved.door = (self.saved.door + dt).min(1.4);
        }
        if self.saved.essence_wait > 0. {
            self.saved.essence_wait = (self.saved.essence_wait - dt).max(0.);
            if self.saved.essence_wait == 0. {
                self.saved.essence_live = true;
            }
        }
        if self.saved.talk_done {
            self.saved.after_talk = (self.saved.after_talk + dt).min(3600.);
        }
        if matches!(self.saved.phase, Phase::Cat | Phase::WatchCat)
            && self.saved.talk_done
            && self.saved.after_talk >= 3.
        {
            self.finish_cat();
        }
        if self.saved.phase == Phase::Gryphon
            && self.saved.talk_done
            && self.saved.after_talk >= 10.85
        {
            self.finish_escape();
        }
        self.rebuild(map)?;
        // Carry a standing rider with the same pose used for drawing. Roll back blocked machinery.
        if !self.scripted() {
            let mut feet = p.feet;
            for (i, b) in self.data.brushes.iter().enumerate() {
                if !matches!(b.name.as_str(), "end_plat1" | "end_plat2" | "drawbridge") {
                    continue;
                }
                let old = old_poses[i];
                let next = self.poses[i];
                if old == next {
                    continue;
                }
                let hull = self.brush_collider(map, b, old.1, old.2)?;
                let tr = hull.trace(
                    p.feet + crate::collision::PLAYER_CENTER,
                    p.feet + crate::collision::PLAYER_CENTER - Vec3::Z * 3.,
                    vec3(1., 1., crate::collision::PLAYER_HALF.z),
                );
                if p.velocity.z <= 1. && !tr.start_solid && tr.fraction < 1. && tr.normal.z > 0.65 {
                    feet = next.1 + next.2 * (old.2.inverse() * (p.feet - old.1));
                }
            }
            w.set_dynamic(fixed.to_vec());
            let tr = w.body_trace(p.feet, feet);
            w.set_dynamic(fixed.iter().cloned().chain(self.shapes.clone()).collect());
            if tr.start_solid || tr.fraction < 1. || !w.body_clear(feet) {
                self.saved.bridge = old_bridge;
                self.saved.lifts = old_lifts;
                self.saved.door = old_door;
                self.poses = old_poses;
                self.shapes = old_shapes;
            } else {
                p.feet = feet;
            }
        }
        w.set_dynamic(fixed.iter().cloned().chain(self.shapes.clone()).collect());
        Ok(())
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
    fn traversal(&self, t: &mut crate::traversal::Traversal) {
        for p in &mut t.pushes {
            if matches!(p.id.0, 54 | 175) {
                p.enabled =
                    self.saved.phase == Phase::Fight && (6000..6144).contains(&self.saved.cycle);
            }
        }
    }
    fn scripted(&self) -> bool {
        matches!(self.saved.phase, Phase::Gryphon | Phase::Done)
    }
    fn controlled(&self) -> bool {
        self.scripted()
    }
    fn hides_player(&self) -> bool {
        self.scripted()
    }
    fn scene_id(&self) -> Option<&'static str> {
        match self.saved.phase {
            Phase::Cat => Some("hatter_cat"),
            Phase::WatchCat => Some("ExitTest"),
            Phase::Gryphon => Some(GRYPHON),
            _ => None,
        }
    }
    fn entry_story(&mut self, _: &mut Story) -> bool {
        true
    }
    fn prepare_story(&self, s: &mut Story) -> bool {
        if let Some(n) = self.scene_id() {
            let start = if n == GRYPHON { 11.6 } else { 2. };
            if self.saved.time >= start && !self.saved.talk_done && !s.busy() {
                if !s.has_seen(n) {
                    if n == "ExitTest" {
                        s.trigger_gated(
                            n,
                            "hatter2.rewards",
                            self.won() && self.saved.blade && self.saved.watch,
                        );
                    } else {
                        s.trigger(n);
                    }
                } else if !s.sequence_pending(n) {
                    s.resume_scene(n, self.saved.line, self.saved.line_time);
                }
            }
        }
        true
    }
    fn sync_story(&mut self, s: &Story) {
        if let Some(n) = self.scene_id() {
            if let Some((i, t)) = s.progress(n) {
                self.saved.line = i;
                self.saved.line_time = t;
            }
        }
    }
    fn skip(&mut self, _: &Bsp, _: &mut World, _: &mut Player, s: &mut Story) -> Result<bool> {
        let Some(n) = self.scene_id() else {
            return Ok(false);
        };
        s.finish_sequence(n);
        if self.saved.phase == Phase::Gryphon {
            self.finish_escape();
        } else {
            self.finish_cat();
        }
        Ok(true)
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        self.scene_camera()
    }
    fn objective(&self) -> Option<String> {
        Some(match self.saved.phase{
  Phase::Arrival|Phase::Cat=>"Cross the bridge and approach the blade at the centre of the clock.",
  Phase::Fight if self.saved.boss.dying()=>"Stand back while the Hatter breaks down.",
  Phase::Fight if self.saved.cycle>=5760=>"Avoid the Clockworks and the falling tea. The Hatter will return with the clock.",
  Phase::Fight=>"Defeat the Hatter. Dodge his cups and syringes, and keep clear of his cane.",
  Phase::Victory if !self.saved.blade=>"Collect the blade from the centre of the clock.",
  Phase::Victory=>"Ride a moving clock platform to collect the Pocket Watch.",
  Phase::WatchCat=>"Listen to the Cheshire Cat.",Phase::ExitReady=>"Continue through the doors beyond the Watch.",_=>"Escape with the Gryphon."}.into())
    }
    fn music_mood(&self) -> Option<&'static str> {
        Some(if self.saved.phase == Phase::Fight {
            "action"
        } else {
            "normal"
        })
    }
    fn targets(&self) -> Vec<Target> {
        if self.scripted() {
            return vec![];
        };
        let mut t = vec![];
        if !self.saved.boss.dying() && self.scale() > 0.25 {
            t.push(self.saved.boss.target());
        }
        t.extend(
            self.saved
                .waves
                .iter()
                .enumerate()
                .filter(|(_, w)| w.launch >= 1.5 && w.actor.health > 0.)
                .map(|(i, w)| w.actor.target(BASE + 100 + i)),
        );
        t
    }
    fn hit(&mut self, h: Hit) -> Option<&'static str> {
        if h.id == BASE && !self.scripted() && self.scale() > 0.25 {
            self.saved.boss.hit(h);
            None
        } else {
            self.saved
                .waves
                .get_mut(h.id.checked_sub(BASE + 100)?)
                .filter(|w| w.launch >= 1.5)
                .and_then(|w| w.actor.hit(h))
        }
    }
    fn combat(&mut self, c: &mut Combat<'_>) -> Feedback {
        let mut out = Feedback::default();
        if c.dt <= 0. || !c.stats.alive() || self.scripted() {
            return out;
        }
        self.saved.remainder += f64::from(c.dt.min(0.1));
        while self.saved.remainder + 1e-9 >= 1. / 120. {
            self.saved.remainder = (self.saved.remainder - 1. / 120.).max(0.);
            let stopped = c.stats.powers.stopped > 0.;
            self.saved.boss.projectile_step(
                battle::STEP,
                c.world,
                c.player.eye(),
                stopped,
                &mut out,
            );
            if stopped {
                continue;
            }
            if self.saved.phase == Phase::Fight && !self.saved.boss.dying() {
                let tick = self.saved.cycle;
                if tick % 2880 == 0 {
                    out.spatial_sounds.push((
                        "sound/ambience/special/clock_gong_2.wav",
                        self.data.points["tower_down"].translation,
                    ));
                }
                if tick == 0 {
                    self.saved.boss.set(battle::Action::Idle);
                }
                if tick == 240 {
                    self.saved.boss.at = self.data.points["tower_down"].translation;
                    self.saved.boss.set(battle::Action::Ready);
                    self.saved.essence = 0;
                    self.saved.essence_live = true;
                    self.saved.essence_wait = 0.;
                }
                if tick == 5760 {
                    self.saved.boss.set(battle::Action::Stand);
                }
                let up =
                    5760 + (self.data.boss().duration("ready_2_stand") * 120.).ceil() as u32 + 240;
                if tick == up {
                    self.saved.boss.at = self.data.points["tower_up"].translation;
                    self.saved.boss.set(battle::Action::Idle);
                }
                if tick == 6024 {
                    self.spawn_wave();
                }
                if tick == 9000 {
                    for w in &mut self.saved.waves {
                        w.actor.hit(Hit {
                            id: 0,
                            damage: 1000.,
                            kind: crate::combat::DamageKind::Other,
                            knockback: Vec3::ZERO,
                        });
                    }
                    let centre = self.data.points["hatter_splash"].translation;
                    let eye = c.player.eye();
                    let ray = c.world.sweep(centre + Vec3::Z * 24., eye, Vec3::ZERO);
                    if centre.distance(eye) < 300. && !ray.start_solid && ray.fraction >= 1. {
                        out.damage += 200.;
                        out.impulse += (eye - centre).normalize_or_zero() * 200.;
                    }
                }
                self.saved.cycle = (tick + 1) % 11520;
            }
            for (_, age) in &mut self.saved.drops {
                *age += battle::STEP;
            }
            self.saved.drops.retain(|(_, t)| *t < 70.);
            let mobile =
                self.saved.phase == Phase::Fight && (300..5760).contains(&self.saved.cycle);
            let early = self.saved.phase == Phase::Arrival
                && self.saved.boss.at.distance(c.player.eye()) < 500.;
            self.saved.boss.step(
                c.world,
                c.player.eye(),
                mobile || early,
                mobile,
                c.notarget,
                &self.data,
                &mut out,
            );
            for wave in &mut self.saved.waves {
                if wave.launch < 1.5 {
                    wave.launch = (wave.launch + battle::STEP).min(1.5);
                    let t = wave.launch / 1.5;
                    wave.actor.feet =
                        wave.from.lerp(wave.to, t) + Vec3::Z * (4. * t * (1. - t) * 100.);
                } else {
                    wave.actor.notarget = c.notarget;
                    wave.actor
                        .update(battle::STEP, c.world, c.player.eye(), &self.data, &mut out);
                }
            }
            for wave in &mut self.saved.waves {
                if wave.actor.health == 0. && !wave.rewarded {
                    wave.rewarded = true;
                    if self.saved.drops.len() < 4 {
                        self.saved.drops.push((wave.actor.feet + Vec3::Z * 20., 0.));
                    }
                }
            }
            if self.saved.boss.action == battle::Action::Gone && !self.won() {
                self.saved.waves.clear();
                self.saved.essence_live = false;
                self.phase(Phase::Victory);
            }
        }
        out
    }
    fn sound_state(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        _: &mut Vec<crate::audio::world::Clock>,
    ) {
        if self.saved.boss.action == battle::Action::Malfunction {
            loops.push(crate::audio::LoopCue {
                id: BASE + 900,
                path: "sound/character/mad_hatter/death_malfunction.wav",
                origin: self.saved.boss.at,
                clock: Some(self.saved.boss.time),
            });
        }
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        let next = state::load(v, state::Visit { returning: false })?;
        let old = std::mem::replace(&mut self.saved, next);
        if let Err(e) = self.rebuild(map) {
            self.saved = old;
            return Err(e);
        }
        Ok(())
    }
    fn upgrade(&self) -> Upgrade {
        Upgrade {
            respawn: Respawn::Always,
            rearm: vec!["hatter_cat".into(), "drawbridgeMove".into(), GRYPHON.into()],
            ..Default::default()
        }
    }
    fn recovery_entry(&self, normal: (Vec3, f32)) -> (Vec3, f32) {
        if self.saved.bridge.is_some() {
            (vec3(5664., -160., 48.), std::f32::consts::FRAC_PI_2)
        } else {
            normal
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub(crate) use check::drive as drive_route;
