//! Majestic Maze: the live child cast holds the authored plate and sliding gates.
mod check;
mod data;
mod route;
mod save_check;
use super::{state, Check, Registration, Run};
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    combat::Target,
    event::{Condition, Facts},
    interaction::Events,
    level::{CompanionContact, LevelController, TriggerInfo, Upgrade},
    movement::Player,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;

const EXIT: crate::level::spec::ExitSpec = crate::level::spec::ExitSpec {
    map: "tower1",
    entrance: "tower1_start1",
};
#[derive(Clone, Default, Serialize, Deserialize)]
struct Saved {
    version: u8,
    end_seen: bool,
    start_seen: bool,
    following: bool,
    holding: bool,
    occupied: bool,
    pulse: f32,
    release: f32,
    doors: [f32; 3],
    debt: f64,
    exit: crate::level::exit::ExitState,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, v: state::Visit) -> Result<()> {
        ensure!(!v.returning, "Maze is a first visit");
        state::clock("plate pulse", self.pulse, 0.2)?;
        state::clock("gate release", self.release, 0.3)?;
        for p in self.doors {
            state::clock("gate fraction", p, 1.)?;
        }
        ensure!(
            (0. ..0.009).contains(&self.debt),
            "Invalid maze step remainder"
        );
        ensure!(
            !self.following || self.start_seen,
            "Follower before handoff"
        );
        ensure!(!self.holding || self.following, "Held plate before escort");
        state::clock("exit retry", self.exit.retry_time, 1.)?;
        ensure!(
            !self.exit.committed || (self.holding && self.doors[1..].iter().all(|v| *v > 0.98)),
            "Maze exit bypassed escort"
        );
        Ok(())
    }
}
pub struct Maze {
    saved: Saved,
    data: data::Data,
    companions: Vec<CompanionContact>,
    actors: Vec<Target>,
}
impl Maze {
    fn load(map: &Bsp) -> Result<Self> {
        Ok(Self {
            saved: Saved {
                version: 1,
                ..Default::default()
            },
            data: data::Data::load(map)?,
            companions: vec![],
            actors: vec![],
        })
    }
    fn ready(&self) -> bool {
        self.saved.holding && self.saved.doors[1..].iter().all(|v| *v > 0.98)
    }
    fn touch(&self, center: Vec3, half: Vec3) -> bool {
        self.data.plate.touches(center, center, half)
    }
}
impl LevelController for Maze {
    fn id(&self) -> &'static str {
        "hedge1"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("hedge1.ready", self.ready());
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        match t.id.0 {
            // These two occupancy volumes are sampled against real actor bodies below.
            // Alice cannot invoke the monsters-only Hold thread through the generic trigger path.
            21 | 30 => Some(Condition::Always.not()),
            29 => Some(Condition::flag("hedge1.ready")),
            _ => None,
        }
    }
    fn event(&mut self, thread: &str) -> Option<Events> {
        match thread.trim_end_matches("()") {
            "SeekEnd" => self.saved.end_seen = true,
            "SeekStart" => self.saved.start_seen = true,
            "SeekHold" => {}
            _ => return None,
        }
        Some(Events::default())
    }
    fn companion_contacts(&mut self, c: &[CompanionContact]) {
        self.companions = c.to_vec();
    }
    fn actor_contacts(&mut self, a: &[Target]) {
        self.actors = a.to_vec();
    }
    fn advance(
        &mut self,
        dt: f32,
        _: &Bsp,
        world: &mut World,
        player: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. {
            return Ok(());
        }
        self.saved.debt += f64::from(dt.min(0.1));
        while self.saved.debt + 1e-9 >= 1. / 120. {
            self.saved.debt = (self.saved.debt - 1. / 120.).max(0.);
            let dt = 1. / 120.;
            self.saved.exit.advance(dt);
            self.saved.following = self
                .companions
                .iter()
                .any(|c| c.name == "seek_kid_chase" && c.visible);
            // Older NPC-only saves may already contain the completed handoff.
            self.saved.start_seen |= self.saved.following;
            self.saved.holding = self.companions.iter().any(|c| {
                c.name == "seek_kid_chase"
                    && c.visible
                    && c.holding
                    && self.touch(c.feet + Vec3::Z * c.half.z, c.half)
            });
            let occupied = self.touch(player.feet + PLAYER_CENTER, PLAYER_HALF)
                || self
                    .companions
                    .iter()
                    .any(|c| c.visible && self.touch(c.feet + Vec3::Z * c.half.z, c.half))
                || self.actors.iter().any(|a| self.touch(a.center, a.half));
            self.saved.pulse = (self.saved.pulse - dt).max(0.);
            self.saved.release = (self.saved.release - dt).max(0.);
            if occupied && (!self.saved.occupied || self.saved.pulse <= 0.) {
                self.saved.pulse = 0.2;
                self.saved.release = 0.3;
            }
            self.saved.occupied = occupied;
            let opening = self.saved.release > 0.;
            for i in 0..3 {
                let old = self.saved.doors[i];
                let p = (old
                    + if opening {
                        dt / self.data.doors[i].duration
                    } else {
                        -dt / self.data.doors[i].duration
                    })
                .clamp(0., 1.);
                self.saved.doors[i] = p;
                world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
                // A descending plate carries Alice; a closing gate waits instead of crushing.
                let lift = self.data.doors[i].slide * (p - old);
                let on_plate = i == 0
                    && player.grounded
                    && (player.feet.z
                        - (self.data.doors[i].top + old * self.data.doors[i].slide.z))
                        .abs()
                        < 1.
                    && self.data.doors[i]
                        .shape
                        .at(
                            self.data.doors[i].base + self.data.doors[i].slide * old,
                            Quat::IDENTITY,
                        )
                        .rider_feet(player.feet)
                        .is_some();
                let candidate = player.feet + if on_plate { lift } else { Vec3::ZERO };
                if world.body_clear(candidate) {
                    player.feet = candidate;
                } else {
                    self.saved.doors[i] = old;
                }
            }
            world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        }
        Ok(())
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.data
            .doors
            .iter()
            .zip(self.saved.doors)
            .map(|(d, p)| (d.model, d.base + d.slide * p, Quat::IDENTITY))
            .collect()
    }
    fn colliders(&self) -> Vec<Collider> {
        self.data
            .doors
            .iter()
            .zip(self.saved.doors)
            .map(|(d, p)| d.shape.at(d.base + d.slide * p, Quat::IDENTITY))
            .collect()
    }
    fn exit_contact(&mut self, d: &(String, Option<String>)) -> Option<Events> {
        if d != &EXIT.destination() {
            return None;
        }
        let mut e = Events::default();
        if self.ready() {
            self.saved.exit.committed = true;
            e.transition = self.saved.exit.request(EXIT);
        }
        Some(e)
    }
    fn teleport_destination(&self, destination: Vec3) -> Option<Vec3> {
        (destination.distance_squared(self.data.portal_origin) < 0.01)
            .then_some(self.data.portal_landing)
    }
    fn update(&mut self, _: &mut World, _: &Player, _: Vec3, _: bool) -> Events {
        let mut e = Events::default();
        e.transition = self.saved.exit.request(EXIT);
        e
    }
    fn transition_failed(&mut self, d: &(String, Option<String>)) {
        if d == &EXIT.destination() {
            self.saved.exit.failed();
        }
    }
    fn upgrade(&self) -> Upgrade {
        Upgrade {
            rearm: vec!["SeekEnd".into(), "SeekStart".into()],
            ..Default::default()
        }
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, _: &Bsp) -> Result<()> {
        self.saved = state::load(v, state::Visit { returning: false })?;
        Ok(())
    }
    fn objective(&self) -> Option<String> {
        Some(
            if self.ready() {
                "The child is holding the plate. Go through the double gate."
            } else if self.saved.following {
                "Find the child in the maze and guide him back to the pressure plate."
            } else if self.saved.start_seen {
                "Find the child in the maze. The nearby portal is a shortcut."
            } else {
                "Follow the maze to the child and the pressure plate."
            }
            .into(),
        )
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
pub static REGISTRATION: Registration = Registration {
    id: "hedge1",
    applies: |m, e| super::first_visit(m, e, "hedge1"),
    load: |_, m, _, _| Ok(Box::new(Maze::load(m)?)),
    art: None,
    owns_submodel: |m, e| m == "hedge1" && e.get("classname").is_some_and(|s| s == "func_door"),
    owns_npc: |_, _| false,
    target_base: None,
    story_beats: &[],
    checks: &[
        Check {
            flag: "--hedge1-check",
            help: "Verify the maze escort gate, plate, pause and saves.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--hedge1-route-check",
            help: "Reach tower1 through the native Majestic Maze escort.",
            run: Run::Windowed(|a| Box::pin(route::check(a))),
        },
        Check {
            flag: "--hedge1-render-check",
            help: "Capture Majestic Maze progression checkpoints.",
            run: Run::Windowed(|a| Box::pin(check::render(a))),
        },
        Check {
            flag: "--hedge1-save-write",
            help: "Write maze route checkpoints through the real save system.",
            run: Run::Windowed(|a| Box::pin(save_check::run(a, true))),
        },
        Check {
            flag: "--hedge1-save-read",
            help: "Restore maze checkpoints in a fresh process.",
            run: Run::Windowed(|a| Box::pin(save_check::run(a, false))),
        },
    ],
    save_cases: &[],
    visibility: &[],
};

pub(crate) use route::drive as drive_route;
