# Persistent saves

After death, **Enter** (also R/Home or console `restart`) restores the whole last successful save/load or authored checkpoint for the current attempt. Before any save, it restores the current visit's entry snapshot. Alice, enemies, scripted hazards, guides, dialogue and puzzle progress return to the same moment; retry no longer revives Alice in a world that has continued ahead. Death retry uses an in-memory snapshot and does not overwrite a disk slot. See [RECOVERY.md](RECOVERY.md).

The Load/Save screen uses the original ornate photo frames, brass camera controls, animated film texture and shutters. Slot selection and confirmations retain the existing six-slot behavior. Chapter names and save dates (UTC) appear on the camera's caption plates; the effects-volume and mute settings also control its original projector/button sounds. See [MENUS.md](MENUS.md).

New manual saves, quick saves and autosaves include a screenshot of the game at the saved location. **Escape → Load/Save** shows it in the slot and in the larger selected preview. The capture excludes menus, HUD, console and cursor, and preserves the view's proportions. Older saves remain loadable and show **No preview** until saved again.

The small JPEG is optional metadata inside the same JSON file, so overwrites and `.previous.json` recovery keep the picture paired with its game state. No separate image files need to be copied. Missing or damaged preview data does not prevent a valid game from loading. This addition keeps format **11**; earlier readers ignore the optional metadata.

`--save-preview-check` uses isolated files under `private/save-preview-check` to verify clean-frame capture, two different views, overwrite/reopen, failed-write preservation, matching backup recovery, old-save and damaged-image fallbacks, and the menu display. Native keyboard/mouse checks also covered F5, saving from the menu, clean-quit autosave and loading the manual slot in a new process.

**Format 12 (campaign plan F2, decision DG-6).** This build writes format **12**, reading formats 1–11. The only addition is the optional per-visit `levels` map, one key per registered level controller ([LEVEL_REGISTRY.md](LEVEL_REGISTRY.md)). No existing field, event signature or controller state changes, so every legacy visit serializes exactly as it did in format 11. The bump exists so that an older executable refuses a file that carries registered state with a clear version message, instead of failing later on an event-signature mismatch. The reader rejects a non-empty `levels` map in a file whose version is below 12; the guard fails closed, so any value there other than absent, null or an empty object counts as state. The writer writes 12 and refuses to write registered state under an older version. This is the only bump the campaign makes: a level branch never changes the version, and any later bump is a new decision.

Version 0.31.7 writes format **11**, reading formats 1–10. The console's `notarget` setting and explicitly set health above 100 persist alongside God mode, granted inventory and view mode. Old saves default `notarget` off. An isolated native save/restart check restores 250 Sanity, all toys, God, notarget and first-person view. Camera distance is stored with the options separately from the campaign save. See [CONSOLE.md](CONSOLE.md).

Beyond the Wall now saves its levers, moving walkway, arch sequence and shuffled doors. Older generic fortress2 saves acquire the controller and restart at its entrance while retaining resources and inventory. Partial-motion JSON round trips are covered by `--beyond-check`.

Version 0.28 writes format **10**, reading formats 1–9. Difficulty, Rage/Tea/Glass duration, Watch stopped time and recharge, live enemy essence and the per-visit reward ledger persist. Timers resume without counting time while closed. Legacy saves default to Normal and preserve their existing Glass timer. Previously omitted difficulty-specific actors are added without resetting old actors. Nine new native cases bring the restart suite to **52**. See [ITEMS.md](ITEMS.md).

Version 0.27 writes format **9**, reading formats 1–8. Remaining air and drowning cadence, the temple breathing upgrade, generic rope attachment/momentum, updraft linger and knockback control delay now persist. Movement volumes use shared saved entity enable flags. Older saves receive a fresh air budget and the upgrade when their campaign history establishes temple access. New movement state is checked against the rebuilt world. Five added cases bring the native restart suite to **43**; see [TRAVERSAL.md](TRAVERSAL.md).

Version 0.26 writes format **8**, reading formats 1–7. Duchess saves include combat/action state, timed attack cues, projectiles/explosions, arena changes, replenishing essence, shell reward and exit latch. Cached older potears3 visits upgrade their reviewed gates and arena controller. Ten new native cases bring the restart suite to **38**. See [DUCHESS.md](DUCHESS.md).

