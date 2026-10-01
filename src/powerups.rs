//! Reviewed gameplay rules from the locally supplied gameplay DLL and item TIKs.
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
    Nightmare,
}
impl Difficulty {
    pub const ALL: [Self; 4] = [Self::Easy, Self::Normal, Self::Hard, Self::Nightmare];
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "0" | "easy" => Ok(Self::Easy),
            "1" | "normal" | "medium" => Ok(Self::Normal),
            "2" | "hard" => Ok(Self::Hard),
            "3" | "nightmare" => Ok(Self::Nightmare),
            _ => anyhow::bail!("Choose Easy, Normal, Hard or Nightmare"),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Normal => "Normal",
            Self::Hard => "Hard",
            Self::Nightmare => "Nightmare",
        }
    }
    pub fn incoming(self) -> f32 {
        [0.75, 1., 1.4, 1.75][self as usize]
    }
    pub fn outgoing(self) -> f32 {
        [1.25, 1., 0.7, 0.55][self as usize]
    }
    pub fn duration(self, seconds: f32) -> f32 {
        (seconds * [1., 1., 0.8, 0.6][self as usize]).floor()
    }
    /// Original inhibit bits; Hard and Nightmare share the hard spawn set.
    pub fn allows(self, flags: u32) -> bool {
        flags & (1 << [8, 9, 10, 10][self as usize]) == 0
    }
    pub fn regen(self) -> (f32, f32) {
        // Original fgamex86 recovery: +0.3/+0.1 at whole-second time checks.
        // Integrate by simulation time so rendering frequency cannot change it.
        match self {
            Self::Easy => (0.3, 100.),
            Self::Normal => (0.1, 10.),
            _ => (0., 0.),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Rage,
    Tea,
    Glass,
}
impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Rage => "Rage Box",
            Self::Tea => "Grasshopper Tea",
            Self::Glass => "Darkened Looking Glass",
        }
    }
    pub fn duration(self) -> f32 {
        match self {
            Self::Rage => 42.,
            Self::Tea => 26.,
            Self::Glass => 45.,
        }
    }
    pub fn model(self) -> &'static str {
        match self {
            Self::Rage => "w_ragebox",
            Self::Tea => "w_grasshopperteaitem",
            Self::Glass => "w_lookingglass",
        }
    }
    pub fn class(class: &str) -> Option<Self> {
        match class.to_ascii_lowercase().as_str() {
            "item_ragebox" => Some(Self::Rage),
            "item_grasshoppertea" => Some(Self::Tea),
            "item_darkenedlookingglass" => Some(Self::Glass),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub rage: f32,
    pub tea: f32,
    pub stopped: f32,
    pub recharge: f32,
}
impl State {
    pub fn validate(&self, invisible: f32) -> Result<()> {
        ensure!(
            (0.0..=42.).contains(&self.rage)
                && (0.0..=26.).contains(&self.tea)
                && (0.0..=20.).contains(&self.stopped)
                && (0.0..=360.).contains(&self.recharge)
                && self.stopped <= self.recharge
                && [self.rage, self.tea, invisible]
                    .iter()
                    .filter(|t| **t > 0.)
                    .count()
                    <= 1,
            "Invalid saved power-up or Watch timers"
        );
        Ok(())
    }
    pub fn update(&mut self, dt: f32) {
        for t in [
            &mut self.rage,
            &mut self.tea,
            &mut self.stopped,
            &mut self.recharge,
        ] {
            *t = (*t - dt).max(0.);
        }
    }
    /// Advance only the unfrozen part of a frame, including the frame of expiry.
    pub fn world_dt(&self, dt: f32) -> f32 {
        (dt - self.stopped).max(0.)
    }
    pub fn clear_effects(&mut self) {
        self.rage = 0.;
        self.tea = 0.;
        self.stopped = 0.;
    }
}

pub fn check(assets: &mut crate::assets::Assets) -> Result<()> {
    crate::beyond_check::check_rage(assets)?;
    use crate::inventory::{self, PickupKind};
    let catalog = inventory::Catalog::load(assets)?;
    ensure!(
        catalog.weapons[9].primary == 1.,
        "Original Watch cost changed"
    );
    let watch = String::from_utf8(assets.read("models/w_watch.tik")?)?;
    ensure!(
        watch
            .split_whitespace()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|w| w == ["cycletime", "360"]),
        "Watch recharge changed"
    );
    for (model, grade) in [
        ("cardguard_club", "small"),
        ("cardguard_diamond", "medium"),
        ("c_boojum", "large"),
        ("c_ladybug", "medium"),
        ("c_duchess", "super"),
    ] {
        let bytes = assets.read(&format!("models/{model}.tik"))?;
        let tokens = crate::bsp::tokens(&String::from_utf8_lossy(&bytes))?;
        ensure!(
            tokens
                .windows(2)
                .any(|w| w[0].eq_ignore_ascii_case("manatype") && w[1].eq_ignore_ascii_case(grade)),
            "Enemy essence definition changed: {model}"
        );
    }
    let mut counts = [0; 4];
    let mut power_counts = [0; 3];
    for name in assets.maps() {
        let mut map = crate::bsp::Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        for d in Difficulty::ALL {
            map.difficulty = d;
            let p = inventory::pickups(&map, &name, &catalog);
            counts[d as usize] += p.len();
            if d == Difficulty::Normal {
                for p in p {
                    if let PickupKind::Power(k) = p.kind {
                        power_counts[match k {
                            Kind::Rage => 0,
                            Kind::Tea => 1,
                            Kind::Glass => 2,
                        }] += 1;
                    }
                }
            }
        }
    }
    ensure!(
        power_counts.iter().all(|n| *n > 0),
        "Missing campaign power-ups"
    );
    for d in Difficulty::ALL {
        println!(
            "PASS {}: {} available pickups; damage in/out {}/{}",
            d.name(),
            counts[d as usize],
            d.incoming(),
            d.outgoing()
        );
    }
    println!("PASS Normal power-ups: {} Rage, {} Tea, {} Glass (plus school secret); Watch cost/recharge and all supported enemy reward tiers match archive",power_counts[0],power_counts[1],power_counts[2]);
    check_watch_world(assets)?;
    crate::loot::check_school_contacts(assets)?;
    Ok(())
}

