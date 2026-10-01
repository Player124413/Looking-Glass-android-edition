# Second school

**tools/launchers/Launch-School2.cmd** starts at the normal `skool2` entrance. The main quest now runs through the Elder Gnome, gym battle, laboratory rescue, ingredients, potion and return portal. **H** shows the current objective, **I** shows collected quest items, and **E** operates doors/the gym lever or advances dialogue. Walk into available items to collect them. Blade, Cards and Mallet work against the implemented enemies; right-click throws the Blade without spending Will.

## Route

1. Enter the east gym. Face the lever and press E to extend its bleachers. Climb the tiers and upper beams to approach the Elder Gnome. The floating books and cabinet, swinging rods and attached lamps now have moving collision; Alice can ride supported surfaces.
2. Finish the Gnome's conversation, then defeat the three activated Boojums. They fly, scream, react to hits and die. Their moving scream rings damage Alice only when they reach her without a wall in the way. Completing the battle opens the next quest stage.
3. Return to the main hall and climb its broken central stairs to the upper laboratory. Rescue the Gnome from the two club guards. His next conversation supplies Spice Drops and opens the Jumbogrow cabinet. Collect the bottle there.
4. Use the corridor mirror to reach the upper west classrooms and greenhouse. Approach the small lollipop with Jumbogrow: the bottle is consumed and the lollipop grows over 5.5 seconds. Collect the grown lollipop, jump out of its raised bed, and return through the mirror.
5. Return to the Gnome with Mushroom, Spice Drops and Lollipop. Finish his conversation; the ingredients are consumed and the condenser/potion sequence runs. Collect **both the Drink Me potion and Lucky Star** near him. The return portal then leads to the first school's `skool1_start2` entrance.

Ordinary doors use E. The gym battle temporarily closes its doors; if recovery leaves Alice outside, she can re-enter to finish it. Neither the early arrival portal nor a single reward bypasses the quest. Repeated triggers, pickups and dialogue completions do not grant duplicate progress.

Home, R and death retry retain this visit's quest progress and collected ingredients. Choosing a fresh level with Tab starts a fresh quest. Normal level transitions retain quest items and cache each visited level. F5/F9 and Continue restore quest progress after closing the program; see SAVES.md. This map creates a shrinking potion; Alice's actual shrinking and the first-school return quest are implemented separately; see [SCHOOL-RETURN.md](SCHOOL-RETURN.md).

## Implementation and limits

The independent Rust state machine uses names, placements, clip metadata and reviewed event ordering from the user's local BSP, TIKI and script data. It does not execute original scripts or native game code. Original meshes, voices, subtitles and animation clips are read from the local archives at runtime; none are included in the source package.

The gym pendulums use shared rigid motion for each rod and its lamp. Moving surfaces carry Alice only when the destination clears the world, and stop/resume around obstructions. Floating book/cabinet paths, slamming doors, cabinet opening, Gnome disappearance/reappearance, growing lollipop, pouring/condenser animation and growing potion are connected. The Lucky Star uses a reachable location near the Gnome's hand marker.

Boojums have bounded flight and local collision, timed scream attacks, pain interruption and falling/shrinking death presentation. The laboratory guards initially threaten the Gnome and turn on Alice when hit. Selected extra school-two ambushes are connected, including the Dice detour. These are independently implemented behaviours, not an exact reproduction of original navigation, seeking, knockback, difficulty tuning or every animation event. Other enemy types, general crowd collision and loot remain incomplete. The Mushroom conversation, post-battle laboratory reappearance and Spice Drops exchange now use the local camera tracks, acting clips, mouth animation, held ingredients, Boojum entrance path and disappearance effects. The rescue itself remains a live fight: it starts at its existing trigger and both guards must be defeated before Spice Drops. The greenhouse growth and final laboratory exchange now use authored cameras, saved animation phases, condenser motion and audio, potion growth, held Star and Gnome disappearance. Other optional scenes remain incomplete; floating-path timing is approximate.

## Gnome scenes and saves

The Mushroom exchange ends by releasing three live Boojums. Defeating all three
starts the laboratory reappearance: the Gnome grows into view and walks to his
laboratory marker. Skipping that reveal does not defeat the rescue guards.
Spice Drops follows the rescue, with a camera cut to the opening cabinet and
back to Alice and the Gnome. The held ingredients and cabinet reveal are
presentation; the existing quest gates still control collection and departure.