Version 0.25 writes format **7**, reading formats 1–6. Pandemonium scenes save camera/actor clocks, dialogue position, return position and completed disappearances. Save/load works during and after skips. Older active transports keep their existing attachment and progress; already completed events do not replay the new staging. Cached Pandemonium visits upgrade before resaving. The native writer/reader covers 28 cases, including post-skip saves and an actual v0.24 airship migration. See [CINEMATICS.md](CINEMATICS.md).

Version 0.24 writes format **6**, reading formats 1-5. School-return saves retain quest phase, spent potion/star, globe/drinking/shrinking time, doors and lift motion alongside the existing inventory, resources, pickups and enemy state. Older return visits gain the observatory objects and controller; only previously unsupported observatory contacts are rearmed. Reviewed condition changes suppress obsolete first-visit story contacts without replaying combat. Every cached return visit is upgraded before resaving.




Version 0.23 introduced format **5** with support for formats 1–4. Pandemonium saves retain rope grip/momentum, transport attachment, cart phase, key/door/return state, airship time, departure dialogue, guards and delayed activations. The loader validates attachment positions and gate combinations before replacing the live game. Old Pandemonium visits acquire the new controller and encounter ownership; existing resources, pickups and decorative NPC identity records are preserved, and old club-guard health/death state is imported. Previously unsupported script contacts are rearmed so the new progression can run. Start a fresh visit for an unambiguous normal-route playthrough if an older save was made using cheats.



During the cart or airship ride, recovery waits until landing. After the one-way cart ride and before the return portal, Home/fallback recovery uses the far landing so it cannot strand Alice behind the spent cart.



Press **F5** to quick-save and **F9** to quick-load. Loading pauses the game; press **P** (or its rebound key), click, or press controller **A** to resume. On launch, **Continue** (Enter or click) restores the newest readable manual, quick or automatic save. **N** starts a new visit in the launcher's selected map. Escape closes the start screen without writing a save.



Normal level exits and clean quit write a separate automatic slot. They never overwrite the quick slot. Saving is unavailable while dead or in staged preview modes; load a save or retry first. There is no periodic autosave, so use F5 before difficult sections. Starting a new visit, including the Tab chooser or console `map`, resets the in-memory campaign and updates the automatic slot when that new session ends; the existing quick save remains until F5 replaces it.



The console accepts `save`, `save quick`, `load`, `load quick`, and `load auto`. Paths and arbitrary slot names are not console commands. Command-line options:



```

Launch.cmd --load quick

Launch.cmd --load auto

Launch.cmd --load slot2

Launch.cmd --new-game

Launch.cmd --save-dir private/my-other-saves

```



`--new-game` skips Continue; it does not delete files. Ordinary launchers run from the project directory, so the default folder is `C:\DEV\McGee\private\saves`. The six slots are `slot1.json`–`slot4.json`, `quick.json` and `auto.json`. Each keeps a prior validated `.previous.json` copy. In the original **Escape → Load/Save** menu, select a thumbnail and choose Save. Empty manual slots work immediately; occupied slots ask before replacement. The automatic slot is load-only in the menu. Quick save and autosave never replace manual slots. Copy this folder while the game is closed to keep a separate backup.



## Preserved state



- Sanity, Will, regeneration delay, Looking Glass duration, all weapon ownership/counts, selection, and school quest items.

- Player position, velocity, water/climb state, view direction/mode, animation/action progress, active gameplay projectiles, and recovery footing history.

- Demon Dice action, thrown dice, roll/random state, summon recovery, demon type/position/health/target/attack state and damaging fireballs. Cosmetic lightning and trails restart; loading cannot reroll a pending summon or charge its Will again.

- Collected pickup identities, secret rewards, door angles/locks, trigger contacts/cooldowns/one-shot history, and pending enemy activations.

- Shared entity identities and enable flags, puzzle flags/counters, rule activation history, event cooldowns, pending delayed events and their deterministic order. Paused games and cached visits do not advance these event clocks.

- Implemented enemy activation, position, health, death, AI timers and attacks; ordinary NPC animation/reaction state.

- Pool of Tears Ladybug patrol/navigation state, carried acorns, airborne bombs, rearm/pain/death timers and pending second ambush arrivals. Cosmetic blast flashes restart.

- School shelves, lifts, flying books, recipe and secret; second-school bleachers, platforms, pendulums, ingredient transformations, rewards and exit gate; village machinery and hatch.

- State of a registered level controller ([LEVEL_REGISTRY.md](LEVEL_REGISTRY.md)), under one key per controller id in the visit's optional `levels` map. Legacy visits never write the key, so their saves are unchanged. Only format 12 and later may hold it. A save that names a controller the visit lacks is refused. A save that omits a controller the visit now has predates its registration and is upgraded on load (see below).

