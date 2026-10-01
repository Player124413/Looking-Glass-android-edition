//! Reviewed scene cues; all geometry and animation payloads come from local assets.
use super::*;

pub(super) const GUARDS: f32 = 42.6;
pub(super) const END: f32 = 46.6;
pub(super) const VANISH: f32 = 47.15;

pub(super) fn platform(i: usize, t: f32) -> f32 {
    match i {
        1 | 4 | 7 => {
            let start = 13.95 + (i - 1) as f32 * 0.5;
            -192. * (t / 0.1).clamp(0., 1.) + 192. * ((t - start) / 5.).clamp(0., 1.)
        }
        5 => -192. * ((t - 30.3) / 2.).clamp(0., 1.) + 192. * ((t - 33.6) / 2.5).clamp(0., 1.),
        _ => 0.,
    }
}
pub(super) fn steam(i: usize, t: f32) -> bool {
    match i {
        0 => (13.95..16.15).contains(&t),
        1 => (15.45..16.95).contains(&t),
        _ => (16.95..18.45).contains(&t) || (31.3..33.4).contains(&t),
    }
}
pub(super) fn child_clip(i: usize, t: f32) -> (&'static str, f32) {
    let cues: &[(f32, &str)] = match i {
        0 => &[
            (0., "idle01"),
            (13.95, "idle03"),
            (33.4, "antictrans_antic01"),
        ],
        1 => &[
            (0., "idle01"),
            (15.45, "idle_reg_swipe"),
            (20.45, "idle_reg_tick"),
            (33.4, "idle_reg_swipe"),
            (39.6, "idle_reg_tick"),
            (43.6, "idle_reg_swipe"),
        ],
        _ => &[(0., "idle01"), (16.95, "idle06")],
    };
    let &(at, clip) = cues.iter().rev().find(|(at, _)| t >= *at).unwrap();
    (clip, t - at)
}

/// Distance-driven navigation through reviewed BSP stair nodes. Floor projection
/// follows the treads, rather than moving a puppet through a straight diagonal.
pub(super) fn walk(world: &World, path: &[Vec3], distance: f32) -> Transform {
    let mut pose = walk_path(path, distance);
    if let Some(foot) = world.actor_footing(
        pose.translation + Vec3::Z * 32.,
        Vec3::Z * 30.,
        vec3(16., 16., 30.),
        224.,
    ) {
        pose.translation = foot;
    }
    pose
}
fn walk_path(path: &[Vec3], distance: f32) -> Transform {
    let mut left = distance.max(0.);
    let mut pose = Transform {
        translation: path[0],
        rotation: Quat::IDENTITY,
    };
    for pair in path.windows(2) {
        let delta = pair[1] - pair[0];
        let length = delta.truncate().length();
        let f = (left / length.max(0.01)).min(1.);
        pose = Transform {
            translation: pair[0].lerp(pair[1], f),
            rotation: Quat::from_rotation_z(delta.y.atan2(delta.x)),
        };
        left -= length;
        if left <= 0. {
            break;
        }
    }
    pose
}

impl Data {
    pub(super) fn ship_pose(&self, t: f32) -> Transform {
        if t < self.performance_camera2() {
            self.points["tiny_airship1"]
        } else {
            self.ship.sample(t - self.performance_camera2(), true)
        }
    }
    pub(super) fn alice_pose(&self, t: f32) -> Transform {
        if t < 0.2 {
            self.points["fake_pos1"]
        } else if t < 14.3 {
            walk(
                &self.camera_world,
                &self.alice_walk,
                (t - 0.2) * self.alice_speed,
            )
        } else {
            self.points["alice_gnome_pos"]
        }
    }
    fn alice_attention_pose(&self, t: f32) -> Transform {
        if t < 0.2 {
            self.points["fake_pos1"]
        } else if t < 14.3 {
            walk_path(&self.alice_walk, (t - 0.2) * self.alice_speed)
        } else {
            self.points["alice_gnome_pos"]
        }
    }
    pub(super) fn child_pose(&self, i: usize, t: f32) -> Transform {
        let moved = i == 0 && t >= 33.3;
        let mut p = self.points[if moved {
            "theatre_pos5"
        } else {
            ["theatre_pos1", "theatre_pos4", "insane_actor3"][i]
        }];
        p.translation.z += platform(if moved { 5 } else { [1, 4, 7][i] }, t);
        p
    }
    // A seekable bounded head turn preserves the clip's authored body gestures.
    // Native headwatch speed is used as relative response; angular limits remain
    // the shared rig-safe limits, not a claimed reconstruction of native IK.
    pub(super) fn watch(&self, actor: usize, t: f32) -> crate::facial::Watch {
        let mut watch = crate::facial::Watch::default();
        let mut at = 0.;
        while at < t.min(END) {
            let dt = (t.min(END) - at).min(1. / 30.);
            at += dt;
            let (pose, target, speed) = match actor {
                0 if at >= 26.8 => (
                    self.alice_attention_pose(at),
                    self.child_pose(1, at).translation + Vec3::Z * 40.,
                    3.,
                ),
                0 if at >= 22.8 => (
                    self.alice_attention_pose(at),
                    self.child_pose(2, at).translation + Vec3::Z * 40.,
                    2.,
                ),
                0 if at >= 19.3 => (
                    self.alice_attention_pose(at),
                    self.child_pose(0, at).translation + Vec3::Z * 40.,
                    6.,
                ),
                0 if at >= 10.8 => (
                    self.alice_attention_pose(at),
                    self.points["theatre_plat2"].translation,
                    1.,
                ),
                0 if at >= 0.3 => (
                    self.alice_attention_pose(at),
                    self.points["gnome_start"].translation + Vec3::Z * 50.,
                    8.,
                ),
                1 if at >= 26.8 => (
                    self.points["gnome_start"],
                    self.child_pose(1, at).translation + Vec3::Z * 40.,
                    3.,
                ),
                1 if at >= 18.45 => (
                    self.points["gnome_start"],
                    self.ship_pose(at).translation,
                    8.,
                ),
                2 if at >= 33.5 => (
                    self.child_pose(0, at),
                    self.alice_attention_pose(at).translation + Vec3::Z * 55.,
                    6.,
                ),
                _ => {
                    continue;
                }
            };
            let eye = pose.translation + Vec3::Z * if actor == 0 { 55. } else { 40. };
            watch.update(
                dt * speed / 6.,
                Some(pose.rotation.conjugate() * (target - eye)),
            );
        }
        watch
    }
}
