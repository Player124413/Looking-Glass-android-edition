# Ambient detail audit — 2026-10-01

Scope: follow-up to `ITEM_FX_AUDIT.md`, using the locally supplied, override-resolved assets, the native client binary and the current main working tree. Identifiers, numbers and paraphrase only. This pass covers small environmental presentation details, not every remaining campaign issue.

The static decoration check validates 1,251 placements and their materials, including six mover attachments and 24 avoided baked duplicates. It separately records 40 placements requiring a scene/controller. That deferred count is not a count of missing objects: their existing owners can draw them during the appropriate scene. The per-map inventory is retained in the private headless log.

## Confirmed fixes

| Detail | Finding and fix | Confidence |
|---|---|---|
| Lantern pixies | Register the `lantern.tik` emitter for 24 `gvillage` and seven `pandemonium` placements. Read the `lantern` animation and `tag_flame`; retain the 0.1/s rate, ten-second lifetime, 3.5-unit bounds and base velocity `(5,5,5)`. Reverse motion at the bounds. Use the 16-pixel sprite dimensions and seed one light at entry. | High on missing binding and numeric properties; motion is a bounded reconstruction, not identical native random samples. |
| Firefly glow | All 15 `firefly.tik` placements now sample the supplied image style. Read up to 128 pixels from its first row; interpolate RGB every 0.05 seconds, hold the final pixel for one tick, then wrap. The saved presentation clock determines phase, so pausing and restoration do not advance it. | High on image format, sampling rule and placement coverage. Exact original lighting intensity is not claimed. |
| Vertical emitter orientation | Use the shared placement rotation for special angles `-1` and `-2`, which previously became almost-zero horizontal yaw. Affects `funhouse:1157/1171/1172`, `garden1:53` and `hedge3:818/823`. | High on placement values and the orientation mismatch. |

World light styles replace the emitter's RGB, matching the native dynamic-light path. Other already supported world emitters can use the same image-style reader when their referenced image exists. Text-based styles and actor-owned lights remain outside this change.

## Native evidence

Read-only inspection of `cgamex86.dll`, image base `0x30000000`:

- `0x3000FF00`: three constraint dimensions; `0x30010010`: three base-velocity components.
- `0x300154D0`: particle initialization; `0x300122A0`: axis-limit checks and reversal. A negative limit leaves that axis free.
- `0x3000CAE0`: style registration; `0x3001CD80`: image colour strip; `0x3001C720`: 50 ms sampling, interpolation and final-sample hold.
- `0x300179E0`: dynamic lights use the sampled RGB.

The lantern uses the existing world-particle simulation with its axis bounds. It does not request world collisions: the native trace is gated by `0x800`, independently of constraint flag `0x1000`. Applying ordinary sprite collision killed the pixie inside its enclosure in the first native fixture; enclosed pixies now keep their lifetime and remain depth-occluded. Transient particles are recreated on loading; no particle history or new save fields are introduced. The image light phase is reconstructed from the existing presentation clock.

## Verification and boundaries

Private evidence and frozen compiled sources: `private/minor-details-20261001/`. The native fixture enumerates all 31 lanterns and 15 fireflies, checks image styles, pause/restored clocks, constrained particles and repeated paused frames. Existing fidelity fixtures continue to cover world rendering. Unit tests cover bounds, reversal, disabled emission, an unconstrained axis and image sampling.

Save version 12, event keys, visit order, hit-ID ranges, pickup grants and combat/route conditions are unchanged. No original assets, dialogue or script prose are added to the repository.

Remaining recorded details include the machine-owned finger effects, Duchess fireplace staging and actor-owned light styles. These require their owning performances and activation conditions; this pass does not mark them complete. The separately tracked carried campaign run remains at visit 28.

Integration: the combined release retains the separately verified school-floor renderer fix and the 8 MiB Windows main-thread stack fix. Their records are in `private/skool-floor-fix/` and `docs/LEVEL_SWITCH_CRASH.md`.

Final validation for build **53C745AE**: 650 unit tests, 12 headless checks, school-floor, world-fidelity and pickup-effect native suites, paused-frame comparisons and an existing-save load passed. Lantern distance-boundary tests cover immediate visibility without duplicate births. The final source/executable manifest is `private/minor-details-20261001/release.json`.

Publication uses the tested source snapshot and preserves published parent DD6526FB. Concurrent death-retry work in main is separately owned and is not included in this snapshot; the manifest records those pending file differences. No pending main files are overwritten during publication.