- Current/queued dialogue references and timing, seen/completed events, pending story exits, Cheshire hint/appearance state, environment/pickup clocks.

- Current map and entrance, completed visit markers and cached state for previously visited levels. First and return visits use separate state; named first entrances and default first entrances share a visit. Visited levels remain frozen while Alice is elsewhere.



Free flight, god mode and inspection lighting are retained if enabled. Sound preferences remain in their existing separate settings file. Music and cosmetic steam/trail/spark particles restart; voices resume at the saved line time. Console history and open menus are not saved. This stores implemented gameplay only, not the remaining original script systems, original checkpoints or an unfinished campaign route.



## Format and failure handling



This is a Rust save format, **version 12**. Original Alice saves cannot be imported. The reader also accepts **versions 1 through 11 from earlier prototypes**. Versions 1/2 start with no active Dice summon while retaining inventory counts; version 3 preserves its existing Dice state. Version 1 reconstructs entity IDs and imports consumed trigger, encounter, lever and quest events for every cached visit without replaying effects. Older Pool of Tears visits acquire the new Ladybug encounter controller while retaining existing event history and NPC identities; already-passed ambush triggers are not replayed. Previously decorative resident Ladybugs become live combat actors. Version 11 adds the persistent notarget setting and accepts explicit console health above 100; older saves default notarget off. Version 12 adds only the registered controllers' state (see the history above). The next save writes version 12, which older executables cannot load. Prior executable/source backups and unmodified legacy fixtures remain private.



JSON stores runtime values and asset/event identifiers; it does not embed original meshes, images, audio or subtitle text. The reader checks version, a fingerprint of the resolved game-data archive entries, payload checksum, size and runtime bounds. Shared event state also checks the compiled rule/registry signature, fact types, identities, event references, bounded queues and activation timestamps. Saves from changed data or incompatible event definitions are rejected instead of applying state to different entities. This signature check is deliberately conservative; later changes to event definitions may need an explicit migration.

NPC identities are matched exactly on load, so a change in which models load changes the saved cast. The Clockwork Automaton placeholder actors were skipped by the checkpoint executables and load again after the off-norm tag fix (NPCS.md). A save written inside funhouse, hatter1, hedge2 or hedge3 by an executable that skipped them is refused with a cast mismatch; one written before the skip loads again. Those visits are not yet playable; no legacy fixture covers them and the save format version is unchanged.



Writes use a flushed temporary file followed by replacement of the committed slot. A damaged primary file can fall back to its previous validated copy, with a visible notice. Invalid files produce an error rather than resetting the current game. Loading constructs and checks the saved scene, collision and current actors before replacing the live session. Cached visits are rebuilt and validated when revisited. A checksum detects accidental damage; it is not a signature or a security boundary.



Save files and backups stay under `private/` and are excluded from the source-review archive. Nothing is uploaded.



## Restart verification



Run `--save-check-write`, let it exit, then run `--save-check-read` in another process. These native checks need the local game data and a graphics context. They create only staged fixtures under `private/save-check`, and verify that writer and reader process IDs differ. Fourteen cases cover partial school/library state, an active Boojum battle, lollipop growth, potion mixing, one collected reward, first-school return, village machinery/Cheshire, the completed quest's exit approach, an unreleased Dice action, rolling dice, an active summon, a delayed Ladybug ambush, an airborne acorn and a dead Ladybug. Every cached visit is also rebuilt, including Pool of Tears.



The reader compares saved fields against reconstructed controllers, NPCs and Alice, then compares continued simulation against the writer's recorded continuation. Behaviour checks cover no duplicate pickups/secret rewards/dialogue or resurrected enemies, a battle that can still finish, resumed growth/mixing, and an exit requiring both rewards. Storage checks cover atomic overwrite, previous-copy recovery, truncated files, unsupported version, different game data, bad checksum, interrupted temporary writes and failed-write preservation.



The school fixture includes a theatre-completion event due 0.25 seconds after saving. The reader resumes the same queued reinforcement/door event and matches the writer's continuation. Where the retained private v0.19/v0.20 school/battle and actual v0.21 Pool of Tears fixtures exist, the reader also loads them, resaves as version 6 and loads them again. `--event-check` independently tests saved enable flags, delayed school events at 30/60/144 Hz, pause, duplicate completions and premature quest gates. See [EVENTS.md](EVENTS.md).



