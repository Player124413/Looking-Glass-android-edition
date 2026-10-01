//! Liquid membership and provisional swimming support, independent of original game code.
use crate::{collision::World, movement::EYE_HEIGHT};
use anyhow::{ensure, Result};
use macroquad::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Liquid {
    #[default]
    Dry,
    Water,
    Slime,
    Lava,
}
impl Liquid {
    pub fn from_contents(mask: i32) -> Self {
        if mask & 8 != 0 {
            Self::Lava
        } else if mask & 16 != 0 {
            Self::Slime
        } else if mask & 32 != 0 {
            Self::Water
        } else {
            Self::Dry
        }
    }
    pub fn tint(self) -> Vec4 {
        match self {
            Self::Dry => Vec4::ZERO,
            Self::Water => vec4(0.055, 0.16, 0.19, 1500.),
            Self::Slime => vec4(0.16, 0.22, 0.065, 800.),
            Self::Lava => vec4(0.45, 0.09, 0.015, 500.),
        }
    }
    pub fn damage_per_second(self) -> f32 {
        match self {
            Self::Slime => 8.,
            Self::Lava => 25.,
            _ => 0.,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Immersion {
    pub level: u8,
    pub kind: Liquid,
}

/// Remaining air is derived from a persisted elapsed clock, never a wall-clock time.
/// The original grants 5 seconds, extended by 15 with the Mock Turtle shell flag.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Breath {
    pub shell: bool,
    pub submerged: f64,
    pub next_hurt: f64,
    pub hits: u64,
}
impl Default for Breath {
    fn default() -> Self {
        Self {
            shell: false,
            submerged: 0.,
            next_hurt: 0.,
            hits: 0,
        }
    }
}
impl Breath {
    pub fn refill(&mut self) {
        self.submerged = 0.;
        self.next_hurt = 0.;
    }
    pub fn capacity(&self) -> f64 {
        if self.shell {
            20.
        } else {
            5.
        }
    }
    pub fn remaining(&self) -> f32 {
        (self.capacity() - self.submerged).max(0.) as f32
    }
    /// The original mouth-bubble warning begins after three quarters of the air.
    pub fn warning_age(&self) -> Option<f32> {
        let age = self.submerged - self.capacity() * 0.75;
        (age >= 0.).then_some(age as f32)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.submerged.is_finite()
                && (0.0..=86400.).contains(&self.submerged)
                && self.next_hurt.is_finite()
                && (0.0..=86401.).contains(&self.next_hurt),
            "Invalid saved breath"
        );
        Ok(())
    }
    pub fn tick(&mut self, underwater: bool) -> f32 {
        if !underwater {
            self.submerged = 0.;
            self.next_hurt = 0.;
            return 0.;
        }
        self.submerged = (self.submerged + 1. / 120.).min(86400.);
        if self.submerged > self.capacity() + 1e-8 && self.submerged + 1e-8 >= self.next_hurt {
            self.next_hurt = self.submerged + 0.5;
            self.hits += 1;
            12.
        } else {
            0.
        }
    }
}
impl Immersion {
    pub fn sample(world: &World, feet: Vec3) -> Self {
        let mut mask = 0;
        let mut level = 0;
        for z in [1., 28., EYE_HEIGHT] {
            let c = world.liquid_at(feet + Vec3::Z * z);
            if c == 0 {
                break;
            }
            mask |= c;
            level += 1;
        }
        Self {
            level,
            kind: Liquid::from_contents(mask),
        }
    }
}

pub fn check(assets: &mut crate::assets::Assets) -> anyhow::Result<()> {
    use crate::{
        bsp::Bsp,
        movement::{Controls, FixedClock, Player},
    };
    use anyhow::{ensure, Context};
    let mut volumes = 0;
    let mut wet_maps = 0;
    for name in assets.maps() {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let world = World::from_bsp(&map)?;
        volumes += world.liquid_count();
        wet_maps += usize::from(world.liquid_count() > 0);
        if name == "pandemonium" {
            let mut checked = 0;
            for s in map
                .surfaces
                .iter()
                .filter(|s| map.shaders[s.shader].name == "textures/liquid/green_slime2_1")
            {
                let (vertices, indices) = map.triangulate(s);
                let mut probe = None;
                'triangles: for triangle in indices.chunks_exact(3) {
                    let center = triangle
                        .iter()
                        .map(|&i| vertices[i as usize].position)
                        .sum::<Vec3>()
                        / 3.;
                    for depth in [8., 24., 48., 64.] {
                        let feet = center - Vec3::Z * depth;
                        if world.body_clear(feet)
                            && Immersion::sample(&world, feet).kind == Liquid::Slime
                        {
                            probe = Some(Player::new(feet));
                            break 'triangles;
                        }
                    }
                }
                let mut player =
                    probe.context("No accessible liquid below visible Pandemonium slime")?;
                let mut clock = FixedClock::default();
                for _ in 0..60 {
                    clock.advance(1. / 60., &world, &mut player, Controls::default());
                }
                ensure!(
                    player.liquid_damage > 0.,
                    "Pandemonium slime no longer causes damage"
                );
                checked += 1;
            }
            ensure!(checked == 3, "Missing Pandemonium slime surfaces");
            println!("PASS all 3 Pandemonium slime surfaces cover harmful liquid");
        }
    }
    println!("Liquid geometry: {volumes} convex volumes across {wet_maps} maps");
    let map = Bsp::parse(&assets.read("maps/garden1.bsp")?)?;
    let mut world = World::from_bsp(&map)?;
    // Isolate swimming/ledge behavior here. --traversal-check exercises the actual
    // active garden current which now carries the normal entrance out of this pool.
    for p in &mut world.traversal.pushes {
        p.enabled = false;
    }
    let mut reference = None::<Vec3>;
    for fps in [30, 60, 144] {
        let mut player = Player::spawn(&world, map.spawn().0).context("Blocked garden spawn")?;
        player.breath.shell = true;
        ensure!(
            player.immersion.level == 3,
            "Garden start must be submerged"
        );
        let origin = player.feet;
        let mut clock = FixedClock::default();
        for (phase, seconds, input) in [
            ("float", 3, Controls::default()),
            (
                "dive",
                1,
                Controls {
                    rise: -1.,
                    ..Default::default()
                },
            ),
            (
                "surface",
                5,
                Controls {
                    rise: 1.,
                    ..Default::default()
                },
            ),
            (
                "swim",
                1,
                Controls {
                    wish: Vec2::X,
                    swim: Vec3::X,
                    ..Default::default()
                },
            ),
            ("stop", 1, Controls::default()),
        ] {
            for _ in 0..fps * seconds {
                clock.advance(1. / fps as f64, &world, &mut player, input);
                ensure!(
                    player.feet.is_finite() && world.body_clear(player.feet),
                    "Invalid body during {phase} at {fps} Hz: {:?}",
                    player.feet
                );
            }
            if phase == "float" {
                ensure!(player.feet.distance(origin) < 0.01, "Idle swimmer sank");
            }
            if phase == "dive" {
                ensure!(player.feet.z < origin.z - 100., "Dive did not descend");
            }
            if phase == "surface" {
                ensure!(
                    player.immersion.level < 3 && player.swimming,
                    "Could not surface: {:?}",
                    player.feet
                );
            }
            println!(
                "{fps} Hz {phase}: {:?}, depth {}",
                player.feet, player.immersion.level
            );
        }
        ensure!(
            player.landings == 0 && player.liquid_damage == 0.,
            "Ordinary deep water caused damage/landing"
        );
        if let Some(expected) = reference {
            ensure!(
                player.feet.distance(expected) < 0.01,
                "Swim depends on rendering rate"
            );
        }
        reference = Some(player.feet);
    }
    println!("PASS isolated garden liquid controller: float, dive, surface, swim and stop at 30/60/144 Hz (currents checked separately)");
    for angle in 0..8 {
        let a = angle as f32 * std::f32::consts::FRAC_PI_4;
        let direction = vec2(a.cos(), a.sin());
        let mut player = Player::spawn(&world, map.spawn().0).unwrap();
        for _ in 0..360 {
            player.tick(
                &world,
                Controls {
                    rise: 1.,
                    ..Default::default()
                },
            );
        }
        for _ in 0..1200 {
            player.tick(
                &world,
                Controls {
                    wish: direction,
                    swim: direction.extend(0.),
                    rise: 1.,
                    ..Default::default()
                },
            );
            ensure!(
                world.body_clear(player.feet),
                "Bank probe {angle} entered solid: {:?}",
                player.feet
            );
            if player.grounded && player.immersion.level == 0 {
                break;
            }
        }
        println!(
            "Garden bank direction {angle}: {:?}, grounded={}, depth={}, climbs={}",
            player.feet, player.grounded, player.immersion.level, player.climbs
        );
        if angle == 4 || angle == 5 {
            ensure!(
                player.grounded && player.immersion.level == 0,
                "Garden bank exit {angle} failed"
            );
            ensure!(player.climbs > 0, "Unexpected bank exit mode");
        }
    }
    println!("PASS garden bank routes: checked climbs out to the west and southwest; eight approaches stay outside solids");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shallow_slime_jump_clears_bank_and_resumes_in_midair() {
        use crate::movement::{Controls, Player};
        let mut w=World::fixture(&[
            (vec3(-200.,-200.,-100.),vec3(1000.,200.,0.)),
            (vec3(80.,-200.,0.),vec3(1000.,200.,88.)),
        ]);
        w.add_liquid(vec3(-200.,-200.,-10.),vec3(80.,200.,40.),16,None);
        let mut p=Player::new(vec3(45.,0.,0.03125));
        p.tick(&w,Controls::default());
        assert!(p.grounded && p.immersion.level==2);
        let mut resumed=None;
        for t in 0..180 {
            let c=Controls {wish:Vec2::X,swim:Vec3::X,rise:1.,jump:t==0,run:true,..Default::default()};
            p.tick(&w,c);
            assert!(w.body_clear(p.feet));
            if t==15 {
                resumed=Some(serde_json::from_value::<Player>(serde_json::to_value(&p).unwrap()).unwrap());
            } else if let Some(other)=&mut resumed {
                other.tick(&w,c);
                assert_eq!(other.feet,p.feet);
            }
        }
        assert!(p.feet.x>100. && p.feet.z>87. && p.immersion.level==0,"{:?}",p.feet);
        assert!(p.liquid_damage>0.);
    }
    #[test]
    fn convex_water_membership_depth_and_hazard_priority() {
        let mut world = World::fixture(&[]);
        world.add_liquid(
            vec3(-200., -200., -200.),
            vec3(200., 200., 100.),
            32,
            Some(crate::bsp::Plane {
                normal: vec3(-1., 0., 1.).normalize(),
                distance: 0.,
            }),
        );
        assert_eq!(world.liquid_at(vec3(-100., 0., 0.)), 0); // Inside bounds, outside sloped face.
        assert_eq!(Immersion::sample(&world, vec3(50., 0., -30.)).level, 3);
        assert_eq!(Immersion::sample(&world, vec3(0., 0., -30.)).level, 2);
        assert_eq!(Immersion::sample(&world, vec3(0., 0., -10.)).level, 1);
        assert_eq!(
            Immersion::sample(&world, vec3(0., 0., 10.)).kind,
            Liquid::Dry
        );
        assert!(world.body_clear(Vec3::ZERO));
        world.add_liquid(Vec3::splat(-10.), Vec3::splat(10.), 8 | 16, None);
        assert_eq!(
            Liquid::from_contents(world.liquid_at(Vec3::ZERO)),
            Liquid::Lava
        );
    }
    fn pool() -> World {
        let mut w = World::fixture(&[(vec3(-500., -500., -250.), vec3(500., 500., -200.))]);
        w.add_liquid(vec3(-500., -500., -200.), vec3(500., 500., 100.), 32, None);
        w
    }
    #[test]
    fn swim_dive_surface_collision_and_no_ground_damage() {
        use crate::movement::{Controls, Player};
        let w = pool();
        let mut p = Player::spawn(&w, Vec3::Z * 48.).unwrap();
        p.breath.shell = true;
        for _ in 0..600 {
            p.tick(&w, Controls::default());
        }
        assert!(p.swimming && p.feet.length() < 0.01);
        for _ in 0..300 {
            p.tick(
                &w,
                Controls {
                    rise: -1.,
                    ..Default::default()
                },
            );
            assert!(w.body_clear(p.feet));
        }
        assert!(p.feet.z < -199. && p.landings == 0);
        for _ in 0..480 {
            p.tick(
                &w,
                Controls {
                    rise: 1.,
                    ..Default::default()
                },
            );
        }
        assert!((55.9..56.1).contains(&p.feet.z));
        assert!(p.swimming && p.immersion.level == 2 && p.jumps == 0);
        assert_eq!(p.liquid_damage, 0.);
    }
    #[test]
    fn water_catches_fast_fall_and_does_not_tunnel_through_bottom() {
        use crate::movement::{Controls, Player};
        let w = pool();
        let mut p = Player::new(Vec3::Z * 500.);
        p.velocity.z = -1000.;
        for _ in 0..360 {
            p.tick(&w, Controls::default());
            assert!(w.body_clear(p.feet));
        }
        assert!(p.swimming && p.landings == 0 && p.feet.z > -190.);
    }
    #[test]
    fn shallow_supported_water_is_wading_not_surface_swimming() {
        use crate::movement::{Controls, Player};
        let mut w = World::fixture(&[(vec3(-200., -200., -20.), vec3(200., 200., 0.))]);
        w.add_liquid(vec3(-200., -200., 0.), vec3(200., 200., 40.), 32, None);
        let mut p = Player::spawn(&w, Vec3::Z * 48.).unwrap();
        for _ in 0..120 {
            p.tick(&w, Controls::default());
        }
        assert!(p.grounded && !p.swimming && p.immersion.level == 2);
        assert!(p.feet.z.abs() < 0.1);
    }
    #[test]
    fn shallow_entry_cannot_rise_past_the_float_line() {
        use crate::movement::{Controls, Player};
        let w = pool();
        let mut p = Player::spawn(&w, Vec3::Z * (70. + 48.)).unwrap();
        assert!(p.swimming && p.immersion.level == 2);
        for _ in 0..240 {
            p.tick(
                &w,
                Controls {
                    rise: 1.,
                    ..Default::default()
                },
            );
            assert!(p.swimming && p.feet.z <= 70.);
        }
        assert!((55.9..56.1).contains(&p.feet.z));
    }
    #[test]
    fn banks_climb_with_clearance_and_reject_tall_walls_and_ceilings() {
        use crate::movement::{Controls, Player};
        for (height, ceiling, allowed) in
            [(16., false, true), (60., false, false), (16., true, false)]
        {
            let mut boxes = vec![(vec3(25., -200., -200.), vec3(200., 200., height))];
            if ceiling {
                boxes.push((vec3(-200., -200., 40.), vec3(200., 200., 50.)));
            }
            let mut w = World::fixture(&boxes);
            w.add_liquid(vec3(-200., -200., -200.), vec3(25., 200., 0.), 32, None);
            let mut p = Player::spawn(&w, vec3(0., 0., 4.)).unwrap(); // Feet -44, shoulders at the surface.
            let input = Controls {
                wish: Vec2::X,
                swim: Vec3::X,
                rise: 1.,
                ..Default::default()
            };
            for _ in 0..160 {
                p.tick(&w, input);
                assert!(w.body_clear(p.feet));
            }
            assert_eq!(
                p.climbs > 0,
                allowed,
                "height={height}, ceiling={ceiling}, {:?}",
                p.feet
            );
            if allowed {
                assert!(p.feet.x > 40. && p.grounded && !p.swimming);
            }
        }
    }
    #[test]
    fn paused_clock_stops_swimming_and_liquid_damage_and_rates_match() {
        use crate::movement::{Controls, FixedClock, Player};
        let mut w = World::fixture(&[]);
        w.add_liquid(Vec3::splat(-500.), Vec3::splat(500.), 16, None);
        let mut last = None;
        for fps in [30, 60, 144] {
            let mut p = Player::spawn(&w, Vec3::Z * 48.).unwrap();
            let mut clock = FixedClock::default();
            let input = Controls {
                swim: Vec3::X,
                rise: 0.2,
                ..Default::default()
            };
            for _ in 0..fps {
                clock.advance(1. / fps as f64, &w, &mut p, input);
            }
            assert!((p.liquid_damage - 24.).abs() < 0.001);
            let before = (p.feet, p.liquid_damage);
            clock.pause();
            clock.advance(0., &w, &mut p, input);
            assert_eq!((p.feet, p.liquid_damage), before);
            if let Some(v) = last {
                assert!(p.feet.distance(v) < 0.01);
            }
            last = Some(p.feet);
        }
    }
    #[test]
    fn shell_warning_and_refills_follow_saved_air_budget() {
        for (shell, threshold) in [(false, 3.75), (true, 15.)] {
            let mut breath = Breath { shell, submerged: threshold - 0.01, ..Default::default() };
            assert!(breath.warning_age().is_none());
            breath.submerged = threshold + 0.25;
            assert_eq!(breath.warning_age(), Some(0.25));
            let mut restored: Breath = serde_json::from_str(&serde_json::to_string(&breath).unwrap()).unwrap();
            assert_eq!(restored.warning_age(), breath.warning_age());
            restored.refill();
            assert!(restored.warning_age().is_none());
            assert_eq!(restored.remaining(), if shell { 20. } else { 5. });
            assert_eq!(restored.shell, shell);
        }
    }
    #[test]
    fn drowning_uses_original_breath_budgets_and_half_second_damage() {
        for (shell, budget) in [(false, 5), (true, 20)] {
            let mut b = Breath {
                shell,
                ..Default::default()
            };
            for _ in 0..budget * 120 {
                assert_eq!(b.tick(true), 0.);
            }
            assert_eq!(b.tick(true), 12.);
            for _ in 0..59 {
                assert_eq!(b.tick(true), 0.);
            }
            assert_eq!(b.tick(true), 12.);
            assert_eq!(b.hits, 2);
            assert_eq!(b.tick(false), 0.);
            assert_eq!(b.remaining(), budget as f32);
            assert_eq!(b.tick(true), 0.);
        }
    }
    #[test]
    fn underwater_restart_preserves_air_damage_cadence_and_eventual_death() {
        use crate::{
            inventory::Stats,
            movement::{Controls, FixedClock, Player},
        };
        let w = pool();
        let mut p = Player::new(Vec3::ZERO);
        for _ in 0..599 {
            p.tick(&w, Controls::default());
        }
        let mut restored: Player =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        let mut stats = Stats::default();
        for _ in 0..510 {
            p.tick(&w, Controls::default());
            restored.tick(&w, Controls::default());
            assert_eq!(p.breath.hits, restored.breath.hits);
            stats.damage(std::mem::take(&mut restored.liquid_damage));
        }
        assert!(!stats.alive() && p.breath.hits == 9);
        let json = serde_json::to_value(&p).unwrap();
        let mut clock = FixedClock::default();
        clock.pause();
        clock.advance(0., &w, &mut p, Controls::default());
        assert_eq!(serde_json::to_value(p).unwrap(), json);
    }
}
