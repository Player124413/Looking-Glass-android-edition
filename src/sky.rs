//! Portal viewpoints are data references, not executable map scripts.
use crate::{assets::Assets, bsp::Bsp};
use anyhow::Result;
use macroquad::prelude::*;
use std::collections::BTreeSet;

/// Remaster presentation: carry the existing vortex into Fortress 1's outer void.
/// Only large outer-shell faces in the playable exterior and airship staging
/// rooms qualify. Windows, props and the interior sky room remain opaque.
pub fn fortress_exterior(name: &str, model: usize, material: &str, min: Vec3, max: Vec3) -> bool {
    name == "fortress1"
        && model == 0
        && material == "textures/common/black"
        && min.y >= 1600.
        && (max.x <= 2752. || (min.x >= 3144. && max.x <= 6720. && min.z >= 200. && max.z <= 1528.))
        && (max - min).max_element() >= 1000.
}

pub fn origin(assets: &mut Assets, name: &str, map: &Bsp) -> Result<Option<Vec3>> {
    if name == "keep" { return Ok(map.entities.iter().find(|e| e.get("targetname").is_some_and(|n| n=="black_sky")).and_then(|e| e.get("origin")).and_then(|s| crate::interaction::vector(s))); }
    let script = format!("maps/{name}.scr");
    let mut names = BTreeSet::new();
    if assets.contains(&script) {
        let text = String::from_utf8_lossy(&assets.read(&script)?).into_owned();
        if matches!(name, "hedge1" | "hedge3" | "grounds1") {
            let initial = crate::sky_sequence::sky_target(&crate::sky_sequence::body(
                &text,
                if name == "grounds1" {
                    "WorldInit"
                } else {
                    "main"
                },
            ));
            if let Some(p) = map
                .entities
                .iter()
                .find(|e| e.get("targetname") == initial.as_ref())
                .and_then(|e| e.get("origin"))
                .and_then(|p| crate::interaction::vector(p))
            {
                return Ok(Some(p));
            }
        }
        for line in crate::materials::lines(&text) {
            let line = line.join(" ");
            if let Some((target, effect)) = line.split_once(".rendereffects") {
                if effect.to_ascii_lowercase().contains("+skyorigin") {
                    if let Some(target) = target.trim().strip_prefix('$') {
                        names.insert(target.to_owned());
                    }
                }
            }
        }
    }
    let points = map
        .entities
        .iter()
        .filter(|e| {
            e.get("classname").is_some_and(|c| c == "script_skyorigin")
                || e.get("targetname").is_some_and(|n| names.contains(n))
        })
        .filter_map(|e| e.get("origin").and_then(|p| crate::interaction::vector(p)))
        .collect::<Vec<_>>();
    // A changing sky camera needs its owning level controller. Do not pick an
    // arbitrary room from mutually exclusive views (notably the hedge maps).
    Ok(if points.len() == 1 {
        Some(
            points[0]
                + if name == "fortress1" {
                    vec3(0., -64., 40.)
                } else if name == "fortress2" {
                    vec3(0., -40., 32.)
                } else {
                    Vec3::ZERO
                },
        )
    } else {
        None
    })
}

pub fn check(assets: &mut Assets) -> Result<()> {
    crate::particles::check(assets)?;
    let specs = crate::texture::read_materials(assets)?;
    let mut portals = 0;
    let mut skies = 0;
    let mut reflective = 0;
    for name in assets.maps() {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let sky = map
            .shaders
            .iter()
            .filter(|s| s.flags & 4 != 0 || specs.get(&s.name).is_some_and(|s| s.sky))
            .count();
        skies += usize::from(sky > 0);
        let origin = origin(assets, &name, &map)?;
        portals += usize::from(origin.is_some());
        reflective += map
            .shaders
            .iter()
            .filter(|s| {
                specs
                    .get(&s.name)
                    .is_some_and(|s| s.stages.iter().any(|s| s.environment))
            })
            .count();
        if sky > 0 {
            println!("Sky {name}: {sky} materials; portal {origin:?}");
        }
    }
    anyhow::ensure!(
        portals >= 10 && skies >= 10 && reflective > 0,
        "Missing authored rendering data"
    );
    println!("PASS {skies} sky maps, {portals} unambiguous portal viewpoints, {reflective} reflective map materials");
    Ok(())
}

