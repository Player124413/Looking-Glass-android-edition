# Power-ups, Pocket Watch, drops and difficulty — v0.28

These rules are independently implemented in Rust. Read-only inspection of the user's local item definitions and gameplay DLL supplied durations, damage multipliers, difficulty flags and reward tiers. No original executable or gameplay DLL is loaded by the remake. Original models, animations and sounds are read from the user's archives at runtime.

## Playing

Walk into a power-up to activate it. Its original icon appears beside the Will meter; a compact HUD line shows its remaining time. Only one of Rage Box, Grasshopper Tea and Darkened Looking Glass can be active; another pickup remains available until the current effect expires. Death/retry clears these effects. Normal map transitions retain them.

| Power-up | Effect | Easy / Normal | Hard | Nightmare |
| --- | --- | --- | --- | --- |
| Rage Box | Outgoing Blade damage ×4, Mallet ×3, other supported attacks ×2, in addition to difficulty | 42 s | 33 s | 25 s |
| Grasshopper Tea | Ground/swim movement speed ×2; jump launch speed ×1.5 | 26 s | 20 s | 15 s |
| Darkened Looking Glass | Supported enemies stop acquiring Alice as a target; existing projectiles and environmental hazards remain dangerous | 45 s | 36 s | 27 s |

Rage does not grant invulnerability or an extra incoming-damage reduction. Its original back, hair and claw attachments appear on Alice. Tea has a green appearance cue. The first school's gated Looking Glass secret uses these same duration/exclusivity rules. The existing Turtle breathing upgrade remains a separate permanent quest reward and works with these temporary effects.

**0** selects an owned Pocket Watch (called Deadtime Watch in the original inventory). Either mouse button activates the same ability: **1 Will**, **20 seconds** stopped time, **360 seconds** recharge measured from activation. Alice can move, attack and collect items while supported enemies, enemy projectiles, autonomous movers and summoned demons are stopped. Thrown Dice continue rolling, as indicated by their original `ignore_deadtime` definition; the resulting demon waits for the world to resume. The HUD shows stopped time, then recharge time. Busy, unowned, unaffordable or recharging uses do not spend Will. Retry clears the stop but retains the outstanding recharge. Use the ordinary campaign pickup/loadout or the explicitly staged weapon preview to obtain it.

## Fortress Rage Box (2026-09-30)

The reviewed `fortress2` spawner `spawn_ragebox` now supplies the visible Rage Box
at entity 37, origin (-160,3552,88). Ordinary placed Rage Boxes keep their existing
loader and difficulty filtering. Contact grants the existing transformation,
plays the pickup cue and queues the original Cat voice asset. After one second,
the existing difficulty-filtered guard group activates. The supporting lift stays
raised during the transformation, then lowers 48 units over four seconds. Collection and the delayed callback survive
saving without duplication; the late-room sequence removes an unclaimed box.

The pickup now starts the nine-pose, 12.5-second transformation with the reviewed
player-relative camera and control lock. Its optional saved scene is independent
of the one-second guard delay. Watching and skipping release the lift at the same
completion boundary. Restoring retains both clocks and the single grant; no path
solves puzzles or unlocks the exit. Older saves retain any descent already in progress.
Existing completed pickups remain consumed. Exact native camera damping,
replenishing-wave counter and used-box opening/vent presentation remain fidelity
work; see `POWER_PRESENTATION.md`.

## Difficulty

New games default to Normal. Use `Launch.cmd --new-game --difficulty easy|normal|hard|nightmare`, or open **Tab**, press **D** to select difficulty, then Enter to start the chosen visit fresh. Continue/F9 always restore the saved difficulty; changing the next-start choice does not change the current campaign. Difficulty carries through ordinary exits.

| Rule | Easy | Normal | Hard | Nightmare |
| --- | --- | --- | --- | --- |
| Incoming damage multiplier | 0.75 | 1 | 1.4 | 1.75 |
| Alice's outgoing damage multiplier | 1.25 | 1 | 0.7 | 0.55 |
| Temporary power duration | 100% | 100% | 80%, rounded down | 60%, rounded down |
| Passive Will recovery per second | 0.3, up to 100 | 0.1, only up to 10 | None | None |
| Map spawn inhibit bit | 256 | 512 | 1024 | 1024 |

The Watch's duration/cost/recharge are independent of difficulty. Supported placed pickups and enemies now respect the difficulty bits separately from scripted activation. Will recovery runs during attacks, with no spending cooldown; fractional recovery uses the Rust simulation clock. Sanity does not passively recover. Starting resources and inventory still use the existing campaign loadouts. Fatal fall volumes remain fatal on Easy.

### Recovery correction (2026-10-01)

