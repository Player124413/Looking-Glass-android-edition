# Herbaceous Border

Visit 14, `garden2$garden2_start1`, has one registry owner. Save envelope 12,
route order and existing trigger keys are unchanged. New Ant receiver keys use
`garden2/ant/<entity>`; targets occupy the reserved 7,300,000 range.

The arrival uses the supplied Rabbit, miniature Rabbit/Hatter, Alice and Cheshire
assets and camera tracks. Dialogue is read from the user's data at runtime. The
script's `sit_smile_close` request maps to the model's `sit_smile_shut` clip.
Missing references remain unresolved rather than creating invented entities.

Watched and skipped scenes share their completion transaction:

| Scene | Required result |
| --- | --- |
| `Garden2_Start` | Dead Rabbit visible, live Rabbits and first Cat removed, Hatter ready at `rabbit_tiny_pos1`; Ladybug 154 active and 599 queued after 2.1 seconds; Alice at `alice_pos_squish2`, yaw 135 |
| `Collapse_Bridge1` / `Collapse_Bridge2` | Six first-bridge pieces removed from drawing and collision; both entry triggers gated; Alice's exact saved approach pose, zero velocity; underworld fog |
| `Collapse_Second_Bridge1` | Twelve pieces removed; Alice at `fakeplayer_bridge_pos2` on solid fulcrum 86; difficulty-filtered `t131` Ant activation |
| `Garden2_Cat_End` | Last Cat removed, both triggers gated; Alice's saved pose retained |

The bridge meshes and colliders use the same translated/rotated pose. Authored
piece delays, 2,400-unit drops and 55-degree rotations run on saved scene clocks.
The central fulcrum dips under the rider, up to 12 degrees at 2 degrees/second,
and returns toward level when unloaded. Its saved pose supplies both collision
and rendering; checked rider movement cannot push Alice into a wall. Entrance
rocks and the four difficulty pieces are restored; the latter are omitted on
Hard/Nightmare. Scene-local 0.5
and 0.8 rates never alter the global physics step or enemy timers.

Older controller-less visits continue without replaying arrival or granting
resources. Trigger history is inspected before pending rules are rearmed. Old
bridge events are retired, and the current player's route position can retire
bridges behind her. Restored solids use the existing invalid-position recovery;
ordinary valid saves retain their position. New scenes save their home, cast,
camera/phase and dialogue clocks and derive world geometry from the same state.

All seven spawned Ladybugs start their own patrol, including delayed spawns and
active older saves. Duplicate spawn names remain separate entity identities.
The underworld Corporal's missing ambush now waits one second, runs toward its
authored position with collision checks, then resumes combat with its specified
sight range. Both the delay and run survive saving. Ladybugs leave normal essence
loot. Placed Antlions, Bloodroses and Mushrooms use the production resident cast.

Two movement gaps blocked the actual route: an ascending jump from shallow slime
was being cancelled by swimming, and both hands missed diagonal ledges because
the square player body reaches farther toward them. The corrected checks retain
liquid damage, body clearance, two-hand support and saved mid-pull continuation.

`--garden2-route-check` runs fresh watched and skipped routes on Normal with the
native enemy cast, real damage, ordinary inputs and authored pickups. It crosses
both collapsing bridges, the southeast slime bank and thermal chain, the Mallet
detour, fulcrum and launch ledge, the northern tunnel, final Cat, two cavern vines,
Cards ledges, diagonal exit steps, surface vine and final rabbit hole. It requires
the bridge pieces to be gone and the real exit to load `garden3$garden3_start1`
with identical carried resources. The checked input goals are in
`src/levels/garden2/route_steps.json`; no private calibration file is required.

Thirteen marked moments per route compare the running and disk-restored future,
including the fulcrum, updrafts and three vine grips. The route then continues
from those disk saves. `--garden2-check` additionally exercises every scene's
watched/skipped handoff, legacy migration, all Ladybug patrols, the Corporal run,
and fulcrum rider/restore behavior. `--garden2-render-check` captures six authored
shots. Six native save fixtures cover arrival, both falling bridges and the final
Cat, with separate writer and reader processes. `tools/launchers/Launch-Herbaceous-Border.cmd`
uses separate playtest saves; the usual root launcher uses the selected build.

Exact original quake noise, actor approach avoidance and translucent facial
deformation remain fidelity limits. The route proof covers the described Normal
path, not every optional branch or difficulty. Native checks run muted. No
original script or subtitle prose is stored.
