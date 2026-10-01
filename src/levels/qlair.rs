//! Heart of Darkness: authored two-arena finale and durable scene/ending lifecycle.
mod art;
mod audit;
mod battle;
mod check;
mod data;
mod effects;
mod projectile;
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
    level::{Combat, LevelArt, LevelController, Respawn, TriggerClass, TriggerInfo, Upgrade},
    movement::Player,
    skeletal::Transform,
    story::{registry::BeatSpec, Story},
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;
const BASE: usize = 9_800_000;
const INTRO: &str = "QLair_Cinema1";
const TALK: &str = "QLair_Body_Dialog";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    Corridor,
    Intro,
    Queen1,
    Birth,
    Queen2,
    Death,
    Ending,
    Done,
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    phase: Phase,
    time: f32,
    clock: f32,
    initialized: bool,
    queen1: f32,
    queen2: f32,
    parts: [f32; 4],
    #[serde(default)]
    part_death: [f32; 4],
    attack: battle::Attack,
    #[serde(default)]
    attack_variant: u8,
    attack_time: f32,
    #[serde(default = "battle::default_delay")]
    attack_delay: f32,
    #[serde(default)]
    part_attack: [Option<battle::Attack>; 4],
    #[serde(default)]
    part_time: [f32; 4],
    #[serde(default)]
    part_fired: [u64; 4],
    attack_target: Vec3,
    fired: u64,
    random: u32,
    projectiles: Vec<battle::Shot>,
    #[serde(default)]
    impacts: Vec<projectile::Impact>,
    #[serde(default)]
    debris: Vec<effects::Particle>,
    #[serde(default)]
    death_burst: bool,
    #[serde(default)]
    death_effects: u8,
    #[serde(default)]
    head_watch: crate::facial::Watch,
    #[serde(default)]
    grab_time: f32,
    #[serde(default)]
    grab_arrived: bool,
    body: Transform,
    platform: i8,
    position: u8,
    motion: u8,
    motion_time: f32,
    middle: f32,
    collapsed: f32,
    dialogue_done: bool,
    powered: bool,
    refilled: bool,
    checkpoint: bool,
    checkpoint_saved: bool,
    essence: u8,
    essence_wait: f32,
    alice: Transform,
    grabbed: bool,
    alive: bool,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        for (n, t, max) in [
            ("scene", self.time, 1000.),
            ("level", self.clock, 86400.),
            ("attack", self.attack_time, 100.),
            ("motion", self.motion_time, 100.),
            ("middle", self.middle, 17.),
            ("collapse", self.collapsed, 20.),
            ("essence", self.essence_wait, 10.),
        ] {
            state::clock(n, t, max)?;
        }
        ensure!(
            (0. ..=2500.).contains(&self.queen1) && (0. ..=4500.).contains(&self.queen2),
            "Invalid Queen health"
        );
        for (health, max) in self.parts.iter().zip([1425., 1925., 1325., 1625.]) {
            ensure!(
                health.is_finite() && (0. ..=max).contains(health),
                "Invalid tentacle health"
            );
        }
        ensure!(
            (-4..=4).contains(&self.platform)
                && self.platform != 0
                && (1..=5).contains(&self.position)
                && self.motion <= 2
                && self.attack_variant
                    <= if self.attack == battle::Attack::Pain {
                        2
                    } else {
                        1
                    }
                && self.essence < 4,
            "Invalid Queen movement or essence index"
        );
        for t in self.part_death {
            state::clock("tentacle death", t, 100.)?;
        }
        state::clock("attack delay", self.attack_delay, 3.)?;
        state::clock("grab", self.grab_time, 10.)?;
        ensure!(
            self.head_watch.valid() && self.death_effects < 64,
            "Invalid Queen controllers"
        );
        for t in self.part_time {
            state::clock("part attack", t, 100.)?;
        }
        for p in [self.body, self.alice] {
            ensure!(
                p.translation.is_finite()
                    && p.translation.abs().max_element() < 100000.
                    && p.rotation.is_finite()
                    && (p.rotation.length_squared() - 1.).abs() < 0.01,
                "Invalid finale pose"
            );
        }
        ensure!(
            self.attack_target.is_finite()
                && self.attack_target.abs().max_element() < 100000.
                && self.projectiles.len() < 256,
            "Invalid boss attacks"
        );
        for p in &self.projectiles {
            p.validate()?;
        }
        ensure!(
            self.impacts.len() <= 128 && self.impacts.iter().all(projectile::Impact::valid),
            "Invalid Queen impact effects"
        );
        ensure!(
            self.debris.len() <= 192 && self.debris.iter().all(effects::Particle::valid),
            "Invalid Queen debris"
        );
        ensure!(
            !self.checkpoint_saved || self.checkpoint,
            "Checkpoint acknowledged before request"
        );
        ensure!(!self.refilled || self.powered, "Power refill without power");
        let second = matches!(
            self.phase,
            Phase::Queen2 | Phase::Death | Phase::Ending | Phase::Done
        );
        ensure!(
            !second || (self.powered && self.checkpoint && self.queen1 == 0.),
            "Invalid second-phase prerequisites"
        );
        ensure!(
            !matches!(
                self.phase,
                Phase::Birth | Phase::Queen2 | Phase::Death | Phase::Ending | Phase::Done
            ) || self.queen1 == 0.,
            "Live Queen1 after defeat"
        );
        ensure!(
            !matches!(self.phase, Phase::Death | Phase::Ending | Phase::Done)
                || self.queen2 <= 1000.,
            "Live Queen2 after defeat"
        );
        ensure!(
            !self.grabbed || (self.phase == Phase::Queen1 && self.attack == battle::Attack::Grab),
            "Stale Queen grab"
        );
        Ok(())
    }
}
struct Queen {
    data: data::Data,
    saved: Saved,
    colliders: Vec<Collider>,
    geometry: Vec<(usize, Vec3, Quat)>,
}
impl Queen {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let data = data::Data::load(a, map)?;
        let saved = Saved {
            version: 1,
            phase: Phase::Corridor,
            time: 0.,
            clock: 0.,
            initialized: false,
            queen1: 2500.,
            queen2: 4500.,
            parts: [1425., 1925., 1325., 1625.],
            part_death: [0.; 4],
            attack: battle::Attack::Idle,
            attack_variant: 0,
            attack_time: 0.,
            attack_delay: battle::default_delay(),
            part_attack: [None; 4],
            part_time: [0.; 4],
            part_fired: [0; 4],
            attack_target: Vec3::ZERO,
            fired: 0,
            random: 0x514c4149,
            projectiles: Vec::new(),
            impacts: Vec::new(),
            debris: Vec::new(),
            death_burst: false,
            death_effects: 0,
            head_watch: Default::default(),
            grab_time: 0.,
            grab_arrived: false,
            body: data.points["bitch_pos2"],
            platform: 4,
            position: 5,
            motion: 0,
            motion_time: 0.,
            middle: 17.,
            collapsed: 0.,
            dialogue_done: false,
            powered: false,
            refilled: false,
            checkpoint: false,
            checkpoint_saved: false,
            essence: 0,
            essence_wait: 0.,
            alice: data.points["qlair_start1"],
            grabbed: false,
            alive: true,
        };
        let mut out = Self {
            data,
            saved,
            colliders: Vec::new(),
            geometry: Vec::new(),
        };
        out.rebuild(map)?;
        Ok(out)
    }
    fn start(&mut self, p: Phase) {
        self.saved.phase = p;
        self.saved.time = 0.;
        self.saved.dialogue_done = false;
        self.saved.attack = battle::Attack::Idle;
        self.saved.attack_variant = 0;
        self.saved.attack_time = 0.;
        self.saved.fired = 0;
        self.saved.part_attack = [None; 4];
        self.saved.part_time = [0.; 4];
        self.saved.part_fired = [0; 4];
        self.saved.grabbed = false;
        self.saved.grab_time = 0.;
        self.saved.grab_arrived = false;
        self.saved.projectiles.clear();
    }
    fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        let poses = self
            .transforms()
            .into_iter()
            .filter(|(m, _, _)| {
                *m != 1
                    || matches!(
                        self.saved.phase,
                        Phase::Corridor | Phase::Intro | Phase::Queen1
                    )
            })
            .collect::<Vec<_>>();
        if poses == self.geometry {
            return Ok(());
        }
        self.colliders = poses
            .iter()
            .map(|&(m, p, r)| Collider::model(map, m, p, r, true))
            .collect::<Result<_>>()?;
        self.geometry = poses;
        Ok(())
    }
    fn refill(stats: &mut Stats) {
        stats.full_stats();
    }
    fn essence_pose(&self) -> Transform {
        self.data.points[&format!(
            "{}_me{}",
            if self.saved.phase == Phase::Queen1 {
                "get"
            } else {
                "help"
            },
            self.saved.essence + 1
        )]
    }
}
impl LevelController for Queen {
    fn id(&self) -> &'static str {
        "qlair"
    }
    fn allow_cheshire(&self) -> bool {
        false
    }
    fn particles(&self, steam: &mut crate::particles::Steam) {
        let active = matches!(
            self.saved.phase,
            Phase::Queen2 | Phase::Death | Phase::Ending
        ) || (self.saved.phase == Phase::Birth
            && self.saved.time >= self.data.reveal());
        let t = if self.saved.phase == Phase::Birth {
            self.saved.time - self.data.reveal()
        } else {
            self.saved.time + 10.5
        } % 13.;
        steam.gate(
            &self
                .data
                .emitters
                .iter()
                .map(|(id, name)| {
                    (
                        *id,
                        active
                            && name != "alice_magic_power"
                            && (self.saved.phase != Phase::Queen2
                                || if name == "lightning_back" {
                                    (4. ..8.).contains(&t)
                                } else {
                                    t >= 10.
                                }),
                    )
                })
                .collect::<Vec<_>>(),
        );
    }
    fn music_mood(&self) -> Option<&'static str> {
        Some(
            if self.saved.phase == Phase::Birth
                && self.saved.time >= 11.
                && self.saved.time < self.data.reveal()
            {
                "suspense"
            } else {
                "normal"
            },
        )
    }
    fn ignores_watch(&self) -> bool {
        true
    }
    fn facts(&self) -> crate::event::Facts {
        let mut f = crate::event::Facts::default();
        f.flag("qlair.queen2", self.saved.phase == Phase::Queen2);
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<crate::event::Condition> {
        (t.class == TriggerClass::Teleport || t.thread.starts_with("MoveQueen"))
            .then(|| crate::event::Condition::flag("qlair.queen2"))
    }
    fn event(&mut self, thread: &str) -> Option<Events> {
        if thread == INTRO {
            if self.saved.phase == Phase::Corridor {
                self.start(Phase::Intro);
            }
        } else if let Some(n) = thread
            .strip_prefix("MoveQueen")
            .and_then(|s| s.parse::<i8>().ok())
            .filter(|n| (1..=4).contains(n))
        {
            if self.saved.phase == Phase::Queen2 {
                if self.saved.platform > 0 {
                    self.saved.platform = -n;
                    if self.saved.position != 5 && self.saved.middle == 0. {
                        self.saved.middle = 17.;
                        self.saved.motion = 1;
                        self.saved.motion_time = 0.;
                    }
                } else {
                    self.saved.platform = n;
                }
            }
        } else {
            return None;
        }
        Some(Events::default())
    }
    fn prepare_player(&mut self, stats: &mut Stats, p: &mut Player) {
        self.saved.alive = stats.alive();
        if !self.saved.initialized {
            Self::refill(stats);
            self.saved.initialized = true;
        }
        if self.saved.powered && !self.saved.refilled && stats.alive() {
            Self::refill(stats);
            self.saved.refilled = true;
        }
        self.saved.alice = Transform {
            translation: p.feet,
            rotation: Quat::from_rotation_z(p.script_facing),
        };
        let essence_delta = (p.feet - self.essence_pose().translation).abs();
        if matches!(self.saved.phase, Phase::Queen1 | Phase::Queen2)
            && self.saved.essence_wait == 0.
            && stats.alive()
            && essence_delta.x <= 32.
            && essence_delta.y <= 32.
            && essence_delta.z <= 64.
            && (stats.sanity() < 100. || stats.will() < 100.)
        {
            let amount = if self.saved.phase == Phase::Queen1 {
                25.
            } else {
                50.
            };
            stats.essence(amount);
            self.saved.essence = (self.saved.essence + 1)
                % if self.saved.phase == Phase::Queen1 {
                    2
                } else {
                    4
                };
            self.saved.essence_wait = 10.;
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
        if dt <= 0. || !self.saved.alive {
            return Ok(());
        }
        let dt = dt.min(0.1);
        self.saved.clock = (self.saved.clock + dt).min(86400.);
        self.saved.time = (self.saved.time + dt).min(1000.);
        self.saved.essence_wait = (self.saved.essence_wait - dt).max(0.);
        for impact in &mut self.saved.impacts {
            impact.age += dt;
        }
        self.saved.impacts.retain(|impact| impact.age < 8.);
        self.advance_effects(dt, world);
        if self.saved.collapsed > 0. {
            self.saved.collapsed = (self.saved.collapsed + dt).min(20.);
        }
        self.advance_scene(world, p)?;
        self.rebuild(map)?;
        world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        Ok(())
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.brush_poses()
    }
    fn lights(&self) -> Vec<crate::lighting::Light> {
        self.actor_lights()
    }
    fn colliders(&self) -> Vec<Collider> {
        self.colliders.clone()
    }
    fn scripted(&self) -> bool {
        matches!(
            self.saved.phase,
            Phase::Intro | Phase::Birth | Phase::Death | Phase::Ending | Phase::Done
        )
    }
    fn controlled(&self) -> bool {
        self.scripted() || self.saved.grabbed
    }
    fn scene_id(&self) -> Option<&'static str> {
        match self.saved.phase {
            Phase::Intro => Some(INTRO),
            Phase::Birth => Some("Die_Bitch"),
            Phase::Death => Some("Queen2_Death"),
            _ => None,
        }
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        self.scene_camera()
    }
    fn fade(&self) -> Option<(Color, f32)> {
        self.scene_fade()
    }
    fn prepare_story(&self, story: &mut Story) -> bool {
        self.scene_story(story)
    }
    fn dialogue_complete(&mut self, name: &str) -> Events {
        if (name == INTRO && self.saved.phase == Phase::Intro)
            || (name == TALK && self.saved.phase == Phase::Birth)
        {
            self.saved.dialogue_done = true;
        }
        Events::default()
    }
    fn skip(
        &mut self,
        map: &Bsp,
        world: &mut World,
        p: &mut Player,
        story: &mut Story,
    ) -> Result<bool> {
        self.skip_scene(map, world, p, story)
    }
    fn targets(&self) -> Vec<Target> {
        self.battle_targets()
    }
    fn hit(&mut self, h: Hit) -> Option<&'static str> {
        self.battle_hit(h)
    }
    fn combat(&mut self, c: &mut Combat<'_>) -> Feedback {
        self.battle(c)
    }
    fn checkpoint_requested(&self) -> bool {
        self.saved.checkpoint && !self.saved.checkpoint_saved
    }
    fn checkpoint_written(&mut self) {
        self.saved.checkpoint_saved = true;
    }
    fn ending_ready(&self) -> bool {
        self.saved.phase == Phase::Done
    }
    fn fog_distance(&self) -> Option<f32> {
        Some(
            if matches!(
                self.saved.phase,
                Phase::Queen2 | Phase::Death | Phase::Ending | Phase::Done
            ) {
                3000.
            } else if self.saved.phase == Phase::Birth {
                10000. - 5000. * ((self.saved.time - 23.5) / 8.).clamp(0., 1.)
            } else {
                10000.
            },
        )
    }
    fn objective(&self) -> Option<String> {
        Some(match self.saved.phase {Phase::Corridor=>"Enter the throne room.",Phase::Intro=>"The Queen reveals herself.",Phase::Queen1=>"Defeat the Queen. Keep moving to escape her magic; essence appears at the back of the hall.",Phase::Birth=>"The Heart of Darkness awakens.",Phase::Queen2=>"Defeat the Queen's true form. Destroy tentacles to disable attacks; circle the platforms for essence.",_=>"The nightmare is ending."}.into())
    }
    fn recovery_entry(&self, normal: (Vec3, f32)) -> (Vec3, f32) {
        if matches!(
            self.saved.phase,
            Phase::Queen2 | Phase::Death | Phase::Ending | Phase::Done
        ) {
            (
                self.data.points["alice_fight_queen"].translation,
                45f32.to_radians(),
            )
        } else if self.saved.phase == Phase::Queen1 {
            (
                self.data.points["alice_start_pos1"].translation,
                90f32.to_radians(),
            )
        } else {
            normal
        }
    }
    fn snapshot(&self) -> serde_json::Value {
        // A persisted second-arena snapshot already IS a usable checkpoint.
        // The live request remains pending until the writer succeeds; restoring
        // that saved moment must not issue the same autosave again.
        let mut s = self.saved.clone();
        s.checkpoint_saved |= s.checkpoint;
        state::save(&s)
    }
    fn sound_state(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        use crate::audio::world::Clock;
        if self.saved.phase == Phase::Queen2 {
            let (clip, at) = self.part_animation(1);
            if clip == "attack_jabberwock_eye" {
                loops.push(crate::audio::LoopCue {
                    id: BASE + 120,
                    path: "sound/character/jabberwock/attack5loop.wav",
                    clock: Some(at),
                    origin: self.part_pose(1).translation,
                });
            }
        }
        if self.saved.phase == Phase::Death {
            let (clip, at, pose) = self.body_performance();
            let path = if clip.contains("endloop") {
                "sound/character/queen/death_endloop.wav"
            } else {
                "sound/character/queen/death_slumploop.wav"
            };
            if clip != "death_start" {
                loops.push(crate::audio::LoopCue {
                    id: BASE + 121,
                    path,
                    clock: Some(at),
                    origin: pose.translation,
                });
            }
            clocks.push(Clock {
                key: "qlair.death.start",
                time: self.saved.time,
                period: None,
                origin: self.saved.body.translation,
                cues: &[(0., "sound/character/queen/death_start.wav")],
            });
        }
        if self.saved.phase == Phase::Ending
            && self.saved.death_burst
            && !self.saved.debris.is_empty()
        {
            clocks.push(Clock {
                key: "qlair.death.burst",
                time: self.saved.time,
                period: None,
                origin: self.saved.body.translation,
                cues: &[
                    (0., "sound/weapon/jackbomb/jackbomb_explode.wav"),
                    (0.16666667, "sound/weapon/blunderbuss/bb_explode.wav"),
                ],
            });
        }
        for (n, shot) in self.saved.projectiles.iter().enumerate() {
            let path = match shot.model_key() {
                "prj_queen_iceball" => "sound/character/queen/iceblast_loop.wav",
                "prj_queen1_blaster1" => "sound/weapon/mallet/mallet_ball_loop.wav",
                _ => continue,
            };
            loops.push(crate::audio::LoopCue {
                id: BASE + 200 + n,
                path,
                clock: Some(shot.age),
                origin: shot.at,
            });
        }
        if self.saved.collapsed > 0. {
            clocks.push(Clock {
                key: "qlair.collapse",
                time: self.saved.collapsed,
                period: None,
                origin: self.data.points["throne"].translation,
                cues: &[
                    (0., "sound/ambience/special/queensplit.wav"),
                    (1., "sound/ambience/special/thronebreak1.wav"),
                    (2.5, "sound/ambience/special/thronebreak2.wav"),
                ],
            });
        }
        if self.saved.phase == Phase::Birth {
            clocks.push(Clock {
                key: "qlair.birth",
                time: self.saved.time,
                period: None,
                origin: self.data.points["throne"].translation,
                cues: &[
                    (0., "sound/character/queen/death.wav"),
                    (23.5, "sound/character/queen/death_pullthrough.wav"),
                ],
            });
        }
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        let mut s: Saved = state::load(v, state::Visit { returning: false })?;
        if v.get("part_attack").is_none() {
            if let Some(n) = Self::attack_part(s.attack) {
                if s.parts[n] > 25. {
                    s.part_attack[n] = Some(s.attack);
                    s.part_time[n] = s.attack_time;
                }
            }
        }
        // Pre-audit v1 saves could retain a fighting tentacle below its authored
        // collapse threshold. Migrate that state to the 25-health death pose.
        for n in 0..4 {
            if s.parts[n] > 25. && s.parts[n] < 1030. {
                s.parts[n] = 25.;
                s.part_death[n] = 0.;
            }
        }
        self.saved = s;
        self.rebuild(map)
    }
    fn upgrade(&self) -> Upgrade {
        Upgrade {
            respawn: Respawn::Always,
            rearm: vec![INTRO.into()],
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
    id: "qlair",
    applies: |m, e| super::first_visit(m, e, "qlair"),
    load: |a, m, _, _| Ok(Box::new(Queen::load(a, m)?)),
    art: Some(|a, _, l| {
        Ok(Box::new(art::Art::load(
            a,
            l.downcast_ref::<Queen>().context("Queen owner missing")?,
        )?))
    }),
    owns_submodel: |m, e| m == "qlair" && e.get("classname").is_some_and(|s| s == "script_object"),
    owns_npc: |n, m| {
        m.starts_with("c_queen")
            || m.starts_with("c_q2_")
            || matches!(
                n,
                "alice_magic_power" | "lightning_front" | "lightning_back"
            )
    },
    target_base: Some(BASE),
    story_beats: &[
        BeatSpec::linear("qlair", INTRO, "../qlair", INTRO, 2),
        BeatSpec::linear("qlair", TALK, "../qlair", TALK, 3),
    ],
    checks: &[
        super::Check {
            flag: "--qlair-skip-route-check",
            help: "Fight both Queens with skipped scenes on Normal and Hard.",
            run: super::Run::Headless(check::skip_route),
        },
        super::Check {
            flag: "--qlair-second-route-check",
            help: "Staged second-arena traversal and combat diagnostic.",
            run: super::Run::Headless(check::second_route),
        },
        super::Check {
            flag: "--qlair-route-check",
            help: "Fight both Queens through the real corridor on Normal and Hard.",
            run: super::Run::Headless(check::route),
        },
        super::Check {
            flag: "--qlair-check",
            help: "Verify Queen scene, combat, checkpoint and ending contracts.",
            run: super::Run::Headless(check::check),
        },
        super::Check {
            flag: "--qlair-render-check",
            help: "Render the Queen phases and finale scenes.",
            run: super::Run::Windowed(check::render),
        },
        super::Check {
            flag: "--qlair-save-write",
            help: "Write native finale persistence fixtures.",
            run: super::Run::Windowed(saves::write),
        },
        super::Check {
            flag: "--qlair-save-read",
            help: "Restore finale fixtures in a fresh native process.",
            run: super::Run::Windowed(saves::read),
        },
    ],
    save_cases: &[],
    visibility: &[],
};

pub(crate) use check::drive as drive_route;
