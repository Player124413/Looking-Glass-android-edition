//! Real archive and native GPU checks for the Blade/Cards combat path.
use super::*;
use crate::{character::Character, movement::Player};

pub fn check(assets: &mut Assets) -> Result<()> {
    super::check(assets)?;
    super::check_actions(assets)?;
    let def = Definition::alice(assets)?;
    let skeleton =
        crate::skeletal::Skeleton::parse(&assets.read(&format!("{}/{}", def.path, def.model))?)?;
    let ready = Animation::parse(
        &assets.read("models/alice/ready.ska")?,
        skeleton.bones.len(),
    )?;
    let base = ready.sample(0., false);
    let upper = vec![true; base.len()];
    let tag = skeleton
        .bones
        .iter()
        .position(|b| b.name == "tag_weapon")
        .context("Missing hand tag")?;
    // This existing clip is intentionally inactive: the engine's prefix lookup
    // excludes aliases whose next character is '_', and Cards has no ammo clip.
    let reload = Animation::parse(
        &assets.read("models/alice/cards_att_prim_reload.ska")?,
        base.len(),
    )?;
    ensure!(
        (reload.duration() - 1.1).abs() < 0.001,
        "Changed original Cards reload clip"
    );
    let specs = texture::read_materials(assets)?;
    let path = texture::resolve(assets, "meta", &specs).context("Missing return sprite")?;
    texture::decode(assets, &path)?;
    for path in [
        "sound/weapon/shared/weapon_reappear.wav",
        "sound/weapon/knife/knife_spin.wav",
        "sound/weapon/knife/knife_phit_flesh1.wav",
        "sound/weapon/cards/cards_loop1.wav",
    ] {
        ensure!(
            !assets.read(path)?.is_empty(),
            "Missing weapon sound {path}"
        );
    }
    let world = World::fixture(&[(vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.))]);
    let guard = combat::Guard::new(vec3(55., 0., 0.04), 0., 1.);
    let targets = [guard.target(1)];
    for variation in 0..3 {
        let mut a = Actions::load(assets, base.len(), &base)?;
        a.begin(
            1 + variation,
            Action::Attack { alternate: false },
            Vec3::X,
            crate::weapon_rules::Stage::Shot,
        );
        let events = a.update(
            0.5,
            WeaponInput {
                aim: Vec3::X,
                ..Default::default()
            },
            &base,
            &upper,
            true,
        );
        let e = events
            .emissions
            .iter()
            .find(|e| e.fire)
            .context("Missing melee contact")?;
        let pose = skeleton.global_pose(&e.pose);
        let origin = pose[tag].translation * def.scale;
        let hits = projectile::melee(
            &CombatContext {
                world: &world,
                targets: &targets,
            },
            origin,
            Vec3::X,
        );
        ensure!(
            hits.len() == 1 && hits[0].damage == 25.,
            "Swing {} misses close guard from {origin:?}",
            variation + 1
        );
        println!(
            "PASS Blade swing {}: contact at {:.2}, original hand {origin:?}, 25 damage",
            variation + 1,
            e.play.time
        );
    }
    for hz in [30, 60, 144] {
        for (weapon, alt, first, interval, cost) in [
            (0, false, 0.35, 1.05, 0.),
            (0, true, 0.4, 3.9, 0.),
            (1, false, 0., 0.25, 3.),
            (1, true, 0.35, 0.85, 20.),
        ] {
            let mut a = Actions::load(assets, base.len(), &base)?;
            a.selected = weapon;
            let mut stats = crate::inventory::Stats::weapon_preview();
            stats.select(weapon);
            let mut times = Vec::new();
            for frame in 0..hz * 4 {
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
                for e in e.emissions.iter().filter(|e| {
                    e.fire && e.weapon == weapon && matches!(e.play.action, Action::Attack { .. })
                }) {
                    times.push(frame as f32 / hz as f32 + e.at);
                }
                if frame == hz * 2 {
                    let saved: Snapshot =
                        serde_json::from_value(serde_json::to_value(a.snapshot())?)?;
                    a.restore(&saved)?;
                }
                a.finish_frame();
            }
            for (i, t) in times.iter().enumerate() {
                ensure!(
                    (t - (first + i as f32 * interval)).abs() < 0.0002,
                    "Cadence mismatch {weapon}/{alt} at {hz} Hz: {times:?}"
                );
            }
            ensure!(
                (stats.will() - (100. - times.len() as f32 * cost)).abs() < 0.001,
                "Duplicate Will debit"
            );
        }
        println!("PASS {hz} Hz: both Blade/Cards modes, exact release/cadence, save continuation and single Will debits");
    }
    ensure!(
        crate::dice::SUMMON != crate::duchess::ID,
        "Boss and summon targets collide"
    );
    println!("PASS original Cards reload alias exclusion, all projectile art/sounds and unique boss target");
    Ok(())
}

