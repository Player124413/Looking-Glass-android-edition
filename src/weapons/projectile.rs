//! Blade/Cards projectile rules recovered from the supplied TIKIs and native
//! Projectile::Seek/Drunk/Fragment handlers. No original machine code executes.
use crate::{
    collision::World,
    combat::{self, Context, Hit},
    skeletal::Transform,
};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Kind {
    #[default]
    Legacy,
    Blade,
    Card,
    Carrier,
    Fragment,
    Ball,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Projectile {
    #[serde(default)]
    pub(super) trail: super::trail::Trail,
    pub(super) model: usize,
    pub(super) position: Vec3,
    pub(super) velocity: Vec3,
    pub(crate) age: f64,
    pub(super) bounces: u8,
    #[serde(default)]
    pub(super) audio_id: usize,
    #[serde(default)]
    pub(super) kind: Kind,
    #[serde(default)]
    target: Option<usize>,
    #[serde(default)]
    angles: Vec2,
    #[serde(default)]
    spin: f32,
    #[serde(default)]
    seek_at: f64,
    #[serde(default)]
    drift_at: f64,
    #[serde(default)]
    split: bool,
    #[serde(default)]
    seed: u32,
}

pub(super) struct Impact {
    pub position: Vec3,
    pub normal: Vec3,
    pub model: usize,
    pub hit: Option<Hit>,
    pub blast: bool,
}

fn random(seed: &mut u32) -> f32 {
    // A saved local stream prevents render rate, unrelated emitters, or a reload
    // from rerolling a card's authored crandom spin/drift.
    *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
    ((*seed >> 16) & 0x7fff) as f32 / 32768.
}
fn angles(direction: Vec3) -> Vec2 {
    vec2(
        (-direction.z).atan2(direction.truncate().length()),
        direction.y.atan2(direction.x),
    )
}
fn direction(angles: Vec2) -> Vec3 {
    vec3(
        angles.x.cos() * angles.y.cos(),
        angles.x.cos() * angles.y.sin(),
        -angles.x.sin(),
    )
}
fn approach(current: f32, target: f32, step: f32) -> f32 {
    current
        + ((target - current + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI)
            .clamp(-step, step)
}

impl Projectile {
    pub(crate) fn thrown_blade(origin: Vec3, aim: Vec3) -> Self {
        Self::new(Kind::Blade, origin, aim, None, 1)
    }
    /// A primary-fire card that seeks `target`, for headless route drivers.
    pub(crate) fn card(origin: Vec3, aim: Vec3, target: Option<usize>) -> Self {
        Self::new(Kind::Card, origin, aim, target, 1)
    }
    pub(super) fn new(
        kind: Kind,
        position: Vec3,
        aim: Vec3,
        target: Option<usize>,
        mut seed: u32,
    ) -> Self {
        let speed = match kind {
            Kind::Blade => 1200.,
            Kind::Card => 700.,
            Kind::Carrier | Kind::Fragment => 1000.,
            _ => 2500.,
        };
        let aim = aim.try_normalize().unwrap_or(Vec3::X);
        let spin = if matches!(kind, Kind::Card | Kind::Carrier | Kind::Fragment) {
            ((random(&mut seed) * 2. - 1.) * 200.).to_radians()
        } else {
            1440_f32.to_radians()
        };
        Self {
            trail: Default::default(),
            model: match kind {
                Kind::Blade => 0,
                Kind::Card | Kind::Carrier | Kind::Fragment => 10,
                _ => 11,
            },
            position,
            velocity: aim * speed,
            age: 0.,
            bounces: 0,
            audio_id: 0,
            kind,
            target,
            angles: angles(aim),
            spin,
            seek_at: 0.2,
            drift_at: 0.,
            split: false,
            seed,
        }
    }
    pub(super) fn migrate(&mut self) {
        if self.kind != Kind::Legacy {
            return;
        }
        self.kind = match self.model {
            0 => Kind::Blade,
            10 => Kind::Card,
            _ => Kind::Ball,
        };
        self.angles = angles(self.velocity);
        self.seed = 1;
        self.spin = if self.model == 0 {
            1440_f32.to_radians()
        } else {
            0.
        };
        // Old five-card fans stay as five existing cards. Loading cannot spawn a
        // fresh carrier or fragments, or replay the original firing debit.
        self.split = true;
        self.seek_at = self.age + 0.2;
        self.drift_at = self.age + 0.15;
    }
    pub(super) fn valid(&self) -> bool {
        self.trail.valid(self.age)
            && matches!(self.model, 0 | 10 | 11)
            && (0. ..=10.).contains(&self.age)
            && self.position.is_finite()
            && self.velocity.is_finite()
            && self.velocity.length_squared() <= 10000_f32.powi(2)
            && self.angles.is_finite()
            && self.spin.is_finite()
            && self.seek_at.is_finite()
            && self.drift_at.is_finite()
            && (0. ..=11.).contains(&self.seek_at)
            && (0. ..=11.).contains(&self.drift_at)
            && self.audio_id < 1000
            && match self.kind {
                Kind::Legacy => true,
                Kind::Blade => self.model == 0,
                Kind::Card | Kind::Carrier | Kind::Fragment => self.model == 10,
                Kind::Ball => self.model == 11,
            }
    }
    pub(super) fn life(&self) -> f64 {
        match self.kind {
            Kind::Blade => 1.,
            Kind::Card => 1.5,
            Kind::Carrier | Kind::Fragment => 2.,
            _ => 5.,
        }
    }
    pub(super) fn alive(&self) -> bool {
        self.age < self.life() - 1e-7
    }
    pub(super) fn half(&self) -> Vec3 {
        Vec3::splat(8.)
    }
    pub(super) fn damage(&self) -> f32 {
        match self.kind {
            Kind::Blade => 45.,
            Kind::Card | Kind::Fragment => 7.,
            Kind::Carrier => 15.,
            _ => 30.,
        }
    }
    pub(super) fn prop(&self) -> usize {
        match self.kind {
            Kind::Blade => 13,
            Kind::Card => 14,
            Kind::Carrier => 15,
            Kind::Fragment => 16,
            _ => 11,
        }
    }
    pub(super) fn pose(&self) -> Transform {
        let forward = direction(self.angles);
        let rotation = if self.kind == Kind::Blade {
            Quat::from_rotation_arc(-Vec3::X, forward)
                * Quat::from_rotation_y(self.age as f32 * self.spin)
        } else {
            Quat::from_rotation_arc(Vec3::X, forward)
                * Quat::from_rotation_x(self.age as f32 * self.spin)
        };
        Transform {
            translation: self.position,
            rotation,
        }
    }
    pub(crate) fn threatens(&self, world: &World, target: combat::Target) -> bool {
        self.alive()
            && combat::contact_box(
                &Context {
                    world,
                    targets: &[target],
                },
                self.position,
                self.position + self.velocity * 0.28,
                self.half(),
            )
            .is_some()
    }
    fn drunk(&self) -> bool {
        matches!(self.kind, Kind::Carrier | Kind::Fragment)
    }
    fn speed(&self) -> f32 {
        if self.kind == Kind::Card {
            700.
        } else {
            1000.
        }
    }
    fn control(&mut self, ctx: &Context<'_>) {
        if self.drunk() && self.age + 1e-7 >= self.drift_at {
            // Native Drunk: pitch crandom 3, yaw crandom 5, every 0.15 s.
            self.angles.y += ((random(&mut self.seed) * 2. - 1.) * 5.).to_radians();
            self.angles.x += ((random(&mut self.seed) * 2. - 1.) * 3.).to_radians();
            self.velocity = direction(self.angles) * self.speed();
            self.drift_at += 0.15;
        }
        if self.model == 10 && self.age + 1e-7 >= self.seek_at {
            if let Some(t) = self
                .target
                .and_then(|id| ctx.targets.iter().find(|t| t.id == id))
            {
                let desired = angles(t.center - self.position);
                let current = angles(self.velocity);
                let limit = if self.kind == Kind::Card {
                    10_f32
                } else {
                    25_f32
                }
                .to_radians();
                self.velocity = direction(vec2(
                    approach(current.x, desired.x, limit),
                    approach(current.y, desired.y, limit),
                )) * self.speed();
            } else {
                self.target = None;
            }
            self.seek_at += 0.1;
        }
    }
    fn fragments(&mut self, ctx: &Context<'_>, owner_target: Option<usize>) -> Vec<Self> {
        self.split = true;
        let axis = direction(self.angles);
        let right = vec3(self.angles.y.sin(), -self.angles.y.cos(), 0.);
        (0..8)
            .filter_map(|i| {
                // Native Fragment rotates the right vector in eight 45-degree steps.
                // It keeps the carrier alive and launches children 16 units away.
                let radial =
                    Quat::from_axis_angle(axis, i as f32 * std::f32::consts::FRAC_PI_4) * right;
                let to = self.position + radial * 16.;
                let trace = ctx.world.sweep(self.position, to, Vec3::splat(8.));
                if trace.start_solid {
                    return None;
                }
                let origin = self.position.lerp(to, trace.fraction);
                random(&mut self.seed);
                Some(Self::new(
                    Kind::Fragment,
                    origin,
                    axis + radial * 0.25,
                    owner_target,
                    self.seed,
                ))
            })
            .collect()
    }
    fn run(
        &mut self,
        dt: f64,
        ctx: &Context<'_>,
        impacts: &mut Vec<Impact>,
        births: &mut Vec<(Self, f64)>,
        owner_target: Option<usize>,
    ) {
        self.migrate();
        let mut remaining = dt;
        for _ in 0..512 {
            if !self.alive() {
                if self.kind == Kind::Ball && self.age < 10. {
                    impacts.push(Impact {
                        position: self.position - self.velocity.normalize_or_zero() * 36.,
                        normal: Vec3::Z,
                        model: self.model,
                        hit: None,
                        blast: true,
                    });
                }
                self.age = 10.;
                break;
            }
            if self.kind == Kind::Carrier && !self.split && self.age + 1e-7 >= 0.1 {
                births.extend(
                    self.fragments(ctx, owner_target)
                        .into_iter()
                        .map(|p| (p, remaining)),
                );
            }
            self.control(ctx);
            if remaining <= 1e-8 {
                break;
            }
            let mut step = remaining.min(1. / 120.).min(self.life() - self.age);
            if self.model == 10 {
                step = step.min((self.seek_at - self.age).max(0.));
            }
            if self.drunk() {
                step = step.min((self.drift_at - self.age).max(0.));
            }
            if self.kind == Kind::Carrier && !self.split {
                step = step.min((0.1 - self.age).max(0.));
            }
            if step <= 1e-8 {
                break;
            }
            let t = step as f32;
            if self.kind == Kind::Ball {
                self.trail.record(self.age as f32, self.pose());
            }
            let gravity = match self.kind {
                Kind::Blade => 100.,
                Kind::Ball => 400.,
                _ => 0.,
            };
            let acceleration = -Vec3::Z * gravity;
            let end = self.position + self.velocity * t + acceleration * (0.5 * t * t);
            let wall = ctx.world.sweep(self.position, end, self.half());
            if wall.start_solid {
                self.age = 10.;
                break;
            }
            if let Some((id, fraction)) = combat::contact_box(ctx, self.position, end, self.half())
            {
                self.position = self.position.lerp(end, fraction);
                let normal = -self.velocity.normalize_or_zero();
                impacts.push(Impact {
                    position: self.position
                        - if self.kind == Kind::Ball {
                            self.velocity.normalize_or_zero() * 36.
                        } else {
                            Vec3::ZERO
                        },
                    normal,
                    model: self.model,
                    blast: self.kind == Kind::Ball,
                    hit: Some(Hit {
                        id,
                        damage: self.damage(),
                        kind: if self.model == 0 {
                            combat::DamageKind::Knife
                        } else if self.model == 10 {
                            combat::DamageKind::Cards
                        } else {
                            combat::DamageKind::Electric
                        },
                        knockback: if self.model == 10 {
                            -normal * 60.
                        } else {
                            Vec3::ZERO
                        },
                    }),
                });
                self.age = 10.;
                break;
            }
            self.position = self.position.lerp(end, wall.fraction);
            let elapsed = if self.kind == Kind::Ball {
                step * wall.fraction as f64
            } else {
                step
            };
            self.velocity += acceleration * elapsed as f32;
            self.age += elapsed;
            if self.kind == Kind::Ball {
                self.trail.record(self.age as f32, self.pose());
            }
            remaining -= elapsed;
            if wall.fraction < 1. {
                impacts.push(Impact {
                    position: self.position,
                    normal: wall.normal,
                    model: self.model,
                    hit: None,
                    blast: false,
                });
                if self.kind == Kind::Ball {
                    self.velocity =
                        (self.velocity - 2. * self.velocity.dot(wall.normal) * wall.normal) * 0.85;
                    if wall.normal.z > 0. && self.velocity.z < 45. {
                        self.velocity.z += 60.;
                    }
                    self.position += wall.normal * 0.1;
                    self.bounces = self.bounces.saturating_add(1);
                } else {
                    self.age = 10.;
                    break;
                }
            }
        }
    }
    pub(crate) fn contact_step(
        &mut self,
        dt: f32,
        context: &Context<'_>,
    ) -> (Option<Hit>, Option<Vec3>) {
        let mut impacts = Vec::new();
        if dt.is_finite() && dt > 0. {
            self.run(
                dt.min(1.) as f64,
                context,
                &mut impacts,
                &mut Vec::new(),
                self.target,
            );
        }
        impacts
            .into_iter()
            .next()
            .map_or((None, None), |i| (i.hit, Some(i.normal)))
    }
    #[cfg(test)]
    pub(super) fn advance(&mut self, dt: f32, world: &World) -> Option<Vec3> {
        self.contact_step(
            dt,
            &Context {
                world,
                targets: &[],
            },
        )
        .1
    }
}

/// Run births for the remaining portion of this frame, rather than delaying all
/// fragments to the next rendered frame. Every split is recorded in the save.
pub(super) fn advance(
    projectiles: &mut Vec<Projectile>,
    dt: f32,
    _stopped: bool,
    ctx: &Context<'_>,
    owner_target: Option<usize>,
) -> Vec<Impact> {
    let mut impacts = Vec::new();
    if !dt.is_finite() || dt <= 0. {
        return impacts;
    }
    let mut pending: std::collections::VecDeque<_> = std::mem::take(projectiles)
        .into_iter()
        .map(|p| (p, dt.min(1.) as f64))
        .collect();
    let mut processed = 0;
    while let Some((mut p, time)) = pending.pop_front() {
        processed += 1;
        if processed > 256 {
            break;
        }
        p.migrate();
        // Both omitted and explicit zero ignore_deadtime arguments opt out.
        let mut births = Vec::new();
        p.run(time, ctx, &mut impacts, &mut births, owner_target);
        pending.extend(births);
        if p.alive() {
            projectiles.push(p);
        }
    }
    impacts
}

pub(super) fn target(ctx: &Context<'_>, origin: Vec3, aim: Vec3) -> Option<usize> {
    let forward = aim.normalize_or_zero();
    ctx.targets
        .iter()
        .filter(|t| {
            t.id != crate::dice::SUMMON
                && t.id != crate::dice::ALICE
                && !(crate::interaction::SHOT_BASE..crate::encounters::BASE).contains(&t.id)
        })
        .filter_map(|t| {
            let offset = t.center - origin;
            let distance = offset.length();
            if distance <= 0. || distance > 2048. {
                return None;
            }
            let alignment = (offset / distance).dot(forward);
            // Player::FindClosestEnemyInRadius uses a 90-degree full cone and
            // chooses angular closeness, not the projectile's separate 0.4 helper.
            if alignment <= std::f32::consts::FRAC_1_SQRT_2
                || ctx.world.sweep(origin, t.center, Vec3::ZERO).fraction < 1.
            {
                return None;
            }
            Some((t.id, alignment))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|t| t.0)
}

/// KnifeAttack's 40-unit sweep from the weapon origin, ±15 in X/Y and 0..24
/// vertically. The native helper can damage more than one actor in that volume.
pub(super) fn melee(ctx: &Context<'_>, origin: Vec3, aim: Vec3) -> Vec<Hit> {
    melee_toy(ctx, origin, aim, false)
}
pub(super) fn melee_toy(ctx: &Context<'_>, origin: Vec3, aim: Vec3, mallet: bool) -> Vec<Hit> {
    let aim = aim.normalize_or_zero();
    let end = origin + aim * if mallet { 50. } else { 40. };
    let wall = ctx.world.sweep(origin, end, Vec3::ZERO);
    if wall.start_solid {
        return Vec::new();
    }
    let end = origin.lerp(end, wall.fraction);
    let raised = Vec3::Z * 12.;
    let mut seen = std::collections::BTreeSet::new();
    ctx.targets
        .iter()
        .filter_map(|t| {
            if seen.contains(&t.id) {
                return None;
            }
            let fraction = combat::segment_box(
                origin + raised,
                end + raised,
                t.center,
                t.half + vec3(15., 15., 12.),
            )?;
            let sample = (origin + raised)
                .lerp(end + raised, fraction)
                .clamp(t.center - t.half, t.center + t.half);
            let blocked = if t.id >= super::ice::WALL_BASE {
                ctx.world
                    .sweep_geometry(origin, sample, Vec3::ZERO)
                    .fraction
                    < 1.
            } else {
                ctx.world.sweep(origin, sample, Vec3::ZERO).fraction < 1.
            };
            if blocked {
                return None;
            }
            seen.insert(t.id);
            Some(Hit {
                id: t.id,
                damage: if mallet { 24. } else { 25. },
                kind: if mallet {
                    combat::DamageKind::Electric
                } else {
                    combat::DamageKind::Knife
                },
                knockback: aim * if mallet { 100. } else { 0. },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn open() -> World {
        World::fixture(&[])
    }
    fn ctx(world: &World) -> Context<'_> {
        Context {
            world,
            targets: &[],
        }
    }
    #[test]
    fn croquet_ball_has_five_second_fuse_and_single_direct_damage() {
        let world = open();
        for hz in [30, 60, 144] {
            let mut ps = vec![Projectile::new(Kind::Ball, Vec3::ZERO, Vec3::X, None, 1)];
            let mut explosions = 0;
            for frame in 0..hz * 5 {
                let events = advance(&mut ps, 1. / hz as f32, true, &ctx(&world), None);
                explosions += events.iter().filter(|e| e.blast).count();
                if frame == hz - 1 {
                    assert!((ps[0].position.x - 2500.).abs() < 0.05);
                    assert!((ps[0].position.z + 200.).abs() < 0.01);
                }
                if frame == hz * 2 {
                    ps = serde_json::from_value(serde_json::to_value(&ps).unwrap()).unwrap();
                }
            }
            assert!(ps.is_empty());
            assert_eq!(explosions, 1);
        }
        let target = combat::Target {
            id: 7,
            center: vec3(90., 0., 0.),
            half: Vec3::splat(10.),
        };
        let targets = [target];
        let ctx = Context {
            world: &world,
            targets: &targets,
        };
        let mut ps = vec![Projectile::new(Kind::Ball, Vec3::ZERO, Vec3::X, None, 1)];
        let events = advance(&mut ps, 0.1, false, &ctx, None);
        assert_eq!(events.len(), 1);
        assert!(events[0].blast);
        assert_eq!(events[0].hit.unwrap().damage, 30.);
        assert!(ps.is_empty());
    }
    #[test]
    fn mallet_melee_extends_to_fifty_with_twenty_four_damage_and_knockback() {
        let world = open();
        let targets = [
            combat::Target {
                id: 1,
                center: vec3(65., 0., 12.),
                half: Vec3::ONE,
            },
            combat::Target {
                id: 2,
                center: vec3(68., 0., 12.),
                half: Vec3::ONE,
            },
        ];
        let ctx = Context {
            world: &world,
            targets: &targets,
        };
        assert!(melee(&ctx, Vec3::ZERO, Vec3::X).is_empty());
        let hits = melee_toy(&ctx, Vec3::ZERO, Vec3::X, true);
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].id, hits[0].damage), (1, 24.));
        assert_eq!(hits[0].knockback, Vec3::X * 100.);
    }
    #[test]
    fn blade_ballistics_lifetime_and_cards_deadtime_exception() {
        let world = open();
        for hz in [30, 60, 144] {
            let mut blade = Projectile::thrown_blade(Vec3::ZERO, Vec3::X);
            for _ in 0..hz / 2 {
                blade.contact_step(1. / hz as f32, &ctx(&world));
            }
            assert!((blade.position.x - 600.).abs() < 0.02);
            assert!((blade.position.z + 12.5).abs() < 0.005);
            for _ in 0..hz / 2 {
                blade.contact_step(1. / hz as f32, &ctx(&world));
            }
            assert!(!blade.alive());
            assert!((blade.position.x - 1200.).abs() < 0.04);
        }
        let mut ps = vec![
            Projectile::thrown_blade(Vec3::ZERO, Vec3::X),
            Projectile::new(Kind::Card, Vec3::ZERO, Vec3::X, None, 1),
        ];
        advance(&mut ps, 0.5, true, &ctx(&world), None);
        assert!((ps[0].position.x - 600.).abs() < 0.01);
        assert!((ps[0].age - 0.5).abs() < 1e-6);
        assert!((ps[1].position.x - 350.).abs() < 0.01);
    }
    #[test]
    fn contacts_sweep_original_box_apply_one_hit_and_stop_before_walls() {
        let world = World::fixture(&[(vec3(160., -100., -100.), vec3(162., 100., 100.))]);
        let targets = [
            combat::Target {
                id: 9,
                center: vec3(140., 14., 0.),
                half: Vec3::splat(8.),
            },
            combat::Target {
                id: 10,
                center: vec3(250., 0., 0.),
                half: Vec3::splat(20.),
            },
        ];
        let ctx = Context {
            world: &world,
            targets: &targets,
        };
        for (kind, damage) in [
            (Kind::Blade, 45.),
            (Kind::Card, 7.),
            (Kind::Carrier, 15.),
            (Kind::Fragment, 7.),
        ] {
            let mut p = Projectile::new(kind, Vec3::ZERO, Vec3::X, None, 37);
            // Keep this geometry test independent of random drift.
            p.drift_at = 1.;
            let (hit, _) = p.contact_step(0.5, &ctx);
            let hit = hit.expect("16-unit box catches the grazing actor");
            assert_eq!((hit.id, hit.damage), (9, damage));
            assert!(
                (hit.knockback.length() - if kind == Kind::Blade { 0. } else { 60. }).abs() < 0.001
            );
            assert!(p.contact_step(0.5, &ctx).0.is_none());
        }
        let mut p = Projectile::new(Kind::Card, Vec3::ZERO, Vec3::X, None, 1);
        let only_behind = Context {
            world: &world,
            targets: &targets[1..],
        };
        assert!(p.contact_step(0.5, &only_behind).0.is_none());
        assert!(!p.alive() && p.position.x < 160.);
        let mut p = Projectile::new(Kind::Card, vec3(161., 0., 0.), Vec3::X, None, 1);
        assert!(p.contact_step(0.5, &ctx).0.is_none());
    }
    #[test]
    fn melee_hits_the_authored_volume_once_per_actor_without_crossing_walls() {
        let world = open();
        let targets = [
            combat::Target {
                id: 1,
                center: vec3(40., 18., 20.),
                half: Vec3::splat(4.),
            },
            combat::Target {
                id: 2,
                center: vec3(50., -10., 10.),
                half: Vec3::splat(4.),
            },
            combat::Target {
                id: 3,
                center: vec3(60., 0., 12.),
                half: Vec3::splat(4.),
            },
            combat::Target {
                id: 4,
                center: vec3(20., 0., -5.),
                half: Vec3::splat(4.),
            },
        ];
        let hits = melee(
            &Context {
                world: &world,
                targets: &targets,
            },
            Vec3::ZERO,
            Vec3::X,
        );
        assert_eq!(hits.iter().map(|h| h.id).collect::<Vec<_>>(), [1, 2]);
        assert!(hits
            .iter()
            .all(|h| h.damage == 25. && h.kind == combat::DamageKind::Knife));
        let wall = World::fixture(&[(vec3(25., -100., -100.), vec3(27., 100., 100.))]);
        assert!(melee(
            &Context {
                world: &wall,
                targets: &targets
            },
            Vec3::ZERO,
            Vec3::X
        )
        .is_empty());
    }
    #[test]
    fn melee_checks_later_segments_but_damages_the_actor_only_once() {
        let world = open();
        let box_at = |center| combat::Target {
            id: 77,
            center,
            half: Vec3::splat(4.),
        };
        let targets = [
            box_at(vec3(400., 0., 12.)),
            box_at(vec3(20., 0., 12.)),
            box_at(vec3(30., 0., 12.)),
        ];
        let hits = melee(
            &Context {
                world: &world,
                targets: &targets,
            },
            Vec3::ZERO,
            Vec3::X,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, 77);
        assert_eq!(hits[0].damage, 25.);
    }
    #[test]
    fn carrier_keeps_flying_and_spawns_eight_radial_children_at_point_one() {
        let world = open();
        let mut p = Projectile::new(Kind::Carrier, Vec3::ZERO, Vec3::X, Some(42), 7);
        let children = p.fragments(&ctx(&world), Some(42));
        assert_eq!(children.len(), 8);
        assert!(p.alive());
        for (i, c) in children.iter().enumerate() {
            assert!((c.position.length() - 16.).abs() < 0.0001);
            assert!((c.velocity.length() - 1000.).abs() < 0.001);
            assert_eq!(c.target, Some(42));
            let expected = Quat::from_rotation_x(i as f32 * std::f32::consts::FRAC_PI_4) * -Vec3::Y;
            assert!(c.position.distance(expected * 16.) < 0.0001);
            assert!(
                c.velocity
                    .normalize()
                    .distance((Vec3::X + expected * 0.25).normalize())
                    < 0.0001
            );
        }
        let mut ps = vec![Projectile::new(Kind::Carrier, Vec3::ZERO, Vec3::X, None, 7)];
        advance(&mut ps, 0.099, false, &ctx(&world), None);
        assert_eq!(ps.len(), 1);
        advance(&mut ps, 0.002, false, &ctx(&world), None);
        assert_eq!(ps.len(), 9);
        assert_eq!(ps[0].kind, Kind::Carrier);
        assert!(ps[1..].iter().all(|p| (p.age - 0.001).abs() < 0.000001));
        advance(&mut ps, 0.1, false, &ctx(&world), None);
        assert_eq!(ps.len(), 9);
    }
    #[test]
    fn seeker_waits_then_turns_in_ten_degree_steps_and_does_not_retarget() {
        let world = open();
        let targets = [combat::Target {
            id: 5,
            center: vec3(300., 300., 0.),
            half: Vec3::ONE,
        }];
        let context = Context {
            world: &world,
            targets: &targets,
        };
        let mut p = Projectile::new(Kind::Card, Vec3::ZERO, Vec3::X, Some(5), 1);
        p.contact_step(0.199, &context);
        assert_eq!(p.velocity, Vec3::X * 700.);
        p.contact_step(0.001, &context);
        assert!((angles(p.velocity).y.to_degrees() - 10.).abs() < 0.001);
        p.contact_step(0.1, &context);
        assert!((angles(p.velocity).y.to_degrees() - 20.).abs() < 0.001);
        p.contact_step(0.1, &ctx(&world));
        assert_eq!(p.target, None);
        let v = p.velocity;
        p.contact_step(0.1, &context);
        assert_eq!(p.velocity, v);
    }
    #[test]
    fn splitting_drift_and_save_restoration_are_frame_rate_independent() {
        let world = open();
        let mut results = vec![];
        for hz in [30, 60, 144] {
            let mut ps = vec![Projectile::new(
                Kind::Carrier,
                Vec3::ZERO,
                Vec3::X,
                None,
                19,
            )];
            for frame in 0..hz {
                advance(&mut ps, 1. / hz as f32, false, &ctx(&world), None);
                if frame == hz / 2 {
                    ps = serde_json::from_str(&serde_json::to_string(&ps).unwrap()).unwrap();
                }
            }
            assert_eq!(ps.len(), 9);
            results.push(ps);
        }
        for ps in &results[1..] {
            for (a, b) in results[0].iter().zip(ps) {
                assert!(
                    a.position.distance(b.position) < 0.03,
                    "{:?} {:?}",
                    a.position,
                    b.position
                );
                assert!(a.velocity.distance(b.velocity) < 0.005);
                assert_eq!(a.seed, b.seed);
            }
        }
        for before in [0.099, 0.101] {
            let mut ps = vec![Projectile::new(
                Kind::Carrier,
                Vec3::ZERO,
                Vec3::X,
                None,
                19,
            )];
            advance(&mut ps, before, false, &ctx(&world), None);
            let mut restored: Vec<Projectile> =
                serde_json::from_str(&serde_json::to_string(&ps).unwrap()).unwrap();
            advance(&mut ps, 0.3, false, &ctx(&world), None);
            advance(&mut restored, 0.3, false, &ctx(&world), None);
            assert_eq!(
                serde_json::to_value(ps).unwrap(),
                serde_json::to_value(restored).unwrap()
            );
        }
    }
}
