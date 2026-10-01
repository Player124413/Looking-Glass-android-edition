use super::*;
use crate::cinematic::Camera;
impl Queen {
    pub(super) fn actor_lights(&self) -> Vec<crate::lighting::Light> {
        let mut actors = Vec::new();
        if matches!(
            self.saved.phase,
            Phase::Corridor | Phase::Intro | Phase::Queen1 | Phase::Birth
        ) {
            let (clip, at, _, _) = self.queen1_performance();
            actors.push((
                "c_queen1".to_owned(),
                clip.to_owned(),
                at,
                self.queen1_pose(),
            ));
        }
        if matches!(
            self.saved.phase,
            Phase::Birth | Phase::Queen2 | Phase::Death
        ) {
            let (clip, at, pose) = self.body_performance();
            actors.push(("c_q2_body".into(), clip.into(), at, pose));
            let halo = self.data.rigs["c_q2_body"].tag("tag_halo", clip, at, pose, true);
            let (clip, at) = self.halo_performance();
            actors.push(("c_q2_halo".into(), clip.into(), at, halo));
            for n in 0..4 {
                if self.saved.parts[n] > 25. {
                    let (clip, at) = self.part_animation(n);
                    actors.push((
                        format!("c_q2_t0{}", n + 1),
                        clip.into(),
                        at,
                        self.part_pose(n),
                    ));
                }
            }
        }
        let mut out = Vec::new();
        for (name, clip, at, pose) in actors {
            let rig = &self.data.rigs[&name];
            let animation = &rig.clips[&clip];
            let visual =
                rig.events
                    .visual(&clip, at, animation.duration(), animation.frame_time, true);
            for (emitter, tag, enabled, light) in &rig.lights {
                if visual.emitters.get(emitter).copied().unwrap_or(*enabled) {
                    let p = tag
                        .as_ref()
                        .map_or(pose, |tag| rig.tag(tag, &clip, at, pose, true));
                    out.push(crate::lighting::Light {
                        position: p.point(light.position),
                        color: light.color
                            * if name == "c_q2_t02" && !self.data.head_strobe.is_empty() {
                                self.data.head_strobe[(self.saved.clock * 10.) as usize
                                    % self.data.head_strobe.len()]
                            } else {
                                Vec3::ONE
                            },
                        ..*light
                    });
                }
            }
        }
        out
    }
    pub(super) fn brush_poses(&self) -> Vec<(usize, Vec3, Quat)> {
        self.data
            .brushes
            .iter()
            .filter_map(|b| {
                if b.name == "queen_breast_hide"
                    && (matches!(
                        self.saved.phase,
                        Phase::Queen2 | Phase::Death | Phase::Ending | Phase::Done
                    ) || (self.saved.phase == Phase::Birth
                        && self.saved.time > self.data.pull_end()))
                {
                    return None;
                }
                let mut p = b.base;
                if b.name.starts_with("brokenfloor") || b.name == "queen_stairs" {
                    let n = b
                        .name
                        .trim_start_matches("brokenfloor")
                        .parse::<usize>()
                        .unwrap_or(0);
                    let (delay, duration, x, z) = if n == 0 {
                        (2.5, 9., -30., 35.)
                    } else {
                        let k = (n - 1) % 6;
                        (
                            if n > 6 {
                                [0.2, 0.3, 0.6, 0.8, 0.95, 1.25][k]
                            } else {
                                [0., 0.2, 0.3, 0.6, 0.8, 0.9][k]
                            },
                            if n > 6 {
                                [10., 10.6, 9.6, 10., 11.5, 10.2][k]
                            } else {
                                [10., 10.4, 10., 10., 9.5, 10.][k]
                            },
                            [-55., -55., 0., -45., 0., -55.][k],
                            [55., 0., 55., 45., 60., 55.][k],
                        )
                    };
                    let f = ((self.saved.collapsed - delay) / duration).clamp(0., 1.);
                    p.translation.z -= 2400. * f;
                    let tilt = if n == 0 {
                        ((self.saved.collapsed - 1.) / 0.5).clamp(0., 1.)
                    } else {
                        0.
                    };
                    p.rotation = Quat::from_rotation_z((z * f - 8. * tilt).to_radians())
                        * Quat::from_rotation_x((x * f - 7. * tilt).to_radians());
                }
                Some((b.model, p.translation, p.rotation))
            })
            .collect()
    }
    pub(super) fn advance_scene(&mut self, world: &World, p: &mut Player) -> Result<()> {
        match self.saved.phase {
            Phase::Intro => {
                if self.saved.time >= self.data.intro_break() && self.saved.collapsed == 0. {
                    self.saved.collapsed = 0.001;
                }
                if self.saved.time >= self.data.intro_end() && self.saved.dialogue_done {
                    self.finish_intro(world, p)?;
                }
            }
            Phase::Birth => {
                if self.saved.time >= self.data.pull_end() {
                    self.saved.powered = true;
                }
                if self.saved.time >= self.data.birth_end() && self.saved.dialogue_done {
                    self.finish_birth(world, p)?;
                }
            }
            Phase::Death => {
                self.death_explosions();
                let end = self.data.rigs["c_q2_body"].duration("death_start") + 8.75;
                if self.saved.time >= end {
                    self.final_burst();
                    self.start(Phase::Ending);
                }
            }
            Phase::Ending => {
                if self.saved.time >= 10. && self.saved.alive {
                    self.start(Phase::Done);
                }
            }
            _ => (),
        }
        if self.scripted() {
            p.velocity = Vec3::ZERO;
            p.script_motion = 1;
            p.cancel_climb();
            p.release_rope();
        }
        Ok(())
    }
    fn finish_intro(&mut self, world: &World, p: &mut Player) -> Result<()> {
        crate::cinematic::land_player(p, world, self.data.points["alice_start_pos1"])?;
        self.start(Phase::Queen1);
        self.saved.essence = 0;
        self.saved.essence_wait = 0.;
        Ok(())
    }
    fn finish_birth(&mut self, world: &World, p: &mut Player) -> Result<()> {
        crate::cinematic::land_player(p, world, self.data.points["alice_fight_queen"])?;
        self.start(Phase::Queen2);
        self.saved.powered = true;
        self.saved.checkpoint = true;
        self.saved.body = self.data.points["bitch_pos2"];
        self.saved.motion = 1;
        self.saved.motion_time = 0.;
        self.saved.position = 5;
        self.saved.middle = 17.;
        self.saved.essence = 0;
        self.saved.essence_wait = 0.;
        Ok(())
    }
    pub(super) fn skip_scene(
        &mut self,
        map: &Bsp,
        world: &mut World,
        p: &mut Player,
        story: &mut Story,
    ) -> Result<bool> {
        if !self.saved.alive {
            return Ok(false);
        }
        match self.saved.phase {
            Phase::Intro => {
                story.trigger(INTRO);
                story.finish_sequence(INTRO);
                self.saved.collapsed = 20.;
                self.rebuild(map)?;
                self.finish_intro(world, p)?;
            }
            Phase::Birth => {
                story.trigger(TALK);
                story.finish_sequence(TALK);
                self.finish_birth(world, p)?;
            }
            Phase::Death => {
                // A skip commits the final hide without replaying a fresh explosion.
                self.saved.death_burst = true;
                self.saved.debris.clear();
                self.start(Phase::Ending);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
    pub(super) fn scene_story(&self, story: &mut Story) -> bool {
        if self.saved.phase == Phase::Intro && self.saved.time >= 6. {
            story.trigger(INTRO);
            story.line_limit = if self.saved.time < 6. + self.data.intro_line {
                Some(0)
            } else {
                None
            };
        } else if self.saved.phase == Phase::Birth && self.saved.time >= self.data.speech_start() {
            story.trigger(TALK);
        }
        true
    }
    pub(super) fn scene_fade(&self) -> Option<(Color, f32)> {
        let cuts = match self.saved.phase {
            Phase::Intro => vec![
                (0.5, Color::new(0.5, 0., 0., 1.)),
                (4., Color::new(0.5, 0., 0., 1.)),
                (self.data.intro_end(), BLACK),
            ],
            Phase::Birth => vec![
                (0.5, Color::new(0.5, 0., 0., 1.)),
                (5.5, Color::new(0.5, 0., 0., 1.)),
                (9., Color::new(0.5, 0., 0., 1.)),
                (18.5, WHITE),
                (23.5, BLACK),
                (
                    self.data.speech_start() + self.data.speech + 0.5,
                    Color::new(0.5, 0., 0., 1.),
                ),
                (self.data.reveal(), BLACK),
                (self.data.birth_end(), BLACK),
            ],
            _ => return None,
        };
        cuts.into_iter().find_map(|(cut, color)| {
            let alpha = 1. - (self.saved.time - cut).abs() / 0.5;
            (alpha > 0.).then_some((color, alpha))
        })
    }
    pub(super) fn scene_camera(&self) -> Option<Camera> {
        let t = self.saved.time;
        let (name, start) = match self.saved.phase {
            Phase::Intro => {
                if t < 4. {
                    ("path1", 0.5)
                } else {
                    ("path2", 4.)
                }
            }
            Phase::Birth => {
                let speech = self.data.speech_start();
                let reveal = self.data.reveal();
                if t < 5.5 {
                    ("q1dead", 0.5)
                } else if t < 9. {
                    ("bodybounce", 5.5)
                } else if t < 18.5 {
                    ("q1deadpx1", 9.)
                } else if t < 23.5 {
                    ("getpower", 18.5)
                } else if t < self.data.pull_end() {
                    ("slitherp1", 23.5)
                } else if t < speech - 8.5 {
                    ("alicefallp1", self.data.pull_end())
                } else if t < speech + 16.5 {
                    ("bodytx1", speech - 8.5)
                } else if t < speech + 25.5 {
                    ("bodytx2", speech + 16.5)
                } else if t < speech + 31.5 {
                    ("bodytx3", speech + 25.5)
                } else if t < reveal - 8.7 {
                    ("bodyt2", speech + 31.5)
                } else if t < reveal {
                    ("bodytx4", reveal - 8.7)
                } else {
                    ("showq2", reveal)
                }
            }
            Phase::Death | Phase::Ending | Phase::Done => {
                return Some(Camera::look(
                    self.saved.alice.translation + Vec3::Z * 90.,
                    self.saved.body.translation + Vec3::Z * 1050.,
                ))
            }
            Phase::Queen1 if self.saved.grabbed && self.saved.grab_arrived => {
                let mut c = self.data.cameras["grab1"].sample(self.saved.grab_time);
                c.target = self.saved.alice.translation + Vec3::Z * 35.;
                return Some(c);
            }
            _ => return None,
        };
        let elapsed = if self.saved.phase == Phase::Intro && name == "path2" {
            let resume = 6. + self.data.intro_line + 2.5;
            if t < 8. {
                t - start
            } else if t < resume {
                4.
            } else {
                4. + t - resume
            }
        } else {
            t - start
        };
        Some(self.data.cameras[name].sample(elapsed.max(0.)))
    }
    pub(super) fn alice_performance(&self) -> (&str, f32, Transform) {
        let t = self.saved.time;
        if self.saved.phase == Phase::Intro {
            let mut p = self.data.points["alice_start_pos1"];
            p.translation = p.translation.lerp(
                self.data.points["alice_pos1"].translation,
                ((t - 0.5) / 3.).clamp(0., 1.),
            );
            return (if t < 3.5 { "walk" } else { "ready" }, t, p);
        }
        let mut p = self.data.points["alice_walkto_dq1"];
        if t < 18.5 {
            return ("idle_base_02", t, p);
        }
        if t < self.data.pull_end() {
            return ("use_recharger", t - 18.5, p);
        }
        let speech = self.data.speech_start();
        if t < speech - 8.5 {
            return ("pain_knockdown", (t - self.data.pull_end()) * 0.8, p);
        }
        p.translation = p.translation.lerp(
            self.data.points["alice_talk_queen_x1"].translation,
            ((t - speech) / 3.).clamp(0., 1.),
        );
        let time = t - speech;
        if t >= self.data.reveal() {
            return (
                "idle_base_02",
                t - self.data.reveal(),
                self.data.points["alice_fight_queen"],
            );
        }
        if time >= self.data.speech + 0.5 {
            let stand = self.data.rigs["alice"].duration("kneel_2_base_02");
            let elapsed = time - self.data.speech - 0.5;
            if elapsed < stand {
                return ("kneel_2_base_02", elapsed, p);
            }
            if elapsed >= stand + 6.7 {
                p.translation = p.translation.lerp(
                    self.data.points["alice_runto_queen2"].translation,
                    ((elapsed - stand - 6.7) / 2.).clamp(0., 1.),
                );
                return ("walk", elapsed - stand - 6.7, p);
            }
            return ("idle_base_02", elapsed - stand, p);
        }
        let d = |name| self.data.rigs["alice"].duration(name);
        let (clip, at) = timeline(
            time,
            &[
                ("walk", 3.),
                ("idle_base_02", 11. + d("idle_base_02")),
                ("idle_base_02_kneel", d("idle_base_02_kneel")),
                ("kneel_idle", d("kneel_idle")),
                ("kneel_shakeno", d("kneel_shakeno") + 8.),
                ("kneel_2_weep", d("kneel_2_weep")),
                ("weep_loop", 6.),
                ("weep_sobbing", 4.),
                ("weep_loop", 3.),
                ("weep_2_kneel", d("weep_2_kneel")),
                ("kneel_shakeno", d("kneel_shakeno")),
                ("kneel_idle", 1000.),
            ],
        );
        (clip, at, p)
    }
    pub(super) fn queen1_performance(&self) -> (&str, f32, &str, f32) {
        let s = &self.saved;
        let t = s.time;
        match s.phase {
            Phase::Corridor => ("idle_sit_throne", t, "insidethrone", t),
            Phase::Intro => {
                let at = 6. + self.data.intro_line;
                if t < at {
                    ("idle_sit_throne", t, "insidethrone", t)
                } else {
                    ("thronelift", t - at, "thronelift", t - at)
                }
            }
            Phase::Birth => {
                if t < 9. {
                    ("death", t, "death", t)
                } else if t < 23.5 {
                    ("death_idle", t - 9., "death_idle", t - 9.)
                } else {
                    ("death_pullthrough", t - 23.5, "death_pullthrough", t - 23.5)
                }
            }
            _ => {
                let (clip, t) = self.attack_animation("c_queen1");
                (
                    clip,
                    t,
                    if s.queen1 < 1100. {
                        "weakloop"
                    } else {
                        "fightloop"
                    },
                    s.clock,
                )
            }
        }
    }
    pub(super) fn queen1_pose(&self) -> Transform {
        let (_, _, clip, t) = self.queen1_performance();
        self.data.rigs["c_queen1_bigtent"].tag(
            "tag_queen",
            clip,
            t,
            self.data.points["throne"],
            self.saved.phase == Phase::Queen1,
        )
    }
    pub(super) fn body_performance(&self) -> (&str, f32, Transform) {
        let s = &self.saved;
        if s.phase == Phase::Birth {
            if s.time < self.data.reveal() {
                let d = |name| self.data.rigs["c_q2_body"].duration(name);
                let (clip, at) = if s.time >= self.data.speech_start() + self.data.speech {
                    (
                        "pullout",
                        s.time - self.data.speech_start() - self.data.speech,
                    )
                } else {
                    timeline(
                        (s.time - self.data.pull_end() - 0.1).max(0.),
                        &[
                            ("intro", d("intro")),
                            ("talk_base", d("talk_base") + 2.),
                            ("talk_negative", d("talk_negative")),
                            ("talk_base", d("talk_base")),
                            ("talk_blink-snarl", d("talk_blink-snarl")),
                            ("talk_base", 20.),
                            ("talk_in-yo-face", d("talk_in-yo-face")),
                            ("talk_base", 8.),
                            ("talk_in-yo-face", d("talk_in-yo-face")),
                            ("talk_blink-snarl", d("talk_blink-snarl")),
                            ("talk_negative", d("talk_negative")),
                            ("talk_in-yo-face", d("talk_in-yo-face")),
                            ("talk_base", 1000.),
                        ],
                    )
                };
                return (clip, at, self.data.points["bitch2_pos1"]);
            }
            return (
                "idle",
                s.time - self.data.reveal(),
                self.data.points["bitch_pos2"],
            );
        }
        if matches!(s.phase, Phase::Death | Phase::Ending | Phase::Done) {
            let d = self.data.rigs["c_q2_body"].duration("death_start");
            let (clip, t) = timeline(
                s.time,
                &[
                    ("death_start", d),
                    ("death_1slumploop", 1.),
                    ("death_2slumploop", 0.25),
                    ("death_3slumploop", 1.),
                    ("death_4slumploop", 0.5),
                    ("death_1endloop", 2.),
                    ("death_2endloop", 2.),
                    ("death_3endloop", 2.),
                    ("death_endloop", 1000.),
                ],
            );
            return (clip, t, s.body);
        }
        if s.motion != 0 {
            return ("move_front", s.motion_time, s.body);
        }
        let (clip, t) = self.attack_animation("c_q2_body");
        (clip, t, s.body)
    }
    pub(super) fn part_pose(&self, index: usize) -> Transform {
        let (clip, t, p) = self.body_performance();
        self.data.rigs["c_q2_body"].tag(&format!("tag_t0{}", index + 1), clip, t, p, true)
    }
    pub(super) fn halo_performance(&self) -> (&'static str, f32) {
        match self.saved.phase {
            Phase::Birth => ("strobe", self.saved.time),
            Phase::Death => ("death", self.saved.time),
            Phase::Queen2
                if self.saved.motion == 0 && self.saved.attack == battle::Attack::Scream =>
            {
                ("strobe", self.saved.attack_time)
            }
            Phase::Queen2
                if self.saved.motion == 0 && self.saved.attack != battle::Attack::Idle =>
            {
                ("stiff", self.saved.attack_time)
            }
            _ => ("idle", self.saved.clock),
        }
    }
}

fn timeline(mut t: f32, steps: &[(&'static str, f32)]) -> (&'static str, f32) {
    for &(clip, length) in steps {
        if t < length {
            return (clip, t);
        }
        t -= length;
    }
    (steps.last().unwrap().0, t)
}
