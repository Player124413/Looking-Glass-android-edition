# Cutscene and NPC handoff prompts

Reviewed 30 September 2026 against current source, `CUTSCENE-AUDIT.md`, the level specs and the latest validation entries. This is a code/document review, not a new native playthrough. The prompts below propose implementation work; writing this pack does not implement or verify those changes.

## What the records show

- Several early-level events already play dialogue or complete a quest but still lack their full camera sequence, actor staging and movement. `story::beats` deliberately reads reviewed dialogue lists; that does not execute the associated cinematic.
- The generic friendly-NPC fallback still displays a short acknowledgement. Real dialogue must be bound to the authored actor, visit and quest state; decorative actors must not acquire invented conversations.
- Pool currently registers its first Turtle dialogue. Its arrival, later Turtle encounters, boulder scenes and ending remain recorded gaps. Some pending triggers were previously consumed, so existing saves need explicit migration.
- Later cinematic work depends on the corresponding world changes and encounter gates. A staged scene capture does not prove the level can reach it or leave it correctly.
- The latest notes already record fixes for the Fortress arrival camera/cast, its sky, Pandemonium pipe smoke, flexible ropes, ledge recovery and Tower2 water. These are regression coverage, not new restoration tasks. Tower2's entire visit is still incomplete.
- Some older school/spec paragraphs call route checks broken or lip sync absent; newer validation supersedes those claims. Recheck each assigned issue before changing it. The recorded visit-7 campaign frontier is distinct from a missing school cutscene.

Sources: [cutscene audit](CUTSCENE-AUDIT.md), [campaign plan](CAMPAIGN_PLAN.md), [level specs](CAMPAIGN_LEVEL_SPECS.md), [validation](VALIDATION.md), [story](STORY.md), [NPCs](NPCS.md), [Pool](POOL.md).

## Instructions to paste before each task

> Work in the Looking Glass repository. Read the latest applicable campaign plan, level spec, validation entry and source before editing; distinguish historical limitations from current defects. Follow existing ownership and integration rules, preserve other work, and use the supplied original data with the existing reviewed controller approach. Implement the task below. Preserve authored quest gates, rewards, difficulty rules and old saves. For scenes, verify the actual gameplay trigger, cameras, visible cast, movement, dialogue/lip sync where supplied, pause, early/middle/late skip, mid-scene save/restart and safe return to gameplay. Natural completion and skip must have equivalent required outcomes and fire rewards/exits once. Add focused state and native visual regressions, identify the tested executable, and update the issue record with evidence and remaining limits. Keep captures/original data private. Report staged-only coverage honestly; do not mark a map complete from scene tests. Do not publish or replace the shared player build as part of this handoff unless separately authorized.

The common instructions are part of every prompt. No agent needs to implement unrelated maps or introduce a general script interpreter to complete one of these tasks.

## First batch: scenes in the already implemented opening

These give the most immediate visible improvement. Tasks for the same map or shared files should run sequentially, or have one owner coordinate them. Existing legacy scene controllers can be extended; the later SceneRunner task need not block these fixes.

### 1. Beyond the Wall: puzzle cameras

> Restore the missing authored puzzle camera sequences in `fortress2`, especially the arch-room reveal. Use the existing Beyond controller and supplied camera/actor markers. Preserve the musical puzzle, moving platforms and three portal outcomes; verify watch, skip and save/load return Alice safely without advancing an unsolved puzzle.

References: `BEYOND.md`, level spec v04. The route exists; full puzzle-camera presentation remains recorded as unfinished.

### 2. First School: theatre

> Restore full staging for `skool1`'s `Theatre_Cinematic`: original shots, participating actors, movements, gestures and dialogue timing. Preserve the existing theatre/library progression and reinforcement triggers. Demonstrate the scene through its normal contact trigger, with equivalent watched/skipped outcomes and no duplicate actors or enemy activation after loading.

References: `SCHOOL.md`, `STORY.md`, level spec v06. Extend the current event instead of adding a second quest-completion path.

### 3. First School: library and recipe

> Restore the missing staging for `shelf_cinematic`, `book_cinematic` and `Book_Ingredients_Exit` in `skool1`. Synchronize cameras, Gnome/Alice acting and the book with existing puzzle progress. Keep lifts and the recipe gate working, and verify that advancing dialogue or skipping cannot cause an early or duplicate transition to school two.

Run after task 2, or give both to the same school owner.

### 4. Second School: Gnome introduction and rescue

> Restore the original camera and character staging around `Old_Gnome_Mushroom`, the laboratory rescue and `Old_Gnome_SpiceDrops` in `skool2`. Connect existing dialogue to the correct visible Gnome and Alice performances. Preserve the three-Boojum and two-guard gates, Mushroom/Spice rewards and one-time disappearance/reappearance across saves.

References: `SCHOOL2.md`, level spec v07. Keep scene work separate from route-driver resource tuning.

