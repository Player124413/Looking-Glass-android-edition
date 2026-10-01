# Campaign chain harness

This file describes the machinery that joins the headless route checks into a campaign chain. It has two halves, both described here and both in the plan's chain harness (`docs/CAMPAIGN_PLAN.md`, section 5, F3).

- F3a: one shared transition, the upgraded `Route`, and the eight legs of the opening segment.
- F3b: the whole-chain check `--campaign-route-check` (strict mode, reward provenance, checkpoints, resume, the frontier report), save and continue at every boundary and in every leg, the static exit graph `--campaign-graph-check`, the windowed save-chain companions, `tools/test_campaign_chain.ps1`, and the live status of all 39 visits below.

Everything except the two windowed companions is headless. Nothing here opens a window (apart from those two), calls a console command, warps Alice, refills a bar or calls a controller's `event()`. The one exception that refills is named and fenced: `--campaign-allow-retry` is diagnosis only and is always reported as failing.

## The transition: `campaign::arrive`

The viewer's normal exit and a headless route share one function, so they cannot drift.

- `campaign::load_visit(assets, map, entry, cached, difficulty, play_entry)` loads one visit without a window: the map with its difficulty applied, collision, the controllers, the story and the Cheshire hints. A cached visit is restored from its ledger entry; a fresh one starts its controllers and, when `play_entry` is set, plays its opening scene. A cached visit never replays it. `Route::enter` and the viewer's `enter_level` both start from it, and the viewer builds its scene on top with `Scene::from_parts`.
- `campaign::arrive(assets, stats, ledger, leaving, exit, strict) -> NextVisit` is the whole exit:
  - the destination loads at `stats.difficulty`;
  - the entrance must be clear: an obstructed entrance fails the exit with an error instead of dropping Alice into flight (the viewer used to fall back to flight);
  - only then does the ledger record the visit that was left (`completed` and `levels`), and the baseline loadout (`campaign::loadout`) is filled in. `strict` skips the baseline and keeps only authored arrival grants: the Mock Turtle's shell (`turtle_air`) on entering the temple applies in both modes (`Stats::arrival_grants`, called by `Stats::ensure_level_weapons`);
  - a failed exit changes neither the resources nor the ledger.
- The viewer calls `arrive` on copies of its resources and ledger and keeps them only after the destination's window half (scene, cast, particles, art) has loaded, so a failure anywhere still leaves the player in the level they were in.
- The pickup ledger stays keyed per map (decision DG-15): `Stats::collected` holds `<map>:<entity>` keys, so a return visit does not respawn what the first visit's pickups gave, and enemy drops are keyed per visit (`drop:<visit key>:<id>`). The chain reports how many keys of each kind the resources hold at the last entrance.
- `campaign::visits()` lists the 39 visit keys in story order and `campaign::visit_index(map, entry)` finds one, both aliased exactly like a save's `save::visit_key`: a named first entrance shares its map's first visit, and only the return entrance of fortress1, skool1 and wforest is `$return`. Unit tests hold all 39 keys unique and every one of the eight opening exits (Appendix A rows 01 to 08) to index `i + 1`.

## `Route`

