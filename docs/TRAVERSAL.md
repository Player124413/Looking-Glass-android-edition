# Campaign movement — version 0.27

## September 2026 update

Rope contacts now slide along surfaces while retaining tangential momentum. Climbing is resolved separately from swinging, so pushing into a wall no longer cancels a clear ascent; both shared ropes and Pandemonium use this response. The pendulum remains constrained to its grip length and swept against the player body. The flexible tail also slides at contacts instead of losing all motion.

Alice now alternates the original right/left ascent strokes and left/right descent strokes. The complete cycle advances with resolved travel along the rope, independently of swing height, and retains stationary hand-placement frames. Each hand keeps its original sound cue. Facing turns smoothly and remains stable directly under the anchor. Saved animation state retains the last grip length; older saves default this optional value on resume.

Verification for this fix: 501 unit tests pass, including wall/floor particle slides, climbing and escaping under inward swing input, grip-length constraints, paired poses, save continuation and anchor-facing stability. `--animation-runtime-check` verifies real-asset climb cadence at 30/60/144 Hz; `--animation-runtime-render` captures both hands ascending and descending. `--traversal-check`, `--pandemonium-check` and the normal Pandemonium route pass. The route completes in 21,790 ticks with 56.6 Sanity. Separate native save write/read runs pass the `movement-rope` case. These are targeted fixtures and an automated route, not an exhaustive playthrough of every rope location.

Airborne edge catches, hanging, shimmy and pull-up are now available; see [LEDGES.md](LEDGES.md) for inputs, saves and limits. Rope brushes now supply grab bounds rather than visible thickness. All 27 placed ropes across seven maps use narrow textured strands with constrained segments, gravity, damping, a pinned anchor/grip and a free tail that continues moving after release. The grip slides continuously between fixed material points; the taut upper span follows Alice while the tail keeps its inertia. Climbing no longer switches a rounded grip index or redistributes every segment's length. Hand samples are interpolated at the rope's 120 Hz simulation rate. The existing swept pendulum still controls Alice; this is not a replacement of her collision or swing rules. Flexible presentation is rebuilt after load while saved player grips and momentum continue normally. See [TOWER2.md](TOWER2.md) for moving entity water.

The older verification record below describes the initial traversal implementation. Current saves use format 12.

Currents, launch volumes, updrafts, limited air, generic rope climbing and supported enemy knockback now share the normal movement controller. Launch normally to use them; saves made before this version upgrade when loaded. The garden entrance now carries Alice upward in its authored current, so it no longer behaves like a still-water demonstration.

## Controls and behavior

- **E** grabs a nearby visible rope and releases a held rope. Touching one closely while airborne can also catch it. **Space/Ctrl** climbs up/down; **WASD** swings. E while holding Space jumps off. Otherwise release preserves swing momentum. Walls and ceilings block grabs and movement. Rope mounting suppresses weapon attacks and their resource costs.
- On land, forward movement plus **E** or **Space** requests a supported low-ledge climb. At a water bank, move toward the bank and hold Space. This remains limited ledge climbing, with clearance checked throughout the motion.
- Fully submerging Alice's head consumes air. The ordinary budget is **five seconds**; the Mock Turtle breathing upgrade grants **twenty seconds**. The original temple entrance grants this upgrade, independently of returning the Duchess's shell quest item. Fresh later-map starts receive it through their campaign profile; normal transitions retain the earned flag.
- A small air meter appears while submerged. After air runs out, drowning takes **12 Sanity every half second**. Surfacing refills air. Pause, menus and focus loss freeze the clock. Drowning uses the original choke sound and death pose, and the existing death/retry loop.
- Authored water currents, wind and launch volumes move Alice through swept collision. Negative-speed acceleration volumes supply the original upward gravity effect and brief exit linger. Boojum screams, Diamond projectiles, Ladybug acorn explosions and Duchess impacts push Alice; a brief input delay prevents movement input from instantly cancelling the impulse. Hits release a generic rope grip and interrupt a ledge climb.

## Original evidence and implementation

The supplied 36 BSPs contain 158 `trigger_push`, 42 `trigger_accelerate` and 27 `func_rope` entities. Forty-four push volumes explicitly exclude players. The runtime loads the remaining **156 player force volumes** (114 pushes plus 42 accelerators) and all 27 ropes. Pandemonium keeps its existing rope controller and progression choreography. No generic ladder entities were found. Tower3's named ladder/crank/debris targets are absent from its BSP; its ladder trigger is a documented no-op, not a climbable setpiece.

