//! Presentation-only return to gameplay; authored cuts inside scenes stay exact.
use crate::{cinematic::Camera, collision::World};
use macroquad::prelude::*;

struct Return {
    from: Camera,
    previous_eye: Vec3,
    time: f32,
    cover: Color,
    cut: bool,
    duration: f32,
}

fn blend(from: Camera, to: Camera, f: f32) -> Camera {
    if f <= 0. {
        return from;
    }
    if f >= 1. {
        return to;
    }
    // Orbit around Alice instead of taking a straight shortcut through her body.
    // Keep height independent of yaw so a half-turn cannot dip below the floor.
    let a = from.eye - to.target;
    let b = to.eye - to.target;
    let start = a.y.atan2(a.x);
    let turn = (b.y.atan2(b.x) - start + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    let angle = start + turn * f;
    let radius = a.truncate().length() * (1. - f) + b.truncate().length() * f;
    let eye = to.target
        + vec3(
            angle.cos() * radius,
            angle.sin() * radius,
            a.z * (1. - f) + b.z * f,
        );
    let from_aim = from.eye + (from.target - from.eye).normalize_or_zero() * a.length().max(1.);
    Camera::look(eye, from_aim.lerp(to.target, f))
}

fn clear_return(world: &World, from: Camera, to: Camera) -> bool {
    let mut previous = from.eye;
    for step in 1..=16 {
        let next = blend(from, to, step as f32 / 16.).eye;
        let trace = world.camera_sweep(previous, next, super::HALF);
        if trace.start_solid || trace.fraction < 1. {
            return false;
        }
        previous = next;
    }
    true
}

#[derive(Default)]
pub struct Handoff {
    last: Option<(Camera, Color)>,
    returning: Option<Return>,
    overlay: Color,
    return_seconds: f32,
}

impl Handoff {
    /// Set while an authored shot is active; its return retains this duration.
    pub fn return_duration(&mut self, seconds: f32) {
        self.return_seconds = if seconds.is_finite() {
            seconds.clamp(0.1, 10.)
        } else {
            0.45
        };
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn was_scripted(&self) -> bool {
        self.last.is_some()
    }
    pub fn skip(&mut self) {
        if let Some((_, fade)) = &mut self.last {
            *fade = BLACK;
        }
    }
    pub fn overlay(&self) -> Color {
        self.overlay
    }
    pub fn update(
        &mut self,
        world: &World,
        scripted: Option<Camera>,
        gameplay: Camera,
        fade: Color,
        dt: f32,
    ) -> Camera {
        self.overlay = Color::new(0., 0., 0., 0.);
        if let Some(camera) = scripted {
            self.last = Some((camera, fade));
            self.returning = None;
            return camera;
        }
        if let Some((from, cover)) = self.last.take() {
            let cut = cover.a >= 0.9
                || from.eye.distance(gameplay.eye) > 480.
                || !clear_return(world, from, gameplay);
            self.returning = Some(Return {
                from,
                previous_eye: from.eye,
                time: 0.,
                cover: if cut && cover.a < 0.9 { BLACK } else { cover },
                cut,
                duration: if self.return_seconds > 0. {
                    self.return_seconds
                } else {
                    0.45
                },
            });
        } else if let Some(r) = &mut self.returning {
            r.time += dt.clamp(0., 0.05);
        }
        let Some(r) = &mut self.returning else {
            return gameplay;
        };
        let duration = if r.cut { 0.25 } else { r.duration };
        let t = (r.time / duration).clamp(0., 1.);
        let f = t * t * (3. - 2. * t);
        let camera = if r.cut {
            gameplay
        } else {
            let view = blend(r.from, gameplay, f);
            let trace = world.camera_sweep(r.previous_eye, view.eye, super::HALF);
            if trace.start_solid || trace.fraction < 1. {
                // A moving obstacle can invalidate the original clear segment.
                // Hide the cut instead of easing a camera through its surface.
                r.cut = true;
                r.cover = BLACK;
                r.time = 0.;
                gameplay
            } else {
                r.previous_eye = view.eye;
                view
            }
        };
        self.overlay = Color {
            a: r.cover.a * (1. - if r.time == 0. { 0. } else { f }),
            ..r.cover
        };
        if r.time >= duration {
            self.returning = None;
            return gameplay;
        }
        camera
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const CLEAR: Color = Color::new(0., 0., 0., 0.);
    fn shot(x: f32) -> Camera {
        Camera::look(vec3(x, 0., 80.), vec3(0., 0., 30.))
    }

    #[test]
    fn authored_two_second_return_releases_gameplay_after_the_blend() {
        let world = World::fixture(&[]);
        let mut h = Handoff::default();
        let start = shot(-80.);
        let end = shot(-100.);
        h.return_duration(2.);
        h.update(&world, Some(start), end, CLEAR, 0.);
        h.update(&world, None, end, CLEAR, 0.);
        for _ in 0..20 {
            h.update(&world, None, end, CLEAR, 0.05);
        }
        assert!(h.returning.is_some());
        for _ in 0..21 {
            h.update(&world, None, end, CLEAR, 0.05);
        }
        assert!(h.returning.is_none());
    }
    #[test]
    fn local_return_starts_at_last_shot_and_finishes_without_frame_rate_lag() {
        let world = World::fixture(&[]);
        for hz in [30, 60, 144] {
            let mut h = Handoff::default();
            let start = shot(-180.);
            let end = shot(-80.);
            h.update(&world, Some(start), end, CLEAR, 0.);
            assert_eq!(h.update(&world, None, end, CLEAR, 0.).eye, start.eye);
            let mut last = start.eye;
            for _ in 0..hz {
                let c = h.update(&world, None, end, CLEAR, 1. / hz as f32);
                assert!(c.eye.distance(last) < 13.);
                assert!(c.eye.x >= last.x);
                assert!((c.target - c.eye).length() > 0.9);
                last = c.eye;
            }
            assert_eq!(last, end.eye);
            assert_eq!(h.overlay().a, 0.);
        }
    }

    #[test]
    fn returning_from_the_front_orbits_alice_instead_of_crossing_her_body() {
        let from = shot(120.);
        let to = shot(-120.);
        for i in 0..=60 {
            let c = blend(from, to, i as f32 / 60.);
            assert!((c.eye - to.target).truncate().length() > 119.);
            assert!((c.eye.z - 80.).abs() < 0.001);
            assert!(
                (c.target - c.eye)
                    .normalize()
                    .dot((to.target - c.eye).normalize())
                    > 0.999
            );
        }
    }

    #[test]
    fn authored_cuts_pause_and_reload_do_not_inherit_a_return() {
        let world = World::fixture(&[]);
        let mut h = Handoff::default();
        let end = shot(-80.);
        for start in [shot(-180.), shot(800.), shot(100.)] {
            assert_eq!(
                h.update(&world, Some(start), end, CLEAR, 0.1).eye,
                start.eye
            );
        }
        let a = h.update(&world, None, end, CLEAR, 0.);
        for _ in 0..20 {
            assert_eq!(h.update(&world, None, end, CLEAR, 0.).eye, a.eye);
        }
        h.reset();
        assert_eq!(h.update(&world, None, end, CLEAR, 0.).eye, end.eye);
        assert_eq!(h.overlay().a, 0.);
    }

    #[test]
    fn remote_blocked_skipped_and_already_faded_shots_use_a_covered_cut() {
        let empty = World::fixture(&[]);
        let wall = World::fixture(&[(vec3(-140., -30., 0.), vec3(-130., 30., 150.))]);
        for (world, start, fade, skip) in [
            (&empty, shot(-800.), CLEAR, false),
            (&wall, shot(-180.), CLEAR, false),
            (&empty, shot(-180.), WHITE, false),
            (&empty, shot(-180.), CLEAR, true),
        ] {
            let mut h = Handoff::default();
            let end = shot(-80.);
            h.update(world, Some(start), end, fade, 0.);
            if skip {
                h.skip();
            }
            assert_eq!(h.update(world, None, end, CLEAR, 0.).eye, end.eye);
            assert_eq!(h.overlay().a, 1.);
            for _ in 0..30 {
                h.update(world, None, end, CLEAR, 1. / 60.);
            }
            assert_eq!(h.overlay().a, 0.);
        }
    }

    #[test]
    fn obstacle_appearing_during_return_covers_the_cut_then_reveals_gradually() {
        let empty = World::fixture(&[]);
        let wall = World::fixture(&[(vec3(-90., -30., 0.), vec3(-75., 30., 150.))]);
        let mut h = Handoff::default();
        let end = shot(-80.);
        h.update(&empty, Some(shot(-180.)), end, CLEAR, 0.);
        h.update(&empty, None, end, CLEAR, 0.);
        for _ in 0..8 {
            h.update(&empty, None, end, CLEAR, 0.05);
        }
        h.update(&wall, None, end, CLEAR, 0.05);
        assert_eq!(h.overlay().a, 1.);
        h.update(&wall, None, end, CLEAR, 0.05);
        assert!(h.overlay().a > 0.8 && h.overlay().a < 1.);
    }
}
