use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Kind {
    Arrival,
    Staff,
    Caterpillar,
    Chess,
    Wall,
}
impl Kind {
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn returning(self) -> bool {
        self == Self::Wall
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::Arrival => "Cat1_Start",
            Self::Staff => "Alice_Gets_Eyestaff",
            Self::Caterpillar => "WForest_Cinema1",
            Self::Chess => "Cat_ChessTalk",
            Self::Wall => "Alice_Destroy_Wall",
        }
    }
    pub fn dialogue(self) -> &'static str {
        if self == Self::Arrival {
            "Cat1_Dialog"
        } else {
            self.id()
        }
    }
    fn lines(self) -> usize {
        match self {
            Self::Caterpillar => 5,
            Self::Chess => 3,
            _ => 1,
        }
    }
    fn delay(self) -> f32 {
        match self {
            Self::Caterpillar => 2.,
            Self::Chess => 0.5,
            Self::Wall => 3.5,
            _ => 1.5,
        }
    }
    fn tail(self) -> f32 {
        match self {
            Self::Staff => 9.,
            Self::Caterpillar => 4.7,
            Self::Wall => 2.,
            _ => 0.5,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Scene {
    pub kind: Kind,
    pub time: f32,
    pub line: usize,
    pub line_time: f32,
    pub shot_start: f32,
    pub ending: Option<f32>,
    pub home: Option<Transform>,
}
impl Scene {
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            time: 0.,
            line: 0,
            line_time: 0.,
            shot_start: 0.,
            ending: None,
            home: None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        state::clock("scene", self.time, 3600.)?;
        state::clock("speech", self.line_time, 180.)?;
        state::clock("shot start", self.shot_start, self.time)?;
        if let Some(t) = self.ending {
            state::clock("scene closing", t, 30.)?;
        }
        ensure!(
            self.line < self.kind.lines(),
            "Invalid WForest dialogue cursor"
        );
        ensure!(
            self.home.is_none_or(|p| p.translation.is_finite()
                && p.translation.abs().max_element() < 100000.
                && p.rotation.is_finite()
                && (p.rotation.length_squared() - 1.).abs() < 0.01),
            "Invalid WForest scene origin"
        );
        Ok(())
    }
}
impl Forest {
    pub(super) fn alice_pose(&self) -> Option<Transform> {
        let s = self.saved.scene.as_ref()?;
        Some(match s.kind {
            Kind::Arrival => s.home.unwrap_or(Transform {
                translation: vec3(4528., 460., 296.),
                rotation: Quat::from_rotation_z(45_f32.to_radians()),
            }),
            Kind::Staff => self.data.at("alice_eyestaff_pos1"),
            Kind::Caterpillar => self.data.at("alice_pos1"),
            Kind::Chess => self.data.at("alice_chess_pos"),
            Kind::Wall => self.data.at("alice_wall_posx1"),
        })
    }
    pub(super) fn prepare_scene_story(&self, story: &mut Story) -> bool {
        if let Some(s) = &self.saved.scene {
            if s.time >= s.kind.delay() && s.ending.is_none() {
                story.resume_scene(s.kind.dialogue(), s.line, s.line_time);
                return true;
            }
            return false;
        }
        if self.saved.blunder.is_some_and(|t| t >= 2.) && !self.saved.blunder_done {
            story.trigger("blunder_cat");
        }
        true
    }
    pub(super) fn sync_scene_story(&mut self, story: &Story) {
        if let Some(s) = &mut self.saved.scene {
            if let Some((line, time)) = story.progress(s.kind.dialogue()) {
                if s.line != line && s.kind == Kind::Caterpillar && matches!(line, 2 | 4) {
                    s.shot_start = s.time;
                }
                s.line = line;
                s.line_time = time;
            }
        }
    }
    pub(super) fn complete_scene_dialogue(&mut self, n: &str) {
        if n == "blunder_cat" && self.saved.blunder.is_some() && !self.saved.blunder_done {
            self.saved.blunder_done = true;
            self.saved.blunder = Some(0.);
        }
        if let Some(s) = &mut self.saved.scene {
            if n == s.kind.dialogue()
                && s.line + 1 == s.kind.lines()
                && s.time >= s.kind.delay()
                && s.ending.is_none()
            {
                s.ending = Some(0.);
                s.shot_start = s.time;
            }
        }
    }
    pub(super) fn advance_scene(&mut self, dt: f32, w: &World, p: &mut Player) -> Result<()> {
        if let Some(s) = &mut self.saved.scene {
            s.home.get_or_insert(Transform {
                translation: p.feet,
                rotation: Quat::from_rotation_z(p.script_facing),
            });
            s.time = (s.time + dt).min(3600.);
            if let Some(t) = &mut s.ending {
                *t = (*t + dt).min(30.);
            }
            p.cancel_climb();
            p.release_rope();
            p.velocity = Vec3::ZERO;
            p.script_motion = 1;
            let intro_length = 2.5
                + [
                    "sit_idle1",
                    "sit_talk1",
                    "sit_idle1",
                    "sit_talk2",
                    "sit_smile_open",
                ]
                .iter()
                .map(|clip| self.data.duration("c_cheshire", clip))
                .sum::<f32>();
            let ready = s.ending.is_some_and(|t| t >= s.kind.tail())
                && (s.kind != Kind::Arrival || s.time >= intro_length);
            let gate_ready = match s.kind {
                Kind::Staff => self.saved.cave == 4.,
                Kind::Caterpillar => self.saved.chess == 4.2,
                _ => true,
            };
            if ready && gate_ready {
                self.finish_scene(w, p, false)?;
            }
        }
        Ok(())
    }
    pub(super) fn finish_scene(&mut self, w: &World, p: &mut Player, skipped: bool) -> Result<()> {
        let Some(s) = &self.saved.scene else {
            return Ok(());
        };
        let kind = s.kind;
        let pose = self.alice_pose().unwrap();
        // Authored handoffs are grounded once at load and rechecked against live movers.
        ensure!(
            w.body_clear(pose.translation),
            "WForest scene handoff blocked: {:?} {:?}",
            kind,
            pose.translation
        );
        p.feet = pose.translation;
        p.velocity = Vec3::ZERO;
        p.script_motion = 0;
        let f = pose.rotation * Vec3::X;
        p.script_facing = f.y.atan2(f.x);
        p.grounded = true;
        self.saved.done[kind.index()] = true;
        if skipped {
            match kind {
                Kind::Staff => self.saved.cave = 4.,
                Kind::Caterpillar => self.saved.chess = 4.2,
                _ => {}
            }
        }
        self.saved.scene = None;
        Ok(())
    }
    pub(super) fn scene_camera(&self) -> Option<crate::cinematic::Camera> {
        let s = self.saved.scene.as_ref()?;
        let name = match s.kind {
            Kind::Arrival => "wforest_catp1",
            Kind::Staff => {
                if s.ending.is_some() {
                    "wforest_gatecam1"
                } else {
                    "wforest_gatecam1x"
                }
            }
            Kind::Caterpillar => {
                if s.ending.is_some() {
                    "wforest_path3"
                } else {
                    match s.line {
                        0 | 1 => "wforest_path1",
                        2 | 3 => "wforest_path2",
                        _ => "wforest_walice1",
                    }
                }
            }
            Kind::Chess => "wforest_catp2",
            Kind::Wall => "wforest_jdm2",
        };
        Some(self.data.cameras[name].camera((s.time - s.shot_start - 0.5).max(0.)))
    }
    pub(super) fn cat(&self) -> Option<(Transform, &'static str, f32, bool, f32)> {
        if let Some(s) = &self.saved.scene {
            let marker = match s.kind {
                Kind::Arrival => "cat_pos_start_first",
                Kind::Staff => "cat_eyestaff_pos1",
                Kind::Chess => "cat_chess_pos",
                _ => return None,
            };
            let alpha = ((s.time - 0.5) / 2.).clamp(0., 1.)
                * if s.kind == Kind::Staff {
                    s.ending.map_or(1., |t| (1. - t / 2.).max(0.))
                } else {
                    1.
                };
            let (clip, t, looping) = if s.kind == Kind::Arrival {
                self.sequence(
                    "c_cheshire",
                    (s.time - 1.5).max(0.),
                    &[
                        ("sit_idle1", None),
                        ("sit_talk1", None),
                        ("sit_idle1", None),
                        ("sit_talk2", None),
                        ("sit_smile_open", None),
                    ],
                    "sit_idle1",
                )
            } else if s.kind == Kind::Chess {
                self.sequence(
                    "c_cheshire",
                    (s.time - 0.5).max(0.),
                    &[
                        ("sit_idle1", Some(2.)),
                        ("sit_talk2", None),
                        ("sit_idle1", Some(7.5)),
                        ("sit_talk3", None),
                    ],
                    "sit_idle1",
                )
            } else {
                ("sit_idle1", s.time, true)
            };
            return (alpha > 0.).then_some((self.data.at(marker), clip, t, looping, alpha));
        }
        let t = self.saved.blunder?;
        let alpha = if self.saved.blunder_done {
            (1. - t / 2.).max(0.)
        } else {
            (t / 2.).min(1.)
        };
        (alpha > 0.).then_some((self.data.at("blunder_cat"), "sit_idle1", t, true, alpha))
    }
    pub(super) fn alice_act(&self) -> (&'static str, f32, bool) {
        let s = self.saved.scene.as_ref().unwrap();
        match s.kind {
            Kind::Caterpillar => self.sequence(
                "alice",
                (s.time - 2.).max(0.),
                &[("idle_shrug", None)],
                "idle_stand",
            ),
            Kind::Chess => self.sequence(
                "alice",
                (s.time - 0.5).max(0.),
                &[
                    ("idle_stand", Some(6.)),
                    ("idle_shrug_headtilt", None),
                    ("idle_stand", Some(4.)),
                    ("idle_stand_rocktoes", None),
                ],
                "idle_stand",
            ),
            _ => ("idle_stand", s.time, true),
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
    pub(super) fn humpty_act(&self) -> (&'static str, f32, bool) {
        let steps = [
            ("idle_hand", Some(8.)),
            ("idle_hand_ash", None),
            ("idle_hand", Some(4.)),
            ("idle_hand_poke", None),
            ("idle_hand_smokein", None),
            ("idle_hand_smokeout", None),
            ("idle_hand", Some(8.)),
            ("cigar_2_mouth", None),
            ("idle_mouth", Some(15.)),
            ("idle_mouth_strech", None),
            ("cigar_2_hand", None),
            ("idle_hand", Some(4.)),
            ("idle_hand_ash", None),
            ("idle_hand", Some(2.)),
        ];
        let duration: f32 = steps
            .iter()
            .map(|(c, t)| t.unwrap_or_else(|| self.data.duration("c_humptydumpty", c)))
            .sum();
        self.sequence(
            "c_humptydumpty",
            self.saved.clock % duration,
            &steps,
            "idle_hand",
        )
    }
}
