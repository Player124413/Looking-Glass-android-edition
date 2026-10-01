//! Native Store fixtures, deliberately separate from the input-driven victory route.
use super::*;
use crate::save::{Campaign, Game, Slot, Store, View};
pub(super) async fn run(a: &mut Assets, kind: Kind, write: bool) -> Result<()> {
    let fingerprint = a.fingerprint()?;
    let mut alice = crate::character::Character::load(a)?;
    for (name, phase, time) in [
        ("intro", Phase::Intro, 24.),
        ("battle", Phase::Fight, 32.),
        ("ground", Phase::Fight, 32.),
        ("death", Phase::Death, 1.),
        ("outro", Phase::Outro, 4.),
        (
            "reward",
            if kind == Kind::Lair {
                Phase::Reward
            } else {
                Phase::Bridge
            },
            7.,
        ),
        ("done", Phase::Done, 0.),
    ] {
        if kind == Kind::Lair && matches!(name, "ground" | "death") {
            continue;
        }
        let dir =
            std::path::PathBuf::from(format!("private/jabberwock/saves/{}-{name}", kind.map()));
        let store = Store::new(dir.clone(), fingerprint.clone());
        if write {
            let mut r = crate::route::Route::new(a, kind.map(), None)?;
            let o = r
                .interactions
                .levels
                .iter_mut()
                .find_map(|s| s.ctl.downcast_mut::<Encounter>())
                .unwrap();
            o.saved.phase = phase;
            o.saved.time = time;
            if kind == Kind::Lair && phase != Phase::Intro {
                o.saved.ticks = if phase == Phase::Fight { 3840 } else { 10800 };
                o.wave();
                if phase != Phase::Fight {
                    o.wave();
                    o.saved.waves.clear();
                }
            }
            if kind == Kind::Grounds && phase != Phase::Intro {
                if phase == Phase::Fight && name != "ground" {
                    o.saved.boss.at.z += 320.;
                    o.saved.boss.flying = true;
                    o.saved.boss.set(battle::Action::FlyFire);
                    o.saved.boss.time = 0.6;
                } else if phase == Phase::Fight {
                    o.saved.boss.health = 800.;
                    o.saved.boss.landed = true;
                    o.saved.boss.set(battle::Action::Fire);
                } else {
                    o.saved.boss.health = 0.;
                    o.saved.boss.set(battle::Action::Dead);
                }
            }
            if phase == Phase::Fight {
                let at = o.saved.boss.at + Vec3::Z * 100.;
                o.saved.boss.shots.push(battle::Shot {
                    at,
                    start: at,
                    velocity: Vec3::X * 400.,
                    age: 0.25,
                    ended: None,
                    spiral: kind == Kind::Grounds,
                    bounced: false,
                    id: 700,
                });
                o.saved.boss.serial = 700;
            }
            if kind == Kind::Lair && phase == Phase::Done {
                o.saved.eye = true;
                o.saved.exit.committed = true;
                r.stats.apply(crate::inventory::PickupKind::Weapon(7), 100.);
            }
            o.saved.checkpoint = false;
            o.rebuild(&r.map)?;
            let position = if phase == Phase::Intro {
                crate::interaction::spawn(&r.map, None).0
            } else {
                o.recovery_entry((Vec3::ZERO, 0.)).0
            };
            r.interactions.sync(&mut r.world);
            r.player =
                Player::spawn(&r.world, position).context("Save fixture position blocked")?;
            if phase == Phase::Intro {
                r.story.trigger(kind.intro());
                r.story.tick(0.1, false);
            }
            let mut cp = r.checkpoint();
            cp.level.npcs =
                crate::npc::Npcs::load(a, &r.map, kind.map(), None, false, false)?.snapshot();
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
                "Save reader must be a fresh process"
            );
        }
        let game = store.read(Slot::One)?.game;
        let restored = crate::save::Restored::build(a, game.clone())?;
        ensure!(
            serde_json::to_value(restored.interactions.snapshot())?
                == serde_json::to_value(&game.level()?.interactions)?,
            "{name} controller changed on restore"
        );
        ensure!(
            serde_json::to_value(restored.story.snapshot())?
                == serde_json::to_value(&game.level()?.story)?,
            "{name} story changed on restore"
        );
        ensure!(
            serde_json::to_value(restored.alice.snapshot())?
                == serde_json::to_value(&game.character)?,
            "{name} character changed on restore"
        );
        ensure!(
            restored.game.player.feet == game.player.feet,
            "{name} moved Alice"
        );
        let o = restored
            .interactions
            .levels
            .iter()
            .find_map(|s| s.ctl.downcast_ref::<Encounter>())
            .context("Missing restored encounter")?;
        ensure!(o.saved.phase == phase, "{name} lost phase");
        let _art = art::Art::load(a, o)?;
        println!(
            "PASS {} {} {name} native save: scene, inventory, actor, projectiles, waves and world",
            if write { "write" } else { "fresh-process read" },
            kind.map()
        );
        next_frame().await;
    }
    Ok(())
}
