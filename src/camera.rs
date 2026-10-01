//! Responsive orbit with collision-safe retraction and a damped return from obstacles.
use crate::collision::World;
use macroquad::prelude::*;

mod handoff;
pub use handoff::Handoff;

const HALF: Vec3 = Vec3::splat(4.);
const RELEASE_DELAY: f32 = 0.12;

#[derive(Default)]
pub struct Follow {
    previous_feet: Option<Vec3>,
    height: f32,
    arm: f32,
    release: f32,
    visible: bool,
}

impl Follow {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn update(
        &mut self,
        world: &World,
        feet: Vec3,
        direction: Vec3,
        distance: f32,
        dt: f32,
    ) -> (Vec3, Vec3, bool) {
        let dt = dt.clamp(0., 0.05);
        let fresh = self
            .previous_feet
            .is_none_or(|last| last.distance(feet) > 128.);
        self.previous_feet = Some(feet);
        let pivot = feet + Vec3::Z * 36.;
        // Filter small steps/landings vertically, without lagging mouse aim or strafing.
        self.height = if fresh {
            pivot.z
        } else {
            (self.height + (pivot.z - self.height) * (1. - (-18. * dt).exp()))
                .clamp(pivot.z - 12., pivot.z + 12.)
        };
        let mut target = vec3(pivot.x, pivot.y, self.height);
        let anchor_hit = world.camera_sweep(pivot, target, HALF);
        target = pivot.lerp(
            target,
            if anchor_hit.start_solid {
                0.
            } else {
                anchor_hit.fraction
            },
        );
        let shoulder = vec3(direction.y, -direction.x, 0.).normalize_or_zero() * 24.;
        let offset = -direction * distance.clamp(0., 1024.) + Vec3::Z * 12. + shoulder;
        self.resolve(world, target, offset, dt, fresh, f32::INFINITY)
    }

    /// Preserve the rail camera's framing, but keep its arm out of tunnel walls.
    pub fn rail(
        &mut self,
        world: &World,
        camera: crate::cinematic::Camera,
        ahead: &[crate::cinematic::Camera],
        dt: f32,
    ) -> crate::cinematic::Camera {
        let fresh = self
            .previous_feet
            .is_none_or(|p| p.distance(camera.target) > 128.);
        self.previous_feet = Some(camera.target);
        let length = camera.eye.distance(camera.target);
        let anticipated = ahead.iter().fold(length, |limit, next| {
            let hit = world.camera_sweep(next.target, next.eye, HALF);
            // An embedded future anchor cannot supply a useful arm constraint.
            if hit.start_solid {
                limit
            } else {
                limit.min(length * hit.fraction)
            }
        });
        let (eye, target, _) = self.resolve(
            world,
            camera.target,
            camera.eye - camera.target,
            dt.clamp(0., 0.05),
            fresh,
            anticipated,
        );
        crate::cinematic::Camera::look(eye, target)
    }

    fn resolve(
        &mut self,
        world: &World,
        target: Vec3,
        offset: Vec3,
        dt: f32,
        fresh: bool,
        anticipated: f32,
    ) -> (Vec3, Vec3, bool) {
        let length = offset.length();
        let hit = world.camera_sweep(target, target + offset, HALF);
        let limit = length * if hit.start_solid { 0. } else { hit.fraction };
        let preferred = limit.min(anticipated);
        if fresh {
            self.arm = preferred;
            self.release = if limit < length { RELEASE_DELAY } else { 0. };
        } else if limit < self.arm || (limit < length && limit <= self.arm + 0.01) {
            // Safety wins over smoothing when a wall or moving object closes in.
            self.arm = limit;
            self.release = RELEASE_DELAY;
        } else if preferred < self.arm {
            // The known rail route gives time to ease inward before the hard stop.
            self.arm += (preferred - self.arm) * (1. - (-12. * dt).exp());
            self.release = RELEASE_DELAY;
        } else {
            let ease_dt = (dt - self.release).max(0.);
            self.release = (self.release - dt).max(0.);
            self.arm += (preferred - self.arm) * (1. - (-8. * ease_dt).exp());
        }
        let eye = target + offset * (self.arm / length.max(0.001));
        // Hysteresis avoids flickering Alice on/off as the camera grazes an edge.
        self.visible = self.arm > if self.visible { 30. } else { 42. };
        let look_at = if self.arm < 0.1 {
            eye - offset.normalize_or_zero()
        } else {
            target
        };
        (eye, look_at, self.visible)
    }
}

