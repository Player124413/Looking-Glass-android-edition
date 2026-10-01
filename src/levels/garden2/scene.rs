use super::*;
/// Split at rate boundaries so low frame rates cannot overshoot a slow-motion cue.
pub(super) fn local_delta(s: &Scene, dt: f32, d: &data::Data) -> f32 {
    if s.kind != Kind::Arrival || s.phase != Phase::Squish || s.skipping {
        return dt;
    }
    let end = 16.5_f32.max(11.5 + d.duration("c_madhatter", "stomp_yo_ass"));
    let mut t = s.elapsed();
    let mut remaining = dt;
    for (boundary, rate) in [(7., 1.), (10.5, 0.5), (end, 0.8), (3600., 1.)] {
        if t >= boundary {
            continue;
        }
        let used = remaining.min((boundary - t) / rate);
        t += used * rate;
        remaining -= used;
        if remaining <= 0. {
            break;
        }
    }
    t - s.elapsed()
}
fn travel(a: Transform, b: Transform, t: f32, speed: f32) -> Transform {
    let delta = b.translation - a.translation;
    let f = (t.max(0.) * speed / delta.length().max(0.001)).min(1.);
    Transform {
        translation: a.translation.lerp(b.translation, f),
        rotation: if f >= 1. {
            b.rotation
        } else {
            Quat::from_rotation_z(delta.y.atan2(delta.x))
        },
    }
}
impl Garden {
    fn path(&self, name: &str, time: f32) -> crate::cinematic::Camera {
        self.data.cameras[name].camera(time.max(0.))
    }
    pub(super) fn scene_camera(&self) -> Option<crate::cinematic::Camera> {
        let s = self.saved.scene.as_ref()?;
        let t = s.elapsed();
        let mut c = if s.kind.bridge() {
            let cut = self.data.collapse() - 1.3;
            let (a, b) = if s.kind == Kind::Second {
                ("garden2_jump3", "garden2_jump4")
            } else {
                ("garden2_jump1", "garden2_jump2")
            };
            if s.clock.time < cut {
                self.path(a, s.clock.time - 0.5)
            } else {
                self.path(b, s.clock.time - cut)
            }
        } else if s.kind == Kind::Cat {
            self.path("garden2_catend", s.clock.time - 0.5)
        } else {
            match s.phase {
                Phase::Approach => self.path("garden2_path1", s.clock.time),
                Phase::Talk => self.path("garden2_path1", s.start + t.min(1.5)),
                Phase::Run => self.path(
                    "garden2_path1",
                    self.data
                        .travel("c_whiterabbit", "run", "rabbit_pos1", "rabbit_pos2")
                        + 2.5
                        + t,
                ),
                Phase::Squish => {
                    let (n, start) = if t < 3.5 {
                        ("garden2_path2", 0.)
                    } else if t < 7. {
                        ("garden2_path3", 3.5)
                    } else if t < 10.5 {
                        ("garden2_path4", 7.)
                    } else if t < 15. {
                        ("garden2_path5", 10.5)
                    } else {
                        ("garden2_path7", 15.)
                    };
                    self.path(n, t - start)
                }
                Phase::Kneel | Phase::Outro => self.path(
                    "garden2_path7",
                    self.data
                        .travel("alice", "run", "alice_pos_squish3", "alice_pos_squish2")
                        + self.kneel_time(),
                ),
            }
        };
        // Source quakes, deterministic and restricted to the cinematic camera.
        let age = if s.kind.bridge() {
            s.clock.time - (self.data.collapse() - 0.8)
        } else if s.phase == Phase::Squish {
            t - 12.1
        } else {
            -1.
        };
        if (0. ..1.5).contains(&age) {
            let q = vec3((age * 71.).sin(), (age * 89.).sin(), (age * 59.).cos())
                * 1.2
                * (1. - age / 1.5);
            c.eye += q;
            c.target += q;
        }
        Some(c)
    }
    pub(super) fn scene_fade(&self) -> Option<(Color, f32)> {
        let Some(s) = &self.saved.scene else {
            return (self.saved.fade > 0.).then_some((WHITE, self.saved.fade / 1.5));
        };
        let t = s.elapsed();
        if s.skipping {
            return Some((WHITE, (t / 0.5).min(1.)));
        }
        let flash = |time: f32, at: f32, width: f32| (1. - (time - at).abs() / width).clamp(0., 1.);
        let alpha = if s.kind.bridge() {
            flash(s.clock.time, 0.5, 0.5)
                .max(flash(s.clock.time, self.data.collapse() - 1.3, 0.5))
                .max(
                    ((s.clock.time
                        - self.data.collapse()
                        - if s.kind == Kind::Second { 3.5 } else { 3. })
                        / 0.5)
                        .clamp(0., 1.),
                )
        } else if s.kind == Kind::Cat {
            flash(s.clock.time, 0.5, 0.5).max(if s.phase == Phase::Outro {
                ((t - 4.) / 0.5).clamp(0., 1.)
            } else {
                0.
            })
        } else if s.phase == Phase::Squish {
            (1. - t / 1.5)
                .max(0.)
                .max(flash(t, 3.5, 0.5))
                .max(flash(t, 7., 0.5))
                .max(flash(t, 10.5, 0.5))
                .max(flash(t, 15., 1.5))
        } else if s.phase == Phase::Outro {
            ((t - 3.5) / 1.5).clamp(0., 1.)
        } else {
            0.
        };
        Some((WHITE, alpha))
    }
    pub(super) fn alice_pose(&self) -> Transform {
        let Some(s) = &self.saved.scene else {
            return self.data.points["alice_pos_squish2"];
        };
        if s.kind != Kind::Arrival {
            return if s.kind == Kind::Second && s.clock.time >= self.data.collapse() - 1.3 {
                self.data.footing(
                    self.data.points["fakeplayer_bridge_pos2"],
                    crate::collision::PLAYER_HALF,
                )
            } else {
                s.clock.home.unwrap_or(self.data.points["alice_pos1"])
            };
        }
        let t = s.elapsed();
        let points = &self.data.points;
        let p = match s.phase {
            Phase::Approach | Phase::Talk | Phase::Run => points["alice_pos1"],
            Phase::Squish if t < 3.5 => travel(
                points["alice_pos1"],
                points["alice_pos_squish1"],
                t - 1.5,
                self.data.speed("alice", "run") * 1.5,
            ),
            Phase::Squish if t < 15. => travel(
                points["alice_pos_squish1"],
                points["alice_pos_squish3"],
                t - 3.5,
                self.data.speed("alice", "run"),
            ),
            Phase::Squish => travel(
                points["alice_pos_squish3"],
                points["alice_pos_squish2"],
                t - 15.,
                self.data.speed("alice", "run"),
            ),
            Phase::Kneel | Phase::Outro => points["alice_pos_squish2"],
        };
        self.data.footing(p, crate::collision::PLAYER_HALF)
    }
    pub(super) fn rabbit_pose(&self, tiny: bool) -> Transform {
        let p = &self.data.points;
        let Some(s) = &self.saved.scene else {
            return p["rabbit_pos3"];
        };
        if tiny {
            return if s.phase == Phase::Squish {
                travel(
                    p["rabbit_tiny_pos2"],
                    p["rabbit_pos_dead"],
                    s.elapsed() - 11.5,
                    self.data.speed("c_whiterabbit", "run") * 0.1,
                )
            } else {
                p["rabbit_tiny_pos2"]
            };
        }
        let pose = match s.phase {
            Phase::Approach => travel(
                p["rabbit_pos1"],
                p["rabbit_pos2"],
                s.clock.time,
                self.data.speed("c_whiterabbit", "run"),
            ),
            Phase::Talk => p["rabbit_pos2"],
            Phase::Run => travel(
                p["rabbit_pos2"],
                p["rabbit_pos3"],
                s.elapsed() - 0.5,
                self.data.speed("c_whiterabbit", "run"),
            ),
            _ => p["rabbit_pos3"],
        };
        self.data.footing(pose, vec3(16., 16., 32.))
    }
    pub(super) fn hatter_pose(&self) -> Transform {
        let p = &self.data.points;
        let Some(s) = &self.saved.scene else {
            return p["rabbit_tiny_pos1"];
        };
        if s.kind.bridge() {
            return p[if s.kind == Kind::Second {
                "hatter_jump_pos2"
            } else {
                "hatter_jump_pos1"
            }];
        }
        if s.phase == Phase::Squish {
            if s.elapsed() >= 10.5 {
                return p["rabbit_tiny_pos1"];
            }
            return travel(
                p["hatter_newpos1"],
                p["hatter_newpos3"],
                s.elapsed() - 7.,
                self.data.speed("c_madhatter", "walk"),
            );
        }
        if matches!(s.phase, Phase::Kneel | Phase::Outro) {
            p["rabbit_tiny_pos1"]
        } else {
            p["hatter_newpos1"]
        }
    }
    fn sequence(
        &self,
        m: &str,
        mut t: f32,
        steps: &[(&'static str, f32)],
        last: &'static str,
    ) -> (&'static str, f32, bool) {
        t = t.max(0.);
        for &(clip, extra) in steps {
            let d = self.data.duration(m, clip) + extra;
            if t < d {
                return (clip, t, extra > 0.);
            }
            t -= d;
        }
        (last, t, true)
    }
    pub(super) fn acting(&self, m: &str) -> (&'static str, f32, bool) {
        let s = self.saved.scene.as_ref();
        let t = s.map_or(0., |s| s.elapsed());
        if m == "c_madhatter" {
            if let Some(s) = s {
                if s.kind.bridge() {
                    let t = s.clock.time - 1.5;
                    if t >= 0. && t < self.data.duration(m, "jump") {
                        return ("jump", t, false);
                    }
                }
                if s.kind == Kind::Arrival && s.phase == Phase::Squish {
                    if (7. ..10.).contains(&t) {
                        return ("walk", t - 7., true);
                    }
                    if t >= 11.5 && t < 11.5 + self.data.duration(m, "stomp_yo_ass") {
                        return ("stomp_yo_ass", t - 11.5, false);
                    }
                }
            }
            return ("ready", 0., true);
        }
        if m == "c_whiterabbit" {
            if let Some(s) = s {
                if s.phase == Phase::Approach || s.phase == Phase::Run {
                    return ("run", t, true);
                }
                if s.phase == Phase::Squish && t >= 1.5 {
                    return self.sequence(
                        m,
                        t - 1.5,
                        &[("i_calm_front_2_right", 0.)],
                        "i_calm_eartwitch_l",
                    );
                }
            }
            return ("i_calm_l", t, true);
        }
        if m == "c_cheshire" {
            if let Some(s) = s {
                if s.kind == Kind::Cat {
                    return self.sequence(
                        m,
                        s.clock.time - 2.5,
                        &[
                            ("sit_idle1", 0.),
                            ("sit_smile_open", 0.),
                            ("sit_smile_shut", 0.),
                        ],
                        "sit_idle1",
                    );
                }
            }
            return self.sequence(
                m,
                (self.kneel_time() - 5.).max(0.),
                &[
                    ("sit_idle1", 6. - self.data.duration(m, "sit_idle1")),
                    ("sit_talk2", 0.),
                    ("sit_talk1", 0.),
                ],
                "sit_idle1",
            );
        }
        if let Some(s) = s {
            if s.kind == Kind::Arrival {
                if s.phase == Phase::Squish && ((1.5..7.).contains(&t) || t >= 15.) {
                    return ("run", t, true);
                }
                if matches!(s.phase, Phase::Kneel | Phase::Outro) {
                    return self.sequence(
                        m,
                        self.kneel_time(),
                        &[
                            ("idle_base_02", 0.),
                            ("idle_base_02_kneel", 0.),
                            ("kneel_idle", 5.),
                            ("kneel_shakeno", 0.),
                            ("kneel_2_weep", 0.),
                            ("weep_sobbing", 8.),
                            ("weep_2_kneel", 0.),
                            ("kneel_2_base_02", 0.),
                        ],
                        "idle_base_02",
                    );
                }
            }
        }
        ("idle", t, true)
    }
    pub(super) fn kneel_time(&self) -> f32 {
        let Some(s) = &self.saved.scene else {
            return 0.;
        };
        if s.phase == Phase::Kneel {
            s.elapsed()
        } else if s.phase == Phase::Outro && s.kind == Kind::Arrival {
            s.clock.time - s.kneel_at.unwrap_or(s.start)
        } else {
            0.
        }
    }
    pub(super) fn cat_alpha(&self) -> f32 {
        let Some(s) = &self.saved.scene else {
            return 0.;
        };
        if s.kind == Kind::Cat {
            return if s.phase == Phase::Outro {
                (1. - (s.elapsed() - 1.) / 2.).clamp(0., 1.)
            } else {
                ((s.clock.time - 0.5) / 2.).clamp(0., 1.)
            };
        }
        if s.kind == Kind::Arrival && matches!(s.phase, Phase::Kneel | Phase::Outro) {
            let t = self.kneel_time() - 5.;
            let end = 6.
                + self.data.duration("c_cheshire", "sit_talk2")
                + self.data.duration("c_cheshire", "sit_talk1");
            return (t / 2.).clamp(0., 1.) * (1. - (t - end) / 2.).clamp(0., 1.);
        }
        0.
    }
    pub(super) fn sounds(&self, out: &mut Vec<crate::audio::world::Clock>) {
        let Some(s) = &self.saved.scene else { return };
        if s.skipping {
            return;
        }
        if s.kind.bridge() {
            out.push(crate::audio::world::Clock {
                key: "garden2.bridge.jump",
                time: s.clock.time - (1.5 + self.data.duration("c_madhatter", "jump")),
                period: None,
                origin: self.alice_pose().translation,
                cues: &[
                    (0., "sound/ambience/special/marble1.wav"),
                    (1., "sound/ambience/special/quake1.wav"),
                ],
            });
            out.push(crate::audio::world::Clock {
                key: "garden2.bridge.break",
                time: s.clock.time - self.data.collapse(),
                period: None,
                origin: self.alice_pose().translation,
                cues: if s.kind == Kind::Second {
                    &[
                        (-0.8, "sound/ambience/special/stone_breaking1.wav"),
                        (0., "sound/ambience/special/rock_falling1.wav"),
                        (0.2, "sound/ambience/special/thronebreak2.wav"),
                        (0.8, "sound/ambience/special/thronebreak3.wav"),
                    ]
                } else {
                    &[
                        (-0.8, "sound/ambience/special/rock_falling1.wav"),
                        (0., "sound/ambience/special/stone_breaking1.wav"),
                    ]
                },
            });
        }
        if s.kind == Kind::Arrival && s.phase == Phase::Squish {
            out.push(crate::audio::world::Clock {
                key: "garden2.steps",
                time: s.elapsed(),
                period: None,
                origin: self.alice_pose().translation,
                cues: &[
                    (1.5, "sound/ambience/special/quake_step1.wav"),
                    (5., "sound/ambience/special/quake_step1.wav"),
                    (7., "sound/ambience/special/quake_step1.wav"),
                    (7.75, "sound/ambience/special/quake_step2.wav"),
                    (8.5, "sound/ambience/special/quake_step1.wav"),
                    (9.25, "sound/ambience/special/quake_step1.wav"),
                    (12.1, "sound/ambience/special/quake_step1.wav"),
                ],
            });
        }
        if s.kind == Kind::Arrival && s.phase == Phase::Squish {
            out.push(crate::audio::world::Clock {
                key: "garden2.hatter.stomp",
                time: s.elapsed() - 11.5,
                period: None,
                origin: self.alice_pose().translation,
                cues: &[(0., "sound/character/mad_hatter/stomp_yo_ass.wav")],
            });
        }
        if s.kind == Kind::Cat && s.phase == Phase::Outro {
            out.push(crate::audio::world::Clock {
                key: "garden2.cat.depart",
                time: s.elapsed() - 1.,
                period: None,
                origin: self.data.points["cat_end_pos"].translation,
                cues: &[(0., "sound/character/cheshire_cat/disappear.wav")],
            });
        }
        let t = if s.kind == Kind::Cat {
            s.clock.time - 0.5
        } else {
            self.kneel_time() - 5.
        };
        if self.cat_alpha() > 0. {
            out.push(crate::audio::world::Clock {
                key: "garden2.cat",
                time: t,
                period: None,
                origin: self.data.points[if s.kind == Kind::Cat {
                    "cat_end_pos"
                } else {
                    "cat_squish_pos1"
                }]
                .translation,
                cues: &[(0., "sound/character/cheshire_cat/appear.wav")],
            });
        }
    }
}
