# School first visit — updated for version 0.18

## Broken-floor backdrop correction (2026-10-01)

The striped sheets below the broken library floor came from the reverse faces
of the sky's book models. Sky TAN models now respect material face selection,
using their clockwise front winding and final animated positions. The open
floor still reveals the intended distant book stacks. Gameplay collision,
puzzles, fog and saved progression are unchanged.

The native school render check compares five viewpoints with the old two-sided
behavior. At the broken-floor viewpoint, it removes 51,629 erroneous pixels
while preserving the foreground carpet exactly. The school state check, 647
unit tests, material/sky GPU regressions and loading/drawing all 36 maps passed.
The map-wide checks cover rendering, not continuous campaign traversal.
Evidence and the exact build snapshot are in `private/skool-floor-fix/`.

Version 0.18 connects the missing entrance, theatre and library enemy activations and the optional Looking Glass secret. Both the ordinary route and a secret-collecting route reach the existing recipe exit with combat active. The original library puzzle and lift/shelf collision fixes remain in place.

Version 0.15 added theatre, shelf, book and recipe dialogue. The current recipe exit is owned by its cinematic timeline, including the delay after the voice. Version 0.16 connects the second school's main route and potion quest; see [SCHOOL2.md](SCHOOL2.md). See [STORY.md](STORY.md).

The first visit to `skool1` now has a connected gameplay progression: theatre, library passage, two lifts, four flying books, the recipe book, and the transition to `skool2$skool2_start1`. Use the normal **Launch.cmd**. **H** briefly shows the current objective along with the controls. **E** still operates doors; **Space** jumps or climbs a low ledge, and **Shift** gives a running jump more horizontal reach.

## First-visit performances and walking pupil (2026-09-30)

`Theatre_Cinematic` plays five local camera tracks and the staged cast. Alice
walks down the aisle at her animation's measured speed, then joins the Gnome.
Pupils ride their platforms, change gestures and remain on stage after control
returns. Steam follows the entrance cues. The miniature airship and rider use
the speed-weighted spline; the Gnome retains his pipe and smoke, then shrinks
with the disappearance burst. Explicit headwatch targets use bounded rig turns.

Dialogue begins at 22.8 seconds; each voice uses its WAV duration followed by
the authored one-second wait. The reinforcement/door event fires once at 42.6
seconds, with the guards' approach from 43.6. Finishing or skipping cannot
activate that event again. Walking back out starts `Skool1_OG_MoveShelf`, with
Gnome growth, walking/mixing, the remote camera and opening bookshelf. The
library remains locked until that separate event. The theatre returns Alice to
`alice_gnome_pos` downstairs, facing the stage; watching, skipping and restoring
an active scene use the same supported landing. Other first-visit scenes retain
their pre-scene handoff. The return visit retains its separate controller.

The entrance pupil `return_insane2` follows the five authored waypoints with the
original walking animation and its measured travel speed. Movement uses a swept
body, floor support and a fixed clock. Walking resumes after greetings and saves.
The actor retains its original saved identity and does not become a new blocker.

Optional scene/walking fields keep save format 12. Old saves preserve completed
scenes; start a fresh first visit to see performances already consumed in a save.
No event rule identity, route number or hit reservation changes.

The scene clock, pending reinforcement cue, speech position and post-scene cast
clock survive saving. Pause freezes them; skipping stops the scene's remaining
presentation while retaining its earned access changes. Loading a completed
scene does not replay it or revive defeated guards.

Compatibility choices: the repaired gameplay floor remains. The camera retains
its existing Gnome clearance correction.
The missing `theatre_pos7` BSP marker uses the placed third pupil's position.
Head turns use the original targets and relative response values within the
port's rig limits; this is not an exact reconstruction of the native IK solver.
The entrance pupil's original pause cadence remains approximate.

## Library and recipe staging (2026-09-30)

The first visit now stages `shelf_cinematic`, `book_cinematic` and
`Book_Ingredients_Exit` with the local camera tracks, Cat appearances, Alice's
push and reaction gestures, the book's sway, tumble, opening and page turns,
white fades and the authored sound identifiers. The small switch in
`Skool1_OG_MoveShelf` moves 16 units separately from the secret wall.

The switch book remains attached to `secret_shelf` through its 648-unit travel,
including collision and completed older saves. Its 16-unit press is local to
that attachment. No scene replay or saved progression reset is required.
The theatre's `theatre_pic1/2/4` brush pictures now draw at their initialized
positions on both visits. `theatre_pic3` retains its authored hidden state.
Scenery and the shelf binding are derived from map data and existing saved
mover poses, so they require no save-format or event-key change.

The four-book bridge remains a prerequisite. Its supports descend only after
the book performance ends; the recipe contact is held until they settle.
Advancing dialogue shortens the voice without bypassing the push, landing or
final departure delay. Holding the scene-skip control commits only that scene's
accepted changes. A skipped book scene cannot start school two; Alice must
still reach the recipe downstairs. The recipe scene emits its departure once.

