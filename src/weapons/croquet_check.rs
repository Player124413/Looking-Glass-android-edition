//! Croquet feedback, death selection and the renderer's actual funded input path.
use super::*;
use crate::{character::Character, movement::Player};

pub fn check(assets: &mut Assets) -> Result<()> {
    let specs = texture::read_materials(assets)?;
    ensure!(
        specs.contains_key("powerups/shock1"),
        "Missing original electric overlay"
    );
    for name in [
        "gb_meatbone1",
        "gb_meatbone2",
        "gb_meatbone3",
        "gb_ribs",
        "w_clubstaff",
        "w_diamondstaff",
    ] {
        let (_, model) = read_model(assets, name)?;
        ensure!(!model.surfaces.is_empty(), "Missing debris geometry {name}");
    }
    let timing = crate::npc::guard_timing(assets)?;
    let world = World::fixture(&[(vec3(-5000., -5000., -20.), vec3(5000., 5000., 0.))]);
    for hz in [30, 60, 144] {
        for ranged in [false, true] {
            let mut g = combat::Guard::new(vec3(0., 0., 0.04), 0., 1.);
            g.ranged = ranged;
            g.notarget = true;
            g.attacks = (0..1000)
                .find(|&n| crate::dismember::burst::seed(g.feet, n) % 10 == 0)
                .unwrap();
            g.hurt_kind(24., combat::DamageKind::Electric);
            ensure!(
                g.health == 31. && g.electric > 0. && g.burst.is_none(),
                "Nonlethal electric reaction"
            );
            g.hurt_kind(32., combat::DamageKind::Electric);
            ensure!(
                g.burst.as_ref().is_some_and(|b| b.pieces.len() == 8),
                "Eligible electric kill did not burst"
            );
            g.validate_save()?;
            for _ in 0..hz / 4 {
                g.advance(1. / hz as f32, &world, Vec3::ZERO, timing);
            }
            let saved = serde_json::to_vec(&g)?;
            g.advance(0., &world, Vec3::ZERO, timing);
            ensure!(
                saved == serde_json::to_vec(&g)?,
                "Pause changed electrical death"
            );
            let mut restored: combat::Guard = serde_json::from_slice(&saved)?;
            for _ in 0..hz * 6 {
                g.advance(1. / hz as f32, &world, Vec3::ZERO, timing);
                restored.advance(1. / hz as f32, &world, Vec3::ZERO, timing);
            }
            ensure!(
                serde_json::to_vec(&g)? == serde_json::to_vec(&restored)?,
                "Debris restore drift"
            );
            ensure!(
                g.electric == 0. && g.burst.as_ref().is_some_and(|b| b.pieces.is_empty()),
                "Death effects did not expire"
            );
            println!("PASS Croquet {hz} Hz ranged={ranged}: electric hit, burst, pause, exact saved continuation and cleanup");
        }
    }
    Ok(())
}

