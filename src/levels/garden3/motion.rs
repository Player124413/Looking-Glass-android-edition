//! Drawing and collision share the gate, pillar and floor transforms.
use super::*;
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Door {
    pub open: bool,
    pub fraction: f32,
    pub latched: bool,
}
impl Door {
    pub fn new(open: bool) -> Self {
        Self {
            open,
            fraction: if open { 1. } else { 0. },
            latched: false,
        }
    }
    pub fn advance(&mut self, dt: f32, travel: f32) {
        let delta = dt * 10000. / travel;
        self.fraction = if self.open {
            (self.fraction + delta).min(1.)
        } else {
            (self.fraction - delta).max(0.)
        };
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.fraction.is_finite()
                && (0. ..=1.).contains(&self.fraction)
                && (!self.latched || self.open),
            "Invalid chase door"
        );
        Ok(())
    }
}
pub(super) struct Object {
    pub model: usize,
    pub name: String,
    pub base: Transform,
    pub door: Option<usize>,
    pub pose: Option<Transform>,
    pub collider: Collider,
}
pub(super) fn owns(e: &crate::levels::Entity) -> bool {
    e.get("classname")
        .is_some_and(|s| matches!(s.as_str(), "script_object" | "func_door"))
}
pub(super) fn load(map: &Bsp) -> Result<Vec<Object>> {
    map.entities
        .iter()
        .enumerate()
        .filter(|(_, e)| owns(e))
        .map(|(id, e)| {
            let model = e["model"].trim_start_matches('*').parse()?;
            let base = data::at(e);
            Ok(Object {
                model,
                name: e.get("targetname").cloned().unwrap_or_default(),
                base,
                door: DOORS.iter().position(|&n| n == id),
                pose: Some(base),
                collider: Collider::model(map, model, base.translation, base.rotation, true)?,
            })
        })
        .collect()
}
// Independent numeric schedule: piece, delay after callback, travel seconds, X/Z degrees.
const ICE: &[(usize, f32, f32, f32, f32)] = &[
    (1, 0.9, 6., -55., 55.),
    (2, 1.1, 6.4, -55., 0.),
    (3, 1.2, 6.4, 0., 55.),
    (4, 1.5, 6., -45., 45.),
    (5, 1.7, 7.5, 0., 60.),
    (6, 1.7, 6.4, 0., -55.),
    (7, 1.8, 6., -55., 55.),
    (8, 2., 6.4, -55., 0.),
    (9, 2., 6., 0., 35.),
    (10, 2.3, 6.4, 0., 55.),
    (11, 2.3, 6., -45., 45.),
    (12, 2.4, 6.4, 0., -35.),
    (13, 2.6, 6., -45., 45.),
    (14, 2.6, 6.4, -55., 0.),
    (15, 2.8, 6., -45., 55.),
    (16, 3.1, 6.4, 0., 55.),
    (17, 3.1, 6., -45., 45.),
    (18, 3.2, 6.4, 0., 55.),
    (19, 3.5, 6., -45., 45.),
];
const MARBLE_ICE: &[(usize, f32, f32, f32, f32)] = &[
    (1, 0.2, 6., -55., 55.),
    (2, 0.5, 6.4, 0., 55.),
    (3, 0.6, 6.4, -55., 0.),
    (4, 0.9, 7.5, 0., 55.),
];
const END_PIECES: &[(usize, f32, f32, f32, f32)] = &[
    (1, 0.1, 6., -55., 55.),
    (2, 0.1, 6.4, -55., 55.),
    (3, 0.1, 5.8, -55., 55.),
    (4, 0.3, 6., -55., 55.),
    (5, 0.3, 6.4, -55., 55.),
    (6, 0.4, 7., -55., 55.),
    (7, 0.4, 6., -55., 55.),
    (8, 0.4, 5.8, -55., 55.),
];
fn falling(
    mut p: Transform,
    t: f32,
    piece: usize,
    schedule: &[(usize, f32, f32, f32, f32)],
) -> Option<Transform> {
    let &(_, delay, seconds, x, z) = schedule.iter().find(|v| v.0 == piece)?;
    let f = ((t - delay) / seconds).clamp(0., 1.);
    if f == 1. {
        return None;
    }
    p.translation.z -= 2400. * f;
    p.rotation = p.rotation
        * Quat::from_rotation_z((z * f).to_radians())
        * Quat::from_rotation_x((x * f).to_radians());
    Some(p)
}
impl Garden {
    pub(super) fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        for o in &mut self.objects {
            let mut pose = Some(o.base);
            if let Some(k) = o.door {
                pose.as_mut().unwrap().translation.z +=
                    self.data.travel[k] * self.saved.doors[k].fraction;
            }
            if o.name == "falling_pillar1" {
                if let Some(at) = self.saved.pillar {
                    let t = (self.saved.age - at - 0.2).max(0.);
                    if t >= 4. {
                        pose = None;
                    } else {
                        pose.as_mut().unwrap().rotation *=
                            Quat::from_rotation_z((-10. * t * t).to_radians());
                    }
                }
            }
            if let Some(at) = self.saved.ice {
                if let Some(n) = o
                    .name
                    .strip_prefix("icefloor2nd")
                    .and_then(|v| v.parse().ok())
                {
                    pose = falling(o.base, self.saved.age - at, n, ICE);
                }
                if let Some(n) = o
                    .name
                    .strip_prefix("marble_ice")
                    .and_then(|v| v.parse().ok())
                {
                    pose = falling(o.base, self.saved.age - at, n, MARBLE_ICE);
                }
            }
            if let Some(at) = self.saved.end {
                if let Some(n) = o
                    .name
                    .strip_prefix("end_piece")
                    .and_then(|v| v.parse().ok())
                {
                    pose = falling(o.base, self.saved.age - at, n, END_PIECES);
                }
                if o.name == "end_fall1" && self.saved.age - at >= 0.6 {
                    pose = None;
                }
            }
            if let Some(p) = pose {
                if o.pose.is_none_or(|old| {
                    old.translation != p.translation || old.rotation != p.rotation
                }) {
                    o.collider = Collider::model(map, o.model, p.translation, p.rotation, true)?;
                }
            }
            o.pose = pose;
        }
        let shroom_pose = self.objects.iter()
            .find(|o| o.name == "falling_pillar1").and_then(|o| o.pose)
            .map(|pillar| {
                let base = self.data.points["falling_pillar1"];
                let shroom = self.data.points["falling_pillar_shroom1"];
                Transform {
                    translation: pillar.point(base.rotation.conjugate() * (shroom.translation - base.translation)),
                    rotation: pillar.rotation * base.rotation.conjugate() * shroom.rotation,
                }
            });
        if let Some(pose) = shroom_pose {
            if self.shroom.as_ref().is_none_or(|(old, _)|
                old.translation != pose.translation || old.rotation != pose.rotation) {
                self.shroom = Some((pose, Collider::triangles(
                    &self.data.shroom_triangles, pose.translation, pose.rotation)));
            }
        } else {
            self.shroom = None;
        }
        let colliders = self.colliders();
        self.data.world.set_dynamic(colliders);
        Ok(())
    }
}
