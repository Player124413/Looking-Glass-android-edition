//! Archive-backed fidelity checks and restartable native render fixtures.
use super::*;
use crate::{character::Character, movement::Player};

pub fn check(assets: &mut Assets) -> Result<()> {
    super::croquet_check::check(assets)?;
    let data = bomb::Data::load(assets)?;
    ensure!(
        (data.crank - 1.2).abs() < 0.001 && (data.open - 0.3).abs() < 0.001,
        "Changed original Jackbomb timing"
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
    let hand = skeleton
        .bones
        .iter()
        .position(|b| b.name == "tag_weapon")
        .context("Hand tag")?;
    let world = World::fixture(&[(vec3(-5000., -5000., -20.), vec3(5000., 5000., 0.))]);
    let target = combat::Guard::new(vec3(55., 0., 0.04), 0., 1.).target(1);
    for clip in [7, 8] {
        let mut a = Actions::load(assets, base.len(), &base)?;
        a.selected = 2;
        a.begin(
            clip,
            Action::Attack { alternate: false },
            Vec3::X,
            crate::weapon_rules::Stage::Shot,
        );
        let e = a.update(
            0.4,
            WeaponInput {
                selected: 2,
                aim: Vec3::X,
                ..Default::default()
            },
            &base,
            &upper,
            true,
        );
        let fire = e
            .emissions
            .iter()
            .find(|e| e.fire)
            .context("No mallet contact")?;
        let pose = skeleton.global_pose(&fire.pose);
        let origin = pose[hand].translation * def.scale;
        let hits = projectile::melee_toy(
            &CombatContext {
                world: &world,
                targets: &[target],
            },
            origin,
            Vec3::X,
            true,
        );
        ensure!(
            hits.len() == 1 && hits[0].damage == 24. && hits[0].knockback.length() == 100.,
            "Mallet swing misses guard: {origin:?}"
        );
        println!("PASS mallet clip {clip}: contact at {:.2}, original hand {origin:?}, 24 damage/100 knockback",fire.play.time);
    }
    for hz in [30, 60, 144] {
        for (weapon, alt, first, interval, cost) in [
            (2, false, 0.35, 0.8, 0.),
            (2, true, 0.65, 1.35, 8.),
            (3, false, 0.35, 3.35, 15.),
            (3, true, 0.35, 8.35, 20.),
        ] {
            let mut a = Actions::load(assets, base.len(), &base)?;
            a.selected = weapon;
            let mut stats = crate::inventory::Stats::weapon_preview();
            stats.select(weapon);
            let mut times = vec![];
            for frame in 0..hz * 10 {
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
                if frame == hz * 2 || frame == hz * 5 {
                    let saved = serde_json::from_value(serde_json::to_value(a.snapshot())?)?;
                    a.restore(&saved)?;
                }
                a.finish_frame();
            }
            for (i, t) in times.iter().enumerate() {
                ensure!(
                    (t - (first + i as f32 * interval)).abs() < 0.001,
                    "Cadence {weapon}/{alt}/{hz}: {times:?}"
                );
            }
            ensure!(
                (stats.will() - (100. - cost * times.len() as f32)).abs() < 0.001,
                "Duplicated resource use"
            );
        }
        let ctx = CombatContext {
            world: &world,
            targets: &[],
        };
        let owner = combat::Target {
            id: crate::dice::ALICE,
            center: vec3(-5000., 0., 50.),
            half: Vec3::ONE,
        };
        for alt in [false, true] {
            let mut state = bomb::State::default();
            state.launch(vec3(0., 0., 50.), Vec3::X, Vec3::ZERO, alt);
            let (mut explosions, mut popped, mut emitted) = (0, 0, 0);
            for frame in 0..hz * 11 {
                let e = state.advance(1. / hz as f32, &ctx, owner, &data);
                explosions += e.blasts.len();
                popped += e.sounds.iter().filter(|(p, _)| *p == bomb::POP).count();
                if let Some(b) = state.bombs.first() {
                    emitted = b.flames;
                }
                if frame % hz == 0 {
                    state = serde_json::from_value(serde_json::to_value(&state)?)?;
                    state.validate()?;
                }
            }
            ensure!(
                (explosions, popped, emitted) == (1, 1, if alt { 46 } else { 0 }),
                "Jack event mismatch {hz}/{alt}: {explosions}/{popped}/{emitted}"
            );
        }
        println!("PASS {hz} Hz: mallet/Jackbomb firing, Will, saved reuse delays, both fuses, 46 animated flame births");
    }
    let mut b = bomb::State::default();
    b.launch(Vec3::ZERO, Vec3::X, Vec3::ZERO, true);
    let b = &mut b.bombs[0];
    b.resting = true;
    b.angles = Vec2::ZERO;
    b.age = 1.55;
    let a = data.tag(b, "tag_mouth");
    b.age = 2.35;
    let z = data.tag(b, "tag_mouth");
    ensure!(
        (a.rotation * Vec3::X).distance(z.rotation * Vec3::X) > 0.2,
        "Animated mouth does not rotate"
    );
    for path in [
        bomb::MUSIC,
        bomb::POP,
        bomb::BREATH,
        bomb::EXPLODE,
        bomb::TOSS,
        "sound/weapon/mallet/mallet_ball_flesh1.wav",
        "sound/weapon/mallet/mallet_ball_bounce.wav",
    ] {
        ensure!(!assets.read(path)?.is_empty(), "Missing {path}");
    }
    println!("PASS animated mouth orientation and original sound assets");
    Ok(())
}
#[derive(serde::Serialize, serde::Deserialize)]
struct Resume {
    alice: crate::character::Snapshot,
    stats: crate::inventory::Stats,
    next: ProjectileSave,
    will: f32,
    hits: usize,
}
fn projectile_save(alice: &Character) -> Result<ProjectileSave> {
    Ok(serde_json::from_value(
        serde_json::to_value(alice.snapshot())?["projectiles"].clone(),
    )?)
}
fn compare(a: &ProjectileSave, b: &ProjectileSave) -> Result<()> {
    ensure!(
        (
            a.shots,
            a.impacts,
            a.projectiles.len(),
            a.bombs.bombs.len(),
            a.bombs.flames.len(),
            a.blasts.len()
        ) == (
            b.shots,
            b.impacts,
            b.projectiles.len(),
            b.bombs.bombs.len(),
            b.bombs.flames.len(),
            b.blasts.len()
        ),
        "Restored event/entity counts differ"
    );
    for (a, b) in a.bombs.bombs.iter().zip(&b.bombs.bombs) {
        ensure!(
            a.position.distance(b.position) < 0.001
                && a.velocity.distance(b.velocity) < 0.001
                && (a.age - b.age).abs() < 0.0001
                && a.flames == b.flames
                && a.popped == b.popped,
            "Restored bomb differs"
        );
    }
    for (a, b) in a.bombs.flames.iter().zip(&b.bombs.flames) {
        ensure!(
            a.position.distance(b.position) < 0.001
                && a.velocity.distance(b.velocity) < 0.001
                && (a.age - b.age).abs() < 0.0001
                && a.id == b.id,
            "Restored flame differs"
        );
    }
    for (a, b) in a.projectiles.iter().zip(&b.projectiles) {
        ensure!(
            a.position.distance(b.position) < 0.001 && (a.age - b.age).abs() < 0.0001,
            "Restored ball differs"
        );
    }
    Ok(())
}
pub async fn render(assets: &mut Assets) -> Result<()> {
    super::croquet_check::render(assets).await?;
    let root = std::path::Path::new("private/mallet-jack-check");
    std::fs::create_dir_all(root)?;
    let world = World::fixture(&[
        (vec3(-5000., -5000., -20.), vec3(5000., 5000., 0.)),
        (vec3(500., -5000., 0.), vec3(510., 5000., 300.)),
    ]);
    let context = CombatContext {
        world: &world,
        targets: &[],
    };
    let mut player = Player::new(Vec3::Z * 0.04);
    player.grounded = true;
    let mut alice = Character::load(assets)?;
    alice.reset(&player, 0.);
    let initial = alice.snapshot();
    for (name, weapon, alt, time, first_person) in [
        ("mallet-contact", 2, false, 0.36, false),
        ("mallet-launch", 2, true, 0.67, false),
        ("mallet-bounce", 2, true, 0.95, false),
        ("jack-cranking", 3, false, 1.0, false),
        ("jack-opening", 3, false, 1.6, false),
        ("jack-before-explosion", 3, false, 3.34, false),
        ("jack-explosion", 3, false, 3.55, false),
        ("jack-fire", 3, true, 3.5, false),
        ("jack-fire-repeat", 3, true, 7., false),
        ("jack-before-alt-explosion", 3, true, 10.34, false),
        ("jack-alt-explosion", 3, true, 10.6, false),
        ("jack-first-person", 3, true, 0.15, true),
    ] {
        let input = WeaponInput {
            selected: weapon,
            aim: Vec3::X,
            first_person,
            ..Default::default()
        };
        let path = root.join(format!("resume-{name}.json"));
        if path.exists() {
            let mut s: Resume = serde_json::from_slice(&std::fs::read(&path)?)?;
            alice.restore(&s.alice)?;
            alice.update_funded(
                0.05,
                &player,
                false,
                input.clone(),
                &context,
                Some(&mut s.stats),
            );
            compare(&projectile_save(&alice)?, &s.next)?;
            ensure!(
                s.stats.will() == s.will && alice.take_hits().len() == s.hits,
                "Restart extra cost/damage"
            );
            println!("PASS restart {name}: exact continuation, no repeated attack, debit, flame or explosion");
        }
        alice.restore(&initial)?;
        let mut stats = crate::inventory::Stats::weapon_preview();
        stats.select(weapon);
        for _ in 0..90 {
            alice.update_funded(
                1. / 60.,
                &player,
                false,
                input.clone(),
                &context,
                Some(&mut stats),
            );
        }
        let mut elapsed = 0_f32;
        while elapsed < time - 0.00001 {
            let dt = (time - elapsed).min(1. / 60.);
            let mut i = input.clone();
            i.click = (elapsed == 0.).then_some(alt);
            alice.update_funded(dt, &player, false, i, &context, Some(&mut stats));
            elapsed += dt;
        }
        let ps = projectile_save(&alice)?;
        let committed = time
            >= if weapon == 2 {
                if alt {
                    0.65
                } else {
                    0.35
                }
            } else {
                0.35
            };
        ensure!(ps.shots == u64::from(committed), "{name} shot count");
        ensure!(
            stats.will()
                == 100.
                    - if committed {
                        combat::will_cost(weapon, alt)
                    } else {
                        0.
                    },
            "{name} cost"
        );
        if weapon == 3 && committed {
            ensure!(
                ps.bombs.bombs.len() == usize::from(time < if alt { 10.35 } else { 3.35 }),
                "{name} fuse"
            );
        }
        let snapshot = alice.snapshot();
        alice.take_hits();
        let before = stats.clone();
        alice.update_funded(
            0.05,
            &player,
            false,
            input.clone(),
            &context,
            Some(&mut stats),
        );
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&Resume {
                alice: snapshot.clone(),
                stats: before,
                next: projectile_save(&alice)?,
                will: stats.will(),
                hits: alice.take_hits().len(),
            })?,
        )?;
        alice.restore(&snapshot)?;
        let target = ps
            .bombs
            .bombs
            .first()
            .map(|b| b.position + Vec3::Z * 20.)
            .or_else(|| ps.blasts.first().map(|b| b.origin))
            .or_else(|| ps.projectiles.first().map(|p| p.position))
            .unwrap_or(vec3(20., 0., 40.));
        for frame in 0..4 {
            clear_background(Color::new(0.10, 0.12, 0.16, 1.));
            let camera = if first_person {
                player.eye()
            } else {
                target + vec3(130., -240., 100.)
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
            "PASS native {name}: saved and rendered {} balls, {} bombs, {} flames, {} explosions",
            ps.projectiles.len(),
            ps.bombs.bombs.len(),
            ps.bombs.flames.len(),
            ps.blasts.len()
        );
    }
    Ok(())
}
