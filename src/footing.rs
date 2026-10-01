//! Bounded visual leg placement. Never moves the player or changes collision.
use crate::{
    collision::World,
    skeletal::{Skeleton, Transform},
};
use macroquad::prelude::*;
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    offsets: [f32; 2],
}
impl State {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.offsets.iter().all(|v| v.is_finite() && v.abs() <= 10.),
            "Invalid saved foot placement"
        );
        Ok(())
    }
}
pub struct Rig {
    legs: Vec<([usize; 3], f32)>,
}
impl Rig {
    pub fn new(skeleton: &Skeleton, rest: &[Transform], scale: f32) -> Self {
        let pose = skeleton.global_pose(rest);
        let mut legs = Vec::new();
        for side in ["L", "R"] {
            let ids: Option<Vec<_>> = ["Thigh", "Calf", "Foot"]
                .iter()
                .map(|part| {
                    skeleton
                        .bones
                        .iter()
                        .position(|b| b.name == format!("Bip01 {side} {part}"))
                })
                .collect();
            let Some(ids) = ids else { continue };
            let foot = ids[2];
            let bottom = skeleton
                .surfaces
                .iter()
                .flat_map(|s| &s.vertices)
                .filter(|v| v.weights.iter().any(|w| w.bone == foot && w.amount > 0.5))
                .map(|v| v.position(&pose).z)
                .fold(pose[foot].translation.z, f32::min);
            legs.push((
                [ids[0], ids[1], foot],
                ((pose[foot].translation.z - bottom) * scale).clamp(0., 8.),
            ));
        }
        Self { legs }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &self,
        skeleton: &Skeleton,
        local: &mut [Transform],
        state: &mut State,
        world: &World,
        transform: Transform,
        scale: f32,
        dt: f32,
        enabled: bool,
    ) {
        if dt <= 0. {
            return;
        }
        for (i, (leg, sole)) in self.legs.iter().enumerate() {
            let global = skeleton.global_pose(local);
            let foot = transform.point(global[leg[2]].translation * scale);
            let mut correction = 0.;
            if enabled {
                let start = foot + Vec3::Z * (10. - sole);
                let end = start - Vec3::Z * 22.;
                let trace = world.sweep(start, end, vec3(1.5, 1.5, 0.));
                if !trace.start_solid && trace.fraction < 1. && trace.normal.z >= 0.65 {
                    let ground = start.lerp(end, trace.fraction).z;
                    let delta = ground + sole + 0.15 - foot.z;
                    // Correct penetration and low stance feet, preserving lifted swing feet.
                    if delta > 0. || foot.z - sole - transform.translation.z < 5. {
                        correction = delta.clamp(-8., 8.);
                    }
                }
            }
            state.offsets[i] += (correction - state.offsets[i]) * (1. - (-dt * 24.).exp());
            if enabled {
                solve(
                    skeleton,
                    local,
                    *leg,
                    global[leg[2]].translation + Vec3::Z * state.offsets[i] / scale,
                );
            }
        }
    }
}
fn solve(
    skeleton: &Skeleton,
    local: &mut [Transform],
    [hip, knee, foot]: [usize; 3],
    target: Vec3,
) {
    let global = skeleton.global_pose(local);
    let (a, b, c) = (
        global[hip].translation,
        global[knee].translation,
        global[foot].translation,
    );
    let (upper, lower) = ((b - a).length(), (c - b).length());
    if upper < 0.01 || lower < 0.01 {
        return;
    }
    let direction = (target - a).normalize_or_zero();
    let distance = (target - a)
        .length()
        .clamp((upper - lower).abs() + 0.001, upper + lower - 0.001);
    let along = (upper * upper - lower * lower + distance * distance) / (2. * distance);
    let bend = ((b - a) - direction * (b - a).dot(direction)).normalize_or_zero();
    if bend.length_squared() < 0.5 {
        return;
    }
    let desired = a + direction * along + bend * (upper * upper - along * along).max(0.).sqrt();
    let hip_rotation = Quat::from_rotation_arc((b - a).normalize(), (desired - a).normalize())
        * global[hip].rotation;
    local[hip].rotation = skeleton.bones[hip]
        .parent
        .map_or(hip_rotation, |p| {
            global[p].rotation.inverse() * hip_rotation
        })
        .normalize();
    let bent = skeleton.global_pose(local);
    let rotation = Quat::from_rotation_arc(
        (bent[foot].translation - bent[knee].translation).normalize(),
        (a + direction * distance - bent[knee].translation).normalize(),
    ) * bent[knee].rotation;
    local[knee].rotation = (bent[hip].rotation.inverse() * rotation).normalize();
    // Keep the authored boot orientation; bending a knee must not roll its sole.
    local[foot].rotation = (rotation.inverse() * global[foot].rotation).normalize();
}

