use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Arrival,
    Jack,
    Gas,
    Tweedles,
    Hatter,
}
impl Kind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Arrival => "Funhouse_Start",
            Self::Jack => "Cat_Jack_Dialog",
            Self::Gas => "GasOpenCine",
            Self::Tweedles => "Funhouse_Tweedle_Cinema1",
            Self::Hatter => "Funhouse_Hatter_Cinema1",
        }
    }
    fn lead(self) -> f32 {
        match self {
            Self::Arrival => 28.1,
            Self::Jack => 2.,
            Self::Gas => 9.3,
            Self::Tweedles => 3.5,
            Self::Hatter => 2.,
        }
    }
    fn lines(self) -> usize {
        match self {
            Self::Arrival | Self::Jack => 1,
            Self::Gas => 0,
            Self::Tweedles => 9,
            Self::Hatter => 8,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Scene {
    pub kind: Kind,
    pub clock: SceneState,
    pub ending: Option<f32>,
    pub line_start: f32,
    pub prior_line: usize,
    pub second_shot: Option<f32>,
}
impl Scene {
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            clock: SceneState::new(&spec(kind, None)),
            ending: None,
            line_start: 0.,
            prior_line: 0,
            second_shot: None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.clock.validate(&spec(self.kind, None))?;
        ensure!(
            !self.clock.finished && self.clock.line < self.kind.lines().max(1),
            "Invalid Funhouse scene cursor"
        );
        state::clock("scene line start", self.line_start, self.clock.time)?;
        for t in [self.ending, self.second_shot].into_iter().flatten() {
            state::clock("scene cue", t, self.clock.time)?;
        }
        Ok(())
    }
}
fn spec(kind: Kind, landing: Option<Transform>) -> SceneSpec {
    SceneSpec {
        id: kind.id(),
        version: 1,
        duration: 3600.,
        shots: &[ShotSpec {
            start: 0.,
            track: "funhouse",
            offset: 0.,
            hold: 3600.,
        }],
        cues: &[],
        end: EndSpec {
            landing,
            exit: if kind == Kind::Hatter {
                Some(EXIT)
            } else {
                None
            },
        },
    }
}
pub struct Performance {
    pub model: &'static str,
    pub clip: &'static str,
    pub time: f32,
    pub looping: bool,
    pub pose: Transform,
    pub alpha: f32,
}
impl Funhouse {
    pub fn prepare_scene_story(&self, story: &mut Story) -> bool {
        let Some(s) = &self.saved.scene else {
            return true;
        };
        if s.clock.time < s.kind.lead() || s.ending.is_some() || s.kind == Kind::Gas {
            return false;
        }
        if s.kind == Kind::Hatter && s.clock.line == 1 && s.clock.time - s.line_start < 3.1 {
            return false;
        }
        story.resume_scene(s.kind.id(), s.clock.line, s.clock.line_time);
        true
    }
    pub fn sync_scene_story(&mut self, story: &Story) {
        if let Some(s) = &mut self.saved.scene {
            if let Some((line, time)) = story.progress(s.kind.id()) {
                if line != s.prior_line {
                    s.line_start = s.clock.time;
                    s.prior_line = line;
                    if (s.kind == Kind::Tweedles && line == 5)
                        || (s.kind == Kind::Hatter && (line == 1 || line == 2))
                    {
                        s.second_shot = Some(s.clock.time);
                    }
                }
                s.clock.line = line;
                s.clock.line_time = time;
            }
        }
    }
    pub fn advance_scene(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
    ) -> Result<()> {
        let Some(s) = &mut self.saved.scene else {
            return Ok(());
        };
        let line_time = s.clock.line_time;
        let mut runner = SceneRunner {
            spec: &spec(s.kind, None),
            state: &mut s.clock,
        };
        runner.capture(p);
        runner.advance(dt);
        s.clock.line_time = line_time;
        if s.kind != Kind::Jack {
            p.cancel_climb();
            p.release_rope();
            p.velocity = Vec3::ZERO;
            p.script_motion = 1;
        }
        let t = s.clock.time;
        let kind = s.kind;
        if kind == Kind::Arrival && t >= 12. {
            self.saved.cells |= 2;
        }
        if kind == Kind::Hatter && s.ending.is_some_and(|end| t - end >= 2.5) {
            self.saved.floor.get_or_insert(self.saved.time);
        }
        let complete = if kind == Kind::Gas {
            t >= 9.3
        } else {
            s.ending.is_some_and(|end| {
                t - end
                    >= match kind {
                        Kind::Arrival => 0.5,
                        Kind::Jack => 3.,
                        Kind::Tweedles => 0.5,
                        Kind::Hatter => 6.5,
                        Kind::Gas => 0.,
                    }
            })
        };
        if complete {
            self.finish_scene(map, w, p)?;
        }
        Ok(())
    }
    pub fn finish_scene(&mut self, map: &Bsp, w: &mut World, p: &mut Player) -> Result<()> {
        let Some(mut s) = self.saved.scene.take() else {
            return Ok(());
        };
        // Every path uses the same final transaction. A failed safe landing retains the scene.
        let before = self.saved.clone();
        let landing = match s.kind {
            Kind::Arrival => Some(self.data.points["funhouse_start1"]),
            Kind::Tweedles => Some(self.data.points["alice_pos1"]),
            _ => s.clock.home,
        };
        match s.kind {
            Kind::Arrival => {
                self.cell(1);
                self.saved.arrived = true;
                self.saved.stand.get_or_insert(self.saved.time);
            }
            Kind::Jack => self.saved.jack = true,
            Kind::Gas => {
                self.saved.gas = true;
            }
            Kind::Tweedles => {
                self.saved.fight = true;
                self.saved.essence = true;
                self.saved.essence_side = 1;
            }
            Kind::Hatter => {
                self.saved.minis.clear();
                self.saved.essence = false;
                self.saved
                    .floor
                    .get_or_insert((self.saved.time - 2.5).max(0.));
            }
        }
        self.rebuild(map)?;
        // Keep all unrelated static/shared dynamic colliders supplied by the interaction owner.
        let result = if s.kind == Kind::Jack {
            Ok(true)
        } else {
            SceneRunner {
                spec: &spec(s.kind, landing),
                state: &mut s.clock,
            }
            .finish(w, p)
        };
        if let Err(e) = result {
            self.saved = before;
            self.saved.scene = Some(s);
            self.rebuild(map)?;
            return Err(e);
        }
        if s.kind == Kind::Hatter {
            self.saved.exit.committed = true;
            self.saved.fight = false;
        }
        p.script_motion = 0;
        Ok(())
    }
    pub fn scene_camera(&self) -> Option<crate::cinematic::Camera> {
        let s = self.saved.scene.as_ref()?;
        let t = s.clock.time;
        let (name, time) = match s.kind {
            Kind::Jack => return None,
            Kind::Gas => ("funhouse_pipe1", t),
            Kind::Arrival => {
                if t < 5.5 {
                    ("funhouse_path3", t)
                } else if t < 8. {
                    ("funhouse_jpath2", t - 5.5)
                } else if t < 13.5 {
                    ("funhouse_cell2", t - 8.)
                } else if t < 16.6 {
                    ("funhouse_jpath1", t - 13.5)
                } else {
                    ("funhouse_path4", t - 16.6)
                }
            }
            Kind::Tweedles => {
                if s.clock.line < 5 {
                    ("funhouse_path1", (t - 2.5).max(0.))
                } else {
                    ("funhouse_path2", t - s.second_shot.unwrap_or(t))
                }
            }
            Kind::Hatter => {
                if s.ending.is_some_and(|end| t - end >= 2.) {
                    ("funhouse_end1", t - s.ending.unwrap() - 2.)
                } else if s.clock.line == 0 {
                    ("funhouse_path5", (t - 2.).max(0.))
                } else if s.clock.line == 1 {
                    ("funhouse_awatch1", t - s.second_shot.unwrap_or(t))
                } else {
                    ("funhouse_path6", (t - s.second_shot.unwrap_or(t)).max(0.))
                }
            }
        };
        Some(self.data.cameras[name].camera(time))
    }
    pub fn scene_fov(&self, aspect: f32) -> Option<f32> {
        let s = self.saved.scene.as_ref()?;
        if s.kind != Kind::Hatter || s.clock.line != 1 {
            return None;
        }
        let t = s.clock.time - s.second_shot.unwrap_or(s.clock.time);
        let hfov = 50. + 80. * ((t - 0.5) / 2.3).clamp(0., 1.);
        Some(2. * ((hfov.to_radians() * 0.5).tan() / aspect.max(0.1)).atan())
    }
    pub fn performances(&self) -> Vec<Performance> {
        let Some(s) = &self.saved.scene else {
            return vec![];
        };
        let t = s.clock.time;
        let mut out = vec![];
        let mut add = |model, clip, time, looping, pose, alpha| {
            out.push(Performance {
                model,
                clip,
                time,
                looping,
                pose,
                alpha,
            })
        };
        let pt = |n: &str| self.data.points[n];
        match s.kind {
            Kind::Arrival => {
                add("alice", "idle_stand", t, true, pt("funhouse_start1"), 1.);
                add(
                    "c_tweedle_dum",
                    if t < 14.5 { "idle" } else { "alert02" },
                    if t < 14.5 { t } else { t - 14.5 },
                    t < 14.5,
                    pt("fake_dum"),
                    1.,
                );
                if t >= 26.1 {
                    add(
                        "c_cheshire",
                        if t < 29.1 { "sit_idle1" } else { "sit_talk3" },
                        t - 26.1,
                        true,
                        pt("cat_pos1"),
                        ((t - 26.1) / 2.).min(1.),
                    );
                }
            }
            Kind::Jack => add(
                "c_cheshire",
                "sit_talk1",
                t,
                true,
                pt("jack_cat"),
                (t / 2.).min(1.)
                    * s.ending
                        .map_or(1., |e| (1. - (t - e - 1.) / 2.).clamp(0., 1.)),
            ),
            Kind::Gas => (),
            Kind::Tweedles => {
                let at = (t - 2.5).max(0.);
                add(
                    "alice",
                    if at < 6. {
                        "idle_base_01"
                    } else if at < 7. {
                        "idle_base_01_2_base_02"
                    } else {
                        "idle_base_02"
                    },
                    if at < 6. { at } else { at - 6. },
                    true,
                    pt("alice_pos1"),
                    1.,
                );
                for (model, marker, clip, start) in [
                    ("c_tweedle_dee", "dee_pos1", "twitch1", 6.),
                    ("c_tweedle_dum", "dum_pos1", "twitch2", 3.),
                ] {
                    let elapsed = at - start;
                    let duration = self.data.rigs[model].duration(clip);
                    add(
                        model,
                        if elapsed >= 0. && elapsed < duration {
                            clip
                        } else {
                            "idle"
                        },
                        elapsed.max(0.),
                        elapsed < 0. || elapsed >= duration,
                        pt(marker),
                        1.,
                    );
                }
                if let Some(start) = s.second_shot {
                    add(
                        "c_cheshire",
                        if s.clock.line == 6 || s.clock.line == 8 {
                            "sit_talk2"
                        } else {
                            "sit_idle1"
                        },
                        t - start,
                        true,
                        pt("cat_pos2"),
                        ((t - start) / 2.).min(1.),
                    );
                }
            }
            Kind::Hatter => {
                let falling = s.ending.is_some_and(|end| t - end >= 4.5);
                let mut p = pt("alice_pos2");
                if falling {
                    p.translation.z -= 256. * (t - s.ending.unwrap() - 4.5);
                }
                add(
                    "alice",
                    if falling {
                        "death_falling1"
                    } else {
                        "idle_stand"
                    },
                    if falling {
                        t - s.ending.unwrap() - 4.5
                    } else {
                        t
                    },
                    !falling,
                    p,
                    1.,
                );
                if t >= 1. {
                    add(
                        "c_madhatter",
                        "stand_base",
                        t - 1.,
                        true,
                        pt("hatter_actor1"),
                        1.,
                    );
                }
            }
        }
        out
    }
}
