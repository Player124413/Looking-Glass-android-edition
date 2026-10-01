# Red King — Checkmate in Red

The `rchess1` controller restores the Red King battle and the surrounding visit.
It reads models, animations, map markers and camera paths from the local data;
reviewed Rust controls the encounter. Original scripts and AI programs never execute.

## Playable behavior

- The castle's ten ambush groups use the existing pawn, knight, rook and bishop
  combat, including group-five suppression, reactions, deaths and one-time rewards.
- The White Queen execution uses the original cameras, separate head/body models,
  blade movement and spectator departure. The upper route and arena drawbridge
  remain available at their appropriate points.
- The King appears during his introduction, then fights with 1,300 health,
  a damage-once melee strike, four-pulse beam, bouncing explosive balls,
  diamond shots and an incoming-projectile counter with a seeking ball.
  Emission uses animated `tag_ball` and `tag_barrel` transforms.
- Pain reactions, directional deaths, frozen death, a boss meter and the three
  cycling medium essence positions are supported. Dice allies are dismissed
  for the arena; the shared Watch slowdown remains effective.
- Defeat waits for the King's death and shrink before bringing back the platform,
  transforming the pawn into the Queen, revealing the royal pieces and showing
  the Hatter's strike and the fall into `funhouse$funhouse_start1`.
- Scenes can be watched or skipped. The arena writes a checkpoint request;
  reload preserves battle state. Exit requests are committed once and retry
  safely if destination loading fails.

## Persistence and bounds

The controller's version-one payload is inside the existing save envelope. It
stores phases, scene clocks, bridge/platform state, all chess receivers, boss
health/action, recoil, shield recovery, projectiles, essence position/cooldown
and exit status. Restores reject malformed or inconsistent state before mutation.
Boss simulation runs at 120 Hz with at most 16 shots and four beams. Finished
shots and their effects expire; death clears live hazards. Pause and dead-player
states do not advance the encounter.

Pre-controller saves use the shared visit upgrade path and restart this visit's
new encounter state at a safe entry. Previously defeated generic chess receivers
are not imported into the new controller; start a fresh visit for this playtest.

## Fidelity limits

The original AI's descending `HEALTH` tests are interpreted as lower bounds:
beam at 1,000–1,300 health, balls at 500–999, diamonds below 500. This is a
compatibility choice that makes all three authored branches reachable; native
condition semantics have not been confirmed. Local steering, random attack
selection, pain cooldown, shield direction/window and projectile steering are
bounded approximations. Bodies are non-solid to Alice. Explosion radius follows
the shared damage-plus-60 convention, with cover checks and no doubled direct hit.

The beam uses a depth-tested line/core effect rather than the original beam
shader. Models and attached effects use the production material queue. Spectator
exits, royal-piece movement, Alice's walking and intermediate camera fades are
approximations. The Hatter uses his authored portal mark after unbinding so his
strike reaches Alice. Optional asylum scenery is rendered, but this is not a
complete restoration of every optional scene or original head-tracking behavior.
Audible mixing has not been verified; native checks run without audio in Anode.

## Verification

`--rchess1-check` checks source tags/clips, hidden/dead target gates, attacks at
30/60/144 Hz, pause, restore, health stages, melee contact, counter activation,
frozen death, invalid saves and the one-shot exit.

`LOOKING_GLASS_KING_ARENA=1 --rchess1-route-check` starts at an explicitly
staged bridge entrance (set the environment variable before running the flag).
`LOOKING_GLASS_WATCH_SCENES=1` watches scenes instead of skipping. Both variants
pass and are only evidence for the boss segment. They defeat the King with ordinary
Blade/Cards inputs, taking real damage and collecting essence, then arrived
safely in the Funhouse with all four ranged/counter branches used (95 sanity left).

The separate `--rchess1-route-check` is an incomplete full-visit replay. It reaches
the swim, rope climb, third Demon Die, execution, and northern castle route through
normal movement and combat. It currently dies during the northern rook fight;
a version that runs past the rooks instead dies on the final drop with one sanity.
The full approach is therefore **not verified as an uninterrupted playthrough**.
These failures are retained for follow-up instead of refilling resources or disabling
enemies. Private movement variants can use `LOOKING_GLASS_KING_ROUTE_INPUT`; those
files only express navigation, movement and fighting, never world-state edits.

`--rchess1-render-check` captures twelve staged native views, including the
execution, bridge, attacks, frozen death, Queen and Hatter ending. Setting
`LOOKING_GLASS_KING_SAVE=write` then `read` in separate native processes tests
seven real Store snapshots from approach through committed exit. These are
explicit fixtures, not claims of uninterrupted gameplay. Evidence is local in
`private/rchess1/`.

Use `tools/launchers/Launch-Red-King.cmd` for the installed private candidate and separate
playtest saves. The default player launcher and existing saves are untouched.

Validation on 30 September 2026: 627 unit tests pass, including four new King
regressions; the shared chess audit passes. Native captures and all seven
separate-process Store cases pass. Audio was disabled.
