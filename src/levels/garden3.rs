//! Rolling Stones: one scene owner and the W9 marble that continues into play.
mod art;
mod check;
mod data;
mod motion;
mod route;
pub(crate) use route::drive as drive_route;
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
    story::Story,
};
use anyhow::{ensure, Context, Result};
pub use check::stage;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;
const BASE: usize = 7_400_000;
const RELEASE: f32 = 19.5;
const GRAVITY_CHANGE: f32 = 24.5;
const END: f32 = 25.;
const ALICE: &[&str] = &["idle", "ready", "run"];
const BUG: &[&str] = &["special_cheering_middle", "fly_normal", "fly_fast"];
const DOORS: [usize; 4] = [44, 45, 46, 49];
fn spec(landing: Option<Transform>) -> SceneSpec {
    SceneSpec {
        id: "Garden3_Start",
        version: 1,
        duration: END,
        shots: &[
            ShotSpec {
                start: 0.,
                track: "garden3_path1",
                offset: 0.,
                hold: 4.,
            },
            ShotSpec {
                start: 4.,
                track: "garden3_path2",
                offset: 4.,
                hold: 8.,
            },
            ShotSpec {
                start: 8.,
                track: "garden3_path3",
                offset: 0.,
                hold: 8.5,
            },
            ShotSpec {
                start: 16.5,
                track: "garden3_path4",
                offset: 0.,
                hold: 2.5,
            },
            ShotSpec {
                start: 19.,
                track: "garden3_path5",
                offset: 0.,
                hold: 6.,
            },
        ],
        cues: &[],
        end: EndSpec {
            landing,
            exit: None,
        },
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    age: f32,
    scene: Option<SceneState>,
    skip: bool,
    arrived: bool,
    legacy: bool,
    rock: crate::falling_rock::State,
    gravity: f32,
    speed: f32,
    intro_ticks: u16,
    doors: [motion::Door; 4],
    first_quake: Option<f32>,
    pillar: Option<f32>,
    second_quake: Option<f32>,
    ice: Option<f32>,
    end: Option<f32>,
    bounce: Option<f32>,
    damage: f32,
    impulse: Vec3,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        state::clock("garden3 age", self.age, 1e6)?;
        for t in [
            self.first_quake,
            self.pillar,
            self.second_quake,
            self.ice,
            self.end,
            self.bounce,
        ]
        .into_iter()
        .flatten()
        {
            state::clock("garden3 event", t, self.age)?;
        }
        ensure!(
            (self.gravity == 0.6 || self.gravity == 0.2) && self.speed == 175.,
            "Invalid marble gravity"
        );
        ensure!(
            self.damage.is_finite()
                && (0. ..=999.).contains(&self.damage)
                && self.impulse.is_finite()
                && self.impulse.length() <= 126.,
            "Invalid marble contact"
        );
        for d in &self.doors {
            d.validate()?;
        }
        ensure!(
            self.first_quake.is_some() == (self.rock.callbacks > 1)
                && self.pillar.is_some() == (self.rock.callbacks > 31)
                && self.second_quake.is_some() == (self.rock.callbacks > 32)
                && (self.rock.callbacks <= 54 || self.ice.is_some()),
            "Chase callback history mismatch"
        );
        if let Some(s) = &self.scene {
            s.validate(&spec(None))?;
            ensure!(
                !self.arrived
                    && !s.finished
                    && (s.time - self.age).abs() < 0.001
                    && !self.rock.solid,
                "Inconsistent introduction"
            );
            ensure!(
                self.gravity == if s.time < GRAVITY_CHANGE { 0.6 } else { 0.2 },
                "Premature gravity change"
            );
            ensure!(
                self.rock.started == (s.time >= RELEASE) && self.rock.visible == (s.time >= 18.5),
                "Invalid marble release"
            );
            ensure!(
                self.intro_ticks <= 660
                    && (self.rock.elapsed - f32::from(self.intro_ticks) / 120.).abs() < 0.001
                    && self.intro_ticks
                        == (((s.time - RELEASE).max(0.) * 120. + 0.0001).floor() as u16).min(660),
                "Inconsistent release clock"
            );
        } else {
            ensure!(
                self.arrived
                    && !self.skip
                    && !self.legacy
                    && self.intro_ticks == 660
                    && self.rock.started
                    && self.gravity == 0.2
                    && self.first_quake.is_some(),
                "Incomplete chase handoff"
            );
        }
        Ok(())
    }
}
struct Garden {
    saved: Saved,
    data: data::Data,
    objects: Vec<motion::Object>,
    shroom: Option<(Transform, Collider)>,
}
impl Garden {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let data = data::Data::load(a, map)?;
        let saved = Saved {
            version: 1,
            age: 0.,
            scene: Some(SceneState::new(&spec(None))),
            skip: false,
            arrived: false,
            legacy: false,
            rock: crate::falling_rock::State::new(&data.rock, false, false),
            gravity: 0.6,
            speed: 175.,
            intro_ticks: 0,
            doors: std::array::from_fn(|i| motion::Door::new(i == 0 || i == 2)),
            first_quake: None,
            pillar: None,
            second_quake: None,
            ice: None,
            end: None,
            bounce: None,
            damage: 0.,
            impulse: Vec3::ZERO,
        };
        let mut g = Self {
            saved,
            data,
            objects: motion::load(map)?,
            shroom: None,
        };
        g.rebuild(map)?;
        Ok(g)
    }
    fn callback(&mut self, n: &str) -> bool {
        let slot = match n {
            "Garden3_FirstQuake" => &mut self.saved.first_quake,
            "Garden3_Pillar_Break1" => &mut self.saved.pillar,
            "Garden3_Quake1" => &mut self.saved.second_quake,
            "Garden3_IceFloor_Break1" => &mut self.saved.ice,
            "Garden3_Fall_End" => &mut self.saved.end,
            _ => return false,
        };
        slot.get_or_insert(self.saved.age);
        true
    }
    fn rock_step(&mut self, dt: f32, player: Option<&Player>) {
        if dt <= 0. {
            return;
        }
        self.data.rock.gravity = 800. * self.saved.gravity;
        let bodies = player.map(|p| crate::combat::Target {
            id: crate::dice::ALICE,
            center: p.feet + crate::collision::PLAYER_CENTER,
            half: crate::collision::PLAYER_HALF,
        });
        let out = self
            .data
            .rock
            .advance(&mut self.saved.rock, dt, bodies.as_slice());
        for (_, damage, impulse) in out.hits {
            self.saved.damage = self.saved.damage.max(damage);
            self.saved.impulse = impulse;
        }
        if !out.arrivals.is_empty() {
            self.saved.bounce = Some(self.saved.age);
        }
        for n in out.threads {
            self.callback(&n);
        }
    }
    fn intro_step(&mut self, _from: f32, to: f32) {
        if to >= 18.5 {
            self.saved.rock.visible = true;
        }
        if to >= RELEASE && !self.saved.rock.started {
            self.data.rock.gravity = 480.;
            self.data.rock.activate(&mut self.saved.rock);
            self.saved.rock.solid = false;
        }
        // A single saved fixed-step cursor drives the on-camera hazard. Skip seeks
        // this same instance forward, never restarts it or repeats a callback.
        let ticks = (((to - RELEASE).max(0.) * 120. + 0.0001).floor() as u16).min(660);
        while self.saved.intro_ticks < ticks {
            self.saved.gravity = if self.saved.intro_ticks < 600 {
                0.6
            } else {
                0.2
            };
            self.saved.intro_ticks += 1;
            self.saved.age = RELEASE + f32::from(self.saved.intro_ticks) / 120.;
            self.rock_step(1. / 120., None);
        }
        self.saved.age = to;
        if to >= GRAVITY_CHANGE {
            self.saved.gravity = 0.2;
        }
    }
    fn finish(
        &mut self,
        map: &Bsp,
        world: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        let from = self.saved.age;
        self.intro_step(from, END);
        self.saved.gravity = 0.2;
        self.data.rock.gravity = 160.;
        self.saved.rock.solid = true;
        self.rebuild(map)?;
        world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        let landing = self.data.landing(world)?;
        ensure!(
            landing.translation.distance(self.saved.rock.position) > 200.,
            "Insufficient marble lead"
        );
        let s = self
            .saved
            .scene
            .as_mut()
            .context("Missing introduction state")?;
        SceneRunner {
            spec: &spec(Some(landing)),
            state: s,
        }
        .finish(world, p)?;
        self.saved.scene = None;
        self.saved.skip = false;
        self.saved.arrived = true;
        self.saved.legacy = false;
        self.saved.damage = 0.;
        self.saved.impulse = Vec3::ZERO;
        Ok(())
    }
}
impl LevelController for Garden {
    fn id(&self) -> &'static str {
        "garden3"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("garden3.play", self.saved.arrived);
        f.flag(
            "garden3.exit",
            self.saved.arrived && self.saved.end.is_some_and(|t| self.saved.age - t >= 0.6),
        );
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        if t.id.0 == 74 {
            return Some(Condition::flag("garden3.exit"));
        }
        (matches!(t.id.0, 5 | 16 | 41 | 47 | 48 | 73 | 74)).then(|| Condition::flag("garden3.play"))
    }
    fn receivers(&self, _: &crate::entity::Registry) -> Vec<crate::entity::Id> {
        DOORS.map(crate::entity::Id).to_vec()
    }
    fn rules(&self, _: &crate::level::RuleContext<'_>) -> Vec<crate::event::Rule> {
        DOORS
            .iter()
            .map(|&n| {
                let id = crate::entity::Id(n);
                crate::event::Rule {
                    key: format!("garden3/door/{n}"),
                    event: crate::event::Event::Entity(id, crate::event::Input::Activate),
                    condition: Condition::flag("garden3.play"),
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
            if let Some(k) = DOORS.iter().position(|&n| n == id.0) {
                let d = &mut self.saved.doors[k];
                if self.saved.arrived && !d.latched {
                    d.open = !d.open;
                }
                return Some(Events::default());
            }
        }
        None
    }
    fn event(&mut self, n: &str) -> Option<Events> {
        if n == "Garden3_Start" || (self.saved.arrived && self.callback(n)) {
            Some(Events::default())
        } else {
            None
        }
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
        self.scripted().then_some("Garden3_Start")
    }
    fn entry_story(&mut self, _: &mut Story) -> bool {
        true
    }
    fn prepare_story(&self, _: &mut Story) -> bool {
        true
    }
    fn upgraded(&mut self) {
        self.saved.legacy = true;
        self.saved.skip = true;
    }
    fn upgrade(&self) -> crate::level::Upgrade {
        crate::level::Upgrade {
            respawn: crate::level::Respawn::Always,
            ..Default::default()
        }
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
        if let Some(s) = &mut self.saved.scene {
            let from = s.time;
            let mut runner = SceneRunner {
                spec: &spec(None),
                state: s,
            };
            runner.capture(p);
            let complete = runner.advance(dt);
            let to = runner.state.time;
            p.velocity = Vec3::ZERO;
            p.script_motion = 1;
            p.cancel_climb();
            p.release_rope();
            self.intro_step(from, to);
            if complete || self.saved.skip {
                self.finish(map, world, p, fixed)?;
                return Ok(());
            }
        } else {
            self.saved.age += dt;
            self.rock_step(dt, Some(p));
            for (k, passed) in [7, 55, 35, 24].into_iter().enumerate() {
                if self.saved.rock.node >= passed {
                    self.saved.doors[k].latched = true;
                    self.saved.doors[k].open = true;
                }
            }
            for (k, d) in self.saved.doors.iter_mut().enumerate() {
                d.advance(dt, self.data.travel[k]);
            }
        }
        self.rebuild(map)?;
        world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        Ok(())
    }
    fn skip(&mut self, _: &Bsp, _: &mut World, _: &mut Player, _: &mut Story) -> Result<bool> {
        if !self.scripted() || self.saved.skip {
            return Ok(false);
        }
        self.saved.skip = true;
        Ok(true)
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        self.scene_camera()
    }
    fn fade(&self) -> Option<(Color, f32)> {
        if self.saved.arrived {
            return (self.saved.age < END + 0.5)
                .then_some((WHITE, 1. - (self.saved.age - END) * 2.));
        }
        let t = self.saved.age;
        if t < 3. {
            return Some((BLACK, 1. - t / 3.));
        }
        let flash = [16.5, 19., END]
            .into_iter()
            .map(|v| (1. - (t - v).abs() * 2.).max(0.))
            .fold(0., f32::max);
        (flash > 0.).then_some((WHITE, flash))
    }
    fn scene_fog(&self) -> Option<Vec4> {
        let initial = vec4(0.33, 0.23, 0.1, 1800.);
        if !self.saved.arrived {
            return Some(initial);
        }
        let colors = [
            vec4(0.27, 0.23, 0.15, 3700.),
            vec4(0.33, 0.23, 0.1, 4000.),
            vec4(0.2, 0.2, 0.1, 3700.),
        ];
        let t = (self.saved.age - END).max(0.);
        let step = (t / 10.) as usize;
        Some(
            (if step == 0 {
                initial
            } else {
                colors[(step - 1) % 3]
            })
            .lerp(colors[step % 3], (t % 10.) / 10.),
        )
    }
    fn quake_offset(&self) -> Vec3 {
        if self.scripted() {
            return Vec3::ZERO;
        }
        let strength = [
            (self.saved.first_quake, 2., 0.4),
            (self.saved.pillar, 3., 0.6),
            (self.saved.second_quake, 2., 0.4),
            (self.saved.ice, 4., 0.6),
        ]
        .into_iter()
        .filter_map(|(at, duration, magnitude)| {
            at.map(|at| magnitude * (1. - (self.saved.age - at) / duration).clamp(0., 1.))
        })
        .fold(0., f32::max);
        vec3(
            (self.saved.age * 97.).sin(),
            (self.saved.age * 113.).sin(),
            (self.saved.age * 83.).cos(),
        ) * strength
            * 2.
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
            .chain(self.shroom.as_ref().map(|(_, c)| c.clone()))
            .chain(self.data.rock.collider(&self.saved.rock))
            .collect()
    }
    fn settled_supports(&self) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| {
                o.door.is_none()
                    && o.pose.is_some_and(|p| {
                        p.translation == o.base.translation && p.rotation == o.base.rotation
                    })
            })
            .map(|o| o.collider.clone())
            .chain(self.shroom.as_ref().filter(|_| self.objects.iter().any(|o|
                o.name == "falling_pillar1" && o.pose.is_some_and(|p|
                    p.translation == o.base.translation && p.rotation == o.base.rotation)))
                .map(|(_, c)| c.clone()))
            .collect()
    }
    fn combat(&mut self, c: &mut crate::level::Combat<'_>) -> crate::combat::Feedback {
        if c.dt <= 0. || self.scripted() {
            return Default::default();
        }
        crate::combat::Feedback {
            damage: std::mem::take(&mut self.saved.damage),
            impulse: std::mem::take(&mut self.saved.impulse),
            ..Default::default()
        }
    }
    fn sound_state(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        self.sounds(loops, clocks);
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        let s: Saved = state::load(v, state::Visit { returning: false })?;
        self.data.rock.validate(&s.rock)?;
        ensure!(
            s.rock.callbacks == s.rock.node,
            "Invalid chase callback cursor"
        );
        self.saved = s;
        self.data.rock.gravity = 800. * self.saved.gravity;
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
    id: "garden3",
    applies: |m, e| super::first_visit(m, e, "garden3"),
    load: |a, m, _, _| Ok(Box::new(Garden::load(a, m)?)),
    art: Some(|a, _, _| Ok(Box::new(art::Art::load(a)?))),
    owns_submodel: |m, e| m == "garden3" && motion::owns(e),
    owns_npc: |n, _| {
        n.starts_with("lady_model")
            || matches!(
                n,
                "lady_actor1"
                    | "big_bug1"
                    | "bug_marble1"
                    | "chase_marble1"
                    | "falling_pillar_shroom1"
            )
    },
    target_base: Some(BASE),
    story_beats: &[],
    checks: &[
        Check {
            flag: "--garden3-route-check",
            help: "Run Rolling Stones from arrival to Icy Reception with a live marble.",
            run: Run::Headless(route::watched),
        },
        Check {
            flag: "--garden3-skip-route-check",
            help: "Run the chase after skipping its arrival, including live save continuation.",
            run: Run::Headless(route::skipped),
        },
        Check {
            flag: "--garden3-check",
            help: "Verify Rolling Stones camera, real marble, gates and saved handoffs.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--garden3-render-check",
            help: "Render Rolling Stones introduction and chase.",
            run: Run::Windowed(check::render),
        },
    ],
    save_cases: &[
        SaveCase {
            name: "garden3-live-gates",
            visit: "garden3$first",
            stage: None,
            behavior: None,
        },
        SaveCase {
            name: "garden3-live-airborne",
            visit: "garden3$first",
            stage: None,
            behavior: None,
        },
        SaveCase {
            name: "garden3-live-pillar",
            visit: "garden3$first",
            stage: None,
            behavior: None,
        },
        SaveCase {
            name: "garden3-live-ice",
            visit: "garden3$first",
            stage: None,
            behavior: None,
        },
        SaveCase {
            name: "garden3-live-ending",
            visit: "garden3$first",
            stage: None,
            behavior: None,
        },
        SaveCase {
            name: "garden3-fleet",
            visit: "garden3$first",
            stage: None,
            behavior: None,
        },
        SaveCase {
            name: "garden3-carrier",
            visit: "garden3$first",
            stage: None,
            behavior: None,
        },
        SaveCase {
            name: "garden3-before-release",
            visit: "garden3$first",
            stage: None,
            behavior: None,
        },
        SaveCase {
            name: "garden3-marble",
            visit: "garden3$first",
            stage: None,
            behavior: None,
        },
        SaveCase {
            name: "garden3-running",
            visit: "garden3$first",
            stage: None,
            behavior: None,
        },
        SaveCase {
            name: "garden3-chase",
            visit: "garden3$first",
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
        .context("Rolling Stones owner missing")
}
