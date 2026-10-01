# Ascension

Start with `tools/launchers/Launch-Ascension.cmd` and locally installed original game data.

The upper gate's two leaves now draw and block movement, open through the authored
target trigger, then close after their hold. Closing leaves reverse if Alice is
in the doorway. Both rising-air columns use the shared updraft movement, while
the nearby monster-only launch pad excludes Alice.

The castle lift is visible and solid from arrival. Boarding starts the departure:
a half-second white fade, the original camera path, and a rising, turning lift
with Alice attached to its deck. The watched transfer is at 4.5 seconds; skipping
fades for half a second. The physical exit and scripted exit share one transaction,
with duplicate suppression, retry after a failed destination load and saved
delivery after a restart. The physical volume cannot bypass the lift scene.

Gate movement, scene/skip clocks, rider pose and the exit commitment are saved.
Keep starts at its named arrival lift and retains carried resources, selected
weapon, inventory, difficulty and the ledger of completed visits. Keep's previous
unconditional arrival refill was removed; the original arrival scripts contain
no such grant.

Checks are `--facade-check`, `--facade-route-check`,
`--facade-skip-route-check` and `--facade-render-check`. The native save fixtures
use `LOOKING_GLASS_SAVE_CASE=facade-` with `--save-check-write` and then
`--save-check-read` in a fresh process. Private evidence is under
`private/facade-work`.

This work owns the doors and departure. Existing placed enemies and ambushes keep
their shared family implementations; the unused alternate camera scene remains
unused as in the map's original trigger graph.

Verification uses staged component starts followed by ordinary movement through
both full updraft ascents and landings, the gate, and onto the lift. The watched
and skipped transfer matrix runs at 30, 60 and 144 Hz and checks both normal and
strict campaign arrival, the exact Keep entry (512, 288, -56), duplicate exit
suppression, failed-load retry and restoration after commitment. Five native
save fixtures cover moving gates, the opening fade, mid-ride, skip fade and the
committed exit, with a new reader process.

This is not a completed full-map combat playthrough. The lower-cave route planner
could not find the bridge connection; its exploratory logs are retained privately.
Native captures verify the gates and moving lift. Audio assets and cue clocks are
checked, but sound has not been audibly verified in the test seat.

Normal viewer runs also watched the saved lift ride and held Enter to skip it.
Both transition autosaves contained Keep entry (512, 288, -56), Sanity 37, Will
41 and the same selected Croquet Mallet. The watched autosave then loaded in a
new game process at the arrival scene.
