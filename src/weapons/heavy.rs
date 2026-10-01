//! Eye Staff and cannon simulation. Values are observations of the local TIKIs
//! and native class events; no original code is executed by this implementation.
use super::{blast, trail::Trail};
use crate::{
    combat::{self, Context, Hit, Target},
    inventory::Stats,
    skeletal::Transform,
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

pub(super) const CHARGE: &str = "sound/weapon/staff/charge1.wav";
pub(super) const BEAM: &str = "sound/weapon/staff/beam_loop.wav";
pub(super) const OFF: &str = "sound/weapon/staff/charge_off.wav";
pub(super) const LIFT: &str = "sound/weapon/staff/liftoff.wav";
pub(super) const EXPLODE: &str = "sound/weapon/staff/explode1.wav";
pub(super) const BUSS: &str = "sound/weapon/blunderbuss/bb_fire.wav";
pub(super) const BUSS_LOOP: &str = "sound/weapon/blunderbuss/bb_loop.wav";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Kind {
    Spiral,
    Comet,
    Cannon,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Shot {
    pub id: u32,
    pub kind: Kind,
    pub position: Vec3,
    pub velocity: Vec3,
    pub age: f64,
    pub trail: Trail,
}
impl Shot {
    fn valid(&self) -> bool {
        self.position.is_finite()
            && self.velocity.is_finite()
            && (self.velocity.length()
                - if self.kind == Kind::Cannon {
                    2500.
                } else {
                    1000.
                })
            .abs()
                < 1.
            && (0. ..=self.life()).contains(&self.age)
            && self.trail.valid(self.age)
    }
    pub fn pose(&self) -> Transform {
        Transform {
            translation: self.position,
            rotation: Quat::from_rotation_arc(Vec3::X, self.velocity.normalize_or_zero())
                * Quat::from_rotation_x(
                    self.age as f32
                        * match self.kind {
                            Kind::Cannon => {
                                let seed = self.id.wrapping_mul(214013).wrapping_add(2531011);
                                ((((seed >> 16) & 32767) as f32 / 16384. - 1.) * 200.).to_radians()
                            }
                            Kind::Spiral => 1100_f32.to_radians(),
                            Kind::Comet => 0.,
                        },
                ),
        }
    }
    fn life(&self) -> f64 {
        if self.kind == Kind::Cannon {
            5.
        } else {
            2.5
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Charge {
    pub id: u32,
    pub alternate: bool,
    pub age: f64,
    pub tick: u32,
    pub pose: Transform,
    pub end: Vec3,
    pub trail: Trail,
    pub ground: Vec3,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Volley {
    pub id: u32,
    pub origin: Vec3,
    pub duration: f64,
    pub age: f64,
    pub next: u32,
    pub previous: Option<usize>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Impact {
    pub id: u32,
    pub kind: Kind,
    pub origin: Vec3,
    pub normal: Vec3,
    pub actor: bool,
    pub age: f64,
    #[serde(default)]
    pub source: Option<Shot>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct State {
    pub charge: Option<Charge>,
    pub volleys: Vec<Volley>,
    pub shots: Vec<Shot>,
    pub impacts: Vec<Impact>,
    pub ignition: Option<(f64, Transform)>,
    serial: u32,
    seed: u32,
}
#[derive(Default)]
pub(super) struct Events {
    pub hits: Vec<Hit>,
    pub sounds: Vec<(&'static str, Vec3)>,
    pub impacts: u64,
}
fn valid_pose(p: Transform) -> bool {
    p.translation.is_finite()
        && p.rotation.is_finite()
        && (p.rotation.length_squared() - 1.).abs() < 0.01
}
impl State {
    fn id(&mut self) -> u32 {
        self.serial = self.serial.wrapping_add(1);
        self.serial
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.charge.as_ref().is_none_or(|c| c.age.is_finite()
                && c.age >= 0.
                && c.age < 86400.
                && c.tick < 2_000_000
                && c.ground.is_finite()
                && (if c.alternate {
                    (c.tick + 1) as f64 * 0.05
                } else {
                    2.3 + c.tick as f64 * 0.05
                }) >= c.age - 0.00001
                && (if c.alternate {
                    (c.tick + 1) as f64 * 0.05
                } else {
                    2.3 + c.tick as f64 * 0.05
                }) <= (c.age + 0.051).max(2.3)
                && valid_pose(c.pose)
                && c.end.is_finite()
                && c.trail.valid(c.age)),
            "Invalid Staff charge"
        );
        ensure!(
            self.volleys.len() <= 32
                && self
                    .volleys
                    .iter()
                    .all(|v| (0.5..=5.1).contains(&v.duration)
                        && (0. ..=13.).contains(&v.age)
                        && (1..=16).contains(&v.next)
                        && v.origin.is_finite()),
            "Invalid Staff volley"
        );
        ensure!(
            self.shots.len() <= 128 && self.shots.iter().all(Shot::valid),
            "Invalid heavy projectile"
        );
        ensure!(
            self.impacts.len() <= 128
                && self.impacts.iter().all(|p| p.origin.is_finite()
                    && p.normal.is_finite()
                    && (0. ..=5.).contains(&p.age)
                    && p.source.as_ref().is_none_or(Shot::valid)),
            "Invalid heavy impact"
        );
        ensure!(
            self.ignition
                .is_none_or(|(age, p)| (0. ..=4.).contains(&age) && valid_pose(p)),
            "Invalid cannon ignition"
        );
        Ok(())
    }
    pub fn start(&mut self, alternate: bool, pose: Transform) {
        if self.charge.is_some() {
            return;
        }
        let id = self.id();
        let mut trail = Trail::default();
        trail.record(0., pose);
        self.charge = Some(Charge {
            id,
            alternate,
            age: 0.,
            tick: 0,
            pose,
            end: pose.translation,
            trail,
            ground: pose.translation,
        });
    }
    pub fn release(&mut self) -> Vec<(&'static str, Vec3)> {
        let Some(c) = self.charge.take() else {
            return vec![];
        };
        let mut sounds = vec![(OFF, c.pose.translation)];
        if c.alternate {
            if c.age > 0.5 + 1e-6 {
                self.volleys.push(Volley {
                    id: c.id,
                    origin: c.ground,
                    duration: c.age.min(5.05),
                    age: 0.,
                    next: 1,
                    previous: None,
                });
            }
        } else if c.age + 1e-6 >= 2.3 {
            self.launch(Kind::Spiral, c.pose.translation, c.pose.rotation * Vec3::X);
            sounds.push((LIFT, c.pose.translation));
        }
        sounds
    }
    pub fn launch(&mut self, kind: Kind, origin: Vec3, direction: Vec3) {
        let id = self.id();
        let mut p = Shot {
            id,
            kind,
            position: origin,
            velocity: direction.try_normalize().unwrap_or(Vec3::X)
                * if kind == Kind::Cannon { 2500. } else { 1000. },
            age: 0.,
            trail: Default::default(),
        };
        p.trail.record(0., p.pose());
        self.shots.push(p);
    }
    pub fn advance(
        &mut self,
        dt: f32,
        ctx: &Context<'_>,
        owner: Target,
        pose: Transform,
        _stopped: bool,
        mut wallet: Option<&mut Stats>,
    ) -> Events {
        let mut out = Events::default();
        if !dt.is_finite() || dt <= 0. {
            return out;
        }
        let mut remaining = dt as f64;
        // Fixed boundaries, including births, keep combat independent of render rate.
        while remaining > 1e-8 {
            let mut step = remaining.min(1. / 120.);
            if let Some(c) = &self.charge {
                let next = if c.alternate {
                    (c.tick + 1) as f64 * 0.05
                } else {
                    2.3 + c.tick as f64 * 0.05
                };
                step = step.min((next - c.age).max(0.));
            }
            for v in &self.volleys {
                if (v.next as f64) < v.duration * 3. - 1e-6 {
                    step = step.min((5. + v.next as f64 * 0.333 - v.age).max(0.));
                }
            }
            for p in &mut self.impacts {
                p.age += step;
            }
            self.impacts.retain(|p| p.age < 4.);
            if let Some((age, _)) = &mut self.ignition {
                *age += step;
                if *age >= 4. {
                    self.ignition = None;
                }
            }
            for p in &mut self.shots {
                let t = step.min(p.life() - p.age).max(0.);
                let end = p.position + p.velocity * t as f32;
                let half = if p.kind == Kind::Cannon {
                    Vec3::splat(8.)
                } else {
                    Vec3::ZERO
                };
                let wall = ctx.world.sweep(p.position, end, half);
                let contact = combat::contact_box(ctx, p.position, end, half);
                p.age += t;
                p.position = p.position.lerp(
                    end,
                    if wall.start_solid {
                        0.
                    } else {
                        contact.map_or(wall.fraction, |(_, f)| f)
                    },
                );
                p.trail.record(p.age as f32, p.pose());
                let expired = p.age + 1e-8 >= p.life();
                if contact.is_some() || wall.start_solid || wall.fraction < 1. || expired {
                    let dir = p.velocity.normalize_or_zero();
                    // Cannonball::Touch bypasses Projectile::Touch, so its TIKI
                    // hitdamage 8 is unused. The explosion excludes its OWNER.
                    if let Some((id, _)) = contact.filter(|_| p.kind != Kind::Cannon) {
                        out.hits.push(Hit {
                            id,
                            damage: if p.kind == Kind::Spiral { 150. } else { 100. },
                            kind: combat::DamageKind::EyeStaff,
                            knockback: dir * 400.,
                        });
                    }
                    let origin = p.position - dir * 36.;
                    let kind = if p.kind == Kind::Cannon {
                        blast::Kind::Cannon
                    } else {
                        blast::Kind::Staff
                    };
                    let excluded = if p.kind == Kind::Cannon && !expired {
                        Some(owner.id)
                    } else {
                        contact.map(|(id, _)| id)
                    };
                    out.hits
                        .extend(blast::Blast::new(kind, origin, p.id).damage(ctx, owner, excluded));
                    self.impacts.push(Impact {
                        id: p.id,
                        kind: p.kind,
                        origin: p.position,
                        normal: if contact.is_some() || wall.start_solid || expired {
                            -dir
                        } else {
                            wall.normal
                        },
                        actor: contact.is_some(),
                        age: 0.,
                        source: Some(p.clone()),
                    });
                    out.sounds.push((
                        if p.kind == Kind::Cannon {
                            "sound/weapon/blunderbuss/bb_explode.wav"
                        } else {
                            EXPLODE
                        },
                        origin,
                    ));
                    out.impacts += 1;
                    p.age = p.life();
                }
            }
            self.shots.retain(|p| p.age + 1e-8 < p.life());
            for v in &mut self.volleys {
                v.age += step;
            }
            for i in 0..self.volleys.len() {
                let v = &mut self.volleys[i];
                if v.next as f64 >= v.duration * 3. - 1e-6
                    || v.age + 1e-7 < 5. + v.next as f64 * 0.333
                {
                    continue;
                }
                let direction = pose.rotation * Vec3::X;
                let mut targets: Vec<_> = ctx
                    .targets
                    .iter()
                    .filter(|t| {
                        let offset = t.center - owner.center;
                        t.id != crate::dice::ALICE
                            && t.id != crate::dice::SUMMON
                            && !(crate::interaction::SHOT_BASE..crate::encounters::BASE)
                                .contains(&t.id)
                            && t.id < super::ice::WALL_BASE
                            && offset.length() <= 2048.
                            && offset.normalize_or_zero().dot(direction)
                                > std::f32::consts::FRAC_1_SQRT_2
                            && ctx.world.sweep(owner.center, t.center, Vec3::ZERO).fraction >= 1.
                    })
                    .collect();
                targets.sort_by(|a, b| {
                    let alignment =
                        |t: &Target| (t.center - owner.center).normalize_or_zero().dot(direction);
                    alignment(b).total_cmp(&alignment(a)).then(a.id.cmp(&b.id))
                });
                let next = v
                    .previous
                    .and_then(|id| targets.iter().position(|t| t.id == id))
                    .map_or(0, |i| i + 1);
                let target = (!targets.is_empty()).then(|| targets[next % targets.len()]);
                v.previous = target.map(|t| t.id);
                v.next += 1;
                let random = |seed: &mut u32| {
                    *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
                    ((*seed >> 16) & 32767) as f32 / 16384. - 1.
                };
                let point = target.map_or_else(
                    || {
                        let f = ctx
                            .world
                            .sweep(owner.center, owner.center + direction * 800., Vec3::ZERO)
                            .fraction;
                        owner.center
                            + direction * 750. * f
                            + vec3(random(&mut self.seed), random(&mut self.seed), 0.) * 200.
                    },
                    |t| t.center,
                );
                let down = vec3(
                    random(&mut self.seed) * 0.7,
                    random(&mut self.seed) * 0.7,
                    -1.,
                )
                .normalize();
                let ceiling = ctx.world.sweep(point, point - down * 1000., Vec3::ZERO);
                if !ceiling.start_solid {
                    self.launch(Kind::Comet, point - down * (ceiling.fraction * 950.), down);
                    out.sounds.push((LIFT, point));
                }
            }
            // Keep the last upward cosmetic comet and its 2.5 s smoke tail
            // after all damaging comet births have completed.
            self.volleys.retain(|v| v.age < v.duration + 7.5);
            let mut release = false;
            if let Some(c) = &mut self.charge {
                c.age += step;
                c.ground = owner.center - Vec3::Z * 28.;
                c.pose = pose;
                c.trail.record(c.age as f32, pose);
                let next = if c.alternate {
                    (c.tick + 1) as f64 * 0.05
                } else {
                    2.3 + c.tick as f64 * 0.05
                };
                if c.age + 1e-7 >= next {
                    let cost: f32 = if c.alternate { 1. } else { 0.4 };
                    let accepted = wallet
                        .as_deref_mut()
                        .is_none_or(|s| s.will() > 0. && s.spend_will(cost.min(s.will())) || s.god);
                    if accepted {
                        c.tick += 1;
                        if !c.alternate {
                            let end = pose.translation + pose.rotation * Vec3::X * 2000.;
                            let wall = ctx.world.sweep(pose.translation, end, Vec3::ZERO);
                            let hit = combat::contact(ctx, pose.translation, end, 0.);
                            c.end = pose
                                .translation
                                .lerp(end, hit.map_or(wall.fraction, |(_, f)| f));
                            if let Some((id, _)) = hit {
                                out.hits.push(Hit {
                                    id,
                                    damage: 5.,
                                    kind: combat::DamageKind::EyeStaff,
                                    knockback: pose.rotation * Vec3::X * 5.,
                                });
                            }
                        }
                    } else {
                        release = true;
                    }
                    if (c.alternate && c.age > 5. + 1e-7)
                        || wallet.as_deref().is_some_and(|s| !s.god && s.will() < 0.4)
                    {
                        release = true;
                    }
                }
            }
            if release {
                out.sounds.extend(self.release());
            }
            remaining -= step;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::World;
    fn pose() -> Transform {
        Transform {
            translation: vec3(0., 0., 50.),
            rotation: Quat::IDENTITY,
        }
    }
    fn owner() -> Target {
        Target {
            id: crate::dice::ALICE,
            center: Vec3::ZERO,
            half: Vec3::splat(15.),
        }
    }
    fn roundtrip(s: &State) -> State {
        let s: State = serde_json::from_str(&serde_json::to_string(s).unwrap()).unwrap();
        s.validate().unwrap();
        s
    }
    #[test]
    fn staff_charge_release_pulses_cost_and_restart_are_rate_independent() {
        let world = World::fixture(&[]);
        let targets = [Target {
            id: 1,
            center: vec3(300., 0., 50.),
            half: Vec3::splat(10.),
        }];
        let ctx = Context {
            world: &world,
            targets: &targets,
        };
        for hz in [30, 60, 144] {
            let mut s = State::default();
            let mut wallet = Stats::weapon_preview();
            s.start(false, pose());
            let mut damage = 0.;
            for i in 0..hz * 3 {
                let e = s.advance(
                    1. / hz as f32,
                    &ctx,
                    owner(),
                    pose(),
                    false,
                    Some(&mut wallet),
                );
                damage += e.hits.iter().map(|h| h.damage).sum::<f32>();
                if i % 17 == 0 {
                    s = roundtrip(&s);
                }
            }
            assert!((damage - 75.).abs() < 0.01, "{hz}: {damage}");
            assert!(
                (wallet.will() - 94.).abs() < 0.002,
                "{hz}: {}",
                wallet.will()
            );
            let before = serde_json::to_value(&s).unwrap();
            assert!(s
                .advance(0., &ctx, owner(), pose(), false, Some(&mut wallet))
                .hits
                .is_empty());
            assert_eq!(before, serde_json::to_value(&s).unwrap());
            s.release();
            assert_eq!(s.shots.len(), 1);
            s.release();
            assert_eq!(s.shots.len(), 1);
            let e = s.advance(0.4, &ctx, owner(), pose(), false, Some(&mut wallet));
            assert_eq!(e.hits.iter().find(|h| h.id == 1).unwrap().damage, 150.);
        }
        let mut s = State::default();
        s.start(false, pose());
        s.advance(2.29, &ctx, owner(), pose(), false, None);
        s.release();
        assert!(s.shots.is_empty());
    }
    #[test]
    fn cannon_is_one_ball_with_owner_excluded_radius_damage_and_no_pellets() {
        let world = World::fixture(&[(vec3(300., -1000., -1000.), vec3(310., 1000., 1000.))]);
        let targets = [Target {
            id: 1,
            center: vec3(280., 0., 50.),
            half: Vec3::splat(5.),
        }];
        let ctx = Context {
            world: &world,
            targets: &targets,
        };
        let mut s = State::default();
        s.launch(Kind::Cannon, pose().translation, Vec3::X);
        s.advance(0.05, &ctx, owner(), pose(), true, None);
        assert!((s.shots[0].position.x - 125.).abs() < 0.01);
        s = roundtrip(&s);
        let e = s.advance(0.1, &ctx, owner(), pose(), true, None);
        assert_eq!(e.impacts, 1);
        assert!(s.shots.is_empty());
        assert_eq!(e.hits.len(), 1);
        assert_eq!(e.hits[0].id, 1);
        assert!(e.hits[0].damage > 900.);
        assert!(e.hits[0].knockback.length() > 700.);
        assert!(s
            .advance(1., &ctx, owner(), pose(), true, None)
            .hits
            .is_empty());
    }
    #[test]
    fn comets_charge_cost_delay_limit_and_saved_randomness() {
        let world = World::fixture(&[(vec3(-5000., -5000., -20.), vec3(5000., 5000., 0.))]);
        let targets = [Target {
            id: 1,
            center: vec3(300., 0., 50.),
            half: Vec3::splat(25.),
        }];
        let ctx = Context {
            world: &world,
            targets: &targets,
        };
        let mut s = State::default();
        let mut wallet = Stats::weapon_preview();
        s.start(true, pose());
        s.advance(2., &ctx, owner(), pose(), true, Some(&mut wallet));
        assert_eq!(wallet.will(), 60.);
        s.release();
        assert_eq!(s.volleys.len(), 1);
        s.advance(5.2, &ctx, owner(), pose(), true, None);
        assert!(s.shots.is_empty());
        let mut loaded = roundtrip(&s);
        let mut damage = 0.;
        for _ in 0..180 {
            let e = s.advance(1. / 60., &ctx, owner(), pose(), true, None);
            damage += e
                .hits
                .iter()
                .filter(|h| h.id == 1)
                .map(|h| h.damage)
                .sum::<f32>();
            loaded.advance(1. / 60., &ctx, owner(), pose(), true, None);
        }
        assert_eq!(
            serde_json::to_value(&s).unwrap(),
            serde_json::to_value(&loaded).unwrap()
        );
        assert_eq!(damage, 500.);
        s = State::default();
        s.start(true, pose());
        s.advance(5.1, &ctx, owner(), pose(), false, None);
        assert!(s.charge.is_none());
        assert_eq!(s.volleys.len(), 1);
    }
    #[test]
    fn wall_blocks_beam_god_is_free_and_spiral_continues_during_watch() {
        let world = World::fixture(&[(vec3(100., -1000., -1000.), vec3(110., 1000., 1000.))]);
        let targets = [Target {
            id: 1,
            center: vec3(300., 0., 50.),
            half: Vec3::splat(5.),
        }];
        let ctx = Context {
            world: &world,
            targets: &targets,
        };
        let mut s = State::default();
        let mut wallet = Stats::weapon_preview();
        wallet.god = true;
        s.start(false, pose());
        assert!(s
            .advance(3., &ctx, owner(), pose(), true, Some(&mut wallet))
            .hits
            .is_empty());
        assert_eq!(wallet.will(), 100.);
        assert!(s.charge.as_ref().unwrap().end.x <= 100.);
        s.release();
        assert_eq!(s.advance(1., &ctx, owner(), pose(), true, None).impacts, 1);
        assert!(s.shots.is_empty());
        assert_eq!(
            s.advance(0.2, &ctx, owner(), pose(), false, None).impacts,
            0
        );
    }
    #[test]
    fn lifetime_explosions_are_saved_once_and_empty_will_stops_channels() {
        let world = World::fixture(&[]);
        let ctx = Context {
            world: &world,
            targets: &[],
        };
        for (kind, life) in [(Kind::Spiral, 2.5), (Kind::Comet, 2.5), (Kind::Cannon, 5.)] {
            let mut s = State::default();
            s.launch(kind, vec3(0., 0., 3000.), Vec3::X);
            s.advance(life - 0.01, &ctx, owner(), pose(), false, None);
            s = roundtrip(&s);
            assert_eq!(
                s.advance(0.02, &ctx, owner(), pose(), false, None).impacts,
                1
            );
            s = roundtrip(&s);
            assert_eq!(
                s.advance(0.1, &ctx, owner(), pose(), false, None).impacts,
                0
            );
        }
        for alternate in [false, true] {
            let mut s = State::default();
            let mut wallet = Stats::weapon_preview();
            s.start(alternate, pose());
            s.advance(
                if alternate { 0.75 } else { 2.5 },
                &ctx,
                owner(),
                pose(),
                false,
                Some(&mut wallet),
            );
            wallet.spend_will(wallet.will());
            s.advance(0.1, &ctx, owner(), pose(), false, Some(&mut wallet));
            assert!(s.charge.is_none());
            assert_eq!(wallet.will(), 0.);
            assert_eq!(s.volleys.len(), usize::from(alternate));
            assert_eq!(s.shots.len(), usize::from(!alternate));
        }
    }
    #[test]
    fn comets_cycle_aim_ranked_visible_enemies_after_restoration() {
        let world = World::fixture(&[(vec3(80., 40., -100.), vec3(90., 300., 300.))]);
        let target = |id, x, y| Target {
            id,
            center: vec3(x, y, 50.),
            half: Vec3::splat(5.),
        };
        let targets = [
            target(1, 300., -100.), // eligible, but less aligned
            target(2, 300., 0.),
            target(3, 300., 300.), // occluded
            target(4, -300., 0.),  // behind
            target(5, 2500., 0.),  // too far
            target(crate::dice::SUMMON, 100., 0.),
            target(crate::interaction::SHOT_BASE, 90., 0.),
        ];
        let ctx = Context {
            world: &world,
            targets: &targets,
        };
        let mut s = State::default();
        s.start(true, pose());
        s.advance(2., &ctx, owner(), pose(), false, None);
        s.release();
        s.advance(5.34, &ctx, owner(), pose(), false, None);
        assert_eq!(s.volleys[0].previous, Some(2));
        s = roundtrip(&s);
        s.advance(0.333, &ctx, owner(), pose(), false, None);
        assert_eq!(s.volleys[0].previous, Some(1));
        s = roundtrip(&s);
        s.advance(0.333, &ctx, owner(), pose(), false, None);
        assert_eq!(s.volleys[0].previous, Some(2));
    }
    #[test]
    fn cannon_overlapped_by_moving_world_explodes_once() {
        let world = World::fixture(&[(vec3(-100., -100., -100.), vec3(100., 100., 100.))]);
        let ctx = Context {
            world: &world,
            targets: &[],
        };
        let mut s = State::default();
        s.launch(Kind::Cannon, pose().translation, Vec3::X);
        assert_eq!(
            s.advance(0.05, &ctx, owner(), pose(), false, None).impacts,
            1
        );
        assert!(s.shots.is_empty());
        s = roundtrip(&s);
        assert_eq!(
            s.advance(0.05, &ctx, owner(), pose(), false, None).impacts,
            0
        );
    }
}
