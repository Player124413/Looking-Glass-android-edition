# Castling — Wchess2

Single registered owner for campaign index 20. Entrance `wchess2_start1`; exit `rchess1$rchess1_start1`. Hit IDs remain in the reserved range starting at 8,000,000. Save envelope 12 and existing trigger identities are unchanged.

## Doors and world

All 32 rotating leaves and the small sliding secret use the visit owner. Rendered and colliding poses agree. The shared door system retains stable save entries but disables its duplicate colliders, including after restore. The 4096 bit is not a lock; 64 enables player proximity, 16 excludes actors, and explicit script locks remain separate. Door proximity uses the native 60/60/8 expansion; a nearby accepted activator renews the authored wait. Direct use retains facing, reach and wall checks for the manual secret. Castle front doors open during the Queen scene.

Native door findings: constructor 0x100631b0, linking 0x10062800, proximity 0x10064690, use 0x10063c40, completion 0x100623b0. Confidence: high for flag meanings and expansion; medium for exact simultaneous leaf linking and collision reversal fidelity.

## Royal scenes

`cinema_queen_abduction_thread` uses both supplied camera tracks, ten parent waypoints, bound Queen/abductors, knight approaches, fades and moving red platform. Dead chess props retain their frozen source frames until removal. Watching retains Alice's captured approach; skipping lands at `alice_queen_dest1`. Both remove the same cast and platform, open the castle and release the two knights/pawn spawns after the closing delay.

`cinema_king_thread` restores Alice/King approaches, rook movement, the authored camera cuts and holds, eleven dialogue calls, supplied voice/lip bindings, pawn spline/shrink/pickup presentation and weapon-change performance. The pawn scene does not grant an inventory weapon. Both completion paths land at `alice_king_dest2`, make the King solid, remove the specified battle/side actors, enable late ambush groups and start the rook paths to the portal. Dialogue is divided at the authored pawn presentation, retaining source call order and one-second gaps. Saved phase, shot, line and actor clocks resume without restarting rewards or scene completion.

The portal is hidden and non-solid before the King finishes. Its supplied four-node loop drives the brush, collision, rider carry and bound exit trigger together. The exit is one-shot per current visit and can retry a failed destination load. A restored committed exit is delivered again until the destination actually loads.

## Encounters and compatibility

Placed and spawned chess pieces belong to this owner; the generic NPC renderer and combat fallback defer. Explicit duels select the scripted opponent. White battle pieces retain 10,000 health; royal rooks retain their health floor. Side ambush alternatives exclude one another; battle two disables earlier side triggers. Broken source references are preserved as inactive follow-ups rather than invented kill gates: neither battle counter blocks the route, group four's swapped spawner names do not create actors, and the unreferenced third battle stays dormant.

Pre-controller saves restart this visit at its entrance with carried resources retained. Previously consumed scene/exit triggers are rearmed behind the new gates. Normal saves retain doors, actors, scene clocks, portal epoch and exit commitment. No source dialogue or script prose is stored here.

## Verification

`--wchess2-check` exercises real approach triggers, both royal outcomes, dialogue, pause, save futures, moving-platform support, bound exits, ambush alternatives and migration. `--wchess2-route-check` and `--wchess2-route-skip-check` traverse from the real entrance with Normal combat into a loaded Checkmate visit. Route checkpoints are private diagnostics; they do not change player saves. `--wchess2-render-check` captures eight staged views. Native save fixtures are `wchess2-queen-mid`, `wchess2-king-mid`, and `wchess2-exit-open`.

Use `tools/launchers/Launch-Castling.cmd` for a fresh visit with separate playtest saves. Release identity, exact test results and limits are recorded in `private/wchess2/WORKLOG.md`.

Native pathfinding, turn-rate details and precise sub-frame camera/animation phase remain fidelity comparisons. Static sound/voice validation and silent native rendering do not establish an audible side-by-side comparison.
