//! Regression captures at Skool1 gameplay positions, with the normal follow camera.
use super::*;
use crate::{character::Character, inventory::Stats, movement::Player};

pub(super) async fn render(assets: &mut Assets) -> Result<()> {
    let root = std::path::Path::new("private/qlair-audit/skool-weapons");
    std::fs::create_dir_all(root)?;
    let mut scene = crate::render::Scene::load(assets, "skool1")?;
    let world = World::from_bsp(&scene.map)?;
    let mut alice = Character::load(assets)?;
    // First room from the live mouse reproduction, then the user's school corridor.
    for (site, feet, yaw, pitch) in [
        (
            "room",
            vec3(-1888., 1752., -511.96875),
            0.1634264_f32,
            -0.353891_f32,
        ),
        (
            "floor",
            vec3(-1888., 1752., -511.96875),
            0.1634264,
            -0.803891,
        ),
        (
            "corridor",
            vec3(230.4024, 4156.2974, 64.03125),
            4.133426,
            -0.2425,
        ),
    ] {
        let mut player = Player::new(feet);
        for _ in 0..120 {
            player.tick(&world, crate::movement::Controls::default());
        }
        ensure!(player.grounded, "Skool1 regression did not land: {site}");
        let aim = vec3(
            yaw.cos() * pitch.cos(),
            yaw.sin() * pitch.cos(),
            pitch.sin(),
        );
        for first in [false, true] {
            for weapon in [4, 7] {
                alice.reset(&player, yaw);
                let mut stats = Stats::default();
                // Same inventory-only grant as the console's wuss command.
                stats.grant_weapons();
                stats.select(weapon);
                let mut buttons = crate::weapon_rules::Buttons::default();
                for frame in 0..300 {
                    let down = [weapon == 7 && frame >= 90, weapon == 4 && frame == 90];
                    alice.update_funded(
                        1. / 60.,
                        &player,
                        false,
                        WeaponInput {
                            selected: weapon,
                            first_person: first,
                            aim,
                            click: buttons.sample(down, true),
                            ..Default::default()
                        },
                        &CombatContext {
                            world: &world,
                            targets: &[],
                        },
                        Some(&mut stats),
                    );
                }
                let ps: ProjectileSave = serde_json::from_value(
                    serde_json::to_value(alice.snapshot())?["projectiles"].clone(),
                )?;
                if weapon == 4 {
                    ensure!(
                        ps.ice.walls.len() == 1 && stats.will() == 90.,
                        "Skool1 {site} first={first}: {} walls, {} Will",
                        ps.ice.walls.len(),
                        stats.will()
                    );
                    let wall = ps.ice.walls[0].target();
                    ensure!(
                        !world.sweep(wall.center, wall.center, wall.half).start_solid,
                        "Skool1 wall overlaps geometry"
                    );
                } else {
                    let charge = ps.heavy.charge.as_ref().context("Staff lost held charge")?;
                    ensure!(
                        charge.age > 3.4 && charge.tick > 20 && stats.will() < 100.,
                        "Staff never entered its funded beam state"
                    );
                    ensure!(
                        charge.pose.translation.distance(charge.end) > 20.,
                        "Collapsed Staff beam"
                    );
                }
                let (camera, target, show) = if first {
                    (player.eye(), player.eye() + aim, false)
                } else {
                    crate::camera::Follow::default().update(
                        &world,
                        player.feet,
                        aim,
                        128.,
                        1. / 60.,
                    )
                };
                for _ in 0..3 {
                    clear_background(BLACK);
                    let view = Camera3D {
                        position: camera,
                        target,
                        up: Vec3::Z,
                        z_near: 2.,
                        z_far: 30000.,
                        fovy: 75_f32.to_radians(),
                        ..Default::default()
                    };
                    set_camera(&view);
                    crate::lighting::select(alice.lights(), camera, &world);
                    crate::render_fx::begin_view(&view, 5., &scene.atmosphere, false);
                    scene.draw(camera, 5., false, false, &[]);
                    alice.atmosphere(&scene.atmosphere, camera);
                    if show {
                        alice.draw(player.feet, false);
                    }
                    alice.draw_effects(camera, false, show);
                    crate::render_fx::finish();
                    if first {
                        crate::render::clear_view_depth();
                        alice.draw_first_person(false);
                    }
                    set_default_camera();
                    crate::viewer::save_capture(&root.join(format!(
                        "{site}-{weapon}-{}.png",
                        if first { "first" } else { "third" }
                    )))?;
                    next_frame().await;
                }
                println!(
                    "PASS Skool1 wuss {site}: weapon {weapon}, first={first}, {} Will",
                    stats.will()
                );
            }
        }
    }
    Ok(())
}