fn check_watch_world(assets: &mut crate::assets::Assets) -> Result<()> {
    use crate::{bsp::Bsp, collision::World, inventory::Stats, movement::Player, school2::School2};
    use macroquad::prelude::*;
    let map = Bsp::parse(&assets.read("maps/skool2.bsp")?)?;
    let initial = School2::load(assets, &map)?;
    for fps in [30, 60, 144] {
        let mut school = initial.clone();
        school.boojums[0].active = true;
        let mut world = World::from_bsp(&map)?;
        world.set_dynamic(school.colliders().collect());
        let mut player = Player::new(vec3(-64., -3440., -24.));
        let mut stats = Stats::weapon_preview();
        stats.watch().unwrap();
        let before = serde_json::to_value(school.snapshot())?;
        for _ in 0..fps * 2 {
            let dt = 1. / fps as f32;
            let world_dt = stats.powers.world_dt(dt);
            school.advance(world_dt, &map, &mut world, &mut player, &[])?;
            let (_, feedback) = school.update(world_dt, &world, player.feet, false);
            ensure!(feedback.damage == 0., "Frozen enemy attacked Alice");
            stats.update(dt);
        }
        ensure!(
            before == serde_json::to_value(school.snapshot())?,
            "Watch advanced school movers or enemies at {fps} Hz"
        );
        // Damage remains possible during the stop; only the enemy's own clock is frozen.
        school.hit(crate::school2::ENEMY_BASE, 1000.);
        ensure!(
            school.boojums[0].health == 0.,
            "Frozen enemy could not be defeated"
        );
        let frozen = serde_json::to_value(school.snapshot())?;
        let saved = serde_json::to_vec(&stats)?;
        let mut restored: Stats = serde_json::from_slice(&saved)?;
        restored.update(20.);
        let resumed_dt = restored.powers.world_dt(0.1);
        school.advance(resumed_dt, &map, &mut world, &mut player, &[])?;
        school.update(resumed_dt, &world, player.feet, false);
        ensure!(
            frozen != serde_json::to_value(school.snapshot())? && resumed_dt == 0.1,
            "Watch failed to resume the school after reload/expiry"
        );
    }
    println!("PASS Watch freezes real school movers/enemies at 30/60/144 Hz, permits damage and resumes after saved expiry");
    Ok(())
}

