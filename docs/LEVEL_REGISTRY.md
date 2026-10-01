# Level-controller registry (F1)

Task F1 of [CAMPAIGN_PLAN.md](CAMPAIGN_PLAN.md) section 5. Before it, a new visit had to add arms
to the controller chains in `interaction.rs`, `viewer.rs` and `route.rs`, and to `render.rs`,
`decorations.rs`, `npc.rs`, `audio/world.rs`, `story.rs`, `main.rs`, `save_check.rs` and the
source packager. After it, **a new visit touches its own files plus one uncommented registration
line**, and the existing visits behave exactly as before: no save `VERSION`, Cargo version,
existing rule key, action, precedence or hit-ID range changed, and every existing map's event
program and snapshot is byte-identical to the F1.0 baseline.

## What a new visit touches

1. `src/levels/<map>.rs`, and optionally `src/levels/<map>/{cinema,boss,check,route,render}.rs`:
   a `static REGISTRATION: Registration` (wforest has two, `REGISTRATION` and
   `RETURN_REGISTRATION`, in one module) and a type implementing `LevelController`.
2. `src/levels/mod.rs`: uncomment the visit's own `// pub mod <map>;` line and its own
   `// &<map>::REGISTRATION,` line. Both are pre-seeded in campaign order, one per line and
   separated by blank lines, so two agents never edit the same line.
3. `Launch-<Title>.cmd`, `docs/<MAP>.md` and the usual README and VALIDATION entries.
   `tools/package_source.py` globs `Launch*.cmd`, so it needs no edit.

Nothing else: not `main.rs`, `viewer.rs`, `route.rs`, `interaction.rs`, `save_check.rs`,
`story.rs`, `render.rs`, `decorations.rs`, `npc.rs` or `audio/world.rs`. The registration line is
the only place that names the visit.

Visits owned by a typed legacy controller have no line: gvillage, pandemonium, fortress1 (both
visits), fortress2, skool1 (both visits), skool2, potears1 and potears3 (CAMPAIGN_PLAN F1.4a).
Their new gates, exits and scenes go inside the legacy module, with their own migration, until a
`system/port-<id>` task ports them. Registering one is refused at load time and by
`--registry-check`.

## The contract

