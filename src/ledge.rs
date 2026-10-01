//! Geometry-driven airborne catches. All motion uses the ordinary swept player body.
use crate::{
    collision::World,
    movement::{Controls, FIXED_DT},
    water::Immersion,
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

pub const PULL_TIME: f32 = 2.35;
const HAND_HEIGHT: f32 = 60.875;
const SHIMMY_SPEED: f32 = 11.63 / 0.8;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hang {
    pub start: Vec3,
    pub top: f32,
    pub forward: Vec2,
    pub elapsed: f32,
    pub pulling: bool,
    pub side: f32,
    forward_held: bool,
}

fn clear(world: &World, from: Vec3, to: Vec3) -> bool {
    let t = world.body_trace(from, to);
    !t.start_solid && !t.all_solid && t.fraction >= 1. && world.body_clear(to)
}

/// Require both hands on a near-horizontal static lip. Dynamic bodies still
/// participate in all clearance sweeps, but cannot become a hanging support.
fn contact(world: &World, feet: Vec3, direction: Vec2) -> Option<f32> {
    let f = direction.extend(0.);
    let side = vec3(-direction.y, direction.x, 0.);
    let wall = world.body_trace(feet, feet + f * 16.);
    if wall.start_solid || wall.fraction >= 1. || wall.normal.z.abs() > 0.1 {
        return None;
    }
    let ceiling = world.ledge_trace(feet + Vec3::Z * 4., feet + Vec3::Z * 212., vec3(4., 4., 4.));
    if ceiling.start_solid {
        return None;
    }
    let high = feet.z + 208. * ceiling.fraction;
    let mut heights = [0.; 2];
    // The upright player's square footprint reaches farther toward a diagonal
    // wall. Keep both hand probes at its front edge, as on an axis-aligned lip.
    let diagonal = crate::collision::PLAYER_HALF
        .truncate()
        .dot(direction.abs())
        - 15.;
    for (i, (reach, lateral)) in [(14.931, -8.244), (15.160, 7.733)].into_iter().enumerate() {
        let mut top = None;
        for extra in [0., 8., 16.] {
            let hand = feet + f * (reach + diagonal + extra) + side * lateral;
            let from = vec3(hand.x, hand.y, high + 4.);
            let to = vec3(hand.x, hand.y, feet.z + 4.);
            let hit = world.ledge_trace(from, to, Vec3::splat(4.));
            if hit.start_solid || hit.fraction >= 1. {
                return None;
            }
            if hit.normal.z >= 0.9 {
                top = Some(from.lerp(to, hit.fraction).z - 4.);
                break;
            }
            // A narrow bevel can precede the horizontal cap (the Tower pipe
            // rims). Reach across that connected slope, never across empty
            // space or a vertical wall. Both hands still need a flat lip and
            // the complete pull-up still sweeps the player's full body.
            if !(0.1..0.9).contains(&hit.normal.z) {
                return None;
            }
        }
        heights[i] = top?;
    }
    (heights[0] - heights[1])
        .abs()
        .le(&5.)
        .then_some(heights[0].max(heights[1]))
}

impl Hang {
    pub fn catch(
        world: &World,
        before: Vec3,
        feet: Vec3,
        velocity: Vec3,
        direction: Vec2,
        input: Controls,
    ) -> Option<Self> {
        if velocity.z < -500. || direction.length_squared() < 0.1 {
            return None;
        }
        let mut forward = direction.normalize();
        let wall = world.body_trace(feet, feet + forward.extend(0.) * 16.);
        if !wall.start_solid && wall.fraction < 1. && wall.normal.z.abs() <= 0.1 {
            // Square both hands to the contacted face, including an oblique
            // approach. The full-body and two-hand clearance checks still apply.
            forward = -wall.normal.truncate().normalize();
        }
        let top = contact(world, feet, forward)?;
        let low = before.z.min(feet.z) + HAND_HEIGHT - 5.;
        let high = before.z.max(feet.z) + HAND_HEIGHT + 5.;
        if !(low..=high).contains(&top) {
            return None;
        }
        let start = vec3(feet.x, feet.y, top - HAND_HEIGHT);
        if !clear(world, feet, start) {
            return None;
        }
        Some(Self {
            start,
            top,
            forward,
            elapsed: 0.,
            pulling: false,
            side: 0.,
            forward_held: input.wish.dot(forward) > 0.25,
        })
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.start.is_finite()
                && self.top.is_finite()
                && self.forward.is_finite()
                && (self.forward.length() - 1.).abs() < 0.001
                && (0. ..=PULL_TIME).contains(&self.elapsed)
                && (-1. ..=1.).contains(&self.side)
                && (self.top - self.start.z - HAND_HEIGHT).abs() < 0.1,
            "Invalid saved ledge contact"
        );
        Ok(())
    }
    pub fn valid_contact(&self, world: &World) -> bool {
        contact(world, self.start, self.forward).is_some_and(|z| (z - self.top).abs() < 0.2)
    }
    pub fn matches_position(&self, world: &World, feet: Vec3) -> bool {
        if !self.pulling {
            return feet.distance(self.start) < 0.1;
        }
        let Some((raised, end)) = self.destination(world) else {
            return false;
        };
        let t = self.elapsed / PULL_TIME;
        let expected = if t < 0.6 {
            self.start.lerp(raised, smooth(t / 0.6))
        } else {
            raised.lerp(end, smooth((t - 0.6) / 0.4))
        };
        feet.distance(expected) < 0.1
    }
    fn destination(&self, world: &World) -> Option<(Vec3, Vec3)> {
        let raised = vec3(self.start.x, self.start.y, self.top + 0.125);
        let end = raised + self.forward.extend(0.) * 28.;
        let floor = world.body_trace(end, end - Vec3::Z * 2.);
        if clear(world, self.start, raised)
            && clear(world, raised, end)
            && !floor.start_solid
            && floor.fraction < 1.
            && floor.normal.z >= 0.9
            && Immersion::sample(world, end).level < 2
        {
            // Retain the original path for flat ledges and existing pull-up saves.
            return Some((raised, end));
        }
        // The hands sample the lip, whereas the whole body lands farther inland.
        // Pool's rock shelves rise across that footprint. Find their actual
        // support height, then clear the entire rise and crossing before moving.
        for reach in [28., 20.] {
            let high =
                vec3(self.start.x, self.start.y, self.top + 24.) + self.forward.extend(0.) * reach;
            let low = high - Vec3::Z * 42.;
            let floor = world.body_trace(high, low);
            if floor.start_solid || floor.all_solid || floor.fraction >= 1. || floor.normal.z < 0.65
            {
                continue;
            }
            let end = high.lerp(low, floor.fraction) + Vec3::Z * 0.125;
            let raised = vec3(self.start.x, self.start.y, end.z.max(self.top + 0.125));
            if clear(world, self.start, raised)
                && clear(world, raised, end)
                && Immersion::sample(world, end).level < 2
            {
                return Some((raised, end));
            }
        }
        None
    }
    /// None releases; Some(feet, complete) advances one fixed tick.
    pub fn tick(&mut self, world: &World, feet: Vec3, input: Controls) -> Option<(Vec3, bool)> {
        if !self.valid_contact(world) {
            return None;
        }
        if self.pulling {
            let (raised, end) = self.destination(world)?;
            self.elapsed = (self.elapsed + FIXED_DT).min(PULL_TIME);
            let t = self.elapsed / PULL_TIME;
            // Preserve full body clearance while the visual plays the authored
            // pull-up. This path does not claim native per-frame root motion.
            let next = if t < 0.6 {
                self.start.lerp(raised, smooth(t / 0.6))
            } else {
                raised.lerp(end, smooth((t - 0.6) / 0.4))
            };
            return clear(world, feet, next).then_some((next, t >= 1.));
        }
        let forward = input.wish.dot(self.forward);
        if input.rise < -0.25 || forward < -0.25 {
            return None;
        }
        let pressed = forward > 0.25 && !self.forward_held;
        self.forward_held = forward > 0.25;
        if (pressed || input.jump) && self.destination(world).is_some() {
            self.pulling = true;
            self.elapsed = 0.;
            self.side = 0.;
            return Some((feet, false));
        }
        let right = vec2(self.forward.y, -self.forward.x);
        let side = input.wish.dot(right).clamp(-1., 1.);
        let candidate = feet + right.extend(0.) * side * SHIMMY_SPEED * FIXED_DT;
        let can_shimmy = side.abs() > 0.25
            && clear(world, feet, candidate)
            && contact(world, candidate, self.forward).is_some_and(|z| (z - self.top).abs() < 0.2);
        let side = if can_shimmy { side.signum() } else { 0. };
        if side != self.side {
            self.elapsed = 0.;
        }
        self.side = side;
        self.elapsed = (self.elapsed + FIXED_DT).rem_euclid(if side == 0. { 2. } else { 0.8 });
        if can_shimmy {
            self.start = candidate;
        }
        Some((self.start, false))
    }
    pub fn clip(&self) -> usize {
        if self.pulling {
            29
        } else if self.side < 0. {
            27
        } else if self.side > 0. {
            28
        } else {
            26
        }
    }
}
fn smooth(t: f32) -> f32 {
    t * t * (3. - 2. * t)
}

