//! Bill and the house ending, inside the existing Hollow Hideaway owner.
mod check;
mod data;
mod fish;
mod guards;
mod route;
pub(crate) use route::drive as drive_route;
mod save_check;
mod scene;
mod transport;
mod transport_check;
use super::{state, Check, Registration, Run, SaveCase};
use crate::combat::{Feedback, Hit, Target};
use crate::level::scene::{SceneRunner, SceneState};
use crate::level::spec::{EndSpec, ExitSpec, SceneSpec, ShotSpec};
use crate::loot::Source;
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World},
    event::{Condition, Facts},
    interaction::{Events, Interactions},
    level::{LevelArt, LevelController, TriggerClass, TriggerInfo},
    movement::Player,
    npc::Puppet,
    skeletal::Transform,
    story::{registry::AnimationRef, BeatSpec, Story},
};
use anyhow::{ensure, Context, Result};
use check::{check, render, saved_behavior};
pub use check::{stage, stage_scene};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;

pub const DIALOGUE: &str = "tears2_end_cinematic";
const BEATS: &[BeatSpec] = &[
    BeatSpec::linear(
        "potears2",
        DIALOGUE,
        "../potears2",
        "tears2_end_cinematic",
        1,
    ),
    BeatSpec {
        map: "potears2",
        event: DIALOGUE,
        script: "../potears2",
        thread: "tears2_dialog",
        source_lines: 12,
        calls: crate::story::registry::Calls::Gated {
            key: "potears2.bill_dialogue_ready",
            indices: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        },
    },
];
const BILL_CLIPS: &[&str] = &[
    "walk",
    "idle_01",
    "idle_liftbelt",
    "idle_eyes",
    "idle_sneeze",
    "talk_shrug",
    "talk_fist",
    "talk_no",
    "run",
    "paniked_run",
];
const ALICE_CLIPS: &[&str] = &[
    "walk",
    "held",
    "idle_base_02",
    "idle_base_02_2_stand",
    "idle_stand",
    "idle_stand_nodyes",
    "idle_stand_shakeno",
    "idle_stand_tiptoes",
];
fn animation_refs() -> Vec<AnimationRef> {
    BILL_CLIPS
        .iter()
        .map(|&requested| AnimationRef {
            model: "c_bill",
            requested,
            fallback: None,
        })
        .chain(ALICE_CLIPS.iter().map(|&requested| AnimationRef {
            model: "alice",
            requested,
            fallback: None,
        }))
        .chain([AnimationRef {
            model: "c_bill",
            requested: "talk_liftbelt",
            fallback: Some("idle_liftbelt"),
        }])
        .collect()
}