pub async fn render(assets: &mut Assets) -> Result<()> {
    crate::npc::electric_check::render(assets).await?;
    let root = std::path::Path::new("private/croquet-hit");
    std::fs::create_dir_all(root)?;
    let world = World::fixture(&[(vec3(-5000., -5000., -20.), vec3(5000., 5000., 0.))]);
    let specs = texture::read_materials(assets)?;
    let timing = crate::npc::guard_timing(assets)?;
    let mut model = crate::npc::Puppet::load(assets, "cardguard_club", &[], &specs)?;
    let mut alice = Character::load(assets)?;
    let mut player = Player::new(Vec3::Z * 0.04);
    player.grounded = true;
    alice.reset(&player, 0.);
    let initial = alice.snapshot();
    for (name, alternate, time, x, health, burst) in [
        ("melee-electric", false, 0.43, 55., 55., false),
        ("ball-electric", true, 0.83, 220., 55., false),
        ("ball-burst", true, 0.95, 220., 25., true),
        ("ball-debris", true, 1.35, 220., 25., true),
        ("ball-normal-death", true, 0.95, 220., 25., false),
        ("ball-cleanup", true, 6.1, 220., 25., true),
        ("ball-trail", true, 0.95, 3000., 55., false),
    ] {
        alice.restore(&initial)?;
        let mut g = combat::Guard::new(vec3(x, 0., 0.04), std::f32::consts::PI, 1.);
        g.notarget = true;
        g.health = health;
        g.place(&world);
        g.attacks = (0..1000)
            .find(|&n| (crate::dismember::burst::seed(g.feet, n) % 10 == 0) == burst)
            .unwrap();
        let mut stats = crate::inventory::Stats::weapon_preview();
        stats.select(2);
        let input = WeaponInput {
            selected: 2,
            aim: Vec3::X,
            ..Default::default()
        };
        for _ in 0..90 {
            alice.update_funded(
                1. / 60.,
                &player,
                false,
                input.clone(),
                &CombatContext {
                    world: &world,
                    targets: &[],
                },
                Some(&mut stats),
            );
        }
        let mut elapsed = 0_f32;
        let mut hits = 0;
        while elapsed < time - 0.00001 {
            let dt = (time - elapsed).min(1. / 60.);
            let targets = if g.health > 0. {
                vec![g.target(1)]
            } else {
                vec![]
            };
            let mut i = input.clone();
            i.click = (elapsed == 0.).then_some(alternate);
            alice.update_funded(
                dt,
                &player,
                false,
                i,
                &CombatContext {
                    world: &world,
                    targets: &targets,
                },
                Some(&mut stats),
            );
            for hit in alice.take_hits() {
                if hit.id == 1 {
                    hits += 1;
                    g.hit(hit);
                }
            }
            g.advance(dt, &world, vec3(-1000., 0., 48.), timing);
            elapsed += dt;
        }
        let mut ps: ProjectileSave =
            serde_json::from_value(serde_json::to_value(alice.snapshot())?["projectiles"].clone())?;
        ensure!(
            hits == usize::from(name != "ball-trail"),
            "{name}: expected one contact, got {hits}"
        );
        ensure!(
            stats.will() == if alternate { 92. } else { 100. },
            "{name}: cost changed"
        );
        ensure!(
            g.burst.is_some() == burst,
            "{name}: death selection changed"
        );
        if name.ends_with("electric") {
            ensure!(
                g.electric > 0. && ps.blasts.iter().any(|b| b.kind == blast::Kind::ElectricHit),
                "{name}: no electric feedback"
            );
        }
        let saved = serde_json::to_value((&g, alice.snapshot(), &stats))?;
        let file = root.join(format!("{name}.json"));
        if file.exists() {
            let previous: serde_json::Value = serde_json::from_slice(&std::fs::read(&file)?)?;
            ensure!(
                previous == saved,
                "{name}: cross-process continuation changed"
            );
        }
        std::fs::write(file, serde_json::to_vec_pretty(&saved)?)?;
        let checkpoint = alice.snapshot();
        let guard_saved = serde_json::to_vec(&g)?;
        alice.update_funded(
            0.,
            &player,
            false,
            input,
            &CombatContext {
                world: &world,
                targets: &[],
            },
            Some(&mut stats),
        );
        g.advance(0., &world, Vec3::ZERO, timing);
        ensure!(
            guard_saved == serde_json::to_vec(&g)?,
            "{name}: pause moved guard"
        );
        // Keep the live swing ribbon for the capture; transient trails are
        // deliberately cleared by loading a save. Check restoration afterward.
        g = serde_json::from_slice(&guard_saved)?;
        g.validate_save()?;
        let target = if name == "ball-trail" {
            ps.projectiles.pop().context("Missing ball")?.position
        } else {
            vec3(x, 0., 45.)
        };
        let camera = target + vec3(85., -135., 55.);
        let atmosphere = crate::environment::Atmosphere::default();
        for frame in 0..3 {
            clear_background(Color::new(0.06, 0.08, 0.10, 1.));
            let view = Camera3D {
                position: camera,
                target,
                up: Vec3::Z,
                fovy: 50_f32.to_radians(),
                z_near: 0.5,
                z_far: 20000.,
                ..Default::default()
            };
            set_camera(&view);
            crate::render_fx::begin_view(&view, time, &atmosphere, true);
            draw_cube(
                vec3(0., 0., -3.),
                vec3(10000., 10000., 4.),
                None,
                Color::new(0.18, 0.20, 0.22, 1.),
            );
            model.atmosphere(&atmosphere, camera);
            model.draw_guard(&g, true, camera, &atmosphere);
            alice.atmosphere(&atmosphere, camera);
            if name != "ball-trail" {
                alice.draw(player.feet, true);
            }
            alice.draw_effects(camera, true, true);
            crate::render_fx::finish();
            if frame == 2 {
                crate::viewer::save_capture(&root.join(format!("{name}.png")))?;
            }
            next_frame().await;
        }
        alice.restore(&checkpoint)?;
        println!(
            "PASS native {name}: {hits} hit, health {}, electric {}, burst {}",
            g.health,
            g.electric,
            g.burst.is_some()
        );
    }
    Ok(())
}
