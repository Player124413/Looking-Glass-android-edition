use super::*;
use crate::ant::Timing;
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Kind {
    Queen,
    King,
}
impl Kind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Queen => "cinema_queen_abduction_thread",
            Self::King => "cinema_king_thread",
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Phase {
    Approach,
    Talk,
    Pawn,
    Grab,
    Last,
    Done,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Scene {
    pub kind: Kind,
    pub time: f32,
    pub phase: Phase,
    pub start: f32,
    pub home: Transform,
    pub line: usize,
    pub line_time: f32,
    pub shot: usize,
    pub shot_start: f32,
    pub king_camera: Option<f32>,
    pub alice_camera: Option<f32>,
    pub alice_stop: Option<f32>,
    pub pawn_camera: Option<f32>,
}
impl Scene {
    pub fn elapsed(&self) -> f32 {
        self.time - self.start
    }
    pub fn phase(&mut self, p: Phase) {
        self.phase = p;
        self.start = self.time;
        self.line = 0;
        self.line_time = 0.;
    }
    pub fn dialogue(&self) -> Option<&'static str> {
        match self.phase {
            Phase::Talk => Some(TALK),
            Phase::Last => Some(LAST),
            _ => None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        state::clock("royal scene", self.time, 600.)?;
        state::clock("royal phase", self.start, self.time)?;
        state::clock("royal shot", self.shot_start, self.time)?;
        state::clock("royal line", self.line_time, 180.)?;
        ensure!(
            self.line < 9
                && self.shot < 10
                && self.home.translation.is_finite()
                && self.home.rotation.is_finite(),
            "Invalid royal scene cursor"
        );
        for t in [
            self.king_camera,
            self.alice_camera,
            self.alice_stop,
            self.pawn_camera,
        ]
        .into_iter()
        .flatten()
        {
            state::clock("royal camera", t, self.time)?;
        }
        ensure!(
            self.kind != Kind::Queen || self.phase == Phase::Approach,
            "Invalid abduction phase"
        );
        ensure!(
            self.phase != Phase::Last || self.line < 2,
            "Invalid departure line"
        );
        Ok(())
    }
}
impl Castle {
    pub(super) fn start_scene(&mut self, p: &Player) {
        let Some(kind) = self.saved.pending.take() else {
            return;
        };
        self.saved.scene = Some(Scene {
            kind,
            time: 0.,
            phase: Phase::Approach,
            start: 0.,
            home: Transform {
                translation: p.feet,
                rotation: Quat::from_rotation_z(p.script_facing),
            },
            line: 0,
            line_time: 0.,
            shot: 0,
            shot_start: 0.,
            king_camera: None,
            alice_camera: None,
            alice_stop: None,
            pawn_camera: None,
        });
        if kind == Kind::Queen {
            for n in ["r_knight_queen1", "r_knight_queen2"] {
                self.walk(n, &[&format!("{n}_dest1")], false, false);
            }
            for o in &self.objects {
                if o.name == "castle_frontdoor" {
                    self.saved.doors[o.door.unwrap()].open();
                }
            }
        }
    }
    fn approach_times(&self) -> (f32, f32) {
        let alice = self
            .data
            .point("alice_king_dest1")
            .translation
            .distance(self.data.point("alice_king_dest2").translation)
            / self.data.speed("alice", "walk").max(1.);
        let king = self
            .data
            .point("w_king")
            .translation
            .distance(self.data.point("w_king_dest1").translation)
            / self.data.speed("c_chess_king", "walk").max(1.);
        (1. + alice + 3., 1. + alice + 3. + 2. + king)
    }
    pub(super) fn step_scene(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
    ) -> Result<()> {
        self.start_scene(p);
        let approach = self.approach_times();
        let Some(s) = &mut self.saved.scene else {
            return Ok(());
        };
        let old = s.time;
        s.time += dt;
        p.velocity = Vec3::ZERO;
        p.cancel_climb();
        p.release_rope();
        if s.kind == Kind::Queen {
            if s.time >= 20.25 {
                self.finish_scene(map, w, p, false)?;
            }
            return Ok(());
        }
        let mut rooks = false;
        match s.phase {
            Phase::Approach => {
                rooks = old < approach.0 && s.time >= approach.0;
                if s.time >= approach.1 {
                    s.phase(Phase::Talk);
                    s.shot = 8;
                    s.shot_start = s.time;
                    s.king_camera = Some(s.time);
                }
            }
            Phase::Pawn => {
                if s.elapsed() >= self.data.pawn_duration + 2.45 {
                    s.phase(Phase::Grab);
                    s.shot = 6;
                    s.shot_start = s.time;
                }
            }
            Phase::Grab => {
                if s.elapsed() >= 2.6 {
                    s.phase(Phase::Last);
                    s.shot = 7;
                    s.shot_start = s.time - 1.1;
                }
            }
            Phase::Done => {
                self.finish_scene(map, w, p, false)?;
                return Ok(());
            }
            _ => {}
        }
        if rooks {
            for n in ["w_rook1", "w_rook2"] {
                self.walk(n, &[&format!("{n}_dest1")], false, false);
            }
        }
        if let Some((pose, _, _, _)) = self.alice_pose() {
            p.feet = pose.translation;
            p.script_facing = pose.rotation.to_euler(EulerRot::ZYX).0;
        }
        Ok(())
    }
    pub(super) fn finish_scene(
        &mut self,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        skip: bool,
    ) -> Result<()> {
        let Some(s) = self.saved.scene.take() else {
            return Ok(());
        };
        let landing = match s.kind {
            Kind::Queen => {
                self.saved.queen = true;
                self.saved.queen_release = Some(0.5);
                for n in ["r_knight_queen1", "r_knight_queen2"] {
                    if skip {
                        self.warp_actor(n, &format!("{n}_dest1"));
                    }
                    if let Some(a) = self.actor_mut(n) {
                        a.piece.script_wait = true;
                        a.walk = None;
                    }
                }

                if skip {
                    self.data.point("alice_queen_dest1")
                } else {
                    s.home
                }
            }
            Kind::King => {
                self.saved.king = true;
                self.saved.portal = Some(self.saved.age);
                for a in &mut self.saved.cast {
                    if a.name.starts_with("enemy_group") || a.name.starts_with("battle2_w_") {
                        a.piece.active = false;
                        a.piece.spawn_delay = None;
                    }
                }
                for n in ["w_rook1", "w_rook2"] {
                    self.warp_actor(n, &format!("{n}_dest1"));
                    let nodes: Vec<_> = (2..=10).map(|i| format!("{n}_dest{i}")).collect();
                    self.walk(
                        n,
                        &nodes.iter().map(String::as_str).collect::<Vec<_>>(),
                        false,
                        false,
                    );
                }
                self.data.point("alice_king_dest2")
            }
        };
        self.saved.fade = 0.5;
        self.rebuild(map)?;
        w.set_dynamic(self.colliders());
        crate::cinematic::land_player(p, w, landing)?;
        Ok(())
    }
    pub(super) fn sync_dialogue(&mut self, story: &Story) {
        let Some(s) = &mut self.saved.scene else {
            return;
        };
        let Some(id) = s.dialogue() else {
            return;
        };
        if let Some((line, time)) = story.progress(id) {
            let changed = s.line != line;
            s.line = line;
            s.line_time = time;
            if s.phase == Phase::Talk {
                if line >= 1 && s.alice_camera.is_none() {
                    s.alice_camera = Some(s.time);
                }
                if line >= 4 && s.alice_stop.is_none() {
                    s.alice_stop = Some(s.time);
                }
                if line == 8 && s.pawn_camera.is_none() {
                    s.pawn_camera = Some(s.time);
                }
                let shot = if line == 0 || (4..=6).contains(&line) {
                    8
                } else {
                    9
                };
                if changed && s.shot != shot {
                    s.shot = shot;
                    s.shot_start = s.time;
                }
            }
        }
    }
    pub(super) fn end_dialogue(&mut self, n: &str) {
        if let Some(s) = &mut self.saved.scene {
            if s.phase == Phase::Talk && n == TALK && s.line == 8 {
                s.phase(Phase::Pawn);
                s.shot = 5;
                s.shot_start = s.pawn_camera.unwrap_or(s.time);
            } else if s.phase == Phase::Last && n == LAST && s.line == 1 {
                s.phase(Phase::Done);
            }
        }
    }
    pub(super) fn queen_parent(&self) -> Transform {
        let mut p = self.data.point("w_queen_parent");
        let Some(s) = self.saved.scene.as_ref().filter(|s| s.kind == Kind::Queen) else {
            return p;
        };
        let mut t = (s.time - 1.).max(0.);
        let mut yaw = 0_f32;
        for (i, milliseconds) in [2750, 1000, 500, 500, 500, 500, 3500, 2000, 500, 2000]
            .into_iter()
            .enumerate()
        {
            let d = milliseconds as f32 / 1000.;
            let goal = self
                .data
                .point(&format!("w_queen_parent_way{}", i + 1))
                .translation;
            let turn = match i {
                2..=4 => -30_f32,
                5 => -26.5,
                8 => 26.5,
                _ => 0.,
            };
            let f = (t / d).clamp(0., 1.);
            p.translation = p.translation.lerp(goal, f);
            yaw += turn * f;
            if t < d {
                p.rotation = Quat::from_rotation_z(yaw.to_radians());
                return p;
            }
            t -= d;
        }
        p.translation.x -= 512. * ((t - 0.5) / 5.).clamp(0., 1.);
        p.rotation = Quat::from_rotation_z(yaw.to_radians());
        p
    }
    pub(super) fn king_pose(&self) -> (Transform, bool) {
        let from = self.data.point("w_king");
        let to = self.data.point("w_king_dest1");
        if self.saved.king {
            return (to, false);
        }
        let Some(s) = self.saved.scene.as_ref().filter(|s| s.kind == Kind::King) else {
            return (from, false);
        };
        let (start, end) = self.approach_times();
        let f = ((s.time - start - 2.) / (end - start - 2.)).clamp(0., 1.);
        let mut position = from.translation.lerp(to.translation, f);
        let k = f * 120.;
        let a = (k.floor() as usize).min(120);
        let b = (a + 1).min(120);
        position.z = self.data.king_floor[a]
            + (self.data.king_floor[b] - self.data.king_floor[a]) * k.fract();
        (
            Transform {
                translation: position,
                rotation: to.rotation,
            },
            f > 0. && f < 1.,
        )
    }
    pub(super) fn alice_pose(&self) -> Option<(Transform, &'static str, f32, bool)> {
        let s = self.saved.scene.as_ref()?;
        if s.kind == Kind::Queen {
            return Some((s.home, "idle_stand", s.time, true));
        }
        let from = self.data.point("alice_king_dest1");
        let to = self.data.point("alice_king_dest2");
        if s.phase == Phase::Approach {
            if s.time < 0.5 {
                return Some((s.home, "idle_stand", s.time, true));
            }
            let walk = from.translation.distance(to.translation)
                / self.data.speed("alice", "walk").max(1.);
            let f = ((s.time - 1.) / walk).clamp(0., 1.);
            return Some((
                Transform {
                    translation: from.translation.lerp(to.translation, f),
                    rotation: to.rotation,
                },
                if f > 0. && f < 1. {
                    "walk"
                } else {
                    "idle_stand"
                },
                (s.time - 1.).max(0.),
                true,
            ));
        }
        if s.phase == Phase::Grab {
            let t = s.elapsed();
            return Some((
                to,
                if t < 0.5 {
                    "idle_stand"
                } else if t < 1.5 {
                    "darkened_lookingglass"
                } else if t < 1.9 {
                    "changeweapon"
                } else {
                    "ready"
                },
                if t < 0.5 {
                    t
                } else if t < 1.5 {
                    t - 0.5
                } else if t < 1.9 {
                    t - 1.5
                } else {
                    t - 1.9
                },
                false,
            ));
        }
        Some((
            to,
            if s.phase == Phase::Last {
                "ready"
            } else {
                "idle_stand"
            },
            s.elapsed(),
            true,
        ))
    }
    pub(super) fn pawn_pose(&self) -> Option<(Transform, f32, &'static str, f32)> {
        let s = self.saved.scene.as_ref()?;
        if s.kind != Kind::King {
            return None;
        }
        if s.phase == Phase::Pawn {
            let t = s.elapsed();
            let p = self.data.pawn.sample(t, false);

            let shrink = (t - self.data.pawn_duration - 1.).max(0.);
            return Some((
                p,
                if t < self.data.pawn_duration + 1. {
                    0.7
                } else {
                    (1. - (shrink / 0.05).floor() * 0.1).clamp(0.2, 1.)
                },
                if shrink > 0.45 {
                    "walk"
                } else if shrink > 0. {
                    "pain1"
                } else {
                    "idle"
                },
                shrink,
            ));
        }
        if s.phase == Phase::Grab && s.elapsed() < 1.5 {
            let p = self.data.pawn.sample(self.data.pawn_duration, false);

            return Some((p, 0.2, "walk", s.elapsed()));
        }
        None
    }
    pub(super) fn scene_camera(&self) -> Option<crate::cinematic::Camera> {
        let s = self.saved.scene.as_ref()?;
        if s.kind == Kind::Queen {
            return Some(self.data.camera(
                if s.time < 12.25 {
                    "queen_camera1"
                } else {
                    "queen_camera2"
                },
                if s.time < 12.25 {
                    s.time
                } else {
                    s.time - 12.25
                },
            ));
        }
        let (approach, _) = self.approach_times();
        let (name, t) = match s.phase {
            Phase::Approach => {
                if s.time < approach {
                    ("king_camera1", (s.time - 0.5).max(0.))
                } else {
                    ("king_camera2", s.time - approach)
                }
            }
            Phase::Talk => {
                if s.shot == 8 {
                    (
                        "king_talk",
                        if s.line == 0 {
                            s.time - s.king_camera.unwrap_or(s.time)
                        } else {
                            s.alice_camera.unwrap_or(s.time) - s.king_camera.unwrap_or(s.time)
                        },
                    )
                } else {
                    (
                        "alice_talk",
                        if s.line <= 3 {
                            s.time - s.alice_camera.unwrap_or(s.time)
                        } else {
                            s.alice_stop.unwrap_or(s.time) - s.alice_camera.unwrap_or(s.time)
                        },
                    )
                }
            }
            Phase::Pawn => {
                let mut c = self.data.camera("king_camera4", s.time - s.shot_start);
                if let Some((p, _, _, _)) = self.pawn_pose() {
                    c.target = p.translation + Vec3::Z * 32.;
                }
                return Some(c);
            }
            Phase::Grab => {
                if s.elapsed() < 1.5 {
                    ("grab", s.elapsed())
                } else {
                    ("grab2", s.elapsed() - 1.5)
                }
            }
            _ => ("grab2", s.time - s.shot_start),
        };
        Some(self.data.camera(name, t))
    }
    pub(super) fn scene_fade(&self) -> Option<(Color, f32)> {
        let Some(s) = &self.saved.scene else {
            return (self.saved.fade > 0.).then_some((BLACK, self.saved.fade / 0.5));
        };
        let t = s.time;
        let a = if s.kind == Kind::Queen {
            if t < 0.2 {
                1. - t / 0.2
            } else if (6.75..12.25).contains(&t) {
                ((t - 6.75) / 3.5).min(1.)
            } else if (12.25..12.75).contains(&t) {
                1. - (t - 12.25) / 0.5
            } else {
                ((t - 15.25) / 4.).clamp(0., 1.)
            }
        } else if s.phase == Phase::Approach {
            if t < 0.5 {
                t / 0.5
            } else {
                (1. - (t - 0.5) / 0.5).max(0.)
            }
        } else if s.phase == Phase::Grab {
            let t = s.elapsed();
            if t < 1.5 {
                ((t - 0.5) / 1.).clamp(0., 1.)
            } else {
                (1. - (t - 1.5)).max(0.)
            }
        } else {
            0.
        };
        Some((BLACK, a))
    }
}
