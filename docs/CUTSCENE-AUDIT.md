# Campaign scene audit — 2026-10-01

This replaces the September table, which predates most level controllers. Evidence is override-resolved local scripts, BSP bindings, cameras, animations and the current implementation. Identifiers, numbers and paraphrase only.

**Present** means an owned, reachable scene has presentation and completion logic. It does not mean exact original parity. **Partial** names known presentation limits. **Unbound** means no live caller, model event or BSP binding was found. A matching identifier, playing voice or working exit alone is not scene completion.

Confidence: **high** for the bindings and tested state commitments below; **medium** for visual equivalence without a running-original comparison. The private per-function inventory includes helpers and unused variants, so its row count is not a count of missing scenes.

## Gaps repaired in this audit

| Visit | Missing moment | Restoration | Progression and compatibility |
| --- | --- | --- | --- |
| 1 `gvillage` | Essence tail after `Torchgnome2_Dialog` | `Essence_Cat_Dialog_Thread`, `gvillage_metap2`, actor `essence_cat`, fade/sound pair, idle/smile and facial binding. Owner: `src/village/cinema.rs`; appended line: `src/story/registry.rs`. | Fidelity. Repeat Gnome conversations keep four lines; completed old saves do not replay the tail. Existing line indices stay stable. |
| 6 `skool1` | `Skool1_Book_Win` | Fourth book opening starts `skool1_bookp1`: 0.5-second fades and six-second hold. `src/school.rs`, `src/school/cinema.rs`. | Reveal only. No recipe grant, book descent or early school-two exit. Saved camera/home clock; settled books do not replay. |
| 7 `skool2` | Gym lever staging | Real E use starts `extendBleachers`: `skool1_bleach_p1`, then `skool2_gnomew1`, Gnome `letsgo`, Alice attention and final fade. `src/school2/cinema.rs`, `src/interaction.rs`, `src/gym.rs`. | Existing machinery owner retained. Movement starts after 1.5 seconds plus its one-second delay. Skip becomes available at 1.5 seconds; movers continue normally. Saved movement sound follows machinery after skip. Exact earthquake displacement remains partial. |
| 7 `skool2` | Dice pickup shot | Pickup 44 queues `dice_cat`; owner supplies `skool2_dicep1`, Cat fade, supplied animation sequence, voice delay and closing fade. | Existing ambush starts only after completion. Watch/skip share the callback. Generic NPC identity stays in old saves but its duplicate rendering is suppressed. Completed ambushes do not replay/resurrect. |
| 7 `skool2` | Demon-vial Cat hint | `trigger_cat_rage`, actor 20 `rage_cat`, `cat028.wav`, saved appearance/headwatch and sound pair. `src/school2/rage_hint.rs`. Contact 783 on Easy; 32 on Normal/Hard/Nightmare. | Fidelity; no camera/reward invented. Previously unsupported consumed contacts rearm only for an unseen hint. Completed hints stay consumed. Exact fade-reversal opacity remains approximate. |

Save envelope 12, event-rule keys, visit numbers and reserved hit-ID ranges are unchanged. Optional fields default when absent. No migration guesses that a puzzle was solved.

## Coverage by campaign visit

Numbers are one-based, including returns. Helper threads are grouped with their owning scene. Linked map notes retain detailed implementation and fidelity limits.

