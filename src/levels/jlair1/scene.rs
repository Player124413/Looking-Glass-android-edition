use super::*;
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Kind {
    Arrival,
    Caterpillar,
}
impl Kind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Arrival => ARRIVAL,
            Self::Caterpillar => CATERPILLAR,
        }
    }
    fn delay(self) -> f32 {
        match self {
            Self::Arrival => 7.,
            Self::Caterpillar => 0.5,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Scene {
    pub kind: Kind,
    pub time: f32,
    pub line: usize,
    pub line_time: f32,
    pub starts: [f32; 15],
    pub reveal: Option<f32>,
    pub ending: Option<f32>,
    pub skip: Option<f32>,
}
impl Scene {
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            time: 0.,
            line: 0,
            line_time: 0.,
            starts: [0.; 15],
            reveal: None,
            ending: None,
            skip: None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        state::clock("scene", self.time, 3600.)?;
        state::clock("speech", self.line_time, 180.)?;
        ensure!(
            self.line < if self.kind == Kind::Arrival { 1 } else { 15 },
            "Invalid speech cursor"
        );
        for t in self.starts {
            state::clock("line start", t, self.time)?;
        }
        if let Some(t) = self.reveal {
            state::clock("reveal", t, self.time)?;
            ensure!(
                self.kind == Kind::Caterpillar && self.line >= 2,
                "Invalid reveal"
            );
        }
        for t in [self.ending, self.skip].into_iter().flatten() {
            state::clock("scene ending", t, 10.)?;
        }
        Ok(())
    }
}
impl Curiosity {
    pub(super) fn prepare_scene_story(&self, story: &mut Story) -> bool {
        let Some(s) = &self.saved.scene else {
            return true;
        };
        if s.skip.is_some() || s.ending.is_some() || s.time < s.kind.delay() {
            return false;
        }
        story.resume_scene(s.kind.id(), s.line, s.line_time);
        if s.kind == Kind::Caterpillar && s.reveal.is_none_or(|t| s.time < t + 3.2) {
            story.line_limit = Some(2);
        }
        true
    }
    pub(super) fn sync_scene_story(&mut self, story: &Story) {
        if let Some(s) = &mut self.saved.scene {
            if let Some((line, time)) = story.progress(s.kind.id()) {
                if line != s.line {
                    s.starts[line] = s.time;
                }
                s.line = line;
                s.line_time = time;
                if s.kind == Kind::Caterpillar && line == 2 && story.line_finished() {
                    s.reveal.get_or_insert(s.time);
                }
            }
        }
    }
    pub(super) fn advance_scene(&mut self, dt: f32, w: &World, p: &mut Player) -> Result<()> {
        let Some(s) = &mut self.saved.scene else {
            return Ok(());
        };
        s.time = (s.time + dt).min(3600.);
        if let Some(t) = &mut s.ending {
            *t += dt;
        }
        if let Some(t) = &mut s.skip {
            *t = (*t + dt).min(0.5);
        }
        p.cancel_climb();
        p.release_rope();
        p.velocity = Vec3::ZERO;
        p.script_motion = 1;
        if s.skip == Some(0.5)
            || s.ending
                .is_some_and(|t| t >= if s.kind == Kind::Arrival { 6. } else { 0.5 })
        {
            let kind = s.kind;
            let mut at = self.point(if kind == Kind::Arrival {
                "alice_pos1"
            } else {
                "alice_cater_pos1"
            });
            at.translation = w
                .actor_footing(
                    at.translation + Vec3::Z * 8.,
                    PLAYER_CENTER,
                    PLAYER_HALF,
                    128.,
                )
                .context("Scene handoff has no safe floor")?;
            crate::cinematic::land_player(p, w, at)?;
            if kind == Kind::Arrival {
                self.saved.arrived = true;
            } else {
                self.saved.caterpillar = true;
            }
            self.saved.scene = None;
        }
        Ok(())
    }
    pub(super) fn scene_camera(&self) -> Option<Camera> {
        let s = self.saved.scene.as_ref()?;
        let (path, epoch) = match s.kind {
            Kind::Arrival => {
                if let Some(t) = s.ending.filter(|t| *t >= 0.5) {
                    ("jlair1_path3", s.time - t + 0.5)
                } else if s.time < 7. {
                    ("jlair1_path1", 0.)
                } else {
                    ("jlair1_path2", 7.)
                }
            }
            Kind::Caterpillar => match s.line {
                0 => ("jlair1_path4", 0.5),
                1 | 2 if s.reveal.is_none() => ("jlair1_path5", s.starts[1]),
                2 | 3 => ("jlair1_jdm1", s.reveal.unwrap_or(s.starts[3]) + 0.2),
                4..=6 => ("jlair1_jdm2", s.starts[4]),
                7 => ("jlair1_jdm4", s.starts[7]),
                8..=11 => ("jlair1_jdm5", s.starts[8]),
                12 => ("jlair1_path6", s.starts[12]),
                _ => ("jlair1_jdm6", s.starts[13]),
            },
        };
        Some(self.paths[path].camera((s.time - epoch).max(0.)))
    }
    pub(super) fn fovy(&self, aspect: f32) -> Option<f32> {
        let s = self.saved.scene.as_ref()?;
        let horizontal: f32 =
            if s.kind == Kind::Caterpillar && (2..=3).contains(&s.line) && s.reveal.is_some() {
                20. + 100. * ((s.time - s.reveal.unwrap() - 0.2) / 3.).clamp(0., 1.)
            } else {
                90.
            };
        Some(2. * ((horizontal.to_radians() * 0.5).tan() / aspect.max(0.1)).atan())
    }
    pub(super) fn scene_fade(&self) -> Option<(Color, f32)> {
        let s = self.saved.scene.as_ref()?;
        if let Some(t) = s.skip {
            return Some((WHITE, (t / 0.5).min(1.)));
        }
        if s.kind == Kind::Arrival && s.time < 3. {
            return Some((BLACK, 1. - s.time / 3.));
        }
        let alpha = if let Some(t) = s.ending {
            if s.kind == Kind::Arrival {
                (1. - (t - 0.5).abs() / 0.5)
                    .max(0.)
                    .max(((t - 5.5) / 0.5).clamp(0., 1.))
            } else {
                (t / 0.5).min(1.)
            }
        } else if s.kind == Kind::Arrival {
            (1. - (s.time - 6.5).abs() / 0.5).max(0.)
        } else {
            (1. - (s.time - 0.5).abs() / 0.5).max(0.).max(
                s.reveal
                    .map_or(0., |t| (1. - (s.time - t - 0.2).abs() / 0.2).max(0.)),
            )
        };
        Some((WHITE, alpha))
    }
}
