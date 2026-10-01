//! Archive-backed timing checks and native rendering across process restarts.
use super::*;
use crate::{character::Character, inventory::Stats, movement::Player};

pub fn check(assets: &mut Assets) -> Result<()> {
    let data = ice::Data::load(assets)?;
    println!(
        "Ice wall idle/rise/melt: {}/{}/{}",
        data.idle, data.rise, data.death
    );
    let def = Definition::alice(assets)?;
    let skeleton =
        crate::skeletal::Skeleton::parse(&assets.read(&format!("{}/{}", def.path, def.model))?)?;
    let ready = Animation::parse(
        &assets.read("models/alice/ready.ska")?,
        skeleton.bones.len(),
    )?;
    let base = ready.sample(0., false);
    let upper = vec![true; base.len()];
    for hz in [30, 60, 144] {
        for (weapon, alt, first, interval, cost, seconds) in [
            (4, false, 0., 0.05, 0.75, 2),
            (4, true, 0.45, 1.6, 10., 6),
            (5, false, 0.35, 6.85, 10., 14),
            (5, true, 0.4, 2.9, 20., 10),
        ] {
            let mut a = Actions::load(assets, base.len(), &base)?;
            a.selected = weapon;
            let mut stats = Stats::weapon_preview();
            stats.select(weapon);
            let mut times = vec![];
            for frame in 0..hz * seconds {
                a.time_stopped = weapon == 5;
                let e = a.update_funded(
                    1. / hz as f32,
                    WeaponInput {
                        selected: weapon,
                        aim: Vec3::X,
                        click: Some(alt),
                        ..Default::default()
                    },
                    &base,
                    &upper,
                    true,
                    Some(&mut stats),
                );
                for e in e.emissions.iter().filter(|e| e.fire && e.weapon == weapon) {
                    times.push(frame as f32 / hz as f32 + e.at);
                }
                if frame % 17 == 0 {
                    a.restore(&serde_json::from_value(serde_json::to_value(
                        a.snapshot(),
                    )?)?)?;
                }
                a.finish_frame();
            }
            ensure!(!times.is_empty(), "No {weapon}/{alt} attacks");
            for (i, t) in times.iter().enumerate() {
                ensure!(
                    (t - (first + i as f32 * interval)).abs() < 0.002,
                    "Cadence {weapon}/{alt}/{hz}: {times:?}"
                );
            }
            ensure!(
                (stats.will() - (100. - cost * times.len() as f32)).abs() < 0.001,
                "Repeated Will debit"
            );
            let spent = stats.will();
            let e = a.update_funded(
                0.,
                WeaponInput {
                    selected: weapon,
                    click: Some(alt),
                    ..Default::default()
                },
                &base,
                &upper,
                true,
                Some(&mut stats),
            );
            ensure!(
                e.emissions.is_empty() && stats.will() == spent,
                "Paused attack advanced"
            );
            println!("PASS {hz} Hz toy {weapon}/{alt}: {} attacks, cadence, Will, saved cooldowns, pause", times.len());
        }
    }
    for ranged in [false, true] {
        let mut guard = combat::Guard::new(Vec3::ZERO, 0., 1.);
        guard.ranged = ranged;
        guard.hurt_kind(1., combat::DamageKind::Ice);
        ensure!(!guard.frozen, "Living guard incorrectly frozen");
        ensure!(
            guard.hurt_kind(1000., combat::DamageKind::Ice) == Some(ice::FREEZE),
            "Missing frozen sound"
        );
        let guard: combat::Guard = serde_json::from_value(serde_json::to_value(&guard)?)?;
        guard.validate_save()?;
        ensure!(
            guard.frozen && guard.clip() == "death_frozen",
            "Lost frozen guard"
        );
    }
    let mut boojum = crate::boojum::Boojum::new(Vec3::ZERO, 0.);
    boojum.active = true;
    let hit = Hit {
        id: 0,
        damage: 1000.,
        kind: combat::DamageKind::Ice,
        knockback: Vec3::ZERO,
    };
    ensure!(
        boojum.hit_attack(hit) == Some(ice::FREEZE),
        "Missing Boojum freeze"
    );
    let boojum: crate::boojum::Boojum = serde_json::from_value(serde_json::to_value(boojum)?)?;
    boojum.validate_save()?;
    ensure!(boojum.frozen, "Lost Boojum freeze");
    let original = crate::ladybug::Ladybug::new(Vec3::ZERO, 0., 1., vec![]);
    let mut bug = original.clone();
    ensure!(
        bug.hit_attack(hit) == Some(ice::FREEZE),
        "Missing ladybug freeze"
    );
    let bug: crate::ladybug::Ladybug = serde_json::from_value(serde_json::to_value(bug)?)?;
    bug.validate_save(&original)?;
    ensure!(bug.frozen, "Lost ladybug freeze");
    for path in [
        ice::FIRE,
        ice::WALL_FIRE,
        ice::FREEZE,
        jacks::TOSS,
        "sound/weapon/icewand/icewand_flesh1.wav",
        "sound/weapon/icewand/icewand_world1.wav",
        "sound/weapon/icewand/icewand_icewall.wav",
        "sound/weapon/icewand/icewand_icewall_death.wav",
        "sound/weapon/jacks/jacks_flesh1.wav",
        "sound/weapon/jacks/jacks_ricochet1.wav",
        "sound/weapon/jacks/jacks_ball_bounce.wav",
    ] {
        ensure!(!assets.read(path)?.is_empty(), "Missing sound {path}");
    }
    println!("PASS frozen enemy reactions, save validation and original sound assets");
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Resume {
    alice: crate::character::Snapshot,
    stats: Stats,
    next: ProjectileSave,
    will: f32,
    hits: usize,
}
fn state(alice: &Character) -> Result<ProjectileSave> {
    Ok(serde_json::from_value(
        serde_json::to_value(alice.snapshot())?["projectiles"].clone(),
    )?)
}
fn tick(
    alice: &mut Character,
    world: &mut World,
    player: &Player,
    input: WeaponInput,
    stats: &mut Stats,
    dt: f32,
) {
    world.set_weapon_obstacles(alice.ice_targets());
    let mut targets = alice.ice_targets();
    targets.push(combat::Target {
        id: 1,
        center: vec3(280., 0., 50.),
        half: vec3(24., 24., 50.),
    });
    alice.update_funded(
        dt,
        player,
        false,
        input,
        &CombatContext {
            world,
            targets: &targets,
        },
        Some(stats),
    );
}
pub async fn render(assets: &mut Assets) -> Result<()> {
    ice_jacks_art::Art::check_idle(assets).await?;
    let root = std::path::Path::new("private/ice-jacks-check");
    std::fs::create_dir_all(root)?;
    let mut world = World::fixture(&[
        (vec3(-5000., -5000., -20.), vec3(5000., 5000., 0.)),
        (vec3(500., -5000., 0.), vec3(510., 5000., 300.)),
    ]);
    let mut player = Player::new(Vec3::Z * 0.04);
    player.grounded = true;
    let mut alice = Character::load(assets)?;
    alice.reset(&player, 0.);
    let initial = alice.snapshot();
    for (name, weapon, alt, time, first_person, water) in [
        ("ice-stream", 4, false, 0.63, false, false),
        ("ice-first-person", 4, false, 0.63, true, false),
        ("wall-charge", 4, true, 0.44, false, false),
        ("wall-rise", 4, true, 0.75, false, false),
        ("wall-stand", 4, true, 3., false, false),
        ("wall-first-person", 4, true, 3., true, false),
        ("wall-melt", 4, true, 12.05, false, false),
        ("wall-expired", 4, true, 13., false, false),
        ("ice-water", 4, false, 0.70, false, true),
        ("ice-water-down", 4, false, 6.7, false, true),
        ("ice-water-release", 4, false, 8.2, false, true),
        ("jacks-before-children", 5, false, 0.44, false, false),
        ("jacks-swarm", 5, false, 1.2, false, false),
        ("jacks-return", 5, false, 5.65, false, false),
        ("jacks-alt-before-trace", 5, true, 0.48, false, false),
        ("jacks-alt-hit", 5, true, 0.60, false, false),
        ("jacks-alt-bounce", 5, true, 1.5, false, false),
        ("jacks-first-person", 5, false, 0.25, true, false),
    ] {
        player.immersion = crate::water::Immersion {
            level: if water { 3 } else { 0 },
            kind: if water {
                crate::water::Liquid::Water
            } else {
                crate::water::Liquid::Dry
            },
        };
        let held = weapon == 4 && !alt && !water;
        let input = WeaponInput {
            selected: weapon,
            aim: Vec3::X,
            first_person,
            click: held.then_some(false),
            ..Default::default()
        };
        let path = root.join(format!("resume-{name}.json"));
        if path.exists() {
            let mut s: Resume = serde_json::from_slice(&std::fs::read(&path)?)?;
            alice.restore(&s.alice)?;
            tick(
                &mut alice,
                &mut world,
                &player,
                input.clone(),
                &mut s.stats,
                0.05,
            );
            let actual = state(&alice)?;
            ensure!(
                serde_json::to_value(&actual.ice)? == serde_json::to_value(&s.next.ice)?
                    && serde_json::to_value(&actual.jacks)? == serde_json::to_value(&s.next.jacks)?
                    && actual.shots == s.next.shots,
                "Restart state mismatch: {name}"
            );
            ensure!(
                s.stats.will() == s.will && alice.take_hits().len() == s.hits,
                "Restart charge/damage mismatch: {name}"
            );
            println!("PASS restart {name}: exact state, event count, Will and damage");
        }
        alice.restore(&initial)?;
        let mut stats = Stats::weapon_preview();
        stats.select(weapon);
        for _ in 0..90 {
            tick(
                &mut alice,
                &mut world,
                &player,
                WeaponInput {
                    click: None,
                    ..input.clone()
                },
                &mut stats,
                1. / 60.,
            );
        }
        let mut elapsed = 0_f32;
        while elapsed < time - 0.00001 {
            let dt = (time - elapsed).min(1. / 60.);
            let mut i = input.clone();
            i.click = (elapsed == 0. || held).then_some(alt);
            tick(&mut alice, &mut world, &player, i, &mut stats, dt);
            elapsed += dt;
        }
        let ps = state(&alice)?;
        ensure!(
            ps.shots > 0 || name == "wall-charge" || name == "jacks-first-person",
            "Missing release {name}"
        );
        if held {
            ensure!(
                (stats.will() - (100. - ps.shots as f32 * 0.75)).abs() < 0.001,
                "Ice resource cadence"
            );
        } else if water {
            ensure!(
                stats.will() == 0. && ps.ice.shell.is_some() && alice.ice_locked() == (time < 7.5),
                "Water shell {name}"
            );
        } else {
            ensure!(
                stats.will() == 100. - ps.shots as f32 * combat::will_cost(weapon, alt),
                "Will cost/refund {name}: {}",
                stats.will()
            );
        }
        if name == "wall-stand" {
            ensure!(alice.ice_targets().len() == 1, "No solid wall");
        }
        if name == "jacks-swarm" {
            ensure!(ps.jacks.pieces.len() == 6, "Missing children");
        }
        if name == "jacks-alt-hit" {
            ensure!(
                ps.jacks.pieces.len() == 16 && !alice.take_hits().is_empty(),
                "No alternate hits"
            );
        }
        let snapshot = alice.snapshot();
        alice.take_hits();
        let before = stats.clone();
        let will = stats.will();
        tick(
            &mut alice,
            &mut world,
            &player,
            input.clone(),
            &mut stats,
            0.05,
        );
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&Resume {
                alice: snapshot.clone(),
                stats: before,
                next: state(&alice)?,
                will: stats.will(),
                hits: alice.take_hits().len(),
            })?,
        )?;
        alice.restore(&snapshot)?;
        let target = if water {
            player.feet + Vec3::Z * 40.
        } else if let Some(w) = ps.ice.walls.first() {
            w.origin + Vec3::Z * 80.
        } else {
            vec3(140., 0., 40.)
        };
        for frame in 0..4 {
            clear_background(Color::new(0.10, 0.12, 0.16, 1.));
            let camera = if first_person {
                player.eye()
            } else {
                target + vec3(130., -340., 130.)
            };
            let view = Camera3D {
                position: camera,
                target: if first_person {
                    camera + Vec3::X
                } else {
                    target
                },
                up: Vec3::Z,
                z_near: 0.5,
                z_far: 20000.,
                fovy: 50_f32.to_radians(),
                ..Default::default()
            };
            set_camera(&view);
            crate::lighting::select(alice.lights(), camera, &world);
            crate::render_fx::begin_view(
                &view,
                time,
                &crate::environment::Atmosphere::default(),
                true,
            );
            alice.atmosphere(&crate::environment::Atmosphere::default(), camera);
            draw_cube(
                vec3(0., 0., -3.),
                vec3(10000., 10000., 4.),
                None,
                Color::new(0.16, 0.19, 0.22, 1.),
            );
            draw_cube(
                vec3(280., 0., 50.),
                vec3(48., 48., 100.),
                None,
                Color::new(0.35, 0.2, 0.12, 1.),
            );
            if !first_person {
                alice.draw(player.feet, true);
            }
            alice.draw_effects(camera, true, !first_person);
            crate::render_fx::finish();
            if first_person {
                crate::render::clear_view_depth();
                alice.draw_first_person(true);
            }
            set_default_camera();
            draw_text(name, 20., 30., 24., WHITE);
            if frame == 3 {
                crate::viewer::save_capture(&root.join(format!("{name}.png")))?;
            }
            next_frame().await;
        }
        println!(
            "PASS native {name}: {} shots, {} walls, {} jacks, {will} Will",
            ps.shots,
            ps.ice.walls.len(),
            ps.jacks.pieces.len()
        );
    }
    let specs = texture::read_materials(assets)?;
    for (name, hold) in [
        ("cardguard_club", 1.5),
        ("cardguard_diamond", 0.25),
        ("c_boojum", 0.3),
        ("c_ladybug", 0.05),
    ] {
        let mut puppet = crate::npc::Puppet::load(assets, name, &["death_frozen"], &specs)?;
        for frame in 0..4 {
            clear_background(Color::new(0.10, 0.12, 0.16, 1.));
            let view = Camera3D {
                position: vec3(130., -210., 95.),
                target: vec3(0., 0., 40.),
                up: Vec3::Z,
                z_near: 0.5,
                fovy: 50_f32.to_radians(),
                ..Default::default()
            };
            set_camera(&view);
            crate::lighting::select(vec![], view.position, &world);
            crate::render_fx::begin_view(
                &view,
                1.,
                &crate::environment::Atmosphere::default(),
                true,
            );
            puppet.draw(
                "death_frozen",
                hold,
                false,
                Transform {
                    translation: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                1.,
                true,
            );
            crate::render_fx::finish();
            set_default_camera();
            draw_text(name, 20., 30., 24., WHITE);
            if frame == 3 {
                crate::viewer::save_capture(&root.join(format!("frozen-{name}.png")))?;
            }
            next_frame().await;
        }
        println!("PASS frozen native artwork {name}: authored death pose and layered ice shaders");
    }
    finale_walls(assets).await?;
    Ok(())
}

