# Battle Royale

The `grounds2` controller restores the entrance floor, automatic introduction,
staggered collapse, pawn launch and four staged duels. Watching or skipping the
introduction leaves Alice on the supported path, removes the scene actors and
closes the way back. The launched pawn is scene-only in both outcomes. The map
does not contain placements for the airship references in its initialization;
this implementation does not invent those actors.

Each Spade company has a lifetime quota and at most two living members. Quotas
are 2, 2, 3, 3, 4, 4, 6 and 6. Normal, Hard and Nightmare activate paired
companies; Easy activates the even companies, for 15 guards instead of 30.
Successful launches are 1.1 seconds apart, including the original setup delay.
Repeated trigger or death callbacks cannot reset the quota. Every emitted guard
has a permanent identity, so deaths, projectiles, loot and saved progress remain
attached to the same actor.

Guards receive the scripted attack order and use the existing card combat,
electricity, dismemberment and loot systems. Their launches and the map's native
enemies use solid/monster collision instead of the invisible ceiling intended
only to contain Alice. Dynamic obstacles still participate in those traces.
Alice retains her normal collision rules. The native Magma ambush, Diamond and
Heart guards remain under the shared enemy system.

The exit goes to `facade$facade_start1`. It is unavailable during the
introduction; the original map does not impose a kill-all requirement afterward.
The combat route check nevertheless requires every recurring company to finish
and every recurring guard to be defeated before verifying the transfer.

Saved data includes the scene clock, collapsed floor, reinforcement timers,
unique guard slots, flight velocities and combat state. Old saves without this
controller restart safely at the entrance while preserving carried resources.

## Checks

- `--grounds2-check`: arrival and watched/skipped equivalence at 30/60/144 Hz on
  all four difficulties; real launch landings; cadence, live caps, quotas,
  deterministic saved futures and atomic rejection of malformed state.
- `--grounds2-route-check`: input-driven Royal Rage battle, actual campaign
  transfer into Battle Royale, recurring combat and the Ascension handoff.
  `LOOKING_GLASS_GROUNDS2_SKIP=1` skips the battlefield introduction.
- `--grounds2-render-check`: native frames for arrival, collapse, pawn, duels,
  Spade launch and Magma ambush.
- `--grounds2-save-check`: staged native save files at scene, launch and
  reinforcement boundaries. Run once with `LOOKING_GLASS_GROUNDS2_WRITE=1`, then
  again without it in a fresh process to compare restored state and future play.

Diagnostic route entry/checkpoint/itinerary overrides are separate from the
default end-to-end proof. They never alter production enemy health or damage.
All asset inspection and test output stay under ignored `private/grounds2-work`.

## Verified result

Normal difficulty, from the completed Royal Rage battle: both watched and
skipped introductions reach Ascension with all 30 Spades and the native
encounters resolved. Alice never touches lava, leaves the level bounds or uses
an authored teleport. Both runs finish at 80 Sanity and approximately 68 Will;
resource values carry unchanged into the supported Ascension entrance. The
recorded route collects available essence and uses normal weapon costs and
damage. Separate processes also restore all eight staged save cases, including
the Magma ambush, with identical controller and native-enemy futures.

Rendering was inspected in the native application. Audio cue timing is wired to
the scene clock; isolated desktop audio was unavailable for a listening check.
