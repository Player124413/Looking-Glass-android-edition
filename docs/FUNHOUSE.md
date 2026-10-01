# Mirror Image (`funhouse`)

The October 2026 performance update and original mirror behavior are described in [MIRRORS.md](MIRRORS.md).

Visit 22; one controller owns machinery, clock progression, Tweedles and scene completion. Save envelope remains 12. Controller payload version 1; hit IDs use the existing 8,200,000 reservation. Source: `src/levels/funhouse.rs` and its companion modules.

## Implemented behavior

- Intro commits cell 2 once and withdraws the entrance stand after the handoff.
- Eight cell bits, shootable mirror/cell clocks, closet glass and existing encounter activation, rotating tube, gear and gas-door sequence.
- All 20 rotating doors admit the player: 4096 means automatic closing, 64 player proximity, and 16 excludes Actors. The older locked-cell assumption in v23 was incorrect. Native evidence: private research `phase2/TRIGGERS_AND_DOORS.md`.
- Weighted fulcrums, difficulty-specific pit handling, suction/teeth cycles and three pendulums. Movable geometry carries riders with upright body clearance.
- Arena floors are present and solid before entry. Both named Tweedles retain distinct identities, health, animations, attacks and miniature caps. Only the two named deaths count toward the five-second ending gate.
- Hatter scene, saved dialogue/camera clocks, physical floor collapse and one retriable exit to `hatter1$hatter1_start1`. Skipping dialogue retains the physical ending tail and removes live miniatures.
- Existing map and model identifiers resolve against local installed data at runtime; no original dialogue is stored here.

## Verification and publication

Playtest candidate **6C9850B5**, based on Labyrinthine Revenge **D654E222** and retaining its earlier releases. Root `Launch.cmd` selects this candidate; `tools/launchers/Launch-Mirror-Image.cmd` opens a fresh visit with separate saves. This is a restored-systems playtest, **not a completed full-route certification**.

- Headless contracts pass: idempotent clock mask, real arena-floor traces, pause, machinery restoration, knife-frame damage, miniature caps, defeat/exit transactions, both door flags/directions, Easy rescue versus Normal fatal-fall volumes, weighted fulcrum carry/jump, a complete pendulum period and native radial-damage falloff.
- Nine native Store cases pass in independent writer/reader processes. Each compares the same 20-second owner/world/player future. The legacy case uses an actual pre-controller interaction snapshot with a consumed arena trigger; migration rearms it and restarts at the entrance without changing resources or inventory.
- Thirteen staged native renders pass and were inspected. The map references `textures/notexture`; it is the sole unresolved material identifier. Arrival pause, quicksave and held-Enter skipping were exercised using real keyboard input.
- 632 unit tests and root compilation pass. All 38 other visit fingerprints and event signatures match D654E222. Funhouse's new controller changes its own fingerprint and event signature; existing rule identifiers, visit order, save envelope and hit ranges remain unchanged. The 39-visit exit graph passes.
- The Normal native checkpoint lineage reaches all clocks, crosses the rotating tube and weighted fulcrums, and reaches the pendulum bank with live enemies, ordinary weapons, real loot, no invulnerability and no recovery teleports. The full combat course remains open: Boojum knockback defeats the route driver at the pendulums. Neither the movement-only probe nor staged arena fixtures are full-route proof.
- A separately labelled staged arena fixture restores the native placed cast and fights both bosses with ordinary attacks. The watched ending enters Hatter1 alive with exactly carried 100 Sanity/100 Will. A fresh process resumes the earned 4.958-second defeat-wait checkpoint, skips the Hatter scene and reaches the same required outcome. The staged arena placement is not evidence that the incoming route passed.
- Owner-only arena diagnostics start with 30 Sanity, collect both authored essence pads by jumping from their ramps, kill both named bosses and complete the watched/skipped Hatter ending at 100 Sanity. This does not cover the surrounding placed NPC cast or the incoming course.

Use `--funhouse-check`, `--funhouse-route-check`, `--funhouse-skip-route-check`, and `--funhouse-render-check`. The route checks are currently diagnostic; a complete pass is still required. `--funhouse-traversal-probe` disables the placed NPC cast and deliberately reports failure at its end. Private inputs, logs, source manifest and publication metadata are under `private/funhouse-work/`.

## Fidelity limits

AI selector probabilities and fulcrum return/damping frequency are bounded interpretations where native details remain unresolved. Detailed head tracking, death sinking and some scene gestures need further comparison. Child waypoint walks, the split player-clip, propeller loops and some exact actor navigation remain fidelity follow-ups. Audio identifiers and cue timing are implemented; listening has not been verified on the isolated test desktop.

## Entrance presentation correction (2026-10-01)

The six `begin_lamps` fixtures now receive the original World_Init roll of
270 degrees and render alongside the existing BSP flame surfaces. Previously
the unsupported placement command deferred their geometry, leaving floating fire.
The hanging lamps and chandelier retain their original models and materials.

The unnamed passable screen behind the starting stand now blocks camera sweeps
only. Player and weapon collision retain the original non-solid flag. Both the
follow camera and its cutscene return use this extra occlusion; it is rebuilt
by the normal interaction synchronization after entry or restore.

The headless fixture checks all six lamps, production synchronization, and
watched/skipped/restored arrival returns at 30, 60 and 144 Hz. Native entrance
captures cover 36 return frames and two views of the lamps. These checks run
as part of `--funhouse-check` and `--funhouse-render-check`.