E advances dialogue without bypassing the scene's remaining action. Hold Enter
to skip the current scene. F5/F9 resumes the camera, dialogue, cabinet motion and
Gnome appearance from their saved state. A completed scene does not replay.
Older format-12 saves without staging fields keep their existing quest progress
and can finish an already active conversation normally. New scenes retain Alice's
pre-scene position for a supported return to control.

## Lollipop and potion creation

Jumbogrow starts the greenhouse camera, the rising and tilting beaker, and the
lollipop growth. It is consumed once when the trigger is accepted. The grown
lollipop becomes collectible after the scene, with the existing live ambush.

The final Gnome scene requires Mushroom, Spice Drops and Lollipop. The first
voice leads into the full mixing clip and condenser animation. After the camera
cut, the potion grows while the Gnome holds and presents the Star. He disappears
before control returns. Completion consumes the three ingredients once; potion
and Star remain separate pickups, both required for the return portal.

E can shorten a voice but cannot bypass mixing or potion growth. Holding Enter
commits the same result as watching. F5/F9 restores pouring, mixing, camera cuts,
Star presentation and disappearance without replaying consumption or rewards.
Older saves already growing or mixing retain their previous completion path.
Alice uses her authored idle staging: the supplied final Alice animation thread
has no additional gestures. The Gnome uses the supplied mixing and talk clips.

## Encounter ownership (M0 item 8)

The six placed Diamond guards now fight through the shared `Encounters` owner.
The lollipop trigger also releases `cgd_spawn1`; its watched and skipped endings
remove `kill_diamond`. The dice conversation releases both authored `dice_boojum`
placements on Normal/Hard/Nightmare and only the first on Easy (entity 202 has
spawnflags 320). Existing quest Boojums, rescue guards and dice Clubs retain their
School2 owner. Other placed Clubs retain their NPC owner.

The two floor markers `spawn_floor2_guard1` and `spawn_floor3_guard1` now have
explicit spawn receivers. Static inspection found no incoming BSP target or
executable script call for either marker, so they remain dormant in normal play.
No new trigger or automatic ambush is inferred from their names.

Format-12 saves made before this repair gain the new encounter list while
retaining quest, reward, trigger and existing combat state. The NPC cast is
regenerated with its original slots, keeping Club hit/loot indices stable; the
NPC owner suppresses the old decorative Diamonds. New saves retain injuries,
deaths, projectiles and one-time activations. Original event keys, visit numbers
and hit-range reservations are unchanged; only School2's event signature grows.

## Verification

- `--school2-check` loads the real map and checks mover collision, riders, shared rod/lamp phase, pause and 30/60/144 Hz agreement. It checks premature exits, repeated rewards, and the requirement for two distinct rewards before the correct return entrance unlocks. These component cases explicitly stage states.
- `--school2-route-check` continuously replays movement, jumping, looking, E interactions, dialogue advance and combat from the normal entrance. The staging run completes in **35,802 ticks with 7 jumps, 12 throws, 3 swings, 126 cards, 100 combat damage and 86 Sanity remaining**. It reaches the Gnome, defeats all three gym Boojums, rescues him from both guards, completes the ingredients, collects both rewards and reaches the gated return. Its two teleports are the level's normal mirrors. It uses no flight, debug warps, recovery, resource refills or direct quest-state changes. The separate strict campaign remains limited at visit 7 with carried resources; see VALIDATION.md.
- This route is a headless input simulation, not a native human playthrough. It uses the shared animation/projectile/contact path but launches its thrown Blade from eye height rather than the rendered weapon attachment. Native staged tests separately checked moving/quest models, Boojum activation and scream rendering, damage/death/retry, and the inventory's retained Mushroom.
- `--school2-render-check` provides seven encounter shots, twenty-two scene shots and eight existing staged presentation fixtures. These do not establish progression. `--story-check` validates the new voices/subtitles; `--audio-check` fully decodes the added sound effects. Native testing used `--no-audio`, so listening and perceived mixing were not verified.

See [VALIDATION.md](VALIDATION.md) for logs and regression results. Original data, research extracts and captures remain private and excluded from source packaging.
