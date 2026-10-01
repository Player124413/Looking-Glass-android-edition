use super::*;
use crate::{ant::Timing, cinematic::Camera};
impl Centipede {
    fn shove_start(&self) -> f32 {
        let arrival = 6.
            + self.data.points["centipede2_start1"]
                .translation
                .distance(self.data.points["alice_pos1"].translation)
                / self.data.speed("alice", "walk").max(1.);
        arrival.max(10.)
            + self.data.points["ant_guard2"]
                .translation
                .distance(self.data.points["ant_push_pos1"].translation)
                / self.data.speed("c_armyant", "walk").max(1.)
    }
    pub fn slide_start(&self) -> f32 {
        self.shove_start() + self.data.rigs["c_armyant"].duration("attack_2_start")
    }
    pub fn finish_intro(&mut self, w: &World, p: &mut Player) -> Result<()> {
        crate::cinematic::land_player(p, w, self.data.points["alice_slide_pos1"])?;
        self.phase(Phase::Slide);
        Ok(())
    }
    pub fn advance_scene(&mut self, map: &Bsp, w: &mut World, p: &mut Player) -> Result<()> {
        match self.saved.phase {
            Phase::Intro => {
                if self.saved.time >= self.slide_start() + 3.5 {
                    self.finish_intro(w, p)?;
                }
            }
            Phase::Fight => {
                if self.saved.battle.action == battle::Action::Dead {
                    crate::cinematic::land_player(p, w, self.data.points["alice_pos3"])?;
                    self.saved.battle.shots.clear();
                    self.saved.battle.larvae.clear();
                    self.phase(Phase::Drop);
                }
            }
            Phase::Drop => {
                if self.saved.time >= 7.7 {
                    self.phase(Phase::Climb);
                    self.rebuild(map)?;
                    // All falling tips are retired before the permanent climbing geometry appears.
                }
            }
            Phase::Eat => {
                if self.saved.time >= self.data.rigs["alice"].duration("eat_mushroom") + 1. {
                    self.phase(Phase::Grow);
                }
            }
            Phase::Grow => {
                if self.saved.time >= 5. {
                    self.phase(Phase::Done);
                    self.saved.exit.committed = true;
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub fn brush_poses(&self) -> Vec<(usize, Vec3, Quat)> {
        let p = self.saved.phase;
        let t = self.saved.time;
        let after = matches!(
            p,
            Phase::Climb | Phase::Greeting | Phase::Eat | Phase::Grow | Phase::Done
        );
        self.data
            .brushes
            .iter()
            .filter_map(|b| {
                let mut at = b.origin;
                if b.name == "no_back1" && p == Phase::Intro || b.name == "climbers" && !after {
                    return None;
                }
                if b.name.starts_with("spike") {
                    if after {
                        return None;
                    }
                    let index = b.name.get(5..7)?.parse::<usize>().ok()?.checked_sub(1)?;
                    let delay = [0., 0.4, 0.9, 1.5, 1.9, 2.2][index];
                    if p == Phase::Drop && t >= delay {
                        if !b.name.ends_with("tip") {
                            return None;
                        }
                        let (distance, duration) = match b.name.as_str() {
                            "spike01tip" => (1090., 3.),
                            "spike01atip" => (1050., 2.),
                            "spike03tip" => (1132., 2.2),
                            _ => (0., 1.),
                        };
                        at.z -= distance * ((t - delay) / duration).clamp(0., 1.);
                    }
                }
                if b.name.starts_with("teeth_") {
                    let amount = if p == Phase::Intro && t < self.slide_start() {
                        (t % 1. / 0.5).min(2. - t % 1. / 0.5)
                    } else {
                        1.
                    };
                    at.z += if b.name == "teeth_top" {
                        32. * amount
                    } else {
                        -48. * amount
                    };
                }
                Some((b.model, at, Quat::IDENTITY))
            })
            .collect()
    }
    pub fn alice_performance(&self) -> Option<(&'static str, f32, bool, Transform, f32)> {
        let t = self.saved.time;
        match self.saved.phase {
            Phase::Intro => {
                let mut p = self.data.points["centipede2_start1"];
                if t < 6. {
                    return Some(("idle_shrug", t, false, p, 1.));
                }
                let goal = self.data.points["alice_pos1"];
                let f = ((t - 6.) * self.data.speed("alice", "walk")
                    / p.translation.distance(goal.translation))
                .clamp(0., 1.);
                p.translation = p.translation.lerp(goal.translation, f);
                if t >= self.slide_start() {
                    let elapsed = t - self.slide_start();
                    p.translation = p.translation.lerp(
                        self.data.points["alice_slide_pos1"].translation,
                        (elapsed / 3.).clamp(0., 1.),
                    );
                    Some(("slide", elapsed, true, p, 1.))
                } else {
                    Some((
                        if f < 1. { "walk" } else { "idle_stand" },
                        t - 6.,
                        true,
                        p,
                        1.,
                    ))
                }
            }
            Phase::Drop => Some(("idle_stand", t, true, self.data.points["alice_pos3"], 1.)),
            Phase::Greeting => Some((
                if t < 3.5 {
                    "idle_stand"
                } else {
                    "idle_stand_rocktoes"
                },
                (t - 3.5).max(0.),
                false,
                self.data.points["alice_pos2"],
                1.,
            )),
            Phase::Eat => Some((
                "eat_mushroom",
                (t - 0.5).max(0.),
                false,
                self.data.points["alice_pos2"],
                1.,
            )),
            Phase::Grow | Phase::Done => Some((
                "ready",
                t,
                true,
                self.data.points["alice_pos2"],
                1. + 0.7
                    * if self.saved.phase == Phase::Done {
                        5.
                    } else {
                        t
                    },
            )),
            Phase::Fight
                if matches!(
                    self.saved.battle.action,
                    battle::Action::Strike | battle::Action::Shake
                ) =>
            {
                Some((
                    "ready",
                    self.saved.battle.time,
                    true,
                    Transform {
                        translation: self.saved.battle.victim_feet,
                        rotation: self.saved.battle.pose.rotation,
                    },
                    1.,
                ))
            }
            _ => None,
        }
    }
    pub fn ant_performance(
        &self,
        name: &str,
        base: Transform,
    ) -> (&'static str, f32, bool, Transform) {
        let t = if self.saved.phase == Phase::Intro {
            self.saved.time
        } else {
            self.slide_start() + 4.
        };
        if name == "ant_guard4" && t >= 10. {
            return (
                "attack_3_fire_idle",
                t - 10.,
                true,
                self.data.points["ant_shoot_pos1"],
            );
        }
        if name == "ant_guard2" && t >= 10. {
            let goal = self.data.points["ant_push_pos1"];
            let end = self.shove_start();
            let mut p = base;
            p.translation = p.translation.lerp(
                goal.translation,
                ((t - 10.) / (end - 10.).max(0.01)).clamp(0., 1.),
            );
            p.rotation = goal.rotation;
            return (
                if t < end {
                    "walk"
                } else if t < self.slide_start() {
                    "attack_2_start"
                } else {
                    "idle"
                },
                if t < end { t - 10. } else { t - end },
                t < end || t >= self.slide_start(),
                p,
            );
        }
        if t >= 6. && t < self.slide_start() && matches!(name, "ant_guard1" | "ant_guard3") {
            ("alert_1", t - 6., true, base)
        } else {
            ("idle", t, true, base)
        }
    }
    pub fn scene_camera(&self, w: &World) -> Option<Camera> {
        let t = self.saved.time;
        let (name, time) = match self.saved.phase {
            Phase::Intro => {
                if t < 6. {
                    ("centipede2_path1", t)
                } else if t < 10. {
                    ("centipede2_path2", t - 6.)
                } else if t < self.slide_start() {
                    ("centipede2_path3", t - 10.)
                } else {
                    ("centipede2_jump1a", t - self.slide_start())
                }
            }
            Phase::Drop => ("centipede2_path6", t),
            Phase::Greeting => ("centipede2_path4", (t - 0.5).max(0.)),
            Phase::Eat => ("centipede2_path5", t),
            Phase::Grow | Phase::Done => ("centipede2_path7", t),
            _ => {
                let b = &self.saved.battle;
                if matches!(b.action, battle::Action::Strike | battle::Action::Shake) {
                    let target = b.victim_feet + Vec3::Z * 32.;
                    let tr = w.sweep(target, b.grab_camera, Vec3::splat(4.));
                    return Some(Camera::look(
                        target.lerp(b.grab_camera, tr.fraction * 0.95),
                        target,
                    ));
                }
                return None;
            }
        };
        let p = self.data.cameras[name].sample(time, false);
        let mut camera = Camera::look(p.translation, p.translation + p.rotation * Vec3::X * 100.);
        if self.saved.phase == Phase::Intro && t >= self.slide_start() {
            if let Some((_, _, _, p, _)) = self.alice_performance() {
                camera.target = p.translation + Vec3::Z * 32.;
            }
        }
        if self.saved.phase == Phase::Drop || self.saved.phase == Phase::Grow {
            camera.eye += vec3((t * 51.).sin(), (t * 63.).sin(), (t * 73.).cos()) * 1.2;
        }
        Some(camera)
    }
    pub fn scene_fade(&self) -> Option<(Color, f32)> {
        let t = self.saved.time;
        let (color, alpha) = match self.saved.phase {
            Phase::Intro => (BLACK, (1. - t / 3.).clamp(0., 1.)),
            Phase::Drop => (WHITE, ((t - 7.2) / 0.5).clamp(0., 1.)),
            Phase::Climb => (WHITE, (1. - t / 0.5).clamp(0., 1.)),
            Phase::Greeting => (WHITE, (1. - (t - 0.5).abs() / 0.5).clamp(0., 1.)),
            Phase::Eat => {
                let end = self.data.rigs["alice"].duration("eat_mushroom") + 0.5;
                (WHITE, (1. - t / 0.5).max((t - end) / 0.5).clamp(0., 1.))
            }
            Phase::Grow => (WHITE, (1. - t / 0.5).clamp(0., 1.)),
            _ => return None,
        };
        (alpha > 0.).then_some((color, alpha))
    }
    pub fn boss_lights(&self) -> Vec<crate::lighting::Light> {
        let b = &self.saved.battle;
        if self.saved.phase == Phase::Fight
            && (b.action == battle::Action::Ready || b.weak(&self.data))
        {
            vec![crate::lighting::Light {
                position: b.tag(&self.data, "tag_target").translation,
                color: vec3(1., 0.2, 0.2),
                radius: 250.,
                only_models: false,
                flare: false,
            }]
        } else {
            vec![]
        }
    }
    pub fn scene_sounds(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        out: &mut Vec<crate::audio::world::Clock>,
    ) {
        use crate::audio::world::Clock;
        if self.saved.phase == Phase::Fight && self.saved.battle.action == battle::Action::Charge {
            loops.push(crate::audio::LoopCue {
                id: BASE + 500,
                path: "sound/character/centipede/attack_juggernaught.wav",
                clock: Some(self.saved.battle.time),
                origin: self.saved.battle.pose.translation,
            });
        }
        if self.saved.phase == Phase::Grow {
            out.push(Clock {
                key: "centipede2.grow",
                time: self.saved.time,
                period: None,
                origin: self.data.points["alice_pos2"].translation,
                cues: &[(0.1, "sound/ambience/special/alice_grow2.wav")],
            });
        }
        if self.saved.phase == Phase::Greeting {
            out.push(Clock {
                key: "centipede2.cat",
                time: self.saved.time,
                period: None,
                origin: self.data.points["cat_actor1"].translation,
                cues: &[(0.5, "sound/character/cheshire_cat/appear.wav")],
            });
        }
        if self.saved.phase == Phase::Intro {
            if self.saved.time < self.slide_start() {
                loops.push(crate::audio::LoopCue {
                    id: BASE + 501,
                    path: "sound/ambience/special/nutcracker.wav",
                    clock: Some(self.saved.time),
                    origin: self.data.points["teeth_bottom"].translation,
                });
            }
            out.push(Clock {
                key: "centipede2.slide",
                time: self.saved.time - self.slide_start(),
                period: None,
                origin: self.data.points["alice_pos1"].translation,
                cues: &[
                    (0., "sound/character/alice/pain4.wav"),
                    (0., "sound/character/alice/death_fall.wav"),
                ],
            });
        }
    }
}