The [licensed strategy guide](https://oldgamesdownload.com/manual/american-mcgee-s-alice-windows-strategy-guide-english/) describes passive Will recovery only on Easy/Medium and restorative pickups for Sanity. Read-only inspection of the supplied `fgamex86.dll` confirms increments of 0.3 on Easy and 0.1 on Medium (our Normal), the Normal threshold of 10, and no attack-delay condition. The recovery branch tests integer game-seconds multiplied by 100 modulo 100; the Rust implementation integrates those nominal per-second increments independently of rendering frequency. It does not reproduce native frame/command-timing quirks.

The former two-second delay, which repeated spending could continually restart, is removed. Normal's provisional 0.2 rate is corrected to 0.1. Old saves still load their legacy delay field, but it no longer suppresses recharge. Health still requires Sanity shards or Meta-Essence; Will vials and Meta-Essence remain the main replenishment during combat. Weapon costs are unchanged. Private evidence and build validation are under `private/will-recovery/`.

The recovery source snapshot passes 658 unit tests. Packaged build `51B603A1` passes ten headless checks covering resources, items, weapon input/assets, Mallet/Jackbomb, Ice/Jacks, Staff/Blunderbuss, Dice, Keep and Hedge 3. The normal launcher was checked after publication; the dedicated Jabberwock and finale builds were updated as well. Existing save slots were not changed.

## Enemy rewards

Supported combat enemies release one Meta Essence on the live-to-dead transition, using their original `manatype`. Touching it restores **both** Sanity and Will, capped at 100; it is left in the world if neither meter needs it. Collection respects walls. Drops settle against world collision, use the original essence model, and get weaker if left behind.

Enemy drops render their original three material layers: the view-dependent reflection, coloured skin and animated swirl. They use transparent depth and fog instead of treating the grey reflection image as an opaque texture. The original pickup box also includes its corners; visibility is checked within the height Alice actually touches, so standing on a step beside a drop no longer prevents collection. These fixes apply to existing saved drops as well as new kills. Full Sanity **and** full Will still leave a drop available for later. Touching one in that state now displays "Sanity and Will full" once per contact, using the ordinary HUD notice. Placed Sanity and Will pickups explain their full meter in the same way. A notice never consumes an item or changes a save.

| Enemy / tier | Sanity and Will each | Time in this tier | Next tier |
| --- | --- | --- | --- |
| Club guard / Small | 15 | 40 s | Disappears |
| Diamond guard or Ladybug / Medium | 25 | 20 s | Small |
| Boojum / Large | 50 | 10 s | Medium |
| Duchess / Super | 100 | 10 s | Large |

Timers apply successively: a fresh Large lasts 70 seconds in total if uncollected. Watch time and pause do not age essence. Cached visits are frozen while Alice is elsewhere. The corpse's stable identity is recorded independently of collection, so loading, retrying or replaying a death animation cannot issue a second reward. Already-dead actors from older saves do not generate retroactive drops. The Duchess's existing arena replenishment is separate from her one-time death reward.

## Saving and verification

Save format **10** preserves difficulty, all power clocks, Watch recharge and each visit's live drops, remaining decay times and awarded-enemy ledger. F5/F9 and Continue resume remaining simulation time; time spent with the game closed does not consume effects. Formats 1–9 load with Normal difficulty, no newly active powers/drops, and their existing Glass timer. Migration preserves old actors' health/state while adding previously omitted difficulty-specific actors. The loader rejects invalid/conflicting timers and malformed drop records before replacing the live game. See [SAVES.md](SAVES.md).

Validation covers unit tests for effects, damage, costs, exclusivity, pause, death/retry, serialization, Tea collision/movement and summon freezing. `--items-check` reads all 36 maps and finds **338 / 314 / 293 / 293** available supported pickups across the four difficulties. It checks item/Watch/enemy archive declarations and freezes actual second-school movers/enemies at 30/60/144 Hz, permits damage while frozen, then resumes them after saved expiry. These are component fixtures, not campaign playthroughs.

`--loot-render-check` renders all four saved drop grades at two animation times, checks coloured pixels, verifies wall occlusion and additive/filter fog, and checks that reloads share GPU materials. Private captures are `private/loot-textures-{0,2}.png`. `--items-check` also tests both reported library-door drop locations against the real first-school brushes, including direct/diagonal contact, either depleted meter, full-meter feedback and saved-drop collection exactly once. Pickup regressions cover the formerly blocked step, diagonal contact, out-of-range rejection, walls, full meters, save/reload and exactly-once rewards. The September 28 drop fix passed these native checks, all **221** current unit tests, `--items-check`, and a native load of the existing live-drop save fixture.

Separate native writer/reader processes pass **52 save cases**, including nine new power/Watch/drop cases and exact continued simulation. Anode input checks confirmed Rage appearance and the Watch's one-Will activation; a full process restart retained Hard difficulty and the exact Watch timers. The normal-input village, first school, second school, school return, Pandemonium and Duchess route checks pass with drops enabled. Logs and original-art test captures stay private; user save slots were not used. See [VALIDATION.md](VALIDATION.md).

## Remaining fidelity limits

Rage/Tea presentation has since been expanded; see [power presentation](POWER_PRESENTATION.md). The September 28 [Dice/Watch update](DICE_WATCH.md) supersedes the earlier Watch and summon-scaling limitations: implemented toys now follow the reviewed deadtime exceptions, random environmental sound scheduling stops separately from PCM, and demons use independent damage values. Unsupported enemy/map controllers and exact original renderer-clock parity remain outside that coverage.
