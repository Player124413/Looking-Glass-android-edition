//! Reviewed second-school staging; local identifiers and timings, never script execution.
use super::*;
use crate::{
    cinematic::{Camera, Track},
    fortress::spline::Spline,
    npc::Puppet,
    story::Story,
};
use anyhow::ensure;
mod potion;
pub const GROW: &str = "Skool2_GrowLollypop";
pub const FINAL: &str = "Skool2_LastGnome_Cinema";
pub const MUSHROOM: &str = "Old_Gnome_Mushroom";
pub const SPICE: &str = "Old_Gnome_SpiceDrops";
pub const DICE: &str = "dice_cat";
pub const SOUNDS: &[&str] = &[
    "sound/character/gnome/elder/vanish.wav",
    "sound/world/door/door wood open 02.wav",
    "sound/ambience/special/lollipop_grow.wav",
    "sound/character/gnome/elder/mixing.wav",
    "sound/world/machine/condenser.wav",
    "sound/character/cheshire_cat/appear.wav",
    "sound/character/cheshire_cat/disappear.wav",
];
const GNOME: &[&str] = &[
    "idle", "talk", "handout", "walk", "vanish01", "vanish02", "mixing", "letsgo",
];
const CAT: &[&str] = &["sit_idle1", "sit_talk1", "sit_smile_open", "sit_smile_shut"];
const ALICE: &[&str] = &["idle_stand", "idle_stand_tiptoes", "idle_stand_rocktoes"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Beat {
    Bleachers,
    Dice,
    Mushroom,
    Warp,
    Spice,
    Growth,
    Final,
}
impl Beat {
    fn dialogue(self) -> Option<&'static str> {
        match self {
            Self::Dice => Some(DICE),
            Self::Mushroom => Some(MUSHROOM),
            Self::Spice => Some(SPICE),
            Self::Final => Some(FINAL),
            Self::Warp | Self::Growth | Self::Bleachers => None,
        }
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub beat: Beat,
    pub active: bool,
    pub time: f32,
    pub home: Option<Transform>,
    line: usize,
    final_line: Option<f32>,
    dialogue_done: bool,
    ending: Option<f32>,
    pub cabinet: Option<f32>,
    cues: u32,
    pub sounds: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    potion: Option<potion::State>,
}
impl State {
    fn new(beat: Beat) -> Self {
        Self {
            beat,
            active: true,
            time: 0.,
            home: None,
            line: 0,
            final_line: None,
            dialogue_done: false,
            ending: None,
            cabinet: None,
            cues: 0,
            sounds: Vec::new(),
            potion: (beat == Beat::Final).then(potion::State::default),
        }
    }
    pub fn validate(&self, stage: Stage) -> Result<()> {
        ensure!(
            self.time.is_finite()
                && (0. ..=10000.).contains(&self.time)
                && self.line <= 3
                && self.cues < 32
                && self.sounds.len() <= 5
                && self.sounds.iter().all(|&n| n < SOUNDS.len())
                && [self.final_line, self.ending, self.cabinet]
                    .iter()
                    .all(|v| v.is_none_or(|t| t.is_finite() && (0. ..=self.time).contains(&t)))
                && self.home.is_none_or(|p| p.translation.is_finite()
                    && p.translation.abs().max_element() < 100000.
                    && p.rotation.is_normalized()),
            "Invalid second-school scene save"
        );
        ensure!(
            !self.active
                || stage
                    == match self.beat {
                        Beat::Bleachers | Beat::Dice => stage,
                        Beat::Mushroom => Stage::MushroomDialogue,
                        Beat::Warp => Stage::Laboratory,
                        Beat::Spice => Stage::SpiceDialogue,
                        Beat::Growth => Stage::Growing,
                        Beat::Final => Stage::FinalDialogue,
                    },
            "Second-school scene disagrees with quest"
        );
        if let Some(p) = &self.potion {
            p.validate(self.time)?;
        }
        ensure!(
            !self.active || self.beat != Beat::Final || self.potion.is_some(),
            "Missing potion scene clock"
        );
        Ok(())
    }
    fn cue(&mut self, bit: u32, sound: usize) {
        if self.cues & (1 << bit) == 0 {
            self.cues |= 1 << bit;
            self.sounds.push(sound);
        }
    }
    fn action(&self) -> f32 {
        self.final_line.map_or(0., |t| self.time - t)
    }
}
pub struct Data {
    tracks: BTreeMap<String, Spline>,
    points: BTreeMap<String, Transform>,
    lengths: BTreeMap<String, f32>,
    walk_time: f32,
    boojums: Spline,
}
impl Data {
    pub fn load(assets: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut tracks = BTreeMap::new();
        for name in [
            "skool1_bleach_p1",
            "skool2_gnomew1",
            "skool2_dicep1",
            "skool2_path1",
            "skool2_path1b",
            "skool2_gnomew2",
            "skool2_path3",
            "skool2_path7",
            "skool2_path7a",
            "skool2_path4",
            "skool2_path5",
            "skool2_path5a",
        ] {
            tracks.insert(
                name.into(),
                Spline::camera_track(Track::load(assets, name)?.controls().collect()),
            );
        }
        let mut points = BTreeMap::new();
        let world = World::from_bsp(map)?;
        for name in [
            "dice_cat",
            "bleach8",
            "old_gnome_1",
            "alice_mush1",
            "gnome_killpos1",
            "alice_killpos1",
            "gnome_pos1",
            "gnome_fire1",
            "gnome_fire2",
            "jumbo_door2",
            "get_booj1",
            "alice_pop1",
            "alice_last_pos1",
            "gnome_star_pos1",
            "gnome_fire3",
        ] {
            let e = entity(map, name)?;
            let mut p = origin(map, name)?;
            if [
                "old_gnome_1",
                "alice_mush1",
                "gnome_killpos1",
                "alice_killpos1",
                "gnome_pos1",
                "alice_pop1",
                "alice_last_pos1",
                "gnome_star_pos1",
            ]
            .contains(&name)
            {
                p = world
                    .actor_footing(p, Vec3::Z * 30., vec3(16., 16., 30.), 128.)
                    .unwrap_or(p);
            }
            points.insert(
                name.into(),
                Transform {
                    translation: p,
                    rotation: Quat::from_rotation_z(
                        e.get("angle")
                            .and_then(|v| v.parse::<f32>().ok())
                            .unwrap_or(0.)
                            .to_radians(),
                    ),
                },
            );
        }
        let mut lengths = BTreeMap::new();
        let mut speed = 1.;
        for (model, clips) in [("c_gnomeold", GNOME), ("alice", ALICE), ("c_cheshire", CAT)] {
            let def = crate::skeletal::Definition::load(assets, &format!("models/{model}.tik"))?;
            let rig = crate::skeletal::Skeleton::parse(
                &assets.read(&format!("{}/{}", def.path, def.model))?,
            )?;
            for clip in clips {
                let file = def.animations.get(*clip).with_context(|| format!("Missing School2 {model}/{clip}"))?;
                let a = crate::skeletal::Animation::parse(
                    &assets.read(&format!("{}/{}", def.path, file))?,
                    rig.bones.len(),
                )?;
                lengths.insert((*clip).into(), a.duration());
                if *clip == "walk" {
                    speed = a.distance * def.scale / a.duration();
                }
            }
        }
        ensure!(speed > 0., "Missing Gnome walk speed");
        let walk_time = points["gnome_killpos1"]
            .translation
            .distance(points["gnome_pos1"].translation)
            / speed;
        let mut nodes = Vec::new();
        let mut name = "booj_path1";
        let mut seen = std::collections::BTreeSet::new();
        while seen.insert(name.to_owned()) {
            let e = entity(map, name)?;
            nodes.push((
                origin(map, name)?,
                Quat::IDENTITY,
                e.get("speed").and_then(|v| v.parse().ok()).unwrap_or(1.),
            ));
            ensure!(nodes.len() < 128, "Invalid school Boojum path");
            let Some(next) = e.get("target") else {
                break;
            };
            name = next;
        }
        Ok(Self {
            tracks,
            points,
            lengths,
            walk_time,
            boojums: Spline::new(nodes, true),
        })
    }
    fn warp_end(&self) -> f32 {
        7. + self.walk_time
    }
    fn handout(&self, beat: Beat) -> f32 {
        match beat {
            Beat::Mushroom => 12. + self.lengths["talk"],
            _ => 10. + self.lengths["talk"],
        }
    }
    fn spice_return(&self) -> f32 {
        self.handout(Beat::Spice) + self.lengths["handout"] + 12. + self.lengths["talk"]
    }
    fn gnome_clip(&self, beat: Beat, t: f32) -> (&'static str, f32, bool) {
        let hand = self.handout(beat);
        let cues = if beat == Beat::Mushroom {
            vec![
                (0., "idle"),
                (3., "talk"),
                (3. + self.lengths["talk"], "idle"),
                (hand, "handout"),
                (hand + self.lengths["handout"], "idle"),
                (
                    hand + self.lengths["handout"] + self.lengths["idle"],
                    "talk",
                ),
                (
                    hand + self.lengths["handout"] + self.lengths["idle"] + self.lengths["talk"],
                    "idle",
                ),
            ]
        } else {
            vec![
                (0., "idle"),
                (8., "talk"),
                (8. + self.lengths["talk"], "idle"),
                (hand, "handout"),
                (hand + self.lengths["handout"], "idle"),
                (self.spice_return() - self.lengths["talk"], "talk"),
                (self.spice_return(), "handout"),
                (self.spice_return() + self.lengths["handout"], "idle"),
            ]
        };
        let &(at, clip) = cues.iter().rev().find(|(at, _)| t >= *at).unwrap();
        (clip, t - at, clip == "idle")
    }
}
impl School2 {
    pub fn begin_bleachers(&mut self) {
        if !self.cinematic() { self.start_scene(Beat::Bleachers); }
    }
    pub fn hold_bleachers(&self) -> bool {
        self.cinema.as_ref().is_some_and(|s| s.active && s.beat == Beat::Bleachers && s.time < 1.5)
    }
    pub fn begin_queued_dice(&mut self, story: &Story) {
        if !self.dice_guards && !self.cinematic() && story.sequence_pending(DICE) {
            self.start_scene(Beat::Dice);
        }
    }
    pub fn hold_dice_completion(&mut self) -> bool { self.scene_dialogue(DICE) }
    pub(super) fn start_scene(&mut self, beat: Beat) {
        self.cinema = Some(State::new(beat));
    }
    pub fn scene_id(&self) -> Option<&'static str> {
        self.cinema
            .as_ref()
            .filter(|s| s.active)
            .map(|s| match s.beat {
                Beat::Bleachers => "extendBleachers",
                Beat::Growth => GROW,
                _ => s.beat.dialogue().unwrap_or("OG1_Warp"),
            })
    }
    pub fn cinematic(&self) -> bool {
        self.cinema.as_ref().is_some_and(|s| s.active)
    }
    pub(super) fn scene_dialogue(&mut self, name: &str) -> bool {
        if let Some(s) = self
            .cinema
            .as_mut()
            .filter(|s| s.active && s.beat.dialogue() == Some(name))
        {
            s.dialogue_done = true;
            true
        } else {
            false
        }
    }
    pub fn prepare_scene_story(&self, story: &mut Story) -> bool {
        let Some(s) = self.cinema.as_ref().filter(|s| s.active) else {
            return true;
        };
        if s.beat == Beat::Final {
            return self.prepare_potion_story(story);
        }
        if s.beat == Beat::Dice { return s.time >= 1.5; }
        if matches!(s.beat, Beat::Warp | Beat::Growth | Beat::Bleachers) {
            return false;
        }
        if s.time < if s.beat == Beat::Spice { 0.7 } else { 0.5 } {
            return false;
        }
        // Keep the final line alive while its independently timed scene runs.
        story.line_limit = Some(if s.beat == Beat::Mushroom { 2 } else { 3 });
        if (s.beat == Beat::Mushroom && s.action() >= 13.6)
            || (s.beat == Beat::Spice && s.cabinet.is_some_and(|t| s.time - t >= 6.))
        {
            story.line_limit = None;
        }
        true
    }
    pub fn sync_scene_story(&mut self, story: &Story) {
        self.sync_rage_hint(story);
        self.sync_potion_story(story);
        let Some(s) = self.cinema.as_mut().filter(|s| s.active) else {
            return;
        };
        let Some(id) = s.beat.dialogue() else {
            return;
        };
        if let Some((line, _)) = story.progress(id) {
            s.line = line;
            if matches!(s.beat, Beat::Mushroom | Beat::Spice)
                && line == if s.beat == Beat::Mushroom { 2 } else { 3 }
            {
                s.final_line.get_or_insert(s.time);
                if s.beat == Beat::Spice {
                    s.cabinet.get_or_insert(s.time);
                }
            }
        }
    }
    fn open_cabinet(&mut self, leaf: usize, skip: bool) {
        let o = self
            .objects
            .iter_mut()
            .find(|o| o.name == format!("jumbo_door{leaf}"))
            .unwrap();
        o.motion = Motion::Open {
            angle: if leaf == 1 { -75. } else { 70. },
            duration: if skip {
                0.1
            } else if leaf == 1 {
                3.
            } else {
                3.2
            },
        };
        o.time = 0.;
    }
    pub(super) fn advance_scene(
        &mut self,
        dt: f32,
        world: &World,
        player: &mut Player,
    ) -> Result<()> {
        if dt <= 0. || !dt.is_finite() {
            return Ok(());
        }
        let Some(s) = &mut self.cinema else {
            return Ok(());
        };
        let before = s.time;
        s.time = (s.time + dt.min(0.1)).min(1000.);
        if !s.active {
            if matches!(s.beat, Beat::Mushroom | Beat::Spice)
                && before < self.cinema_data.lengths["vanish01"]
                && s.time >= self.cinema_data.lengths["vanish01"]
            {
                s.cue(2, 0);
            }
            return Ok(());
        }
        let home = *s.home.get_or_insert(Transform {
            translation: player.feet,
            rotation: Quat::from_rotation_z(player.script_facing),
        });
        player.feet = home.translation;
        player.velocity = Vec3::ZERO;
        player.script_motion = 1;
        player.release_rope();
        player.cancel_climb();
        let mut opens = Vec::new();
        if let Some(t) = s.cabinet {
            for (bit, at) in [(0, 0.5), (1, 1.)] {
                if s.time - t >= at && s.cues & (1 << bit) == 0 {
                    s.cue(bit, 1);
                    opens.push(bit as usize + 1);
                }
            }
        }
        if s.beat == Beat::Warp && s.time >= 2.5 + self.cinema_data.lengths["vanish01"] {
            s.cue(2, 0);
        }
        potion::advance(s, &self.cinema_data);
        if s.beat == Beat::Dice {
            if s.time >= 0.5 { s.cue(3, 5); }
            if s.dialogue_done {
                s.final_line.get_or_insert(s.time);
                s.cue(4, 6);
            }
        }
        let ready = match s.beat {
            Beat::Bleachers => s.time >= 11.3 + self.cinema_data.lengths["letsgo"],
            Beat::Dice => s.dialogue_done && s.action() >= 1.,
            Beat::Growth => s.time >= 9.4,
            Beat::Final => s
                .potion
                .as_ref()
                .unwrap()
                .departure
                .is_some_and(|at| s.time - at >= 4.),
            Beat::Warp => s.time >= self.cinema_data.warp_end() - 0.5,
            Beat::Mushroom => s.dialogue_done && s.action() >= 13.6,
            Beat::Spice => {
                s.dialogue_done
                    && s.time
                        >= 0.6
                            + self.cinema_data.spice_return()
                            + self.cinema_data.lengths["handout"]
                    && s.cabinet.is_some_and(|t| s.time - t >= 6.)
            }
        };
        if ready {
            s.ending.get_or_insert(s.time);
        }
        let done = s.ending.is_some_and(|t| s.time - t >= 0.6);
        for leaf in opens {
            self.open_cabinet(leaf, false);
        }
        if done {
            self.finish_scene(world, player)?;
        }
        Ok(())
    }
    fn finish_scene(&mut self, world: &World, player: &mut Player) -> Result<()> {
        let Some(s) = self.cinema.as_mut().filter(|s| s.active) else {
            return Ok(());
        };
        if let Some(home) = s.home {
            crate::cinematic::land_player(player, world, home)?;
        } else {
            player.script_motion = 0;
            player.velocity = Vec3::ZERO;
            player.release_rope();
            player.cancel_climb();
        }
        let beat = s.beat;
        s.active = false;
        s.time = 0.;
        s.ending = None;
        s.final_line = None;
        s.potion = None;
        if matches!(beat, Beat::Growth | Beat::Final) {
            s.sounds.clear();
        }
        // Relative clocks no longer refer to the completed presentation.
        s.cabinet = s.cabinet.map(|_| 0.);
        match beat {
            Beat::Bleachers => (),
            Beat::Dice => { self.event(DICE); }
            Beat::Growth => {
                if self.quest.finish_growing() {
                    for b in &mut self.boojums[8..10] {
                        b.active = true;
                    }
                }
            }
            Beat::Final => {
                self.quest.dialogue_finished(FINAL);
                self.quest.finish_mixing();
            }
            Beat::Mushroom => {
                self.completed_dialogue(MUSHROOM);
            }
            Beat::Spice => {
                self.completed_dialogue(SPICE);
            }
            Beat::Warp => {
                for b in &mut self.boojums[6..8] {
                    b.active = true;
                }
            }
        }
        Ok(())
    }
    pub fn skip_scene(
        &mut self,
        world: &World,
        player: &mut Player,
        story: &mut Story,
    ) -> Result<bool> {
        let Some(s) = self.cinema.as_ref().filter(|s| s.active) else {
            return Ok(false);
        };
        let beat = s.beat;
        if beat == Beat::Bleachers && s.time < 1.5 { return Ok(false); }
        if beat == Beat::Spice {
            self.cinema.as_mut().unwrap().cabinet = Some(0.);
            self.open_cabinet(1, true);
            self.open_cabinet(2, true);
        }
        self.finish_scene(world, player)?;
        if let Some(id) = beat.dialogue() {
            story.finish_sequence(id);
        }
        Ok(true)
    }
    pub fn scene_camera(&self) -> Option<Camera> {
        let s = self.cinema.as_ref().filter(|s| s.active)?;
        let (name, time) = match s.beat {
            Beat::Bleachers if s.time >= 9. => ("skool2_gnomew1", s.time - 9.),
            Beat::Bleachers => ("skool1_bleach_p1", s.time),
            Beat::Dice => ("skool2_dicep1", (s.time - 0.5).max(0.)),
            Beat::Growth => ("skool2_path4", (s.time - 0.5).max(0.)),
            Beat::Final => match s.potion.as_ref().and_then(|p| p.presentation) {
                Some(at) => ("skool2_path5a", s.time - at),
                None => ("skool2_path5", (s.time - 0.5).max(0.)),
            },
            Beat::Mushroom if s.line >= 1 => ("skool2_path1b", (s.action() - 9.).max(0.)),
            Beat::Mushroom => ("skool2_path1", (s.time - 0.5).max(0.)),
            Beat::Warp => ("skool2_gnomew2", (s.time - 2.5).max(0.)),
            Beat::Spice if s.cabinet.is_some() => {
                let t = s.time - s.cabinet.unwrap();
                if t < 6. {
                    ("skool2_path7", t)
                } else {
                    ("skool2_path7a", t - 6.)
                }
            }
            Beat::Spice => ("skool2_path3", (s.time - 0.5).max(0.)),
        };
        let p = self.cinema_data.tracks[name].sample(time, false);
        Some(Camera {
            eye: p.translation,
            target: p.translation + p.rotation * Vec3::X * 100.,
            up: p.rotation * Vec3::Z,
        })
    }
    pub fn scene_fade(&self) -> (Color, f32) {
        let a = self.cinema.as_ref().map_or(0., |s| {
            if !s.active {
                return (1. - s.time / 0.5).clamp(0., 1.);
            }
            let t = if s.beat == Beat::Warp {
                s.time - 2.
            } else {
                s.time
            };
            let start = if s.beat == Beat::Bleachers || t < 0. {
                0.
            } else if t < 0.5 {
                t / 0.5
            } else {
                (1. - (t - 0.5) / 0.5).max(0.)
            };
            let middle = potion::fade(s, &self.cinema_data);
            start
                .max(middle)
                .max(s.ending.map_or(0., |at| ((s.time - at) / 0.5).min(1.)))
        });
        (WHITE, a)
    }
}

