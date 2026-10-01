# Fortress of Doors

Launch **tools/launchers/Launch-Fortress.cmd** and start a fresh visit, or choose **Fortress of Doors** in the Tab chapter menu. The separate **Fortress of Doors - return** entry starts the upper route from Beyond the Wall.

The first visit now plays the airship arrival: cave flight and conversation, the lookout, lowering drawbridge, guard pursuit, tower flyby and landing. Hold **Enter** (controller **A**) to skip. After landing, follow the lower platforms to the portal, cross the splitting room, climb its platforms and enter the raised door to Beyond the Wall (`fortress2`). The return visit follows the upper walkways to the school window (`skool1`), including the brief Boojum reveal. Run and jump from the raised edge of the walkway to reach the window.

Implemented:

- Playable arrival, including the explicit `fortress1_start1` entrance used by the campaign.
- Visible schoolhouse, clock machinery, doors and both halves of the splitting room.
- Shared drawing/collision transforms for the room and its moving platforms, including carrying Alice and stopping motion when it would embed her in solid geometry.
- First/return portal visibility and trigger gates, return window shutters, supported Card Guards and Boojums, and the two available Cat recordings/subtitles.
- The return window's linked camera view into the original furnished room, including its walking pupil. The aperture follows the schoolhouse tilt, responds to the player's viewing angle, and remains behind shutters and foreground geometry.
- Saved room progress, visit state and machinery time. Earlier saves acquire the controller; a saved player still in the offstage arrival room moves to the playable entrance.
- The sky portal's scripted offset and orientation. The remaster now extends the existing animated vortex to the outer walkways as well as the splitting room. This is a presentation change: the original exterior used black. Small black windows, props, offstage rooms and all collision remain separate from the outer sky.
- The help objective explains why the Skool window is closed on the first visit, and directs the return visit to the raised takeoff edge.

Arrival uses the original camera/path declarations, ship seat tags, Alice/Gnome clips, four voiced lines with subtitles, moving stalactites and drawbridge sound cues. Return Boojums follow their authored reveal paths and then resume the existing encounter. Pause freezes these scenes; F5/F9 retain their scene clocks and dialogue. Watching and skipping commit the same landing/ambush state. Older saves keep their playable state without replaying an already bypassed arrival.

Arrival camera and airship paths use cubic curves with blended node speeds and the native parameter lead-in. The cave and tower retain their authored viewing directions; the cave adds a smooth clearance adjustment for foreground pillars. The chase uses the placed camera's orbit bearing/distance and checks world clearance. Passengers use the ship's looping seat poses. All of this is derived from the saved scene clock, without changing the save format or arrival outcome.

Fidelity work remains: guard movement is reconstructed, and guard projectile effects/head tracking are not complete. Full prop attachments, weight-responsive fulcrum tilting and background flying paths remain incomplete. The window uses the original room, camera link, pupil model, walk clip and linked path; exact original projection and pupil turn timing remain unmeasured. The return reveal retains its previous path interpolation. Supported enemies use the project's current combat behaviour. Beyond the Wall has a verified main route; see [BEYOND.md](BEYOND.md).

## Verification

`--fortress-cinematic-check` checks watched/skipped arrival, return gating, one-shot Boojum reveal, pause, mid-scene restoration, old-save migration and matching ambush positions. It samples the arrival camera at 120 Hz, checking finite poses, static/moving-brush clearance and continuity outside explicit cuts. `--fortress-cinematic-render-check` captures sixteen staged shots and compares images with individual cast members omitted to verify actual visibility in the conversation, pursuit and guard shots. Passengers can be offscreen or occluded during establishing shots; those samples are diagnostic, not visibility assertions. Both normal traversal routes still pass after adding the scenes (first: 10 jumps/9 throws/32 damage; return: 2 jumps/15 throws/0 damage). These results supersede the older counts below.

`--fortress-route-check` uses the same movement, collision, weapons and trigger logic as the game. It traverses both visits continuously, permits the first visit's authored portal, and checks the destination and room/return state. It also checks pause, visit gates, saved partial movement and migration from the previous generic controller. The migration/state fixtures are separate from the continuous routes. Re-baselined 2026-09-29 for the Blade/Cards rules: the first visit takes 15,300 ticks (6 jumps, 1 throw, 26 cards, no combat damage, 100 Sanity) and the return 6,990 ticks (2 jumps, 5 throws, 42 cards, 2 combat damage, 100 Sanity).

`--fortress-render-check` produces staged private captures of the school window, sky room, exterior and splitting room. Window checks cover three viewing angles and schoolhouse poses, pupil visibility, identical paused/rebuilt views, containment within the aperture, and first-visit/closed-shutter occlusion. The remote view has its own visibility set and clips geometry in front of the linked camera plane. The pupil is presentation-only and uses the saved environmental clock, so the gameplay actor set and save format stay unchanged. These are visual fixtures, not proof of traversal. Original game data and captures remain local and ignored by Git.

Verified against the local game archives: both continuous routes pass (first visit: 10 jumps, 9 blade throws; return: 2 jumps, 11 throws), along with the state/migration checks, 181 unit tests and strict Clippy checks. Native Anode captures confirm the playable arrival, reconstructed room, and original sky artwork.

The merged project also passes both routes, 188 current unit tests and strict Clippy. A native return-visit F5/F9 round trip restores the level, actors and machinery successfully (isolated saves under `private/fortress-native-saves`).

The current return check also verifies both shutter-event Boojums become active. It passes with two jumps and 17 Blade throws after correcting their activation.

The sky follow-up rechecks that same return jump with normal movement (two jumps, 17 Blade throws, no combat damage) and adds native exterior, schoolhouse and upward-facing room views. The exterior's black fog fades its towers into a captured copy of the actual animated sky. Small opaque black surfaces stay in separate batches so windows do not accidentally become sky apertures.

The 2026-09-30 school-window build passes 510 unit tests, strict Clippy, both Fortress routes and the native window/sky fixtures. Three window views show the original room and moving pupil; closed shutters remain opaque, and paused/rebuilt views match exactly. An earlier format-11 Fortress return save also loads successfully.
