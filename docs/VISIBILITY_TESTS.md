# Automated character visibility checks

Run before accepting changes to actors, cutscenes, animation visibility, material effects, save restoration or render ordering:

```powershell
powershell -ExecutionPolicy Bypass -File tools/test_visibility.ps1
```

The script builds the current source in an isolated target directory and runs `--visibility-check`. It requires the configured original game data and a Windows graphics session. It creates no player saves, does not change preferences, and does not publish anything. To test an already built executable, use `-SkipBuild -Executable <path>`; `-Data <folder>` optionally selects the data. A failed check exits nonzero. Results go to `private/visibility/run.log`, with Village pose counts in `private/visibility/village-report.json`; failed Village poses produce `private/visibility/failure.png`.

## What the suite checks

- Destination-level asset loading follows the same path as the Tab chooser and console map command. Two school-to-Village switches verify the correct cast is present and produces visible pixels in the original opening camera. Scene and cast assets are prepared as one package before replacing the previous level.
- GPU readback of actual gameplay Alice and cinematic Alice after deliberately stale dissolve, Glass ghost and power tint, across frame/readback boundaries. Both immediate rendering and the deferred material queue are tested; the entire normal image must match the clean baseline. Intentional ghosting and partial/full Cat dissolution must still work.
- All six Village scenes run through their real story/choreography clocks, naturally and with two skip timings. The same actors/material remain alive across cases. Every half-second and beat change, the actual Alice pose is rendered and its pixel coverage checked. Mid-scene serialization, paused clocks, gameplay handoff and restored gameplay appearance are included.
- The headless Village check also compares final cinematic feet/facing with the first gameplay state and checks three skip timings against the same endpoint. Each natural ending renders 36 camera-return frames with collision assertions and saves first/intermediate/final captures. The shared pose check verifies that the exact cinematic local pose survives handoff and serialization. Camera unit tests cover 30/60/144 Hz, pause/reset, orbital framing, blocked/distant/skipped returns and an obstacle appearing mid-return.
- Eight representative Village shots also measure Alice against the actual map and authored camera, including the later Gnome conversations and Blade scene. The third Gnome conversation additionally checks Alice's speaking lines throughout the replay, before and after the Cat camera cut.
- Fortress and Pandemonium scene state checks cover watched/skipped outcomes, pause and intermediate saves. Native fixtures for Fortress, Pandemonium, the school return and Duchess are followed by pixel checks of gameplay Alice using the still-live shared pipeline.
- The shared material still has one GPU pipeline for 64 simultaneous owners; creating an independent shader per actor is not a valid fix.

Pose coverage uses a fitted diagnostic camera for deliberate cutaways. It detects missing geometry/effects, not every possible camera obstruction or choreography mismatch. These checks complement traversal, audio and other render tests; they are not a claim that all campaign cinematic work is complete.

## Renderer contract

`SkinMaterial` keeps its raw material private. Ordinary actor/prop bindings assign every appearance field (dissolve, ghost, power) explicitly. A cinematic puppet binds its own opaque appearance for every draw. Fades use `Puppet::draw_dissolving`; gameplay powers are scoped to Alice's draw. Do not implement a new effect by changing a raw shared uniform and assuming a later reset is sufficient.

Macroquad restores each queued draw's uniform snapshot into its pipeline during a flush. An effect reset issued before that flush can be overwritten, leaving subsequent frames invisible. Scoped nondefault effects flush before clearing, and the next actor also binds complete defaults. New visibility fields must be added to the single `Appearance` binding and this GPU regression. Deferred render queues must capture each draw's appearance rather than reading the pipeline's later state.

New cinematic controllers should add a replay/ending fixture here; a screenshot alone does not prove that Alice survives the return to gameplay.

The authored-camera test first failed on the third Village Gnome conversation with **zero Alice pixels**. That exposed the incorrect Rabbit cutaway track (`gvillage_jdm3` instead of `gvillage_tg3_p1`) and a missing move to `alice_tg4_cat2` when the Cat shot begins. The corrected scene preserves the original camera hold through the first three lines, resumes the path for the next exchange, and moves Alice with the camera cut. Existing saves derive the corrected pose/camera without migration.


## Tab/console level entry

The chooser previously rebuilt `LevelArt` against the outgoing `scene` and `interactions` before assigning the incoming level. Choosing the opening Village from school therefore started its scripted cameras/dialogue while retaining school art. The renderer correctly hid ordinary gameplay Alice during the cinematic, but there was no Village cast to replace her.

`Entered` now owns the destination `LevelArt`, prepared together with its world, logic and NPCs. Tab, console map changes and normal campaign exits install that package; a failed asset load leaves the current level intact. The repeated entry GPU check is part of `--visibility-check`. The full resource-replacement stress test also uses the production package, with no separate art-loading workaround.