### 5. Second School: ingredient and potion sequence

> Complete the original staging for `Skool2_GrowLollypop` and `Skool2_LastGnome_Cinema`: Alice/Gnome choreography, ingredient handover, pouring, condenser, potion and Star presentation. Reuse existing quest machinery. Prove each ingredient is consumed once, both rewards remain collectible, and watched/skipped/restored scenes unlock the same return route.

Run after task 4; both touch the school-two controller.

### 6. School return: observatory and shrinking

> Refine the existing `skool1` return observatory, Star, globe-opening, drinking and shrinking presentation against the supplied sequence. Restore missing shots, acting and effects while preserving the working lift and exit. Verify one-time Star/potion consumption and a safe transition to the correct Pool entrance after watching, skipping or loading midway.

References: `SCHOOL-RETURN.md`, level spec v08. This is a fidelity repair to an existing sequence, not a new shrinking mechanic.

## Second batch: NPC behaviour and shared scene support

Tasks 7, 9, 10 and 11 can touch common interaction/story files. Assign clear ownership and integrate those changes sequentially. The generic systems should have a real first consumer, not only synthetic tests.

### 7. Friendly NPC interactions

> Audit friendly NPC interaction bindings in the opening levels and Pool. Where original data defines a conversation, connect E or the authored trigger to the correct actor, visit and quest state instead of the generic acknowledgement. Keep repeatable talk separate from one-time rewards, consume E once, and suppress misleading talk prompts on decorative or unavailable actors.

First record the supported bindings so this does not invent conversations for every friendly-looking model. Preserve existing reach, facing and wall-occlusion checks.

### 8. Second School: inert enemies and missing activations

> Recheck the recorded inert Diamond guards and missing `dice_boojum`/floor-spawn activations in `skool2`; restore those still missing through the existing encounter ownership. Avoid duplicate decorative and combat actors. Add save migration, verify damage/death and authored difficulty gates, and rerun the school route without weakening combat to make the test pass.

Reference: level spec v07 and campaign M0 item 8. Schedule after tasks 4–5 or use the same owner.

### 9. Shared scene lifecycle and Pool ending pilot

> Implement the minimal planned C1 SceneRunner and adopt it for the Pool ending pilot B. Support saved shot/cast/line clocks, pause and one completion path for watch/skip. Integrate `Tears1_End_Cinematic` and its gated, one-shot exit inside the existing Pool owner. Resolve the pilot's documented dependencies and migrate previously consumed pending triggers.

References: campaign plan C1, pilot B and F1.4a; level spec v09. Follow the documented dependency order, including required world/hazard/exit support. Do not register a second controller for a legacy-owned visit or port every existing scene in this task.

### 10. Dialogue, speakers and facial bindings

> Implement the planned C3 dialogue/speaker registry for new scenes, preserving existing dialogue order and saved progress. Map cinematic Alice aliases consistently, validate audio/subtitle/animation references, and connect supplied lip tracks with a neutral fallback when absent. Use Bill's `potears2` conversation as the first data fixture and integrate it with its scene owner.

The complete Bill scene remains task 15. Reading and validating its dialogue alone is not scene completion.

### 11. Cheshire appearances and contextual hints

> Implement the missing planned C4 Cheshire bindings, starting with Pool's before/after-Turtle regions. Restore authored appearance, speech and disappearance with the correct visibility and quest conditions. Keep summoned hints independent of quest progression, preserve or migrate saved hint selection, and verify repeated summoning cannot duplicate scenes or rewards.

References: campaign plan C4, `CONSOLE.md`, level spec v09. Establish the shared pattern here; later Tower introductions belong to their map owners.

## Third batch: Pool and Temple

These scenes need the planned shared support and their level mechanics. Gate-dependent scenes must remain gated; do not make an unfinished boss or encounter auto-complete to display a cinematic. Record any necessary prerequisite separately.

### 12. Pool: missing arrival and Rabbit

> Implement `potears1`'s missing arrival/Rabbit scene from the supplied cinematic data and actual entrance thread. Restore the correct camera, Alice/Rabbit poses, movement and final cast visibility. Play it on fresh entry, preserve old-save continuation, and verify both watched and skipped paths hand control back at the authored playable location.

References: level spec v09, `POOL.md`. The current dialogue-only Turtle support does not cover this opening.

### 12. Pool: missing arrival and Rabbit

> Implement `potears1`'s missing arrival/Rabbit scene from the supplied cinematic data and actual entrance thread. Restore the correct camera, Alice/Rabbit poses, movement and final cast visibility. Play it on fresh entry, preserve old-save continuation, and verify both watched and skipped paths hand control back at the authored playable location.

References: level spec v09, `POOL.md`. The current dialogue-only Turtle support does not cover this opening.

### 14. Pool: boulder scenes and ant pushers