pub fn check(
    skeleton: &Skeleton,
    rest: &crate::skeletal::Animation,
    walk: &crate::skeletal::Animation,
    scale: f32,
) -> anyhow::Result<()> {
    use anyhow::ensure;
    let rig = Rig::new(skeleton, &rest.sample(0., true), scale);
    ensure!(rig.legs.len() == 2, "Alice foot rig is incomplete");
    let world = World::fixture(&[(vec3(-500., -500., -100.), vec3(500., 500., 2.))]);
    let mut before = 0.;
    let mut after = 0.;
    for sample in 0..20 {
        let original = walk.sample(sample as f32 * walk.duration() / 20., true);
        let mut local = original.clone();
        let mut state = State::default();
        for _ in 0..45 {
            local = original.clone();
            rig.apply(
                skeleton,
                &mut local,
                &mut state,
                &world,
                Transform {
                    translation: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                scale,
                1. / 120.,
                true,
            );
        }
        state.validate()?;
        let old = skeleton.global_pose(&original);
        let new = skeleton.global_pose(&local);
        for (leg, sole) in &rig.legs {
            before += (2. + sole - old[leg[2]].translation.z * scale).max(0.);
            after += (2. + sole - new[leg[2]].translation.z * scale).max(0.);
        }
        ensure!(
            original
                .iter()
                .zip(&local)
                .all(|(a, b)| a.translation == b.translation && b.rotation.is_finite()),
            "Foot placement changed bone lengths or root translation"
        );
        let paused = serde_json::to_vec(&state)?;
        let pose = local.clone();
        rig.apply(
            skeleton,
            &mut local,
            &mut state,
            &world,
            Transform {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            scale,
            0.,
            true,
        );
        ensure!(
            paused == serde_json::to_vec(&state)?
                && pose
                    .iter()
                    .zip(&local)
                    .all(|(a, b)| a.rotation == b.rotation),
            "Paused feet changed"
        );
    }
    ensure!(
        before > 1. && after < before * 0.5,
        "Foot placement did not reduce penetration: {before} -> {after}"
    );
    println!("PASS original Alice leg placement: summed stance penetration {before:.2} -> {after:.2}, unchanged bone lengths/root and paused pose");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leg_solver_preserves_lengths_and_reaches_floor_without_moving_root() {
        let skeleton = Skeleton {
            surfaces: vec![],
            bones: vec![
                crate::skeletal::Bone {
                    parent: None,
                    name: "hip".into(),
                },
                crate::skeletal::Bone {
                    parent: Some(0),
                    name: "knee".into(),
                },
                crate::skeletal::Bone {
                    parent: Some(1),
                    name: "foot".into(),
                },
            ],
        };
        let mut pose = vec![
            Transform {
                translation: vec3(0., 0., 18.),
                rotation: Quat::IDENTITY,
            },
            Transform {
                translation: vec3(3., 0., -9.),
                rotation: Quat::IDENTITY,
            },
            Transform {
                translation: vec3(-3., 0., -9.),
                rotation: Quat::IDENTITY,
            },
        ];
        let root = pose[0].translation;
        let target = vec3(0., 0., 3.);
        solve(&skeleton, &mut pose, [0, 1, 2], target);
        let global = skeleton.global_pose(&pose);
        assert!(global[2].translation.distance(target) < 0.001);
        assert_eq!(pose[0].translation, root);
        assert!((pose[1].translation.length() - 90_f32.sqrt()).abs() < 0.001);
        assert!((pose[2].translation.length() - 90_f32.sqrt()).abs() < 0.001);
    }
}