pub struct Art {
    rage_cat: Puppet,
    dice_cat: Puppet,
    alice: Puppet,
    gnome: Puppet,
    boojum: Puppet,
    mushroom: Prop,
    spice: Prop,
    fire: crate::particles::Attached,
    potion: potion::Art,
}
impl Art {
    pub fn load(
        assets: &mut Assets,
        specs: &BTreeMap<String, crate::texture::MaterialSpec>,
    ) -> Result<Self> {
        let mut gnome = Puppet::load(assets, "c_gnomeold", GNOME, specs)?;
        gnome.show_attachments(false);
        Ok(Self {
            dice_cat: Puppet::load(assets, "c_cheshire", CAT, specs)?,
            rage_cat: Puppet::load(assets, "c_cheshire", &["sit_idle1"], specs)?,
            potion: potion::Art::load(assets, specs)?,
            alice: Puppet::load(assets, "alice", ALICE, specs)?,
            gnome,
            boojum: Puppet::load(assets, "c_boojum", &["fly"], specs)?,
            mushroom: Prop::load(assets, "tart", specs)?,
            spice: Prop::load(assets, "taffy", specs)?,
            fire: crate::particles::Attached::load_clip_bursts(
                assets,
                "fx_pickup",
                Some("on"),
                0.05,
                specs,
            )?
            .context("School Gnome effect missing")?,
        })
    }
    pub fn story_pose(&mut self, story: &Story) {
        self.dice_cat.mouth(story.mouth(&["dice_cat"]));
        self.rage_cat.mouth(story.mouth(&["rage_cat"]));
        self.alice.mouth(story.mouth(&["fakeplayer"]));
        self.gnome
            .mouth(story.mouth(&["old_gnome_1", "old_gnome_2"]));
    }
    pub fn draw(
        &mut self,
        school: &School2,
        bright: bool,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
    ) -> bool {
        if let Some(hint) = &school.rage_hint {
            if hint.opacity() > 0. {
                self.rage_cat.atmosphere(atmosphere, camera);
                self.rage_cat.watch(hint.watch.clone());
                self.rage_cat.draw_dissolving("sit_idle1", hint.time.unwrap_or(0.), true,
                    school.rage_pose, 1., bright, 1. - hint.opacity());
            }
        }
        let Some(s) = &school.cinema else {
            return false;
        };
        let d = &school.cinema_data;
        if matches!(s.beat, Beat::Bleachers | Beat::Dice) {
            if !s.active { return false; }
            if let Some(home) = s.home {
                self.alice.atmosphere(atmosphere, camera);
                let target = d.points[if s.beat == Beat::Dice { "dice_cat" } else { "bleach8" }];
                self.alice.watch(watch(home, target.translation));
                self.alice.draw("idle_stand", s.time, true, home, 1., bright);
                if s.beat == Beat::Dice {
                    let pose = d.points["dice_cat"];
                    self.dice_cat.atmosphere(atmosphere, camera);
                    self.dice_cat.watch(watch(pose, home.translation + Vec3::Z * 55.));
                    let mut age = (s.time - 1.5).max(0.);
                    let mut clip = "sit_idle1";
                    for c in CAT {
                        clip = c;
                        if age < d.lengths[*c] { break; }
                        age -= d.lengths[*c];
                        clip = "sit_idle1";
                    }
                    let alpha = ((s.time - 0.5) / 2.).clamp(0., 1.)
                        * s.final_line.map_or(1., |at| (1. - (s.time - at) / 2.).max(0.));
                    self.dice_cat.draw_dissolving(clip, age, clip == "sit_idle1", pose, 1., bright, 1. - alpha);
                } else {
                    let pose = d.points["old_gnome_1"];
                    self.gnome.atmosphere(atmosphere, camera);
                    self.gnome.watch(watch(pose, home.translation + Vec3::Z * 55.));
                    let age = s.time - 11.3;
                    let gesture = age >= 0. && age < d.lengths["letsgo"];
                    self.gnome.draw(if gesture { "letsgo" } else { "idle" },
                        if gesture { age } else { s.time }, !gesture, pose, 1., bright);
                }
            }
            return s.beat == Beat::Bleachers;
        }
        if matches!(s.beat, Beat::Growth | Beat::Final) {
            return self.draw_potion(school, bright, atmosphere, camera);
        }
        if !s.active
            && (s.beat == Beat::Warp
                || !matches!(school.quest.stage, Stage::Battle | Stage::Jumbogrow))
        {
            return false;
        }
        for p in [&mut self.alice, &mut self.gnome, &mut self.boojum] {
            p.atmosphere(atmosphere, camera);
            p.watch(Default::default());
        }
        let (pose, clip, t, looping, scale, fire) = if !s.active {
            let shrinking = s.time - d.lengths["vanish01"];
            let pos = d.points[if s.beat == Beat::Mushroom {
                "old_gnome_1"
            } else {
                "gnome_killpos1"
            }];
            (
                pos,
                if shrinking < 0. {
                    "vanish01"
                } else {
                    "vanish02"
                },
                if shrinking < 0. { s.time } else { shrinking },
                false,
                (1. - (shrinking.max(0.) / 0.1).floor() * 0.04).clamp(0., 1.),
                Some(s.time),
            )
        } else if s.beat == Beat::Warp {
            let t = (s.time - 2.5).max(0.);
            let mut p = d.points["gnome_killpos1"];
            if t >= 4. {
                let end = d.points["gnome_pos1"];
                let delta = end.translation - p.translation;
                let f = ((t - 4.) / d.walk_time).min(1.);
                p.translation = p.translation.lerp(end.translation, f);
                p.rotation = if f < 1. {
                    Quat::from_rotation_z(delta.y.atan2(delta.x))
                } else {
                    end.rotation
                };
            }
            let clip = if t < d.lengths["vanish01"] {
                "vanish01"
            } else if t < 4. {
                "vanish02"
            } else if t < 4. + d.walk_time {
                "walk"
            } else {
                "idle"
            };
            (
                p,
                clip,
                if t >= 4. {
                    t - 4.
                } else if clip == "vanish02" {
                    t - d.lengths["vanish01"]
                } else {
                    t
                },
                clip == "walk" || clip == "idle",
                (((t - d.lengths["vanish01"]) / 0.1).floor() * 0.04).clamp(0., 1.),
                if s.time >= 2.7 {
                    Some(s.time - 2.7)
                } else {
                    None
                },
            )
        } else {
            let t = (s.time - if s.beat == Beat::Mushroom { 0.5 } else { 0.6 }).max(0.);
            let (clip, at, looping) = d.gnome_clip(s.beat, t);
            let gp = d.points[if s.beat == Beat::Mushroom {
                "old_gnome_1"
            } else {
                "gnome_killpos1"
            }];
            let ap = d.points[if s.beat == Beat::Mushroom {
                "alice_mush1"
            } else {
                "alice_killpos1"
            }];
            let (aclip, atime) = if s.beat == Beat::Mushroom && (7. ..10.).contains(&t) {
                ("idle_stand_rocktoes", t - 7.)
            } else if s.beat == Beat::Spice
                && (6. ..6. + d.lengths["idle_stand_tiptoes"]).contains(&t)
            {
                ("idle_stand_tiptoes", t - 6.)
            } else {
                ("idle_stand", t)
            };
            let attention = if s.beat == Beat::Mushroom && s.action() >= 8.5 {
                Some(d.points["get_booj1"].translation)
            } else if s.beat == Beat::Spice && s.cabinet.is_some_and(|at| s.time - at < 6.) {
                Some(d.points["jumbo_door2"].translation)
            } else {
                None
            };
            self.alice.watch(watch(
                ap,
                attention.unwrap_or(gp.translation + Vec3::Z * 48.),
            ));
            self.gnome.watch(watch(
                gp,
                attention.unwrap_or(ap.translation + Vec3::Z * 55.),
            ));
            self.alice
                .draw(aclip, atime, aclip == "idle_stand", ap, 1., bright);
            let held = t >= d.handout(s.beat) + 1.
                && (s.beat == Beat::Mushroom || t < d.spice_return() + 1.);
            if held {
                if let Some(tag) = if looping {
                    self.gnome.looping_tag("tag_pipe", clip, at, gp, 1.)
                } else {
                    self.gnome.tag("tag_pipe", clip, at, gp, 1.)
                } {
                    if s.beat == Beat::Mushroom {
                        self.mushroom.draw(tag, 0.3, bright);
                    } else {
                        self.spice.draw(tag, 0.4, bright);
                    }
                }
            }
            if s.beat == Beat::Mushroom {
                for offset in [0., 1.2, 2.4] {
                    let age = s.action() - 10. - offset;
                    if age >= 0. {
                        self.boojum
                            .draw("fly", age, true, d.boojums.sample(age, true), 1., bright);
                    }
                }
            }
            (gp, clip, at, looping, 1., None)
        };
        if s.active && s.beat == Beat::Warp {
            if let Some(home) = s.home {
                self.alice
                    .draw("idle_stand", s.time, true, home, 1., bright);
            }
        }
        if scale > 0. {
            self.gnome.draw(clip, t, looping, pose, scale, bright);
        }
        if let Some(age) = fire.filter(|t| *t < 5.) {
            let at = d.points[if s.beat == Beat::Mushroom {
                "gnome_fire1"
            } else {
                "gnome_fire2"
            }];
            crate::render::depth_read_only(|| {
                self.fire
                    .draw(age, 1., |_, _| at, |_, _, v| v, camera, atmosphere)
            });
        }
        true
    }
}
fn watch(pose: Transform, target: Vec3) -> crate::facial::Watch {
    let mut w = crate::facial::Watch::default();
    w.update(
        1.,
        Some(pose.rotation.conjugate() * (target - pose.translation - Vec3::Z * 50.)),
    );
    w
}