/// Explicitly staged camera comparisons, separate from playable traversal proof.
pub async fn render_check(assets: &mut Assets) -> Result<()> {
    crate::particles::ambient_render(assets).await?;
    for (name, eye, target) in [
        ("skool1", vec3(208., 4200., 1450.), vec3(208., 4700., 1700.)),
        ("skool2", vec3(-64., -3440., 70.), vec3(-64., -2800., 480.)),
        (
            "garden1",
            vec3(-1772., -600., -220.),
            vec3(-1772., -160., -140.),
        ),
        (
            "potears1",
            vec3(-200., 3000., 2150.),
            vec3(-448., 3352., 2000.),
        ),
        (
            "gvillage",
            vec3(-5200., 5000., 250.),
            vec3(-3100., 3000., 600.),
        ),
        (
            "pandemonium",
            vec3(-4150., 2750., -170.),
            vec3(-3700., 2400., -376.),
        ),
        (
            "pandemonium",
            vec3(-3320., -700., -400.),
            vec3(-2950., -350., -704.),
        ),
        ("wforest", Vec3::ZERO, Vec3::ZERO),
    ] {
        let mut scene = crate::render::Scene::load(assets, name)?;
        if name == "skool1" {
            crate::render::check_layers(&scene).await?;
        }
        let lower_slime = name == "pandemonium" && eye.z < -300.;
        let (eye, target) = if name == "pandemonium" {
            let surface = scene
                .map
                .surfaces
                .iter()
                .filter(|s| scene.map.shaders[s.shader].name == "textures/liquid/green_slime2_1")
                .nth(usize::from(lower_slime))
                .unwrap();
            let (vertices, indices) = scene.map.triangulate(surface);
            let mut view = None;
            'triangles: for triangle in indices.chunks_exact(3) {
                let target = triangle
                    .iter()
                    .map(|&i| vertices[i as usize].position)
                    .sum::<Vec3>()
                    / 3.;
                for offset in [
                    vec3(160., 0., 80.),
                    vec3(-160., 0., 80.),
                    vec3(0., 160., 80.),
                    vec3(0., -160., 80.),
                ] {
                    let eye = target + offset;
                    let sight = scene
                        .world
                        .sweep(target + Vec3::Z * 8., eye, Vec3::splat(4.));
                    if !sight.start_solid
                        && sight.fraction == 1.
                        && scene.world.body_clear(eye - Vec3::Z * 48.)
                    {
                        view = Some((eye, target));
                        break 'triangles;
                    }
                }
            }
            anyhow::ensure!(view.is_some(), "No clear slime inspection camera");
            view.unwrap()
        } else if eye == Vec3::ZERO {
            let (eye, yaw) = crate::interaction::spawn(&scene.map, None);
            (eye, eye + vec3(yaw.cos(), yaw.sin(), 0.4) * 300.)
        } else {
            (eye, target)
        };
        let mut particles = crate::particles::Steam::load(assets, &scene.map)?;
        let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
        interactions.set_entry(assets, &scene.map, name, None)?;
        interactions.sync(&mut scene.world);
        if name == "garden1" {
            particles.check_runtime(&scene.world, &mut interactions.event_world)?;
        }
        println!(
            "Camera {name}: {eye:?}, body clear {}",
            scene.world.body_clear(eye - Vec3::Z * 48.)
        );
        let transforms = interactions.transforms();
        for frame in 0..121 {
            clear_background(scene.atmosphere.background());
            set_camera(&Camera3D {
                position: eye,
                target,
                up: Vec3::Z,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 30000.,
                ..Default::default()
            });
            particles.update(1. / 60., eye, &scene.world);
            scene.draw(eye, frame as f32 / 60., false, false, &transforms);
            crate::render::depth_read_only(|| {
                scene.draw_with_particles(
                    eye,
                    (target - eye).normalize_or_zero(),
                    frame as f32 / 60.,
                    false,
                    &transforms,
                    &particles,
                );
            });
            if frame == 120 {
                let suffix = if lower_slime { "-lower" } else { "" };
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/fidelity-{name}{suffix}.png"
                )))?;
                println!(
                    "PASS staged {name}: {} triangles, {} missing images {:?}",
                    scene.triangles,
                    scene.missing.len(),
                    scene.missing
                );
            }
            set_default_camera();
            next_frame().await;
        }
    }
    crate::render::check_depth().await?;
    Ok(())
}

