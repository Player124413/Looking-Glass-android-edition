# Village introduction and cinematic refinement

Start a fresh village visit through **tools/launchers/Launch-Village.cmd**, New Game, or the first chapter in Tab to see the falling introduction. Existing saves retain progress; saves from before this implementation do not replay the introduction or an already collected Blade scene. Hold the displayed skip control (Enter on the default keyboard layout, A on the controller) to finish the current scene safely. E advances a dialogue line.

## Village

- The opening now stages Alice falling down the chute, shrinking in the first shot, landing and recovering, followed by the Rabbit's recorded line and run between the original path markers. Cheshire appears, smiles, walks toward Alice and delivers the original opening dialogue before control returns.
- The actual hallway Blade pickup starts the restored knife conversation with Cheshire, Alice's weapon inspection, and the Rabbit walking to the little door, shrinking and running through it. The door opens and closes with the sequence. Giving a weapon through the console does not substitute for touching this pickup.
- All four principal gnome conversations use their original camera assets and character variants. Alice and the gnomes use the reviewed gesture/idle/talk sequences and original clip durations; speaking faces use the shared lip tracks. The first gnome retreats to his original ending marker. Actor footing is checked against the map, and the ordinary actors are hidden while their cinematic counterparts draw.
- The six scene states persist their phase, animation/dialogue clocks, camera shot clock, return position and completion history. Pause freezes them. Watching and skipping commit the same progression, including the Rabbit's removal and the third conversation's guard activation. Alice continues at the final scene marker: the end of her introduction walk, each final Gnome conversation position, or the Blade pickup marker while the camera cuts away to the Rabbit. The old return-position field stays readable for save compatibility but no longer pulls Alice back.
- Falling, landing, Cheshire appearance/disappearance and Rabbit running cues follow the saved scene clocks. The falling cry is level-wide, as declared by the original scene, so it remains audible while the camera is high above the landing.

## Pandemonium

The existing warning, minecart, return and departure scenes retain their progression gates. This pass corrects the cart lift's standing-to-seated timing and short vertical bumps, trackside gnome end headings and held alert poses, and the shared flight clock for the ship and attached cast. The two track collapses now have their separate original trigger delays, individual rising/falling/tilting pieces, rotation axes and three sound cues each.

Gnome disappearance now uses the original `supra_p` particle sprite with a finite 200-particle burst, upward acceleration and fade-in/fade-out. It reconstructs directly from the saved phase rather than accumulating particles after load. White conversation fades and grey cart cuts replace the common black fade. Single-node original camera files also load as stationary shots.

## Verification and limits

`--village-cinematic-check` exercises six scenes with full dialogue duration and three skip points each, comparing completion history and clear landing positions. It also checks pause, serialized continuation, invalid clocks and old-save migration. `--village-cinematic-render-check` captures 13 staged views through the actual renderer and verifies that Alice contributes visible pixels to the falling shot. Staged captures are separate from normal route proof.

The normal village input route passes through the introduction, real Blade pickup, gnome conversations and combat to the authored Pandemonium exit. Pandemonium's watched and skipped input routes, cinematic checks, transport gates and 30/60/144 Hz tunnel-camera checks also pass. The save writer/reader adds falling, knife, shrinking and gnome conversation cases to the existing campaign fixtures. Current measurements and native test results are recorded in [VALIDATION.md](VALIDATION.md).

These are reviewed Rust reconstructions using local asset identifiers, not an original script interpreter. Camera interpolation and actor travel remain reconstructed; conversation hand-offs retain the reviewed final pose and check floor support. Cheshire's transparency is an ordered-pixel dissolve. Exact head tracking, the original disappearance dynamic light, auxiliary cutaways such as the village essence demonstration, and complete cinematic coverage of other maps remain future fidelity work. Machinery keeps its existing level clock during the village introduction. This pass validates audio references and timing, not a new audible-device playback session.

Original scripts, models, recordings, captures and saves remain private and are excluded from source packaging.

### Third Gnome gesture timing

Alice's third Gnome conversation restores the reviewed standing holds and the six Gnome-shot pauses. Its gestures follow the saved dialogue line clock, with line offsets measured from the local recordings, so advancing a preceding line cannot move the crouch into another speaker's turn. The crouch begins 4.983 seconds into Alice's first size-description line; her later size gesture begins 3.045 seconds into her next reply. The Cat cutaway uses its separate reply gesture. Existing saves remain readable without a format change.

`--village-cinematic-check` checks both cues at 30/60/144 Hz, ordinary playback and advancing the introduction, pause, exact pose restoration and one completion. `--village-gesture-render-check` captures Alice before, during and after the two gestures through the game renderer.

### Alice visibility after the introduction

The Cat's dissolve is now flushed before the shared actor material is restored. Macroquad replays queued uniform values during a flush; resetting the value earlier allowed the final fully invisible Cat frame to hide Alice and other actors afterward. Both the opening and Blade-conversation fades use the corrected scope. The render check verifies partial/full dissolve coverage and an opaque actor on the following frame. Native playback of the complete introduction, subsequent walking and F5/F9 restoration all retain visible Alice; existing save data needs no migration.

The shared material now explicitly initializes dissolve, ghost and power appearance for every actor draw, with raw uniform access kept private. The later third Gnome conversation also had a separate camera/placement error: it used the Rabbit cutaway track and did not move Alice for the Cat shot. It now uses the original Gnome track, its initial camera hold, and the matching Cat-shot Alice marker. The [automated visibility suite](VISIBILITY_TESTS.md) replays all six scenes, checks actual Alice pixels, skip/reload handoffs and representative world-camera views; its camera assertion detected the erroneous shot before the correction.
