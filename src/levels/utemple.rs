//! One owner for the temple guide, physical set pieces, breath sources and exit.
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, Liquid, World, PLAYER_CENTER, PLAYER_HALF},
    event::{Condition, Facts},
    interaction::{Events, Interactions},
    inventory::Stats,
    level::{
        scene::{SceneRunner, SceneState},
        spec::{EndSpec, ExitSpec, SceneSpec, ShotSpec},
        LevelArt, LevelController, Respawn, TriggerClass, TriggerInfo, Upgrade,
    },
    movement::Player,
    skeletal::Transform,
    story::Story,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::{any::Any, collections::BTreeMap};
mod art;
mod check;
pub(crate) use check::drive as drive_route;
mod data;
mod motion;
mod swim_check;
mod verify;
pub use check::stage;
const ID: &str = "Utemple_Exit_Cinematic";
const EXIT: ExitSpec = ExitSpec {
    map: "garden1",
    entrance: "garden1_start1",
};
const SCENE: SceneSpec = SceneSpec {
    id: ID,
    version: 1,
    duration: 7.,
    shots: &[ShotSpec {
        start: 0.,
        track: "utemple_path1",
        offset: 0.,
        hold: 7.,
    }],
    cues: &[],
    end: EndSpec {
        landing: None,
        exit: Some(EXIT),
    },
};

