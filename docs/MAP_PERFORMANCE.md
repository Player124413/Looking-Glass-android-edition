# Maze and Clockwork performance audit

Measured 2026-10-01 against the supplied Alice maps `hedge2` (Mystifying Madness),
`hedge3` (Labyrinthine Revenge), and `hatter1` (Crazed Clockwork).

## Causes and changes

The dominant maze cost was enemy collision queries, particularly long sight lines.
The old BVH query used the axis-aligned box enclosing the entire segment. Diagonal
queries consequently visited brushes far from the actual ray, repeatedly for each
enemy simulation step. `SweepBounds` retains that inexpensive initial rejection,
then intersects the segment against bounds expanded by the swept body's half-size
and collision skin. Both tree nodes and individual static/dynamic hulls use it.
Convex clipping, hit ordering, movement, AI frequency and detection ranges are
unchanged. No enemies are disabled or slowed down.

Clockwork reconstructed local brush vertices, edges and transformed planes for
moving machinery every frame. It now retains a brush-only `model_shape::Template`
per object and lazily places collision hulls when queried. This preserves the old
brush-only collision contract (it does not introduce patch collision there).

Labyrinthine Revenge also republished the entire dynamic collider list after each
moving object. Contact branches already publish current poses excluding their own
pusher, and non-contact branches query object colliders directly. One final publish
now replaces those redundant list copies, including frames with no moving parts.

## Measurements

Release builds, native hidden Windows desktop, 1200 × 680, audio disabled, same
entrance cameras. Each ordinary viewer run used 240 frames with the first 20 samples
discarded. These are median CPU frame-processing times, including rendering
submission; they exclude presentation pacing and are **not GPU timings or FPS**.

| Map | Before | After | Reduction |
| --- | ---: | ---: | ---: |
| Mystifying Madness | 30.021 ms | 6.507 ms | 78.3% |
| Labyrinthine Revenge | 54.090 ms | 10.737 ms | 80.1% |
| Crazed Clockwork | 8.564 ms | 6.032 ms | 29.6% |

To separate simulation work from frame-rate feedback, a second native-cast replay
ran 360 identical 120 Hz input ticks. Before/after enemy-update medians were
3.835 → 0.210 ms (`hedge2`) and 11.244 → 1.984 ms (`hedge3`). All three final replay
checkpoints were byte-identical. Clockwork's headless mover-update median fell
from 1.194 to 0.311 ms. Timings vary by scene/hardware and background workload;
these are representative entrance measurements, not an exhaustive map-wide claim.

## Regression evidence

- A deterministic 10,000-sweep synthetic test compares every trace field with the
  original broad phase, including stationary and vertical sweeps, different body
  sizes, and starts inside hulls. Its diagonal query also proves a reduction in
  candidate count, independent of wall-clock timing.
- The supplied-map test compares another 60,000 sweeps across all three player and
  actor collision worlds, plus probes starting inside sampled hulls. Fractions,
  normals, `start_solid` and `all_solid` match exactly.
- 31 paired native staged captures cover gates, machinery, cutscenes, particles
  and skies. 28 match pixel-for-pixel; the other three differ by one 8-bit step in
  at most eight pixels (driver rounding). No geometry or material setting changed.
- Existing level contract checks cover moving supports, doors, hazards, pause,
  restored phases, scene skips and Clockwork's actual Hatter-arena arrival.
+- The combined build passes 660 unit tests. Native traversal reached `tower2`
+  from Mystifying Madness with both watched and skipped cutscenes, `tower3` from
+  Labyrinthine Revenge, and `hatter2` after all five Clockwork levers. The long
+  Labyrinth replay exceeded the initial four-minute runner limit; it continued
+  from the verified bellows-clear disk checkpoint and completed 40,486 cumulative
+  ticks with no recovery teleports. Clockwork used its two authored teleports.
+  The final executable retains the subsequently published Will-recovery fix and
+  reran the three level contracts and power-up checks.

Run `cargo test` for the synthetic regression. To run the original-data trace
comparison, set `LOOKING_GLASS_TEST_DATA` to the supplied game's `base` directory,
then run `cargo test collision::performance_tests::original_maze_and_clockwork_traces_match -- --ignored`.
The level checks are `--hedge2-check`, `--hedge3-check` and `--hatter1-check`.
Windowed route/render checks belong in the background desktop.

Diagnostic executables, exact instrumented sources, CPU profiles, checkpoint
comparisons, captures and final packaging evidence are retained locally under
`private/map-performance-fix`. Profiling instrumentation is excluded from the
normal playtest build. Save format and persistent controller fields are unchanged.
