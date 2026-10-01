# NPCs — updated for version 0.31.1

Red pawns, knights, bishops and rooks now have combat, pain and death animations,
freezing, essence drops and saved encounter state. Reviewed ambushes work across
the four chess maps; white allies and scripted royalty retain scene ownership.
See [chess enemies, playtest and remaining scene work](CHESS_ENEMIES.md).

Army Ant soldiers and Corporals now have shared combat, damage reactions, three
death variants, freezing, essence rewards and persistent defeat. Ordinary placed
Ants fight; actors waiting for unfinished scripted sequences remain passive but
damageable. Existing Pool pushers and house guards keep their scene owners.
See [Ant behaviour and playtest](ANTS.md).

The September animation pass adds same-posture idle/talk variations, Torch Gnome talk transitions, varied Cheshire hints and skeletal prop support for the Hatter's cane. See [animation restoration](ANIMATIONS.md).

Current facial support: original lip-sync envelopes and Alice’s blink texture are now connected for implemented dialogue. See [facial animation, verification and remaining limits](FACIAL.md). Earlier facial limitations below describe previous milestones.

Version 0.31.1 grounds walking placements using original declared body bounds and current world support. Flyers/swimmers retain their altitude. Dialogue presentation can turn toward Alice while ordinary AI pauses, and the second-school Gnome follows Alice rather than the camera. Grounded positions retain the original saved actor identities. See [ACTOR_PLACEMENT.md](ACTOR_PLACEMENT.md) for the village/Centipede checks and remaining embedded or scripted cases.

The twelve `potears1` Ladybugs now belong to the combat encounter controller. It handles their navigation, bombs, reactions, death and scripted activation. Legacy decorative records remain in save snapshots for compatibility but are neither updated nor rendered in this map, preventing duplicates. Other maps' Ladybugs remain decorative. See [LADYBUGS.md](LADYBUGS.md).

Version 0.18 moves the reviewed village/first-school Club guards, Diamond guards and Boojums into explicit encounter ownership. Their original models/animations and target-based activations are connected without duplicate generic placements. First and return school entries select separate activation groups. These actors retain death/activation across recovery, unlike the older generic club-guard reset behaviour below. See [SCHOOL.md](SCHOOL.md), [VILLAGE.md](VILLAGE.md) and [COMBAT.md](COMBAT.md).

Version 0.16 connects the second-school Gnome's gym, rescue and potion appearances, their dialogue, and the scripted Boojum/club-guard encounters. Quest actors use a separate state machine so the generic placement reader does not duplicate them. See [SCHOOL2.md](SCHOOL2.md). Version 0.15's original voices and subtitles remain available for selected village and school characters, with named speakers using their talk clips. Script-hidden Cats appear during their conversation. See [STORY.md](STORY.md) for supported beats and remaining choreography limits.

Version 0.13 gives **club card guards** a bounded combat state machine, local pursuit, timed melee attacks, damage reactions and death. The ordinary NPC preview remains peaceful; the separate combat preview enables a single guard. See [COMBAT.md](COMBAT.md) for weapon damage, Will costs, reset behaviour and limitations. The stationary behaviour below still applies to other NPC types.

Normal launches render supported characters placed in the level data. **E** is offered only for a reviewed, currently available conversation. Ambient/decorative actors no longer offer generic acknowledgements. Active dialogue takes priority over other E actions, and repeated village speech cannot repeat quest rewards. See [the supported bindings and automatic-only scenes](FRIENDLY_NPCS.md).

**tools/launchers/Launch-NPC-Preview.cmd** stages the Cheshire Cat, a gnome, a club card guard and a schoolchild in the school entrance corridor. These four preview placements replace the ordinary NPC list only for the school's first visit. Use **V** for a clear first-person look and **P** to freeze the scene. The ordinary **Launch.cmd** uses the authored placements instead. The preview does not unlock weapons or save changes to game data.

## Implemented

