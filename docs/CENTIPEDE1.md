# Fungiferous Flora (`centipede1`)

The visit is owned by `levels/centipede1`. It uses the local map, camera track,
voice/subtitle data, animation clips and models without executing original scripts.

- The Centipede stays hidden during exploration. The first voice-over leaves Alice
  free to move. The second Demon Die remains an ordinary, persistent altar pickup.
- Crossing the ambush trigger starts the runners' rush, followed by the reveal,
  yawn/snarl animation, dialogue, fades and return to play. The first two seconds
  cannot be skipped. Scene AI pauses with the scene; dead runners stay dead.
- Watching and skipping commit the same encounter: living runners activate, the
  Centipede disappears, Ant waves start, and a ten-second departure countdown begins.
  The watched version's delay and actor policy are used for both paths. The two
  extra Ants present only in the original skip branch are omitted from both.
- Both spawn chains forward past points visible from Alice, spawning at the first
  occluded point. Facing direction does not alter visibility. The shared live-brave
  cap also gates the killer chain, preserving the original loop's quirk. Four wave
  attempts fit the countdown at 0, 3.1, 6.2 and 9.3 seconds, with up to eight new
  soldiers. Ordinary Ant combat supplies projectiles, melee, damage, death and loot.
- During the countdown, damage cannot reduce existing sanity below ten. This never
  heals or resurrects Alice and is recalculated by the current visit after loading.
- The mushroom lid is visible, solid and eligible for ledge support. The authored
  forest fog distance is applied. Existing Ladybug, plant, Antlion and Snark behavior
  remains with the shared enemy controllers.
- Departure requests `centipede2$centipede2_start1` once, with a bounded retry after
  a failed load. Inventory and resources carry into the Sanctum intro. Older saves
  without this controller restart at the forest entrance with resources preserved.

Use the regular `Launch.cmd` and select Fungiferous Flora, or use
`tools/launchers/Launch-Fungiferous-Flora.cmd` for direct entry into the selected local build.

## Verification commands

- `--centipede1-check`: watched/skip equivalence, 30/60/144 Hz timing, wave ownership,
  damage, pause, malformed snapshots and exit retries.
- `--centipede1-route-check` / `--centipede1-skip-route-check`: ordinary movement and
  weapons on Normal and Hard from the entrance through the altar and ambush into
  the Sanctum intro. A checkpoint during live countdown combat must continue with
  identical actors, resources and departure after restoring.
- `--centipede1-render-check`: forest, runner rush, Centipede reveal, waves and lid.
- `--centipede1-save-write` / `--centipede1-save-read`: native saves for exploration,
  speech, rush, reveal, outro, countdown, active/dead wave actors and committed exit;
  fresh-process restore and migration from a previously consumed ambush trigger.

The headless route includes the visit-owned Ants and world hazards. The other enemy
families use the native viewer's shared NPC simulation and its existing family checks.
Spawn chains and soldier slots are bounded; exact original AI physics is not claimed.
Research extracts, captures, saves and local builds remain under `private/`.

The installed build passes all four Normal/Hard watched/skipped walking routes,
including continued countdown combat after restoring and the Sanctum intro.
Eight native save cases, legacy recovery and five native scene captures pass.
The shared suite passes 616 unit tests; the neighboring Centipede2 and Wforest
routes and existing weapon, swimming and enemy checks also pass.
