//! Staged visual checks, separate from the normal-start movement proof.
use crate::{
    assets::Assets,
    character::{Character, WeaponInput},
    interaction::Interactions,
    movement::{Controls, Player},
    render::Scene,
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
pub async fn check(assets: &mut Assets) -> Result<()> {
    check_machinery(assets).await?;
    check_cast(assets).await
}

pub async fn check_cast(assets: &mut Assets) -> Result<()> {
    for phase in [
        "warning",
        "warning-pipe",
        "warning-close",
        "warning-key",
        "vanish",
        "boarding",
        "cart",
        "rail-start",
        "wheels",
        "rail",
        "landing",
        "leave",
        "board",
        "flight",
        "flight-start",
        "flight-turn",
        "flight-end",
        "rope",
        "rope-climb",
        "key",
        "return",
    ] {
        let mut scene = Scene::load(assets, "pandemonium")?;
        let mut i = Interactions::load(&scene.map)?;
        i.set_entry(assets, &scene.map, "pandemonium", None)?;
        i.sync(&mut scene.world);
        let mut p = Player::new(vec3(-3498.4, 2082.8, -32.7));
        if phase.starts_with("rope") {
            i.update(1. / 120., &scene.map, &mut scene.world, &p, Vec3::Y, true)?;
            ensure!(
                i.pandemonium.as_ref().unwrap().controlled(),
                "Rope render fixture failed to grab"
            );
        } else {
            i.pandemonium.as_mut().unwrap().fixture(
                &format!(
                    "pand-{}",
                    if phase == "wheels" {
                        "rail-start"
                    } else if phase.starts_with("warning") {
                        "warning"
                    } else {
                        phase
                    }
                ),
                &scene.map,
                &mut scene.world,
                &mut p,
            )?;
        }
        if phase == "warning-pipe" {
            i.pandemonium.as_mut().unwrap().state.cinema.time = 0.;
        }
        if matches!(phase, "warning-close" | "warning-key") {
            let state = &mut i.pandemonium.as_mut().unwrap().state.cinema;
            state.shot = if phase == "warning-close" { 1 } else { 2 };
            state.shot_time = 4.;
        }
        if phase == "vanish" {
            // Capture the live burst before the gnome has disappeared and the
            // scene has already handed control back to the normal camera.
            i.pandemonium.as_mut().unwrap().state.cinema.time = 0.;
        }
        if phase == "key" {
            i.pandemonium.as_mut().unwrap().state.key = false;
        }
        let mut art = crate::pandemonium::Art::load(assets)?;
        let mut alice = Character::load(assets)?;
        alice.reset(&p, 0.);
        let mut rail_camera = crate::camera::Follow::default();
        let frames = if phase == "rope-climb" {
            300
        } else if matches!(phase, "landing" | "wheels") {
            30
        } else {
            75
        };
        let start_height = p.feet.z;
        let mut high = start_height;
        for frame in 0..frames {
            i.advance_school(1. / 60., &scene.map, &mut scene.world, &mut p)?;
            i.update(1. / 60., &scene.map, &mut scene.world, &p, Vec3::Y, false)?;
            if phase.starts_with("rope") {
                i.pandemonium.as_mut().unwrap().control(
                    1. / 60.,
                    &scene.world,
                    &mut p,
                    Controls {
                        wish: if phase == "rope-climb" {
                            Vec2::ZERO
                        } else {
                            vec2(-0.4, 0.2)
                        },
                        rise: if phase == "rope-climb" {
                            if frame < 120 {
                                1.
                            } else if frame < 240 {
                                -1.
                            } else {
                                0.
                            }
                        } else {
                            -0.4
                        },
                        ..Default::default()
                    },
                );
                high = high.max(p.feet.z);
            }
            alice.update(
                1. / 60.,
                &p,
                false,
                WeaponInput {
                    dice: 1,
                    selected: 0,
                    click: None,
                    aim: Vec3::X,
                    first_person: false,
                },
                &crate::combat::Context {
                    world: &scene.world,
                    targets: &[],
                },
            );
            let cinematic_camera = i.pandemonium.as_ref().unwrap().camera().map(|camera| {
                if i.pandemonium.as_ref().unwrap().rail_camera_active() {
                    rail_camera.rail(
                        &scene.world,
                        camera,
                        &i.pandemonium.as_ref().unwrap().rail_camera_ahead(),
                        1. / 60.,
                    )
                } else {
                    rail_camera.reset();
                    camera
                }
            });
            let (eye, target) = if let Some(c) = cinematic_camera {
                (c.eye, c.target)
            } else {
                match phase {
                    "rope" | "rope-climb" => {
                        let (e, t, _) = crate::character::follow_camera(
                            &scene.world,
                            p.feet,
                            vec3(1., 0., -0.2).normalize(),
                        );
                        (e, t)
                    }
                    "cart" => {
                        let (e, t, _) = crate::character::follow_camera(
                            &scene.world,
                            p.feet,
                            vec3(0., -1., -0.2).normalize(),
                        );
                        (e, t)
                    }
                    "key" => (vec3(-4312., -2330., 650.), vec3(-4312., -2528., 576.)),
                    "return" => (vec3(-3600., 760., -205.), vec3(-3600., 1000., -210.)),
                    _ => (p.feet + vec3(190., -230., 100.), p.feet),
                }
            };
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: eye,
                target,
                up: cinematic_camera.map_or(Vec3::Z, |c| c.up),
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 30000.,
                ..Default::default()
            });
            let transforms = i.transforms();
            scene.update_ropes(
                1. / 60.,
                i.pandemonium.as_ref().and_then(|p| p.rope_hand()),
                &transforms,
            );
            scene.draw(eye, frame as f32 / 60., false, false, &transforms);
            art.draw(
                i.pandemonium.as_ref().unwrap(),
                false,
                &scene.atmosphere,
                eye,
            );
            if !matches!(phase, "key" | "return") && !i.pandemonium.as_ref().unwrap().cinematic() {
                alice.atmosphere(&scene.atmosphere, eye);
                alice.draw(p.feet, false);
            }
            crate::render::depth_read_only(|| {
                scene.draw(eye, frame as f32 / 60., false, true, &transforms);
                art.draw_effects(i.pandemonium.as_ref().unwrap(), eye, &scene.atmosphere);
            });
            set_default_camera();
            draw_text(
                &format!("STAGED VISUAL CHECK / {phase}"),
                24.,
                32.,
                22.,
                WHITE,
            );
            if frame == frames - 1 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/pand-{phase}.png"
                )))?;
            }
            if phase == "rope-climb" && [59, 119, 179, 239].contains(&frame) {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/pand-rope-climb-{frame}.png"
                )))?;
            }
            next_frame().await;
        }
        if phase == "rope-climb" {
            ensure!(
                high > start_height + 40. && p.feet.z < high - 40.,
                "Rope fixture must climb both ways"
            );
            println!(
                "PASS rendered rope ascent/descent: {start_height} -> {high} -> {}",
                p.feet.z
            );
        }
        alice.check_visible(&format!("pandemonium/{phase}")).await?;
    }
    println!("PASS Pandemonium native models, materials and mounted animation fixtures");
    Ok(())
}

