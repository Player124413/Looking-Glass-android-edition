# Pandemonium — version 0.25

Current facial support: original lip-sync envelopes and Alice’s blink texture are now connected for implemented dialogue. See [facial animation, verification and remaining limits](FACIAL.md). Earlier facial limitations below describe previous milestones.

Version 0.31.4 corrects the cart's wheel axis and parent-relative mounts, including the lift and final downward pitch. Wheel spin uses the original one-second turn timing and the existing saved cinematic clock. The renderer now includes the original `lantern2` bodies that surround the BSP flames, retaining their authored position, scale, material and depth/fog behaviour. Baked static copies are not duplicated. Axial flame billboards keep their original rectangular dimensions and long axis as the camera moves. This also restores the same hanging-lantern model where placed in other maps; other unsupported decorative model types remain outside this change.

Version 0.25 adds the warning, cart, house-return and departure cinematics. **Hold Enter to skip the current scene** while preserving its required world changes. **E** advances one line. [CINEMATICS.md](CINEMATICS.md) records the cameras, acting, saved-state behavior and current verification.

**tools/launchers/Launch-Pandemonium.cmd** opens the next restored campaign map, between the village and Fortress 1. A new visit starts at the authored entrance with the Vorpal Blade. Normal entry from the village also works through the existing transition system. If Continue is offered, choose N to start a fresh visit.

## Route and controls

1. Descend from the entrance using the rope. E grabs/releases a nearby visible rope; Space climbs up, Ctrl climbs down, and WASD swings it. Speak to the Elder Gnome below; E advances dialogue.
2. Follow the street west, then climb the uneven mine approach. Enter the minecart boarding area. The lift rises and the cart follows the original rail markers, delivering Alice to the far ledge as the original scripted ride does.
3. Collect the Cards, pass through the two doors and fight or evade the guards. Find the Gnome's key at the end of the lower hallway.
4. The key opens the upper door. Climb the staircase around the hallway, pass that door and enter the return portal.
5. Leave the return room through its raised gate. Return along the lower street, climb toward the Gnome's house and enter it. The airship carries Alice and the Gnome away; completion of its dialogue transitions to `fortress1$fortress1_start1`.

H shows the current objective. Saving works during ropes, the cart and the airship. Recovery is suspended during a ride; after the one-way cart and before the return portal, Home/fallback recovery uses the far landing, keeping progress reachable.

## Restored systems

