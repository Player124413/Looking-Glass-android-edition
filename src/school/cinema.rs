//! First-school performances, expressed independently from the local asset data.
use super::*;
use crate::{cinematic::Camera, fortress::spline::Spline, npc::Puppet, story::Story};
use anyhow::ensure;
#[path = "library.rs"]
pub(crate) mod library;
#[path = "theatre.rs"]
mod theatre;
pub const THEATRE: &str = "Theatre_Cinematic";
pub const SHELF: &str = "Skool1_OG_MoveShelf";
pub const BOOK_WIN: &str = "Skool1_Book_Win";
/// Retain these map identities in saves, but let the scene supply their sole pose.
pub fn owns(name: &str) -> bool {
    matches!(name, "talk_gnome1" | "shelf_cat" | "book_cat")
}
const GNOME: &[&str] = &[
    "idle", "pipeout", "smoke", "walk", "mixing", "vanish01", "vanish02", "ride",
];
const ALICE: &[&str] = &[
    "idle_stand",
    "idle",
    "walk",
    "idle_shrug_headtilt",
    "push_loop",
    "push_end",
    "idle_base_01_play3",
    "idle_stand_rocktoes",
];
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Beat {
    Theatre,
    Shelf,
    Shelves,
    BookWin,
    Book,
    Recipe,
}
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    #[serde(default, skip_serializing_if = "zero")]
    pub handoff_fade: f32,
    pub beat: Option<Beat>,
    pub time: f32,
    pub home: Option<Transform>,
    pub dialogue_done: bool,
    pub completion_pending: bool,
    pub shelf_moved: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub guards_cued: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub skipped: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cast_time: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library: Option<library::State>,
}
fn zero(value: &f32) -> bool {
    *value == 0.
}
impl State {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.handoff_fade.is_finite() && (0. ..=0.5).contains(&self.handoff_fade),
            "Invalid school fade"
        );
        if let Some(l) = &self.library {
            l.validate()?;
        }
        ensure!(
            !matches!(self.beat, Some(Beat::Shelves | Beat::Book | Beat::Recipe))
                || self.library.is_some(),
            "Missing library scene state"
        );
        ensure!(
            self.cast_time
                .is_none_or(|t| t.is_finite() && (0. ..=10000.).contains(&t))
                && self.time.is_finite()
                && (0. ..=10000.).contains(&self.time)
                && self.home.is_none_or(|p| p.translation.is_finite()
                    && p.translation.abs().max_element() < 100000.
                    && p.rotation.is_finite()
                    && (p.rotation.length_squared() - 1.).abs() < 0.01),
            "Invalid school scene save"
        );
        Ok(())
    }
}
pub struct Data {
    tracks: BTreeMap<String, Spline>,
    points: BTreeMap<String, Transform>,
    lengths: BTreeMap<String, f32>,
    walk_speed: f32,
    ship: Spline,
    alice_walk: Vec<Vec3>,
    alice_speed: f32,
    guards: [Vec<Vec3>; 2],
    guard_speed: f32,
    platforms: BTreeMap<usize, usize>,
    camera_world: World,
}
impl Data {
    pub fn load(assets: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut tracks = BTreeMap::new();
        for name in [
            "skool_play",
            "skool_play2",
            "skool_play3",
            "skool_play4",
            "skool1_jpath1",
            "skool1_shelfp1",
            "skool1_path2",
            "skool1_path3",
            "skool1_path4",
            "skool1_path5",
            "skool1_path6",
            "skool1_bjbook",
            "skool1_bookp1",
        ] {
            let t = crate::cinematic::Track::load(assets, name)?;
            tracks.insert(name.into(), Spline::camera_track(t.controls().collect()));
        }
        let mut points = BTreeMap::new();
        let mut platforms = BTreeMap::new();
        for e in &map.entities {
            if let (Some(n), Some(p)) =
                (e.get("targetname"), e.get("origin").and_then(|v| vector(v)))
            {
                points.entry(n.clone()).or_insert(Transform {
                    translation: p,
                    rotation: Quat::from_rotation_z(
                        e.get("angle")
                            .and_then(|s| s.parse::<f32>().ok())
                            .unwrap_or(0.)
                            .to_radians(),
                    ),
                });
                if let Some(i) = n
                    .strip_prefix("theatre_plat")
                    .and_then(|v| v.parse::<usize>().ok())
                {
                    if let Some(model) = e
                        .get("model")
                        .and_then(|v| v.trim_start_matches('*').parse::<usize>().ok())
                    {
                        platforms.insert(model, i);
                    }
                }
            }
        }
        for n in [
            "gnome_start",
            "fake_pos1",
            "alice_pos1",
            "alice_gnome_pos",
            "theatre_pos1",
            "theatre_pos4",
            "theatre_pos5",
            "gnome_warp_shelf",
            "gnome_push2",
            "insane_actor3",
            "tiny_airship1",
            "theatre_plat2",
            "steam1",
            "steam2",
            "steam3",
            "alicepush",
            "bookpush",
            "alice_bookwatch_pos1",
            "shelf_cat",
            "book_cat",
        ] {
            ensure!(points.contains_key(n), "Missing school scene marker {n}");
        }
        // Path nodes are navigation targets above their supporting brush, not
        // skeletal foot positions. Resolve the same floor contact actors use.
        let mut world = World::from_bsp(map)?;
        let supports = platforms
            .keys()
            .map(|&model| {
                let e = map
                    .entities
                    .iter()
                    .find(|e| e.get("model") == Some(&format!("*{model}")))
                    .unwrap();
                Collider::model(
                    map,
                    model,
                    vector(&e["origin"]).unwrap(),
                    Quat::IDENTITY,
                    true,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        world.set_dynamic(supports);
        for name in [
            "gnome_start",
            "fake_pos1",
            "alice_pos1",
            "alice_gnome_pos",
            "theatre_pos1",
            "theatre_pos4",
            "theatre_pos5",
            "insane_actor3",
            "gnome_warp_shelf",
            "gnome_push2",
            "alicepush",
            "bookpush",
            "alice_bookwatch_pos1",
        ] {
            let p = &mut points.get_mut(name).unwrap().translation;
            if let Some(floor) = world.actor_footing(*p, Vec3::Z * 30., vec3(16., 16., 30.), 192.) {
                *p = floor;
            }
        }
        let def = crate::skeletal::Definition::load(assets, "models/c_gnomeold.tik")?;
        let rig = crate::skeletal::Skeleton::parse(
            &assets.read(&format!("{}/{}", def.path, def.model))?,
        )?;
        let mut lengths = BTreeMap::new();
        let mut walk_speed = 1.;
        for clip in GNOME {
            let a = crate::skeletal::Animation::parse(
                &assets.read(&format!("{}/{}", def.path, def.animations[*clip]))?,
                rig.bones.len(),
            )?;
            lengths.insert((*clip).into(), a.duration());
            if *clip == "walk" {
                walk_speed = a.distance * def.scale / a.duration();
            }
        }
        let mut ship_points = Vec::new();
        let mut node = "tinyship_path1";
        while let Some(e) = map
            .entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|n| n == node))
        {
            ship_points.push((
                vector(&e["origin"]).context("Ship node origin")?,
                points[node].rotation,
                e.get("speed").and_then(|v| v.parse().ok()).unwrap_or(1.),
            ));
            ensure!(ship_points.len() < 128, "School ship path cycle");
            let Some(next) = e.get("target") else {
                break;
            };
            node = next;
        }
        ensure!(!ship_points.is_empty(), "Missing school ship spline");
        let nodes = |ids: &[usize]| -> Result<Vec<Vec3>> {
            ids.iter()
                .map(|&i| {
                    map.entities
                        .get(i)
                        .and_then(|e| e.get("origin"))
                        .and_then(|p| vector(p))
                        .context("Missing school stair node")
                })
                .collect()
        };
        let speed = |assets: &mut Assets, name: &str, clip: &str| -> Result<f32> {
            let def = crate::skeletal::Definition::load(assets, &format!("models/{name}.tik"))?;
            let rig = crate::skeletal::Skeleton::parse(
                &assets.read(&format!("{}/{}", def.path, def.model))?,
            )?;
            let a = crate::skeletal::Animation::parse(
                &assets.read(&format!("{}/{}", def.path, def.animations[clip]))?,
                rig.bones.len(),
            )?;
            Ok(a.distance * def.scale / a.duration())
        };
        Ok(Self {
            tracks,
            points,
            lengths,
            walk_speed,
            ship: Spline::new(ship_points, false),
            alice_walk: nodes(&[321, 320, 319, 318, 317, 316, 122])?,
            alice_speed: speed(assets, "alice", "walk")?,
            guards: [
                nodes(&[617, 320, 319, 318, 317, 316, 122])?,
                nodes(&[63, 327, 616, 615, 326, 325, 324, 323, 322, 123])?,
            ],
            guard_speed: speed(assets, "cardguard_diamond", "run")?,
            platforms,
            camera_world: world,
        })
    }
    fn performance_camera2(&self) -> f32 {
        6.5 + self.lengths["pipeout"] + self.lengths["smoke"] * 2.
    }
    fn growth_end(&self) -> f32 {
        0.5 + self.lengths["vanish01"] + 2.1
    }
    fn walk_end(&self) -> f32 {
        self.growth_end()
            + self.points["gnome_warp_shelf"]
                .translation
                .distance(self.points["gnome_push2"].translation)
                / self.walk_speed.max(1.)
    }
    fn shelf_start(&self) -> f32 {
        self.walk_end() + self.lengths["mixing"] + 1.2
    }
    fn shelf_end(&self) -> f32 {
        self.shelf_start() + 6.5
    }
    fn gnome(&self, time: f32) -> (&'static str, f32) {
        let mut at = 1.;
        for (clip, duration) in [
            ("pipeout", self.lengths["pipeout"]),
            ("smoke", self.lengths["smoke"]),
            ("idle", 1.5),
            ("smoke", self.lengths["smoke"]),
            ("idle", 4.),
            ("smoke", self.lengths["smoke"]),
        ] {
            if time < at {
                return ("idle", time);
            }
            if time < at + duration {
                return (clip, time - at);
            }
            at += duration;
        }
        if (24.45..24.45 + self.lengths["smoke"]).contains(&time) {
            ("smoke", time - 24.45)
        } else if time >= theatre::VANISH {
            ("vanish02", time - theatre::VANISH)
        } else if time >= theatre::END {
            ("vanish01", time - theatre::END)
        } else {
            if time >= 24.45 + self.lengths["smoke"] {
                at = 24.45 + self.lengths["smoke"];
            }
            ("idle", (time - at).max(0.))
        }
    }
}
impl School {
    pub fn configure_cinema(&mut self, assets: &mut Assets, map: &Bsp) -> Result<()> {
        if !self.returning {
            self.cinema_data = Some(Data::load(assets, map)?);
        } else {
            self.return_data = Some(super::return_cinema::Data::load(assets, map)?);
        }
        Ok(())
    }
    pub(crate) fn check_scene_walks(&self) -> Result<()> {
        let d = self.cinema_data.as_ref().context("Missing theatre data")?;
        for i in 0..143 {
            let t = i as f32 / 10.;
            let p = d.alice_pose(t).translation;
            ensure!(
                !d.camera_world
                    .sweep(p + Vec3::Z * 30., p + Vec3::Z * 30., vec3(12., 12., 28.))
                    .start_solid,
                "Theatre Alice intersects architecture at {t}: {p:?}"
            );
        }
        for path in &d.guards {
            for i in 0..31 {
                let p = theatre::walk(&d.camera_world, path, i as f32 / 10. * d.guard_speed)
                    .translation;
                ensure!(
                    !d.camera_world
                        .sweep(p + Vec3::Z * 30., p + Vec3::Z * 30., vec3(12., 12., 28.))
                        .start_solid,
                    "Theatre reinforcement intersects architecture: {p:?}"
                );
            }
        }
        Ok(())
    }
    pub fn scene_id(&self) -> Option<&'static str> {
        if self.return_visit.is_some() {
            return self.return_scene_id();
        }
        match self.first_cinema.as_ref()?.beat? {
            Beat::Theatre => Some(THEATRE),
            Beat::Shelf => Some(SHELF),
            Beat::Shelves => Some(library::SHELVES),
            Beat::BookWin => Some(BOOK_WIN),
            Beat::Book => Some(library::BOOK),
            Beat::Recipe => Some(library::RECIPE),
        }
    }
    pub fn scene_start(&mut self, beat: Beat) {
        let cast_time = self.first_cinema.as_ref().and_then(|s| s.cast_time);
        self.first_cinema = Some(State {
            beat: Some(beat),
            cast_time,
            library: matches!(
                beat,
                Beat::Shelf | Beat::Shelves | Beat::Book | Beat::Recipe
            )
            .then(Default::default),
            ..Default::default()
        });
    }
    pub fn prepare_scene_story(&self, story: &mut Story) -> bool {
        let Some(s) = &self.first_cinema else {
            return true;
        };
        match s.beat {
            Some(Beat::BookWin) => false,
            Some(Beat::Theatre) => s.time >= 22.8,
            Some(Beat::Shelves) => s.time >= 4.,
            Some(Beat::Recipe) => s.time >= 0.5,
            Some(Beat::Book) => {
                let l = s.library.as_ref().unwrap();
                story.line_limit = Some(if l.phase < 2 || (l.phase == 2 && l.time < 12.) {
                    1
                } else {
                    3
                });
                s.time >= 2.
            }
            _ => true,
        }
    }
    pub fn hold_dialogue_completion(&mut self, name: &str) -> bool {
        if matches!(name, library::SHELVES | library::BOOK | library::RECIPE)
            && self.scene_id() == Some(name)
        {
            self.first_cinema.as_mut().unwrap().dialogue_done = true;
            return true;
        }
        if name == THEATRE {
            if let Some(s) = self
                .first_cinema
                .as_mut()
                .filter(|s| s.beat == Some(Beat::Theatre) || s.guards_cued)
            {
                s.dialogue_done = true;
                return true;
            }
        }
        false
    }
    pub fn take_scene_completion(&mut self) -> bool {
        self.first_cinema
            .as_mut()
            .is_some_and(|s| std::mem::take(&mut s.completion_pending))
    }
    pub fn scene_step(&mut self, dt: f32, world: &World, player: &mut Player) -> Result<()> {
        let Some(s) = self.first_cinema.as_mut() else {
            return Ok(());
        };
        if let Some(t) = &mut s.cast_time {
            *t = (*t + dt).min(1000.);
        }
        if s.beat.is_none() {
            s.handoff_fade = (s.handoff_fade - dt).max(0.);
            // Preserve the asynchronous disappearance and final pupil idles.
            if s.guards_cued {
                s.time = (s.time + dt).min(1000.);
            }
            return Ok(());
        }
        let home = *s.home.get_or_insert(Transform {
            translation: player.feet,
            rotation: Quat::from_rotation_z(player.script_facing),
        });
        s.time += dt;
        player.feet = home.translation;
        player.velocity = Vec3::ZERO;
        player.script_motion = 1;
        player.release_rope();
        player.cancel_climb();
        let data = self
            .cinema_data
            .as_ref()
            .context("School scene data unavailable")?;
        if matches!(s.beat, Some(Beat::Shelves | Beat::Book | Beat::Recipe)) {
            if self.library_step(dt) {
                self.finish_scene(world, player)?;
            }
            return Ok(());
        }
        let end = match s.beat.unwrap() {
            Beat::Theatre => theatre::END,
            Beat::Shelf => data.shelf_end(),
            Beat::BookWin => 7.,
            _ => unreachable!(),
        };
        if s.beat == Some(Beat::Theatre) && s.time >= theatre::GUARDS && !s.guards_cued {
            s.guards_cued = true;
            s.completion_pending = true;
        }
        let done = s.time >= end && (matches!(s.beat, Some(Beat::Shelf | Beat::BookWin)) || s.dialogue_done);
        let switch = s.beat == Some(Beat::Shelf) && s.time >= data.walk_end() + 0.5;
        let switch_return = s.time >= data.shelf_end();
        if s.beat == Some(Beat::Shelf) {
            s.library
                .get_or_insert_with(Default::default)
                .shelf_sounds(s.time, data);
        }
        if s.beat == Some(Beat::Shelf) && !s.shelf_moved && s.time >= data.shelf_start() {
            s.shelf_moved = true;
            self.open_library(false);
        }
        if switch {
            let o = self.object_mut("shelf_book");
            let target = if switch_return {
                o.base.origin
            } else {
                o.base.origin - Vec3::Y * 16.
            };
            if o.motion.is_none() && o.pose.origin.distance(target) > 0.01 {
                o.move_to(
                    Pose {
                        origin: target,
                        ..o.base
                    },
                    if switch_return { 0.1 } else { 2. },
                );
            }
        }
        if done {
            self.finish_scene(world, player)?;
        }
        Ok(())
    }
    fn open_library(&mut self, skip: bool) {
        let o = self.object_mut("secret_shelf");
        let end = Pose {
            origin: o.base.origin + Vec3::Y * 648.,
            ..o.base
        };
        o.move_to(end, if skip { 0.1 } else { 10. });
    }
    fn finish_scene(&mut self, world: &World, player: &mut Player) -> Result<()> {
        let Some(s) = self.first_cinema.as_mut() else {
            return Ok(());
        };
        let Some(beat) = s.beat else {
            return Ok(());
        };
        if matches!(beat, Beat::Shelf | Beat::Book | Beat::BookWin) {
            s.handoff_fade = 0.5;
        }
        let landing = if beat == Beat::Theatre {
            Some(
                self.cinema_data
                    .as_ref()
                    .context("Missing theatre landing")?
                    .points["alice_gnome_pos"],
            )
        } else {
            s.home
        };
        if let Some(landing) = landing {
            crate::cinematic::land_player(player, world, landing)?;
        } else {
            player.script_motion = 0;
            player.velocity = Vec3::ZERO;
            player.release_rope();
            player.cancel_climb();
        }
        s.beat = None;
        if beat == Beat::Theatre {
            s.cast_time = Some(s.time.max(theatre::END));
        }
        if beat == Beat::Theatre && !s.guards_cued {
            s.guards_cued = true;
            s.completion_pending = true;
        } else if beat == Beat::Theatre {
            // The cue already committed; finishing is presentation only.
        } else if beat == Beat::Shelf {
            self.library = true;
            self.object_mut("secret_shelf_block").enabled = false;
            let o = self.object_mut("shelf_book");
            o.move_to(o.base, 0.1);
        } else if beat == Beat::Book {
            self.finish_book();
        } else if beat == Beat::Recipe {
            s.library.as_mut().unwrap().exit_ready = true;
        }
        Ok(())
    }
    pub fn skip_first_scene(
        &mut self,
        world: &World,
        player: &mut Player,
        story: &mut Story,
    ) -> Result<bool> {
        if self.return_visit.is_some() {
            return Ok(self.skip_return_scene());
        }
        let Some(id) = self.scene_id() else {
            return Ok(false);
        };
        if id == SHELF {
            self.open_library(true);
        }
        if id == library::SHELVES {
            self.tip_shelves();
        }
        if id == THEATRE {
            let s = self.first_cinema.as_mut().unwrap();
            s.skipped = true;
            s.time = theatre::END;
        }
        self.finish_scene(world, player)?;
        story.finish_sequence(id);
        Ok(true)
    }
    pub fn scene_camera(&self) -> Option<Camera> {
        if self.return_visit.is_some() {
            return self.return_camera();
        }
        let s = self.first_cinema.as_ref()?;
        let data = self.cinema_data.as_ref()?;
        let (name, time) = match s.beat? {
            Beat::BookWin => ("skool1_bookp1", (s.time - 0.5).max(0.)),
            Beat::Shelves | Beat::Book | Beat::Recipe => self.library_camera()?,
            Beat::Shelf => ("skool1_shelfp1", (s.time - data.growth_end()).max(0.)),
            Beat::Theatre => {
                let b = data.performance_camera2();
                let c = b + data.lengths["smoke"] + 8.5;
                if s.time >= 40.6 {
                    ("skool1_jpath1", s.time - 40.6)
                } else if s.time >= 30.3 {
                    ("skool_play4", s.time - 30.3)
                } else if s.time >= c {
                    ("skool_play3", s.time - c)
                } else if s.time >= b {
                    ("skool_play2", s.time - b)
                } else {
                    ("skool_play", s.time)
                }
            }
        };
        let pose = data.tracks[name].sample(time, false);
        let (mut eye, q) = (pose.translation, pose.rotation);
        let target = eye + q * Vec3::X * 100.;
        // The early authored pan crosses the port's Gnome body. Keep a small
        // camera stand-off while preserving its stage-facing target and roll.
        if s.beat == Some(Beat::Theatre) {
            let focus = data.points["gnome_start"].translation + Vec3::Z * 40.;
            let delta = eye - focus;
            if delta.truncate().length() < 120. && delta.z.abs() < 125. {
                let direction = delta.truncate().normalize_or_zero();
                eye.x = focus.x + direction.x * 120.;
                eye.y = focus.y + direction.y * 120.;
                let hit = data
                    .camera_world
                    .sweep_geometry(focus, eye, Vec3::splat(6.));
                if !hit.start_solid && hit.fraction < 1. {
                    eye = focus.lerp(eye, hit.fraction) + hit.normal * 2.;
                }
            }
        }
        Some(Camera {
            eye,
            target,
            up: q * Vec3::Z,
        })
    }
    pub fn scene_platform(&self, model: usize) -> f32 {
        let Some(s) = self
            .first_cinema
            .as_ref()
            .filter(|s| s.beat == Some(Beat::Theatre))
        else {
            return 0.;
        };
        let d = self.cinema_data.as_ref().unwrap();
        d.platforms
            .get(&model)
            .map_or(0., |&i| theatre::platform(i, s.time))
    }
    pub fn scene_fade(&self) -> (Color, f32) {
        if let Some(s) = self.return_visit.as_ref().and_then(|r| r.scene.as_ref()) {
            return (WHITE, s.fade());
        }
        if let Some(s) = &self.first_cinema {
            if s.handoff_fade > 0. {
                return (WHITE, s.handoff_fade / 0.5);
            }
        }
        if let Some(fade) = self.library_fade() {
            return (WHITE, fade);
        }
        if let (Some(s), Some(d)) = (&self.first_cinema, &self.cinema_data) {
            if s.beat == Some(Beat::BookWin) {
                return (WHITE, (s.time / 0.5).min(1.) * ((1. - s.time) / 0.5).clamp(0., 1.)
                    + ((s.time - 6.5) / 0.5).clamp(0., 1.));
            }
            if s.beat == Some(Beat::Shelf) {
                return (
                    WHITE,
                    (1. - s.time / 0.5)
                        .clamp(0., 1.)
                        .max(((s.time - d.shelf_end() + 0.5) / 0.5).clamp(0., 1.)),
                );
            }
        }
        (
            BLACK,
            self.first_cinema
                .as_ref()
                .filter(|s| s.beat == Some(Beat::Theatre))
                .map_or(0., |s| (1. - s.time / 2.).clamp(0., 1.)),
        )
    }
    pub fn stage_guards(&self, encounters: &mut crate::encounters::Encounters) {
        let Some(s) = self
            .first_cinema
            .as_ref()
            .filter(|s| s.beat == Some(Beat::Theatre) && s.guards_cued)
        else {
            return;
        };
        let Some(d) = self.cinema_data.as_ref() else {
            return;
        };
        for (i, name) in ["play_guard1", "play_guard2"].into_iter().enumerate() {
            let Some(a) = encounters
                .actors
                .iter_mut()
                .find(|a| a.name == name && a.active && a.enabled)
            else {
                continue;
            };
            if let crate::encounters::Enemy::Guard(g) = &mut a.enemy {
                if g.health <= 0. {
                    continue;
                }
                let at = (s.time - 43.6).max(0.);
                let pose = theatre::walk(&d.camera_world, &d.guards[i], at * d.guard_speed);
                g.feet = pose.translation;
                g.yaw = pose.rotation.to_euler(EulerRot::ZYX).0;
                g.state = if at > 0. {
                    crate::combat::State::Chase
                } else {
                    crate::combat::State::Idle
                };
                g.time = at;
            }
        }
    }
}
fn travel(a: Transform, b: Transform, t: f32) -> Transform {
    let f = t.clamp(0., 1.);
    let delta = b.translation - a.translation;
    Transform {
        translation: a.translation.lerp(b.translation, f),
        rotation: if f < 1. {
            Quat::from_rotation_z(delta.y.atan2(delta.x))
        } else {
            b.rotation
        },
    }
}
pub struct Art {
    pub alice: Puppet,
    gnome: Puppet,
    cat: Puppet,
    children: Vec<Puppet>,
    ship: Puppet,
    tiny: Puppet,
    pipe: Prop,
    pipe_smoke: crate::particles::Attached,
    steam: [crate::particles::Attached; 3],
    disappear: crate::particles::Attached,
}
impl Art {
    pub fn load(
        assets: &mut Assets,
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let mut gnome = Puppet::load(assets, "c_gnomeold", GNOME, specs)?;
        // This scene owns a persistent script attachment across animation changes.
        gnome.show_attachments(false);
        let steam = crate::particles::Attached::load(assets, "fx_emitter_steam", specs)?
            .context("School steam missing")?;
        Ok(Self {
            steam: [steam.fork(), steam.fork(), steam],
            // Only the frame-zero burst is used; its TAN frame rate cannot shift it.
            disappear: crate::particles::Attached::load_clip_bursts(
                assets,
                "fx_pickup",
                Some("on"),
                0.05,
                specs,
            )?
            .context("School disappearance missing")?,
            alice: Puppet::load(assets, "alice", ALICE, specs)?,
            gnome,
            cat: Puppet::load(assets, "c_cheshire", library::CAT, specs)?,
            pipe: Prop::load(assets, "gnomepipe", specs)?,
            pipe_smoke: crate::particles::Attached::load(assets, "gnomepipe", specs)?
                .context("School pipe smoke missing")?,
            children: [
                (
                    "c_insanechild_spikes",
                    &["idle01", "idle03", "antictrans_antic01"][..],
                ),
                (
                    "c_insanechild_brain",
                    &["idle01", "idle_reg_swipe", "idle_reg_tick"][..],
                ),
                ("c_insanechild_vise", &["idle01", "idle06"][..]),
            ]
            .into_iter()
            .map(|(m, c)| Puppet::load(assets, m, c, specs))
            .collect::<Result<_>>()?,
            ship: Puppet::load(assets, "gnome_airship", &["fly", "hover"], specs)?,
            tiny: Puppet::load(assets, "c_gnomeold", &["ride"], specs)?,
        })
    }
    pub fn story_pose(&mut self, story: &Story) {
        self.alice.mouth(story.mouth(&["fakeplayer", "alice"]));
        self.gnome.mouth(story.mouth(&["talk_gnome1"]));
        self.cat.mouth(story.mouth(&["book_cat", "shelf_cat"]));
    }
    pub fn draw(
        &mut self,
        school: &School,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        let Some(d) = &school.cinema_data else {
            return;
        };
        for p in [
            &mut self.alice,
            &mut self.gnome,
            &mut self.tiny,
            &mut self.cat,
        ] {
            p.atmosphere(atmosphere, camera);
        }
        for p in &mut self.children {
            p.atmosphere(atmosphere, camera);
        }
        self.ship.atmosphere(atmosphere, camera);
        let s = school.first_cinema.as_ref();
        let beat = s.and_then(|s| s.beat);
        let t = s.map_or(school.clock, |s| s.time);
        self.gnome.watch(Default::default());
        self.alice.watch(Default::default());
        if matches!(beat, Some(Beat::Shelves | Beat::Book | Beat::Recipe)) {
            self.draw_library(school, bright);
        } else if beat == Some(Beat::BookWin) {
            if let Some(home) = s.and_then(|s| s.home) {
                self.alice.draw("idle_stand", t, true, home, 1., bright);
            }
        } else if beat == Some(Beat::Shelf) {
            let start = d.points["gnome_warp_shelf"];
            let end = d.points["gnome_push2"];
            let (clip, at, pose, scale) = if t < d.growth_end() {
                (
                    if t < 0.5 + d.lengths["vanish01"] {
                        "vanish01"
                    } else {
                        "vanish02"
                    },
                    (t - 0.5).max(0.),
                    start,
                    ((t - 0.5 - d.lengths["vanish01"]) / 2.1).clamp(0.05, 1.),
                )
            } else if t < d.walk_end() {
                (
                    "walk",
                    t - d.growth_end(),
                    travel(
                        start,
                        end,
                        (t - d.growth_end()) / (d.walk_end() - d.growth_end()),
                    ),
                    1.,
                )
            } else {
                (
                    if t < d.shelf_start() {
                        "mixing"
                    } else {
                        "idle"
                    },
                    t - d.walk_end(),
                    end,
                    1.,
                )
            };
            self.gnome.draw(clip, at, true, pose, scale, bright);
            self.gnome
                .draw_effects(clip, at, true, pose, scale, camera, atmosphere);
            if let Some(home) = s.and_then(|s| s.home) {
                self.alice.draw("idle_stand", t, true, home, 1., bright);
            }
        } else if !school.theatre
            || (s.is_some_and(|s| !s.skipped && s.guards_cued) && t < 49.15)
            || beat == Some(Beat::Theatre)
        {
            let scene = beat == Some(Beat::Theatre) || school.theatre;
            let (clip, at) = if scene { d.gnome(t) } else { ("idle", t) };
            let pose = d.points["gnome_start"];
            let scale = if scene && t >= theatre::VANISH {
                (1. - ((t - theatre::VANISH) / 0.1).floor() * 0.05).clamp(0., 1.)
            } else {
                1.
            };
            if scene {
                self.gnome.watch(d.watch(1, t));
            }
            let looping = clip == "idle";
            self.gnome.draw(clip, at, looping, pose, scale, bright);
            if scene && t >= 1.4 && scale > 0. {
                let pipe_pose = if looping {
                    self.gnome.looping_tag("tag_pipe", clip, at, pose, scale)
                } else {
                    self.gnome.tag("tag_pipe", clip, at, pose, scale)
                }
                .unwrap_or(pose);
                self.pipe.draw_frame(pipe_pose, scale, bright, t, true);
                let pipe_at = |birth: f32, tag: Option<&str>| {
                    let (clip, at) = d.gnome(birth);
                    let p = if clip == "idle" {
                        self.gnome.looping_tag("tag_pipe", clip, at, pose, scale)
                    } else {
                        self.gnome.tag("tag_pipe", clip, at, pose, scale)
                    }
                    .unwrap_or(pose);
                    Transform {
                        translation: tag
                            .map_or(p.translation, |tag| self.pipe.point(p, tag, scale)),
                        ..p
                    }
                };
                self.pipe_smoke.draw(
                    t - 1.4,
                    scale,
                    |birth, tag| pipe_at(birth + 1.4, tag),
                    |_, birth, default| default && birth + 1.4 < theatre::VANISH,
                    camera,
                    atmosphere,
                );
            }
        }
        let theatre = beat == Some(Beat::Theatre);
        let completed = school.theatre && !theatre;
        if theatre && !(14.3..19.3).contains(&t) {
            self.alice.watch(d.watch(0, t));
            self.alice.draw(
                if (0.2..14.3).contains(&t) {
                    "walk"
                } else {
                    "idle"
                },
                if t < 14.3 {
                    (t - 0.2).max(0.)
                } else {
                    t - 14.3
                },
                true,
                d.alice_pose(t),
                1.,
                bright,
            );
        }
        if school.recipe {
            return;
        }
        // These actors remain in the room after the camera and dialogue end.
        let child_time = if completed {
            s.and_then(|s| s.cast_time).unwrap_or(theatre::END)
        } else if theatre {
            t
        } else {
            0.
        };
        for i in 0..3 {
            if theatre && i == 0 && (33.3..33.4).contains(&t) {
                continue;
            }
            self.children[i].watch(if i == 0 && school.theatre {
                d.watch(2, child_time)
            } else {
                Default::default()
            });
            let (clip, at) = theatre::child_clip(i, child_time);
            self.children[i].draw(clip, at, true, d.child_pose(i, child_time), 1., bright);
        }
        if theatre {
            for (i, effect) in self.steam.iter_mut().enumerate() {
                let pose = d.points[["steam1", "steam2", "steam3"][i]];
                effect.draw(
                    t,
                    2.,
                    |_, _| pose,
                    |_, birth, _| theatre::steam(i, birth),
                    camera,
                    atmosphere,
                );
            }
        }
        if !completed {
            let pose = if theatre {
                d.ship_pose(t)
            } else {
                d.points["tiny_airship1"]
            };
            self.ship.draw("fly", t, true, pose, 0.1, bright);
            let rider = self
                .ship
                .looping_tag("tag_gnome", "fly", t, pose, 0.1)
                .unwrap_or(pose);
            self.tiny.draw("ride", t, true, rider, 0.1, bright);
        }
        if s.is_some_and(|s| s.guards_cued && !s.skipped)
            && beat != Some(Beat::Shelf)
            && t >= theatre::VANISH
        {
            self.disappear.draw(
                t - theatre::VANISH,
                1.,
                |_, _| d.points["gnome_start"],
                |_, _, default| default,
                camera,
                atmosphere,
            );
        }
    }
}
