//! Pale Realm. One owner for chess lessons, factions, gates and water machinery.
mod art;
mod cast;
mod check;
pub(crate) use check::drive as drive_route;
mod data;
mod graph;
mod motion;
mod save;
mod scene;
mod sound;
mod tests;
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
    story::Story,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
pub use save::stage;
use serde::{Deserialize, Serialize};
use std::{any::Any, collections::BTreeSet};
const BASE: usize = 7_900_000;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Piece {
    Bishop,
    Knight,
}
impl Piece {
    fn name(self) -> &'static str {
        match self {
            Self::Bishop => "bishop",
            Self::Knight => "knight",
        }
    }
    fn nodes(self) -> &'static [graph::Node] {
        match self {
            Self::Bishop => graph::BISHOP,
            Self::Knight => graph::KNIGHT,
        }
    }
    fn dirs(self) -> [&'static str; 4] {
        match self {
            Self::Bishop => ["nw", "ne", "se", "sw"],
            Self::Knight => ["n", "e", "s", "w"],
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Move {
    node: usize,
    leg: usize,
    path: Vec<Vec3>,
    from: Vec3,
    elapsed: f32,
    duration: f32,
}
#[derive(Clone, Serialize, Deserialize)]
struct Board {
    piece: Piece,
    node: usize,
    moving: Option<Move>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    #[serde(default)]
    pull: Option<(u8, f32)>,
    #[serde(default)]
    lever_started: [Option<f32>; 2],
    #[serde(default)]
    knight_opened: Option<f32>,
    version: u8,
    age: f32,
    intro: bool,
    scene: Option<scene::Scene>,
    pending: Option<scene::Kind>,
    board: Option<Board>,
    bishop_done: bool,
    knight_done: bool,
    knight_gate: bool,
    bell: Option<f32>,
    water: Option<f32>,
    spikes: Option<f32>,
    spikes_off: Option<f32>,
    gang: Option<f32>,
    fired: BTreeSet<String>,
    cast: Vec<cast::Actor>,
    elevator: motion::Lift,
    secret: bool,
    damage: f32,
    legacy: bool,
    retry: Option<Piece>,
    refusal: Option<f32>,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        state::clock("chess age", self.age, 1e7)?;
        for t in [
            self.bell,
            self.water,
            self.spikes,
            self.spikes_off,
            self.gang,
            self.refusal,
            self.knight_opened,
        ]
        .into_iter()
        .flatten()
        {
            state::clock("chess event", t, self.age)?;
        }
        ensure!(
            self.damage.is_finite() && (0. ..=1000.).contains(&self.damage),
            "Invalid chess damage"
        );
        ensure!(
            self.fired.len() < 128 && self.cast.len() < 256,
            "Invalid chess state size"
        );
        ensure!(
            self.scene.is_none() || self.pending.is_none(),
            "Two chess scenes queued"
        );
        ensure!(
            self.board.is_none() || (self.scene.is_none() && self.pending.is_none()),
            "Scene and disguise overlap"
        );
        ensure!(
            self.knight_opened.is_none() || self.knight_gate,
            "Moving closed knight gate"
        );
        for t in self.lever_started.into_iter().flatten() {
            state::clock("lever origin", t, self.age)?;
        }
        self.elevator.validate()?;
        if let Some((i, t)) = self.pull {
            ensure!(i < 2, "Invalid lever");
            state::clock("lever pull", t, 10.)?;
        }
        if let Some(s) = &self.scene {
            s.validate()?;
        }
        if let Some(b) = &self.board {
            ensure!(b.node < b.piece.nodes().len(), "Invalid chess node");
            ensure!(
                !(if b.piece == Piece::Bishop {
                    self.bishop_done
                } else {
                    self.knight_done
                }),
                "Completed disguise replay"
            );
            if let Some(m) = &b.moving {
                ensure!(
                    m.node < b.piece.nodes().len()
                        && m.leg < m.path.len()
                        && m.path.len() <= 4
                        && m.path.iter().all(|p| p.is_finite())
                        && m.from.is_finite(),
                    "Invalid chess move"
                );
                state::clock("chess move", m.elapsed, m.duration)?;
                ensure!(
                    m.duration.is_finite() && (0.001..=10.).contains(&m.duration),
                    "Invalid chess speed"
                );
            }
        }
        for actor in &self.cast {
            actor.validate()?;
        }
        ensure!(
            !self.knight_done || self.bishop_done,
            "Knight completed before bishop"
        );
        ensure!(
            !self.knight_gate || self.bishop_done,
            "Knight gate before bishop"
        );
        Ok(())
    }
}
struct Realm {
    saved: Saved,
    data: data::Data,
    objects: Vec<motion::Object>,
    player_pose: Transform,
}
impl Realm {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let data = data::Data::load(a, map)?;
        let saved = Saved {
            knight_opened: None,
            pull: None,
            lever_started: [None, None],
            version: 1,
            age: 0.,
            intro: false,
            scene: None,
            pending: Some(scene::Kind::Intro),
            board: None,
            bishop_done: false,
            knight_done: false,
            knight_gate: false,
            bell: None,
            water: None,
            spikes: None,
            spikes_off: None,
            gang: None,
            fired: BTreeSet::new(),
            cast: cast::load(map)?,
            elevator: motion::Lift::new(),
            secret: false,
            damage: 0.,
            legacy: false,
            retry: None,
            refusal: None,
        };
        let mut r = Self {
            saved,
            data,
            objects: motion::load(map)?,
            player_pose: Transform {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
        };
        r.rebuild(map)?;
        Ok(r)
    }
    fn water_ready(&self) -> bool {
        self.saved.water.is_some_and(|t| self.saved.age - t >= 8.)
    }
    fn direction(&mut self, d: usize, p: &Player) {
        let Some(b) = &mut self.saved.board else {
            return;
        };
        if b.moving.is_some() {
            return;
        }
        let Some(n) = b.piece.nodes()[b.node].next[d] else {
            return;
        };
        let mut from = p.feet;
        let mut path = Vec::new();
        for (&square, &heading) in b.piece.nodes()[n]
            .legs
            .iter()
            .zip(b.piece.nodes()[n].headings)
        {
            let goal = self.data.square(b.piece, square) + Vec3::Z * crate::collision::SKIN;
            if b.piece == Piece::Knight
                && (goal.x - from.x).abs() > 32.
                && (goal.y - from.y).abs() > 32.
            {
                let corner = if matches!(heading, "e" | "w") {
                    vec3(goal.x, from.y, goal.z)
                } else {
                    vec3(from.x, goal.y, goal.z)
                };
                path.push(corner);
            }
            path.push(goal);
            from = goal;
        }
        b.moving = Some(Move {
            node: n,
            leg: 0,
            from: p.feet,
            elapsed: 0.,
            duration: p.feet.distance(path[0]).max(1.) / 260.,
            path,
        });
    }
    fn step_board(&mut self, dt: f32, p: &mut Player, w: &World) {
        let Some(b) = &mut self.saved.board else {
            return;
        };
        if let Some(m) = &mut b.moving {
            m.elapsed = (m.elapsed + dt).min(m.duration);
            let node = &b.piece.nodes()[m.node];
            let goal = m.path[m.leg];
            let next = m.from.lerp(goal, m.elapsed / m.duration);
            if self.objects.iter().any(|o| {
                o.hazard
                    && o.solid
                    && o.volume
                        .trace(p.feet + PLAYER_CENTER, next + PLAYER_CENTER, PLAYER_HALF)
                        .fraction
                        < 1.
            }) {
                self.saved.damage = 1000.;
                self.saved.retry = Some(b.piece);
                self.saved.board = None;
                p.velocity = Vec3::ZERO;
                return;
            }
            let trace = w.body_trace(p.feet, next);
            if !trace.start_solid && trace.fraction < 1. {
                m.elapsed = (m.elapsed - dt).max(0.);
                return;
            }
            p.feet = next;
            p.velocity = Vec3::ZERO;
            let v = goal - m.from;
            p.script_facing = v.y.atan2(v.x);
            p.cancel_climb();
            p.release_rope();
            if m.elapsed >= m.duration {
                if m.leg + 1 < m.path.len() {
                    m.leg += 1;
                    m.from = goal;
                    m.elapsed = 0.;
                    m.duration = goal.distance(m.path[m.leg]).max(1.) / 260.;
                } else {
                    b.node = m.node;
                    b.moving = None;
                    match node.end {
                        graph::End::Goal => {
                            if b.piece == Piece::Bishop {
                                self.saved.bishop_done = true;
                            } else {
                                self.saved.knight_done = true;
                            }
                            self.saved.board = None;
                        }
                        graph::End::Drop => {
                            self.saved.retry = Some(b.piece);
                            self.saved.board = None;
                            p.grounded = false;
                        }
                        graph::End::Stay => {}
                    }
                }
            }
        } else {
            let piece = b.piece;
            let center = self
                .data
                .point(&format!("{}{}", piece.name(), piece.nodes()[b.node].square))
                .translation;
            let prefix = if piece == Piece::Bishop { "bs" } else { "kn" };
            for (i, d) in piece.dirs().iter().enumerate() {
                if piece.nodes()[b.node].next[i].is_none() {
                    continue;
                }
                let name = format!("{prefix}trig_{d}");
                let base = self.data.point(&name).translation;
                let parent = self.data.point(&format!("{name}_parent")).translation;
                // The source binds each touch strip to its parent, then moves that parent.
                if let Some(c) = self.data.volumes.get(&name) {
                    let local = p.feet + PLAYER_CENTER - (center - parent);
                    if c.trace(local, local, PLAYER_HALF).start_solid {
                        let _ = base;
                        self.direction(i, p);
                        break;
                    }
                }
            }
        }
    }
    fn fire(&mut self, n: &str) -> bool {
        if !self.saved.fired.insert(n.into()) {
            return false;
        }
        true
    }
    fn lever(&self, w: &World, eye: Vec3, aim: Vec3) -> Option<usize> {
        if self.scripted() || self.saved.board.is_some() || self.saved.pull.is_some() {
            return None;
        }
        ["bell_lever", "water_lever"]
            .iter()
            .enumerate()
            .find_map(|(i, n)| {
                if (i == 0 && self.saved.bell.is_some())
                    || (i == 1 && self.saved.water.is_some())
                    || self.saved.fired.contains(*n)
                {
                    return None;
                }
                let at = self.data.points.get(*n)?;
                let at = at.point(vec3(36., 0., 26.));
                let delta = at - eye;
                let dist = delta.length();
                let trace = w.sweep(eye, at, Vec3::splat(0.5));
                (dist <= 125.
                    && dist > 0.01
                    && aim.normalize_or_zero().dot(delta / dist) > 0.45
                    && !trace.start_solid
                    && trace.fraction >= 1.)
                    .then_some(i)
            })
    }
}
impl LevelController for Realm {
    fn id(&self) -> &'static str {
        "wchess1"
    }
    fn shootable_thread(&self, n: &str) -> bool {
        matches!(n, "white_pawn_bullied_thread" | "OpenFloatSecret")
    }
    fn door_locked(&self, id: crate::entity::Id) -> Option<bool> {
        matches!(id.0, 121 | 122).then_some(true)
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag(
            "wchess1.knight",
            self.saved.knight_gate && !self.saved.knight_done,
        );
        f.flag(
            "wchess1.bishop",
            self.saved.intro
                && !self.saved.bishop_done
                && self.saved.board.is_none()
                && !self.scripted(),
        );
        f.flag(
            "wchess1.bell",
            self.saved.bell.is_some_and(|t| self.saved.age - t >= 5.) && !self.scripted(),
        );
        f.flag("wchess1.exit", self.water_ready() && self.saved.knight_done);
        f.flag(
            "wchess1.gang",
            !self.saved.fired.contains("white_pawn_bullied_thread"),
        );
        f.flag("wchess1.never", false);
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        match t.thread {
            "cinema_bishop_start_thread" => Some(Condition::flag("wchess1.bishop")),
            "cinema_knight_start_thread" => Some(Condition::flag("wchess1.knight")),
            "white_pawn_bullied_thread" => Some(Condition::flag("wchess1.gang")),
            "red_knight2_thread" => Some(Condition::flag("wchess1.bell")),
            _ if t.exit == Some("wchess2") => Some(Condition::flag("wchess1.exit")),
            _ if t.name.starts_with("bstrig_") || t.name.starts_with("kntrig_") => {
                Some(Condition::flag("wchess1.never"))
            }
            _ => None,
        }
    }
    fn receivers(&self, _: &crate::entity::Registry) -> Vec<crate::entity::Id> {
        self.saved
            .cast
            .iter()
            .map(|c| crate::entity::Id(c.entity))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    fn rules(&self, ctx: &crate::level::RuleContext<'_>) -> Vec<crate::event::Rule> {
        self.receivers(ctx.registry)
            .into_iter()
            .map(|id| crate::event::Rule {
                key: format!("wchess1/actor/{}", id.0),
                event: crate::event::Event::Entity(id, crate::event::Input::Activate),
                condition: Condition::Always,
                once: true,
                cooldown: 0.,
                actions: vec![crate::event::Action::Output(
                    crate::event::Effect::Activate(id),
                )],
            })
            .collect()
    }
    fn output(&mut self, e: &crate::event::Effect) -> Option<Events> {
        if let crate::event::Effect::Activate(id) = e {
            if self.activate_entity(
                id.0,
                match id.0 {
                    46 | 55 => 0.5,
                    1385 => 1.,
                    _ => 0.,
                },
            ) {
                return Some(Events::default());
            }
        }
        None
    }
    fn event(&mut self, n: &str) -> Option<Events> {
        match n {
            "cinema_wchess1_intro_thread" | "last_enemy_attack_thread" => {}
            "cinema_bishop_start_thread" => {
                if !self.saved.bishop_done && self.saved.board.is_none() && !self.scripted() {
                    self.saved.pending = Some(scene::Kind::Bishop);
                }
            }
            "cinema_knight_start_thread" => {
                if self.saved.knight_gate
                    && !self.saved.knight_done
                    && self.saved.board.is_none()
                    && !self.scripted()
                {
                    self.saved.pending = Some(scene::Kind::Knight);
                }
            }
            "cinema_ring_bell_thread" => {
                if self.saved.bell.is_none() {
                    self.saved.bell = Some(self.saved.age);
                }
            }
            "cinema_raise_water_thread" => {
                if self.saved.water.is_none() && self.fire(n) {
                    self.saved.pending = Some(scene::Kind::Water);
                }
            }
            "elevator_up_pause_thread" | "elevator_up_thread" => self.saved.elevator.up(),
            "elevator_down_thread" => self.saved.elevator.down(),
            "op1_spikes_thread" => {
                if self.saved.spikes.is_none() {
                    self.saved.spikes = Some(self.saved.age);
                    self.show("pawn_npc2");
                }
            }
            "OpenFloatSecret" => {
                self.saved.secret = true;
            }
            "NoNoThread" => {
                if !self.saved.fired.contains("bell_complete") {
                    self.saved.refusal = Some(self.saved.age);
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
    fn prepare_player(&mut self, stats: &mut Stats, p: &mut Player) {
        if !stats.alive() {
            if let Some(b) = self.saved.board.take() {
                self.saved.retry = Some(b.piece);
            }
        }
        self.player_pose = Transform {
            translation: p.feet,
            rotation: Quat::from_rotation_z(p.script_facing),
        };
    }
    fn filter_controls(&self, c: &mut Controls) {
        if self.saved.board.is_some() {
            c.jump = false;
            c.rise = 0.;
        }
    }
    fn hides_player(&self) -> bool {
        self.saved.board.is_some() || self.saved.pull.is_some()
    }
    fn blocks_weapons(&self) -> bool {
        self.saved.board.is_some() || self.saved.pull.is_some()
    }
    fn update(&mut self, w: &mut World, p: &Player, aim: Vec3, use_pressed: bool) -> Events {
        if use_pressed {
            if let Some(i) = self.lever(w, p.eye(), aim) {
                let name = if i == 0 { "bell_lever" } else { "water_lever" };
                self.fire(name);
                self.saved.pull = Some((i as u8, 0.));
                self.saved.lever_started[i] = Some(self.saved.age);
            }
        }
        Events::default()
    }
    fn prompt(&self, w: &World, eye: Vec3, aim: Vec3) -> Option<&'static str> {
        self.lever(w, eye, aim).map(|_| "E: use lever")
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
        self.saved.age += dt;
        if let Some((i, t)) = &mut self.saved.pull {
            *t += dt;
            p.velocity = Vec3::ZERO;
            if *t >= self.data.lever_duration {
                let i = *i;
                self.saved.pull = None;
                if i == 0 {
                    self.saved.bell = Some(self.saved.age);
                } else {
                    self.saved.pending = Some(scene::Kind::Water);
                }
            }
        }
        if let Some(piece) = self
            .saved
            .retry
            .filter(|_| !self.scripted() && self.saved.board.is_none())
        {
            let key = if piece == Piece::Bishop {
                "bishop_retry"
            } else {
                "cinema_knight_start_trigger"
            };
            if self.data.volumes[key]
                .trace(p.feet + PLAYER_CENTER, p.feet + PLAYER_CENTER, PLAYER_HALF)
                .start_solid
            {
                self.saved.retry = None;
                self.saved.pending = Some(if piece == Piece::Bishop {
                    scene::Kind::Bishop
                } else {
                    scene::Kind::Knight
                });
            }
        }
        self.move_world(dt, map, w, p, fixed)?;
        self.step_scenes(dt, map, w, p)?;
        if !self.scripted() {
            self.step_board(dt, p, w);
        }
        self.player_pose = Transform {
            translation: p.feet,
            rotation: Quat::from_rotation_z(p.script_facing),
        };
        self.rebuild(map)?;
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        Ok(())
    }
    fn controlled(&self) -> bool {
        self.saved.pull.is_some()
            || self
                .saved
                .board
                .as_ref()
                .is_some_and(|b| b.moving.is_some())
    }
    fn scripted(&self) -> bool {
        self.saved.scene.is_some() || self.saved.pending.is_some()
    }
    fn allow_cheshire(&self) -> bool {
        self.saved.board.is_none()
    }
    fn scene_id(&self) -> Option<&'static str> {
        self.saved
            .scene
            .as_ref()
            .map(|s| s.kind.id())
            .or(self.saved.pending.map(|s| s.id()))
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        self.scene_camera()
    }
    fn fade(&self) -> Option<(Color, f32)> {
        self.scene_fade()
    }
    fn skip(&mut self, map: &Bsp, w: &mut World, p: &mut Player, _: &mut Story) -> Result<bool> {
        self.skip_scene(map, w, p)
    }
    fn entry_story(&mut self, _: &mut Story) -> bool {
        true
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.objects
            .iter()
            .filter(|o| o.draw)
            .map(|o| (o.model, o.pose.translation, o.pose.rotation))
            .collect()
    }
    fn colliders(&self) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| o.solid)
            .map(|o| o.collider.clone())
            .collect()
    }
    fn liquids(&self) -> Vec<crate::collision::Liquid> {
        self.objects
            .iter()
            .filter(|o| o.liquid)
            .map(|o| crate::collision::Liquid {
                contents: 32,
                volume: o.volume.clone(),
            })
            .collect()
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
        l: &mut Vec<crate::audio::LoopCue>,
        c: &mut Vec<crate::audio::world::Clock>,
    ) {
        self.sounds(l, c);
    }
    fn objective(&self) -> Option<String> {
        Some(
            if !self.saved.bishop_done {
                "Complete the bishop lesson"
            } else if !self.saved.knight_gate {
                "Clear the galleries and ring the bell"
            } else if !self.saved.knight_done {
                "Complete the knight lesson"
            } else if !self.water_ready() {
                "Raise the water"
            } else {
                "Swim to the western passage"
            }
            .into(),
        )
    }
    fn recovery_entry(&self, normal: (Vec3, f32)) -> (Vec3, f32) {
        if let Some(piece) = self.saved.retry {
            let pose = self.data.point(if piece == Piece::Bishop {
                "alice_bishop_puzzle_dest1"
            } else {
                "alice_knight_puzzle_dest1"
            });
            (pose.translation, pose.rotation.to_euler(EulerRot::ZYX).0)
        } else {
            normal
        }
    }
    fn upgrade(&self) -> crate::level::Upgrade {
        crate::level::Upgrade {
            respawn: crate::level::Respawn::Always,
            rearm: [
                "cinema_bishop_start_thread",
                "cinema_knight_start_thread",
                "cinema_ring_bell_thread",
                "cinema_raise_water_thread",
                "op1_spikes_thread",
                "pawn_npc1_thread",
                "red_pawns1_thread",
                "red_pawns2_thread",
                "red_pawns3_thread",
                "red_pawns4_thread",
                "red_pawns5_thread",
                "red_knight1_thread",
                "red_knight2_thread",
                "red_knight3_thread",
                "red_bishop1_thread",
                "rook_guard1_attacked_thread",
                "white_pawn_bullied_thread",
                "OpenFloatSecret",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
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
                        && a.model == b.model
                        && a.name == b.name
                        && a.white == b.white
                        && a.floor == b.floor
                        && a.counted == b.counted
                        && a.piece.kind == b.piece.kind),
            "Chess roster mismatch"
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
    id: "wchess1",
    applies: |m, e| super::first_visit(m, e, "wchess1"),
    load: |a, m, _, _| Ok(Box::new(Realm::load(a, m)?)),
    art: Some(|a, _, r| {
        Ok(Box::new(art::Art::load(
            a,
            r.downcast_ref::<Realm>().unwrap(),
        )?))
    }),
    owns_submodel: |m, e| m == "wchess1" && motion::owns(e),
    owns_npc: |n, m| {
        m.starts_with("c_chess_")
            || m == "lever"
            || matches!(
                n,
                "bishop_grow"
                    | "knight_grow"
                    | "bishop_effect"
                    | "knight_effect"
                    | "bell_lever"
                    | "water_lever"
            )
    },
    target_base: Some(BASE),
    story_beats: &[],
    checks: &[
        Check {
            flag: "--wchess1-check",
            help: "Verify Pale Realm puzzles, factions, gates and persistence.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--wchess1-route-check",
            help: "Walk the Pale Realm route into Castling.",
            run: Run::Headless(check::route),
        },
        Check {
            flag: "--wchess1-route-skip-check",
            help: "Walk Pale Realm with scene skips.",
            run: Run::Headless(check::route_skip),
        },
        Check {
            flag: "--wchess1-floor-survey",
            help: "Record collision support for Pale Realm route calibration.",
            run: Run::Headless(check::survey),
        },
        Check {
            flag: "--wchess1-render-check",
            help: "Render Pale Realm staging and disguises.",
            run: Run::Windowed(check::render),
        },
    ],
    save_cases: &[
        super::SaveCase {
            name: "wchess1-intro-mid",
            visit: "wchess1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "wchess1-bishop-square",
            visit: "wchess1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "wchess1-bishop-move",
            visit: "wchess1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "wchess1-knight-gate",
            visit: "wchess1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "wchess1-bell-delay",
            visit: "wchess1$first",
            stage: None,
            behavior: None,
        },
        super::SaveCase {
            name: "wchess1-water-rising",
            visit: "wchess1$first",
            stage: None,
            behavior: None,
        },
    ],
    visibility: &[],
};
fn owner(i: &mut Interactions) -> Result<&mut Realm> {
    i.levels
        .iter_mut()
        .find_map(|s| s.ctl.downcast_mut())
        .context("Pale Realm owner missing")
}
