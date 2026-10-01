//! Continuous Ice Wand contacts, temporary walls and the underwater ice shell.
use crate::{
    assets::Assets,
    combat::{self, Context, Hit, Target},
    skeletal::Transform,
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

pub(crate) const WALL_BASE: usize = 800_000_000;
pub(super) const FIRE: &str = "sound/weapon/icewand/icewand_fire.wav";
pub(super) const WALL_FIRE: &str = "sound/weapon/icewand/icewand_fire2.wav";
pub(super) const FREEZE: &str = "sound/character/shared/freeze_death.wav";
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Wall {
    pub id: u32,
    pub origin: Vec3,
    pub yaw: f32,
    pub age: f64,
    pub health: f32,
    pub melt_at: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::World;
    #[test]
    fn stream_has_range_falloff_and_no_penetration() {
        let target = Target {
            id: 1,
            center: vec3(200., 0., 50.),
            half: Vec3::splat(5.),
        };
        for blocked in [false, true] {
            let world = World::fixture(&if blocked {
                vec![(vec3(90., -100., -100.), vec3(100., 100., 200.))]
            } else {
                vec![]
            });
            let mut state = State::default();
            let (hit, _) = state.primary(
                &Context {
                    world: &world,
                    targets: &[target],
                },
                vec3(0., 0., 50.),
                Vec3::X,
            );
            assert_eq!(hit.is_none(), blocked);
            if let Some(hit) = hit {
                assert!((hit.damage - 2.75).abs() < 0.001);
                assert_eq!(hit.kind, combat::DamageKind::Ice);
            }
        }
    }
    #[test]
    fn walls_block_actors_can_be_destroyed_and_expire_across_saves() {
        let mut world = World::fixture(&[(vec3(-1000., -1000., -20.), vec3(1000., 1000., 0.))]);
        let mut state = State::default();
        assert!(state
            .place(
                &Context {
                    world: &world,
                    targets: &[]
                },
                vec3(0., 0., 48.),
                vec3(30., 0., 45.),
                Vec3::X
            )
            .is_some());
        world.set_weapon_obstacles(state.targets());
        assert!(
            world
                .sweep(vec3(0., 0., 48.), vec3(150., 0., 48.), Vec3::splat(15.))
                .fraction
                < 1.
        );
        let target = state.targets()[0];
        let behind = Target {
            id: 5,
            center: vec3(250., 0., 50.),
            half: Vec3::splat(10.),
        };
        assert!(combat::contact(
            &Context {
                world: &world,
                targets: &[behind]
            },
            vec3(0., 0., 50.),
            behind.center,
            1.
        )
        .is_none());
        assert_eq!(
            combat::contact(
                &Context {
                    world: &world,
                    targets: &[target, behind]
                },
                vec3(0., 0., 50.),
                behind.center,
                1.
            )
            .unwrap()
            .0,
            target.id
        );
        let h = Hit {
            id: target.id,
            damage: 100.,
            kind: combat::DamageKind::Ice,
            knockback: Vec3::ZERO,
        };
        assert!(!state.hit(h));
        assert_eq!(state.walls[0].health, 100.);
        assert!(state.hit(Hit {
            kind: combat::DamageKind::Knife,
            ..h
        }));
        assert!(state.targets().is_empty());
        let data = Data {
            idle: 0.1,
            rise: 1.,
            death: 1.,
        };
        state = serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        state.validate().unwrap();
        state.advance(1.01, false, &data);
        assert!(state.walls.is_empty());
        world.set_weapon_obstacles(vec![]);
        state
            .place(
                &Context {
                    world: &world,
                    targets: &[],
                },
                vec3(0., 0., 48.),
                vec3(30., 0., 45.),
                Vec3::X,
            )
            .unwrap();
        state.advance(20., true, &data);
        assert_eq!(state.walls[0].age, 0.);
        state.advance(11., false, &data);
        assert_eq!(state.targets().len(), 1);
        state.advance(0.11, false, &data);
        assert!(state.targets().is_empty());
        state.advance(1., false, &data);
        assert!(state.walls.is_empty());
    }
    #[test]
    fn downward_aim_places_a_wall_in_front_instead_of_below_the_floor() {
        let world = World::fixture(&[(vec3(-1000., -1000., -20.), vec3(1000., 1000., 0.))]);
        for pitch in [-0.24_f32, -0.35, -0.8, -1.3] {
            let mut state = State::default();
            let origin = state
                .place(
                    &Context {
                        world: &world,
                        targets: &[],
                    },
                    vec3(0., 0., 48.),
                    vec3(30., 0., 35.),
                    vec3(pitch.cos(), 0., pitch.sin()),
                )
                .expect("Downward aim must not bury the floor probe");
            assert!((origin.x - 94.).abs() < 0.001);
            assert!(origin.z >= 0. && origin.z < 0.2);
            assert_eq!(state.targets().len(), 1);
        }
    }
    #[test]
    fn wall_can_stand_across_a_shallow_step() {
        let world = World::fixture(&[
            (vec3(-1000., -1000., -20.), vec3(1000., 1000., 0.)),
            (vec3(105., -1000., 0.), vec3(1000., 1000., 6.)),
        ]);
        let mut state = State::default();
        let origin = state
            .place(
                &Context {
                    world: &world,
                    targets: &[],
                },
                vec3(0., 0., 48.),
                vec3(30., 0., 45.),
                Vec3::X,
            )
            .unwrap();
        assert!(origin.z >= 6. && origin.z < 6.2);
        let wall = state.targets()[0];
        assert!(!world.sweep(wall.center, wall.center, wall.half).start_solid);
    }
    #[test]
    fn wall_rejects_overlapping_actor_and_ceiling_and_long_session_saves_work() {
        for ceiling in [false, true] {
            let mut hulls = vec![(vec3(-1000., -1000., -20.), vec3(1000., 1000., 0.))];
            if ceiling {
                hulls.push((vec3(50., -100., 80.), vec3(180., 100., 90.)));
            }
            let world = World::fixture(&hulls);
            let actor = Target {
                id: 4,
                center: vec3(94., 0., 45.),
                half: Vec3::splat(15.),
            };
            let mut state = State::default();
            assert!(state
                .place(
                    &Context {
                        world: &world,
                        targets: if ceiling {
                            &[]
                        } else {
                            std::slice::from_ref(&actor)
                        }
                    },
                    vec3(0., 0., 48.),
                    vec3(30., 0., 45.),
                    Vec3::X
                )
                .is_none());
            state.clock = 3600.123456;
            state.pulse(
                Transform {
                    translation: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                false,
            );
            state.validate().unwrap();
        }
    }
    #[test]
    fn water_shell_locks_and_releases_once_after_restore() {
        let mut state = State::default();
        let data = Data {
            idle: 0.1,
            rise: 1.,
            death: 1.,
        };
        assert!(state.underwater(Vec3::ZERO, Vec3::X));
        assert!(!state.underwater(Vec3::ZERO, Vec3::X));
        assert!(!state.locked());
        assert_eq!(state.advance(0.66, false, &data).len(), 1);
        assert!(state.locked());
        state = serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        state.validate().unwrap();
        assert!(state.advance(6.8, false, &data).is_empty());
        assert!(state.locked());
        assert_eq!(state.advance(0.05, false, &data).len(), 1);
        assert!(!state.locked());
        state.advance(4., false, &data);
        assert!(state.shell.is_none());
    }
}
impl Wall {
    pub fn target(&self) -> Target {
        Target {
            id: WALL_BASE + self.id as usize,
            // setsize supplies world bounds; the 1.8 model scale applies only
            // to the TAN artwork (whose standing height is about 124 units).
            center: self.origin + Vec3::Z * 59.,
            half: vec3(24., 24., 59.),
        }
    }
    pub fn pose(&self) -> Transform {
        Transform {
            translation: self.origin,
            rotation: Quat::from_rotation_z(self.yaw),
        }
    }
}
pub(super) struct Data {
    pub idle: f64,
    pub rise: f64,
    pub death: f64,
}
impl Data {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let duration = |a: &mut Assets, clip: &str| -> Result<f64> {
            let (_, m) = super::read_model_clip(a, "fx_icewall", Some(clip))?;
            Ok(m.surfaces[0].frames.len() as f64 * m.frame_time as f64)
        };
        Ok(Self {
            idle: duration(assets, "aidle")?,
            rise: duration(assets, "rise")?,
            death: duration(assets, "death")?,
        })
    }
    pub fn stage(&self, wall: &Wall) -> (usize, f32, bool) {
        if let Some(at) = wall.melt_at {
            (3, (wall.age - at) as f32, false)
        } else if wall.age < self.idle {
            (0, wall.age as f32, false)
        } else if wall.age < self.idle + self.rise {
            (1, (wall.age - self.idle) as f32, false)
        } else {
            (2, (wall.age - self.idle - self.rise) as f32, true)
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Shell {
    pub origin: Vec3,
    pub age: f64,
    pub yaw: f32,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct State {
    pub walls: Vec<Wall>,
    pub shell: Option<Shell>,
    pub clock: f64,
    pub trail: super::trail::Trail,
    /// Intervals of actual paid stream emission or alternate startup.
    pub pulses: Vec<(f64, f64, bool)>,
    next_id: u32,
    sound_at: f64,
}
impl State {
    pub fn validate(&self) -> Result<()> {
        let mut ids = std::collections::BTreeSet::new();
        ensure!(
            self.clock.is_finite()
                && self.clock >= 0.
                && self.sound_at.is_finite()
                && self.trail.valid(self.clock)
                && self.walls.len() <= 32
                && self.pulses.len() <= 200
                && self.pulses.iter().all(|&(a, b, _)| a.is_finite()
                    && b.is_finite()
                    && a >= 0.
                    && a <= b
                    && b <= self.clock + 1.1)
                && self.walls.iter().all(|w| ids.insert(w.id)
                    && w.origin.is_finite()
                    && w.yaw.is_finite()
                    && (0. ..=120.).contains(&w.age)
                    && (0. ..=100.).contains(&w.health)
                    && w.melt_at
                        .is_none_or(|a| a.is_finite() && a >= 0. && a <= w.age))
                && self.shell.as_ref().is_none_or(|s| s.origin.is_finite()
                    && s.yaw.is_finite()
                    && (0. ..=11.).contains(&s.age)),
            "Invalid saved Ice Wand state"
        );
        Ok(())
    }
    pub fn locked(&self) -> bool {
        self.shell
            .as_ref()
            .is_some_and(|s| s.age >= 0.65 && s.age < 7.5)
    }
    pub fn underwater(&mut self, origin: Vec3, aim: Vec3) -> bool {
        if self.shell.is_some() {
            return false;
        }
        self.shell = Some(Shell {
            origin,
            age: 0.,
            yaw: aim.y.atan2(aim.x),
        });
        true
    }
    pub fn pulse(&mut self, pose: Transform, alternate: bool) {
        self.trail.record(self.clock as f32, pose);
        let until = self.clock + if alternate { 1.05 } else { 0.05 };
        if let Some(last) = self
            .pulses
            .last_mut()
            .filter(|p| p.2 == alternate && p.1 + 0.00001 >= self.clock)
        {
            last.1 = until;
        } else {
            self.pulses.push((self.clock, until, alternate));
        }
    }
    pub fn flowing(&self, at: f32, alternate: bool) -> bool {
        self.pulses
            .iter()
            .any(|&(a, b, alt)| alt == alternate && at as f64 + 1e-6 >= a && (at as f64) < b)
    }
    pub fn stop(&mut self) {
        for p in &mut self.pulses {
            p.1 = p.1.min(self.clock);
        }
    }
    pub fn primary(
        &mut self,
        ctx: &Context<'_>,
        origin: Vec3,
        aim: Vec3,
    ) -> (Option<Hit>, Option<(&'static str, Vec3)>) {
        let end = origin + aim * 400.;
        let hit = combat::contact_box(ctx, origin, end, Vec3::splat(15.));
        let damage = hit.map(|(id, f)| Hit {
            id,
            damage: 5. * (1. - f),
            kind: combat::DamageKind::Ice,
            knockback: aim * 5.,
        });
        let wall = ctx.world.sweep(origin, end, Vec3::splat(15.));
        let sound = if (hit.is_some() || wall.fraction < 1.) && self.clock + 1e-8 >= self.sound_at {
            self.sound_at = self.clock + 0.15;
            Some((
                if hit.is_some() {
                    "sound/weapon/icewand/icewand_flesh1.wav"
                } else {
                    "sound/weapon/icewand/icewand_world1.wav"
                },
                origin.lerp(end, hit.map_or(wall.fraction, |(_, f)| f)),
            ))
        } else {
            None
        };
        (damage, sound)
    }
    pub fn place(
        &mut self,
        ctx: &Context<'_>,
        eye: Vec3,
        muzzle: Vec3,
        mut aim: Vec3,
    ) -> Option<Vec3> {
        if self.walls.len() >= 32 {
            return None;
        }
        // The wall grows ahead of Alice even while she aims at the floor.
        // Keep upward aim, but flatten downward aim before the floor probe.
        if aim.z < 0. {
            aim.z = 0.;
            aim = aim.normalize_or_zero();
        }
        let end = muzzle + aim * 64.;
        let clearance = ctx.world.sweep(eye, end, Vec3::splat(8.));
        if clearance.start_solid || clearance.fraction < 1. {
            return None;
        }
        // Support the wall's whole footprint. A narrow probe can land below
        // a slope or step under its corners, causing the clearance check to
        // reject an otherwise usable placement on uneven arena floors.
        let floor = ctx
            .world
            .sweep(end, end - Vec3::Z * 96., vec3(24., 24., 8.));
        if floor.start_solid || floor.fraction >= 1. || floor.normal.z < 0.7 {
            return None;
        }
        let origin = end - Vec3::Z * (96. * floor.fraction + 8. - 0.05);
        let wall = Wall {
            id: self.next_id,
            origin,
            yaw: aim.y.atan2(aim.x),
            age: 0.,
            health: 100.,
            melt_at: None,
        };
        let t = wall.target();
        // A wall may not grow through the ceiling, Alice, an enemy or another wall.
        let clear = ctx.world.sweep(t.center, t.center, t.half);
        if clear.start_solid
            || ctx
                .targets
                .iter()
                .any(|a| (a.center - t.center).abs().cmplt(a.half + t.half).all())
            || (eye - Vec3::Z * 20. - t.center)
                .abs()
                .cmplt(t.half + vec3(15., 15., 28.))
                .all()
        {
            return None;
        }
        self.next_id = self.next_id.wrapping_add(1);
        self.walls.push(wall);
        Some(origin)
    }
    pub fn hit(&mut self, hit: Hit) -> bool {
        let Some(w) = self
            .walls
            .iter_mut()
            .find(|w| w.id as usize + WALL_BASE == hit.id)
        else {
            return false;
        };
        if hit.kind != combat::DamageKind::Ice
            && hit.damage.is_finite()
            && hit.damage > 0.
            && w.melt_at.is_none()
        {
            w.health = (w.health - hit.damage).max(0.);
            if w.health == 0. {
                w.melt_at = Some(w.age);
                return true;
            }
        }
        false
    }
    pub fn targets(&self) -> Vec<Target> {
        self.walls
            .iter()
            .filter(|w| w.melt_at.is_none())
            .map(Wall::target)
            .collect()
    }
    #[cfg(test)]
    pub fn advance(&mut self, dt: f32, frozen: bool, data: &Data) -> Vec<(&'static str, Vec3)> {
        self.advance_clocks(dt, if frozen { 0. } else { dt }, data)
    }
    pub fn advance_clocks(
        &mut self,
        dt: f32,
        world_dt: f32,
        data: &Data,
    ) -> Vec<(&'static str, Vec3)> {
        let mut sounds = vec![];
        self.clock += dt as f64;
        self.pulses.retain(|p| p.1 > self.clock - 5.1);
        for w in &mut self.walls {
            let old = w.age;
            w.age += world_dt.clamp(0., dt) as f64;
            if old < data.idle && w.age >= data.idle {
                sounds.push(("sound/weapon/icewand/icewand_icewall.wav", w.origin));
            }
            if w.melt_at.is_none() && w.age >= data.idle + data.rise + 10. {
                w.melt_at = Some(data.idle + data.rise + 10.);
                w.health = 0.;
                sounds.push(("sound/weapon/icewand/icewand_icewall_death.wav", w.origin));
            }
        }
        self.walls
            .retain(|w| w.melt_at.is_none_or(|at| w.age < at + data.death));
        if let Some(s) = &mut self.shell {
            let old = s.age;
            s.age += dt as f64;
            if old < 0.65 && s.age >= 0.65 {
                sounds.push((FREEZE, s.origin));
            }
            if old < 7.5 && s.age >= 7.5 {
                sounds.push(("sound/character/shared/unfreeze.wav", s.origin));
            }
            if s.age >= 11. {
                self.shell = None;
            }
        }
        sounds
    }
}