| Visits / map | Current scene coverage | Owner / remaining limits |
| --- | --- | --- |
| 1 `gvillage` | Present: arrival, Rabbit/Blade/shrink, Gnomes, new essence tail and contextual Cats. | `src/village/cinema.rs`; [Village](VILLAGE_CINEMATICS.md). Navigation/interpolation approximate. |
| 2 `pandemonium` | Present: warning, cart, return-house and airship departure. | `src/pandemonium.rs`; [Cinematics](CINEMATICS.md). Movement/portal-view fidelity limits remain. |
| 3,5 `fortress1` | Present: airship arrival, vial transformation, return Boojum reveal and exit. | `src/fortress/cinema.rs`; [Fortress](FORTRESS.md). Exact camera/guard choreography and effects remain comparisons. |
| 4 `fortress2` | Present: puzzle cameras including arch room, Rage sequence. | `src/beyond/cinema.rs`; [Fortress](FORTRESS.md). Watching does not solve unsolved machinery. |
| 6 `skool1` | Present: theatre, shelf opening/conversation, new four-book reveal, recipe book and recipe exit. | `src/school/cinema.rs`, `src/school/library.rs`; [School](SCHOOL.md). Retains paintings and downstairs theatre handoff. |
| 7 `skool2` | Present: new gym/Dice/vial support, Mushroom, rescue, Spice, growth and final potion/Star. | `src/school2/cinema.rs`; [School](SCHOOL.md). Quake, precise navigation and sub-frame acting remain partial. |
| 8 `skool1` return | Present: observatory, Star, globe, drink/shrink and Pool handoff. | `src/school/return_cinema.rs`; [School](SCHOOL.md). Existing one-time item gates retained. |
| 9 `potears1` | Present: arrival/Rabbit, two camera boulders, Turtle conversations/departures and final jump. Boulder3 is a hazard activation, not a third camera scene. | `src/pool/cinema.rs`; [Pool](POOL.md). Real rocks/ant pushers connected; exact curves/navigation approximate. |
| 10 `potears2` | Present: thirteen-line Bill conversation, approaches, doors and suction after antguard gate. | `src/levels/potears2/scene.rs`; [Hideaway](HOLLOW_HIDEAWAY.md). Missing source animation uses documented supplied idle fallback. |
| 11 `potears3` | Present, partial parity: Duchess introduction/victory focus/effects, shell/Turtle and exit. | `src/duchess.rs`; [Duchess](DUCHESS.md). Body stretching and thrown head/brain geometry remain missing. |
| 12 `utemple` | Present: guide/breath cues and `Utemple_Exit_Cinematic`, using real water/movers. | `src/levels/utemple.rs`; [Temple](UTEMPLE.md). Final bubble bounds and exact swim interpolation approximate. |
| 13 `garden1` | Present: Turtle arrival, Rabbit conversation, dead-tree Rabbit. | `src/levels/garden1.rs`; [Dry Landing](GARDEN1.md). Real currents, movers and gates integrated. |
| 14 `garden2` | Present: Rabbit/Hatter, both collapsing bridges, miniature cast and final Cat. | `src/levels/garden2/scene.rs`; [Border](GARDEN2.md). Real geometry; exact deformation/slow-motion rendering partial. |
| 15 `garden3` | Present: `Garden3_Start` and real marble-chase handoff. | `src/levels/garden3.rs`; [Rolling Stones](GARDEN3.md). Hazard clocks persist; native navigation approximate. |
| 16 `garden4` | Present: marble arrival, Caterpillar and gated portal. | `src/levels/garden4.rs`; [Icy Reception](GARDEN4.md). Drug-view and exact portal rotation partial. |
| 17,18 `centipede1/2` | Present: Flora speech/rush/reveal; Sanctum arrival, grab/defeat, growth and exit. | `src/levels/centipede1.rs`, `src/levels/centipede2.rs`; [Flora](CENTIPEDE1.md), [Sanctum](CENTIPEDE2.md). Native AI/navigation/attachment parity partial. |
| 19,28 `wforest` | Present: arrival, Staff, Caterpillar/chess, return wall warning/destruction. | `src/levels/wforest/scene.rs`; [WForest](WFOREST.md). Rubble trajectories approximate. Carried full run still stops in return combat. |
| 20 `wchess1` | Present: arrival, two transformations, bell and water. | `src/levels/wchess1`; [Pale Realm](WCHESS1.md). Puff and precise animation phase partial. Unbound Rook alternate is not a missing live scene. |
| 21 `wchess2` | Present: Queen/abduction, King/pawn, moving portal. | `src/levels/wchess2`; [Castling](WCHESS2.md). Exact navigation and animation phase approximate. |
| 22 `rchess1` | Present: King encounter/beheading and Hatter transition; partial optional asylum acting. | `src/levels/rchess1/scene.rs`; [Checkmate](RCHESS1.md). Head tracking and optional performance incomplete. |
| 23 `funhouse` | Present: Tweedle defeat, Hatter and physical floor ending. | `src/levels/funhouse`; [Mirror Image](FUNHOUSE.md). Detailed gestures, death sinking and child paths partial. |
| 24 `hatter1` | Present: mirror, Hare/laboratory, port and Gryphon/clockroom. | `src/levels/hatter1`; [Clockwork](HATTER1.md). Approaches and dunk/electric processing are bounded interpretations. |
| 25 `hatter2` | Present: Hatter encounter, Cats, Gryphon release/ride and ceiling departure. | `src/levels/hatter2`; [About Face](HATTER2.md). Exact movement/blending partial. |
| 26 `jlair1` | Present: arrival and Caterpillar/Cheshire reveal. | `src/levels/jlair1`; [Burning Curiosity](JLAIR1.md). Native interpolation comparison remains. |
| 27 `jlair2` | Present: Jabberwock intro, survival encounter and departure. | `src/jabberwock`; [Jabberwock](JABBERWOCK.md). Aerial choreography/root motion approximate. |
| 29 `hedge1` | No live cinematic supplied; child chase/plate/departure are gameplay. `SeekCine` and its call are commented out. | `src/levels/hedge1`; [Maze](HEDGE1.md). Do not invent a departure movie. |
| 30 `tower1` | Present: Cheshire introduction. | `src/levels/tower1`; [Airborne Terror](TOWER1.md). Gust physics has separate fidelity limits. |
| 31 `hedge2` | Present: lever callbacks and camera with actual world motion. | `src/levels/hedge2`; [Mystifying Madness](HEDGE2.md). Same owner controls completion. |
| 32 `tower2` | Present: opening Cat with real water/flush machinery. | `src/levels/tower2`; [Water Logged](TOWER2.md). Bubble bounds approximate. |
| 33 `hedge3` | No supplied cinematic required. | `src/levels/hedge3`; [Labyrinth](HEDGE3.md). Pendulum dynamics are a separate remaining item. |
| 34 `tower3` | Present: authored camera/Cat progression and bound machinery. | `src/levels/tower3`; [Machinations](TOWER3.md). Native rotation orientation follow-up remains. |
| 35 `grounds1` | Present: Gryphon/Jabberwock intro, battle phases, farewell, Cat, bridge/gnome departure. | `src/jabberwock`; [Royal Rage](JABBERWOCK.md). Some aerial/bridge staging approximate. |
| 36 `grounds2` | Present: arrival, launch/collapse and exit gates. | `src/levels/grounds2`; [Battle Royale](GROUNDS2.md). Scene-only pawn is not an extra enemy. |
| 37 `facade` | Present: live lift/exit scene. Alternate `Facade_Cinema1` unbound. | `src/levels/facade`; [Ascension](FACADE.md). Fresh standalone native route passed; carried campaign not yet here. |
| 38 `keep` | Present: arrival, Cat/portrait/heart and Queen transition. | `src/levels/keep`; [Keep](KEEP.md). Existing portrait/route gates retained. |
| 39 `qlair` | Present: corridor, first Queen, birth, second Queen destruction and film request. | `src/levels/qlair`; [Finale](QLAIR.md), [audit](QLAIR_AUDIT.md). Standalone watched routes passed; Hard skipped route and this carried ending remain unproved. |

