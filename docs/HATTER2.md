# Mad Hatter — About Face

The `hatter2` controller owns the Mad Hatter battle and its onward route. It uses the user's local models, animations, effects, voices and camera paths. No original script or AI program executes.

## Implemented

- The arrival bridge is solid immediately and closes after Alice crosses it. Arrival restores resources once and disables the Cheshire summon and Demon Dice allies.
- The centre blade is a distinct quest component, recorded in persistent pickup history. It does not grant the completed Eye Staff.
- Cheshire's first appearance starts the four-stage, 96-second clock cycle. The minute and hour hands move, gongs mark the stages, and the Hatter shrinks, changes between tower and arena, then grows again.
- The boss has 2,600 health, locomotion, damage reactions, directional cane/slap attacks, ballistic cups and homing syringes emitted from the animated attachment tags. Melee contact and projectile release use the reviewed source animation frames. Winding up a melee strike commits his facing so Alice can dodge.
- East and west Clockworks emerge through the timed doors and launch into the arena. They share the existing Clockwork combat controller. The tea phase clears surviving reinforcements; their death rewards expire and cannot be collected twice. At most two reinforcements, four drops and 24 Hatter projectiles exist.
- The final 100 health starts the breakdown: start animation, three malfunction clips, then the final collapse. This works on the tower as well as in the arena. The supplied animation events control the cane, hat, surfaces and attached sparks.
- Victory stops the arena clock and starts the alternating reward platforms. Rider movement and platform collision use the same transforms; blocked machinery rolls back rather than embedding Alice.
- Collecting both the blade and the Pocket Watch enables Cheshire's second appearance, the exit doors, the teleport and the Gryphon escape. The scene includes the opening cage, dialogue, camera paths, Alice's riding attachment and the breakaway ceiling. Watching or skipping commits the same transactional `jlair1$jlair1_start1` departure.
- Encounter state, phase timers, projectiles, reinforcement deaths, rewards, moving supports, dialogue progress and exit delivery are saved. Older controller-less visits restart at the entrance while retaining carried resources.

## Deliberate approximations

Syringe steering uses a bounded 90 degrees/second interpretation of the supplied seeker setting; its native conversion has not been recovered. Cups use the supplied speed and gravity with a simple ballistic aim correction. Cups and syringes apply direct contact damage; their secondary explosion-effect radial damage is not reproduced. The tea impact uses a reviewed 300-unit radius; its native default radius is not supplied in the effect definition. The malformed timer model supplies timing only, not a mesh. The moving discs use 112-by-112 support boxes around their walkable tops; the tall decorative rods do not form solid walls. The patch-only pendulum is animated but has no additional solid collision. Ceiling debris follows bounded scripted motions and is cosmetic. Camera and path playback use the engine's existing independent spline implementation. These are playable compatibility behaviors, not a claim of frame-perfect original parity.

## Verification commands

The clock floor retains its authored opaque lightmap base before its detail,
face and pulsing glow layers. Removing that base caused black triangular seams
and overlapping numerals. Both scripted Cheshire placements now receive the
normal actor floor placement (arena cover z=0, Watch perch z=396) and use the lit,
mouth-animated dissolve renderer. This is derived when loading the map, including
existing saves; no save-format migration is needed.

`--hatter2-check` checks clock timing at 30/60/144 Hz, reinforcement limits, pause, state restoration, death while on the tower, reward gates and rejected saves. `--hatter2-route-check` uses production movement and weapon inputs through the battle and departure. Set `LOOKING_GLASS_WATCH_SCENES=1` to watch instead of skipping the dialogue scenes during this route.

`--hatter2-render-check` creates native staged captures under `private/hatter2/captures`. With `LOOKING_GLASS_HATTER_SAVE=write` or `read`, it instead exercises native Store snapshots in separate processes. These staged checks are distinct from the input-driven route. Run native checks on the hidden test desktop with `--no-audio` when that desktop has no audio device.

The headless check also verifies both Cheshire support heights. Native captures
include the complete clock floor, its close Cheshire conversation and the Watch
Cat on the ledge.

This work covers the About Face boss encounter. It does not claim completion of every puzzle and cinematic in the preceding `hatter1` map.