Anode also tested actual F5/F9, Continue after process restart, clean-close automatic saving, and crossing the second school's authored exit into `skool1_start2`, then loading that automatic save in a fresh process. These persistence fixtures are staged; independent entrance-to-exit route regressions are documented in VALIDATION.md. Native checks were silent; no new audible playback verification is claimed.


The v0.23 suite adds four Pandemonium cases (cart, key, return and departure) for 18 writer/reader cases total, plus an actual v0.22 save migration. The flight fixture includes its pending departure dialogue.

The v0.24 suite adds return-lift, return-potion and return-shrink to the existing return entrance case: **21 separate writer/reader cases**. It also loads, upgrades, resaves and reloads an actual v0.23 return save, preserving resources and position. These are deliberately staged persistence fixtures; the normal input route is verified separately.



## Alice's saved block in legacy saves (P0.8)

Saves written before the power-up presentation, foot placement and idle-performance blocks carry none of them, so those fields load as defaults. Restoring such a save changes exactly one thing in Alice's block: the power-up presentation is initialised to the form implied by the stats (Rage, Tea or none), already fully played, so a pickup performance is never replayed and an effect that is active at load time simply continues. A field-for-field comparison of the old block with the restored one therefore fails by design; this is what stopped the unfiltered `--save-check-read` at the v0.20 school fixture after the presentation and Dice/Watch work.

Every other field must come back as saved. `character::Snapshot::migrated` documents and computes the expected block: it initialises the power-up presentation, converts a pre-clock Blade cooldown into the recovery clock, gives legacy projectiles their current kinds (with unique audio ids), drops the toy pose for an unarmed player and dismisses a dead player's summon. `character::power_form`, `weapons::Snapshot::migrated` and `weapons::ProjectileSave::{validate, migrated}` are the same helpers the restore uses, so the prediction cannot drift from the load. The windowed v0.20 case compares the migrated expectation, additionally requires that everything outside the power block is unchanged, pins the power state (initialised, not performing, the stats' form), and resaves the migrated block to confirm it loads unchanged. The save format version is untouched.

`--save-legacy-check` (headless, needs the local data) is the window-free counterpart of the retained-fixture half of `--save-check-read`. For each private fixture that exists (`private/duchess-native-reward/`, `cinema-legacy-v6/`, `pand-legacy-v4/`, `event-legacy-v1-*.json`, `dice-legacy-v2-*.json`, `return-legacy-v5/`, `ladybug-legacy-v3/`, `duchess-legacy-v7/`) it decodes the file, runs the cached-visit upgrades and the current visit's logic through `save::rebuild_headless` (the same code `Restored::build` runs), checks the player and resource assertions of the matching windowed case, checks the saved NPC cast still matches the authored placements, restores Alice's block with the real weapon-action restore on the real clips, and resaves and reloads in the current format. It also reads the cases a windowed `--save-check-write` left in `private/save-check` and confirms each rebuilds to exactly its saved logic. It skips missing fixtures, never edits them (copies live in `private/save-legacy-check`) and cannot cover NPC model loading, Alice's skin, the scene or the continued simulation; those stay in the Anode run. `cargo test` covers the same block rules with synthetic legacy blocks and, when the private fixtures exist, all fifteen retained files.

## Registered controllers: state, upgrade and limits (F2)

Format 12 exists for one reason: the `levels` map of [LEVEL_REGISTRY.md](LEVEL_REGISTRY.md). Nothing else in a save changed, and no later level branch may bump the version, the Cargo version, an existing rule key or an existing controller's state. A further bump is a new user decision (DG-6).

### A controller's saved state

Every controller writes its state under its own id and reads it through `levels::state`, so all 39 visits obey one template: a serde struct whose first field is `version: u8`, `#[serde(default)]` on every field added after release, and an exhaustive `validate` on load. That validation requires finite, bounded clocks; flags that agree with each other; counters within their maxima; and every entry-dependent flag (for example `returning`) equal to the visit being restored. `state::load` builds and validates a fresh value before the live state is replaced, and refuses `null`, a missing or unknown version and a malformed body. The file-level bounds (numbers finite and at most 1e12, strings at most 1 KiB, arrays at most 10,000 elements, 8 MiB per file, 72 cached visits) are checked by the reader before a controller sees its state.

### The generic upgrade

When a registration applies to a cached visit but its id is missing from that visit's `levels` map, the save was made by a build that served the visit without a controller. The visit is upgraded, on load and for every cached visit (`Level::needs_upgrade`), instead of refused:

