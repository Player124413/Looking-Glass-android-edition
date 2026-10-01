//! Village vent attachments and authored on/off intervals. Particle velocities
//! retain the original effect's world-space randvel (rising steam).
use super::{sample, Village};
use crate::{assets::Assets, bsp::Bsp, entity::Id, interaction::vector, skeletal::Transform};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;

pub(super) struct Attachment {
    id: Id,
    owner: String,
    offset: Vec3,
    rotation: Quat,
    bound: bool,
    timing: Timing,
}
enum Timing {
    Spin(f32),
    Roof(f32),
    Puff(f32),
}
impl Timing {
    fn active(&self, time: f32) -> bool {
        match *self {
            Self::Spin(delay) => {
                time >= delay && (1.5..3.5).contains(&(time - delay).rem_euclid(5.))
            }
            Self::Roof(wait) => roof(time, wait).1,
            Self::Puff(wait) => {
                time >= wait && (time - wait).rem_euclid(0.5 + wait * 2.) < 0.5 + wait
            }
        }
    }
}

/// A single initial delay, then compression, dwell and release. Steam is
/// visible only during compression; the initial delay is not repeated.
pub(super) fn roof(time: f32, wait: f32) -> (f32, bool) {
    if time < wait {
        return (0., false);
    }
    let t = (time - wait).rem_euclid(wait + 0.5);
    let amount = super::curve(
        t,
        &[(0., 0.), (0.3, 1.), (wait + 0.3, 1.), (wait + 0.5, 0.)],
    );
    (amount, t < 0.3)
}

pub(super) fn attachments(map: &Bsp) -> Result<Vec<Attachment>> {
    let mut out = Vec::new();
    let point = |name: &str| -> Result<Vec3> {
        map.entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|n| n == name))
            .and_then(|e| e.get("origin"))
            .and_then(|s| vector(s))
            .with_context(|| format!("Missing village machinery origin: {name}"))
    };
    for (id, e) in map.entities.iter().enumerate() {
        let Some(name) = e.get("targetname") else {
            continue;
        };
        let def = (1..=3).find_map(|n| {
            if name == &format!("spinrod{n}_smoke") {
                Some((
                    format!("spinrod{n}_arm"),
                    Timing::Spin((n - 1) as f32 * 2.),
                    true,
                    0.,
                ))
            } else if name == &format!("mushroom_steam{n}") {
                Some((
                    format!("mushroom_roof{n}"),
                    Timing::Roof([2., 3., 1.][n - 1]),
                    true,
                    -4.,
                ))
            } else if n <= 2 && name == &format!("puff_ball{n}_smoke") {
                Some((
                    format!("puff_ball{n}"),
                    Timing::Puff([0.5, 2.][n - 1]),
                    false,
                    0.,
                ))
            } else {
                None
            }
        });
        let Some((owner, timing, bound, z)) = def else {
            continue;
        };
        let angles = e
            .get("angles")
            .and_then(|s| vector(s))
            .unwrap_or(Vec3::ZERO);
        out.push(Attachment {
            id: Id(id),
            offset: point(name)? - if bound { point(&owner)? } else { Vec3::ZERO } + Vec3::Z * z,
            rotation: Quat::from_rotation_z(angles.y.to_radians())
                * Quat::from_rotation_y(-angles.x.to_radians())
                * Quat::from_rotation_x(angles.z.to_radians()),
            owner,
            timing,
            bound,
        });
    }
    Ok(out)
}

impl Village {
    fn emissions(&self) -> Vec<(Id, Transform, bool)> {
        self.emitters
            .iter()
            .filter_map(|e| {
                let o = self.objects.iter().find(|o| o.name == e.owner)?;
                let pose = if e.bound {
                    Transform {
                        translation: o.pose.origin + o.pose.rotation() * e.offset,
                        rotation: o.pose.rotation() * e.rotation,
                    }
                } else {
                    Transform {
                        translation: e.offset,
                        rotation: e.rotation,
                    }
                };
                Some((e.id, pose, e.timing.active(o.time)))
            })
            .collect()
    }
    pub fn place_particles(&self, steam: &mut crate::particles::Steam) {
        let emissions = self.emissions();
        steam.place(
            &emissions
                .iter()
                .map(|(id, pose, _)| (*id, *pose))
                .collect::<Vec<_>>(),
        );
        steam.gate(
            &emissions
                .iter()
                .map(|(id, _, active)| (*id, *active))
                .collect::<Vec<_>>(),
        );
    }
}

