# Story restoration — updated for version 0.18

Current facial support: original lip-sync envelopes and Alice’s blink texture are now connected for implemented dialogue. See [facial animation, verification and remaining limits](FACIAL.md). Earlier facial limitations below describe previous milestones.

Version 0.18 adds the first-school Looking Glass explanation, connects theatre dialogue completion to its two guard reinforcements and doors, and activates the village guard after the third gnome conversation. Repeated dialogue does not respawn enemies. The village now reaches its original Pandemonium exit; see [VILLAGE.md](VILLAGE.md) for the limits of the longer journey to school.

Version 0.17 also restores **C** to summon Cheshire independently of automatic story scenes. His contextual/general recordings, subtitles and talk model are connected; summoned hints never advance quests. See [CONSOLE.md](CONSOLE.md).

## Try it

- **tools/launchers/Launch-Village.cmd** starts the village at `alice_cat_talk1`, the original script's playable arrival marker. The Rabbit, Alice and Cat's opening dialogue begins automatically. Walk into the village; its four gnome conversations play at their original trigger volumes or through E when near a visible gnome. E can replay a gnome conversation after it finishes.
- **Launch.cmd** retains the first school route and adds the theatre exchange, mallet hint, shelf hint, book-opening exchange and recipe reading. The recipe transition waits for the dialogue to finish. H still shows the current objective.
- **tools/launchers/Launch-School2.cmd** opens the next school area. In the east gym, face the lever and press E to extend the bleachers. All twelve tiers draw and collide; the eleven moving tiers follow the script's displacement and duration. They stop if Alice obstructs them and carry a rider where clearance permits. Approach the Elder Gnome at his original upper-gym trigger to begin his conversation. The Mallet hint, Dice pickup hint and three spatial Cat hints are also connected.

Version 0.16 adds the second school's Spice Drops and final potion conversations and connects their completions to its quest. Its entrance-to-Gnome route, Boojum encounter, laboratory rescue, growing lollipop, potion/star collection and gated return are covered by the [school-two work](SCHOOL2.md). The arrival exit remains disabled.

## Controls and presentation

**E** advances one visible line during a conversation. That press is consumed by dialogue and cannot also greet another actor, move the lever or open a door. Outside dialogue, E follows the nearby interaction prompt. Subtitles show the speaker above a bottom-centre panel, wrap to the window width, and paginate long lines rather than covering the scene. They remain available with muted/disabled/unavailable sound. F3's effects/voices setting controls speech; music is reduced while a voice is playing.

P, inventory, the level chooser, lost focus, free flight and death pause dialogue. F3 also pauses dialogue while allowing music-volume adjustment. Conversations queue without overlapping. During a conversation, Alice can still explore, while NPC combat and new weapon attacks pause. Environmental hazards still apply. H keeps instructions temporary. Home/R/Enter recovery retains completed conversation history, restarts an interrupted conversation, and cancels a deferred exit; choosing a fresh level with Tab resets story history.

Original NPC talk clips provide approximate speaking gestures. Named Cat actors hidden at startup appear for the duration of their supported scene. Lip synchronisation, automatic camera cutting, cinematic Alice replacement and exact dialogue-to-animation timing remain incomplete. School two now has explicit Gnome disappearance/reappearance, lollipop growth, potion creation and ingredient handover through inventory; it does not shrink Alice in this map. Actors outside these implemented beats can still use placeholder greetings/editor placements. The recipe book's existing gameplay motion is not resynchronised to its original cinematic.

## Implementation and provenance

The reader selects reviewed functions from the local school and village scripts and extracts only literal `playdialog` actor/path pairs in order. It does not execute script code, camera operations, console commands, spawns or quest instructions. Comments, prototypes and other functions are excluded. An explicit registry maps implemented gameplay events to those functions. The source contains asset/function identifiers and independently written code, not dialogue transcripts or voice recordings.

Subtitles are loaded from each map's `dialog/*.tlk`, handling Windows-1252 and CRCRLF line endings. WAV duration controls subtitle lifetime; missing audio falls back to a reading-time estimate. One independent Rodio voice sink streams a line at a time. Level changes dispose it with the old level's audio. The new gym interaction reads its placement and brush geometry from the BSP and uses the original lever's TAN pull animation. Camera delays and mover interaction remain approximations.

## Verification commands

`--story-check` validates **50 voice/subtitle pairs, 21 conversation/hint entries across three maps**, including full sample decoding, timing, safe village/school-two spawns, real trigger contacts, pause/re-entry and blocked premature school-two exits. Repeated assets used in two maps count as two pairs.

`--gym-check` uses the real school-two geometry to check lever facing/reach from supported footing, paused movement, completed displacement and a physical staircase climb at 30/60/144 Hz. It also checks stop/resume around a stationary player. `--school-check` and `--school-route-check` retain the first school's earlier traversal/state regressions.

`--story-preview <event>` explicitly stages a supported conversation for inspection. For example, `--map skool2 --start-at "2020 -3044 440" --story-preview Old_Gnome_Mushroom` stages the gym dialogue. This is a presentation diagnostic, not proof of reaching that scene through normal gameplay. All captures, extracts and audio remain local under `private/` and are excluded from source packaging.


Pandemonium (v0.23) adds four reviewed sequences / 15 original voiced lines: the rope hint, Elder Gnome warning, Cards hint and combined departure dialogue. Completion of the last sequence gates the airship's Fortress 1 transition. The original chase, camera cuts, fire choreography and exact acting remain incomplete; the normal route uses in-place conversation and scripted transport.
