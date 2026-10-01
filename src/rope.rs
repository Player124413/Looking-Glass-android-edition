//! Flexible presentation for the shared and controller-owned climbing ropes.
//! The player's checked pendulum supplies the grip; the free tail keeps inertia.
use crate::{collision::World, traversal::Rope};
use macroquad::prelude::*;

/// Swept, frictionless contact for a strand particle. Keep the tangential part
/// of both the remaining displacement and its inertia when it touches a wall.
fn slide_particle(world: &World, from: Vec3, to: Vec3, velocity: Vec3) -> (Vec3, Vec3) {
    let half = Vec3::splat(0.7);
    let mut position = from;
    let mut remaining = to - from;
    let mut normals = Vec::new();
    for _ in 0..4 {
        let hit = world.sweep(position, position + remaining, half);
        if hit.start_solid {
            if !world.sweep(to, to, half).start_solid {
                return (to, velocity);
            }
            return (position, Vec3::ZERO);
        }
        position += remaining * hit.fraction;
        if hit.fraction >= 1. {
            break;
        }
        normals.push(hit.normal);
        remaining = crate::movement::constrain(remaining * (1. - hit.fraction), &normals);
        if remaining.length_squared() < 1e-10 {
            break;
        }
    }
    (position, crate::movement::constrain(velocity, &normals))
}

/// Both rope controllers use the same collision response. Resolve climbing
/// separately so an inward swing cannot cancel an otherwise clear ascent.
#[allow(clippy::too_many_arguments)]
pub fn step_grip(
    world: &World,
    anchor: Vec3,
    max_length: f32,
    feet: &mut Vec3,
    length: &mut f32,
    velocity: &mut Vec3,
    input: crate::movement::Controls,
    dt: f32,
) {
    let center = anchor - Vec3::Z * 40.;
    let radial = (*feet - center).normalize_or_zero();
    let wanted = (*length - input.rise * 100. * dt).clamp(32., max_length);
    let end = center + radial * wanted;
    let hit = world.body_trace(*feet, end);
    if !hit.start_solid {
        let next = feet.lerp(end, hit.fraction);
        if world.body_clear(next) {
            *feet = next;
            *length = feet.distance(center);
        }
    }

    *velocity += (input.wish.extend(0.) * 450. - Vec3::Z * 500.) * dt;
    *velocity *= 1. - 0.7 * dt;
    *velocity = velocity.clamp_length_max(500.);
    let direction = (*feet + *velocity * dt - center).normalize_or_zero();
    let mut target = center + direction * *length;
    let mut normals = Vec::new();
    for _ in 0..4 {
        let hit = world.body_trace(*feet, target);
        if hit.start_solid {
            *velocity = Vec3::ZERO;
            return;
        }
        if hit.fraction >= 1. && world.body_clear(target) {
            *feet = target;
            let radial = (*feet - center).normalize_or_zero();
            // Remove radial momentum without reintroducing motion into a wall.
            normals.extend([radial, -radial]);
            *velocity = crate::movement::constrain(*velocity, &normals);
            return;
        }
        normals.push(hit.normal);
        *velocity = crate::movement::constrain(*velocity, &normals);
        let contact = feet.lerp(target, hit.fraction);
        let height = (contact - center).dot(hit.normal);
        if height.abs() >= *length {
            break;
        }
        // The wall cuts the pendulum sphere into a circle. Project onto that
        // circle rather than normalising a slide back inside the same wall.
        let circle = center + hit.normal * height;
        let tangent = target - circle;
        let tangent = tangent - hit.normal * tangent.dot(hit.normal);
        target =
            circle + tangent.normalize_or_zero() * (*length * *length - height * height).sqrt();
    }
}

pub fn grip_facing(anchor: Vec3, feet: Vec3, facing: f32, dt: f32) -> f32 {
    let toward = (anchor - feet).truncate();
    // Directly below the anchor the facing is undefined; numerical drift must
    // not turn Alice through 180 degrees as she passes underneath it.
    if toward.length_squared() < 4. {
        return facing;
    }
    let delta = (toward.y.atan2(toward.x) - facing + std::f32::consts::PI)
        .rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    facing + delta.clamp(-dt * 6., dt * 6.)
}

