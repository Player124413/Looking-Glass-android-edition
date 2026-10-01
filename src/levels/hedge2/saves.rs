//! Separate-process native Store fixtures; these are staged persistence tests.
use super::*;
use crate::save::{Campaign, Game, Slot, Store, View};
pub(super) async fn run(a: &mut Assets, write: bool) -> Result<()> {
    let fingerprint = a.fingerprint()?;
    let mut alice = crate::character::Character::load(a)?;
    for phase in 0..6 {
        let dir = std::path::PathBuf::from(format!("private/hedge2-work/saves/{phase}"));
        let store = Store::new(dir.clone(), fingerprint.clone());
        if write {
            let mut r = crate::route::Route::new(a, "hedge2", Some("hedge2_start1"))?;
            r.enable_native_cast(a)?;
            let k = usize::from(phase >= 4);
            r.player = Player::new(check::owner(&mut r).levers[k].translation);
            let o = check::owner(&mut r);
            if phase == 0 {
                o.saved.used[0] = true;
                o.saved.pull = Some(Pull {
                    index: 0,
                    time: 1.5,
                    pose: o.levers[0],
                });
            } else {
                o.fire(0);
                if phase == 1 {
                    o.saved.scene.as_mut().unwrap().time = 7.;
                } else if phase == 2 {
                    o.saved.scene.as_mut().unwrap().time = 9.;
                    o.saved.motion[3] = 0.5;
                } else {
                    o.finish(0);
                    o.saved.release = Some(0.1);
                    o.saved.motion = [0., 1., 1., 1., 0., 0.];
                }
            }
            if phase >= 4 {
                o.fire(1);
                o.saved.scene.as_mut().unwrap().time = 1.2;
                o.saved.motion[4] = 0.5;
                o.saved.motion[5] = 0.5;
            }
            if phase == 5 {
                o.finish(1);
                o.saved.motion[4] = 1.;
                o.saved.motion[5] = 1.;
            }
            r.interactions.levels[0]
                .ctl
                .downcast_mut::<Maze>()
                .unwrap()
                .rebuild(&r.map)?;
            r.interactions.prepare_player(&mut r.stats, &mut r.player);
            r.interactions.sync(&mut r.world);
            r.native_cast
                .as_mut()
                .unwrap()
                .activate_levels(&r.interactions.levels);
            let cp = r.checkpoint();
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
        ensure!(
            serde_json::to_value(restored.interactions.snapshot())?
                == serde_json::to_value(&game.level()?.interactions)?,
            "Mystifying Madness phase {phase} changed on restore"
        );
        ensure!(
            serde_json::to_value(restored.story.snapshot())?
                == serde_json::to_value(&game.level()?.story)?,
            "Mystifying Madness phase {phase} replayed speech"
        );
        ensure!(
            restored.game.player.feet == game.player.feet,
            "Mystifying Madness phase {phase} moved Alice on restore"
        );
        let o = restored
            .interactions
            .levels
            .iter_mut()
            .find_map(|s| s.ctl.downcast_mut::<Maze>())
            .context("Missing saved maze")?;
        for _ in 0..120 * 20 {
            o.advance(
                1. / 120.,
                &restored.scene.map,
                &mut restored.scene.world,
                &mut restored.game.player,
                &[],
            )?;
        }
        let future = o.snapshot();
        let future_path = dir.join("future.json");
        if write {
            std::fs::write(&future_path, serde_json::to_vec(&future)?)?;
        } else {
            let expected: serde_json::Value = serde_json::from_slice(&std::fs::read(future_path)?)?;
            ensure!(
                future == expected,
                "Saved gate future diverged in fresh reader"
            );
        }
        println!(
            "PASS Mystifying Madness Store {} phase {phase}",
            if write { "write" } else { "fresh process read" }
        );
        next_frame().await;
    }
    Ok(())
}
