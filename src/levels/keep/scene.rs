use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Kind {
    Arrival,
    Hint,
    Mirror,
    Heart,
    Death,
}
impl Kind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Arrival => "Start_Keep",
            Self::Hint => "Lever_Cat_Dialog",
            Self::Mirror => "Keep_Rotate_Mirror",
            Self::Heart => "Heart_Door_Open",
            Self::Death => "Keep_Cheshire_Dead",
        }
    }
    pub fn dialogue(self) -> Option<&'static str> {
        match self {
            Self::Arrival => Some("Start_Keep"),
            Self::Hint => Some("Lever_Cat_Dialog"),
            Self::Death => Some("Keep_Cheshire_Dialog"),
            _ => None,
        }
    }
    fn delay(self) -> f32 {
        match self {
            Self::Arrival => 5.,
            Self::Hint => 1.5,
            _ => 1.,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Scene {
    pub kind: Kind,
    pub time: f32,
    pub line: usize,
    pub line_time: f32,
    pub starts: [f32; 4],
    pub ending: Option<f32>,
    pub after_speech: Option<f32>,
    pub home: Option<Transform>,
    pub skip: Option<f32>,
}
impl Scene {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            time: 0.,
            line: 0,
            line_time: 0.,
            starts: [0.; 4],
            ending: None,
            after_speech: None,
            home: None,
            skip: None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        state::clock("scene", self.time, 3600.)?;
        state::clock("line", self.line_time, 180.)?;
        ensure!(
            self.line < if self.kind == Kind::Death { 4 } else { 1 },
            "Invalid Keep speech cursor"
        );
        for t in self.starts {
            state::clock("line start", t, self.time)?;
        }
        for t in [self.ending, self.after_speech, self.skip]
            .into_iter()
            .flatten()
        {
            state::clock("scene tail", t, 3600.)?;
        }
        ensure!(
            self.home.is_none_or(|p| p.translation.is_finite()
                && p.translation.abs().max_element() < 100000.
                && p.rotation.is_finite()
                && (p.rotation.length_squared() - 1.).abs() < 0.01),
            "Invalid Keep handoff"
        );
        Ok(())
    }
}
impl Keep {
    pub(super) fn begin(&mut self, kind: Kind) {
        if self.saved.scene.is_some() {
            return;
        }
        if (kind == Kind::Arrival && self.saved.arrival)
            || (kind == Kind::Hint && self.saved.hint)
            || (kind == Kind::Death && self.saved.death_done)
        {
            return;
        }
        self.saved.scene = Some(Scene::new(kind));
    }
    pub(super) fn strike_offset(&self) -> f32 {
        self.data.duration("c_cheshire", "sit_smile_open") * 2.
            + self.data.duration("c_cheshire", "sit_smile_shut") * 2.
            + self.data.duration("c_cheshire", "sit_talk1")
            + self.data.duration("c_cheshire", "sit_idle2")
            + 6.8
    }
    pub(super) fn strike_time(&self) -> Option<f32> {
        let s = self.saved.scene.as_ref()?;
        (s.kind == Kind::Death && s.line >= 2).then(|| s.time - s.starts[2] - self.strike_offset())
    }
    pub(super) fn prepare_scene_story(&self, story: &mut Story) -> bool {
        let Some(s) = &self.saved.scene else {
            return true;
        };
        let Some(id) = s.kind.dialogue() else {
            return false;
        };
        if s.skip.is_some() || s.time < s.kind.delay() || s.ending.is_some() {
            return false;
        }
        if s.kind == Kind::Death {
            story.line_limit = Some(if s.line < 2 {
                2
            } else if s.after_speech.is_none_or(|t| t < 9.) {
                2
            } else {
                3
            });
        }
        story.resume_scene(id, s.line, s.line_time);
        true
    }
    pub(super) fn sync_scene_story(&mut self, story: &Story) {
        if let Some(s) = &mut self.saved.scene {
            if let Some((line, t)) = s.kind.dialogue().and_then(|id| story.progress(id)) {
                if line != s.line {
                    s.starts[line] = s.time;
                }
                s.line = line;
                s.line_time = t;
                if s.kind == Kind::Death && line == 2 && story.line_finished() {
                    s.after_speech.get_or_insert(0.);
                }
            }
        }
    }
    pub(super) fn complete_dialogue(&mut self, n: &str) {
        if let Some(s) = &mut self.saved.scene {
            if s.kind.dialogue() == Some(n) && s.time >= s.kind.delay() && s.skip.is_none() {
                s.ending.get_or_insert(0.);
            }
        }
    }
    pub(super) fn advance_scene(&mut self, dt: f32, w: &World, p: &mut Player) -> Result<()> {
        let strike = self.strike_time();
        let offset = self.strike_offset();
        if let Some(t) = strike.filter(|t| *t >= 0.) {
            let at = self.data.at("cat_pace2");
            self.saved.head.update(
                dt * 0.7,
                t,
                at.translation,
                at.rotation.to_euler(EulerRot::ZYX).0,
                1.,
                w,
                Some(&self.data.sever),
            );
        }
        let Some(s) = &mut self.saved.scene else {
            return Ok(());
        };
        s.home.get_or_insert(Transform {
            translation: p.feet,
            rotation: Quat::from_rotation_z(p.script_facing),
        });
        // Slow only this scene's animations, cameras and scripted waits. Physics and global clocks keep their fixed step.
        let scene_dt = if strike.is_some_and(|t| t >= 0.) {
            dt * 0.7
        } else {
            dt
        };
        s.time = (s.time + scene_dt).min(3600.);
        if let Some(t) = &mut s.ending {
            *t += scene_dt;
        }
        if let Some(t) = &mut s.after_speech {
            *t += scene_dt;
        }
        if let Some(t) = &mut s.skip {
            *t = (*t + dt).min(0.5);
        }
        p.cancel_climb();
        p.release_rope();
        p.velocity = Vec3::ZERO;
        p.script_motion = 1;
        if s.kind == Kind::Death {
            if s.time >= 0.5 {
                self.saved.heart_open = true;
            }
            if s.line >= 2
                && s.time - s.starts[2]
                    >= offset
                        - 6.8
                        - self.data.duration("c_cheshire", "sit_smile_open")
                        - self.data.duration("c_cheshire", "sit_smile_shut")
            {
                self.saved.queen_open = true;
            }
        }
        let done = match s.kind {
            Kind::Mirror => self.saved.mirror_time == 5. && s.time >= 7.,
            Kind::Heart => s.time >= 6.,
            Kind::Arrival => s.ending.is_some_and(|t| t >= 1.2),
            Kind::Hint => s.ending.is_some_and(|t| t >= 1.5),
            Kind::Death => (s.line == 3 && s.line_time >= 2.) || s.skip == Some(0.5),
        };
        if done {
            self.finish_scene(w, p, false)?;
        }
        Ok(())
    }
    pub(super) fn finish_scene(&mut self, w: &World, p: &mut Player, skipped: bool) -> Result<()> {
        let Some(s) = &self.saved.scene else {
            return Ok(());
        };
        let kind = s.kind;
        let pose = match kind {
            Kind::Hint => Some(self.data.at("alice_levercat_pos")),
            Kind::Death => Some(self.data.at("alice_pos1")),
            Kind::Arrival => None,
            _ => s.home,
        };
        if let Some(pose) = pose {
            ensure!(
                w.body_clear(pose.translation),
                "Keep handoff blocked {:?}",
                pose.translation
            );
            p.feet = pose.translation;
            p.script_facing = (pose.rotation * Vec3::X)
                .y
                .atan2((pose.rotation * Vec3::X).x);
        }
        p.velocity = Vec3::ZERO;
        p.script_motion = 0;
        match kind {
            Kind::Arrival => self.saved.arrival = true,
            Kind::Hint => self.saved.hint = true,
            Kind::Mirror => {
                if skipped {
                    self.saved.mirror_time = 5.;
                }
            }
            Kind::Heart => self.saved.heart_open = true,
            Kind::Death => {
                self.saved.death_done = true;
                self.saved.queen_open = true;
                self.saved.heart_open = true;
                self.saved.exit.committed = true;
            }
        }
        self.saved.scene = None;
        Ok(())
    }
    pub(super) fn alice_pose(&self) -> Option<Transform> {
        let s = self.saved.scene.as_ref()?;
        Some(match s.kind {
            Kind::Hint => self.data.at("alice_levercat_pos"),
            Kind::Death => self.data.at("alice_pos1"),
            Kind::Arrival => {
                let mut p = s.home.unwrap_or(Transform {
                    translation: vec3(512., 288., -72.),
                    rotation: Quat::from_rotation_z(90_f32.to_radians()),
                });
                p.translation.z = -72. + 192. * self.saved.elapsed / 5.;
                p
            }
            _ => s.home?,
        })
    }
    pub(super) fn scene_camera(&self) -> Option<crate::cinematic::Camera> {
        let s = self.saved.scene.as_ref()?;
        let strike = self.strike_time();
        let (name, t) = match s.kind {
            Kind::Arrival => ("keep_introp1", s.time),
            Kind::Hint => ("keep_leverpx1", (s.time - 0.5).max(0.)),
            Kind::Mirror => ("keep_path2", s.time),
            Kind::Heart => ("keep_heartdoor1", (s.time - 0.5).max(0.)),
            Kind::Death => {
                if strike.is_some_and(|t| t >= 2.) {
                    ("keep_nop1", strike.unwrap() - 2.)
                } else {
                    ("keep_path1", (s.time - 0.5).max(0.))
                }
            }
        };
        Some(self.data.cameras[name].camera(t))
    }
    pub(super) fn scene_fade(&self) -> Option<(Color, f32)> {
        if let Some(s) = &self.saved.scene {
            if let Some(t) = s.skip {
                return Some((Color::new(0.5, 0., 0., 1.), t / 0.5));
            }
            return Some((
                if s.kind == Kind::Arrival {
                    BLACK
                } else {
                    WHITE
                },
                (1. - s.time / if s.kind == Kind::Arrival { 3. } else { 0.5 }).clamp(0., 1.),
            ));
        }
        if self.saved.lost.is_some() && self.saved.loss_time < 1.5 {
            let t = self.saved.loss_time;
            return Some((
                Color::new(0.5, 0., 0., 1.),
                if t < 0.5 {
                    t * 2.
                } else {
                    (1.5 - t).min(0.5) * 2.
                },
            ));
        }
        None
    }
    pub(super) fn cat(&self) -> Option<(Transform, &'static str, f32, bool, f32)> {
        let s = self.saved.scene.as_ref()?;
        let (at, steps, elapsed): (_, Vec<(&'static str, Option<f32>)>, _) = match s.kind {
            Kind::Arrival => (
                self.data.at("lift_cat"),
                vec![
                    ("sit_idle1", Some(2.)),
                    ("sit_talk1", None),
                    ("sit_talk3", None),
                ],
                s.time - 3.,
            ),
            Kind::Hint => (self.data.at("lever_cat_pos1"), vec![], s.time - 0.5),
            Kind::Death => match s.line {
                0 => {
                    let mut p = self.data.at("cat_spawn1");
                    p.translation = p.translation.lerp(
                        self.data.at("cat_pos1").translation,
                        ((s.time - 1.) / 1.6).clamp(0., 1.),
                    );
                    (
                        p,
                        vec![
                            ("walk", Some(1.6)),
                            ("sit_talk3", None),
                            ("sit_talk1", None),
                        ],
                        s.time - 1.,
                    )
                }
                1 => (
                    self.data.at("cat_pace1"),
                    vec![
                        ("sit_idle2", None),
                        ("sit_talk3", None),
                        ("sit_talk1", None),
                        ("sit_idle2", None),
                    ],
                    s.time - s.starts[1],
                ),
                _ => {
                    if let Some(t) = self.strike_time().filter(|t| *t >= 0.) {
                        return Some((
                            self.data.at("cat_pace2"),
                            if t >= self.data.duration("c_cheshire", "death") {
                                "death_idle"
                            } else {
                                "death"
                            },
                            if t >= self.data.duration("c_cheshire", "death") {
                                t - self.data.duration("c_cheshire", "death")
                            } else {
                                t
                            },
                            t >= self.data.duration("c_cheshire", "death"),
                            1.,
                        ));
                    }
                    (
                        self.data.at("cat_pace2"),
                        vec![
                            ("sit_smile_open", None),
                            ("sit_smile_shut", None),
                            ("sit_talk1", None),
                            ("sit_idle2", None),
                            ("sit_smile_open", None),
                            ("sit_smile_shut", None),
                        ],
                        s.time - s.starts[2],
                    )
                }
            },
            _ => return None,
        };
        let alpha = (elapsed / 2.).clamp(0., 1.) * s.ending.map_or(1., |t| (1. - t / 2.).max(0.));
        if alpha <= 0. {
            return None;
        }
        let (clip, t, looping) = self.sequence("c_cheshire", elapsed.max(0.), &steps, "sit_idle1");
        Some((at, clip, t, looping, alpha))
    }
    pub(super) fn alice_act(&self) -> (&'static str, f32, bool) {
        if let Some(t) = self.strike_time().filter(|t| *t >= 2.) {
            self.sequence(
                "alice",
                t - 2.,
                &[
                    ("idle_base_02", None),
                    ("idle_base_02", Some(2.)),
                    ("idle_base_02_kneel", None),
                    ("kneel_idle", None),
                    ("kneel_shakeno", Some(2.)),
                    ("kneel_2_weep", None),
                ],
                "weep_loop",
            )
        } else {
            ("idle_stand", self.saved.scene.as_ref().unwrap().time, true)
        }
    }
    fn sequence(
        &self,
        model: &str,
        mut t: f32,
        steps: &[(&'static str, Option<f32>)],
        last: &'static str,
    ) -> (&'static str, f32, bool) {
        for &(clip, hold) in steps {
            let d = hold.unwrap_or_else(|| self.data.duration(model, clip));
            if t < d {
                return (clip, t, hold.is_some());
            }
            t -= d;
        }
        (last, t, true)
    }
}
