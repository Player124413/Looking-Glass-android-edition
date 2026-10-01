# Pool of Tears: transports and scenes

The arrival and ending use the shared C1 lifecycle described in [SCENE_RUNNER.md](SCENE_RUNNER.md).
It retains the legacy Pool owner, saves shot/cast/line clocks, holds Alice through
the one-shot exit and retries failed destination loads. The other scene timelines are unchanged. Reviewed difficulty-filtered clip posts are collision-only, and
the final kill volume uses its authored `setdamage` value.

Start `tools/launchers/Launch-Pool-Route.cmd` for the dedicated route build, or select Pool of Tears in the chapter menu. Climb the riverbank to the Mock Turtle; his conversation and departure release the first ride leaf. Hold Enter to skip a scene; the same progression gate is committed. Board its centre to start it. Follow the banks and leaf rides downstream. More leaves follow at the authored intervals if one is missed.

This pass restores the four ride leaves, eight repeating replacement leaves, two wobbling lily pads and the swinging branch. The original leaf models and reviewed path nodes supply the visuals and movement. Collision and rendering share the same position and rotation, and grounded riders move with their support. The first, second and fourth leaves have their original drop gates; the first two waterfall contacts remove their leaves' support as authored. Fourteen original Alice/Mock Turtle voice lines and subtitles play through the shared dialogue system.

Saving preserves the dialogue gate, drop timers, rides, train cycles and platform positions. Saves predating the transports restart Alice at the entrance, retaining inventory/resources and saved encounter progress. More recent saves retain scene progress and position; an old consumed pending ending contact becomes available again.

`--pool-check` tests staged boarding of each leaf through its actual contact trigger, carrying, jumping off, a mid-ride save round trip, deterministic continuation, pause, conversation gates and legacy interaction migration. `--pool-render-check` captures the restored transport artwork. These checks are separate from a continuous entrance-to-exit route proof.

All four ride fixtures pass with the local archives. Native captures show the original leaf/lily-pad artwork, and two separate native launches successfully load both a pre-combat v3 Pool save and a later combat-enabled Pool save. Story validation passes 83 original voice/subtitle pairs across seven maps, including this conversation's fourteen lines. The shared unit suite passed 219 tests at this checkpoint, along with strict Clippy.

The complete Normal route is covered by the ordinary-input checks described below. Paths still interpolate the supplied nodes and rotations; exact spline/pendulum fidelity, navigation, acting and sound-event timing remain approximations. The route uses the current Army Ant, Snark, Bloodrose and Ladybug systems. This establishes traversal and encounter integration, not exact original-game fidelity.

## September 30 scene and climbing repair

A saved Pool-owned scene timeline now restores the Rabbit arrival, both boulder cutaways, all five Turtle dialogue camera tracks, the Turtle leaf departure, later Turtle encounters and the final jump to `potears2$potears2_start1`. The third boulder contact activates its physical hazard without a camera scene, then enables both scene-owned ant pushers after two seconds. The Turtle rides the leaf's authored attachment tag and has no shell in this visit. The dialogue releases Alice's first leaf after the Turtle boards his; skipping commits the leaf drop immediately. The exit volume cannot bypass the final scene, and the scene sends one transition. Named Cheshire hints switch after the conversation starts.

Existing format-12 saves without scene fields keep Alice's position and transport state. Arrival does not replay; previously unhandled scene contacts are rearmed. The older pre-transport migration described above remains specific to saves without any Pool controller. Completed dialogue stays complete. No save-version, existing event-key or visit/hit-ID changes.

Pool's sloping rock shelves now land using full-body support height rather than the hand-contact height. Both the rise and inland crossing must remain clear. While hanging, press Space or release and press Forward again.

`--pool-check` also exercises five scenes watched and skipped at two points, saved camera/player/dialogue continuation, pause, one-shot completion, safe landing, legacy contacts and the final exit gate. `--pool-render-check` covers scene cameras and the transports. These are staged fixtures, not a continuous route proof. `--ledge-check` includes six actual Pool shelves; the native ledge check includes a Pool pull-up.

## Arrival / Rabbit (task 12)

