# Croquet Mallet and Jackbomb fidelity

28 September 2026. This supersedes these toys' prototype entries in [the historical audit](TOY_AUDIT.md). Values are base damage before the existing difficulty, Rage and target-specific rules.

## Implemented behaviour

| Attack | Will | Action and reuse | Projectile / damage |
| --- | --- | --- | --- |
| Mallet primary | 0 | Two original 0.80 s swing clips; contact at 0.35 s | 50-unit sweep from the animated weapon origin, native ±15 X/Y and 0..24 Z volume. 24 electric damage and 100 authored knockback per contacted actor, once; world geometry blocks contact. |
| Mallet alternate | 8 | Linked ball at 0.15 s, free attachment at 0.35 s, launch at 0.65 s in a 1.35 s action | 2,500 units/s, gravity 400 units/s², half-extents 8, five-second lifetime. 30 electric direct damage; separately constructed explosion applies 30 maximum splash damage with a 90-unit radius and 30 knockback. |
| Jackbomb primary | 15 | Release at 0.35 s in the original 0.90 s throw; three-second recovery from release | Launch speed 600, gravity 800; three-second fuse. Explodes for up to 300 fire damage in a 300-unit radius, with 300 knockback. |
| Jackbomb alternate | 20 | Same throw; eight-second recovery from release | Same launch; ten-second fuse. Cranks, opens, then rotates and emits the original timed fire projectiles before its final explosion. |

Positive owner velocity along the throw direction adds to ball/bomb speed. Moving backwards does not reduce launch speed. Jackbomb primary uses the explicit ±4 collision box; alternate and breath use the Projectile constructor's ±1 default, because their definitions omit `setsize`.

Croquet balls reflect their full velocity and retain 85% of its magnitude. A shallow floor bounce adds 60 vertical units/s if the reflected vertical component is below 45. There is no three-bounce limit. A living-target contact applies direct damage and explodes; the direct target is excluded from splash. Expiry also explodes. The ball's `radius 100` is **not copied into the separately created Explosion**: the latter omits radius, so its native fallback is damage + 60 = 90.

Jackbombs bounce off walls and actors without contact damage or impact detonation. Their normal velocity reflects at half magnitude while tangential velocity is retained. On a floor normal above 0.7, a bounce below 60 vertical units/s stops translation and box rotation. The head's animation continues. Both bombs have the original -720°/s yaw spin while moving.

The cranking TAN changes animation at its last frame (1.20 s), and the opening TAN changes after another 0.30 s. Alternate rotation lasts 4.05 s and automatically repeats. Its 22 frame commands produce 46 flame births before the ten-second fuse expires. Flame launches use the animated `tag_mouth` origin **and orientation**, at 250 or 300 units/s according to the frame command. Each lives three seconds, has gravity 800, bounces like the croquet ball, and inflicts one 10-damage `firesword` hit with 20 knockback on a living contact. Its emitter belongs to the bomb, so breath can also hit Alice. It has no invented lingering damage tick or contact explosion.

Explosion damage happens once. Distance gives linear falloff, Alice receives half self-damage, and a directly struck ball target is excluded. The original close-radius exception skips world occlusion inside half the radius, or one quarter for Alice; outside that distance a world trace blocks splash. Native explosion placement backs the origin 36 units against nonzero projectile velocity. Existing guard/Boojum/Ladybug recoil consumes the authored knockback through the shared mass-based response; Alice receives a swept impulse too.

Jackbomb's held model disappears on release. Its original reappearance effect starts half a second before the three/eight-second recovery ends. Switching, holding either button and restoring a save cannot bypass the recovery or add another Will debit. Both toys' projectiles continue during Pocket Watch time stop, as declared by `ignore_deadtime`.

## Presentation and saving

The renderer uses the original bomb TANs, animated fuse and mouth tags, croquet particle sprites, fire/smoke sprites, explosion cylinder and shockwaves. Spawned material animation uses effect age; translucent effects join the game's depth-tested material pass. The rotating mouth jet follows its attachment frame. Original toss, music, opening, breath, bounce, impact, explosion and reappearance sounds are connected; Jackbomb's authored sound gains are retained.

The 30 September follow-up restores the original impact spark burst on both Mallet swings and direct ball hits. The animated electrical shell follows the current pose for half a second. The correction below extends its original shared behavior beyond Card Guards. Its material waves and texture movement use the saved hit age, so pausing or loading does not change the flash. The dense ball and explosion-fragment trails use one fifth of their previous sprite size and half their previous opacity; projectile motion, particle lifetimes, damage and Will cost are unchanged.

Eligible Club and Diamond Guard deaths now include the original occasional whole-body burst: approximately 10% of non-knife, non-ice kills, including a lethal Croquet ball. The choice is deterministic per guard placement and attack count and saved once. Seven original red debris models and the dropped staff replace the body, bounce against swept world collision and fade over five seconds. Cleanup remains saved so loading cannot recreate the body or scatter the pieces again. Knife sever variants and frozen deaths retain their own presentation. Debris trajectories and staff release are independent approximations; the original blood emitters on each fragment are not yet reproduced.
Save state includes bomb identity, position/velocity/orientation, rest state, age, opening-event status, number of emitted flames, live flame identity/motion/age, explosion visual age, ball motion/lifetime and action/recovery/debit state. Bounded emitter pose histories retain particle birth positions across movement and loading. Explosion visuals never reapply damage. Existing saves without these new fields remain compatible. Menu pause stops simulation; loading cannot replay the opening cue, previous flame releases or an already resolved explosion.

## Original evidence