async fn finale_walls(assets: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(assets, "qlair")?;
    let mut world = World::from_bsp(&scene.map)?;
    let spawn = scene
        .map
        .entities
        .iter()
        .find(|e| e.get("targetname").is_some_and(|s| s == "alice_start_pos1"))
        .and_then(|e| e.get("origin"))
        .and_then(|s| crate::interaction::vector(s))
        .context("Finale arena entry")?;
    let mut alice = Character::load(assets)?;
    for first in [false, true] {
        let mut player = Player::new(spawn);
        for _ in 0..120 {
            player.tick(&world, crate::movement::Controls::default());
        }
        ensure!(
            player.grounded,
            "Finale Ice test did not land: {:?}",
            player.feet
        );
        alice.reset(&player, 0.);
        let mut stats = Stats::weapon_preview();
        stats.select(4);
        let input = WeaponInput {
            selected: 4,
            aim: Vec3::Y,
            first_person: first,
            ..Default::default()
        };
        for frame in 0..270 {
            alice.update_funded(
                1. / 60.,
                &player,
                false,
                WeaponInput {
                    click: (frame == 90).then_some(true),
                    ..input.clone()
                },
                &CombatContext {
                    world: &world,
                    targets: &[],
                },
                Some(&mut stats),
            );
        }
        let ps = state(&alice)?;
        ensure!(
            ps.ice.walls.len() == 1 && stats.will() == 90.,
            "Finale wall failed (first-person={first}): feet {:?}, walls {}, Will {}",
            player.feet,
            ps.ice.walls.len(),
            stats.will()
        );
        let wall = ps.ice.walls[0].target();
        ensure!(
            !world.sweep(wall.center, wall.center, wall.half).start_solid,
            "Finale wall intersects world"
        );
        let camera = if first {
            player.eye()
        } else {
            player.eye() + vec3(160., -220., 100.)
        };
        for _ in 0..3 {
            clear_background(BLACK);
            let view = Camera3D {
                position: camera,
                target: wall.center,
                up: Vec3::Z,
                z_near: 0.5,
                z_far: 20000.,
                fovy: 60_f32.to_radians(),
                ..Default::default()
            };
            set_camera(&view);
            crate::render_fx::begin_view(&view, 3., &scene.atmosphere, false);
            scene.draw(camera, 3., false, false, &[]);
            if !first {
                alice.draw(player.feet, false);
            }
            alice.draw_effects(camera, false, !first);
            crate::render_fx::finish();
            set_default_camera();
            crate::viewer::save_capture(std::path::Path::new(&format!(
                "private/ice-jacks-check/finale-wall-{}.png",
                if first { "first" } else { "third" }
            )))?;
            next_frame().await;
        }
        println!("PASS finale Ice Wand right click: first-person={first}, one solid wall, 10 Will");
        world.set_weapon_obstacles(vec![]);
    }
    Ok(())
}