#[derive(Clone, Serialize, Deserialize)]
struct Bubble {
    at: Vec3,
    born: f64,
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    initialized: bool,
    clock: f64,
    guide: Option<f64>,
    node: usize,
    events: BTreeMap<String, f64>,
    movers: BTreeMap<String, f32>,
    clams: [Option<f64>; 4],
    fishhead: Option<f32>,
    pulse: [bool; 4],
    open: bool,
    scene: Option<SceneState>,
    next_bubble: f64,
    bubbles: Vec<Bubble>,
    contacts: u64,
    crush_cooldown: f32,
    crush_damage: f32,
}
impl Default for Saved {
    fn default() -> Self {
        Self {
            version: 1,
            initialized: false,
            clock: 0.,
            guide: None,
            node: 0,
            events: BTreeMap::new(),
            movers: BTreeMap::new(),
            clams: [None, None, Some(0.), None],
            fishhead: None,
            pulse: [false; 4],
            open: false,
            scene: None,
            next_bubble: 0.,
            bubbles: Vec::new(),
            contacts: 0,
            crush_cooldown: 0.,
            crush_damage: 0.,
        }
    }
}
pub struct Temple {
    saved: Saved,
    data: data::Data,
    motion: motion::Motion,
}
impl Temple {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        Ok(Self {
            saved: Saved::default(),
            data: data::Data::load(a, map)?,
            motion: motion::Motion::load(map)?,
        })
    }
    fn turtle_pose(&self, clock: f64) -> Transform {
        self.saved.guide.map_or(self.data.turtle, |start| {
            self.data
                .guide
                .curve
                .sample((clock - start).max(0.) as f32, true)
        })
    }
    fn fire(&mut self, name: &str) {
        if self.saved.events.contains_key(name) {
            return;
        }
        self.saved.events.insert(name.into(), self.saved.clock);
        motion::activate(&mut self.saved, name);
        match name {
            "ClamAttack1" => self.saved.clams[0] = Some(-3.),
            "ClamAttack2" => self.saved.clams[1] = Some(-3.),
            "ClamAttack4" => self.saved.clams[3] = Some(0.),
            "breakwall" => self.saved.open = true,
            _ => {}
        }
    }
    fn emitter_pose(&self, name: &str, base: Vec3) -> Transform {
        let bound = if name == "column1_bubbles2" {
            Some("column1b")
        } else {
            name.strip_suffix("_bubbles")
                .or_else(|| name.strip_suffix("_bubbles2"))
                .or_else(|| name.strip_suffix("_bubbles3"))
        };
        if let Some(o) = bound.and_then(|b| self.motion.objects.iter().find(|o| o.name == b)) {
            if self.saved.movers.get(&o.name).is_some_and(|t| *t >= 0.) {
                return Transform {
                    translation: o.pose.point(base - o.base),
                    rotation: o.pose.rotation,
                };
            }
        }
        Transform {
            translation: base,
            rotation: Quat::IDENTITY,
        }
    }
    fn emitter_visible(&self, name: &str) -> bool {
        if name == "panel11_bubbles" {
            return false;
        }
        if name.starts_with("endbubbles") {
            return self
                .saved
                .events
                .get("breakwall")
                .is_some_and(|t| self.saved.clock - t < 1.);
        }
        let base = name.split("_bubbles").next().unwrap_or(name);
        if let Some(t) = self.saved.movers.get(base) {
            return *t >= 0. && *t < 9.;
        }
        !matches!(
            base,
            "column1" | "column2" | "column3c" | "column3" | "column6" | "panel12" | "spike2"
        ) && !name.starts_with("column1_bubbles")
    }
    fn emit(&mut self) {
        while self.saved.next_bubble <= self.saved.clock {
            let born = self.saved.next_bubble;
            let at = self
                .data
                .shell_pose(self.turtle_pose(born), born as f32)
                .translation;
            self.saved.bubbles.push(Bubble { at, born });
            for (_, name, base, air) in &self.data.emitters {
                // Visibility does not stop the original server animation events.
                if *air {
                    self.saved.bubbles.push(Bubble {
                        at: self.emitter_pose(name, *base).translation,
                        born,
                    });
                }
            }
            self.saved.next_bubble += 1.;
        }
        self.saved
            .bubbles
            .retain(|b| self.saved.clock - b.born < 4.);
    }
    fn breathe(&mut self, p: &mut Player) {
        let c = p.feet + PLAYER_CENTER;
        let (lo, hi) = self.data.bubble_bounds;
        let touch = self.saved.bubbles.iter().any(|b| {
            (b.at + lo).cmple(c + PLAYER_HALF).all() && (b.at + hi).cmpge(c - PLAYER_HALF).all()
        });
        if self.saved.guide.is_none() || self.scripted() || touch {
            if touch && p.breath.submerged > 0.1 {
                self.saved.contacts += 1;
            }
            p.breath.refill();
        }
    }
    fn guard_state(&self) -> Result<()> {
        let s = &self.saved;
        ensure!(
            s.version == 1
                && s.clock.is_finite()
                && (0.0..=86400.).contains(&s.clock)
                && s.node <= 160,
            "Invalid temple state"
        );
        ensure!(
            s.guide
                .is_none_or(|t| t.is_finite() && (0.0..=s.clock).contains(&t)),
            "Invalid guide clock"
        );
        ensure!(
            s.events.len() <= 20
                && s.events
                    .iter()
                    .all(|(n, t)| valid_event(n) && t.is_finite() && (0.0..=s.clock).contains(t)),
            "Invalid temple event history"
        );
        ensure!(
            s.open == s.events.contains_key("breakwall") && (!s.open || s.guide.is_some()),
            "Invalid temple exit gate"
        );
        ensure!(
            s.movers.len() <= 37
                && s.movers
                    .iter()
                    .all(|(n, t)| self.motion.objects.iter().any(|o| &o.name == n)
                        && t.is_finite()
                        && (-10.0..=60.).contains(t)),
            "Invalid temple mover clocks"
        );
        ensure!(
            s.clams
                .iter()
                .flatten()
                .all(|t| t.is_finite() && (-3.0..=86400.).contains(t))
                && s.fishhead
                    .is_none_or(|t| t.is_finite() && (0.0..=1.5).contains(&t)),
            "Invalid trap clocks"
        );
        ensure!(
            s.next_bubble.is_finite()
                && s.next_bubble >= s.clock
                && s.next_bubble <= s.clock + 1.001
                && s.bubbles.len() <= 32
                && s.bubbles.iter().all(|b| b.at.is_finite()
                    && b.at.abs().max_element() < 100000.
                    && b.born.is_finite()
                    && b.born >= 0.
                    && b.born <= s.clock
                    && s.clock - b.born <= 4.001),
            "Invalid breath points"
        );
        ensure!(
            s.crush_cooldown.is_finite()
                && (0.0..=0.5).contains(&s.crush_cooldown)
                && s.crush_damage.is_finite()
                && (0.0..=100_000.).contains(&s.crush_damage),
            "Invalid crush cooldown"
        );
        if let Some(scene) = &s.scene {
            ensure!(s.open, "Scene bypassed wall gate");
            scene.validate(&SCENE)?;
        }
        Ok(())
    }
}
fn valid_event(n: &str) -> bool {
    matches!(
        n,
        "Fish2Dart"
            | "Spike1Fall"
            | "Spike3Fall"
            | "ClamAttack1"
            | "ClamAttack2"
            | "Column3Fall"
            | "FishCalm"
            | "Panels"
            | "ClamAttack4"
            | "Pillar1Fall"
            | "Panels2"
            | "Pillar2Fall"
            | "Panels4"
            | "breakwall"
            | "Column1Fall"
            | "Panel12Fall"
            | "Panels3"
            | "Column6Fall"
    )
}
impl LevelController for Temple {
    fn id(&self) -> &'static str {
        "utemple"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("utemple.exit_ready", self.saved.open && !self.scripted());
        let active = self.saved.guide.is_some() && !self.scripted();
        f.flag("utemple.hazards", active);
        for n in 0..4 {
            let on = self.saved.pulse[n]
                || if n >= 2 {
                    self.saved.clams[n].is_none_or(|t| t < 1.01)
                } else {
                    false
                };
            f.flag(&format!("utemple.oyster{}", n + 1), active && on);
        }
        f.flag(
            "utemple.fishhead",
            active && self.saved.fishhead.is_some_and(|t| (0.5..1.).contains(&t)),
        );
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        if t.thread == ID {
            return Some(Condition::flag("utemple.exit_ready"));
        }
        if t.class == TriggerClass::Hurt {
            return Some(Condition::flag(
                &if let Some(n) = t.name.strip_prefix("oyster_hurt") {
                    format!("utemple.oyster{n}")
                } else if t.name == "fishhead_hurt" {
                    "utemple.fishhead".into()
                } else {
                    "utemple.hazards".into()
                },
            ));
        }
        None
    }
    fn event(&mut self, n: &str) -> Option<Events> {
        if n == "startturtle" {
            self.saved.guide.get_or_insert(self.saved.clock);
        } else if n == ID {
            if self.saved.open && self.saved.scene.is_none() {
                self.saved.scene = Some(SceneState::new(&SCENE));
            }
        } else if n == "FishHeadAttack" {
            if self.saved.guide.is_some() && !self.scripted() {
                self.saved.fishhead = Some(0.);
            }
        } else if matches!(
            n,
            "Column1Fall" | "Panel12Fall" | "Panels3" | "Panels4" | "Column6Fall"
        ) {
            self.fire(n);
        } else {
            return None;
        }
        Some(Events::default())
    }
    fn prepare_player(&mut self, stats: &mut Stats, p: &mut Player) {
        if !self.saved.initialized {
            stats.restore();
            self.saved.initialized = true;
        }
        stats.arrival_grants("utemple");
        stats.prepare_player(p);
        self.breathe(p);
    }
    fn upgraded(&mut self) {
        self.saved.initialized = true;
    }
    fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        world: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. {
            return Ok(());
        }
        let dt = dt.min(0.1);
        self.saved.clock = (self.saved.clock + dt as f64).min(86400.);
        self.saved.crush_cooldown = (self.saved.crush_cooldown - dt).max(0.);
        if let Some(start) = self.saved.guide {
            let parameter = self
                .data
                .guide
                .curve
                .parameter((self.saved.clock - start) as f32);
            let reached = (parameter.floor() as isize + 1).clamp(0, 159) as usize;
            if reached >= self.saved.node {
                for i in self.saved.node..=reached {
                    let n = self.data.guide.nodes[i].1.clone();
                    if !n.is_empty() {
                        self.fire(&n);
                    }
                }
                self.saved.node = reached + 1;
            }
        }
        let old_clams = self.saved.clams;
        for n in 0..4 {
            self.saved.pulse[n] = false;
            if let Some(t) = &mut self.saved.clams[n] {
                let old = *t;
                *t = (*t + dt as f64).min(86400.);
                self.saved.pulse[n] = *t >= 1.
                    && (old < 1. || ((*t - 1.) / 4.91).floor() > ((old - 1.) / 4.91).floor());
            }
        }
        if let Some(t) = &mut self.saved.fishhead {
            *t += dt;
            if *t >= 1.5 {
                self.saved.fishhead = None;
            }
        }
        let old = self.saved.movers.clone();
        for t in self.saved.movers.values_mut() {
            *t = (*t + dt).min(60.);
        }
        self.motion.rebuild(map, &self.saved)?;
        world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        world.set_dynamic_liquids(self.liquids());
        if !self.scripted() && !world.body_clear(p.feet) {
            let moving = self
                .motion
                .objects
                .iter()
                .filter(|o| {
                    o.collider
                        .touches(p.feet + PLAYER_CENTER, p.feet + PLAYER_CENTER, PLAYER_HALF)
                        && (self.saved.movers.contains_key(&o.name)
                            || o.name.starts_with("oyster_block"))
                })
                .map(|o| o.damage)
                .fold(0., f32::max);
            let mut clear = None;
            'escape: for radius in [2., 4., 8., 16., 32., 64.] {
                for direction in [
                    Vec3::Z,
                    Vec3::NEG_Z,
                    Vec3::X,
                    Vec3::NEG_X,
                    Vec3::Y,
                    Vec3::NEG_Y,
                ] {
                    let q = p.feet + direction * radius;
                    if world.body_clear(q) && self.data.world.body_trace(p.feet, q).fraction == 1. {
                        clear = Some(q);
                        break 'escape;
                    }
                }
            }
            if let Some(q) = clear {
                p.feet = q;
            } else {
                self.saved.movers = old;
                self.saved.clams = old_clams;
                self.saved.pulse = [false; 4];
                self.motion.rebuild(map, &self.saved)?;
                world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
            }
            if moving > 0. && self.saved.crush_cooldown == 0. {
                self.saved.crush_damage += moving;
                self.saved.crush_cooldown = 0.5;
            }
        }
        self.emit();
        self.breathe(p);
        if let Some(s) = &mut self.saved.scene {
            p.velocity = Vec3::ZERO;
            p.cancel_climb();
            p.release_rope();
            p.script_motion = 1;
            let mut runner = SceneRunner {
                spec: &SCENE,
                state: s,
            };
            runner.capture(p);
            if runner.advance(dt) {
                runner.finish(world, p)?;
            }
        }
        Ok(())
    }
    fn update(&mut self, _: &mut World, _: &Player, _: Vec3, _: bool) -> Events {
        let mut e = Events {
            damage: std::mem::take(&mut self.saved.crush_damage),
            ..Default::default()
        };
        if let Some(s) = &mut self.saved.scene {
            e.transition = s.exit.request(EXIT);
        }
        e
    }
    fn transition_failed(&mut self, e: &(String, Option<String>)) {
        if EXIT.matches(e) {
            if let Some(s) = &mut self.saved.scene {
                s.exit.failed();
            }
        }
    }
    fn scripted(&self) -> bool {
        self.saved.scene.is_some()
    }
    fn controlled(&self) -> bool {
        self.scripted()
    }
    fn allow_cheshire(&self) -> bool {
        false
    }
    fn scene_id(&self) -> Option<&'static str> {
        self.saved
            .scene
            .as_ref()
            .filter(|s| !s.finished)
            .map(|_| ID)
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        self.saved
            .scene
            .as_ref()
            .map(|s| self.data.camera.camera(s.time))
    }
    fn skip(&mut self, _: &Bsp, w: &mut World, p: &mut Player, _: &mut Story) -> Result<bool> {
        let Some(s) = &mut self.saved.scene else {
            return Ok(false);
        };
        SceneRunner {
            spec: &SCENE,
            state: s,
        }
        .finish(w, p)
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.motion.transforms(&self.saved)
    }
    fn colliders(&self) -> Vec<Collider> {
        self.motion.colliders(&self.saved)
    }
    fn liquids(&self) -> Vec<Liquid> {
        vec![self.motion.water.clone()]
    }
    fn particles(&self, steam: &mut crate::particles::Steam) {
        steam.gate(
            &self
                .data
                .emitters
                .iter()
                .map(|(id, n, _, _)| (crate::entity::Id(*id), self.emitter_visible(n)))
                .collect::<Vec<_>>(),
        );
        steam.place(
            &self
                .data
                .emitters
                .iter()
                .map(|(id, n, p, _)| (crate::entity::Id(*id), self.emitter_pose(n, *p)))
                .collect::<Vec<_>>(),
        );
    }
    fn objective(&self) -> Option<String> {
        Some(
            if self.saved.open {
                "Swim through the opened temple passage."
            } else {
                "Follow the Turtle; touch his bubble trail to refill your air."
            }
            .into(),
        )
    }
    fn sound_state(
        &self,
        _: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        clocks.push(crate::audio::world::Clock {
            key: "utemple.turtle.swim",
            time: self.saved.clock as f32,
            period: Some(self.data.swim_duration()),
            origin: self.turtle_pose(self.saved.clock).translation,
            cues: &[(0., "sound/character/mock_turtle/swim.wav")],
        });
    }
    fn snapshot(&self) -> serde_json::Value {
        serde_json::to_value(&self.saved).unwrap()
    }
    fn restore(&mut self, s: &serde_json::Value, map: &Bsp) -> Result<()> {
        self.saved = serde_json::from_value(s.clone())?;
        self.guard_state()?;
        self.motion.rebuild(map, &self.saved)
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
pub static REGISTRATION: super::Registration = super::Registration {
    id: "utemple",
    applies: |m, e| super::first_visit(m, e, "utemple"),
    load: |a, m, _, _| Ok(Box::new(Temple::load(a, m)?)),
    art: Some(|a, _, l| {
        Ok(Box::new(art::Art::load(
            a,
            l.downcast_ref::<Temple>()
                .context("Temple art owner missing")?,
        )?))
    }),
    owns_submodel: |m, e| {
        m == "utemple" && e.get("classname").is_some_and(|s| s == "script_object")
    },
    owns_npc: |n, _| {
        n == "turtle"
            || n == "fake_alice"
            || n.starts_with("fish_school")
            || n == "fish1"
            || n == "lanternfish"
    },
    target_base: None,
    story_beats: &[],
    checks: &[
        super::Check {
            flag: "--utemple-check",
            help: "Verify temple guide, breath, water, movers and ending.",
            run: super::Run::Headless(check::check),
        },
        super::Check {
            flag: "--utemple-route-check",
            help: "Swim the temple with the guide and watch the exit.",
            run: super::Run::Headless(|a| check::route(a, false)),
        },
        super::Check {
            flag: "--utemple-skip-route-check",
            help: "Swim the temple and skip the exit.",
            run: super::Run::Headless(|a| check::route(a, true)),
        },
        super::Check {
            flag: "--utemple-render-check",
            help: "Render temple guide and exit samples.",
            run: super::Run::Windowed(|a| Box::pin(check::render(a))),
        },
    ],
    save_cases: &[
        super::SaveCase {
            name: "utemple-waiting",
            visit: "utemple$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "utemple-mid-guide",
            visit: "utemple$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "utemple-mid-collapse",
            visit: "utemple$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "utemple-oyster-closed",
            visit: "utemple$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "utemple-brush-water",
            visit: "utemple$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "utemple-exit-scene",
            visit: "utemple$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "utemple-exit-done",
            visit: "utemple$first",
            stage: None,
            behavior: None,
        },
    ],
    visibility: &[],
};
