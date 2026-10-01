# Hollow Hideaway: pond route, fish and Bill ending

The October 1 shared water update restores the pond's authored animated reflection
normals and corrects waterfall texture turbulence. See [RENDERING.md](RENDERING.md).
The dedicated launcher now uses that tested combined build while retaining its
existing entry, difficulty and save directory.

The existing `potears2` owner stages `tears2_end_cinematic` after its two live antguards die and the authored two-second delay expires. Trigger 50 must then be contacted. Bill has no decorative duplicate and is hidden outside the scene. Regular guards occupy BSP identities 71/337 on Easy/Normal; Corporals occupy 5/554 on Hard/Nightmare. Combat and death rewards use the reserved `6_900_000 + entity` identities.

The controller now owns the complete pond route: five looping lily clips, two one-shot leaf clips, their supplied models, the sixth decorative lily and four falling leaves. Normal movement boards and rides these supports. The fifth lily carries its original essence. All seven paths use linked BSP nodes and inherited speeds through the shared cubic spline sampler; scripts are inspected as data, never executed. The leaf-one node starts the existing third/fourth Ladybugs; leaf two disappears two seconds after its authored removal node. The other two Ladybug pairs retain their native contact triggers and all shore enemies keep their saved identities.

Rider support uses the same player footprint as movement, including the deck edges. If carrying a rider would embed Alice in world geometry, that mover waits while Alice moves around the obstruction. Other movers and hazard clocks keep running. The two clip posts on the first ride remain solid. Visual pendulum motion does not tilt the collision deck. The supplied sixth lily has no brush and receives no invented collision.

The fish hazard counts six seconds touching the pond layer, tolerates interruptions up to half a second, and resets after a longer gap. It then locks the camera, presents the fish and splash, hides Alice after one second and applies lethal damage after two seconds. The attack cannot be skipped. Ordinary death/retry and the normal save system remain responsible for recovery. Its clocks, location and one-shot damage flag are saved; assets and emitters are reused with bounded counts.

The scene uses five supplied camera tracks, the initial Bill camera and final tracking camera; both approaches, the C3 thirteen-line sequence, timed actor gestures, Bill's departure, door oscillation/open/reset, Alice's held pose and the two-second pull. Dialogue uses recorded lengths and C3 speaker/lip bindings. A missing `talk_liftbelt` animation retains the declared `idle_liftbelt` fallback. Marker movement uses model speed and static ground support; it approximates native navigation.

`SceneRunner` owns the final saved clock and durable exit commitment. Watch and skip converge on that completion; skip first consumes the same C3 identity and runs the half-second white fade. The elevated changelevel brush cannot bypass the scene. Exit delivery is one-shot per live owner, retryable on load failure, and retried after restoring a still-current committed visit. Successful departure replaces that owner.

Controller revision 3 accepts the existing C3 revision 1 and house revision 2. Older visits rearm only the five formerly pending pond/Ladybug contacts; completed dialogue and guard deaths remain consumed. A legacy low pond position that depended on the missing supports requests a safe entrance respawn instead of loading Alice inside a newly restored mover. It validates the original closed-gate event program before migrating its usage and adopts the guard cast without duplicating it. An active C3 conversation continues its saved line but cannot commit an exit by itself. Already-read C3 dialogue remains consumed; after the actual guard gate, the owner stages Bill and proceeds to the house ending. Controller-less pending contacts still use F2. No blanket reset of consumed events is performed. Save envelope 12, existing event keys, visit numbers and hit ranges are unchanged.

Validation entry points:

- `--potears2-route-native-check`: ordinary entrance, complete production enemy cast, both leaf rides, lilies, live gate combat, Bill and a strict resource-preserving arrival in Just Desserts. `LOOKING_GLASS_HOLLOW_CARRY` accepts a completed Pool checkpoint; `LOOKING_GLASS_HOLLOW_WATCH=1` watches Bill instead of skipping. No position/stat edits, recovery assists, god mode or authored teleports are used.
- `--potears2-route-check`: the same controls with the headless house-guard cast; use the native check for full encounter evidence.
- `--potears2-transport-check`: rider safety, local clocks, pause, restore, path cues/removal at 30/60/144 Hz and a separate lethal fish fixture. Fish clock unit checks also cover 120 Hz.
- Separate `--potears2-store-write-check` / `--potears2-store-read-check` processes: actual native route moments on lily one, leaf one, leaf two and at the house. Each restores exact player/resources/controller/NPC state and compares 240 further input ticks.
- `LOOKING_GLASS_HOLLOW_CAPTURE=<checkpoint>` with the native route flag produces a diagnostic scene capture; `LOOKING_GLASS_HOLLOW_LOOK="x y z"` optionally supplies its look target.
- `--potears2-check`: staged real contact, difficulty sets, live guard damage/death, watched/skip/restore clocks and exit transaction; retained C3 fixtures when locally present.
- `--potears2-render-check`: staged cameras/cast/door samples using the real map.
- `LOOKING_GLASS_SAVE_CASE=potears2-scene-` with separate `--save-check-write` / `--save-check-read` processes: guards, approach, conversation, door motion, suction and committed exit.

The bounded ant combat is sufficient to own the kill gate; it is not a completed family implementation. Regular alternate grab/fling attacks and full native navigation remain later work. Corporal projectile arc lift, rebound damping and explosion radius are explicit port approximations. Native audio listening and comparison with a running original are not part of these checks.

The checked controls wait for lily five to approach the bank rather than assuming a fixed delay, since combat changes the phase on arrival. The route still requires jumps and steering around the first leaf's obstacles. `tools/launchers/Launch-Hollow-Hideaway.cmd` opens the tested local build with a separate save directory when available; it falls back to the standard launcher elsewhere.

Remaining fidelity approximations: spline timing and pendulum/falling-leaf curves use the port's sampler and bounded visual motion; no comparison with a running original was performed. Native audio listening is unverified. The full-route evidence is Normal difficulty; the separate gate contract covers all four difficulty-selected guard sets. Broader enemy-family fidelity, slide-surface behavior and fog density are not claimed complete by this change.


The complete native carried-resource check starts with 61.47 health and 1.97 Will from the preceding Pool run. Watched and skipped endings both finish with 100 health and 43.10 Will, the same weapons, collected items and reward identities, and exactly one transition. Both runs record zero teleports, lost ticks or damaging-liquid ticks. Uncollected world loot ages during the longer watched scene; its remaining lifetime is therefore intentionally not identical. The four ride/house Store checks passed exact restore and continued simulation in a fresh process. Fish presentation was also inspected in the hidden native renderer, separately from the successful route (which never triggered a fish attack).