/// Original item meshes share ordinary world depth and fog.
pub struct Art {
    props: Vec<crate::weapons::Prop>,
    resources: Vec<crate::weapons::Prop>,
    material: crate::character::SkinMaterial,
    essence: crate::loot_art::Art,
    collection_fx: std::collections::BTreeMap<String, crate::pickup_effects::Effect>,
    collected: Vec<(String, macroquad::prelude::Vec3, f32)>,
}
impl Art {
    pub fn load(assets: &mut crate::assets::Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(assets)?;
        let mut props = Vec::new();
        for model in [Kind::Rage.model(), Kind::Tea.model(), Kind::Glass.model()] {
            props.push(crate::weapons::Prop::load_animation(
                assets, model, "idle", &specs,
            )?);
        }
        Ok(Self {
            props,
            resources: ["p_h1", "p_h2", "p_m1", "p_m2"]
                .iter()
                .map(|name| crate::weapons::Prop::load_animation(assets, name, "idle", &specs))
                .collect::<Result<_>>()?,
            material: crate::character::skin_material()?,
            essence: crate::loot_art::Art::load(assets, &specs)?,
            collection_fx: ["p_h1", "p_h2", "p_m1", "p_m2"]
                .into_iter()
                .chain(crate::loot_art::GRADES.iter().map(|g| g.model()))
                .map(|name| {
                    Ok((
                        name.into(),
                        crate::pickup_effects::Effect::load(assets, name, "pickup", &specs)?,
                    ))
                })
                .collect::<Result<_>>()?,
            collected: Vec::new(),
        })
    }
    pub fn collected(&mut self, item: &crate::inventory::Pickup, clock: f32) {
        if self.collection_fx.contains_key(&item.model) {
            self.collected
                .push((item.model.clone(), item.origin, clock));
        }
    }
    pub fn clear_effects(&mut self) {
        self.collected.clear();
    }
    pub fn draw(
        &mut self,
        items: &[crate::inventory::Pickup],
        stats: &crate::inventory::Stats,
        visit: &str,
        atmosphere: &crate::environment::Atmosphere,
        camera: macroquad::prelude::Vec3,
        clock: f32,
    ) {
        use macroquad::prelude::*;
        self.material.atmosphere(atmosphere, camera);
        self.material.bind();
        let mut essences = Vec::new();
        for item in items {
            if stats.collected.contains(&item.id)
                || camera.distance_squared(item.origin) > 2500. * 2500.
            {
                continue;
            }
            let prop = match item.kind {
                crate::inventory::PickupKind::Power(kind) => {
                    &mut self.props[match kind {
                        Kind::Rage => 0,
                        Kind::Tea => 1,
                        Kind::Glass => 2,
                    }]
                }
                crate::inventory::PickupKind::Sanity | crate::inventory::PickupKind::Will => {
                    let Some(i) = ["p_h1", "p_h2", "p_m1", "p_m2"]
                        .iter()
                        .position(|name| *name == item.model)
                    else {
                        continue;
                    };
                    &mut self.resources[i]
                }
                crate::inventory::PickupKind::Essence => {
                    if let Some(grade) = crate::loot_art::GRADES
                        .iter()
                        .find(|g| g.model() == item.model)
                    {
                        essences.push((*grade, item.origin));
                    }
                    continue;
                }
                crate::inventory::PickupKind::Weapon(_) => continue,
            };
            prop.draw_frame(
                crate::skeletal::Transform {
                    translation: item.origin,
                    rotation: Quat::IDENTITY,
                },
                1.,
                false,
                clock,
                true,
            );
        }
        if let Some(loot) = stats.loot.get(visit) {
            essences.extend(loot.drops.iter().map(|d| (d.grade, d.origin)));
        }
        // Placed and dropped translucent items share both the material passes
        // and ordering, including when they overlap in the same view.
        essences.sort_by(|a, b| {
            camera
                .distance_squared(b.1)
                .total_cmp(&camera.distance_squared(a.1))
        });
        for (grade, origin) in essences {
            self.essence.draw(grade, origin, atmosphere, camera, clock);
        }
        // These authored bursts live at most 3 seconds. Keep a short tail,
        // bounded even if many items are collected between rendered frames.
        self.collected
            .retain(|(_, _, start)| (0. ..=4.).contains(&(clock - start)));
        crate::render::depth_read_only(|| {
            for (model, origin, start) in &self.collected {
                self.collection_fx[model].draw(clock - start, *origin, 0., camera, atmosphere);
            }
        });
        gl_use_default_material();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn power_exclusivity_damage_expiry_death_and_resume() {
        use crate::inventory::{PickupKind, Stats};
        for d in Difficulty::ALL {
            for kind in [Kind::Rage, Kind::Tea, Kind::Glass] {
                let mut s = Stats::for_level("skool1", None);
                s.difficulty = d;
                assert!(s.powerup(kind));
                assert!(!s.powerup(kind));
                assert!(!s.powerup(Kind::Glass));
                s.select(0);
                assert!(
                    (s.attack_damage(20.)
                        - 20. * d.outgoing() * if kind == Kind::Rage { 4. } else { 1. })
                    .abs()
                        < 0.001
                );
                s.apply(PickupKind::Weapon(2), 100.);
                assert!(
                    (s.attack_damage(20.)
                        - 20. * d.outgoing() * if kind == Kind::Rage { 3. } else { 1. })
                    .abs()
                        < 0.001
                );
                s.damage(10.);
                assert!((s.sanity() - (100. - 10. * d.incoming())).abs() < 0.001);
                s.update(d.duration(kind.duration()) - 0.25);
                let mut restored: Stats =
                    serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
                restored.validate_save().unwrap();
                let before = serde_json::to_value(&restored).unwrap();
                restored.update(0.);
                assert_eq!(before, serde_json::to_value(&restored).unwrap());
                restored.update(0.3);
                assert!(restored.active_power().is_none());
                assert!(restored.powerup(kind));
                restored.damage(10000.);
                assert!(restored.active_power().is_none());
            }
        }
    }
    #[test]
    fn watch_cost_repetition_recharge_pause_and_difficulty_saved() {
        use crate::inventory::{PickupKind, Stats};
        for d in Difficulty::ALL {
            let mut s = Stats::for_level("skool1", None);
            s.difficulty = d;
            assert!(s.watch().is_err());
            s.apply(PickupKind::Weapon(9), 100.);
            assert!(s.watch().is_ok());
            assert_eq!(s.will(), 99.);
            for _ in 0..100 {
                assert!(s.watch().is_err());
            }
            assert_eq!(s.will(), 99.);
            s.update(19.75);
            let mut r: Stats = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
            assert_eq!(r.difficulty, d);
            r.update(0.);
            assert_eq!(r.powers.stopped, 0.25);
            r.update(0.5);
            assert_eq!(r.powers.stopped, 0.);
            assert!(r.watch().is_err());
            r.restore(); // Retry cancels effects; it must not recharge the Watch.
            assert!(r.watch().is_err());
            r.update(339.75);
            assert!(r.watch().is_ok());
        }
        let mut s = Stats::weapon_preview();
        s.spend_will(100.);
        assert!(s.watch().is_err());
        assert_eq!(s.powers.recharge, 0.);
    }
    #[test]
    fn will_recovery_has_original_difficulty_caps() {
        use crate::inventory::Stats;
        for d in Difficulty::ALL {
            let mut s = Stats::for_level("skool1", None);
            s.difficulty = d;
            s.spend_will(100.);
            s.update(2.);
            assert!((s.will() - [0.6, 0.2, 0., 0.][d as usize]).abs() < 0.001);
            s.update(48.);
            assert!((s.will() - [15., 5., 0., 0.][d as usize]).abs() < 0.001);
            s.update(1000.);
            assert_eq!(s.will(), [100., 10., 0., 0.][d as usize]);
        }
    }
    #[test]
    fn tea_changes_actual_jump_and_speed_and_remains_collision_safe() {
        use crate::{
            collision::World,
            movement::{Controls, Player},
        };
        use macroquad::prelude::*;
        let world = World::fixture(&[(vec3(-2000., -2000., -100.), vec3(2000., 2000., 0.))]);
        let mut ordinary = Player::new(vec3(0., 0., 0.1));
        let mut tea = ordinary.clone();
        tea.tea = true;
        for _ in 0..120 {
            ordinary.tick(
                &world,
                Controls {
                    wish: Vec2::X,
                    ..Default::default()
                },
            );
            tea.tick(
                &world,
                Controls {
                    wish: Vec2::X,
                    ..Default::default()
                },
            );
        }
        assert!((tea.velocity.x / ordinary.velocity.x - 2.).abs() < 0.001);
        ordinary.tick(
            &world,
            Controls {
                jump: true,
                ..Default::default()
            },
        );
        tea.tick(
            &world,
            Controls {
                jump: true,
                ..Default::default()
            },
        );
        assert!(tea.velocity.z > ordinary.velocity.z * 1.49);
        for _ in 0..120 {
            tea.tick(&world, Controls::default());
            assert!(world.body_clear(tea.feet));
        }
    }
    #[test]
    fn recovered_difficulty_rules() {
        for (i, d) in Difficulty::ALL.into_iter().enumerate() {
            assert_eq!(d.duration(42.), [42., 42., 33., 25.][i]);
            assert_eq!(d.duration(26.), [26., 26., 20., 15.][i]);
            assert_eq!(d.duration(45.), [45., 45., 36., 27.][i]);
            assert!(!d.allows(1 << [8, 9, 10, 10][i]));
            assert!(d.allows(64)); // Activation flags are independent.
        }
    }
    #[test]
    fn watch_frame_of_expiry_and_saved_validation() {
        let mut s = State {
            stopped: 0.01,
            recharge: 341.,
            ..Default::default()
        };
        assert!((s.world_dt(0.02) - 0.01).abs() < 0.0001);
        s.update(0.02);
        assert_eq!(s.stopped, 0.);
        s.recharge = f32::NAN;
        assert!(s.validate(0.).is_err());
    }
}
