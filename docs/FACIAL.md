# Facial animation

The original game supplies facial animation. The local archives contain 366 `.lip` files alongside dialogue recordings, skeletal `tag_mouth` joints, per-model `maxmouthangle` settings, and Alice's `blinkinfo` declaration and alternate closed-eye texture. The earlier renderer played body clips but omitted the mouth controller and blink texture selection.

## Restored behaviour

- Alice and supported speaking characters now open their mouths using the original recording's lip envelope. This includes placed story actors, summoned Cheshire hints, the school-two Gnome, Pandemonium's cinematic cast, and the Duchess/Bill/Mock Turtle scenes.
- Mouth movement composes with the existing head and body animation. Each actor uses its own dialogue assignment; listeners retain their authored pose. Missing lip files leave the mouth controller neutral.
- Alice blinks using her original face texture during gameplay and supported cinematic puppet animations. Ordinary Alice's blink clock freezes with gameplay and is retained in new saves; older saves start it at zero.
- Lip sampling uses the existing saved dialogue clock, including pause, line advance, cinematic skip and resumed recordings. Muting or running without an audio device does not disable facial presentation. The controller returns to neutral when the recording ends.

Relaunch **Launch.cmd** or **tools/launchers/Launch-Village.cmd**. Faces are easiest to see during a nearby conversation or a summoned Cheshire hint (**C**). Existing saves remain compatible.

## Implementation and limits

`src/facial.rs` checks the plain-text LIP duration, sample count and byte-valued envelope, then interpolates its observed 20 Hz samples. Trailing samples are not stretched across the recording. It rotates the mouth joint around its observed local transverse axis before skeletal hierarchy accumulation and skinning. Authored mouth-angle overrides are respected; the default is 10 degrees, verified against both Actor and Sentient constructors in the supplied gameplay DLL. Alice's four-second blink interval and 140 ms closed phase are an approximation; cinematic blink phase currently follows the supplied acting clock.

This restores the missing mouth and blink presentation for dialogue already implemented in this project. It does not implement unfinished campaign scenes or add expression/eye-tracking choreography absent from the source. There has been no frame-for-frame comparison against the original executable. Original assets remain read-only and are excluded from source packaging.

## Verification

- `cargo test --locked`: 205 tests pass, including LIP validation/interpolation, local jaw layering, metadata scope, and dialogue speaker/pause/skip/save restoration.
- Strict Clippy and the release build pass.
- `--facial-check`: all 366 original envelopes / 42,378 samples pass; eight principal character rigs deform finite vertices; all 79 currently implemented story lines have active lip envelopes and close after speech.
- `--cheshire-check`: 38 authored hint regions and 40 unique recordings pass.
- `--facial-render`: native GPU captures of open/closed mouths for eight rigs and Alice's blink. Close-ups of all eight rigs were visually inspected. A normal village launch with a dialogue preview also rendered and exited cleanly. Captures and logs remain under `private/facial-*`.

The separate broad `--story-check` currently stops at `fortress2/cat_dialog_easy` in its trigger assertions, after decoding the preceding story assets. Facial code does not change those trigger rules; the focused facial check validates both Fortress dialogue envelopes independently. No fresh audible-output verification is claimed for the hidden-desktop render checks.

## Presentation fidelity update

The supplied gameplay DLL initializes mouth range to 10 degrees (constant `0x1015c1d8` in the Actor/Sentient constructors). Alice, Bill and Mock Turtle now use that default; explicit TIKI overrides still win. The previous reconstructed default was 20 degrees. Original LIP envelopes, speaker assignment and their saved/pause-aware clock are retained. The speech controller leaves all non-mouth skeletal channels intact.

Reviewed linear dialogue functions now retain explicit `headwatch` actor/target declarations per line. Placed speaking actors can follow those targets when they resolve to the player or another loaded actor. The bounded additive controller uses saved angles and pauses with the dialogue. Unresolved targets and conditional branches do not invent attention directions. Camera-owned cinematic choreography and independent authored eye/eyelid tracks remain in their original clips; this is not a complete game-script interpreter. Headwatch smoothing and the 45/30-degree safety limits are reconstructed, not a claim of exact engine parity.

Alice's original blink texture belongs only to her ordinary skin. Rage/Tea faces retain their own source texture throughout a blink because no matching closed-eye power textures are supplied. This avoids flashes back to ordinary Alice. The ordinary four-second/140-ms blink phase remains the earlier timing approximation.
