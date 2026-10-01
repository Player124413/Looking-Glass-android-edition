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
    for label in ["lily1", "leaf1", "leaf2", "house"] {
        let cp: Checkpoint =
            serde_json::from_slice(&std::fs::read(format!("private/potears2/{label}.json"))?)?;
        let dir = std::path::PathBuf::from(format!("private/potears2/saves/{label}"));
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
            "PASS Hollow Store {} {label}, exact state and 240 continuation ticks",
            if write { "write" } else { "fresh read" }
        );
        next_frame().await;
    }
    Ok(())
}
