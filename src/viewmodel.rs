//! Weapon-only camera presentation. The archives have no dedicated first-person hand rig.
use crate::{
    skeletal::Transform,
    weapons::{Action, Actions},
};
use macroquad::prelude::*;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ViewModel {
    phase: f32,
    bob: Vec3,
}
pub struct Pose {
    pub anchors: [Transform; 3],
    pub scale: f32,
}

/// Local +X is forward, +Y left, +Z up, including when looking up or down.
pub fn camera_frame(eye: Vec3, direction: Vec3) -> Transform {
    let forward = direction.try_normalize().unwrap_or(Vec3::X);
    let left = Vec3::Z.cross(forward).try_normalize().unwrap_or(Vec3::Y);
    let up = forward.cross(left);
    Transform {
        rotation: Quat::from_mat3(&Mat3::from_cols(forward, left, up)),
        translation: eye,
    }
}

fn base(weapon: usize) -> (Transform, f32) {
    let (position, axis, tip, scale) = match weapon {
        0 => (vec3(20., -12., -17.), -Vec3::X, vec3(0.45, 0.15, 0.88), 1.),
        1 => (vec3(20., -10., -13.), Vec3::Z, vec3(0.3, 0., 0.95), 1.),
        2 => (vec3(23., -12., -20.), Vec3::X, vec3(0.42, 0.05, 0.9), 0.8),
        3 => (vec3(20., -10., -7.), Vec3::X, Vec3::X, 1.),
        4 => (vec3(22., -12., -19.), Vec3::X, vec3(0.55, 0., 0.84), 0.65),
        5 => (vec3(21., -11., -12.), Vec3::X, vec3(0.7, 0., 0.7), 1.),
        6 => (vec3(17., -9., -9.), Vec3::X, Vec3::X, 1.5),
        7 => (vec3(24., -12., -20.), Vec3::X, vec3(0.6, 0., 0.8), 0.5),
        8 => (vec3(25., -11., -13.), Vec3::X, Vec3::X, 0.65),
        _ => (vec3(20., -10., -11.), Vec3::X, Vec3::X, 1.5),
    };
    (
        Transform {
            translation: position,
            rotation: Quat::from_rotation_arc(axis, tip.normalize())
                * if weapon == 0 {
                    Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)
                } else if weapon == 1 {
                    Quat::from_rotation_z(1.)
                } else {
                    Quat::IDENTITY
                },
        },
        scale,
    )
}

impl ViewModel {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.phase.is_finite() && self.bob.is_finite(),
            "Invalid saved first-person presentation"
        );
        Ok(())
    }
    pub fn pose(&mut self, dt: f32, speed: f32, actions: &Actions, camera: Transform) -> Pose {
        if dt > 0. {
            self.phase =
                (self.phase + dt * (speed / 35.).clamp(0., 9.)).rem_euclid(std::f32::consts::TAU);
            let amount = (speed / 220.).clamp(0., 1.);
            let target = vec3(0., self.phase.sin() * 0.3, (self.phase * 2.).cos() * 0.25) * amount;
            self.bob = self.bob.lerp(target, 1. - (-dt * 12.).exp());
        }
        let (weapon, _) = actions.equipment();
        let (mut grip, scale) = base(weapon);
        grip.translation += self.bob;
        if let Some(play) = &actions.playing {
            if matches!(play.action, Action::Equip(_)) {
                let dip = (1. - ((play.frame() - 5.) / 5.).abs()).max(0.);
                grip.translation.z -= dip * 10.;
            } else {
                let swing = (play.frame() / 14.).clamp(0., 1.);
                let pulse = (swing * std::f32::consts::PI).sin();
                if weapon == 8 {
                    let recoil = ((play.time - 0.75) / 0.32).clamp(0., 1.);
                    let kick = (recoil * std::f32::consts::PI).sin();
                    grip.translation += vec3(-9., 0., 4.) * kick;
                    grip.rotation = Quat::from_rotation_y(-kick * 0.35) * grip.rotation;
                } else if !play.alternate() && [0, 2].contains(&weapon) {
                    let side = if play.clip == 2 { -1. } else { 1. };
                    grip.translation += vec3(3., side * 12., 7.) * pulse;
                    grip.rotation = Quat::from_rotation_x(side * pulse * 1.5) * grip.rotation;
                } else {
                    grip.translation += vec3(5., 3., 5.) * pulse;
                    grip.rotation = Quat::from_rotation_y(-pulse * 0.4) * grip.rotation;
                }
            }
        }
        let mut anchors = [grip; 3];
        anchors[1].translation = vec3(22., 7., -10.) + self.bob;
        anchors[2].translation = vec3(25., 4., -5.) + self.bob;
        for anchor in &mut anchors {
            anchor.translation = camera.point(anchor.translation);
            anchor.rotation = camera.rotation * anchor.rotation;
        }
        Pose { anchors, scale }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vertical_and_empty_aims_have_finite_camera_frames() {
        for direction in [Vec3::Z, -Vec3::Z, Vec3::ZERO] {
            let frame = camera_frame(Vec3::ZERO, direction);
            assert!(frame.rotation.is_finite());
            assert!((frame.rotation.length_squared() - 1.).abs() < 0.0001);
        }
    }
    #[test]
    fn view_frame_tracks_yaw_and_pitch_without_changing_handedness() {
        for yaw in [0.0f32, 1., 3., 5.] {
            for pitch in [-1.4f32, 0., 1.4] {
                let direction = vec3(
                    yaw.cos() * pitch.cos(),
                    yaw.sin() * pitch.cos(),
                    pitch.sin(),
                );
                let frame = camera_frame(vec3(3., 4., 5.), direction);
                assert!((frame.rotation * Vec3::X - direction).length() < 0.0001);
                assert!((frame.rotation * Vec3::Y).dot(Vec3::Z).abs() < 0.0001);
                for weapon in 0..10 {
                    let (grip, scale) = base(weapon);
                    let world = frame.point(grip.translation);
                    assert!(
                        (frame.rotation.conjugate() * (world - frame.translation)
                            - grip.translation)
                            .length()
                            < 0.0001
                    );
                    assert!(scale > 0. && grip.translation.x > 0. && grip.translation.y < 0.);
                }
            }
        }
    }
}