/// Find a real ledge for a repeatable local-asset traversal probe. This is a
/// staged recovery test, not an assertion that the campaign requires this edge.
pub fn probe(map: &crate::bsp::Bsp, world: &World) -> Option<(Vec3, Vec2)> {
    for vertex in map.vertices.iter().filter(|v| v.normal.z > 0.9) {
        for forward in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
            let side = vec2(-forward.y, forward.x);
            for shift in [-32., 32., 0.] {
                let feet = vertex.position - forward.extend(0.) * 16.2 + side.extend(0.) * shift
                    - Vec3::Z * (HAND_HEIGHT - 2.);
                if !world.body_clear(feet) {
                    continue;
                }
                if let Some(h) = Hang::catch(
                    world,
                    feet + Vec3::Z,
                    feet,
                    -Vec3::Z * 80.,
                    forward,
                    Controls::default(),
                ) {
                    if h.destination(world).is_some() {
                        let mut p = crate::movement::Player::new(feet);
                        p.velocity = -Vec3::Z * 80.;
                        p.tick(
                            world,
                            Controls {
                                wish: forward,
                                ..Default::default()
                            },
                        );
                        if p.ledge
                            .as_ref()
                            .is_some_and(|h| h.destination(world).is_some())
                        {
                            return Some((feet, forward));
                        }
                    }
                }
            }
        }
    }
    None
}
pub fn check(assets: &mut crate::assets::Assets) -> Result<()> {
    use crate::{bsp::Bsp, interaction::Interactions, movement::Player};
    // Sloped rock shelves on the Pool riverbank: catches worked, pull-up failed.
    let map = Bsp::parse(&assets.read("maps/potears1.bsp")?)?;
    let world = World::from_bsp(&map)?;
    for feet in [
        vec3(-2640.2, 2624., 326.687),
        vec3(-2248.2, 3040., 1544.0905),
        vec3(-2248.2, 3104., 1559.1122),
        vec3(-2176.2, 3648., 1612.5306),
        vec3(-2160.2, 2368., 1863.3569),
        vec3(-2160.2, 2624., 1908.9702),
    ] {
        let mut p = Player::new(feet);
        p.velocity = -Vec3::Z * 80.;
        p.tick(
            &world,
            Controls {
                wish: Vec2::X,
                ..Default::default()
            },
        );
        ensure!(p.ledge.is_some(), "Pool shelf catch failed at {feet:?}");
        p.tick(
            &world,
            Controls {
                jump: true,
                ..Default::default()
            },
        );
        ensure!(
            p.ledge.as_ref().is_some_and(|h| h.pulling),
            "Pool shelf rejected pull-up at {feet:?}"
        );
        for tick in 0..360 {
            p.tick(&world, Controls::default());
            ensure!(
                world.body_clear(p.feet),
                "Pool shelf entered collision at {feet:?}"
            );
            if tick == 120 {
                p = serde_json::from_value(serde_json::to_value(&p)?)?;
                p.validate_world(&world)?;
            }
        }
        ensure!(
            p.ledge.is_none() && p.grounded && p.feet.z > feet.z + 50.,
            "Pool shelf failed to land at {feet:?}"
        );
        println!("PASS Pool slope {feet:?} -> {:?}: catch, Space pull, saved continuation and clear landing", p.feet);
    }
    for name in ["fortress1", "garden1", "wforest"] {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let mut world = World::from_bsp(&map)?;
        let mut i = Interactions::load(&map)?;
        i.set_entry(
            assets,
            &map,
            name,
            if name == "fortress1" {
                Some("fortress1_start2")
            } else {
                None
            },
        )?;
        i.sync(&mut world);
        let (feet, forward) =
            probe(&map, &world).ok_or_else(|| anyhow::anyhow!("No ledge fixture in {name}"))?;
        let mut p = Player::new(feet);
        p.velocity = -Vec3::Z * 80.;
        p.tick(
            &world,
            Controls {
                wish: forward,
                ..Default::default()
            },
        );
        ensure!(
            p.ledge.is_some(),
            "{name}: automatic airborne catch failed at {feet:?}"
        );
        p.tick(&world, Controls::default());
        p.tick(
            &world,
            Controls {
                wish: forward,
                ..Default::default()
            },
        );
        let mut loaded: Player = serde_json::from_value(serde_json::to_value(&p)?)?;
        for _ in 0..360 {
            p.tick(&world, Controls::default());
            loaded.tick(&world, Controls::default());
            ensure!(
                p.feet == loaded.feet && world.body_clear(p.feet),
                "Saved ledge diverged or entered a solid"
            );
        }
        ensure!(
            p.ledge.is_none() && p.grounded && p.feet.z > feet.z + 55.,
            "{name}: pull-up failed"
        );
        println!(
            "PASS ledge {name}: {feet:?}, facing {forward:?}, catch/pull-up/save -> {:?}",
            p.feet
        );
    }
    Ok(())
}

