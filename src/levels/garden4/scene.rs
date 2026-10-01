use super::*;
impl Garden {
    pub(super) fn last_acting(&self, time: f32) -> (&'static str, f32, bool) {
        self.sequence(
            "c_caterpillar",
            time,
            &[
                ("portal_smoke", None),
                ("idle_base", Some(5.)),
                ("idle_smoke", None),
                ("idle_base", Some(3.)),
                ("idle_adjust", None),
            ],
            "idle_base",
        )
    }
    pub(super) fn portal_visible(&self) -> bool {
        self.saved.complete
            || self
                .saved
                .scene
                .as_ref()
                .is_some_and(|s| s.phase == Phase::Reveal && s.elapsed() >= 5.)
    }
    pub(super) fn portal_pose(&self) -> Transform {
        let t = if self.saved.complete {
            1000.
        } else {
            self.saved
                .scene
                .as_ref()
                .filter(|s| s.phase == Phase::Reveal)
                .map_or(0., |s| (s.elapsed() - 5.).max(0.))
        };
        let mut p = self.data.portal.sample(t, false);
        if !self.portal_visible() {
            p.translation = self.data.portal_origin;
        }
        p.rotation = Quat::from_rotation_z(48_f32.to_radians());
        p
    }
    pub(super) fn drugview(&self) -> bool {
        self.saved
            .scene
            .as_ref()
            .is_some_and(|s| s.phase == Phase::Reveal && s.elapsed() >= 4.)
    }
    pub(super) fn scene_camera(&self) -> Option<crate::cinematic::Camera> {
        let s = self.saved.scene.as_ref()?;
        let mut c = self.data.cameras[s.shot].camera((s.clock.time - s.shot_start).max(0.));
        if self.drugview() {
            // Bounded scene-local roll; neither world time nor gameplay camera is changed.
            c.up = Quat::from_axis_angle(
                (c.target - c.eye).normalize_or_zero(),
                0.018 * (s.clock.time * 1.7).sin(),
            ) * c.up;
        }
        Some(c)
    }
    pub(super) fn scene_fade(&self) -> Option<(Color, f32)> {
        let Some(s) = &self.saved.scene else {
            return (self.saved.fade > 0.).then_some((WHITE, self.saved.fade / 0.5));
        };
        let t = s.elapsed();
        let a = match s.phase {
            Phase::Setup | Phase::SkipSetup => (t / 0.5).min(1.),
            Phase::Talk | Phase::LastLine => (1. - t / 0.5).clamp(0., 1.),
            Phase::Reveal if (5. ..5.5).contains(&t) => (t - 5.) / 0.5,
            Phase::Reveal if (5.5..6.).contains(&t) => 1. - (t - 5.5) / 0.5,
            Phase::Reveal if !s.skipping => ((t - 13.5) / 0.5).clamp(0., 1.),
            _ => 0.,
        };
        Some((WHITE, a))
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
    pub(super) fn acting_at(&self, model: &str, time: f32) -> (&'static str, f32, bool) {
        let Some(s) = &self.saved.scene else {
            return self
                .saved
                .outro
                .map_or(("idle_base", self.saved.idle, true), |t| {
                    self.last_acting(t)
                });
        };
        if model == "alice" {
            let second = s.alice_second.filter(|v| time >= *v);
            return if let Some(t) = second {
                self.sequence(
                    model,
                    time - t - 0.1,
                    &[
                        ("idle_stand", Some(3.)),
                        ("idle_stand_shakeno", None),
                        ("idle_stand", Some(12.)),
                        ("idle_stand_rocktoes", None),
                    ],
                    "idle_stand",
                )
            } else {
                self.sequence(
                    model,
                    time - 0.6,
                    &[("idle_stand", Some(2.)), ("idle_stand_shakeno", None)],
                    "idle_stand",
                )
            };
        }
        if s.phase == Phase::Reveal && time >= s.start {
            return self.sequence(
                model,
                time - s.start,
                &[
                    ("portal_smoke", None),
                    ("idle_base", Some(5.)),
                    ("idle_smoke", None),
                    ("idle_base", Some(3.)),
                    ("idle_adjust", None),
                ],
                "idle_base",
            );
        }
        if let Some(t) = s.cater_second.filter(|v| time >= *v) {
            return self.sequence(
                model,
                time - t,
                &[
                    ("talk04", None),
                    ("talk05", None),
                    ("talk06", None),
                    ("talk07", None),
                    ("talk01", None),
                    ("talk03", None),
                    ("talk02", None),
                    ("idle_base", Some(8.)),
                    ("idle_adjust", None),
                    ("idle_base", None),
                    ("talk04", None),
                    ("talk06", None),
                    ("talk02", None),
                    ("talk07", None),
                    ("idle_base", Some(4.)),
                    ("talk03", None),
                    ("talk01", None),
                ],
                "idle_base",
            );
        }
        self.sequence(
            model,
            time - 0.5,
            &[
                ("idle_base", Some(5.)),
                ("idle_smoke", None),
                ("idle_base", Some(3.)),
                ("talk06", None),
                ("idle_base", Some(8.)),
                ("talk04", None),
                ("talk01", None),
                ("idle_base", Some(8.)),
                ("idle_adjust", None),
                ("talk01", None),
                ("talk03", None),
            ],
            "idle_base",
        )
    }
}
