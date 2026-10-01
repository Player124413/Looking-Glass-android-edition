# Swimming, low ledges and the depth fix — updated for version 0.27

Launch `tools/launchers/Launch-Swimming-Preview.cmd` to start `garden1` underwater. Use the mouse to look, WASD to swim toward the view, Space to rise, Left Ctrl to dive and Shift to swim faster. Holding Space at the surface keeps Alice afloat. Move toward a low bank and hold Space to climb out; shallow slopes can be walked out. On land, Space and movement toward a suitable low ledge attempts a climb before jumping. E remains interact. H shows the relevant swimming controls briefly.

## Liquid and movement implementation

The supplied BSP material contents distinguish water (`0x20`), slime (`0x10`) and lava (`0x08`). Corroborating material names include normal water, green slime and lava. The runtime retains the full convex planes of world-model liquid brushes, including sloped faces; bounding boxes alone are insufficient. Overlapping contents combine with lava/slime taking priority. These volumes do not become solid collision.

Feet, body and eye samples distinguish wading, swimming and submersion. At 120 fixed ticks/second, swimming accelerates toward a bounded three-dimensional input vector and decelerates to rest with neutral buoyancy. Looking vertically affects swimming but not walking speed. Walk/run swimming targets are 170/220 units per second, acceleration is 700, and surface flotation keeps the feet roughly 44 units below a flat surface. These are newly authored, provisional values, not recovered original physics.

Deep water arrests a fast fall and suppresses dry-land landing events. Shallow wading slows movement and reduces the provisional landing impact. Slime/lava apply 8/25 Sanity per second per occupied sample, integrated on fixed simulation ticks. Version 0.27 adds the original five-second air budget (twenty seconds after the temple upgrade), followed by 12 Sanity damage every half second of continued full submersion. Surfacing refills air; a compact meter appears while submerged. The drowning sound and original death pose are connected. See [TRAVERSAL.md](TRAVERSAL.md). Pause, menus and focus loss stop movement and liquid damage. Home resets the player at the current entry, correctly detecting an underwater spawn.

A low-ledge climb checks the full upright body along an upward segment, a 42-unit forward segment and a landing trace. Supported heights are approximately 18–64 units. A 0.65-second traversal rechecks collision each tick and aborts on a new obstruction. Tall walls, low ceilings, unsupported landings and deep-water destinations are rejected. This also provides a limited land-climbing foundation; E plus forward movement also requests this climb on land. Version 0.27 adds campaign rope climbing separately; arbitrary mantling and general moving supports remain unfinished. No generic ladder entities were found in the supplied campaign maps.

## Presentation and evidence

The Mock Turtle upgrade now displays the supplied `mock_shell_alice` model on
Alice's animated `tag_back` during ordinary swimming, using its second
`turtleshell` skin. This is the original view-dependent translucent material;
the separate story fade remains unchanged. The shell is hidden on land, in lava,
and before the reward. First-person view continues to hide Alice's body.

At 75% of the air budget (3.75 seconds normally, 15 seconds with the upgrade),
the supplied mouth-bubble warning and choke cue start. Bubble timing follows the
model's animation events. Surfacing or touching a temple air source clears the
warning and refills air. The upgrade still supplies 20 seconds, not infinite
breathing. The shell and warning derive from persisted player state, so paused
loads do not need another movement tick to display them; no save migration is
required.

`--swim-render-check` checks the normal Character renderer with and without the
reward, on land/in lava, while paused, after restoration and after refilling.
It also captures an actual Garden underwater view. `--swim-check`,
`--utemple-check` and `--garden1-check` cover movement, breath budgets, air
sources and the story reward.

The supplied `models/alice.tik` and leg state definitions associate treading with `swim_tred_wep` and forward swimming with `swim_frog`. Those two original clips are now connected to the movement states, along with the original 32/48/64 ledge-climb clips. Clip selection and blending are implemented independently; the original state-machine language is not executed. Version 0.9 aligns the frog stroke with a blended 24-unit presentation offset because it is authored lower than the upright tread pose. The body and held-weapon anchors use the same offset; collision stays continuous. Head-above-water contact with a shallow floor selects wading, not swimming. Model/collision shape, climb timing and root-motion alignment remain approximate.

The supplied splash, leave-water and two swim WAVs are connected to water entry/exit and accepted swimming motion. Strokes use a provisional timed cadence, not exact animation events. Dry footstep playback is suppressed while wet. Audio files pass decode validation; Anode tests used `--no-audio`, so they do not establish audible playback of the new events.

Underwater fog/tint is determined by the actual camera position, including third person, rather than Alice's feet. It affects the existing world/character/effect shaders and clears when the camera leaves the liquid. Its colour and density are provisional. Version 0.27 adds authored currents. Breath-warning and authored environmental bubbles are supported. Refraction, caustics, exact water-surface blending and segment-limited underwater visibility remain unfinished.

## Effects showing through walls

The reported wall-visibility bug was real. The pinned Miniquad 0.4.8 OpenGL backend uses `PipelineParams.depth_write` to enable/disable `GL_DEPTH_TEST`; it does not independently set the depth write mask. Setting `depth_write: false` for transparent materials therefore disabled wall occlusion.

World, steam and weapon-effect pipelines now use the shared depth-enabled configuration. The transparent pass explicitly flushes queued opaque draws, disables only `glDepthMask`, renders and flushes the transparent draws, then restores writes. This preserves wall occlusion while allowing transparent layers to blend. The workaround targets this pinned OpenGL backend; another backend needs its own equivalent. No global Cargo cache or third-party source was edited.

`--render-check` opens a native OpenGL window and reads back rendered pixels. It verifies an opaque wall hides a fogged effect under each of the six supported translucent blend modes. A second case verifies a nearer transparent layer does not write depth and incorrectly reject a farther transparent layer. This catches the failure the earlier headless tests missed; it does not prove every material's fidelity or transparency sort order.

## Verification boundaries

`--swim-check` builds **371 liquid volumes across 26 of the 36 supplied maps**. With forces disabled only in this isolated swimming check, the real garden start is tested at 30/60/144 render Hz through idle float, dive, surface, horizontal swim and stop, requiring finite coordinates and no solid overlap. A separate `--traversal-check` exercises its active authored upward current with ordinary forces enabled. Eight bank approaches remain outside solids; the west approach performs a checked climb onto dry ground, and the southwest approach now climbs onto its sloping bank from the deeper stance. An initial surface-height clamp could push Alice into a sloped bank; re-sweeping the adjusted endpoint fixed that regression.

Synthetic tests also cover sloped liquid membership, overlapping hazard priority, bottom collision, fast water entry, tall/ceiling-blocked exits, pause/damage timing and swimming animation selection. These checks demonstrate a usable swimming foundation, not a completed garden map. The school book puzzle/lifts, NPCs, combat, cinematics and scripted progression remain separate work.

All archive observations, logs, screenshots and recordings stay local under `private/`. Newly authored code and synthetic checks are included in the source-review package; original data and extracted state definitions are excluded.