pub async fn render_check(assets: &mut crate::assets::Assets) -> Result<()> {
    use crate::{
        character::{Character, WeaponInput},
        interaction::Interactions,
        movement::Player,
        render::Scene,
    };
    for name in ["fortress1", "potears1"] {
        let mut scene = Scene::load(assets, name)?;
        let mut i = Interactions::load(&scene.map)?;
        i.set_entry(
            assets,
            &scene.map,
            name,
            if name == "fortress1" {
                Some("fortress1_start2")
            } else {
                None
            },
        )?;
        i.sync(&mut scene.world);
        let (feet, forward) = if name == "potears1" {
            (vec3(-2248.2, 3040., 1544.0905), Vec2::X)
        } else {
            probe(&scene.map, &scene.world).ok_or_else(|| anyhow::anyhow!("No render ledge"))?
        };
        let mut p = Player::new(feet);
        p.velocity = -Vec3::Z * 80.;
        let mut alice = Character::load(assets)?;
        let mut pull_sounds = 0;
        alice.reset(&p, forward.y.atan2(forward.x));
        for frame in 0..440 {
            let input = if frame == 0 || frame == 80 {
                Controls {
                    wish: forward,
                    ..Default::default()
                }
            } else {
                Controls::default()
            };
            p.tick(&scene.world, input);
            alice.update(
                FIXED_DT,
                &p,
                false,
                WeaponInput {
                    selected: 0,
                    dice: 0,
                    click: Some(false),
                    aim: forward.extend(0.),
                    first_person: false,
                },
                &crate::combat::Context {
                    world: &scene.world,
                    targets: &[],
                },
            );
            pull_sounds += alice
                .take_audio()
                .iter()
                .filter(|c| c.path.ends_with("ledge_climb.wav"))
                .count();
            if frame < 360 {
                ensure!(alice.visual_counts().0 == 0, "Hanging fired a weapon");
            }
            let target = p.feet + Vec3::Z * 38.;
            let eye = target - forward.extend(0.) * 135.
                + vec3(-forward.y, forward.x, 0.) * 90.
                + Vec3::Z * 35.;
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: eye,
                target,
                up: Vec3::Z,
                z_near: 2.,
                z_far: 30000.,
                fovy: 75_f32.to_radians(),
                ..Default::default()
            });
            let poses = i.transforms();
            scene.draw(eye, frame as f32 * FIXED_DT, false, false, &poses);
            alice.atmosphere(&scene.atmosphere, eye);
            alice.draw(p.feet, false);
            crate::render::depth_read_only(|| {
                scene.draw(eye, frame as f32 * FIXED_DT, false, true, &poses)
            });
            set_default_camera();
            if [30, 160, 280, 400].contains(&frame) {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/ledge-{name}-{frame}.png"
                )))?;
            }
            next_frame().await;
        }
        ensure!(p.ledge.is_none() && p.grounded, "Native pull-up failed");
        ensure!(
            pull_sounds == 1,
            "Pull-up sound should occur once, got {pull_sounds}"
        );
        println!("PASS native ledge catch, held weapon, pull-up and return to ground");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::{FixedClock, Player};
    #[test]
    fn diagonal_rock_steps_catch_with_both_hands_and_restore_mid_pull() {
        let a = vec3(16., -100., 64.);
        let b = vec3(200., -100., 64.);
        let c = vec3(200., 100., 64.);
        let d = vec3(16., 100., 64.);
        let low = Vec3::Z * 164.;
        for degrees in [15_f32, 45., 75.] {
            let rotation = Quat::from_rotation_z(degrees.to_radians());
            let lip = crate::collision::Collider::triangles(
                &[[a, b, c], [a, c, d], [a - low, a, d], [a - low, d, d - low]],
                Vec3::ZERO,
                rotation,
            );
            let mut w = World::fixture(&[]);
            w.set_dynamic(vec![lip.clone()]);
            w.set_settled_supports(vec![lip]);
            let forward = (rotation * Vec3::X).truncate();
            let extra = crate::collision::PLAYER_HALF.truncate().dot(forward.abs()) - 15.;
            let mut p = Player::new((-forward * extra).extend(6.));
            p.velocity = (forward * 30.).extend(-80.);
            p.tick(
                &w,
                Controls {
                    wish: forward,
                    ..Default::default()
                },
            );
            assert!(
                p.ledge.is_some(),
                "{degrees}-degree lip missed at {:?}",
                p.feet
            );
            p.tick(
                &w,
                Controls {
                    jump: true,
                    ..Default::default()
                },
            );
            for _ in 0..120 {
                p.tick(&w, Controls::default());
            }
            let mut restored: Player =
                serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
            restored.validate_world(&w).unwrap();
            for _ in 0..240 {
                p.tick(&w, Controls::default());
                restored.tick(&w, Controls::default());
                assert_eq!(p.feet, restored.feet);
                assert!(w.body_clear(p.feet));
            }
            assert!(p.grounded && p.ledge.is_none() && p.feet.z > 63.9);
        }
    }
    fn world() -> World {
        World::fixture(&[(vec3(16., -100., -100.), vec3(200., 100., 64.))])
    }
    fn falling(w: &World) -> Player {
        let mut p = Player::new(vec3(0., 0., 6.));
        p.velocity = vec3(30., 0., -80.);
        p.tick(
            w,
            Controls {
                wish: Vec2::X,
                ..Default::default()
            },
        );
        assert!(p.ledge.is_some(), "automatic catch at {:?}", p.feet);
        p
    }
    #[test]
    fn airborne_catch_pullup_release_and_save_are_fixed_rate() {
        let w = world();
        let caught = falling(&w);
        let mut reference = None;
        for hz in [30, 60, 144] {
            let mut p = caught.clone();
            p.tick(&w, Controls::default());
            let mut clock = FixedClock::default();
            for _ in 0..hz * 3 {
                clock.advance(
                    1. / hz as f64,
                    &w,
                    &mut p,
                    Controls {
                        wish: Vec2::X,
                        ..Default::default()
                    },
                );
            }
            assert!(p.ledge.is_none() && p.grounded && p.feet.z > 63.9);
            let json = serde_json::to_value(&p).unwrap();
            if let Some(r) = &reference {
                assert_eq!(&json, r);
            }
            reference = Some(json);
        }
        let mut p = caught;
        // Held forward must leave a usable stationary hang.
        for _ in 0..100 {
            p.tick(
                &w,
                Controls {
                    wish: Vec2::X,
                    ..Default::default()
                },
            );
        }
        assert!(!p.ledge.as_ref().unwrap().pulling);
        p.tick(&w, Controls::default());
        p.tick(
            &w,
            Controls {
                wish: Vec2::X,
                ..Default::default()
            },
        );
        for _ in 0..130 {
            p.tick(&w, Controls::default());
        }
        let mut resumed: Player =
            serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
        resumed.validate_save().unwrap();
        resumed.validate_world(&w).unwrap();
        for _ in 0..600 {
            p.tick(&w, Controls::default());
            resumed.tick(&w, Controls::default());
            assert_eq!(p.feet, resumed.feet);
            assert!(w.body_clear(p.feet));
        }
        let mut dropped = falling(&w);
        dropped.tick(
            &w,
            Controls {
                rise: -1.,
                ..Default::default()
            },
        );
        assert!(dropped.ledge.is_none());
        for _ in 0..30 {
            dropped.tick(
                &w,
                Controls {
                    wish: Vec2::X,
                    ..Default::default()
                },
            );
            assert!(dropped.ledge.is_none());
        }
    }
    #[test]
    fn rejects_one_hand_fast_fall_ceiling_and_blocked_pullup() {
        let w = world();
        let p = vec3(0., 0., 4.);
        assert!(Hang::catch(&w, p, p, vec3(0., 0., -501.), Vec2::X, Controls::default()).is_none());
        let narrow = World::fixture(&[(vec3(16., 0., -100.), vec3(200., 100., 64.))]);
        assert!(Hang::catch(&narrow, p, p, Vec3::ZERO, Vec2::X, Controls::default()).is_none());
        let low = World::fixture(&[
            (vec3(16., -100., -100.), vec3(200., 100., 64.)),
            (vec3(-100., -100., 100.), vec3(200., 100., 120.)),
        ]);
        let mut p = falling(&low);
        p.tick(
            &low,
            Controls {
                jump: true,
                ..Default::default()
            },
        );
        assert!(!p.ledge.as_ref().unwrap().pulling);
        let mut p = falling(&w);
        for _ in 0..120 {
            p.tick(
                &w,
                Controls {
                    wish: Vec2::Y,
                    ..Default::default()
                },
            );
        }
        assert!(p.feet.y > 14. && p.feet.y < 15.);
    }
}
