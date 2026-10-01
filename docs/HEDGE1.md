# Majestic Maze: child, pressure plate and departure

`hedge1$hedge1_start1` uses the existing saved companion actors. Crossing BSP 25 starts the Muzzle child's run to `seek_end`. Its body presses plate 30, sinking brush 31 by four units and opening leaves 32/33 by 120 units at 400 units per second. Crossing BSP 527 sends that child through `seek_middle` to `seek_tele_path`, removes it and activates the chase child. The empty plate lets the gates close.

The chase child approaches Alice when she is visible within 300 units, walks inside 150, runs farther away and stops at 60. Its own contact with the monsters-only Hold volume, while approaching, starts the held pose. Continued occupancy refreshes Hold and the plate. Alice and enemy bodies can press the ordinary plate, but Alice cannot invoke Hold or complete the escort alone. The player and companion systems feed their real body contacts to the same controller; there is no second child or decorative gate.

The gate uses fixed steps, source brush collision and the same saved fractions for rendering and collision. It waits rather than crushing Alice. The plate carries a standing player. Pause freezes its clocks. Child movement and animation remain in the existing NPC snapshot; the new `hedge1` controller saves the gate, occupancy and exit state. An older NPC-only save keeps compatible actor states and rearms only the formerly pending Seek contacts. Invalid positions inside newly restored solids use the existing safe entrance recovery.

The exit is BSP 29, beyond the double gate, to `tower1$tower1_start1`. Source inspection confirms the entire `SeekCine` function and its map call are inside a block comment in `maps/hedge1.scr` lines 131–153. No cinematic is invented or required. The physical exit requires the chase child holding the plate and the leaves open. Departure is delivered once, and a failed destination load can retry.

The `t30` portal marker lies in the solid `teleporter2_1red` patch (world surface 549). A map-specific correction places Alice 16 units in the marker's facing direction, at `(4584,2648,2056)`, after checking full-body clearance. Surrounding collision stays intact. This is a documented landing approximation.

The native enemy system retains ownership of placed enemies and ambush groups. The seven untargeted actors in the sealed precache room remain in their original snapshot slots but are hidden and inactive. This preserves identities for the live enemies and their drops.

Deliberate differences and remaining fidelity work:

- Only the chase child's own occupancy can hold it. The original thread also renames that child when an unrelated monster touches the volume, potentially stranding it elsewhere.
- Long-delay repeat ambush spawning, hedge-top launch trajectories, puffball motion and gate sound fidelity retain their existing approximations. No new voiced scene is supplied by this map.
- The follower keeps up to 32 recent visible positions of Alice to round corners with collision-checked steps. Obstructed trail fragments are skipped only when the whole supported path to a later remembered point is clear, with retries limited to four per second. The saved trail can continue briefly after losing sight; it never learns unseen positions and clears when Alice is beyond 300 units. This is the bounded navigation approximation used in place of the original engine's path search.

Checks:

- `--hedge1-check`: supplied-map source bindings, closed/open collision, plate occupancy, escort gate, pause, restoration, retry and shared companion checks at 30/60/144 updates per second.
- `--hedge1-route-check`: normal entrance, native combat, child demonstration, handoff, portal, return escort and strict resource-preserving departure to tower1. Uses ordinary movement and paid weapon attacks.
- `--hedge1-save-write` / `--hedge1-save-read`: real save files from route checkpoints, exact state restoration and identical continuation in a fresh process.
- `--hedge1-render-check`: a route checkpoint supplied through `LOOKING_GLASS_MAZE_CAPTURE`; optional look target `LOOKING_GLASS_MAZE_LOOK`.

Private diagnostics and checkpoints live under `private/hedge1`; no original assets or script bodies are redistributed.

Verified on 2026-10-01:

- The full default native route starts at the ordinary entrance on Normal difficulty and reaches `tower1$tower1_start1` in 25,580 simulation ticks. It completes the demonstration, handoff, portal, return escort, Hold and physical departure. It collects 17 essence drops and finishes with 48.196 Sanity and 98.400 Will; the tower arrival preserves both exactly. There are zero hazard/lost ticks, zero jumps, one authored teleport and exactly one transition. God mode, notarget and minimum-health assistance are disabled.
- All 637 unit tests pass, including saved corner navigation, recovery after doubling back, and the narrowly scoped precache identity upgrade. All targets compile. Supplied-map gate/companion checks pass at 30/60/144 Hz.
- Real save files for the demonstration, moving leaves, handoff, following and holding restore exactly in a fresh process and continue identically for 240 ticks. A pre-controller save fixture retains compatible live actors and resources, deactivates the seven precache actors and recovers an old player position inside a newly solid leaf at the entrance.
- Native closed-gate and held-plate captures were visually inspected; the scene reports zero fallback images. Audio was disabled during these runs, so this is not an audio-fidelity result.
