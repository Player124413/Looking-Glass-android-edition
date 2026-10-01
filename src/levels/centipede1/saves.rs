//! Native save/read fixtures complement the ordinary-input route.
use super::*;
use crate::save::{Campaign, Game, Slot, Store, View};
const CASES: &[(&str, Phase, f64)] = &[
    ("forest", Phase::Explore, 0.),
    ("voice", Phase::Explore, 0.),
    ("rush", Phase::Rush, 1.),
    ("reveal", Phase::Reveal, 2.),
    ("outro", Phase::Outro, 0.2),
    ("countdown", Phase::Countdown, 1.2),
    ("waves", Phase::Countdown, 6.5),
    ("exit", Phase::Done, 0.),
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
    for &(name, phase, time) in CASES {
        let dir = std::path::Path::new("private/centipede1-work/saves").join(name);
        let store = Store::new(dir.clone(), fingerprint.clone());
        if write {
            let mut r = crate::route::Route::new(a, "centipede1", Some("centipede1_start1"))?;
            r.player = Player::spawn(&r.world, vec3(-280., 4700., -600.))
                .context("Flora save fixture obstructed")?;
            let at = r.player.feet;
            let w = &r.world;
            let o = r
                .interactions
                .levels
                .iter_mut()
                .find_map(|s| s.ctl.downcast_mut::<Flora>())
                .unwrap();
            o.saved.phase = phase;
            o.saved.time = time;
            o.saved.alice.translation = at;
            o.saved.talk = name == "voice";
            if matches!(phase, Phase::Countdown | Phase::Done) {
                for s in &mut o.saved.ants {
                    s.ant.script_wait = false;
                }
                for _ in 0..if name == "waves" { 3 } else { 1 } {
                    o.spawn_wave(
                        w,
                        if name == "waves" {
                            vec3(1136., -1712., -400.)
                        } else {
                            at + Vec3::Z * 32.
                        },
                    );
                }
            }
            o.saved.exit.committed = phase == Phase::Done;
            if name == "waves" {
                if let Some(s) = o.saved.ants.iter_mut().find(|s| s.group == 1) {
                    s.ant.hit(Hit {
                        id: BASE + s.ant.id,
                        damage: 1000.,
                        kind: crate::combat::DamageKind::Knife,
                        knockback: Vec3::ZERO,
                    });
                }
                if let Some(s) = o.saved.ants.iter_mut().find(|s| s.group == 2) {
                    s.ant.shots.push(crate::ant::Bullet {
                        at: s.ant.feet + Vec3::Z * 50.,
                        grenade: false,
                        direction: Vec3::X,
                        age: 0.1,
                    });
                }
            }
            r.interactions.prepare_player(&mut r.stats, &mut r.player);
            r.interactions.prepare_story(&mut r.story);
            r.story.tick(0.2, false);
            r.interactions.sync_cinematic_story(&r.story);
            let mut cp = r.checkpoint();
            cp.level.npcs =
                crate::npc::Npcs::load(a, &r.map, "centipede1", None, false, false)?.snapshot();
            alice.reset(&r.player, r.player.script_facing);
            alice.power_appearance(&r.stats);
            let key = cp.level.key();
            let mut campaign = Campaign::default();
            campaign.levels.insert(key.clone(), cp.level);
            store.write(
                Slot::One,
                &Game {
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
                },
            )?;
            std::fs::write(dir.join("writer.pid"), std::process::id().to_string())?;
        } else {
            ensure!(
                std::fs::read_to_string(dir.join("writer.pid"))? != std::process::id().to_string(),
                "Reader must be a fresh process"
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
            "{name}: speech changed on restore"
        );
        ensure!(
            restored.game.player.feet == game.player.feet,
            "{name}: player moved on restore"
        );
        restored
            .interactions
            .prepare_player(&mut restored.game.stats, &mut restored.game.player);
        ensure!(
            restored.game.stats.minimum_sanity
                == if matches!(phase, Phase::Countdown | Phase::Done) {
                    10.
                } else {
                    0.
                },
            "{name}: sanity floor not reconstructed"
        );
        let o = restored
            .interactions
            .levels
            .iter_mut()
            .find_map(|s| s.ctl.downcast_mut::<Flora>())
            .unwrap();
        let before = o.snapshot();
        o.advance(
            0.,
            &restored.scene.map,
            &mut restored.scene.world,
            &mut restored.game.player,
            &[],
        )?;
        ensure!(o.snapshot() == before, "{name}: paused state advanced");
        let _art = art::Art::load(a)?;
        println!(
            "PASS {} Flora {name}: native save, actors, speech, player and pause",
            if write {
                "wrote"
            } else {
                "restored fresh process"
            }
        );
        if name == "exit" {
            let mut legacy = game.clone();
            legacy.stats.damage(37.);
            legacy.stats.spend_will(17.);
            let carried = serde_json::to_value(&legacy.stats)?;
            let mut old = crate::interaction::Interactions::load(&restored.scene.map)?;
            let at = vec3(-280., 4792., -652.) - crate::collision::PLAYER_CENTER;
            old.triggers(crate::movement::FIXED_DT, at, at);
            let level = legacy.campaign.levels.get_mut(&legacy.current).unwrap();
            level.interactions = old.snapshot();
            level.story = Story::load(a, "centipede1").snapshot();
            let migrated = crate::save::Restored::build(a, legacy)?;
            let o = migrated
                .interactions
                .levels
                .iter()
                .find_map(|s| s.ctl.downcast_ref::<Flora>())
                .unwrap();
            ensure!(
                o.saved.phase == Phase::Explore && !o.saved.exit.committed,
                "Legacy save did not reset encounter"
            );
            ensure!(
                serde_json::to_value(&migrated.game.stats)? == carried,
                "Legacy migration changed carried inventory"
            );
            let entry = crate::interaction::spawn(&migrated.scene.map, Some("centipede1_start1")).0;
            ensure!(
                migrated
                    .game
                    .player
                    .feet
                    .distance(Player::spawn(&migrated.scene.world, entry).unwrap().feet)
                    < 0.1,
                "Legacy save stranded Alice"
            );
            println!("PASS old consumed ambush save migrated safely with resources preserved");
        }
        next_frame().await;
    }
    Ok(())
}
