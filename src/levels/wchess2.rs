//! Castling: one owner for the royal scenes, door access, duels and bound exit.
mod art;
mod cast;
mod check;
pub(crate) use check::drive as drive_route;
mod data;
mod motion;
mod save;
mod scene;
use super::{state, Check, Registration, Run};
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    event::{Condition, Facts},
    interaction::{Events, Interactions},
    inventory::Stats,
    level::{LevelArt, LevelController, TriggerInfo},
    movement::{Controls, Player},
    skeletal::Transform,
    story::{BeatSpec, Story},
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
pub use save::stage;
use serde::{Deserialize, Serialize};
use std::{any::Any, collections::BTreeSet};
const BASE: usize = 8_000_000;
pub const TALK: &str = "wchess2_king_audience";
pub const LAST: &str = "wchess2_king_departure";
const ALICE: &[&str] = &[
    "idle_stand",
    "walk",
    "darkened_lookingglass",
    "changeweapon",
    "ready",
];
const KING: &[&str] = &["idle", "walk", "talk1", "talk2", "talk3", "talk4"];
const EXIT: crate::level::spec::ExitSpec = crate::level::spec::ExitSpec {
    map: "rchess1",
    entrance: "rchess1_start1",
};
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    age: f32,
    queen: bool,
    queen_release: Option<f32>,
    king: bool,
    portal: Option<f32>,
    scene: Option<scene::Scene>,
    pending: Option<scene::Kind>,
    fired: BTreeSet<String>,
    cast: Vec<cast::Actor>,
    doors: Vec<motion::DoorState>,
    exit: crate::level::exit::ExitState,
    damage: f32,
    fade: f32,
    legacy: bool,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        state::clock("Castling age", self.age, 1e7)?;
        state::clock("Castling fade", self.fade, 0.5)?;
        state::clock("Castling damage", self.damage, 1000.)?;
        state::clock("exit retry", self.exit.retry_time, 1.)?;
        ensure!(
            self.king == self.portal.is_some() && (!self.king || self.queen),
            "Invalid royal completion"
        );
        ensure!(!self.exit.committed || self.king, "Premature Castling exit");
        ensure!(
            self.scene.is_none() || self.pending.is_none(),
            "Overlapping royal scenes"
        );
        if let Some(t) = self.portal {
            state::clock("Portal epoch", t, self.age)?;
        }
        for kind in self
            .pending
            .into_iter()
            .chain(self.scene.as_ref().map(|s| s.kind))
        {
            ensure!(
                !self.king
                    && if kind == scene::Kind::Queen {
                        !self.queen
                    } else {
                        self.queen
                    },
                "Invalid pending royal scene"
            );
        }
        if let Some(s) = &self.scene {
            s.validate()?;
            ensure!(
                !(self.king || (s.kind == scene::Kind::Queen && self.queen)),
                "Replayed royal scene"
            );
        }
        ensure!(
            self.fired.len() < 64 && self.cast.len() < 100 && self.doors.len() == 33,
            "Invalid Castling collection"
        );
        if let Some(t) = self.queen_release {
            state::clock("Queen release", t, 0.5)?;
            ensure!(self.queen, "Hostiles before Queen completion");
        }
        for a in &self.cast {
            a.validate()?;
        }
        for d in &self.doors {
            d.validate()?;
            if let Some(t) = d.started {
                state::clock("door epoch", t, self.age)?;
            }
        }
        Ok(())
    }
}
struct Castle {
    saved: Saved,
    data: data::Data,
    objects: Vec<motion::Object>,
    player_pose: Transform,
}
impl Castle {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let (objects, doors) = motion::load(map)?;
        let mut r = Self {
            saved: Saved {
                version: 1,
                age: 0.,
                queen: false,
                queen_release: None,
                king: false,
                portal: None,
                scene: None,
                pending: None,
                fired: BTreeSet::new(),
                cast: cast::load(map)?,
                doors,
                exit: Default::default(),
                damage: 0.,
                fade: 0.,
                legacy: false,
            },
            data: data::Data::load(a, map)?,
            objects,
            player_pose: Transform {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
        };
        r.rebuild(map)?;
        Ok(r)
    }
    fn fire(&mut self, n: &str) -> bool {
        self.saved.fired.insert(n.into())
    }
    fn group_ready(&self, g: &str) -> bool {
        let number = g.as_bytes().first().copied().unwrap_or(b'0');
        if !(b'1'..=b'5').contains(&number) {
            return false;
        }
        if number >= b'4' && !self.saved.king {
            return false;
        }
        if number <= b'3'
            && (self.saved.king || self.saved.fired.contains("start_battle_group2_thread"))
        {
            return false;
        }
        !['a', 'b'].iter().any(|c| {
            self.saved
                .fired
                .contains(&format!("enemy_group{}{c}_thread", number as char))
        })
    }
}
impl LevelController for Castle {
    fn id(&self) -> &'static str {
        "wchess2"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("wchess2.queen", !self.saved.queen && !self.scripted());
        f.flag(
            "wchess2.king",
            self.saved.queen && !self.saved.king && !self.scripted(),
        );
        f.flag("wchess2.exit", self.saved.king && !self.scripted());
        for g in ["1a", "1b", "2a", "2b", "3a", "3b", "4a", "4b", "5a", "5b"] {
            f.flag(&format!("wchess2.group{g}"), self.group_ready(g));
        }
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        if t.exit == Some("rchess1") {
            return Some(Condition::flag("wchess2.exit"));
        }
        match t.thread {
            "cinema_queen_abduction_thread" => Some(Condition::flag("wchess2.queen")),
            "cinema_king_thread" => Some(Condition::flag("wchess2.king")),
            n => n
                .strip_prefix("enemy_group")
                .and_then(|s| s.strip_suffix("_thread"))
                .map(|g| Condition::flag(&format!("wchess2.group{g}"))),
        }
    }
    fn event(&mut self, n: &str) -> Option<Events> {
        match n {
            "cinema_queen_abduction_thread" => {
                if !self.saved.queen && !self.scripted() {
                    self.saved.pending = Some(scene::Kind::Queen);
                }
            }
            "cinema_king_thread" => {
                if self.saved.queen && !self.saved.king && !self.scripted() {
                    self.saved.pending = Some(scene::Kind::King);
                }
            }
            _ => {
                if !self.cast_event(n) {
                    return None;
                }
            }
        }
        Some(Events::default())
    }
    fn prepare_player(&mut self, _: &mut Stats, p: &mut Player) {
        self.player_pose = Transform {
            translation: p.feet,
            rotation: Quat::from_rotation_z(p.script_facing),
        };
    }
    fn update(&mut self, w: &mut World, p: &Player, aim: Vec3, use_pressed: bool) -> Events {
        if !self.scripted() && use_pressed {
            if let Some(k) = self.use_door(w, p.eye(), aim) {
                self.saved.doors[k].open();
            }
        }
        let mut e = Events::default();
        if self.saved.exit.committed {
            e.transition = self.saved.exit.request(EXIT);
        }
        e
    }
    fn prompt(&self, w: &World, eye: Vec3, aim: Vec3) -> Option<&'static str> {
        (!self.scripted())
            .then(|| self.use_door(w, eye, aim))
            .flatten()
            .map(|_| "E: open door")
    }
    fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. {
            return Ok(());
        }
        let dt = dt.min(0.1);
        let old = self.portal_pose();
        let support = self.portal_support(p);
        self.saved.age += dt;
        if let Some(t) = &mut self.saved.queen_release {
            *t = (*t - dt).max(0.);
            if *t == 0. {
                self.saved.queen_release = None;
                for n in ["r_knight_queen1", "r_knight_queen2"] {
                    if let Some(a) = self.actor_mut(n) {
                        a.piece.script_wait = false;
                    }
                }
                self.spawn("r_rook_attack1_spawn");
                self.spawn("r_rook_attack2_spawn");
            }
        }
        self.saved.fade = (self.saved.fade - dt).max(0.);
        self.saved.exit.advance(dt);
        self.step_doors(dt, p);
        self.step_scene(dt, map, w, p)?;
        self.move_cast(dt);
        let delta = self.portal_pose().translation - old.translation;
        if support && delta.length_squared() > 0. {
            let others: Vec<_> = fixed
                .iter()
                .cloned()
                .chain(
                    self.objects
                        .iter()
                        .filter(|o| o.id != 23 && o.solid)
                        .map(|o| o.collider.clone()),
                )
                .collect();
            w.set_dynamic(others);
            let tr = w.body_trace(p.feet, p.feet + delta);
            if !tr.start_solid {
                p.feet += delta * tr.fraction;
                p.grounded = true;
            }
        }
        self.rebuild(map)?;
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        self.player_pose = Transform {
            translation: p.feet,
            rotation: Quat::from_rotation_z(p.script_facing),
        };
        Ok(())
    }
    fn scripted(&self) -> bool {
        self.saved.scene.is_some() || self.saved.pending.is_some()
    }
    fn scene_id(&self) -> Option<&'static str> {
        self.saved
            .scene
            .as_ref()
            .map(|s| s.kind.id())
            .or(self.saved.pending.map(|k| k.id()))
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        self.scene_camera()
    }
    fn fade(&self) -> Option<(Color, f32)> {
        self.scene_fade()
    }
    fn skip(
        &mut self,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        story: &mut Story,
    ) -> Result<bool> {
        if !self.scripted() {
            return Ok(false);
        }
        self.start_scene(p);
        self.finish_scene(map, w, p, true)?;
        story.finish_sequence(TALK);
        story.finish_sequence(LAST);
        Ok(true)
    }
    fn entry_story(&mut self, _: &mut Story) -> bool {
        true
    }
    fn prepare_story(&self, s: &mut Story) -> bool {
        if let Some(c) = &self.saved.scene {
            if let Some(id) = c.dialogue() {
                s.resume_scene(id, c.line, c.line_time);
                return true;
            }
            return false;
        }
        true
    }
    fn sync_story(&mut self, s: &Story) {
        self.sync_dialogue(s);
    }
    fn dialogue_complete(&mut self, n: &str) -> Events {
        self.end_dialogue(n);
        Events::default()
    }
    fn trigger_pose(&self, n: &str, base: Vec3) -> Option<(Vec3, Quat)> {
        (n == "exit_trigger").then(|| {
            (
                base + self.portal_pose().translation - self.data.point("exit_portal").translation,
                Quat::IDENTITY,
            )
        })
    }
    fn exit_contact(&mut self, d: &(String, Option<String>)) -> Option<Events> {
        if d.0 != "rchess1" {
            return None;
        }
        let mut e = Events::default();
        if self.saved.king && !self.scripted() {
            self.saved.exit.committed = true;
            e.transition = self.saved.exit.request(EXIT);
        }
        Some(e)
    }
    fn transition_failed(&mut self, e: &(String, Option<String>)) {
        if e.0 == "rchess1" {
            self.saved.exit.failed();
        }
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.objects
            .iter()
            .filter(|o| o.draw)
            .map(|o| (o.model, o.pose.translation, o.pose.rotation))
            .collect()
    }
    fn colliders(&self) -> Vec<Collider> {
        let mut c: Vec<_> = self
            .objects
            .iter()
            .filter(|o| o.solid)
            .map(|o| o.collider.clone())
            .collect();
        if self.saved.king {
            let p = self.data.point("w_king_dest1").translation;
            c.push(Collider::box_bounds(
                p - vec3(24., 24., 0.),
                p + vec3(24., 24., 128.),
            ));
        }
        c
    }
    fn targets(&self) -> Vec<crate::combat::Target> {
        self.cast_targets()
    }
    fn hostile_target(&self, id: usize) -> bool {
        self.saved.cast.iter().any(|a| a.id() == id && !a.white)
    }
    fn melee_target(&self, id: usize) -> bool {
        self.saved
            .cast
            .iter()
            .any(|a| a.id() == id && a.piece.kind != crate::chess::Kind::Bishop)
    }
    fn hit(&mut self, h: crate::combat::Hit) -> Option<&'static str> {
        self.cast_hit(h)
    }
    fn combat(&mut self, c: &mut crate::level::Combat<'_>) -> crate::combat::Feedback {
        self.fight(c)
    }
    fn loot_sources(&self) -> Vec<crate::loot::Source> {
        self.cast_loot()
    }
    fn sound_state(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        if let Some(s) = self
            .saved
            .scene
            .as_ref()
            .filter(|s| s.kind == scene::Kind::Queen && s.time >= 1.)
        {
            loops.push(crate::audio::LoopCue {
                id: BASE + 90_001,
                clock: Some(s.time - 1.),
                path: "sound/character/chess_piece/queen/idle_struggle02.wav",
                origin: self.queen_parent().translation,
            });
        }
        for (k, d) in self.saved.doors.iter().enumerate() {
            if let Some(t) = d.started {
                let o = self.objects.iter().find(|o| o.id == d.id).unwrap();
                if o.id != 10 {
                    clocks.push(crate::audio::world::Clock {
                        key: DOOR_KEYS[k],
                        time: self.saved.age - t,
                        period: None,
                        origin: o.base.translation,
                        cues: &[(0.001, "sound/world/door/castle_door2.wav")],
                    });
                }
            }
        }
    }
    fn objective(&self) -> Option<String> {
        Some(
            if !self.saved.queen {
                "Enter the castle"
            } else if !self.saved.king {
                "Reach the King"
            } else {
                "Enter the moving portal"
            }
            .into(),
        )
    }
    fn upgrade(&self) -> crate::level::Upgrade {
        crate::level::Upgrade {
            respawn: crate::level::Respawn::Always,
            rearm: self.data.threads.clone(),
            ..Default::default()
        }
    }
    fn upgraded(&mut self) {
        self.saved.legacy = true;
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        let s: Saved = state::load(v, state::Visit { returning: false })?;
        ensure!(
            s.cast.len() == self.saved.cast.len()
                && s.cast
                    .iter()
                    .zip(&self.saved.cast)
                    .all(|(a, b)| a.entity == b.entity
                        && a.instance == b.instance
                        && a.source == b.source
                        && a.counted == b.counted
                        && a.model == b.model
                        && a.name == b.name
                        && a.white == b.white
                        && a.floor == b.floor
                        && a.piece.kind == b.piece.kind),
            "Castling roster mismatch"
        );
        ensure!(
            s.doors
                .iter()
                .zip(&self.saved.doors)
                .all(|(a, b)| a.id == b.id),
            "Castling doors differ"
        );
        self.saved = s;
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
    id: "wchess2",
    applies: |m, e| super::first_visit(m, e, "wchess2"),
    load: |a, m, _, _| Ok(Box::new(Castle::load(a, m)?)),
    art: Some(|a, _, r| {
        Ok(Box::new(art::Art::load(
            a,
            r.downcast_ref::<Castle>().unwrap(),
        )?))
    }),
    owns_submodel: |m, e| m == "wchess2" && motion::owns(e),
    owns_npc: |_, m| m.starts_with("c_chess_"),
    target_base: Some(BASE),
    story_beats: &[
        BeatSpec {
            map: "wchess2",
            event: TALK,
            script: "../wchess2",
            thread: "cinema_king_thread",
            source_lines: 11,
            calls: crate::story::registry::Calls::Gated {
                key: "wchess2.audience",
                indices: &[0, 1, 2, 3, 4, 5, 6, 7, 8],
            },
        },
        BeatSpec {
            map: "wchess2",
            event: LAST,
            script: "../wchess2",
            thread: "cinema_king_thread",
            source_lines: 11,
            calls: crate::story::registry::Calls::Gated {
                key: "wchess2.pawn",
                indices: &[9, 10],
            },
        },
    ],
    checks: &[
        Check {
            flag: "--wchess2-floor-survey",
            help: "Record supported Castling route points.",
            run: Run::Headless(check::survey),
        },
        Check {
            flag: "--wchess2-render-check",
            help: "Render Castling royal scenes and portal.",
            run: Run::Windowed(check::render),
        },
        Check {
            flag: "--wchess2-check",
            help: "Verify Castling doors, scenes and portal persistence.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--wchess2-route-check",
            help: "Walk Castling into Checkmate in Red.",
            run: Run::Headless(check::route),
        },
        Check {
            flag: "--wchess2-route-skip-check",
            help: "Walk Castling with royal scenes skipped.",
            run: Run::Headless(check::route_skip),
        },
    ],
    save_cases: &[
        super::SaveCase {
            name: "wchess2-queen-mid",
            visit: "wchess2$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "wchess2-king-mid",
            visit: "wchess2$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "wchess2-exit-open",
            visit: "wchess2$first",
            stage: None,
            behavior: None,
        },
    ],
    visibility: &[],
};
fn owner(i: &mut Interactions) -> Result<&mut Castle> {
    i.levels
        .iter_mut()
        .find_map(|s| s.ctl.downcast_mut())
        .context("Castling owner missing")
}

const DOOR_KEYS: &[&str] = &[
    "wchess2.door-0",
    "wchess2.door-1",
    "wchess2.door-2",
    "wchess2.door-3",
    "wchess2.door-4",
    "wchess2.door-5",
    "wchess2.door-6",
    "wchess2.door-7",
    "wchess2.door-8",
    "wchess2.door-9",
    "wchess2.door-10",
    "wchess2.door-11",
    "wchess2.door-12",
    "wchess2.door-13",
    "wchess2.door-14",
    "wchess2.door-15",
    "wchess2.door-16",
    "wchess2.door-17",
    "wchess2.door-18",
    "wchess2.door-19",
    "wchess2.door-20",
    "wchess2.door-21",
    "wchess2.door-22",
    "wchess2.door-23",
    "wchess2.door-24",
    "wchess2.door-25",
    "wchess2.door-26",
    "wchess2.door-27",
    "wchess2.door-28",
    "wchess2.door-29",
    "wchess2.door-30",
    "wchess2.door-31",
    "wchess2.door-32",
];
