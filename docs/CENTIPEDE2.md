# Centipede arena (`centipede2`)

The visit is owned by `levels/centipede2`. It uses the supplied BSP, animation clips,
camera tracks, voices, models and particle declarations. Original scripts are not executed.

## Progression

- Arrival plays the ant escort/shove and teeth sequence. Watching or skipping commits
  the same slide position, open teeth and return barrier, without refilling resources.
  The slide temporarily prevents damage from reducing existing sanity below 30; the
  authored arena trigger removes that protection. It cannot heal or resurrect Alice.
- The Centipede has three stages, with two successful weak-point reactions per stage.
  Its animated armor boxes absorb hits. The underside at `tag_target` accepts damage
  during the crush animation before frame 25; fire remains ineffective. One attack
  window produces at most one reaction, regardless of splash damage or pellet count.
- Attacks include approach/contact, crush, acid spit, Larvae, charging, and the final
  stage's grab/toss. Acid splits into six smaller damaging projectiles on impact.
  Larvae use the shared leap, attach, suck, detach, death and freezing behavior.
- The sixth reaction plays `death_start`, three thrashes and the falling-spike scene.
  All entities sharing each spike target name move together. The original inactive
  tip movements remain inactive; every tip is retired before the climbing geometry
  appears. The settled steps support jumping, ledge grabs and pull-ups.
- Climb to the mushroom and enter its trigger. Cheshire's two conversations, eating,
  camera cuts and growth play before departure to `wforest$wforest_start1`.
  The old ground-level `c2_changelevel` volume stays disabled, even after victory.
- Medium essence cycles through the three authored stations, ten seconds after each
  collection. Demon Dice allies are dismissed in this boss arena. Recovery after the
  intro returns to the accessible arena rather than behind the closed barrier.

No boss defeat, pickup, dialogue, or departure can be committed by an unrelated or
out-of-order completion callback. Failed destination loads retry the committed exit.
Inventory, turtle-shell ownership, sanity and Will carry into the next map.

## Persistence and checks

The versioned controller snapshot owns the stage, weak-point hits, deterministic
combat clock/random state, Larvae, projectiles, grab, spike timing, essence station,
dialogue phase and exit transaction. Controller-less saves restart this changed visit
at the entrance while preserving carried resources. Pausing holds simulation clocks.

- `--centipede2-check`: weak-point boundaries, fire/armor rejection, repeated hits,
  attack timing at 30/60/144 Hz, Larvae, grab release, pause, scene skipping, death
  gating, spike completion and exit retry.
- `--centipede2-route-check`: Normal with watched intro and Hard with skipped intro;
  production movement and weapon input, all six weak-point hits, jumping/climbing,
  both conversations, growth and actual next-map loading. No victory writes or
  position warps are used to complete the route. The authored scenes own their warps.
- `--centipede2-render-check`: native frame captures of arrival, weak point, acid,
  Larvae, grab, spikes, climbing route and growth.
- `--centipede2-save-write` and `--centipede2-save-read`: eleven native Store/Restored
  fixtures, including the grabbed player, active Larvae, falling spikes, dialogue and
  a committed exit; the reader runs in a separate process. A controller-less bypass
  save also verifies safe entrance migration without changing carried resources.

## Implementation boundaries

Movement steering, fragment spread, grab interpolation and cosmetic impact fade are
independent bounded implementations. The malformed original second-Larva launch
token is treated as 250 units/s. Live Larvae are capped at 16 and acid carriers at 128;
expired slots are reused. Damage traces use the animated tag boxes; player separation
is handled independently so a solid enclosing box cannot hide the weak point.
The damage window, stage count, attack frames, spike groups/distances and exit timing
come from local asset facts. This is not a claim of identical native-engine physics.

Private research extracts, captures, test logs and local builds remain under `private/`.
