# Ladybug acorn bombers — version 0.22

The Pool of Tears (`potears1`) is the next campaign area introducing enemy types beyond the implemented guards and Boojums. Its Ladybugs now fight: four resident actors and eight ambush actors across four paired spawns. Other maps' Ladybugs are not converted yet.

Open **tools/launchers/Launch-Pool-of-Tears.cmd** and choose **N** for a new visit if Continue is offered. This starts at the normal entrance with the campaign loadout. Existing saved progress can still be continued. The whole map is not yet completable: leaf rides, additional story/progression and the Snarks, Army Ants and Bloodroses remain unfinished.

## Combat

Ladybugs spot Alice within 2,048 units when geometry permits, fly above her, and drop an acorn on frame ten of the original attack animation. A falling acorn can be dodged. Direct contact deals 20 damage; the explosion adds up to 25 with linear falloff over 192 units, blocked by walls. Acorns collide with geometry and water and expire after three seconds. The Ladybug returns to its authored route or home position to rearm, with a minimum two-second delay. Looking Glass invisibility prevents new targeting but does not remove existing bombs.

The original body, carried acorn, falling projectile, flight/attack/pain/death animations and six sound references are used. Nonlethal damage interrupts the attack; lethal damage removes the actor from weapon targets. If still armed, it drops that one remaining acorn, falls, plays its landing sequence and disappears. A spent Ladybug cannot release another bomb after death. Existing bombs finish their flight. Blade, Cards, Mallet and Demon Dice use the shared enemy damage interface.

Effective health is **39**: the local actor definition starts at 1,039, and its AI begins the death sequence at 1,000. Flight follows authored path-node loops with swept body collision, line-of-sight acquisition, five seconds of remembered pursuit and bounded 3D detours around obstructions. If a destination cannot be reached by the bounded search, the actor approaches an available partial route and retries.

## Scripted activation and persistence

| Original trigger | Effect |
| --- | --- |
| `ladies1and2`, `ladies3and4` | Start the matching resident pair's patrols |
| `Spawn_LadyX1` through `Spawn_LadyX4` | Activate the matching ambush pair, with the second arriving 2.1 seconds after the first |

These are explicit reviewed bindings in the shared entity/event system. Walking through the original trigger volumes fires them once; pause freezes the delay. Repeating an activation cannot revive dead actors. The decorative NPC layer no longer draws or updates duplicate Ladybugs in this map.

F5/F9 and Continue preserve health, activation, position, navigation route, patrol waypoint, attack/rearm/pain/death timers, the held acorn and each falling bomb. The event queue preserves a pending second arrival. Cosmetic blast flashes restart. Save format 4 imports older Pool of Tears visits without replaying already-passed ambush triggers; their previously decorative resident Ladybugs become combat actors. Older save formats remain readable. See [SAVES.md](SAVES.md).

## Verification and limits

`--ladybug-check` verifies all twelve actors and their spawn clearance, all six actual map trigger groups, delayed save restoration, pause and no resurrection at 30/60/144 Hz. A stationary staged target below a resident Ladybug takes identical damage at each rate; weapon contacts interrupt and defeat it. Unit tests cover navigation around a wall beneath a ceiling, water avoidance, dodging, blast obstruction, death drops and deterministic saved bomb continuation.

`--ladybug-render-check` captures explicitly staged patrol, attack, release, blast, pain, falling death and landing poses in the native renderer. Native save checks write and read separate processes, including an ambush delay, an airborne bomb and a dead Ladybug. These checks are not an entrance-to-exit Pool of Tears playthrough. See [VALIDATION.md](VALIDATION.md) for results.

Original data was inspected locally, read-only. New AI, routing, event bindings and explosion flashes are independent Rust implementations. Authored curved flight paths are approximated by node segments. Exact original steering, particle choreography, knockback, loot and difficulty tuning remain incomplete; NPC bodies are not solid to Alice or one another. Sounds decode and are connected, but this revision's native tests were silent. Original artwork, extracted observations, captures and test saves remain private and are excluded from source packaging.
