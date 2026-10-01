//! Reviewed decorative mechanisms, sampled from the persisted level clock.
//! Source Euler X is pitch (our Y), Z is roll (our X). Bound joints carry
//! both their orientation and their authored offset through the parent pose.
use super::{fraction, point};
use crate::{bsp::Bsp, entity::Id, skeletal::Transform};
use anyhow::{Context, Result};
use macroquad::prelude::*;

pub(super) struct Machinery {
    widget: [Vec3; 3],
    legs: [[Vec3; 3]; 2],
    smoke: [Id; 2],
}

fn pose(translation: Vec3, rotation: Quat) -> Transform {
    Transform {
        translation,
        rotation,
    }
}
fn roll(degrees: f32) -> Quat {
    Quat::from_rotation_x(degrees.to_radians())
}
fn pitch(degrees: f32) -> Quat {
    Quat::from_rotation_y(degrees.to_radians())
}
fn bound(parent: Transform, offset: Vec3, rotation: Quat) -> Transform {
    pose(
        parent.translation + parent.rotation * offset,
        parent.rotation * rotation,
    )
}

/// Segment and interpolation amount, including wrap without an end-of-cycle snap.
fn segment(time: f32, durations: &[f32]) -> (usize, f32) {
    let mut t = time.max(0.).rem_euclid(durations.iter().sum());
    for (i, &duration) in durations.iter().enumerate() {
        if t < duration || i == durations.len() - 1 {
            return (i, fraction(t, duration));
        }
        t -= duration;
    }
    unreachable!()
}
fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

struct Leg {
    offset: Vec3,
    angles: [f32; 3],
    piston: f32,
    wheel: f32,
    smoke: bool,
}
fn leg(age: f32, side: usize) -> Leg {
    let init = fraction(age, 1.);
    let elapsed = age - 1. - if side == 1 { 0.35 } else { 0. };
    let (stroke, piston, wheel, smoke) = if elapsed < 0. {
        (0., 0., 0., false)
    } else {
        let (i, t) = segment(elapsed, &[0.15, 0.15, 0.35, 0.35]);
        let s = [0., 0.5, 1., 0.5, 0.];
        let p = [0., -8., 0., -8., 0.];
        (
            mix(s[i], s[i + 1], t),
            mix(p[i], p[i + 1], t),
            -30. * (elapsed.floor() + fraction(elapsed.fract(), 0.3)),
            i < 2,
        )
    };
    Leg {
        offset: vec3(0., 32. * stroke, -32. * stroke),
        angles: [
            60. * init - 90. * stroke,
            -90. * init + 70. * stroke,
            30. * init - 60. * stroke,
        ],
        piston,
        wheel,
        smoke,
    }
}

fn cam(age: f32, side: usize) -> (Vec3, f32, f32) {
    let elapsed = (age - if side == 0 { 1. } else { 3. }).max(0.);
    let (i, t) = segment(elapsed, &[0.1, 0.1, 0.5, 0.5, 0.5, 0.2, 0.2, 1.]);
    let x = [0., 16., 48., 80., 96., 80., 48., 16., 0.];
    let z = [0., -32., -48., -32., 0., 32., 48., 32., 0.];
    let a = [0., -7., -14., -7., 0., 7., 14., 7., 0.];
    (
        vec3(mix(x[i], x[i + 1], t), 0., mix(z[i], z[i + 1], t)),
        mix(a[i], a[i + 1], t),
        -45. * (i as f32 + t),
    )
}