/// Isolated native rendering through the actual Character/funded attack path.
/// Snapshots/captures stay private and never touch campaign slots/preferences.
#[derive(serde::Serialize, serde::Deserialize)]
struct Resume {
    alice: crate::character::Snapshot,
    stats: crate::inventory::Stats,
}
pub async fn render(assets: &mut Assets) -> Result<()> {
    let root = std::path::Path::new("private/blade-cards-check");
    std::fs::create_dir_all(root)?;
    let world = World::fixture(&[(vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.))]);
    let context = CombatContext {
        world: &world,
        targets: &[],
    };
    let mut player = Player::new(Vec3::Z * 0.04);
    player.grounded = true;
    let mut alice = Character::load(assets)?;
    alice.reset(&player, 0.);
    let initial = alice.snapshot();
    for (name, weapon, alt, time, first_person, expected) in [
        ("blade-contact", 0, false, 0.5, false, 0),
        ("blade-flight", 0, true, 0.6, false, 1),
        ("blade-return", 0, true, 3.55, false, 0),
        ("blade-ready", 0, true, 4.0, false, 0),
        ("cards-primary", 1, false, 0.12, false, 1),
        ("cards-carrier", 1, true, 0.42, false, 1),
        ("cards-split", 1, true, 0.52, false, 9),
        ("blade-first-person", 0, true, 3.55, true, 0),
        ("cards-first-person", 1, true, 0.52, true, 9),
    ] {
        alice.restore(&initial)?;
        let resume_path = root.join(format!("resume-{name}.json"));
        if resume_path.exists() {
            let mut prior: Resume = serde_json::from_slice(&std::fs::read(&resume_path)?)?;
            alice.restore(&prior.alice)?;
            let state = serde_json::to_value(alice.snapshot())?;
            let mut continuation: ProjectileSave =
                serde_json::from_value(state["projectiles"].clone())?;
            ensure!(
                continuation.projectiles.len() == expected,
                "{name}: restart changed the saved projectile count"
            );
            // A downward fragment can legitimately hit the floor on the next
            // tick. Compare the saved projectiles' normal continuation, rather
            // than requiring every projectile to survive that tick.
            projectile::advance(
                &mut continuation.projectiles,
                1. / 60.,
                false,
                &context,
                None,
            );
            let will = prior.stats.will();
            let shots = alice.visual_counts().0;
            alice.update_funded(
                1. / 60.,
                &player,
                false,
                WeaponInput {
                    selected: weapon,
                    aim: Vec3::X,
                    first_person,
                    ..Default::default()
                },
                &context,
                Some(&mut prior.stats),
            );
            ensure!(
                alice.visual_counts().0 == shots && prior.stats.will() == will,
                "{name}: restart re-fired or re-charged"
            );
            let state = serde_json::to_value(alice.snapshot())?;
            let ps: ProjectileSave = serde_json::from_value(state["projectiles"].clone())?;
            ensure!(
                ps.projectiles.len() == continuation.projectiles.len()
                    && ps
                        .projectiles
                        .iter()
                        .zip(&continuation.projectiles)
                        .all(|(a, b)| {
                            a.kind == b.kind
                                && a.position.distance(b.position) < 0.001
                                && a.velocity.distance(b.velocity) < 0.001
                                && (a.age - b.age).abs() < 0.00001
                        }),
                "{name}: restart changed projectile continuation"
            );
            println!("PASS restart {name}: loaded previous process snapshot with no extra shot, fragment or debit");
            alice.restore(&initial)?;
        }

        let mut stats = crate::inventory::Stats::weapon_preview();
        stats.select(weapon);
        for _ in 0..90 {
            alice.update_funded(
                1. / 60.,
                &player,
                false,
                WeaponInput {
                    selected: weapon,
                    aim: Vec3::X,
                    first_person,
                    ..Default::default()
                },
                &context,
                Some(&mut stats),
            );
        }
        let mut elapsed = 0_f32;
        let mut return_sounds = 0;
        while elapsed < time - 0.00001 {
            let dt = (time - elapsed).min(1. / 60.);
            let sounds = alice.update_funded(
                dt,
                &player,
                false,
                WeaponInput {
                    selected: weapon,
                    aim: Vec3::X,
                    first_person,
                    click: (elapsed == 0.).then_some(alt),
                    ..Default::default()
                },
                &context,
                Some(&mut stats),
            );
            return_sounds += sounds
                .iter()
                .filter(|s| s.ends_with("weapon_reappear.wav"))
                .count();
            elapsed += dt;
        }
        let state = serde_json::to_value(alice.snapshot())?;
        let projectiles: ProjectileSave = serde_json::from_value(state["projectiles"].clone())?;
        ensure!(
            projectiles.projectiles.len() == expected,
            "{name}: expected {expected} live projectiles, got {}",
            projectiles.projectiles.len()
        );
        ensure!(projectiles.shots == 1, "{name}: duplicate attack");
        ensure!(
            (stats.will() - (100. - combat::will_cost(weapon, alt))).abs() < 0.001,
            "{name}: duplicate cost"
        );
        if weapon == 0 && alt && time > 3.4 {
            ensure!(return_sounds == 1, "Return cue count");
        }
        let saved = alice.snapshot();
        std::fs::write(
            &resume_path,
            serde_json::to_vec_pretty(&Resume {
                alice: saved.clone(),
                stats: stats.clone(),
            })?,
        )?;
        // Exercise the same validator as loading a player save, including after
        // the carrier has split, then leave it at the captured simulation time.
        let restored = serde_json::from_slice(&serde_json::to_vec(&saved)?)?;
        alice.restore(&restored)?;
        std::fs::write(
            root.join(format!("{name}.json")),
            serde_json::to_vec_pretty(&restored)?,
        )?;
        for frame in 0..3 {
            clear_background(Color::new(0.10, 0.12, 0.16, 1.));
            let target = if expected > 0 {
                vec3(100., 0., 40.)
            } else {
                vec3(0., 0., 40.)
            };
            let camera = if first_person {
                player.eye()
            } else {
                target + vec3(130., -260., 115.)
            };
            let camera_target = if first_person {
                camera + Vec3::X
            } else {
                target
            };
            set_camera(&Camera3D {
                position: camera,
                target: camera_target,
                up: Vec3::Z,
                z_near: 0.5,
                z_far: 5000.,
                fovy: 50_f32.to_radians(),
                ..Default::default()
            });
            alice.atmosphere(&crate::environment::Atmosphere::default(), camera);
            draw_cube(
                vec3(200., 0., -3.),
                vec3(1000., 1000., 4.),
                None,
                Color::new(0.16, 0.19, 0.22, 1.),
            );
            if first_person {
                alice.draw_first_person(true);
            } else {
                alice.draw(player.feet, true);
            }
            alice.draw_effects(camera, true, !first_person);
            set_default_camera();
            draw_text(name, 20., 30., 24., WHITE);
            if frame == 2 {
                crate::viewer::save_capture(&root.join(format!("{name}.png")))?;
            }
            next_frame().await;
        }
        println!("PASS {name}: one attack, {expected} live projectiles, {} Will, restored on native renderer",stats.will());
    }
    // Actual animated hand contacts against a nearby supported enemy.
    for first_person in [false, true] {
        alice.restore(&initial)?;
        let guard = combat::Guard::new(vec3(55., 0., 0.04), 0., 1.);
        let targets = [guard.target(1)];
        let context = CombatContext {
            world: &world,
            targets: &targets,
        };
        let mut hits = Vec::new();
        for frame in 0..70 {
            alice.update(
                1. / 60.,
                &player,
                false,
                WeaponInput {
                    aim: Vec3::X,
                    click: (frame == 0).then_some(false),
                    first_person,
                    ..Default::default()
                },
                &context,
            );
            hits.extend(alice.take_hits());
        }
        ensure!(
            hits.len() == 1 && hits[0].damage == 25.,
            "First person {first_person}: animated Blade contact failed"
        );
    }
    println!("PASS native Blade/Cards art, held/loose props, funded actions, return effect, live split and first/third-person hand hit detection");
    Ok(())
}
