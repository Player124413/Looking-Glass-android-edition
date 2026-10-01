//! Separate-process native Store fixtures; these are staged persistence tests.
use super::*;
use crate::save::{Campaign, Game, Slot, Store, View};
pub(super) async fn run(a: &mut Assets, write: bool) -> Result<()> {
    let fingerprint = a.fingerprint()?;
    let mut alice = crate::character::Character::load(a)?;
    for phase in 0..9 {
        let dir = std::path::PathBuf::from(format!("private/funhouse-work/saves/{phase}"));
        let store = Store::new(dir.clone(), fingerprint.clone());
        if write {
            let mut r = crate::route::Route::new(a, "funhouse", None)?;
            let o = r
                .interactions
                .levels
                .iter_mut()
                .find_map(|s| s.ctl.downcast_mut::<Funhouse>())
                .unwrap();
            o.saved.initialized = true;
            o.saved.time = 100.;
            o.saved.scene = None;
            if phase == 0 {
                o.begin(Kind::Arrival);
                o.saved.scene.as_mut().unwrap().clock.time = 16.;
            } else {
                o.saved.arrived = true;
                o.cell(1);
                o.saved.stand = Some(0.);
            }
            if phase == 1 {
                o.saved.tube = Some(96.);
            }
            if phase >= 2 {
                o.saved.cells = 127;
            }
            if phase >= 3 {
                o.saved.cells = 255;
                o.saved.all_cells = Some(90.);
                o.saved.gas_start = Some(99.);
            }
            if phase == 3 {
                o.begin(Kind::Gas);
                o.saved.scene.as_mut().unwrap().clock.time = 1.;
            }
            if phase >= 4 {
                o.saved.gas = true;
                o.saved.fight = true;
                o.saved.essence = true;
            }
            if phase >= 5 {
                o.hit(crate::combat::Hit {
                    id: BASE,
                    damage: 800.,
                    kind: crate::combat::DamageKind::Ice,
                    knockback: Vec3::ZERO,
                });
            }
            if phase >= 6 {
                o.hit(crate::combat::Hit {
                    id: BASE + 1,
                    damage: 900.,
                    kind: crate::combat::DamageKind::Knife,
                    knockback: Vec3::ZERO,
                });
                o.saved.death_wait = 5.;
                o.begin(Kind::Hatter);
                let s = o.saved.scene.as_mut().unwrap();
                s.clock.time = 8.;
                s.clock.line = 1;
                s.prior_line = 1;
                s.line_start = 4.;
                s.second_shot = Some(4.);
                s.clock.line_time = 0.8;
            }
            if phase == 7 {
                o.saved.scene = None;
                o.saved.floor = Some(96.);
                o.saved.exit.committed = true;
                o.saved.fight = false;
                o.saved.essence = false;
            }
            if let Some(s) = &mut o.saved.scene {
                s.clock.cast_time = s.clock.time;
                s.clock.shot_time = s.clock.time;
                s.clock.home = Some(Transform {
                    translation: r.player.feet,
                    rotation: Quat::IDENTITY,
                });
            }
            o.prepare_scene_story(&mut r.story);
            o.rebuild(&r.map)?;
            r.interactions.sync(&mut r.world);
            let mut cp = r.checkpoint();
            cp.level.npcs =
                crate::npc::Npcs::load(a, &r.map, "funhouse", None, false, false)?.snapshot();
            if phase == 8 {
                r.stats.damage(23.);
                // A version-12 visit saved before this controller existed, with the
                // arena trigger already reported by the old pending-thread path.
                let mut old = serde_json::to_value(
                    crate::interaction::Interactions::load(&r.map)?.snapshot(),
                )?;
                for t in old["triggers"].as_array_mut().unwrap() {
                    if t["id"] == 43 {
                        t["fired"] = true.into();
                        t["reported"] = true.into();
                    }
                }
                cp.level.interactions = serde_json::from_value(old)?;
            }
            alice.reset(&r.player, 0.);
            alice.power_appearance(&r.stats);
            let key = cp.level.key();
            let mut campaign = Campaign::default();
            campaign.levels.insert(key.clone(), cp.level);
            let game = Game {
                current: key,
                campaign,
                stats: r.stats,
                view: View {
                    position: r.player.eye(),
                    yaw: 0.,
                    pitch: 0.,
                    third_person: true,
                    flying: false,
                    fullbright: false,
                    spawn_landing: false,
                },
                player: r.player,
                recovery: Default::default(),
                character: alice.snapshot(),
            };
            store.write(Slot::One, &game)?;
            std::fs::write(dir.join("writer.pid"), std::process::id().to_string())?;
        } else {
            ensure!(
                std::fs::read_to_string(dir.join("writer.pid"))? != std::process::id().to_string(),
                "Reader must be a separate process"
            );
        }
        let game = store.read(Slot::One)?.game;
        let mut restored = crate::save::Restored::build(a, game.clone())?;
        if phase < 8 {
            ensure!(
                serde_json::to_value(restored.interactions.snapshot())?
                    == serde_json::to_value(&game.level()?.interactions)?,
                "Funhouse phase {phase} changed on restore"
            );
        }
        if phase < 8 {
            ensure!(
                serde_json::to_value(restored.story.snapshot())?
                    == serde_json::to_value(&game.level()?.story)?,
                "Funhouse phase {phase} replayed speech"
            );
        }
        if phase < 8 {
            ensure!(
                restored.game.player.feet == game.player.feet,
                "Funhouse phase {phase} moved Alice on restore"
            );
        } else {
            ensure!(
                serde_json::to_value(&restored.game.stats)? == serde_json::to_value(&game.stats)?,
                "Legacy upgrade changed inventory/resources"
            );
            let snapshot = serde_json::to_value(restored.interactions.snapshot())?;
            ensure!(
                snapshot["triggers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|t| t["id"] == 43)
                    .is_some_and(|t| t["fired"] == false),
                "Legacy consumed arena trigger was not rearmed"
            );
            ensure!(
                restored.game.player.feet.distance(vec3(32., 2928., 208.)) < 3.,
                "Legacy upgrade did not choose the authored entrance"
            );
        }
        let o = restored
            .interactions
            .levels
            .iter_mut()
            .find_map(|s| s.ctl.downcast_mut::<Funhouse>())
            .context("Missing saved Funhouse")?;
        for _ in 0..120 * 20 {
            o.advance(
                1. / 120.,
                &restored.scene.map,
                &mut restored.scene.world,
                &mut restored.game.player,
                &[],
            )?;
        }
        let future = serde_json::json!({"owner":o.snapshot(), "player":restored.game.player});
        let future_path = dir.join("future.json");
        if write {
            std::fs::write(future_path, serde_json::to_vec(&future)?)?;
        } else {
            let expected: serde_json::Value = serde_json::from_slice(&std::fs::read(future_path)?)?;
            ensure!(
                future == expected,
                "Funhouse phase {phase} world/scene future diverged"
            );
        }
        println!(
            "PASS Funhouse Store {} phase {phase}",
            if write { "write" } else { "fresh process read" }
        );
        next_frame().await;
    }
    Ok(())
}