/// Fixed inspection cameras; mechanism clocks still advance through the live
/// interaction update. These views do not claim entrance-to-exit traversal.
pub async fn check_machinery(assets: &mut Assets) -> Result<()> {
    for (name, eye, target) in [
        (
            "widget",
            vec3(-3870., 1540., 210.),
            vec3(-4090., 1710., 145.),
        ),
        ("legs", vec3(-3860., 2000., 210.), vec3(-4100., 2000., 135.)),
        ("cams", vec3(-4420., -200., 560.), vec3(-4504., -382., 560.)),
    ] {
        let mut scene = Scene::load(assets, "pandemonium")?;
        let mut i = Interactions::load(&scene.map)?;
        i.set_entry(assets, &scene.map, "pandemonium", None)?;
        i.sync(&mut scene.world);
        // The rail crosses in front of the cams. Find a clear inspection view
        // of the entire wheel, rather than capturing its arm behind that beam.
        let eye = if name == "cams" {
            let mut best = None;
            for x in (-240..=240).step_by(40) {
                for y in (160..=400).step_by(40) {
                    for z in (-160..=160).step_by(40) {
                        let candidate = target + vec3(x as f32, y as f32, z as f32);
                        if !scene
                            .world
                            .body_clear(candidate - crate::collision::PLAYER_CENTER)
                        {
                            continue;
                        }
                        let visible = [
                            Vec3::ZERO,
                            Vec3::X * 50.,
                            -Vec3::X * 50.,
                            Vec3::Z * 50.,
                            -Vec3::Z * 50.,
                        ]
                        .iter()
                        .all(|offset| {
                            let ray =
                                scene
                                    .world
                                    .sweep(candidate, target + *offset, Vec3::splat(2.));
                            !ray.start_solid && ray.fraction >= 1.
                        });
                        let distance = candidate.distance_squared(eye);
                        if visible && best.is_none_or(|(_, score)| distance < score) {
                            best = Some((candidate, distance));
                        }
                    }
                }
            }
            let (eye, _) = best.ok_or_else(|| anyhow::anyhow!("No clear cam inspection view"))?;
            println!("Machinery cam inspection eye {eye:?}");
            eye
        } else {
            eye
        };
        ensure!(
            scene
                .world
                .body_clear(eye - crate::collision::PLAYER_CENTER),
            "Machinery inspection camera {name} is inside geometry"
        );
        let mut player = Player::new(vec3(-3498.4, 2082.8, -32.7));
        let mut steam = crate::particles::Steam::load(assets, &scene.map)?;
        for frame in 0..420 {
            i.advance_school(1. / 60., &scene.map, &mut scene.world, &mut player)?;
            steam.sync(&i.event_world);
            i.pandemonium.as_ref().unwrap().gate_particles(&mut steam);
            steam.update(1. / 60., eye, &scene.world);
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
            let transforms = i.transforms();
            let time = (frame + 1) as f32 / 60.;
            scene.draw(eye, time, false, false, &transforms);
            crate::render::depth_read_only(|| {
                scene.draw_with_particles(
                    eye,
                    (target - eye).normalize(),
                    time,
                    false,
                    &transforms,
                    &steam,
                );
            });
            set_default_camera();
            draw_text(
                &format!("STAGED MACHINERY CHECK / {name} / {time:.2}s"),
                24.,
                32.,
                22.,
                WHITE,
            );
            if [0, 59, 68, 77, 98, 131, 161, 185, 245].contains(&frame) {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/pand-machinery-{name}-{frame}.png"
                )))?;
            }
            next_frame().await;
        }
        println!("PASS native machinery animation and phased steam: {name}");
    }
    Ok(())
}
