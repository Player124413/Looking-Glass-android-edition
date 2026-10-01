# Castle Keep

Start with `tools/launchers/Launch-Castle-Keep.cmd`, or select Castle Keep in the level chooser.
The original data must be installed locally; no game data is included.

The arrival lift and Cheshire hint lead to the working mirror lever. Its three
positions expose the club, diamond and spade clues in the reflection only.
The matching character portraits remain in the room. Suit doors, portrait
triggers, room sky and shaft visibility follow the selected suit.

Shooting a wrong portrait summons the room's guards and enables the return
teleporter. Turning the lever can arm the room again. A correct answer breaks
the portrait, records that suit once and summons the heart guards. All three
wins open the heart gate. The final Cheshire sequence uses the original voices,
cameras and actor clips, the Queen's strike, detached head and blood, followed
by Alice kneeling and weeping. Watching or skipping commits the same qlair exit;
the physical exit cannot bypass the puzzle or repeat a committed transition.

The controller saves mirror movement, room selection, each win, guards and their
projectiles, doors, actor/dialogue clocks, the detached head and exit delivery.
Arrival-only saves migrate without refilling resources or replaying arrival.
The paired hallway doors slide for their authored one second independently of
the contact hold timer. Mid-slide saves preserve their pose; older saves retain
their previous pose before resuming movement. Arrival Cheshire uses the placed
actor's position, settled onto the landing rather than an elevated path marker.

Checks:

- `--keep-check`: scenes at 30/60/144 Hz, skips, repeated failures, actual portrait
  shot dispatch, save restoration, legacy event migration and exit retry;
  hallway travel/hold/close and saved continuation, plus arrival cat support.
- `--keep-route-check` / `--keep-skip-route-check`: continuous navigation and
  portrait shots through the level into qlair. Enemy targeting and damage are disabled
  for these traversal checks; they do not establish combat or fall-damage balance.
- `--keep-render-check`: staged native captures of the three reflected clues,
  arrival, Cheshire and Alice's grief.
- `LOOKING_GLASS_SAVE_CASE=keep-` with `--save-check-write`, then
  `--save-check-read` in a fresh process: seven native persistence fixtures.

The mirror currently reflects the map and puzzle brushes; other animated actors
are not drawn into it. The isolated test desktop has no audio output, so sound
assets and event timing are checked without claiming an audible listening test.
