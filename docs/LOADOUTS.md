# Campaign weapon loadouts — version 0.31.3

Every supplied level starts with the toys Alice could have collected by completing the preceding campaign visits. This assumes thorough exploration, including optional pickups. New toys in the current level remain pickups. All 36 maps are covered, with separate return visits for `fortress1`, `skool1` and `wforest` (39 visits in total).

This is an exploration convenience while campaign scripts are unfinished. It does not complete puzzles, simulate boss defeats or claim a full playable campaign. The opening `gvillage` starts unarmed. Its authored hallway Blade pickup equips the knife; subsequent campaign visits include that reward. The original opening film and the village's falling introduction now play; see [CUTSCENE-AUDIT.md](CUTSCENE-AUDIT.md) and [VILLAGE_CINEMATICS.md](VILLAGE_CINEMATICS.md).

## Using it

Launch normally, or choose a level with **Tab**, **Up/Down**, **Enter**. The selector follows the 39-visit story order, displaying level names and map IDs. The three return visits have their own rows (5, 8 and 28). **Home/End** jumps to the first/last visit; reopening the selector highlights the current visit. A return visit uses the corresponding named entrance and supported world setup; the school also uses its return-visit steam-floor platforms.

A menu selection starts a fresh visit: full Sanity/Will, the earlier weapons, Blade selected where owned, and an empty pickup history. The opening village has no selected equipment until its first pickup. This also applies when reselecting the current map. Jumping back to an early map therefore removes later weapons from that fresh start. Command-line `--map` starts the first visit.

Normal level exits preserve resources, selection, earned toys and collected-item history. They fill in any missing earlier weapons from the destination's profile, taking the greater existing/profile Dice count rather than adding it again. **R**, death retry and **Home** retain the current inventory as before. F5/F9 and Continue retain this inventory across restarts, including the unarmed opening; see SAVES.md. Older saves keep their weapons. An uncollected village Blade can still be consumed once if an earlier build granted the knife prematurely, even at full Will.

The all-toy weapon preview and sample HUD preview keep their explicit preview inventories, including after menu selection. These are separate from normal level starts. Available equipment does not imply completed attacks: Blade, Cards, Mallet and Demon Dice have implemented actions and damage; the other six toys currently have equipment models only. See [DICE.md](DICE.md) for the shared primary/alternate summon and the effect of collecting more dice.

## Acquisition milestones

The table lists when each reward becomes part of subsequent starts. Earlier rewards accumulate. Dice are counted individually, capped at three.

| After completing this visit | Added to subsequent starts |
| --- | --- |
| `gvillage` | Vorpal Blade |
| `pandemonium` | Cards |
| `skool1`, first visit | Croquet Mallet |
| `skool2` | First Demon Die |
| `potears3` | Jackbomb |
| `garden4` | Ice Wand |
| `centipede1` | Second Demon Die |
| `rchess1` | Third Demon Die |
| `funhouse` | Jacks |
| `hatter2` | Deadtime Watch |
| `jlair2` | Complete Jabberwock's Eye Staff |
| `wforest`, return visit | Optional Blunderbuss |

For example, the default school first visit starts with Blade and Cards; the school return adds Mallet and one Die. `garden1` starts with Blade, Cards, Mallet, Jackbomb and one Die. The first forest visit has the Ice Wand and two Dice but no completed Eye Staff. The forest return has every toy except the Blunderbuss, which can be found during that visit. `hedge1` and later starts have all ten toys and three Dice.

The complete visit order is:

```
gvillage → pandemonium → fortress1 → fortress2 → fortress1 (return)
→ skool1 → skool2 → skool1 (return) → potears1 → potears2 → potears3
→ utemple → garden1 → garden2 → garden3 → garden4 → centipede1
→ centipede2 → wforest → wchess1 → wchess2 → rchess1 → funhouse
→ hatter1 → hatter2 → jlair1 → jlair2 → wforest (return)
→ hedge1 → tower1 → hedge2 → tower2 → hedge3 → tower3
→ grounds1 → grounds2 → facade → keep → qlair
```

## Evidence and checks

The profiles use bounded factual observations from the user's supplied map entities, named entrances, transition destinations and weapon grants. No original script bodies are included here or executed. The completed Eye Staff is granted after `jlair2`; the forest's earlier staff part is not treated as a usable weapon. The forest first/return setup gates access to the optional Blunderbuss. The licensed [Prima strategy guide](https://oldgamesdownload.com/manual/american-mcgee-s-alice-windows-strategy-guide-english/) corroborates the earlier staff section (p. 152) and the later return's secret Blunderbuss route (pp. 213–214).

The original `Item_WeaponPickup_DeadtimeWatch` class now maps to the Watch inventory slot as well, fixing a previously omitted pickup. The supported static-pickup count is 219; reachability and script conditions remain separate limitations.

`--loadout-check` validates every map and named return entrance, corroborates pickup milestones against the installed data, checks the scripted Eye Staff grant and confirms that the actual Watch pickup is recognised. Synthetic tests cover acquisition boundaries, return visits, Dice counts, fresh starts, transition preservation, repeated grants, unknown-map fallback and preview inventories. Native checks and their limits are recorded in [VALIDATION.md](VALIDATION.md). Research excerpts and original-art captures remain private and are excluded from the source package.
