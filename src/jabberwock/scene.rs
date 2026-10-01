use super::*;
pub(super) fn cameras(k: Kind) -> &'static [&'static str] {
    if k == Kind::Lair {
        &[
            "jlair_path1",
            "jlair_path2",
            "jlair_path3",
            "jlair_path4",
            "jlair_path5",
            "jlair_eye1",
            "jlair_eye2",
            "jlair_eye3",
            "jlair_path8",
        ]
    } else {
        &[
            "grounds1_a1",
            "grounds1_gintro3",
            "grounds1_jintro3",
            "grounds1_takeoff",
            "grounds1_strike1",
            "grounds1_flyby",
            "grounds1_closer",
            "grounds1_jroar",
            "grounds1_fire",
            "grounds1_cross",
            "grounds1_jdive",
            "grounds1_fall1",
            "grounds1_fall2",
            "grounds1_ga1",
            "grounds1_bridge2",
            "grounds1_jend",
            "grounds1_end1",
            "grounds1_deathrise",
        ]
    }
}
pub(super) struct Performance {
    pub model: &'static str,
    pub clip: &'static str,
    pub time: f32,
    pub loops: bool,
    pub pose: Transform,
}
impl Performance {
    fn new(
        model: &'static str,
        clip: &'static str,
        time: f32,
        loops: bool,
        pose: Transform,
    ) -> Self {
        Self {
            model,
            clip,
            time: time.max(0.),
            loops,
            pose,
        }
    }
}
impl Encounter {
    fn bridge_start(&self) -> f32 {
        39.85 + self.data.rigs["c_gryphon"].duration("death_crash")
    }
    pub fn intro_talk_start(&self) -> f32 {
        if self.saved.kind == Kind::Lair {
            3.
        } else {
            self.bridge_start() + 8. + self.data.boss().duration("fly_landing")
        }
    }
    pub fn brush_poses(&self) -> Vec<(usize, Vec3, Quat)> {
        let s = &self.saved;
        self.data
            .brushes
            .iter()
            .map(|b| {
                let mut p = b.pose;
                p.translation += b.slide * (s.door / 2.);
                let collapse = if s.phase == Phase::Intro {
                    (s.time - self.bridge_start() - 3.).max(0.)
                } else {
                    8.
                };
                let opening = if s.phase == Phase::Bridge {
                    ((s.time - 3.) / 10.).clamp(0., 1.)
                } else if s.phase == Phase::Done {
                    1.
                } else {
                    0.
                };
                if (b.name == "wock_clips" && s.phase == Phase::Intro)
                    || (b.name == "jabberjump_clips" && (s.phase == Phase::Intro || opening > 0.))
                {
                    p.translation.z -= 100000.;
                }
                if b.name.starts_with("plank_") {
                    let f = (collapse / 5.).min(1.);
                    p.translation.z -= 2400. * f;
                    p.translation.y += if b.name.ends_with('1') {
                        64. * f
                    } else {
                        -64. * f
                    };
                    p.rotation *= Quat::from_rotation_y(55_f32.to_radians() * f)
                        * Quat::from_rotation_x(45_f32.to_radians() * f);
                }
                if matches!(b.name.as_str(), "bridge_east" | "bridge_west") {
                    let angle = if b.name == "bridge_west" {
                        90_f32
                    } else {
                        -90_f32
                    };
                    p.rotation *= Quat::from_rotation_y(angle.to_radians() * collapse.min(1.));
                }
                if b.name == "drawbridge" {
                    p.rotation *= Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2 * opening);
                }
                if b.name.starts_with("drawbridge_gear") {
                    p.rotation *= Quat::from_rotation_x(
                        1440_f32.to_radians()
                            * opening
                            * if b.name.ends_with("01") { -1. } else { 1. },
                    );
                }
                (b.index, p.translation, p.rotation)
            })
            .collect()
    }
    pub fn scene_camera(&self) -> Option<crate::cinematic::Camera> {
        let s = &self.saved;
        let t = s.time;
        let (name, clock) = match (s.kind, s.phase) {
            (Kind::Lair, Phase::Intro) => {
                let i = s.dialogue_line.min(7);
                (
                    [
                        "jlair_path1",
                        "jlair_path2",
                        "jlair_path3",
                        "jlair_path3",
                        "jlair_path3",
                        "jlair_path4",
                        "jlair_path4",
                        "jlair_path5",
                    ][i],
                    if t < 3. { t } else { s.dialogue_time },
                )
            }
            (Kind::Lair, Phase::Outro) => {
                if t < 2.1 {
                    ("jlair_eye3", t)
                } else if t < 3.8 {
                    ("jlair_eye1", t - 2.1)
                } else if t < 10.55 {
                    ("jlair_eye2", t - 3.8)
                } else {
                    ("jlair_path8", t - 10.55)
                }
            }
            (Kind::Grounds, Phase::Intro) => {
                let shots = [
                    (0., "grounds1_a1"),
                    (5., "grounds1_gintro3"),
                    (8., "grounds1_jintro3"),
                    (11., "grounds1_takeoff"),
                    (14.5, "grounds1_strike1"),
                    (17.9, "grounds1_flyby"),
                    (19.2, "grounds1_closer"),
                    (21.4, "grounds1_jroar"),
                    (23.4, "grounds1_fire"),
                    (27.45, "grounds1_cross"),
                    (29.45, "grounds1_jdive"),
                    (31.45, "grounds1_fall1"),
                    (33.35, "grounds1_fall2"),
                    (34.85, "grounds1_ga1"),
                    (self.bridge_start(), "grounds1_bridge2"),
                    (self.bridge_start() + 6., "grounds1_jend"),
                ];
                let (at, n) = shots.iter().rev().find(|(at, _)| t >= *at).unwrap();
                (*n, t - at)
            }
            (Kind::Grounds, Phase::Outro) => {
                if s.dialogue_done && s.after_talk >= 5.85 {
                    ("grounds1_deathrise", s.after_talk - 5.85)
                } else {
                    ("grounds1_end1", t)
                }
            }
            _ => return None,
        };
        let p = self.data.cameras[name].sample(clock, false);
        let mut target = p.translation + p.rotation * Vec3::X * 100.;
        if s.kind == Kind::Lair && s.phase == Phase::Intro && s.dialogue_line >= 2 {
            target = self.lair_walk().2.translation + Vec3::Z * 140.;
        }
        if s.kind == Kind::Lair && s.phase == Phase::Outro && (7.55..10.55).contains(&t) {
            target = self.data.path("t6", t - 7.55).translation + Vec3::Z * 100.;
        }
        Some(crate::cinematic::Camera::look(p.translation, target))
    }
    fn point(&self, name: &str, fallback: Transform) -> Transform {
        self.data.points.get(name).copied().unwrap_or(fallback)
    }
    fn lair_walk(&self) -> (&'static str, f32, Transform) {
        let mut from = self.data.points["jabber_actor1"];
        let Some(start) = self.saved.walk_started else {
            return ("idle1", self.saved.time, from);
        };
        let mut t = (self.saved.time - start).max(0.);
        for (i, name) in [
            "jabber_pos1",
            "jabber_pos2",
            "jabber_pos1",
            "jabber_pos2",
            "jabber_pos1",
            "jabber_pos2",
            "jabber_pos3",
        ]
        .iter()
        .enumerate()
        {
            if i == 6 {
                if t < 5. {
                    return ("idle1", t, from);
                }
                t -= 5.;
            }
            let to = self.data.points[*name];
            let clip = if i % 2 == 0 { "walk_1" } else { "walk_2" };
            let duration = from.translation.distance(to.translation)
                / crate::ant::Timing::speed(&self.data, "c_jabberwock", clip).max(1.);
            if t < duration {
                let mut pose = from;
                pose.translation = from.translation.lerp(to.translation, t / duration);
                let d = to.translation - from.translation;
                pose.rotation = Quat::from_rotation_z(d.y.atan2(d.x));
                return (clip, t, pose);
            }
            t -= duration;
            from = to;
        }
        ("ready_idle1", t, from)
    }
    pub(super) fn performances(&self) -> Vec<Performance> {
        let s = &self.saved;
        let b = &s.boss;
        let t = s.time;
        let mut out = vec![];
        let mut add = |m, c, t, l, p| out.push(Performance::new(m, c, t, l, p));
        match (s.kind, s.phase) {
            (Kind::Lair, Phase::Intro) => {
                let (clip, at, p) = self.lair_walk();
                add("c_jabberwock", clip, at, true, p);
                add(
                    "alice",
                    "idle_stand",
                    t,
                    true,
                    self.point(
                        "jlair2_start1",
                        Transform {
                            translation: vec3(-848., 416., 96.),
                            rotation: Quat::IDENTITY,
                        },
                    ),
                );
            }
            (Kind::Lair, Phase::Outro) => {
                let mut p = self.data.points["j"];
                let eye = self.data.points["eye_point1"];
                let (clip, at, loops) = if t < 0.5 {
                    ("attack1d", 0., false)
                } else if t < 2.1 {
                    p.translation = p
                        .translation
                        .lerp(eye.translation, ((t - 0.5) / 1.6).min(1.));
                    ("walk_1", 0.5, true)
                } else if t < 3.3 {
                    p = eye;
                    ("ready_idle1", 2.1, true)
                } else if t < 3.9 {
                    p = eye;
                    ("attack4a", 3.3, false)
                } else if t < 3.95 {
                    p = eye;
                    ("eyeloss", 3.9, false)
                } else if t < 4.55 {
                    p = eye;
                    ("pain3", 3.95, false)
                } else if t < 7.55 {
                    p = eye;
                    ("alert2", 4.55, false)
                } else {
                    p = self.data.path("t6", t - 7.55);
                    ("fly_1", 7.55, true)
                };
                if t < 10.55 {
                    add("c_jabberwock", clip, t - at, loops, p);
                }
                let ap = if t < 10.55 {
                    Transform {
                        translation: vec3(-600., 536., 96.),
                        rotation: Quat::from_rotation_z(std::f32::consts::PI),
                    }
                } else {
                    self.data.points["alice_gryphon1"]
                };
                add(
                    "alice",
                    if t < 0.8 {
                        "idle_stand"
                    } else if t < 2.1 {
                        "pain_knockdown"
                    } else if t < 10.55 {
                        "sit"
                    } else {
                        "idle_stand"
                    },
                    (t - 0.8).max(0.),
                    t >= 2.1,
                    ap,
                );
                if t >= 0.9 {
                    let gp = if s.dialogue_done {
                        self.data.path("jabber_leave1", s.after_talk)
                    } else if t < 10.55 {
                        self.data.path("p6", t - 0.9)
                    } else {
                        self.data.points["gryphon_pos1"]
                    };
                    add(
                        "c_gryphon",
                        if s.dialogue_done {
                            "fly"
                        } else if t < 10.55 {
                            if (3.4..4.55).contains(&t) {
                                "fly_dive02_strike"
                            } else {
                                "fly_dive02"
                            }
                        } else {
                            "idle"
                        },
                        t - 0.9,
                        true,
                        gp,
                    );
                }
            }
            (Kind::Grounds, Phase::Intro) => {
                let bridge = self.bridge_start();
                let jbase = self.point("cine_wock", b.pose());
                let gbase = self.point("cine_gryphon", self.data.points["gryphon_dying_pos1"]);
                let (j, g, jc, gc, at) = if t < 11. {
                    (jbase, gbase, "idle1", "idle", 0.)
                } else if t < 14.5 {
                    let mut j = jbase;
                    let mut g = gbase;
                    j.translation.z += (t - 12.).max(0.) * 160.;
                    g.translation.z += (t - 11.) * 160.;
                    (j, g, "takeoff", "fly_takeoff", 11.)
                } else if t < 17.9 {
                    (
                        self.data.path("jabberwock_path", t - 14.5),
                        self.data.path("gryphon_path", t - 14.5),
                        "fly_attack_breath",
                        "fly_dive02_strike",
                        14.5,
                    )
                } else if t < 21.4 {
                    (
                        self.data.path("jreact_path", t - 17.9),
                        self.data.path("side_strike_path", (t - 18.7).max(0.)),
                        "fly_pain2",
                        "fly_dive02_strike",
                        17.9,
                    )
                } else if t < 23.4 {
                    (
                        self.data.path("jreact_path", 4.),
                        self.data.path("side_strike_path", 4.),
                        "alert1",
                        "fly_idle",
                        21.4,
                    )
                } else if t < 27.45 {
                    (
                        self.data.path("wock_fire", t - 22.9),
                        self.data.path("gryphon_fire", t - 22.9),
                        "fly_attack_breath",
                        "fly_pain02",
                        23.4,
                    )
                } else if t < 29.45 {
                    (
                        self.data.path("flyby_south", t - 27.45),
                        self.data.path("flyby_north", t - 27.45),
                        "fly_1",
                        "fly_dive02",
                        27.45,
                    )
                } else if t < 34.85 {
                    (
                        self.data.path("wock_dive_path", (t - 31.45).max(0.)),
                        self.point("cine_gryphon_fall", gbase),
                        "fly_attack_dive",
                        "death_plunge",
                        29.45,
                    )
                } else if t < bridge {
                    (
                        self.data.path("wock_dive_path", 6.),
                        self.data.points["gryphon_dying_pos1"],
                        "fly_2",
                        "death_crash",
                        37.35,
                    )
                } else if t < bridge + 6. {
                    (
                        self.data.path("bridge_path", t - bridge),
                        self.data.points["gryphon_dying_pos1"],
                        if t < bridge + 2.5 {
                            "fly_1"
                        } else {
                            "fly_attack_breath"
                        },
                        "death_speech",
                        bridge,
                    )
                } else {
                    (
                        b.pose(),
                        self.data.points["gryphon_dying_pos1"],
                        "fly_landing",
                        "death_speech",
                        bridge + 6.,
                    )
                };
                add("c_jabberwock", jc, t - at, t < bridge + 6., j);
                add("c_gryphon", gc, t - at, t < 34.85 || t >= bridge, g);
                let ap = if t < 32.35 {
                    self.data.points["grounds1_start1"]
                } else {
                    let mut p = self.data.points["grounds1_start1"];
                    p.translation = p.translation.lerp(
                        self.data.points["gdeath"].translation,
                        ((t - 32.35) / 5.).clamp(0., 1.),
                    );
                    p
                };
                add(
                    "alice",
                    if (32.35..37.35).contains(&t) {
                        "run"
                    } else {
                        "idle_stand"
                    },
                    (t - 32.35).max(0.),
                    true,
                    ap,
                );
            }
            (Kind::Grounds, Phase::Outro) => {
                add(
                    "alice",
                    if !s.dialogue_done || s.after_talk < 4.45 {
                        "kneel_idle"
                    } else if s.after_talk < 4.85 {
                        "kneel_2_weep"
                    } else {
                        "weep_sobbing"
                    },
                    s.after_talk,
                    true,
                    self.data.points["alice_end1"],
                );
                add(
                    "c_gryphon",
                    if !s.dialogue_done {
                        "death_speech"
                    } else if s.after_talk < 3.95 {
                        "death_lastbreath"
                    } else {
                        "dead"
                    },
                    s.after_talk,
                    !s.dialogue_done,
                    self.data.points["gryphon_dying_pos1"],
                );
            }
            _ => {
                if s.phase == Phase::Fight || s.phase == Phase::Death {
                    add("c_jabberwock", b.clip(), b.time, b.loops(), b.pose());
                }
                if s.kind == Kind::Grounds {
                    add(
                        "c_gryphon",
                        if matches!(s.phase, Phase::Bridge | Phase::Done) {
                            "dead"
                        } else {
                            "death_speech"
                        },
                        t,
                        true,
                        self.data.points["gryphon_dying_pos1"],
                    );
                }
            }
        }
        if s.kind == Kind::Grounds {
            if let Some(t) = s.cat {
                if !s.cat_ending || t < 2. {
                    add(
                        "c_cheshire",
                        if s.cat_ending {
                            "sit_idle1"
                        } else {
                            "sit_talk1"
                        },
                        t,
                        true,
                        self.data.points["cat"],
                    );
                }
            }
            if let Some(t) = s.gnome {
                if t < 7. {
                    let mut pose = self.data.points["dead_guard"];
                    if t > 1. {
                        let age = t - 1.;
                        pose.translation += vec3(-100. * age, 0., -400. * age * age);
                        pose.translation.z = pose.translation.z.max(220.);
                    }
                    add(
                        "cardguard_diamond",
                        "death_1",
                        (t - 1.).max(0.),
                        false,
                        pose,
                    );
                }
                if t >= 2. {
                    let mut age = t - 2.;
                    let mut from = self.data.points["gnome"];
                    for (i, name) in ["gate_ledge", "gate_back", "gate_hide"].iter().enumerate() {
                        if i == 1 {
                            let hold = self.data.rigs["c_gnomeold"].duration("letsgo");
                            if age < hold {
                                add("c_gnomeold", "letsgo", age, false, from);
                                break;
                            }
                            age -= hold;
                        }
                        let to = self.data.points[*name];
                        let duration = from.translation.distance(to.translation)
                            / crate::ant::Timing::speed(&self.data, "c_gnomeold", "walk").max(1.);
                        if age < duration {
                            let mut p = from;
                            p.translation = p.translation.lerp(to.translation, age / duration);
                            let d = to.translation - from.translation;
                            p.rotation = Quat::from_rotation_z(d.y.atan2(d.x));
                            add("c_gnomeold", "walk", age, true, p);
                            break;
                        }
                        age -= duration;
                        from = to;
                    }
                }
            }
        }
        out
    }
}
