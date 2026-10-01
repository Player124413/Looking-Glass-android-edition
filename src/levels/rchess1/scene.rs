use super::*;
pub(super) struct Performance {
    pub name: String,
    pub clip: &'static str,
    pub time: f32,
    pub loops: bool,
    pub pose: Transform,
    pub scale: f32,
}
impl Performance {
    fn new(n: &str, c: &'static str, t: f32, p: Transform) -> Self {
        Self {
            name: n.into(),
            clip: c,
            time: t.max(0.),
            loops: true,
            pose: p,
            scale: 1.,
        }
    }
}
impl Encounter {
    pub(super) fn pawn_time(&self) -> f32 {
        1.1 + self.data.queener_time
            + self.data.points["alice_king_dest2"]
                .translation
                .distance(self.data.points["alice_king_dest3"].translation)
                / 90.
            + self.data.points["alice_king_dest3"]
                .translation
                .distance(self.data.points["alice_king_dest4"].translation)
                / 90.
            + 1.
    }
    pub(super) fn pieces_time(&self) -> f32 {
        self.pawn_time() + 6.6
    }
    pub(super) fn portal_time(&self) -> f32 {
        self.pieces_time() + 9.
    }
    pub(super) fn fall_time(&self) -> f32 {
        self.portal_time() + 7.1 + self.data.rigs["c_madhatter"].duration("attack_bitchslap")
    }
    pub(super) fn exit_duration(&self) -> f32 {
        self.fall_time() + 11.
    }
    fn killed(&self) -> bool {
        matches!(self.saved.phase, Phase::Killed | Phase::Done)
    }
    fn queener_pose(&self) -> Transform {
        if self.killed() {
            if self.saved.time < 1.1 + self.data.queener_time {
                self.data
                    .queener
                    .sample((self.saved.time - 1.1).max(0.), false)
            } else {
                self.data.points["t2"]
            }
        } else {
            self.data.points["queener_way1"]
        }
    }
    pub(super) fn brush_poses(&self) -> Vec<(usize, Vec3, Quat)> {
        let s = &self.saved;
        let bridge = if s.phase == Phase::Intro {
            ((s.time - 1.5) / 4.).clamp(0., 1.)
        } else if matches!(s.phase, Phase::Fight | Phase::Killed | Phase::Done) {
            1.
        } else {
            0.
        };
        self.data
            .brushes
            .iter()
            .map(|b| {
                let mut p = b.pose;
                match b.name.as_str() {
                    "king_bridge" => {
                        p.rotation *= Quat::from_rotation_y(-80_f32.to_radians() * bridge)
                    }
                    "king_bridge_chains" => {
                        let q = Quat::from_rotation_y(-80_f32.to_radians() * bridge);
                        let root = self.data.points["king_bridge"].translation;
                        p.translation = root + q * (p.translation - root);
                        p.rotation =
                            q * p.rotation * Quat::from_rotation_y(65_f32.to_radians() * bridge);
                    }
                    "king_clip" => {
                        if self.killed() {
                            p.translation.z -= 100000.;
                        } else if matches!(s.phase, Phase::Intro | Phase::Fight) {
                            p.translation = self.data.points["king_clip_way1"].translation;
                        }
                    }
                    "bridge_monster_clip" => {
                        if !self.killed() {
                            p.translation.z -= 100000.;
                        }
                    }
                    "exit_portal" => p.translation.z -= 100000.,
                    "queener" => p = self.queener_pose(),
                    "blade" => {
                        let t = if s.beheaded {
                            23.7
                        } else if s.phase == Phase::Beheading {
                            s.time
                        } else {
                            0.
                        };
                        p.translation.z += 48. * ((t - 6.7) / 3.).clamp(0., 1.)
                            - 96. * ((t - 14.4) / 0.5).clamp(0., 1.);
                    }
                    "spec_exit_door" => {
                        let f = if s.beheaded {
                            1.
                        } else if s.phase == Phase::Beheading {
                            ((s.time - 17.45) / 1.5).clamp(0., 1.)
                        } else {
                            0.
                        };
                        p.rotation *= Quat::from_rotation_z(
                            90_f32.to_radians()
                                * f
                                * if p.translation.x < 1792. { -1. } else { 1. },
                        );
                    }
                    "waterwheel" => {
                        p.translation = self.data.points["waterwheel_start"].translation;
                        p.rotation *= Quat::from_rotation_y(45_f32.to_radians() * s.clock);
                    }
                    _ => {}
                }
                (b.index, p.translation, p.rotation)
            })
            .collect()
    }
    pub(super) fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        let poses = self.brush_poses();
        if poses == self.poses {
            return Ok(());
        }
        let mut shapes = vec![];
        for (b, (_, p, q)) in self.data.brushes.iter().zip(&poses) {
            if p.z < -90000.
                || matches!(
                    b.name.as_str(),
                    "king_clip" | "bridge_monster_clip" | "blade" | "waterwheel"
                )
            {
                continue;
            }
            shapes.push(Collider::model(map, b.index, *p, *q, true)?);
        }
        self.poses = poses;
        self.shapes = shapes;
        Ok(())
    }
    pub(super) fn scene_camera(&self) -> Option<crate::cinematic::Camera> {
        let t = self.saved.time;
        let (n, time) = match self.saved.phase {
            Phase::Beheading => {
                if t < 5.7 {
                    ("queen_camera1", t - 0.5)
                } else if t < 10.4 {
                    ("queen_camera2", t - 5.7)
                } else if t < 14.4 {
                    ("queen_camera3", t - 10.4)
                } else {
                    ("queen_camera4", t - 14.4)
                }
            }
            Phase::Intro => {
                if t < 6. {
                    ("king_camera1", t - 0.5)
                } else {
                    ("king_camera2", t - 6.)
                }
            }
            Phase::Killed | Phase::Done => {
                if t >= self.fall_time() {
                    ("funhouse", t - self.fall_time())
                } else if t >= self.portal_time() {
                    ("portal", t - self.portal_time())
                } else if t >= self.pieces_time() + 4.5 {
                    ("kmpx2", t - self.pieces_time() - 4.5)
                } else if t >= self.pieces_time() - 2. {
                    ("kmpx1", t - self.pieces_time() + 2.)
                } else if t >= 1.1 + self.data.queener_time + 2. {
                    ("kmpx4", t - 1.1 - self.data.queener_time - 2.)
                } else {
                    ("kmpx3", t - 0.6)
                }
            }
            _ => return None,
        };
        Some(self.data.cameras[n].camera(time.max(0.)))
    }
    pub(super) fn scene_fade(&self) -> Option<(Color, f32)> {
        if !self.scripted() {
            return None;
        }
        let t = self.saved.time;
        if self.killed() {
            let f = self.fall_time();
            if (f - 1. ..f + 1.).contains(&t) {
                return Some((WHITE, if t < f { t - f + 1. } else { 1. - (t - f) }));
            }
        }
        Some((
            BLACK,
            if t < 0.5 {
                t / 0.5
            } else {
                (1. - (t - 0.5) / 0.5).max(0.)
            },
        ))
    }
    pub(super) fn performances(&self) -> Vec<Performance> {
        let s = &self.saved;
        let mut out = vec![];
        if matches!(s.phase, Phase::Intro | Phase::Fight) {
            let mut p = Performance::new(
                "r_king",
                s.boss.clip(),
                s.boss.sample_time(&self.data),
                s.boss.pose(),
            );
            p.loops = s.boss.loops();
            p.scale = if s.phase == Phase::Intro {
                ((s.time - 6.) / 1.).clamp(0., 1.)
            } else {
                s.boss.scale(&self.data)
            };
            if p.scale > 0. {
                out.push(p);
            }
        }
        for c in &self.data.cast {
            let n = c.name.as_str();
            let mut p = Performance::new(n, "idle", s.clock, c.pose);
            if n == "asylum_alice" {
                p.clip = "death_faint";
                p.time = 54. * self.data.rigs["alice"].frame("death_faint");
                p.loops = false;
            } else if n.starts_with("gallery") {
                if self.killed() {
                    continue;
                }
                p.clip = if c.model.contains("bishop") {
                    "idlea"
                } else {
                    "idle"
                };
            } else if matches!(n, "w_queen_head" | "w_queen_body") {
                let t = if s.beheaded {
                    23.7
                } else if s.phase == Phase::Beheading {
                    s.time
                } else {
                    0.
                };
                if t >= 14.7 {
                    p.clip = if n == "w_queen_head" {
                        "gib_head"
                    } else {
                        "death_silent"
                    };
                    p.time = (t - 14.7).min(if n == "w_queen_head" { 0.3 } else { 0.75 });
                    p.loops = false;
                } else {
                    p.clip = if (5.7..6.7).contains(&t) || (11.4..12.4).contains(&t) {
                        "idle_struggle02"
                    } else {
                        "idle_struggle01"
                    };
                    p.time = t;
                }
            } else if n == "r_king_beheader" {
                if s.beheaded || s.phase == Phase::Beheading && s.time >= 20.45 {
                    continue;
                }
                if s.phase == Phase::Beheading && (13.4..14.4).contains(&s.time) {
                    p.clip = "attack_1";
                    p.time = s.time - 13.4;
                    p.loops = false;
                }
            } else if n.starts_with("spec_") {
                if s.beheaded {
                    continue;
                }
                p.clip = if n.contains("bishop") {
                    "idlea"
                } else {
                    "idle"
                };
                if s.phase == Phase::Beheading {
                    let delay = if n.contains("knight") {
                        17.45
                    } else if n.contains("bishop") {
                        18.45
                    } else if n.contains("rook") {
                        19.45
                    } else {
                        20.45
                    };
                    let t = (s.time - delay).max(0.);
                    if t > 0. {
                        p.clip = if n.contains("pawn") { "walk" } else { "walk_1" };
                        let to = self.data.points["spec_dest1"].translation;
                        let dist = p.pose.translation.distance(to);
                        let f = (t * 250. / dist.max(1.)).min(1.);
                        p.pose.translation = p.pose.translation.lerp(to, f);
                        p.pose.rotation = Quat::from_rotation_z(
                            (to.y - c.pose.translation.y).atan2(to.x - c.pose.translation.x),
                        );
                        p.time = t;
                        if f == 1. {
                            continue;
                        }
                    }
                }
            } else if n == "kings_pawn" || n == "revived_queen" {
                if !self.killed() {
                    continue;
                }
                let t = s.time - self.pawn_time();
                p.pose.translation +=
                    self.queener_pose().translation - self.data.points["queener"].translation;
                if n == "kings_pawn" {
                    if !(0. ..3.).contains(&t) {
                        continue;
                    }
                    p.scale = if t < 1. {
                        t
                    } else if t < 2. {
                        1.
                    } else {
                        3. - t
                    };
                    p.clip = if t < 1. { "idle" } else { "pain1" };
                    p.time = (t - 1.).max(0.);
                } else {
                    if t < 3.1 {
                        continue;
                    }
                    p.scale = (t - 3.1).min(1.);
                    p.clip = "walk";
                    p.time = t - 3.1;
                    let walk = (s.time - self.pieces_time() - 3.).max(0.);
                    p.pose.translation.x -= if walk < 3. {
                        64. * walk
                    } else if walk < 6. {
                        192. + 128. * (walk - 3.) / 3.
                    } else {
                        320. + 64. * (walk - 6.).min(3.)
                    };
                    p.pose.translation.z -= 64. * ((walk - 3.) / 3.).clamp(0., 1.);
                }
            } else if n.starts_with("king_") {
                if !self.killed() {
                    continue;
                }
                let (start, marker) = match n {
                    "king_knight2" => (0., "king_knight2_dest1"),
                    "king_rook2" => (0.6, "king_rook2_dest1"),
                    "king_bishop2" => (1., "king_bishop2_destx1"),
                    "king_rook1" => (1.5, "king_rook1_dest1"),
                    "king_bishop1" => (2.2, "king_bishop1_destx1"),
                    _ => (2.7, "king_knight1_dest1"),
                };
                let t = s.time - self.pieces_time() - start;
                if t < 0. {
                    continue;
                }
                p.pose = self.data.points[marker];
                p.scale = t.min(1.);
                p.clip = if n.contains("bishop") {
                    "idlea"
                } else {
                    "idle"
                };
                p.time = t;
            } else if n == "portal_hatter" || n == "portal_alice" {
                if !self.killed() {
                    continue;
                }
                let t = s.time - self.portal_time();
                if t < 0. || n == "portal_hatter" && t < 1.8 {
                    continue;
                }
                if n == "portal_hatter" {
                    // The unbound Hatter uses his authored portal mark. Carrying the
                    // parked platform offset here leaves his strike suspended in space.
                    p.clip = if t < 3.2 {
                        "stand_2_ready"
                    } else if t < 3.2 + self.data.rigs["c_madhatter"].duration("attack_bitchslap") {
                        "attack_bitchslap"
                    } else {
                        "ready"
                    };
                    p.time = if t < 3.2 { t - 1.8 } else { t - 3.2 };
                    p.loops = t >= 3.2 + self.data.rigs["c_madhatter"].duration("attack_bitchslap");
                } else {
                    p.pose.translation +=
                        self.queener_pose().translation - self.data.points["queener"].translation;
                    p.clip = if t < 4. { "ready" } else { "death_faint" };
                    p.time = (t - 4.).max(0.);
                    p.loops = t < 4.;
                }
            } else {
                continue;
            }
            out.push(p);
        }
        if self.scripted() && (!self.killed() || s.time < self.portal_time()) {
            let mut p = Performance::new(
                "alice",
                "idle_stand",
                s.time,
                self.data.points["alice_king_dest1"],
            );
            match s.phase {
                Phase::Beheading => {
                    p.pose = self.data.points["beheading_node1"];
                    let f = (s.time / 3.).min(1.);
                    p.pose.translation = s.scene_from.translation.lerp(p.pose.translation, f);
                    if f < 1. {
                        p.clip = "walk";
                    }
                }
                Phase::Intro => {}
                _ => {
                    p.pose = self.data.points["alice_king_dest2"];
                    let t = (s.time - 1.1 - self.data.queener_time).max(0.);
                    let to = self.data.points["alice_king_dest3"].translation;
                    let duration = p.pose.translation.distance(to) / 90.;
                    if t < duration {
                        p.pose.translation = p.pose.translation.lerp(to, t / duration);
                        p.clip = "walk";
                    } else {
                        let end = self.data.points["alice_king_dest4"];
                        let f = ((t - duration) * 90. / to.distance(end.translation)).min(1.);
                        p.pose = end;
                        p.pose.translation = to.lerp(end.translation, f);
                        p.clip = if f < 1. { "walk" } else { "ready" };
                    }
                    p.time = t;
                }
            }
            out.push(p);
        }
        out
    }
}
