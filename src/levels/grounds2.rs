//! Battle Royale: the collapsing entrance and finite, recurring guard companies.
mod art;
mod check;
mod data;
mod route;
mod saves;
mod waves;
use super::{state, Check, Registration, Run};
use crate::{
    assets::Assets,
    bsp::Bsp,
    cinematic::Camera,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    combat::{Hit, Target},
    event::{Condition, Facts},
    interaction::{self, Events},
    inventory::Stats,
    level::{LevelArt, LevelController, TriggerInfo},
    movement::Player,
    skeletal::Transform,
    story::Story,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;
const BASE: usize = 9_500_000;
const TOTAL: [usize; 8] = [2, 2, 3, 3, 4, 4, 6, 6];
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    time: f32,
    done: bool,
    initialized: bool,
    groups: [waves::Group; 8],
    actors: Vec<waves::Actor>,
    accumulator: f64,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        state::clock("introduction", self.time, 22.)?;
        ensure!(
            self.done == (self.time == 22.),
            "Introduction completion disagrees with its clock"
        );
        ensure!(
            self.accumulator.is_finite() && (0. ..=0.009).contains(&self.accumulator),
            "Invalid combat remainder"
        );
        ensure!(self.actors.len() == 30, "Invalid company capacity");
        ensure!(
            self.done || self.groups.iter().all(|g| !g.started),
            "Reinforcements before arrival"
        );
        let mut offset = 0;
        for (k, total) in TOTAL.into_iter().enumerate() {
            let group = &self.groups[k];
            state::clock("reinforcement", group.wait, 1.101)?;
            ensure!(
                group.count <= total && (group.started || group.count == 0),
                "Invalid reinforcement count"
            );
            let mut alive = 0;
            for (ordinal, a) in self.actors[offset..offset + total].iter().enumerate() {
                ensure!(
                    a.spawned == (ordinal < group.count),
                    "Noncontiguous company history"
                );
                ensure!(
                    a.body.kind == crate::cards::Kind::Spade && a.body.scale == 1.,
                    "Changed reinforcement identity"
                );
                a.body.validate()?;
                state::clock("electric hit", a.electric, crate::electric::LIFE)?;
                if let Some(f) = &a.flight {
                    ensure!(
                        a.spawned && f.velocity.is_finite() && f.velocity.length() < 10000.,
                        "Invalid launch velocity"
                    );
                    state::clock("launch", f.time, 12.)?;
                }
                if a.spawned && a.body.health > 0. {
                    alive += 1;
                }
            }
            ensure!(alive <= 2, "Too many live guards in one company");
            offset += total;
        }
        Ok(())
    }
}
struct Royale {
    saved: Saved,
    data: data::Data,
    solids: Vec<Collider>,
    rig: crate::npc::CardRig,
}
impl Royale {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let data = data::Data::load(a, map)?;
        let actors = data
            .spawns
            .iter()
            .enumerate()
            .flat_map(|(k, p)| (0..TOTAL[k]).map(move |n| waves::Actor::new(*p, k * 8 + n)))
            .collect();
        let mut s = Self {
            saved: Saved {
                version: 1,
                time: 0.,
                done: false,
                initialized: false,
                groups: std::array::from_fn(|_| waves::Group::default()),
                actors,
                accumulator: 0.,
            },
            data,
            solids: vec![],
            rig: crate::npc::CardRig::load(a, crate::cards::Kind::Spade)?,
        };
        s.rebuild(map)?;
        Ok(s)
    }
    fn floor_pose(&self, k: usize) -> (Vec3, Quat) {
        let t = ((self.saved.time - 4. - k as f32 * 0.2) / 6.).clamp(0., 1.);
        let angle = (55. * t).to_radians();
        (
            self.data.floor[k].1 - Vec3::Z * 2400. * t,
            Quat::from_rotation_z(angle) * Quat::from_rotation_x(-angle),
        )
    }
    fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        let mut solids = Vec::new();
        for (k, (m, _)) in self.data.floor.iter().enumerate() {
            let (p, r) = self.floor_pose(k);
            solids.push(Collider::model(map, *m, p, r, true)?);
        }
        if self.saved.done {
            solids.push(Collider::model(
                map,
                self.data.clip.0,
                self.data.clip.1,
                Quat::IDENTITY,
                true,
            )?);
        }
        self.solids = solids;
        Ok(())
    }
    fn alice_pose(&self) -> Transform {
        let mut distance = (self.saved.time * 320.).min(self.data.length);
        let mut point = self.data.path[0];
        let mut forward = Vec3::Y;
        for edge in self.data.path.windows(2) {
            let d = edge[1] - edge[0];
            let len = d.length();
            forward = d.normalize_or_zero();
            if distance <= len {
                point = edge[0] + forward * distance;
                break;
            }
            distance -= len;
            point = edge[1];
        }
        point = self
            .data
            .support
            .actor_footing(point + Vec3::Z * 64., PLAYER_CENTER, PLAYER_HALF, 96.)
            .unwrap_or(point);
        Transform {
            translation: point,
            rotation: Quat::from_rotation_z(forward.y.atan2(forward.x)),
        }
    }
    fn finish(&mut self, map: &Bsp, w: &mut World, p: &mut Player) -> Result<()> {
        self.saved.time = 22.;
        self.saved.done = true;
        self.rebuild(map)?;
        w.set_dynamic(self.colliders());
        crate::cinematic::land_player(
            p,
            w,
            Transform {
                translation: *self.data.path.last().unwrap(),
                rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            },
        )?;
        Ok(())
    }
}
impl LevelController for Royale {
    fn id(&self) -> &'static str {
        "grounds2"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("grounds2.arrived", self.saved.done);
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        (t.id.0 == 7 || matches!(t.id.0, 9 | 10 | 293 | 294 | 379 | 384 | 385 | 386 | 387))
            .then(|| Condition::flag("grounds2.arrived"))
    }
    fn event(&mut self, name: &str) -> Option<Events> {
        if let Some(k) = name
            .strip_prefix("Spawner")
            .and_then(|s| s.parse::<usize>().ok())
            .and_then(|n| n.checked_sub(1))
            .filter(|k| *k < 8)
        {
            if self.saved.done && !(self.data.easy && k % 2 == 0) {
                self.saved.groups[k].started = true;
                if k % 2 == 0 {
                    self.saved.groups[k + 1].started = true;
                }
            }
            return Some(Events::default());
        }
        // Death callbacks are represented by the saved guard's actual health, never a mutable decrement.
        if name
            .strip_prefix("SpawnDead")
            .and_then(|s| s.parse::<usize>().ok())
            .is_some_and(|n| (1..=8).contains(&n))
        {
            return Some(Events::default());
        }
        (name == "grounds2Cine").then(Events::default)
    }
    fn prepare_player(&mut self, _: &mut Stats, _: &mut Player) {
        self.saved.initialized = true;
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
        if !self.saved.done {
            self.saved.time = (self.saved.time + dt.min(0.1)).min(22.);
            let pose = self.alice_pose();
            p.feet = pose.translation;
            p.velocity = Vec3::ZERO;
            p.script_facing = std::f32::consts::FRAC_PI_2;
            self.rebuild(map)?;
            w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
            if self.saved.time == 22. {
                self.finish(map, w, p)?;
                w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
            }
        }
        Ok(())
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.data
            .floor
            .iter()
            .enumerate()
            .map(|(k, (m, _))| {
                let (p, r) = self.floor_pose(k);
                (*m, p, r)
            })
            .chain(
                self.data
                    .decor
                    .iter()
                    .map(|(m, p)| (*m, *p, Quat::IDENTITY)),
            )
            .collect()
    }
    fn colliders(&self) -> Vec<Collider> {
        self.solids.clone()
    }
    fn scripted(&self) -> bool {
        !self.saved.done
    }
    fn hides_player(&self) -> bool {
        self.scripted()
    }
    fn controlled(&self) -> bool {
        self.scripted()
    }
    fn scene_id(&self) -> Option<&'static str> {
        self.scripted().then_some("grounds2Cine")
    }
    fn camera(&self, _: &World) -> Option<Camera> {
        if self.saved.done {
            return None;
        }
        let (k, start) = if self.saved.time < 6. {
            (0, 0.)
        } else if self.saved.time < 14. {
            (1, 6.)
        } else if self.saved.time < 15. {
            (2, 14.)
        } else if self.saved.time < 16. {
            (3, 15.)
        } else {
            (4, 16.)
        };
        let t = self.data.cameras[k].sample(self.saved.time - start, false);
        Some(Camera {
            eye: t.translation,
            target: t.point(Vec3::X),
            up: t.rotation * Vec3::Z,
        })
    }
    fn fade(&self) -> Option<(Color, f32)> {
        (!self.saved.done && self.saved.time < 3.).then_some((BLACK, 1. - self.saved.time / 3.))
    }
    fn skip(&mut self, map: &Bsp, w: &mut World, p: &mut Player, _: &mut Story) -> Result<bool> {
        if self.saved.done {
            return Ok(false);
        }
        self.finish(map, w, p)?;
        Ok(true)
    }
    fn sound_state(
        &self,
        _: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        if self.saved.done {
            return;
        }
        clocks.push(crate::audio::world::Clock {
            key: "grounds2/collapse",
            time: self.saved.time,
            period: None,
            origin: self.data.floor[2].1,
            cues: &[
                (4., "sound/ambience/special/thronebreak3.wav"),
                (4.6, "sound/ambience/special/thronebreak3.wav"),
                (11., "sound/character/chess_piece/pawn/alert2.wav"),
            ],
        });
    }
    fn combat(&mut self, c: &mut crate::level::Combat<'_>) -> crate::combat::Feedback {
        self.step_waves(c)
    }
    fn targets(&self) -> Vec<Target> {
        self.saved
            .actors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.spawned && a.body.health > 0.)
            .map(|(i, a)| a.body.target(BASE + i))
            .collect()
    }
    fn hit(&mut self, h: Hit) -> Option<&'static str> {
        let a = self.saved.actors.get_mut(h.id.checked_sub(BASE)?)?;
        if !a.spawned {
            return None;
        }
        if h.kind.means() == crate::combat::DamageKind::Electric {
            a.electric = crate::electric::LIFE;
        }
        a.body.hit(h)
    }
    fn loot_sources(&self) -> Vec<crate::loot::Source> {
        self.saved
            .actors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.spawned)
            .map(|(i, a)| crate::loot::Source {
                id: BASE + i,
                feet: a.body.feet,
                grade: crate::loot::Grade::Medium,
                dead: a.body.health == 0.,
            })
            .collect()
    }
    fn objective(&self) -> Option<String> {
        Some("Cross the battlefield and reach the castle approach.".into())
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        let saved: Saved = state::load(v, state::Visit { returning: false })?;
        ensure!(
            !self.data.easy
                || saved
                    .groups
                    .iter()
                    .enumerate()
                    .all(|(k, g)| k % 2 == 1 || !g.started),
            "Harder companies in an Easy save"
        );
        self.saved = saved;
        self.rebuild(map)
    }
    fn upgrade(&self) -> crate::level::Upgrade {
        crate::level::Upgrade {
            rearm: (1..=8).map(|n| format!("Spawner{n:02}")).collect(),
            respawn: crate::level::Respawn::Always,
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
    id: "grounds2",
    applies: |m, e| super::first_visit(m, e, "grounds2"),
    load: |a, m, _, _| Ok(Box::new(Royale::load(a, m)?)),
    art: Some(|a, _, o| {
        Ok(Box::new(art::Art::load(
            a,
            o.downcast_ref::<Royale>().unwrap(),
        )?))
    }),
    owns_submodel: |_, e| {
        e.get("targetname")
            .is_some_and(|n| n == "start_clip" || n.starts_with("fall"))
            || e.get("model")
                .is_some_and(|m| matches!(m.as_str(), "*3" | "*27"))
    },
    owns_npc: |n, _| {
        matches!(
            n,
            "good1"
                | "good2"
                | "good3"
                | "good4"
                | "bad1"
                | "bad2"
                | "bad3"
                | "bad4"
                | "spawnfall01"
        ) || n.starts_with("spawnbad")
    },
    target_base: Some(BASE),
    story_beats: &[],
    checks: &[
        Check {
            flag: "--grounds2-check",
            help: "Verify Battle Royale arrival, collapse, companies and persistence.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--grounds2-route-check",
            help: "Play from Royal Rage across Battle Royale to Ascension.",
            run: Run::Windowed(route::check),
        },
        Check {
            flag: "--grounds2-render-check",
            help: "Capture Battle Royale introduction and reinforcements.",
            run: Run::Windowed(check::render),
        },
        Check {
            flag: "--grounds2-save-check",
            help: "Round trip battlefield scenes and guards through native save files.",
            run: Run::Windowed(saves::run),
        },
    ],
    save_cases: &[],
    visibility: &[],
};

pub(crate) use route::drive as drive_route;
