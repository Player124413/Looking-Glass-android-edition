//! Staged persistence fixtures; combat completion is proved separately by the route.
use super::*;
use crate::save::{Campaign, Game, Slot, Store, View};
const ROOT: &str = "private/qlair-work/save-fixtures";
const CASES: &[(&str, Phase, f32)] = &[
    ("intro", Phase::Intro, 8.),
    ("birth", Phase::Birth, 25.),
    ("queen2", Phase::Queen2, 7.),
    ("death", Phase::Death, 1.),
    ("ending", Phase::Ending, 9.8),
];
pub(super) fn write(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move { run(a, true).await })
}
pub(super) fn read(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move { run(a, false).await })
}
async fn run(a: &mut Assets, write: bool) -> Result<()> {
    let fingerprint = a.fingerprint()?;
    let mut alice = crate::character::Character::load(a)?;
    let mut checkpoint = None;
    for &(name, phase, time) in CASES {
        let dir = std::path::Path::new(ROOT).join(name);
        let store = Store::new(dir.clone(), fingerprint.clone());
        if write {
            let mut r = crate::route::Route::new(a, "qlair", None)?;
            let q = r
                .interactions
                .levels
                .iter_mut()
                .find_map(|s| s.ctl.downcast_mut::<Queen>())
                .unwrap();
            q.start(phase);
            q.saved.time = time;
            q.saved.initialized = true;
            if phase != Phase::Intro {
                q.saved.queen1 = 0.;
                q.saved.collapsed = 20.;
            }
            let second = matches!(phase, Phase::Queen2 | Phase::Death | Phase::Ending);
            if second {
                q.saved.powered = true;
                q.saved.refilled = true;
                q.saved.checkpoint = true;
                q.saved.checkpoint_saved = true;
                q.saved.body.translation.z = 40.;
            }
            if matches!(phase, Phase::Death | Phase::Ending) {
                q.saved.queen2 = 900.;
            }
            if phase == Phase::Ending {
                q.final_burst();
                q.advance_effects(0.25, &r.world);
            }
            if phase == Phase::Queen2 {
                q.saved.body.translation.z = -900.;
                q.saved.motion = 2;
                q.saved.motion_time = 1.;
                q.saved.attack = battle::Attack::Hatter;
                q.saved.attack_time = 1.3;
                q.saved.part_attack[1] = Some(battle::Attack::Hatter);
                q.saved.part_time[1] = 1.3;
                q.saved.parts[0] = 0.;
                q.saved.part_death[0] = 0.5;
            }
            let pose = q.data.points[if second {
                "alice_fight_queen"
            } else {
                "alice_start_pos1"
            }];
            q.rebuild(&r.map)?;
            r.interactions.sync(&mut r.world);
            r.player = Player::spawn(&r.world, pose.translation).context("Saved scene endpoint")?;
            r.player.script_facing = if second {
                45f32.to_radians()
            } else {
                90f32.to_radians()
            };
            r.interactions
                .levels
                .iter_mut()
                .for_each(|s| s.ctl.prepare_player(&mut r.stats, &mut r.player));
            if phase == Phase::Intro {
                r.story.trigger(INTRO);
                r.story.tick(0.01, false);
            }
            let mut cp = r.checkpoint();
            cp.level.npcs =
                crate::npc::Npcs::load(a, &r.map, "qlair", None, false, false)?.snapshot();
            alice.reset(&r.player, r.player.script_facing);
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
                    yaw: r.player.script_facing,
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
            if phase == Phase::Queen2 {
                checkpoint = Some(game.clone());
            }
            if phase == Phase::Ending {
                store.write_finale_checkpoint(checkpoint.as_ref().unwrap())?;
            }
            std::fs::write(dir.join("writer.pid"), std::process::id().to_string())?;
        } else {
            ensure!(
                std::fs::read_to_string(dir.join("writer.pid"))? != std::process::id().to_string(),
                "Reader must be a fresh process"
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
            "{name} dialogue changed on restore"
        );
        ensure!(
            serde_json::to_value(restored.alice.snapshot())?
                == serde_json::to_value(&game.character)?,
            "{name} actor changed on restore"
        );
        ensure!(
            restored.game.player.feet == game.player.feet,
            "{name} moved Alice on restore"
        );
        let q = restored
            .interactions
            .levels
            .iter()
            .find_map(|s| s.ctl.downcast_ref::<Queen>())
            .unwrap();
        ensure!(
            q.saved.phase == phase && !q.checkpoint_requested(),
            "{name} phase/checkpoint mismatch"
        );
        let _art = art::Art::load(a, q)?;
        println!(
            "PASS {} {name} native save, dialogue, actor, position and checkpoint",
            if write {
                "wrote"
            } else {
                "fresh-process restored"
            }
        );
        next_frame().await;
    }
    if write {
        let store = Store::new(std::path::Path::new(ROOT).join("postgame"), fingerprint);
        store.write_finale_checkpoint(checkpoint.as_ref().unwrap())?;
        store.mark_complete()?;
        let (_, latest) = store.latest().context("Post-game Continue absent")?;
        ensure!(
            latest.game.campaign.completed.contains("qlair$first"),
            "Completion not retained"
        );
        let restored = crate::save::Restored::build(a, latest.game)?;
        let q = restored
            .interactions
            .levels
            .iter()
            .find_map(|s| s.ctl.downcast_ref::<Queen>())
            .unwrap();
        ensure!(
            q.saved.phase == Phase::Queen2 && !q.ending_ready(),
            "Continue repeats ending"
        );
        println!("PASS completed campaign Continue restores the pre-ending Queen2 checkpoint");
    }
    Ok(())
}
