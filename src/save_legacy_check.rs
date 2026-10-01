//! Window-free counterpart of the retained-fixture half of `save_check` (`--save-legacy-check`).
//!
//! The windowed `--save-check-read` needs the Anode seat, so a legacy-save regression normally
//! surfaces only there. This check runs every part of that restore which needs no window: the
//! real decode, the cached-visit upgrades and the current visit's logic against its collision
//! world (`save::rebuild_headless`, the same code `Restored::build` runs), Alice's saved block
//! (through the real `Actions::restore` on the real clips and the documented migration), the
//! NPC cast identity, the player and resource assertions of each case, and a current-format
//! resave/reload. It also reads whatever cases a windowed `--save-check-write` left under
//! `private/save-check`. It skips a fixture that is missing, exactly like `save_check`, and it
//! never edits the fixtures: each is copied under `private/save-legacy-check` first.
//!
//! What it cannot run: `Npcs::load`/`Npcs::restore` (models), `Character::load`,
//! `Acting::restore` (idle clips) and the scene. Those remain Anode-only.
use crate::{
    assets::Assets,
    character::Snapshot as Alice,
    interaction,
    save::{self, Game, Headless, Slot, Store},
    skeletal::{Definition, Skeleton, Transform},
    weapons::Actions,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::Serialize;
use std::{fs, path::Path};

const ROOT: &str = "private/save-legacy-check";
/// Where the windowed `--save-check-write` leaves its cases.
const WRITTEN: &str = "private/save-check";

struct Fixture {
    label: &'static str,
    path: &'static str,
    line: &'static str,
}
const FIXTURES: [Fixture; 15] = [
    Fixture {
        label: "temple",
        path: "private/duchess-native-reward/auto.json",
        line: "actual v0.26 temple save",
    },
    Fixture {
        label: "temple-quick",
        path: "private/duchess-native-reward/quick.json",
        line: "retained format-8 Duchess quick save",
    },
    Fixture {
        label: "temple-previous",
        path: "private/duchess-native-reward/auto.previous.json",
        line: "retained format-8 temple backup save",
    },
    Fixture {
        label: "airship",
        path: "private/cinema-legacy-v6/quick.json",
        line: "actual v0.24 airship save",
    },
    Fixture {
        label: "pandemonium",
        path: "private/pand-legacy-v4/quick.json",
        line: "actual v0.22 Pandemonium save",
    },
    Fixture {
        label: "pandemonium-auto",
        path: "private/pand-legacy-v4/auto.json",
        line: "actual v0.22 Pandemonium automatic save",
    },
    Fixture {
        label: "school-v1",
        path: "private/event-legacy-v1-school.json",
        line: "original v0.19 school save",
    },
    Fixture {
        label: "battle-v1",
        path: "private/event-legacy-v1-battle.json",
        line: "original v0.19 battle save",
    },
    Fixture {
        label: "school-v2",
        path: "private/dice-legacy-v2-school.json",
        line: "original v0.20 school save",
    },
    Fixture {
        label: "battle-v2",
        path: "private/dice-legacy-v2-battle.json",
        line: "original v0.20 battle save",
    },
    Fixture {
        label: "return",
        path: "private/return-legacy-v5/quick.json",
        line: "actual v0.23 return save",
    },
    Fixture {
        label: "pool",
        path: "private/ladybug-legacy-v3/quick.json",
        line: "actual v0.21 Pool of Tears save",
    },
    Fixture {
        label: "pool-auto",
        path: "private/ladybug-legacy-v3/auto.json",
        line: "actual v0.21 Pool of Tears automatic save",
    },
    Fixture {
        label: "duchess-v7",
        path: "private/duchess-legacy-v7/quick.json",
        line: "retained format-7 Duchess quick save",
    },
    Fixture {
        label: "duchess-v7-auto",
        path: "private/duchess-legacy-v7/auto.json",
        line: "retained format-7 Duchess automatic save",
    },
];

fn same(a: &impl Serialize, b: &impl Serialize, label: &str) -> Result<()> {
    ensure!(
        serde_json::to_value(a)? == serde_json::to_value(b)?,
        "Restored {label} differs"
    );
    Ok(())
}
/// The restored visit's logic state, as `save_check::snapshot` saves it minus the NPC models.
fn logic(h: &Headless) -> serde_json::Value {
    serde_json::json!({
        "interactions": h.logic.interactions.snapshot(),
        "story": h.logic.story.snapshot(),
        "hints": h.logic.hints.snapshot(),
    })
}
/// Top-level keys of a saved Alice block that differ.
fn changed(a: &Alice, b: &Alice) -> Result<Vec<String>> {
    let (a, b) = (serde_json::to_value(a)?, serde_json::to_value(b)?);
    let (a, b) = (
        a.as_object().context("Alice block is not an object")?,
        b.as_object().context("Alice block is not an object")?,
    );
    let mut keys: Vec<_> = a
        .keys()
        .chain(b.keys())
        .filter(|k| a.get(*k) != b.get(*k))
        .cloned()
        .collect();
    keys.sort();
    keys.dedup();
    Ok(keys)
}
fn rest_pose(bones: usize) -> Vec<Transform> {
    vec![
        Transform {
            rotation: Quat::IDENTITY,
            translation: Vec3::ZERO,
        };
        bones
    ]
}

/// Alice's saved block: only the power-up presentation, which older saves lack, may change on
/// restore; the real weapon-action restore must accept the block; the documented migration
/// must predict the result exactly; and a current-format block must restore unchanged.
fn alice(assets: &mut Assets, bones: usize, old: &Game, had_power: bool) -> Result<Alice> {
    let mut actions = Actions::load(assets, bones, &rest_pose(bones))?;
    let restored = old
        .character
        .restored_headless(bones, &mut actions, &old.stats)
        .context("Alice block is not accepted")?;
    let expected: &[&str] = if had_power { &[] } else { &["power"] };
    let differing = changed(&old.character, &restored)?;
    println!("  Alice block, saved versus restored, differs in {differing:?}");
    ensure!(
        differing == expected,
        "Restoring Alice rewrote {differing:?}, expected {expected:?}"
    );
    same(
        &old.character.migrated(&old.stats),
        &restored,
        "Alice block after its documented migration",
    )?;
    let mut again = Actions::load(assets, bones, &rest_pose(bones))?;
    let twice = restored.restored_headless(bones, &mut again, &old.stats)?;
    ensure!(
        changed(&restored, &twice)?.is_empty(),
        "A current-format Alice block does not restore unchanged"
    );
    Ok(restored)
}

fn verify(
    assets: &mut Assets,
    root: &Path,
    fingerprint: &str,
    bones: usize,
    f: &Fixture,
) -> Result<()> {
    let dir = root.join(f.label);
    fs::create_dir_all(&dir)?;
    fs::copy(f.path, dir.join("quick.json"))?;
    let store = Store::new(dir, fingerprint.into());
    let old = store.read(Slot::Quick)?.game;
    let raw: serde_json::Value = serde_json::from_slice(&fs::read(f.path)?)?;
    let had_power = raw["payload"]["character"].get("power").is_some();
    let level = old.level()?.clone();
    match f.label {
        "temple" => {
            ensure!(level.map == "utemple", "Expected temple migration fixture");
            ensure!(
                old.stats.turtle_air
                    && old.player.breath.shell
                    && old.player.breath.remaining() == 20.,
                "Legacy temple save lost its breathing upgrade"
            );
        }
        "airship" => ensure!(
            !level.interactions.has_pandemonium_cinema(),
            "Expected v0.24 cinematic migration fixture"
        ),
        "pandemonium" => ensure!(
            !level.interactions.has_pandemonium(),
            "Expected actual v0.22 Pandemonium save"
        ),
        "school-v1" | "battle-v1" => ensure!(
            old.campaign
                .levels
                .values()
                .all(|l| !l.interactions.has_shared_state()),
            "Legacy fixture is not v1"
        ),
        "pool" => ensure!(
            !level.interactions.has_encounters(),
            "Expected actual old Pool of Tears save"
        ),
        _ => (),
    }
    let rebuilt = save::rebuild_headless(assets, old.clone())?;
    same(
        &old.stats,
        &rebuilt.game.stats,
        "legacy resources and inventory",
    )?;
    // The windowed cases that compare the player compare it exactly.
    if matches!(
        f.label,
        "airship" | "pandemonium" | "school-v1" | "battle-v1" | "return"
    ) {
        same(&old.player, &rebuilt.game.player, "legacy player")?;
    }
    ensure!(
        rebuilt
            .game
            .campaign
            .levels
            .values()
            .all(|l| l.interactions.has_shared_state()),
        "A cached visit escaped migration"
    );
    match f.label {
        "pandemonium" => ensure!(
            rebuilt.logic.interactions.pandemonium.is_some()
                && rebuilt
                    .logic
                    .interactions
                    .encounters
                    .as_ref()
                    .is_some_and(|e| e.actors.len() == 7),
            "Missing migrated Pandemonium components"
        ),
        "duchess-v7" | "duchess-v7-auto" => ensure!(
            level.map == "potears3"
                && !level.interactions.has_duchess()
                && rebuilt.logic.interactions.duchess.is_some(),
            "Missing migrated Duchess controller"
        ),
        "return" => ensure!(
            rebuilt
                .logic
                .interactions
                .school
                .as_ref()
                .is_some_and(|s| s.return_visit.is_some()),
            "Return migration missing"
        ),
        "pool" | "pool-auto" => {
            if level.interactions.has_pool() {
                same(
                    &old.player,
                    &rebuilt.game.player,
                    "legacy Pool of Tears player",
                )?;
            } else {
                // The leaf-transport upgrade deliberately starts pre-transport saves at the
                // playable entrance, retaining inventory and encounter history.
                let entrance = interaction::spawn(&rebuilt.map, level.entry.as_deref()).0;
                let expected = crate::movement::Player::spawn(&rebuilt.world, entrance)
                    .context("Legacy Pool of Tears entrance obstructed")?;
                same(
                    &expected,
                    &rebuilt.game.player,
                    "legacy Pool of Tears entrance migration",
                )?;
            }
            ensure!(
                rebuilt
                    .logic
                    .interactions
                    .encounters
                    .as_ref()
                    .is_some_and(|e| e.actors.len() == 12),
                "Missing upgraded Ladybugs"
            );
        }
        _ => (),
    }
    // Cast identity: every saved actor must still be an authored placement of this visit.
    rebuilt
        .level
        .npcs
        .matches_placements(
            &rebuilt.map,
            &rebuilt.level.map,
            rebuilt.level.entry.as_deref(),
        )
        .context("NPC cast")?;
    let block = alice(assets, bones, &old, had_power)?;
    // A current-format resave (with the migrated Alice block) must load to the same state.
    let mut resaved = rebuilt.game.clone();
    resaved.character = block.clone();
    store.write(Slot::Auto, &resaved)?;
    let next = save::rebuild_headless(assets, store.read(Slot::Auto)?.game)?;
    same(
        &logic(&rebuilt),
        &logic(&next),
        "migrated logic written/read in current format",
    )?;
    same(&rebuilt.game.player, &next.game.player, "migrated player")?;
    same(&rebuilt.game.stats, &next.game.stats, "migrated resources")?;
    let mut actions = Actions::load(assets, bones, &rest_pose(bones))?;
    let reloaded = next
        .game
        .character
        .restored_headless(bones, &mut actions, &next.game.stats)?;
    same(&block, &reloaded, "resaved Alice block")?;
    println!(
        "PASS headless {}: {} decoded, {} cached visits, Alice block {}, current-format resave/reload",
        f.line,
        level.map,
        old.campaign.levels.len(),
        if had_power {
            "unchanged"
        } else {
            "restored with only its power-up presentation initialised"
        }
    );
    Ok(())
}

/// The cases a windowed `--save-check-write` left under `private/save-check`, read window-free:
/// each must decode, rebuild to exactly the saved logic, keep Alice's block unchanged, and
/// rebuild every cached visit unchanged. The continued-simulation half of the windowed reader
/// (`future.json`) needs a scene and stays Anode-only.
fn written(assets: &mut Assets, fingerprint: &str, bones: usize) -> Result<usize> {
    let root = Path::new(WRITTEN);
    let Ok(entries) = fs::read_dir(root) else {
        println!("SKIP headless written cases: no windowed writer output under {WRITTEN}");
        return Ok(0);
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("writer.pid").is_file() && e.path().join("quick.json").is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for name in &names {
        let case = || format!("written case {name}");
        let store = Store::new(root.join(name), fingerprint.into());
        let game = store.read(Slot::Quick).with_context(case)?.game;
        let level = game.level()?.clone();
        let live = save::rebuild_headless(assets, game.clone()).with_context(case)?;
        same(
            &logic(&live),
            &serde_json::json!({
                "interactions": level.interactions,
                "story": level.story,
                "hints": level.hints,
            }),
            "level, enemies, puzzle and dialogue",
        )
        .with_context(case)?;
        let mut actions = Actions::load(assets, bones, &rest_pose(bones))?;
        let restored = game
            .character
            .restored_headless(bones, &mut actions, &game.stats)
            .with_context(case)?;
        same(&restored, &game.character, "Alice animation and actions").with_context(case)?;
        // Every cached visit is rebuilt too, including first/return school separation.
        for cached in game.campaign.levels.values() {
            let mut map =
                crate::bsp::Bsp::parse(&assets.read(&format!("maps/{}.bsp", cached.map))?)?;
            map.difficulty = game.stats.difficulty;
            let (i, story, hints) = cached.restore_logic(assets, &map).with_context(case)?;
            same(
                &i.snapshot(),
                &cached.interactions,
                "cached puzzle and enemies",
            )
            .with_context(case)?;
            same(&story.snapshot(), &cached.story, "cached story").with_context(case)?;
            same(&hints.snapshot(), &cached.hints, "cached Cheshire").with_context(case)?;
            cached
                .npcs
                .matches_placements(&map, &cached.map, cached.entry.as_deref())
                .with_context(case)?;
        }
        println!("PASS headless written case {name}: logic, Alice block and {} cached visits restore exactly", game.campaign.levels.len());
    }
    Ok(names.len())
}

pub fn check(assets: &mut Assets) -> Result<()> {
    let fingerprint = assets.fingerprint()?;
    let def = Definition::alice(assets)?;
    let bones = Skeleton::parse(&assets.read(&format!("{}/{}", def.path, def.model))?)?
        .bones
        .len();
    let root = Path::new(ROOT);
    let mut checked = 0;
    for f in &FIXTURES {
        if !Path::new(f.path).exists() {
            println!("SKIP headless {}: private fixture not present", f.line);
            continue;
        }
        verify(assets, root, &fingerprint, bones, f)
            .with_context(|| format!("{} ({})", f.line, f.path))?;
        checked += 1;
    }
    let cases = written(assets, &fingerprint, bones)?;
    println!(
        "PASS save-legacy-check: {checked} of {} retained legacy fixtures and {cases} written cases restore window-free; NPC models, Alice's skin, the scene and the continued simulation stay in the Anode --save-check-read",
        FIXTURES.len()
    );
    Ok(())
}
