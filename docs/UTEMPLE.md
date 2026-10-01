# Underwater Temple: guide and exit

`utemple` has one registered owner. The normal entrance grants the shell and restores resources once. The waiting interval protects breath and gates map hurts until contact with trigger 25 starts the Turtle. Older controller-less saves use F2: restart at the entrance because the new obstacles and water invalidate former positions, preserve resources/inventory, and rearm previously pending script contacts. Already handled events are not reset indiscriminately.

The Turtle follows the supplied 160-control `tp2` spline, with its swim animation, shell attachment and bubble trail. Node callbacks start fish, oyster and collapse programs once. `Panels4` remains idempotent when reached through either its brush or guide callback. The guide's `tp158` callback opens the end wall; invoking the exit thread alone cannot open it. There is no authored conversation or E binding in this map.

Breath contacts use Alice's actual `Player::breath` and body, including during ordinary input play. One stationary point is emitted each second and remains for four seconds. Launcher model identity determines whether it supplies air; hiding a launcher does not stop its server events. Bound launchers follow brush poses; previously emitted points stay in place. Contact bounds use the supplied model geometry at scale 5 (approximately 38 units each way). Native initialization/link ordering for the final bounds remains unresolved, so that shape is an explicit approximation.

All 38 script objects are adopted: 37 drawing/solid objects, including three surface-only beams, and static liquid brush 32. The liquid contributes to the same `World` queries that drive swimming, drowning and the underwater view; it is not rendered as opaque geometry. Rendered movers and collision share transforms, including the bound column. Oyster blocks/hurt windows, the fish-head attack and vent retain their hazards. Moving contact pushes Alice toward a clear nearby position or pauses obstructed motion; crushing damage is retained. This bounded push-out approximates native physics.

Contact with trigger 30 after the wall gate starts `Utemple_Exit_Cinematic`. `SceneRunner` saves the seven-second scene clock, the `utemple_path1` camera and the eight-control `alice_path` performance using `swim_forward_frog`. Watch and skip commit the same `garden1$garden1_start1` exit. Control remains held until departure. Failed loading permits a delayed retry, and restoring a still-current committed exit permits delivery again. No dialogue, reward or additional controller is attached to this scene.

Save envelope 12, existing event rule keys, visit/route numbers and reserved hit-ID ranges remain unchanged. The temple's new controller state is revision 1. Only its visit gains new event gates and a fresh controller snapshot.

Verification entry points:

- `--utemple-check`: real liquid/breath contacts, hidden and visual-only emitters, expiration, arrival resources, pending-trigger migration and twelve watched/skipped/restored scene combinations at 30/60/144 Hz.
- `--utemple-route-check` and `--utemple-skip-route-check`: ordinary swimming controls from the normal entrance to the real exit, with active collision/hazards, underwater checkpoint restoration and strict carried-resource arrival in Garden. The test's input planner avoids impending movers and push vents; it never teleports, refills resources or disables hazards.
- `--utemple-render-check`: staged guide, trap, liquid and exit captures.
- `LOOKING_GLASS_SAVE_CASE=utemple-` with separate `--save-check-write` and `--save-check-read` processes: waiting, guide, collapse, oyster, brush water, active ending and committed ending.

Guide callback phase, accelerated column rotation and breakwall-piece motion are independent interpretations of the supplied data. Exact native mover physics, fuller quake/rumble presentation, chest animation and the four non-gating Snarks' combat remain further work. This guide/exit route does not certify those unfinished systems or the entire campaign chain. Native visual tests do not constitute an audible or running-original comparison.
