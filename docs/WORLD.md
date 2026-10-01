# Environment and world interactions — version 0.7

**Version 0.29 update:** portal skies, environment reflections, cutouts, deformations, oriented multi-emitter effects, shared transparent sorting and blend-aware fog are now supported. The environment limitations below describe the earlier milestone; see [RENDERING.md](RENDERING.md) for current coverage and remaining fidelity work.

**Version 0.18 update:** village brush bridges and machinery now draw with matching moving collision, reviewed village/school target triggers activate encounters once, and two reviewed shootable triggers operate the village hatch and school secret. The village reaches its authored Pandemonium exit; a complete route through the intervening fortress levels to school remains unfinished. See [VILLAGE.md](VILLAGE.md) and [SCHOOL.md](SCHOOL.md). Sections below are the historical v0.7 record, not the current school completion status.

**Version 0.14 update:** the first school visit now has explicit theatre/library gates, moving lifts and bookcases, the four-book bridge and the recipe exit. A continuous physics route reaches `skool2` without flight or recovery. See [SCHOOL.md](SCHOOL.md) for the current scope; the blocker list below records the earlier v0.7 milestone.

**Version 0.9 update:** the school theatre floor now loads its entry-specific set of eight solid platforms, and fatal-fall volumes lead to a usable retry loop. General scripted motion remains unfinished. See [RECOVERY.md](RECOVERY.md).

**Version 0.8 update:** swimming, low-bank/ledge climbing and liquid hazards are now implemented; the transparent-effect wall-occlusion bug is fixed. See [SWIMMING.md](SWIMMING.md). The notes below retain the scope of the original environment milestone.

This milestone adds atmosphere and a first interaction layer. It does **not** complete a level or execute the original gameplay scripts. The new code reads the user's archives without loading the original executable or DLLs.

## Controls and behaviour

While walking, look toward a nearby rotating door and press **E**. A small open/close prompt appears when it is in reach and unobstructed. Adjacent leaves operate together, and the original `sound_move` reference is played when available. Doors rotate with matching collision; an obstructed leaf waits rather than sweeping through Alice. A door interpreted as locked stays locked. P, inventory, menus and focus loss pause this interaction. E remains ascend in free flight.

Walking across supported volumes can teleport Alice, reduce Sanity or load another map at its named entrance. Inventory and collected pickups retain their existing per-run persistence. Home returns to the current map entry and restores resources; it does not reset all world state. Script-trigger volumes display a brief unfinished-event notice and log the pending thread instead of pretending to run it.

## Environment support

- Material parsing preserves line boundaries, including `animMap` frame lists. Supported layers include opaque, alpha, additive and filter blends; animated image sequences; UV scale, scroll, rotation, turbulence and stretch; selected colour/alpha waves and detail textures. Lightmaps are combined with the primary diffuse layer. Unsupported stages are skipped, with a base-image fallback where possible.
- Distance fog reads a single unambiguous literal `setfarplane` declaration from the map script as data. This works for eight supplied maps. Maps with multiple changing declarations are left unresolved; no script control flow is inferred. Its linear density curve is provisional, beginning at 35% of the authored distance.
- Fog volumes use the BSP fog records, material `fogParms`, and validated axis-aligned brush bounds. All eleven volumes in this corpus fit the supported shape and maximum of two per map. Density uses the view segment's length through each box and is approximate. Fog is applied to world geometry, Alice, equipped toys, weapon effects and steam; pickup billboards remain unfogged.
- The original steam and hot-steam emitter definitions provide textures, emission rate, particle scale/growth, lifetime, random offsets and velocity. Particles fade, rotate and clip against world collision. Emission is bounded by distance and a 2,048-particle cap. Other emitter families, entity orientation, script activation and attachment rules are not yet reproduced.

Transparency is sorted by batch, not triangle. Sky portals/skyboxes, reflection/environment mapping, view-dependent alpha, deformation, dynamic lighting and exact original blend/fog conventions remain incomplete. The school entrance has no general distance fog: its authored fog volume is elsewhere in the level. A blanket haze was not added to that room.

## Interaction fidelity limits

The supported entity classes are `func_rotatingdoor`, `trigger_changelevel`, `trigger_teleport` and `trigger_hurt`. Thread-bearing `trigger_once`/`trigger_multiple` volumes are recognised but their scripts do not run. Inline geometry and collision are transformed together.

Door behaviour is provisional: a 90-degree rotation opens away from Alice, the entity's `time` sets duration, nearby leaf centres determine pairing, and bit 4096 is interpreted as locked. These choices and the remaining spawn flags have not been verified against the original runtime. There is no automatic closing, scripted unlocking, crushing, sliding-door motion or general entity target/event dispatch. Damage has a provisional half-second cooldown. Trigger enable/disable and spawn-flag conditions are not implemented; a supported class does not imply full authored progression semantics. `trigger_fall` remains unsupported because its behaviour has not been established.

## Why the school is not yet completable

The school has different first-visit and return-visit setup paths. Reading the supplied level data identifies these concrete blockers:

1. The first visit requires the flying-book puzzle: moving supports follow paths, counters unlock bridge platforms, and nearby bookcases rotate. None of that event/state machinery runs yet.
2. The ingredient/book cinematic enables an exit trigger and invokes a transition through a script. A direct `trigger_changelevel` works, but a script-only map command still needs script execution.
3. Lifts and the observatory path depend on movement waits, targets, locking and visit-specific setup. The return visit removes or changes parts of the first puzzle, so one unconditional collection of objects cannot represent both visits.
4. Versions 0.12–0.13 add placed NPCs, E greetings and a bounded club-guard combat encounter. Scripted NPC movement, story dialogue, other enemy AI and cinematics remain absent. See [NPCS.md](NPCS.md) and [COMBAT.md](COMBAT.md). Swimming was missing at version 0.7; version 0.8 now supports the underwater garden start and getting out of its first pool.

The next playability milestone should be **one complete, intended first-visit school route**: a bounded script/event scheduler, map-start setup, entity targeting, waits and counters, moving supports that carry Alice, then the book puzzle and its actual exit condition. This needs its own end-to-end playthrough. An exit reached using free flight is only an exit-system test.

## Evidence and checks

`--world-check` validates all 36 supplied maps: 149 rotating-door objects, 506 supported or pending trigger volumes, 11 fog volumes and 8 literal distance-fog settings. It checks exit-map/named-spawn references and reports 376 distinct map/thread pairs still requiring scripts. Counts describe parsed support, not completed content. A deterministic school route at 30, 60 and 144 render Hz proves that the closed leaves block movement, E opens both and Alice can walk through without embedding.

Anode checks covered school door use from both sides, walking through, pause, fire animation and the school's three steam emitters. Garden atmosphere was inspected from free flight. Reaching the school exit in free flight, then walking into its volume, loaded `skool2` at the named entrance with inventory/resources preserved. This is **not** a full-level completion test. Desktop tests used `--no-audio`, so the new door sound event has not received a listening check.

The new work was authored from local map/material/emitter observations and general graphics/collision methods. No new Ghidra decompilation or original-versus-Rust runtime comparison was performed for this milestone. Raw extracted data, logs, screenshots and recordings stay under `private/` and are excluded from source packaging.