- Both rope brushes are visible, can be grabbed and swung with collision checks, and follow the hanging pose. Rope momentum and climbing length persist.
- The minecart, wheels and 640-unit lift move through explicit boarding/lift/rail/landing phases. The two collapsing track groups animate; the exit rocks and mine door seal the spent route. These are scripted transports with the original landing transfer, not ordinary collision-driven vehicle simulation.
- The key is visible and collected by a nearby unobstructed contact after the cart. Its door rotates open; the return gate rises and the house entry becomes available on return. Opening solid gates stop if their next position overlaps Alice, then resume when clear.
- Seven guards in the supported difficulty layout use the shared encounter system. Cart arrival, the upper approach (including its delayed second activation) and return contact activate the appropriate groups. The original key script does not require every guard to die; no extra kill-all exit gate was invented.
- The return portal, mine-backtracking block and Gnome-house exit are gated by saved progress. The final transition is emitted once, after both the flight and departure dialogue finish.
- The original airship mesh and animation play, with Alice and the Gnome on its skeleton attachment tags. Alice uses rope-hang, minecart-seat and airship-seat animations and cannot fire while attached. The airship now yaws along its flight path (its nose is the model's +X axis, where the propeller thrusts away from the -X stern), blending the heading smoothly across each path node; this is presentation only, derived from the saved flight clock, and the rest pose and the boarding beat keep yaw 0 as authored.
- Four conversations with 15 original voice recordings/subtitles are connected. The wall mechanisms use their reviewed original linkage, movement and timing declarations (see below).

## Verification

`--pandemonium-route-check` starts at the normal entrance and replays normal walking, rope use/climbing, six jumps, E interactions, dialogue advancement and Blade throws. It collects Cards and the key, reaches the return portal, enters the house and receives the Fortress 1 transition. The verified run (re-measured 2026-09-29; the driver is unchanged) takes **21,730 fixed ticks**, uses **11 throws**, and finishes alive with **57 Sanity**. Six guards die and one return guard is bypassed. It uses no free flight, recovery, resource refill, debug warp or direct puzzle-state edits. The authored cart landing, return portal and airship boarding transfers remain part of the actual level mechanics. Navigation searches plan inputs and replay them through live collision, movers and enemies; aiming is automated. This is an input simulation, not a claim of a complete human playthrough.

`--pandemonium-check` covers premature gate contacts, cart landing at 30/60/144 Hz, pause, key pickup, opening-door obstruction/resumption, rope collision and release, malformed state rejection, state restoration with continued simulation, return guards, and the dialogue-gated one-shot exit.

`--pandemonium-render-check` produces staged private GPU captures of ropes, the cart, key, return gate and airship (boarding, and the flight at its start, mid-turn and end: `pand-flight-start.png`, `pand-flight-turn.png`, `pand-flight-end.png`). These are visual fixtures, separate from traversal proof. The native save writer/reader checks cover cart, key, return and flight states in separate processes, alongside the existing campaign cases; an actual v0.22 Pandemonium save is used to verify upgrade compatibility. See VALIDATION.md for results of this build.

## Cinematic refinement

The current pass restores separate track-collapse timings, moving pieces and sound cues, original-sprite disappearance particles, cart sitting/bump timing, trackside gnome headings, white/grey fades and synchronized ship/cast animation clocks. See [cinematic refinement and verification](VILLAGE_CINEMATICS.md). Current route checks complete in 21,765 ticks watched and 21,150 skipped, with six jumps, 18 throws, one authored return portal and 70.6 Sanity; the earlier measurements above are historical.

## Remaining blockers and fidelity work

No blocker remains on the tested Pandemonium entrance-to-exit route. The four main scenes have camera tracks, acting and disappearance staging; the refinement above adds original-sprite particles, distinct collapse timelines and corresponding audio. Exact spline interpolation, head tracking, particle lighting and boarding arcs still have fidelity limits. The return room's unnamed rear panel uses a reviewed approximate opening motion. Ropes are independent constrained pendulums, without the original full physics and climbing animation set. Original lip-sync is supported; see [FACIAL.md](FACIAL.md).

The Fortress 1 transition now enters its authored playable arrival rather than the dark cinematic staging room. Both Fortress of Doors visits have continuous input-route checks; see [FORTRESS.md](FORTRESS.md). **Beyond the Wall (`fortress2`) now has a verified main route:** its moving corridor, levers, rolling walkway, rising staircase and shuffled return doors are restored; see [BEYOND.md](BEYOND.md). The full Fortress 1 arrival cinematic is also deferred. These changes do not establish a complete village-to-school or whole-campaign playthrough.

All original map/model/audio/script data remains in the user's local archives. Private captures, fixtures and original script extracts are excluded from the local source-review package; nothing is published.

## Wall machinery (v0.31.6)

The 16 inline machinery parts now use the original map's authored pivots and the reviewed Widget, Legs and Cam thread declarations. Widget children follow the main arm's position and rotation, with their own counter-rotation. Each leg has a thigh/calf/foot hierarchy, a one-second starting pose, and a staggered four-stroke cycle. Pistons move vertically eight units without rotating. The fixed wheel receives the two legs' non-overlapping power strokes on its X axle. Cams turn on their Y (source pitch) axles, while their arms follow eight translation/tilt segments with unequal durations and separate one-/three-second startup delays. This replaces independent spinning and disconnected joints.

Steam is emitted during each leg's downstroke, with existing puffs allowed to expire. These presentation intervals preserve the shared entity activation flags. All mechanism poses and emission intervals derive from the existing saved level age; no save-format change or new game is needed. The clock respects pause and the existing Pocket Watch world-time scaling. Original mover audio loops remain unfinished.

`--pandemonium-check` verifies all six joint mounts, fixed wheel/arm pivots, piston travel, stroke-boundary continuity, and serialized phase restoration against the real map. `--pandemonium-machinery-render-check` renders the moving assemblies and particles from staged inspection cameras; it is also included in `--pandemonium-render-check`. `--pandemonium-route-check` verifies ordinary entrance-to-exit progression separately. These changes are specific to Pandemonium; they do not establish fidelity for every campaign map's mechanisms.
