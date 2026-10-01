//! Store round trips of moments reached by the ordinary route, never staged placements.
use crate::{
    assets::Assets,
    route::{Checkpoint, Route},
    save::{Game, Slot, Store, View},
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;

pub async fn run(a: &mut Assets, write: bool) -> Result<()> {
    let fingerprint = a.fingerprint()?;
    let mut alice = crate::character::Character::load(a)?;
    for label in ["prelude", "mid-slide", "handoff", "following", "holding"] {
        let cp: Checkpoint =
            serde_json::from_slice(&std::fs::read(format!("private/hedge1/{label}.json"))?)?;
        let dir = std::path::PathBuf::from(format!("private/hedge1/saves/{label}"));
        let store = Store::new(dir.clone(), fingerprint.clone());
        let mut reference = Route::resume(a, &cp)?;
        ensure!(
            reference.native_cast.is_some(),
            "Store evidence requires the full native route"
        );
        if write {
            alice.reset(&reference.player, 0.);
            alice.power_appearance(&reference.stats);
            let level = reference.level();
            let key = level.key();
            let mut campaign = reference.ledger.clone();
            campaign.levels.insert(key.clone(), level);
            store.write(
                Slot::One,
                &Game {
                    current: key,
                    campaign,
                    stats: reference.stats.clone(),
                    player: reference.player.clone(),
                    view: View {
                        position: reference.player.eye(),
                        yaw: 0.,
                        pitch: 0.,
                        third_person: true,
                        flying: false,
                        fullbright: false,
                        spawn_landing: false,
                    },
                    recovery: Default::default(),
                    character: alice.snapshot(),
                },
            )?;
            std::fs::write(dir.join("writer.pid"), std::process::id().to_string())?;
        } else {
            ensure!(
                std::fs::read_to_string(dir.join("writer.pid"))? != std::process::id().to_string(),
                "Store reader needs a fresh process"
            );
        }
        let game = store.read(Slot::One)?.game;
        if write && label == "following" {
            let mut legacy = game.clone();
            let level = legacy.campaign.levels.get_mut(&legacy.current).unwrap();
            let mut logic = serde_json::to_value(&level.interactions)?;
            logic["levels"].as_object_mut().unwrap().remove("hedge1");
            let mut old = crate::interaction::Interactions::load(&reference.map)?;
            old.set_entry(a, &reference.map, "hedge1", Some("hedge1_start1"))?;
            old.probe_levels(&reference.map, vec![])?;
            // Keep the reached trigger history but bind it to the actual pre-controller
            // event definitions. Dropping only the level key would be a corrupt save.
            logic["shared"]["signature"] =
                serde_json::to_value(old.snapshot())?["shared"]["signature"].clone();
            level.interactions = serde_json::from_value(logic)?;
            let mut cast = serde_json::to_value(&level.npcs)?;
            let mut precache = 0;
            for actor in cast["actors"].as_array_mut().unwrap() {
                let p: Vec3 = serde_json::from_value(actor["spawn"]["origin"].clone())?;
                if p.y > 6000. && p.z < 1200. {
                    actor["spawn"]["hidden"] = false.into();
                    precache += 1;
                }
                if let Some(guide) = actor["guide"].as_object_mut() {
                    guide.remove("trail");
                    guide.remove("path_retry");
                }
            }
            ensure!(precache == 7, "Precache identity count changed");
            level.npcs = serde_json::from_value(cast)?;
            let expected = serde_json::to_value(&level.npcs)?;
            // A formerly passable exit leaf is solid after this controller joins the save.
            legacy.player.feet = vec3(2912., 4288., 1920.1);
            let upgraded = crate::save::Restored::build(a, legacy)?;
            let actual = serde_json::to_value(upgraded.npcs.snapshot())?;
            let live = |v: &serde_json::Value| {
                v["actors"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|a| {
                        let p: Vec3 = serde_json::from_value(a["spawn"]["origin"].clone()).unwrap();
                        !(p.y > 6000. && p.z < 1200.)
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            };
            ensure!(
                live(&actual) == live(&expected) && actual["greetings"] == expected["greetings"],
                "Older save lost a compatible live actor state"
            );
            ensure!(
                actual["actors"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|a| {
                        let p: Vec3 = serde_json::from_value(a["spawn"]["origin"].clone()).unwrap();
                        p.y > 6000. && p.z < 1200. && a["spawn"]["hidden"] == true
                    })
                    .count()
                    == 7,
                "Precache actors were not deactivated"
            );
            ensure!(
                upgraded.game.player.feet.distance(cp.player.feet) > 100.
                    && upgraded
                        .game
                        .player
                        .feet
                        .distance(vec3(4806., -100., 2056.))
                        < 200.,
                "Old leaf-overlap save did not recover at the entrance"
            );
            ensure!(
                serde_json::to_value(&upgraded.game.stats)? == serde_json::to_value(&cp.stats)?,
                "Older save changed resources"
            );
            println!("PASS Majestic Maze older NPC-only save: live cast and resources retained, solid-leaf recovery");
        }
        let restored = crate::save::Restored::build(a, game.clone())?;
        ensure!(
            serde_json::to_value(&restored.game.player)? == serde_json::to_value(&cp.player)?
                && serde_json::to_value(&restored.game.stats)? == serde_json::to_value(&cp.stats)?
                && serde_json::to_value(restored.interactions.snapshot())?
                    == serde_json::to_value(&cp.level.interactions)?
                && serde_json::to_value(restored.npcs.snapshot())?
                    == serde_json::to_value(&cp.level.npcs)?,
            "{label}: Store changed the player, resources, movers or enemies"
        );
        let mut resumed_cp = cp.clone();
        resumed_cp.player = restored.game.player;
        resumed_cp.stats = restored.game.stats;
        resumed_cp.level = game.level()?.clone();
        let mut resumed = Route::resume(a, &resumed_cp)?;
        reference.quiet();
        resumed.quiet();
        ensure!(
            crate::route::run_identical(&mut reference, &mut resumed, 240)?.is_none(),
            "{label}: continuation failed"
        );
        println!(
            "PASS Majestic Maze Store {} {label}, exact state and 240 continuation ticks",
            if write { "write" } else { "fresh read" }
        );
        next_frame().await;
    }
    Ok(())
}
