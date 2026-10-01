# Pale Realm implementation

`wchess1`, route 19, is owned by `levels::wchess1`. The destination is
`wchess2$wchess2_start1`. Start it through `tools/launchers/Launch-Pale-Realm.cmd`; its saves are
separate from normal playtest saves. Save envelope 12 and the 7,900,000 hit range
are retained. Data and footage are not redistributed.

## Playable systems

The owner draws and collides the quad gate, knight gate, lift, puzzle controls,
crushers, spike corridor and pool machinery. Model ownership excludes duplicate
placed chess actors. White and red pieces use the chess combat family, faction
targeting, authored spawn receivers and per-actor saved health/activation. White
actors start invulnerable; the escort and guard explicitly opt into damage.
Instructor health floors are 125/150; the rook floor is 180. The escorted pawn,
rook/bishop duel and optional bullied-pawn sequence retain their distinct paths.
Monster launch pads apply a swept ballistic flight using the same target formula
as player launch pads. Difficulty filters apply before the roster is built.

The bishop and knight lessons use reviewed arrival-thread tables, including
intermediate legs, heading and arrow masks. Jumping, attacking and summoning are
suppressed while disguised. Corner/edge contact starts a collision-checked run;
the normal model and held toy are replaced by the supplied TAN chess piece,
attached to Alice's ORIGIN bone so its authored coordinates stand above the board.
Pits revert the disguise, crushers inflict 1000 damage, and failed lessons have a
saved retry. A completed lesson cannot be re-armed.

The knight gate latches after four counted deaths, or eight if the optional gang
was activated before the original threshold. A late gang cannot close or re-open
that gate. Shot and touch activation of the gang share a one-shot latch. Both
red gallery bishops must die before the spike corridor is removed. The bell
lever retains a five-second free interval before the gate scene. Each lever has
reach, facing and obstruction checks, a single use and the supplied pull motion.

The water sequence starts its world clock 4.7 seconds into staging. A three-second
wait precedes a five-second rise of 96 units. The liquid surface moves from -384
to -288; the exit block is removed only when the rise finishes. Skipping never
fast-forwards that machinery. Swimming and climbing use the player's real liquid
and collision state. The exit requires completed knight traversal and raised water.

Five scenes cover arrival, both transformations, bell and water. They save the
C1 clock, captured player pose and actor starting pose. Pausing holds the clocks;
watching and skipping use the same completion code, with the authored bell-skip
rook endpoint applied explicitly. The supplied camera tracks, actor clips,
transformation particles, bell, refusal motion and machinery cues are loaded from
local data. This visit contains no voiced dialogue.

## Data decisions and native research

- **High confidence:** the native `chesspuzzle` setter toggles player flag 0x40;
  the state predicate reads that flag. `bosslevel` writes a separate global.
  `killdemons` is a separate entity traversal, not an implicit effect of either
  setter. Research addresses: 0x100aabf0, 0x100a7460, 0x100aac50, 0x100a5c40.
- **High confidence:** Actor HEALTH compares health below a floating threshold;
  Player HEALTH uses an integer threshold. Addresses 0x1000e000/0x100ab5a0.
- **High confidence:** `max_inactive_time` controls enemy retention. Native idle
  callbacks in combat mode (Actor+0x528 = 2) follow enemy loss and a five-second
  idle interval. PVS refresh occurs only in the separate mode 1; it does not keep
  a lost combat order alive. Addresses 0x10010fb0, 0x1000c380, 0x10022510 and
  0x10020bd0. This owner uses ten seconds without sight followed by five idle
  seconds. Native polling adds a variable delay which is not reproduced. This
  corrects an earlier private interpretation that applied the idle-mode PVS test
  to combat orders.
- **Reviewed deviation:** the bishop file comments out its corner-opening calls.
  Keeping every wedge solid makes the first reachable move impossible. Only the
  arrival thread's armed corners are opened here; all other directions remain shut.
- **High confidence:** three knight destinations are undefined. Those directions
  remain unavailable. `knight_11_13` retains its authored crusher leg; it is not
  silently repaired into a safe route. Graph identities describe arrival threads,
  not merely board square numbers.
- **Reviewed choice:** duplicate instructor destination names resolve to the first
  BSP entity, #293. Crusher initial phases are deterministic per entity within the
  authored half-second window, and persist through the saved world clock.
- **Source correction:** `pawn_npc2` starts its walk when the gallery bishops spawn;
  it does not wait for their deaths. Their death callback only removes the spikes.
- **High confidence:** dead red-rook/wall-break threads have no placed caller. They
  are intentionally absent. The final undefined attack callback is a no-op.

## Persistence and checks

The tower clock hands turn about the map's Y axle, keeping their original pivots
and separate speeds. The script's X angle denotes pitch; interpreting it as a
Cartesian X rotation swung the hands through the clock face. Their phase still
uses the saved world clock, including pause and load.

New snapshots include arrival nodes and move legs, disguised state, cast, levers,
world clocks, water, lifts and gate latches. Invalid state is rejected before it
replaces the owner. A save from before this controller restarts at the entrance,
retains inventory/resources, and re-arms the supported formerly consumed scene and
combat threads. This deliberately avoids stranding an old dry-pool position.

- `--wchess1-check`: scene watch/skip/pause/save futures, actual damage callbacks,
  four/eight thresholds, shot/touch latches, hazards, retries and migration.
  Clock coverage samples both supplied hand meshes over a complete 48-second
  slow-hand turn, checking fixed depth, radius, pivots and pause/restore phase.
- `--wchess1-route-check` and `--wchess1-route-skip-check`: ordinary player inputs
  from the entrance through both puzzles, gallery fights, lift, both levers,
  water swim/climb and the real Castling exit. No teleport or resource injection.
- `--save-check-write` / `--save-check-read` with
  `LOOKING_GLASS_SAVE_CASE=wchess1-`: six independent native save/restart cases.
- `--wchess1-render-check`: authored staging plus representative gameplay poses.
  Includes seven samples of the opening clock camera across the fast hand's turn.

Private evidence, exact build identity and remaining fidelity limits are recorded
in `private/wchess1/WORKLOG.md`. Precise native navigation, randomized polling,
sub-frame camera timing and the transformation puff remain fidelity comparisons;
static and silent native checks do not constitute an audio listening comparison.
