//! Labyrinthine Revenge: saved machine phases, automatic doors and bellows.
mod check;
mod motion;
mod route;
use super::{state, Check, Registration, Run};
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    combat::Target,
    entity::{Id, Registry},
    event::Effect,
    interaction::{self, Events, Interactions},
    level::{LevelController, Respawn, Upgrade},
    movement::{Controls, Player, FIXED_DT},
    skeletal::Transform,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;

#[derive(Clone, Default, Serialize, Deserialize)]
struct Mechanism {
    time: f64,
    door: f32,
    hold: f32,
    latched: bool,
    depth: f32,
    velocity: f32,
    #[serde(default)]
    stops: u32,
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    machines: Vec<Mechanism>,
    crush: bool,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        ensure!(self.machines.len() <= 128, "Too many Hedge machines");
        for m in &self.machines {
            ensure!(
                m.time.is_finite() && (0. ..=1e8).contains(&m.time),
                "Invalid machine clock"
            );
            ensure!(m.stops <= 1_000_000, "Invalid door stop counter");
            state::fraction("Hedge door", m.door)?;
            state::clock("Hedge door hold", m.hold, 10.)?;
            state::clock("Hedge sink", m.depth, 1000.)?;
            ensure!(
                m.velocity.is_finite() && m.velocity.abs() < 1000.,
                "Invalid sink velocity"
            );
        }
        Ok(())
    }
}
struct Labyrinth {
    objects: Vec<motion::Object>,
    saved: Saved,
    actors: Vec<Target>,
}
impl Labyrinth {
    fn load(map: &Bsp) -> Result<Self> {
        let objects = motion::load(map)?;
        let saved = Saved {
            version: 1,
            machines: vec![Mechanism::default(); objects.len()],
            crush: false,
        };
        let mut out = Self {
            objects,
            saved,
            actors: vec![],
        };
        out.rebuild();
        Ok(out)
    }
    fn index(&self, name: &str) -> usize {
        self.objects
            .iter()
            .position(|o| o.name == name)
            .expect("Reviewed Hedge machine missing")
    }
    fn gust(&self, number: usize) -> bool {
        self.saved.machines[self.index(&format!("bellow{number}"))]
            .time
            .rem_euclid(8.)
            >= 4.
    }
    fn rebuild(&mut self) {
        for (o, s) in self.objects.iter_mut().zip(&self.saved.machines) {
            o.pose = o.at(s);
            o.collider = o.shape.at(o.pose.translation, o.pose.rotation);
        }
    }
}
impl LevelController for Labyrinth {
    fn id(&self) -> &'static str {
        "hedge3"
    }
    fn actor_contacts(&mut self, actors: &[Target]) {
        self.actors = actors.to_vec();
    }
    fn event(&mut self, thread: &str) -> Option<Events> {
        matches!(
            thread,
            "GearMoves"
                | "Bellow1Move"
                | "Bellow2Move"
                | "SpikeGearsMesh"
                | "ThreeGearMesh"
                | "Crushers"
                | "PistonMove"
        )
        .then(Events::default)
    }
    fn receivers(&self, _: &Registry) -> Vec<Id> {
        self.objects
            .iter()
            .filter(|o| o.door() && !o.name.is_empty())
            .map(|o| Id(o.id))
            .collect()
    }
    fn output(&mut self, e: &Effect) -> Option<Events> {
        let (id, open) = match *e {
            Effect::Activate(id) => (id, true),
            Effect::Door { id, open, .. } => (id, open),
            _ => return None,
        };
        let k = self.objects.iter().position(|o| o.id == id.0 && o.door())?;
        self.saved.machines[k].hold = if open {
            self.objects[k].wait.max(3.)
        } else {
            0.
        };
        self.saved.machines[k].latched = open && self.objects[k].wait < 0.;
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
        self.move_all(dt.min(0.1), w, p, fixed)
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.objects
            .iter()
            .map(|o| (o.model, o.pose.translation, o.pose.rotation))
            .collect()
    }
    fn colliders(&self) -> Vec<Collider> {
        self.objects.iter().map(|o| o.collider.clone()).collect()
    }
    fn settled_supports(&self) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| o.name == "t30" || o.name == "t26")
            .map(|o| o.collider.clone())
            .collect()
    }
    fn traversal(&self, t: &mut crate::traversal::Traversal) {
        for p in &mut t.pushes {
            if p.id == Id(57) {
                p.enabled = self.gust(2);
            }
            if p.id == Id(822) {
                p.enabled = self.gust(1);
            }
        }
    }
    fn particles(&self, s: &mut crate::particles::Steam) {
        s.gate(&[(Id(818), self.gust(2)), (Id(823), self.gust(1))]);
    }
    fn combat(&mut self, c: &mut crate::level::Combat<'_>) -> crate::combat::Feedback {
        let mut f = crate::combat::Feedback::default();
        if c.dt > 0. && std::mem::take(&mut self.saved.crush) {
            f.damage = 1000.;
        }
        f
    }
    fn sound_state(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        for (o, m) in self.objects.iter().zip(&self.saved.machines) {
            if o.door() && o.sound && m.door > 0. && m.door < 1. {
                loops.push(crate::audio::LoopCue {
                    id: 9_200_000 + o.id,
                    path: "sound/ambience/special/metal_door_1.wav",
                    origin: o.pose.translation,
                    clock: None,
                });
            }
            if o.door() && o.sound {
                let key = match o.id {
                    1 => "hedge3.d1",
                    6 => "hedge3.d6",
                    56 => "hedge3.d56",
                    80 => "hedge3.d80",
                    83 => "hedge3.d83",
                    84 => "hedge3.d84",
                    87 => "hedge3.d87",
                    684 => "hedge3.d684",
                    685 => "hedge3.d685",
                    688 => "hedge3.d688",
                    689 => "hedge3.d689",
                    1022 => "hedge3.d1022",
                    1070 => "hedge3.d1070",
                    1103 => "hedge3.d1103",
                    _ => continue,
                };
                clocks.push(crate::audio::world::Clock {
                    key,
                    time: m.stops as f32,
                    period: Some(1.),
                    origin: o.pose.translation,
                    cues: &[(0.5, "sound/ambience/special/metal_door_end.wav")],
                });
            }
            let (key, offset, slam) = match o.name.as_str() {
                "pendulum01" => ("hedge3.crusher1", 0., 5.),
                "pendulum02" => ("hedge3.crusher2", 4., 1.),
                _ => continue,
            };
            if (m.time as f32 - offset).rem_euclid(8.) < 4. {
                loops.push(crate::audio::LoopCue {
                    id: 9_200_000 + o.id,
                    path: "sound/ambience/special/chain_loop.wav",
                    origin: o.pose.translation,
                    clock: Some((m.time as f32 - offset).rem_euclid(8.)),
                });
            }
            clocks.push(crate::audio::world::Clock {
                key,
                time: m.time as f32,
                period: Some(8.),
                origin: o.pose.translation,
                cues: if slam == 5. {
                    &[(5., "sound/ambience/special/door_slam.wav")]
                } else {
                    &[(1., "sound/ambience/special/door_slam.wav")]
                },
            });
        }
    }
    fn upgrade(&self) -> Upgrade {
        Upgrade {
            respawn: Respawn::Always,
            ..Default::default()
        }
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, _: &Bsp) -> Result<()> {
        let s: Saved = state::load(v, state::Visit { returning: false })?;
        ensure!(
            s.machines.len() == self.objects.len(),
            "Hedge machinery count changed"
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
    id: "hedge3",
    applies: |m, e| super::first_visit(m, e, "hedge3"),
    load: |_, m, _, _| Ok(Box::new(Labyrinth::load(m)?)),
    art: None,
    owns_submodel: |m, e| m == "hedge3" && motion::owned(e),
    owns_npc: |_, _| false,
    target_base: None,
    story_beats: &[],
    checks: &[
        Check {
            flag: "--hedge3-check",
            help: "Verify Labyrinth machinery, doors, skies and saved movement.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--hedge3-route-check",
            help: "Traverse Labyrinthine Revenge into tower3 with live machinery.",
            run: Run::Windowed(route::check),
        },
        Check {
            flag: "--hedge3-render-check",
            help: "Capture restored Hedge machinery, bellows and seven skies.",
            run: Run::Windowed(|a| Box::pin(check::render(a))),
        },
    ],
    save_cases: check::SAVES,
    visibility: &[],
};
fn owner(i: &mut Interactions) -> Result<&mut Labyrinth> {
    i.levels
        .iter_mut()
        .find(|s| s.reg.id == "hedge3")
        .and_then(|s| s.ctl.downcast_mut::<Labyrinth>())
        .context("Missing Hedge controller")
}

pub(crate) use route::drive as drive_route;
