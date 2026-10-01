//! Scene-local cubic path playback. Native rules researched privately; independent
//! implementation. Immutable time tables make seeking and save/resume repeatable.
use macroquad::prelude::*;

pub(crate) struct Spline {
    points: Vec<(Vec3, Quat, f32)>,
    times: Vec<f64>,
    looping: bool,
}
const SUBDIVISIONS: usize = 128;
const STEP: f64 = 1. / SUBDIVISIONS as f64;

impl Spline {
    pub fn new(points: Vec<(Vec3, Quat, f32)>, looping: bool) -> Self {
        Self::build(points, looping, false)
    }
    pub fn camera_track(points: Vec<(Vec3, Quat, f32)>) -> Self {
        Self::build(points, false, true)
    }
    fn build(points: Vec<(Vec3, Quat, f32)>, looping: bool, camera: bool) -> Self {
        assert!(!points.is_empty());
        let mut s = Self {
            points,
            times: vec![0.],
            looping,
        };
        // The negative parameter lead-in clamps out-of-range control indices.
        // Looping paths then repeat 0..N; open followers end at N-2.
        let steps = (s.points.len() + if looping { 2 } else { usize::from(camera) }) * SUBDIVISIONS;
        for i in 0..steps {
            let p = -2. + i as f64 * STEP;
            let dt = STEP / 6.
                * (s.seconds_per_unit(p)
                    + 4. * s.seconds_per_unit(p + STEP / 2.)
                    + s.seconds_per_unit(p + STEP));
            s.times.push(s.times.last().unwrap() + dt);
        }
        s
    }
    fn basis(&self, parameter: f64) -> [(usize, f32); 4] {
        let base = parameter.floor() as i32;
        let f = (parameter - parameter.floor()) as f32;
        let w = [
            (1. - f).powi(3) / 6.,
            (3. * f.powi(3) - 6. * f * f + 4.) / 6.,
            (-3. * f.powi(3) + 3. * f * f + 3. * f + 1.) / 6.,
            f.powi(3) / 6.,
        ];
        std::array::from_fn(|i| {
            let index = base + i as i32;
            let index = if self.looping && parameter >= 0. {
                index.rem_euclid(self.points.len() as i32)
            } else {
                index.clamp(0, self.points.len() as i32 - 1)
            };
            (index as usize, w[i])
        })
    }
    fn seconds_per_unit(&self, parameter: f64) -> f64 {
        1. / self
            .basis(parameter)
            .iter()
            .map(|(i, w)| self.points[*i].2 as f64 * *w as f64)
            .sum::<f64>()
    }
    pub fn parameter(&self, time: f32) -> f64 {
        let mut t = f64::from(time.max(0.));
        let end = *self.times.last().unwrap();
        if self.looping {
            let lead = self.times[2 * SUBDIVISIONS];
            if t >= lead {
                t = lead + (t - lead).rem_euclid(end - lead);
            }
        } else {
            t = t.min(end);
        }
        let i = self
            .times
            .partition_point(|v| *v <= t)
            .clamp(1, self.times.len() - 1);
        let f = (t - self.times[i - 1]) / (self.times[i] - self.times[i - 1]);
        -2. + (i as f64 - 1. + f) * STEP
    }
    pub fn position_at(&self, parameter: f64) -> Vec3 {
        let base = self.points[0].0;
        base + self
            .basis(parameter)
            .into_iter()
            .map(|(i, w)| (self.points[i].0 - base) * w)
            .sum::<Vec3>()
    }
    pub fn sample(&self, time: f32, travel_angles: bool) -> crate::skeletal::Transform {
        let p = self.parameter(time);
        let translation = self.position_at(p);
        let mut forward = Vec3::ZERO;
        let mut roll = 0.;
        for (i, w) in self.basis(p) {
            let q = self.points[i].1;
            forward += q * Vec3::X * w;
            roll += q.to_euler(EulerRot::ZYX).2 * w;
        }
        if travel_angles {
            let f = (p - p.floor()) as f32;
            let derivative = [
                -(1. - f).powi(2) / 2.,
                1.5 * f * f - 2. * f,
                -1.5 * f * f + f + 0.5,
                f * f / 2.,
            ];
            let base = self.points[self.basis(p)[0].0].0;
            let tangent: Vec3 = self
                .basis(p)
                .iter()
                .zip(derivative)
                .map(|((i, _), w)| (self.points[*i].0 - base) * w)
                .sum();
            forward = if tangent.length_squared() > 1e-8 {
                tangent
            } else {
                self.points
                    .windows(2)
                    .map(|pair| pair[1].0 - pair[0].0)
                    .find(|v| v.length_squared() > 0.001)
                    .unwrap_or(forward)
            };
        }
        let forward = forward.normalize_or_zero();
        let yaw = forward.y.atan2(forward.x);
        let pitch = (-forward.z).atan2(forward.truncate().length());
        crate::skeletal::Transform {
            translation,
            rotation: Quat::from_rotation_z(yaw)
                * Quat::from_rotation_y(pitch)
                * Quat::from_rotation_x(roll),
        }
    }
    pub fn camera(&self, time: f32) -> crate::cinematic::Camera {
        let p = self.sample(time, false);
        crate::cinematic::Camera {
            eye: p.translation,
            target: p.point(Vec3::X * 100.),
            up: p.rotation * Vec3::Z,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cubic_rounds_corners_and_has_the_native_lead_in() {
        let s = Spline::new(
            vec![
                (Vec3::ZERO, Quat::IDENTITY, 1.),
                (Vec3::X * 60., Quat::IDENTITY, 1.),
                (Vec3::Y * 60., Quat::IDENTITY, 1.),
            ],
            false,
        );
        assert!(s.sample(0., false).translation.length() < 0.001);
        assert!((s.parameter(1.) + 1.).abs() < 1e-5);
        assert!(s.sample(1., false).translation.distance(Vec3::X * 10.) < 0.001);
        let a = s.sample(1.999, false).translation;
        let b = s.sample(2., false).translation;
        let c = s.sample(2.001, false).translation;
        assert!((b - a).distance(c - b) < 0.001);
    }
    #[test]
    fn variable_speed_is_blended_and_loop_seek_is_repeatable() {
        let points = vec![
            (Vec3::ZERO, Quat::IDENTITY, 0.2),
            (Vec3::X, Quat::IDENTITY, 1.),
            (Vec3::Y, Quat::IDENTITY, 0.5),
        ];
        let s = Spline::new(points.clone(), true);
        let other = Spline::new(points, true);
        let lead = s.times[2 * SUBDIVISIONS] as f32;
        let period = (*s.times.last().unwrap() - s.times[2 * SUBDIVISIONS]) as f32;
        for t in [0., 1.37, 6.1, 13.2, 90.] {
            assert_eq!(
                s.sample(t, true).translation,
                other.sample(t, true).translation
            );
            if t > lead {
                assert!(
                    s.sample(t, true)
                        .translation
                        .distance(s.sample(t + period, true).translation)
                        < 0.001
                );
            }
        }
        assert!((s.parameter(5.) + 1.).abs() > 0.1);
    }
}
