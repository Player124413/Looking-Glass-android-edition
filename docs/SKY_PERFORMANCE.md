# Skies, fog and rendering cost

The renderer now reads checked FAKK node, leaf, face-membership and PVS rows. World face spans are rejected by the current view's PVS and frustum before vertex updates/uploads; the portal pass uses the sky viewpoint's own cluster. Missing visibility, solid or out-of-map viewpoints fail open. Moving inline objects use transformed bounds, and material-deformed surfaces and scripted sky objects keep conservative drawing. This does not infer closed-door area portals or add a new occlusion system.

Original TAN collapse maps supply successive detail levels for decorations. A level is eligible only when its maximum vertex displacement, measured across every animation frame, projects below 0.35 pixels. Nearby models retain full detail. The renderer compacts retained indices before uploading. Invalid collapse order disables reduction. Skeletal actors are not simplified by this change. The conservative threshold can leave ordinary views at full detail even while visibility culling saves substantial work.

Reviewed original sequences now run through the normal interaction and saved-state paths:

- `hedge1` and `hedge3` begin at their script-selected camera. The original trigger threads select three cameras in hedge1 and all seven in hedge3.
- `hatter1` and `hatter2` rotate their sky camera, move eight miniature props on the linked original path nodes with staggered starts, apply the source rotation speeds/scales and retain animated clock hands. Reviewed node callbacks shrink and regrow the props.
- `garden2` fades between the lower and upper fog colors/distances over one second. `garden4` fades toward the source warm fog over four seconds. Interruption starts at the currently displayed fog value. Repeated requests for the same destination do not restart a fade.

Sky selection and fog progress persist with interactions. Sky motion uses the existing saved environmental clock. Pausing freezes both. Old saves without the new presentation field use the source initial state.

The Fortress vortex now covers the airship's separate staging rooms as well as the playable exterior. Small black windows and interior surfaces remain opaque. Portal visibility is always isolated to the sky viewpoint, including the profiling baseline. Fortress also retains its required world PVS in both variants: extending the sky through its former black shell would otherwise expose disconnected rooms. The Fortress regression uses the normal shared rendering path, checks the arrival and playable-entry sky colors, and compares eleven cameras with optional frustum/detail reduction enabled/disabled.

The original executable is not run. Path interpolation uses an interpolating cubic spline through the supplied nodes, with reciprocal node-speed segment timing; the original spline tension and callback timing remain approximations. Shrink/grow is continuous instead of the original 0.1-second script increments. Distance-fog density, cloud projection and the previously documented portal-lighting approximation remain. Other conditional sky/fog scripts need reviewed owners; this is not a general script interpreter.

Weapon altars now display the original world weapon models instead of 20-unit inventory billboards. The source additive plume is intentionally bright, but its 200+100-particles/second contribution overwhelmed the small pickup in this LDR renderer. A documented 0.12 opacity gain applies only to weapon-altar emitters; source colors, rates, motion and other emitter families remain intact. Collecting an item removes its existing altar particles and disables further emission and its light. Cosmetic puffs are still reconstructed, not serialized.

## Reproducing the checks

Build a release executable, then run `tools/test_sky_performance.ps1 -Executable <absolute executable path>` on the isolated test desktop. The script records the exact executable hash and runs the new paired performance/visual checks, shared-effect regressions, the entire map corpus, full resource-owner level switching and actor/cutscene visibility checks. It stops on the first failure or a changed executable. It does not use player saves.

`--sky-performance-check` compares three fixed views in each of School, Pool of Tears, Hedge and Hatter at 1200×680. It requests an uncapped swap interval only for this diagnostic, warms each variant and reports median rendering plus synchronized readback time over seven samples. This includes a fixed readback cost and is not GPU-only timing or gameplay FPS. Baseline and optimized images, counters and `profile.json` remain in `private/sky-performance/`. The check also compares a source prop at near/far distances, captures six source presentation sequences, verifies paused/restored pixels (at most 32 pixels may differ by one 8-bit step for driver rounding), and checks pickup contrast with and without its plume. Serialized state is compared exactly in unit checks.

See [VALIDATION.md](VALIDATION.md) for the executed results and their limits. Captures contain original artwork and are excluded from source packages.
