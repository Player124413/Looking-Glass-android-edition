# Airborne Terror

`tower1$tower1_start1` has one registered controller for the Cheshire
introduction and four blow faces. Existing native NPC ownership supplies the
ten Boojum ambushes; no second combat cast is created. The sealed precache
actor #410 remains hidden and inert, including in older saves.

The introduction uses `tower1_p1`, the authored Cat marker, seated animation
lengths, head and lip bindings, speech and appearance/disappearance cues.
The two-second opening fade leads into the Cat's appearance at 4.5 seconds.
Control returns at 9.3 seconds in the supplied data. Watching and skipping
restore Alice's captured playable pose, complete the same speech state, and
release combat. The separate hint trigger cannot interrupt the introduction;
summoned hints remain independent of progression.

Each face closes for 0.35 seconds, holds for 0.2 seconds before applying its
two-second push, then opens for 0.4 seconds. The six-second trigger cycle
repeats while occupied, including the overlapping second and third triggers.
Rendering uses the authored TAN clips; smoke follows the model attachment.
The saved face clock drives drawing, smoke and physical push together.

The existing twenty updrafts, twenty-four hurting vent pushes, four lethal
volumes and launch pad #12 retain their gameplay. Reviewed actor push volumes,
including #8, #15 and #354, now act on the native enemies through their saved
recoil velocity and swept collision. The four face pushes use the same enabled
phase for Alice and enemies. The three placed medium essences remain ordinary
pickups. Exit #22 is an ungated volume into `hedge2$hedge2_start1`.

## Saves and compatibility

New saves retain scene, shot, cast and line clocks, Cat fade tail and all four
face phases. Pause advances none of them. Saves made before the controller
continue without replaying the introduction, preserving valid player position,
resources and native enemy state. Only the old precache actor's hidden state is
migrated. Save envelope 12, existing event-rule keys, visit order and the
8,900,000 hit-ID reservation are unchanged.

## Verification

`--tower1-check` covers watched/skipped handoffs at 30/60/144 Hz, exact restored
face futures, pause, actor-pad velocity restoration and real trigger contacts
with six-second occupied repeats. `--tower1-actors-check` checks all ten native
activations on each difficulty, damage/death, reload and no duplicate spawn.

`--tower1-route-check` and `--tower1-skip-route-check` start fresh on Normal with
the production cast. Ordinary movement and funded attacks climb the columns,
cross the face gauntlet in its idle windows and enter the actual Hedge2 map.
Both runs finish with 58 Sanity, zero teleports or recovery, no lost or lethal
hazard ticks, and exactly carried resources at a clear destination. They do
not grant health, remove enemies or weaken combat.

The production Store fixtures `tower1-early`, `tower1-intro`, `tower1-ending`
and `tower1-gust` are selected by `LOOKING_GLASS_SAVE_CASE=tower1-` with the
separate `--save-check-write` and `--save-check-read` processes. These compare
saved owner/world/player futures; they are distinct from the full combat routes.
Staged renders cover Cat appearance/smile and face close/hold/blow/open/idle.
Private logs and release hashes are under `private/tower1-work/`.

## Limits

Actor post-contact velocity uses the existing recoil decay, rather than a newly
reconstructed native airborne integrator. Existing updraft acceleration and vent
hurt cadence remain the port's approximations. Pad #12 remains an enabled
hazard and is not required by the verified upper route. Exact sub-frame acting,
vent steam synchronization and audible playback remain comparison work; the
isolated desktop had no audio output. Full-route proof is on Normal only, and
is not a full campaign-chain certification.