Saved phases retain camera, acting, machinery and dialogue positions. Older
format-12 deferred exits are adopted into recipe staging rather than released
by dialogue completion. Already completed scenes stay completed. Camera
orientation and cuts use local authored controls; Cat fades use the existing
stippled dissolve, and headwatch uses bounded bone offsets. Native post-recipe
screen distortion is not reproduced. No live original-game comparison or
listening comparison is claimed.

The first-visit scene controller exclusively draws `shelf_cat` and `book_cat`.
Dialogue no longer reveals an additional map copy with its own animation clock.
Their existing NPC save identities remain compatible, including saves made
mid-conversation. The native scene check now draws the ordinary NPCs alongside
the scene actors and compares against explicitly hidden legacy Cats, covering
speech, restoration and the disappearance transition.

## Upper hall doors

The second-floor opening now contains its two original `slamming_door` brush
leaves on both visits. They swing independently: one opens 32 degrees over
0.4 seconds and returns over 0.3 seconds; the other opens 26 degrees in the
opposite direction over 0.2 seconds and returns over 0.4 seconds. The original
positional flapping loop plays once at the first leaf.

The doors participate in collision and pause with gameplay. A leaf stops if its
next pose would overlap Alice, then resumes when she moves clear. New saves
retain each leaf's phase. Existing format-12 saves recreate the missing pair
without replaying scenes or changing puzzle progress.

`--school-check` covers both visits at 30/60/144 Hz, the blocked opening,
pause/restoration, older-save migration and stop/resume behavior.
`--school-render-check` also captures three native views of the doors, with a
pixel-change check confirming that their geometry visibly moves. These are
staged checks, not an original-game comparison.

## Route guide

1. Open the entrance doors, turn left and enter the theatre through its doors. The late camera cue activates the two theatre reinforcements and opens the theatre doors; skipping commits that cue once. Return through those doors to activate the library passage. The first visit now plays the theatre performance and the separate shelf-opening cutaway; the on-screen skip control commits their access changes safely.
2. Climb/jump up the damaged stairs in the main hall, follow the upper corridor, and open its doors. Follow the revealed passage into the library.
3. Approach the closed blue book on the lowest library floor's right side. Approaching each book wakes it; it flies to a position in the final bridge.
4. Ride the spiral lift at the far end. Find the next book on the right of the middle floor. The bookshelves tip to form climbing ramps; use them and a running jump to reach the upper balcony.
5. Find the upper-floor book on the left. Ride the larger lift at the far left to the top floor and approach the fourth book there.
6. Wait for the flying books to settle, then jump across their bridge to the large recipe book. The recipe book falls, and the flying books move into descent positions.
7. Descend using the book platforms and balconies. Approach the open recipe book on the lowest floor to continue into `skool2`. Avoid jumping straight down the entire height of the library.

Lifts return automatically after being activated. They carry Alice through translations and rotation, stop if she would be trapped against another surface, and resume when there is clearance. Books do not become solid inside Alice. Pause, inventory, menus and focus loss stop these movements.

## Optional secret and encounters

Shoot the face picture on the theatre wall. The picture moves and a concealed library panel opens. Later, enter the revealed alcove and collect the original heart-shaped Looking Glass. It grants 45 seconds of invisibility and plays the original explanatory voice/subtitle. Alice and her held toy use a faint stippled appearance, and the HUD shows the remaining time. Enemy sight no longer acquires Alice during the effect; existing projectiles and environmental hazards can still hurt her. Pause freezes the timer, recovery clears it, and the pickup cannot be collected again during the same run. The secret does not unlock the recipe exit or change the book count.

The entrance/first-pass groups, the theatre's two spawned guards and the library groups activate once at their reviewed contacts or dialogue completion. Both Club and Diamond guards can attack and die; Diamond guards fire visible swept projectiles. The return visit uses its separate Club and Boojum groups and disables first-visit activations. Difficulty filtering follows the selected difficulty (Easy, Normal, Hard or Nightmare) through the authored spawn flags; see [ITEMS.md](ITEMS.md). Completion of the return visit's observatory quest is proven by `--school-return-check` and `--school-return-chain-check` (see [SCHOOL-RETURN.md](SCHOOL-RETURN.md)).

**R**, death retry and **Home** preserve the current puzzle progress, as they already preserve pickups and door state. **Tab → skool1 → first visit → Enter** starts the puzzle afresh. F5/F9 and Continue preserve puzzle, pickup and enemy state across restarts. See SAVES.md.

## Implemented behaviour and limits

The Rust code explicitly handles the school events needed for this route. It reads named objects and book waypoints from the user's BSP and renders the original book TAN animation clips. It never executes game scripts as host commands. Secret-shelf movement, rotating bookcases, spiral and upper lifts, book counts, bridge collision, the falling recipe book and the ingredient exit are connected. The early direct exit is gated, and theatre steam follows the staged entrance cues.

