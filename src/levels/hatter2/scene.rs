use super::*;
pub(super) struct Performance {
    pub model: &'static str,
    pub clip: &'static str,
    pub time: f32,
    pub loops: bool,
    pub pose: Transform,
    pub scale: f32,
}
impl Performance {
    fn new(
        model: &'static str,
        clip: &'static str,
        time: f32,
        loops: bool,
        pose: Transform,
    ) -> Self {
        Self {
            model,
            clip,
            time: time.max(0.),
            loops,
            pose,
            scale: 1.,
        }
    }
}
pub(super) fn lift(t: f32) -> f32 {
    if t < 2. {
        t / 2.
    } else if t < 4. {
        1.
    } else if t < 6. {
        1. - (t - 4.) / 2.
    } else {
        0.
    }
}
impl Encounter {
    pub(super) fn brush_poses(&self) -> Vec<(usize, Vec3, Quat)> {
        let s = &self.saved;
        let cage = if self.scripted() {
            ((s.time - 0.5) / 5.).clamp(0., 1.)
        } else {
            0.
        };
        self.data
            .brushes
            .iter()
            .map(|b| {
                let mut p = b.pose;
                let n = b.name.as_str();
                if n == "drawbridge" {
                    p.rotation *=
                        Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 * s.bridge.unwrap_or(0.));
                }
                if n == "end_plat1" {
                    p.translation.z += 392. * lift(s.lifts);
                }
                if n == "end_plat2" {
                    p.translation.z -= 392. * lift(s.lifts);
                }
                if n == "testpend1" {
                    p.rotation *= Quat::from_rotation_y(
                        15_f32.to_radians()
                            * (s.clock * std::f32::consts::TAU / 16. + std::f32::consts::FRAC_PI_2)
                                .sin(),
                    );
                }
                if n == "clockmin" || n == "clockhr" {
                    p.rotation *= Quat::from_rotation_z(
                        -std::f32::consts::FRAC_PI_2 * s.cycle as f32
                            / 120.
                            / if n == "clockmin" { 2. } else { 24. },
                    );
                }
                if n.starts_with("clockdr_") {
                    let t = s.cycle as f32 / 120. - 50.;
                    let f = if (0. ..0.2).contains(&t) {
                        t / 0.2
                    } else if (0.2..0.7).contains(&t) {
                        1.
                    } else if (0.7..1.2).contains(&t) {
                        1. - (t - 0.7) / 0.5
                    } else {
                        0.
                    };
                    p.rotation *= Quat::from_rotation_z(
                        std::f32::consts::FRAC_PI_2 * f * if n.ends_with('1') { -1. } else { 1. },
                    );
                }
                if n == "end_doors1" {
                    p.rotation *= Quat::from_rotation_z(
                        std::f32::consts::FRAC_PI_2
                            * (s.door / 1.4)
                            * if b.pose.translation.x > 5664. {
                                -1.
                            } else {
                                1.
                            },
                    );
                }
                let parent = if n == "left_cage"
                    || matches!(
                        n,
                        "bar01"
                            | "bar02"
                            | "bar03"
                            | "knob01"
                            | "knob03"
                            | "chainstill01"
                            | "chainstill02"
                            | "chainmoving01"
                            | "chainmoving02"
                    ) {
                    Some("left_cage")
                } else if n == "right_cage"
                    || matches!(
                        n,
                        "bar04"
                            | "bar05"
                            | "bar06"
                            | "knob02"
                            | "knob04"
                            | "chainstill03"
                            | "chainstill04"
                            | "chainmoving03"
                            | "chainmoving04"
                    )
                {
                    Some("right_cage")
                } else {
                    None
                };
                if let Some(parent) = parent {
                    let origin = self.data.points[parent].translation;
                    let sign = if parent == "left_cage" { 1. } else { -1. };
                    let q = Quat::from_rotation_y(10_f32.to_radians() * cage * sign);
                    p.translation =
                        origin + q * (p.translation - origin) + Vec3::X * (150. * cage * sign);
                    p.rotation = q * p.rotation;
                }
                if matches!(n, "chain01" | "chain02" | "chain03" | "chain04") {
                    p.translation.z -= 300. * cage;
                }
                if n.starts_with("knob") {
                    p.rotation *= Quat::from_rotation_x(100_f32.to_radians() * cage);
                }
                if n.starts_with("chainmoving") || n == "skycamera01" {
                    p.translation.z -= 100000.;
                }
                if self.scripted() && s.talk_done && s.after_talk >= 6.35 {
                    let t = s.after_talk - 6.35;
                    if matches!(n, "circle" | "smashme") {
                        p.translation.z -= 100000.;
                    } else if n.starts_with("circle") || n.starts_with("beam") {
                        let i = n
                            .chars()
                            .filter(char::is_ascii_digit)
                            .collect::<String>()
                            .parse::<usize>()
                            .unwrap_or(1);
                        let f = (t / (3. + (i % 6) as f32)).min(1.);
                        p.translation.z -= 1500. * f;
                        p.rotation *= Quat::from_rotation_y(
                            (if i % 2 == 0 { -40_f32 } else { 40. }).to_radians() * f,
                        );
                    }
                }
                (b.index, p.translation, p.rotation)
            })
            .collect()
    }
    pub(super) fn brush_collider(
        &self,
        map: &Bsp,
        b: &data::Brush,
        p: Vec3,
        q: Quat,
    ) -> Result<Collider> {
        if b.name.starts_with("end_plat") {
            // The original discs are patch-only; these reviewed support boxes cover their walkable tops.
            let mid = (b.min + b.max) * 0.5 + p;
            return Ok(Collider::box_bounds(
                vec3(
                    mid.x - 56.,
                    mid.y - 56.,
                    if b.name == "end_plat1" {
                        p.z - 8.
                    } else {
                        p.z + 384.
                    },
                ),
                vec3(
                    mid.x + 56.,
                    mid.y + 56.,
                    if b.name == "end_plat1" {
                        p.z + 8.
                    } else {
                        p.z + 400.
                    },
                ),
            ));
        }
        Collider::model(map, b.index, p, q, true)
    }
    pub(super) fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        let poses = self.brush_poses();
        if poses == self.poses {
            return Ok(());
        }
        let mut shapes = vec![];
        for (b, (_, p, q)) in self.data.brushes.iter().zip(&poses) {
            if p.z < -90000.
                || b.name.starts_with("circle")
                || b.name.starts_with("beam")
                || matches!(
                    b.name.as_str(),
                    "clockmin" | "clockhr" | "clock_minute_hand" | "clock_hour_hand"
                )
            {
                continue;
            }
            shapes.push(self.brush_collider(map, b, *p, *q)?);
        }
        self.poses = poses;
        self.shapes = shapes;
        Ok(())
    }
    pub(super) fn scene_camera(&self) -> Option<crate::cinematic::Camera> {
        if !self.scripted() {
            return None;
        }
        let s = &self.saved;
        let (n, t) = if s.talk_done {
            let t = s.after_talk;
            if t < 2. {
                (2, t)
            } else if t < 5. {
                (4, t - 2.)
            } else {
                (5, t - 5.)
            }
        } else if s.time < 9.1 {
            (7, (s.time - 0.5).max(0.))
        } else if s.time < 11.6 {
            (1, s.time - 9.1)
        } else {
            ([1, 2, 8, 6, 6][s.line.min(4)], s.line_time)
        };
        let p = self.data.cameras[&format!("hatter2_path{n}")].sample(t, false);
        Some(crate::cinematic::Camera::look(
            p.translation,
            p.translation + p.rotation * Vec3::X * 100.,
        ))
    }
    pub(super) fn performances(&self) -> Vec<Performance> {
        let s = &self.saved;
        let mut out = vec![];
        let scale = self.scale();
        if scale > 0. {
            let mut b = Performance::new(
                "c_madhatter",
                s.boss.clip(),
                s.boss.time,
                s.boss.loops(),
                s.boss.pose(),
            );
            b.scale = scale;
            out.push(b);
        }
        if matches!(s.phase, Phase::Cat | Phase::WatchCat) {
            out.push(Performance::new(
                "c_cheshire",
                if s.time < 2. || s.talk_done {
                    "sit_idle1"
                } else {
                    "sit_talk1"
                },
                s.time,
                true,
                self.data.points[if s.phase == Phase::Cat {
                    "hatter_cat"
                } else {
                    "watch_cat"
                }],
            ));
        }
        let mut g = self.data.points["gryphon_actor1"];
        let mut clip = "idle";
        let mut time = s.time;
        let mut looping = true;
        if self.scripted() {
            if s.talk_done {
                let t = s.after_talk;
                if t < 2. {
                    g = self.data.path("gryphon_land", 0.5);
                } else if t < 5. {
                    g = self.data.path("t137", t - 2.);
                    clip = "fly_dive02";
                    time = t - 2.;
                } else if t < 6.85 {
                    g = self.data.path("gryphon_attack", t - 5.1);
                    clip = if t < 5.85 {
                        "attack_breath_start"
                    } else {
                        "attack_breath_loop"
                    };
                    time = t - if t < 5.85 { 5. } else { 5.85 };
                } else {
                    g = self.data.path("t145", t - 6.85);
                    clip = "fly";
                    time = t - 6.85;
                }
            } else if s.time >= 11.6 {
                g = self.data.path("gryphon_land", 0.5);
                time = s.line_time;
                clip = match s.line {
                    0 if time < 2. => "talk_paw",
                    1 if time < 2. => "idle_nose_scratch",
                    2 if (3.6..5.6).contains(&time) => "talk_paw_circle",
                    4 if (4.2..6.2).contains(&time) => "talk_head_point",
                    _ => "idle",
                };
            } else if s.time >= 11.1 {
                g = self.data.path("gryphon_land", s.time - 11.1);
                clip = "fly_land";
                time = s.time - 11.1;
                looping = false;
            } else if s.time >= 3.1 {
                g = self.data.path("gyphon_hover", s.time - 3.1);
                clip = "fly";
                time = s.time - 1.1;
            } else if s.time >= 1.1 {
                clip = "fly";
                time = s.time - 1.1;
            } else if s.time >= 0.5 {
                clip = "fly_takeoff";
                time = s.time - 0.5;
                looping = false;
            }
        }
        out.push(Performance::new("c_gryphon", clip, time, looping, g));
        if self.scripted() {
            let mut pose = self.data.points["alice_pos2"];
            let ac;
            let mut at = s.line_time;
            if s.time < 3. {
                let f = (s.time / 3.).min(1.);
                pose.translation = s.scene_from.translation.lerp(pose.translation, f);
                ac = "run";
                at = s.time;
            } else if s.talk_done {
                let t = s.after_talk;
                if t < 2. {
                    pose.translation = pose
                        .translation
                        .lerp(self.data.points["alice_pos3"].translation, t / 2.);
                    ac = "walk";
                    at = t;
                } else {
                    pose = self.data.rigs["c_gryphon"].tag("tag_alice", clip, time, g, looping);
                    ac = "riding";
                    at = t - 2.;
                }
            } else {
                ac = match s.line {
                    1 if at >= 2. && at < 4. => "talk_04",
                    3 if at < 1.5 => "talk_01",
                    4 if (2.2..4.2).contains(&at) => "idle_shrug_headtilt",
                    4 if (9.7..11.7).contains(&at) => "idle_shrug_nodyes",
                    _ => "idle_shrug",
                };
            }
            out.push(Performance::new("alice", ac, at, true, pose));
        }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lifts_alternate_with_two_second_dwells() {
        assert_eq!(lift(0.), 0.);
        assert_eq!(lift(1.), 0.5);
        assert_eq!(lift(2.), 1.);
        assert_eq!(lift(3.9), 1.);
        assert_eq!(lift(5.), 0.5);
        assert_eq!(lift(6.), 0.);
        assert_eq!(lift(7.9), 0.);
    }
}
