# Mystifying Madness

`tools/launchers/Launch-Mystifying-Madness.cmd` enters hedge2 at its authored entrance.

The two levers use the local game's TAN animations. Alice approaches the original use offset before the synchronized pull begins. The short approach checks the whole player body against collision. Their callbacks wait for the pull to finish. Each lever can be used once per visit. Both cutaways use the original camera tracks and their authored viewing directions.

The first lever releases the two relay-owned Clockwork Automatons behind their doors. The tunnel camera runs for eight seconds before the water gate rises 168 units over two seconds. At the end of the eleven-second cutaway, the robot doors rise 144 units and the entrance trapdoor drops 184 units over two seconds. A closing brush stops if Alice blocks it.

The second lever shows the tower gate. After one second its two leaves move 40 units in opposite directions. The two-second cutaway then returns control. The tower2 transition requires both lever sequences to be complete and both end leaves fully open.

The turtle shell is granted idempotently, preserving carried health, ammunition and other items. All eight authored air-bubble emitters refill breath when Alice overlaps their original hidden breath-model bounds; the decorative bubble cloud alone does not grant air. The three gate leaves play their authored scrape and stopping sounds. Sealed model-loading-room actors are excluded from the playable cast. Ordinary maze and water ambushes remain owned by the shared enemy system.

Lever animation, camera time, completion, robot activation and each physical door position are saved. Skipping opens the cutaway's gate and completes the same sequence; the trapdoor still closes with collision checks after control returns. Saves predating this controller restart at the entrance with carried resources preserved.

Checks:

- `--hedge2-check`: actual lever reach and use, delayed callbacks, scene outcomes at 30/60/144 Hz, closed collision gates, saved continuation and invalid-state rejection.
- `--hedge2-route-check`: production movement, swimming, lever input and native enemy simulation.
- `--hedge2-render-check`: native lever and cutaway captures under `private/hedge2-work/captures`.

Source references: local `maps/hedge2.scr`, `models/lever.tik`, the two camera tracks, and map entities 11, 32–34, 52–54, 402, 450, 507, 567, 576 and 577. These are identifiers and independently written behavior descriptions; no original script or dialogue is distributed.

The route check walks from the entrance through the maze, first dive and bubble stops, rock crossing, first lever, alternate corridor, spiral stairs, return dive, opened water gate, tall well, upper maze and final lever into `tower2$tower2_start1`. It uses production movement and combat on Normal difficulty. Native automated tests run silently; no human audio review is claimed.

Verification on 2026-10-01: the native Normal route reached tower2 with watched cutaways after an unedited checkpoint reload at the well. A fresh run from the entrance also completed with both cutaways skipped. Both arrived with 51.42 Sanity, no teleportation and no drowning damage; the destination, spawn, difficulty and carried inventory were checked. The replay collects the original recovery items and uses the owned Eye Staff in the upper maze.

The 629 unit tests passed. Six actual save phases resumed in separate game processes with the same state and subsequent twenty seconds of machinery. Native captures cover both pulls and both camera sequences. The other 38 visit snapshots and event signatures match the previously selected Dry Landing release. Native checks use programmatic production inputs with audio disabled.