/// Exercise real map corners, doorways and moving-model collision from supported feet.
pub fn check(assets: &mut crate::assets::Assets) -> anyhow::Result<()> {
    use crate::{bsp::Bsp, interaction, movement::Player};
    use anyhow::ensure;
    for name in ["gvillage", "skool1", "skool2"] {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let mut world = World::from_bsp(&map)?;
        let mut interactions = interaction::Interactions::load(&map)?;
        interactions.set_entry(assets, &map, name, None)?;
        interactions.sync(&mut world);
        let mut points = vec![interaction::spawn(&map, None).0];
        points.extend(
            map.entities
                .iter()
                .filter_map(|e| e.get("origin").and_then(|s| interaction::vector(s))),
        );
        let mut checked = 0;
        let mut blocked = 0;
        for point in points {
            let Some(player) = Player::spawn(&world, point) else {
                continue;
            };
            if !world.body_clear(player.feet) {
                continue;
            }
            let mut camera = Follow::default();
            for frame in 0..360 {
                let yaw = frame as f32 * std::f32::consts::TAU / 360.;
                let pitch = (yaw * 2.).sin() * 0.7;
                let direction = vec3(
                    yaw.cos() * pitch.cos(),
                    yaw.sin() * pitch.cos(),
                    pitch.sin(),
                );
                let (eye, _, _) = camera.update(&world, player.feet, direction, 220., 1. / 60.);
                let pivot = player.feet + Vec3::Z * 36.;
                let sight = world.camera_sweep(pivot, eye, HALF * 0.99);
                ensure!(
                    !world.camera_sweep(eye, eye, HALF * 0.99).start_solid
                        && !sight.start_solid
                        && sight.fraction > 0.999,
                    "Camera crossed collision in {name}, feet={:?}, yaw={yaw}",
                    player.feet
                );
                blocked += usize::from(camera.arm < 200.);
            }
            checked += 1;
            if checked == 48 {
                break;
            }
        }
        ensure!(
            checked >= 8 && blocked > 0,
            "Insufficient camera collision coverage in {name}"
        );
        println!("PASS camera {name}: {checked} supported positions, {} orbit samples, {blocked} shortened arms, clear camera volumes and sightlines", checked * 360);
    }
    crate::pandemonium::cinema::camera_check(assets)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wall(x: f32) -> World {
        World::fixture(&[(vec3(x - 10., -200., -100.), vec3(x, 200., 200.))])
    }
    fn safe(world: &World, eye: Vec3, target: Vec3) {
        assert!(!world.camera_sweep(eye, eye, HALF * 0.99).start_solid);
        let sight = world.camera_sweep(target, eye, HALF * 0.99);
        assert!(!sight.start_solid && sight.fraction > 0.999);
    }

    #[test]
    fn wall_retraction_is_safe_without_the_old_overhead_lift() {
        let world = wall(-22.);
        let mut camera = Follow::default();
        let (eye, target, visible) = camera.update(&world, Vec3::ZERO, Vec3::X, 132., 1. / 60.);
        safe(&world, eye, target);
        assert!(!visible);
        assert!(
            eye.z - target.z < 3.,
            "A close wall must not swing the camera overhead"
        );
        assert!(eye.x > -18.);
    }

    #[test]
    fn intermittent_obstacles_cannot_pump_camera_back_and_forth() {
        let empty = World::fixture(&[]);
        let wall = wall(-50.);
        let mut camera = Follow::default();
        camera.update(&empty, Vec3::ZERO, Vec3::X, 132., 1. / 60.);
        let (near, target, _) = camera.update(&wall, Vec3::ZERO, Vec3::X, 132., 1. / 60.);
        safe(&wall, near, target);
        for i in 0..30 {
            let w = if i % 2 == 0 { &empty } else { &wall };
            let (eye, target, _) = camera.update(w, Vec3::ZERO, Vec3::X, 132., 1. / 60.);
            safe(w, eye, target);
            assert!(eye.distance(near) < 9.);
        }
        let mut previous = near;
        for _ in 0..90 {
            let (eye, target, _) = camera.update(&empty, Vec3::ZERO, Vec3::X, 132., 1. / 60.);
            safe(&empty, eye, target);
            assert!(eye.distance(previous) < 12.);
            previous = eye;
        }
        assert!(camera.arm > 132.);
    }

    #[test]
    fn orbit_around_a_corner_always_keeps_a_clear_sightline() {
        let world = World::fixture(&[
            (vec3(-80., -300., -40.), vec3(-70., 70., 200.)),
            (vec3(-300., 70., -40.), vec3(70., 80., 200.)),
        ]);
        let mut camera = Follow::default();
        for i in 0..1440 {
            let angle = i as f32 * 0.013;
            let direction = vec3(angle.cos(), angle.sin(), 0.);
            let (eye, target, _) = camera.update(&world, Vec3::ZERO, direction, 220., 1. / 120.);
            safe(&world, eye, target);
        }
    }

    #[test]
    fn steps_pause_teleports_and_frame_rates_remain_stable() {
        let empty = World::fixture(&[]);
        let wall = wall(-50.);
        let mut ends = Vec::new();
        for hz in [30, 60, 144] {
            let mut camera = Follow::default();
            camera.update(&wall, Vec3::ZERO, Vec3::X, 132., 0.);
            for _ in 0..hz {
                camera.update(&empty, Vec3::ZERO, Vec3::X, 132., 1. / hz as f32);
            }
            ends.push(camera.arm);
        }
        assert!(ends.windows(2).all(|p| (p[0] - p[1]).abs() < 0.001));
        let mut camera = Follow::default();
        camera.update(&empty, Vec3::ZERO, Vec3::X, 132., 0.);
        let (eye, target, _) = camera.update(&empty, Vec3::Z * 18., Vec3::X, 132., 1. / 60.);
        assert!((42. ..54.).contains(&target.z));
        assert_eq!(
            camera.update(&empty, Vec3::Z * 18., Vec3::X, 132., 0.).0,
            eye
        );
        let (_, target, _) = camera.update(&empty, Vec3::Z * 1000., Vec3::X, 132., 1. / 60.);
        assert_eq!(target.z, 1036.);
        camera.reset();
        let (_, target, _) = camera.update(&empty, Vec3::Z * 18., Vec3::X, 132., 0.);
        assert_eq!(target.z, 54.);
    }
}