The selected `info_player_start` must name `Tears1_Start_Cinematic`; the normal
binding is `potears1_start1` (#284). Fresh default and named entries play it.
Loading an existing visit resumes its saved state; a completed introduction
cannot restart. The existing Pool controller owns this scene and its cast.

`alice_watchx` follows `tears1_path4`, with a black fade lasting 2 seconds.
The boulder cue occurs at 4 seconds, Rabbit starts at 5, and Alice walks at 8.
Alice begins at the entrance and walks to `alice_posx1` (#643). Rabbit starts
at `rabbit_start_pos1`, facing 270 degrees, and follows `rabbit_run_pos1`–`3`.
The intermediate navigation markers (#739–#746) shape the route; the run animation
clock continues across destinations. Rabbit uses solid/monster-clip geometry,
while Alice retains her player-clip geometry. Cast origins are resolved
against the actual terrain; editor markers above the floor no longer make them
float. The Rabbit body narrows after his first destination, as authored.

Watching and skipping use one C1 completion path: Alice stands safely below
`alice_posx1`, at (-3536, 2912, 256.03125), facing 0; the scene puppets disappear,
and the gameplay Alice returns. `dk_boulder4` remains at `t495`, non-collidable.
No Turtle gate, reward, leaf drop or level exit is committed by this introduction.
The watched camera return requests 2 seconds through the existing collision-safe
handoff; skipping or an obstructed return uses its covered cut.

Pool scene revision 3 migrates active revision-1/2 arrivals at their existing
clock and preserves completed/pre-scene saves without replaying the entrance.
Cached visits upgrade too. The enclosing save version remains 12; event keys,
visit numbering and hit-ID reservations are unchanged.

`--pool-check` includes default/named/missing-thread entrance selection, first-frame
cast visibility, ground support, cue timing, original track sampling, pause and
six restore checkpoints at 30/60/144 Hz, watched and five skipped paths, migration
and one-shot completion. `pool-arrival-*` native save fixtures cover six phases
from the initial fade through completed gameplay. `--pool-render-check` includes
five arrival shots. These fixtures do not prove the complete Pool route.

Remaining fidelity: Rabbit steering uses terrain projection through the reviewed authored
navigation corridor, not the original pathfinder. Falling-rock movement still
uses the reviewed guide-node approximation; native gravity, bounce, collision and
sound events remain separate work. The arrival contains no dialogue.


## Pool boulders (task 14)

`Tears1_Boulder1/2` preserve their original track bindings, push timing and white
fades. Rock activation is at one second; the watch/skip completion advances any
omitted rock time without playing old contact damage or sounds. Flags6 leave the
rock moving after the scene; flags14 stop the arrival rock at its last waypoint.
Boulder2 hands Alice back at the supported `alice_boulder_2` marker facing `t317`.

The existing Pool owner contains the W9 adapter and both ant states. No second
controller is registered. Their hit IDs are `6_800_634` and `6_800_727`, within
Pool's Appendix F reservation. Primary melee, musket, pain/death, saved projectile
clocks and Demon retaliation use the same shared combat hooks as other enemies.
This is the bounded pusher encounter, not a full ArmyAnt-family port: alternate
grab/fling attacks, global navigation and dismemberment remain later work.

The optional `boulders` snapshot has its own revision. Cached and active older
format12 saves reconstruct only state supported by consumed timestamps or scene
completion; they do not clear trigger keys. Old elapsed contacts and sounds are
not emitted. Pause preserves scene, motion, collision cooldown and release clocks.

`--pool-check` adds actual-volume contact checks at30/60/144Hz, watch/skip outcome
comparison, paused/active saves, consumed-event migration and ant hit/death checks.
`LOOKING_GLASS_SAVE_CASE=pool-boulder-` selects the separate-process native save
fixtures. These boulder fixtures are separate from the continuous route below.


## Complete transport route

The ordinary route begins at `potears1_start1`, watches or skips arrival, climbs
past all three boulders, completes the Turtle conversation, and follows all four
leaf rides to `potears2$potears2_start1`. It includes the first ride's bank pickup,
a jump onto the east bank before the first waterfall, the lily pads, the second
ride's bank pickup, the second waterfall and swim onto leaf three, the Normal
posts, the timed jump/E rope grab, the climb and upper bank, and the final leaf.
The end scene holds Alice and sends one transition with unchanged carried resources.

Follow the current objective for the next transport. On the third leaf, jump
and press E near the hanging rope before the waterfall. Space climbs the rope;
move toward the bank and press E to release. Repeating leaves remain available
at the supplied intervals when a ride is missed.

A swimmer meeting a moving leaf can no longer rewind every Pool timer. A bounded
push searches space clear of fixed terrain, clips and other movers. When there
is no room, only the obstructed mover waits. Its delay is saved with a default
of zero for older saves. Other leaves, Turtle cues and boulders keep advancing;
pause freezes all of them. There are at most fifteen saved mover delays.

`--potears1-route-check` and `--potears1-skip-route-check` run the recorded Normal
route without a window. `--potears1-render-check` runs it with all native NPCs;
set `LOOKING_GLASS_WATCH_SCENES=1` to watch the scenes in that check. Movement is
ordinary controller input, attacks spend their normal resources, and the driver
conserves Cards during the climb for the river encounters. It never relocates Alice, refills
resources, disables enemies or grants invulnerability. Completion requires all
four rides, three Turtle encounters, key encounter contacts, all scene outcomes,
no unsupported contact, no recovery, no authored teleport and exactly one exit.
The destination must be clear and preserve Stats exactly.

Each run writes checkpoints and `private/potears1/report.json` under its working
directory. `LOOKING_GLASS_POOL_FROM` and `LOOKING_GLASS_POOL_INPUT` are diagnostic
resume/input overrides; resumed runs identify themselves and are not fresh
entrance proof. Use a separate working directory for concurrent runs.

After a native route, run `--potears1-save-write` and `--potears1-save-read` in
separate native processes with that same working directory. They exercise actual
Store files reached on leaves one, two and three, on the rope, and before the
ending. Each rebuild compares player, resources, NPC and mover state and follows
240 identical ticks. Existing arrival/boulder/ending migration fixtures remain
in `--pool-check` / `--save-check-write` / `--save-check-read`.

The new collision fixtures in `--pool-check` reproduce the second ride's bank
obstruction both with an escape and inside a tight enclosure; they check clear
player collision, advancing visit time, local waiting, pause, bounded state and
deterministic save continuation.

Validation on 2026-09-30, standalone Normal with the supplied archives:

| Check | Result |
| --- | --- |
| Fresh native entrance, scenes skipped | PASS: 51,214 ticks; 25 jumps; 27 Blade throws; 84 Cards; two pickups and three enemy drops; Sanity 100 to 19.04, Will 100 to 1.97 |
| Fresh native entrance, scenes watched | PASS: 56,732 ticks; 25 jumps; 27 throws; 84 Cards; two pickups and three drops; Sanity 100 to 19.04, Will 100 to 2.97 |
| Fresh headless entrance, scenes watched | PASS: 56,185 ticks; 25 jumps; seven throws; 72 Cards; two pickups and four drops; Sanity 100 to 100, Will 100 to 34 |
| All three route runs | All four rides and scene outcomes; zero recovery, teleports, lost or hazard ticks; one exit; clear Hollow Hideaway spawn and exact resource carry |
| Native Store write and fresh-process read | All five route checkpoints pass exact player/resources/enemies/movers and 240 continuation ticks |
| Real input after loading the rope Store file | Space climbs from z277.03 to z375.20 while retaining rope #57; F5 saves the climbed state |
| Shared regressions | `--pool-check`, `--ladybug-check`, 628 unit tests and release build pass |

The native run includes 35 placed actors across eight model families as well as
the Pool-owned pushers and Ladybugs. Headless combat uses its existing reduced
cast, so its resource totals are not a native balance claim. Watching scenes
allows more world time to pass; full-route combat outcomes need not match a
skipped run. Individual watched/skipped scene commitments remain checked by
`--pool-check`. No audible validation was made in the hidden desktop. Easy,
Hard/Nightmare and strict incoming resources from the whole campaign are not
established by these standalone Normal runs.

Integration with the newer shared ledge catch required a short westward run-up
at the first Normal post and an inland waypoint after the rope. The fresh
integrated headless run also passes: 51,089 ticks, Sanity 100, Will 34, two pickups
and four drops, with no recovery or teleports. Rope checkpoints explicitly require
attachment to #57 and the attained climb height; a missed grab cannot silently
continue. The combined source passes 629 unit tests.

Final packaged validation, 2026-10-01: the full native Normal route passes from
the ordinary entrance in the integrated build, including those ledge changes.
It steers toward the third leaf at the second fall and collects the nearby
enemy loot before boarding. Result: 51,659 ticks, 25 jumps, 24 throws, 92 Cards,
two map pickups and four drops; Sanity 100 to 61.47 and Will 100 to 1.97. It has
one safe exit and no recovery, teleports or hazard ticks. All carried resources
match after arrival in Hollow Hideaway. The dedicated launcher uses this tested
binary and supplies its check modes with `private/potears1/route-input.json`,
which matches the final `src/pool/route_input.rs` recording. This input override
does not affect normal player-controlled gameplay.

## Upper-bank ant update (2026-10-01)

The live viewer now includes Pool's scene-owned boulder ants in its combat
update even though the registered controller list is empty for this visit.
Previously the route driver advanced these ants but normal gameplay did not:
after Boulder3 enabled them, their action clocks stayed frozen. Viewer and
route now use the same eligibility check. The original two-second release,
cutscene/dialogue protection and Pocket Watch timing remain in force. Existing
saves resume the ants from their saved action and animation clocks.

The boulder regression checks the production eligibility predicate, advancement
of both released ants, pause and state restoration through the shared hook.
