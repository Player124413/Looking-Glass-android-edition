//! Village brush machinery, with solid transforms shared by drawing and movement.
//! Parameters are measurements from the user's map and reviewed mover declarations.
pub mod cinema;
mod machinery;
pub mod machinery_render;
pub fn check(assets: &mut crate::assets::Assets) -> Result<()> {
    machinery::check(assets)
}
use crate::{
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    interaction::{vector, Events},
    movement::Player,
};
use anyhow::{Context, Result};
use macroquad::prelude::*;
use std::collections::BTreeMap;

pub fn supported(e: &BTreeMap<String, String>) -> bool {
    e.get("classname").is_some_and(|s| s == "script_object")
        && e.get("model").is_some_and(|s| s.starts_with('*'))
        && e.get("targetname").is_some_and(|s| s != "hole_clip")
}
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
struct Pose {
    origin: Vec3,
    angles: Vec3,
}
impl Pose {
    fn composed(origin: Vec3, rotation: Quat) -> Self {
        // Keep the existing Euler save representation, including older saves.
        let (z, y, x) = rotation.as_dquat().normalize().to_euler(EulerRot::ZYX);
        Self {
            origin,
            angles: vec3(
                x.to_degrees() as f32,
                y.to_degrees() as f32,
                z.to_degrees() as f32,
            ),
        }
    }
    fn rotation(self) -> Quat {
        Quat::from_rotation_z(self.angles.z.to_radians())
            * Quat::from_rotation_y(self.angles.y.to_radians())
            * Quat::from_rotation_x(self.angles.x.to_radians())
    }
}
struct Object {
    name: String,
    model: usize,
    base: Vec3,
    pose: Pose,
    time: f32,
    solid: bool,
    collider: Collider,
}
pub struct Village {
    pub cinema: cinema::Cinema,
    objects: Vec<Object>,
    emitters: Vec<machinery::Attachment>,
    hatch_open: bool,
    hatch_time: f32,
}
fn curve(t: f32, keys: &[(f32, f32)]) -> f32 {
    for k in keys.windows(2) {
        if t <= k[1].0 {
            return k[0].1 + (k[1].1 - k[0].1) * ((t - k[0].0) / (k[1].0 - k[0].0)).clamp(0., 1.);
        }
    }
    keys.last().unwrap().1
}
fn sample(name: &str, base: Vec3, time: f32, hatch: f32) -> Pose {
    let mut p = Pose {
        origin: base,
        angles: Vec3::ZERO,
    };
    let t = time.rem_euclid(3.);
    let swing = curve(t, &[(0., 0.), (1., 1.), (3., 0.)]);
    if let Some(i) = name
        .strip_prefix("sawmill_slat")
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|i| (1..=5).contains(i))
    {
        let height = [32., 16., 0., -16., -32.][i - 1];
        p.origin.z += height * (1. - 2. * swing);
        p.angles.y = -[5., 4., 3., 2., 1.][i - 1] + [9., 6., 3., 6., 9.][i - 1] * swing;
    } else if name == "sawmill_beam" {
        p.angles.y = 30. - 60. * swing;
    } else if name.starts_with("sawmill_cam_") {
        let angle = 45. - 90. * swing;
        let rotation = Quat::from_rotation_y(angle.to_radians());
        if name == "sawmill_cam_wheel" {
            p.angles.y = angle;
        } else {
            let pivot = vec3(-3028., 3092., 208.);
            p.origin = pivot + rotation * (base - pivot);
        }
    } else if name.starts_with("mushroom_roof") {
        let i = name.as_bytes()[name.len() - 1] as usize - b'1' as usize;
        let (amount, wait) = [(4., 2.), (6., 3.), (2., 1.)][i];
        p.origin.z -= amount * machinery::roof(time, wait).0;
    } else if name.starts_with("spinrod") {
        let delay = (name.as_bytes()[7] - b'1') as f32 * 2.;
        let t = (time - delay).max(0.).rem_euclid(5.);
        p.origin.x += curve(t, &[(0., 88.), (0.5, 0.), (4.5, 0.), (5., 88.)]);
        let roll = curve(t, &[(0., 0.), (1.5, 0.), (3.5, 720.), (5., 720.)]);
        p.angles.x = roll;
        if name.ends_with("arm") {
            let pitch = curve(
                t,
                &[
                    (0., 0.),
                    (0.5, 0.),
                    (1.5, 90.),
                    (3.5, 90.),
                    (4.5, 0.),
                    (5., 0.),
                ],
            );
            // The arm bends in its parent's frame before the shaft spins.
            // R_y * R_x incorrectly bends the spinning shaft's world axis.
            return Pose::composed(
                p.origin,
                Quat::from_rotation_x(roll.to_radians())
                    * Quat::from_rotation_y(-pitch.to_radians()),
            );
        }
    } else if name.starts_with("puff_ball") {
        let (height, wait) = if name.ends_with('1') {
            (128., 0.5)
        } else {
            (96., 2.)
        };
        let t = (time - wait).max(0.).rem_euclid(0.5 + wait * 2.);
        p.origin.z += curve(
            t,
            &[
                (0., 0.),
                (0.5, height),
                (0.5 + wait, height),
                (0.5 + wait * 2., 0.),
            ],
        );
        // Keep rotations unwrapped across loops; no snap at the cycle boundary.
        p.angles.z = -time * 240.;
    } else if name == "cam_wheel" {
        p.angles.y = -curve(
            time.rem_euclid(1.8),
            &[
                (0., 0.),
                (0.2, 45.),
                (0.3, 90.),
                (0.5, 135.),
                (0.8, 180.),
                (1.2, 225.),
                (1.5, 270.),
                (1.7, 315.),
                (1.8, 360.),
            ],
        );
    } else if name == "cam_piston" || name == "cam_arm" {
        let t = time.rem_euclid(1.8);
        if name == "cam_piston" {
            p.origin.z += curve(
                t,
                &[
                    (0., 0.),
                    (0.2, -48.),
                    (0.3, 0.),
                    (0.5, -48.),
                    (0.8, 0.),
                    (1.2, -48.),
                    (1.5, 0.),
                    (1.7, -48.),
                    (1.8, 0.),
                ],
            );
        } else {
            p.origin.x += curve(
                t,
                &[
                    (0., 0.),
                    (0.2, 16.),
                    (0.3, 48.),
                    (0.5, 80.),
                    (0.8, 96.),
                    (1.2, 80.),
                    (1.5, 48.),
                    (1.7, 16.),
                    (1.8, 0.),
                ],
            );
            p.origin.z += curve(
                t,
                &[
                    (0., 0.),
                    (0.2, -32.),
                    (0.3, -48.),
                    (0.5, -32.),
                    (0.8, 0.),
                    (1.2, 32.),
                    (1.5, 48.),
                    (1.7, 32.),
                    (1.8, 0.),
                ],
            );
            p.angles.y = -curve(
                t,
                &[
                    (0., 0.),
                    (0.2, 7.),
                    (0.3, 14.),
                    (0.5, 7.),
                    (0.8, 0.),
                    (1.2, -7.),
                    (1.5, -14.),
                    (1.7, -7.),
                    (1.8, 0.),
                ],
            );
        }
    } else if name.starts_with("teeter_") {
        let t = time.rem_euclid(7.7);
        if name == "teeter_flap" {
            p.angles.x = curve(
                t,
                &[
                    (0., 30.),
                    (1.5, -15.),
                    (2., 5.),
                    (2.5, 0.),
                    (4.7, 0.),
                    (5.2, 45.),
                    (5.9, 30.),
                    (7.7, 30.),
                ],
            );
        }
        if name == "teeter_ramp" {
            p.angles.x = curve(
                t,
                &[
                    (0., 0.),
                    (2., 10.),
                    (3., 15.),
                    (4.7, 15.),
                    (5.7, 5.),
                    (6.7, -5.),
                    (7.7, 0.),
                ],
            );
        }
        if name == "teeter_roller" {
            p.origin.y += curve(
                t,
                &[
                    (0., 0.),
                    (2., -80.),
                    (3., -120.),
                    (3.2, -116.),
                    (3.7, -120.),
                    (4.7, -120.),
                    (5.7, -72.),
                    (6.7, -24.),
                    (7.7, 0.),
                ],
            );
            p.origin.z += curve(
                t,
                &[
                    (0., 0.),
                    (2., -12.),
                    (3., -32.),
                    (3.2, -30.),
                    (3.7, -32.),
                    (4.7, -32.),
                    (5.7, -2.),
                    (6.7, 8.),
                    (7.7, 0.),
                ],
            );
            p.angles.x = curve(
                t,
                &[
                    (0., 0.),
                    (2., 180.),
                    (3., 270.),
                    (3.2, 265.),
                    (3.7, 270.),
                    (4.7, 270.),
                    (5.7, 180.),
                    (6.7, 90.),
                    (7.7, 0.),
                ],
            );
        }
    } else if name == "tenticle2" {
        p.origin += vec3(128., -64., 128.);
        p.angles.z = 1.;
    } else if name == "tenticle3" {
        p.origin += vec3(0., -128., 64.);
        p.angles = vec3(3., 0., 3.);
    } else if name == "tenticle3_rock" {
        p.origin += vec3(0., -128., 128.);
        p.angles.x = 180.;
    } else if name == "openthis" {
        p.angles.y = hatch.clamp(0., 1.) * 90.;
    }
    p
}
impl Village {
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            cinema: Some(self.cinema.state.clone()),
            hatch_open: self.hatch_open,
            hatch_time: self.hatch_time,
            objects: self
                .objects
                .iter()
                .map(|o| (o.name.clone(), o.pose, o.time))
                .collect(),
        }
    }
    pub fn restore(&mut self, s: &Snapshot, map: &Bsp) -> Result<()> {
        anyhow::ensure!(
            self.objects.len() == s.objects.len() && (0.0..=1.).contains(&s.hatch_time),
            "Invalid saved village state"
        );
        for (o, (name, pose, time)) in self.objects.iter_mut().zip(&s.objects) {
            anyhow::ensure!(
                &o.name == name && *time >= 0.,
                "Saved village object does not match map"
            );
            o.pose = *pose;
            o.time = *time;
            o.collider = Collider::model(map, o.model, pose.origin, pose.rotation(), true)?;
        }
        self.cinema.restore(s.cinema.as_ref())?;
        self.hatch_open = s.hatch_open;
        self.hatch_time = s.hatch_time;
        Ok(())
    }
    pub fn load(assets: &mut crate::assets::Assets, map: &Bsp) -> Result<Self> {
        let mut objects = Vec::new();
        for e in map.entities.iter().filter(|e| supported(e)) {
            let model = e["model"].trim_start_matches('*').parse()?;
            let base = vector(&e["origin"]).context("Invalid village brush origin")?;
            let name = e["targetname"].clone();
            let pose = sample(&name, base, 0., 0.);
            objects.push(Object {
                solid: !name.starts_with("spinrod"),
                name,
                model,
                base,
                pose,
                time: 0.,
                collider: Collider::model(map, model, pose.origin, pose.rotation(), true)?,
            });
        }
        Ok(Self {
            cinema: cinema::Cinema::load(assets, map)?,
            objects,
            emitters: machinery::attachments(map)?,
            hatch_open: false,
            hatch_time: 0.,
        })
    }
    pub fn sound_state(&self, clocks: &mut Vec<crate::audio::world::Clock>) {
        self.cinema.sound_state(clocks);
        if let Some(o) = self.objects.iter().find(|o| o.name == "teeter_roller") {
            clocks.push(crate::audio::world::Clock {
                key: "village-roller",
                time: o.time,
                period: Some(7.7),
                origin: o.pose.origin,
                cues: &[
                    (0., "sound/world/mover/roll_1.wav"),
                    (4.7, "sound/world/mover/roll_2.wav"),
                ],
            });
        }
    }
    pub fn colliders(&self) -> impl Iterator<Item = Collider> + '_ {
        self.objects
            .iter()
            .filter(|o| o.solid && !(o.name == "openthis" && self.hatch_open))
            .map(|o| o.collider.clone())
    }
    pub fn transforms(&self) -> impl Iterator<Item = (usize, Vec3, Quat)> + '_ {
        self.objects
            .iter()
            .map(|o| (o.model, o.pose.origin, o.pose.rotation()))
    }
    pub fn event(&mut self, thread: &str) -> Option<Events> {
        if thread == "openthis" {
            self.hatch_open = true;
            return Some(Events {
                message: Some("The hatch swings open.".into()),
                ..Default::default()
            });
        }
        crate::story::supports("gvillage", thread).then(|| Events {
            story: vec![thread.into()],
            ..Default::default()
        })
    }
    pub fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. {
            return Ok(());
        }
        self.cinema.advance(dt.min(0.1), player, world)?;
        let mut remaining = dt.min(0.1);
        while remaining > 0.000001 {
            let step = remaining.min(1. / 120.);
            remaining -= step;
            if self.hatch_open {
                self.hatch_time = (self.hatch_time + step).min(1.);
            }
            for i in 0..self.objects.len() {
                let o = &self.objects[i];
                let mut next = sample(&o.name, o.base, o.time + step, self.hatch_time);
                if o.name == "shrink_door1" {
                    next.angles.z = self.cinema.door_angle();
                }
                if next.origin == o.pose.origin && next.angles == o.pose.angles {
                    self.objects[i].time += step;
                    continue;
                }
                let collider = Collider::model(map, o.model, next.origin, next.rotation(), true)?;
                let solid = o.solid && !(o.name == "openthis" && self.hatch_open);
                let ground = o.collider.trace(
                    player.feet + PLAYER_CENTER,
                    player.feet + PLAYER_CENTER - Vec3::Z * 3.,
                    PLAYER_HALF,
                );
                let riding = solid
                    && player.velocity.z <= 1.
                    && !ground.start_solid
                    && ground.fraction < 1.
                    && ground.normal.z > 0.65;
                world.set_dynamic(
                    fixed
                        .iter()
                        .cloned()
                        .chain(
                            self.objects
                                .iter()
                                .enumerate()
                                .filter(|(j, o)| {
                                    *j != i && o.solid && !(o.name == "openthis" && self.hatch_open)
                                })
                                .map(|(_, o)| o.collider.clone()),
                        )
                        .collect(),
                );
                let mut carried = next.origin
                    + next.rotation() * o.pose.rotation().inverse() * (player.feet - o.pose.origin);
                if solid {
                    if riding {
                        let Some((feet, normal)) = collider.rider_feet(carried) else {
                            continue;
                        };
                        carried = feet;
                        let trace = world.body_trace(player.feet, carried);
                        if trace.start_solid || trace.fraction < 1. || !world.body_clear(carried) {
                            continue;
                        }
                        player.ground_normal = normal;
                    } else if collider.touches(
                        player.feet + PLAYER_CENTER,
                        player.feet + PLAYER_CENTER,
                        PLAYER_HALF,
                    ) {
                        continue;
                    }
                }
                if riding {
                    player.feet = carried;
                    player.grounded = true;
                }
                let o = &mut self.objects[i];
                o.pose = next;
                o.collider = collider;
                o.time += step;
            }
        }
        world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        Ok(())
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    cinema: Option<cinema::State>,
    objects: Vec<(String, Pose, f32)>,
    hatch_open: bool,
    hatch_time: f32,
}