## Unbound source and verification limits

High confidence for search results, medium for historical intent: `GTeaPickup`, `Bridge_Tenticle_Thread` / `Bridge_Cat_Thread`, `cinema_red_rook1_attack_thread`, `QLair_Dialog1`, `WockRoar` and `Facade_Cinema1` have definitions/declarations but no live caller, model event or BSP binding in the supplied resolved assets. No new contacts are invented for them. `oldqlair.scr` and alternate cinematic files are variants, not extra obligations. Fortress2's live Rage sequence already uses its relevant Cat line; its unused `trigger_cat_rage` is not a second pickup scene.

Private evidence: `private/scene-audit-20261001/` contains the function inventory, source references/hashes, checks and captures. `private/full-run-20261001/WORKLOG.md` separately records continuous-route evidence. Source references alone do not establish visual equivalence.

The refreshed `--story-check` passes 261 registered voice/subtitle pairs and runs each registered owner's `--<owner>-check` for its gates and scene clocks. The old binary reproduces the generic check's incorrect immediate-dialogue assumption for staged contacts. Legacy owner checks remain necessary. All 644 unit tests passed at the audit checkpoint; final-candidate evidence is recorded privately.

The newly restored scenes have pause, watch/skip and save fixtures. Dice/gym shots were rendered and inspected using supplied cameras and cast. Separate fresh native watched and skipped runs now pass all eight opening visits with live enemies and carried resources. Audio assets are decoded/validated; the isolated desktop has no audio output for a listening comparison.

The native continuous campaign currently proves visits 1–27 with live enemies, carried resources and real exits. Return-WForest combat/traversal remains the frontier. Later standalone checks and prior ending-film/credits playback do not replace a carried completion. The selected root `Launch.cmd` build is updated after relevant candidate checks; exact hash and remaining acceptance are recorded privately. Opening/ending films retain existing playback, pause and deliberate skip. New Game plays the opening; loading does not replay it.