pub struct Strand {
    pub model: usize,
    pub anchor: Vec3,
    pub length: f32,
    pub points: Vec<Vec3>,
    previous: Vec<Vec3>,
    remaining: f32,
    grip: Option<Vec3>,
}
impl Strand {
    pub fn new(r: &Rope) -> Self {
        let count = (r.length / 16.).ceil().clamp(8., 128.) as usize;
        let points: Vec<_> = (0..=count)
            .map(|i| r.anchor - Vec3::Z * r.length * i as f32 / count as f32)
            .collect();
        Self {
            model: r.model,
            anchor: r.anchor,
            length: r.length,
            previous: points.clone(),
            points,
            remaining: 0.,
            grip: None,
        }
    }
    pub fn update(&mut self, dt: f32, grip: Option<Vec3>, world: &World) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        let count = self.points.len() - 1;
        let spacing = self.length / count as f32;
        if grip.is_some() && self.grip.is_none() {
            let hand = grip.unwrap();
            let length = hand.distance(self.anchor);
            let direction = (hand - self.anchor).normalize_or_zero();
            for (i, p) in self.points.iter_mut().enumerate() {
                let along = spacing * i as f32;
                *p = if along <= length {
                    self.anchor + direction * along
                } else {
                    hand - Vec3::Z * (along - length)
                };
            }
            self.previous.clone_from(&self.points);
        }
        let dt = dt.min(0.1);
        const STEP: f32 = 1. / 120.;
        let mut elapsed = STEP - self.remaining;
        self.remaining += dt;
        while self.remaining + 1e-6 >= STEP {
            self.remaining = (self.remaining - STEP).max(0.);
            // Sample the moving hand at each physics step, rather than applying
            // an entire render frame's climb/swing as one abrupt displacement.
            let hand = grip.map(|p| self.grip.unwrap_or(p).lerp(p, (elapsed / dt).clamp(0., 1.)));
            elapsed += STEP;
            let length = hand.map_or(0., |p| p.distance(self.anchor).min(self.length));
            let direction = hand.map_or(-Vec3::Z, |p| (p - self.anchor).normalize_or_zero());
            let pinned = (length / spacing).floor().min(count as f32) as usize;
            let before = self.points.clone();
            self.points[0] = self.anchor;
            for i in 1..=count {
                let velocity = (self.points[i] - self.previous[i]) * 0.992;
                self.previous[i] = self.points[i];
                if i <= pinned {
                    // The loaded span is taut. Material positions never change
                    // as the hand slides across a segment boundary.
                    self.points[i] = self.anchor + direction * (spacing * i as f32);
                } else {
                    self.points[i] += velocity - Vec3::Z * (500. * STEP * STEP);
                }
            }
            for _ in 0..16 {
                for i in pinned + 1..=count {
                    let first = i == pinned + 1;
                    let from = if first {
                        hand.unwrap_or(self.anchor)
                    } else {
                        self.points[i - 1]
                    };
                    // Only the short span from the hand to the next material
                    // point changes length; every complete segment stays fixed.
                    let rest = if first {
                        spacing * i as f32 - length
                    } else {
                        spacing
                    };
                    let delta = self.points[i] - from;
                    let correction = delta.normalize_or_zero() * (delta.length() - rest);
                    if first {
                        self.points[i] -= correction;
                    } else {
                        self.points[i] -= correction * 0.5;
                        self.points[i - 1] += correction * 0.5;
                    }
                }
            }
            for (i, from) in before.into_iter().enumerate().skip(pinned + 1) {
                let (point, velocity) = slide_particle(
                    world,
                    from,
                    self.points[i],
                    self.points[i] - self.previous[i],
                );
                self.points[i] = point;
                self.previous[i] = point - velocity;
            }
        }
        self.grip = grip;
    }
    pub fn vertices(&self, origin: Vec3, rotation: Quat) -> Vec<Vertex> {
        let mut vertices = Vec::new();
        for (i, &p) in self.points.iter().enumerate() {
            let tangent = (self.points[(i + 1).min(self.points.len() - 1)]
                - self.points[i.saturating_sub(1)])
            .normalize_or_zero();
            let axis = if tangent.x.abs() < 0.8 {
                Vec3::X
            } else {
                Vec3::Y
            };
            let right = tangent.cross(axis).normalize_or_zero();
            let up = tangent.cross(right);
            let along = i as f32 / (self.points.len() - 1) as f32 * self.length;
            for side in 0..=8 {
                let angle = side as f32 / 8. * std::f32::consts::TAU;
                let radius = 1.25 * (1. + 0.1 * (angle * 3. + along * 0.8).sin());
                vertices.push(Vertex {
                    position: rotation.conjugate()
                        * (p + (right * angle.cos() + up * angle.sin()) * radius - origin),
                    uv: vec2(side as f32 / 8. + along / 64., along / 24.),
                    color: [195, 185, 155, 255],
                    normal: Vec4::ZERO,
                });
            }
        }
        vertices
    }
    pub fn indices(&self) -> Vec<u16> {
        let mut out = Vec::new();
        for i in 0..self.points.len() - 1 {
            for s in 0..8 {
                let a = (i * 9 + s) as u16;
                out.extend([a, a + 9, a + 1, a + 1, a + 9, a + 10]);
            }
        }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn particles_slide_along_contacts_and_keep_tangential_inertia() {
        let world = World::fixture(&[
            (vec3(-100., -100., -10.), vec3(100., 100., 0.)),
            (vec3(10., -100., 0.), vec3(20., 100., 100.)),
        ]);
        let (p, v) = slide_particle(
            &world,
            vec3(0., 0., 2.),
            vec3(20., 15., -4.),
            vec3(20., 15., -6.),
        );
        assert!(!world.sweep(p, p, Vec3::splat(0.7)).start_solid);
        assert!(p.y > 14.9 && p.x < 9.31 && p.z > 0.69, "{p:?}");
        assert_eq!(v, vec3(0., 15., 0.));
        let (escaped, _) = slide_particle(&world, p, p + vec3(-5., 0., 5.), vec3(-5., 0., 5.));
        assert!(escaped.x < p.x - 4.9 && escaped.z > p.z + 4.9);
    }

    #[test]
    fn grip_can_climb_slide_and_escape_while_pushing_against_a_wall() {
        let world = World::fixture(&[(vec3(40., -500., -100.), vec3(60., 500., 500.))]);
        let anchor = vec3(0., 0., 320.);
        let mut feet = vec3(24.96, 0., 100.);
        let mut length = (feet + Vec3::Z * 40.).distance(anchor);
        let mut velocity = Vec3::X * 100.;
        let original = feet;
        for _ in 0..90 {
            step_grip(
                &world,
                anchor,
                320.,
                &mut feet,
                &mut length,
                &mut velocity,
                crate::movement::Controls {
                    wish: Vec2::ONE,
                    rise: 1.,
                    ..Default::default()
                },
                1. / 120.,
            );
            assert!(world.body_clear(feet));
            assert!(((feet + Vec3::Z * 40.).distance(anchor) - length).abs() < 0.01);
        }
        assert!(feet.z > original.z + 60. && feet.y > 30., "{feet:?}");
        let before = feet;
        for _ in 0..90 {
            step_grip(
                &world,
                anchor,
                320.,
                &mut feet,
                &mut length,
                &mut velocity,
                crate::movement::Controls {
                    wish: -Vec2::X,
                    ..Default::default()
                },
                1. / 120.,
            );
            assert!(world.body_clear(feet));
        }
        assert!(feet.x < before.x - 20., "{feet:?}");
    }

    #[test]
    fn anchor_crossing_preserves_and_smoothly_changes_facing() {
        assert_eq!(grip_facing(Vec3::Z * 300., Vec3::ZERO, 1.2, 1. / 120.), 1.2);
        let next = grip_facing(Vec3::Z * 300., Vec3::X * 10., 0., 1. / 120.);
        assert!(next.abs() <= 0.051);
    }
    fn fixture() -> Rope {
        Rope {
            id: crate::entity::Id(1),
            model: 1,
            enabled: true,
            origin: Vec3::ZERO,
            anchor: Vec3::Z * 320.,
            length: 320.,
        }
    }
    #[test]
    fn climbing_a_vertical_rope_does_not_shake_its_material_points() {
        let w = World::fixture(&[]);
        let r = fixture();
        for hz in [30, 60, 144] {
            let mut s = Strand::new(&r);
            let rest = s.points.clone();
            let mut error = 0_f32;
            // Descend through every segment boundary, reach the end, then climb
            // back up. Vertical climbing should not stretch/re-space the strand.
            for frame in 0..6 * hz {
                let t = frame as f32 / hz as f32;
                let length = if t <= 3. {
                    32. + t * 96.
                } else {
                    320. - (t - 3.) * 96.
                };
                s.update(1. / hz as f32, Some(r.anchor - Vec3::Z * length), &w);
                for (p, initial) in s.points.iter().zip(&rest) {
                    error = error.max(p.distance(*initial));
                }
            }
            println!("vertical climb {hz} Hz: maximum material drift {error}");
            assert!(
                error < 1.,
                "climbing re-spaced rope material at {hz} Hz: {error}"
            );
        }
    }
    #[test]
    fn climbing_while_swinging_stays_continuous_across_frame_rates() {
        let w = World::fixture(&[]);
        let r = fixture();
        let hand = |t: f32| {
            let angle = 0.35 + 0.1 * (t * 1.2).sin();
            r.anchor + vec3(angle.sin(), 0., -angle.cos()) * (160. + 100. * t.sin())
        };
        let mut reference: Option<Vec<Vec3>> = None;
        for hz in [30, 60, 144] {
            let mut s = Strand::new(&r);
            s.update(1. / 120., Some(hand(0.)), &w);
            let mut peak_speed = 0_f32;
            for frame in 1..=12 * hz {
                let before = s.points.clone();
                s.update(1. / hz as f32, Some(hand(frame as f32 / hz as f32)), &w);
                for (p, old) in s.points.iter().zip(before) {
                    peak_speed = peak_speed.max(p.distance(old) * hz as f32);
                    assert!(p.is_finite());
                }
            }
            println!("climb/swing {hz} Hz: peak material speed {peak_speed}");
            assert!(
                peak_speed < 300.,
                "climbing kicked a material point: {peak_speed}"
            );
            if let Some(points) = &reference {
                let error = s
                    .points
                    .iter()
                    .zip(points)
                    .map(|(a, b)| a.distance(*b))
                    .fold(0_f32, f32::max);
                println!("climb/swing {hz} Hz: final deviation from 30 Hz {error}");
                assert!(
                    error < 2.,
                    "rope motion depends on rendering frequency: {error}"
                );
            } else {
                reference = Some(s.points.clone());
            }
        }
    }
    #[test]
    fn anchor_grip_tail_inertia_and_pause() {
        let w = World::fixture(&[]);
        let r = Rope {
            id: crate::entity::Id(1),
            model: 1,
            enabled: true,
            origin: Vec3::ZERO,
            anchor: Vec3::Z * 320.,
            length: 320.,
        };
        let mut s = Strand::new(&r);
        for i in 0..240 {
            let a = i as f32 / 240. * 0.5;
            let hand = r.anchor + vec3(a.sin(), 0., -a.cos()) * 160.;
            s.update(1. / 120., Some(hand), &w);
            assert_eq!(s.points[0], r.anchor);
            assert!(s.points.iter().all(|p| p.is_finite()));
        }
        let hand = s.points[10];
        let tail = *s.points.last().unwrap();
        assert!(
            (tail - hand)
                .normalize()
                .distance((hand - r.anchor).normalize())
                > 0.1,
            "tail must bend below the grip"
        );
        let paused = s.points.clone();
        s.update(0., None, &w);
        assert_eq!(s.points, paused);
        s.update(1. / 60., None, &w);
        assert!(s.points.last().unwrap().distance(tail) > 0.01);
        assert_eq!(s.points[0], r.anchor);
    }
}
