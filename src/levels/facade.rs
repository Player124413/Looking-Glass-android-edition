//! Ascension's targeted gates and saved, scene-owned departure lift.
mod check;
mod route;
pub(crate) use route::drive as drive_route;
use super::{state, Check, Registration, Run};
use crate::{
    assets::Assets,
    bsp::Bsp,
    cinematic::{Camera, Track},
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    entity::{Id, Registry},
    event::{Action, Condition, Effect, Event, Facts, Input, Rule},
    fortress::spline::Spline,
    interaction::{self, Events},
    level::exit::ExitState,
    level::spec::ExitSpec,
    level::{LevelArt, LevelController, TriggerInfo},
    movement::Player,
    skeletal::Transform,
    story::Story,
};
use anyhow::{ensure, Context, Result};
pub(crate) use check::stage_native;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;

const EXIT: ExitSpec = ExitSpec {
    map: "keep",
    entrance: "keep_start1",
};
const LIFT_START: f32 = 0.5;
const DEPART: f32 = 4.5;
#[derive(Clone, Serialize, Deserialize)]
struct Ride {
    time: f32,
    skip: Option<f32>,
    local: Option<Transform>,
    #[serde(default)]
    floor: Option<f32>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    gate: f32,
    gate_hold: f32,
    gate_opening: bool,
    ride: Option<Ride>,
    exit: ExitState,
}
impl Default for Saved {
    fn default() -> Self {
        Self {
            version: 1,
            gate: 0.,
            gate_hold: 0.,
            gate_opening: false,
            ride: None,
            exit: ExitState::default(),
        }
    }
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        state::fraction("gate", self.gate)?;
        state::clock("gate hold", self.gate_hold, 3.)?;
        state::clock("exit retry", self.exit.retry_time, 1.)?;
        if let Some(r) = &self.ride {
            state::clock("lift scene", r.time, DEPART)?;
            if let Some(t) = r.skip {
                state::clock("lift skip", t, 0.5)?;
            }
            if let Some(z) = r.floor {
                ensure!(
                    z.is_finite() && (-40. ..=80.).contains(&z),
                    "Invalid lift deck height"
                );
            }
            if let Some(p) = r.local {
                ensure!(
                    p.translation.is_finite()
                        && p.translation.truncate().length() < 600.
                        && (-40. ..=250.).contains(&p.translation.z),
                    "Invalid lift rider position"
                );
                ensure!(
                    p.rotation.is_finite() && (p.rotation.length() - 1.).abs() < 0.01,
                    "Invalid lift rider facing"
                );
            }
            ensure!(
                self.exit.committed == (r.time == DEPART || r.skip == Some(0.5)),
                "Lift exit and scene disagree"
            );
        } else {
            ensure!(!self.exit.committed, "Exit without lift scene");
        }
        Ok(())
    }
}
struct Brush {
    model: usize,
    base: Vec3,
    delta: Vec3,
    duration: f32,
}
struct Facade {
    saved: Saved,
    lift: Brush,
    gates: Vec<Brush>,
    solids: Vec<Collider>,
    camera: Spline,
}
fn brush(map: &Bsp, id: usize) -> Result<Brush> {
    let e = &map.entities[id];
    let model: usize = e
        .get("model")
        .and_then(|s| s.strip_prefix('*'))
        .context("Missing Ascension brush")?
        .parse()?;
    let base = e
        .get("origin")
        .and_then(|s| interaction::vector(s))
        .context("Missing Ascension origin")?;
    let angle: f32 = e.get("angle").and_then(|s| s.parse().ok()).unwrap_or(0.);
    let direction = vec3(angle.to_radians().cos(), angle.to_radians().sin(), 0.);
    let bounds = &map.models[model];
    let lip: f32 = e.get("lip").and_then(|s| s.parse().ok()).unwrap_or(8.);
    let distance = ((bounds.max - bounds.min).dot(direction.abs()) - lip).max(0.);
    let speed: f32 = e.get("speed").and_then(|s| s.parse().ok()).unwrap_or(100.);
    let duration = e
        .get("time")
        .and_then(|s| s.parse().ok())
        .unwrap_or(distance / speed.max(1.));
    Ok(Brush {
        model,
        base,
        delta: direction * distance,
        duration: duration.max(0.01),
    })
}
impl Facade {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        ensure!(
            map.entities[57]
                .get("targetname")
                .is_some_and(|s| s == "facade_lift"),
            "Unexpected Ascension layout"
        );
        let mut f = Self {
            saved: Saved::default(),
            lift: brush(map, 57)?,
            gates: vec![brush(map, 31)?, brush(map, 32)?],
            solids: vec![],
            camera: Spline::camera_track(Track::load(a, "facade_path3")?.controls().collect()),
        };
        f.rebuild(map)?;
        Ok(f)
    }
    fn lift_pose(&self) -> Transform {
        let elapsed = self
            .saved
            .ride
            .as_ref()
            .map_or(0., |s| (s.time - LIFT_START).max(0.));
        Transform {
            translation: self.lift.base + Vec3::Z * (150. * elapsed),
            rotation: Quat::from_rotation_z((102.4 * elapsed).to_radians()),
        }
    }
    fn alice_pose(&self) -> Option<Transform> {
        let ride = self.saved.ride.as_ref()?;
        let mut local = ride.local?;
        // The trigger can catch Alice while jumping onto the raised deck. Let
        // that captured pose settle under gravity instead of freezing in midair.
        local.translation.z = (local.translation.z
            - 0.5 * crate::movement::GRAVITY * ride.time * ride.time)
            .max(ride.floor.unwrap_or(local.translation.z));
        let lift = self.lift_pose();
        Some(Transform {
            translation: lift.point(local.translation),
            rotation: lift.rotation * local.rotation,
        })
    }
    fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        self.solids = self
            .transforms()
            .into_iter()
            .map(|(m, p, r)| Collider::model(map, m, p, r, true))
            .collect::<Result<_>>()?;
        Ok(())
    }
    fn start(&mut self) {
        if self.saved.ride.is_none() {
            self.saved.ride = Some(Ride {
                time: 0.,
                skip: None,
                local: None,
                floor: None,
            });
        }
    }
}
impl LevelController for Facade {
    fn id(&self) -> &'static str {
        "facade"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("facade.depart", self.saved.exit.committed);
        f.flag("facade.free", self.saved.ride.is_none());
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        match t.id.0 {
            56 => Some(Condition::flag("facade.free")),
            67 => Some(Condition::flag("facade.depart")),
            _ => None,
        }
    }
    fn receivers(&self, _: &Registry) -> Vec<Id> {
        vec![Id(31), Id(32)]
    }
    fn rules(&self, _: &crate::level::RuleContext<'_>) -> Vec<Rule> {
        [31, 32]
            .into_iter()
            .map(|n| Rule {
                key: format!("facade/gate/{n}"),
                event: Event::Entity(Id(n), Input::Activate),
                condition: Condition::Always,
                once: false,
                cooldown: 0.,
                actions: vec![Action::Output(Effect::Activate(Id(n)))],
            })
            .collect()
    }
    fn output(&mut self, e: &Effect) -> Option<Events> {
        if matches!(e, Effect::Activate(Id(31 | 32))) {
            self.saved.gate_opening = true;
            self.saved.gate_hold = 3.;
            Some(Events::default())
        } else {
            None
        }
    }
    fn event(&mut self, n: &str) -> Option<Events> {
        if n == "Facade_Lift" {
            self.start();
            Some(Events::default())
        } else {
            None
        }
    }
    fn update(&mut self, _: &mut World, _: &Player, _: Vec3, _: bool) -> Events {
        Events {
            transition: self.saved.exit.request(EXIT),
            ..Default::default()
        }
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
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        let p = self.lift_pose();
        let mut poses = vec![(self.lift.model, p.translation, p.rotation)];
        poses.extend(
            self.gates
                .iter()
                .map(|b| (b.model, b.base + b.delta * self.saved.gate, Quat::IDENTITY)),
        );
        poses
    }
    fn colliders(&self) -> Vec<Collider> {
        self.solids.clone()
    }
    fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if !dt.is_finite() || dt <= 0. {
            return Ok(());
        }
        let dt = dt.min(0.1);
        let before = self.saved.clone();
        self.saved.exit.advance(dt);
        if let Some(s) = self.saved.ride.as_mut() {
            if s.local.is_none() {
                s.local = Some(Transform {
                    translation: p.feet - self.lift.base,
                    rotation: Quat::from_rotation_z(p.script_facing),
                });
                p.cancel_climb();
                p.release_rope();
            }
            if s.floor.is_none() {
                // Trace the walking surface at this rider's position: the
                // brush bounds include a raised rim above the middle of the deck.
                let deck = w
                    .actor_footing(p.feet + Vec3::Z, PLAYER_CENTER, PLAYER_HALF, 400.)
                    .context("Ascension rider has no lift deck")?;
                s.floor = Some(deck.z - self.lift.base.z - 150. * (s.time - LIFT_START).max(0.));
            }
            if !self.saved.exit.committed {
                s.time = (s.time + dt).min(DEPART);
                if let Some(t) = s.skip.as_mut() {
                    *t = (*t + dt).min(0.5);
                }
                self.saved.exit.committed = s.time == DEPART || s.skip == Some(0.5);
            }
        }
        let speed = 1. / self.gates.iter().map(|b| b.duration).fold(0_f32, f32::max);
        if self.saved.gate_opening {
            self.saved.gate = (self.saved.gate + dt * speed).min(1.);
            if self.saved.gate == 1. {
                self.saved.gate_hold = (self.saved.gate_hold - dt).max(0.);
                if self.saved.gate_hold == 0. {
                    self.saved.gate_opening = false;
                }
            }
        } else {
            self.saved.gate = (self.saved.gate - dt * speed).max(0.);
        }
        self.rebuild(map)?;
        let feet = self.alice_pose().map_or(p.feet, |at| at.translation);
        w.set_dynamic(fixed.to_vec());
        let sweep = w.sweep(p.feet + PLAYER_CENTER, feet + PLAYER_CENTER, PLAYER_HALF);
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        if sweep.start_solid || sweep.fraction < 1. || !w.body_clear(feet) {
            // A closing leaf reverses instead of crushing or embedding Alice.
            self.saved.gate = before.gate;
            self.saved.gate_opening = true;
            self.saved.gate_hold = 3.;
            self.rebuild(map)?;
            w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
            ensure!(
                w.body_clear(feet),
                "Ascension lift rider obstructed at {feet:?}"
            );
        }
        if let Some(at) = self.alice_pose() {
            p.feet = at.translation;
            p.velocity = Vec3::ZERO;
            p.grounded = true;
            p.ground_normal = Vec3::Z;
            p.script_facing = at.rotation.to_euler(EulerRot::ZYX).0;
        }
        Ok(())
    }
    fn scripted(&self) -> bool {
        self.saved.ride.is_some()
    }
    fn controlled(&self) -> bool {
        self.scripted()
    }
    fn hides_player(&self) -> bool {
        self.scripted()
    }
    fn scene_id(&self) -> Option<&'static str> {
        self.scripted().then_some("Facade_Lift")
    }
    fn camera(&self, _: &World) -> Option<Camera> {
        self.saved
            .ride
            .as_ref()
            .filter(|s| s.time >= LIFT_START)
            .map(|s| self.camera.camera(s.time - LIFT_START))
    }
    fn fade(&self) -> Option<(Color, f32)> {
        self.saved.ride.as_ref().map(|s| {
            (
                WHITE,
                s.skip.map_or_else(
                    || {
                        if s.time < LIFT_START {
                            s.time / LIFT_START
                        } else {
                            (1. - (s.time - LIFT_START) / 0.5).clamp(0., 1.)
                        }
                    },
                    |t| t / 0.5,
                ),
            )
        })
    }
    fn skip(&mut self, _: &Bsp, _: &mut World, _: &mut Player, _: &mut Story) -> Result<bool> {
        let Some(s) = self.saved.ride.as_mut() else {
            return Ok(false);
        };
        if s.skip.is_none() && !self.saved.exit.committed {
            s.skip = Some(0.);
        }
        Ok(true)
    }
    fn sound_state(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        _: &mut Vec<crate::audio::world::Clock>,
    ) {
        if let Some(s) = self
            .saved
            .ride
            .as_ref()
            .filter(|s| s.time >= LIFT_START && !self.saved.exit.committed)
        {
            loops.push(crate::audio::LoopCue {
                id: 9_600_000,
                path: "sound/ambience/special/lift_loop.wav",
                origin: self.lift_pose().translation,
                clock: Some(s.time - LIFT_START),
            });
        }
    }
    fn objective(&self) -> Option<String> {
        Some(
            if self.scripted() {
                "Ride the lift into the castle."
            } else {
                "Climb the rising air, pass the gates and reach the castle lift."
            }
            .into(),
        )
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        let saved: Saved = state::load(v, state::Visit { returning: false })?;
        self.saved = saved;
        self.rebuild(map)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
struct Art(crate::npc::Puppet);
impl Art {
    fn load(a: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        Ok(Self(crate::npc::Puppet::load(
            a,
            "alice",
            &["idle_stand"],
            &specs,
        )?))
    }
}
impl LevelArt for Art {
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atmo: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        let f = l.downcast_ref::<Facade>().unwrap();
        if f.saved.ride.as_ref().is_some_and(|r| r.time < LIFT_START) {
            return; // The first fade still uses Alice's gameplay camera.
        }
        if let Some(p) = f.alice_pose() {
            self.0.atmosphere(atmo, camera);
            self.0.draw(
                "idle_stand",
                f.saved.ride.as_ref().unwrap().time,
                true,
                p,
                1.,
                bright,
            );
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
    id: "facade",
    applies: |m, e| super::first_visit(m, e, "facade"),
    load: |a, m, _, _| Ok(Box::new(Facade::load(a, m)?)),
    art: Some(|a, _, _| Ok(Box::new(Art::load(a)?))),
    owns_submodel: |m, e| {
        m == "facade"
            && e.get("targetname")
                .is_some_and(|n| n == "facade_lift" || n == "t15")
    },
    owns_npc: |_, _| false,
    target_base: None,
    story_beats: &[],
    checks: &[
        Check {
            flag: "--facade-route-native-check",
            help: "Replay the full Ascension input course with resident enemies.",
            run: Run::Windowed(route::check),
        },
        Check {
            flag: "--facade-check",
            help: "Verify Ascension gates, updrafts, lift and saved transfer.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--facade-route-check",
            help: "Check the Ascension updraft/gate/lift traversal fixture.",
            run: Run::Headless(check::route),
        },
        Check {
            flag: "--facade-skip-route-check",
            help: "Check Ascension traversal with skipped lift departure.",
            run: Run::Headless(check::skip_route),
        },
        Check {
            flag: "--facade-render-check",
            help: "Capture Ascension gates and lift ride.",
            run: Run::Windowed(check::render),
        },
    ],
    save_cases: check::SAVES,
    visibility: &[],
};