impl Machinery {
    pub(super) fn load(map: &Bsp) -> Result<Self> {
        let joints = |side: &str| -> Result<[Vec3; 3]> {
            Ok([
                point(map, &format!("{side}leg_thigh"))?,
                point(map, &format!("{side}leg_calf"))?,
                point(map, &format!("{side}leg_foot"))?,
            ])
        };
        let smoke = |name: &str| -> Result<Id> {
            map.entities
                .iter()
                .position(|e| e.get("targetname").is_some_and(|n| n == name))
                .map(Id)
                .context("Missing machinery steam emitter")
        };
        Ok(Self {
            widget: [
                point(map, "widget_arm1")?,
                point(map, "widget_arm2")?,
                point(map, "widget_arm3")?,
            ],
            legs: [joints("left")?, joints("rght")?],
            smoke: [smoke("leftleg_smoke")?, smoke("rghtleg_smoke")?],
        })
    }
    pub(super) fn emissions(&self, age: f32) -> [(Id, bool); 2] {
        [
            (self.smoke[0], leg(age, 0).smoke),
            (self.smoke[1], leg(age, 1).smoke),
        ]
    }
    pub(super) fn sample(&self, name: &str, base: Vec3, age: f32) -> Option<Transform> {
        match name {
            "widget_arm1" | "widget_arm2" | "widget_arm3" => {
                let (i, t) = segment(age, &[0.2, 0.1, 0.5, 0.2]);
                let amounts = [0., 1., 1., 0., 0.];
                let a = mix(amounts[i], amounts[i + 1], t);
                let parent = pose(self.widget[0], roll(60. * a));
                Some(match name {
                    "widget_arm1" => parent,
                    "widget_arm2" => bound(parent, self.widget[1] - self.widget[0], roll(-50. * a)),
                    _ => bound(parent, self.widget[2] - self.widget[0], roll(-50. * a)),
                })
            }
            "leg_wheel" => Some(pose(base, roll(leg(age, 0).wheel + leg(age, 1).wheel))),
            "leftleg_thigh" | "leftleg_calf" | "leftleg_foot" | "leftleg_piston"
            | "rghtleg_thigh" | "rghtleg_calf" | "rghtleg_foot" | "rghtleg_piston" => {
                let side = usize::from(name.starts_with("rght"));
                let l = leg(age, side);
                if name.ends_with("piston") {
                    return Some(pose(base + Vec3::Z * l.piston, Quat::IDENTITY));
                }
                let b = self.legs[side];
                let thigh = pose(b[0] + l.offset, roll(l.angles[0]));
                let calf = bound(thigh, b[1] - b[0], roll(l.angles[1]));
                Some(if name.ends_with("thigh") {
                    thigh
                } else if name.ends_with("calf") {
                    calf
                } else {
                    bound(calf, b[2] - b[1], roll(l.angles[2]))
                })
            }
            "cam_wheel1" | "cam_wheel2" | "cam_arm1" | "cam_arm2" => {
                let (offset, arm, wheel) = cam(age, usize::from(name.ends_with('2')));
                Some(if name.starts_with("cam_arm") {
                    pose(base + offset, pitch(arm))
                } else {
                    pose(base, pitch(wheel))
                })
            }
            _ => None,
        }
    }
}

