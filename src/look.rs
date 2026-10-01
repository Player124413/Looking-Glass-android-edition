//! Relative mouse-look; movement is measured in counts, never scaled by frame time.
use macroquad::prelude::*;

const SENSITIVITY: f32 = 0.0025;

#[derive(Default)]
pub struct MouseLook {
    grabbed: bool,
    previous: Option<Vec2>,
    pub sensitivity: f32,
    pub inverted: bool,
}
impl MouseLook {
    pub fn update(&mut self, capture: bool, yaw: &mut f32, pitch: &mut f32) {
        if self.grabbed != capture {
            set_cursor_grab(capture);
            show_mouse(!capture);
            self.grabbed = capture;
            self.previous = None;
        }
        // Macroquad accumulates raw motion while grabbed. Undo its logical-pixel DPI
        // conversion so sensitivity stays independent of window size and display scale.
        let (x, y) = mouse_position();
        let position = vec2(x, y) * miniquad::window::dpi_scale();
        let delta = sample_motion(&mut self.previous, position, capture);
        let sensitivity = if self.sensitivity > 0. {
            self.sensitivity
        } else {
            1.
        };
        turn(
            vec2(delta.x, delta.y * if self.inverted { -1. } else { 1. }) * sensitivity,
            yaw,
            pitch,
        );
    }
    pub fn release(&mut self) {
        if self.grabbed {
            set_cursor_grab(false);
            show_mouse(true);
            self.grabbed = false;
        }
        self.previous = None;
    }
}
impl Drop for MouseLook {
    fn drop(&mut self) {
        self.release();
    }
}

fn sample_motion(previous: &mut Option<Vec2>, position: Vec2, capture: bool) -> Vec2 {
    if !capture || !position.is_finite() {
        *previous = None;
        return Vec2::ZERO;
    }
    previous
        .replace(position)
        .map_or(Vec2::ZERO, |last| position - last)
}
fn turn(delta: Vec2, yaw: &mut f32, pitch: &mut f32) {
    *yaw = (*yaw - delta.x * SENSITIVITY).rem_euclid(std::f32::consts::TAU);
    *pitch = (*pitch - delta.y * SENSITIVITY).clamp(-1.5, 1.5);
}

// Miniquad 0.4.8 explicitly does not release its desktop grab on focus loss, and
// Macroquad has no public focus query. Compare only this UI thread's active window.
#[cfg(target_os = "windows")]
pub fn window_focused() -> bool {
    #[link(name = "user32")]
    extern "system" {
        fn GetActiveWindow() -> *mut std::ffi::c_void;
        fn GetForegroundWindow() -> *mut std::ffi::c_void;
    }
    // Both functions return opaque handles; neither handle is dereferenced.
    unsafe {
        let own = GetActiveWindow();
        !own.is_null() && own == GetForegroundWindow()
    }
}
#[cfg(not(target_os = "windows"))]
pub fn window_focused() -> bool {
    true
} // Other desktop platforms are not verified yet.

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_and_recapture_ignore_absolute_cursor_jumps() {
        let mut previous = None;
        assert_eq!(
            sample_motion(&mut previous, vec2(600., 300.), true),
            Vec2::ZERO
        );
        assert_eq!(
            sample_motion(&mut previous, vec2(620., 290.), true),
            vec2(20., -10.)
        );
        assert_eq!(
            sample_motion(&mut previous, vec2(10., 10.), false),
            Vec2::ZERO
        );
        assert_eq!(
            sample_motion(&mut previous, vec2(1000., 700.), true),
            Vec2::ZERO
        );
        assert_eq!(
            sample_motion(&mut previous, vec2(1002., 697.), true),
            vec2(2., -3.)
        );
    }
    #[test]
    fn right_and_up_motion_turn_right_and_up_without_flipping() {
        let (mut yaw, mut pitch) = (1., 0.);
        turn(vec2(100., -50.), &mut yaw, &mut pitch);
        assert!((yaw - 0.75).abs() < 1e-6 && (pitch - 0.125).abs() < 1e-6);
        turn(vec2(0., -100000.), &mut yaw, &mut pitch);
        assert_eq!(pitch, 1.5);
        turn(vec2(0., 100000.), &mut yaw, &mut pitch);
        assert_eq!(pitch, -1.5);
    }
    #[test]
    fn equal_mouse_distance_has_equal_turn_at_different_frame_rates() {
        let mut angles = Vec::new();
        for frames in [30, 60, 144] {
            let (mut yaw, mut pitch) = (3., 0.);
            for _ in 0..frames {
                turn(vec2(300., 80.) / frames as f32, &mut yaw, &mut pitch);
            }
            angles.push(vec2(yaw, pitch));
        }
        assert!(angles
            .windows(2)
            .all(|pair| pair[0].distance(pair[1]) < 0.0001));
    }
}