Read-only inspection of the user's local archives and DLL informed independent Rust implementations. Extracted assets and decompilation stay in `private/` and are excluded from source packages.

- `models/alice.tik`, `models/w_mallet.tik`, `models/croquetball.tik`, `models/fx_croquetballexp.tik`, `models/w_jackbomb.tik`, `models/prj_jackbomb.tik`, `models/prj_jackalt.tik`, `models/prj_jackbomb_breath.tik`, `models/fx_jackbombexp.tik` and their SKA/TAN/effect references.
- MalletAttack: `0x100bdda0`; melee helper `0x100fe6e0`. Assembly confirms 24 damage, 50 reach and 100 knockback.
- Projectile constructor/launch: `0x100feec0`, `0x10100110`; touch/explosion `0x100ffb30`, `0x100ff710`.
- Physics toss / full reflection / clipped bounce: `0x10077790`, `0x10075990`, `0x10075810`.
- Explosion construction and radius damage: `0x100ff250`, `0x100ff120`, `0x100fb970`.
- Entity animation projectile command: `0x10066740` (distinct from Actor's longer `proj` signature).
- Last-frame scheduling and automatic animation repeat: `0x10024a60`, `0x100256f0`, `0x10025650`.

## Verification and limits

- 299 unit tests passed, including swept thin-wall/actor contacts, repeated ball bounces, lifetime, single direct/splash damage, both bomb fuses, settling, flame contacts, resource handling and saved continuation.
- `--mallet-jack-check` reads the real archives. Both mallet swings hit a close guard using the original animated hand at 0.35 s. All four modes match release/repeat timing and resource totals at 30/60/144 Hz. Both fuses and all 46 alternate flame events survive periodic saves.
- Strict all-target Clippy and the release build passed.
- `--mallet-jack-render-check` exercises the funded Character path and writes isolated snapshots/captures at twelve stages. Running it again loads the previous process's files and compares continued movement, attacks, Will, flame/explosion counts and damage. It uses the normal material pass and includes first-person equipment. All twelve stages passed again after restarting the native process.
- Anode live school input produced a 24-damage Mallet hit on the preview guard and exercised alternate Jackbomb throws and rotating fire in the normal viewer. Both sessions closed cleanly. The preview guard also killed the idle test player; this was an input smoke test, not a certified encounter play-through. User saves/settings were isolated.
- Anode reported `NoDevice` for sound output. Sound assets, scheduling and gains are checked; audible playback was unavailable in this session.

The 30 September follow-up passed 515 unit tests, formatting, a release build and all-target Clippy with the existing counter/fold/unwrap/cast lint exclusions. The extended `--mallet-jack-check` covers electrical hits, both supported Guard classes, burst selection, pause, saved continuation and cleanup at 30/60/144 Hz. Native rendering now covers nineteen stages, including seven Croquet hit/trail/death snapshots, and passed again in a separate process. The original animation-runtime and school combat checks also pass. Screenshots, logs and exact executable/source provenance remain private in `private/croquet-hit-check/`.
This implements the requested gameplay rules for currently supported combat targets. Unsupported enemies still need their own original immunities/reactions. Cosmetic fragment trajectories use deterministic approximations; explosion scorch decals remain a rendering gap; the shared electrical shell is connected to Card Guards, Army Ants, Boojums, Ladybugs and protected Skool pupils. Smoke/fire voices already in flight are not serialized; saved gameplay entities and active looping sound clocks are restored, while past one-shot sound cues are deliberately not replayed. This does not certify full campaign routes or a pixel-identical original renderer.

## Electrical aura correction

The first hit-effect revision selected the original shock material but did not render its repeating texture or explicit alpha threshold correctly. Material-backed model textures now honor repeat versus clamp sampling, and the supplied numeric alpha-test comparisons are carried through both world and model passes. This restores visible lightning around the pose instead of a small dark patch. Effect age remains the clock for both material waves and UV motion.

The original electrocution flag is shared actor presentation. Army Ants, Boojums and Ladybugs now save and advance the same short hit state alongside Card Guards. Protected Skool pupils expose their original contact bounds to Mallet attacks and receive the shell on both a swing and direct ball impact. They retain their protected health, zero knockback and exclusion from hostile AI targets. Hidden and controller-owned cast members are excluded from these contact surfaces.

Private native inspection confirms the Card Guard state enters its whole-body death effect by selecting red debris recipe 5 and dropping its staff. This is a body replacement effect, not a different skeletal death clip. The seven pieces in the existing implementation match that asset; trajectories and the staff's initial release remain approximations. The state graph contains a 0.1 chance gate; the port's once-per-death deterministic selection is not proof of the original state machine's cumulative probability.
## Outward lightning and attached weapons

Reference-image comparison exposed a second rendering error: the model triangle order makes the reconstructed mesh normals point inward. The shock pass now expands along the outward direction, placing the original lightning around the body rather than beneath its opaque skin. The pass also covers visible attached weapons and clears attachment visibility between actors, so removed or hidden staffs do not retain a ghost effect. Other model shading is unchanged.

Both Mallet swing trails now sample the original lightning-swipe material with varying UVs instead of a single point in a plain spark texture. The native check requires bright lightning outside each of six model silhouettes, tests visible versus hidden staff coverage, and retains both actual pupil hit paths and pause/save continuation checks. Croquet captures use a closer camera to make the effect reviewable.

The local launch candidate includes the already implemented finale controller alongside these corrections. This prevents the root launcher's finale compatibility fallback from selecting an older executable that omits the corrected effect. Save format remains 12.
