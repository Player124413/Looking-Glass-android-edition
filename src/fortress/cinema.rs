//! Reviewed Fortress scenes, read from local markers/cameras; never a script VM.
use super::spline::Spline;
use crate::{
    assets::Assets,
    bsp::Bsp,
    cinematic::{Camera, Track},
    collision::World,
    interaction::vector,
    movement::Player,
    npc::Puppet,
    skeletal::Transform,
    story::Story,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

pub const ARRIVAL: &str = "Fortress1_Start";
pub const BOOJUM: &str = "Fortress1_Boojum_Attack";
const CAVE: f32 = 10.5;
const EXTERIOR: f32 = 34.5;
const BRIDGE: f32 = 44.8;
const CHASE: f32 = 51.3;
const SPLIT: f32 = 64.5;
const LAND: f32 = 82.5;
const END: f32 = 83.5;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Beat {
    Arrival,
    Boojum,
}
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub version: u8,
    pub beat: Option<Beat>,
    pub time: f32,
    pub arrival_done: bool,
    pub boojum_done: bool,
    pub dialogue_done: bool,
}
impl State {
    pub fn fresh(returning: bool) -> Self {
        Self {
            version: 1,
            arrival_done: returning,
            ..Default::default()
        }
    }
    pub fn active(&self) -> bool {
        self.beat.is_some()
    }
    pub fn scene_id(&self) -> Option<&'static str> {
        self.beat.map(|b| match b {
            Beat::Arrival => ARRIVAL,
            Beat::Boojum => BOOJUM,
        })
    }
    pub fn validate(&self, returning: bool) -> Result<()> {
        ensure!(
            self.version <= 1
                && self.time.is_finite()
                && (0. ..=300.).contains(&self.time)
                && (self.beat != Some(Beat::Arrival) || !returning && !self.arrival_done)
                && (self.beat != Some(Beat::Boojum) || returning && !self.boojum_done),
            "Invalid Fortress cinematic state"
        );
        Ok(())
    }
}
struct Node {
    p: Vec3,
    time: f32,
    visible: bool,
}
struct Path {
    nodes: Vec<Node>,
    looping: bool,
    spline: Spline,
}
impl Path {
    fn load(map: &Bsp, first: &str) -> Result<Self> {
        let mut next = Some(first);
        let mut nodes: Vec<Node> = Vec::new();
        let mut seen = BTreeSet::new();
        let mut time = 0.;
        let mut visible = true;
        let mut looping = false;
        let mut controls = Vec::new();
        while let Some(name) = next {
            if seen.contains(name) {
                ensure!(name == first, "Unexpected Fortress spline cycle");
                looping = true;
                nodes.push(Node {
                    p: nodes[0].p,
                    time,
                    visible: true,
                });
                break;
            }
            ensure!(
                seen.insert(name) && nodes.len() < 256,
                "Invalid Fortress spline"
            );
            let e = map
                .entities
                .iter()
                .find(|e| e.get("targetname").is_some_and(|n| n == name))
                .with_context(|| format!("Missing Fortress spline node {name}"))?;
            let p = e
                .get("origin")
                .and_then(|s| vector(s))
                .context("Invalid spline position")?;
            let speed = e.get("speed").map_or(Ok(1.), |s| s.parse::<f32>())?;
            ensure!(
                p.is_finite() && p.abs().max_element() < 100000. && (0.01..=100.).contains(&speed),
                "Invalid spline data"
            );
            if let Some(thread) = e.get("thread") {
                if thread.ends_with("_hide") {
                    visible = false;
                }
                if thread.ends_with("_show") {
                    visible = true;
                }
            }
            nodes.push(Node { p, time, visible });
            controls.push((p, Quat::IDENTITY, speed));
            time += 1. / speed;
            next = e.get("target").map(String::as_str);
        }
        ensure!(nodes.len() >= 2, "Short Fortress spline");
        Ok(Self {
            nodes,
            looping,
            spline: Spline::new(controls, looping || first == "ship_finalpath"),
        })
    }
    fn sample(&self, time: f32) -> (Transform, bool) {
        let end = self.nodes.last().unwrap().time;
        let time = if self.looping {
            time.max(0.).rem_euclid(end)
        } else {
            time.clamp(0., end)
        };
        let i = self
            .nodes
            .iter()
            .position(|n| n.time > time)
            .unwrap_or(self.nodes.len() - 1)
            .max(1);
        let a = &self.nodes[i - 1];
        let b = &self.nodes[i];
        let f = ((time - a.time) / (b.time - a.time)).clamp(0., 1.);
        let direction = b.p - a.p;
        (
            Transform {
                translation: a.p.lerp(b.p, f),
                rotation: Quat::from_rotation_z(direction.y.atan2(direction.x)),
            },
            a.visible,
        )
    }
    fn curved(&self, time: f32) -> (Transform, bool) {
        let p = self.spline.parameter(time);
        // The first callback runs at start. Index 1 advances the callback cursor;
        // later index changes call the preceding node before advancing it again.
        // Derive visibility from time so seeking never replays events.
        let index = if self.looping && p >= 0. {
            (p.floor() as i32 - 1).rem_euclid(self.nodes.len() as i32 - 1) as usize
        } else {
            ((p.floor() - 1.).max(0.) as usize).min(self.nodes.len() - 1)
        };
        (self.spline.sample(time, true), self.nodes[index].visible)
    }
}
pub struct Data {
    tracks: BTreeMap<&'static str, Track>,
    curves: BTreeMap<&'static str, Spline>,
    paths: BTreeMap<&'static str, Path>,
    points: BTreeMap<String, Transform>,
    pub landing: Vec3,
    line_starts: [f32; 4],
    chase_distance: f32,
    chase_yaw: f32,
    ship_height: f32,
    camera_world: World,
    cave_clearance: Vec<f32>,
}
impl Data {
    pub fn load(assets: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut tracks = BTreeMap::new();
        for name in [
            "fortress1_begin",
            "watch_ship1",
            "watch_ship2",
            "spotter_watch1",
            "drawbridge_path",
            "watch_airship4",
            "watch_airship5",
            "alice_drop1",
            "fortress1_s1path1",
            "ship_path3",
        ] {
            tracks.insert(name, Track::load(assets, name)?);
        }
        let mut paths = BTreeMap::new();
        for name in [
            "ship_path1",
            "ship_skip1",
            "ship_finalpath",
            "ship_splitup",
            "stal_path1",
            "stal_path2",
            "stal_path3",
            "stal_path4",
            "stal_path5",
            "s1_boojpath1",
            "s1_boojpath2",
        ] {
            paths.insert(name, Path::load(map, name)?);
        }
        let points = map
            .entities
            .iter()
            .filter_map(|e| {
                Some((
                    e.get("targetname")?.clone(),
                    Transform {
                        translation: vector(e.get("origin")?)?,
                        rotation: Quat::from_rotation_z(
                            e.get("angle")
                                .and_then(|s| s.parse::<f32>().ok())
                                .unwrap_or(0.)
                                .to_radians(),
                        ),
                    },
                ))
            })
            .collect::<BTreeMap<_, _>>();
        let mut line_starts = [21., 0., 0., 75.5];
        for (i, path) in ["gnome/elder/vo/go1008.wav", "alice/vo/alcz1018.wav"]
            .iter()
            .enumerate()
        {
            let wav = hound::WavReader::new(std::io::Cursor::new(
                assets.read(&format!("sound/character/{path}"))?,
            ))?;
            line_starts[i + 1] = line_starts[i]
                + wav.duration() as f32 / wav.spec().sample_rate as f32
                + 0.35
                + if i == 1 { 8. } else { 0. };
        }
        let world = World::from_bsp(map)?;
        let curves = tracks
            .iter()
            .map(|(name, track)| {
                (
                    *name,
                    if *name == "ship_path3" {
                        Spline::new(track.controls().collect(), false)
                    } else {
                        Spline::camera_track(track.controls().collect())
                    },
                )
            })
            .collect();
        let camera_entity = map
            .entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|n| n == "ship_close_cam1"))
            .context("Missing ship follow camera")?;
        let chase_distance = camera_entity
            .get("follow_distance")
            .context("Missing follow distance")?
            .parse::<f32>()?;
        let chase_yaw = camera_entity
            .get("follow_yaw")
            .context("Missing follow yaw")?
            .parse::<f32>()?
            .to_radians();
        ensure!(
            chase_distance.is_finite()
                && chase_distance > 16.
                && chase_distance < 1024.
                && chase_yaw.is_finite(),
            "Invalid ship follow camera"
        );
        let def = crate::skeletal::Definition::load(assets, "models/gnome_airship.tik")?;
        let skeleton = crate::skeletal::Skeleton::parse(
            &assets.read(&format!("{}/{}", def.path, def.model))?,
        )?;
        let animation = crate::skeletal::Animation::parse_tags(
            &assets.read(&format!("{}/{}", def.path, def.animations["fly"]))?,
            &skeleton.unskinned_tags(),
        )?
        .0;
        let ship_height = animation.frames[0].max.z * def.scale;
        let marker = points
            .get("camerarest")
            .context("Missing Fortress landing")?
            .translation;
        let trace = world.body_trace(marker, marker - Vec3::Z * 512.);
        ensure!(
            !trace.start_solid && trace.fraction < 1. && trace.normal.z > 0.65,
            "Fortress landing has no floor"
        );
        let landing = marker - Vec3::Z * 512. * trace.fraction + Vec3::Z * 0.1;
        ensure!(world.body_clear(landing), "Fortress landing obstructed");
        let mut data = Self {
            tracks,
            curves,
            paths,
            points,
            landing,
            line_starts,
            chase_distance,
            chase_yaw,
            ship_height,
            camera_world: world,
            cave_clearance: Vec::new(),
        };
        data.clear_cave_camera(map)?;
        Ok(data)
    }
    /// Keep the authored viewing direction, but make room for the moving
    /// foreground pillars. Bake a conservative smooth lateral correction once;
    /// playback/seek/load then need no camera history and cannot jitter on contact.
    fn clear_cave_camera(&mut self, map: &Bsp) -> Result<()> {
        let obstacles = map
            .entities
            .iter()
            .filter(|e| {
                e.get("targetname").is_some_and(|n| {
                    matches!(n.as_str(), "stal1" | "stal2" | "stal3" | "stal4" | "stal5")
                })
            })
            .map(|e| {
                Ok((
                    e["targetname"].as_str(),
                    vector(&e["origin"]).context("Invalid pillar origin")?,
                    crate::collision::Collider::model(
                        map,
                        e["model"].trim_start_matches('*').parse()?,
                        Vec3::ZERO,
                        Quat::IDENTITY,
                        true,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let frames = ((EXTERIOR - CAVE) * 60.) as usize + 1;
        let mut needed = Vec::with_capacity(frames);
        let mut choices = Vec::with_capacity(frames);
        for frame in 0..frames {
            let t = frame as f32 / 60.;
            let c = self.curves["watch_ship1"].camera(t);
            let left = c.up.cross(c.target - c.eye).normalize();
            let state = State {
                beat: Some(Beat::Arrival),
                time: CAVE + t,
                ..Default::default()
            };
            let placed = obstacles
                .iter()
                .filter_map(|(name, base, collider)| {
                    self.brush(name, *base, &state)
                        .map(|(p, q)| (collider, p, q))
                })
                .collect::<Vec<_>>();
            let clear = |eye: Vec3| {
                !self
                    .camera_world
                    .sweep_geometry(eye, eye, Vec3::splat(6.))
                    .start_solid
                    && placed.iter().all(|(collider, p, q)| {
                        let local = q.inverse() * (eye - *p);
                        !collider.trace(local, local, Vec3::splat(6.)).start_solid
                    })
            };
            let row = (0..=128)
                .map(|n| clear(c.eye + left * n as f32))
                .collect::<Vec<_>>();
            let offset = row
                .iter()
                .position(|v| *v)
                .context("Fortress cave camera cannot clear moving scenery")?;
            needed.push(offset as f32);
            choices.push(row);
        }
        // Dilation followed by a positive smoothing kernel stays above the
        // clearance required at every sample, while anticipating each crossing.
        const RADIUS: usize = 30;
        for _ in 0..32 {
            let envelope = (0..frames)
                .map(|i| {
                    needed[i.saturating_sub(RADIUS)..=(i + RADIUS).min(frames - 1)]
                        .iter()
                        .copied()
                        .fold(0., f32::max)
                })
                .collect::<Vec<_>>();
            self.cave_clearance = (0..frames)
                .map(|i| {
                    let mut total = 0.;
                    let mut weight = 0.;
                    for j in 0..=2 * RADIUS {
                        let w = (RADIUS + 1 - j.abs_diff(RADIUS)) as f32;
                        let index = (i + j).saturating_sub(RADIUS).min(frames - 1);
                        total += envelope[index] * w;
                        weight += w;
                    }
                    total / weight
                })
                .collect();
            let mut clear = true;
            for (i, offset) in self.cave_clearance.iter().enumerate() {
                if !choices[i][offset.floor() as usize] || !choices[i][offset.ceil() as usize] {
                    clear = false;
                    needed[i] = (offset.ceil() as usize..=128)
                        .find(|n| choices[i][*n])
                        .context("Fortress cave correction blocked by another pillar")?
                        as f32;
                }
            }
            if clear {
                return Ok(());
            }
        }
        anyhow::bail!("Fortress cave clearance did not converge")
    }
    pub fn begin(&self, state: &mut State, story: &mut Story) {
        if !state.arrival_done && state.beat.is_none() {
            state.beat = Some(Beat::Arrival);
            state.time = 0.;
            story.trigger(ARRIVAL);
        }
    }
    pub fn prepare_story(&self, state: &State, story: &mut Story) -> bool {
        if state.beat != Some(Beat::Arrival) {
            return true;
        }
        story.line_limit = Some(
            self.line_starts
                .iter()
                .rposition(|t| state.time >= *t)
                .unwrap_or(0),
        );
        // Release the final line once it finishes, allowing the shared completion callback.
        if state.time >= self.line_starts[3] {
            story.line_limit = None;
        }
        state.time >= self.line_starts[0]
    }
    pub fn advance(&self, s: &mut State, dt: f32, player: &mut Player) {
        if !s.active() || dt <= 0. {
            return;
        }
        s.time = (s.time + dt.min(0.1)).min(300.);
        player.velocity = Vec3::ZERO;
        if s.beat == Some(Beat::Arrival) && s.time >= LAND {
            player.feet = self.landing;
            player.script_facing = std::f32::consts::PI;
        }
        if (s.beat == Some(Beat::Arrival) && s.time >= END && s.dialogue_done)
            || (s.beat == Some(Beat::Boojum) && s.time >= 5.5)
        {
            self.finish(s, player);
        }
    }
    fn finish(&self, s: &mut State, player: &mut Player) {
        match s.beat {
            Some(Beat::Arrival) => {
                player.feet = self.landing;
                player.script_facing = std::f32::consts::PI;
                player.grounded = true;
                player.ground_normal = Vec3::Z;
                s.arrival_done = true;
                s.dialogue_done = true;
            }
            Some(Beat::Boojum) => s.boojum_done = true,
            None => return,
        }
        player.velocity = Vec3::ZERO;
        s.time = if s.beat == Some(Beat::Arrival) {
            END
        } else {
            5.5
        };
        s.beat = None;
    }
    pub fn skip(&self, s: &mut State, player: &mut Player, story: &mut Story) -> bool {
        let Some(id) = s.scene_id() else {
            return false;
        };
        self.finish(s, player);
        story.finish_sequence(id);
        true
    }
    pub fn ship(&self, time: f32) -> Transform {
        if time < CAVE {
            let mut pose = self.paths["ship_path1"].curved(time).0;
            pose.rotation = Quat::from_rotation_z(std::f32::consts::PI);
            pose
        } else if time < EXTERIOR {
            let mut pose = self.paths["ship_skip1"].curved(time - CAVE - 0.5).0;
            pose.rotation = Quat::from_rotation_z(std::f32::consts::PI);
            pose
        } else if time < BRIDGE + 5.9 {
            self.curves["ship_path3"].sample(time - EXTERIOR, true)
        } else if time < SPLIT {
            self.paths["ship_finalpath"].curved(time - BRIDGE - 5.9).0
        } else {
            self.paths["ship_splitup"].curved(time - SPLIT).0
        }
    }
    pub fn camera(&self, s: &State) -> Option<Camera> {
        let t = s.time;
        Some(match s.beat? {
            Beat::Boojum => self.tracks["fortress1_s1path1"].sample(t - 0.3),
            Beat::Arrival if t < CAVE => self.curves["fortress1_begin"].camera(t),
            Beat::Arrival if t < EXTERIOR => {
                let mut c = self.curves["watch_ship1"].camera(t - CAVE);
                let frame = (t - CAVE) * 60.;
                let i = (frame as usize).min(self.cave_clearance.len() - 1);
                let a = self.cave_clearance[i];
                let b = self.cave_clearance[(i + 1).min(self.cave_clearance.len() - 1)];
                let offset = a + (b - a) * frame.fract();
                let shift = c.up.cross(c.target - c.eye).normalize() * offset;
                c.eye += shift;
                c.target += shift;
                c
            }
            Beat::Arrival if t < EXTERIOR + 7. => self.curves["watch_ship2"].camera(t - EXTERIOR),
            Beat::Arrival if t < EXTERIOR + 10. => {
                self.curves["spotter_watch1"].camera(t - EXTERIOR - 7.)
            }
            Beat::Arrival if t < CHASE => self.curves["drawbridge_path"].camera(t - EXTERIOR - 10.),
            Beat::Arrival if t < SPLIT => {
                let ship = self.ship(t);
                let anchor = ship.translation + Vec3::Z * self.ship_height;
                let (yaw, pitch, _) = ship.rotation.to_euler(EulerRot::ZYX);
                let forward = Quat::from_rotation_z(yaw + self.chase_yaw)
                    * Quat::from_rotation_y(pitch)
                    * Vec3::X;
                let desired = anchor - forward * self.chase_distance + Vec3::Z * 24.;
                let trace = self
                    .camera_world
                    .sweep_geometry(anchor, desired, Vec3::splat(2.));
                let eye = anchor.lerp(desired, trace.fraction) + forward * 16.;
                Camera::look(eye, ship.translation + Vec3::Z * 32.)
            }
            Beat::Arrival if t < SPLIT + 8. => self.curves["watch_airship4"].camera(t - SPLIT),
            Beat::Arrival if t < LAND => self.curves["watch_airship5"].camera(t - SPLIT - 8.),
            Beat::Arrival => self.curves["alice_drop1"].camera(0.),
        })
    }
    pub fn fade(&self, s: &State) -> (Color, f32) {
        if s.beat == Some(Beat::Boojum) {
            let t = s.time;
            return (
                WHITE,
                if t < 0.3 {
                    t / 0.3
                } else if t < 1. {
                    1.
                } else if t < 1.5 {
                    (1.5 - t) / 0.5
                } else {
                    ((t - 5.) / 0.5).clamp(0., 1.)
                },
            );
        }
        if s.beat != Some(Beat::Arrival) {
            return (BLACK, 0.);
        }
        let t = s.time;
        if t >= LAND - 1. {
            return (
                WHITE,
                if t < LAND {
                    (t - LAND + 1.).clamp(0., 1.)
                } else {
                    (LAND + 1. - t).clamp(0., 1.)
                },
            );
        }
        let mut alpha = (1. - t / 3.).clamp(0., 1.);
        for (cut, start) in [(CAVE, CAVE - 0.5), (EXTERIOR, EXTERIOR - 0.5)] {
            if (start..cut + 1.5).contains(&t) {
                alpha = if t < cut + 0.5 {
                    ((t - start) / 0.5).min(1.)
                } else {
                    1. - (t - cut - 0.5)
                };
            }
        }
        (BLACK, alpha)
    }
    pub fn brush(&self, name: &str, base: Vec3, s: &State) -> Option<(Vec3, Quat)> {
        if s.beat != Some(Beat::Arrival) {
            return None;
        }
        let t = s.time;
        if name == "drawbridge" {
            return Some((
                base - Vec3::X * 112.,
                Quat::from_rotation_y(
                    ((t - BRIDGE) / 5.).clamp(0., 1.) * std::f32::consts::FRAC_PI_2,
                ),
            ));
        }
        let index = ["stal1", "stal2", "stal3", "stal4", "stal5"]
            .iter()
            .position(|n| *n == name)?;
        let time = t - CAVE - [1.8, 0.8, 6.8, 8.3, 10.8][index];
        if !(CAVE..EXTERIOR).contains(&t) {
            return Some((Vec3::splat(-100000.), Quat::IDENTITY));
        }
        if time < 0. {
            return Some((base, Quat::IDENTITY));
        }
        let (p, visible) = self.paths[[
            "stal_path1",
            "stal_path2",
            "stal_path3",
            "stal_path4",
            "stal_path5",
        ][index]]
            .curved(time);
        Some((
            if visible {
                p.translation
            } else {
                Vec3::splat(-100000.)
            },
            Quat::IDENTITY,
        ))
    }
    pub fn sound_state(
        &self,
        s: &State,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        if s.beat != Some(Beat::Arrival) {
            return;
        }
        let origin = self.points["bridge_gong"].translation;
        if (BRIDGE..BRIDGE + 4.9).contains(&s.time) {
            loops.push(crate::audio::LoopCue {
                id: 400001,
                clock: Some(s.time - BRIDGE),
                path: "sound/ambience/special/drawbridge.wav",
                origin,
            });
        }
        clocks.push(crate::audio::world::Clock {
            key: "fortress-arrival",
            time: s.time,
            period: None,
            origin,
            cues: &[
                (BRIDGE, "sound/ambience/special/cookooclock.wav"),
                (BRIDGE, "sound/ambience/special/clock_gong.wav"),
                (BRIDGE + 1.5, "sound/ambience/special/clock_gong.wav"),
                (BRIDGE + 3., "sound/ambience/special/clock_gong.wav"),
                (BRIDGE + 4.5, "sound/ambience/special/clock_gong.wav"),
                (BRIDGE + 4.9, "sound/ambience/special/door_slam2.wav"),
            ],
        });
    }
}
impl Data {
    pub fn place_boojums(&self, time: f32, encounters: &mut crate::encounters::Encounters) {
        for (i, name) in ["s1_booj1", "s1_booj2"].iter().enumerate() {
            let pose = self.paths[["s1_boojpath1", "s1_boojpath2"][i]]
                .sample(time - (i + 1) as f32 * 0.1)
                .0;
            for actor in encounters.actors.iter_mut().filter(|a| a.name == *name) {
                if let crate::encounters::Enemy::Boojum(b) = &mut actor.enemy {
                    b.feet = pose.translation;
                    b.time = time;
                    let forward = pose.rotation * Vec3::X;
                    b.yaw = forward.y.atan2(forward.x);
                }
            }
        }
    }
}
pub struct Art {
    child: Puppet,
    ship: Puppet,
    pub(crate) alice: Puppet,
    gnome: Puppet,
    diamond: Puppet,
    club: Puppet,
    material: crate::character::SkinMaterial,
}
impl Art {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(assets)?;
        let mut alice = Puppet::load(assets, "alice", &["sit_airship", "ready"], &specs)?;
        alice.show_attachments(false);
        Ok(Self {
            child: Puppet::load(
                assets,
                "c_insanechildnew",
                &["antictrans_antic01", "jump"],
                &specs,
            )?,
            ship: Puppet::load(assets, "gnome_airship", &["fly"], &specs)?,
            alice,
            gnome: Puppet::load(assets, "c_gnomeold", &["ride"], &specs)?,
            diamond: Puppet::load(
                assets,
                "cardguard_diamond",
                &["idle", "run", "stand_attack"],
                &specs,
            )?,
            club: Puppet::load(assets, "cardguard_club", &["idle", "run"], &specs)?,
            material: crate::character::skin_material()?,
        })
    }
    pub fn story_pose(&mut self, story: &Story) {
        self.alice.mouth(story.mouth(&["fakeplayer"]));
        self.gnome.mouth(story.mouth(&["riding_gnome"]));
    }
    pub fn draw(
        &mut self,
        f: &super::Fortress,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
        fullbright: bool,
    ) {
        self.draw_omitting(f, atmosphere, camera, fullbright, None);
    }
    pub(super) fn draw_omitting(
        &mut self,
        f: &super::Fortress,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
        fullbright: bool,
        omit: Option<&str>,
    ) {
        let s = &f.state.cinema;
        let d = &f.cinema;
        if s.beat != Some(Beat::Arrival) {
            return;
        }
        self.material.atmosphere(atmosphere, camera);
        self.material.bind();
        let t = s.time;
        if t < CAVE && omit != Some("child") {
            let mut at = d.points["insane_pillar1"];
            let jump = ((t - 9.5) / 1.).clamp(0., 1.);
            at.translation = at
                .translation
                .lerp(d.points["insane_jump1"].translation, jump)
                + Vec3::Z * (jump * (1. - jump) * 160.);
            self.child.draw(
                if t < 9.5 {
                    "antictrans_antic01"
                } else {
                    "jump"
                },
                if t < 9.5 { t } else { t - 9.5 },
                t < 9.5,
                at,
                1.,
                fullbright,
            );
        }
        {
            if t < LAND {
                let ship = d.ship(t);
                if omit != Some("ship") {
                    self.ship.draw("fly", t, true, ship, 1., fullbright);
                }
                for (tag, actor, clip) in [
                    ("tag_alice", &mut self.alice, "sit_airship"),
                    ("tag_gnome", &mut self.gnome, "ride"),
                ] {
                    if omit == Some(tag) {
                        continue;
                    }
                    if let Some(pose) = self.ship.looping_tag(tag, "fly", t, ship, 1.) {
                        actor.draw(
                            clip,
                            t,
                            true,
                            Transform {
                                translation: pose.translation,
                                rotation: ship.rotation,
                            },
                            1.,
                            fullbright,
                        );
                    }
                }
            } else if omit != Some("tag_alice") {
                self.alice.draw(
                    "ready",
                    t - LAND,
                    true,
                    Transform {
                        translation: d.landing,
                        rotation: Quat::from_rotation_z(std::f32::consts::PI),
                    },
                    1.,
                    fullbright,
                );
            }
            if t < LAND - 1. {
                let mut spotter = d.points["cardguard_spotter1"];
                let walk = ((t - EXTERIOR - 9.5) / 2.).clamp(0., 1.);
                spotter.translation = spotter.translation.lerp(
                    d.points["guard_spotter_pos1"].translation - Vec3::Z * 8.,
                    walk,
                );
                let dir = d.ship(t).translation - spotter.translation;
                spotter.rotation = Quat::from_rotation_z(dir.y.atan2(dir.x));
                if omit != Some("spotter") {
                    self.club.draw(
                        if walk > 0. && walk < 1. {
                            "run"
                        } else {
                            "idle"
                        },
                        t,
                        true,
                        spotter,
                        1.,
                        fullbright,
                    );
                }
                for i in 0..4 {
                    let name = [
                        "bridge_guard1",
                        "bridge_guard2",
                        "tower_shooter1",
                        "tower_shooter2",
                    ][i];
                    if omit == Some(name) {
                        continue;
                    }
                    if t < if i < 2 { BRIDGE } else { CHASE } {
                        continue;
                    }
                    let mut pose = d.points[name];
                    let target = if i < 2 {
                        if t < CHASE + 10.2 {
                            ["bridge_guard_pos1", "bridge_guard_pos2"][i]
                        } else {
                            ["bridge_guard_pos3", "bridge_guard_pos4"][i]
                        }
                    } else if t < SPLIT + 11. {
                        ["tower_shooter1_pos1", "tower_shooter2_pos1"][i - 2]
                    } else {
                        ["tower_shooter1_pos2", "tower_shooter2_pos3"][i - 2]
                    };
                    let start = if i < 2 {
                        BRIDGE + 4.3 + i as f32 * 0.2
                    } else {
                        CHASE + 10.7 + (3 - i) as f32 * 0.5
                    };
                    let blend = ((t - start) / 2.).clamp(0., 1.);
                    pose.translation = pose
                        .translation
                        .lerp(d.points[target].translation - Vec3::Z * 8., blend);
                    let dir = d.ship(t).translation - pose.translation;
                    pose.rotation = Quat::from_rotation_z(dir.y.atan2(dir.x));
                    self.diamond.draw(
                        if blend < 1. { "run" } else { "stand_attack" },
                        t - start,
                        true,
                        pose,
                        1.,
                        fullbright,
                    );
                }
            }
        }
        gl_use_default_material();
    }
}
