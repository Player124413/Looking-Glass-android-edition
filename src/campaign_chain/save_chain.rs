//! Real saves of the chain: the window-free round trip that every `--campaign-route-check` runs,
//! and its windowed companions `--campaign-save-chain-write` and `--campaign-save-chain-read`
//! (F3 item 9), which run only in the Anode seat (`tools/test_campaign_chain.ps1
//! -InsideAnodeSeat`, recipe 13c) because they open a window.
//!
//! The headless chain leaves a checkpoint per visit under `private/campaign-chain/`. Both halves
//! turn each of them into real `save::Game` values with one function (`build_game`) and write them
//! through the save `Store`: an Auto slot holding the arrival autosave the viewer writes after
//! every normal exit (the entrance of the visit), and, when the visit's leg passed, a manual slot
//! holding the state one second before that leg's exit (the pre-exit save).
//!
//! * The chain itself round-trips both slots without a window (`verify_headless`): the file is
//!   written by the `Store`, read back and rebuilt by `save::rebuild_headless`, the part of the
//!   viewer's Continue that needs no window, and the rebuilt logic, resources and player must equal
//!   what was saved. Alice's block is a stand-in there: the smallest value her deserializer accepts.
//! * The windowed writer builds the same games with the real Alice and the real cast of every
//!   visit, and the reader, a separate process, restores each through `Restored::build`, the
//!   viewer's own loader, and requires the restored visit, the cached visits of the ledger and
//!   Alice's block to equal what was saved.
//!
//! The fortress1 and fortress2 cases are the native fixtures the plan lacked for those visits.
//! The cast of a visit is regenerated from its models (a headless route has none), so guards and
//! other actors start as the level places them; everything else is the chain's state.
use super::{file_path, read_visit, VisitFile};
use crate::{
    assets::Assets,
    bsp::Bsp,
    campaign,
    character::{Character, Snapshot as Alice},
    interaction,
    npc::Npcs,
    powerups::Difficulty,
    recovery::Recovery,
    route::Checkpoint,
    save::{self, Campaign, Game, Level, Restored, Slot, Store, View},
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::Serialize;
use std::{fs, path::Path};

const ROOT: &str = "private/campaign-save-chain";
/// Where the chain's window-free round trip keeps its files.
pub const SCRATCH: &str = "private/campaign-chain/saves";

fn same(a: &impl Serialize, b: &impl Serialize, label: &str) -> Result<()> {
    ensure!(
        serde_json::to_value(a)? == serde_json::to_value(b)?,
        "Restored {label} differs"
    );
    Ok(())
}

/// One real game at a moment of the chain: the ledger of finished visits, the visit being played
/// (`checkpoint.level`) as the current one, the resources and Alice's body. `landing` is the view
/// flag of an arrival.
pub fn build_game(
    checkpoint: &Checkpoint,
    ledger: &Campaign,
    yaw: f32,
    landing: bool,
    character: Alice,
) -> Game {
    let level = checkpoint.level.clone();
    let key = level.key();
    let mut campaign = ledger.clone();
    campaign.levels.insert(key.clone(), level);
    let player = checkpoint.player.clone();
    Game {
        current: key,
        campaign,
        stats: checkpoint.stats.clone(),
        view: View {
            position: player.eye(),
            yaw,
            pitch: 0.,
            third_person: true,
            flying: false,
            fullbright: false,
            spawn_landing: landing,
        },
        player,
        recovery: Recovery::default(),
        character,
    }
}

/// Alice's saved block as the smallest value her deserializer accepts. The window-free round trip
/// holds no character; the bounds `Game::validate` checks never look inside it.
fn stand_in() -> Result<Alice> {
    let anchor = serde_json::json!({ "rotation": [0., 0., 0., 1.], "translation": [0., 0., 0.] });
    Ok(serde_json::from_value(serde_json::json!({
        "animator": {
            "motion": "Idle", "time": 0., "transition": 1., "previous": [], "pose": [],
            "jumps": 0, "landings": 0, "large": false, "offset": 0., "previous_offset": 0.
        },
        "facing": 0.,
        "actions": { "playing": null, "selected": 0, "pose": [], "start": [], "variation": 0 },
        "projectiles": { "projectiles": [], "shots": 0, "impacts": 0 },
        "anchors": [anchor, anchor, anchor],
        "viewmodel": { "phase": 0., "bob": [0., 0., 0.] }
    }))?)
}

/// The yaw of a visit's entrance.
fn entrance_yaw(assets: &mut Assets, level: &Level) -> Result<f32> {
    let map = Bsp::parse(&assets.read(&format!("maps/{}.bsp", level.map))?)?;
    Ok(interaction::spawn(&map, level.entry.as_deref()).1)
}

/// The rebuilt logic of a game's current visit, as `Level` stores it.
fn logic(h: &save::Headless) -> serde_json::Value {
    serde_json::json!({
        "interactions": h.logic.interactions.snapshot(),
        "story": h.logic.story.snapshot(),
        "hints": h.logic.hints.snapshot(),
    })
}

/// Write one game through the save `Store`, read it back, rebuild it window-free and require the
/// rebuilt visit, resources and player to be what was written. Returns the size of the file.
pub fn verify_headless(
    assets: &mut Assets,
    checkpoint: &Checkpoint,
    ledger: &Campaign,
    slot: Slot,
    label: &str,
) -> Result<u64> {
    let fingerprint = assets.fingerprint()?;
    let yaw = entrance_yaw(assets, &checkpoint.level)?;
    let game = build_game(checkpoint, ledger, yaw, slot == Slot::Auto, stand_in()?);
    let dir = Path::new(SCRATCH).join(label);
    let _ = fs::remove_dir_all(&dir);
    let store = Store::new(dir.clone(), fingerprint);
    store.write(slot, &game)?;
    let bytes = fs::metadata(dir.join(format!("{}.json", slot.name())))?.len();
    let loaded = store.read(slot)?.game;
    ensure!(
        !store.read(slot)?.backup,
        "The {label} save came from its backup"
    );
    let rebuilt = save::rebuild_headless(assets, loaded)?;
    let saved = game.level()?;
    same(
        &logic(&rebuilt),
        &serde_json::json!({
            "interactions": saved.interactions,
            "story": saved.story,
            "hints": saved.hints,
        }),
        "level, enemies, puzzle and dialogue",
    )?;
    same(&rebuilt.game.stats, &game.stats, "resources and inventory")?;
    same(&rebuilt.game.player, &game.player, "player")?;
    ensure!(
        rebuilt.game.campaign.levels.len() == game.campaign.levels.len(),
        "A cached visit was lost"
    );
    Ok(bytes)
}

/// The level as the live objects stand: the restored logic, story, hints and cast, with the
/// visual clocks the save recorded.
fn level_of(r: &Restored) -> Result<Level> {
    let saved = r.game.level()?;
    Ok(Level {
        map: saved.map.clone(),
        entry: saved.entry.clone(),
        interactions: r.interactions.snapshot(),
        npcs: r.npcs.snapshot(),
        story: r.story.snapshot(),
        hints: r.hints.snapshot(),
        environment_clock: saved.environment_clock,
        pickup_clock: saved.pickup_clock,
    })
}

/// The cast the visit places, from its models. A headless route keeps no cast of its own.
fn cast(assets: &mut Assets, level: &mut Level, difficulty: Difficulty) -> Result<()> {
    let mut map = Bsp::parse(&assets.read(&format!("maps/{}.bsp", level.map))?)?;
    map.difficulty = difficulty;
    level.npcs = Npcs::load(
        assets,
        &map,
        &level.map,
        level.entry.as_deref(),
        false,
        false,
    )?
    .snapshot();
    Ok(())
}

/// One real game with the real Alice and the real cast of every visit in it.
fn native_game(
    assets: &mut Assets,
    alice: &mut Character,
    checkpoint: &Checkpoint,
    ledger: &Campaign,
    landing: bool,
) -> Result<Game> {
    let yaw = entrance_yaw(assets, &checkpoint.level)?;
    alice.reset(&checkpoint.player, yaw);
    alice.power_appearance(&checkpoint.stats);
    let mut game = build_game(checkpoint, ledger, yaw, landing, alice.snapshot());
    for level in game.campaign.levels.values_mut() {
        cast(assets, level, checkpoint.difficulty)?;
    }
    Ok(game)
}

/// The checkpoints the headless chain left, in visit order.
fn chain_files() -> Result<Vec<VisitFile>> {
    let mut files = Vec::new();
    for index in 0..campaign::route().len() {
        if file_path(index).is_file() {
            files.extend(read_visit(index)?);
        }
    }
    ensure!(
        !files.is_empty(),
        "No chain checkpoints in {}: run --campaign-route-check --campaign-strict first",
        super::DIR
    );
    Ok(files)
}

/// What a checkpoint came from, for the writer's log: a save fixture is only as good as its label,
/// and the directory may hold files that different runs left.
fn origin(file: &VisitFile) -> &'static str {
    if file.diagnosis {
        "from a --campaign-allow-retry diagnosis run, never proof"
    } else if file.fixture {
        "from a staged chapter loadout, a fixture"
    } else if file.strict {
        "from a strict chain"
    } else {
        "from a viewer-style chain"
    }
}

