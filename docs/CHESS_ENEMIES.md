# Chess enemies

Red pawns, knights, bishops and rooks use the supplied skeletons, equipment,
animation clips, health, body bounds and essence grades. Pawns stab, knights
approach with swords and block directional blade/card impacts, bishops alternate
staff melee with a short ranged beam, and rooks punch or charge on supported
ground. Attacks check cover, range, facing and the authored contact frame.
Knight shield recovery leaves an opening; rear and energy hits remain effective.

All four turn, pursue visible targets and briefly investigate their last seen
position. Pain interrupts attacks without restarting on every damage tick.
Two normal deaths and the frozen death are loaded for each piece. Lethal hits
immediately remove the weapon target, cancel unfinished strikes and award the
appropriate essence once. Corpses finish their pose, hold for five seconds and
shrink over two. Combat, corpses and beam lifetime stop with gameplay time.
Existing Demon Dice targeting and knockback are connected.

## Placement and activation

Existing NPC identities remain unchanged. New ambush slots are appended, keyed
by their original map entity number, and stay invisible and untargetable until
their reviewed trigger volume is touched. Trigger delays and difficulty flags
are honored. A fixed cast, one optional beam per bishop and bounded simulation
steps limit resource usage.

- `wforest`: ordinary red pawns and the three authored pawn ambush target groups.
- `wchess1`: eligible placed red pieces, direct spawn links and the independent
  second/fourth pawn callbacks. The knight-gate encounters remain deferred.
- `wchess2`: the first three pairs of side ambushes. Taking one side suppresses
  its counterpart; reaching the second battle suppresses unused side ambushes.
- `rchess1`: all ten ordinary enemy groups. Group five suppresses untriggered
  group three as in the source sequence. The registered Red King controller now owns
  these same 18 receivers and their shared combat/reward state (see RCHESS1.md).

White pieces retain their friendly ambient/scene ownership and cannot be targeted
as enemies. The bully, queen escorts and beheading spectators keep their authored
AI-wait state. The King's bishops remain hidden pending their scene. Royalty,
disguise puzzle props and scene spawns are not promoted to ordinary combatants.

## Persistence

Format 12 gains optional chess state on NPC records. It stores health, action,
contact state, pain/death variant, recoil, pursuit memory, shield recovery,
activation delay, suppression and corpse/beam age. Old records without combat
state initialize at their saved positions; absent new ambush slots start dormant.
Returning to a saved visit retains defeated pieces and the shared reward ledger.
Resetting generic Card guards does not revive chess enemies.

## Limits

This restores the enemy family and reviewed standalone encounters, not the full
chess campaign. White-versus-red scripted battles, instructors/disguises,
knight-gate kill counts and their inactivity failsafes, bullied-pawn choreography,
queen abduction and other chess campaign scenes remain unfinished. The `rchess1`
beheading and Red King boss/exit sequence are now covered by RCHESS1.md.
Pending callbacks with these consequences are deliberately left pending.

Movement uses bounded local steering rather than a navigation graph. Actor bodies
remain non-solid to Alice and one another. The short beam uses a depth-tested
line effect from a body-relative staff position; the original beam shader and
exact skeletal emission point are approximations. Shields use incoming projectile
warnings and impact direction, without a full original attack-intent system.
Pain thresholds, idle variants, directional death selection and corpse hold/shrink
timing are approximations. No original scripts or game binaries execute.

## Checks and local playtest

`--chess-check` audits source clips and placements and runs original-animation
combat at 30/60/144 FPS. `tools/test_chess.ps1 -Executable <candidate>` adds staged
native captures for every type and separate-process save write/read checks.
Those cover actual BSP ambush activation, pause, old NPC migration, death,
continued encounter state and one essence reward. These are targeted fixtures,
not evidence of complete map playthroughs. Evidence stays in `private/chess/`.

`tools/launchers/Launch-Chess.cmd` uses the local verified candidate when installed, with a
separate playtest save directory. It starts in `wforest`; append `--map rchess1`
to test the ten ambush groups. The shared playtest launcher remains independent.

## Verification — 30 September 2026

Candidate `c84f38a2` passes 512 unit tests, the four chess checks, 35 distinct
native pose captures and four separate-process chess save continuations. The
source audit finds 63 eligible red pieces, including 47 dormant ambush receivers,
across the four maps. All 63 supported NPC models load; all 39 registered visits
build deterministically. Source/provenance and formatting checks pass.

The existing Ant checks, both Pool scene checks, automated actor visibility and
78 full level replacements also pass on this candidate. General campaign save
fixtures were not rerun; the four chess and two Ant process-restart cases were.
Native audio was disabled, so audible mixing is unverified. The staged views
explicitly use the production material queue, including reflective skin layers.

Rust 1.98's strict all-target Clippy run reports five pre-existing findings in
unchanged code (four lint categories). With those categories excluded, it passes
without additional findings. The integrated main checkout also passes an
all-target compile check; ongoing changes from other chats were preserved.
