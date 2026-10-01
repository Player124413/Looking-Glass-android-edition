//! Staged visual fixtures; continuous movement proof lives in the route checks.
use crate::{assets::Assets, interaction::Interactions, movement::Player, render::Scene};
use anyhow::Result;
use macroquad::prelude::*;
pub async fn check(assets: &mut Assets) -> Result<()> {
    let mut scene = Scene::load(assets, "gvillage")?;
    let mut i = Interactions::load(&scene.map)?;
    i.set_entry(assets, &scene.map, "gvillage", None)?;
    let mut p = Player::new(vec3(-4168., 6160., 18.03125));
    for (name, eye, target) in [
        (
            "village-bridge",
            vec3(-5050., 5350., -62.),
            vec3(-5120., 5060., -100.),
        ),
        (
            "village-machinery",
            vec3(-4830., 4920., 180.),
            vec3(-5040., 4660., 120.),
        ),
        (
            "village-sawmill",
            vec3(-3220., 3950., 0.),
            vec3(-3100., 3730., 40.),
        ),
    ] {
        for frame in 0..180 {
            i.advance_school(1. / 60., &scene.map, &mut scene.world, &mut p)?;
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
            scene.draw(eye, frame as f32 / 60., false, false, &transforms);
            crate::render::depth_read_only(|| {
                scene.draw(eye, frame as f32 / 60., false, true, &transforms)
            });
            set_default_camera();
            draw_text("STAGED VISUAL CHECK", 24., 35., 20., WHITE);
            if frame == 30 || frame == 179 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/{name}-{frame}.png"
                )))?;
            }
            next_frame().await;
        }
    }
    let mut scene = Scene::load(assets, "skool1")?;
    let mut i = Interactions::load(&scene.map)?;
    i.set_entry(assets, &scene.map, "skool1", None)?;
    let specs = crate::texture::read_materials(assets)?;
    let mut art = crate::school::Art::load(assets, &specs)?;
    let mut enemies = crate::encounters::Art::load(assets)?;
    let mut p = Player::new(vec3(-2554., 2624., -479.96875));
    for (name, eye, target, open) in [
        (
            "school-secret-switch",
            vec3(-2592., 2624., -345.),
            vec3(-2476., 2624., -352.),
            false,
        ),
        (
            "school-secret-closed",
            vec3(624., 2820., -170.),
            vec3(624., 2581., -195.),
            false,
        ),
        (
            "school-secret-open",
            vec3(624., 2820., -170.),
            vec3(624., 2581., -195.),
            true,
        ),
        (
            "school-diamond",
            vec3(-2700., 2490., -380.),
            vec3(-2592., 2580., -375.),
            true,
        ),
    ] {
        if open {
            i.school.as_mut().unwrap().event("Open_Bookcase_Goodie");
        }
        i.completed_dialogue("Theatre_Cinematic");
        i.activate_enemies();
        for frame in 0..120 {
            i.advance_school(1. / 60., &scene.map, &mut scene.world, &mut p)?;
            i.encounters
                .as_mut()
                .unwrap()
                .update(1. / 60., &scene.world, eye);
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
            scene.draw(eye, frame as f32 / 60., false, false, &transforms);
            art.draw(i.school.as_ref().unwrap(), false, &scene.atmosphere, eye);
            enemies.draw(
                i.encounters.as_ref().unwrap(),
                false,
                &scene.atmosphere,
                eye,
            );
            crate::render::depth_read_only(|| {
                scene.draw(eye, frame as f32 / 60., false, true, &transforms)
            });
            set_default_camera();
            draw_text("STAGED VISUAL CHECK", 24., 35., 20., WHITE);
            if frame == 60 {
                crate::viewer::save_capture(std::path::Path::new(&format!("private/{name}.png")))?;
            }
            next_frame().await;
        }
    }
    let mut alice = crate::character::Character::load(assets)?;
    let p = Player::new(vec3(-1888., 1840., -504.));
    alice.reset(&p, 0.);
    let eye = p.feet + vec3(-100., -75., 70.);
    for ghost in [false, true] {
        for frame in 0..60 {
            alice.update(
                1. / 60.,
                &p,
                false,
                crate::character::WeaponInput {
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
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: eye,
                target: p.feet + Vec3::Z * 36.,
                up: Vec3::Z,
                fovy: 60_f32.to_radians(),
                z_near: 2.,
                z_far: 30000.,
                ..Default::default()
            });
            scene.draw(eye, 0., false, false, &i.transforms());
            alice.atmosphere(&scene.atmosphere, eye);
            alice.draw_ghost(p.feet, false, ghost);
            set_default_camera();
            draw_text("STAGED VISUAL CHECK", 24., 35., 20., WHITE);
            if frame == 59 {
                crate::viewer::save_capture(std::path::Path::new(if ghost {
                    "private/school-looking-glass-active.png"
                } else {
                    "private/school-looking-glass-inactive.png"
                }))?;
            }
            next_frame().await;
        }
    }
    println!("PASS progression visual fixtures; inspect captures separately from route proof");
    Ok(())
}
