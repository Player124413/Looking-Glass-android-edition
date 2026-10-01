# Pandemonium cinematics — version 0.25

**Current additions:** [Fortress of Doors](FORTRESS.md) now plays its airship arrival and return Boojum reveal. New Game plays the original opening film before the Village scene. See the [campaign cutscene audit](CUTSCENE-AUDIT.md) for what is implemented and what remains missing.

**September 2026 refinement:** the village now has its falling introduction, Blade/Rabbit sequence and four staged gnome conversations. Pandemonium's cart acting, collapse choreography, particle burst, fades and sound timing have been refined. See [current changes, verification and limits](VILLAGE_CINEMATICS.md). The versioned results below describe the earlier Pandemonium milestone.

Current facial support: original lip-sync envelopes and Alice’s blink texture are now connected for implemented dialogue. See [facial animation, verification and remaining limits](FACIAL.md). Earlier facial limitations below describe previous milestones.

Launch **tools/launchers/Launch-Pandemonium.cmd** and choose **N** for a fresh visit, or enter normally from the village. The four main cinematic sequences now run during the playable route. Cheshire's rope and Cards hints remain ordinary conversations.

| Scene | Restored presentation and outcome |
| --- | --- |
| Elder Gnome's warning | Original conversation camera track, Gnome close shot and key-location reveal; Alice's reactions and the Gnome's pipe/talk/smoke animations; disappearance. Control returns at the authored conversation marker and the Gnome stays gone. |
| Minecart | Boarding jump, lift and shrug, following camera, bumps at authored rail cues, trackside Gnome movement, collapsing rails, final camera, cart plunge and Alice's landing. The spent cart, blocked mine entrance and arrival guard use the same completion path when skipped. |
| Return to the house | Original camera flight along the street, opened house entrance, Gnome's beckon and shrinking disappearance with depth-tested sparks. Alice resumes at her contact position; return guards and the house exit retain their progression state. |
| Airship departure | Alice walks to the deck marker and changes stance during the first exchange. The second exchange attaches Alice and the Gnome to the moving ship, changes camera and completes the Fortress 1 transition. The ship, its Gnome and Alice now yaw along the flight path (heading blended across each path node; yaw only, no pitch or bank). |

The camera tracks, skeletal models, acting clips, voices and subtitles are read from the local archives. Animation clip durations come from those assets. Camera shot changes follow dialogue position, including E advancement, and the cinematic clocks stop with pause, menus and loss of focus. Combat, shooting, hints and recovery are suspended while a scene owns Alice. Normal controls and the selected weapon return afterward.

## Skipping and saving

**Hold Enter for about two-thirds of a second to skip the current scene.** The top-right prompt shows the hold progress. Release the key before another skip: carrying a held key into a scene or through pause cannot skip it accidentally. **E** still advances one dialogue line; **P** pauses.

Skipping commits the scene's required actor, gate, cart and exit changes directly. It does not simulate a large jump in time, move enemies through combat, spend items or grant unearned quest progress. Only that scene's dialogue is completed; unrelated queued conversations remain queued. Dialogue callbacks, guard activations and map exits retain their one-shot history. The airship exit still requires normal key/return progression before its scene can start.

**F5/F9** work during and after scenes. Save format **7** stores the scene phase, acting clock, shot clock, dialogue position, return position and completed disappearances. It reads formats 1–6. Old active cart/flight saves continue their existing transport; old completed events do not replay newly added staging. Cached visits upgrade before resaving. Loaded games pause, as before.

## Verification

- `--cinematic-check`: all four actual trigger contacts, full-duration dialogue playback, camera validity, pause, malformed-state rejection and skips from saved intermediate points, including before the first line. Watched and skipped endpoints match required world poses/visibility/collision, actor removal, guard activation, dialogue completion and exit count after independent gates settle. Non-exit scenes leave a clear, supported landing; repeated completion cannot emit another exit.
- `--pandemonium-route-check`: normal entrance-to-exit route with ordinary movement, rope use, jumps, E interactions, combat and dialogue advancement. **21,730 fixed ticks, six jumps, 11 Blade throws, one authored return portal, 57 Sanity** (re-measured 2026-09-29; the driver is unchanged).
- `--pandemonium-skip-route-check`: the same input route, skipping each cinematic through the live skip handler. **21,115 ticks**, with the same jumps, throws and portal, finishing with 55 Sanity. Both routes collect Cards and the key and reach the correct Fortress 1 entrance. These are input simulations, not complete human playthrough claims.
- `--pandemonium-check`: existing key/exit gates, ropes, obstruction-safe doors, transport landing at 30/60/144 Hz, save continuation and one-shot exit still pass.
- Native save writer/reader: **28 cases in separate processes**, including warning, disappearance, return scene, cart landing, boarding, flight and saves after skipped warning/cart. An actual v0.24 airship save upgrades, resaves and reloads without changing player position or inventory. Earlier migration fixtures still pass.
- Anode inspection: staged captures of all camera cuts and transport poses; real keyboard P/Enter/F5 skipped a loaded warning, saved its completed state and restored normal first-person play. Holding Enter during loaded boarding loaded and autosaved Fortress 1, retaining 37 Sanity, selected Mallet and two Dice. Test saves are separate from ordinary player slots.
- **133 unit tests pass**, and the warning-denying code check passes. Shared school event and first-school route checks also pass.

## Fidelity and campaign limits

This is a reviewed Rust implementation of these sequences, not an interpreter for the original scripts. Camera node positions, angles and speeds are original; interpolation between nodes and the transport-following shot are reconstructed. Boarding/jump arcs, mouth/head tracking, some track choreography, fades, disappearance sparks and machinery audio loops are not exact original behavior. Native test audio had no output device, so this pass does not claim audible verification of the existing voice playback.

The former Fortress arrival blocker described by this milestone has since been removed. Both playable visits and their arrival/reveal scenes are now implemented; see [FORTRESS.md](FORTRESS.md).

Original archives, extracted scripts, captures and saves remain private. The local source-review package includes Rust source and documentation only; nothing is published.
