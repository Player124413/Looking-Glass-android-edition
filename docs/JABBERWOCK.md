# Jabberwock: both major battles

## Eye altar visibility correction (2026-10-01)

The placed Eye Staff altar emitted its purple plume during the introduction and
survival fight even though the encounter hid the reward model. The encounter now
suppresses that ambient copy by its BSP entity identity. Its own reward art is
the only altar effect, starting at the authored reveal and ending on collection.
The reward model and effect share the same visibility predicate. The second
Jabberwock encounter has no Eye altar and keeps its ordinary ambience.

The native check now includes world particles alongside encounter art. It proves
the old ungated plume, absence during intro/fight, reveal timing, collection and
restored visibility. All 27 weapon altar placements were audited: 24 ordinary
pickups follow availability/collection, and the three scripted Staff component
altars follow their owners. Hatter's blade and the forest Staff already had the
required controls; the forest return hides its former Staff altar. Complete boss
routes and separate save writer/reader checks also pass. Evidence and the frozen
release source are in `private/jabber-altar-fix/`.

## Fire direction correction (2026-10-01)

Ground and flying fire now launch along the animated mouth attachment's forward
axis. Previously, the mouth supplied only the start point: the carrier aimed
at Alice and gained a distance-based upward velocity, so the fire could leave
sideways or climb while the head faced another way. Each new packet now follows
the head's pitch, turn and animation sweep. Emitted packets continue their own
trajectory as the head turns; their gravity and collision behavior are retained.

The bright mouth jet also uses the animated attachment's local velocity axes;
its previous world-space velocity could point behind the head. This applies to
the breath emitter only, preserving the smokestacks and other ambient effects.

The separate aimed spiral attack, attack timing, speed, damage, lifetime and
save structure remain unchanged. Existing in-flight projectiles retain their
saved velocities. Both encounters share this fix.

Verification uses the original rig at every ground/flying emission frame,
three body headings and four target positions: 144 launch cases per encounter.
It also checks fire damage, blocked shots, gravity, spiral aiming and exact
saved continuation. Both complete encounter routes, native ground/flying
captures and separate save writer/reader processes pass. Evidence and the
compiled source snapshot are in `private/jabber-fire-fix/`.

The `jlair2` and `grounds1` registrations use the shared actor in
`src/jabberwock/`. They have separate progression, scene and save state. Models,
animations, speech, cameras, brush geometry and authored markers are read from
the user's installed original data; no original assets are distributed here.

## Playable encounters

- **Jabberwock's Lair (`jlair2`)**: original introduction, invulnerable ground
  boss, fire/melee/eye attacks, a 90-second survival clock and two groups of
  three Jabberspawn at 30 and 60 seconds. Gryphon interrupts the battle, the
  Jabberwock loses its eye, Alice collects the visible Eye Staff reward, and
  progression returns to `wforest$wforest_start2`.
- **Royal Rage (`grounds1`)**: Gryphon/Jabberwock introduction and sky changes,
  2,000-health boss, takeoff, flight attacks and dives, landing at 1,000 health,
  ground pursuit/attacks, pain and death. The Gryphon farewell leads to the
  lowering drawbridge, Cheshire's advice, the gnome/guard scene, opening castle
  doors and `grounds2$grounds2_start1`.

Both battles use ordinary player movement, weapons, damage, difficulty scaling,
essence healing and recovery. The boss ignores the Pocket Watch as in its data;
Jabberspawn and spiral projectiles pause. The survival timer still runs. Combat
advances in fixed 120 Hz steps, checks cover/solid surfaces, and bounds waves,
projectiles, attached effects and dynamic lights. Reward and exit state are
latched; failed transitions can retry without duplicate grants.

Scene skipping uses the same checked handoff positions and progression as the
watched scenes. Versioned saves retain boss action, aim, health, shots, waves,
survival clock, dialogue progress, reward, bridge, door and epilogue state.
Invalid saves are rejected before mutation. Old saves without these owners
restart the encounter at its reviewed entry.

Shared fixes forward animation sound cues and Will drain from level controllers
and restore Royal Rage's initial and cinematic sky viewpoints. A shared animation
loop boundary fix prevents a floating-point frame index crash encountered in
Royal Rage's live introduction; its regression test exercises the exact boundary.

## Validation

Use the normal original-data setup described in INSTALL.md. Headless checks:

```powershell
cargo test --locked
looking-glass.exe --jlair2-check --no-audio
looking-glass.exe --grounds1-check --no-audio
looking-glass.exe --jlair2-route-check --no-audio
looking-glass.exe --grounds1-route-check --no-audio
$env:JABBER_SKIP='1'
looking-glass.exe --jlair2-route-check --no-audio
looking-glass.exe --grounds1-route-check --no-audio
Remove-Item Env:JABBER_SKIP
```

The routes use Normal difficulty, real combat/movement, healing and the ordinary
exit/arrival path; neither god mode nor notarget is enabled. Contract fixtures
separately test 30/60/144 Hz timing, pause, Pocket Watch policy, invulnerability,
half-health landing, completion gates, corrupt saves and scene asset references.

Run `--jlair2-render-check` and `--grounds1-render-check` on a graphics-capable
desktop to capture representative scenes, attacks, reward, flight and death.
Captures remain under `private/jabberwock/captures`. The checks assert no dropped
transparent submissions. These staged captures are separate from route proof.

For native save checks, run each render flag with
`LOOKING_GLASS_JABBER_SAVE=write`, then run it again in a new process with
`LOOKING_GLASS_JABBER_SAVE=read`. Each checks introduction, combat, outro,
reward/drawbridge and completed state (plus Royal Rage ground and death states)
through the production Store and full
restore path, including actor art reconstruction. Saves remain private.

The integrated project passed all 621 unit tests on 30 September 2026. Both
full routes passed with scenes watched and skipped, and all 12 native save
fixtures passed in fresh reader processes. The release build and both launchers
also passed smoke checks. Clippy on the source snapshot completed with 22
existing warnings and none in the new encounter modules; a global
`-D warnings` run remains blocked by those baseline warnings.

## Remaining original-game parity limits

These are playable reconstructions, not a claim of frame-perfect original AI.
Native steering, beam pulse cadence, dive/jump decisions and projectile bounce
response are approximations where script/TIKI data delegates to unavailable
engine behavior. Authored camera paths, animation clips and dialogue are used,
but portions of aerial choreography and root-motion alignment remain approximate.
The gnome's falling guard uses bounded kinematic motion. The moving drawbridge
pauses if it would embed Alice instead of reproducing the original crushing
damage. Original game audio assets are wired in; automated runs use no audio.
