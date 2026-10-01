//! Reviewed village scene choreography. Asset identifiers and measurements only;
//! no original script source or executable commands are embedded here.
use crate::{
    assets::Assets,
    bsp::Bsp,
    cinematic::{Camera, Track},
    collision::World,
    movement::Player,
    npc::Puppet,
    skeletal::Transform,
    story::Story,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod gnome3;
pub mod visibility;

const GNOMES: [&str; 4] = [
    "Torchgnome1_Dialog",
    "Torchgnome2_Dialog",
    "Torchgnome3_Dialog_part2",
    "Torchgnome4_Dialog",
];
const CAST: [&str; 4] = ["torchgnome2", "torchgnome1", "torchgnome3", "torchgnome4"];
const ALICE: &[&str] = &[
    "death_falling2_silent",
    "jump_falling2",
    "pain_knockdown",
    "ready",
    "knife_idle2",
    "idle_base_03",
    "idle_base_01",
    "idle_base_01_play2",
    "idle_base_01_2_base_02",
    "idle_base_02",
    "idle_base_02_2_shrug",
    "walk",
    "idle_stand",
    "idle_stand_rocktoes",
    "idle_stand_nodyes",
    "idle_stand_shakeno",
    "idle_base_02_2_stand",
    "idle_shrug",
    "idle_shrug_headtilt",
    "idle_shrug_tap",
    "talk_gnomes_01",
    "talk_gnomes_03",
];
const TORCH: &[&str] = &["idle", "run", "talkbegin", "talk1", "talk2"];
const CAT: &[&str] = &[
    "sit_idle1",
    "sit_idle2",
    "sit_talk1",
    "sit_talk3",
    "sit_smile_open",
    "sit_smile_shut",
    "sit_2walk",
    "walk",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Beat {
    Knife,
    KnifeEnd,
    Fall,
    Rabbit,
    Cat,
    IntroEnd,
    Gnome(u8),
    Essence,
    End(u8),
    Retreat,
    Shrink,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(default, skip_serializing_if = "is_false")]
    legacy: bool,
    pub beat: Option<Beat>,
    pub time: f32,
    pub line: usize,
    pub line_time: f32,
    pub shot_time: f32,
    pub done: BTreeSet<String>,
    pub home: Option<Vec3>,
}
fn is_false(v: &bool) -> bool {
    !v
}
impl State {
    fn start(&mut self, beat: Beat) {
        self.beat = Some(beat);
        self.time = 0.;
        self.shot_time = 0.;
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            [self.time, self.line_time, self.shot_time]
                .iter()
                .all(|v| v.is_finite() && (0. ..=1e6).contains(v))
                && self.line < 32
                && self
                    .home
                    .is_none_or(|p| p.is_finite() && p.abs().max_element() < 100000.)
                && !matches!(self.beat, Some(Beat::Gnome(4..) | Beat::End(4..)))
                && self
                    .done
                    .iter()
                    .all(|s| s == "entry" || s == "knife_cat" || GNOMES.contains(&s.as_str())),
            "Invalid village cinematic state"
        );
        Ok(())
    }
}
pub struct Cinema {
    knife_id: usize,
    pub state: State,
    tracks: BTreeMap<&'static str, Track>,
    points: BTreeMap<String, Transform>,
    durations: BTreeMap<String, f32>,
    speeds: BTreeMap<String, f32>,
    fall_start: Vec3,
    cat_start: Vec3,
    gnome3_lines: [f32; 6],
}
fn fraction(t: f32, d: f32) -> f32 {
    (t / d).clamp(0., 1.)
}
fn pose(at: Vec3, yaw: f32) -> Transform {
    Transform {
        translation: at,
        rotation: Quat::from_rotation_z(yaw.to_radians()),
    }
}
impl Cinema {
    pub fn load(assets: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut tracks = BTreeMap::new();
        for name in [
            "gvillage_start_p1",
            "gvillage_start_p2",
            "gvillage_start_p3",
            "gvillage_start_p4",
            "gvillage_tg1_path1",
            "gvillage_tg2p1",
            "gvillage_tg2p2",
            "gvillage_tg3_p1",
            "gvillage_jdm2",
            "gvillage_jdm3",
            "gvillage_shrink_p1",
            "gvillage_cat2_p1",
            "gvillage_tg4_p1",
            "gvillage_metap2",
        ] {
            tracks.insert(name, Track::load(assets, name)?);
        }
        let world = World::from_bsp(map)?;
        let mut points = BTreeMap::new();
        for e in &map.entities {
            if let (Some(name), Some(at)) = (
                e.get("targetname"),
                e.get("origin").and_then(|s| crate::interaction::vector(s)),
            ) {
                // Authored path nodes sit above support. Resolve their
                // feet with collision rather than showing actors hovering at those nodes.
                let ground = world
                    .actor_footing(at, vec3(0., 0., 24.), vec3(8., 8., 24.), 80.)
                    .unwrap_or(at);
                points.insert(
                    name.clone(),
                    pose(
                        ground,
                        e.get("angle").and_then(|s| s.parse().ok()).unwrap_or(0.),
                    ),
                );
            }
        }
        for name in [
            "alice_knife_start",
            "alice_knife_node",
            "knife_cat",
            "rabbit_start_shrink2",
            "alice_cat_talk1",
            "alice_cat_talk2",
            "rabbit_pos1",
            "rabbit_pos2",
            "rabbit_pos3",
            "rabbit_start_shrink",
            "rabbit_shrink_pos1",
            "cat_start_pos1",
            "alice_torch2_talk1",
            "torchgnome2_talk_pos1",
            "torchgnome2_pos1",
            "alice_talk_posx2",
            "torchgnome1_talk",
            "alice_tg3_talk",
            "alice_tg4_cat2",
            "torchgnome3_talk",
            "alice_tg4_talk",
            "torchgnome4_talk",
            "cat_tg3_talk",
            "climb_cat_node",
            "exit_cat_node",
            "essence_cat",
        ] {
            ensure!(
                points.contains_key(name),
                "Missing village scene marker {name}"
            );
        }
        let mut durations = BTreeMap::new();
        let mut speeds = BTreeMap::new();
        for (model, clips) in [
            ("alice", ALICE),
            ("c_torchgnome", TORCH),
            ("c_cheshire", CAT),
            ("c_whiterabbit", &["idle", "run", "walk"][..]),
        ] {
            let def = crate::skeletal::Definition::load(assets, &format!("models/{model}.tik"))?;
            let rig = crate::skeletal::Skeleton::parse(
                &assets.read(&format!("{}/{}", def.path, def.model))?,
            )?;
            for clip in clips {
                let file = def
                    .animations
                    .get(*clip)
                    .with_context(|| format!("Missing {model}/{clip}"))?;
                let a = crate::skeletal::Animation::parse(
                    &assets.read(&format!("{}/{file}", def.path))?,
                    rig.bones.len(),
                )?;
                durations.insert(format!("{model}/{clip}"), a.duration());
                speeds.insert(
                    format!("{model}/{clip}"),
                    a.distance / a.duration() * def.scale,
                );
            }
        }
        let fall_start = map
            .entities
            .iter()
            .find(|e| {
                e.get("thread")
                    .is_some_and(|s| s == "Gvillage_RabbitHole_Start")
            })
            .and_then(|e| e.get("origin"))
            .and_then(|s| crate::interaction::vector(s))
            .context("Missing falling entrance")?;
        // The Cat begins at the lower doorway, before walking toward Alice.
        let cat_start = points["cat_hole1"].translation;
        Ok(Self {
            knife_id: map
                .entities
                .iter()
                .position(|e| e.get("pickup_thread").is_some_and(|s| s == "knife_cat"))
                .context("Missing Blade pickup scene")?,
            state: State::default(),
            tracks,
            points,
            durations,
            speeds,
            fall_start,
            cat_start,
            gnome3_lines: gnome3::line_starts(assets)?,
        })
    }
    pub fn restore(&mut self, state: Option<&State>) -> Result<()> {
        if let Some(s) = state {
            s.validate()?;
            self.state = s.clone();
        } else {
            // Older saves began after the introduction and kept conversations free.
            // Do not seize control halfway through an already saved conversation.
            self.state = State::default();
            self.state.done.insert("entry".into());
            self.state.legacy = true;
        }
        Ok(())
    }
    fn at(&self, name: &str) -> Vec3 {
        self.points[name].translation
    }
    fn duration(&self, model: &str, clip: &str) -> f32 {
        self.durations[&format!("{model}/{clip}")]
    }
    pub fn sync_pickups(&mut self, stats: &crate::inventory::Stats, story: &mut Story) {
        if self.state.legacy
            && stats
                .collected
                .contains(&format!("gvillage:{}", self.knife_id))
        {
            self.state.done.insert("knife_cat".into());
        }
        if !self.state.legacy
            && !self.active()
            && !story.busy()
            && !self.state.done.contains("knife_cat")
            && stats
                .collected
                .contains(&format!("gvillage:{}", self.knife_id))
        {
            self.state.home = None;
            self.state.line = 0;
            self.state.line_time = 0.;
            self.state.start(Beat::Knife);
            story.trigger("knife_cat");
        }
    }
    pub fn begin(&mut self, story: &mut Story) {
        if !self.state.done.contains("entry") {
            self.state.start(Beat::Fall);
            story.trigger("entry");
        }
    }
    pub fn active(&self) -> bool {
        self.state.beat.is_some()
    }
    pub fn scene_id(&self) -> Option<&'static str> {
        match self.state.beat? {
            Beat::Gnome(i) | Beat::End(i) => Some(GNOMES[i as usize]),
            Beat::Essence => Some(GNOMES[1]),
            Beat::Retreat => Some(GNOMES[0]),
            Beat::Knife | Beat::KnifeEnd | Beat::Shrink => Some("knife_cat"),
            _ => Some("entry"),
        }
    }
    fn cat_prelude(&self) -> f32 {
        2. + self.duration("c_cheshire", "sit_smile_open")
            + self.duration("c_cheshire", "sit_smile_shut")
            + self.duration("c_cheshire", "sit_2walk")
            + 1.6
    }
    fn acting(
        &self,
        model: &str,
        mut time: f32,
        steps: &[(&'static str, Option<f32>)],
    ) -> (&'static str, f32, bool) {
        for &(clip, hold) in steps {
            let duration = hold.unwrap_or_else(|| self.duration(model, clip));
            if time < duration {
                return (clip, time, hold.is_some());
            }
            time -= duration;
        }
        (
            if model == "alice" {
                "idle_stand"
            } else {
                "idle"
            },
            time,
            true,
        )
    }
    pub fn prepare_story(&self, story: &mut Story) -> bool {
        story.line_limit = if matches!(self.state.beat, Some(Beat::Fall | Beat::Rabbit)) {
            Some(0)
        } else if self.state.beat == Some(Beat::Gnome(1)) {
            Some(3)
        } else if self.state.beat == Some(Beat::Essence) && self.state.time < 11.5 {
            Some(4)
        } else {
            None
        };
        match self.state.beat {
            Some(Beat::Fall) => self.state.time >= 9.,
            Some(Beat::Rabbit) => false,
            Some(Beat::Knife) => self.state.time >= 2.,
            Some(Beat::Cat) => self.state.time >= self.cat_prelude(),
            Some(Beat::Essence) => self.state.time >= 0.5,
            _ => true,
        }
    }
    pub fn sync_story(&mut self, story: &Story) {
        if self.state.legacy {
            self.state.done.extend(
                GNOMES
                    .into_iter()
                    .chain(std::iter::once("knife_cat"))
                    .filter(|id| story.has_seen(id))
                    .map(str::to_owned),
            );
            self.state.legacy = false;
        }

        if self.state.beat.is_none() {
            for (i, id) in GNOMES.iter().enumerate() {
                if !self.state.done.contains(*id) && story.progress(id).is_some() {
                    self.state.home = None;
                    self.state.line = 0;
                    self.state.line_time = 0.;
                    self.state.start(Beat::Gnome(i as u8));
                    break;
                }
            }
        }
        if let Some(id) = self.scene_id() {
            if let Some((line, time)) = story.progress(id) {
                let shot = |line: usize| match self.state.beat {
                    Some(Beat::Gnome(1)) => usize::from(line > 0),
                    Some(Beat::Gnome(2)) => usize::from(line >= 3) + usize::from(line >= 6),
                    _ => 0,
                };
                if shot(line) != shot(self.state.line) {
                    self.state.shot_time = 0.;
                }
                self.state.line = line;
                self.state.line_time = time;
            }
        }
        if self.state.beat == Some(Beat::Fall) && self.state.time >= 9. && story.line_finished() {
            self.state.start(Beat::Rabbit);
        }
        if self.state.beat == Some(Beat::Gnome(1)) && self.state.line == 3 && story.line_finished() {
            self.state.start(Beat::Essence);
        }
    }
    pub fn completed(&mut self, id: &str) {
        if self.scene_id() != Some(id) {
            return;
        }
        match self.state.beat {
            Some(Beat::Cat) => self.state.start(Beat::IntroEnd),
            Some(Beat::Gnome(0)) => self.state.start(Beat::Retreat),
            Some(Beat::Knife) => self.state.start(Beat::KnifeEnd),
            Some(Beat::Essence) => self.state.start(Beat::End(1)),
            Some(Beat::Gnome(i)) => self.state.start(Beat::End(i)),
            _ => {}
        }
    }
    fn finish(&mut self, player: &mut Player, world: &World) -> Result<()> {
        let Some(id) = self.scene_id() else {
            return Ok(());
        };
        // Watched and skipped scenes share the final authored Alice marker.
        // Returning to `home` undid visible walking and teleported her backwards.
        crate::cinematic::land_player(player, world, self.exit_pose(id))?;
        self.state.done.insert(id.into());
        self.state.beat = None;
        self.state.time = 0.;
        Ok(())
    }
    fn exit_pose(&self, id: &str) -> Transform {
        self.points[match id {
            "entry" => "alice_cat_talk2",
            "knife_cat" => "alice_knife_node",
            "Torchgnome1_Dialog" => "alice_torch2_talk1",
            "Torchgnome2_Dialog" => "alice_talk_posx2",
            "Torchgnome3_Dialog_part2" => "alice_tg4_cat2",
            "Torchgnome4_Dialog" => "alice_tg4_talk",
            _ => unreachable!("unreviewed Village scene endpoint"),
        }]
    }
    pub fn skip(&mut self, player: &mut Player, world: &World, story: &mut Story) -> Result<bool> {
        let Some(id) = self.scene_id() else {
            return Ok(false);
        };
        self.finish(player, world)?;
        story.line_limit = None;
        story.finish_sequence(id);
        Ok(true)
    }
    pub fn advance(&mut self, dt: f32, player: &mut Player, world: &World) -> Result<()> {
        if dt <= 0. || !self.active() {
            return Ok(());
        }
        if self.state.home.is_none() {
            self.state.home = Some(player.feet);
        }
        self.state.time += dt;
        self.state.shot_time += dt;
        match self.state.beat {
            Some(Beat::KnifeEnd) if self.state.time >= 5. => self.state.start(Beat::Shrink),
            Some(Beat::Rabbit) if self.state.time >= self.rabbit_duration() => {
                self.state.start(Beat::Cat)
            }
            Some(Beat::IntroEnd) if self.state.time >= 3.5 => self.finish(player, world)?,
            Some(Beat::End(_)) if self.state.time >= 0.5 => self.finish(player, world)?,
            Some(Beat::Retreat)
                if self.state.time
                    >= 0.5
                        + self
                            .at("torchgnome2_talk_pos1")
                            .distance(self.at("torchgnome2_pos1"))
                            / self.speeds["c_torchgnome/run"] =>
            {
                self.finish(player, world)?
            }
            Some(Beat::Shrink) if self.state.time >= 5.5 => self.finish(player, world)?,
            _ => {}
        }
        if let Some(Beat::Gnome(i)) = self.state.beat {
            let node = if i == 2 && self.state.line >= 6 {
                "alice_tg4_cat2"
            } else {
                [
                    "alice_torch2_talk1",
                    "alice_talk_posx2",
                    "alice_tg3_talk",
                    "alice_tg4_talk",
                ][i as usize]
            };
            player.feet = self.at(node);
            player.script_facing = self.points[node].rotation.to_euler(EulerRot::ZYX).0;
        }
        if self.active() {
            player.velocity = Vec3::ZERO;
        }
        Ok(())
    }
    fn rabbit_duration(&self) -> f32 {
        ((self.at("rabbit_pos2") - self.at("rabbit_pos1")).length()
            + (self.at("rabbit_pos3") - self.at("rabbit_pos2")).length())
            / self.speeds["c_whiterabbit/run"]
    }
    fn travel(&self, a: &str, b: &str, t: f32, speed: f32) -> Transform {
        let from = self.at(a);
        let delta = self.at(b) - from;
        pose(
            from.lerp(from + delta, fraction(t, delta.length() / speed)),
            delta.y.atan2(delta.x).to_degrees(),
        )
    }
    fn alice_pose(&self, beat: Beat) -> (&'static str, f32, bool, Transform, f32) {
        let t = self.state.time;
        match beat {
            Beat::Essence => ("idle_stand", t, true, self.points["alice_talk_posx2"], 1.),
            Beat::Knife => {
                let walk = self
                    .at("alice_knife_node")
                    .distance(self.at("alice_knife_start"))
                    / 65.;
                if t < walk {
                    (
                        "walk",
                        t,
                        true,
                        self.travel("alice_knife_start", "alice_knife_node", t, 65.),
                        1.,
                    )
                } else {
                    (
                        "knife_idle2",
                        t - walk,
                        false,
                        self.points["alice_knife_node"],
                        1.,
                    )
                }
            }
            Beat::KnifeEnd => ("idle_base_03", t, true, self.points["alice_knife_node"], 1.),
            Beat::Fall if t < 4. => (
                "death_falling2_silent",
                t,
                true,
                pose(self.fall_start, 270.),
                (1. - 0.2 * t).max(0.25),
            ),
            Beat::Fall if t < 6. => (
                "jump_falling2",
                t - 4.,
                true,
                pose(
                    self.fall_start
                        .lerp(self.at("alice_cat_talk1"), fraction(t - 4., 2.).powi(2)),
                    270.,
                ),
                1.,
            ),
            Beat::Fall if t < 8.5 => (
                "pain_knockdown",
                t - 6.,
                false,
                pose(self.at("alice_cat_talk1"), 270.),
                1.,
            ),
            Beat::Cat => {
                let walk = (t - self.cat_prelude() + 1.6).max(0.);
                let duration =
                    (self.at("alice_cat_talk2") - self.at("alice_cat_talk1")).length() / 60.;
                if walk < duration {
                    (
                        "walk",
                        walk,
                        true,
                        self.travel("alice_cat_talk1", "alice_cat_talk2", walk, 60.),
                        1.,
                    )
                } else {
                    let (clip, time, looping) = self.acting(
                        "alice",
                        walk - duration,
                        &[
                            ("idle_base_02_2_stand", None),
                            (
                                "idle_stand",
                                Some(self.duration("alice", "idle_stand") + 8.),
                            ),
                            ("idle_stand_rocktoes", None),
                        ],
                    );
                    (clip, time, looping, self.points["alice_cat_talk2"], 1.)
                }
            }
            Beat::Gnome(i) | Beat::End(i) => {
                let name = if i == 2 && self.state.line >= 6 {
                    "alice_tg4_cat2"
                } else {
                    [
                        "alice_torch2_talk1",
                        "alice_talk_posx2",
                        "alice_tg3_talk",
                        "alice_tg4_talk",
                    ][i as usize]
                };
                let steps: &[(&'static str, Option<f32>)] = match i {
                    1 => &[
                        (
                            "idle_base_01",
                            Some(self.duration("alice", "idle_base_01") + 3.),
                        ),
                        ("idle_base_01_play2", None),
                        ("idle_base_01", None),
                        ("idle_base_01_2_base_02", None),
                        ("idle_base_02", None),
                        ("idle_base_02_2_shrug", None),
                        ("idle_shrug", None),
                        ("idle_shrug_headtilt", None),
                        ("idle_shrug", None),
                    ],
                    2 => &[
                        ("idle_base_02", Some(4.)),
                        ("idle_base_02_2_stand", None),
                        (
                            "idle_stand",
                            Some(self.duration("alice", "idle_stand") + 9.),
                        ),
                        ("idle_stand_shakeno", None),
                        (
                            "idle_stand",
                            Some(self.duration("alice", "idle_stand") + 2.4),
                        ),
                        ("talk_gnomes_01", None),
                        ("idle_stand_rocktoes", None),
                        (
                            "idle_stand",
                            Some(self.duration("alice", "idle_stand") + 9.),
                        ),
                        ("talk_gnomes_03", None),
                    ],
                    3 => &[
                        ("idle_shrug_headtilt", None),
                        ("idle_shrug", None),
                        ("idle_shrug_tap", None),
                    ],
                    _ => &[],
                };
                let (clip, time, looping) = if i == 2 && self.state.line >= 6 {
                    // The Cat cutaway has its own short reply gesture.
                    if self.state.line == 7 && self.state.line_time >= 1. {
                        self.acting(
                            "alice",
                            self.state.line_time - 1.,
                            &[("idle_stand_rocktoes", None)],
                        )
                    } else {
                        ("idle_stand", self.state.line_time, true)
                    }
                } else {
                    let time = if i == 2 {
                        self.gnome3_lines[self.state.line] + self.state.line_time
                    } else {
                        t
                    };
                    self.acting("alice", time, steps)
                };
                (clip, time, looping, self.points[name], 1.)
            }
            Beat::Retreat => ("idle_stand", t, true, self.points["alice_torch2_talk1"], 1.),
            // This is a Rabbit cutaway. Alice remains where she picked up the
            // Blade; the original script's warp to the Gnome marker is disabled.
            Beat::Shrink => ("idle_stand", t, true, self.points["alice_knife_node"], 1.),
            Beat::IntroEnd => ("idle_stand", t, true, self.points["alice_cat_talk2"], 1.),
            _ => (
                "ready",
                (t - 8.5).max(0.),
                true,
                self.points["alice_cat_talk1"],
                1.,
            ),
        }
    }

    pub fn camera(&self) -> Option<Camera> {
        let s = &self.state;
        let t = s.time;
        let (name, time) = match s.beat? {
            Beat::Fall if t < 4. => ("gvillage_start_p1", t),
            Beat::Fall if t < 8.5 => ("gvillage_start_p2", t - 4.),
            Beat::Fall => ("gvillage_start_p3", 0.),
            Beat::Rabbit => ("gvillage_start_p3", (t - 0.5).max(0.)),
            Beat::Cat | Beat::IntroEnd => (
                "gvillage_start_p4",
                if s.beat == Some(Beat::Cat) {
                    (t - self.cat_prelude() - 5.).max(0.)
                } else {
                    30.
                },
            ),
            Beat::Gnome(0) | Beat::Retreat => ("gvillage_tg1_path1", t),
            Beat::Essence => ("gvillage_metap2", (t - 0.5).max(0.)),
            Beat::End(1) if s.line == 4 => ("gvillage_metap2", 11. + t),
            Beat::Gnome(1) | Beat::End(1) => (
                if s.line == 0 {
                    "gvillage_tg2p1"
                } else {
                    "gvillage_tg2p2"
                },
                s.shot_time,
            ),
            Beat::Gnome(2) | Beat::End(2) => (
                if s.line >= 6 {
                    "gvillage_cat2_p1"
                } else {
                    "gvillage_tg3_p1"
                },
                if s.line < 3 { 0. } else { s.shot_time },
            ),
            Beat::Gnome(_) | Beat::End(_) => ("gvillage_tg4_p1", t),
            Beat::Knife => ("gvillage_jdm2", t),
            Beat::KnifeEnd => ("gvillage_jdm3", (t - 1.).max(0.)),
            Beat::Shrink => ("gvillage_shrink_p1", (t - 1.5).max(0.)),
        };
        Some(self.tracks[name].sample(time))
    }
    pub fn fade(&self) -> (Color, f32) {
        let t = self.state.time;
        if self.state.beat == Some(Beat::Fall) {
            (BLACK, (1. - t / 3.).clamp(0., 1.))
        } else {
            (
                WHITE,
                match self.state.beat {
                    Some(Beat::Essence) => if t < 0.5 { t / 0.5 } else { (1. - (t - 0.5) / 0.5).max(0.) },
                    Some(Beat::End(1)) if self.state.line == 4 => (t / 0.5).min(1.),
                    Some(Beat::Gnome(_) | Beat::Knife) => (1. - t / 0.5).clamp(0., 1.),
                    _ => 0.,
                },
            )
        }
    }
    pub fn door_angle(&self) -> f32 {
        if self.state.beat == Some(Beat::Shrink) {
            -75. * fraction(self.state.time, 4.) * (1. - fraction(self.state.time - 5., 0.1))
        } else {
            0.
        }
    }
    pub fn apply_npcs(&self, npcs: &mut crate::npc::Npcs) {
        let mut hidden = vec!["rabbit_actor1", "cat_hole1", "knife_cat"];
        match self.state.beat {
            Some(Beat::Gnome(i) | Beat::End(i)) => {
                hidden.push(CAST[i as usize]);
                if i == 2 && self.state.line >= 6 {
                    hidden.push("cat_shrink1");
                }
            }
            Some(Beat::Retreat) => hidden.push(CAST[0]),
            Some(Beat::Essence) => hidden.push(CAST[1]),
            Some(Beat::Shrink) => hidden.push(CAST[2]),
            _ => {}
        }
        npcs.scene_hidden(&hidden);
        for (i, marker) in [
            "torchgnome2_pos1",
            "torchgnome1_talk",
            "torchgnome3_talk",
            "torchgnome4_talk",
        ]
        .into_iter()
        .enumerate()
        {
            if self.state.done.contains(GNOMES[i]) {
                npcs.scene_place(CAST[i], self.points[marker]);
            }
        }
        // BSP origins store these actors away from the playable hint location.
        // Apply the authored relocation to presentation, preserving save identity.
        for (actor, marker) in [
            ("climb_cat", "climb_cat_node"),
            ("exit_cat", "exit_cat_node"),
        ] {
            npcs.scene_place(actor, self.points[marker]);
        }
    }
    pub fn sound_state(&self, clocks: &mut Vec<crate::audio::world::Clock>) {
        use crate::audio::world::Clock;
        let s = &self.state;
        match s.beat {
            Some(Beat::Fall) => clocks.push(Clock {
                key: "village-fall",
                time: s.time,
                period: None,
                origin: self.at("alice_cat_talk1"),
                cues: &[
                    (1.5, "sound/character/alice/death_fall.wav"),
                    (6., "sound/character/alice/pain_knockdown.wav"),
                ],
            }),
            Some(Beat::Essence) => clocks.push(Clock {
                key: "village-essence-cat",
                time: s.time,
                period: None,
                origin: self.at("essence_cat"),
                cues: &[(0.5, "sound/character/cheshire_cat/appear.wav"),
                        (10.5, "sound/character/cheshire_cat/disappear.wav")],
            }),
            Some(Beat::Cat | Beat::Knife) => clocks.push(Clock {
                key: "village-cat-appear",
                time: s.time,
                period: None,
                origin: self.at(if s.beat == Some(Beat::Knife) {
                    "knife_cat"
                } else {
                    "cat_start_pos1"
                }),
                cues: &[(0., "sound/character/cheshire_cat/appear.wav")],
            }),
            Some(Beat::IntroEnd | Beat::KnifeEnd) => clocks.push(Clock {
                key: "village-cat-disappear",
                time: s.time,
                period: None,
                origin: self.at(if s.beat == Some(Beat::KnifeEnd) {
                    "knife_cat"
                } else {
                    "cat_start_pos1"
                }),
                cues: &[(0., "sound/character/cheshire_cat/disappear.wav")],
            }),
            Some(Beat::Rabbit | Beat::Shrink) => clocks.push(Clock {
                key: "village-rabbit-run",
                time: s.time,
                period: Some(0.4),
                origin: self.rabbit_pose().translation,
                cues: &[
                    (0.1, "sound/character/white_rabbit/step1.wav"),
                    (0.2, "sound/character/white_rabbit/step2.wav"),
                ],
            }),
            _ => {}
        }
    }
    fn rabbit_pose(&self) -> Transform {
        let s = &self.state;
        let speed = self.speeds["c_whiterabbit/run"];
        if s.beat == Some(Beat::Rabbit) {
            let first = (self.at("rabbit_pos2") - self.at("rabbit_pos1")).length() / speed;
            if s.time < first {
                self.travel("rabbit_pos1", "rabbit_pos2", s.time, speed)
            } else {
                self.travel("rabbit_pos2", "rabbit_pos3", s.time - first, speed)
            }
        } else if s.beat == Some(Beat::Shrink) {
            self.travel(
                "rabbit_start_shrink2",
                "rabbit_shrink_pos1",
                // Integrate the shrinking model scale over movement time. A
                // constant slow speed lets the authored camera overtake him.
                {
                    let t = s.time.clamp(0.5, 4.5);
                    (t - 0.5) - 0.1 * (t * t - 0.25) + (s.time - 4.5).max(0.) * 0.1
                },
                speed,
            )
        } else if s.beat == Some(Beat::KnifeEnd) {
            self.travel(
                "rabbit_start_shrink",
                "rabbit_start_shrink2",
                (s.time - 3.).max(0.),
                self.speeds["c_whiterabbit/walk"],
            )
        } else if matches!(s.beat, Some(Beat::Fall)) {
            self.points["rabbit_pos1"]
        } else {
            self.points["rabbit_start_shrink"]
        }
    }
}

pub struct Art {
    knife: crate::weapons::Prop,
    pub(crate) alice: Puppet,
    rabbit: Puppet,
    gnomes: Vec<Puppet>,
    cat: Puppet,
    material: crate::character::SkinMaterial,
}

/// Actual archive data, time advancement, safe hand-off, and saved scene continuation.
pub fn check(assets: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&assets.read("maps/gvillage.bsp")?)?;
    let mut world = World::from_bsp(&map)?;
    let mut initial = crate::interaction::Interactions::load(&map)?;
    initial.set_entry(assets, &map, "gvillage", None)?;
    initial.sync(&mut world);
    let hints = crate::cheshire::Hints::load(assets, &map, "gvillage")?;
    gnome3::check(assets, &map, &world, &hints)?;
    let mut rabbit_shot = Cinema::load(assets, &map)?;
    rabbit_shot.state.start(Beat::Shrink);
    for time in [0.75, 1.5, 2.5, 3.] {
        rabbit_shot.state.time = time;
        let camera = rabbit_shot.camera().unwrap();
        let rabbit = rabbit_shot.rabbit_pose().translation + Vec3::Z * 24. * (1. - time * 0.2);
        ensure!(
            (rabbit - camera.eye)
                .normalize()
                .dot((camera.target - camera.eye).normalize())
                > 0.7,
            "Shrinking Rabbit fell behind his authored camera at {time}s"
        );
    }
    let spawn = crate::interaction::spawn(&map, None).0;
    for event in ["entry", "knife_cat"].into_iter().chain(GNOMES) {
        let mut endpoints = Vec::new();
        for skip_at in [None, Some(0_f32), Some(2.5), Some(12.)] {
            let mut c = Cinema::load(assets, &map)?;
            let mut story = Story::load(assets, "gvillage");
            let mut player =
                Player::spawn(&world, spawn).context("Village playable opening blocked")?;
            if event == "entry" {
                c.begin(&mut story);
            } else if event == "knife_cat" {
                c.state.start(Beat::Knife);
                story.trigger(event);
            } else {
                story.trigger(event);
                story.tick(1. / 60., false);
                c.sync_story(&story);
            }
            let mut time = 0.;
            let mut restored = false;
            let mut essence_restored = false;
            for _ in 0..18000 {
                let final_pose = c.state.beat.map(|beat| c.alice_pose(beat).3);
                if c.active() && skip_at.is_some_and(|at| time >= at) {
                    c.skip(&mut player, &world, &mut story)?;
                }
                c.advance(1. / 60., &mut player, &world)?;
                if skip_at.is_none() && !c.active() {
                    let at = final_pose.context("Missing final Village pose")?;
                    ensure!(
                        player.feet.distance(at.translation) < 8.1,
                        "Alice teleported on return from {event}: {:?} -> {:?}",
                        at.translation,
                        player.feet
                    );
                    ensure!(
                        (player.script_facing - at.rotation.to_euler(EulerRot::ZYX).0).cos()
                            > 0.999,
                        "Alice changed facing on return from {event}"
                    );
                }
                if c.prepare_story(&mut story) {
                    story.tick(1. / 60., false);
                }
                c.sync_story(&story);
                if c.state.beat == Some(Beat::Essence) && c.state.time >= 3. && !essence_restored {
                    let saved = c.state.clone();
                    let speech = story.snapshot();
                    let mut fresh = Cinema::load(assets, &map)?;
                    fresh.restore(Some(&saved))?;
                    ensure!(serde_json::to_value(&fresh.state)? == serde_json::to_value(&saved)?,
                        "Essence cutaway lost saved camera clock");
                    fresh.advance(0., &mut player, &world)?;
                    ensure!(serde_json::to_value(&fresh.state)? == serde_json::to_value(&saved)?,
                        "Paused essence cutaway advanced");
                    story.restore(&speech, &hints)?;
                    c = fresh;
                    essence_restored = true;
                }
                for id in story.take_completed() {
                    c.completed(&id);
                }
                if let Some(camera) = c.camera() {
                    ensure!(
                        camera.eye.is_finite()
                            && camera.target.is_finite()
                            && (camera.target - camera.eye).length() > 1.,
                        "Invalid village camera"
                    );
                }
                if time >= 1. && !restored {
                    let bytes = serde_json::to_vec(&c.state)?;
                    let before = bytes.clone();
                    let st = story.snapshot();
                    let mut other = Cinema::load(assets, &map)?;
                    other.restore(Some(&serde_json::from_slice(&bytes)?))?;
                    ensure!(
                        serde_json::to_vec(&other.state)? == before,
                        "Village scene restore changed phase"
                    );
                    c = other;
                    story.restore(&st, &hints)?;
                    restored = true;
                    let before = serde_json::to_vec(&c.state)?;
                    c.advance(0., &mut player, &world)?;
                    ensure!(
                        serde_json::to_vec(&c.state)? == before,
                        "Paused village scene advanced"
                    );
                }
                time += 1. / 60.;
                if !c.active() && !story.busy() {
                    break;
                }
            }
            ensure!(
                !c.active() && !story.busy() && c.state.done.contains(event),
                "Village scene did not finish: {event}"
            );
            if event == GNOMES[1] && skip_at.is_none() {
                ensure!(essence_restored, "Second Gnome omitted essence cutaway");
                ensure!(story.repeat_conversation(event), "Gnome conversation stopped being repeatable");
                let repeat = story.snapshot();
                story.restore(&repeat, &hints)?;
                for _ in 0..18000 {
                    story.tick(1. / 60., false);
                    ensure!(story.line().is_none_or(|l| l.actor != "essence_cat"),
                        "Repeat Gnome talk replayed the one-time Cat cutaway");
                    if !story.busy() { break; }
                }
                ensure!(!story.busy(), "Repeated Gnome dialogue failed to finish");
            }
            ensure!(
                world.body_clear(player.feet),
                "Village scene ended in geometry: {event} {:?}",
                player.feet
            );
            ensure!(
                player.feet.distance(c.exit_pose(event).translation) < 8.1,
                "Watched/skipped scene missed its final marker: {event}"
            );
            ensure!(
                !c.skip(&mut player, &world, &mut story)?,
                "Completed scene skipped twice"
            );
            endpoints.push((c.state.done, story.completed, player.feet));
            println!(
                "PASS village scene {event}, skip {skip_at:?}, {time:.2}s, clear landing {:?}",
                player.feet
            );
        }
        ensure!(
            endpoints.windows(2).all(|v| v[0] == v[1]),
            "Watching/skipping village scene changed progression: {event}"
        );
    }
    let mut c = Cinema::load(assets, &map)?;
    c.restore(None)?;
    ensure!(
        c.state.done.len() == 1 && !c.active(),
        "Legacy village save replays cinema"
    );
    let cached = serde_json::to_vec(&c.state)?;
    c.restore(Some(&serde_json::from_slice(&cached)?))?;
    let mut stats = crate::inventory::Stats::default();
    stats.collected.insert(format!("gvillage:{}", c.knife_id));
    let mut story = Story::load(assets, "gvillage");
    c.sync_pickups(&stats, &mut story);
    c.sync_story(&story);
    ensure!(
        !c.active() && !story.busy() && c.state.done.contains("knife_cat"),
        "Resaving a legacy cached visit replayed the collected Blade scene"
    );
    c.state.time = f32::NAN;
    ensure!(c.state.validate().is_err(), "Invalid clock accepted");
    println!("PASS village cinema: six scenes, watched/skipped endpoints, pause, save/restore, legacy migration and invalid state rejection");
    Ok(())
}

pub async fn render_check(assets: &mut Assets) -> Result<()> {
    crate::character::check_shared_material().await?;
    let mut scene = crate::render::Scene::load(assets, "gvillage")?;
    let village = super::Village::load(assets, &scene.map)?;
    let mut c = Cinema::load(assets, &scene.map)?;
    let mut art = Art::load(assets)?;
    std::fs::create_dir_all("private/village-cinema")?;
    for (name, beat, time, line) in [
        ("fall", Beat::Fall, 2., 0),
        ("landing", Beat::Fall, 6.6, 0),
        ("rabbit", Beat::Rabbit, 0.6, 0),
        ("cat-appear", Beat::Cat, 1.5, 0),
        ("cat-walk", Beat::Cat, 10., 1),
        ("cat-dialogue", Beat::Cat, 17., 2),
        ("gnome1", Beat::Gnome(0), 3., 0),
        ("retreat", Beat::Retreat, 1.2, 2),
        ("gnome2", Beat::Gnome(1), 4., 1),
        ("gnome3", Beat::Gnome(2), 3., 2),
        ("gnome3-cat", Beat::Gnome(2), 3., 7),
        ("knife", Beat::Knife, 3., 0),
        ("shrink", Beat::Shrink, 3., 0),
        ("gnome4", Beat::Gnome(3), 3., 1),
    ] {
        c.state.start(beat);
        c.state.time = time;
        c.state.shot_time = time;
        c.state.line = line;
        let camera = c.camera().unwrap();
        clear_background(BLACK);
        set_camera(&Camera3D {
            position: camera.eye,
            target: camera.target,
            up: camera.up,
            fovy: 75_f32.to_radians(),
            z_near: 2.,
            z_far: 30000.,
            ..Default::default()
        });
        let transforms = village
            .objects
            .iter()
            .map(|o| {
                let mut p = super::sample(&o.name, o.base, time, 0.);
                if o.name == "shrink_door1" {
                    p.angles.z = c.door_angle();
                }
                (o.model, p.origin, p.rotation())
            })
            .collect::<Vec<_>>();
        scene.draw(camera.eye, time, false, false, &transforms);
        if [
            "landing",
            "cat-dialogue",
            "gnome1",
            "gnome2",
            "gnome3",
            "gnome3-cat",
            "gnome4",
            "knife",
        ]
        .contains(&name)
        {
            // These authored shots frame Alice: assert real world/camera coverage,
            // separately from the isolated pose checks used for cutaway shots.
            let backdrop = get_screen_data();
            art.material.atmosphere(&scene.atmosphere, camera.eye);
            let (clip, time, looping, pose, scale) = c.alice_pose(beat);
            art.alice.draw(clip, time, looping, pose, scale, false);
            let pixels = crate::character::visible_pixels(&backdrop, &get_screen_data());
            ensure!(
                pixels > 100,
                "Alice not visible in authored {name} shot: {pixels} pixels"
            );
            println!("PASS authored Village {name} camera: {pixels} Alice pixels");
        }
        let backdrop = (name == "fall").then(get_screen_data);
        art.draw(&c, &scene.atmosphere, camera.eye, false);
        if let Some(backdrop) = backdrop {
            let composed = get_screen_data();
            let visible = backdrop
                .bytes
                .chunks_exact(4)
                .zip(composed.bytes.chunks_exact(4))
                .filter(|(a, b)| a[..3] != b[..3])
                .count();
            ensure!(
                visible > 100,
                "Falling Alice is not visible through the chute: {visible} pixels"
            );
        }
        get_screen_data().export_png(&format!("private/village-cinema/{name}.png"));
        set_default_camera();
        next_frame().await;
        println!("CAPTURE village {name}");
    }
    Ok(())
}
impl Art {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(assets)?;
        let mut alice = Puppet::load(assets, "alice", ALICE, &specs)?;
        alice.show_attachments(false);
        let mut gnomes = Vec::new();
        for model in [
            "c_torchgnome3",
            "c_torchgnome5",
            "c_torchgnome6",
            "c_torchgnome4",
        ] {
            gnomes.push(Puppet::load(assets, model, TORCH, &specs)?);
        }
        Ok(Self {
            alice,
            knife: crate::weapons::Prop::load(assets, "w_knife", &specs)?,
            rabbit: Puppet::load(assets, "c_whiterabbit", &["idle", "run", "walk"], &specs)?,
            gnomes,
            cat: Puppet::load(assets, "c_cheshire", CAT, &specs)?,
            material: crate::character::skin_material()?,
        })
    }
    pub fn story_pose(&mut self, story: &Story) {
        self.alice.mouth(story.mouth(&["fakeplayer", "player"]));
        self.rabbit.mouth(story.mouth(&["rabbit_actor1"]));
        self.cat
            .mouth(story.mouth(&["cat_hole1", "cat_shrink1", "knife_cat", "essence_cat"]));
        for (g, name) in self.gnomes.iter_mut().zip(CAST) {
            g.mouth(story.mouth(&[name]));
        }
    }
    pub fn draw(
        &mut self,
        c: &Cinema,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
        fullbright: bool,
    ) {
        self.material.atmosphere(atmosphere, camera);
        self.material.bind();
        let s = &c.state;
        let t = s.time;
        if !(s.done.contains("knife_cat") || s.beat == Some(Beat::Shrink) && t >= 5.) {
            let running = matches!(s.beat, Some(Beat::Rabbit | Beat::Shrink));
            let walking = s.beat == Some(Beat::KnifeEnd) && t >= 3.;
            self.rabbit.draw(
                if running {
                    "run"
                } else if walking {
                    "walk"
                } else {
                    "idle"
                },
                if walking { t - 3. } else { t },
                true,
                c.rabbit_pose(),
                if s.beat == Some(Beat::Shrink) {
                    (1. - t * 0.2).max(0.1)
                } else {
                    1.
                },
                fullbright,
            );
        }
        if let Some(beat) = s.beat {
            let (clip, time, looping, at, scale) = c.alice_pose(beat);
            self.alice.draw(clip, time, looping, at, scale, fullbright);
            if matches!(beat, Beat::Knife | Beat::KnifeEnd | Beat::Shrink) {
                if let Some(tag) = self.alice.tag("tag_weapon", clip, time, at, 1.) {
                    self.knife.draw_frame(tag, 1., fullbright, time, true);
                }
            }
            if beat == Beat::Knife || (beat == Beat::KnifeEnd && t < 2.) {
                self.cat.draw_dissolving(
                    "sit_talk1",
                    s.line_time,
                    true,
                    c.points["knife_cat"],
                    1.,
                    fullbright,
                    if beat == Beat::Knife {
                        1. - fraction(t, 2.)
                    } else {
                        fraction(t, 2.)
                    },
                );
            }
            let gnome = match beat {
                Beat::Gnome(i) | Beat::End(i) => Some(i as usize),
                Beat::Essence => Some(1),
                Beat::Retreat => Some(0),
                Beat::Shrink => Some(2),
                _ => None,
            };
            if let Some(i) = gnome {
                let name = [
                    "torchgnome2_talk_pos1",
                    "torchgnome1_talk",
                    "torchgnome3_talk",
                    "torchgnome4_talk",
                ][i];
                let at = if beat == Beat::Retreat {
                    c.travel(
                        name,
                        "torchgnome2_pos1",
                        (t - 0.5).max(0.),
                        c.speeds["c_torchgnome/run"],
                    )
                } else {
                    let from = c.at(name);
                    let delta = at.translation - from;
                    pose(from, delta.y.atan2(delta.x).to_degrees())
                };
                let (clip, time, looping) = if beat == Beat::Retreat {
                    (
                        if t < 0.5 { "idle" } else { "run" },
                        (t - 0.5).max(0.),
                        true,
                    )
                } else {
                    c.acting(
                        "c_torchgnome",
                        t,
                        match i {
                            0 => &[
                                ("idle", Some(5.)),
                                ("talkbegin", None),
                                ("talk1", None),
                                ("talk2", None),
                            ],
                            3 => &[
                                ("talkbegin", None),
                                ("talk2", None),
                                ("talk1", None),
                                ("idle", Some(8.)),
                                ("talk2", None),
                            ],
                            _ => &[
                                ("talkbegin", None),
                                ("talk1", None),
                                ("talk2", None),
                                ("idle", Some(8.)),
                                ("talk2", None),
                            ],
                        },
                    )
                };
                self.gnomes[i].draw(clip, time, looping, at, 1., fullbright);
            }
            if matches!(beat, Beat::Cat | Beat::IntroEnd) {
                let begin = 2.
                    + c.duration("c_cheshire", "sit_smile_open")
                    + c.duration("c_cheshire", "sit_smile_shut")
                    + c.duration("c_cheshire", "sit_2walk");
                let walk = (t - begin).max(0.);
                let duration = c.cat_start.distance(c.at("cat_start_pos1")) / 75.;
                let at = if beat == Beat::IntroEnd {
                    c.points["cat_start_pos1"]
                } else {
                    pose(
                        c.cat_start
                            .lerp(c.at("cat_start_pos1"), fraction(walk, duration)),
                        90.,
                    )
                };
                let (clip, time, looping) = if beat == Beat::IntroEnd || t < 2. {
                    ("sit_idle1", t, true)
                } else if t < 2. + c.duration("c_cheshire", "sit_smile_open") {
                    ("sit_smile_open", t - 2., false)
                } else if t < begin - c.duration("c_cheshire", "sit_2walk") {
                    (
                        "sit_smile_shut",
                        t - 2. - c.duration("c_cheshire", "sit_smile_open"),
                        false,
                    )
                } else if t < begin {
                    (
                        "sit_2walk",
                        t - begin + c.duration("c_cheshire", "sit_2walk"),
                        false,
                    )
                } else if walk < duration {
                    ("walk", walk, true)
                } else {
                    (
                        if s.line == 2 || s.line >= 4 {
                            "sit_talk3"
                        } else {
                            "sit_idle2"
                        },
                        s.line_time,
                        true,
                    )
                };
                self.cat.draw_dissolving(
                    clip,
                    time,
                    looping,
                    at,
                    1.,
                    fullbright,
                    if beat == Beat::IntroEnd {
                        fraction(t, 2.)
                    } else {
                        1. - fraction(t, 2.)
                    },
                );
            } else if beat == Beat::Essence || (beat == Beat::End(1) && s.line == 4) {
                let age = if beat == Beat::Essence { (t - 0.5).max(0.) } else { 11. + t };
                let opacity = fraction(age, 2.) * (1. - fraction(age - 10., 2.));
                self.cat.draw_dissolving(
                    if age < 6. { "sit_idle1" } else { "sit_smile_open" },
                    if age < 6. { age } else { age - 6. }, age < 6.,
                    c.points["essence_cat"], 1., fullbright, 1. - opacity,
                );
            } else if beat == Beat::Gnome(2) && s.line >= 6 {
                self.cat.draw(
                    if s.line == 7 {
                        "sit_idle1"
                    } else {
                        "sit_talk1"
                    },
                    s.line_time,
                    true,
                    c.points["cat_tg3_talk"],
                    1.,
                    fullbright,
                );
            }
        }
        gl_use_default_material();
    }
}
