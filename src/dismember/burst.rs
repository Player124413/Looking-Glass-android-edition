//! Card Guards' occasional whole-body death, with saved cosmetic debris.
use crate::{assets::Assets, collision::World, skeletal::Transform, texture, weapons::Prop};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const LIFE: f32 = 5.;
pub fn seed(home: Vec3, attacks: u32) -> u32 {
    let mut n = home.x.to_bits()
        ^ home.y.to_bits().rotate_left(11)
        ^ home.z.to_bits().rotate_left(22)
        ^ attacks.wrapping_mul(0x9e3779b9);
    n ^= n >> 16;
    n = n.wrapping_mul(0x7feb352d);
    n ^ (n >> 15)
}
fn random(n: &mut u32) -> f32 {
    *n = n.wrapping_mul(1664525).wrapping_add(1013904223);
    (*n >> 8) as f32 / 16777216.
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Piece {
    model: usize,
    position: Vec3,
    velocity: Vec3,
    angles: Vec3,
    spin: Vec3,
    scale: f32,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct State {
    pub age: f32,
    pub pieces: Vec<Piece>,
}
impl State {
    pub fn new(feet: Vec3, yaw: f32, scale: f32, mut seed: u32) -> Self {
        // The reviewed red-gib recipe contains seven pieces plus the dropped staff.
        let pieces = [0, 0, 1, 2, 2, 1, 3, 4]
            .into_iter()
            .enumerate()
            .map(|(i, model)| {
                let offset = vec3(random(&mut seed), random(&mut seed), random(&mut seed)) * 20.
                    - Vec3::splat(10.);
                let velocity = vec3(
                    random(&mut seed) * 200. - 100.,
                    random(&mut seed) * 200. - 100.,
                    random(&mut seed) * 600.,
                );
                let spin = (vec3(random(&mut seed), random(&mut seed), random(&mut seed)) * 360.
                    - Vec3::splat(180.))
                    * std::f32::consts::PI
                    / 180.;
                Piece {
                    model,
                    position: feet + (Vec3::Z * 40. + offset) * scale,
                    velocity,
                    angles: vec3(
                        0.,
                        if model == 4 {
                            std::f32::consts::FRAC_PI_2
                        } else {
                            0.
                        },
                        yaw,
                    ),
                    spin,
                    scale: scale
                        * if i == 5 {
                            0.4 + random(&mut seed) * 0.2
                        } else if model == 4 {
                            1.
                        } else {
                            0.8 + random(&mut seed) * 0.4
                        },
                }
            })
            .collect();
        Self { age: 0., pieces }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (0. ..=LIFE).contains(&self.age)
                && self.pieces.len() <= 8
                && (self.age < LIFE || self.pieces.is_empty())
                && self.pieces.iter().all(|p| p.model < 5
                    && p.position.is_finite()
                    && p.position.abs().max_element() < 100000.
                    && p.velocity.is_finite()
                    && p.velocity.length() < 2500.
                    && p.angles.is_finite()
                    && p.spin.is_finite()
                    && p.spin.length() < 10.
                    && (0.001..=12.).contains(&p.scale)),
            "Invalid saved Card Guard debris"
        );
        Ok(())
    }
    pub fn step(&mut self, dt: f32, world: &World) {
        if dt <= 0. {
            return;
        }
        self.age = (self.age + dt).min(LIFE);
        if self.age >= LIFE {
            self.pieces.clear();
            return;
        }
        self.pieces.retain_mut(|p| {
            let half = Vec3::splat(8. * p.scale);
            if world.sweep(p.position, p.position, half).start_solid {
                return false;
            }
            p.velocity.z -= 650. * dt;
            let end = p.position + p.velocity * dt;
            let hit = world.sweep(p.position, end, half);
            p.position = p.position.lerp(end, hit.fraction);
            if hit.fraction < 1. {
                p.position += hit.normal * 0.02;
                p.velocity = (p.velocity - 2. * p.velocity.dot(hit.normal) * hit.normal) * 0.6;
                if hit.normal.z > 0.7 && p.velocity.length() < 35. {
                    p.velocity = Vec3::ZERO;
                    p.spin = Vec3::ZERO;
                }
            }
            p.angles += p.spin * dt;
            true
        });
    }
}

pub struct Art {
    props: Vec<Prop>,
}
impl Art {
    pub fn load(
        assets: &mut Assets,
        specs: &BTreeMap<String, texture::MaterialSpec>,
        ranged: bool,
    ) -> Result<Self> {
        let props = [
            "gb_meatbone1",
            "gb_meatbone2",
            "gb_meatbone3",
            "gb_ribs",
            if ranged {
                "w_diamondstaff"
            } else {
                "w_clubstaff"
            },
        ]
        .into_iter()
        .map(|name| Prop::load(assets, name, specs))
        .collect::<Result<Vec<_>>>()?;
        Ok(Self { props })
    }
    pub fn draw(&mut self, state: &State, fullbright: bool) {
        for piece in &state.pieces {
            let transform = Transform {
                translation: piece.position,
                rotation: Quat::from_euler(
                    EulerRot::XYZ,
                    piece.angles.x,
                    piece.angles.y,
                    piece.angles.z,
                ),
            };
            crate::render::depth_read_only(|| {
                for mesh in self.props[piece.model].meshes_at(
                    transform,
                    piece.scale,
                    fullbright,
                    state.age,
                    false,
                ) {
                    crate::render_fx::skin_effect(mesh, state.age, 1. - state.age / LIFE);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn burst_is_an_occasional_saved_choice_and_never_overrides_knife_or_ice() {
        use crate::combat::{DamageKind, Guard};
        let mut count = 0;
        for i in 0..1000 {
            let mut g = Guard::new(vec3(i as f32, 0., 0.), 0., 1.);
            g.hurt_kind(100., DamageKind::Electric);
            count += usize::from(g.burst.is_some());
            let saved = serde_json::to_vec(&g).unwrap();
            g.hurt_kind(100., DamageKind::Other);
            assert_eq!(saved, serde_json::to_vec(&g).unwrap());
        }
        assert!((60..140).contains(&count));
        for kind in [DamageKind::Knife, DamageKind::Ice] {
            let mut g = Guard::new(Vec3::ZERO, 0., 1.);
            g.hurt_kind(100., kind);
            assert!(g.burst.is_none());
            g.validate_save().unwrap();
        }
    }
    #[test]
    fn debris_is_bounded_pauses_restores_and_expires() {
        let world = World::fixture(&[
            (vec3(-1000., -1000., -20.), vec3(1000., 1000., 0.)),
            (vec3(60., -1000., 0.), vec3(62., 1000., 1000.)),
        ]);
        let mut state = State::new(Vec3::ZERO, 0., 1., 42);
        assert_eq!(state.pieces.len(), 8);
        let before = serde_json::to_vec(&state).unwrap();
        state.step(0., &world);
        assert_eq!(before, serde_json::to_vec(&state).unwrap());
        for _ in 0..120 {
            state.step(1. / 120., &world);
        }
        let mut loaded: State =
            serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
        for _ in 0..600 {
            state.step(1. / 120., &world);
            loaded.step(1. / 120., &world);
            state.validate().unwrap();
            assert!(state
                .pieces
                .iter()
                .all(|p| p.position.x < 60. && p.position.z >= 0.));
        }
        assert_eq!(
            serde_json::to_vec(&state).unwrap(),
            serde_json::to_vec(&loaded).unwrap()
        );
        assert!(state.pieces.is_empty());
    }
}
