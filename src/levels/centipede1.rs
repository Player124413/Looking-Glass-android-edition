//! Fungiferous Flora: hidden speaker, ambush, live Ant waves and a story-owned departure.
mod art;
mod check;
pub(crate) use check::drive as drive_route;
mod data;
mod saves;
use super::state;
use crate::{
    ant::{Ant, Timing},
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World},
    combat::{Feedback, Hit, Target},
    event::{Condition, Facts},
    interaction::Events,
    inventory::Stats,
    level::{Combat, LevelArt, LevelController, Respawn, TriggerInfo, Upgrade},
    movement::Player,
    skeletal::Transform,
    story::{registry::BeatSpec, Story},
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;
const BASE: usize = 7_600_000;
const TALK: &str = "Centipede1_Talk1";
const AMBUSH: &str = "Centipede1_Ambush_Cinema1";
const BEAT: &str = "Centipede1_Ambush_Dialog";
const EXIT: crate::level::spec::ExitSpec = crate::level::spec::ExitSpec {
    map: "centipede2",
    entrance: "centipede2_start1",
};
const COUNTDOWN: f64 = 10.;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    Explore,
    Rush,
    Reveal,
    Outro,
    Countdown,
    Done,
}
#[derive(Clone, Serialize, Deserialize)]
struct Soldier {
    ant: Ant,
    group: u8,
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    phase: Phase,
    time: f64,
    clock: f64,
    talk: bool,
    alice: Transform,
    speech: f32,
    reveal_end: f32,
    ants: Vec<Soldier>,
    wave: u8,
    exit: crate::level::exit::ExitState,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        ensure!(
            (0. ..=86400.).contains(&self.time) && (0. ..=86400.).contains(&self.clock),
            "Invalid Flora clock"
        );
        state::clock("Flora speech", self.speech, 120.)?;
        state::clock("Flora reveal", self.reveal_end, 120.)?;
        state::clock("Flora retry", self.exit.retry_time, 1.)?;
        ensure!(
            self.alice.translation.is_finite()
                && self.alice.translation.abs().max_element() < 100000.
                && self.alice.rotation.is_finite()
                && (self.alice.rotation.length_squared() - 1.).abs() < 0.01,
            "Invalid ambush player pose"
        );
        ensure!(
            self.wave <= 4 && self.ants.len() <= 24,
            "Invalid ambush wave budget"
        );
        ensure!(
            self.exit.committed == (self.phase == Phase::Done),
            "Premature Flora departure"
        );
        ensure!(
            matches!(self.phase, Phase::Countdown | Phase::Done) || self.wave == 0,
            "Waves before ambush"
        );
        let mut ids = std::collections::BTreeSet::new();
        for s in &self.ants {
            s.ant.validate()?;
            ensure!(
                s.group <= 2 && s.ant.id < 2000 && ids.insert(s.ant.id),
                "Invalid Flora Ant identity"
            );
        }
        Ok(())
    }
}
struct Flora {
    data: data::Data,
    saved: Saved,
    alive: bool,
}
impl Flora {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let data = data::Data::load(a, map)?;
        let ants = data
            .placed
            .iter()
            .cloned()
            .map(|ant| Soldier { ant, group: 0 })
            .collect();
        Ok(Self {
            saved: Saved {
                version: 1,
                phase: Phase::Explore,
                time: 0.,
                clock: 0.,
                talk: false,
                alice: data.points["centipede1_start1"],
                speech: 0.,
                reveal_end: 0.,
                ants,
                wave: 0,
                exit: Default::default(),
            },
            data,
            alive: true,
        })
    }
    fn phase(&mut self, p: Phase) {
        self.saved.phase = p;
        self.saved.time = 0.;
    }
    fn skippable(&self) -> bool {
        self.saved.phase == Phase::Rush && self.saved.time >= 2.
            || matches!(self.saved.phase, Phase::Reveal | Phase::Outro)
    }
    fn reveal(&self) -> bool {
        matches!(self.saved.phase, Phase::Reveal | Phase::Outro)
    }
    fn commit(&mut self, w: &World, p: &mut Player) {
        if !self.alive || !matches!(self.saved.phase, Phase::Rush | Phase::Reveal | Phase::Outro) {
            return;
        }
        for s in &mut self.saved.ants {
            if let Some(goal) = self.data.runners.get(&s.ant.id) {
                if s.ant.health > 0. {
                    s.ant.feet = goal.translation;
                    s.ant.yaw = goal.rotation.to_euler(EulerRot::ZYX).0;
                    s.ant.time = 0.;
                }
                s.ant.script_wait = false;
            }
        }
        p.velocity = Vec3::ZERO;
        self.phase(Phase::Countdown);
        self.saved.speech = 0.;
        self.saved.reveal_end = 0.;
        self.spawn_wave(w, p.feet + Vec3::Z * 32.);
    }
    fn spawn_wave(&mut self, w: &World, eye: Vec3) {
        if self.saved.wave >= 4 {
            return;
        }
        // Both original loops consult the live brave group, including the killer-loop quirk.
        let allowed = self
            .saved
            .ants
            .iter()
            .filter(|s| s.group == 1 && s.ant.health > 0.)
            .count()
            < 3;
        if allowed {
            for (group, chain) in [(1, &self.data.braves), (2, &self.data.killers)] {
                if let Some(node) = chain
                    .iter()
                    .find(|node| w.sweep(node.translation, eye, Vec3::ZERO).fraction < 1.)
                {
                    let id = 1000 + self.saved.wave as usize * 2 + (group - 1) as usize;
                    let feet =
                        w.actor_footing(node.translation, Vec3::Z * 36., Vec3::splat(36.), 256.);
                    if let Some(feet) = feet {
                        let direction = eye - feet;
                        let mut ant = Ant::new(id, false, feet, direction.y.atan2(direction.x));
                        ant.phase = crate::ant::Phase::Chase;
                        self.saved.ants.push(Soldier { ant, group });
                    }
                }
            }
        }
        self.saved.wave += 1;
    }
    fn boss_clip(&self) -> (&'static str, f32, bool) {
        let t = self.shot_time();
        let yawn = self.data.duration("c_centipede", "idle_yawn");
        if t < yawn {
            ("idle_yawn", t, false)
        } else {
            ("idle_snarl", t - yawn, false)
        }
    }
    fn shot_time(&self) -> f32 {
        self.saved.time as f32
            + if self.saved.phase == Phase::Outro {
                self.saved.reveal_end
            } else {
                0.
            }
    }
}
impl LevelController for Flora {
    fn id(&self) -> &'static str {
        "centipede1"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag(
            "centipede1.explore",
            self.saved.phase == Phase::Explore && self.alive,
        );
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        if matches!(t.thread, TALK | AMBUSH) {
            Some(Condition::flag("centipede1.explore"))
        } else {
            None
        }
    }
    fn event(&mut self, n: &str) -> Option<Events> {
        match n {
            TALK => {
                if self.saved.phase == Phase::Explore && self.alive {
                    self.saved.talk = true;
                }
            }
            AMBUSH => {
                if self.saved.phase == Phase::Explore && self.alive {
                    self.phase(Phase::Rush);
                }
            }
            "C1_End" | "Skipthread_C1" => {} // A stray callback may never commit this encounter.
            _ => return None,
        }
        Some(Events::default())
    }
    fn dialogue_complete(&mut self, n: &str) -> Events {
        if n == BEAT && self.saved.phase == Phase::Reveal {
            self.saved.reveal_end = self.saved.time as f32;
            self.phase(Phase::Outro);
        }
        Events::default()
    }
    fn prepare_player(&mut self, s: &mut Stats, p: &mut Player) {
        self.alive = s.alive();
        s.minimum_sanity = if matches!(self.saved.phase, Phase::Countdown | Phase::Done) {
            10.
        } else {
            0.
        };
        if self.saved.phase == Phase::Explore {
            self.saved.alice = Transform {
                translation: p.feet,
                rotation: Quat::from_rotation_z(p.script_facing),
            };
        }
    }
    fn advance(
        &mut self,
        dt: f32,
        _: &Bsp,
        w: &mut World,
        p: &mut Player,
        _: &[Collider],
    ) -> Result<()> {
        if dt <= 0. || !self.alive {
            return Ok(());
        }
        let dt = f64::from(dt.min(0.1));
        self.saved.time = (self.saved.time + dt).min(86400.);
        self.saved.clock = (self.saved.clock + dt).min(86400.);
        self.saved.exit.advance(dt as f32);
        match self.saved.phase {
            Phase::Rush => {
                for s in &mut self.saved.ants {
                    if let Some(goal) = self.data.runners.get(&s.ant.id) {
                        if s.ant.health <= 0. {
                            continue;
                        }
                        let delta = goal.translation - s.ant.feet;
                        let step = delta.clamp_length_max(
                            self.data.speed("c_armyant", "walk_fast") * dt as f32,
                        );
                        s.ant.feet =
                            crate::combat::walk_body(w, s.ant.feet, step, Vec3::splat(36.));
                        s.ant.yaw = delta.y.atan2(delta.x);
                        s.ant.time += dt as f32;
                    }
                }
                if self.saved.time >= 3. {
                    self.phase(Phase::Reveal);
                }
            }
            Phase::Outro => {
                if self.saved.time >= 0.5 {
                    self.commit(w, p);
                }
            }
            Phase::Countdown => {
                while self.saved.wave < 4
                    && self.saved.time + 1e-6 >= f64::from(self.saved.wave) * 3.1
                {
                    self.spawn_wave(w, p.feet + Vec3::Z * 32.);
                }
                if self.saved.time + 1e-6 >= COUNTDOWN {
                    self.phase(Phase::Done);
                    self.saved.exit.committed = true;
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn update(&mut self, _: &mut World, _: &Player, _: Vec3, _: bool) -> Events {
        Events {
            transition: self.saved.exit.request(EXIT),
            ..Default::default()
        }
    }
    fn transition_failed(&mut self, e: &(String, Option<String>)) {
        if EXIT.matches(e) {
            self.saved.exit.failed();
        }
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        vec![(
            self.data.lid_model,
            self.data.lid_pose.translation,
            self.data.lid_pose.rotation,
        )]
    }
    fn colliders(&self) -> Vec<Collider> {
        vec![self.data.lid.clone()]
    }
    fn settled_supports(&self) -> Vec<Collider> {
        self.colliders()
    }
    fn scene_fog(&self) -> Option<Vec4> {
        Some(vec4(0., 0.082353, 0.168627, 2500.))
    }
    fn scripted(&self) -> bool {
        matches!(
            self.saved.phase,
            Phase::Rush | Phase::Reveal | Phase::Outro | Phase::Done
        )
    }
    fn controlled(&self) -> bool {
        self.scripted()
    }
    fn allow_cheshire(&self) -> bool {
        self.saved.phase == Phase::Explore
    }
    fn scene_id(&self) -> Option<&'static str> {
        self.skippable().then_some(AMBUSH)
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        self.reveal().then(|| self.data.camera(self.shot_time()))
    }
    fn fade(&self) -> Option<(Color, f32)> {
        let t = self.saved.time as f32;
        let alpha = match self.saved.phase {
            Phase::Rush => ((t - 2.5) * 2.).clamp(0., 1.),
            Phase::Reveal | Phase::Countdown => (1. - t * 2.).clamp(0., 1.),
            Phase::Outro => (t * 2.).clamp(0., 1.),
            _ => 0.,
        };
        (alpha > 0.).then_some((WHITE, alpha))
    }
    fn prepare_story(&self, s: &mut Story) -> bool {
        if self.saved.phase == Phase::Explore && self.saved.talk && !s.has_seen(TALK) {
            s.trigger(TALK);
        }
        if self.scripted() {
            s.finish_sequence(TALK);
            s.finish_sequence("cheshire_hint");
        }
        if self.saved.phase == Phase::Reveal {
            if !s.has_seen(BEAT) {
                s.trigger(BEAT);
            } else if !s.sequence_pending(BEAT) {
                s.resume_scene(BEAT, 0, self.saved.speech);
            }
        }
        true
    }
    fn sync_story(&mut self, s: &Story) {
        if let Some((_, t)) = s.progress(BEAT) {
            self.saved.speech = t;
        }
    }
    fn skip(&mut self, _: &Bsp, w: &mut World, p: &mut Player, s: &mut Story) -> Result<bool> {
        if !self.skippable() || !self.alive {
            return Ok(false);
        }
        s.trigger(BEAT);
        s.finish_sequence(BEAT);
        self.commit(w, p);
        Ok(true)
    }
    fn receivers(&self, _: &crate::entity::Registry) -> Vec<crate::entity::Id> {
        self.data
            .placed
            .iter()
            .map(|a| crate::entity::Id(a.id))
            .collect()
    }
    fn rules(&self, _: &crate::level::RuleContext<'_>) -> Vec<crate::event::Rule> {
        self.data
            .placed
            .iter()
            .map(|a| {
                let id = crate::entity::Id(a.id);
                crate::event::Rule {
                    key: format!("centipede1/ant/{}", a.id),
                    event: crate::event::Event::Entity(id, crate::event::Input::Activate),
                    condition: Condition::Always,
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
            if let Some(s) = self
                .saved
                .ants
                .iter_mut()
                .find(|s| s.group == 0 && s.ant.id == id.0)
            {
                s.ant.enabled = true;
                return Some(Events::default());
            }
        }
        None
    }
    fn targets(&self) -> Vec<Target> {
        if self.scripted() {
            vec![]
        } else {
            self.saved
                .ants
                .iter()
                .filter(|s| s.ant.enabled && s.ant.health > 0.)
                .map(|s| s.ant.target(BASE + s.ant.id))
                .collect()
        }
    }
    fn hit(&mut self, h: Hit) -> Option<&'static str> {
        if self.scripted() {
            return None;
        }
        self.saved
            .ants
            .iter_mut()
            .find(|s| s.ant.enabled && BASE + s.ant.id == h.id)?
            .ant
            .hit(h)
    }
    fn combat(&mut self, c: &mut Combat<'_>) -> Feedback {
        let mut out = Feedback::default();
        if c.dt > 0. && !self.scripted() && c.stats.alive() {
            for s in &mut self.saved.ants {
                s.ant.advance(c, &self.data, &mut out);
            }
        }
        out
    }
    fn loot_sources(&self) -> Vec<crate::loot::Source> {
        self.saved
            .ants
            .iter()
            .filter(|s| s.ant.enabled)
            .map(|s| crate::loot::Source {
                id: BASE + s.ant.id,
                feet: s.ant.feet,
                dead: s.ant.health == 0.,
                grade: s.ant.grade(),
            })
            .collect()
    }
    fn objective(&self) -> Option<String> {
        Some(
            match self.saved.phase {
                Phase::Explore => "Follow the forest path toward the Centipede.",
                Phase::Countdown | Phase::Done => {
                    "Survive the ambush until the path to the Sanctum opens."
                }
                _ => "The Centipede has set a trap.",
            }
            .into(),
        )
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn sound_state(
        &self,
        _: &mut Vec<crate::audio::LoopCue>,
        out: &mut Vec<crate::audio::world::Clock>,
    ) {
        if self.reveal() {
            let t = self.shot_time();
            let origin = self.data.points["cent_posx1"].translation;
            out.push(crate::audio::world::Clock {
                key: "centipede1.yawn",
                time: t,
                period: None,
                origin,
                cues: &[(0., "sound/character/centipede/idle_yawn.wav")],
            });
            out.push(crate::audio::world::Clock {
                key: "centipede1.snarl",
                time: t - self.data.duration("c_centipede", "idle_yawn"),
                period: None,
                origin,
                cues: &[(0., "sound/character/centipede/idle_snarl.wav")],
            });
        }
    }
    fn restore(&mut self, v: &serde_json::Value, _: &Bsp) -> Result<()> {
        let s: Saved = state::load(v, state::Visit { returning: false })?;
        ensure!(
            s.ants
                .iter()
                .filter(|s| s.group == 0)
                .map(|s| s.ant.id)
                .eq(self.data.placed.iter().map(|a| a.id)),
            "Flora placements changed"
        );
        for ant in &s.ants {
            ensure!(
                !ant.ant.corporal && ant.ant.scale == 1.,
                "Flora Ant model changed"
            );
            ensure!(
                ant.group == 0 || ant.ant.id >= 1000 && ant.ant.id < 1008,
                "Invalid wave identity"
            );
        }
        self.saved = s;
        Ok(())
    }
    fn upgrade(&self) -> Upgrade {
        Upgrade {
            respawn: Respawn::Always,
            rearm: vec![TALK.into(), AMBUSH.into()],
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
    id: "centipede1",
    applies: |m, e| super::first_visit(m, e, "centipede1"),
    load: |a, m, _, _| Ok(Box::new(Flora::load(a, m)?)),
    art: Some(|a, _, _| Ok(Box::new(art::Art::load(a)?))),
    owns_submodel: |m, e| {
        m == "centipede1" && e.get("targetname").is_some_and(|n| n == "shroom_lid")
    },
    owns_npc: |_, m| matches!(m, "c_centipede" | "c_armyant"),
    target_base: Some(BASE),
    story_beats: &[
        BeatSpec::linear("centipede1", TALK, "centipede1_cinematics", TALK, 1),
        BeatSpec::linear("centipede1", BEAT, "centipede1_cinematics", AMBUSH, 1),
    ],
    checks: &[
        super::Check {
            flag: "--centipede1-check",
            help: "Verify Fungiferous Flora ambush, waves, skip and exit.",
            run: super::Run::Headless(check::check),
        },
        super::Check {
            flag: "--centipede1-route-check",
            help: "Walk Fungiferous Flora into Centipede's Sanctum.",
            run: super::Run::Headless(check::route),
        },
        super::Check {
            flag: "--centipede1-skip-route-check",
            help: "Walk Fungiferous Flora with the ambush skipped.",
            run: super::Run::Headless(check::skip_route),
        },
        super::Check {
            flag: "--centipede1-render-check",
            help: "Capture Flora arrival, rush, reveal and Ant waves.",
            run: super::Run::Windowed(check::render),
        },
        super::Check {
            flag: "--centipede1-save-write",
            help: "Write Flora native persistence fixtures.",
            run: super::Run::Windowed(saves::write),
        },
        super::Check {
            flag: "--centipede1-save-read",
            help: "Restore Flora native fixtures in a fresh process.",
            run: super::Run::Windowed(saves::read),
        },
    ],
    save_cases: &[],
    visibility: &[],
};
