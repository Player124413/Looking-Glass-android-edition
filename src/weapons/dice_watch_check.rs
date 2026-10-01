//! Full player action/effect saves, exercised in separate native processes.
use super::*;
use crate::{character::Character, inventory::Stats, movement::Player};

#[derive(serde::Serialize, serde::Deserialize)]
struct Resume {
    character: crate::character::Snapshot,
    stats: Stats,
    expected: serde_json::Value,
    will: f32,
}
fn tick(
    alice: &mut Character,
    stats: &mut Stats,
    player: &Player,
    world: &World,
    weapon: usize,
    click: Option<bool>,
    dt: f32,
) {
    let world_dt = stats.powers.world_dt(dt);
    alice.world_time(dt, world_dt);
    stats.update(dt);
    alice.power_appearance(stats);
    alice.update_funded(
        dt,
        player,
        false,
        WeaponInput {
            selected: weapon,
            click,
            aim: Vec3::X,
            dice: 3,
            ..Default::default()
        },
        &CombatContext {
            world,
            targets: &[combat::Target {
                id: 42,
                center: vec3(600., 0., 70.),
                half: vec3(30., 30., 60.),
            }],
        },
        Some(stats),
    );
    alice.take_hits();
}
pub async fn render(a: &mut Assets) -> Result<()> {
    let root = std::path::Path::new("private/dice-watch-check-v1");
    std::fs::create_dir_all(root)?;
    let world = World::fixture(&[
        (vec3(-4000., -4000., -30.), vec3(4000., 4000., 0.)),
        (vec3(1200., -4000., 0.), vec3(1210., 4000., 500.)),
    ]);
    let mut player = Player::new(Vec3::Z * 0.04);
    player.grounded = true;
    let mut alice = Character::load(a)?;
    alice.reset(&player, 0.);
    let initial = alice.snapshot();
    for weapon in 0..10 {
        for alternate in [false, true] {
            if alternate && [6, 8, 9].contains(&weapon) {
                continue;
            }
            let name = format!(
                "watch-toy-{weapon}-{}",
                if alternate { "alternate" } else { "primary" }
            );
            let path = root.join(format!("{name}.json"));
            let stats;
            if path.exists() {
                let r: Resume = serde_json::from_slice(&std::fs::read(&path)?)?;
                alice.restore(&r.character)?;
                let mut restored = r.stats.clone();
                for _ in 0..12 {
                    tick(
                        &mut alice,
                        &mut restored,
                        &player,
                        &world,
                        weapon,
                        None,
                        1. / 60.,
                    );
                }
                ensure!(
                    serde_json::to_value(alice.snapshot())? == r.expected,
                    "Fresh-process Watch state mismatch for {name}"
                );
                ensure!(
                    restored.will() == r.will,
                    "Fresh-process Watch Will mismatch for {name}"
                );
                alice.restore(&r.character)?;
                stats = r.stats;
                println!("PASS fresh-process {name}: exact action/effects/resources restoration");
            } else {
                alice.restore(&initial)?;
                let mut s = Stats::weapon_preview();
                s.difficulty = crate::powerups::Difficulty::Hard;
                s.select(weapon);
                // Equip first through the real action controller.
                for _ in 0..90 {
                    tick(&mut alice, &mut s, &player, &world, weapon, None, 1. / 60.);
                }
                if weapon != 9 {
                    s.watch().map_err(|e| anyhow::anyhow!(e))?;
                }
                let duration = if weapon == 7 {
                    3.0
                } else if weapon == 6 {
                    2.8
                } else if weapon == 9 {
                    1.5
                } else if weapon == 0 && alternate {
                    0.65
                } else {
                    0.9
                };
                let count = (duration * 60.) as usize;
                for frame in 0..count {
                    tick(
                        &mut alice,
                        &mut s,
                        &player,
                        &world,
                        weapon,
                        if frame == 0 || weapon == 7 && frame < count - 1 {
                            Some(alternate)
                        } else {
                            None
                        },
                        1. / 60.,
                    );
                }
                ensure!(
                    s.powers.stopped > 0. && s.powers.recharge > 330.,
                    "Watch not active in {name}"
                );
                let snapshot = alice.snapshot();
                let save: ProjectileSave = serde_json::from_value(
                    serde_json::to_value(&snapshot)?["projectiles"].clone(),
                )?;
                match (weapon, alternate) {
                    (0, true) => ensure!(
                        save.projectiles
                            .iter()
                            .any(|p| p.kind == projectile::Kind::Blade && p.age > 0.),
                        "Frozen thrown knife"
                    ),
                    (4, true) => ensure!(
                        !save.ice.walls.is_empty() && save.ice.walls.iter().all(|w| w.age == 0.),
                        "Ice wall advanced during Watch"
                    ),
                    (6, _) => ensure!(
                        save.dice.demon.as_ref().is_some_and(|d| d.time == 0.),
                        "New demon did not freeze"
                    ),
                    (7, _) => ensure!(
                        !save.heavy.shots.is_empty() || !save.heavy.volleys.is_empty(),
                        "Staff release failed during Watch"
                    ),
                    (8, _) => ensure!(
                        s.will() == 0. && !save.heavy.shots.is_empty(),
                        "Cannon Will or release changed during Watch"
                    ),
                    (9, _) => ensure!(s.will() == 99., "Watch cost changed"),
                    _ => (),
                }
                let before = s.clone();
                for _ in 0..12 {
                    tick(&mut alice, &mut s, &player, &world, weapon, None, 1. / 60.);
                }
                std::fs::write(
                    &path,
                    serde_json::to_vec_pretty(&Resume {
                        character: snapshot.clone(),
                        stats: before.clone(),
                        expected: serde_json::to_value(alice.snapshot())?,
                        will: s.will(),
                    })?,
                )?;
                alice.restore(&snapshot)?;
                stats = before;
                println!("PASS native {name}: attack/cost/time-stop policy; saved active state");
            }
            let camera = vec3(260., -460., 210.);
            for frame in 0..3 {
                clear_background(Color::new(0.10, 0.12, 0.16, 1.));
                let view = Camera3D {
                    position: camera,
                    target: vec3(160., 0., 60.),
                    up: Vec3::Z,
                    z_near: 1.,
                    z_far: 20000.,
                    fovy: 55_f32.to_radians(),
                    ..Default::default()
                };
                set_camera(&view);
                let atmosphere = crate::environment::Atmosphere::default();
                crate::render_fx::begin_view(&view, 1., &atmosphere, true);
                alice.atmosphere(&atmosphere, camera);
                draw_cube(
                    vec3(0., 0., -3.),
                    vec3(10000., 10000., 4.),
                    None,
                    Color::new(0.16, 0.19, 0.22, 1.),
                );
                alice.draw(player.feet, true);
                alice.draw_effects(camera, true, true);
                crate::render_fx::finish();
                set_default_camera();
                draw_text(
                    &format!(
                        "{name} / Will {} / Watch {:.2}s",
                        stats.will(),
                        stats.powers.stopped
                    ),
                    20.,
                    30.,
                    23.,
                    WHITE,
                );
                if frame == 2 {
                    crate::viewer::save_capture(&root.join(format!("{name}.png")))?;
                }
                next_frame().await;
            }
        }
    }
    Ok(())
}
