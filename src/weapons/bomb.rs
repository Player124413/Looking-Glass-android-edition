//! Saved Jackbomb motion, animation-owned flame releases and fuse events.
use super::blast::{Blast, Kind as BlastKind};
use crate::{
    assets::Assets,
    combat::{self, Context, Hit, Target},
    skeletal::{Definition, Transform},
    tan,
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

pub(super) const MUSIC: &str = "sound/weapon/jackbomb/jackbomb_music.wav";
pub(super) const POP: &str = "sound/weapon/jackbomb/jackbomb_pop.wav";
pub(super) const BREATH: &str = "sound/weapon/jackbomb/jackbomb_breath.wav";
pub(super) const EXPLODE: &str = "sound/weapon/jackbomb/jackbomb_explode.wav";
pub(super) const TOSS: &str = "sound/weapon/jackbomb/jackbomb_toss.wav";

pub(super) struct Data {
    pub clips: Vec<tan::Model>,
    pub crank: f64,
    pub open: f64,
    pub rotation: f64,
    pub flame_events: Vec<(f64, f32)>,
}
impl Data {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let mut clips = Vec::new();
        for (model, clip) in [
            ("prj_jackbomb", "idle"),
            ("prj_jackbomb", "open"),
            ("prj_jackbomb", "spring"),
            ("prj_jackalt", "spring"),
        ] {
            let d = Definition::load(assets, &format!("models/{model}.tik"))?;
            clips.push(tan::Model::parse(
                &assets.read(&format!("{}/{}", d.path, d.animations[clip]))?,
            )?);
        }
        let duration =
            |i: usize| clips[i].frame_time as f64 * clips[i].surfaces[0].frames.len() as f64;
        let crank = duration(0) - clips[0].frame_time as f64;
        let open = duration(1) - clips[1].frame_time as f64;
        let rotation = duration(3);
        let text = String::from_utf8(assets.read("models/prj_jackalt.tik")?)?;
        let mut flame_events = Vec::new();
        for t in crate::materials::lines(&text) {
            if t.len() >= 5 && t[1] == "proj" && t[2] == "tag_mouth" {
                flame_events.push((
                    t[0].parse::<u32>()? as f64 * clips[3].frame_time as f64,
                    t[4].parse::<f32>()?,
                ));
            }
        }
        flame_events.sort_by(|a, b| a.0.total_cmp(&b.0));
        ensure!(
            flame_events.len() == 22 && (rotation - 4.05).abs() < 0.001,
            "Changed Jackbomb rotation events"
        );
        Ok(Self {
            clips,
            crank,
            open,
            rotation,
            flame_events,
        })
    }
    pub fn stage(&self, bomb: &Bomb) -> (usize, f32, bool) {
        if bomb.age < self.crank {
            (0, bomb.age as f32, false)
        } else if bomb.age < self.crank + self.open {
            (1, (bomb.age - self.crank) as f32, false)
        } else {
            (
                if bomb.alternate { 3 } else { 2 },
                (bomb.age - self.crank - self.open) as f32,
                true,
            )
        }
    }
    pub fn tag(&self, bomb: &Bomb, name: &str) -> Transform {
        let (i, time, looping) = self.stage(bomb);
        let model = &self.clips[i];
        let (Some(points), Some(rotations)) = (model.tags.get(name), model.tag_rotations.get(name))
        else {
            return bomb.pose();
        };
        let f = time / model.frame_time;
        let f = if looping {
            f.rem_euclid(points.len() as f32)
        } else {
            f.min(points.len() as f32 - 1.)
        };
        let a = f.floor() as usize;
        let b = if looping {
            (a + 1) % points.len()
        } else {
            (a + 1).min(points.len() - 1)
        };
        let pose = bomb.trail.sample(bomb.age as f32, bomb.pose());
        Transform {
            translation: pose.point(points[a].lerp(points[b], f.fract())),
            rotation: pose.rotation * rotations[a].slerp(rotations[b], f.fract()),
        }
    }
    fn flame(&self, n: usize) -> (f64, f32) {
        let (time, speed) = self.flame_events[n % self.flame_events.len()];
        (
            self.crank + self.open + self.rotation * (n / self.flame_events.len()) as f64 + time,
            speed,
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Bomb {
    #[serde(default)]
    pub trail: super::trail::Trail,
    pub id: u32,
    pub alternate: bool,
    pub position: Vec3,
    pub velocity: Vec3,
    pub angles: Vec2,
    pub age: f64,
    pub resting: bool,
    pub flames: usize,
    pub popped: bool,
}
impl Bomb {
    pub fn life(&self) -> f64 {
        if self.alternate {
            10.
        } else {
            3.
        }
    }
    pub fn pose(&self) -> Transform {
        Transform {
            translation: self.position,
            rotation: Quat::from_rotation_z(self.angles.y) * Quat::from_rotation_y(self.angles.x),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Flame {
    #[serde(default)]
    pub trail: super::trail::Trail,
    pub position: Vec3,
    pub velocity: Vec3,
    pub age: f64,
    pub id: u32,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct State {
    pub bombs: Vec<Bomb>,
    pub flames: Vec<Flame>,
    serial: u32,
}
#[derive(Default)]
pub(super) struct Events {
    pub hits: Vec<Hit>,
    pub blasts: Vec<Blast>,
    pub sounds: Vec<(&'static str, Vec3)>,
}
impl State {
    fn id(&mut self) -> u32 {
        self.serial = self.serial.wrapping_add(1);
        self.serial
    }
    pub fn launch(&mut self, position: Vec3, aim: Vec3, velocity: Vec3, alternate: bool) {
        let aim = aim.try_normalize().unwrap_or(Vec3::X);
        let id = self.id();
        self.bombs.push(Bomb {
            trail: Default::default(),
            id,
            alternate,
            position,
            velocity: aim * (600. + velocity.dot(aim).max(0.)),
            angles: vec2((-aim.z).atan2(aim.truncate().length()), aim.y.atan2(aim.x)),
            age: 0.,
            resting: false,
            flames: 0,
            popped: false,
        });
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.bombs.len() <= 16 && self.flames.len() <= 256,
            "Too many saved Jackbomb entities"
        );
        let mut ids = std::collections::BTreeSet::new();
        for b in &self.bombs {
            ensure!(
                b.trail.valid(b.age)
                    && b.position.is_finite()
                    && b.velocity.is_finite()
                    && b.velocity.length() <= 10000.
                    && b.angles.is_finite()
                    && (0. ..=b.life()).contains(&b.age)
                    && b.flames <= 64
                    && (b.alternate || b.flames == 0)
                    && ids.insert(b.id),
                "Invalid saved Jackbomb"
            );
        }
        for f in &self.flames {
            ensure!(
                f.trail.valid(f.age)
                    && f.position.is_finite()
                    && f.velocity.is_finite()
                    && f.velocity.length() <= 10000.
                    && (0. ..=3.).contains(&f.age)
                    && ids.insert(f.id),
                "Invalid saved Jackbomb flame"
            );
        }
        Ok(())
    }
    pub fn advance(&mut self, dt: f32, ctx: &Context<'_>, owner: Target, data: &Data) -> Events {
        let mut out = Events::default();
        if !dt.is_finite() || dt <= 0. {
            return out;
        }
        let dt = dt.min(1.) as f64;
        let mut births = Vec::new();
        for b in &mut self.bombs {
            b.trail.record(b.age as f32, b.pose());
            let mut remaining = dt;
            for _ in 0..256 {
                if b.age + 1e-7 >= b.life() {
                    let mut blast = Blast::new(
                        BlastKind::Jack,
                        b.position - b.velocity.normalize_or_zero() * 36.,
                        b.id,
                    );
                    blast.age = remaining as f32;
                    out.blasts.push(blast);
                    out.sounds.push((EXPLODE, b.position));
                    b.age = 11.;
                    break;
                }
                if !b.popped && b.age + 1e-7 >= data.crank {
                    b.popped = true;
                    out.sounds.push((POP, b.position));
                }
                let (next, speed) = data.flame(b.flames);
                if b.alternate && b.age + 1e-7 >= next {
                    let tag = data.tag(b, "tag_mouth");
                    // `proj` receives the animated tag's forward axis, including
                    // the rotating head and the box's own saved orientation.
                    let dir = tag.rotation * Vec3::X;
                    let trace = ctx.world.sweep(b.position, tag.translation, Vec3::ONE);
                    births.push((
                        Flame {
                            trail: Default::default(),
                            position: b.position.lerp(tag.translation, trace.fraction),
                            velocity: dir * speed,
                            age: 0.,
                            id: 0,
                        },
                        remaining,
                    ));
                    b.flames += 1;
                }
                if remaining <= 1e-8 {
                    break;
                }
                let mut step = remaining.min(1. / 120.).min(b.life() - b.age);
                if !b.popped {
                    step = step.min((data.crank - b.age).max(0.));
                }
                if b.alternate {
                    step = step.min((data.flame(b.flames).0 - b.age).max(0.));
                }
                if step <= 1e-8 {
                    break;
                }
                if !b.resting {
                    let half = Vec3::splat(if b.alternate { 1. } else { 4. });
                    let touch = motion(
                        &mut b.position,
                        &mut b.velocity,
                        step as f32,
                        half,
                        ctx,
                        true,
                    );
                    b.angles.y -= 720_f32.to_radians() * step as f32;
                    if let Some((_, normal)) = touch {
                        // MOVETYPE_BOUNCE clips with overbounce 1.5; it does not
                        // use the ball/flame's 0.85 full reflection.
                        b.velocity -= normal * (1.5 * b.velocity.dot(normal));
                        if normal.z > 0.7 && b.velocity.z < 60. {
                            b.resting = true;
                            b.velocity = Vec3::ZERO;
                        }
                    }
                } else if ctx
                    .world
                    .sweep(
                        b.position,
                        b.position - Vec3::Z * 0.25,
                        Vec3::splat(if b.alternate { 1. } else { 4. }),
                    )
                    .fraction
                    == 1.
                {
                    b.resting = false;
                }
                b.age += step;
                b.trail.record(b.age as f32, b.pose());
                remaining -= step;
            }
        }
        self.bombs.retain(|b| b.age < b.life());
        for f in &mut self.flames {
            advance_flame(f, dt, ctx, owner, &mut out.hits);
        }
        for (mut flame, remaining) in births {
            flame.id = self.id();
            advance_flame(&mut flame, remaining, ctx, owner, &mut out.hits);
            self.flames.push(flame);
        }
        self.flames.retain(|f| f.age < 3.);
        out
    }
}

/// Swept motion returns a contact normal. Owner is deliberately absent from box
/// contacts; emitted breath has a different owner and can hit Alice.
fn motion(
    position: &mut Vec3,
    velocity: &mut Vec3,
    dt: f32,
    half: Vec3,
    ctx: &Context<'_>,
    actors: bool,
) -> Option<(Option<usize>, Vec3)> {
    let end = *position + *velocity * dt - Vec3::Z * (400. * dt * dt);
    let wall = ctx.world.sweep(*position, end, half);
    if wall.start_solid {
        *velocity = Vec3::ZERO;
        return Some((None, Vec3::Z));
    }
    let actor = actors
        .then(|| combat::contact_box(ctx, *position, end, half))
        .flatten();
    let (fraction, normal, id) = if let Some((id, f)) = actor {
        let target = ctx.targets.iter().find(|t| t.id == id).unwrap();
        let point = position.lerp(end, f) - target.center;
        let extent = target.half + half;
        let axis = (0..3)
            .min_by(|&a, &b| {
                (extent[a] - point[a].abs())
                    .abs()
                    .total_cmp(&(extent[b] - point[b].abs()).abs())
            })
            .unwrap();
        let mut normal = Vec3::ZERO;
        normal[axis] = if point[axis] >= 0. { 1. } else { -1. };
        (f, normal, Some(id))
    } else {
        (wall.fraction, wall.normal, None)
    };
    *position = position.lerp(end, fraction);
    *velocity -= Vec3::Z * (800. * dt);
    if fraction < 1. {
        *position += normal * 0.05;
        Some((id, normal))
    } else {
        None
    }
}
fn advance_flame(f: &mut Flame, dt: f64, ctx: &Context<'_>, owner: Target, hits: &mut Vec<Hit>) {
    let mut targets = ctx.targets.to_vec();
    if !targets.iter().any(|t| t.id == owner.id) {
        targets.push(owner);
    }
    let flame_ctx = Context {
        world: ctx.world,
        targets: &targets,
    };
    let mut remaining = dt;
    while remaining > 1e-8 && f.age < 3. - 1e-7 {
        f.trail.record(
            f.age as f32,
            Transform {
                translation: f.position,
                rotation: Quat::IDENTITY,
            },
        );
        let step = remaining.min(1. / 120.).min(3. - f.age);
        if let Some((id, normal)) = motion(
            &mut f.position,
            &mut f.velocity,
            step as f32,
            Vec3::ONE,
            &flame_ctx,
            true,
        ) {
            if let Some(id) = id {
                hits.push(Hit {
                    id,
                    damage: 10.,
                    kind: combat::DamageKind::FireSword,
                    knockback: f.velocity.normalize_or_zero() * 20.,
                });
                f.age = 4.;
                return;
            }
            f.velocity = (f.velocity - normal * (2. * f.velocity.dot(normal))) * 0.85;
            if normal.z > 0. && f.velocity.z < 45. {
                f.velocity.z += 60.;
            }
        }
        f.age += step;
        f.trail.record(
            f.age as f32,
            Transform {
                translation: f.position,
                rotation: Quat::IDENTITY,
            },
        );
        remaining -= step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::World;
    fn data() -> Data {
        Data {
            clips: (0..4)
                .map(|_| tan::Model {
                    surfaces: vec![],
                    tags: [("tag_mouth".into(), vec![Vec3::Z * 20.])].into(),
                    tag_rotations: [("tag_mouth".into(), vec![Quat::IDENTITY])].into(),
                    frame_time: 0.05,
                })
                .collect(),
            crank: 1.2,
            open: 0.3,
            rotation: 4.05,
            flame_events: [
                1, 4, 9, 13, 17, 21, 25, 28, 31, 34, 39, 41, 43, 49, 51, 57, 58, 64, 65, 71, 73, 79,
            ]
            .map(|f| (f as f64 * 0.05, 250.))
            .to_vec(),
        }
    }
    fn owner() -> Target {
        Target {
            id: crate::dice::ALICE,
            center: vec3(-5000., 0., 0.),
            half: Vec3::ONE,
        }
    }
    #[test]
    fn bombs_settle_and_explode_on_fuse_once_after_restore() {
        let world = World::fixture(&[(vec3(-5000., -5000., -20.), vec3(5000., 5000., 0.))]);
        let ctx = Context {
            world: &world,
            targets: &[],
        };
        let data = data();
        for alt in [false, true] {
            for hz in [30, 60, 144] {
                let mut s = State::default();
                s.launch(vec3(0., 0., 40.), Vec3::X, Vec3::ZERO, alt);
                let (mut pops, mut blasts) = (0, 0);
                let mut last_flames = 0;
                for frame in 0..hz * 11 {
                    let e = s.advance(1. / hz as f32, &ctx, owner(), &data);
                    pops += e.sounds.iter().filter(|(p, _)| *p == POP).count();
                    blasts += e.blasts.len();
                    if let Some(b) = s.bombs.first() {
                        last_flames = b.flames;
                        if frame == hz * 2 {
                            assert!(b.resting, "not resting at {hz} Hz");
                            assert!(b.position.z < 5.);
                        }
                    }
                    if [hz, hz * 2, hz * 3, hz * 5, hz * 9, hz * 10].contains(&frame) {
                        let mut restored: State =
                            serde_json::from_value(serde_json::to_value(&s).unwrap()).unwrap();
                        restored.validate().unwrap();
                        let mut uninterrupted = s.clone();
                        let a = uninterrupted.advance(0.017, &ctx, owner(), &data);
                        let b = restored.advance(0.017, &ctx, owner(), &data);
                        assert_eq!(
                            serde_json::to_value(&uninterrupted).unwrap(),
                            serde_json::to_value(&restored).unwrap()
                        );
                        assert_eq!(
                            (a.hits.len(), a.blasts.len(), a.sounds.len()),
                            (b.hits.len(), b.blasts.len(), b.sounds.len())
                        );
                        s = serde_json::from_value(serde_json::to_value(&s).unwrap()).unwrap();
                    }
                }
                assert_eq!((pops, blasts), (1, 1));
                assert_eq!(last_flames, if alt { 46 } else { 0 });
                assert!(s.bombs.is_empty());
            }
        }
    }
    #[test]
    fn bombs_bounce_off_living_targets_without_contact_damage() {
        let world = World::fixture(&[]);
        let targets = [Target {
            id: 1,
            center: vec3(60., 0., 30.),
            half: Vec3::splat(10.),
        }];
        let ctx = Context {
            world: &world,
            targets: &targets,
        };
        let mut s = State::default();
        s.launch(vec3(0., 0., 30.), Vec3::X, Vec3::ZERO, false);
        let e = s.advance(0.1, &ctx, owner(), &data());
        assert!(e.hits.is_empty() && e.blasts.is_empty());
        assert!(s.bombs[0].velocity.x < 0. && s.bombs[0].age < 3.);
    }
    #[test]
    fn flame_sweeps_actor_once_and_world_blocks_it() {
        let owner = Target {
            id: crate::dice::ALICE,
            center: vec3(30., 0., 20.),
            half: Vec3::splat(5.),
        };
        for blocked in [false, true] {
            let world = World::fixture(&if blocked {
                vec![(vec3(10., -100., -100.), vec3(12., 100., 100.))]
            } else {
                vec![]
            });
            let ctx = Context {
                world: &world,
                targets: &[],
            };
            let mut hits = vec![];
            let mut flame = Flame {
                trail: Default::default(),
                position: vec3(0., 0., 20.),
                velocity: Vec3::X * 300.,
                age: 0.,
                id: 1,
            };
            advance_flame(&mut flame, 0.2, &ctx, owner, &mut hits);
            if blocked {
                assert!(hits.is_empty());
                assert!(flame.velocity.x < 0.);
            } else {
                assert_eq!(hits.len(), 1);
                assert_eq!((hits[0].id, hits[0].damage), (owner.id, 10.));
                assert!((hits[0].knockback.length() - 20.).abs() < 0.001);
                advance_flame(&mut flame, 0.5, &ctx, owner, &mut hits);
                assert_eq!(hits.len(), 1);
            }
        }
    }
    #[test]
    fn launch_inherits_only_forward_velocity_and_invalid_saves_fail() {
        let mut s = State::default();
        s.launch(Vec3::ZERO, Vec3::X, vec3(120., 300., 0.), false);
        assert_eq!(s.bombs[0].velocity, Vec3::X * 720.);
        s.launch(Vec3::ZERO, Vec3::X, -Vec3::X * 200., true);
        assert_eq!(s.bombs[1].velocity, Vec3::X * 600.);
        s.validate().unwrap();
        s.bombs[1].id = s.bombs[0].id;
        assert!(s.validate().is_err());
    }
}
