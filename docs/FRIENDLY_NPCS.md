# Friendly NPC interactions — 2026-09-30

The allow-list was recorded in `private/friendly-npcs/BINDINGS.md` before implementation, against main f640a49 and the locally supplied original BSP/script files. Only executable script calls count; dialogue prose is loaded from the user's archives at runtime.

## Selectable conversations

| Map / campaign visit | Actor | Authored thread / zero-based BSP trigger | Availability |
|---|---|---|---|
| gvillage /1 | torchgnome2 | Torchgnome1_Dialog /316 | First conversation; speech-only repeat after scene completion |
| gvillage /1 | torchgnome1 | Torchgnome2_Dialog /320,360 | First conversation; speech-only repeat after scene completion |
| gvillage /1 | torchgnome3 | Torchgnome3_Dialog_part2 /23 | First conversation; speech-only repeat after scene completion |
| gvillage /1 | torchgnome4 | Torchgnome4_Dialog /19 | First conversation; speech-only repeat after scene completion |
| skool2 /7 | old_gnome_1 | Old_Gnome_Mushroom /80 | Explore only |
| skool2 /7 | old_gnome_2 | Skool2_LastGnome_Cinema /60 | Lollipop stage with all three ingredients |
| potears1 /9 | turtle_talk | Tears1_Turtle_Cinema1 /110 | Before the first conversation and departure |

These pairings have **high confidence** from placed entities and executable dialogue callers. E and repeat village speech are **port interaction policy**, not a claim about the original input key. First E activates the same registry-owned trigger as authored contact, preserving its gate, one-shot consumption, staging and rewards. School2 and Pool obtain their target from the controller's actual drawn actor pose. No reward or inventory mutation lives in the binding layer.

## Automatic and excluded actors

**High confidence:** village entrance/Cat pickup/hint scenes; Pandemonium warning, return/departure and Cats; fortress arrival/return, pickup and difficulty/puzzle Cats; school theatre, shelf, book, recipe and pickup scenes; school-two rescue/Spice Drops and weapon Cats; school-return Star/globe/potion scenes; Pool arrival, Turtle movement encounters and exit retain their authored triggers or scene/combat gates. They do not acquire idle E bindings.

**High confidence:** working minecart gnomes, walking/ambient pupils, fortress muzzled/pillar actors, Gnome props and Walkrocks have no reviewed standalone conversation. Friendly model metadata cannot create a Talk prompt. Hidden, controller-owned, cooling-down or unavailable actors cannot intercept E.

**High confidence:** `cat_fluff_cinematic` has commented-out voice/camera instructions; its executable body removes the Cat. Village `Essence_Cat_Thread` and `Bridge_Tenticle_Thread` have no caller/placed trigger in the reviewed map. Their dialogue definitions alone are insufficient for an interaction binding.

**Known separate gap:** `potears2` Bill has authored dialogue, but its complete ant-guard rescue/scene owner is pending. It remains unavailable for E rather than offering a generic acknowledgement or an unearned rescue reward. `potears3` Bill/Mock Turtle belong to Duchess victory staging.

## Input, persistence and verification

The same resolver selects HUD prompt and input owner: current dialogue, shared rope, legacy Pandemonium rope, supported conversation, world use, then traversal fallback. Cinematics suppress world prompts. A press starting a conversation cannot also advance its first line, use a lever/door or grab a rope. The legacy rope consumes its key before world use.

The existing NPC reach checks remain exact: target height `clamp(32*scale,12,100)`, squared distance strictly below `140^2`, facing dot strictly above `0.75` except within `20`, and a half-unit world sweep with no starting solid or obstruction. Controller-owned actors use the same geometry. No through-wall selection or extended reach is introduced.

Only completed village conversations repeat, as speech. They do not emit `DialogueFinished`, add completion counts, replay cameras/shrink/encounters, or change items/exits. An optional omitted-when-false `repeat` bit on saved Story sequence references preserves this distinction through load. Old saves default to authored completion. Save version12, fresh snapshots, event rule definitions, visit/route numbers and hit-ID ranges remain unchanged.

Automatic village conversations also arm the NPC's existing saved cooldown. A quick
extra E press at the end cannot immediately restart the speech; after one second
of gameplay the explicit **Talk again** prompt becomes available. Completed Gnomes
stay at their scene endpoints. See [the opening audit](OPENING_MAP_AUDIT.md).

`--friendly-check` checks actual BSP bindings, existing trigger consumption, wrong visits, quest stages, all ingredient combinations, active-scene restoration, safe scene handoffs and reward-free repeats. Unit tests cover geometry boundaries, single-owner input and active/queued repeat save restoration. Full scene checks remain separate from route proof. Native E/prompt and save/load checks use isolated test saves; details and results are recorded in `private/friendly-npcs/WORKLOG.md`.
