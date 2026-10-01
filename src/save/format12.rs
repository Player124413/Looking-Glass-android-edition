//! Save format 12 (F2): the single bump the campaign makes, its reader and writer guards, and (in
//! the same module, so one builder serves both) the size limits of a full 39-visit campaign.
//! Everything here is synthetic: no game data, no window, no private fixture.
use super::*;
use serde_json::{json, Value};
use std::path::PathBuf;

/// Alice's saved block, as the smallest value her deserializer accepts. The bounds
/// `Game::validate` and `Store::decode` check never look inside it.
pub(super) fn character() -> crate::character::Snapshot {
    let anchor = json!({ "rotation": [0., 0., 0., 1.], "translation": [0., 0., 0.] });
    serde_json::from_value(json!({
        "animator": {
            "motion": "Idle", "time": 0., "transition": 1., "previous": [], "pose": [],
            "jumps": 0, "landings": 0, "large": false, "offset": 0., "previous_offset": 0.
        },
        "facing": 0.,
        "actions": { "playing": null, "selected": 0, "pose": [], "start": [], "variation": 0 },
        "projectiles": { "projectiles": [], "shots": 0, "impacts": 0 },
        "anchors": [anchor, anchor, anchor],
        "viewmodel": { "phase": 0., "bob": [0., 0., 0.] }
    }))
    .expect("the synthetic Alice block deserializes")
}
/// A visit's interaction state with an event runtime (`shared`) and the given registered
/// controllers' state under `levels`. Absent, like every legacy visit, when `levels` is null.
pub(super) fn interactions(levels: Value) -> crate::interaction::Snapshot {
    let mut value = serde_json::to_value(Interactions::empty().snapshot()).unwrap();
    if !levels.is_null() {
        value["levels"] = levels;
    }
    serde_json::from_value(value).expect("the synthetic interaction snapshot deserializes")
}
/// A cached visit whose story, Cheshire and cast are empty.
pub(super) fn level(map: &str, entry: Option<&str>, levels: Value) -> Level {
    Level {
        map: map.into(),
        entry: entry.map(str::to_owned),
        interactions: interactions(levels),
        npcs: serde_json::from_value(json!({ "actors": [], "greetings": 0 })).unwrap(),
        story: serde_json::from_value(json!({
            "seen": [], "queue": [], "current": null, "exit": null, "completed": 0, "finished": []
        }))
        .unwrap(),
        hints: serde_json::from_value(json!({
            "selected": null, "next": 0, "cooldown": 0., "appearance": null
        }))
        .unwrap(),
        environment_clock: 3.25,
        pickup_clock: 2.625,
    }
}
/// A game whose cached visits are `levels`, all completed, the first one current.
pub(super) fn game(levels: Vec<Level>) -> Game {
    let player = Player::new(vec3(0., 0., 0.));
    let current = levels[0].key();
    Game {
        current,
        campaign: Campaign {
            completed: levels.iter().map(Level::key).collect(),
            levels: levels.into_iter().map(|l| (l.key(), l)).collect(),
        },
        stats: Stats::for_level("skool1", None),
        view: View {
            position: player.eye(),
            yaw: 0.,
            pitch: 0.,
            third_person: false,
            flying: false,
            fullbright: false,
            spawn_landing: false,
        },
        player,
        recovery: Default::default(),
        character: character(),
    }
}
/// A private scratch folder, removed when dropped.
pub(super) struct Scratch(pub PathBuf);
impl Scratch {
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("looking-glass-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    pub fn store(&self) -> Store {
        Store::new(self.0.clone(), "synthetic-game-data".into())
    }
    pub fn file(&self, slot: Slot) -> PathBuf {
        self.0.join(format!("{}.json", slot.name()))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
/// Rewrite the envelope's version. The checksum covers the payload only, so the file stays valid.
pub(super) fn relabel(path: &Path, version: u64) {
    let mut e: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    e["version"] = version.into();
    fs::write(path, serde_json::to_vec(&e).unwrap()).unwrap();
}
fn version_of(path: &Path) -> u64 {
    let e: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    e["version"].as_u64().unwrap()
}

fn with_state() -> Game {
    game(vec![
        level(
            "garden1",
            None,
            json!({ "garden1": { "version": 1, "open": true } }),
        ),
        level("skool1", None, Value::Null),
    ])
}

#[test]
fn the_writer_writes_format_12_and_the_reader_accepts_one_through_twelve() {
    assert_eq!(VERSION, 12);
    assert_eq!(
        LEVELS_SINCE, 12,
        "the campaign bumps the format exactly once"
    );
    let scratch = Scratch::new("f12-versions");
    let store = scratch.store();
    let plain = game(vec![level("skool1", None, Value::Null)]);
    store.write(Slot::Quick, &plain).unwrap();
    let path = scratch.file(Slot::Quick);
    assert_eq!(version_of(&path), 12);
    // Every earlier format still reads: a file that carries no registered state is unchanged.
    for version in 1..=12 {
        relabel(&path, version);
        let loaded = store
            .read(Slot::Quick)
            .unwrap_or_else(|e| panic!("v{version}: {e:#}"));
        assert!(!loaded.backup, "v{version} came from the backup");
        assert_eq!(loaded.game.current, "skool1$first");
    }
    // Formats from the future and format 0 never do.
    for version in [0, 13, 999] {
        relabel(&path, version);
        assert!(store.read(Slot::Quick).is_err(), "v{version} was accepted");
    }
}
#[test]
fn a_levels_map_in_a_file_older_than_12_is_rejected() {
    let scratch = Scratch::new("f12-levels");
    let store = scratch.store();
    store.write(Slot::Quick, &with_state()).unwrap();
    let path = scratch.file(Slot::Quick);
    assert_eq!(version_of(&path), 12);
    let payload: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert!(holds_registry_state(&payload["payload"]));
    let saved = store.read(Slot::Quick).unwrap().game;
    assert_eq!(
        serde_json::to_value(&saved.campaign.levels["garden1$first"].interactions).unwrap()
            ["levels"]["garden1"]["open"],
        true
    );
    for version in 1..=11 {
        relabel(&path, version);
        let error = store
            .read(Slot::Quick)
            .err()
            .unwrap_or_else(|| panic!("v{version} carrying registered state was accepted"));
        assert!(
            format!("{error:#}").contains("needs save format 12"),
            "v{version}: {error:#}"
        );
    }
    relabel(&path, 12);
    assert!(store.read(Slot::Quick).is_ok());
}
#[test]
fn the_guard_reads_only_a_real_levels_map_and_fails_closed_on_anything_else() {
    let payload = |levels: Value| json!({ "campaign": { "levels": { "a$first": { "interactions": { "levels": levels } } } } });
    for state in [
        Value::Null,
        json!({}),
        // A visit with no `levels` key at all is every legacy visit.
    ] {
        assert!(!holds_registry_state(&payload(state)));
    }
    assert!(!holds_registry_state(&json!({ "campaign": { "levels": {
        "a$first": { "interactions": {} }
    } } })));
    assert!(!holds_registry_state(&json!({})));
    for state in [
        json!({ "garden1": null }),
        json!({ "garden1": { "version": 1 } }),
        json!([1]),
        json!("garden1"),
        json!(3),
    ] {
        assert!(holds_registry_state(&payload(state.clone())), "{state}");
        assert!(
            require_format_for_levels(11, &payload(state.clone())).is_err(),
            "{state}"
        );
        assert!(require_format_for_levels(12, &payload(state)).is_ok());
    }
    // Only the newest format may hold state; an empty map is legal in every one.
    assert!(require_format_for_levels(1, &payload(json!({}))).is_ok());
}
#[test]
fn a_stateless_registered_controller_still_needs_the_new_format() {
    // A controller with no state saves `null`, but the key records that it served the visit,
    // so an older executable must not load the file and lose the visit's identity.
    let scratch = Scratch::new("f12-null");
    let store = scratch.store();
    store
        .write(
            Slot::Quick,
            &game(vec![level("garden1", None, json!({ "garden1": null }))]),
        )
        .unwrap();
    let path = scratch.file(Slot::Quick);
    relabel(&path, 11);
    assert!(store.read(Slot::Quick).is_err());
    relabel(&path, 12);
    assert!(store.read(Slot::Quick).is_ok());
}

// F2 step 6: what `settle` does with Alice when a controller joined the visit after the save.
mod joined {
    use super::*;
    use crate::{collision::World, level::Respawn, levels::synthetic};

    const FLOOR: (Vec3, Vec3) = (Vec3::new(-500., -500., -20.), Vec3::new(500., 500., 0.));
    const NOTHING: Migrations = Migrations {
        pool: false,
        beyond: false,
        fortress: false,
        duchess: false,
        levels: false,
    };
    const JOINED: Migrations = Migrations {
        levels: true,
        ..NOTHING
    };
    /// Alice saved at `x`, looking at yaw 1.
    fn saved_at(x: f32) -> Game {
        let mut g = game(vec![level("garden1", None, Value::Null)]);
        g.player = Player::new(vec3(x, 0., 0.));
        g.view.position = g.player.eye();
        g.view.yaw = 1.;
        g
    }
    /// A crate around x = 300.
    fn crate_world() -> World {
        World::fixture(&[FLOOR, (vec3(260., -40., 0.), vec3(340., 40., 120.))])
    }
    /// A controller that rejects everything beyond x = 200.
    fn controller() -> synthetic::Synthetic {
        let mut c = synthetic::first();
        c.reject_beyond_x = Some(200.);
        c
    }
    fn settle_with(
        game: &mut Game,
        world: &mut World,
        c: synthetic::Synthetic,
        migrate: Migrations,
    ) -> Result<()> {
        let map = synthetic::map();
        settle(
            game,
            None,
            &synthetic::served(&map, c),
            &map,
            world,
            migrate,
        )
    }

    #[test]
    fn a_position_the_controller_accepts_and_the_body_fits_is_kept() {
        let mut g = saved_at(100.);
        let mut world = World::fixture(&[FLOOR]);
        settle_with(&mut g, &mut world, controller(), JOINED).unwrap();
        assert_eq!(g.player.feet.x, 100.);
        assert_eq!(g.view.yaw, 1.);
    }
    #[test]
    fn a_position_the_controller_rejects_restarts_at_the_entrance_only_after_an_upgrade() {
        let mut world = World::fixture(&[FLOOR]);
        // A save that was already current is refused, exactly as before F2.
        let mut g = saved_at(300.);
        assert!(settle_with(&mut g, &mut world, controller(), NOTHING).is_err());
        // One a controller joined restarts at the entrance and can then continue.
        let mut g = saved_at(300.);
        settle_with(&mut g, &mut world, controller(), JOINED).unwrap();
        assert_eq!((g.player.feet.x, g.player.feet.y), (0., 0.));
        assert!(world.body_clear(g.player.feet));
        assert_eq!(g.view.position, g.player.eye());
        assert_eq!(g.view.yaw, 0., "the entrance's facing");
    }
    #[test]
    fn an_obstructed_body_restarts_at_the_entrance_unless_alice_is_flying() {
        let mut world = crate_world();
        // Current format: the obstruction is an error.
        let mut g = saved_at(300.);
        assert!(settle_with(&mut g, &mut world, synthetic::first(), NOTHING).is_err());
        // After an upgrade: back to the entrance, clear of the crate.
        let mut g = saved_at(300.);
        settle_with(&mut g, &mut world, synthetic::first(), JOINED).unwrap();
        assert_eq!((g.player.feet.x, g.player.feet.y), (0., 0.));
        assert!(world.body_clear(g.player.feet));
        // Free flight may sit inside geometry, upgraded or not.
        let mut g = saved_at(300.);
        g.view.flying = true;
        settle_with(&mut g, &mut world, synthetic::first(), JOINED).unwrap();
        assert_eq!(g.player.feet.x, 300.);
    }
    #[test]
    fn a_controller_can_ask_for_the_entrance_whatever_the_saved_position() {
        let mut world = World::fixture(&[FLOOR]);
        let always = || {
            let mut c = synthetic::first();
            c.plan.respawn = Respawn::Always;
            c
        };
        let mut g = saved_at(100.);
        settle_with(&mut g, &mut world, always(), JOINED).unwrap();
        assert_eq!((g.player.feet.x, g.view.yaw), (0., 0.));
        // Without the upgrade the declaration is not consulted.
        let mut g = saved_at(100.);
        settle_with(&mut g, &mut world, always(), NOTHING).unwrap();
        assert_eq!(g.player.feet.x, 100.);
    }
    #[test]
    fn a_blocked_entrance_is_an_error_not_a_silent_flight() {
        // The entrance itself is inside geometry: the upgrade cannot place Alice anywhere.
        let mut world = World::fixture(&[FLOOR, (vec3(-20., -20., 0.), vec3(20., 20., 4000.))]);
        let mut g = saved_at(300.);
        assert!(settle_with(&mut g, &mut world, controller(), JOINED).is_err());
    }
    /// A saved cast of `(name, model, greeting)` actors, placed 64 units apart.
    fn cast(actors: &[(&str, &str, f32)]) -> crate::npc::Snapshot {
        let actors: Vec<Value> = actors
            .iter()
            .enumerate()
            .map(|(i, &(name, model, greeting))| {
                json!({
                    "story_visible": false, "speaking": false, "model": i,
                    "spawn": {
                        "name": name, "hidden": false, "model": model,
                        "origin": [64. * i as f32, 0., 0.], "yaw": 0., "scale": 1.,
                        "animation": null
                    },
                    "yaw": 0., "time": 1., "greeting": greeting, "cooldown": 0., "guard": null
                })
            })
            .collect();
        serde_json::from_value(json!({ "actors": actors, "greetings": 2 })).unwrap()
    }
    fn greetings(s: &crate::npc::Snapshot) -> Vec<f64> {
        serde_json::to_value(s).unwrap()["actors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["greeting"].as_f64().unwrap())
            .collect()
    }
    #[test]
    fn the_cast_is_regenerated_only_when_a_controller_owns_part_of_it() {
        use std::cell::Cell;
        let serving: &[&'static crate::levels::Registration] = &[&synthetic::REGISTRATION];
        let saved = cast(&[("gnome", "c_gnome", 5.), ("synthetic_guard", "c_guard", 7.)]);
        let loads = Cell::new(0);
        let fresh = || {
            loads.set(loads.get() + 1);
            Ok(cast(&[
                ("gnome", "c_gnome", 0.),
                ("synthetic_guard", "c_guard", 0.),
            ]))
        };
        // Owned: the models are loaded once, the guard starts fresh, the gnome keeps its state.
        let adopted = adopt_cast(&saved, serving, fresh).unwrap().unwrap();
        assert_eq!(loads.get(), 1);
        assert_eq!(greetings(&adopted), [5., 0.]);
        // Nothing owned (no controller, or one that owns other actors): the saved cast stands
        // and nothing is loaded.
        let plain = cast(&[("gnome", "c_gnome", 5.)]);
        assert!(adopt_cast(&plain, serving, fresh).unwrap().is_none());
        assert!(adopt_cast(&saved, &[], fresh).unwrap().is_none());
        assert_eq!(loads.get(), 1, "no model load without a change of cast");
        // A failed load is an error, never a silently kept stale cast.
        let broken = || anyhow::bail!("models unavailable");
        assert!(adopt_cast(&saved, serving, broken).is_err());
    }
    #[test]
    fn a_current_format_visit_without_a_registration_needs_no_upgrade() {
        // A synthetic unregistered visit stays independent of campaign coverage.
        for levels in [Value::Null, json!({ "unregistered_fixture": null })] {
            let l = level("unregistered_fixture", None, levels);
            assert!(!l.registry_upgrade());
            assert!(!l.needs_upgrade());
        }
    }
}

// The size limits of a whole campaign (F2, D7): 8 MiB per file, 72 cached visits and 10,000
// elements per array. A campaign has 39 visits, so every one keeps a state of its own; the
// synthetic save below holds all of them, and the forged files prove each limit is enforced by
// the reader and the writer rather than merely respected by the fixture.
mod limits {
    use super::*;

    /// A controller state of `arrays` arrays of `elements` seven-digit integers each.
    fn state(arrays: usize, elements: usize) -> Value {
        json!({
            "version": 1,
            "cells": (0..arrays)
                .map(|a| (0..elements).map(|i| 1_000_000 + a * 7 + i).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        })
    }
    /// `count` distinct synthetic visits, each holding `state` under its controller's key.
    fn visits(count: usize, state: &Value) -> Vec<Level> {
        (1..=count)
            .map(|n| level(&format!("visit{n:02}"), None, json!({ "garden1": state })))
            .collect()
    }
    /// The longest array anywhere in a payload.
    fn longest(value: &Value) -> usize {
        match value {
            Value::Array(a) => a.len().max(a.iter().map(longest).max().unwrap_or(0)),
            Value::Object(o) => o.values().map(longest).max().unwrap_or(0),
            _ => 0,
        }
    }
    /// Change the payload of a written save and keep its checksum valid, as a forger would.
    fn forge(path: &Path, change: impl FnOnce(&mut Value)) {
        let mut e: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        change(&mut e["payload"]);
        e["checksum"] = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&e["payload"]).unwrap())
        )
        .into();
        fs::write(path, serde_json::to_vec(&e).unwrap()).unwrap();
    }
    fn refused(store: &Store, expected: &str) {
        let error = store
            .read(Slot::Quick)
            .err()
            .expect("the save was accepted");
        assert!(format!("{error:#}").contains(expected), "{error:#}");
    }

    #[test]
    fn a_39_visit_campaign_with_a_controller_state_in_every_visit_fits_with_room_to_spare() {
        assert_eq!(crate::campaign::route().len(), 39);
        // Each visit's state is 80 KB, about half as large again as the average legacy visit
        // (the 8-visit fixture was about 418 KB).
        let scratch = Scratch::new("f12-39");
        let store = scratch.store();
        let written = game(visits(39, &state(2, 5_000)));
        store.write(Slot::Quick, &written).unwrap();
        let size = fs::metadata(scratch.file(Slot::Quick)).unwrap().len();
        eprintln!("39 visits of 80 KB state: {size} bytes of {LIMIT}");
        assert!(size < LIMIT / 2, "{size} bytes leaves too little headroom");
        let loaded = store.read(Slot::Quick).unwrap().game;
        assert_eq!(loaded.campaign.levels.len(), 39);
        assert_eq!(loaded.campaign.completed.len(), 39);
        let payload = serde_json::to_value(&loaded).unwrap();
        assert!(longest(&payload) <= 5_000);
        assert!(holds_registry_state(&payload));
        // Every visit's state came back intact.
        for level in loaded.campaign.levels.values() {
            let saved = serde_json::to_value(&level.interactions).unwrap();
            assert_eq!(saved["levels"]["garden1"], state(2, 5_000));
        }
    }
    #[test]
    fn the_reader_and_the_writer_refuse_a_73rd_cached_visit() {
        let scratch = Scratch::new("f12-72");
        let store = scratch.store();
        store
            .write(Slot::Quick, &game(visits(72, &state(1, 4))))
            .unwrap();
        assert_eq!(
            store.read(Slot::Quick).unwrap().game.campaign.levels.len(),
            72
        );
        // The writer.
        let error = store
            .write(Slot::One, &game(visits(73, &state(1, 4))))
            .unwrap_err();
        assert!(format!("{error:#}").contains("too large"), "{error:#}");
        assert!(!scratch.file(Slot::One).exists());
        // The reader, given a file a forger made: a 73rd visit with a valid checksum.
        forge(&scratch.file(Slot::Quick), |payload| {
            let mut extra = payload["campaign"]["levels"]["visit72$first"].clone();
            extra["map"] = "visit73".into();
            payload["campaign"]["levels"]["visit73$first"] = extra;
        });
        refused(&store, "too large");
        // Completion markers are bounded the same way.
        let scratch = Scratch::new("f12-72-completed");
        let store = scratch.store();
        store
            .write(Slot::Quick, &game(visits(72, &state(1, 4))))
            .unwrap();
        forge(&scratch.file(Slot::Quick), |payload| {
            let done = payload["campaign"]["completed"].as_array_mut().unwrap();
            done.push("visit73$first".into());
        });
        refused(&store, "too large");
    }
    #[test]
    fn arrays_are_limited_to_10000_elements_on_write_and_on_read() {
        let scratch = Scratch::new("f12-array");
        let store = scratch.store();
        store
            .write(Slot::Quick, &game(visits(1, &state(1, 10_000))))
            .unwrap();
        assert_eq!(
            longest(&serde_json::to_value(&store.read(Slot::Quick).unwrap().game).unwrap()),
            10_000
        );
        // One more element is refused by the writer ...
        let error = store
            .write(Slot::One, &game(visits(1, &state(1, 10_001))))
            .unwrap_err();
        assert!(
            format!("{error:#}").contains("Invalid save array"),
            "{error:#}"
        );
        // ... and by the reader, for a file that was edited after it was written.
        forge(&scratch.file(Slot::Quick), |payload| {
            let cells = &mut payload["campaign"]["levels"]["visit01$first"]["interactions"]
                ["levels"]["garden1"]["cells"][0];
            cells.as_array_mut().unwrap().push(json!(1));
        });
        refused(&store, "Invalid save array");
    }
    #[test]
    fn the_file_is_limited_to_8_mib_on_write_and_on_read() {
        // Writer: 39 visits of three full arrays each are valid one by one and far too big
        // together.
        let scratch = Scratch::new("f12-size");
        let store = scratch.store();
        let error = store
            .write(Slot::Quick, &game(visits(39, &state(3, 10_000))))
            .unwrap_err();
        assert!(format!("{error:#}").contains("size limit"), "{error:#}");
        assert!(!scratch.file(Slot::Quick).exists());
        // Reader: a valid save padded (with whitespace) to exactly 8 MiB loads, one byte more
        // does not, whatever it holds.
        store
            .write(Slot::Quick, &game(visits(2, &state(1, 100))))
            .unwrap();
        let path = scratch.file(Slot::Quick);
        let mut bytes = fs::read(&path).unwrap();
        bytes.resize(LIMIT as usize, b' ');
        fs::write(&path, &bytes).unwrap();
        assert!(
            store.read(Slot::Quick).is_ok(),
            "exactly the limit is allowed"
        );
        bytes.push(b' ');
        fs::write(&path, &bytes).unwrap();
        refused(&store, "too large");
    }
}
