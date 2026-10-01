//! Reviewed Duchess encounter. Local map/animation data, never original script execution.
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World},
    combat::{self, Feedback, Target},
    inventory::{PickupKind, Stats},
    movement::Player,
    skeletal::{Animation, Definition, Skeleton, Transform},
    story::Story,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::collections::BTreeMap;
mod intro_check;
pub use intro_check::{check as check_intro, render as render_intro};
pub const ID: usize = 4_000_000;
pub const INTRO: &str = "potears3_dialog";
pub const OUTRO: &str = "potears3_end_dialog";
const STEP: f32 = 1. / 120.;
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Stage {
    Waiting,
    Expanding,
    Introduction,
    Fighting,
    Dying,
    Rescue,
    Farewell,
    Well,
    Complete,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Action {
    Chase,
    Pepper,
    Pig,
    Smack,
    Grab,
    Bite,
    Phase,
    Pain,
    Death,
}
impl Action {
    fn clip(self) -> &'static str {
        match self {
            Self::Chase => "run",
            Self::Pepper => "attack_4",
            Self::Pig => "attack_2",
            Self::Smack => "attack_3",
            Self::Grab => "attack_1",
            Self::Bite => "raise",
            Self::Phase => "run_away",
            Self::Pain => "pain1",
            Self::Death => "death",
        }
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Shot {
    pub position: Vec3,
    pub velocity: Vec3,
    pub age: f32,
    pub pig: bool,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Burst {
    pub position: Vec3,
    pub age: f32,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    #[serde(default)]
    intro_camera: Option<(usize, f32)>,
    #[serde(default)]
    pub opponents: combat::Opponents,
    pub stage: Stage,
    pub time: f32,
    pub action: Action,
    pub clock: f32,
    pub health: f32,
    pub feet: Vec3,
    pub yaw: f32,
    pub shots: Vec<Shot>,
    pub bursts: Vec<Burst>,
    pub attacks: u32,
    pub dodges: u32,
    pub shell_returned: bool,
    pub exit_sent: bool,
    pub essence: [f32; 4],
    pub essence_taken: u32,
    initialized: bool,
    started: bool,
    cue: usize,
    wait: f32,
    dodge_wait: f32,
    pain: f32,
    accumulator: f64,
    home: Vec3,
    pub notarget: bool,
}
pub struct Timing {
    intro_tracks: Vec<crate::fortress::spline::Spline>,
    pub clips: BTreeMap<String, (f32, f32)>,
    death_focus: Vec<Vec3>,
}
impl Timing {
    fn load(a: &mut Assets) -> Result<Self> {
        let d = Definition::load(a, "models/c_duchess.tik")?;
        let s = Skeleton::parse(&a.read(&format!("{}/{}", d.path, d.model))?)?;
        let mut clips = BTreeMap::new();
        let mut death_focus = Vec::new();
        for name in [
            "idle_1", "run", "run_away", "attack_1", "raise", "attack_2", "attack_3", "attack_4",
            "pain1", "death",
        ] {
            let file = d
                .animations
                .get(name)
                .with_context(|| format!("Duchess animation missing: {name}"))?;
            let c = Animation::parse(&a.read(&format!("{}/{file}", d.path))?, s.bones.len())?;
            if name == "death" {
                let head = s
                    .bones
                    .iter()
                    .position(|b| b.name == "tag_sneeze")
                    .context("Duchess head tag missing")?;
                let neck = s
                    .bones
                    .iter()
                    .position(|b| b.name == "tag_neck")
                    .context("Duchess neck tag missing")?;
                for frame in 0..c.frames.len() {
                    let pose = s.global_pose(&c.sample(frame as f32 * c.frame_time, false));
                    // Follow the head until its removal, then the emitting neck.
                    let blend = ((frame as f32 - 102.) / 6.).clamp(0., 1.);
                    death_focus
                        .push(pose[head].translation.lerp(pose[neck].translation, blend) * d.scale);
                }
            }
            clips.insert(name.into(), (c.duration(), c.frame_time));
        }
        let mut intro_tracks = Vec::new();
        for name in [
            "potears3_kmpx1",
            "potears3_kmpx2",
            "potears3_kmpx3",
            "potears3_kmpx4",
        ] {
            let track = crate::cinematic::Track::load(a, name)?;
            intro_tracks.push(crate::fortress::spline::Spline::camera_track(
                track.controls().collect(),
            ));
        }
        Ok(Self {
            clips,
            death_focus,
            intro_tracks,
        })
    }
    fn duration(&self, action: Action) -> f32 {
        self.clips[action.clip()].0
    }
    fn frame(&self, action: Action, frame: f32) -> f32 {
        self.clips[action.clip()].1 * frame
    }
    fn focus(&self, time: f32) -> Vec3 {
        let frame = (time / self.clips["death"].1).max(0.);
        let last = self.death_focus.len() - 1;
        self.death_focus[(frame as usize).min(last)].lerp(
            self.death_focus[(frame as usize + 1).min(last)],
            frame.fract(),
        )
    }
}
struct Object {
    shape: crate::collision::model_shape::Template,
    model: usize,
    base: Vec3,
    group: Option<usize>,
    name: String,
    collider: Collider,
    pose: Transform,
}
pub struct Duchess {
    pub state: State,
    pub timing: Timing,
    objects: Vec<Object>,
    groups: Vec<(Vec3, Vec3)>,
    points: BTreeMap<String, Vec3>,
    pub pickup_id: String,
    essence_origins: [Vec3; 4],
}
fn entity<'a>(map: &'a Bsp, name: &str) -> Result<&'a BTreeMap<String, String>> {
    map.entities
        .iter()
        .find(|e| e.get("targetname").is_some_and(|n| n == name))
        .with_context(|| format!("Duchess map object {name} missing"))
}
fn origin(map: &Bsp, name: &str) -> Result<Vec3> {
    entity(map, name)?
        .get("origin")
        .and_then(|s| crate::interaction::vector(s))
        .context("Invalid Duchess map origin")
}
pub fn supported(e: &BTreeMap<String, String>) -> bool {
    e.get("classname").is_some_and(|s| s == "script_object")
}
fn group(name: &str) -> Option<usize> {
    for i in 1..=5 {
        if name == format!("movingwall{i}")
            || name == format!("wall{i}deco")
            || name == format!("mespawner{i}")
        {
            return Some(i - 1);
        }
    }
    for i in 1..=4 {
        if name == format!("movingbeam{i}") {
            return Some(i + 5);
        }
    }
    match name {
        "movingceiling" => Some(5),
        "fireplacebars" | "firesparklies" | "chimneysmoke" => Some(0),
        "entrydoor" => Some(3),
        "secretdoorobj" | "secretdoordeco" | "turtleshellitem" => Some(2),
        _ => None,
    }
}
impl Duchess {
    pub fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let timing = Timing::load(a)?;
        let mut groups = Vec::new();
        for stem in ["wall", "beam"] {
            for i in 1..=if stem == "wall" { 5 } else { 4 } {
                if stem == "beam" && i == 1 {
                    groups.push((origin(map, "ceilsmall")?, origin(map, "ceilbig")?));
                }
                groups.push((
                    origin(map, &format!("{stem}small{i}"))?,
                    origin(map, &format!("{stem}big{i}"))?,
                ));
            }
        }
        let mut points = BTreeMap::new();
        for n in [
            "duchessin",
            "duchessout",
            "fuck_pos",
            "alice_end_pos",
            "billout",
            "turtleout",
            "billstand",
            "turtlestand",
            "turtle_jump_pos",
            "jumpinthewell",
            "secretdoororigin",
            "wellleverorg",
            "billthelizard",
            "mockturtle",
        ] {
            points.insert(n.into(), origin(map, n)?);
        }
        let mut objects = Vec::new();
        for e in map.entities.iter().filter(|e| supported(e)) {
            let model = e
                .get("model")
                .and_then(|s| s.strip_prefix('*'))
                .and_then(|s| s.parse().ok())
                .context("Invalid arena model")?;
            let base = e
                .get("origin")
                .and_then(|s| crate::interaction::vector(s))
                .context("Arena origin")?;
            let name = e.get("targetname").context("Arena object name")?.clone();
            let shape = crate::collision::model_shape::Template::brush_model(map, model)?;
            let collider = shape.at(base, Quat::IDENTITY);
            objects.push(Object {
                shape,
                model,
                base,
                group: group(&name),
                name,
                collider,
                pose: Transform {
                    translation: base,
                    rotation: Quat::IDENTITY,
                },
            });
        }
        let mut essence_origins = [Vec3::ZERO; 4];
        for (i, n) in [1, 2, 3, 5].iter().enumerate() {
            essence_origins[i] = origin(map, &format!("mespawner{n}"))?;
        }
        let pickup_index = map
            .entities
            .iter()
            .position(|e| {
                e.get("classname")
                    .is_some_and(|c| c == "Item_WeaponPickup_JackBomb")
            })
            .context("Missing Jackbomb pickup")?;
        let mut s = Self {
            state: State {
                intro_camera: None,
                opponents: Default::default(),
                stage: Stage::Waiting,
                time: 0.,
                action: Action::Chase,
                clock: 0.,
                health: 600.,
                feet: origin(map, "duchessout")?,
                yaw: -std::f32::consts::FRAC_PI_2,
                shots: vec![],
                bursts: vec![],
                attacks: 0,
                dodges: 0,
                shell_returned: false,
                exit_sent: false,
                essence: [0.; 4],
                essence_taken: 0,
                initialized: false,
                started: false,
                cue: 0,
                wait: 1.,
                dodge_wait: 0.,
                pain: 0.,
                accumulator: 0.,
                home: Vec3::ZERO,
                notarget: false,
            },
            timing,
            objects,
            groups,
            points,
            pickup_id: format!("potears3:{pickup_index}"),
            essence_origins,
        };
        s.rebuild(map)?;
        Ok(s)
    }
    pub fn snapshot(&self) -> State {
        self.state.clone()
    }
    pub fn restore(&mut self, s: &State, map: &Bsp) -> Result<()> {
        ensure!(
            s.intro_camera.is_none_or(|(shot, time)| shot < 4
                && time.is_finite()
                && (0. ..=86400.).contains(&time))
                && s.health.is_finite()
                && (0. ..=600.).contains(&s.health)
                && s.feet.is_finite()
                && s.home.is_finite()
                && s.yaw.is_finite()
                && s.time.is_finite()
                && (0. ..=86400.).contains(&s.time)
                && s.clock.is_finite()
                && (0. ..=86400.).contains(&s.clock)
                && s.accumulator.is_finite()
                && s.accumulator.abs() < 0.01
                && s.cue <= 6
                && [s.wait, s.dodge_wait, s.pain]
                    .iter()
                    .all(|v| v.is_finite() && (0. ..=1000.).contains(v))
                && s.essence
                    .iter()
                    .all(|v| v.is_finite() && (0. ..=10.).contains(v))
                && s.shots.len() <= 64
                && s.bursts.len() <= 64
                && s.bursts
                    .iter()
                    .all(|b| b.position.is_finite() && (0. ..=1.).contains(&b.age)),
            "Invalid saved Duchess state"
        );
        let defeated = matches!(
            s.stage,
            Stage::Dying | Stage::Rescue | Stage::Farewell | Stage::Well | Stage::Complete
        );
        ensure!(
            defeated == (s.health == 0.)
                && (s.action == Action::Death) == defeated
                && s.shell_returned
                    == matches!(s.stage, Stage::Farewell | Stage::Well | Stage::Complete)
                && (!s.exit_sent || s.stage == Stage::Complete),
            "Inconsistent Duchess progression"
        );
        ensure!(
            s.shots.iter().all(|p| p.position.is_finite()
                && p.velocity.is_finite()
                && p.velocity.length() <= 751.
                && p.age.is_finite()
                && (0. ..=3.01).contains(&p.age)),
            "Invalid Duchess projectile"
        );
        self.state = s.clone();
        self.rebuild(map)
    }
    pub fn facts(&self) -> crate::event::Facts {
        let mut f = crate::event::Facts::default();
        f.flag("duchess.complete", self.state.stage == Stage::Complete);
        f.flag("duchess.pickup_only", false);
        f.flag("duchess.waiting", self.state.stage == Stage::Waiting);
        f
    }
    pub fn gate(name: &str, thread: &str, exit: bool) -> crate::event::Condition {
        if exit {
            crate::event::Condition::flag("duchess.complete")
        } else if name == "catstuff" {
            crate::event::Condition::flag("duchess.waiting")
        } else if thread == "expand_room" {
            crate::event::Condition::flag("duchess.pickup_only")
        } else {
            crate::event::Condition::Always
        }
    }
    pub fn controlled(&self) -> bool {
        self.cinematic()
            || (self.state.stage == Stage::Fighting
                && self.state.action == Action::Bite
                && self.state.clock < self.timing.frame(Action::Bite, 64.))
    }
    pub fn cinematic(&self) -> bool {
        !matches!(
            self.state.stage,
            Stage::Waiting | Stage::Fighting | Stage::Complete
        )
    }
    pub fn scene_id(&self) -> Option<&'static str> {
        match self.state.stage {
            Stage::Expanding | Stage::Introduction => Some(INTRO),
            Stage::Dying | Stage::Rescue | Stage::Farewell | Stage::Well => Some(OUTRO),
            _ => None,
        }
    }
    fn stage(&mut self, stage: Stage) {
        self.state.stage = stage;
        self.state.time = 0.;
    }
    fn action(&mut self, action: Action) {
        self.state.action = action;
        self.state.clock = 0.;
        self.state.cue = 0;
    }
    pub fn target(&self) -> Option<Target> {
        (self.state.stage == Stage::Fighting && self.state.action != Action::Phase).then_some(
            Target {
                id: ID,
                center: self.state.feet + Vec3::Z * 64.,
                half: vec3(32., 32., 64.),
            },
        )
    }
    pub fn hit(&mut self, damage: f32) -> Option<&'static str> {
        self.target()?;
        if !damage.is_finite() || damage <= 0. {
            return None;
        }
        self.state.health = (self.state.health - damage).max(0.);
        self.state.pain += damage;
        if self.state.health == 0. {
            self.action(Action::Death);
            self.stage(Stage::Dying);
            self.state.shots.clear();
            self.state.bursts.clear();
            Some("sound/character/duchess/death.wav")
        } else {
            if self.state.pain >= 50. && self.state.action != Action::Pain {
                self.state.pain = 0.;
                self.action(Action::Pain);
            }
            Some("sound/character/duchess/pain1.wav")
        }
    }
    pub fn threatened(&mut self, incoming: bool) {
        if incoming
            && self.state.stage == Stage::Fighting
            && self.state.dodge_wait <= 0.
            && matches!(
                self.state.action,
                Action::Chase | Action::Pepper | Action::Pig
            )
        {
            self.action(Action::Phase);
            self.state.dodge_wait = 5.;
            self.state.dodges += 1;
        }
    }
    pub fn group_offset(&self, g: usize) -> Vec3 {
        let (small, big) = self.groups[g];
        let f = match self.state.stage {
            Stage::Waiting => 0.,
            Stage::Expanding => (self.state.time / 3.).min(1.),
            _ => 1.,
        };
        small.lerp(big, f) - big
    }
    fn object_pose(&self, o: &Object) -> Transform {
        let mut p = o.base + o.group.map_or(Vec3::ZERO, |g| self.group_offset(g));
        let mut q = Quat::IDENTITY;
        match o.name.as_str() {
            "entrydoor" => {
                p.z += if self.state.stage == Stage::Waiting {
                    128.
                } else if self.state.stage == Stage::Expanding {
                    128. * (1. - (self.state.time / 0.5).min(1.))
                } else {
                    0.
                }
            }
            "fireplacebars" => {
                let a = if self.state.stage == Stage::Expanding {
                    (-120. * (self.state.time / 0.3).min(1.)).to_radians()
                } else {
                    0.
                };
                q = Quat::from_rotation_z(a);
            }
            "secretdoorobj" => {
                let f = match self.state.stage {
                    Stage::Rescue => (self.state.time / 1.).min(1.),
                    Stage::Farewell => 1.,
                    Stage::Well => 1. - (self.state.time / 1.).min(1.),
                    _ => 0.,
                };
                q = Quat::from_rotation_y(90_f32.to_radians() * f);
                let pivot = self.points["secretdoororigin"] + self.group_offset(2);
                p = pivot + q * (p - pivot);
            }
            "welldoor1" | "welldoor2" => {
                let f = match self.state.stage {
                    Stage::Well => ((self.state.time - 1.) / 1.).clamp(0., 1.),
                    Stage::Complete => 1.,
                    _ => 0.,
                };
                q = Quat::from_rotation_z(
                    90_f32.to_radians() * f * if o.name == "welldoor1" { 1. } else { -1. },
                );
            }
            "wellleverobj" => {
                let f = if matches!(self.state.stage, Stage::Well | Stage::Complete) {
                    1.
                } else {
                    0.
                };
                q = Quat::from_rotation_x((-30_f32 + 60. * f).to_radians());
                let pivot = self.points["wellleverorg"];
                p = pivot + q * (p - pivot);
            }
            _ => (),
        }
        Transform {
            translation: p,
            rotation: q,
        }
    }
    fn rebuild(&mut self, _map: &Bsp) -> Result<()> {
        for i in 0..self.objects.len() {
            let pose = self.object_pose(&self.objects[i]);
            let o = &mut self.objects[i];
            if o.pose.translation != pose.translation || o.pose.rotation != pose.rotation {
                o.collider = o.shape.at(pose.translation, pose.rotation);
                o.pose = pose;
            }
        }
        Ok(())
    }
    pub fn colliders(&self) -> impl Iterator<Item = Collider> + '_ {
        self.objects.iter().map(|o| o.collider.clone())
    }
    pub fn transforms(&self) -> impl Iterator<Item = (usize, Vec3, Quat)> + '_ {
        self.objects
            .iter()
            .map(|o| (o.model, o.pose.translation, o.pose.rotation))
    }
    pub fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
    ) -> Result<()> {
        if dt <= 0. || !dt.is_finite() {
            return Ok(());
        }
        if self.cinematic() {
            self.state.time += dt.min(0.1);
        }
        if self.state.stage == Stage::Introduction {
            if let Some((_, clock)) = &mut self.state.intro_camera {
                *clock += dt.min(0.1);
            }
        }
        if self.state.stage == Stage::Expanding && !self.state.started {
            self.state.started = true;
            player.feet = self.points["fuck_pos"];
            player.velocity = Vec3::ZERO;
            player.cancel_climb();
            player.release_rope();
            player.script_facing = std::f32::consts::FRAC_PI_2;
            self.state.home = player.feet;
        }
        match self.state.stage {
            Stage::Expanding if self.state.time >= 4.2 => self.stage(Stage::Introduction),
            Stage::Dying => {
                self.state.clock = self.state.time;
                if self.state.time >= 12_f32.max(self.timing.duration(Action::Death)) {
                    self.stage(Stage::Rescue);
                }
            }
            Stage::Rescue if self.state.time >= 13.5 => {
                self.state.shell_returned = true;
                self.stage(Stage::Farewell);
            }
            Stage::Well if self.state.time >= 5.5 => self.stage(Stage::Complete),
            _ => (),
        }
        self.rebuild(map)?;
        world.set_dynamic(self.colliders().collect());
        if self.cinematic() {
            player.velocity = Vec3::ZERO;
            player.cancel_climb();
            player.release_rope();
        }
        Ok(())
    }
    pub fn dialogue_complete(&mut self, name: &str) {
        if name == INTRO && self.state.stage == Stage::Introduction {
            self.stage(Stage::Fighting);
        }
        if name == OUTRO && self.state.stage == Stage::Farewell {
            self.stage(Stage::Well);
        }
    }
    pub fn skip(
        &mut self,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
        story: &mut Story,
    ) -> Result<bool> {
        let Some(id) = self.scene_id() else {
            return Ok(false);
        };
        story.trigger(id);
        story.finish_sequence(id);
        if id == INTRO {
            self.state.started = true;
            self.state.home = self.points["fuck_pos"];
            player.feet = self.state.home;
            player.script_facing = std::f32::consts::FRAC_PI_2;
            self.stage(Stage::Fighting);
        } else {
            self.state.shell_returned = true;
            self.state.clock = self.timing.duration(Action::Death);
            self.stage(Stage::Complete);
        }
        player.velocity = Vec3::ZERO;
        player.cancel_climb();
        player.release_rope();
        self.rebuild(map)?;
        world.set_dynamic(self.colliders().collect());
        Ok(true)
    }
    /// Shared by the live game and input-driven checks. Pickup identity, never mere weapon ownership, starts the encounter.
    pub fn update(
        &mut self,
        dt: f32,
        world: &World,
        player: &mut Player,
        stats: &mut Stats,
        story: &mut Story,
    ) -> Feedback {
        let mut out = Feedback::default();
        if dt <= 0. || !dt.is_finite() || !stats.alive() {
            return out;
        }
        if !self.state.initialized {
            stats.restore();
            self.state.initialized = true;
        }
        if self.state.stage == Stage::Waiting && stats.collected.contains(&self.pickup_id) {
            self.stage(Stage::Expanding);
            out.sounds.push("sound/ambience/special/roomsplit.wav");
        }
        match self.state.stage {
            Stage::Introduction => {
                story.trigger(INTRO);
                if let Some((line, elapsed)) = story.progress(INTRO) {
                    let shot = match line {
                        0 => 0,
                        1..=3 => 1,
                        4 => 2,
                        _ => 3,
                    };
                    if self
                        .state
                        .intro_camera
                        .is_none_or(|(previous, _)| previous != shot)
                    {
                        self.state.intro_camera = Some((
                            shot,
                            if shot == 0 {
                                4.2 + self.state.time
                            } else {
                                elapsed
                            },
                        ));
                    }
                }
            }
            Stage::Farewell => {
                story.trigger(OUTRO);
            }
            Stage::Complete if !self.state.exit_sent => {
                self.state.exit_sent = true;
                story.defer_exit(("utemple".into(), None));
            }
            _ => (),
        }
        if self.state.stage == Stage::Fighting && self.state.attacks == 0 && stats.selected() == 3 {
            stats.select(0);
        }
        if self.state.stage == Stage::Dying {
            let cues = [
                (31., "sound/character/duchess/sneeze1.wav"),
                (61., "sound/character/duchess/sneeze2.wav"),
                (85., "sound/character/duchess/sneeze3.wav"),
                (103., "sound/character/duchess/sneeze4.wav"),
                (145., "sound/character/duchess/deathfall1.wav"),
                (176., "sound/character/duchess/deathfall2.wav"),
            ];
            while self.state.cue < cues.len()
                && self.state.clock >= self.timing.frame(Action::Death, cues[self.state.cue].0)
            {
                out.spatial_sounds
                    .push((cues[self.state.cue].1, self.state.feet + Vec3::Z * 64.));
                self.state.cue += 1;
            }
        }
        if self.state.stage != Stage::Fighting || story.busy() {
            return out;
        }
        self.state.accumulator += dt.min(0.1) as f64;
        while self.state.accumulator + 1e-9 >= 1. / 120. {
            self.state.accumulator -= 1. / 120.;
            self.step(world, player, &mut out);
        }
        for i in 0..4 {
            self.state.essence[i] = (self.state.essence[i] - dt).max(0.);
            let pos = self.essence_origins[i];
            if self.state.essence[i] == 0.
                && (player.feet - pos).truncate().length() < 38.
                && (player.feet.z + 32. - pos.z).abs() < 80.
                && world.sweep(player.eye(), pos, Vec3::ZERO).fraction >= 1.
                && stats.apply(PickupKind::Essence, 25.)
            {
                self.state.essence[i] = 10.;
                self.state.essence_taken += 1;
            }
        }
        out
    }
    fn step(&mut self, world: &World, player: &mut Player, out: &mut Feedback) {
        self.state.clock += STEP;
        self.state.wait = (self.state.wait - STEP).max(0.);
        self.state.dodge_wait = (self.state.dodge_wait - STEP).max(0.);
        let bodies = self.state.opponents.bodies(player.eye());
        let (eye, notarget, victim) = self.state.opponents.aim(
            world,
            self.state.feet + Vec3::Z * 64.,
            player.eye(),
            self.state.notarget,
            self.state.action != Action::Chase,
        );
        let target_feet = if victim == crate::dice::SUMMON {
            eye - Vec3::Z * self.state.opponents.summon.unwrap().half.z
        } else {
            player.feet
        };
        for b in &mut self.state.bursts {
            b.age += STEP;
        }
        self.state.bursts.retain(|b| b.age < 1.);
        let mut impacts = Vec::new();
        self.state.shots.retain_mut(|s| {
            let next = s.position + s.velocity * STEP;
            if let Some((id, _)) = combat::contact(
                &combat::Context {
                    world,
                    targets: &bodies,
                },
                s.position,
                next,
                8.,
            ) {
                out.strike(
                    id,
                    if s.pig { 70. } else { 25. },
                    Vec3::ZERO,
                    combat::DamageKind::Other,
                );
                if s.pig {
                    impacts.push(s.position);
                }
                return false;
            }
            let t = world.sweep(s.position, next, Vec3::splat(8.));
            s.position = s.position.lerp(next, t.fraction);
            if s.pig && (t.start_solid || t.fraction < 1. || s.age + STEP >= 3.) {
                impacts.push(s.position + t.normal * 0.5);
            }
            s.age += STEP;
            t.fraction >= 1. && !t.start_solid && s.age < 3.
        });
        for pos in impacts {
            self.state.bursts.push(Burst {
                position: pos,
                age: 0.,
            });
            out.spatial_sounds
                .push(("sound/character/duchess/pig_explode.wav", pos));
            for target in &bodies {
                let delta = target.center - pos;
                let strength = (1. - delta.length() / 400.).max(0.);
                let sight = world.sweep(pos, target.center, Vec3::ZERO);
                if strength > 0. && !sight.start_solid && sight.fraction >= 1. {
                    out.strike(
                        target.id,
                        15. * strength,
                        delta.normalize_or_zero() * 400. * strength,
                        combat::DamageKind::Other,
                    );
                }
            }
        }
        if notarget {
            return;
        }
        if victim == crate::dice::SUMMON && matches!(self.state.action, Action::Grab | Action::Bite)
        {
            self.action(Action::Smack);
        }
        let delta = target_feet - self.state.feet;
        let distance = delta.truncate().length();
        let visible = world
            .sweep(self.state.feet + Vec3::Z * 64., eye, Vec3::splat(0.5))
            .fraction
            >= 1.;
        if self.state.action != Action::Phase {
            self.state.yaw = delta.y.atan2(delta.x);
        }
        match self.state.action {
            Action::Chase => {
                if visible && self.state.wait <= 0. {
                    let a = if distance < 110. {
                        if self.state.attacks % 2 == 0 && victim == crate::dice::ALICE {
                            Action::Grab
                        } else {
                            Action::Smack
                        }
                    } else if self.state.attacks % 3 == 2 {
                        Action::Pig
                    } else {
                        Action::Pepper
                    };
                    self.action(a);
                    self.state.attacks += 1;
                    out.spatial_sounds.push((
                        match a {
                            Action::Grab => "sound/character/duchess/attack_1.wav",
                            Action::Smack => "sound/character/duchess/attack_3.wav",
                            Action::Pig => "sound/character/duchess/attack_2.wav",
                            _ => "sound/character/duchess/attack_4.wav",
                        },
                        self.state.feet + Vec3::Z * 64.,
                    ));
                } else if distance > 90. {
                    self.walk(
                        world,
                        delta.truncate().normalize_or_zero(),
                        if distance > 350. { 190. } else { 100. },
                    );
                }
            }
            Action::Phase => {
                let side = vec2(-delta.y, delta.x).normalize_or_zero()
                    * if self.state.dodges % 2 == 0 { 1. } else { -1. };
                self.walk(world, side, 340.);
                if self.state.clock >= 0.85 {
                    self.action(Action::Chase);
                    self.state.wait = 0.3;
                }
            }
            Action::Pepper | Action::Pig => {
                let pig = self.state.action == Action::Pig;
                let when = self
                    .timing
                    .frame(self.state.action, if pig { 21. } else { 7. });
                if self.state.cue == 0 && self.state.clock >= when {
                    self.state.cue = 1;
                    if visible {
                        let p = self.state.feet
                            + vec3(self.state.yaw.cos() * 34., self.state.yaw.sin() * 34., 80.);
                        self.state.shots.push(Shot {
                            position: p,
                            velocity: (eye - Vec3::Z * 16. - p).normalize_or_zero()
                                * if pig { 600. } else { 750. },
                            age: 0.,
                            pig,
                        });
                    }
                }
                if self.state.clock >= self.timing.duration(self.state.action) {
                    self.action(Action::Chase);
                    self.state.wait = 0.9;
                }
            }
            Action::Smack => {
                if self.state.cue == 0 && self.state.clock >= self.timing.frame(Action::Smack, 9.) {
                    self.state.cue = 1;
                    if visible && distance < 125. && (delta.z).abs() < 80. {
                        out.strike(
                            victim,
                            15.,
                            delta.normalize_or_zero() * 170. + Vec3::Z * 80.,
                            combat::DamageKind::Other,
                        );
                    }
                }
                if self.state.clock >= self.timing.duration(Action::Smack) {
                    self.action(Action::Chase);
                    self.state.wait = 0.75;
                }
            }
            Action::Grab => {
                if self.state.clock >= self.timing.duration(Action::Grab) {
                    if distance < 120. && visible {
                        self.state.home = player.feet;
                        self.action(Action::Bite);
                    } else {
                        self.action(Action::Chase);
                        self.state.wait = 0.8;
                    }
                }
            }
            Action::Bite => {
                let front = vec3(self.state.yaw.cos(), self.state.yaw.sin(), 0.);
                let hold = self.state.feet + front * 65. + Vec3::Z * 35.;
                if self.state.clock < self.timing.frame(Action::Bite, 64.) && world.body_clear(hold)
                {
                    player.feet = hold;
                    player.velocity = Vec3::ZERO;
                    player.cancel_climb();
                    player.release_rope();
                }
                let frames = [34., 44., 54., 65.];
                while self.state.cue < 4
                    && self.state.clock >= self.timing.frame(Action::Bite, frames[self.state.cue])
                {
                    if self.state.cue < 3 {
                        out.damage += 5.;
                    } else {
                        player.velocity = Vec3::ZERO;
                        player.knockback(
                            front * 400. + Vec3::new(-front.y, front.x, 0.) * 100. + Vec3::Z * 250.,
                        );
                    }
                    self.state.cue += 1;
                }
                if self.state.clock >= self.timing.duration(Action::Bite) {
                    self.action(Action::Chase);
                    self.state.wait = 0.8;
                }
            }
            Action::Pain => {
                if self.state.clock >= self.timing.duration(Action::Pain) {
                    self.action(Action::Chase);
                    self.state.wait = 0.3;
                }
            }
            Action::Death => (),
        }
    }
    fn walk(&mut self, world: &World, wish: Vec2, speed: f32) {
        for turn in [0_f32, 0.6, -0.6, 1.2, -1.2] {
            let q = Quat::from_rotation_z(turn);
            let delta = q * wish.extend(0.) * speed * STEP;
            let c = self.state.feet + Vec3::Z * 64.1;
            let half = vec3(32., 32., 64.);
            let up = world.sweep(c, c + Vec3::Z * 18., half);
            let raised = c.lerp(c + Vec3::Z * 18., up.fraction);
            let across = world.sweep(raised, raised + delta, half);
            if up.start_solid || across.start_solid || across.fraction < 1. {
                continue;
            }
            let down = world.sweep(raised + delta, raised + delta - Vec3::Z * 44., half);
            if !down.start_solid && down.fraction < 1. && down.normal.z >= 0.65 {
                self.state.feet = (raised + delta)
                    .lerp(raised + delta - Vec3::Z * 44., down.fraction)
                    - Vec3::Z * 64.;
                break;
            }
        }
    }
    pub fn camera(&self, world: &World) -> Option<crate::cinematic::Camera> {
        if !self.cinematic() {
            return None;
        }
        // Follow the scene's own camera, never retract it from the boss's future
        // position through a wall that is still moving into place.
        if matches!(self.state.stage, Stage::Expanding | Stage::Introduction) {
            let (shot, time) = if self.state.stage == Stage::Expanding {
                (0, self.state.time)
            } else {
                self.state
                    .intro_camera
                    .unwrap_or((0, 4.2 + self.state.time))
            };
            let mut camera = self.timing.intro_tracks[shot].camera(time);
            // Keep the dialogue close-ups on the staged actors' upper bodies.
            // The port's standing poses differ from the original performance.
            if matches!(shot, 1 | 3) {
                camera.target = self.points["fuck_pos"] + Vec3::Z * 48.;
            } else if shot == 2 {
                camera.target = self.state.feet + Vec3::Z * 100.;
            }
            return Some(camera);
        }
        let (eye, target) = match self.state.stage {
            Stage::Dying => {
                let a = self.state.time * 0.15;
                let target = self.state.feet
                    + Quat::from_rotation_z(self.state.yaw) * self.timing.focus(self.state.clock);
                (
                    self.state.feet + vec3(a.sin() * 300., -a.cos() * 300., 140.),
                    target,
                )
            }
            Stage::Rescue => (vec3(80., 160., 145.), vec3(350., -200., 65.)),
            Stage::Farewell => (vec3(80., 140., 95.), vec3(304., 24., 60.)),
            _ => (vec3(80., 420., 210.), self.points["jumpinthewell"]),
        };
        let trace = world.sweep(target, eye, Vec3::splat(4.));
        let eye = if trace.start_solid {
            eye
        } else {
            target.lerp(eye, (trace.fraction - 0.01).clamp(0., 1.))
        };
        Some(crate::cinematic::Camera::look(eye, target))
    }
    pub fn recovery_entry(&self, normal: (Vec3, f32)) -> (Vec3, f32) {
        if self.state.stage == Stage::Fighting {
            (
                self.points["fuck_pos"] + Vec3::Z * 48.,
                std::f32::consts::FRAC_PI_2,
            )
        } else {
            normal
        }
    }
}

