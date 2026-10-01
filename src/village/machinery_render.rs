//! Staged inspection of the reported village mechanisms, separate from route proof.
use crate::{assets::Assets, interaction::Interactions, movement::Player, render::Scene};
use anyhow::{ensure, Result};
use macroquad::prelude::*;

pub async fn check(assets: &mut Assets) -> Result<()> {
    for (name, eye, target, frames) in [
        (
            "sawmill",
            vec3(-3280., 4090., 0.),
            vec3(-3136., 3744., 10.),
            240,
        ),
        (
            "vents",
            vec3(-2820., 3220., 230.),
            vec3(-2605., 3064., 176.),
            480,
        ),
        (
            "roofs",
            vec3(-3210., 4110., 100.),
            vec3(-3020., 4270., 65.),
            330,
        ),
    ] {
        let mut scene = Scene::load(assets, "gvillage")?;
        let mut i = Interactions::load(&scene.map)?;
        i.set_entry(assets, &scene.map, "gvillage", None)?;
        i.sync(&mut scene.world);
        ensure!(
            scene
                .world
                .body_clear(eye - crate::collision::PLAYER_CENTER),
            "Village inspection camera {name} is inside geometry"
        );
        let mut p = Player::new(vec3(-4168., 6160., 18.03125));
        let mut steam = crate::particles::Steam::load(assets, &scene.map)?;
        for frame in 0..frames {
            i.advance_school(1. / 60., &scene.map, &mut scene.world, &mut p)?;
            steam.sync(&i.event_world);
            i.village.as_ref().unwrap().place_particles(&mut steam);
            steam.update(1. / 60., eye, &scene.world);
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: eye,
                target,
                up: Vec3::Z,
                fovy: 70_f32.to_radians(),
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
                &format!("STAGED VILLAGE MACHINERY / {name} / {time:.2}s"),
                24.,
                32.,
                22.,
                WHITE,
            );
            if [0, 29, 59, 98, 112, 127, 142, 202, 239, 329, 419, 479].contains(&frame) {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/village-machinery-{name}-{frame}.png"
                )))?;
            }
            next_frame().await;
        }
        println!("PASS village native machinery/steam inspection: {name}");
    }
    Ok(())
}
