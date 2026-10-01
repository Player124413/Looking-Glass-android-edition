# Caterpillar's Plot — both visits

The `wforest` and `wforest-return` registrations own separate states in the existing
`wforest$first` and `wforest$return` save slots. Original data is read locally; no
script bodies, dialogue text or assets are distributed with this implementation.

## First visit

Run `tools/launchers/Launch-Caterpillars-Plot.cmd`. The arrival Cat scene starts once. Touch the
Staff altar on the pillar to collect the Staff component and play the gate scene.
The component is a saved quest item, not the usable Eye Staff. The three cavegate
brushes lower together. Either western scene volume starts the same Caterpillar
conversation, which raises the chessgate. The chess approach plays its Cat scene,
and the gated exit leads to `wchess1$wchess1_start1`.

The Humpty button and Blunderbuss pickup are unavailable here. The hedge route
cannot transition. As in the supplied scripts, the Caterpillar remains hidden
during this conversation; the five voice beats and four camera paths still play.
Watched and skipped scenes commit the same progression. Cat and Alice puppets
have a single owner, and grounded handoffs are checked against live collision.

## Return visit

Run `tools/launchers/Launch-Caterpillars-Plot-Return.cmd`. The cavegate and four chesswall brushes
seal the first route, and its level-change trigger is disabled. The working Eye
Staff is granted if missing. Humpty performs his ambient cigar loop. Touch the
button beside him: it retracts for two seconds, then the secret door rises for
four. The optional Blunderbuss altar starts the Cat's appearance, line and departure.

The wall warning plays once, with the voice identified as the Gryphon rather than
Alice. The wall has 100 health and accepts Eye Staff beam, projectile and splash
damage; other weapons cannot open it. Private inspection of the original wall's
flag-8 damage branch and Staff damage discriminator confirmed this filter.
Partial damage persists. Breaking the wall suppresses an unplayed warning,
removes collision, starts two bursts, seven moving brush fragments and fifteen
model fragments, then opens the two hedge doors. Alice has brief scripted damage
protection during the collapse. Only the completed return route can request
`hedge1$hedge1_start1`; failed transitions can retry without replaying the scene.

The two essence spawns alternate after pickup with a ten-second delay. The optional
secret does not gate the hedge exit. Wall burst particles and deterministic small
rubble trajectories approximate the original cannon-driven destruction effect.

## Persistence and compatibility

Controller version 1 saves scene identity, line cursor, shot clock, completed
scenes, Staff pickup, mover clocks, secret progress, wall damage/destruction,
essence cycle and exit commitment. Cross-visit or inconsistent state is rejected
before replacing live state. Pauses freeze these clocks. Older saves without the
controller use the shared upgrade path to rearm formerly pending progression
threads and recover obstructed positions. Previously acquired inventory is kept;
an early Blunderbuss from an older build is not confiscated.

The additive `staff_component` and `scripted_immunity` inventory fields default
for older saves; the overall save format remains 12. Eye Staff damage retains
the existing generic enemy reaction while carrying a distinct wall discriminator.
Other enemy families remain under their existing owners.

## Verification

- `--wforest-check` and `--wforest-return-check`: watched/skipped scenes at 30, 60
  and 144 Hz, dialogue counts, pause/restore, grounded handoffs, duplicate-scene
  rejection, visit isolation, wrong exits, weapon filtering, partial wall damage,
  early destruction and transition retry.
- `--wforest-route-check`: continuous movement from first entry through the altar,
  cave, Caterpillar clearing, chess approach and exit.
- `--wforest-return-route-check`: the optional Humpty reward route and the mandatory
  return route, including actual Eye Staff input and the hedge transition.
  These navigation checks disable enemy targeting; they are not combat balance tests.
- `--wforest-render-check` / `--wforest-return-render-check`: native captures of
  the altar, scenes, Humpty, collapse and open hedge doors.
- `LOOKING_GLASS_SAVE_CASE=wforest-` with `--save-check-write` then
  `--save-check-read` in separate processes: moving Staff gate, half-open secret
  with damaged wall, and destruction in flight. Exact restored state and continued
  simulation are compared.

Evidence and captures are retained privately under `private/wforest-visits`.
Project-wide strict Clippy currently reports unrelated existing warnings; no
WForest diagnostics were reported.

The integrated build passed 616 unit tests, both scene contracts, both walking
routes (including Humpty's optional reward), and the Centipede2, Garden4, Queen,
swimming, world, loadout, event and heavy-weapon checks. The older all-map
`--story-check` still stops at `garden2/Garden2_Cat_End`, identically to the prior
published build. WForest's own voice and trigger checks pass independently.