struct Decoration {
    name: String,
    group: Option<usize>,
    pose: Transform,
    scale: f32,
    prop: crate::weapons::Prop,
}
#[derive(Clone, Copy)]
struct TrailPose {
    at: f32,
    clock: f32,
    transform: Transform,
}
#[derive(Default)]
struct PhaseTrail {
    poses: std::collections::VecDeque<TrailPose>,
    previous: Option<TrailPose>,
    time: f32,
}
impl PhaseTrail {
    fn sample(&mut self, time: f32, phase: Option<(f32, Transform)>) {
        if time < self.time {
            self.poses.clear();
            self.previous = None;
        }
        self.time = time;
        self.poses.retain(|p| time - p.at < 1.);
        let Some((clock, transform)) = phase else {
            self.previous = None;
            return;
        };
        let now = TrailPose {
            at: time,
            clock,
            transform,
        };
        if let Some(old) = self
            .previous
            .filter(|p| clock >= p.clock && time - p.at < 0.2)
        {
            let first = (old.clock * 10. + 0.0001).floor() as u32 + 1;
            let last = (clock * 10. + 0.0001).floor() as u32;
            for n in first..=last {
                let at = n as f32 * 0.1;
                let f = ((at - old.clock) / (clock - old.clock)).clamp(0., 1.);
                self.poses.push_back(TrailPose {
                    at: old.at + (time - old.at) * f,
                    clock: at,
                    transform: Transform {
                        translation: old.transform.translation.lerp(transform.translation, f),
                        rotation: old.transform.rotation.slerp(transform.rotation, f),
                    },
                });
            }
        } else {
            self.poses.push_back(now);
        }
        while self.poses.len() > 10 {
            self.poses.pop_front();
        }
        self.previous = Some(now);
    }
}
pub struct Art {
    ui: std::rc::Rc<crate::ui::Ui>,
    duchess: crate::npc::Puppet,
    bill: crate::npc::Puppet,
    turtle: crate::npc::Puppet,
    pub(crate) alice: crate::npc::Puppet,
    props: Vec<Decoration>,
    pig: crate::weapons::Prop,
    essence: crate::loot_art::Art,
    material: crate::character::SkinMaterial,
    phase_model: crate::npc::Puppet,
    trail: PhaseTrail,
    effect_clock: Option<(Action, f32, u32)>,
    bursts: BTreeMap<String, crate::particles::Attached>,
    pepper: crate::particles::Attached,
    peppermill: crate::particles::Attached,
    gun: crate::weapons::Prop,
    pig_burst: crate::particles::Attached,
    baby: crate::particles::Attached,
}
impl Art {
    pub fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        let timing = Timing::load(a)?;
        let mut bursts = BTreeMap::new();
        for clip in ["attack_4", "death"] {
            bursts.insert(
                clip.into(),
                crate::particles::Attached::load_clip_bursts(
                    a,
                    "c_duchess",
                    Some(clip),
                    timing.clips[clip].1,
                    &specs,
                )?
                .with_context(|| format!("Duchess {clip} bursts missing"))?,
            );
        }
        // The effect timer has 100 frames over 5 seconds and no geometry/tags.
        let timer = 0.05;
        let duchess = crate::npc::Puppet::load(
            a,
            "c_duchess",
            &[
                "idle_1", "run", "run_away", "attack_1", "raise", "attack_2", "attack_3",
                "attack_4", "pain1", "death",
            ],
            &specs,
        )?;
        let bill = crate::npc::Puppet::load(a, "c_bill", &["walk", "talk_no", "idle"], &specs)?;
        let turtle =
            crate::npc::Puppet::load(a, "c_mockturtle", &["walk", "idle", "jump"], &specs)?;
        let alice = crate::npc::Puppet::load(a, "alice", &["idle_base_02", "jump"], &specs)?;
        let mut props = Vec::new();
        for e in &map.entities {
            let name = e.get("targetname").cloned().unwrap_or_default();
            let Some(model) = e.get("model").filter(|n| n.ends_with(".tik")) else {
                continue;
            };
            if !(name.ends_with("deco") || name == "turtleshellitem" || model == "table_button.tik")
            {
                continue;
            }
            if model.starts_with("fx_") {
                continue;
            }
            let Some(base) = e.get("origin").and_then(|s| crate::interaction::vector(s)) else {
                continue;
            };
            let angles = e
                .get("angles")
                .and_then(|s| crate::interaction::vector(s))
                .unwrap_or(vec3(
                    0.,
                    e.get("angle").and_then(|s| s.parse().ok()).unwrap_or(0.),
                    0.,
                ));
            let rotation = Quat::from_rotation_z(angles.y.to_radians())
                * Quat::from_rotation_y(angles.x.to_radians())
                * Quat::from_rotation_x(angles.z.to_radians());
            let prop = crate::weapons::Prop::load(
                a,
                model.trim_start_matches("models/").trim_end_matches(".tik"),
                &specs,
            )?;
            props.push(Decoration {
                name: name.clone(),
                group: group(&name),
                pose: Transform {
                    translation: base,
                    rotation,
                },
                scale: 1.,
                prop,
            });
        }
        let mut art = Self {
            ui: crate::ui::Ui::load(a)?,
            duchess,
            bill,
            turtle,
            alice,
            props,
            pig: crate::weapons::Prop::load(a, "prj_pig_baby", &specs)?,
            essence: crate::loot_art::Art::load(a, &specs)?,
            material: crate::character::skin_material()?,
            phase_model: crate::npc::Puppet::load(a, "fx_duchess_trail", &["run_away"], &specs)?,
            trail: PhaseTrail::default(),
            effect_clock: None,
            bursts,
            pepper: crate::particles::Attached::load(a, "prj_pepper", &specs)?
                .context("Pepper trail missing")?,
            peppermill: crate::particles::Attached::load(a, "w_peppermill", &specs)?
                .context("Peppermill effects missing")?,
            gun: crate::weapons::Prop::load(a, "w_peppermill", &specs)?,
            pig_burst: crate::particles::Attached::load_bursts(
                a,
                "fx_pigbaby_splode",
                timer,
                &specs,
            )?
            .context("Pig impact missing")?,
            baby: crate::particles::Attached::load_clip_bursts(
                a,
                "fx_duchess_babyspawn",
                Some("splash"),
                timer,
                &specs,
            )?
            .context("Pig creation missing")?,
        };
        // Keep the head visible through overlapping billboards. Only this actor's
        // sprites are tuned; native emission timing, placement and travel remain.
        art.duchess.tune_effects(0.4, 0.65);
        for effect in art.bursts.values_mut().chain([
            &mut art.pepper,
            &mut art.peppermill,
            &mut art.pig_burst,
            &mut art.baby,
        ]) {
            effect.resize(0.4);
            effect.attenuate(0.65);
        }
        Ok(art)
    }
    pub fn story_pose(&mut self, story: &crate::story::Story) {
        self.alice
            .mouth(story.mouth(&["fakeplayer", "player", "alice"]));
        self.duchess.mouth(story.mouth(&["duchess"]));
        self.bill.mouth(story.mouth(&["billthelizard"]));
        self.turtle.mouth(story.mouth(&["mockturtle"]));
    }
    pub fn draw(
        &mut self,
        d: &Duchess,
        fullbright: bool,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
    ) {
        self.material.atmosphere(atmosphere, camera);
        self.material.bind();
        for p in &mut self.props {
            if p.name == "turtleshellitem" && d.state.shell_returned {
                continue;
            }
            let mut pose = p.pose;
            pose.translation += p.group.map_or(Vec3::ZERO, |g| d.group_offset(g));
            if p.name == "secretdoordeco" {
                if let Some(o) = d.objects.iter().find(|o| o.name == "secretdoorobj") {
                    let pivot = d.points["secretdoororigin"] + d.group_offset(2);
                    pose.translation = pivot + o.pose.rotation * (pose.translation - pivot);
                    pose.rotation = o.pose.rotation * pose.rotation;
                }
            }
            p.prop.draw(pose, p.scale, fullbright);
        }
        if d.state.stage != Stage::Waiting {
            let (clip, time, looping) =
                if matches!(d.state.stage, Stage::Expanding | Stage::Introduction) {
                    ("idle_1", d.state.time, true)
                } else {
                    (
                        d.state.action.clip(),
                        d.state.clock,
                        d.state.action == Action::Chase || d.state.action == Action::Phase,
                    )
                };
            let grow = if d.state.stage == Stage::Expanding {
                ((d.state.time - 3.) / 1.1).clamp(0., 1.)
            } else {
                1.
            };
            let pos = if d.state.stage == Stage::Expanding {
                d.points["duchessin"].lerp(d.points["duchessout"], grow)
            } else {
                d.state.feet
            };
            self.duchess.show_attachments(
                d.state.action != Action::Death || time < d.timing.frame(Action::Death, 13.),
            );
            let hidden =
                if d.state.action == Action::Death && time >= d.timing.frame(Action::Death, 108.) {
                    &["head_top", "head_viel", "head_hat"][..]
                } else {
                    &["cap_head"][..]
                };
            // The phase body and its afterimages belong in the translucent pass.
            if d.state.action != Action::Phase {
                self.duchess.draw_filtered(
                    clip,
                    time,
                    looping,
                    Transform {
                        translation: pos,
                        rotation: Quat::from_rotation_z(d.state.yaw),
                    },
                    grow,
                    fullbright,
                    hidden,
                );
            }
        }
        if matches!(
            d.state.stage,
            Stage::Rescue | Stage::Farewell | Stage::Well | Stage::Complete
        ) {
            for (actor, start, goal) in [
                (&mut self.bill, "billthelizard", "billstand"),
                (&mut self.turtle, "mockturtle", "turtlestand"),
            ] {
                let f = if d.state.stage == Stage::Rescue {
                    ((d.state.time - 1.5) / 10.).clamp(0., 1.)
                } else {
                    1.
                };
                let mut pos = d.points[start].lerp(d.points[goal], f);
                if goal == "turtlestand" && matches!(d.state.stage, Stage::Well | Stage::Complete) {
                    let t = ((d.state.time - 2.) / 1.).clamp(0., 1.);
                    if d.state.stage == Stage::Complete || t >= 1. {
                        continue;
                    }
                    pos = d.points["turtle_jump_pos"].lerp(vec3(320., 224., -120.), t)
                        + Vec3::Z * (120. * (std::f32::consts::PI * t).sin());
                }
                if goal == "turtlestand" {
                    actor.show_attachments(d.state.shell_returned);
                }
                actor.draw(
                    if f < 1. { "walk" } else { "idle" },
                    d.state.time,
                    true,
                    Transform {
                        translation: pos,
                        rotation: Quat::from_rotation_z(2.4),
                    },
                    1.,
                    fullbright,
                );
            }
        }
        if d.cinematic() {
            let mut p = d.points["fuck_pos"];
            if matches!(d.state.stage, Stage::Rescue | Stage::Farewell | Stage::Well) {
                p = d.points["alice_end_pos"];
                if d.state.stage == Stage::Well && d.state.time > 3.5 {
                    let t = ((d.state.time - 3.5) / 1.5).min(1.);
                    p = d.points["turtle_jump_pos"].lerp(vec3(320., 224., -120.), t)
                        + Vec3::Z * (100. * (std::f32::consts::PI * t).sin());
                }
            }
            self.alice.draw(
                "idle_base_02",
                d.state.time,
                true,
                Transform {
                    translation: p,
                    rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                },
                1.,
                fullbright,
            );
        }
        if d.state.stage == Stage::Fighting {
            for (i, &pos) in d.essence_origins.iter().enumerate() {
                if d.state.essence[i] == 0. {
                    self.essence.draw(
                        crate::loot::Grade::Medium,
                        pos,
                        atmosphere,
                        camera,
                        d.state.clock,
                    );
                }
            }
        }
        for s in &d.state.shots {
            if s.pig {
                self.pig.draw(
                    Transform {
                        translation: s.position,
                        rotation: Quat::from_rotation_z(s.velocity.y.atan2(s.velocity.x))
                            * Quat::from_rotation_x(s.age * 6.),
                    },
                    1.,
                    fullbright,
                );
            }
        }
        gl_use_default_material();
    }
    pub fn effects(
        &mut self,
        d: &Duchess,
        camera: Vec3,
        atmosphere: &crate::environment::Atmosphere,
    ) {
        let active = matches!(d.state.stage, Stage::Fighting | Stage::Dying);
        let phase = active && d.state.action == Action::Phase;
        let pose = Transform {
            translation: d.state.feet,
            rotation: Quat::from_rotation_z(d.state.yaw),
        };
        self.trail
            .sample(d.state.time, phase.then_some((d.state.clock, pose)));
        if active {
            if self.effect_clock.is_none_or(|(action, clock, attacks)| {
                action != d.state.action || d.state.clock < clock || attacks != d.state.attacks
            }) {
                self.duchess.reset_effects();
                self.peppermill = self.peppermill.fork();
            }
            self.effect_clock = Some((d.state.action, d.state.clock, d.state.attacks));
            let clip = d.state.action.clip();
            self.duchess.draw_effects(
                clip,
                d.state.clock,
                matches!(d.state.action, Action::Chase | Action::Phase),
                pose,
                1.,
                camera,
                atmosphere,
            );
            if let Some(burst) = self.bursts.get(clip) {
                burst.fork().draw(
                    d.state.clock,
                    1.,
                    |at, tag| {
                        tag.and_then(|tag| self.duchess.tag(tag, clip, at, pose, 1.))
                            .unwrap_or(pose)
                    },
                    |_, _, _| true,
                    camera,
                    atmosphere,
                );
            }
            if d.state.action == Action::Pig {
                let start = d.timing.frame(Action::Pig, 1.);
                let end = d.timing.frame(Action::Pig, 18.);
                if d.state.clock >= start && d.state.clock < end + 0.6 {
                    self.baby.fork().draw(
                        d.state.clock - start,
                        1.,
                        |at, _| {
                            self.duchess
                                .tag("tag_pigbaby", clip, start + at, pose, 1.)
                                .unwrap_or(pose)
                        },
                        |_, at, _| start + at < end,
                        camera,
                        atmosphere,
                    );
                }
            }
            // Oldest silhouettes first; a continuous current silhouette replaces blinking.
            for p in &self.trail.poses {
                let opacity = (1. - (d.state.time - p.at)).clamp(0., 1.) * 0.22;
                self.phase_model
                    .draw_afterimage("run_away", p.clock, p.transform, opacity);
            }
            if phase {
                self.phase_model
                    .draw_afterimage("run_away", d.state.clock, pose, 0.65);
                if let Some(grip) =
                    self.duchess
                        .looping_tag("tag_weapon", "run_away", d.state.clock, pose, 1.)
                {
                    for mesh in self.gun.meshes_at(grip, 1., true, 0., false) {
                        for vertex in &mut mesh.vertices {
                            vertex.color = [32, 32, 32, 166];
                        }
                        crate::render_fx::effect(mesh, crate::materials::Blend::Alpha);
                    }
                }
                self.peppermill.draw(
                    d.state.clock,
                    1.,
                    |at, tag| {
                        let mut muzzle = self
                            .duchess
                            .looping_tag("tag_weapon", "run_away", at, pose, 1.)
                            .unwrap_or(pose);
                        if let Some(tag) = tag {
                            muzzle.translation = self.gun.point(muzzle, tag, 1.);
                        }
                        muzzle
                    },
                    |name, _, _| name == "pepperamb2",
                    camera,
                    atmosphere,
                );
            }
        } else {
            self.effect_clock = None;
            self.trail = PhaseTrail::default();
        }
        for b in &d.state.bursts {
            self.pig_burst.fork().draw(
                b.age,
                1.,
                |_, _| Transform {
                    translation: b.position,
                    rotation: Quat::IDENTITY,
                },
                |_, _, _| true,
                camera,
                atmosphere,
            );
        }
        for shot in d.state.shots.iter().filter(|s| !s.pig) {
            self.pepper.fork().draw(
                shot.age,
                1.,
                |at, _| Transform {
                    translation: shot.position - shot.velocity * (shot.age - at),
                    rotation: Quat::from_rotation_z(shot.velocity.y.atan2(shot.velocity.x)),
                },
                |_, _, default| default,
                camera,
                atmosphere,
            );
        }
        if d.state.stage == Stage::Expanding {
            for i in 0..20 {
                let t = (d.state.time + i as f32 * 0.13) % 1.;
                crate::render_fx::sphere(
                    vec3((i as f32 * 3.).sin() * 25., 930., 40. + t * 180.) + d.group_offset(0),
                    3. + t * 6.,
                    None,
                    Color::new(1., 0.35, 0.08, 1. - t),
                );
            }
        }
        gl_use_default_material();
    }
    pub fn hud(&self, d: &Duchess) {
        if d.state.stage == Stage::Fighting {
            let w = 240.;
            let x = (screen_width() - w) * 0.5;
            self.ui.dialog(Rect::new(x - 22., 12., w + 44., 48.));
            draw_rectangle(
                x,
                39.,
                w * d.state.health / 600.,
                7.,
                Color::new(0.65, 0.16, 0.21, 0.9),
            );
            self.ui.label(
                if d.state.action == Action::Phase {
                    "DUCHESS / PHASING"
                } else {
                    "DUCHESS"
                },
                x,
                30.,
                18.,
                WHITE,
            );
        }
    }
}

