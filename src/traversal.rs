//! Reviewed BSP movement volumes and ropes. No original scripts are executed.
pub mod current;
mod water_rope_check;
use crate::{
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    entity::Id,
    event::Runtime,
    interaction::vector,
    movement::{Player, GRAVITY},
};
use anyhow::{Context, Result};
use macroquad::prelude::*;

#[derive(Clone, Debug)]
pub struct Push {
    pub id: Id,
    pub enabled: bool,
    pub volume: Collider,
    pub origin: Vec3,
    pub direction: Vec3,
    pub speed: f32,
    pub launch: bool,
    pub accelerate: bool,
}
#[derive(Debug)]
pub struct Rope {
    pub id: Id,
    pub model: usize,
    pub enabled: bool,
    pub origin: Vec3,
    pub anchor: Vec3,
    pub length: f32,
}
#[derive(Default, Debug)]
pub struct Traversal {
    pub currents: Vec<current::Path>,
    pub pushes: Vec<Push>,
    /// Reviewed map-owned forces for live actors; empty in visits without a binding.
    pub actor_pushes: Vec<Push>,
    pub ropes: Vec<Rope>,
    pub managed_ropes: bool,
}
fn number(e: &std::collections::BTreeMap<String, String>, key: &str, default: f32) -> f32 {
    e.get(key)
        .and_then(|s| s.parse::<f32>().ok())
        .filter(|n| n.is_finite())
        .unwrap_or(default)
}
pub fn launch_velocity(origin: Vec3, target: Vec3) -> Option<Vec3> {
    let height = target.z - origin.z;
    if height <= 0. {
        return None;
    }
    let time = (2. * height / GRAVITY).sqrt();
    Some(((target - origin).truncate() / time).extend(GRAVITY * time))
}
impl Traversal {
    pub fn load(map: &Bsp) -> Result<Self> {
        let mut out = Self::default();
        for (index, e) in map.entities.iter().enumerate() {
            let class = e.get("classname").map(String::as_str).unwrap_or("");
            if !["trigger_push", "trigger_accelerate", "func_rope"].contains(&class) {
                continue;
            }
            // TriggerPush responds to players unless bit 4 excludes them. Bits 8/16
            // independently exclude monsters/projectiles; they do not invert Alice's push.
            if class != "func_rope" && number(e, "spawnflags", 0.) as u32 & 4 != 0 {
                continue;
            }
            let model = e
                .get("model")
                .and_then(|s| s.strip_prefix('*'))
                .and_then(|s| s.parse::<usize>().ok())
                .context("Movement entity lacks inline model")?;
            let m = map
                .models
                .get(model)
                .context("Movement inline model out of range")?;
            let origin = e
                .get("origin")
                .and_then(|s| vector(s))
                .unwrap_or(Vec3::ZERO);
            if class == "func_rope" {
                out.ropes.push(Rope {
                    id: Id(index),
                    model,
                    enabled: true,
                    origin,
                    anchor: origin + Vec3::Z * m.max.z,
                    length: m.max.z - m.min.z,
                });
            } else {
                let target = e
                    .get("target")
                    .and_then(|n| map.entities.iter().find(|p| p.get("targetname") == Some(n)))
                    .and_then(|p| p.get("origin"))
                    .and_then(|s| vector(s));
                let angle = number(e, "angle", 0.);
                let mut direction = if angle == -1. {
                    Vec3::Z
                } else if angle == -2. {
                    -Vec3::Z
                } else {
                    vec3(angle.to_radians().cos(), angle.to_radians().sin(), 0.)
                };
                let launch = target.is_some();
                if let Some(target) = target {
                    direction = launch_velocity(origin, target).with_context(|| {
                        format!("Push {index} has a target below its launch origin")
                    })?;
                }
                out.pushes.push(Push {
                    id: Id(index),
                    enabled: true,
                    volume: Collider::model(map, model, origin, Quat::IDENTITY, false)?,
                    origin,
                    direction,
                    speed: number(e, "speed", 1000.).clamp(-1., 10000.),
                    launch,
                    accelerate: class == "trigger_accelerate",
                });
            }
        }
        Ok(out)
    }
    pub fn sync(&mut self, runtime: &Runtime, managed_ropes: bool) {
        self.managed_ropes = managed_ropes;
        for p in &mut self.pushes {
            p.enabled = runtime.enabled(p.id);
        }
        for r in &mut self.ropes {
            r.enabled = runtime.enabled(r.id);
        }
    }
    pub fn push(&self, feet: Vec3, velocity: &mut Vec3) -> bool {
        let mut launch = false;
        for p in self
            .pushes
            .iter()
            .filter(|p| p.enabled && !(p.accelerate && p.speed < 0.))
        {
            // Include the next swept body segment, so fast falls cannot miss a thin pad.
            let center = feet + PLAYER_CENTER;
            if p.volume.touches(
                center,
                center + *velocity * crate::movement::FIXED_DT,
                PLAYER_HALF,
            ) {
                if p.accelerate {
                    *velocity += p.direction * p.speed * (20. * crate::movement::FIXED_DT);
                } else if p.launch {
                    *velocity = p.direction;
                    launch = true;
                } else {
                    *velocity += p.direction * (p.speed - velocity.dot(p.direction));
                }
            }
        }
        launch
    }
    pub fn updraft(&self, feet: Vec3) -> bool {
        let center = feet + PLAYER_CENTER;
        self.pushes.iter().any(|p| {
            p.enabled
                && p.accelerate
                && p.speed < 0.
                && p.volume.touches(center, center, PLAYER_HALF)
        })
    }
    pub fn rope(&self, id: Id) -> Option<&Rope> {
        (!self.managed_ropes)
            .then(|| self.ropes.iter().find(|r| r.id == id && r.enabled))
            .flatten()
    }
    pub fn nearest_rope(&self, world: &World, eye: Vec3, reach: f32) -> Option<&Rope> {
        if self.managed_ropes {
            return None;
        }
        self.ropes
            .iter()
            .filter(|r| r.enabled && r.length > 32.)
            .filter_map(|r| {
                let hand = vec3(
                    r.anchor.x,
                    r.anchor.y,
                    eye.z.clamp(r.anchor.z - r.length, r.anchor.z - 24.),
                );
                let t = world.sweep(eye, hand, Vec3::splat(0.5));
                let distance = eye.distance(hand);
                (distance < reach && !t.start_solid && t.fraction >= 1.).then_some((r, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(r, _)| r)
    }
    pub fn transforms(&self, player: &Player) -> Vec<(usize, Vec3, Quat)> {
        if self.managed_ropes {
            return vec![];
        }
        self.ropes
            .iter()
            .map(|r| {
                let rotation = if player.rope.as_ref().is_some_and(|g| g.id == r.id) {
                    Quat::from_rotation_arc(
                        -Vec3::Z,
                        (player.feet + Vec3::Z * 40. - r.anchor).normalize_or_zero(),
                    )
                } else {
                    Quat::IDENTITY
                };
                (
                    r.model,
                    r.anchor + rotation * (r.origin - r.anchor),
                    rotation,
                )
            })
            .collect()
    }
}

/// These volumes are enabled by later map-specific puzzles/attacks. Until their
/// owning sequence is restored, preserve its authored initial OFF state.
pub fn initially_disabled(map: &str, e: &std::collections::BTreeMap<String, String>) -> bool {
    let name = e.get("targetname").map(String::as_str).unwrap_or("");
    matches!(
        (map, name),
        ("potears2", "suck_push") | ("grounds1", "gate_push") | ("centipede2", "ant_trigger1")
    ) || (map == "tower1" && name.starts_with("face") && name.ends_with("_push"))
        || (map == "hedge3" && ["bellow1push", "bellow2push"].contains(&name))
}

pub fn check(assets: &mut crate::assets::Assets) -> Result<()> {
    use anyhow::ensure;
    let mut pushes = 0;
    let mut ropes = 0;
    for name in assets.maps() {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let world = World::from_bsp(&map).with_context(|| name.clone())?;
        pushes += world.traversal.pushes.len();
        ropes += world.traversal.ropes.len();
        for p in &world.traversal.pushes {
            ensure!(p.direction.is_finite(), "Invalid push direction");
            if world.liquid_at(p.origin) != 0 {
                println!(
                    "WATER FORCE {name} {} {:?} {:?} launch={}",
                    p.id.0, p.origin, p.direction, p.launch
                );
            }
        }
        if !world.traversal.ropes.is_empty() {
            println!("ROPES {name} {}", world.traversal.ropes.len());
        }
    }
    println!("PASS parsed campaign movement: {pushes} player push volumes, {ropes} ropes");
    water_rope_check::check(assets)?;
    for (name, id) in [("potears1", 7), ("gvillage", 9), ("garden1", 54)] {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let mut world = World::from_bsp(&map)?;
        let mut interactions = crate::interaction::Interactions::load(&map)?;
        interactions.set_entry(assets, &map, name, None)?;
        interactions.sync(&mut world);
        let force = world
            .traversal
            .pushes
            .iter()
            .find(|p| p.id == Id(id))
            .context("Missing authored force")?;
        let start = Player::spawn(&world, force.origin + Vec3::Z * 48.)
            .context("Force probe obstructed")?;
        let mut reference = None;
        for hz in [30, 60, 144] {
            let mut player = start.clone();
            let mut clock = crate::movement::FixedClock::default();
            for _ in 0..hz * 2 {
                clock.advance(
                    1. / hz as f64,
                    &world,
                    &mut player,
                    crate::movement::Controls::default(),
                );
                ensure!(
                    world.body_clear(player.feet),
                    "{name}: force penetrated solid geometry"
                );
            }
            println!(
                "LIVE {name} #{id} {hz}Hz {:?} -> {:?}",
                start.feet, player.feet
            );
            ensure!(
                player.feet.distance(start.feet) > 30.,
                "{name}: force did not move Alice"
            );
            if name == "potears1" {
                ensure!(
                    player.feet.x < start.feet.x - 30.,
                    "Current moved backwards"
                );
            } else {
                ensure!(player.feet.z > start.feet.z + 30., "Upward volume failed");
            }
            if let Some(expected) = reference {
                ensure!(
                    player.feet.distance(expected) < 0.01,
                    "Force varies with rendering rate"
                );
            }
            reference = Some(player.feet);
        }
        interactions.event_world.set_enabled(Id(id), false)?;
        let saved = serde_json::to_vec(&interactions.snapshot())?;
        let mut loaded = crate::interaction::Interactions::load(&map)?;
        loaded.set_entry(assets, &map, name, None)?;
        loaded.restore(&serde_json::from_slice(&saved)?, &map)?;
        loaded.sync(&mut world);
        ensure!(
            !world
                .traversal
                .pushes
                .iter()
                .find(|p| p.id == Id(id))
                .unwrap()
                .enabled,
            "Disabled movement volume reactivated on load"
        );
        loaded.event_world.set_enabled(Id(id), true)?;
        loaded.sync(&mut world);
        ensure!(
            world
                .traversal
                .pushes
                .iter()
                .find(|p| p.id == Id(id))
                .unwrap()
                .enabled,
            "Restored movement volume cannot be enabled"
        );
        println!("PASS {name} #{id}: shared activation survives save and can resume");
    }
    // A real garden rope: find a clear point next to the hanging brush, grab,
    // climb/swing, and compare a mid-grip serialization with continued live input.
    for name in [
        "potears1", "garden1", "garden2", "funhouse", "hedge3", "rchess1",
    ] {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let world = World::from_bsp(&map)?;
        let mut mounted = None;
        'ropes: for rope in &world.traversal.ropes {
            for drop in [80., 160., 240.] {
                for offset in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y] {
                    let feet = rope.anchor - Vec3::Z * (drop + 40.) + offset * 24.;
                    if drop > rope.length - 32. || !world.body_clear(feet) {
                        continue;
                    }
                    let mut player = Player::new(feet);
                    player.tick(
                        &world,
                        crate::movement::Controls {
                            use_pressed: true,
                            ..Default::default()
                        },
                    );
                    if player.rope.as_ref().is_some_and(|g| g.id == rope.id) {
                        mounted = Some(player);
                        break 'ropes;
                    }
                }
            }
        }
        let mut p = mounted.with_context(|| format!("No accessible {name} rope"))?;
        let before = p.feet;
        let mut loaded: Player = serde_json::from_value(serde_json::to_value(&p)?)?;
        for _ in 0..60 {
            let input = crate::movement::Controls {
                rise: 1.,
                wish: Vec2::X,
                ..Default::default()
            };
            p.tick(&world, input);
            loaded.tick(&world, input);
            p.validate_world(&world)?;
            ensure!(world.body_clear(p.feet), "Rope penetrated {name} geometry");
        }
        ensure!(
            serde_json::to_value(&p)? == serde_json::to_value(&loaded)?,
            "{name} saved rope diverged"
        );
        ensure!(p.feet.distance(before) > 5., "{name} rope cannot move");
        println!(
            "PASS {name}: original rope {:?}, climb/swing {:?} -> {:?}, restored grip matches",
            p.rope.as_ref().unwrap().id,
            before,
            p.feet
        );
    }
    Ok(())
}

/// Find a quiet, submerged staging point for breath/save checks, without changing
/// the authored world's collision or force activation.
pub fn water_probe(world: &World, origin: Vec3) -> Option<Vec3> {
    for z in [0., -64., -128., 64., -256.] {
        for y in [0., -64., 64., -128., 128., -256., 256., -512., 512.] {
            for x in [0., -64., 64., -128., 128., -256., 256., -512., 512.] {
                let p = origin + vec3(x, y, z);
                if !world.body_clear(p) || crate::water::Immersion::sample(world, p).level != 3 {
                    continue;
                }
                let mut test = Player::new(p);
                for _ in 0..24 {
                    test.tick(world, crate::movement::Controls::default());
                }
                if test.feet.distance(p) < 0.01 {
                    return Some(p);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::{Controls, FixedClock};
    fn current_world() -> World {
        let mut w = World::fixture(&[(vec3(100., -200., -200.), vec3(120., 200., 200.))]);
        w.add_liquid(Vec3::splat(-500.), Vec3::splat(500.), 32, None);
        w.traversal.pushes.push(Push {
            id: Id(1),
            enabled: true,
            origin: Vec3::ZERO,
            volume: Collider::fixture(Vec3::splat(-80.), Vec3::splat(80.)),
            direction: Vec3::X,
            speed: 200.,
            launch: false,
            accelerate: false,
        });
        w
    }
    #[test]
    fn current_moves_idle_swimmer_respects_walls_and_activation() {
        let mut w = current_world();
        let mut p = Player::new(Vec3::ZERO);
        for _ in 0..240 {
            p.tick(&w, Controls::default());
            assert!(w.body_clear(p.feet));
        }
        assert!(p.feet.x > 84. && p.feet.x < 85.1);
        w.traversal.pushes[0].enabled = false;
        let mut stopped = Player::new(Vec3::ZERO);
        for _ in 0..240 {
            stopped.tick(&w, Controls::default());
        }
        assert_eq!(stopped.feet, Vec3::ZERO);
        stopped.feet = vec3(-200., 0., 0.);
        w.traversal.pushes[0].enabled = true;
        for _ in 0..120 {
            stopped.tick(&w, Controls::default());
        }
        assert_eq!(stopped.feet, vec3(-200., 0., 0.));
    }
    #[test]
    fn target_pad_reaches_authored_apex_without_cancelling_crosswise_current_velocity() {
        let mut w = World::fixture(&[]);
        let target = vec3(200., 0., 100.);
        let v = launch_velocity(Vec3::ZERO, target).unwrap();
        let mut p = Player::new(Vec3::ZERO);
        p.velocity = v;
        p.knockback_time = 0.2;
        for _ in 0..60 {
            p.tick(&w, Controls::default());
        }
        assert!(p.feet.distance(target) < 0.1, "{:?}", p.feet);
        w.traversal = current_world().traversal;
        let mut velocity = vec3(-400., 90., -20.);
        w.traversal.push(Vec3::ZERO, &mut velocity);
        assert_eq!(velocity, vec3(200., 90., -20.));
    }
    #[test]
    fn current_and_impulses_are_fixed_rate_pause_safe_and_swept() {
        let w = current_world();
        let mut reference = None;
        for hz in [30, 60, 144] {
            let mut p = Player::new(Vec3::ZERO);
            p.knockback(vec3(600., 0., 0.));
            let mut c = FixedClock::default();
            for _ in 0..hz {
                c.advance(1. / hz as f64, &w, &mut p, Controls::default());
            }
            let json = serde_json::to_value(&p).unwrap();
            if let Some(r) = &reference {
                assert_eq!(&json, r);
            }
            reference = Some(json.clone());
            c.pause();
            c.advance(0., &w, &mut p, Controls::default());
            assert_eq!(serde_json::to_value(p).unwrap(), json);
        }
        let w = World::fixture(&[(vec3(50., -100., -100.), vec3(55., 100., 200.))]);
        let mut p = Player::new(Vec3::ZERO);
        p.knockback(Vec3::X * 1000.);
        for _ in 0..30 {
            p.tick(&w, Controls::default());
            assert!(w.body_clear(p.feet));
        }
        assert!(p.feet.x < 35.);
    }
    fn rope_world() -> World {
        let mut w = World::fixture(&[(vec3(-500., -500., -20.), vec3(500., 500., 0.))]);
        w.traversal.ropes.push(Rope {
            id: Id(2),
            model: 1,
            enabled: true,
            origin: Vec3::Z * 150.,
            anchor: Vec3::Z * 300.,
            length: 290.,
        });
        w
    }
    #[test]
    fn rope_climbs_swings_releases_and_restores_without_teleporting() {
        let w = rope_world();
        let mut p = Player::new(vec3(25., 0., 0.));
        p.grounded = true;
        p.tick(
            &w,
            Controls {
                use_pressed: true,
                ..Default::default()
            },
        );
        assert!(p.rope.is_some());
        let mut restored: Player =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        for _ in 0..120 {
            let input = Controls {
                rise: 1.,
                wish: Vec2::Y,
                ..Default::default()
            };
            p.tick(&w, input);
            restored.tick(&w, input);
            p.validate_world(&w).unwrap();
            assert!(w.body_clear(p.feet));
        }
        assert!(p.feet.z > 90. && p.feet.y > 50.);
        assert_eq!(
            serde_json::to_value(&p).unwrap(),
            serde_json::to_value(restored).unwrap()
        );
        p.tick(
            &w,
            Controls {
                use_pressed: true,
                ..Default::default()
            },
        );
        assert!(p.rope.is_none() && p.script_motion == 0 && p.velocity.length() > 10.);
        for _ in 0..20 {
            p.tick(&w, Controls::default());
        }
        assert!(p.rope.is_none());
    }
    #[test]
    fn rope_rejects_through_wall_grabs_and_blocked_climb_and_releases_on_knockback() {
        let mut w = rope_world();
        let mut p = Player::new(vec3(25., 0., 0.));
        p.grounded = true;
        // Add a moving obstruction after mounting: neither climbing nor swinging may penetrate it.
        p.tick(
            &w,
            Controls {
                use_pressed: true,
                ..Default::default()
            },
        );
        w.set_dynamic(vec![Collider::fixture(
            vec3(-100., -100., 80.),
            vec3(100., 100., 90.),
        )]);
        for _ in 0..120 {
            p.tick(
                &w,
                Controls {
                    rise: 1.,
                    ..Default::default()
                },
            );
            assert!(w.body_clear(p.feet));
        }
        assert!(p.feet.z < 24.1);
        p.knockback(Vec3::Y * 80.);
        assert!(p.rope.is_none());
        w.set_dynamic(vec![Collider::fixture(
            vec3(5., -100., -10.),
            vec3(7., 100., 500.),
        )]);
        assert!(w
            .traversal
            .nearest_rope(&w, vec3(30., 0., 48.), 76.)
            .is_none());
    }
    #[test]
    fn use_press_is_buffered_once_across_high_refresh_frames() {
        let w = rope_world();
        let mut p = Player::new(vec3(25., 0., 0.));
        p.grounded = true;
        let mut c = FixedClock::default();
        c.advance(
            1. / 144.,
            &w,
            &mut p,
            Controls {
                use_pressed: true,
                ..Default::default()
            },
        );
        assert!(p.rope.is_none());
        c.advance(1. / 144., &w, &mut p, Controls::default());
        assert!(p.rope.is_some());
        c.advance(1. / 30., &w, &mut p, Controls::default());
        assert!(p.rope.is_some());
    }
}