pub(super) fn check(p: &mut super::Pandemonium, map: &Bsp) -> Result<()> {
    use anyhow::ensure;
    let original = p.snapshot();
    let ages = [
        0., 0.1, 0.2, 0.3, 0.8, 1., 1.075, 1.15, 1.3, 1.35, 1.5, 1.65, 2., 2.2, 2.7, 3., 3.2, 3.7,
        4.1, 100.275,
    ];
    for age in ages {
        p.state.age = age;
        p.rebuild(map)?;
        let object = |name: &str| p.objects.iter().find(|o| o.name == name).unwrap();
        for (parent, child) in [
            ("widget_arm1", "widget_arm2"),
            ("widget_arm1", "widget_arm3"),
            ("leftleg_thigh", "leftleg_calf"),
            ("leftleg_calf", "leftleg_foot"),
            ("rghtleg_thigh", "rghtleg_calf"),
            ("rghtleg_calf", "rghtleg_foot"),
        ] {
            let (a, b) = (object(parent), object(child));
            let local = a.rotation.inverse() * (b.origin - a.origin);
            ensure!(
                local.distance(b.base - a.base) < 0.002,
                "Detached machinery joint {child} at {age}"
            );
        }
        for name in ["widget_arm1", "leg_wheel", "cam_wheel1", "cam_wheel2"] {
            let o = object(name);
            ensure!(o.origin == o.base, "Machinery axle moved: {name}");
            let axis = if name.starts_with("cam") {
                Vec3::Y
            } else {
                Vec3::X
            };
            ensure!(
                (o.rotation * axis).distance(axis) < 0.0001,
                "Wrong axle axis: {name}"
            );
        }
        for name in ["leftleg_piston", "rghtleg_piston"] {
            let o = object(name);
            ensure!(
                o.rotation == Quat::IDENTITY
                    && o.origin.truncate() == o.base.truncate()
                    && (-8.001..=0.001).contains(&(o.origin.z - o.base.z)),
                "Piston spun or detached"
            );
        }
        let before = p.transforms().collect::<Vec<_>>();
        let smoke = p.machinery.emissions(age);
        let encoded = serde_json::to_vec(&p.snapshot())?;
        p.state.age = 0.;
        p.rebuild(map)?;
        p.restore(&serde_json::from_slice(&encoded)?, map)?;
        ensure!(
            before == p.transforms().collect::<Vec<_>>()
                && smoke == p.machinery.emissions(p.state.age),
            "Save/load changed machinery phase"
        );
    }
    // Every stroke boundary, including cycle wrap, is continuous in world space.
    for age in [
        0.2, 0.3, 0.8, 1., 1.15, 1.2, 1.3, 1.35, 1.5, 1.65, 1.7, 1.85, 2., 2.2, 2.7, 2.9, 3., 3.1,
        3.2, 3.7, 4.1,
    ] {
        for o in &p.objects {
            let Some(a) = p.machinery.sample(&o.name, o.base, age - 0.00001) else {
                continue;
            };
            let b = p.machinery.sample(&o.name, o.base, age + 0.00001).unwrap();
            ensure!(
                a.translation.distance(b.translation) < 0.05
                    && (a.rotation * Vec3::Z).distance(b.rotation * Vec3::Z) < 0.001,
                "Machinery jumps at a stroke boundary: {} / {age}",
                o.name
            );
        }
    }
    p.restore(&original, map)?;
    println!("PASS Pandemonium articulated machinery: authored joints, axle axes, piston travel, continuous strokes and saved phases");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn near(a: f32, b: f32) {
        assert!((a - b).abs() < 0.001, "{a} != {b}");
    }
    #[test]
    fn leg_power_return_stagger_and_independent_piston() {
        let startup = leg(0.5, 0);
        assert_eq!(startup.angles, [30., -45., 15.]);
        assert!(!startup.smoke);
        let down = leg(1.15, 0);
        near(down.offset.y, 16.);
        near(down.offset.z, -16.);
        near(down.piston, -8.);
        near(down.wheel, -15.);
        assert!(down.smoke);
        assert_eq!(leg(1.15, 1).offset, Vec3::ZERO);
        let return_half = leg(1.475, 0);
        near(return_half.offset.y, 24.);
        near(return_half.piston, -4.);
        assert!(!return_half.smoke);
        near(leg(2., 0).offset.length(), 0.);
        near(leg(2., 0).wheel + leg(2., 1).wheel, -60.);
    }
    #[test]
    fn cams_wait_then_follow_eight_unequal_strokes() {
        assert_eq!(cam(0.5, 0), (Vec3::ZERO, 0., 0.));
        assert_eq!(cam(2.9, 1), (Vec3::ZERO, 0., 0.));
        let (p, a, w) = cam(1.2, 0);
        near(p.x, 48.);
        near(p.z, -48.);
        near(a, -14.);
        near(w, -90.);
        let (p, a, w) = cam(2.7, 0);
        near(p.x, 80.);
        near(p.z, 32.);
        near(a, 7.);
        near(w, -225.);
        let (p, a, _) = cam(4.1, 0);
        near(p.length(), 0.);
        near(a, 0.);
        let first = cam(1.5, 0);
        let second = cam(3.5, 1);
        near(first.0.distance(second.0), 0.);
        near(first.1, second.1);
    }
    #[test]
    fn joint_binding_preserves_local_mount_and_composes_rotation() {
        let parent = pose(vec3(12., 20., 30.), roll(60.));
        let offset = vec3(16., -64., 0.);
        let child = bound(parent, offset, roll(-50.));
        assert!(
            (parent.rotation.inverse() * (child.translation - parent.translation)).distance(offset)
                < 0.0001
        );
        assert!((child.rotation * Vec3::Y).distance(roll(10.) * Vec3::Y) < 0.0001);
        assert!(child.translation.distance(parent.translation + offset) > 60.);
    }
}
