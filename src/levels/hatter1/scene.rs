use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Kind {
    Hare,
    Port,
    Gryphon,
    Stop,
    Mirror,
    Hint,
}
impl Kind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Hare => "Hatter1_Cinema1",
            Self::Port => "Port_Cine",
            Self::Gryphon => "Hatter1_Cinema2",
            Self::Stop => "Hatter1_End",
            Self::Mirror => "mirror_cat",
            Self::Hint => "hatter_cat",
        }
    }
    pub fn dialogue(self) -> Option<&'static str> {
        (!matches!(self, Self::Port | Self::Stop)).then(|| self.id())
    }
    fn delay(self) -> f32 {
        match self {
            Self::Hare => 31.,
            Self::Gryphon => 5.5,
            Self::Mirror | Self::Hint => 2.,
            _ => 0.,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Scene {
    pub kind: Kind,
    pub time: f32,
    pub line: usize,
    pub line_time: f32,
    pub starts: [f32; 12],
    pub ending: Option<f32>,
    pub skip: Option<f32>,
    pub home: Option<Transform>,
}
impl Scene {
    pub fn validate(&self) -> Result<()> {
        state::clock("scene", self.time, 3600.)?;
        state::clock("speech", self.line_time, 180.)?;
        let count = match self.kind {
            Kind::Hare => 12,
            Kind::Gryphon => 7,
            _ => 1,
        };
        ensure!(self.line < count, "Invalid scene cursor");
        for t in self.starts {
            state::clock("speech start", t, self.time)?;
        }
        for t in [self.ending, self.skip].into_iter().flatten() {
            state::clock("scene tail", t, 30.)?;
        }
        ensure!(
            self.home.is_none_or(|p| p.translation.is_finite()
                && p.rotation.is_finite()
                && (p.rotation.length() - 1.).abs() < 0.01),
            "Invalid scene handoff"
        );
        Ok(())
    }
}
impl Clockwork {
    pub(super) fn begin(&mut self, kind: Kind) {
        if self.saved.scene.is_none() {
            self.saved.scene = Some(Scene {
                kind,
                time: 0.,
                line: 0,
                line_time: 0.,
                starts: [0.; 12],
                ending: None,
                skip: None,
                home: None,
            });
        }
    }
    pub(super) fn prepare_scene_story(&self, story: &mut Story) -> bool {
        let Some(s) = &self.saved.scene else {
            return true;
        };
        let Some(n) = s.kind.dialogue() else {
            return false;
        };
        if s.skip.is_some() || s.ending.is_some() || s.time < s.kind.delay() {
            return false;
        }
        story.resume_scene(n, s.line, s.line_time);
        true
    }
    pub(super) fn sync_scene_story(&mut self, story: &Story) {
        if let Some(s) = &mut self.saved.scene {
            if let Some((line, t)) = s.kind.dialogue().and_then(|n| story.progress(n)) {
                if line != s.line {
                    s.starts[line] = s.time;
                }
                s.line = line;
                s.line_time = t;
            }
        }
    }
    pub(super) fn advance_scene(&mut self, dt: f32, w: &World, p: &mut Player) -> Result<()> {
        let Some(s) = &mut self.saved.scene else {
            return Ok(());
        };
        s.home.get_or_insert(Transform {
            translation: p.feet,
            rotation: Quat::from_rotation_z(p.script_facing),
        });
        s.time = (s.time + dt).min(3600.);
        if let Some(t) = &mut s.ending {
            *t += dt;
        }
        if let Some(t) = &mut s.skip {
            *t = (*t + dt).min(0.5);
        }
        let complete = s.skip == Some(0.5)
            || match s.kind {
                Kind::Port => s.time >= 14.75,
                Kind::Stop => s.time >= 30.,
                Kind::Mirror => s.ending.is_some_and(|t| t >= 6.),
                Kind::Hint => s.ending.is_some_and(|t| t >= 3.),
                _ => s.ending.is_some_and(|t| t >= 0.5),
            };
        if !matches!(s.kind, Kind::Mirror | Kind::Hint) {
            p.cancel_climb();
            p.release_rope();
            p.velocity = Vec3::ZERO;
            p.script_motion = 1;
        }
        if complete {
            let s = self.saved.scene.clone().unwrap();
            let pose = match s.kind {
                Kind::Hare => Some(self.point("alice_watch_hare")),
                Kind::Gryphon => Some(self.point("alice_pos2")),
                Kind::Port | Kind::Stop => s.home,
                _ => None,
            };
            if let Some(mut at) = pose {
                at.translation = w
                    .actor_footing(
                        at.translation + Vec3::Z * 8.,
                        PLAYER_CENTER,
                        PLAYER_HALF,
                        128.,
                    )
                    .unwrap_or(at.translation);
                crate::cinematic::land_player(p, w, at)?;
            }
            match s.kind {
                Kind::Hare => {
                    self.saved.hare = Some(self.saved.age);
                }
                Kind::Port => {
                    self.saved.port = Some(self.saved.age);
                }
                Kind::Gryphon => {
                    self.saved.gryphon = Some(self.saved.age);
                }
                Kind::Stop => self.saved.stopped = true,
                Kind::Mirror => self.saved.hints[0] = true,
                Kind::Hint => self.saved.hints[1] = true,
            }
            self.saved.scene = None;
        }
        Ok(())
    }
    pub(super) fn scene_camera(&self) -> Option<Camera> {
        let s = self.saved.scene.as_ref()?;
        let (n, epoch) = match s.kind {
            Kind::Port => {
                if s.time < 6. {
                    ("hatter1_jdm1", 1.)
                } else if s.time < 12.15 {
                    ("hatter1_jdm2", 6.)
                } else {
                    ("hatter1_jdm3", 12.15)
                }
            }
            Kind::Stop => {
                if s.time < 12. {
                    ("hatter1_end1", 1.5)
                } else if s.time < 23.5 {
                    ("hatter1_end2", 12.)
                } else {
                    ("hatter1_end3", 23.5)
                }
            }
            Kind::Gryphon => match s.line {
                0..=2 => ("hatter1_path2", 1.5),
                3 => ("hatter1_path3", s.starts[3]),
                _ => ("hatter1_path4", s.starts[4]),
            },
            Kind::Hare => {
                if s.time < 16. {
                    ("hatter1_jpath1", 1.5)
                } else if s.time < 31. {
                    ("hatter1_jpath2", 16.)
                } else {
                    (
                        [
                            "hatter1_jpath3",
                            "hatter1_jpath3",
                            "hatter1_jpath4",
                            "hatter1_jpath10",
                            "hatter1_jpath11",
                            "hatter_jpath9",
                            "hatter1_jpath6",
                            "hatter1_jpath6",
                            "hatter1_jpath7",
                            "hatter1_jpath7",
                            "hatter1_jpath8",
                            "hatter1_jpath5",
                        ][s.line],
                        s.starts[match s.line {
                            1 => 0,
                            7 => 6,
                            9 => 8,
                            n => n,
                        }]
                        .max(31.),
                    )
                }
            }
            _ => return None,
        };
        let at = self.cameras[n].sample((s.time - epoch).max(0.), false);
        Some(Camera {
            eye: at.translation,
            target: at.translation + at.rotation * Vec3::X * 256.,
            up: at.rotation * Vec3::Z,
        })
    }
    pub(super) fn scene_fade(&self) -> Option<(Color, f32)> {
        let s = self.saved.scene.as_ref()?;
        if matches!(s.kind, Kind::Mirror | Kind::Hint) {
            return None;
        }
        let f = if let Some(t) = s.skip {
            (t / 0.5).min(1.)
        } else if let Some(t) = s.ending {
            (t / 0.5).min(1.)
        } else {
            let cuts: &[f32] = match s.kind {
                Kind::Stop => &[1.5, 12., 23.5, 30.],
                Kind::Port => &[1., 6., 13.65],
                _ => &[1.5],
            };
            cuts.iter()
                .map(|at| (1. - (s.time - at).abs()).max(0.))
                .fold(0., f32::max)
        };
        Some((WHITE, f))
    }
    pub(super) fn alice_pose(&self) -> Option<Transform> {
        let s = self.saved.scene.as_ref()?;
        if !self.scripted() {
            return None;
        }
        Some(match s.kind {
            Kind::Gryphon => self.point("alice_pos2"),
            Kind::Hare => {
                if s.time < 1.5 {
                    s.home?
                } else if s.time < 4. {
                    let a = self.point("alice_start_node");
                    let b = self.point("alice_watch_machine");
                    Transform {
                        translation: a.translation.lerp(b.translation, (s.time - 1.5) / 2.5),
                        rotation: b.rotation,
                    }
                } else if s.time < 16. {
                    self.point("alice_watch_machine")
                } else if s.time < 18. {
                    let a = self.point("alice_watch_machine");
                    let b = self.point("alice_watch_machine2");
                    Transform {
                        translation: a.translation.lerp(b.translation, (s.time - 16.) / 2.),
                        rotation: b.rotation,
                    }
                } else if s.line < 8 {
                    self.point("alice_watch_machine2")
                } else if s.line < 10 {
                    self.point("alice_watch_mouse")
                } else {
                    self.point("alice_watch_hare")
                }
            }
            _ => s.home?,
        })
    }
}
