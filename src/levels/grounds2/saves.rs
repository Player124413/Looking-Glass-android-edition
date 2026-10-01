//! Native save files, reopened by a separate process at scene and reinforcement boundaries.
use super::*;
use crate::save::{Campaign, Game, Slot, Store, View};

pub(super) fn run(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let write = std::env::var_os("LOOKING_GLASS_GROUNDS2_WRITE").is_some();
        let fingerprint = a.fingerprint()?;
        let mut alice = crate::character::Character::load(a)?;
        for phase in 0..8 {
            let dir = std::path::PathBuf::from(format!("private/grounds2-work/saves/{phase}"));
            let store = Store::new(dir.clone(), fingerprint.clone());
            if write {
                let mut r = crate::route::Route::new(a, "grounds2", Some("grounds2_start1"))?;
                r.enable_native_cast(a)?;
                if phase < 3 {
                    for _ in 0..[0, 660, 2040][phase] {
                        r.tick(crate::movement::Controls::default())?;
                    }
                } else {
                    r.skip_cinematics = true;
                    r.wait_for_cinematic()?;
                    let o = check::owner(&mut r);
                    if phase < 7 {
                        o.event("Spawner07");
                    }
                    for n in 0..[0, 0, 0, 40, 1440, 1800, 6000, 0][phase] {
                        let o = r.interactions.levels[0]
                            .ctl
                            .downcast_mut::<Royale>()
                            .unwrap();
                        o.combat(&mut crate::level::Combat {
                            dt: 1. / 120.,
                            world: &r.world,
                            player: &mut r.player,
                            stats: &mut r.stats,
                            story: &mut r.story,
                            notarget: true,
                            summon: None,
                            threatens: &|_| false,
                        });
                        if phase >= 5 && n % 360 == 300 {
                            if let Some(t) = o.targets().first() {
                                o.hit(Hit {
                                    id: t.id,
                                    damage: 10000.,
                                    kind: crate::combat::DamageKind::Electric,
                                    knockback: Vec3::ZERO,
                                });
                            }
                        }
                    }
                }
                if phase == 7 {
                    let raw = data::pose(&r.map.entities[189]).translation;
                    r.player = Player::new(
                        r.world
                            .actor_footing(raw, PLAYER_CENTER, PLAYER_HALF, 180.)
                            .context("Unsupported Magma fixture")?,
                    );
                    for _ in 0..40 {
                        r.tick(crate::movement::Controls::default())?;
                    }
                    let cast = serde_json::to_value(r.native_cast.as_ref().unwrap().snapshot())?;
                    let launched = cast["actors"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|a| a["resident"]["body"]["Magma"]["launched"] == true)
                        .count();
                    ensure!(launched == 2, "Magma ambush did not launch both actors");
                }
                r.interactions.sync(&mut r.world);
                r.native_cast
                    .as_mut()
                    .unwrap()
                    .activate_levels(&r.interactions.levels);
                alice.reset(&r.player, 0.);
                alice.power_appearance(&r.stats);
                let cp = r.checkpoint();
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
                    std::fs::read_to_string(dir.join("writer.pid"))?
                        != std::process::id().to_string(),
                    "Save reader is not a fresh process"
                );
            }
            let game = store.read(Slot::One)?.game;
            let mut restored = crate::save::Restored::build(a, game.clone())?;
            ensure!(
                serde_json::to_value(restored.interactions.snapshot())?
                    == serde_json::to_value(&game.level()?.interactions)?
                    && restored.game.player.feet == game.player.feet,
                "Saved battlefield changed at phase {phase}"
            );
            ensure!(
                serde_json::to_value(restored.npcs.snapshot())?
                    == serde_json::to_value(&game.level()?.npcs)?,
                "Native enemies changed on restore"
            );
            let o = restored.interactions.levels[0]
                .ctl
                .downcast_mut::<Royale>()
                .context("Missing battlefield controller")?;
            restored.npcs.notarget(true);
            for _ in 0..120 * 24 {
                o.advance(
                    1. / 120.,
                    &restored.scene.map,
                    &mut restored.scene.world,
                    &mut restored.game.player,
                    &[],
                )?;
                o.combat(&mut crate::level::Combat {
                    dt: 1. / 120.,
                    world: &restored.scene.world,
                    player: &mut restored.game.player,
                    stats: &mut restored.game.stats,
                    story: &mut restored.story,
                    notarget: true,
                    summon: None,
                    threatens: &|_| false,
                });
                restored
                    .npcs
                    .update(1. / 120., &restored.scene.world, restored.game.player.eye());
            }
            let future =
                serde_json::json!({"controller":o.snapshot(),"cast":restored.npcs.snapshot()});
            let path = dir.join("future.json");
            if write {
                std::fs::write(path, serde_json::to_vec(&future)?)?;
            } else {
                let expected: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
                ensure!(
                    future == expected,
                    "Fresh-process future diverged at phase {phase}"
                );
            }
            println!(
                "PASS grounds2 Store {} phase {phase}",
                if write { "write" } else { "fresh read" }
            );
            next_frame().await;
        }
        Ok(())
    })
}