The theatre and shelf scenes now have explicit staging. Other school cutaways, decorative lift mechanisms and full original enemy behaviour remain incomplete. Selected voice dialogue, the optional shootable secret and the reviewed enemy activation groups are connected. Theatre platforms move for the performance while its repaired gameplay floor remains safe. Book paths use bounded linear interpolation through authored waypoints; exact spline speeds, orientations and cinematic timing are approximate. First-visit recipe events stay disabled on the return visit. Version 0.24 completes that visit's observatory route; see [SCHOOL-RETURN.md](SCHOOL-RETURN.md). Version 0.16 separately connects the next map's main quest.

Normal jump launch speed is now 400 units/second under gravity 800, giving approximately 100 units of rise. Local Ghidra inspection of the player jump event established the normal half-gravity launch rule and the default gravity registration. A separate 1.5 multiplier belongs to a timed power-up state; it is not assigned to Shift. Horizontal acceleration, climbing, body dimensions and the existing fall-damage curve remain provisional.

## Verification and provenance

- `--school-check`: actual map contacts, prerequisites and exit destination; both lift cycles at 30/60/144 Hz; pause, blocked-rider stopping/resuming, and preventing a book platform from appearing inside Alice.
- `--school-route-check`: a continuous physics traversal from the normal entrance, using walking, door use, jumps, rides and Blade attacks with enemy combat active. Its bounded input search plans difficult jumps against collision, then replays the inputs through the live interaction/controller code. It does not teleport, fly, restore footing, set puzzle flags directly or replenish resources. This is a headless input simulation; thrown Blades use an eye-height launch point rather than the native attachment. Re-baselined 2026-09-29 for the Blade/Cards rules and again for the strict chain (visit 6, below), the release run completes in **21,887 physics ticks with 17 jumps, 1 climb, 7 throws, 3 swings, 125 cards, 19 combat damage and 98 Sanity remaining**. The driver fights awake enemies with the shared route combat policy ([COMBAT.md](COMBAT.md#route-driver)) and walks over their essence drops; the earlier throw-at-anything driver ran out of Sanity near the recipe exit once a throw cost 3.9 s.
- **Strict chain, first visit.** `--campaign-route-check --campaign-strict` hands this route Alice with what the visits before leave her (Normal: 100 Sanity and 28 Will, Blade and Cards only), and a strict chain has no baseline fill. Two route-driver changes make visit 6 pass at Easy, Normal and Hard; no gameplay value changed. (1) On the library passage a Diamond guard's bolt knocked Alice off the narrow walkway onto a ledge about 45 units below it, where the fight's look-ahead rejected every heading, so she stood still until `clear` gave up; the shared combat helper now climbs back to where the fight began after 240 idle ticks and leaves a foe that has taken no damage for 2,400 ticks (COMBAT.md, route driver). (2) The Croquet Mallet is an authored first-visit pickup (`mallet_skool1`, entity 78) on the theatre's upper landing, which the route's goals never passed; after clearing the start of the library passage the route walks to the altar (about 450 units south, at the passage's height), returns, and the strict chain's reward provenance now finds `authored pickup skool1:78`. Chained on Normal the leg takes 22,741 ticks (18 jumps, 15 throws, 2 swings, 112 cards, 26 combat damage), Sanity 100 to 77 and Will 28 to 6; the secret variant 20,461 ticks and Sanity 77. The same checkpoint at Easy: 17,131 ticks, Sanity 100 to 61; Hard: 21,601 ticks, Sanity 87 to 58. The chain then stops in school two (visit 7), whose recorded route dies in its first fights on the Will it is handed (`docs/CAMPAIGN.md`).
- `--school-secret-check`: the same normal-start route with a detour to shoot the face and collect the Looking Glass, then complete the library and exit. Checks activation of all six first-visit trigger/dialogue groups. Release run: **24,335 ticks, 17 jumps, 1 climb, 16 throws, 6 swings, 113 cards, 52 combat damage and 91 Sanity remaining**.
- `--progression-check`: first/return group separation, one-shot trigger behaviour, shot-switch wall occlusion, unique timed secret collection, panel/face transforms and preserved recipe gating.
- `--progression-render-check`: staged native views of the face, panel, Looking Glass, Diamond guard and Alice's invisibility presentation.
- `--school-render-check`: an explicitly staged native rendering diagnostic that writes three private captures. It is not a playthrough test.

Evidence comes from the local school map and its map/cinematic scripts, the model definitions and TAN clips, and targeted read-only Ghidra analysis of `fgamex86.dll`. Jump event registration points to `100aa360`; normal gravity registration is in `1007d7f0`, with the timed-state expiry handled by `100aa210`. Addresses and behavioural observations are recorded here; extracted scripts and decompiler output remain private. No original executable or DLL was run, and no assets or captures are included in the source-review package.

## Observatory return (v0.24)

The return visit now has its own completed traversal and potion exit. Earlier limitations above refer to the first-visit implementation at the time; see [SCHOOL-RETURN.md](SCHOOL-RETURN.md) for current return behavior and checks.
