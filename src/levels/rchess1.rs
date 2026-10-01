//! Checkmate in Red: reviewed chess ambushes, royal scenes and Red King arena.
mod art;
mod battle;
mod check;
mod data;
mod route_input;
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
    level::{Combat, LevelArt, LevelController, Respawn, Upgrade},
    movement::Player,
    skeletal::Transform,
    story::Story,
};
use anyhow::{ensure, Context, Result};
use data::Data;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::{any::Any, collections::BTreeSet};
const BASE: usize = 8_100_000;
const BEHEAD: &str = "cinema_beheading_thread";
const INTRO: &str = "cinema_king_battle_thread";
const KILLED: &str = "cinema_king_killed_thread";
const EXIT: crate::level::spec::ExitSpec = crate::level::spec::ExitSpec {
    map: "funhouse",
    entrance: "funhouse_start1",
};
fn owns_brush(e: &super::Entity) -> bool {
    e.get("classname").is_some_and(|c| c == "script_object")
        || e.get("targetname").is_some_and(|n| n == "spec_exit_door")
}
fn scene_actor(n: &str) -> bool {
    n.starts_with("spec_")
        || n.starts_with("w_queen_")
        || n.starts_with("king_bishop")
        || matches!(
            n,
            "r_king_beheader"
                | "revived_queen"
                | "kings_pawn"
                | "portal_alice"
                | "portal_hatter"
                | "asylum_alice"
                | "asylum_hatter"
                | "peanut_gallary"
        )
}
pub static REGISTRATION: super::Registration = super::Registration {
    id: "rchess1",
    applies: |m, e| super::first_visit(m, e, "rchess1"),
    load: |a, m, _, _| Ok(Box::new(Encounter::load(a, m)?)),
    art: Some(|a, _, o| {
        Ok(Box::new(art::Art::load(
            a,
            o.downcast_ref::<Encounter>()
                .context("Missing Red King owner")?,
        )?))
    }),
    owns_submodel: |_, e| owns_brush(e),
    owns_npc: |n, m| n == "r_king" || scene_actor(n) || crate::chess::Kind::from_model(m).is_some(),
    target_base: Some(BASE),
    story_beats: &[],
    checks: &[
        super::Check {
            flag: "--rchess1-check",
            help: "Verify Red King attacks, phases, gates and restoration.",
            run: super::Run::Headless(check::check),
        },
        super::Check {
            flag: "--rchess1-route-check",
            help: "Play Checkmate in Red through the King and Funhouse departure.",
            run: super::Run::Headless(check::route),
        },
        super::Check {
            flag: "--rchess1-render-check",
            help: "Capture Red King combat, beheading and the Hatter scene.",
            run: super::Run::Windowed(check::render),
        },
    ],
    save_cases: &[],
    visibility: &[],
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    Approach,
    Beheading,
    Northern,
    Intro,
    Fight,
    Killed,
    Done,
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    phase: Phase,
    time: f32,
    clock: f32,
    beheaded: bool,
    boss: battle::Boss,
    pieces: Vec<(usize, crate::chess::Piece)>,
    ambushed: BTreeSet<usize>,
    essence: usize,
    essence_wait: f32,
    remainder: f64,
    checkpoint: bool,
    scene_from: Transform,
    exit: crate::level::exit::ExitState,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        for (n, t, m) in [
            ("scene", self.time, 3600.),
            ("clock", self.clock, 86400.),
            ("essence", self.essence_wait, 1.),
            ("exit", self.exit.retry_time, 1.),
        ] {
            state::clock(n, t, m)?;
        }
        self.boss.validate()?;
        ensure!(
            self.essence < 3
                && self.remainder.is_finite()
                && (0. ..0.009).contains(&self.remainder)
                && self.pieces.len() <= 40
                && self.ambushed.len() <= 10,
            "Invalid King counters"
        );
        ensure!(
            self.scene_from.translation.is_finite()
                && self.scene_from.rotation.is_finite()
                && (self.scene_from.rotation.length_squared() - 1.).abs() < 0.01,
            "Invalid scene origin"
        );
        let mut ids = BTreeSet::new();
        for (id, p) in &self.pieces {
            ensure!(ids.insert(*id), "Duplicate chess piece");
            p.validate()?;
        }
        ensure!(
            !matches!(self.phase, Phase::Killed | Phase::Done) || self.boss.health == 0.,
            "Premature King victory"
        );
        ensure!(
            !matches!(
                self.phase,
                Phase::Approach | Phase::Beheading | Phase::Northern | Phase::Intro
            ) || self.boss.health == 1300.,
            "Damaged hidden King"
        );
        ensure!(
            self.exit.committed == (self.phase == Phase::Done),
            "Invalid Funhouse exit"
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
}
impl Encounter {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let data = Data::load(a, map)?;
        let saved = Saved {
            version: 1,
            phase: Phase::Approach,
            time: 0.,
            clock: 0.,
            beheaded: false,
            boss: battle::Boss::new(data.points["r_king"]),
            pieces: data.pieces.clone(),
            ambushed: BTreeSet::new(),
            essence: 0,
            essence_wait: 0.,
            remainder: 0.,
            checkpoint: false,
            scene_from: data.points["beheading_node1"],
            exit: Default::default(),
        };
        let mut o = Self {
            data,
            saved,
            poses: vec![],
            shapes: vec![],
            alive: true,
        };
        o.rebuild(map)?;
        Ok(o)
    }
    fn phase(&mut self, p: Phase) {
        self.saved.phase = p;
        self.saved.time = 0.;
    }
    fn settle(&mut self, map: &Bsp, w: &mut World, p: &mut Player, marker: &str) -> Result<()> {
        self.rebuild(map)?;
        w.set_dynamic(self.shapes.clone());
        let at = self.data.points[marker];
        *p = Player::spawn(w, at.translation + Vec3::Z * 48.)
            .with_context(|| format!("Blocked scene handoff {marker}"))?;
        p.script_facing = at.rotation.to_euler(EulerRot::ZYX).0;
        Ok(())
    }
    fn finish_beheading(&mut self, map: &Bsp, w: &mut World, p: &mut Player) -> Result<()> {
        self.saved.beheaded = true;
        self.phase(Phase::Northern);
        self.settle(map, w, p, "beheading_node1")
    }
    fn start_fight(&mut self, map: &Bsp, w: &mut World, p: &mut Player) -> Result<()> {
        self.phase(Phase::Fight);
        self.saved.checkpoint = true;
        self.settle(map, w, p, "alice_king_dest1")
    }
    fn finish_exit(&mut self) {
        self.saved.phase = Phase::Done;
        self.saved.time = self.exit_duration();
        self.saved.exit.committed = true;
    }
    fn essence_position(&self) -> Vec3 {
        self.data.points[&format!("get_me{}", self.saved.essence + 1)].translation + Vec3::Z * 16.
    }
}
impl LevelController for Encounter {
    fn id(&self) -> &'static str {
        "rchess1"
    }
    fn event(&mut self, n: &str) -> Option<Events> {
        if n == BEHEAD {
            if self.saved.phase == Phase::Approach {
                self.phase(Phase::Beheading);
            }
        } else if n == INTRO {
            if matches!(self.saved.phase, Phase::Approach | Phase::Northern) {
                self.phase(Phase::Intro);
            }
        } else if n == KILLED { /* Only a completed boss death may start this scene. */
        } else if n.starts_with("enemy_group") || n.starts_with("Centipede2_ME") {
        } else {
            return None;
        }
        Some(Events::default())
    }
    fn door_locked(&self, id: crate::entity::Id) -> Option<bool> {
        if self.data.usable_doors.contains(&id.0) {
            Some(false)
        } else if [145, 146].contains(&id.0) {
            Some(true)
        } else {
            None
        }
    }
    fn prepare_player(&mut self, s: &mut Stats, p: &mut Player) {
        self.alive = s.alive();
        if !self.alive {
            return;
        }
        if !self.scripted() {
            self.saved.scene_from = Transform {
                translation: p.feet,
                rotation: Quat::from_rotation_z(p.script_facing),
            };
        }
        if self.saved.phase == Phase::Fight
            && self.saved.essence_wait == 0.
            && (s.sanity() < 100. || s.will() < 100.)
            && (p.feet + Vec3::Z * 28. - self.essence_position())
                .abs()
                .cmple(vec3(32., 32., 56.))
                .all()
        {
            s.essence(25.);
            self.saved.essence = (self.saved.essence + 1) % 3;
            self.saved.essence_wait = 1.;
        }
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
        self.saved.time = (self.saved.time + dt).min(3600.);
        self.saved.clock = (self.saved.clock + dt).min(86400.);
        self.saved.exit.advance(dt);
        if self.saved.phase == Phase::Fight {
            self.saved.essence_wait = (self.saved.essence_wait - dt).max(0.);
        }
        if self.saved.phase == Phase::Beheading && self.saved.time >= 23.7 {
            self.finish_beheading(map, w, p)?;
        }
        if self.saved.phase == Phase::Intro && self.saved.time >= 11.5 {
            self.start_fight(map, w, p)?;
        }
        if self.saved.phase == Phase::Killed && self.saved.time >= self.exit_duration() {
            self.finish_exit();
        }
        self.rebuild(map)?;
        w.set_dynamic(fixed.iter().cloned().chain(self.shapes.clone()).collect());
        Ok(())
    }
    fn combat(&mut self, c: &mut Combat<'_>) -> Feedback {
        let mut out = Feedback::default();
        if c.dt <= 0. || !c.stats.alive() || self.scripted() {
            return out;
        }
        for (i, a) in self.data.ambushes.iter().enumerate() {
            if !self.saved.ambushed.contains(&i) && a.touches(c.player.eye()) {
                self.saved.ambushed.insert(i);
                for (id, p) in &mut self.saved.pieces {
                    if a.suppress.contains(id) && !p.active && p.spawn_delay.is_none() {
                        p.suppressed = true;
                    }
                    if a.entities.contains(id)
                        && !p.suppressed
                        && !p.active
                        && p.spawn_delay.is_none()
                    {
                        p.spawn_delay = Some(a.delay + 0.1);
                    }
                }
            }
        }
        for (id, p) in &mut self.saved.pieces {
            p.notarget = c.notarget;
            p.opponents.summon = c.summon;
            p.threatened((c.threatens)(&p.target(BASE + 100 + *id)));
            p.update(c.dt, c.world, c.player.eye(), &self.data, &mut out);
        }
        if self.saved.phase == Phase::Fight {
            self.saved.remainder += f64::from(c.dt.min(0.1));
            while self.saved.remainder + 1e-9 >= 1. / 120. {
                self.saved.remainder = (self.saved.remainder - 1. / 120.).max(0.);
                self.saved.boss.step(
                    c.world,
                    c.player.eye(),
                    c.notarget,
                    (c.threatens)(&self.saved.boss.target()),
                    &self.data,
                    &mut out,
                );
            }
            if self.saved.boss.done(&self.data) && out.damage < c.stats.sanity() {
                self.phase(Phase::Killed);
            }
        }
        out
    }
    fn targets(&self) -> Vec<Target> {
        if self.scripted() {
            return vec![];
        }
        let mut t = vec![];
        if self.saved.phase == Phase::Fight && self.saved.boss.health > 0. {
            t.push(self.saved.boss.target());
        }
        t.extend(
            self.saved
                .pieces
                .iter()
                .filter(|(_, p)| p.active && !p.suppressed && p.health > 0.)
                .map(|(i, p)| p.target(BASE + 100 + i)),
        );
        t
    }
    fn hit(&mut self, h: Hit) -> Option<&'static str> {
        if h.id == BASE {
            if self.saved.phase == Phase::Fight {
                self.saved.boss.hit(h);
            }
            None
        } else {
            let id = h.id.checked_sub(BASE + 100)?;
            self.saved
                .pieces
                .iter_mut()
                .find(|(i, _)| *i == id)?
                .1
                .hit(h)
        }
    }
    fn loot_sources(&self) -> Vec<crate::loot::Source> {
        self.saved
            .pieces
            .iter()
            .map(|(i, p)| crate::loot::Source {
                id: BASE + 100 + i,
                feet: p.feet,
                grade: p.kind.grade(),
                dead: p.health <= 0.,
            })
            .collect()
    }
    fn update(&mut self, _: &mut World, _: &Player, _: Vec3, _: bool) -> Events {
        Events {
            transition: if self.alive {
                self.saved.exit.request(EXIT)
            } else {
                None
            },
            ..Default::default()
        }
    }
    fn transition_failed(&mut self, e: &(String, Option<String>)) {
        if EXIT.matches(e) {
            self.saved.exit.failed();
        }
    }
    fn scripted(&self) -> bool {
        matches!(
            self.saved.phase,
            Phase::Beheading | Phase::Intro | Phase::Killed | Phase::Done
        )
    }
    fn controlled(&self) -> bool {
        self.scripted()
    }
    fn hides_player(&self) -> bool {
        self.scripted()
    }
    fn scene_id(&self) -> Option<&'static str> {
        match self.saved.phase {
            Phase::Beheading => Some(BEHEAD),
            Phase::Intro => Some(INTRO),
            Phase::Killed => Some(KILLED),
            _ => None,
        }
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        self.scene_camera()
    }
    fn fade(&self) -> Option<(Color, f32)> {
        self.scene_fade()
    }
    fn skip(&mut self, m: &Bsp, w: &mut World, p: &mut Player, _: &mut Story) -> Result<bool> {
        match self.saved.phase {
            Phase::Beheading => self.finish_beheading(m, w, p)?,
            Phase::Intro => self.start_fight(m, w, p)?,
            Phase::Killed => self.finish_exit(),
            _ => return Ok(false),
        }
        Ok(true)
    }
    fn dismiss_summons(&self) -> bool {
        matches!(
            self.saved.phase,
            Phase::Intro | Phase::Fight | Phase::Killed | Phase::Done
        )
    }
    fn allow_cheshire(&self) -> bool {
        !self.dismiss_summons()
    }
    fn checkpoint_requested(&self) -> bool {
        self.saved.checkpoint
    }
    fn checkpoint_written(&mut self) {
        self.saved.checkpoint = false;
    }
    fn music_mood(&self) -> Option<&'static str> {
        Some(if self.saved.phase == Phase::Fight {
            "action"
        } else {
            "normal"
        })
    }
    fn recovery_entry(&self, normal: (Vec3, f32)) -> (Vec3, f32) {
        if matches!(self.saved.phase, Phase::Fight | Phase::Killed | Phase::Done) {
            (
                self.data.points["alice_king_dest1"].translation + Vec3::Z * 48.,
                0.,
            )
        } else {
            normal
        }
    }
    fn particles(&self, steam: &mut crate::particles::Steam) {
        steam.gate(
            &self
                .data
                .fire_ids
                .iter()
                .map(|id| (*id, false))
                .collect::<Vec<_>>(),
        );
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.poses.clone()
    }
    fn colliders(&self) -> Vec<Collider> {
        self.shapes.clone()
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn sound_state(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        _: &mut Vec<crate::audio::world::Clock>,
    ) {
        if self.saved.phase == Phase::Fight {
            for shot in &self.saved.boss.shots {
                if shot.ended.is_none() && shot.kind != battle::Kind::Diamond {
                    loops.push(crate::audio::LoopCue {
                        id: BASE + 1000 + shot.id as usize,
                        path: "sound/character/chess_piece/king/king_ball_loop.wav",
                        origin: shot.at,
                        clock: Some(shot.age),
                    });
                }
            }
        }
        if self.saved.phase == Phase::Killed
            && (1.1..1.1 + self.data.queener_time).contains(&self.saved.time)
        {
            loops.push(crate::audio::LoopCue {
                id: BASE + 900,
                path: "sound/ambience/special/redchess_platform.wav",
                origin: self.data.points["queener"].translation,
                clock: Some(self.saved.time - 1.1),
            });
        }
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        let next: Saved = state::load(v, state::Visit { returning: false })?;
        ensure!(
            next.pieces.iter().map(|(i, p)| (*i, p.kind)).eq(self
                .data
                .pieces
                .iter()
                .map(|(i, p)| (*i, p.kind)))
                && next.ambushed.iter().all(|i| *i < self.data.ambushes.len()),
            "Changed chess roster"
        );
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
            rearm: vec![BEHEAD.into(), INTRO.into()],
            ..Default::default()
        }
    }
    fn objective(&self) -> Option<String> {
        Some(
            match self.saved.phase {
                Phase::Approach => "Find the third Demon Die and continue through the red castle.",
                Phase::Beheading => "Watch the royal execution.",
                Phase::Northern => "Follow the upper castle route to the King's drawbridge.",
                Phase::Intro => "Face the Red King.",
                Phase::Fight => {
                    "Defeat the Red King. Dodge his magic and collect the returning essence."
                }
                _ => "Follow the Queen towards the portal.",
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

pub(crate) use check::drive as drive_route;
