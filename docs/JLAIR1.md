# Burning Curiosity (`jlair1`)

`tools/launchers/Launch-Burning-Curiosity.cmd` starts the restored visit at `jlair1_start1`.
The normal exit leads to `jlair2$jlair2_start1`, where the existing Jabberwock
survival encounter takes over. Local original game assets are required.

## Restored behavior

- Gryphon arrival: Alice rides the animated `tag_alice` attachment along the
  authored flight path, dismounts, hears the Gryphon's dialogue, and watches the
  departure. Watching and skipping place her on the same checked floor.
- The lava platform uses the original brush, 100-unit speed and 128-unit sink
  limit. Its visible and collision poses agree. It carries a rider down,
  recovers when unoccupied, and transfers the rider onto the uneven pool bed
  without dragging her through it. Scenes and pause stop the mover.
- The map's native enemies use their existing combat systems and original
  activation groups. Three spawned imps now follow their monster-only launch
  pads once, then resume normal movement. Pending and airborne launches survive
  saves. This visit has 41 supported imps; the original two `t157` placements
  have no activation and remain absent.
- The Caterpillar scene includes the single Cheshire apparition, the reveal
  zoom, original camera tracks, dialogue and actor gestures. Scene actors are
  removed from ordinary placement, so they cannot appear early or be cloned.
  Watching and skipping commit the same position and completion latch.
- The two final triggered meta-essences spawn and remain available until
  collected. Their activation and individual collection flags persist.
- Original route semantics are retained: the Caterpillar speech does not invent
  an exit lock. The exit is inhibited during a scene and uses the authored
  destination. Missing original `magmaman03` remains a no-op.

## Persistence

The versioned controller stores scene kind, speech cursor, gesture epochs,
reveal/ending/skip clocks, arrival and Caterpillar completion, sink displacement
and the two essence flags. Invalid state is rejected before changing the live
controller. Older saves without this controller resume free exploration and
rearm the previously unimplemented Caterpillar trigger instead of replaying the
arrival. Global save format remains unchanged.

## Verification

- `--jlair1-check`: rider carry/recovery at 30, 60 and 144 Hz; pause and corrupt
  state rejection; both scenes on their full dialogue clocks, watched and
  skipped; equal handoffs; no scene replay; restored scene/essence continuation;
  duplicate Jacks Will refill; a separately staged ordinary exit contact.
- `--jlair1-traversal-check`: continuous fresh entrance-to-exit traversal with
  production movement and scene handling, without the native enemy cast.
- `--jlair1-route-check`: the same continuous route with the native cast,
  Normal difficulty and ordinary combat. Set `LOOKING_GLASS_JLAIR1_SKIP=1`
  for the skipped variant. The watched route advances dialogue interactively;
  the full unhurried dialogue timing is checked separately above. Route checks
  reject death, recovery, cheats, authored teleports and a wrong exit, and
  use the carried Pocket Watch before the last ledge to protect the pull-up, and
  compare all ten carried weapon slots, the Staff component and resources
  before starting the destination scene. They then require the survival boss
  to become active.
- `--jlair1-render-check`: staged native captures of flight, landing,
  departure, Cheshire, the reveal, Caterpillar and both platform poses.
- The same render command with `LOOKING_GLASS_JLAIR1_SAVE=write`, then `read`
  in a separate process, exercises the production Store/Continue loader at six
  moments: flight, landing dialogue, a partly sunk platform, Cheshire speech,
  Caterpillar speech and completed scenes.
- Fire Imp unit checks cover launch activation, pause, midair save continuation
  and one-time landing. Existing `--jlair2-check` covers the downstream encounter.

Private recordings and diagnostics live under `private/jlair1-work`; extracted
original scripts and dialogue are not distributed. Native visual tests run in
Anode's hidden desktop with audio disabled. Audio listening and a full
performance sampling campaign are not covered by these checks.

## Recorded acceptance run

The selected release retains Water Logged and the previous restorations. It
passes all 626 unit tests plus the Jabberwock and Water Logged controller checks.
The integrated shared workspace also passes `cargo check --locked`.

Both fresh native routes activate all 41 supported imps and every resident
ambush group. The skipped run uses 18,851 physics ticks and 76 jumps; the watched
run uses 21,230 ticks and 76 jumps. Both finish with 91.58 Sanity, no recovery,
no cheats, no authored teleports, and exactly the intended exit. Both reach the
active Jabberwock survival encounter with all ten weapon counts, resources and
the Staff component preserved. The route uses the carried Pocket Watch before
the exposed final climb, where enemy knockback can otherwise interrupt a pull-up.
Brief lava contact (220 physics ticks) is survived; these are not damage-free runs.

All eight native captures and all six Store write/fresh-read cases pass on the
same selected executable. See `private/jlair1-work/release.json` and its frozen
source manifest for the build identity and detailed evidence.