pub async fn check(assets: &mut Assets, write: bool) -> Result<()> {
    clear_background(BLACK);
    draw_text("Checking the campaign save chain...", 24., 40., 26., WHITE);
    next_frame().await;
    let root = Path::new(ROOT);
    let fingerprint = assets.fingerprint()?;
    if write {
        fs::create_dir_all(root)?;
        let mut alice = Character::load(assets).context("Loading Alice's character")?;
        let mut cases = 0;
        for file in chain_files()? {
            let name = super::file_name(file.index - 1).replace(".json", "");
            let dir = root.join(&name);
            let _ = fs::remove_dir_all(&dir);
            let store = Store::new(dir.clone(), fingerprint.clone());
            // The arrival autosave: the entrance of this visit, as the viewer writes it after a
            // normal exit.
            let arrival = native_game(
                assets,
                &mut alice,
                &file.entrance,
                &file.entrance.ledger,
                true,
            )?;
            store.write(Slot::Auto, &arrival)?;
            let mut slots = vec!["auto"];
            // The manual save one second before this visit's exit, when its leg passed.
            if let Some(pre) = file.leg.as_ref().and_then(|l| l.pre_exit.as_ref()) {
                let before = native_game(assets, &mut alice, pre, &file.entrance.ledger, false)?;
                store.write(Slot::One, &before)?;
                slots.push("slot1");
                let live = Restored::build(assets, before.clone())?;
                same(&level_of(&live)?, before.level()?, "writer pre-exit level")?;
            }
            let live = Restored::build(assets, arrival.clone())?;
            same(&level_of(&live)?, arrival.level()?, "writer arrival level")?;
            same(
                &live.alice.snapshot(),
                &arrival.character,
                "writer character",
            )?;
            fs::write(dir.join("writer.pid"), std::process::id().to_string())?;
            println!(
                "PASS wrote {name}: {} ({} visits in the ledger, {})",
                slots.join(" + "),
                arrival.campaign.levels.len(),
                origin(&file)
            );
            cases += 1;
            next_frame().await;
        }
        println!("PASS campaign save chain write: {cases} visits");
        return Ok(());
    }
    let mut cases = 0;
    let mut names: Vec<_> = fs::read_dir(root)
        .with_context(|| format!("No writer output in {ROOT}"))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("writer.pid").is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    ensure!(!names.is_empty(), "The writer left no cases in {ROOT}");
    for name in names {
        let dir = root.join(&name);
        let writer = fs::read_to_string(dir.join("writer.pid"))?;
        ensure!(
            writer != std::process::id().to_string(),
            "Reader must be a fresh process"
        );
        let store = Store::new(dir.clone(), fingerprint.clone());
        for slot in [Slot::Auto, Slot::One] {
            if !store.exists(slot) {
                continue;
            }
            let game = store.read(slot)?.game;
            let live = Restored::build(assets, game.clone())
                .with_context(|| format!("{name} {}", slot.name()))?;
            same(
                &level_of(&live)?,
                game.level()?,
                "level, enemies, puzzle and dialogue",
            )?;
            same(
                &live.alice.snapshot(),
                &game.character,
                "Alice animation and actions",
            )?;
            same(&live.game.stats, &game.stats, "resources and inventory")?;
            // Every cached visit rebuilds to what was saved, first and return visits apart.
            for level in game.campaign.levels.values() {
                let mut map = Bsp::parse(&assets.read(&format!("maps/{}.bsp", level.map))?)?;
                map.difficulty = game.stats.difficulty;
                let (i, story, hints) = level.restore_logic(assets, &map)?;
                same(
                    &i.snapshot(),
                    &level.interactions,
                    "cached puzzle and enemies",
                )?;
                same(&story.snapshot(), &level.story, "cached story")?;
                same(&hints.snapshot(), &level.hints, "cached Cheshire")?;
            }
            // Saving what was loaded gives what was loaded.
            let scratch = Store::new(dir.join("resaved"), fingerprint.clone());
            let mut again = game.clone();
            again.character = live.alice.snapshot();
            scratch.write(slot, &again)?;
            same(
                &scratch.read(slot)?.game.level()?,
                game.level()?,
                "resaved level",
            )?;
            println!(
                "PASS restarted {name} {}: writer={writer} reader={} exact state, {} cached visits",
                slot.name(),
                std::process::id(),
                game.campaign.levels.len()
            );
            next_frame().await;
        }
        cases += 1;
    }
    println!("PASS campaign save chain read: {cases} visits restored in a fresh process");
    Ok(())
}