- Independent loading of SKB bodies, SKA idle/talk clips, declared skins, hidden initial surfaces and TAN attachments, such as guard weapons and schoolchild headgear. Models are shared between repeated actors; body rendering shares Alice's skinning and fog shader.
- Local turning toward visible Alice within 400 map units, extended to 1200 for an active greeting/dialogue. Turning continues during dialogue presentation, without advancing combat AI. Greetings require a friendly character within 140 units, in front of Alice, with unobstructed sight. Both stationary world geometry and current door collision block this check.
- NPC rendering uses world depth and fog before transparent effects and first-person weapons. Pause, menus and death freeze animation/reactions. Free flight also freezes their simulation. Changing levels rebuilds the local cast; Home and recovery preserve friendly actors but reset club-guard combat.
- Generic unflagged `Characters_*` and `Enemies_*` placements. Named school setup hides the initially hidden scripted cast, moves the talking gnome to the initial marker, and removes the first-visit cast on the return visit. Selected hidden Cats are loaded for story scenes and revealed only during the matching dialogue. Other deferred actors remain absent.

The current local data audit (`--npc-check`, also run by `--animation-check`) covers 484 eligible first-visit placements across 36 maps and validates all 63 model definitions, including 222 selected clips, body textures, finite skinning and attachment textures/tags. The Mad Hatter now loads with his skeletal cane; his boss encounter remains unfinished. Unsupported models are logged and skipped without preventing the rest of the level from opening.

**Off-norm attachment tags (`c_clockwork`).** Every skinned bone's packed rotation must be unit length. The Clockwork Automaton's animations store its two hand tag bones (`tag_left_hand`, `tag_right_hand`; unskinned, they carry no vertex weight) at a reduced length in many clips, including the frozen-death alias. Loading that alias for every model exposed this as a regression: the audit failed for `c_clockwork` from the Ice Jacks checkpoint onwards, and its placements were skipped. The reader now accepts a non-unit rotation only on an unskinned `tag_*` bone, inside a bounded squared-length band, and renormalises it. The audit reports such a model on an `ALLOWED` line from the named `OFF_NORM_TAGS` list in `src/npc.rs` (model, tag bones, written reason). It fails for any unlisted model or tag that needs the tolerance, and for a listed entry that no loaded clip needs any more. The restored model now has combat in funhouse, hatter1, hedge2 and hedge3; see [Clockwork support](CLOCKWORK.md). Earlier builds used idle placeholders; a save written by an executable that skipped them will not match those four casts, see SAVES.md. Multi-stage character materials currently use their resolved base image; effects such as the Boojum's additive skin layer are incomplete.

## Remaining work

NPC bodies remain non-solid to Alice. Supported Club/Diamond guards, Boojums, Army Ant soldiers/Corporals and Pool of Tears Ladybugs fight using the rules in COMBAT.md; other enemy types remain stationary actors with visual reactions and cannot be injured. Selected friendly scenes have original voices and quest consequences, while generic greetings remain visual reactions. Lip sync, general navigation, general scripted path following, loot and broader enemy behaviour are incomplete. Implemented actor/enemy state is now retained by persistent saves; see SAVES.md. Selected club-guard, Boojum and Gnome sounds/events are connected.

Difficulty flags are supported; other nonzero spawn flags are deferred unless an explicit encounter controller handles them. School two handles its researched quest spawns explicitly. General script-spawned actors and multipart bosses are not instantiated by the generic layer. Original map scripts and cutscenes are not executed. Outside the researched rules, authored placement does not establish correct story visibility or visit state; some blocked/scripted actors remain at their editor positions. Both schools' first-visit main routes and the first-school return have implemented progression; broader campaign restoration remains incomplete.

The public source allowlist includes this independent loader and synthetic tests, not models, animation data, script extracts or screenshots. All original assets remain read-only and local.

Fire Imps now have shared combat, fork/death presentation, reviewed ambushes and saved state. See [FIRE_IMPS.md](FIRE_IMPS.md) for coverage, checks and remaining approximations.

Clockwork Automata now have punch combos, seeking fists, steam attacks, pain/death and saved combat state. See [CLOCKWORK.md](CLOCKWORK.md) for supported map activations and remaining scene work.
