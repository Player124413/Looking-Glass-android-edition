# Village progression — version 0.31.8

**The falling introduction and principal Rabbit/gnome scenes are now implemented.** Fresh visits include the opening, original Blade pickup scene and Rabbit's shrinking door sequence. See [current cinematic changes and limits](VILLAGE_CINEMATICS.md). Older route timings below predate these scenes; the current route takes 14,175 fixed ticks, seven jumps and three throws, with no combat damage or cheats.

## Machinery and steam correction

Version 0.31.8 corrects the sawmill's reversed slat/beam tilt and cam rotation. The three wall vents now bend in their rotating shaft's local frame, keeping their hinges and nozzles together through the two full turns. Their emitters follow the actual moving nozzle position and emit only during that vent's spinning phase. The two- and four-second delays between vents come from the original declarations. Existing puffs drift away naturally instead of following the machine after emission; the original effect defines rising, world-space steam velocity.

The three mushroom steam emitters follow their roofs with the declared four-unit offset and compression-only bursts. Roof cycles now repeat compression, dwell and release after a single initial delay. The two puff-ball steam sources stay at their original fixed outlets, emitting during ascent and the raised interval. All eight controlled emitters continue to respect saved activation flags. The current mover poses, clocks, pause behaviour, shared rendering/collision transforms and save format are retained. Reloaded older poses settle onto the corrected motion when simulation resumes.

`--village-machinery-check` checks three vent joints, nozzle attachment, eight emission controls and save restoration at 14 phases. `--village-machinery-render-check` supplies staged native views of the sawmill, vent cycle and roofs. The updated normal village route completes in 10,068 ticks with six jumps, three Blade throws, no combat damage and 100 Sanity. See [VALIDATION.md](VALIDATION.md) for logs and scope.

**tools/launchers/Launch-Village.cmd** starts a fresh visit with the falling introduction. The village has a continuously tested route through that introduction and the playable opening to its original `pandemonium$player_start` exit. Walk with WASD, jump with Space, use Shift for a longer running jump, and press E to advance dialogue.

## Progression restored in version 0.18

- The village's inline bridge pieces and machinery draw and collide in matching positions. This includes the sawmill slats/beam/cam assembly, teeter ramp/flap/roller, cam piston and arm, mushroom roofs and puff balls. Spin rods animate with the authored non-solid setting; the tentacle bridge pieces use their initialized offsets.
- Moving supports carry Alice, stop when they would trap her against another surface, and resume after she moves clear. Pause, menus and loss of focus freeze them.
- The reviewed gnome/Cat trigger beats play their existing voice/subtitles. The third gnome's completed dialogue activates the Rabbit-area guard once. Replaying a conversation does not revive a defeated guard.
- The optional shootable hatch requires the authored 100 damage, rotates open and loses collision. Touching it cannot activate it.
- Steam lift volumes provide bounded upward movement through the shared collision controller.

Follow the gnomes through the opening walkways, cross the moving machinery, descend the central path, cross the sawmill area and climb toward the last gnome and the exit. Normal exploration pickups, damage, combat and death/retry remain active.

## Where the verified route stops

The village does **not** lead directly into the school. The supplied map/script data describes this chain:

`gvillage → pandemonium → fortress1 (first visit) → fortress2 → fortress1 (return) → skool1`

Pandemonium's normal entrance-to-airship route is restored; see [PANDEMONIUM.md](PANDEMONIUM.md). Both Fortress visits and Beyond the Wall now have individual normal-route checks; see [FORTRESS.md](FORTRESS.md) and [BEYOND.md](BEYOND.md). This does not claim one uninterrupted human playthrough of the entire chain. No shortcut exit has been substituted.

## Verification and limits

`--village-route-check` starts at the playable opening and replays ordinary walking/running/jumping, dialogue advancement and Blade attacks through the live controller. It reaches the authored Pandemonium exit in 13,987 fixed physics ticks, six jumps and two throws, taking no combat damage and finishing with 100 Sanity (re-measured 2026-09-29; the driver is unchanged). No free flight, debug warp, recovery, resource refill or direct quest-state change is used. This is an automated input simulation, not a native human playthrough. Its jump search plans against the current collision scene and then replays the chosen inputs with movers and enemies active; projectile release uses the shared attack timing and swept contact code, with an eye-height launch point.

`--progression-check` verifies 30/60/144 Hz mover transforms, paused motion, hatch damage thresholds and one-time guard activation/death. `--progression-render-check` produces explicitly staged native screenshots of bridges and machinery; those are presentation checks, not traversal proof.

Mover interpolation, puff-ball rotation and collision-safe stopping remain independent approximations of the reviewed declarations. The falling introduction and principal Rabbit/gnome scenes are implemented; [VILLAGE_CINEMATICS.md](VILLAGE_CINEMATICS.md) lists their remaining fidelity limits. Steam lift behaviour is provisional: local read-only Ghidra inspection identified a special floating flag and velocity damping in the original acceleration trigger; this implementation uses bounded upward acceleration and does not reproduce the original floating controller. Steam attachment and burst timing are connected for the village mechanisms described above; particle sprite orientation/size and mover sound mixing still have fidelity limits. There is no general script interpreter. Persistent saves retain the implemented movers, triggers, enemies and dialogue; see SAVES.md.

Source contains independent Rust and asset identifiers, not extracted scripts. Game data, decompiler output, recordings and screenshots stay private and are excluded from source packaging.
