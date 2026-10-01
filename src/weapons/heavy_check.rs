//! Checks using the player's original animation assets and full runtime saves.
use super::*;
use crate::{character::Character, inventory::Stats, movement::Player};
pub fn check(a: &mut Assets) -> Result<()> {
    let def = Definition::alice(a)?;
    let sk = crate::skeletal::Skeleton::parse(&a.read(&format!("{}/{}", def.path, def.model))?)?;
    let base =
        Animation::parse(&a.read("models/alice/ready.ska")?, sk.bones.len())?.sample(0., false);
    for hz in [30, 60, 144] {
        for alt in [false, true] {
            let mut actions = Actions::load(a, base.len(), &base)?;
            actions.selected = 8;
            let mut stats = Stats::weapon_preview();
            stats.select(8);
            let mut fire = vec![];
            let mut sound = vec![];
            for frame in 0..hz * 4 {
                let e = actions.update_funded(
                    1. / hz as f32,
                    WeaponInput {
                        selected: 8,
                        click: (frame == 0).then_some(alt),
                        aim: Vec3::X,
                        ..Default::default()
                    },
                    &base,
                    &vec![true; base.len()],
                    true,
                    Some(&mut stats),
                );
                for event in e.emissions {
                    if event.fire {
                        fire.push(frame as f32 / hz as f32 + event.at);
                    }
                    if event.sound {
                        sound.push(frame as f32 / hz as f32 + event.at);
                    }
                }
                if frame % 11 == 0 {
                    actions.restore(&serde_json::from_value(serde_json::to_value(
                        actions.snapshot(),
                    )?)?)?;
                }
                actions.finish_frame();
            }
            ensure!(
                fire.len() == 1 && (fire[0] - 0.75).abs() < 0.001,
                "Cannon release {fire:?}"
            );
            ensure!(
                sound.len() == 1 && (sound[0] - 0.05).abs() < 0.001,
                "Cannon ignition {sound:?}"
            );
            ensure!(stats.will() == 1., "Duplicate cannon debit");
            println!("PASS {hz} Hz cannon input {alt}: one ignition, one .75s release, 99 Will, saved timing");
        }
    }
    for name in [
        "w_eyestaff",
        "w_blunderbuss",
        "fx_eyestaff_spiral",
        "prj_eyestaff_comet",
        "prj_blunderbuss",
        "fx_beam",
    ] {
        let (_, m) = read_model(a, name)?;
        if name == "fx_beam" {
            println!(
                "Beam tags {:?}",
                m.tags.iter().map(|(n, v)| (n, v[0])).collect::<Vec<_>>()
            );
            println!(
                "Beam bounds {:?}",
                m.surfaces.iter().flat_map(|s| s.frames[0].iter()).fold(
                    (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                    |(l, h), &v| (l.min(v), h.max(v))
                )
            );
        }
        ensure!(
            !m.surfaces.is_empty() || !m.tags.is_empty(),
            "Missing {name}"
        );
    }
    for path in [
        heavy::CHARGE,
        heavy::BEAM,
        heavy::OFF,
        heavy::LIFT,
        heavy::EXPLODE,
        heavy::BUSS,
        heavy::BUSS_LOOP,
        "sound/weapon/blunderbuss/bb_explode.wav",
    ] {
        ensure!(a.read(path)?.len() > 44, "Missing {path}");
    }
    println!("PASS original heavy weapon models, beam rig and sound assets");
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
    world: &World,
    player: &Player,
    input: WeaponInput,
    stats: &mut Stats,
    dt: f32,
) {
    alice.update_funded(
        dt,
        player,
        false,
        input,
        &CombatContext {
            world,
            targets: &[combat::Target {
                id: 1,
                center: vec3(280., 0., 50.),
                half: vec3(24., 24., 50.),
            }],
        },
        Some(stats),
    );
}
pub async fn render(a: &mut Assets) -> Result<()> {
    let root = std::path::Path::new("private/heavy-check-v2");
    std::fs::create_dir_all(root)?;
    let world = World::fixture(&[
        (vec3(-5000., -5000., -20.), vec3(5000., 5000., 0.)),
        (vec3(500., -5000., 0.), vec3(510., 5000., 300.)),
    ]);
    let mut player = Player::new(Vec3::Z * 0.04);
    player.grounded = true;
    let mut alice = Character::load(a)?;
    alice.reset(&player, 0.);
    let initial = alice.snapshot();
    for (name, weapon, alt, time, hold, first_person) in [
        ("staff-charge", 7, false, 1.8, 4., false),
        ("staff-beam", 7, false, 3., 4., false),
        ("staff-release", 7, false, 3.1, 3., false),
        ("staff-beam-first", 7, false, 3., 4., true),
        ("staff-sky-charge", 7, true, 2.0, 2.6, false),
        ("staff-sky-queued", 7, true, 3., 2.6, false),
        ("staff-comets", 7, true, 8., 2.6, false),
        ("staff-sky-limit", 7, true, 5.7, 6., false),
        ("buss-ignition", 8, false, 0.35, 0.01, false),
        ("buss-before-release", 8, false, 0.74, 0.01, false),
        ("buss-flight", 8, false, 0.80, 0.01, false),
        ("buss-impact", 8, false, 1.05, 0.01, false),
        ("buss-secondary", 8, true, 1.4, 0.01, false),
        ("buss-stage-two", 8, false, 1.8, 0.01, false),
        ("staff-expiry", 7, false, 6.0, 3., false),
        ("buss-first", 8, false, 0.9, 0.01, true),
    ] {
        let input = WeaponInput {
            selected: weapon,
            aim: Vec3::X,
            first_person,
            click: (time < hold).then_some(alt),
            ..Default::default()
        };
        let path = root.join(format!("resume-{name}.json"));
        if path.exists() {
            let mut r: Resume = serde_json::from_slice(&std::fs::read(&path)?)?;
            alice.restore(&r.alice)?;
            tick(
                &mut alice,
                &world,
                &player,
                input.clone(),
                &mut r.stats,
                0.05,
            );
            ensure!(
                serde_json::to_value(state(&alice)?.heavy)? == serde_json::to_value(r.next.heavy)?,
                "Restart state {name}"
            );
            ensure!(
                r.stats.will() == r.will && alice.take_hits().len() == r.hits,
                "Restart Will/hits {name}"
            );
            println!(
                "PASS restart {name}: exact charge, queue, projectiles, effects, damage and Will"
            );
        }
        alice.restore(&initial)?;
        let mut stats = Stats::weapon_preview();
        stats.select(weapon);
        for _ in 0..90 {
            tick(
                &mut alice,
                &world,
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
            tick(
                &mut alice,
                &world,
                &player,
                WeaponInput {
                    click: (elapsed < hold).then_some(alt),
                    ..input.clone()
                },
                &mut stats,
                dt,
            );
            elapsed += dt;
        }
        let ps = state(&alice)?;
        ps.heavy.validate()?;
        if weapon == 8 {
            ensure!(stats.will() == 1., "Cannon cost {name}");
        }
        if name == "staff-beam" {
            ensure!(!alice.take_hits().is_empty(), "Staff beam missed target");
        }
        if name == "staff-sky-limit" {
            ensure!(
                ps.heavy.charge.is_none(),
                "Staff did not stop at five seconds"
            );
        }
        if name == "buss-impact" {
            ensure!(
                ps.heavy.impacts.len() == 1 && !alice.take_hits().is_empty(),
                "No cannon impact"
            );
        }
        let snapshot = alice.snapshot();
        if name == "staff-beam" || name == "staff-sky-charge" {
            alice.cancel_weapon_action();
            let cancelled = state(&alice)?.heavy;
            ensure!(
                cancelled.charge.is_none()
                    && cancelled.shots.is_empty()
                    && cancelled.volleys.is_empty(),
                "Cancelled Staff fired a release attack"
            );
            alice.restore(&snapshot)?;
        }
        alice.take_hits();
        let before = stats.clone();
        tick(&mut alice, &world, &player, input.clone(), &mut stats, 0.05);
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
        for frame in 0..4 {
            clear_background(Color::new(0.10, 0.12, 0.16, 1.));
            let target = vec3(140., 0., 50.);
            let camera = if first_person {
                player.eye()
            } else {
                target + vec3(130., -420., 160.)
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
            "PASS native {name}: {} projectiles, {} volleys, {} impacts",
            ps.heavy.shots.len(),
            ps.heavy.volleys.len(),
            ps.heavy.impacts.len()
        );
    }
    super::skool_check::render(a).await?;
    Ok(())
}
