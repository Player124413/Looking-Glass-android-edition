//! Separate-process native Store fixtures; these are staged persistence tests.
use super::*;
use crate::save::{Campaign, Game, Slot, Store, View};
pub(super) async fn run(a: &mut Assets, write: bool) -> Result<()> {
    let fingerprint = a.fingerprint()?;
    let mut alice = crate::character::Character::load(a)?;
    for phase in 0..6 {
        let dir = std::path::PathBuf::from(format!("private/jlair1-work/saves/{phase}"));
        let store = Store::new(dir.clone(), fingerprint.clone());
        if write {
            let mut r = crate::route::Route::new(a, "jlair1", None)?;
            let o = check::own(&mut r);
            o.saved.initialized = true;
            if phase == 0 {
                o.saved.scene.as_mut().unwrap().time = 3.;
            }
            if phase == 1 {
                let s = o.saved.scene.as_mut().unwrap();
                s.time = 10.;
                s.line_time = 0.8;
            }
            if phase >= 2 {
                o.saved.arrived = true;
                o.saved.scene = None;
                o.saved.sink = 64.;
            }
            if phase == 3 || phase == 4 {
                o.saved.scene = Some(Scene::new(Kind::Caterpillar));
                let s = o.saved.scene.as_mut().unwrap();
                s.time = 35.;
                s.line = if phase == 3 { 2 } else { 8 };
                s.line_time = 0.8;
                s.starts[s.line] = 34.2;
                if phase == 4 {
                    s.reveal = Some(20.);
                }
            }
            if phase == 5 {
                o.saved.caterpillar = true;
                o.saved.sink = 0.;
            }
            r.interactions.prepare_story(&mut r.story);
            r.interactions
                .levels
                .iter_mut()
                .find_map(|s| s.ctl.downcast_mut::<Curiosity>())
                .unwrap()
                .rebuild(&r.map)?;
            r.interactions.sync(&mut r.world);
            if phase == 2 {
                r.player = Player::new(
                    r.world
                        .actor_footing(vec3(-7048., 716., 1580.), PLAYER_CENTER, PLAYER_HALF, 30.)
                        .context("Saved sink lacks floor")?,
                );
                r.player.grounded = true;
            }
            let mut cp = r.checkpoint();
            cp.level.npcs =
                crate::npc::Npcs::load(a, &r.map, "jlair1", None, false, false)?.snapshot();
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
        let restored = crate::save::Restored::build(a, game.clone())?;
        ensure!(
            serde_json::to_value(restored.interactions.snapshot())?
                == serde_json::to_value(&game.level()?.interactions)?,
            "Burning Curiosity phase {phase} changed on restore"
        );
        ensure!(
            serde_json::to_value(restored.story.snapshot())?
                == serde_json::to_value(&game.level()?.story)?,
            "Burning Curiosity phase {phase} replayed speech"
        );
        ensure!(
            restored.game.player.feet == game.player.feet,
            "Burning Curiosity phase {phase} moved Alice on restore"
        );
        println!(
            "PASS Burning Curiosity Store {} phase {phase}",
            if write { "write" } else { "fresh process read" }
        );
        next_frame().await;
    }
    Ok(())
}