> Restore `Tears1_Boulder1/2/3` as complete camera/world sequences, including ant-pusher choreography and the actual falling-rock paths, collision and activation timing. Integrate the planned rock adapter rather than substituting a visual-only prop. Verify real trigger entry, pause/save restoration and the correct post-scene hazards and enemy state.

References: level spec v09 and world task W9. Do not replay already consumed events indiscriminately during save migration.

### 15. Hollow Hideaway: Bill and the house

> Restore `potears2`'s Bill conversation and suction ending: actor approaches, authored cameras, all thirteen lines, acting, flapping doors and Alice's pull into the house. Connect the real antguard kill gate and one-time Duchess transition. Bill must be hidden outside his authored state; watch/skip/save-load must preserve the same required outcome.

References: level spec v10, C3/task 10. This depends on the map's antguard and transport work; staged presentation must be labelled until the normal route reaches it.

### 16. Underwater Temple: Turtle guide and exit

> Restore the Turtle's authored guide movement, interaction/breath-bubble cues and `Utemple_Exit_Cinematic`. Connect the real exit trigger, camera path and Alice swimming performance to `garden1`. Integrate with the temple's water and mover controller; verify underwater pause/save-load and equivalent watched/skipped arrival without bypassing the route.

Reference: level spec v12. Breath and water support must be verified through the same state the player uses.

## Fourth batch: Garden scenes, one map per task

The source records these maps as incomplete. Each owner must couple scene changes to the specified world and encounter outcomes and report the remaining route work. Do not assign all four to one agent as an unbounded cinematic sweep.

### 17. Dry Landing: arrival and Rabbit

> Restore `garden1`'s arrival/Turtle performance and White Rabbit conversation using the supplied shots, actors and movement. Apply the level spec's watched/skipped end states, visibility changes and encounter cues. Connect both real triggers and verify mid-scene saves plus clear gameplay handoffs; preserve the breathing upgrade and authored route gates.

Reference: level spec v13.

### 18. Herbaceous Border: Rabbit/Hatter and bridges

> Restore `garden2`'s Rabbit/Hatter scene, bridge-collapse camera sequences and final Cheshire exchange. Integrate miniature actors and scene-local slow motion with the actual collapsing geometry. Follow each specified completion pose, especially restoring Alice's pre-scene position after the first bridge; verify saves and skip cannot drop her into missing ground.

Reference: level spec v14. This is one map's presentation scope; world prerequisites must be integrated before calling it playable.

### 19. Rolling Stones: marble chase introduction

> Restore `garden3`'s `Garden3_Start` sequence and its transition into the marble chase. Synchronize the authored camera, Alice placement, marble release and gates with the actual moving hazard. Preserve the level spec's watched/skipped outcomes and prove saving during the introduction cannot duplicate, freeze or prematurely release the chase.

Reference: level spec v15 and W9. Require real hazard support, not an animated substitute.

### 20. Icy Reception: Caterpillar and portal

> Restore `garden4`'s Caterpillar conversation and associated actor/camera staging. Keep the exit portal disabled until the required scene completes, then apply the authored world and player outcomes exactly once. Verify real encounter entry, interrupted/restarted dialogue, skip and save migration against the existing premature-exit risk.

Reference: level spec v16. Check the current level owner before adding new gates.

## Final integration task

### 21. Verify actual scene and NPC coverage

> Integrate the completed scene/NPC changes and verify them on one frozen executable. Exercise the actual triggers as well as staged captures, cast visibility during and after scenes, pause, skip, separate-process save/load and level transitions. Run the applicable route, story, facial and visibility suites; reconcile `CUTSCENE-AUDIT.md` with explicit absent, dialogue-only, staged and naturally-triggered coverage and remaining issues.

Do not convert an expected campaign frontier into a pass or infer campaign completion from a large unit-test count. Existing player builds and saves remain protected under the campaign workflow.

## Suggested order and remaining backlog

Start with tasks 1–6 for the already implemented opening. Follow with 7–11, then the Pool/Temple tasks and one Garden map at a time. Task 8 should share the school-two owner. Integrate and run task 21 at each completed batch rather than waiting until the end.

This pack is a prioritized first tranche, not an exhaustive claim about the later campaign. Centipede, Woods, Chess, Funhouse/Hatter, Jabberwock, Hedges/Towers and the finale retain their scene/cast work in the level specs; the current `keep` and `tower2` modules must be rechecked before assigning those maps. Their scene tasks should be issued with the corresponding encounter/puzzle owner so cinematics cannot bypass absent gameplay. The ending film's automatic trigger belongs to the completed Queen finale, not a standalone shortcut.

Lower-priority recorded presentation follow-ups include the Village essence cutaway and the Fortress guard attack/projectile/head-tracking choreography. They should follow the missing whole scenes; the recently corrected Fortress arrival camera should not be rebuilt merely because older audit text still lists it.
