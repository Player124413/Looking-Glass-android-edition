//! Jacks' native carrier/children and alternate trace transport. Local saved
//! clocks own events; rendering and Pocket Watch never advance them twice.
use crate::{
    combat::{self, Context, Hit, Target},
    skeletal::Transform,
};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

pub(super) const TOSS: &str = "sound/weapon/jacks/jacks_toss.wav";
const STEP: f64 = 1. / 120.;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Kind {
    Carrier,
    Child,
    Ball,
    Burst,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Jack {
    pub id: u32,
    pub kind: Kind,
    pub position: Vec3,
    pub velocity: Vec3,
    pub age: f64,
    pub trail: super::trail::Trail,
    pub silent: bool,
    pub returning: bool,
    target: Option<usize>,
    parent: Option<u32>,
    next_seek: f64,
    last_contact: f64,
    trace: bool,
    split: bool,
    remove_at: f64,
    seed: u32,
    previous: Vec3,
    bounce_seek: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::World;
    fn owner() -> Target {
        Target {
            id: crate::dice::ALICE,
            center: vec3(-100., 0., 40.),
            half: Vec3::splat(15.),
        }
    }
    #[test]
    fn primary_spawns_once_then_returns_without_damage_or_ammo_refund() {
        let world = World::fixture(&[(vec3(-5000., -5000., -20.), vec3(5000., 5000., 0.))]);
        let ctx = Context {
            world: &world,
            targets: &[],
        };
        for hz in [30, 60, 144] {
            let mut state = State::default();
            state.launch(vec3(0., 0., 50.), Vec3::X, None, false);
            let mut max_count = 0;
            for frame in 0..hz * 7 {
                let e = state.advance(1. / hz as f32, &ctx, owner(), Vec3::X);
                assert!(e.hits.is_empty());
                max_count = max_count.max(state.pieces.len());
                if frame % 11 == 0 {
                    state = serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
                    state.validate().unwrap();
                }
            }
            assert_eq!(max_count, 6);
            assert!(state.pieces.is_empty());
        }
    }
    #[test]
    fn alternate_sweeps_each_piece_once_and_cannot_shoot_through_walls() {
        for blocked in [false, true] {
            let world = World::fixture(&if blocked {
                vec![(vec3(100., -500., -500.), vec3(110., 500., 500.))]
            } else {
                vec![]
            });
            let targets = [Target {
                id: 2,
                center: vec3(300., 0., 50.),
                half: Vec3::splat(60.),
            }];
            let ctx = Context {
                world: &world,
                targets: &targets,
            };
            let mut state = State::default();
            state.launch(vec3(0., 0., 50.), Vec3::X, Some(2), true);
            let mut hits = vec![];
            for i in 0..600 {
                hits.extend(state.advance(1. / 120., &ctx, owner(), Vec3::X).hits);
                if i == 8 || i == 13 {
                    state = serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
                }
            }
            assert_eq!(hits.len(), if blocked { 0 } else { 16 });
            assert_eq!(
                hits.iter().map(|h| h.damage).sum::<f32>(),
                if blocked { 0. } else { 67. }
            );
            assert!(state.pieces.is_empty());
        }
    }
    #[test]
    fn primary_bounces_seeks_and_damages() {
        let world = World::fixture(&[(vec3(-5000., -5000., -20.), vec3(5000., 5000., 0.))]);
        let targets = [Target {
            id: 2,
            center: vec3(250., 0., 40.),
            half: vec3(24., 24., 40.),
        }];
        let ctx = Context {
            world: &world,
            targets: &targets,
        };
        let mut state = State::default();
        state.launch(vec3(0., 0., 50.), Vec3::X, Some(2), false);
        let mut hits = 0;
        for _ in 0..700 {
            let e = state.advance(1. / 120., &ctx, owner(), Vec3::X);
            assert!(e.hits.iter().all(|h| h.damage == 7. && h.id == 2));
            hits += e.hits.len();
        }
        assert!(hits > 0);
        assert!(state.pieces.is_empty());
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct State {
    pub pieces: Vec<Jack>,
    next_id: u32,
    accumulator: f64,
}
#[derive(Default)]
pub(super) struct Events {
    pub hits: Vec<Hit>,
    pub impacts: Vec<(Vec3, Vec3)>,
    pub sounds: Vec<(&'static str, Vec3)>,
}
fn random(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
    ((*seed >> 16) & 32767) as f32 / 32768.
}
fn actor_normal(point: Vec3, target: &Target, half: Vec3) -> Vec3 {
    let offset = point - target.center;
    let distance = (offset.abs() - target.half - half).abs();
    let axis = if distance.x <= distance.y && distance.x <= distance.z {
        0
    } else if distance.y <= distance.z {
        1
    } else {
        2
    };
    let mut normal = Vec3::ZERO;
    normal[axis] = offset[axis].signum();
    normal
}
impl Jack {
    fn damage(&self) -> f32 {
        match self.kind {
            Kind::Ball => 0.,
            Kind::Burst if self.silent => 4.,
            _ => 7.,
        }
    }
    pub fn pose(&self) -> Transform {
        Transform {
            translation: self.position,
            rotation: Quat::from_rotation_y(
                self.age as f32
                    * if self.kind == Kind::Carrier {
                        400_f32
                    } else {
                        300_f32
                    }
                    .to_radians(),
            ),
        }
    }
    pub fn alpha(&self) -> f32 {
        if self.kind == Kind::Burst && !self.trace {
            ((self.remove_at - self.age) as f32).clamp(0., 1.)
        } else {
            1.
        }
    }
    fn half(&self) -> Vec3 {
        Vec3::splat(if self.kind == Kind::Ball || self.trace {
            4.
        } else {
            8.
        })
    }
}
impl State {
    pub fn validate(&self) -> anyhow::Result<()> {
        let mut ids = std::collections::BTreeSet::new();
        anyhow::ensure!(
            self.pieces.len() <= 256
                && self.accumulator.is_finite()
                && self.accumulator.abs() < STEP + 1e-7
                && self.pieces.iter().all(|p| ids.insert(p.id)
                    && p.position.is_finite()
                    && p.velocity.is_finite()
                    && p.previous.is_finite()
                    && (0. ..=15.).contains(&p.age)
                    && p.next_seek.is_finite()
                    && p.last_contact.is_finite()
                    && p.remove_at.is_finite()
                    && p.remove_at >= p.age
                    && p.remove_at <= 15.
                    && p.trail.valid(p.age)),
            "Invalid saved Jacks"
        );
        Ok(())
    }
    fn piece(
        &mut self,
        kind: Kind,
        position: Vec3,
        velocity: Vec3,
        target: Option<usize>,
        silent: bool,
    ) -> Jack {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        Jack {
            id,
            kind,
            position,
            velocity,
            age: 0.,
            trail: Default::default(),
            silent,
            returning: false,
            target,
            parent: None,
            next_seek: if kind == Kind::Burst { 0.1 } else { f64::MAX },
            last_contact: -1.,
            trace: kind == Kind::Burst,
            split: false,
            remove_at: if kind == Kind::Carrier {
                5.6
            } else if kind == Kind::Burst {
                4.
            } else {
                5.5
            },
            seed: id.wrapping_mul(104729).wrapping_add(17),
            previous: position,
            bounce_seek: false,
        }
    }
    pub fn launch(&mut self, position: Vec3, aim: Vec3, target: Option<usize>, alternate: bool) {
        if self.pieces.len() + if alternate { 16 } else { 6 } > 256 {
            return;
        }
        if alternate {
            for i in 0..16 {
                // The native alternate starts at rest, then traces toward the
                // acquired target plus independent +/-40 spread after 100 ms.
                let p = self.piece(Kind::Burst, position, Vec3::ZERO, target, i > 0);
                self.pieces.push(p);
            }
        } else {
            let p = self.piece(
                Kind::Carrier,
                position,
                aim * 200. + Vec3::Z * 250.,
                target,
                false,
            );
            self.pieces.push(p);
        }
    }
    pub fn advance(&mut self, dt: f32, ctx: &Context<'_>, owner: Target, aim: Vec3) -> Events {
        let mut out = Events::default();
        if !dt.is_finite() || dt <= 0. {
            return out;
        }
        self.accumulator += dt.min(1.) as f64;
        while self.accumulator + 1e-8 >= STEP {
            self.accumulator = (self.accumulator - STEP).max(0.);
            self.step(ctx, owner, aim, &mut out);
        }
        out
    }
    fn step(&mut self, ctx: &Context<'_>, owner: Target, aim: Vec3, out: &mut Events) {
        let mut children = Vec::new();
        for p in &mut self.pieces {
            p.age += STEP;
            if p.kind == Kind::Carrier && !p.split && p.age + 1e-8 >= 0.1 {
                p.split = true;
                children.push(p.clone());
            }
        }
        for mut parent in children {
            for (i, offset) in [
                Vec2::ZERO,
                vec2(40., 40.),
                vec2(-40., 40.),
                vec2(40., -40.),
                vec2(-40., -40.),
            ]
            .into_iter()
            .enumerate()
            {
                let velocity = if i == 0 {
                    Vec3::ZERO
                } else {
                    parent.velocity + offset.extend(0.)
                } + Vec3::Z * random(&mut parent.seed) * 100.;
                let mut p = self.piece(
                    if i == 0 { Kind::Ball } else { Kind::Child },
                    parent.position,
                    velocity,
                    None,
                    false,
                );
                p.parent = (i != 0).then_some(parent.id);
                self.pieces.push(p);
            }
        }
        let parents: Vec<_> = self.pieces.iter().map(|p| (p.id, p.position)).collect();
        for p in &mut self.pieces {
            let return_at = if p.kind == Kind::Carrier { 5.1 } else { 5. };
            if p.kind != Kind::Burst && p.age + 1e-8 >= return_at {
                p.returning = true;
                // return_player clears solidity and converges in the remaining
                // half second; it does not deal damage or replenish Will.
                p.velocity = (owner.center - p.position) / (p.remove_at - p.age).max(0.1) as f32;
                p.position += p.velocity * STEP as f32;
                p.trail.record(p.age as f32, p.pose());
                continue;
            }
            if p.trace {
                if p.age + 1e-8 < p.next_seek {
                    continue;
                }
                if !p.split {
                    p.split = true;
                    let mut direction = p
                        .target
                        .and_then(|id| ctx.targets.iter().find(|t| t.id == id))
                        .map_or(Vec3::ZERO, |t| (t.center - p.position).normalize_or_zero());
                    direction = direction * 400.
                        + vec3(
                            random(&mut p.seed) * 80. - 40.,
                            random(&mut p.seed) * 80. - 40.,
                            random(&mut p.seed) * 80. - 40.,
                        );
                    p.velocity = direction.try_normalize().unwrap_or(aim);
                }
                let end = p.position + p.velocity * 400.;
                let wall = ctx.world.sweep(p.position, end, Vec3::splat(4.));
                let actor = combat::contact(ctx, p.position, end, 4.);
                let fraction = actor.map_or(wall.fraction, |(_, f)| f);
                let start = p.position;
                p.position = start.lerp(end, fraction);
                p.trail.record(
                    (p.age as f32 - 0.001).max(0.),
                    Transform {
                        translation: start,
                        ..p.pose()
                    },
                );
                if let Some((id, _)) = actor {
                    out.hits.push(Hit {
                        id,
                        damage: p.damage(),
                        kind: combat::DamageKind::Jacks,
                        knockback: p.velocity * 30.,
                    });
                }
                if wall.start_solid || fraction < 1. {
                    let normal = actor
                        .and_then(|(id, _)| ctx.targets.iter().find(|t| t.id == id))
                        .map_or(wall.normal, |t| {
                            actor_normal(p.position, t, Vec3::splat(4.))
                        });
                    if !p.silent {
                        out.sounds.push((
                            if actor.is_some() {
                                "sound/weapon/jacks/jacks_flesh1.wav"
                            } else {
                                "sound/weapon/jacks/jacks_ricochet1.wav"
                            },
                            p.position,
                        ));
                    }
                    out.impacts.push((p.position, normal));
                    p.trace = false;
                    p.remove_at = p.age + 3.;
                    p.position += normal * 0.05;
                    p.velocity = (p.velocity - 2. * normal * p.velocity.dot(normal)) * 800.;
                } else {
                    p.next_seek += 0.05;
                }
            } else {
                if p.kind != Kind::Burst && p.age + 1e-8 >= p.next_seek {
                    let target = p
                        .parent
                        .and_then(|id| parents.iter().find(|(i, _)| *i == id).map(|(_, v)| *v))
                        .or_else(|| {
                            p.target
                                .and_then(|id| ctx.targets.iter().find(|t| t.id == id))
                                .map(|t| t.center)
                        });
                    if let Some(target) = target {
                        let delta = target
                            + vec3(
                                random(&mut p.seed) * 40. - 20.,
                                random(&mut p.seed) * 40. - 20.,
                                random(&mut p.seed) * 40. - 20.,
                            )
                            - p.position;
                        let v = delta.normalize_or_zero() * delta.length().max(250.) * 2.5;
                        p.velocity.x = v.x;
                        p.velocity.y = v.y;
                        if p.bounce_seek {
                            p.velocity.z += 160.;
                            // Native seek reverses a stalled jack when its next
                            // short trace is blocked, preserving vertical speed.
                            if p.position.distance(p.previous) < 1.
                                && ctx
                                    .world
                                    .sweep(
                                        p.position,
                                        p.position + p.velocity * 0.05,
                                        Vec3::splat(4.),
                                    )
                                    .fraction
                                    < 0.5
                            {
                                p.velocity.x = -p.velocity.x;
                                p.velocity.y = -p.velocity.y;
                            }
                        }
                    } else {
                        let right = aim.cross(Vec3::Z).normalize_or_zero();
                        let delta = owner.center - right * 30. - p.position;
                        p.velocity.x = delta.x * 3.;
                        p.velocity.y = delta.y * 3.;
                        if p.bounce_seek {
                            p.velocity.z = 300.;
                        }
                    }
                    p.next_seek = p.age + 0.4;
                    p.bounce_seek = false;
                    p.previous = p.position;
                }
                let gravity = if p.kind == Kind::Carrier {
                    480.
                } else if p.kind == Kind::Burst {
                    0.
                } else {
                    800.
                };
                p.velocity.z -= gravity * STEP as f32;
                let end = p.position + p.velocity * STEP as f32;
                let wall = ctx.world.sweep(p.position, end, p.half());
                let actor = if p.kind == Kind::Burst || p.damage() == 0. {
                    None
                } else {
                    combat::contact_box(ctx, p.position, end, p.half())
                };
                let fraction = actor.map_or(wall.fraction, |(_, f)| f);
                p.position = p.position.lerp(end, fraction);
                if !wall.start_solid && fraction < 1. {
                    let normal = actor
                        .and_then(|(id, _)| ctx.targets.iter().find(|t| t.id == id))
                        .map_or(wall.normal, |t| actor_normal(p.position, t, p.half()));
                    if p.age - p.last_contact > 0.1 {
                        if let Some((id, _)) = actor {
                            out.hits.push(Hit {
                                id,
                                damage: p.damage(),
                                kind: combat::DamageKind::Jacks,
                                knockback: p.velocity.normalize_or_zero() * 30.,
                            });
                        }
                        if !p.silent {
                            out.sounds.push((
                                if p.kind == Kind::Ball {
                                    "sound/weapon/jacks/jacks_ball_bounce.wav"
                                } else if actor.is_some() {
                                    "sound/weapon/jacks/jacks_flesh1.wav"
                                } else {
                                    "sound/weapon/jacks/jacks_ricochet1.wav"
                                },
                                p.position,
                            ));
                        }
                        out.impacts.push((p.position, normal));
                        p.last_contact = p.age;
                    }
                    p.position += normal * 0.04;
                    p.velocity = (p.velocity - 2. * normal * p.velocity.dot(normal)) * 0.85;
                    if normal.z > 0. && p.velocity.z < 45. {
                        p.velocity.z += 60.;
                    }
                    if p.kind != Kind::Burst {
                        p.next_seek = p.next_seek.min(p.age + 0.05);
                        p.bounce_seek = true;
                    }
                }
            }
            p.trail.record(p.age as f32, p.pose());
        }
        self.pieces.retain(|p| p.age + 1e-8 < p.remove_at);
    }
}
