use super::*;
pub(super) fn travel(a: Transform, b: Transform, t: f32, speed: f32) -> Transform {
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
fn jump(a: Transform, b: Transform, t: f32, duration: f32) -> Transform {
    let f = (t / duration).clamp(0., 1.);
    let mut p = travel(a, b, f, a.translation.distance(b.translation));
    p.translation.z += 0.5 * crate::movement::GRAVITY * duration * duration * f * (1. - f);
    p
}
impl Garden {
    pub(super) fn scene_camera(&self) -> Option<crate::cinematic::Camera> {
        let s = self.saved.scene.as_ref()?;
        let (track, time) = match (s.kind, s.phase) {
            (Kind::Arrival, Phase::Launch) => (0, s.clock.time),
            (Kind::Arrival, Phase::Setup) if s.elapsed() < 0.5 => {
                (0, self.data.launch_end() + s.elapsed())
            }
            (Kind::Arrival, Phase::Setup) => (1, s.elapsed() - 0.5),
            (Kind::Arrival, Phase::Talk) => (1, s.elapsed() + 2.1),
            (Kind::Arrival, Phase::Leave) if s.elapsed() < 0.5 || s.skipping => {
                (1, s.clock.time - self.data.launch_end() - 0.5)
            }
            (Kind::Arrival, Phase::Leave) => (3, s.elapsed() - 0.5),
            (Kind::Rabbit, _) => (2, (s.clock.time - 0.5).max(0.)),
        };
        Some(self.data.cameras[track].camera(time))
    }
    pub(super) fn scene_fade(&self) -> Option<(Color, f32)> {
        let Some(s) = &self.saved.scene else {
            return (self.saved.fade > 0.).then_some((WHITE, self.saved.fade / 0.5));
        };
        let t = s.elapsed();
        if s.skipping {
            return Some((WHITE, (t / 0.5).min(1.)));
        }
        let alpha = match (s.kind, s.phase) {
            (Kind::Arrival, Phase::Launch) => {
                return Some((BLACK, (1. - s.clock.time / 3.).max(0.)))
            }
            (_, Phase::Setup) if t < 0.5 => t / 0.5,
            (Kind::Arrival, Phase::Setup) => (1. - (t - 0.5) / 0.5).max(0.),
            (Kind::Rabbit, Phase::Talk) => (1. - t / 0.5).max(0.),
            (Kind::Arrival, Phase::Leave) if t < 0.5 => t / 0.5,
            (Kind::Arrival, Phase::Leave) => (1. - (t - 1.6) / 0.5).clamp(0., 1.),
            (Kind::Rabbit, Phase::Leave) => ((t - self.data.rabbit_run() - 2.) / 0.5).clamp(0., 1.),
            _ => 0.,
        };
        Some((WHITE, alpha))
    }
    pub(super) fn alice_pose(&self) -> Transform {
        let Some(s) = &self.saved.scene else {
            return self.data.points["turtle_waterpos1"];
        };
        if s.kind == Kind::Arrival {
            if s.phase == Phase::Launch || (s.phase == Phase::Setup && s.elapsed() < 0.5) {
                return self.data.launch_pose(s.clock.time - 2.);
            }
            return self.data.footing(
                self.data.points["alice_pos1"],
                crate::collision::PLAYER_HALF,
            );
        }
        let a = s.clock.home.unwrap_or(self.data.points["alice_posx1"]);
        self.data.footing(
            travel(
                a,
                self.data.points["alice_posx1"],
                s.clock.time - 0.5,
                self.data.speed("alice", "walk"),
            ),
            crate::collision::PLAYER_HALF,
        )
    }
    pub(super) fn turtle_pose(&self) -> Transform {
        if let Some(s) = &self.saved.scene {
            if s.kind == Kind::Arrival
                && s.phase == Phase::Leave
                && !s.skipping
                && s.elapsed() >= 0.5
            {
                return if s.elapsed() < 1.6 {
                    jump(
                        self.data.points["turtle_waterpos1"],
                        self.data.points["turtle_jumppos1"],
                        s.elapsed() - 0.6,
                        1.,
                    )
                } else {
                    self.data.swim.sample(s.elapsed() - 1.6, true)
                };
            }
        }
        self.data
            .footing(self.data.points["turtle_pos1"], vec3(24., 24., 48.))
    }
    pub(super) fn rabbit_pose(&self) -> Transform {
        let a = self.data.points["rabbit_posx1"];
        if let Some(s) = &self.saved.scene {
            if s.kind == Kind::Rabbit && s.phase == Phase::Leave && !s.skipping {
                let b = self.data.points["rabbit_jumppos1"];
                if s.elapsed() < self.data.rabbit_run() {
                    return self.data.footing(
                        travel(a, b, s.elapsed(), self.data.speed("c_whiterabbit", "run")),
                        vec3(16., 16., 32.),
                    );
                }
                return jump(
                    b,
                    self.data.points["rabbit_jump1"],
                    s.elapsed() - self.data.rabbit_run(),
                    self.data.duration("c_whiterabbit", "jump").min(2.),
                );
            }
        }
        self.data.footing(a, vec3(16., 16., 32.))
    }
    fn sequence(
        &self,
        model: &str,
        mut time: f32,
        steps: &[(&'static str, Option<f32>)],
        last: &'static str,
    ) -> (&'static str, f32, bool) {
        time = time.max(0.);
        for &(clip, wait) in steps {
            let d = wait.unwrap_or_else(|| self.data.duration(model, clip));
            if time < d {
                return (clip, time, wait.is_some());
            }
            time -= d;
        }
        (last, time, true)
    }
    pub(super) fn acting(&self, model: &str) -> (&'static str, f32, bool) {
        let Some(s) = &self.saved.scene else {
            return (
                if model == "c_whiterabbit" {
                    "i_calm_l"
                } else {
                    "idle_base"
                },
                0.,
                true,
            );
        };
        if model == "alice" {
            if s.kind == Kind::Arrival {
                if s.phase == Phase::Launch || (s.phase == Phase::Setup && s.elapsed() < 0.5) {
                    let t = s.clock.time;
                    return if t < 3. {
                        ("jump_falling1", (t - 2.).max(0.), true)
                    } else if t < 3. + self.data.duration("alice", "pain_knockdown") {
                        ("pain_knockdown", t - 3., false)
                    } else {
                        (
                            "idle",
                            t - 3. - self.data.duration("alice", "pain_knockdown"),
                            false,
                        )
                    };
                }
                return self.sequence(
                    model,
                    s.clock.time - self.data.launch_end() - 0.6,
                    &[("idle_stand", Some(10.)), ("idle_stand_rocktoes", Some(4.))],
                    "idle_stand",
                );
            }
            let time = (s.clock.time - 0.5).max(0.);
            let distance = s.clock.home.map_or(0., |p| {
                p.translation
                    .distance(self.data.points["alice_posx1"].translation)
            });
            if time < distance / self.data.speed("alice", "walk") {
                return ("walk", time, true);
            }
            return self.sequence(
                model,
                time,
                &[
                    ("idle", Some(6.)),
                    ("idle_base_01", None),
                    ("idle_base_01_2_base_02", None),
                    ("idle_base_02_2_shrug", None),
                    ("idle_shrug", Some(3.)),
                    ("idle_shrug_headtilt", None),
                    ("idle_shrug", None),
                    ("idle_shrug_shakeno", None),
                    (
                        "idle_shrug",
                        Some(self.data.duration(model, "idle_shrug") + 7.),
                    ),
                    ("idle_shrug_nodyes", None),
                ],
                "idle_shrug",
            );
        }
        if model == "c_mockturtle" {
            if s.kind != Kind::Arrival || s.phase == Phase::Launch {
                return ("idle_base", s.clock.time, true);
            }
            if s.phase == Phase::Leave && !s.skipping && s.elapsed() >= 0.6 {
                return if s.elapsed() < 1.6 {
                    ("jump", s.elapsed() - 0.6, false)
                } else {
                    ("swim", s.elapsed() - 1.6, true)
                };
            }
            return self.sequence(
                model,
                s.clock.time - self.data.launch_end() - 0.6,
                &[
                    ("idle_nosewipe", None),
                    (
                        "idle_base",
                        Some(self.data.duration(model, "idle_base") + 1.),
                    ),
                    ("talk_shrug", None),
                    ("idle_nosewipe", None),
                    (
                        "idle_base",
                        Some(self.data.duration(model, "idle_base") + 4.),
                    ),
                    ("idle_horn", None),
                    ("idle_base", None),
                    ("talk_shrug", None),
                ],
                "idle_base",
            );
        }
        if s.kind != Kind::Rabbit {
            return ("i_calm_l", s.clock.time, true);
        }
        if s.phase == Phase::Leave && !s.skipping {
            return if s.elapsed() < self.data.rabbit_run() {
                ("run", s.elapsed(), true)
            } else {
                ("jump", s.elapsed() - self.data.rabbit_run(), false)
            };
        }
        self.sequence(
            model,
            s.clock.time - 0.5,
            &[
                ("i_calm_l", None),
                ("i_calm_eartwitch_l", None),
                ("i_calm_front_2_right", None),
                ("i_calm_right_l", None),
                ("i_calm_right_2_front", None),
                ("i_calm_2_ready", None),
                ("i_ready_l", None),
                ("i_ready_2_calm", None),
            ],
            "i_calm_l",
        )
    }
}
