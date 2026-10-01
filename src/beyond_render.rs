//! Explicitly staged visual checks, separate from the continuous gameplay route.
use crate::{assets::Assets, interaction::Interactions, render::Scene};
use anyhow::Result;
use macroquad::prelude::*;
pub async fn check(assets: &mut Assets) -> Result<()> {
    let mut scene = Scene::load(assets, "fortress2")?;
    let mut i = Interactions::load(&scene.map)?;
    i.set_entry(assets, &scene.map, "fortress2", None)?;
    i.sync(&mut scene.world);
    let catalog = crate::inventory::Catalog::load(assets)?;
    let pickups: Vec<_> = crate::inventory::pickups(&scene.map, "fortress2", &catalog)
        .into_iter()
        .filter(|p| p.id == "fortress2:37")
        .collect();
    anyhow::ensure!(pickups.len() == 1, "Rage render pickup missing");
    let mut powers = crate::powerups::Art::load(assets)?;
    let mut stats = crate::inventory::Stats::default();
    let target = pickups[0].origin + Vec3::Z * 30.;
    let eye = (0..16)
        .find_map(|n| {
            let angle = n as f32 * std::f32::consts::TAU / 16.;
            let eye = target + vec3(angle.cos() * 160., angle.sin() * 160., 55.);
            let hit = scene.world.sweep(target, eye, Vec3::splat(2.));
            (!hit.start_solid && hit.fraction >= 1.).then_some(eye)
        })
        .ok_or_else(|| anyhow::anyhow!("Rage camera obstructed"))?;
    let mut images = Vec::new();
    for collected in [false, true] {
        if collected {
            stats.collected.insert("fortress2:37".into());
        }
        clear_background(BLACK);
        set_camera(&Camera3D {
            position: eye,
            target,
            up: Vec3::Z,
            fovy: 60_f32.to_radians(),
            z_near: 2.,
            z_far: 20000.,
            ..Default::default()
        });
        crate::render_fx::begin(eye, target - eye, 2., &scene.atmosphere, false);
        scene.draw(eye, 2., false, false, &i.transforms());
        powers.draw(
            &pickups,
            &stats,
            "fortress2$first",
            &scene.atmosphere,
            eye,
            2.,
        );
        gl_use_default_material();
        scene.draw(eye, 2., false, true, &i.transforms());
        crate::render_fx::finish();
        set_default_camera();
        images.push(get_screen_data());
        crate::viewer::save_capture(std::path::Path::new(&format!(
            "private/rage-box-{collected}.png"
        )))?;
        next_frame().await;
    }
    let pixels = images[0]
        .bytes
        .chunks_exact(4)
        .zip(images[1].bytes.chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    anyhow::ensure!(
        pixels > 100,
        "Rage pickup invisible: {pixels} changed pixels"
    );
    println!("PASS Rage Box present before collection and absent afterward: {pixels} pixels");
    rage_scene(assets, &mut scene, &catalog).await?;
    let mut art = crate::beyond::Art::load(assets)?;
    for (name, eye, target) in [
        (
            "musical-puzzle",
            vec3(1300., 530., -275.),
            vec3(1430., 529., -310.),
        ),
        (
            "rolling-walkway",
            vec3(1300., 576., 60.),
            vec3(1950., 750., 20.),
        ),
        (
            "raised-room",
            vec3(5280., -4150., 100.),
            vec3(5280., -3100., 480.),
        ),
        (
            "shuffled-doors",
            vec3(-160., 4390., 470.),
            vec3(-160., 3850., 470.),
        ),
    ] {
        let b = i.beyond.as_mut().unwrap();
        let mut s = b.snapshot();
        s.age = 25.;
        s.puzzle_started = true;
        if matches!(name, "raised-room" | "shuffled-doors") {
            s.raised = Some(0.);
        }
        if name == "shuffled-doors" {
            s.last_started = Some(0.);
        }
        b.restore(&s, &scene.map)?;
        for frame in 0..3 {
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: eye,
                target,
                up: Vec3::Z,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 30000.,
                ..Default::default()
            });
            let poses = i.transforms();
            scene.draw(eye, 25., false, false, &poses);
            art.draw(i.beyond.as_ref().unwrap(), false, &scene.atmosphere, eye);
            crate::render::depth_read_only(|| scene.draw(eye, 25., false, true, &poses));
            set_default_camera();
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/beyond-{name}.png"
                )))?;
            }
            next_frame().await;
        }
    }
    use crate::beyond::cinema::Beat;
    for beat in [Beat::Demonstration, Beat::Reset, Beat::Solved, Beat::Arches] {
        let (mut i, world, mut player) =
            crate::beyond::cinema_check::setup(assets, &scene.map, beat)?;
        scene.world = world;
        let mut time = 0.;
        for at in [0.75, beat.duration() * 0.45, beat.duration() - 0.65] {
            while time < at {
                i.advance_school(1. / 120., &scene.map, &mut scene.world, &mut player)?;
                i.update(
                    1. / 120.,
                    &scene.map,
                    &mut scene.world,
                    &player,
                    Vec3::X,
                    false,
                )?;
                time += 1. / 120.;
            }
            let b = i.beyond.as_ref().unwrap();
            let camera = b.scene_camera().expect("Active puzzle camera");
            for frame in 0..3 {
                clear_background(BLACK);
                set_camera(&Camera3D {
                    position: camera.eye,
                    target: camera.target,
                    up: camera.up,
                    fovy: b
                        .scene_fovy(screen_width() / screen_height())
                        .unwrap_or(75_f32.to_radians()),
                    z_near: 2.,
                    z_far: 30000.,
                    ..Default::default()
                });
                crate::render_fx::begin(
                    camera.eye,
                    camera.target - camera.eye,
                    time,
                    &scene.atmosphere,
                    false,
                );
                scene.draw(camera.eye, time, false, false, &i.transforms());
                art.draw(b, false, &scene.atmosphere, camera.eye);
                gl_use_default_material();
                scene.draw(camera.eye, time, false, true, &i.transforms());
                crate::render_fx::finish();
                set_default_camera();
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/beyond-scene-{beat:?}-{at:.2}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
    }
    Ok(())
}

async fn rage_scene(
    assets: &mut Assets,
    scene: &mut Scene,
    catalog: &crate::inventory::Catalog,
) -> Result<()> {
    let (mut i, world, mut player) =
        crate::beyond::cinema_check::setup(assets, &scene.map, crate::beyond::cinema::Beat::Rage)?;
    scene.world = world;
    let mut alice = crate::character::Character::load(assets)?;
    let mut machinery = crate::beyond::Art::load(assets)?;
    alice.reset(&player, 0.);
    let mut stats = crate::inventory::Stats::for_level("fortress2", None);
    stats.powerup(crate::powerups::Kind::Rage);
    let hud = crate::hud::Hud::load(assets)?;
    let input = crate::input::Input::default();
    let prefs = crate::preferences::Preferences::default();
    let mut elapsed = 0.;
    for at in [0.75, 2.05, 2.65, 3.4, 5.8, 7.9, 10.5, 12.8] {
        while elapsed < at {
            i.advance_school(1. / 120., &scene.map, &mut scene.world, &mut player)?;
            i.update(
                1. / 120.,
                &scene.map,
                &mut scene.world,
                &player,
                Vec3::X,
                false,
            )?;
            elapsed += 1. / 120.;
        }
        let b = i.beyond.as_ref().unwrap();
        let (time, finished) = b.rage_scene().unwrap();
        alice.power_appearance(&stats);
        alice.rage_scene(time, finished, &player);
        let camera = b.scene_camera_in(&scene.world).unwrap_or_else(|| {
            crate::cinematic::Camera::look(player.feet + vec3(90., -100., 90.), player.eye())
        });
        let saved = serde_json::to_vec(&alice.snapshot())?;
        alice.restore(&serde_json::from_slice(&saved)?)?;
        alice.rage_scene(time, finished, &player);
        anyhow::ensure!(
            saved == serde_json::to_vec(&alice.snapshot())?,
            "Rage scene pose restore differs"
        );
        let mut baseline = None;
        for frame in 0..3 {
            let mut appearance = stats.clone();
            if frame == 0 && at > 7.4 {
                // Keep the form and pose while suppressing only its aura.
                appearance.powers.rage = 0.000001;
            }
            alice.power_appearance(&appearance);
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: camera.eye,
                target: camera.target,
                up: camera.up,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 30000.,
                ..Default::default()
            });
            crate::render_fx::begin(
                camera.eye,
                camera.target - camera.eye,
                time,
                &scene.atmosphere,
                false,
            );
            scene.draw(camera.eye, time, false, false, &i.transforms());
            machinery.draw(b, false, &scene.atmosphere, camera.eye);
            alice.atmosphere(&scene.atmosphere, camera.eye);
            let before = get_screen_data();
            alice.draw_ghost(player.feet, false, false);
            gl_use_default_material();
            let after = get_screen_data();
            let visible = before
                .bytes
                .chunks_exact(4)
                .zip(after.bytes.chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count();
            anyhow::ensure!(visible > 200, "Rage Alice invisible at {at}: {visible}");
            scene.draw(camera.eye, time, false, true, &i.transforms());
            crate::render_fx::finish();
            let rendered = get_screen_data();
            if frame == 0 {
                baseline = Some(rendered);
            } else if frame == 1 && at > 7.4 {
                let red = baseline
                    .as_ref()
                    .unwrap()
                    .bytes
                    .chunks_exact(4)
                    .zip(rendered.bytes.chunks_exact(4))
                    .filter(|(a, b)| b[0] > a[0].saturating_add(3))
                    .count();
                anyhow::ensure!(red > 100, "Rage aura missing at {at}: {red} red pixels");
                println!("PASS Rage aura {at}: {red} red pixels");
            }
            set_default_camera();
            hud.draw(&stats, catalog, false, &input, &prefs);
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/rage-scene-{at:.2}.png"
                )))?;
            }
            next_frame().await;
        }
    }
    println!("PASS Rage authored poses, scene camera, saved presentation and Will-side icon");
    Ok(())
}