- `Route::new(assets, map, entry)` is `Route::enter(assets, map, entry, Stats::for_level(map, entry), Difficulty::Normal)`; every recorded check uses it.
- `Route::enter(assets, map, entry, carried, difficulty)` starts a visit carrying resources. The difficulty is a real input: it filters the map's enemies, pickups and placed guards through the authored inhibit bits, and it scales the damage Alice takes and deals. `--route-difficulty-check` proves it on all 39 visits (see below).
- `Route::depart(self, assets, strict)` takes the exit the route ended on and enters the next visit through `campaign::arrive`, carrying `Stats` and the ledger.
- `Route::level()` is the visit as the ledger stores it. A headless route has no cast (`Npcs` needs textures), so its cast snapshot is empty. `stats`, `player`, `interactions`, `story`, `hints` and `ledger` are public for the checkpoints and the save legs.
- `stop_at_exit`: once set, the route freezes the moment it takes its exit. Every later tick does nothing and the walking, waiting and fighting helpers return, so a leg ends at the exit exactly where the viewer would have left the level. It is off for the recorded single-visit checks, whose numbers include the ticks after the exit.
- `Route::metrics()` returns `Metrics`: ticks, jumps, Blade throws, swings, cards, damage, teleports, Sanity and Will in and out, pickups and loot collected during the leg (drops are the `drop:` keys of `Stats::collected`), and ticks in slime or lava.
- `Route::assert_clean(teleports, slime_allowance)` holds the shared route assertions (R5): no tick in lava, no more slime ticks than the leg documents, no tick beyond the level (the viewer's "lost, retry" case, so no recovery), exactly the authored teleports, a single transition, and Alice alive throughout.
- `Route::navigate(goal)` plans a walk with a search over 24-tick steps (nine headings, with or without a jump) until a standing state lies within 22 units of the goal. A goal on a narrow ledge can fall between two steps, so that no searched state stands inside the radius although Alice can walk there (fortress1's ledge at (-5, -2430, 32) does this from the positions Easy and Hard bring Alice to). When the search finds nothing at 22 units, a second search accepts 40 and prints `Widened the tolerance for <goal>`; a goal that plans at 22 keeps exactly its plan, so no recorded route changed. `Route::plan` is the search itself, unit-tested on a walled room.
- The route loads `cheshire::Hints` and observes and updates them like the viewer, keeps the viewer's visual clocks, and triggers `dice_cat` on a new Demon Dice.
- Placed club guards are the headless stand-in for `Npcs` (`npc::placed_guards`, `npc::guard_timing`): `route::guards_of` returns none for a visit whose controller owns its cast (Pandemonium, the Duchess, registered visits), as `Npcs::update` skips those. Of the eight opening visits only skool2 has any (six).
- School two runs on the shared route: `School2::update` with the viewer's gating (the world clock, frozen while the Watch stops time; the Boojums ignore Alice while she is invisible), Alice's quest items following the controller every tick, the `dice_cat` event, hits delivered whole (`School2::hit_attack`, `Guard::hit`, with the weapon's knockback and damage kind) and the exit going through story deferral. `school2_route::complete_from(assets, stats)` starts it from carried resources.

F3b adds the save-and-continue part (`src/route/resume.rs`):

- `Route::checkpoint()` is one saved moment: the resources, Alice's body, the ledger of finished visits, the visit being played as the save format stores it (`Level`: controllers, story, hints, visual clocks) and the route state a save keeps outside the controllers (the placed guards, Alice's weapon action, the projectiles in flight, the counters). A `Checkpoint` serializes to JSON.
- `Route::resume(assets, &checkpoint)` rebuilds a route from one through `campaign::load_visit` with the ledger-entry restore the viewer's Continue uses, puts Alice back exactly where she was and restores the route state. It is a load, not a warp: it exists for the save/continue probe and for `--campaign-from`, and nothing in a leg calls it.
- `Route::enter_carrying(assets, map, entry, stats, ledger, difficulty)` is `enter` with a ledger, what `arrive` hands over.
- `Route::freeze_at(tick)`, `frozen()` and `thaw()`: the route freezes at the first tick at or after `tick`, like the latch at an exit, so a leg can be driven again to a chosen moment.
- `Route::state()` and `state_hash()` hold everything the probe compares; `Route::quiet()` clears the route bookkeeping that is not game state (the fight tracker's velocity smoothing, the transient click and aim requests, the tactics that use them) so two routes can be compared.
- `run_identical(a, b, ticks)` feeds two routes the same `probe_controls(t)` input (short walks in every direction, pauses, a jump and a use, repeating every 360 ticks) and requires the same result at every tick (or the same error at the same tick), and equal states every 60 ticks and at the end.

## Legs

`campaign_route::LEGS` holds one `Leg` per visit 1 to 8, aligned with `campaign::route()`: the map and entrance, the exit the leg must end on, the authored teleports, the slime allowance, the `drive` (each opening check body is a `pub fn drive(r: &mut Route)`) and an optional variant. `Leg::start` builds the route latched at its exit, `Leg::body(optional)` names the body a run drives, and `Leg::run` drives it and asserts the exit and `assert_clean`.

| Visit | Leg | Exit | Authored teleports | Slime allowance |
| --- | --- | --- | --- | --- |
| 01 | gvillage | pandemonium `player_start` | 0 | 0 |
| 02 | pandemonium | fortress1 `fortress1_start1` | 1 | 460 |
| 03 | fortress1 | fortress2 | 1 | 0 |
| 04 | fortress2 | fortress1 `fortress1_start2` | 3 | 0 |
| 05 | fortress1 return | skool1 `skool1_start1` | 0 | 0 |
| 06 | skool1 (secret room as the optional variant) | skool2 `skool2_start1` | 0 | 0 |
| 07 | skool2 | skool1 `skool1_start2` | 2 | 0 |
| 08 | skool1 return | potears1 `potears1_start1` | 0 | 0 |

The recorded Pandemonium route wades through the slime pit at the foot of the rope and again after the return portal: 460 ticks in slime, watched and skipped alike. The leg carries that figure as its allowance, and it is to shrink to zero when the driver detours around the pit.

## The chain: `--campaign-route-check`

```
--campaign-route-check [--campaign-from <n|map[$entry]>] [--campaign-to <n|map[$entry]>]
    [--difficulty easy|normal|hard|nightmare] [--campaign-strict] [--campaign-skip-cinematics]
    [--campaign-allow-retry] [--campaign-expect frontier=<n|map[$entry]>|no-checkpoint] [--campaign-log <file>]
```

The chain runs the legs of `LEGS` back to back on one `Stats` and one ledger, each exit handed over by `Route::depart` (`campaign::arrive`), and reports how far it gets.

- Visits are numbered from 1, as printed (`visit 04 fortress2$first`). `map$entrance` is normalized like a save's visit (`fortress1$fortress1_start2` is the fortress return, `skool1$skool1_start1` the first school visit); a bare map is its first visit, and `map$first` and `map$return` are the visit keys. `--campaign-from` defaults to 1 (New Game state) and `--campaign-to` is inclusive.
- `--campaign-strict` skips the `campaign::loadout` baseline fill of every exit and starts from no chapter loadout: every toy must come from its authored source (see the provenance report), and a missing required grant fails the leg. `--campaign-from` past visit 1 then needs a checkpoint. Without `--campaign-strict` the chain fills the baseline like the viewer does, prints `BASELINE FILL entering visit NN: <toys>` for what the fill added, and may start from a visit's chapter loadout, labelled as the staged fixture it is.
- `--campaign-skip-cinematics` skips every scene the way holding Enter does (`Route.skip_cinematics`). It is also how a watched and a skipped leg are compared (equal exits, different clocks).
- `--campaign-allow-retry` restores Alice's Sanity and Will and drives a failed leg again, twice at most (`RETRY visit NN ...`; a missing grant is not retried, because refilling cannot supply one). It exists to see how far the chain would get if the drivers survived, so it is diagnosis only: `report.json` says `diagnosis` whatever happened, the run ends in an error unless `--campaign-expect` pins its frontier (below), and the checkpoints such a run writes are marked and refused by every other run.
- `--campaign-expect frontier=<n|map[$entry]>` or `--campaign-expect no-checkpoint` pins an honest failure, so that the check of a failure the plan tells the harness to report (and not to fix) exits 0 while that failure is what happens and exits 1 the day it is not. `frontier=<visit>` holds when the chain stops short of its range with its frontier at that visit (a `--campaign-to` target that is the failing leg included, and a `--campaign-allow-retry` run included); it fails when the chain goes further, stops earlier or completes its range. `no-checkpoint` holds when a strict chain refuses to start because the visit it was asked to resume from has no checkpoint (any other refusal, and a chain that started, fail it). The run prints and writes exactly what it would without the pin (the `FRONTIER` line, the failing leg's message and log, the report, which also records the expectation and whether it was met), then prints `EXPECTED FRONTIER ...` or `REFUSED (expected ...)`. An expectation asserts a failure: it never turns a chain that completed its range, or a diagnosis run, into a pass.
- `--campaign-log <file>` only records where the caller redirects the log, in `report.json` and nowhere else.

### What it prints

| Line | Meaning |
| --- | --- |
| `PASS save/continue at the entrance of visit NN ...` | The entrance was serialized, a second route rebuilt from the text and a third entered fresh; the chain's own route equals the fresh one, and the other two ran 600 ticks of identical input and stayed equal. It also prints the ledger's visits and bytes. |
| `PASS arrival autosave of visit NN ...` | The entrance as a real `save::Game` (Auto slot), written by the save `Store`, read back and rebuilt window-free. |
| `REWARD <visit>: <toy> xN <status>` | Each milestone a visit grants, settled when its leg passes. |
| `PASS visit NN <visit>: <metrics>` | The leg passed its assertions; the metrics are ticks, jumps, throws, swings, cards, combat damage, Sanity and Will in and out, pickups, loot and authored teleports (and slime ticks). |
| `PASS save/continue in the middle of visit NN ...` | The leg was driven again to its midpoint (half its ticks, at most 4,000); a checkpoint there rebuilt a route that ran 600 ticks of identical input beside it. |
| `PASS pre-exit manual save of visit NN ...` | The leg driven again to one second (120 ticks) before its exit, as a real `Game` in a manual slot, written, read back and rebuilt. |
| `FAIL <message>` | The first failing leg, with its error chain: a route that died, no physics-input route, a wrong exit, an assertion, or a missing required grant. |
| `VARIANT <name>: ...` | A variant run (below). |
| `REWARD PROVENANCE ...` | The report of all twelve milestones, the temple's arrival grant and the pickup-ledger key counts. |
| `FRONTIER <map$entry> (<reason>: <message>)` | The first visit the chain could not complete. For a visit without a driver, or one the chain was stopped at, it names the start thread of the entrance and says whether anything starts on entry. |

### Exit code

Only `--campaign-to` gates a chain. With it, the run fails unless that leg passed (`PASS campaign chain reached visit NN`), which is what recipe 13d asks for. Without it the chain runs as far as it can, prints the frontier and ends with `REPORT campaign chain stopped at the frontier; exit 0 because no --campaign-to gates this run`, exactly as `--campaign-legs-check` reports its chained runs without gating. Independently of that, a save/continue mismatch, a broken save limit, an unreadable checkpoint or a failed real-data assertion always fails the run, and `--campaign-allow-retry` always fails it.

The failures that the plan asks the harness to report and not to fix are therefore checked with a pin, so that the check's own exit code is 0 exactly while the documented failure holds. Visits 4 and 6 are no longer among them: Beyond the Wall and school one pass chained on the resources the visits before leave Alice with (see "What the recorded drivers assume" below, `docs/BEYOND.md` and `docs/SCHOOL.md`), so `tools/test_campaign_chain.ps1 -Pinned` first runs the chain gated on visit 4 (`--campaign-strict --campaign-to fortress2$fortress2_start1`, no pin: it exits 1 the day visit 4 stops passing) and the chain gated on visit 6 (`--campaign-to skool1$skool1_start1`) at Normal, and the same gates at Easy and at Hard in scratch working directories, and then the three pins:

| Run | Pin | What it proves today (Normal) |
| --- | --- | --- |
| `--campaign-strict --campaign-from 4` | `--campaign-expect frontier=skool2$skool2_start1` | The strict chain resumed from the visit 4 checkpoint passes Beyond the Wall, the fortress return and school one (with its secret variant) and stops at school two: `FAIL Visit 07 skool2$first: Route died at (1964.28, -3136.97, 448.03)`, then `FRONTIER skool2$skool2_start1`. Without the pin the same run exits 0, because no `--campaign-to` gates it; the pin is what makes the frontier a regression check. |
| `--campaign-strict --campaign-from 4 --campaign-allow-retry` | `--campaign-expect frontier=skool2$skool2_start1` | The diagnosis: Beyond the Wall, the fortress return and school one pass without needing the refill, skool2 dies in its first fights (`RETRY visit 07 ...`), is driven again from the refill and then fails on the missing required Demon Dice grant (a grant is never retried). The report says `diagnosis`. Without the pin it exits 1, always. |
| `--campaign-strict --campaign-from 6`, no checkpoint | `--campaign-expect no-checkpoint` | A strict chain never starts from a chapter loadout: it refuses, and writes a report whose `result` is `refused`. Without the pin it exits 1. |

A pin is a regression check on the frontier, not a claim that the chain works: when a driver change moves the frontier, the pinned run fails until the pin (the `$pins` table of the script, and this table) is moved with it. The changes that made visit 4 and then visit 6 pass did exactly that.

### Checkpoints, resume and `report.json`

- `private/campaign-chain/NN-<map>-<first|return>.json` (NN is the visit number, `first` or `return` the visit key's kind) holds the entrance of that visit: the resources, Alice, the ledger and the visit's snapshots as one `Checkpoint`, its state hash, the options it was written under, and, once its leg ran, what the leg did (`pass` with its metrics, the exit's state hash, the midpoint probe and the pre-exit checkpoint, or `fail` with its message). A file is 90 to 210 KB.
- `--campaign-from N` resumes from that file when it exists. The rebuilt route must reproduce the state hash the file was written with. A checkpoint is proof only for a run it could have come from: the same difficulty; for a strict run, one that was strict, started from New Game state and restored nothing (a staged chapter loadout or a `--campaign-allow-retry` run is refused). Without a file a viewer-style chain starts from the visit's chapter loadout (staged), and a strict chain stops with a message.
- `private/campaign-chain/report.json` is rewritten by every run, including one that refuses to start (`result: "refused"` with the message, so the report of an earlier run never stands in for it). `result` is `reached`, `frontier`, `diagnosis` (a `--campaign-allow-retry` run, never a pass) or `refused`, and a pinned run records its `expectation` and whether it was `met`. A complete report holds the run: the options, how it started, one entry per leg (status, metrics, entrance and exit state hashes, both probes, retries), the frontier (visit, spelling, reason, kind `route`, `grant`, `arrival`, `driver`, `target` or `complete`, message, start thread), the provenance rows, the temple grant, the pickup-ledger key counts, the variants and the log path. The state hashes make two runs comparable: `tools/test_campaign_chain.ps1 -Determinism` runs the chain twice and requires every leg's entrance and exit hash to be equal.

### Variants

`campaign_chain::VARIANTS` lists named ways through the chain. The base variant is the chain proper. Every other variant names the legs whose optional body it drives, and runs from the entrance of its first such leg (rebuilt from the base run's checkpoint) through its last, reporting its own legs. Today there is one: `skool1-secret` (school one's secret room, the optional Darkened Looking Glass shelf). The wforest return's collect-or-skip choice (the optional Blunderbuss, v28) is added the same way when its visit has a driver: a variant for collecting it and the base for skipping it. A variant whose branch the base chain never passed prints `VARIANT <name>: not run: ...`.

## Strict mode and reward provenance (T9)

A strict chain grants nothing from the campaign baseline, so every toy Alice holds has to come from where the level or the story gives it. After each leg the chain settles the milestones that leg's visit grants (`campaign::rewards()`): the pickup is found in the map's entities at run time (never named in the code), and the leg passes only if Alice holds the toy and has collected one of those pickups.

| Visit | Toy (copies) | Source | Required |
| --- | --- | --- | --- |
| gvillage | Vorpal Blade (1) | placed pickup | yes |
| pandemonium | Cards (1) | placed pickup | yes |
| skool1 | Croquet Mallet (1) | placed pickup | yes |
| skool2 | Demon Dice (1) | placed pickup | yes |
| potears3 | Jackbomb (1) | placed pickup | yes |
| garden4 | Ice Wand (1) | placed pickup | yes |
| centipede1 | Demon Dice (2) | placed pickup | yes |
| rchess1 | Demon Dice (3) | placed pickup | yes |
| funhouse | Jacks (1) | placed pickup | yes |
| hatter2 | Deadtime Watch (1) | placed pickup | yes |
| jlair2 | Eye Staff (1) | scripted grant, no adapter yet | yes |
| wforest return | Blunderbuss (1) | placed pickup behind the secret door | optional |
| utemple | Mock Turtle shell | authored arrival grant | kept in strict mode (`utemple arrival grant`) |

- A required toy Alice does not hold is `MISSING required grant` and fails a strict leg; a toy she holds without an authored pickup was filled in by the baseline (`BASELINE FILL`), which fails a strict chain and is reported by a viewer-style one. Optional rewards are reported as skipped, never required. A viewer-style chain does not fail on a grant the leg did not make, because the fill supplies it on exit; it says so in the report.
- The temple's shell is the one authored grant that arrives by entering a level; it applies in both modes, and the report line `utemple arrival grant` records whether the chain held it on entering the temple.
- DG-5 (recorded in the plan): the Blunderbuss leaves the normal-exit fill-in at M6 and stays in chapter-start loadouts. Strict mode ignores the fill either way, so the chain already treats it as optional.
- DG-15: the pickup ledger stays keyed per map, and the report says how many pickup and drop keys the resources remember.

## Save and continue

Every chain run proves that a route which is saved and continued behaves exactly like the route that was saved.

- At each boundary (the entrance of every visit the chain reaches, the frontier included) the entrance is serialized, read back from its text, and a route is rebuilt from it with `Route::resume`; another is entered fresh; the chain's own route must equal the fresh one and the two probes run 600 ticks of identical input, staying equal (a tick that ends both in the same error at the same tick counts as equal).
- In every leg that passes, the leg is driven again from its entrance to its midpoint (half its ticks, at most 4,000), a checkpoint is taken there, a route is rebuilt from its text, and it runs 600 ticks beside the route it was taken from. The drivers are deterministic, so the second drive reaches the same moment.
- The ledger limits are asserted at every checkpoint with the save's own validator: 8 MiB, 72 cached visits, 10,000 elements per array (and the save's number and string bounds). A unit test builds a ledger of all 39 visits and one of 79.
- The entrance and the pre-exit moment are also written as real `save::Game` values through the save `Store` (Auto slot and slot 1), read back and rebuilt with `save::rebuild_headless`, the window-free part of the viewer's Continue; the rebuilt controllers, story, hints, resources and player must equal what was saved. Alice's character block is a stand-in there (the smallest value her deserializer accepts) and the cast is empty; the windowed pair below uses the real ones.

What a checkpoint does not carry: the fight tracker's smoothing, `aim_at`, the click request and the metrics, which the probe clears on both routes; and the cast, which a headless route does not have (the placed club guards are saved).

### Windowed companions (Anode only)

`--campaign-save-chain-write` and `--campaign-save-chain-read` open a window, so they run only in the Anode seat (recipe 13c), as two separate processes, after a headless chain has left its checkpoints. The writer turns each visit's checkpoint into real games with the real Alice and the real cast of every visit in the ledger: an Auto slot with the entrance (the arrival autosave the viewer writes after an exit) and, when the leg passed, slot 1 with the state one second before that leg's exit, into `private/campaign-save-chain/<NN>-<map>-<kind>/`, and checks each through `Restored::build`. The reader, a fresh process (the writer's pid is recorded and must differ), restores every slot through `Restored::build` and requires the restored visit, the cached visits of the ledger, the resources and Alice's block to equal what was saved, and a save of what it loaded to read back equal. The writer turns every checkpoint in the directory into a case and labels its origin in the log (strict chain, staged chapter loadout, `--campaign-allow-retry` diagnosis, viewer-style chain), because a directory can hold files that different runs left (`tools/test_campaign_chain.ps1 -InsideAnodeSeat` removes the checkpoints of earlier runs and runs the strict chain first, so the directory then holds only that chain's files; after any other run, read the labels before counting a case as chain evidence). The fortress1 and fortress2 cases are the native fixtures those visits lacked. Neither was run in the stage that added them; the plan lists them for recipe 13c.

## The exit graph: `--campaign-graph-check`

Headless, all 39 visits. It compiles Appendix A into an identifier-only table and holds every row against the data at run time: the exit volumes in the map and where they lead, the verbs the map's scripts apply to them (enable, disable, fire, bind: identifiers only, read through the asset reader and used for verification, never at play time), the level changes the scripts request and the functions they sit in (comments are ignored, which is how the hedge maze's dead level change stays dead), the scene triggers that start the exit scenes, and what the engine would fire on contact in a freshly entered visit (`Interactions::exit_volumes`, the same question the trigger loop asks). It fails on an unreviewed exit volume, a row the data no longer supports, a live volume that leads anywhere but the next visit (unless it is an authored-closed volume, which list 3 flags), a reviewed adapter that leads elsewhere, or a finale that requests a map. Every enabled exit normalizes to `ROUTE[i+1]`; qlair requests the ending film instead (`video/ending.roq` exists, no script requests a map, the ending function exists).

The reviewed script exits that exist in Rust today are the pandemonium exit latch, the school-one recipe-book exit, the observatory exit at the end of the return phase and the Duchess's deferred exit; no registry exit specification exists yet (F4).

The three lists (each visit is printed with its reason):

1. **Script-only exits without an adapter** (9): utemple (12), centipede1 (17), centipede2 (18, the level change in the growth scene), rchess1 (22), funhouse (23), hatter2 (25), jlair2 (27), the wforest return (28) and qlair (39, the ending film).
2. **BSP exits that the data fires only from a scene, to be scene-gated** (4): potears1 #115 and potears2 #68 (fired by their end scenes), and the fallback volumes beside the scripted exits of facade #67 and keep #71.
3. **Fresh-entry enablement**: the volumes the data closes at load (or never lets Alice touch), against what the engine leaves live. Each is `LIVE at fresh entry: a bypass until it is gated at load` or `gated at load`. Today the 12 listed are: fortress2 #43, skool2 #29 and #75 (gated by their controllers), and, live, potears2 #68 and the scene trigger #50, utemple #30, garden4 #6, centipede2 #43 and the scene trigger #34, wchess2 #32, hatter1 #121 and the wforest return's #126.

Lists 1 and 2 together name exactly {9, 10, 12, 17, 18, 22, 23, 25, 27, 28, 37, 38, 39}, and list 3 flags all eight volumes the plan names: centipede2 #43, hatter1 #121, wchess2 #32, garden4 #6, utemple #30, potears2 #50 and #68, and wforest #126 on the return visit. The check fails if the union changes: a visit that gets a reviewed adapter (or, later, a registered exit specification) leaves list 1 or 2, and `EXPECTED` in `src/campaign_chain/graph.rs` is updated in the same change. A volume that a controller gates at load only flips from `LIVE` to `gated at load` in list 3.

## `tools/test_campaign_chain.ps1`

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/test_campaign_chain.ps1 -SkipBuild -Executable <exe> -Data <abs data> [-Difficulty normal] [-To <visit>] [-Expect frontier=<visit>] [-Pinned] [-Determinism] [-InsideAnodeSeat]
```

It first removes the checkpoint files (`NN-<map>-<kind>.json`) of earlier runs from `private/campaign-chain`, so that the directory, and the native save chain that reads it, hold only this run's chain. It then runs `--campaign-graph-check`, the strict chain and the strict chain with `--campaign-skip-cinematics` (and, with `-Determinism`, the strict chain again with the per-leg state hashes compared; `-Expect` passes `--campaign-expect` to all of them), then, with `-Pinned` (Normal only), the chain gated on visit 4 (a plain gate, no pin: Beyond the Wall passes), the same gate at Easy and at Hard (each from New Game state in its own scratch working directory, so that its checkpoints never replace the Normal proof checkpoints) and the three pinned failures of the table above: the strict chain resumed from the visit 4 checkpoint in the chain directory (its frontier is school one), the diagnosis from visit 4 in a scratch working directory that holds a copy of the visit 4 checkpoint (so a diagnosis never writes next to the proof checkpoints), and the refusal in an empty scratch working directory (so it cannot find a checkpoint whatever earlier runs left); then, only with `-InsideAnodeSeat`, `--campaign-save-chain-write` and `--campaign-save-chain-read` as separate processes. Without `-InsideAnodeSeat` it never opens a window and records that the native modes were skipped. `-InsideAnodeSeat` is passed by the recipe-13c agent through `seat_exec` and by nobody on the host. It writes `private/campaign-chain/run-summary.json` with the executable's SHA-256 (re-checked after every step), every step's arguments, exit code and log, the determinism verdict and the native steps or why they were skipped.

## Live status

The strict chain from New Game state on Normal (`--campaign-route-check --campaign-strict`, executable SHA-256 in `docs/VALIDATION.md`). "Alone" is the leg run by `--campaign-legs-check` from its own baseline (F3a).

| # | Visit | Driver | Strict chain, Normal | Exit (graph check) |
| --- | --- | --- | --- | --- |
| 01 | gvillage | `village_route` | PASS, 12,969 ticks, Sanity 100 | BSP #41, ungated |
| 02 | pandemonium | `pandemonium_route` | PASS, 19,983 ticks, Sanity 100 to 57, 460 ticks in slime | reviewed adapter |
| 03 | fortress1 | `fortress_route` | PASS, 15,300 ticks, Sanity 57 | BSP #41, first visit only |
| 04 | fortress2 | `beyond_route` | PASS, 27,951 ticks, Sanity 57 to 100, Will 22 to 5, 3 pickups, 3 drops (Easy: PASS, 17,077 ticks; Hard: PASS, 36,861 ticks, Sanity 20 to 91, Will 1 to 2) | BSP #43, gated (`beyond.last_open`) |
| 05 | fortress1 return | `fortress_route` | PASS chained, 7,712 ticks, 22 combat damage, Sanity 100, Will 5 to 28 (Easy: 6,123 ticks; Hard: 7,471 ticks, 27 combat damage; alone: PASS, 6,990 ticks) | BSP #40, return only |
| 06 | skool1 | `school_route` | PASS chained, 22,741 ticks, 18 jumps, 15 throws, 2 swings, 112 cards, 26 combat damage, Sanity 100 to 77, Will 28 to 6, 1 pickup (the Croquet Mallet, `skool1:78`), 11 drops; secret variant PASS, 20,461 ticks (Easy: PASS, 17,131 ticks, Sanity 100 to 61; Hard: PASS, 21,601 ticks, Sanity 87 to 58; alone at Normal: PASS, 20,611 ticks) | reviewed adapter; BSP #77 below the floor |
| 07 | skool2 | `school2_route` | **FRONTIER**: `Route died at (1964.28, -3136.97, 448.03)` (arriving from school one with 77 Sanity and 6 Will at Normal; alone: PASS, 25,785 ticks; after a diagnosis refill it stops on the missing Demon Dice grant) | BSP #29 gated by the quest; #75 off |
| 08 | skool1 return | `school_return_route` | not reached (alone: PASS, 12,046 ticks) | reviewed adapter; BSP #92 gated |
| 09 | potears1 | legacy Pool | `--potears1-route-check` / `--potears1-render-check`: ordinary Normal arrival, bank climb, four leaves, encounters and rope | Scene-gated one-shot exit into `potears2$potears2_start1`; exact resource carry checked; see [Pool](POOL.md) |
| 10 | potears2 | none | no driver | BSP #68 fired by the end scene (list 2), #50 closed at load (list 3), live |
| 11 | potears3 | none | no driver | reviewed adapter (Duchess); BSP #36 gated |
| 12 | utemple | none | no driver | script-only (list 1); #30 closed at load (list 3), live |
| 13 | garden1 | none | no driver | BSP #131 |
| 14 | garden2 | windowed `--garden2-route-check` | Standalone Normal watched/skipped routes and saved continuations PASS; not yet integrated into this strict chain | BSP #162 |
| 15 | garden3 | none | no driver | BSP #74 |
| 16 | garden4 | none | no driver | BSP #6 closed at load (list 3), live |
| 17 | centipede1 | none | no driver | script-only (list 1) |
| 18 | centipede2 | none | no driver | script-only (list 1); BSP #43 permanently off (list 3), live |
| 19 | wforest | none | no driver | BSP #126, behind the chess gate |
| 20 | wchess1 | none | no driver | BSP #103 |
| 21 | wchess2 | none | no driver | BSP #32 closed at load (list 3), live |
| 22 | rchess1 | none | no driver | script-only (list 1) |
| 23 | funhouse | none | no driver | script-only (list 1) |
| 24 | hatter1 | none | no driver | BSP #121 closed at load (list 3), live |
| 25 | hatter2 | none | no driver | script-only (list 1) |
| 26 | jlair1 | none | no driver | BSP #106 |
| 27 | jlair2 | none | no driver | script-only (list 1) |
| 28 | wforest return | none | no driver | script-only (list 1); BSP #126 leads back to wchess1 (list 3), live |
| 29 | hedge1 | none | no driver | BSP #29 |
| 30 | tower1 | none | no driver | BSP #22 |
| 31 | hedge2 | none | no driver | BSP #52 |
| 32 | tower2 | standalone native Normal, watched/skipped | `--tower2-route-check`; ten disk continuations per run; live hazards/combat | three flushers, final shaft dive and real entry into hedge3; strict campaign chain pending |
| 33 | hedge3 | native Normal, live checkpoint continuations | `--hedge3-route-check`; 76 disk continuations; live machinery/combat | BSP #101 and collision-clear `tower3$tower3_start1` with carried resources; full campaign chain pending |
| 34 | tower3 | none | no driver | BSP #17 |
| 35 | grounds1 | none | no driver | BSP #33 |
| 36 | grounds2 | none | no driver | BSP #7 |
| 37 | facade | none | no driver | script exit plus fallback BSP #67 (list 2) |
| 38 | keep | none | standalone Keep route checks; campaign-chain driver pending | saved puzzle and Cheshire completion gate both script exit and BSP #71; see [KEEP](KEEP.md) |
| 39 | qlair | none | no driver | ending film (list 1) |

The frontier is `skool2$skool2_start1`. Beyond the Wall used to stop the chain (it entered with 57 Sanity and 22 Will after Pandemonium's slime and 26 Cards in the first Fortress visit, and the route recorded from a full bar died at the same spot on every run); its driver now reads the machinery's clocks and earns the resources it spends (`docs/BEYOND.md`), and the chain passes it at Normal, Easy and Hard, with and without skipped cinematics. The fortress return passes on what Beyond the Wall leaves (Sanity 100, Will 5). School one, entered with that Will, used to stall: a Diamond guard's bolt knocked Alice off the library passage's narrow walkway onto a ledge, the shared fight helper found no safe heading there and stood still until its 90-second limit, and then a strict chain stopped again on the Croquet Mallet the route never went to collect. Both were route-driver changes (`docs/COMBAT.md`, `docs/SCHOOL.md`) and school one now passes at every difficulty. School two, entered with 6 Will, is the same recorded-from-a-full-bar problem one visit later.

### Chain metrics

Executable SHA-256 in `docs/VALIDATION.md`; every number is deterministic (two processes give equal state hashes at every boundary and exit).

| Run | Visit 01 gvillage | 02 pandemonium | 03 fortress1 | 04 fortress2 |
| --- | --- | --- | --- | --- |
| Normal, strict | 12,969 ticks, 6 jumps, 2 throws, Sanity 100, 1 pickup | 19,983 ticks, 6 jumps, 11 throws, 14 combat damage, Sanity 100 to 57, 1 pickup, 1 loot, 1 teleport, 460 slime | 15,300 ticks, 8 jumps, 1 throw, 26 cards, Sanity 57, Will 100 to 22, 1 teleport | 27,951 ticks, 21 jumps, 9 throws, 1 swing, 52 cards, 19 combat damage, Sanity 57 to 100, Will 22 to 5, 3 pickups, 3 loot, 3 teleports |
| Normal, strict, cinematics skipped | 8,962 ticks | 17,517 ticks, Sanity 55 | 5,399 ticks, Sanity 55 | 27,951 ticks, Sanity 55 to 100 (the same route) |
| Easy, strict | 12,969 ticks, 1 throw | 20,013 ticks, 8 throws, Sanity 100 to 78 | 14,991 ticks, 9 jumps, 14 cards, Sanity 78, Will 100 to 65 | 17,077 ticks, 16 jumps, 38 cards, 0 combat damage, Sanity 78 to 100, Will 65 to 97, 3 pickups, 2 loot, 3 teleports |
| Easy, strict, cinematics skipped | 8,962 ticks | 17,552 ticks, Sanity 78 | 5,090 ticks, Sanity 78 | 17,077 ticks (the same route) |
| Hard, strict | 12,969 ticks | 19,983 ticks, 28 combat damage, Sanity 100 to 20 | 15,878 ticks, 11 jumps, 33 cards, Sanity 20, Will 100 to 1 | 36,861 ticks, 22 jumps, 18 throws, 2 swings, 48 cards, 18 combat damage, Sanity 20 to 91, Will 1 to 2, 3 pickups, 4 loot, 3 teleports |
| Hard, strict, cinematics skipped | 8,962 ticks | 17,507 ticks, Sanity 18 | 5,977 ticks, Sanity 18 | 36,861 ticks, Sanity 18 to 89 (the same route) |

| Run | Visit 05 fortress1 return | 06 skool1 (Croquet Mallet collected from `skool1:78`) |
| --- | --- | --- |
| Normal, strict | 7,712 ticks, 2 jumps, 9 throws, 11 cards, 22 combat damage, Sanity 100, Will 5 to 28, 2 loot | 22,741 ticks, 18 jumps, 15 throws, 2 swings, 112 cards, 26 combat damage, Sanity 100 to 77, Will 28 to 6, 1 pickup, 11 loot; secret variant 20,461 ticks, 47 combat damage, Sanity 77, Will 28 to 0 |
| Normal, strict, cinematics skipped | 7,226 ticks, 13 combat damage, Will 5 to 25 | 23,011 ticks, 16 throws, 3 swings, 116 cards, 45 combat damage, Sanity 100 to 48, Will 25 to 1; secret variant 21,721 ticks, Sanity 77 |
| Easy, strict | 6,123 ticks, 3 throws, 43 cards, 9 combat damage, Sanity 100, Will 97 to 29 | 17,131 ticks, 16 jumps, 3 throws, 1 swing, 78 cards, 14 combat damage, Sanity 100 to 61, Will 29 to 40; secret variant 19,951 ticks, Sanity 83 |
| Hard, strict | 7,471 ticks, 27 combat damage, Sanity 94 to 87, Will 2 to 25 | 21,601 ticks, 20 jumps, 17 throws, 6 swings, 106 cards, 99 combat damage, Sanity 87 to 58, Will 25 to 1; secret variant 24,181 ticks, Sanity 68 |

Save/continue at every boundary and midpoint passed in all of these (600 ticks of identical input each); the ledger held 0, 1, 2, 3, 4, 5 and 6 visits at the seven entrances the open Normal chain reaches, 28 to 226,132 bytes. The chain now goes on past visit 4: resumed from its checkpoint (`--campaign-from 4`), the fortress return passes on what Beyond the Wall leaves (Normal: 7,712 ticks, 9 throws, 11 cards, 22 combat damage, Sanity 100, Will 5 to 28; Easy: 6,123 ticks, Sanity 100, Will 97 to 29; Hard: 7,471 ticks, 27 combat damage, Sanity 91 to 87, Will 2 to 25), and school one then passes at every difficulty (above), so the chain stops in school two (Normal: `Route died at (1964.28, -3136.97, 448.03)`; at Easy and Hard, in `--campaign-legs-check`'s chain of all eight legs: no physics-input route to (2057, -3089, 448)). With `--campaign-allow-retry` (Sanity and Will restored once) school two dies the same way, is driven again from the refill and stops on the missing Demon Dice grant. The viewer-style chain (`--campaign-route-check` without `--campaign-strict`) stops at the same place with the same numbers: school one collects the Mallet from its altar there too, and school two dies as F3a found.

## Harness limits

- Auto-aimed Blade throws only, until R2 (weapon selection).
- No `Npcs` simulation apart from the placed club guards above; the saved cast of a headless checkpoint is empty, and the windowed writer regenerates it from the models.
- Single-visit checks run at the default difficulty; the chain takes `--difficulty`.
- The world is a snapshot frozen at load, so a mover that a route has to ride needs an explicit helper.
- The save/continue probe compares 600 ticks of a fixed walking input, not a driven leg; the mid-leg checkpoint is one moment per leg. It proves what a checkpoint carries, not everything a real save holds (Alice's character block and the cast are covered only by the windowed pair).
- The exit graph is a reviewed table held against the data. A visit that gains an adapter adds it to `ADAPTERS` and updates `EXPECTED`; one whose scripts or volumes change fails the check until its row is reviewed.

## What the recorded drivers assume

Every recorded driver was tuned from a full Sanity and Will bar at its entrance, and several finish only because they keep walking after the exit trigger fires (`stop_at_exit` ends a leg at the trigger). Chained on carried resources, the recorded drivers stopped the strict Normal chain at Beyond the Wall (visit 4: Alice arrived with 57 Sanity and 22 Will, and the route spent 53 Sanity and 38 Cards). That driver was retuned (below and `docs/BEYOND.md`), school one's driver was retuned after it (`docs/SCHOOL.md`: a fight stall on the library walkway and the Mallet detour), and the chain now stops in school two, the same problem two visits later, while the school chain (visits 6 to 8, from school one's baseline) stops in school two as well (arriving from school one with 48 Sanity and 3 Will, the Cards-heavy route dies in its first fights). Easy and Hard change the enemy placements the routes were tuned against, and the timing that follows from them. Run alone, all nine leg bodies pass at Normal and at Easy. The Easy corridor of Beyond the Wall differs from Normal's (its four rolling-walkway pieces `longwalk1` to `longwalk4` carry the authored not-on-Easy flag, and a row of static pads at z -64 stands in the map instead, with gaps of about 160 to 190 units between them), so the retuned driver plans its jumps from pad to pad there; the recorded straight, timed crossing had walked off the pads, the fall-recovery portal had returned Alice to the corridor's start and the planner had found no route onward. At Hard every leg passes alone but school two (Alice takes 1.4 times the damage and deals 0.7 times, and the recorded Cards-heavy school route dies in the closing battle). Beyond the Wall passes at Hard alone and chained (Alice enters it with 20 Sanity and 1 Will): the first retune of its driver left two Boojums alive on the way to the walkway (Hard places a third one, `get_booj1`, and its Cards weigh 0.7) and their screams shoved Alice off the rolling walkway, so the driver now brings the far Boojums to the wide floor at the walkway's foot and fights them there, and shoots the bridge's Diamond first (`docs/BEYOND.md`). School one's driver now collects the Croquet Mallet (its leg reports 1 pickup), but the recorded school-two driver collects no toy (0 pickups), so a strict chain lacks the Demon Dice that the baseline loadout would have supplied until that driver walks over them. These are route-driver retunes (health and toy detours, fewer Cards), not gameplay changes; the chain reports them and a later recipe-13d run tunes them. `--campaign-route-check` reports these frontiers as they are and does not retune a driver to make the chain green.

## Checks

- `--campaign-route-check`, `--campaign-graph-check`: above.
- `--campaign-legs-check [--difficulty <d>]`: entrances of all 39 visits (the eight opening ones must be clear; an exit into a missing entrance fails and changes nothing), then each leg alone from its own baseline under `Leg::run`, with its exit handed to the next visit through `Route::depart` (strict) and the hand-over checked (ledger, untouched resources), then school one's secret variant, then the legs chained on carried resources: all eight, and visits 6 to 8. The chained runs are reported, not gating at any difficulty. At Normal every leg body gates (the first failure fails the check). At Easy, Hard and Nightmare the plan reports and never gates, so a leg body that does not pass prints `REPORTED not passing at <difficulty>` with its reason, the run goes on to the next leg, and the closing line is either the `PASS` line or `REPORT campaign legs at <difficulty>: n of 9 leg bodies do not pass alone`; the check exits 0 either way.
- `--route-difficulty-check`: for every visit the loader builds the enemies and pickups at Easy, Normal and Hard and they differ where the authored spawn flags differ (15 of the 39 visits, six of the eight opening visits: Pandemonium differs only at Hard, and gvillage and skool2 not at all); for the eight opening visits the route built by `Route::enter` holds exactly what the loader built, and carries the difficulty in its map, its resources and the damage Alice takes.
- Unit tests: the visit keys and the opening exits, the strict arrival grants, the obstructed entrance, the legs' alignment and portal counts, the metrics, the probe input, the exit table, the script fact scanner, the reward table and its failure rules, the resume guard, the exit policy, the expectation parser and the pinned-frontier gate, the typed missing-checkpoint refusal, the checkpoint round trip and the save limits.

## Re-baseline of `--school2-route-check` (2026-09-29)

School two moved from its private harness onto the shared route. With the private harness's hit delivery (`School2::hit`, `Guard::hurt_kind`, which drop the weapon's knockback) the shared route reproduces the old log byte for byte (26,915 ticks). The shipped route delivers the whole hit like the viewer, so Boojums and guards recoil from hits, and the route finishes in 25,814 ticks with 8 jumps, 10 throws, 3 swings, 109 cards, 69 combat damage and 100 Sanity. Nothing else changed: the gym battle segment is 8,888 ticks with 57 damage taken, and `--school-return-chain-check` still ends in 12,134 ticks with 82 Sanity.
