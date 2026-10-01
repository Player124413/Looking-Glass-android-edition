use super::*;
use crate::{
    ant::Timing,
    level::{
        scene::{SceneRunner, SceneState},
        spec::{EndSpec, SceneSpec, ShotSpec},
    },
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Kind {
    Intro,
    Bishop,
    Bell,
    Knight,
    Water,
}
impl Kind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Intro => "cinema_wchess1_intro_thread",
            Self::Bishop => "cinema_bishop_start_thread",
            Self::Bell => "cinema_ring_bell_thread",
            Self::Knight => "cinema_knight_start_thread",
            Self::Water => "cinema_raise_water_thread",
        }
    }
}
fn spec(k: Kind) -> SceneSpec {
    SceneSpec {
        id: k.id(),
        version: 1,
        duration: 600.,
        shots: &[ShotSpec {
            start: 0.,
            track: "wchess1",
            offset: 0.,
            hold: 600.,
        }],
        cues: &[],
        end: EndSpec {
            landing: None,
            exit: None,
        },
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Scene {
    pub kind: Kind,
    pub clock: SceneState,
    pub actor_from: Transform,
}
impl Scene {
    pub fn validate(&self) -> Result<()> {
        self.clock.validate(&spec(self.kind))?;
        ensure!(
            self.clock.home.is_some()
                && self.actor_from.translation.is_finite()
                && self.actor_from.rotation.is_finite(),
            "Invalid chess scene anchor"
        );
        Ok(())
    }
}
fn blend(a: Transform, b: Transform, t: f32) -> Transform {
    Transform {
        translation: a.translation.lerp(b.translation, t.clamp(0., 1.)),
        rotation: a.rotation.slerp(b.rotation, t.clamp(0., 1.)),
    }
}
impl Realm {
    fn duration(&self, model: &str, clip: &str, a: Transform, b: Transform) -> f32 {
        a.translation.distance(b.translation) / self.data.speed(model, clip).max(1.)
    }
    /// Actor legs and waits determine the shot boundaries; no dialogue text is embedded.
    pub(super) fn scene_times(&self, s: &Scene) -> [f32; 5] {
        let p = |n| self.data.point(n);
        match s.kind {
            Kind::Intro => {
                let a = 8.8
                    + self.duration(
                        "c_chess_red_rook",
                        "walk_1",
                        s.actor_from,
                        p("rook_guard1_cinema_dest1a"),
                    );
                let b = a
                    + 0.8
                    + self.duration(
                        "c_chess_red_rook",
                        "walk_1",
                        p("rook_guard1_cinema_dest1a"),
                        p("rook_guard1_cinema_dest1b"),
                    );
                [8.8, a, b, b + 1., b + 1.]
            }
            Kind::Bishop => {
                let a = 4.4
                    + self.duration(
                        "c_chess_red_bishop",
                        "walk_1",
                        s.actor_from,
                        p("bishop_instructor1_dest1"),
                    );
                let b = a + self.duration(
                    "alice",
                    "walk",
                    p("alice_bishop_puzzle_dest1"),
                    p("alice_bishop_puzzle_dest2"),
                );
                [4.4, a, b, b + 0.5, b + 3.5]
            }
            Kind::Knight => {
                let a = 1.
                    + self.duration(
                        "alice",
                        "walk",
                        s.clock.home.unwrap(),
                        p("alice_knight_puzzle_dest1"),
                    );
                let b = a
                    + 1.
                    + self.duration(
                        "c_chess_red_knight",
                        "walk_1",
                        s.actor_from,
                        p("knight_instructor1_dest1"),
                    );
                let c = b + self.duration(
                    "c_chess_red_knight",
                    "walk_1",
                    p("knight_instructor1_dest1"),
                    p("knight_instructor1_dest2"),
                );
                let d = c + self.duration(
                    "alice",
                    "walk",
                    p("alice_knight_puzzle_dest1"),
                    p("alice_knight_puzzle_dest2"),
                );
                [a, b, c, d + 0.5, d + 2.5]
            }
            Kind::Bell => {
                let a = 2.1
                    + self.duration(
                        "c_chess_red_rook",
                        "walk_1",
                        s.actor_from,
                        p("rook_guard1_dest1"),
                    );
                [2.1, a, a + 3., a + 3., a + 3.]
            }
            Kind::Water => [0.5, 0.7, 4.2, 4.7, 12.7],
        }
    }
    fn begin_scene(&mut self, k: Kind, p: &Player) {
        let n = match k {
            Kind::Intro | Kind::Bell => "rook_guard1",
            Kind::Bishop => "bishop_instructor1",
            Kind::Knight => "knight_instructor1",
            Kind::Water => "",
        };
        let actor_from = self
            .actor(n)
            .map(|a| Transform {
                translation: a.piece.feet,
                rotation: Quat::from_rotation_z(a.piece.yaw),
            })
            .unwrap_or(Transform {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            });
        let mut clock = SceneState::new(&spec(k));
        SceneRunner {
            spec: &spec(k),
            state: &mut clock,
        }
        .capture(p);
        self.saved.scene = Some(Scene {
            kind: k,
            clock,
            actor_from,
        });
    }
    pub(super) fn step_scenes(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
    ) -> Result<()> {
        if self.saved.bell.is_some_and(|t| self.saved.age - t >= 5.)
            && !self.saved.fired.contains("bell_scene")
            && self.saved.scene.is_none()
        {
            self.saved.pending = Some(Kind::Bell);
            self.fire("bell_scene");
        }
        if self.saved.scene.is_none() {
            if let Some(k) = self.saved.pending.take() {
                self.begin_scene(k, p);
            }
        }
        let Some(s) = &mut self.saved.scene else {
            return Ok(());
        };
        SceneRunner {
            spec: &spec(s.kind),
            state: &mut s.clock,
        }
        .advance(dt);
        let s = self.saved.scene.as_ref().unwrap().clone();
        let t = s.clock.time;
        if s.kind == Kind::Water && t >= 4.7 && self.saved.water.is_none() {
            self.saved.water = Some(self.saved.age - (t - 4.7));
        }
        self.place_scene_cast();
        p.velocity = Vec3::ZERO;
        p.cancel_climb();
        p.release_rope();
        if t >= self.scene_times(&s)[4] {
            self.finish_scene(map, w, p)?;
        }
        Ok(())
    }
    pub(super) fn alice_scene_pose(&self) -> Option<(Transform, f32, bool)> {
        let s = self.saved.scene.as_ref()?;
        let t = s.clock.time;
        let q = self.scene_times(s);
        let home = s.clock.home.unwrap();
        let (a, walking, shrink) = match s.kind {
            Kind::Bishop => {
                let a = self.data.point("alice_bishop_puzzle_dest1");
                let b = self.data.point("alice_bishop_puzzle_dest2");
                (
                    blend(a, b, (t - q[1]) / (q[2] - q[1])),
                    (q[1]..q[2]).contains(&t),
                    (1. - (t - q[3]) * 2.).clamp(0., 1.),
                )
            }
            Kind::Knight => {
                let a = self.data.point("alice_knight_puzzle_dest1");
                let b = self.data.point("alice_knight_puzzle_dest2");
                let end = q[3] - 0.5;
                let pose = if t < q[2] {
                    blend(home, a, (t - 1.) / (q[0] - 1.))
                } else {
                    blend(a, b, (t - q[2]) / (end - q[2]))
                };
                (
                    pose,
                    (1. ..q[0]).contains(&t) || (q[2]..end).contains(&t),
                    (1. - (t - q[3]) * 2.).clamp(0., 1.),
                )
            }
            _ => (home, false, 1.),
        };
        Some((a, shrink, walking))
    }
    fn place_scene_cast(&mut self) {
        let Some(s) = &self.saved.scene else {
            return;
        };
        let t = s.clock.time;
        let q = self.scene_times(s);
        let k = s.kind;
        let (name, pose) = match k {
            Kind::Intro => {
                let a = self.data.point("rook_guard1_cinema_dest1a");
                let b = self.data.point("rook_guard1_cinema_dest1b");
                (
                    "rook_guard1",
                    if t < q[1] + 0.8 {
                        blend(s.actor_from, a, (t - q[0]) / (q[1] - q[0]))
                    } else {
                        blend(a, b, (t - q[1] - 0.8) / (q[2] - q[1] - 0.8))
                    },
                )
            }
            Kind::Bishop => (
                "bishop_instructor1",
                blend(
                    s.actor_from,
                    self.data.point("bishop_instructor1_dest1"),
                    (t - q[0]) / (q[1] - q[0]),
                ),
            ),
            Kind::Knight => {
                let a = self.data.point("knight_instructor1_dest1");
                let b = self.data.point("knight_instructor1_dest2");
                (
                    "knight_instructor1",
                    if t < q[1] {
                        blend(s.actor_from, a, (t - q[0] - 1.) / (q[1] - q[0] - 1.))
                    } else {
                        blend(a, b, (t - q[1]) / (q[2] - q[1]))
                    },
                )
            }
            Kind::Bell => (
                "rook_guard1",
                blend(
                    s.actor_from,
                    self.data.point("rook_guard1_dest1"),
                    (t - q[0]) / (q[1] - q[0]),
                ),
            ),
            Kind::Water => return,
        };
        if let Some(a) = self.actor_mut(name) {
            a.piece.feet = pose.translation;
            a.piece.yaw = pose.rotation.to_euler(EulerRot::ZYX).0;
            a.piece.time = t;
        }
    }
    pub(super) fn quad_open(&self) -> f32 {
        if let Some(s) = &self.saved.scene {
            let t = s.clock.time;
            match s.kind {
                Kind::Intro => {
                    let q = self.scene_times(s);
                    return ((t - 8.8).clamp(0., 1.) - (t - q[2]).clamp(0., 1.)).clamp(0., 1.);
                }
                Kind::Bell => return (t - 1.1).clamp(0., 1.),
                _ => {}
            }
        }
        if self.saved.fired.contains("bell_complete") {
            1.
        } else {
            0.
        }
    }
    fn finish_scene(&mut self, map: &Bsp, w: &mut World, p: &mut Player) -> Result<()> {
        let Some(s) = self.saved.scene.take() else {
            return Ok(());
        };
        let home = s.clock.home.unwrap();
        let landing = match s.kind {
            Kind::Intro => {
                self.saved.intro = true;
                self.warp_actor("rook_guard1", "rook_guard1_cinema_dest1b");
                home
            }
            Kind::Bell => {
                self.fire("bell_complete");
                self.warp_actor("rook_guard1", "rook_guard1_dest1");
                self.walk("rook_guard1", &["rook_guard1_dest2"], false, false);
                home
            }
            Kind::Bishop | Kind::Knight => {
                let (piece, n, dest) = if s.kind == Kind::Bishop {
                    (
                        Piece::Bishop,
                        "bishop_instructor1",
                        "bishop_instructor1_dest1",
                    )
                } else {
                    (
                        Piece::Knight,
                        "knight_instructor1",
                        "knight_instructor1_dest2",
                    )
                };
                self.warp_actor(n, dest);
                let start = self
                    .data
                    .square(piece, if piece == Piece::Bishop { 1 } else { 9 })
                    + Vec3::Z * crate::collision::SKIN;
                // The source's final run to the board remains real movement with hazard contact.
                let mut from = self
                    .data
                    .point(&format!("alice_{}_puzzle_dest2", piece.name()));
                from.translation.z = start.z;
                self.saved.board = Some(Board {
                    piece,
                    node: 0,
                    moving: Some(Move {
                        node: 0,
                        leg: 0,
                        path: vec![start],
                        from: from.translation,
                        elapsed: 0.,
                        duration: from.translation.distance(start).max(1.) / 260.,
                    }),
                });
                from
            }
            Kind::Water => home,
        };
        self.rebuild(map)?;
        w.set_dynamic(self.colliders());
        // Board staging occurs inside the source's temporary control walls; normal handoffs
        // require an unobstructed saved home, and the caller restores all shared colliders.
        if self.saved.board.is_some() {
            p.feet = landing.translation;
            p.velocity = Vec3::ZERO;
            p.script_facing = landing.rotation.to_euler(EulerRot::ZYX).0;
            p.grounded = true;
        } else {
            crate::cinematic::land_player(p, w, landing)?;
        }
        Ok(())
    }
    pub(super) fn skip_scene(&mut self, map: &Bsp, w: &mut World, p: &mut Player) -> Result<bool> {
        if self.saved.scene.is_none() {
            if let Some(k) = self.saved.pending.take() {
                self.begin_scene(k, p);
            }
        }
        let Some(s) = &self.saved.scene else {
            return Ok(false);
        };
        if s.kind == Kind::Water && s.clock.time < 4.7 {
            return Ok(false);
        }
        let kind = s.kind;
        self.finish_scene(map, w, p)?;
        if kind == Kind::Bell {
            self.warp_actor("rook_guard1", "rook_guard1_dest2");
        }
        Ok(true)
    }
    pub(super) fn scene_camera(&self) -> Option<crate::cinematic::Camera> {
        let s = self.saved.scene.as_ref()?;
        let t = s.clock.time;
        let q = self.scene_times(s);
        let (track, start, watch) = match s.kind {
            Kind::Intro => {
                if t < 5.2 {
                    ("gate_camera1", 0., None)
                } else {
                    ("gate_camera2", 5.2, None)
                }
            }
            Kind::Bishop => {
                if t < 2.2 {
                    ("bishop_camera1a", 0.5, None)
                } else if t < 3.9 {
                    ("bishop_camera1b", 2.2, None)
                } else if t < q[1] {
                    (
                        "bishop_camera1c",
                        3.9,
                        self.actor("bishop_instructor1")
                            .map(|a| a.piece.feet + Vec3::Z * 54.),
                    )
                } else {
                    (
                        "bishop_camera2",
                        q[1],
                        self.alice_scene_pose()
                            .map(|a| a.0.translation + Vec3::Z * 48.),
                    )
                }
            }
            Kind::Knight => {
                if t < q[2] {
                    ("knight_camera1", 0.5, None)
                } else {
                    (
                        "knight_camera2",
                        q[2],
                        self.alice_scene_pose()
                            .map(|a| a.0.translation + Vec3::Z * 48.),
                    )
                }
            }
            Kind::Bell => ("quad_camera1", 0.5, None),
            Kind::Water => {
                if t < 4.2 {
                    ("water_camera1", 0.5, None)
                } else {
                    ("water_camera2", 4.2, None)
                }
            }
        };
        let mut c = self.data.camera(track, t - start);
        if let Some(v) = watch {
            c.target = v;
        }
        Some(c)
    }
    pub(super) fn scene_fade(&self) -> Option<(Color, f32)> {
        let s = self.saved.scene.as_ref()?;
        let t = s.clock.time;
        let starts: &[f32] = match s.kind {
            Kind::Intro => &[0., 5.2],
            Kind::Bishop => &[0.5, 2.2, 3.9],
            Kind::Water => &[0.5, 4.2],
            _ => &[0.5],
        };
        let value = starts
            .iter()
            .map(|v| (1. - (t - v).abs() * 2.).max(0.))
            .fold(0., f32::max);
        (value > 0.).then_some((BLACK, value))
    }
}
