# Static-world movement milestone

For the current swimming, wading, low-ledge climbing and liquid-damage implementation, see [SWIMMING.md](SWIMMING.md). Rotating-door collision is covered in [WORLD.md](WORLD.md). The following is the original version 0.2 record.

Version 0.15.1 repairs rotated brush bevels, upright riders on tipping shelves and tiny ground overlaps. See [the current footing notes](RECOVERY.md) and [verification record](VALIDATION.md); the historical limitations below describe version 0.2.

Version 0.2, 27 September 2026. This controller is newly authored Rust using general convex geometry. It does not call or translate the original game's movement functions. Its speeds, body dimensions and jump values are provisional.

## Walking cadence correction (October 2026)

Walking now targets 104 units/s for a brisk pace: roughly two cycles per second
for Alice's walking clips (51.3-51.7 units per cycle). The original 210-unit
prototype target below made those clips play about four times too fast.
Animation continues to follow actual collision-resolved travel, keeping the
body and feet in step. Slow analog input animates down to 1 unit/s. Running
remains 320 units/s; swimming, acceleration, jump impulse and collision handling
are unchanged. Walking jumps cover less horizontal distance; use the run
modifier for longer jumps.

## Collision data

The BSP reader now consumes planes (lump 1, 16 bytes), brush sides (lump 10, 8 bytes), brushes (lump 11, 12 bytes), and the world model's first-brush/count values at offsets 32/36. A material's contents mask is at byte 68 of its 76-byte record. It validates finite near-unit normals and all side, plane, shader and brush range references.

Narrow Ghidra inspection corroborated the format layout: `0x0040d310` checks 12-byte brush records and reads material contents at offset `0x44`; `0x0040d730` uses 8-byte side records; `0x0040cff0` checks 40-byte models. `0x0040d3f0` and `0x0040d1e0` also confirm 48-byte leaves and 36-byte nodes, but the Rust collision implementation does not need those acceleration structures. All pseudocode remains local in `private/analysis/functions`.

World brushes with solid or player-clip content (`1 | 0x10000`) become convex hulls. The six axial planes supply validated broad-phase bounds. Curved world surfaces with solid content and without the nonsolid flag are tessellated with the same subdivision as rendering; each nondegenerate triangle becomes a thin two-sided prism with box bevel planes. Other collision content semantics are not fully implemented.

A separate bounding-volume hierarchy narrows each swept-box query. Clipping the swept centre against expanded planes prevents fast motion passing through thin walls. A small contact margin avoids numerically embedding the player on faces. Starting inside geometry is reported separately from a normal collision. Skin margin, curve tessellation and thin-prism construction remain approximations.

## Movement

- Upright player box: 30 × 30 × 56 units, feet origin, eye 48 units above feet.
- Fixed update: 120 Hz, with a pending jump press retained across rendering frames that contain no simulation tick.
- Walk/run targets: 210 / 320 units per second; ground and air acceleration differ.
- Gravity: 800 units per second squared; jump impulse: 270 units per second.
- Ground normals require Z ≥ 0.65. Contact and velocity checks use the ground normal so ascent on ramps does not falsely become airborne.
- Up to five slide contacts per tick; multiple plane constraints handle wall corners.
- Upward step trials are limited to 18 units and accepted only with a walkable landing and improved horizontal progress. Grounded motion can follow small downward steps.
- Tab and P pause simulation; Home resets; F4 changes walk/fly mode. Switching to walking requires a clear body volume. Leaving map bounds returns the prototype player to its start.

The UI uses a first-person camera until Alice's model and third-person camera exist. No weapons, interaction, health, enemy AI or progression is implied by the word "movement".

## Verification

Seventeen synthetic tests pass, including old asset-reader checks plus high-speed wall sweeps, tiny boundary moves, embedded starts, two-sided patch collision, landing, jump apex, low ceilings, wall sliding, corners, steps, slopes, jumping on a slope, fixed-step equivalence at 30/60/144 FPS and short jump press retention.

`--validate-all` builds collision for all 36 maps. `--physics-check --map skool1` executes 900 fixed ticks through settling, walking, strafing, jumping and resting. Each tick requires finite state, no solid overlap and staying above the world floor bound. The route asserts meaningful horizontal progress and successful jump/landing. It is a deterministic integrity check, not original-engine parity or full campaign coverage. Logs are kept under `private/`.

Only static world geometry participates. Scripted/brush entities, moving platforms, doors, ladders, swimming, damage triggers, enemy bodies and original Alice tuning still need implementation and comparison.
