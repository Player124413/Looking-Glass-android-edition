//! Original-data camera pairs and deterministic presentation regressions.
use crate::{assets::Assets, render::Scene};
use anyhow::{ensure, Result};
use macroquad::prelude::*;

fn draw(scene: &mut Scene, camera: &Camera3D, time: f32, transforms: &[(usize, Vec3, Quat)]) {
    clear_background(scene.atmosphere.background());
    set_camera(camera);
    crate::lighting::select(vec![], camera.position, &scene.world);
    crate::render_fx::begin_view(camera, time, &scene.atmosphere, false);
    scene.draw(camera.position, time, false, false, transforms);
    scene.draw(camera.position, time, false, true, transforms);
    let (_, dropped) = crate::render_fx::finish();
    assert_eq!(dropped, 0);
}
fn camera(position: Vec3, direction: Vec3) -> Camera3D {
    Camera3D {
        position,
        target: position + direction,
        up: Vec3::Z,
        fovy: 75_f32.to_radians(),
        z_near: 2.,
        z_far: 30000.,
        ..Default::default()
    }
}
fn capture(path: &str) -> Vec<u8> {
    let image = get_screen_data();
    image.export_png(path);
    image.bytes
}
fn same_pixels(a: &[u8], b: &[u8]) -> bool {
    // Drivers may dither a handful of edge pixels by one 8-bit step.
    a.len() == b.len()
        && a.iter().zip(b).all(|(&a, &b)| a.abs_diff(b) <= 1)
        && a.chunks_exact(4)
            .zip(b.chunks_exact(4))
            .filter(|(a, b)| a != b)
            .count()
            <= 32
}
pub async fn check(assets: &mut Assets) -> Result<()> {
    std::fs::create_dir_all("private/sky-performance")?;
    let mut report = Vec::new();
    let mut saved_triangles = 0;
    let mut detail_saved = 0;
    for name in ["skool1", "potears1", "hedge1", "hatter1"] {
        let mut scene = Scene::load(assets, name)?;
        let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
        interactions.set_entry(assets, &scene.map, name, None)?;
        interactions.sync(&mut scene.world);
        interactions.presentation.apply(&mut scene);
        let transforms = interactions.transforms();
        let (eye, yaw) = crate::interaction::spawn(&scene.map, None);
        for view in 0..3 {
            let a = yaw + view as f32 * 2.094395;
            let camera = camera(eye, vec3(a.cos(), a.sin(), 0.15));
            let mut captures = Vec::new();
            let mut costs = Vec::new();
            let mut counts = Vec::new();
            for optimized in [false, true] {
                scene.optimize = optimized;
                let mut samples = Vec::new();
                for frame in 0..10 {
                    let start = std::time::Instant::now();
                    draw(&mut scene, &camera, 8., &transforms);
                    // Readback waits for submitted rendering; presentation pacing
                    // is outside this interval. Includes a fixed readback cost.
                    let image = get_screen_data();
                    if frame >= 3 {
                        samples.push(start.elapsed().as_secs_f64() * 1000.);
                    }
                    if frame == 9 {
                        image.export_png(&format!(
                            "private/sky-performance/{name}-{view}-{optimized}.png"
                        ));
                        captures.push(image.bytes);
                    }
                    set_default_camera();
                    next_frame().await;
                }
                samples.sort_by(f64::total_cmp);
                costs.push(samples[samples.len() / 2]);
                counts.push(scene.stats);
            }
            let mut error = 0_f64;
            let mut changed = 0_usize;
            for (a, b) in captures[0].chunks_exact(4).zip(captures[1].chunks_exact(4)) {
                if a[..3] != b[..3] {
                    changed += 1;
                }
                error += a[..3]
                    .iter()
                    .zip(&b[..3])
                    .map(|(&a, &b)| (a as f64 - b as f64).powi(2))
                    .sum::<f64>();
            }
            let pixels = captures[0].len() / 4;
            let rmse = (error / (pixels * 3) as f64).sqrt();
            println!("PAIR {name}/{view}: triangles {} -> {}, detail {}, render+readback {:.3} -> {:.3} ms, pixels {changed}/{pixels}, RMSE {rmse:.4}",counts[0].triangles,counts[1].triangles,counts[1].detail_saved,costs[0],costs[1]);
            report.push(serde_json::json!({"map":name,"view":view,"baseline":counts[0],"optimized":counts[1],"median_render_readback_ms":costs,"changed_pixels":changed,"pixels":pixels,"rmse":rmse}));
            ensure!(
                rmse < 0.75 && changed as f64 / (pixels as f64) < 0.01,
                "Visible regression in {name}/{view}: {rmse}"
            );
            saved_triangles += counts[0].triangles.saturating_sub(counts[1].triangles);
            detail_saved += counts[1].detail_saved;
        }
    }
    ensure!(saved_triangles > 1000, "No useful visibility savings");
    println!("DETAIL observed {detail_saved} removed triangles in representative views");
    std::fs::write(
        "private/sky-performance/profile.json",
        serde_json::to_vec_pretty(&report)?,
    )?;
    Scene::load(assets, "potears1")?.check_detail().await?;
    for name in [
        "hedge1", "hedge3", "garden2", "garden4", "hatter1", "hatter2",
    ] {
        let mut scene = Scene::load(assets, name)?;
        let mut c = crate::sky_sequence::Controller::load(assets, &scene.map, name)?;
        let (eye, yaw) = crate::interaction::spawn(&scene.map, None);
        let camera = camera(eye, vec3(yaw.cos(), yaw.sin(), 0.65));
        c.apply(&mut scene);
        draw(&mut scene, &camera, 0., &[]);
        let first = capture(&format!("private/sky-performance/{name}-sequence-0.png"));
        next_frame().await;
        if name.starts_with("hedge") {
            ensure!(c.event("change_to_sky2"), "Missing sky switch");
        }
        if name == "garden2" {
            ensure!(c.event("Garden2_Fog1"), "Missing fog event");
        }
        if name == "garden4" {
            ensure!(c.event("Garden4_Fade_Fog"), "Missing fog event");
        }
        c.update(0.5);
        let state = c.state.clone();
        c.apply(&mut scene);
        draw(&mut scene, &camera, 8., &[]);
        let moved = capture(&format!("private/sky-performance/{name}-sequence-1.png"));
        next_frame().await;
        c.update(0.);
        c.apply(&mut scene);
        draw(&mut scene, &camera, 8., &[]);
        ensure!(
            same_pixels(
                &capture(&format!("private/sky-performance/{name}-paused.png")),
                &moved
            ),
            "Pause changed presentation in {name}"
        );
        next_frame().await;
        c.update(4.);
        c.restore(&state)?;
        c.apply(&mut scene);
        draw(&mut scene, &camera, 8., &[]);
        ensure!(
            same_pixels(
                &capture(&format!("private/sky-performance/{name}-restored.png")),
                &moved
            ),
            "Restore changed presentation in {name}"
        );
        next_frame().await;
        ensure!(first != moved, "Presentation did not change in {name}");
        println!("PASS {name} presentation, pause and restored pixels");
    }
    let mut scene = Scene::load(assets, "skool1")?;
    let catalog = crate::inventory::Catalog::load(assets)?;
    let hud = crate::hud::Hud::load(assets)?;
    let items = crate::inventory::pickups(&scene.map, "skool1", &catalog);
    let item = items
        .iter()
        .find(|p| matches!(p.kind, crate::inventory::PickupKind::Weapon(_)))
        .ok_or_else(|| anyhow::anyhow!("Missing pickup fixture"))?;
    let camera = camera(item.origin + vec3(100., 0., 60.), vec3(-100., 0., -16.));
    let mut steam = crate::particles::Steam::load(assets, &scene.map)?;
    for _ in 0..120 {
        steam.update(1. / 60., camera.position, &scene.world);
    }
    let mut stats = crate::inventory::Stats::for_level("skool1", None);
    let mut visibility = Vec::new();
    for particles in [false, true] {
        let mut images = Vec::new();
        for model in [false, true] {
            clear_background(scene.atmosphere.background());
            set_camera(&camera);
            crate::lighting::select(steam.lights(), camera.position, &scene.world);
            crate::render_fx::begin_view(&camera, 2., &scene.atmosphere, false);
            scene.draw(camera.position, 2., false, false, &[]);
            if model {
                hud.draw_pickups(std::slice::from_ref(item), &stats, camera.position, 2.);
            }
            if particles {
                scene.draw_with_particles(
                    camera.position,
                    camera.target - camera.position,
                    2.,
                    false,
                    &[],
                    &steam,
                );
            } else {
                scene.draw(camera.position, 2., false, true, &[]);
            }
            crate::render_fx::finish();
            images.push(get_screen_data().bytes);
            next_frame().await;
        }
        let readable = images[0]
            .chunks_exact(4)
            .zip(images[1].chunks_exact(4))
            .filter(|(a, b)| {
                a[..3]
                    .iter()
                    .zip(&b[..3])
                    .any(|(&a, &b)| a.abs_diff(b) > 12)
            })
            .count();
        visibility.push(readable);
    }
    ensure!(
        visibility[0] > 100 && visibility[1] > visibility[0] / 2,
        "Pickup obscured by effect: {visibility:?}"
    );
    println!(
        "PASS pickup contrast pixels: bare {}, with plume {}",
        visibility[0], visibility[1]
    );
    for collected in [false, true] {
        if collected {
            stats.collected.insert(item.id.clone());
            steam.collected(&items, &stats);
        }
        clear_background(scene.atmosphere.background());
        set_camera(&camera);
        crate::lighting::select(steam.lights(), camera.position, &scene.world);
        crate::render_fx::begin_view(&camera, 2., &scene.atmosphere, false);
        scene.draw(camera.position, 2., false, false, &[]);
        hud.draw_pickups(&items, &stats, camera.position, 2.);
        scene.draw_with_particles(
            camera.position,
            camera.target - camera.position,
            2.,
            false,
            &[],
            &steam,
        );
        crate::render_fx::finish();
        capture(&format!("private/sky-performance/pickup-{collected}.png"));
        next_frame().await;
    }
    println!("PASS sky/performance checks; artifacts private/sky-performance");
    Ok(())
}
