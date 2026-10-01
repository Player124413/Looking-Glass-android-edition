# Mirror Image performance and original mirror appearance

The original Funhouse mirror shader uses an environment-mapped image and a
border overlay. Its BSP has no portal camera entities. This map keeps that
authored appearance, without adding a live reflection of Alice. Existing live
mirrors in other maps keep their established behavior.

Funhouse machinery now caches local collision shapes and places them lazily at
each moving pose, preserving its brush-only collision contract. The build also
inherits the published broad-phase collision optimization. AI update rates,
encounter timing and the save format are unchanged.

## Verification

`--funhouse-check` includes 7,614 comparisons against the previous collision
builder, plus existing mover, watched/skipped camera and saved-state checks.
`--funhouse-render-check` and separate `LOOKING_GLASS_FUNHOUSE_SAVES=write` /
`read` processes cover map presentation and nine stored phases.

`LOOKING_GLASS_FRAME_PROFILE` optionally names a private output directory for
CPU summaries from a bounded `--frames` run. Inactive frames are excluded from
frame/AI samples. Drawing timings measure submission, not GPU time or FPS.
Normal play records nothing. Evidence: `private/mirror-performance-fix/`.