impl Duchess {
    pub fn fixture(
        &mut self,
        case: &str,
        map: &Bsp,
        world: &mut World,
        p: &mut Player,
        stats: &mut Stats,
        story: &mut Story,
    ) -> Result<()> {
        self.state.initialized = true;
        self.state.started = true;
        stats.collected.insert(self.pickup_id.clone());
        stats.apply(PickupKind::Weapon(3), 1.);
        stats.select(0);
        self.state.home = self.points["fuck_pos"];
        *p = Player::new(self.state.home);
        p.script_facing = std::f32::consts::FRAC_PI_2;
        self.stage(Stage::Fighting);
        self.state.health = 425.;
        match case {
            "duchess-expand" => {
                self.stage(Stage::Expanding);
                self.state.time = 1.5;
                self.state.health = 600.;
            }
            "duchess-intro" => {
                self.stage(Stage::Introduction);
                self.state.health = 600.;
                story.trigger(INTRO);
                story.tick(0.4, false);
            }
            "duchess-phase" => {
                self.threatened(true);
                self.state.clock = 0.4;
            }
            "duchess-pepper" | "duchess-pig" => {
                let pig = case == "duchess-pig";
                self.action(if pig { Action::Pig } else { Action::Pepper });
                self.state.clock = self
                    .timing
                    .frame(self.state.action, if pig { 22. } else { 8. });
                self.state.cue = 1;
                self.state.shots.push(Shot {
                    position: self.state.feet + vec3(0., -150., 70.),
                    velocity: Vec3::NEG_Y * if pig { 600. } else { 750. },
                    age: 0.2,
                    pig,
                });
            }
            "duchess-bite" => {
                self.action(Action::Bite);
                self.state.clock = self.timing.frame(Action::Bite, 40.);
                self.state.cue = 1;
                p.feet = self.state.feet + vec3(0., -65., 35.);
            }
            "duchess-death" | "duchess-rescue" | "duchess-reward" | "duchess-complete" => {
                self.hit(1000.);
                self.state.clock = 5.7;
                self.state.time = 5.7;
                if case == "duchess-rescue" {
                    self.stage(Stage::Rescue);
                    self.state.time = 7.;
                }
                if case == "duchess-reward" {
                    self.state.shell_returned = true;
                    self.stage(Stage::Farewell);
                    story.trigger(OUTRO);
                    story.tick(0.4, false);
                }
                if case == "duchess-complete" {
                    self.state.shell_returned = true;
                    self.stage(Stage::Complete);
                }
            }
            _ => (),
        }
        self.rebuild(map)?;
        world.set_dynamic(self.colliders().collect());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phase_trail_tracks_motion_at_fixed_intervals_and_expires() {
        let mut reference: Option<Vec<Vec3>> = None;
        for fps in [30, 60, 144] {
            let mut trail = PhaseTrail::default();
            let count = (fps as f32 * 0.8).ceil() as usize;
            for i in 0..=count {
                let time = (i as f32 / fps as f32).min(0.8);
                trail.sample(
                    time,
                    Some((
                        time,
                        Transform {
                            translation: Vec3::X * time * 340.,
                            rotation: Quat::IDENTITY,
                        },
                    )),
                );
            }
            let positions: Vec<_> = trail
                .poses
                .iter()
                .map(|p| p.transform.translation)
                .collect();
            assert_eq!(positions.len(), 9);
            if let Some(reference) = &reference {
                assert!(positions
                    .iter()
                    .zip(reference)
                    .all(|(a, b)| a.distance(*b) < 0.001));
            } else {
                reference = Some(positions);
            }
            let last = trail.previous.unwrap();
            trail.sample(0.8, Some((last.clock, last.transform)));
            assert_eq!(trail.poses.len(), 9, "paused rendering added a silhouette");
            trail.sample(1.81, None);
            assert!(trail.poses.is_empty());
            trail.sample(0., Some((0., last.transform)));
            assert_eq!(trail.poses.len(), 1, "rewind retained old silhouettes");
        }
    }
    #[test]
    fn boss_attacks_demons_and_never_grabs_alice_on_their_behalf() {
        let world = floor();
        let mut d = boss();
        let mut p = Player::new(vec3(-900., 0., 0.));
        d.state.opponents.summon = Some(Target {
            id: crate::dice::SUMMON,
            center: d.state.feet + vec3(90., 0., 60.),
            half: Vec3::splat(40.),
        });
        d.state.opponents.demon = true;
        d.state.notarget = true;
        let mut damage = 0.;
        let before = p.feet;
        for _ in 0..120 * 6 {
            let mut f = Feedback::default();
            d.step(&world, &mut p, &mut f);
            assert_eq!(f.damage, 0.);
            assert_eq!(p.feet, before);
            damage += f.summon_hits.iter().map(|h| h.damage).sum::<f32>();
        }
        assert!(damage > 0.);
        assert!(!matches!(d.state.action, Action::Grab | Action::Bite));
    }
    fn boss() -> Duchess {
        let clips = [
            "idle_1", "run", "run_away", "attack_1", "raise", "attack_2", "attack_3", "attack_4",
            "pain1", "death",
        ]
        .into_iter()
        .map(|n| (n.into(), (if n == "raise" { 4. } else { 1.5 }, 0.05)))
        .collect();
        Duchess {
            state: State {
                intro_camera: None,
                opponents: Default::default(),
                stage: Stage::Fighting,
                time: 0.,
                action: Action::Chase,
                clock: 0.,
                health: 600.,
                feet: Vec3::ZERO,
                yaw: 0.,
                shots: vec![],
                bursts: vec![],
                attacks: 0,
                dodges: 0,
                shell_returned: false,
                exit_sent: false,
                essence: [10.; 4],
                essence_taken: 0,
                initialized: true,
                started: true,
                cue: 0,
                wait: 0.,
                dodge_wait: 0.,
                pain: 0.,
                accumulator: 0.,
                home: Vec3::ZERO,
                notarget: false,
            },
            timing: Timing {
                intro_tracks: vec![],
                clips,
                death_focus: vec![Vec3::Z * 100.],
            },
            objects: vec![],
            groups: vec![],
            points: BTreeMap::new(),
            pickup_id: "potears3:21".into(),
            essence_origins: [vec3(1000., 1000., 0.); 4],
        }
    }
    fn floor() -> World {
        World::fixture(&[(vec3(-1000., -1000., -20.), vec3(1000., 1000., 0.))])
    }
    #[test]
    fn phase_is_temporary_and_pain_cannot_stunlock_every_hit() {
        let mut d = boss();
        d.threatened(true);
        assert!(d.target().is_none());
        assert!(d.hit(100.).is_none());
        assert_eq!(d.state.health, 600.);
        let mut p = Player::new(vec3(200., 0., 0.));
        let mut f = Feedback::default();
        for _ in 0..110 {
            d.step(&floor(), &mut p, &mut f);
        }
        assert!(d.target().is_some());
        d.hit(20.);
        assert_ne!(d.state.action, Action::Pain);
        d.hit(35.);
        assert_eq!(d.state.action, Action::Pain);
        d.hit(1000.);
        assert_eq!(d.state.stage, Stage::Dying);
        assert!(d.target().is_none());
        assert!(d.hit(100.).is_none());
        assert!(d.state.shots.is_empty());
    }
    #[test]
    fn melee_hits_once_and_solid_walls_stop_attacks() {
        for wall in [false, true] {
            let world = if wall {
                World::fixture(&[(vec3(30., -100., -20.), vec3(40., 100., 200.))])
            } else {
                floor()
            };
            let mut d = boss();
            d.action(Action::Smack);
            let mut p = Player::new(vec3(90., 0., 0.));
            let mut f = Feedback::default();
            for _ in 0..160 {
                d.step(&world, &mut p, &mut f);
            }
            assert_eq!(f.damage, if wall { 0. } else { 15. });
        }
    }
    #[test]
    fn pig_and_pepper_use_swept_contacts_and_original_direct_damage() {
        for pig in [false, true] {
            let mut d = boss();
            d.state.notarget = true;
            d.state.shots.push(Shot {
                position: vec3(0., 0., 28.),
                velocity: Vec3::X * if pig { 600. } else { 750. },
                age: 0.,
                pig,
            });
            let mut p = Player::new(vec3(60., 0., 0.));
            let mut f = Feedback::default();
            for _ in 0..30 {
                d.step(&floor(), &mut p, &mut f);
            }
            if pig {
                assert!(f.damage > 70. && f.damage <= 85.);
            } else {
                assert_eq!(f.damage, 25.);
            }
            assert!(d.state.shots.is_empty());
        }
    }
    #[test]
    fn interrupted_grab_releases_control_and_toss_resumes_physics() {
        let mut d = boss();
        d.action(Action::Bite);
        assert!(d.controlled());
        d.hit(51.);
        assert!(!d.controlled());
        d.action(Action::Bite);
        d.state.clock = 3.3;
        assert!(!d.controlled());
    }
    #[test]
    fn attack_resume_keeps_cues_projectiles_and_damage() {
        let mut d = boss();
        d.action(Action::Pig);
        let mut p = Player::new(vec3(350., 0., 0.));
        let mut f = Feedback::default();
        for _ in 0..135 {
            d.step(&floor(), &mut p, &mut f);
        }
        let bytes = serde_json::to_vec(&d.state).unwrap();
        let mut resumed = boss();
        resumed.state = serde_json::from_slice(&bytes).unwrap();
        let mut p2 = p.clone();
        let mut f1 = Feedback::default();
        let mut f2 = Feedback::default();
        for _ in 0..360 {
            d.step(&floor(), &mut p, &mut f1);
            resumed.step(&floor(), &mut p2, &mut f2);
        }
        assert_eq!(f1.damage, f2.damage);
        assert_eq!(
            serde_json::to_value(d.state).unwrap(),
            serde_json::to_value(resumed.state).unwrap()
        );
    }
}
