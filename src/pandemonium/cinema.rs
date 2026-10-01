//! Reviewed Pandemonium scenes, with explicit commit points shared by watching/skipping.
use super::*;
use crate::{
    assets::Assets,
    cinematic::{Camera, Track},
    story::Story,
};

pub const WARNING: &str = "Elder_Gnome1_Warn_Thread";
pub const DEPARTURE: &str = "Pand_End_Ship";
pub const ALICE_CLIPS: &[&str] = &[
    "walk",
    "jump",
    "fall",
    "land",
    "idle_stand",
    "idle_stand_rocktoes",
    "idle_stand_nodyes",
    "idle_stand_shakeno",
    "idle_base_01_2_base_02",
    "idle_base_02",
    "idle_base_02_2_base_03",
    "idle_base_03",
    "idle_base_02_2_shrug",
    "idle_shrug",
    "sit_minecart",
    "sit_airship",
    "ready",
];
pub const GNOME_CLIPS: &[&str] = &[
    "idle",
    "smoke",
    "talk",
    "pipetalk01",
    "pipetalk02",
    "letsgo",
    "vanish01",
    "vanish02",
    "ride",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Beat {
    Warning,
    Vanish,
    Return,
    Landing,
    BoardShip,
    Flight,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct State {
    pub version: u8,
    pub beat: Option<Beat>,
    pub time: f32,
    pub shot: usize,
    pub shot_time: f32,
    pub line: usize,
    pub line_time: f32,
    pub warning_done: bool,
    pub return_done: bool,
    pub home: Vec3,
    pub home_set: bool,
}
impl State {
    pub fn fresh() -> Self {
        Self {
            version: 1,
            ..Default::default()
        }
    }
    pub fn start(&mut self, beat: Beat) {
        self.beat = Some(beat);
        self.time = 0.;
        self.shot = 0;
        self.shot_time = 0.;
        self.line = 0;
        self.line_time = 0.;
        self.home_set = false;
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version <= 1
                && self.home.is_finite()
                && self.home.abs().max_element() < 100_000.
                && self.shot <= 2
                && self.line < 32
                && [self.time, self.shot_time, self.line_time]
                    .iter()
                    .all(|t| t.is_finite() && (0. ..=1e6).contains(t)),
            "Invalid cinematic timeline"
        );
        Ok(())
    }
}
pub struct Data {
    tracks: BTreeMap<&'static str, Track>,
    points: BTreeMap<String, Vec3>,
    durations: BTreeMap<String, f32>,
    pub bumps: Vec<f32>,
    pub collapse_times: [f32; 2],
    torch_times: [f32; 3],
    torch_yaws: [f32; 3],
    pub end_camera_time: f32,
}
impl Data {
    pub fn load(assets: &mut Assets, map: &Bsp, rail_times: &[f32]) -> Result<Self> {
        let mut tracks = BTreeMap::new();
        for name in [
            "elder_gnome1_cam_path1",
            "gnome_sad1",
            "pand_key_p1",
            "gnome_leavep1",
            "minecart_endcam_path",
            "pand_endsp1",
        ] {
            tracks.insert(name, Track::load(assets, name)?);
        }
        let points = map
            .entities
            .iter()
            .filter_map(|e| Some((e.get("targetname")?.clone(), vector(e.get("origin")?)?)))
            .collect();
        let mut durations = BTreeMap::new();
        for (model, clips) in [("alice", ALICE_CLIPS), ("c_gnomeold", GNOME_CLIPS)] {
            let d = crate::skeletal::Definition::load(assets, &format!("models/{model}.tik"))?;
            let rig = crate::skeletal::Skeleton::parse(
                &assets.read(&format!("{}/{}", d.path, d.model))?,
            )?;
            for clip in clips {
                let file = d
                    .animations
                    .get(*clip)
                    .with_context(|| format!("Missing cinematic animation {model}/{clip}"))?;
                let a = crate::skeletal::Animation::parse(
                    &assets.read(&format!("{}/{file}", d.path))?,
                    rig.bones.len(),
                )?;
                durations.insert(format!("{model}/{clip}"), a.duration());
            }
        }
        let mut bumps = Vec::new();
        let mut collapse_times = [f32::MAX; 2];
        let mut torch_times = [f32::MAX; 3];
        let mut end_camera_time = 0.;
        let mut name = "minecart_spline";
        for &time in rail_times {
            let e = map
                .entities
                .iter()
                .find(|e| e.get("targetname").is_some_and(|n| n == name))
                .context("Cart cue node missing")?;
            match e.get("thread").map(String::as_str) {
                Some("Minecart_Alice_Bump_Thread") => bumps.push(time),
                Some("BadTrack1_Thread") => collapse_times[0] = time + 4.,
                Some("BadTrack2_Thread") => collapse_times[1] = time,
                Some("Minecart_Jump_Thread") => end_camera_time = time,
                Some("Minecart_Torchgnome1_Thread") => torch_times[0] = time,
                Some("Minecart_Torchgnome2_Thread") => torch_times[1] = time,
                Some("Minecart_Torchgnome3_Thread") => torch_times[2] = time,
                _ => {}
            }
            if let Some(next) = e.get("target") {
                name = next;
            } else {
                break;
            }
        }
        Ok(Self {
            tracks,
            points,
            durations,
            bumps,
            collapse_times,
            torch_times,
            torch_yaws: std::array::from_fn(|i| {
                map.entities
                    .iter()
                    .find(|e| {
                        e.get("targetname")
                            .is_some_and(|s| s == &format!("minecart_torchgnome{}_node1", i + 1))
                    })
                    .and_then(|e| e.get("angle"))
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.)
            }),
            end_camera_time,
        })
    }
    pub fn at(&self, name: &str) -> Vec3 {
        self.points[name]
    }
    fn duration(&self, model: &str, clip: &str) -> f32 {
        self.durations[&format!("{model}/{clip}")]
    }
    fn return_duration(&self) -> f32 {
        5. + self.duration("c_gnomeold", "letsgo") + self.duration("c_gnomeold", "vanish01") + 2.5
    }
    fn acting<'a>(
        &self,
        model: &str,
        mut time: f32,
        clips: &'a [(&'a str, Option<f32>)],
    ) -> (&'a str, f32, bool) {
        for &(clip, fixed) in clips {
            let duration = fixed.unwrap_or_else(|| self.duration(model, clip));
            if time < duration {
                return (clip, time, fixed.is_some());
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
}
impl Pandemonium {
    pub fn scene_id(&self) -> Option<&'static str> {
        if self.state.exit_sent {
            return None;
        }
        match self.state.cinema.beat {
            Some(Beat::Warning | Beat::Vanish) => Some(WARNING),
            Some(Beat::Return) => Some("alice_leave"),
            Some(Beat::BoardShip | Beat::Flight) => Some(DEPARTURE),
            _ if self.state.departure => Some(DEPARTURE),
            _ if self.cinematic() => Some("Minecart_Thread"),
            _ => None,
        }
    }
    pub fn sync_story(&mut self, story: &Story) {
        let s = &mut self.state.cinema;
        let event = match s.beat {
            Some(Beat::Warning) => WARNING,
            Some(Beat::BoardShip | Beat::Flight) => DEPARTURE,
            _ => return,
        };
        if let Some((line, time)) = story.progress(event) {
            s.line = line;
            s.line_time = time;
            let shot = if event == WARNING {
                if line >= 8 {
                    2
                } else if line >= 6 {
                    1
                } else {
                    0
                }
            } else {
                usize::from(line >= 2)
            };
            if shot != s.shot {
                s.shot = shot;
                s.shot_time = 0.;
            }
            if event == DEPARTURE && line >= 2 && s.beat == Some(Beat::BoardShip) {
                s.beat = Some(Beat::Flight);
                s.time = 0.;
                self.state.flight = 0.;
            }
        }
    }
    pub fn dialogue_complete(&mut self, name: &str) {
        if name == WARNING && self.state.cinema.beat == Some(Beat::Warning) {
            self.state.cinema.start(Beat::Vanish);
        }
        if name == DEPARTURE && self.state.departure {
            self.state.dialogue_done = true;
            if self.state.cinema.beat == Some(Beat::BoardShip) {
                self.state.cinema.start(Beat::Flight);
            }
        }
    }
    fn finish_warning(&mut self, player: &mut Player) {
        player.feet = self.cinema.at("elder_gnome1_player_node2");
        player.script_facing = std::f32::consts::PI;
        self.state.cinema.warning_done = true;
        self.state.cinema.beat = None;
        self.release_control(player);
    }
    fn finish_return(&mut self, player: &mut Player) {
        if self.state.cinema.home_set {
            player.feet = self.state.cinema.home;
        }
        self.state.cinema.return_done = true;
        self.state.cinema.beat = None;
        self.release_control(player);
    }
    pub(super) fn finish_cart(&mut self, player: &mut Player) {
        player.feet = self.end;
        player.script_facing = 225_f32.to_radians();
        self.state.cart = Cart::Finished;
        self.state.time = self.rail_times[90];
        self.state.rides = 1;
        self.state.cinema.beat = None;
        self.release_control(player);
    }
    fn release_control(&mut self, player: &mut Player) {
        player.velocity = Vec3::ZERO;
        player.grounded = true;
        player.script_motion = 0;
        self.state.cinema.time = 0.;
        self.state.cinema.shot_time = 0.;
    }
    /// Complete the same world mutations as natural playback; don't fast-forward AI,
    /// hazards, resource regeneration or arbitrary queued events.
    pub fn skip(
        &mut self,
        map: &Bsp,
        world: &World,
        player: &mut Player,
        story: &mut Story,
    ) -> Result<bool> {
        let Some(id) = self.scene_id() else {
            return Ok(false);
        };
        match id {
            WARNING => {
                ensure!(
                    world.body_clear(self.cinema.at("elder_gnome1_player_node2")),
                    "Warning landing obstructed"
                );
                self.finish_warning(player);
            }
            "alice_leave" => self.finish_return(player),
            "Minecart_Thread" => {
                ensure!(world.body_clear(self.end), "Minecart landing obstructed");
                self.finish_cart(player);
            }
            DEPARTURE => {
                self.state.cinema.start(Beat::Flight);
                self.state.flight = 15.;
                self.state.dialogue_done = true;
                player.feet = self.ship_seat();
                player.velocity = Vec3::ZERO;
                player.script_motion = 3;
            }
            _ => unreachable!(),
        }
        story.finish_sequence(id);
        self.rebuild(map)?;
        Ok(true)
    }
    pub(super) fn advance_cinema(&mut self, dt: f32, player: &mut Player) {
        let s = &mut self.state.cinema;
        if s.beat.is_none() {
            return;
        }
        if !s.home_set {
            s.home = player.feet;
            s.home_set = true;
        }
        s.time += dt;
        s.shot_time += dt;
        match s.beat {
            Some(Beat::Warning | Beat::Vanish) => {
                player.feet = self.cinema.at("elder_gnome1_player_node2");
                if s.beat == Some(Beat::Vanish)
                    && s.time >= self.cinema.duration("c_gnomeold", "vanish01") + 2.5
                {
                    self.finish_warning(player);
                }
            }
            Some(Beat::Return) => {
                player.feet = s.home;
                if s.time >= self.cinema.return_duration() {
                    self.finish_return(player);
                }
            }
            Some(Beat::Landing) => {
                player.feet = self.end;
                if s.time + self.rail_times[90] - self.cinema.end_camera_time >= 9.5 {
                    self.finish_cart(player);
                }
            }
            Some(Beat::BoardShip) => {
                player.feet = self.cinema.at("alice_endship_posx1").lerp(
                    self.cinema.at("alice_airship_end1"),
                    fraction((s.time - 0.5).max(0.), 2.),
                );
            }
            _ => {}
        }
    }
    pub fn camera(&self) -> Option<Camera> {
        let s = &self.state.cinema;
        let data = &self.cinema;
        match s.beat {
            Some(Beat::Warning) => Some(
                data.tracks[match s.shot {
                    0 => "elder_gnome1_cam_path1",
                    1 => "gnome_sad1",
                    _ => "pand_key_p1",
                }]
                .sample(s.shot_time),
            ),
            Some(Beat::Vanish) => Some(data.tracks["elder_gnome1_cam_path1"].sample(0.)),
            Some(Beat::Return) => Some(data.tracks["gnome_leavep1"].sample((s.time - 0.5).max(0.))),
            Some(Beat::BoardShip) => Some(data.tracks["pand_endsp1"].sample(s.time)),
            Some(Beat::Landing) => Some(
                data.tracks["minecart_endcam_path"]
                    .sample(self.rail_times[90] - data.end_camera_time + s.time),
            ),
            _ if self.state.departure => {
                let target = self.ship_seat() + Vec3::Z * 25.;
                let offset = data.at("airship_close_cam") - self.ship;
                Some(Camera::look(
                    target + offset.normalize() * 256. + Vec3::Z * 40.,
                    target,
                ))
            }
            _ => match self.state.cart {
                Cart::Boarding => Some(Camera::look(
                    data.at("minecart_startcam"),
                    self.cart_base + Vec3::Z * 35.,
                )),
                Cart::Lift => {
                    let lift = Vec3::Z * (640. * fraction(self.state.time, 6.));
                    Some(Camera::look(
                        data.at(if self.state.time < 3. {
                            "minecart_startcam"
                        } else {
                            "minecart_cam"
                        }) + lift,
                        self.cart_base + lift + Vec3::Z * 35.,
                    ))
                }
                Cart::Rail if self.state.time >= data.end_camera_time => Some(
                    data.tracks["minecart_endcam_path"]
                        .sample(self.state.time - data.end_camera_time),
                ),
                Cart::Rail => {
                    let (p, q) = self.rail_pose();
                    Some(Camera::look(
                        p + q * vec3(-80., -56., 68.),
                        p + Vec3::Z * 32.,
                    ))
                }
                _ => None,
            },
        }
    }
    pub fn rail_camera_active(&self) -> bool {
        self.state.cart == Cart::Rail
            && self.state.cinema.beat.is_none()
            && self.state.time < self.cinema.end_camera_time
    }
    pub fn rail_camera_ahead(&self) -> [Camera; 12] {
        std::array::from_fn(|i| {
            let time = (self.state.time + (i + 1) as f32 * 0.05).min(self.cinema.end_camera_time);
            let (p, q) = self.rail_pose_at(time);
            Camera::look(p + q * vec3(-80., -56., 68.), p + Vec3::Z * 32.)
        })
    }
    pub fn fade_color(&self) -> Color {
        if matches!(self.state.cart, Cart::Boarding | Cart::Lift)
            || self.rail_camera_active()
            || self.state.cinema.beat == Some(Beat::Landing)
        {
            Color::new(0.5, 0.5, 0.5, 1.)
        } else {
            WHITE
        }
    }
    pub fn fade(&self) -> f32 {
        let s = &self.state.cinema;
        if let Some(beat) = s.beat {
            let end = match beat {
                Beat::Vanish => Some(self.cinema.duration("c_gnomeold", "vanish01") + 2.5),
                Beat::Return => Some(self.cinema.return_duration()),
                Beat::Landing => Some(9.5 - self.rail_times[90] + self.cinema.end_camera_time),
                _ => None,
            };
            let out = end.map_or(0., |end| ((s.time - end + 0.5) / 0.5).clamp(0., 1.));
            (1. - s.time / 0.5).clamp(0., 1.).max(out)
        } else if self.state.cart == Cart::Boarding {
            (1. - self.state.time).clamp(0., 1.)
        } else if self.state.cart == Cart::Lift {
            (1. - (self.state.time - 3.).abs()).clamp(0., 1.)
        } else if self.state.cart == Cart::Rail {
            (1. - (self.state.time - self.cinema.end_camera_time).abs()).clamp(0., 1.)
        } else {
            0.
        }
    }
}

fn pose(p: Vec3, yaw: f32) -> Transform {
    Transform {
        translation: p,
        rotation: Quat::from_rotation_z(yaw.to_radians()),
    }
}
/// Each authored track piece has its own pivot, axis and two-stage motion.
pub(super) fn debris(name: &str, base: Vec3, time: f32) -> Transform {
    let second = name.starts_with("badtrack2");
    let part = name.as_bytes().last().copied().unwrap_or(b'1') - b'0';
    let t = time.max(0.);
    let mut p = base;
    let mut q = Quat::IDENTITY;
    if name.contains("rock") {
        let speed = if part == 1 { 250. } else { 300. };
        let f = fraction(t, 1280. / speed);
        p.z += 256. - 1280. * f;
        q = Quat::from_rotation_z(f * std::f32::consts::TAU)
            * Quat::from_rotation_x(f * std::f32::consts::TAU);
    } else if time > 1. {
        if part <= 2 {
            let rise = fraction(t - 1., if part == 1 { 1. } else { 1.5 });
            let drop = fraction(t - 2.5, if part == 1 { 3. } else { 4. });
            p.z += 64. * rise - 1280. * drop;
            let lateral = 64. * rise * if part == 1 { 1. } else { -1. };
            if second {
                p.y += lateral;
            } else {
                p.x -= lateral;
            }
            let angle = (360. * rise + 720. * drop).to_radians();
            q = if second {
                Quat::from_rotation_y(-angle)
            } else {
                Quat::from_rotation_x(angle)
            };
        } else if part == 5 {
            p.z -= 1280. * fraction(t - 2.5, 1.);
        } else {
            let angle = if second {
                if part == 3 {
                    20_f32
                } else {
                    30_f32
                }
            } else if part == 3 {
                10.
            } else {
                15.
            };
            let angle = angle.to_radians() * fraction(t - 2.5, 1.);
            q = if second {
                Quat::from_rotation_y(angle)
            } else {
                Quat::from_rotation_x(-angle)
            };
        }
    }
    Transform {
        translation: p,
        rotation: q,
    }
}

impl Art {
    pub(super) fn draw_cast(&mut self, p: &Pandemonium, fullbright: bool) {
        self.pipe_pose = None;
        let s = &p.state.cinema;
        let d = &p.cinema;
        if matches!(s.beat, Some(Beat::Warning | Beat::Vanish)) {
            let (clip, time, looping) = d.acting(
                "alice",
                s.time,
                &[
                    ("idle_stand", Some(4.)),
                    ("idle_stand_rocktoes", Some(5.)),
                    ("idle_stand", Some(8.)),
                    ("idle_stand_nodyes", None),
                    ("idle_stand", Some(11.)),
                    ("idle_stand_shakeno", None),
                ],
            );
            self.alice.draw(
                clip,
                time,
                looping,
                pose(d.at("elder_gnome1_player_node2"), 180.),
                1.,
                fullbright,
            );
        }
        if !s.warning_done {
            let (clip, time, looping, scale, position, yaw) = if s.beat == Some(Beat::Vanish) {
                let duration = d.duration("c_gnomeold", "vanish01");
                if s.time < duration {
                    (
                        "vanish01",
                        s.time,
                        false,
                        1.,
                        d.at("elder_gnome1_node3"),
                        0.,
                    )
                } else {
                    (
                        "vanish02",
                        s.time - duration,
                        true,
                        (1. - (s.time - duration) / 2.5).max(0.),
                        d.at("elder_gnome1_node3"),
                        0.,
                    )
                }
            } else if s.beat == Some(Beat::Warning) {
                let (c, t, l) = d.acting(
                    "c_gnomeold",
                    s.time,
                    &[
                        ("pipetalk02", None),
                        ("idle", Some(9.)),
                        ("talk", None),
                        ("idle", Some(10.)),
                        ("smoke", None),
                        ("idle", Some(3.)),
                        ("talk", None),
                        ("idle", Some(8.)),
                        ("talk", None),
                        ("idle", Some(10.)),
                        ("pipetalk01", None),
                    ],
                );
                (c, t, l, 1., d.at("elder_gnome1_node3"), 0.)
            } else {
                (
                    "smoke",
                    p.state.age,
                    true,
                    1.,
                    d.at("elder_gnome1_watchplayer"),
                    315.,
                )
            };
            self.gnome
                .draw(clip, time, looping, pose(position, yaw), scale, fullbright);
            self.pipe_pose = Some((clip.into(), time, looping, pose(position, yaw), scale));
        }
        if p.state.returned && !s.return_done {
            let mut time = (s.time - 5.).max(0.);
            let letsgo = d.duration("c_gnomeold", "letsgo");
            let vanish = d.duration("c_gnomeold", "vanish01");
            let (clip, scale) = if s.beat != Some(Beat::Return) || s.time < 5. {
                time = 0.;
                ("idle", 1.)
            } else if time < letsgo {
                ("letsgo", 1.)
            } else if time < letsgo + vanish {
                time -= letsgo;
                ("vanish01", 1.)
            } else {
                time -= letsgo + vanish;
                ("vanish02", (1. - time / 2.5).max(0.))
            };
            self.gnome.draw(
                clip,
                time,
                false,
                pose(d.at("gnome_telep_pos1"), 270.),
                scale,
                fullbright,
            );
        }
        let cart = p.state.cart;
        if matches!(cart, Cart::Boarding | Cart::Lift | Cart::Rail) {
            let (position, yaw, clip, time, looping) = if s.beat == Some(Beat::Landing) {
                let t = (s.time + p.rail_times[90] - d.end_camera_time - 6.).max(0.);
                let q = fraction(t, 1.);
                (
                    d.at("minecart_endnode").lerp(p.end, q) + Vec3::Z * (q * (1. - q) * 100.),
                    225.,
                    if t < 1. {
                        "fall"
                    } else if t < 1. + d.duration("alice", "land") {
                        "land"
                    } else {
                        "ready"
                    },
                    if t < 1. { t } else { t - 1. },
                    t >= 1. + d.duration("alice", "land"),
                )
            } else if cart == Cart::Boarding {
                let f = fraction(p.state.time, 1.);
                (
                    p.state.board_from.lerp(p.cart_base + Vec3::Z * 16., f)
                        + Vec3::Z * (f * (1. - f) * 160.),
                    270.,
                    "jump",
                    p.state.time,
                    false,
                )
            } else if cart == Cart::Lift {
                let t = p.state.time;
                (
                    p.cart_base + Vec3::Z * (16. + 640. * fraction(t, 6.)),
                    270.,
                    if t >= 3. {
                        "sit_minecart"
                    } else if t < 0.5 {
                        "idle_base_02"
                    } else if t < 0.5 + d.duration("alice", "idle_base_02_2_shrug") {
                        "idle_base_02_2_shrug"
                    } else {
                        "idle_shrug"
                    },
                    if t >= 3. { t - 3. } else { (t - 0.5).max(0.) },
                    t >= 3. || t >= 0.5 + d.duration("alice", "idle_base_02_2_shrug"),
                )
            } else {
                let (pos, q) = p.rail_pose();
                let bump = d
                    .bumps
                    .iter()
                    .rev()
                    .find(|&&t| t <= p.state.time)
                    .map_or(0., |t| {
                        let age = p.state.time - t;
                        if age < 0.1 {
                            40. * age
                        } else {
                            4. * (1. - fraction(age - 0.1, 0.5))
                        }
                    });
                let dir = q * Vec3::X;
                (
                    pos + Vec3::Z * (16. + bump),
                    dir.y.atan2(dir.x).to_degrees(),
                    "sit_minecart",
                    p.state.time,
                    true,
                )
            };
            if s.beat != Some(Beat::Landing) || s.time + p.rail_times[90] - d.end_camera_time >= 6.
            {
                self.alice
                    .draw(clip, time, looping, pose(position, yaw), 1., fullbright);
            }
        }
        if s.beat == Some(Beat::BoardShip) {
            let f = fraction((s.time - 0.5).max(0.), 2.);
            let pos = d
                .at("alice_endship_posx1")
                .lerp(d.at("alice_airship_end1"), f);
            let (clip, t, l) = if f < 1. {
                ("walk", s.time, true)
            } else {
                d.acting(
                    "alice",
                    s.time - 2.5,
                    &[
                        ("idle_base_01_2_base_02", None),
                        ("idle_base_02", Some(2.)),
                        ("idle_base_02_2_base_03", None),
                        ("idle_base_03", Some(1000.)),
                    ],
                )
            };
            self.alice.draw(clip, t, l, pose(pos, 90.), 1., fullbright);
        } else if p.state.departure {
            self.alice.draw(
                "sit_airship",
                p.state.flight,
                true,
                Transform {
                    translation: p.ship_seat(),
                    rotation: p.ship_rotation(),
                },
                1.,
                fullbright,
            );
        }
        for i in 0..3 {
            let name = format!("minecart_torchgnome{}", i + 1);
            let start = d.at(&name);
            let end = d.at(&format!("{name}_node1"));
            let t = if cart == Cart::Finished {
                100.
            } else if cart == Cart::Rail {
                p.state.time - d.torch_times[i]
            } else {
                -1.
            };
            let delta = end - start;
            let duration = delta.length() / 100.;
            let f = fraction(t, duration);
            self.torch.draw(
                if t < 0. {
                    "idle"
                } else if f < 1. {
                    "run"
                } else {
                    "alert1"
                },
                if f < 1. { t.max(0.) } else { t - duration },
                t < duration,
                pose(
                    start.lerp(end, f),
                    if f < 1. {
                        delta.y.atan2(delta.x).to_degrees()
                    } else {
                        d.torch_yaws[i]
                    },
                ),
                1.,
                fullbright,
            );
        }
    }
    /// Depth-tested, deterministic disappearance sparks. Their clock is saved with the scene.
    pub fn draw_effects(
        &mut self,
        p: &Pandemonium,
        camera: Vec3,
        atmosphere: &crate::environment::Atmosphere,
    ) {
        if let Some((clip, time, looping, pose, scale)) = &self.pipe_pose {
            self.gnome
                .draw_effects(clip, *time, *looping, *pose, *scale, camera, atmosphere);
        }
        let s = &p.state.cinema;
        let effect = if s.beat == Some(Beat::Vanish) {
            Some((p.cinema.at("gnome_persuefire1"), s.time))
        } else if s.beat == Some(Beat::Return) {
            let t = s.time - 5. - p.cinema.duration("c_gnomeold", "letsgo");
            (t >= 0.).then(|| (p.cinema.at("gnome_home_fire1"), t))
        } else {
            None
        };
        if let Some((at, time)) = effect {
            self.burst.draw(at, time, camera, atmosphere);
        }
    }
}

/// Local-asset integration check: real trigger volumes, saved intermediate timelines,
/// and identical progression commits for watched and skipped playback.
pub fn camera_check(assets: &mut Assets) -> Result<()> {
    use crate::interaction::Interactions;
    let map = Bsp::parse(&assets.read("maps/pandemonium.bsp")?)?;
    let mut i = Interactions::load(&map)?;
    i.set_entry(assets, &map, "pandemonium", None)?;
    let mut world = World::from_bsp(&map)?;
    i.sync(&mut world);
    let mut p = Pandemonium::load(assets, &map)?;
    let mut max_boundary_turn = 0_f32;
    for t in p.rail_times.clone().into_iter().take(90).skip(1) {
        p.state.time = t - 0.0001;
        let before = p.rail_pose().1 * Vec3::X;
        p.state.time = t + 0.0001;
        let after = p.rail_pose().1 * Vec3::X;
        let angle = before.dot(after).clamp(-1., 1.).acos();
        max_boundary_turn = max_boundary_turn.max(angle);
        ensure!(
            angle < 0.01,
            "Minecart heading jumps at rail time {t}: {angle} radians"
        );
    }
    let initial = i.snapshot();
    for hz in [30, 60, 144] {
        i.restore(&initial, &map)?;
        i.sync(&mut world);
        let mut player = Player::new(vec3(-5024., 1368., 32.));
        i.triggers(1. / hz as f32, player.feet, player.feet);
        let mut rig = crate::camera::Follow::default();
        let mut samples = 0;
        let mut previous_arm: Option<Vec3> = None;
        let mut largest_change = 0_f32;
        for _ in 0..9000 {
            i.advance_school(1. / hz as f32, &map, &mut world, &mut player)?;
            let p = i.pandemonium.as_ref().unwrap();
            if p.rail_camera_active() {
                let desired = p.camera().unwrap();
                let camera = rig.rail(&world, desired, &p.rail_camera_ahead(), 1. / hz as f32);
                let half = Vec3::splat(3.9);
                let sight = world.sweep(desired.target, camera.eye, half);
                ensure!(
                    !world.sweep(camera.eye, camera.eye, half).start_solid
                        && !sight.start_solid
                        && sight.fraction > 0.999,
                    "Minecart camera in geometry at t={}, eye={:?}, pivot={:?}",
                    p.state.time,
                    camera.eye,
                    desired.target
                );
                let arm = camera.eye - desired.target;
                if let Some(previous) = previous_arm {
                    largest_change = largest_change.max(arm.distance(previous));
                }
                previous_arm = Some(arm);
                samples += 1;
            }
            if p.state.cart == Cart::Finished {
                break;
            }
        }
        ensure!(samples > 500, "Minecart camera was not exercised");
        ensure!(
            largest_change * (hz as f32) < 1200.,
            "Minecart camera still snaps by {largest_change} units in one frame"
        );
        println!("PASS minecart camera {hz} Hz: {samples} live rail frames, clear tunnel sightlines; node boundary turn {:.3} degrees, largest relative camera step {largest_change:.2}", max_boundary_turn.to_degrees());
    }
    Ok(())
}

pub fn check(assets: &mut Assets) -> Result<()> {
    use crate::{interaction::Interactions, movement::FIXED_DT};
    let map = Bsp::parse(&assets.read("maps/pandemonium.bsp")?)?;
    let hints = crate::cheshire::Hints::load(assets, &map, "pandemonium")?;
    for scene in ["warning", "cart", "return", "departure"] {
        let mut i = Interactions::load(&map)?;
        i.set_entry(assets, &map, "pandemonium", None)?;
        let mut w = World::from_bsp(&map)?;
        i.sync(&mut w);
        let mut p = Player::new(Vec3::ZERO);
        if matches!(scene, "return" | "departure") {
            i.pandemonium
                .as_mut()
                .unwrap()
                .fixture("pand-return", &map, &mut w, &mut p)?;
            i.pandemonium.as_mut().unwrap().state.leaving = false;
        }
        if scene == "departure" {
            p.feet = vec3(-3600., 800., -264.);
            i.triggers(FIXED_DT, p.feet, p.feet);
            let mut earlier = Story::load(assets, "pandemonium");
            ensure!(
                i.skip_cinematic(&map, &mut w, &mut p, &mut earlier)?,
                "Return staging failed"
            );
            i.update(FIXED_DT, &map, &mut w, &p, Vec3::Y, false)?;
        }
        p.feet = match scene {
            "warning" => vec3(-4406., 2608., -264.),
            "cart" => vec3(-5024., 1368., 32.),
            "return" => vec3(-3600., 800., -264.),
            _ => vec3(-3968., 2718., -232.),
        };
        let mut story = Story::load(assets, "pandemonium");
        let e = i.triggers(FIXED_DT, p.feet, p.feet);
        for id in e.story {
            story.trigger(&id);
        }
        ensure!(
            i.pandemonium.as_ref().unwrap().cinematic(),
            "{scene} contact failed"
        );
        let expected = match scene {
            "warning" => WARNING,
            "cart" => "Minecart_Thread",
            "return" => "alice_leave",
            _ => DEPARTURE,
        };
        ensure!(
            i.pandemonium.as_ref().unwrap().scene_id() == Some(expected),
            "Wrong triggered scene: {scene}, got {:?}",
            i.pandemonium.as_ref().unwrap().scene_id()
        );
        let initial = i.snapshot();
        let start_player = p.clone();
        let start_story = story.snapshot();
        let mut slices = Vec::new();
        let mut exits = 0;
        let dt = 1. / 30.;
        for tick in 0..9000 {
            if [0, 1, 60, 180, 360, 600, 900].contains(&tick) {
                slices.push((i.snapshot(), p.clone(), story.snapshot()));
            }
            cinematic_tick(&mut i, &map, &mut w, &mut p, &mut story, dt, &mut exits)?;
            if exits > 0 || !i.pandemonium.as_ref().unwrap().cinematic() {
                break;
            }
        }
        ensure!(
            exits > 0 || !i.pandemonium.as_ref().unwrap().cinematic(),
            "{scene} hung"
        );
        for _ in 0..60 {
            cinematic_tick(&mut i, &map, &mut w, &mut p, &mut story, dt, &mut exits)?;
        }
        let natural = endpoint(&i, &p, &story, exits)?;
        for (index, (state, player, dialogue)) in slices.into_iter().enumerate() {
            i.restore(&serde_json::from_slice(&serde_json::to_vec(&state)?)?, &map)?;
            i.sync(&mut w);
            p = player;
            story.restore(&dialogue, &hints)?;
            i.pandemonium.as_ref().unwrap().validate_player(&p)?;
            let frozen = serde_json::to_value(i.snapshot())?;
            i.advance_school(0., &map, &mut w, &mut p)?;
            ensure!(
                frozen == serde_json::to_value(i.snapshot())?,
                "Paused scene advanced"
            );
            ensure!(
                i.skip_cinematic(&map, &mut w, &mut p, &mut story)?,
                "{scene} skip rejected"
            );
            for id in story.take_completed() {
                i.completed_dialogue(&id);
            }
            let mut exits = 0;
            cinematic_tick(&mut i, &map, &mut w, &mut p, &mut story, dt, &mut exits)?;
            for _ in 0..60 {
                cinematic_tick(&mut i, &map, &mut w, &mut p, &mut story, dt, &mut exits)?;
            }
            ensure!(
                endpoint(&i, &p, &story, exits)? == natural,
                "{scene} skip {index} changed progression"
            );
            if scene != "departure" {
                ensure!(w.body_clear(p.feet), "Skip stranded Alice in geometry");
                ensure!(
                    !i.skip_cinematic(&map, &mut w, &mut p, &mut story)?,
                    "Completed scene skipped again"
                );
            }
            let after = i.snapshot();
            i.restore(&after, &map)?;
            i.sync(&mut w);
            cinematic_tick(&mut i, &map, &mut w, &mut p, &mut story, dt, &mut exits)?;
            ensure!(exits <= 1, "Duplicate exit after saved skip");
        }
        // Restore the trigger contact before the first frame and compare continued playback,
        // including its camera. This also covers saves made before dialogue becomes visible.
        i.restore(&initial, &map)?;
        i.sync(&mut w);
        p = start_player;
        story.restore(&start_story, &hints)?;
        let mut invalid = i.pandemonium.as_ref().unwrap().snapshot();
        invalid.cinema.time = -1.;
        ensure!(
            i.pandemonium
                .as_mut()
                .unwrap()
                .restore(&invalid, &map)
                .is_err(),
            "Malformed scene accepted"
        );
        let before = serde_json::to_value(i.snapshot())?;
        i.advance_school(0., &map, &mut w, &mut p)?;
        ensure!(
            before == serde_json::to_value(i.snapshot())?,
            "Pause changed scene"
        );
        println!("PASS {scene}: watched/skipped endpoints, original trigger, pause, saved intermediate skips, safe landing and one-shot completion");
    }
    Ok(())
}
fn cinematic_tick(
    i: &mut crate::interaction::Interactions,
    map: &Bsp,
    w: &mut World,
    p: &mut Player,
    story: &mut Story,
    dt: f32,
    exits: &mut usize,
) -> Result<()> {
    i.advance_school(dt, map, w, p)?;
    let mut e = i.update(dt, map, w, p, Vec3::Y, false)?;
    e.merge(i.triggers(dt, p.feet, p.feet));
    for id in e.story {
        story.trigger(&id);
    }
    story.tick(dt, false);
    i.sync_cinematic_story(story);
    for id in story.take_completed() {
        i.completed_dialogue(&id);
    }
    i.activate_enemies();
    if let Some(exit) = e.transition {
        ensure!(
            exit == ("fortress1".into(), Some("fortress1_start1".into())),
            "Wrong cinematic exit"
        );
        *exits += 1;
    }
    if let Some(camera) = i.pandemonium.as_ref().unwrap().camera() {
        ensure!(
            camera.eye.is_finite()
                && camera.target.is_finite()
                && (camera.target - camera.eye).length() > 1.
                && camera.up.length() > 0.99,
            "Invalid cinematic camera"
        );
    }
    Ok(())
}
fn endpoint(
    i: &crate::interaction::Interactions,
    p: &Player,
    story: &Story,
    exits: usize,
) -> Result<serde_json::Value> {
    let a = i.pandemonium.as_ref().unwrap();
    let s = &a.state;
    let movers = a
        .objects
        .iter()
        .filter(|o| {
            o.name.starts_with("mine")
                || o.name.starts_with("badtrack")
                || o.name == "guard_last_door"
        })
        .map(|o| {
            (
                &o.name,
                o.visible,
                o.solid,
                if o.visible && o.solid {
                    o.origin
                } else {
                    Vec3::ZERO
                },
                if o.visible && o.solid {
                    o.rotation
                } else {
                    Quat::IDENTITY
                },
            )
        })
        .collect::<Vec<_>>();
    let enemies = i
        .encounters
        .as_ref()
        .unwrap()
        .actors
        .iter()
        .map(|a| (&a.name, a.active))
        .collect::<Vec<_>>();
    Ok(
        serde_json::json!({"cart":s.cart,"rides":s.rides,"key":s.key,"returned":s.returned,"leaving":s.leaving,"departing":s.departure,"warning_done":s.cinema.warning_done,"return_done":s.cinema.return_done,"dialogue_done":s.dialogue_done,"exit_sent":s.exit_sent,"exits":exits,"completed":story.completed,"busy":story.busy(),"movers":movers,"enemies":enemies,"landing":if s.departure {Vec3::ZERO} else {p.feet}}),
    )
}

#[cfg(test)]
mod debris_tests {
    use super::*;
    #[test]
    fn collapse_delays_piece_axes_and_terminal_supports_match_declarations() {
        let base = vec3(100., 200., 300.);
        assert_eq!(debris("badtrack1_piece1", base, 0.9).translation, base);
        let first = debris("badtrack1_piece1", base, 1.25);
        let second = debris("badtrack2_piece1", base, 1.25);
        assert_eq!(first.translation, base + vec3(-16., 0., 16.));
        assert_eq!(second.translation, base + vec3(0., 16., 16.));
        assert!((first.rotation * Vec3::X).distance(Vec3::X) < 0.001);
        assert!((second.rotation * Vec3::Y).distance(Vec3::Y) < 0.001);
        assert_eq!(debris("badtrack1_piece3", base, 20.).translation, base);
        assert_eq!(
            debris("badtrack2_piece5", base, 20.).translation.z,
            base.z - 1280.
        );
        assert_eq!(
            debris("badtrack1_rock1", base, 1.).translation.z,
            base.z + 6.
        );
        for name in ["badtrack1_piece1", "badtrack1_piece2", "badtrack2_piece3"] {
            for at in [1., 2., 2.5, 3.5, 5.5, 6.5] {
                assert!(
                    debris(name, base, at - 0.0001)
                        .translation
                        .distance(debris(name, base, at + 0.0001).translation)
                        < 0.3
                );
            }
        }
    }
}