/// Loading all maps catches late-campaign texture/model/parser regressions.
/// Entrance screenshots are smoke tests, not proof of those maps' playability.
pub async fn corpus_check(assets: &mut Assets) -> Result<()> {
    fn resident_pixels(scene: &mut crate::render::Scene) -> Vec<u8> {
        let eye = vec3(208., 4200., 1450.);
        clear_background(scene.atmosphere.background());
        set_camera(&Camera3D {
            position: eye,
            target: vec3(208., 4700., 1700.),
            up: Vec3::Z,
            fovy: 75_f32.to_radians(),
            z_near: 2.,
            z_far: 30000.,
            ..Default::default()
        });
        crate::lighting::select(vec![], eye, &scene.world);
        crate::render_fx::begin(eye, vec3(0., 500., 250.), 1., &scene.atmosphere, false);
        scene.draw(eye, 1., false, false, &[]);
        scene.draw(eye, 1., false, true, &[]);
        crate::render_fx::finish();
        let pixels = get_screen_data().bytes;
        gl_use_default_material();
        set_default_camera();
        pixels
    }
    // Keep another scene alive, as normal map/save replacement does. Re-render it
    // after every new scene to check the shared pipeline's lightmap rebinding too.
    let mut resident = crate::render::Scene::load(assets, "skool1")?;
    let reference = resident_pixels(&mut resident);
    next_frame().await;
    let names = assets.maps();
    let mut fallbacks = Vec::new();
    for name in &names {
        let mut scene = crate::render::Scene::load(assets, name)?;
        let mut particles = crate::particles::Steam::load(assets, &scene.map)?;
        let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
        interactions.set_entry(assets, &scene.map, name, None)?;
        interactions.sync(&mut scene.world);
        let transforms = interactions.transforms();
        let (eye, yaw) = crate::interaction::spawn(&scene.map, None);
        let direction = vec3(yaw.cos(), yaw.sin(), 0.2).normalize();
        particles.sync(&interactions.event_world);
        particles.update(1. / 30., eye, &scene.world);
        clear_background(scene.atmosphere.background());
        set_camera(&Camera3D {
            position: eye,
            target: eye + direction,
            up: Vec3::Z,
            fovy: 75_f32.to_radians(),
            z_near: 2.,
            z_far: 30000.,
            ..Default::default()
        });
        crate::lighting::select(particles.lights(), eye, &scene.world);
        crate::render_fx::begin(eye, direction, 1., &scene.atmosphere, false);
        scene.draw(eye, 1., false, false, &transforms);
        scene.draw_with_particles(eye, direction, 1., false, &transforms, &particles);
        let (_, dropped) = crate::render_fx::finish();
        anyhow::ensure!(dropped == 0, "Transparency queue budget exceeded in {name}");
        gl_use_default_material();
        set_default_camera();
        next_frame().await;
        anyhow::ensure!(
            resident_pixels(&mut resident) == reference,
            "Shared renderer changed the resident scene after drawing {name}"
        );
        next_frame().await;
        if !scene.missing.is_empty() {
            fallbacks.push((name.clone(), scene.missing.clone()));
        }
        println!("PASS corpus {name}");
    }
    println!(
        "PASS native load/draw for {} maps; fallback image references {fallbacks:?}",
        names.len()
    );
    println!("PASS simultaneous scenes reuse GPU pipelines and preserve exact resident lightmap/sky pixels across all map changes");
    Ok(())
}

/// Script-owned sky-camera orientation, independent of the player's position.
pub fn rotation(name: &str) -> Quat {
    if matches!(name, "fortress1" | "fortress2") {
        Quat::from_rotation_z(110_f32.to_radians()) * Quat::from_rotation_y(35_f32.to_radians())
    } else {
        Quat::IDENTITY
    }
}