const EXIT: ExitSpec = ExitSpec {
    map: "potears3",
    entrance: "potears3_start1",
};
const ENDING: SceneSpec = SceneSpec {
    id: DIALOGUE,
    version: 1,
    duration: 7.8,
    shots: &[ShotSpec {
        start: 0.,
        track: "suck_cam",
        offset: 0.,
        hold: 7.8,
    }],
    cues: &[5.7],
    end: EndSpec {
        landing: None,
        exit: Some(EXIT),
    },
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Dialogue {
    Dormant,
    Playing,
    Read,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    #[default]
    Dormant,
    Approach,
    Conversation,
    Ending,
    Done,
    LegacyDialogue,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Presentation {
    phase: Phase,
    time: f32,
    line: usize,
    line_time: f32,
    shot: usize,
    shot_time: f32,
    acting: Option<f32>,
    run_time: f32,
    ending: Option<SceneState>,
    #[serde(default)]
    skipping: bool,
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    dialogue: Dialogue,
    #[serde(default)]
    scene: Presentation,
    #[serde(default)]
    guards: Vec<guards::Guard>,
    #[serde(default)]
    gate_time: f32,
    #[serde(default)]
    pond: transport::State,
    #[serde(default)]
    fish: fish::State,
}
impl state::State for Saved {
    const VERSION: u8 = 3;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, visit: state::Visit) -> Result<()> {
        ensure!(!visit.returning, "Bill scene belongs to the first visit");
        if self.version == 1 {
            return Ok(());
        }
        self.pond.validate()?;
        self.fish.validate()?;
        ensure!(
            self.fish.attack.is_none() || self.scene.phase == Phase::Dormant,
            "Overlapping fish and Bill scenes"
        );
        ensure!(self.guards.len() == 2, "Invalid house guard count");
        for g in &self.guards {
            ensure!(g.enabled, "Disabled house guard");
            g.validate()?;
        }
        state::clock("antguard delay", self.gate_time, 2.)?;
        let s = &self.scene;
        for t in [s.time, s.line_time, s.shot_time, s.run_time]
            .into_iter()
            .chain(s.acting)
        {
            state::clock("Bill scene", t, 3600.)?;
        }
        ensure!(s.line < 13 && s.shot < 6, "Invalid Bill shot/line");
        ensure!(
            !s.skipping || matches!(s.phase, Phase::Ending | Phase::Done),
            "Invalid Bill skip phase"
        );
        ensure!(
            !matches!(s.phase, Phase::Conversation | Phase::LegacyDialogue)
                || self.dialogue == Dialogue::Playing,
            "Inconsistent Bill dialogue phase"
        );
        ensure!(
            self.gate_time == 0. || self.guards.iter().all(|g| g.health == 0.),
            "Live guard released Bill gate"
        );
        ensure!(
            !matches!(
                s.phase,
                Phase::Approach | Phase::Conversation | Phase::Ending | Phase::Done
            ) || self.gate_time == 2.,
            "Scene bypassed antguard gate"
        );
        ensure!(
            matches!(s.phase, Phase::Ending | Phase::Done) == s.ending.is_some(),
            "Invalid house ending lifecycle"
        );
        if let Some(e) = &s.ending {
            e.validate(&ENDING)?;
            ensure!(
                e.finished == (s.phase == Phase::Done) && self.dialogue == Dialogue::Read,
                "Invalid house completion"
            );
        }
        Ok(())
    }
}
pub struct PoolTwo {
    saved: Saved,
    data: data::Data,
    pond: transport::Data,
    colliders: Vec<Collider>,
    bill: Vec3,
    alice: Vec3,
    pub(crate) legacy_program: bool,
    legacy_pond: bool,
}
impl PoolTwo {
    fn load(assets: &mut Assets, map: &Bsp) -> Result<Self> {
        let data = data::Data::load(assets, map)?;
        let mut guards = Vec::new();
        for (id, e) in map
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| e.get("targetname").is_some_and(|n| n == "antguards"))
        {
            let flags = e
                .get("spawnflags")
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
            if !map.difficulty.allows(flags) {
                continue;
            }
            let feet = e
                .get("origin")
                .and_then(|s| crate::interaction::vector(s))
                .context("Missing guard position")?;
            let yaw = e
                .get("angle")
                .and_then(|n| n.parse::<f32>().ok())
                .unwrap_or(0.)
                .to_radians();
            let corporal = e.get("model").is_some_and(|s| s == "c_armyantcorp.tik");
            guards.push(guards::Guard::new(id, corporal, feet, yaw));
        }
        ensure!(guards.len() == 2, "Expected two authored antguards");
        let mut owner = Self {
            saved: Saved {
                version: 3,
                dialogue: Dialogue::Dormant,
                scene: Presentation::default(),
                guards,
                gate_time: 0.,
                pond: transport::State::default(),
                fish: fish::State::default(),
            },
            bill: data.points["bill_stand"],
            alice: data.points["alice_stand"],
            data,
            pond: transport::Data::load(map)?,
            colliders: Vec::new(),
            legacy_program: false,
            legacy_pond: false,
        };
        owner.rebuild(map)?;
        Ok(owner)
    }
    /// Retained C3 fixture and old-save continuation. Consuming this alone never opens an exit.
    pub fn begin_dialogue(&mut self, story: &mut Story) -> bool {
        if self.saved.dialogue != Dialogue::Dormant || story.busy() || story.has_seen(DIALOGUE) {
            return false;
        }
        if !story.trigger_gated(DIALOGUE, "potears2.bill_dialogue_ready", true) {
            return false;
        }
        self.saved.dialogue = Dialogue::Playing;
        self.saved.scene.phase = Phase::LegacyDialogue;
        true
    }
    fn ready(&self) -> bool {
        self.saved.gate_time == 2.
    }
    fn finish(&mut self, world: &World, player: &mut Player) -> Result<()> {
        if !self.ready() || self.saved.scene.phase == Phase::Done {
            return Ok(());
        }
        let e = self
            .saved
            .scene
            .ending
            .get_or_insert_with(|| SceneState::new(&ENDING));
        SceneRunner {
            spec: &ENDING,
            state: e,
        }
        .finish(world, player)?;
        self.saved.scene.phase = Phase::Done;
        self.saved.dialogue = Dialogue::Read;
        player.velocity = Vec3::ZERO;
        player.cancel_climb();
        player.release_rope();
        player.script_motion = 1;
        Ok(())
    }
}
impl LevelController for PoolTwo {
    fn id(&self) -> &'static str {
        "potears2"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        if !self.legacy_program {
            f.flag(
                "potears2.guards_dead",
                self.ready() && self.saved.scene.phase == Phase::Dormant,
            );
        }
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        if t.thread == DIALOGUE {
            Some(if self.legacy_program {
                Condition::Always.not()
            } else {
                Condition::flag("potears2.guards_dead")
            })
        } else {
            (t.class == TriggerClass::Exit || (!self.legacy_program && t.name == "suck_push"))
                .then(|| Condition::Always.not())
        }
    }
    fn event(&mut self, thread: &str) -> Option<Events> {
        match thread {
            "leaf1startmoving" | "leaf2startmoving" => {
                let i = usize::from(thread == "leaf2startmoving");
                self.saved.pond.leaves[i].get_or_insert(self.saved.pond.age);
                return Some(Events::default());
            }
            "fishtrigger" | "ladies1and2" | "ladies3and4" | "ladies5and6" => {
                return Some(Events::default())
            }
            _ => {}
        }
        if thread != DIALOGUE {
            return None;
        }
        if self.ready() && self.saved.scene.phase == Phase::Dormant {
            self.saved.scene = Presentation {
                phase: Phase::Approach,
                ..Default::default()
            };
        }
        Some(Events::default())
    }
    fn dialogue_complete(&mut self, name: &str) -> Events {
        if name == DIALOGUE && self.saved.dialogue == Dialogue::Playing {
            self.saved.dialogue = Dialogue::Read;
            if self.saved.scene.phase == Phase::LegacyDialogue {
                self.saved.scene = Presentation::default();
            } else if self.saved.scene.phase == Phase::Conversation {
                self.start_ending();
            }
        }
        Events::default()
    }
    fn scripted(&self) -> bool {
        self.saved.fish.attack.is_some() || self.saved.scene.phase != Phase::Dormant
    }
    fn controlled(&self) -> bool {
        self.scripted()
    }
    fn allow_cheshire(&self) -> bool {
        !self.scripted()
    }
    fn scene_id(&self) -> Option<&'static str> {
        (self.saved.fish.attack.is_none()
            && self.scripted()
            && self.saved.scene.phase != Phase::Done
            && !self.saved.scene.skipping)
            .then_some(DIALOGUE)
    }
    fn prepare_story(&self, story: &mut Story) -> bool {
        if self.saved.scene.phase == Phase::Conversation
            && self.saved.dialogue == Dialogue::Playing
            && !story.has_seen(DIALOGUE)
            && !story.busy()
        {
            story.trigger_gated(DIALOGUE, "potears2.bill_dialogue_ready", true);
        }
        !matches!(
            self.saved.scene.phase,
            Phase::Approach | Phase::Ending | Phase::Done
        )
    }
    fn sync_story(&mut self, story: &Story) {
        self.sync_line(story);
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        self.saved.fish.camera().or_else(|| self.scene_camera())
    }
    fn fade(&self) -> Option<(Color, f32)> {
        if self.saved.scene.phase == Phase::Done {
            return Some((WHITE, 1.));
        }
        self.saved.scene.skipping.then(|| {
            (
                WHITE,
                ((self.saved.scene.ending.as_ref().unwrap().time - 7.3) / 0.5).clamp(0., 1.),
            )
        })
    }
    fn skip(
        &mut self,
        _: &Bsp,
        _world: &mut World,
        player: &mut Player,
        story: &mut Story,
    ) -> Result<bool> {
        if self.scene_id().is_none() {
            return Ok(false);
        }
        if self.saved.scene.phase == Phase::LegacyDialogue {
            story.finish_sequence(DIALOGUE);
            self.dialogue_complete(DIALOGUE);
        } else {
            // Queue before finishing even when skipping the initial approach, so
            // watched and skipped paths commit the same C3 dialogue identity.
            if !story.has_seen(DIALOGUE) {
                story.trigger_gated(DIALOGUE, "potears2.bill_dialogue_ready", true);
            }
            story.finish_sequence(DIALOGUE);
            self.saved.dialogue = Dialogue::Read;
            self.start_ending();
            let e = self.saved.scene.ending.as_mut().unwrap();
            e.time = 7.3;
            e.cast_time = 7.3;
            e.shot_time = 7.3;
            e.fired = 1;
            self.saved.scene.skipping = true;
            // The final fade advances through the same C1 completion transaction.
            SceneRunner {
                spec: &ENDING,
                state: e,
            }
            .capture(player);
        }
        Ok(true)
    }
    fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. {
            return Ok(());
        }
        let dt = dt.min(0.1);
        if self.saved.guards.iter().all(|g| g.health == 0.) {
            self.saved.gate_time = (self.saved.gate_time + dt).min(2.);
        }
        let pond_fixed: Vec<_> = fixed
            .iter()
            .cloned()
            .chain(self.colliders.iter().cloned())
            .collect();
        self.pond
            .advance(dt, map, &mut self.saved.pond, world, player, &pond_fixed)?;
        if self.saved.scene.phase == Phase::Dormant {
            let inside = self.pond.fish.touches(
                player.feet + crate::collision::PLAYER_CENTER,
                player.feet + crate::collision::PLAYER_CENTER,
                crate::collision::PLAYER_HALF,
            );
            self.saved.fish.step(dt, inside, player);
        }
        self.advance_scene(dt, world, player)?;
        self.rebuild(map)?;
        world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        Ok(())
    }
    fn update(&mut self, _: &mut World, _: &Player, _: Vec3, _: bool) -> Events {
        let mut e = Events::default();
        if let Some(s) = &mut self.saved.scene.ending {
            e.transition = s.exit.request(EXIT);
            if s.fired & 1 == 0 && s.time >= ENDING.cues[0] && !s.finished {
                s.fired |= 1;
                e.sound = Some("sound/character/alice/death_fall.wav".into());
            }
        }
        e
    }
    fn transition_failed(&mut self, exit: &(String, Option<String>)) {
        if EXIT.matches(exit) {
            if let Some(e) = &mut self.saved.scene.ending {
                e.exit.failed();
            }
        }
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.door_poses()
    }
    fn colliders(&self) -> Vec<Collider> {
        self.colliders
            .iter()
            .cloned()
            .chain(self.pond.colliders(&self.saved.pond))
            .collect()
    }
    fn targets(&self) -> Vec<Target> {
        if self.scripted() {
            return Vec::new();
        }
        self.saved
            .guards
            .iter()
            .filter(|g| g.health > 0.)
            .map(|g| g.target(guards::BASE + g.id))
            .collect()
    }
    fn hit(&mut self, hit: Hit) -> Option<&'static str> {
        if self.scripted() {
            return None;
        }
        self.saved
            .guards
            .iter_mut()
            .find(|g| guards::BASE + g.id == hit.id)?
            .hit(hit)
    }
    fn provoke_summon(&mut self, id: usize) {
        if let Some(g) = self
            .saved
            .guards
            .iter_mut()
            .find(|g| guards::BASE + g.id == id)
        {
            g.opponents.demon = true;
        }
    }
    fn combat(&mut self, combat: &mut crate::level::Combat<'_>) -> Feedback {
        let mut f = Feedback::default();
        if !self.scripted() {
            for g in &mut self.saved.guards {
                g.advance(combat, &self.data, &mut f);
            }
        }
        f
    }
    fn loot_sources(&self) -> Vec<Source> {
        self.saved
            .guards
            .iter()
            .map(|g| Source {
                id: guards::BASE + g.id,
                feet: g.feet,
                dead: g.health == 0.,
                grade: g.grade(),
            })
            .collect()
    }
    fn sound_state(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        if let Some(t) = self.saved.fish.attack {
            clocks.push(crate::audio::world::Clock {
                key: "potears2.fish.bite",
                time: t as f32,
                period: None,
                origin: self.saved.fish.at,
                cues: &[(0.58, "sound/character/snark/water/eat_alice.wav")],
            });
        }
        if let Some(e) = &self.saved.scene.ending {
            if e.time >= 4. && !e.finished {
                loops.push(crate::audio::LoopCue {
                    clock: Some(e.time - 4.),
                    id: guards::BASE + 543,
                    path: "sound/ambience/special/card_doors.wav",
                    origin: self.data.points["duchessdoor1org"],
                });
                clocks.push(crate::audio::world::Clock {
                    key: "potears2.bill.panic",
                    time: (e.time - 4.8).max(0.),
                    period: None,
                    origin: self.bill_pose().translation,
                    cues: &[(0., "sound/character/bill/paniked_run.wav")],
                });
            }
        }
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, value: &serde_json::Value, map: &Bsp) -> Result<()> {
        let mut s: Saved = state::load(value, state::Visit { returning: false })?;
        if s.version == 1 {
            s.version = 2;
            s.guards = self.saved.guards.clone();
            s.gate_time = 0.;
            s.scene = Presentation {
                phase: if s.dialogue == Dialogue::Playing {
                    Phase::LegacyDialogue
                } else {
                    Phase::Dormant
                },
                ..Default::default()
            };
        }
        ensure!(
            s.guards
                .iter()
                .zip(&self.saved.guards)
                .all(|(a, b)| a.id == b.id && a.corporal == b.corporal),
            "Saved guards differ from authored difficulty"
        );
        self.legacy_pond = s.version < 3;
        s.version = 3;
        self.saved = s;
        self.rebuild(map)
    }
    fn upgraded(&mut self) {
        self.legacy_pond = true;
    }
    fn validate_player(&self, p: &Player) -> Result<()> {
        ensure!(
            !(self.legacy_pond
                && self.saved.scene.phase == Phase::Dormant
                && p.feet.z < 0.
                && p.feet.x > -3456.
                && p.feet.x < 2560.
                && p.feet.y > -2176.
                && p.feet.y < 2560.),
            "Legacy pond position needs restored entrance"
        );
        Ok(())
    }
    fn prepare_player(&mut self, stats: &mut crate::inventory::Stats, _: &mut Player) {
        if self.saved.fish.attack == Some(2.) && !self.saved.fish.killed {
            stats.damage(10000.);
            self.saved.fish.killed = true;
        }
    }
    fn hides_player(&self) -> bool {
        self.saved.fish.attack.is_some_and(|t| t >= 1.)
    }
    fn blocks_weapons(&self) -> bool {
        self.saved.fish.attack.is_some()
    }
    fn pickup_origin(&self, id: &str) -> Option<Vec3> {
        (id == "potears2:36").then(|| self.pond.pickup())
    }
    fn patrols(&self) -> &'static [&'static str] {
        if self.saved.pond.ladies {
            &["lady3", "lady4"]
        } else {
            &[]
        }
    }
    fn objective(&self) -> Option<String> {
        Some(
            if self.ready() {
                "Approach the house and speak to Bill."
            } else if self.saved.pond.leaves[1].is_some() {
                "Leave the second leaf before it sinks. Cross the pads and defeat the house guards."
            } else if self.saved.pond.leaves[0].is_some() {
                "Follow the east bank to the second leaf. Stay out of the pond."
            } else {
                "Follow the bank and lily pads to the first ride leaf. The fish are waiting below."
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
    id: "potears2", applies: |m,e| super::first_visit(m,e,"potears2"),
    load: |a,m,_,_| Ok(Box::new(PoolTwo::load(a,m)?)),
    art: Some(|a,_,_| Ok(Box::new(scene::Art::load(a)?))),
    owns_submodel: |m,e| m == "potears2" && e.get("targetname").is_some_and(|s| matches!(s.as_str(), "duchessdoor1obj" | "duchessdoor2obj")) || (m=="potears2" && transport::owns(e)),
    owns_npc: |name,model| (name == "billthelizard" && model == "c_bill") || (name == "antguards" && matches!(model,"c_armyant"|"c_armyantcorp")),
    target_base: Some(guards::BASE), story_beats: BEATS,
    checks: &[
        Check { flag: "--potears2-store-write-check", help: "Write normal-route transport saves.", run: Run::Windowed(|a|Box::pin(save_check::run(a,true))) },
        Check { flag: "--potears2-store-read-check", help: "Read normal-route saves in a fresh process.", run: Run::Windowed(|a|Box::pin(save_check::run(a,false))) },
        Check { flag: "--potears2-transport-check", help: "Verify pond rides, fish timer, pause and restore.", run: Run::Headless(transport_check::check) },
        Check { flag: "--potears2-route-check", help: "Ordinary entrance to Just Desserts.", run: Run::Headless(route::check) },
        Check { flag: "--potears2-route-native-check", help: "Ordinary pond route with the native cast.", run: Run::Windowed(|a|Box::pin(route::render(a))) },
        Check { flag: "--potears2-check", help: "Verify the staged house guard/scene ending; restored pond route has separate checks.", run: Run::Headless(check) },
        Check { flag: "--potears2-render-check", help: "Render staged Bill scene cameras and cast.", run: Run::Windowed(render) },
    ],
    save_cases: &[
        SaveCase { name: "potears2-scene-guards", visit: "potears2$first", stage: None, behavior: None },
        SaveCase { name: "potears2-scene-approach", visit: "potears2$first", stage: None, behavior: None },
        SaveCase { name: "potears2-scene-middle", visit: "potears2$first", stage: None, behavior: None },
        SaveCase { name: "potears2-scene-doors", visit: "potears2$first", stage: None, behavior: None },
        SaveCase { name: "potears2-scene-suction", visit: "potears2$first", stage: None, behavior: None },
        SaveCase { name: "potears2-scene-done", visit: "potears2$first", stage: None, behavior: None },
        SaveCase { name: "potears2-dialogue-first", visit: "potears2$first", stage: None, behavior: Some(saved_behavior) },
        SaveCase { name: "potears2-dialogue-middle", visit: "potears2$first", stage: None, behavior: Some(saved_behavior) },
        SaveCase { name: "potears2-dialogue-last", visit: "potears2$first", stage: None, behavior: Some(saved_behavior) },
        SaveCase { name: "potears2-dialogue-read", visit: "potears2$first", stage: None, behavior: Some(saved_behavior) },
    ],
    visibility: &[super::VisibilityFixture { name: "potears2-dialogue-bindings", run: render }],
};
fn owner(i: &mut Interactions) -> Result<&mut PoolTwo> {
    i.levels
        .iter_mut()
        .find_map(|s| s.ctl.downcast_mut())
        .context("Bill owner missing")
}

#[cfg(test)]
mod tests {
    use super::state::State;
    use super::*;
    fn fresh() -> Saved {
        Saved {
            version: 3,
            dialogue: Dialogue::Dormant,
            scene: Presentation::default(),
            guards: vec![
                guards::Guard::new(71, false, Vec3::ZERO, 0.),
                guards::Guard::new(337, false, Vec3::X * 100., 0.),
            ],
            gate_time: 0.,
            pond: transport::State::default(),
            fish: fish::State::default(),
        }
    }
    #[test]
    fn live_guards_and_dialogue_only_cannot_commit_a_house_exit() {
        let visit = state::Visit { returning: false };
        let mut s = fresh();
        assert!(s.validate(visit).is_ok());
        s.scene.phase = Phase::Conversation;
        s.dialogue = Dialogue::Playing;
        assert!(s.validate(visit).is_err());
        s.scene.phase = Phase::LegacyDialogue;
        assert!(s.validate(visit).is_ok());
        s.scene.skipping = true;
        assert!(s.validate(visit).is_err());
    }
    #[test]
    fn ending_requires_guard_deaths_and_a_consistent_durable_commit() {
        let visit = state::Visit { returning: false };
        let mut s = fresh();
        for g in &mut s.guards {
            g.hit(Hit {
                id: 0,
                damage: 100.,
                kind: crate::combat::DamageKind::Knife,
                knockback: Vec3::ZERO,
            });
        }
        s.gate_time = 2.;
        s.dialogue = Dialogue::Read;
        s.scene.phase = Phase::Ending;
        s.scene.ending = Some(SceneState::new(&ENDING));
        assert!(s.validate(visit).is_ok());
        s.scene.ending.as_mut().unwrap().exit.committed = true;
        assert!(s.validate(visit).is_err());
        s.scene.ending.as_mut().unwrap().exit.committed = false;
        s.guards[0].enabled = false;
        assert!(s.validate(visit).is_err());
    }
}
