# Original animation restoration

The September 2026 audit found 1,402 skeletal animation files and 694 vertex animation files in the mounted local archives. Many belong to later bosses, cutscenes or other mechanics that are still unfinished. This pass connects additional original clips to existing gameplay and ambient scenery; it does not claim full campaign choreography.

## Restored playback

- Alice uses the unarmed idle, walk, run and water-treading poses when she has no toy. Large toys use the original large-weapon ready pose. Weapon changes crossfade between movement poses.
- Alice relaxes after 25 seconds standing idle, then performs the original weapon-specific fidgets after a further idle interval. The 48-clip performance set includes the relaxed transition, idle, numbered toy routines, seven conversation gestures and a damage reaction. Walking, attacking, changing equipment or entering a traversal state interrupts the performance. First-person play keeps its existing weapon presentation.
- Fidget props follow the original animation-frame `attachmodel`, `removeattachedmodel` and `hideweapon` declarations: tossed cards, mallet, balls, dice, staff, and the other toy routines. Their original sound events use the same frame timeline. Attachments are reconstructed from the current frame after loading, so no commands need to be replayed.
- Alice uses conversation gestures during supported dialogue and a pain reaction on a health decrease. Moving reactions affect the upper body; leg movement remains under locomotion control.
- Placed NPCs use numbered idle and talk variants within the same posture. Torch Gnomes regain their two twitches and talk-begin/talk/talk-end sequence. Cheshire hints cycle through the seated talk variations.
- Skeletal props can now use the shared animated attachment renderer. This fixes the Mad Hatter's cane, which previously prevented his whole model from loading. It does not implement his boss encounter.
- Both school maps use the book stack's original `sway` clip. Both Hatter maps use `handsmove` on the eight authored sky-watch placements. Loaded environmental TAN models interpolate their frames instead of remaining frozen on frame zero. These use the existing pause-aware, saved environment clock.

All model, texture, audio and animation data stays in the user's read-only PK3 archives. No original executable or script interpreter is used. The small ambient binding list comes from the named map-script calls, rather than executing arbitrary script commands.

## Save compatibility and verification

The new Alice performance state and NPC talk-exit flag have defaults for older saves. New saves retain gesture selection, elapsed time, idle timer and variation counter. Existing explicit cinematic animation calls remain intact.

`--animation-check` validates Alice's 48 performance clips (3,903 frames), their prop/tag references, ten authored ambient bindings, and all 63 eligible NPC models, including 179 loaded clips across 484 placements. The NPC check skins every frame and resolves attachment textures. This is asset/presentation coverage, not an assertion that every eligible actor has working gameplay AI.

`--animation-render` produces explicitly staged private captures through the production Alice skin/prop renderer, representative NPC performances and both animated sky models. `--facial-check` covers the lip-sync regression. Unit checks cover equipment-dependent movement, interruption, paused gesture state, save restoration, posture-family selection and vertex-frame interpolation/wrapping.

## Remaining contexts

Unused archive clips still include later boss combat, unimplemented cinematics, additional dismemberment actors/variants, power-up transformation choreography, and some traversal/action variants. Their trigger logic, movement, effects or encounter context must be restored together. Hatter sky-object path following and rotation remain separate scenery work; moving clock hands do not restore those paths. Character gestures here use original motion data with bounded reconstructed selection rules, not an exact reproduction of every original AI state decision.

Relaunch `Launch.cmd` or a chapter launcher to use the rebuilt game. No new game or asset download is required.

## Movement, shared events and the first damage-surface actor

Movement clips now advance against collision-resolved distance using the original SKAN per-frame travel curve. Walking/running and equipment changes preserve stride phase; rendering faster than the 120 Hz controller does not repeatedly restart the gait. Jump/landing remain controller-driven, ledge clips follow checked climb progress, and rope motion follows actual vertical travel. Cart and airship poses retain their scripted clocks. Standing on a moving platform does not start a walking gait. Root animation never bypasses a body sweep.

Alice's two legs use bounded, filtered vertical foot placement against walkable collision surfaces. It preserves bone lengths, authored boot orientation and the collision body's position. Placement is disabled while airborne, swimming, climbing, riding or in first person. The saved state includes its offsets and the last observed player position. Older saves default these fields. Pausing freezes the complete pose, including unarmed feet.

`animation_events` is shared by sound playback, Alice's idle props and NPC presentation. It reads a bounded declarative subset of original TIKI commands: sounds, tagged attachments/removal, weapon visibility, ordered surface visibility, emitter switches and sever recipes. Events stay associated with an animation alias, including aliases sharing the same frame file. Numeric/first/last frame crossings repeat on loops; entry runs once and exit ends one-shot performances. Frame skips are processed chronologically; pathological catch-up is limited to 32 cycles. Persistent visuals are reconstructed from the saved clip clock without replaying sound. Attached emitters sample deterministic births, preserve the bone pose at emission, and share shader resources with world particles.

The **Club Card Guard** now uses all three authored knife-death variants on lethal Vorpal Blade damage. `death_3a` and `death_3b` separate the upper body at frame 1, expose the body cap and remove the staff. `death_3c` detaches only the head, retains the torso/arms/staff and emits its effect at the neck tag. Each variant uses its own original sound and the shared frame-event timeline. The selected variant is saved once; stable variation across guard placements and attack counts replaces the old single torso choice. This selection is an independent approximation of the original alias-family random choice, not a new headshot rule.

Detached pieces use collision bounds measured from their own selected surfaces over the complete animation, a five-second lifetime, and saved position, velocity and cleanup state. A piece that cannot fit safely is omitted. Masks and effects remain local to each drawn guard. Older saves retain their existing torso variant; ordinary deaths, freezing and other guard types keep their prior behavior. This does not enable every archived gib or reproduce the original engine's debris physics.

Verification commands:

- `--animation-runtime-check`: original stride curves at 30/60/144 Hz, controller walking/running/jumping/blocked movement/climbing, rope/cart/airship save continuation, real leg geometry, cut events and debris cleanup.
- `--animation-runtime-render`: production movement and sever rendering, pause/save readbacks and staged captures in `private/`.
- Set `LOOKING_GLASS_SAVE_CASE=guard-cut` and run `--save-check-write`, then `--save-check-read` in another process: full campaign saves before separation, with a live piece and after cleanup. Omit the variable for the complete save suite.
- `LOOKING_GLASS_SAVE_CASE=guard-head` exercises the same three saved stages for decapitation. The runtime checks cover all three knife variants at 30/60/144 Hz, their individual sound/effect/staff declarations, saved choice and cleanup. Native captures compare restored corpses and subsequent intact guards to detect leaked surface masks.
