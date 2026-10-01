//! Real Store/Restored fixtures; staged states complement the ordinary-input route.
use super::*;
use crate::save::{Campaign, Game, Slot, Store, View};
const ROOT: &str = "private/centipede2-work/saves";
const CASES: &[(&str, Phase, battle::Action, f32, u8)] = &[
    ("intro", Phase::Intro, battle::Action::Walk, 8., 0),
    ("weak", Phase::Fight, battle::Action::Crush, 0.4, 2),
    ("larvae", Phase::Fight, battle::Action::Larvae, 0.8, 2),
    ("grab", Phase::Fight, battle::Action::Shake, 0.1, 4),
    ("death", Phase::Fight, battle::Action::Death, 0.8, 6),
    ("spikes", Phase::Drop, battle::Action::Dead, 1.8, 6),
    ("climb", Phase::Climb, battle::Action::Dead, 0.6, 6),
    ("greeting", Phase::Greeting, battle::Action::Dead, 4., 6),
    ("eat", Phase::Eat, battle::Action::Dead, 0.8, 6),
    ("grow", Phase::Grow, battle::Action::Dead, 2., 6),
    ("exit", Phase::Done, battle::Action::Dead, 0., 6),
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
    for &(name, phase, action, time, hits) in CASES {
        let dir = std::path::Path::new(ROOT).join(name);
        let store = Store::new(dir.clone(), fingerprint.clone());
        if write {
            let mut r = crate::route::Route::new(a, "centipede2", Some("centipede2_start1"))?;
            let o = r
                .interactions
                .levels
                .iter_mut()
                .find_map(|s| s.ctl.downcast_mut::<Centipede>())
                .unwrap();
            o.saved.phase = phase;
            o.saved.time = time;
            o.saved.battle.set(action);
            o.saved.battle.time = time;
            o.saved.battle.hits = hits;
            o.saved.battle.stage = (hits / 2 + 1).min(3);
            o.saved.exit.committed = phase == Phase::Done;
            let at = o.data.points[if phase == Phase::Intro {
                "centipede2_start1"
            } else {
                "alice_pos3"
            }];
            if action == battle::Action::Shake {
                o.saved.battle.grabbed = true;
                o.saved.battle.grab_feet = at.translation;
                o.saved.battle.victim_feet = at.translation;
                o.saved.battle.grab_camera = at.translation + vec3(80., 280., 200.);
            }
            if name == "larvae" {
                let mut larva = crate::burrow::Insect::new(
                    crate::burrow::Kind::Larva,
                    o.saved.battle.tag(&o.data, "tag_tongue").translation,
                    0.,
                    1.,
                    42,
                );
                larva.launch(vec3(150., 0., 150.));
                o.saved.battle.larvae.push(larva);
            }
            o.rebuild(&r.map)?;
            r.interactions.sync(&mut r.world);
            r.player = Player::spawn(&r.world, at.translation)
                .context("Centipede save point obstructed")?;
            r.interactions.prepare_player(&mut r.stats, &mut r.player);
            if matches!(phase, Phase::Eat | Phase::Grow | Phase::Done) {
                r.story.finish_sequence(TALK1);
            }
            r.interactions.prepare_story(&mut r.story);
            if r.story.busy() {
                r.story.tick(0.2, false);
            }
            let mut cp = r.checkpoint();
            cp.level.npcs =
                crate::npc::Npcs::load(a, &r.map, "centipede2", None, false, false)?.snapshot();
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
            std::fs::write(dir.join("writer.pid"), std::process::id().to_string())?;
        } else {
            ensure!(
                std::fs::read_to_string(dir.join("writer.pid"))? != std::process::id().to_string(),
                "Save reader must be a fresh process"
            );
        }
        let game = store.read(Slot::One)?.game;
        let mut restored = crate::save::Restored::build(a, game.clone())?;
        ensure!(
            serde_json::to_value(restored.interactions.snapshot())?
                == serde_json::to_value(&game.level()?.interactions)?,
            "{name}: controller changed on restore"
        );
        ensure!(
            serde_json::to_value(restored.story.snapshot())?
                == serde_json::to_value(&game.level()?.story)?,
            "{name}: dialogue changed on restore"
        );
        ensure!(
            restored.game.player.feet == game.player.feet,
            "{name}: player moved on restore"
        );
        let o = restored
            .interactions
            .levels
            .iter_mut()
            .find_map(|s| s.ctl.downcast_mut::<Centipede>())
            .unwrap();
        ensure!(
            o.saved.phase == phase && o.saved.battle.hits == hits,
            "{name}: boss resurrected"
        );
        let saved = o.snapshot();
        o.advance(
            0.,
            &restored.scene.map,
            &mut restored.scene.world,
            &mut restored.game.player,
            &[],
        )?;
        ensure!(o.snapshot() == saved, "{name}: paused state advanced");
        let _art = art::Art::load(a, o)?;
        println!(
            "PASS {} Centipede {name}: native save, boss, dialogue, player and pause",
            if write {
                "wrote"
            } else {
                "restored fresh process"
            }
        );
        if name == "exit" {
            let mut legacy = game.clone();
            legacy.stats.damage(23.);
            legacy.stats.spend_will(17.);
            let carried = serde_json::to_value(&legacy.stats)?;
            let level = legacy.campaign.levels.get_mut(&legacy.current).unwrap();
            let mut old = crate::interaction::Interactions::load(&restored.scene.map)?;
            let at = vec3(-2684., 3904., 2880.) - crate::collision::PLAYER_CENTER;
            ensure!(
                old.triggers(crate::movement::FIXED_DT, at, at)
                    .transition
                    .is_some(),
                "Legacy fixture did not exercise the old bypass"
            );
            level.interactions = old.snapshot();
            level.story = Story::load(a, "centipede2").snapshot();
            let migrated = crate::save::Restored::build(a, legacy)?;
            let owner = migrated
                .interactions
                .levels
                .iter()
                .find_map(|s| s.ctl.downcast_ref::<Centipede>())
                .unwrap();
            ensure!(
                owner.saved.phase == Phase::Intro && owner.saved.battle.hits == 0,
                "Legacy bypass save did not restart the arena"
            );
            let (entry, _) =
                crate::interaction::spawn(&migrated.scene.map, Some("centipede2_start1"));
            let player =
                Player::spawn(&migrated.scene.world, entry).context("Legacy entrance blocked")?;
            ensure!(
                migrated.game.player.feet.distance(player.feet) < 0.1,
                "Legacy save stranded Alice behind the closed arena"
            );
            ensure!(
                serde_json::to_value(&migrated.game.stats)? == carried,
                "Legacy migration changed carried resources"
            );
            println!("PASS controller-less bypass save restarts safely with carried resources");
        }
        next_frame().await;
    }
    Ok(())
}