1. **The previous program.** `Interactions::restore` holds the controllers back and rebuilds the visit's event program without them. That is `Interactions::load` plus every legacy controller exactly as shipped, encounters included for the six encounter maps, because it is this visit's own program minus the registry's contribution. A test proves that the rebuilt program's signature equals `Interactions::load`'s on a visit with no legacy controller, and another that a legacy controller's state (the gym) survives the upgrade. Encounters need the game data, so their composition is first exercised by a real `system/port-<id>` task.
2. **The shared runtime** and every legacy `upgrade_*` path restore against that program as they always did, so the generic upgrade runs after them.
3. **The extension.** The current program is extended from the restored runtime by `Runtime::extend_registered_from`, which is `extend_gated_from` plus two additions: writable puzzle facts the controller introduces start at their initial values, and an existing rule may gain sends that activate the controller's own receivers. The trigger rules whose condition changes are the ones the controllers gate (derived from `gate()`, plus `Upgrade::gated`). Anything else that differs from the old program fails the load.
4. **Rearm.** Script triggers the old build left pending (`reported`, which its pending path set together with `fired`) and those whose thread is in `Upgrade::rearm` get `fired = reported = inside = false`, `cooldown = 0` and `import_unhandled`.
5. **Consumed.** `import_consumed` marks the controller's rules keyed on a trigger the old build already handled, and every rule in `Upgrade::consumed`, as run, so ambushes and rewards never replay. The declared rules are consumed even when their trigger is rearmed.
6. **Respawn.** Alice restarts at the entrance when a controller rejects her saved position (`validate_player`), when her body is not clear there (free flight excepted), or when a controller declares `Respawn::Always`. Otherwise she stays where she was.
7. **The cast.** When a controller owns actors of the saved cast (`owns_npc`), the cast is regenerated from the models; owned actors start fresh and every other actor keeps its saved state, matched by authored placement.

The controllers themselves are not restored: they were loaded fresh, which is the state of a visit just entered, and the next save writes it. The upgrade is idempotent: an upgraded visit restores the ordinary way from then on. The window-free `--save-legacy-check` skips step 7 like the fortress cast regeneration, because it needs the NPC models.

Because no visit is registered yet, the upgrade is proven with a synthetic controller, synthetic BSP fixtures and synthetic saves (`levels/synthetic.rs`, `interaction/upgrade.rs`, `save/format12.rs`): pending-trigger rearm, consumed import, respawn, cast regeneration, the gated and receiver extension, the version-1 import path, and the composition with a legacy controller. The first registered visit exercises it on real data.

### Limits of a whole campaign

A synthetic 39-visit save (`save/format12.rs`, no game data) holds a controller state in every visit: 80 KB per visit, about half as large again as the average legacy visit, comes to about 3.2 MB against the 8 MiB limit. Forged files then prove each limit is enforced rather than merely respected: a 73rd cached visit or completion marker, an array of 10,001 elements and a file of 8 MiB plus one byte are refused by the reader, and the same states are refused by the writer.

### Save cases

A registered visit adds its cases through `Registration::save_cases`, named `<id>-<phase>` (mid-motion, mid-scene, active or dead enemies, boss phase). A case names the visit it is saved in, an optional `stage` that stages its state on the freshly restored visit before the writer saves it, and an optional `behavior` that `--save-check-read` runs on the visit it restored in a fresh process, after the continued simulation matched the writer's: no duplicate rewards, no dialogue replay, no resurrection, scenes resume. A visit therefore never edits `save_check.rs`. While iterating, `LOOKING_GLASS_SAVE_CASE=<id>-` selects a visit's cases; the final writer and reader runs are unfiltered, in two processes.

The reader also loads `private/duchess-legacy-v7/quick.json` as the case `legacy-v7-duchess` (a potears3 save that predates the Duchess controller) and skips it cleanly when the fixture is missing.

### Formats without a fixture

The retained fixtures cover formats 1 to 8 (`event-legacy-v1-*`, `dice-legacy-v2-*`, `ladybug-legacy-v3`, `pand-legacy-v4`, `return-legacy-v5`, `cinema-legacy-v6`, `duchess-legacy-v7`, `duchess-native-reward`). **Formats 9, 10 and 11 have no retained fixture and are untested against real files.** Their additions (breath and the temple upgrade, difficulty and power-ups, the notarget flag) default on load or are guarded by the version checks in `Store::decode`. Format 12 is written and read by the staged writer/reader cases and the unit tests.