Read-only analysis of the local gameplay DLL confirmed directional push projection, target-apex launch velocity, the independent player/actor/projectile response mask, upward gravity at -0.25 of ordinary gravity, 0.1-second updraft linger, and the five/twenty-second air budgets with 12-damage/0.5-second drowning. The original model declarations provide Boojum impulse strengths of 80/300, Diamond 150 and acorn blast strength 200 over radius 192. Original Alice hanging/up/down clips drive rope presentation. Climb volumes without visible brush faces receive a narrow independently generated vine mesh using the supplied vine skin; it shares world depth, fog and rope transforms. This mesh is a visual approximation, not recovered original rope rendering.

Rust implements these reviewed rules without executing original scripts or game binaries. Archive extracts, decompiler output and original-art screenshots remain under `private/` and are excluded from the source package.

## Persistence and verification

Save format **9** persists remaining air, drowning cadence, breathing upgrade, rope identity/length/momentum, updraft linger and knockback delay. Existing shared entity flags preserve disabled/enabled movement volumes. The loader validates a held grip against its rebuilt rope and attachment position. Formats 1–8 migrate with fresh air and an upgrade inferred from established campaign history. Ordinary user save slots were not used for testing.

`--traversal-check` parses the entire map set. Real Pool of Tears current, village updraft and garden current probes match at **30/60/144 Hz**, remain outside solid geometry, and retain shared activation through serialization. Real ropes in Pool of Tears, both garden maps, Funhouse, Hedge 3 and Red Chess climb/swing and continue identically after a saved grip is restored. These use staged positions to isolate the mechanics; they do not establish complete routes through those unfinished maps. `--swim-check` separately isolates the swim controller with currents disabled in that test world, including eight garden bank approaches.

**147 unit tests** pass, including collision/activation, impulse frame-rate and pause behavior, launch apex, climbing obstructions, one-shot E buffering, grip persistence, original breath budgets, drowning cadence and death. Native save writing and reading in separate processes pass **43 cases**, including five new movement cases. A nearly exhausted air supply resumes with the same subsequent damage; it cannot be refilled by reloading.

Normal-input route regressions pass with combat active and no debug warps, flight, recovery or refills:

| Route | Result |
| --- | --- |
| Village to its Pandemonium exit | 10,068 ticks, 6 jumps, 100 Sanity |
| First school through library and recipe exit | 15,039 ticks, 15 jumps, 1 climb, 27 Sanity |
| Second school through ingredients, potion/star and gated return | 21,119 ticks, 7 jumps, 50 throws, 30 Sanity; two authored portals |
| Pandemonium with cinematic skipping | 21,049 ticks, 8 jumps, 18 throws, 69.33 Sanity; one authored portal |
| Duchess arena through temple reward transition | 9,571 ticks, 26 throws, 75 Sanity |

The route runners now replan ordinary inputs after live enemy displacement. The school-two runner approaches the grown lollipop again if the greenhouse Boojums push Alice away during growth. These are automated headless input simulations with existing aiming/path-search limitations, separate from desktop testing.

Anode keyboard testing loaded a staged underwater save, displayed remaining air, allowed drowning to reach the retry screen, then retried and saved with replenished air and the upgrade retained. A staged rope save accepted Space/Ctrl movement and E release; quick saves recorded the changed height/grip and released momentum. Native audio output was unavailable on this test desktop; asset decode and event wiring do not establish audible playback.

## Remaining limits

This completes the shared mechanics described above, not every campaign map that uses them. Unfinished owning sequences keep their authored initially disabled forces: Pool of Tears 2 suction, Grounds 1 gate, Centipede 2 ant trigger, Tower 1 face gusts and Hedge 3 bellows. Their future puzzle/boss controllers must enable them through the shared event state. Other enemy types, weapon knockback against enemies, leaf rides and arbitrary moving anchors/supports still need their own work. Fortress 1 remains the earliest unfinished main campaign map.

Rope swing strength, damping, grasp offsets, updraft damping frequency, ordinary acceleration, collision shape and low-ledge timing remain independently tuned approximations. Original per-frame rope-hand transitions and all pain reactions are not reproduced. Slime/lava retain the previous provisional integrated damage rates. See [SWIMMING.md](SWIMMING.md), [SAVES.md](SAVES.md) and [ROADMAP.md](ROADMAP.md).
