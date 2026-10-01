# Actor placement and facing — v0.31.1

Walking characters now settle onto solid support instead of remaining at their BSP editor height. This applies to the generic character/enemy cast across the campaign, including village Gnomes and the ants in both Centipede maps. Club and Diamond guards also settle before their first presentation, even when dialogue has paused combat. Their later falls still use ordinary gravity.

The placement check sweeps the original declared `setsize` bounds, scaled for the placed actor, onto the first walkable surface within 1024 units below the editor origin. It includes current doors and moving-platform collision. Only a small initial overlap correction is allowed; the search does not move an actor sideways through walls or choose a different room. Generic actors fall if their support disappears and can follow small vertical support changes. Full horizontal platform riding for arbitrary NPCs is not implemented.

The original `init/server` declarations distinguish `fly`, `swim` and zero-gravity actors. Those keep their authored altitude. Dedicated encounter/cinematic controllers continue to own their actors; generic grounding does not replace their flight, paths or attachments. No original scripts or binaries are executed.

Speaking characters keep turning toward Alice while dialogue pauses normal AI. Line of sight still matters, pause freezes turning, and a player directly above an actor does not force an arbitrary yaw. The second-school Gnome now faces Alice's position rather than the movable third-person camera. While mixing the potion, he faces his apparatus instead. His facing survives saving.

## Saves

The original spawn description stays unchanged because it identifies actors in existing saves. Settled position and falling speed are separate saved fields. Missing fields in older format-10 saves default safely and acquire support on the first update; guard placement and Gnome yaw have compatible defaults too. Campaign save format remains 10. This pass does not reset health, inventory, quest state or defeated enemies.

## Verification and limits

- `--npc-placement-check` audits the 36-map corpus against current world collision. Of 500 eligible generic declarations, 339 have safe support, with 255 height corrections greater than one unit; 128 declare flight/swimming/zero gravity. These counts include script-owned legacy declarations and are not claims that all those actors are currently visible or all map scripts work.
- 33 declarations have no safe support under these rules. Several mushrooms/roses begin embedded in terrain; other cases involve unrecovered scripted staging or large bodies blocked by scenery. The fix leaves blocked placements unchanged rather than guessing another position. These need individual map/script work.
- `--npc-placement-render-check` stages village Gnomes, Centipede-map ants and the first-school child in their authored worlds, checks facing convergence and snapshot identity, and captures their feet/faces. Other actors are hidden in these close-up fixtures so they cannot obscure the inspected model. It is not a full Centipede playthrough.
- Synthetic regressions cover scaled bounds, dynamic support, obstructed placement, airborne exceptions, dialogue turning, pause, later falling and old/new actor serialization. The real second-school check covers Gnome dialogue and mixing directions, pause and saved yaw.
- The normal-input village, first-school, second-school and Pandemonium route checks pass after guard settling changed. The exact test logs and native save/restart results are recorded in [VALIDATION.md](VALIDATION.md).

Captures and extracted original references remain private and outside the source package. Relaunch the ordinary launcher to use the updated executable; no new game is required.
