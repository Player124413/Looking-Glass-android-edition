//! Dry Landing scenes, physical lily/clip movers, river and optional bridge.
mod art;
mod check;
mod data;
mod route;
pub(crate) use route::drive as drive_route;
mod scene;
mod world;
mod world_check;
use super::{state, Check, Registration, Run};
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World},
    event::{Condition, Facts},
    interaction::{Events, Interactions},
    inventory::Stats,
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

pub const ARRIVAL: &str = "Garden1_Start";
pub const TURTLE: &str = "Garden1_Cinema2";
pub const RABBIT: &str = "Garden1_Rabbit_Cinema1";
const BEATS: &[BeatSpec] = &[
    BeatSpec::linear("garden1", TURTLE, "garden1_cinematics", TURTLE, 5),
    BeatSpec::linear("garden1", RABBIT, "garden1_cinematics", RABBIT, 5),
];
const ALICE_CLIPS: &[&str] = &[
    "jump_falling1",
    "pain_knockdown",
    "idle",
    "walk",
    "idle_stand",
    "idle_stand_rocktoes",
    "idle_base_01",
    "idle_base_01_2_base_02",
    "idle_base_02_2_shrug",
    "idle_shrug",
    "idle_shrug_headtilt",
    "idle_shrug_shakeno",
    "idle_shrug_nodyes",
];
const TURTLE_CLIPS: &[&str] = &[
    "idle_base",
    "idle_nosewipe",
    "talk_shrug",
    "idle_horn",
    "jump",
    "swim",
];
const RABBIT_CLIPS: &[&str] = &[
    "i_calm_l",
    "i_calm_eartwitch_l",
    "i_calm_front_2_right",
    "i_calm_right_l",
    "i_calm_right_2_front",
    "i_calm_2_ready",
    "i_ready_l",
    "i_ready_2_calm",
    "run",
    "jump",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Kind {
    Arrival,
    Rabbit,
}
impl Kind {
    fn id(self) -> &'static str {
        match self {
            Self::Arrival => ARRIVAL,
            Self::Rabbit => RABBIT,
        }
    }
    fn dialogue(self) -> &'static str {
        match self {
            Self::Arrival => TURTLE,
            Self::Rabbit => RABBIT,
        }
    }
    fn landing(self) -> &'static str {
        match self {
            Self::Arrival => "turtle_waterpos1",
            Self::Rabbit => "alice_posx1",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    Launch,
    Setup,
    Talk,
    Leave,
}
#[derive(Clone, Serialize, Deserialize)]
struct Scene {
    kind: Kind,
    phase: Phase,
    start: f32,
    clock: SceneState,
    shell_at: Option<f32>,
    skipping: bool,
}
impl Scene {
    fn elapsed(&self) -> f32 {
        self.clock.time - self.start
    }
    fn phase(&mut self, phase: Phase) {
        self.phase = phase;
        self.start = self.clock.time;
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    initialized: bool,
    arrived: bool,
    rabbit_done: bool,
    shell: bool,
    portals: bool,
    scene: Option<Scene>,
    fade: f32,
    #[serde(default)]
    world: world::State,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        self.world.validate()?;
        state::clock("garden handoff fade", self.fade, 0.5)?;
        ensure!(
            !self.rabbit_done || self.arrived,
            "Rabbit completion precedes arrival"
        );
        ensure!(
            self.arrived || self.scene.as_ref().is_some_and(|s| s.kind == Kind::Arrival),
            "Missing arrival lifecycle"
        );
        if let Some(s) = &self.scene {
            s.clock.validate(&spec(s.kind, None))?;
            ensure!(
                !s.clock.finished && s.clock.line < 5 && s.start <= s.clock.time,
                "Invalid garden scene phase"
            );
            state::clock("garden phase start", s.start, 3600.)?;
            ensure!(
                !(s.kind == Kind::Arrival && self.arrived)
                    && !(s.kind == Kind::Rabbit && self.rabbit_done),
                "Completed garden scene replayed"
            );
            ensure!(
                s.kind == Kind::Arrival || (self.arrived && s.phase != Phase::Launch),
                "Rabbit precedes arrival"
            );
            ensure!(
                !s.skipping || s.phase == Phase::Leave,
                "Invalid garden skip phase"
            );
            if let Some(t) = s.shell_at {
                state::clock("garden shell cue", t, s.clock.time)?;
                ensure!(s.kind == Kind::Arrival && self.shell, "Invalid shell cue");
            }
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
            track: "garden1",
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
    movers: world::WorldData,
}
impl Garden {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut out = Self {
            saved: Saved {
                version: 1,
                initialized: false,
                arrived: false,
                rabbit_done: false,
                shell: false,
                portals: false,
                scene: None,
                fade: 0.,
                world: world::State::default(),
            },
            data: data::Data::load(a, map)?,
            movers: world::WorldData::load(map)?,
        };
        out.movers.rebuild(map, &out.saved.world)?;
        out.begin(Kind::Arrival);
        Ok(out)
    }
    fn begin(&mut self, kind: Kind) {
        if self.saved.scene.is_none() {
            self.saved.scene = Some(Scene {
                kind,
                phase: if kind == Kind::Arrival {
                    Phase::Launch
                } else {
                    Phase::Setup
                },
                start: 0.,
                clock: SceneState::new(&spec(kind, None)),
                shell_at: None,
                skipping: false,
            });
        }
    }
    fn finish(&mut self, world: &World, player: &mut Player) -> Result<()> {
        let Some(s) = &mut self.saved.scene else {
            return Ok(());
        };
        let landing = self.data.points[s.kind.landing()];
        SceneRunner {
            spec: &spec(s.kind, Some(landing)),
            state: &mut s.clock,
        }
        .finish(world, player)?;
        match s.kind {
            Kind::Arrival => {
                self.saved.arrived = true;
                self.saved.shell = true;
            }
            Kind::Rabbit => self.saved.rabbit_done = true,
        }
        player.script_motion = 0;
        self.saved.scene = None;
        self.saved.fade = 0.5;
        Ok(())
    }
}
impl LevelController for Garden {
    fn id(&self) -> &'static str {
        "garden1"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("garden1.portals", self.saved.portals);
        f.flag(
            "garden1.rabbit_ready",
            self.saved.arrived && !self.saved.rabbit_done && !self.scripted(),
        );
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        if t.name == "teleporter_start_trigger" {
            Some(Condition::flag("garden1.portals"))
        } else if t.thread == RABBIT {
            Some(Condition::flag("garden1.rabbit_ready"))
        } else {
            None
        }
    }
    fn event(&mut self, name: &str) -> Option<Events> {
        match name {
            RABBIT => {
                if self.saved.arrived && !self.saved.rabbit_done {
                    self.begin(Kind::Rabbit);
                }
            }
            "Open_Portals" => self.saved.portals = true,
            "Bridge_Drop" => {
                self.saved.world.bridge.get_or_insert(self.saved.world.time);
            }
            "Ant_Deadtree_Ambush" => {
                self.saved.world.rabbit.get_or_insert(self.saved.world.time);
            }
            "Garden1_RockFall" | "Start_Boulder1" | "Boulder_Die" => {
                self.saved.world.rock_seen = true
            }
            "Lady_Bug1_On" | "Lady_Bug2_On" | "Lady_Bug3_On" | "Lady_Bug4_On" | "Ant_Ambush1"
            | "Ant_Ambush2" | "Ant_Ambush3" | "Ant_Ambush4" | "Ant_Ambush5" | "Spawn_Ladybug"
            | "Spawn_Ladybug2" | "Spawn_Ladybug3" | "Spawn_Ladybug4" => (),
            _ => return None,
        }
        Some(Events::default())
    }
    fn prepare_player(&mut self, stats: &mut Stats, p: &mut Player) {
        if !self.saved.initialized {
            stats.apply(crate::inventory::PickupKind::Sanity, 100.);
            stats.apply(crate::inventory::PickupKind::Will, 100.);
            self.saved.initialized = true;
        }
        if self.saved.shell {
            stats.turtle_air = true;
        }
        p.breath.shell = stats.turtle_air;
    }
    fn upgraded(&mut self) {
        // An old generic visit is already in progress. Never replay/refill its arrival.
        self.saved.initialized = true;
        self.saved.arrived = true;
        self.saved.scene = None;
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
        if s.phase == Phase::Talk && !story.busy() && !story.has_seen(s.kind.dialogue()) {
            story.trigger(s.kind.dialogue());
        }
        s.phase == Phase::Talk
    }
    fn sync_story(&mut self, story: &Story) {
        if let Some(s) = &mut self.saved.scene {
            if s.phase == Phase::Talk {
                if let Some((line, time)) = story.progress(s.kind.dialogue()) {
                    s.clock.line = line;
                    s.clock.line_time = time;
                    if s.kind == Kind::Arrival && line >= 3 && s.shell_at.is_none() {
                        self.saved.shell = true;
                        s.shell_at = Some(s.clock.time);
                    }
                }
            }
        }
    }
    fn dialogue_complete(&mut self, name: &str) -> Events {
        if let Some(s) = &mut self.saved.scene {
            if s.phase == Phase::Talk && s.kind.dialogue() == name {
                s.phase(Phase::Leave);
            }
        }
        Events::default()
    }
    fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. {
            return Ok(());
        }
        let dt = dt.min(0.1);
        self.saved.fade = (self.saved.fade - dt).max(0.);
        let portals: Vec<_> = self
            .data
            .portals
            .iter()
            .filter(|_| self.saved.portals)
            .map(|(_, _, c)| c.clone())
            .collect();
        self.movers.carry(
            dt,
            map,
            &mut self.saved.world,
            world,
            player,
            fixed,
            &portals,
        )?;
        if let Some(s) = &mut self.saved.scene {
            let mut runner = SceneRunner {
                spec: &spec(s.kind, None),
                state: &mut s.clock,
            };
            runner.capture(player);
            runner.advance(dt);
            player.cancel_climb();
            player.release_rope();
            player.velocity = Vec3::ZERO;
            player.script_motion = 1;
            match s.phase {
                Phase::Launch if s.elapsed() >= self.data.launch_end() => s.phase(Phase::Setup),
                Phase::Setup if s.elapsed() >= if s.kind == Kind::Arrival { 2.6 } else { 0.5 } => {
                    s.phase(Phase::Talk)
                }
                Phase::Leave
                    if s.elapsed()
                        >= if s.skipping {
                            0.5
                        } else if s.kind == Kind::Arrival {
                            4.6
                        } else {
                            self.data.rabbit_run() + 2.5
                        } =>
                {
                    self.finish(world, player)?
                }
                _ => (),
            }
        }
        self.traversal(&mut world.traversal);
        Ok(())
    }
    fn traversal(&self, traversal: &mut crate::traversal::Traversal) {
        traversal.currents = self.data.currents.clone();
        if let Some(p) = traversal.pushes.iter_mut().find(|p| p.id.0 == 117) {
            p.enabled = self.saved.scene.as_ref().is_some_and(|s| {
                s.kind == Kind::Arrival
                    && s.phase == Phase::Launch
                    && s.clock.time >= 2.
                    && !s.skipping
            });
        }
    }
    fn skip(&mut self, _: &Bsp, _: &mut World, _: &mut Player, story: &mut Story) -> Result<bool> {
        let Some(s) = &mut self.saved.scene else {
            return Ok(false);
        };
        if s.skipping {
            return Ok(false);
        }
        if !story.has_seen(s.kind.dialogue()) {
            story.trigger(s.kind.dialogue());
        }
        story.finish_sequence(s.kind.dialogue());
        s.phase(Phase::Leave);
        s.skipping = true;
        Ok(true)
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        self.scene_camera()
    }
    fn fade(&self) -> Option<(Color, f32)> {
        self.scene_fade()
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.movers
            .transforms()
            .into_iter()
            .chain(
                self.data
                    .portals
                    .iter()
                    .filter(|_| self.saved.portals)
                    .map(|(m, p, _)| (*m, p.translation, p.rotation)),
            )
            .collect()
    }
    fn colliders(&self) -> Vec<Collider> {
        self.movers
            .colliders()
            .into_iter()
            .chain(
                self.data
                    .portals
                    .iter()
                    .filter(|_| self.saved.portals)
                    .map(|(_, _, c)| c.clone()),
            )
            .collect()
    }
    fn particles(&self, steam: &mut crate::particles::Steam) {
        // This emitter belongs to the shell cue, not the map's always-on ambience.
        steam.gate(&[(crate::entity::Id(15), false)]);
    }
    fn recovery_entry(&self, normal: (Vec3, f32)) -> (Vec3, f32) {
        if self.saved.arrived {
            (self.data.points["turtle_waterpos1"].translation, 0.)
        } else {
            normal
        }
    }
    fn sound_state(
        &self,
        _: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        let Some(s) = &self.saved.scene else {
            return;
        };
        if s.skipping {
            return;
        }
        if s.kind == Kind::Arrival {
            let (clip, time, _) = self.acting("c_mockturtle");
            let (key, cues): (_, &'static [(f32, &'static str)]) = match clip {
                "idle_nosewipe" => (
                    "garden1.turtle.nosewipe",
                    &[(0., "sound/character/mock_turtle/idle_nosewipe.wav")],
                ),
                "idle_horn" => (
                    "garden1.turtle.horn",
                    &[(0., "sound/character/mock_turtle/idle_horn.wav")],
                ),
                "jump" => (
                    "garden1.turtle.jump",
                    &[(0., "sound/character/mock_turtle/jump.wav")],
                ),
                "swim" => (
                    "garden1.turtle.swim",
                    &[(0., "sound/character/mock_turtle/swim.wav")],
                ),
                _ => return,
            };
            clocks.push(crate::audio::world::Clock {
                key,
                time,
                period: (clip == "swim").then(|| self.data.duration("c_mockturtle", clip)),
                origin: self.turtle_pose().translation,
                cues,
            });
        } else if s.phase == Phase::Leave {
            let (clip, time, _) = self.acting("c_whiterabbit");
            if clip == "jump" {
                clocks.push(crate::audio::world::Clock {
                    key: "garden1.rabbit.jump",
                    time: time - 5. * self.data.frame("c_whiterabbit", "jump"),
                    period: None,
                    origin: self.rabbit_pose().translation,
                    cues: &[(0., "sound/character/white_rabbit/jump.wav")],
                });
            }
        }
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, saved: &serde_json::Value, map: &Bsp) -> Result<()> {
        let saved: Saved = state::load(saved, state::Visit { returning: false })?;
        self.movers.rebuild(map, &saved.world)?;
        self.saved = saved;
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
    id: "garden1",
    applies: |m, e| super::first_visit(m, e, "garden1"),
    load: |a, m, _, _| Ok(Box::new(Garden::load(a, m)?)),
    art: Some(|a, _, _| Ok(Box::new(art::Art::load(a)?))),
    owns_submodel: |m, e| {
        m == "garden1"
            && (world::owns(e)
                || e.get("targetname").is_some_and(|n| {
                    matches!(
                        n.as_str(),
                        "teleporter_start" | "teleport_end" | "alice_shell"
                    )
                }))
    },
    owns_npc: |n, m| {
        (n == "turtle_actor" && m == "c_mockturtle")
            || (matches!(n, "rabbit_actor" | "rabbit_actor2") && m == "c_whiterabbit")
    },
    target_base: Some(7_200_000),
    story_beats: BEATS,
    checks: &[
        Check {
            flag: "--garden1-path-check",
            help: "Movement-only route diagnostics; excludes combat.",
            run: Run::Headless(route::probe),
        },
        Check {
            flag: "--garden1-cast-check",
            help: "Garden ambushes, difficulty and saved cast",
            run: Run::Windowed(|a| Box::pin(async move { crate::npc::garden_check(a) })),
        },
        Check {
            flag: "--garden1-world-check",
            help: "Verify Dry Landing pads, currents, bridge and dead-tree performance.",
            run: Run::Headless(world_check::check),
        },
        Check {
            flag: "--garden1-route-check",
            help: "Traverse Dry Landing with the production enemy cast.",
            run: Run::Windowed(route::check),
        },
        Check {
            flag: "--garden1-skip-route-check",
            help: "Traverse Dry Landing while skipping its scenes.",
            run: Run::Windowed(route::skipped),
        },
        Check {
            flag: "--garden1-check",
            help: "Verify Dry Landing scene triggers, watch/skip, save continuation and gates.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--garden1-render-check",
            help: "Render Dry Landing's staged scene cameras and cast.",
            run: Run::Windowed(check::render),
        },
    ],
    save_cases: &[
        super::SaveCase {
            name: "garden1-world-lily",
            visit: "garden1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden1-world-bridge",
            visit: "garden1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden1-world-deadtree",
            visit: "garden1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden1-world-current",
            visit: "garden1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden1-arrival-launch",
            visit: "garden1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden1-arrival-talk",
            visit: "garden1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden1-arrival-shell",
            visit: "garden1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden1-arrival-swim",
            visit: "garden1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden1-arrival-done",
            visit: "garden1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden1-rabbit-talk",
            visit: "garden1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden1-rabbit-run",
            visit: "garden1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "garden1-rabbit-done",
            visit: "garden1$first",
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
        .context("Dry Landing owner missing")
}
/// Traversal save fixtures start after the arrival, as their original setup did.
pub(crate) fn stage_gameplay(i: &mut Interactions) {
    if let Ok(o) = owner(i) {
        o.upgraded();
    }
}
