# Icy Reception

`garden4` now owns the full frozen approach and its Caterpillar portal. Start it
with `tools/launchers/Launch-Icy-Reception.cmd`, or select Icy Reception with Tab in the normal
launcher. Save envelope 12, route index 15 and reserved hit base 7,500,000 remain.

## Frozen approach

All 14 breakable-floor brushes, 13 static ramp brushes and eight blocking-wall
slabs have matching drawing and collision. The floors drop and turn in their
authored groups when the rolling-rock waypoints fire their callbacks. The first
two collapses are inhibited on Easy; the altar sequence still runs there through
its separate pickup-sized contact volume. Repeated rock activation is idempotent.

The seven rocks use the shared timed waypoint simulation: per-node speeds,
gravity, continuous player contact, rolling models, callbacks and saved motion.
Ice rock collision bounds come from their supplied models. The altar pair and
wall-smashing boulder remain harmless. The second end boulder and marble retain
their damage. Collapse/bounce cues and bounded camera shake accompany the motion.

The unskippable marble scene uses its two supplied camera tracks. It removes the
lower ice and returns Alice to the authored marker, settled on supporting ground,
with a checked lead over the marble. The end boulder opens the wall in eight
staggered pieces. Crossing the fog trigger restores the return-blocking brush;
the shared fog change and both authored steam vents remain active.

## Caterpillar and moving exit

Trigger #49 starts the Caterpillar's ten dialogue beats, with original voices,
subtitles, lip/head bindings, Alice/Caterpillar acting and eight camera tracks.
The closing performance includes smoke, bounded drugview and the portal reveal.
Watching restores control after the 14-second reveal. Skipping retains the closing
line and 5.5-second reveal, then waits four more seconds before enabling the exit.
Holding skip cannot restart the closing performance.

The initially hidden portal #8 follows its six-node path. Its bound exit #6 uses
the same transform and rotated offset every frame, so its editor position is not
an early exit. Both endings commit the same supported Alice pose and portal
endpoint. Only a completed scene with an expired unlock delay allows entry into
`centipede1$centipede1_start1`. Failed level loads retain the exit retry latch.
No resources or upgrades are granted by the scene.

## Persistence and verification

Floor, wall, rock, scene, speech and unlock clocks are saved. Pause freezes them;
malformed rock states are rejected before replacing live state. Older scene-only
saves preserve consumed cave events, so a previously used wall trigger cannot
strand the player. They do not gain a completed Caterpillar scene. Controller-less
saves likewise keep the portal closed and recover consumed encounter triggers.

- `--garden4-check`: all four difficulty configurations, floor/wall callbacks,
  harmless rocks, fog backstop, pause/restored motion, 30/60/144 Hz marble handoff,
  watched/skipped scene boundaries, interruption, old saves and exit retries.
- `--garden4-route-check` and `--garden4-skip-route-check`: continuous Normal/Easy
  traversal from the entrance, Ice Wand pickup, marble escape, wall passage,
  both steam vents, Caterpillar and normal entry into Fungiferous Flora with
  carried inventory/resources. These drive ordinary movement; scene handoffs
  are the only scripted relocation.
- `--garden4-render-check`: nine explicitly staged native captures.
- `LOOKING_GLASS_SAVE_CASE=garden4-` with separate `--save-check-write` and
  `--save-check-read` processes: nine native persistence fixtures, including
  falling ice, moving wall, marble camera, dialogue, smoke and delayed exit.

The route check covers world progression, not a full Ladybug combat balance pass;
placed Ladybugs retain the shared resident controller. Exact original drugview
distortion and portal rotation semantics remain visual-fidelity research. Native
checks are silent, so they do not establish audible playback quality. Original
scripts/assets and private captures are not part of the source package.