`src/level.rs` defines `LevelController` (the logic) and `LevelArt` (the drawing half, which owns
GPU resources and lives in the viewer's `LevelArt` group). Every method has a no-op default that
mirrors the duck-typed API of the legacy controllers, so a controller overrides only what its
visit needs.

| Group | Methods |
| --- | --- |
| Identity, events | `id`, `facts` (derived), `initial` (writable), `gate`, `receivers`, `rules`, `output`, `event`, `dialogue_complete` |
| Simulation | `update` (script-only exits and levers, behind a persisted latch), `prompt`, `advance`, `trigger_pose` |
| World shape | `transforms`, `colliders`, `liquids` (brush-entity and moving liquids, new) |
| Scenes | `scripted`, `controlled`, `in_transport`, `scene_id`, `camera`, `fade`, `entry_story`, `prepare_story`, `sync_story`, `skip` |
| Help, recovery | `objective`, `recovery_entry` |
| Combat | `targets`, `hit`, `provoke_summon`, `combat`, `loot_sources` |
| Audio, saves | `sound_state`, `snapshot`, `restore`, `validate_player` |

`LevelArt` provides `draw`, `story_pose`, `effects`, `hud` and `handoff_pose`, and downcasts its
controller inside the module (`dyn LevelController::downcast_ref`).

Rules for a controller, checked when the event program is built:

- every rule key starts with `<id>/` and every fact key with `<id>.`, so it can never collide
  with a legacy key or another visit's;
- `advance` is a no-op when `dt <= 0`, clamps with `dt.min(0.1)`, carries riders and never
  embeds Alice;
- `restore` validates exhaustively (finite, bounded clocks, consistent flags);
- combat hit IDs stay inside the visit's own range `[target_base, target_base + 100_000)`;
  anything a controller publishes outside it is dropped.

## The registration table

`Registration { id, applies, load, art, owns_submodel, owns_npc, target_base, story_beats,
checks, save_cases, visibility }` in `src/levels/mod.rs`, with type aliases for the function
types. `pub static LEVELS: &[&Registration]` is a plain slice: no `inventory` or `linkme`
dependency, so a registration exists only if `mod.rs` names it.

- `applies(map, entry)` selects the visit. `levels::first_visit` and `levels::return_visit`
  follow the same rule as `save::visit_key`, so wforest's two registrations are keyed on the
  entry (`wforest_start2` is the return).
- `load(assets, map, name, entry)` builds the controller. `Interactions::set_entry` calls it
  after every legacy load and before the event program is configured, so its facts, receivers
  and rules join the program.
- `art` builds the drawing half in the viewer.
- `owns_submodel(map, entity)` and `owns_npc(name, model)` mark what the generic scene and cast
  must leave to the controller.
- `story_beats` are event, script file and thread identifiers (never dialogue text);
  `story::check` derives its expected counts from them.
- `checks` are `--<id>-*` flags, headless or windowed. `main.rs` resolves any flag it does not
  know from these tables, runs windowed ones inside `macroquad::Window::from_config`, and prints
  their help lines.
- `save_cases` are `<id>-<phase>` cases with the visit they are saved in and an optional staging
  function. `save_check` concatenates them with its legacy cases and adds their visits to the
  writer's fixtures.
- `visibility` fixtures run after the legacy list in `--visibility-check`.

## Reservations (F1.5)

`src/levels/reservations.rs` is Appendix F as code: for each of the 39 route visits, the hit-ID
base (`6_000_000 + route index * 100_000`), rule prefix `<id>/`, fact prefix `<id>.`, save-case
prefix `<id>-`, private path prefix `private/<id>-`, and the check flags of Appendix E-4. The id
is the map name, except `fortress1-return`, `skool1-return` and `wforest-return`.
`reservations::validate` holds a registration to them: it serves exactly its own visit, owns
exactly its own hit range, and registers only the flags, save cases and beats named for it.
Unit tests prove the ranges are disjoint from every legacy range (school2 1,000,000; shootable
switches 2,000,000; encounters 3,000,000 plus an index; the Duchess 4,000,000; Dice 4,500,000 and
5,000,000) and stay below the Ice Wand walls (800,000,000), and that no reserved key prefix
collides with a legacy one.

## Where the generic hooks sit, and why precedence is unchanged

Every hook is placed after the legacy chain it belongs to. The chains themselves are untouched.

| Site | Hook |
| --- | --- |
| `Interactions` | `event_facts`; `gate` (only when no legacy controller owns the visit); `configure_events` (receivers first, then rules and writable facts); `apply_outputs` (every output is mirrored to the controllers after the legacy handlers); `sync`, `transforms`, liquids; `advance_school` (movers, riders, riding trigger volumes); `prompt` (before the plain door prompt); `update`; script threads in `trigger_effect` (before the pending notice); `scripted`, `entry_story`, `prepare_story`, `skip_cinematic`, `sync_cinematic_story`, `completed_dialogue`; `loot_sources`; `Snapshot.levels` |
| `viewer.rs` | `LevelArt.levels`; transport flag, skip id, recovery entrance, movement ownership, the combat step, Demon Dice provocation, targets, hit dispatch, scripted camera, fade, help text, effects, HUD, fade and skip overlays |
| `route.rs` | movement ownership, the same combat step through one shared `level::step_controllers`, targets, hit dispatch, so a headless route keeps proving the viewer |
| `render.rs`, `decorations.rs`, `npc.rs` | `owns_submodel`, `owns_npc` (an owned actor is not updated, drawn, greeted, targeted or a loot source; its identity stays in the snapshot) |
| `audio/world.rs`, `story.rs`, `main.rs`, `save_check.rs` | `sound_state`, registry beats and derived counts, registered flags, help and visibility fixtures, save cases |

The hit dispatch is the one place where the order matters for correctness. The legacy exact-ID
arm (the Duchess) comes first, then each registration's exact range, then the encounters
catch-all `>= encounters::BASE`, which would otherwise swallow every registry id. A gate of a
legacy-owned visit is answered by its legacy controller or by `Always`; the registry is asked
only on visits none owns. Camera, fade, scene id and scene skip take the legacy answer as an
argument (`camera_after`, `fade_after`, `scene_id_after`, `skip_after`), so the registry is not
even consulted when a legacy controller answers.

Unit tests (`interaction/hooks.rs`, `level.rs`, `levels/`) prove those orderings with synthetic
controllers, and `--registry-check` proves the gate and skip ordering on the real data of all ten
legacy-owned visits by injecting a probe that answers every hook.

## Persistence

Registered state lives in `Snapshot.levels: BTreeMap<String, serde_json::Value>`, keyed by
controller id, with `#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`. No legacy
visit serializes the key, so every existing snapshot and event signature is byte-identical, and
the save `VERSION` is unchanged. On restore the saved keys must match the registered ids
exactly, and each controller's `restore` validates its own state. A save that predates the
visit's registration (its key is missing) takes F2's generic upgrade path instead of being
refused, and the save format is 12, the only file version that may hold `levels`, so older
executables refuse such a file with a version message.

### Saved state, the upgrade declaration and save cases (F2)

- **State template.** `levels::state` is the pattern for every controller's state: a serde
  struct with `version: u8` first, `#[serde(default)]` on every field added after release,
  and `State::validate` checking finite and bounded clocks (`state::clock`), consistent flags,
  counters and every entry-dependent flag against the current `Visit`. `snapshot()` returns
  `state::save(&self.state)` and `restore()` assigns `state::load(saved, self.visit)?`.
- **Upgrade declaration.** `LevelController::upgrade()` returns an `Upgrade`. The default
  (declare nothing) suits a controller that only adds its own rules: the gated trigger keys
  come from `gate()`, formerly pending script triggers are rearmed, and rules keyed on
  triggers the old build handled are counted as run. A controller adds `rearm` threads (a
  dialogue or sky trigger that now has world consequences), `consumed` own rule keys
  (ambushes, rewards that must never replay) and `respawn: Respawn::Always` when no old
  position can be trusted (movers, flooded floors).
- **Where the steps live.** Steps 1 to 5 in `interaction/upgrade.rs`, called from
  `Interactions::restore` at its start and end; step 6 in `save::settle` (`levels_respawn`
  decides); step 7 in `save::Level::upgrade` with `npc::Snapshot::{owned_by, adopt}`. The
  whole procedure, its limits and the tests are in [SAVES.md](SAVES.md).
- **Save cases.** `SaveCase { name, visit, stage, behavior }`: `stage` stages the state before
  the writer saves, `behavior(&mut Interactions, &mut Stats, &mut Story)` runs in
  `--save-check-read` on the visit restored in a fresh process. A visit needs no edit in
  `save_check.rs`. `levels::synthetic` is a synthetic registration, controller and map that
  the unit tests use to exercise all of this; it is never listed in `LEVELS`.

## Verification

- `--registry-check` (window-free): fresh-visit snapshot hashes of all 39 visits against the F1.0
  baseline (see below), the table validated against the reservations, and the legacy-precedence
  probe. Visits a registration serves are reported and not compared, since they legitimately
  change.
- `cargo test`: the trait defaults, hit ranges, aggregation, key namespaces, snapshot round trip,
  gate/camera/fade/skip/hit precedence, registration limits, the reservation table and the
  dynamic liquid volume.
- The Anode items of the acceptance list (`tools/test_visibility.ps1`,
  `tools/test_render_fx.ps1`, `--level-swap-check`, and the unfiltered `--save-check-write` then
  `--save-check-read` pair) run through recipe 13c on the integrated build.

### F1.0 baseline capture

`--registry-check` builds a fresh `Interactions` for every one of the 39 campaign visits (the 36
maps' first entrances plus `fortress1_start2`, `skool1_start2` and `wforest_start2`) exactly as
the viewer does on entering a level: `Interactions::load`, `set_entry` and the entry story. It
serializes `Interactions::snapshot()` and prints the SHA-256 of those bytes as one `HASH` line per
visit. The snapshot contains the event signature, which hashes every entity definition, rule,
condition and fact, so a changed rule, gate or fact changes the hash even when no state moved.
Each visit is built twice in the process and the two snapshots must be equal, so an unordered map
or a clock in a snapshot fails the check instead of poisoning later comparisons.

- `--registry-check --registry-record <file>` writes the hashes as JSON. The F1.0 baseline was
  recorded on the untouched code, before any refactor, into `private/registry-baseline/hashes.json`
  (gitignored, like every file under `private/`).
- `--registry-check --registry-baseline <file>` compares the current build with a recorded file
  and fails on any difference, naming the visit and saying whether the event signature moved.
- `--registry-check` alone compares `private/registry-baseline/hashes.json` when it exists and
  otherwise only checks determinism and says so.

The hashes were reproduced by a second, separate process before the refactor started, so the
comparison is not sensitive to hash-map iteration order. wforest's two visits have equal hashes
today because nothing distinguishes them yet.

## Not in F1

- The generic upgrade path and the save format bump (F2, since done: see above and
  [SAVES.md](SAVES.md)).
- The reviewed-adapter toolkit and scene runner (F4) and the enemy framework (F5).
- Porting a legacy controller to the registry (a `system/port-<id>` task with its own migration).
- `src/levels/mod.rs` and `src/level.rs` retain a module-level `allow(dead_code)` for optional
  contract helpers not yet exercised by full controllers. The first registration, `keep`,
  implements only its five-second arrival lift. It adds no event rules or combat targets;
  it does not establish that the rest of that visit is complete.