pub(super) fn check(assets: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&assets.read("maps/gvillage.bsp")?)?;
    let mut v = Village::load(assets, &map)?;
    ensure!(
        v.emitters.len() == 8,
        "Village emitter ownership is incomplete"
    );
    for time in [
        0., 0.4, 1., 1.6, 1.85, 2.1, 2.35, 2.6, 3.4, 4.8, 5.1, 6.1, 10.2, 39.85,
    ] {
        for o in &mut v.objects {
            o.time = time;
            o.pose = sample(&o.name, o.base, time, 0.);
        }
        for n in 1..=3 {
            let base = v
                .objects
                .iter()
                .find(|o| o.name == format!("spinrod{n}_base"))
                .unwrap();
            let arm = v
                .objects
                .iter()
                .find(|o| o.name == format!("spinrod{n}_arm"))
                .unwrap();
            ensure!(
                (base.pose.origin + base.pose.rotation() * (arm.base - base.base))
                    .distance(arm.pose.origin)
                    < 0.002,
                "Village vent detached from shaft"
            );
            let a = &v.emitters.iter().find(|a| a.owner == arm.name).unwrap();
            let (_, smoke, active) = v
                .emissions()
                .into_iter()
                .find(|(id, _, _)| *id == a.id)
                .unwrap();
            ensure!(
                (arm.pose.rotation().inverse() * (smoke.translation - arm.pose.origin))
                    .distance(a.offset)
                    < 0.002,
                "Steam detached from nozzle"
            );
            if active {
                // During its two full turns the bent nozzle describes a circle
                // around the shaft, with a constant axial position and radius.
                let local = base.pose.rotation().inverse() * (smoke.translation - arm.pose.origin);
                ensure!(
                    local.x.abs() < 1. && (local.yz().length() - a.offset.length()).abs() < 0.05,
                    "Spinning nozzle has the wrong bend or axle"
                );
            }
        }
        let before = v.transforms().collect::<Vec<_>>();
        let emissions = v.emissions();
        let bytes = serde_json::to_vec(&v.snapshot())?;
        let mut restored = Village::load(assets, &map)?;
        restored.restore(&serde_json::from_slice(&bytes)?, &map)?;
        ensure!(
            before == restored.transforms().collect::<Vec<_>>(),
            "Saved machinery pose changed"
        );
        for (a, b) in emissions.iter().zip(restored.emissions()) {
            ensure!(
                a.0 == b.0
                    && a.1.translation == b.1.translation
                    && a.1.rotation == b.1.rotation
                    && a.2 == b.2,
                "Saved steam attachment/phase changed"
            );
        }
    }
    println!("PASS village machinery: 3 shaft/arm joints, 8 emitter placements/intervals, 14 phases and saved restoration");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vent_turns_around_its_shaft_and_returns_without_a_snap() {
        for delay in [0., 2., 4.] {
            let name = format!("spinrod{}_arm", delay as usize / 2 + 1);
            for (elapsed, expected) in [(1.5, -Vec3::Z), (2., Vec3::Z), (2.5, -Vec3::Z)] {
                let p = sample(&name, Vec3::ZERO, delay + elapsed, 0.);
                assert!(
                    (p.rotation() * -Vec3::X).distance(expected) < 0.0001,
                    "{name} at {elapsed}: angles {:?}, tip {:?}, expected {expected:?}",
                    p.angles,
                    p.rotation() * -Vec3::X
                );
                assert_eq!(p.origin, Vec3::ZERO);
            }
            for boundary in [0.5, 1.5, 3.5, 4.5, 5.] {
                let a = sample(&name, Vec3::ZERO, delay + boundary - 0.00001, 0.);
                let b = sample(&name, Vec3::ZERO, delay + boundary + 0.00001, 0.);
                assert!(a.origin.distance(b.origin) < 0.01);
                for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                    assert!((a.rotation() * axis).distance(b.rotation() * axis) < 0.001);
                }
            }
        }
    }
    #[test]
    fn steam_intervals_match_each_mechanism() {
        assert!(!Timing::Spin(2.).active(1.9));
        for t in [3.5, 4.5, 5.49] {
            assert!(Timing::Spin(2.).active(t));
        }
        for t in [2., 3.49, 5.5, 6.9] {
            assert!(!Timing::Spin(2.).active(t));
        }
        assert_eq!(roof(1.9, 2.), (0., false));
        assert!((roof(2.15, 2.).0 - 0.5).abs() < 0.0001);
        assert!(roof(2.15, 2.).1);
        assert_eq!(roof(3., 2.), (1., false));
        assert!((roof(4.4, 2.).0 - 0.5).abs() < 0.0001);
        assert!(roof(4.6, 2.).1);
        assert!(!Timing::Puff(0.5).active(0.4));
        assert!(Timing::Puff(0.5).active(1.4));
        assert!(!Timing::Puff(0.5).active(1.6));
    }
    #[test]
    fn sawmill_tilt_tracks_the_supporting_beam() {
        let left = sample("sawmill_slat1", vec3(-64., 0., 244.), 0., 0.);
        let right = sample("sawmill_slat5", vec3(64., 0., 244.), 0., 0.);
        let beam = sample("sawmill_beam", Vec3::ZERO, 0., 0.);
        assert!(left.origin.z > right.origin.z);
        assert!((beam.rotation() * -Vec3::X).z > 0.);
        assert!((sample("sawmill_beam", Vec3::ZERO, 1., 0.).rotation() * -Vec3::X).z < 0.);
    }
}
