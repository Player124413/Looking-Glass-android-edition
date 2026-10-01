# Ice Wand and Jacks

28 September 2026. This replaces these toys' prototype entries in the historical [toy audit](TOY_AUDIT.md). Values below are base damage before existing difficulty, Rage and target-specific rules. Original files are read from the user's local game archives; no original code or assets are included in the Rust source package.

The held Ice Wand also emits its authored idle `ice_effect` mist at `tag_barrel`
(2026-09-30, report 12). Its cosmetic clock pauses with the game; unequipped or
hidden equipment does not draw it. Attack emitters and resource costs are unchanged.

## Implemented attacks

| Mode | Will and input | Behaviour |
| --- | --- | --- |
| Ice primary | Hold for a 0.05-second repeat; 0.75 Will per pulse, 15 Will/second | 400-unit swept cold attack, half-width 15. Damage is `5 * (1 - contact_fraction)`, with authored knockback 5. Walls obstruct it. Release stops the stream and its loop sound. |
| Ice alternate | Event at 0.45 seconds in Alice's 1.6-second clip; 10 Will for successful placement | Traces ahead and down to the floor, then grows the original ice wall. Failed placement refunds that action's cost. It has 100 health, is immune to Ice damage, blocks movement and attacks, and can be destroyed. Original idle/rise/stand/meltdown timing is 0.10 / 1.10 / 10 / 1.10 seconds. |
| Ice underwater | Either mode, when more than waist-deep in non-lava liquid | Consumes the remaining Will and attaches the original animated ice emitter rig. Alice is frozen from 0.65 to 7.5 seconds; the effect lasts 11 seconds. Breath/drowning continue normally. The emitter changes at 0.10, 6.5 and 8 seconds. |
| Jacks primary | Throw at 0.35 seconds; 10 Will; reuse 6.5 seconds after release | Launches a carrier at 200 forward + 250 upward, gravity 480. After 0.1 seconds it creates four child jacks and a harmless ball. Contacts deal 7 damage with 30 knockback, gated by a 0.1-second contact interval. |
| Jacks alternate | Throw at 0.40 seconds; 20 Will; reuse 2.5 seconds after release | Creates sixteen pieces. After 0.1 seconds each traces a 400-unit segment toward the acquired target with independent spread; unobstructed traces repeat every 0.05 seconds. The first piece deals 7 damage, the fifteen silent pieces 4 each. Each damages once, then ricochets visually and expires three seconds later. |

Primary jacks reflect their full velocity at 85% and receive the original small floor-bounce boost. Their contact-triggered seeking repeats at 0.4-second intervals, with target jitter, horizontal pursuit and an upward bounce impulse. Children pursue the carrier; untargeted pieces and the ball follow Alice. A stalled, blocked seeker reverses its horizontal direction. Children start returning after five seconds; the carrier follows 0.1 seconds later. Return movement is non-colliding and non-damaging and ends after half a second. Catching does not refund Will or bypass reuse. Both Jacks modes continue during Pocket Watch time stop, matching `ignore_deadtime`; the ice wall is an ordinary actor and pauses with the world.

The named original Jacks ammo type still debits Alice's Will: the inspected native player `UseAmmo` path does not maintain a separate Jacks ammunition pool. The alternate attack's custom trace transport supersedes the generic projectile speed declaration.

The native Ice attack does not establish a general timed slow or freeze of every living enemy. Supported club/diamond guards, Boojums and Ladybugs use their original `death_frozen` animation when Ice damage kills them, including their distinct held frames, freeze sound and secondary ice shaders. Existing boss-specific handling remains in place. Underwater player freezing is separate from enemy death freezing.

## Rendering, collision and saving

Original held/animated models, Jacks/ball models, ice-wall animations, sprite definitions, swipe texture and sound files are loaded from the archives. World effects use the normal depth-tested render pass. First-person equipment uses the existing camera-relative presentation. The underwater TANs contain animated tags but zero render surfaces; the decoder now accepts those rigs, including their unused sentinel geometry bounds. The wall's unassigned second surface is degenerate; the supplied wall material is retained without introducing a missing-texture placeholder.

Explicit wall collision bounds are `(-24,-24,0)..(24,24,118)`, independent of its 1.8 visual scale. Placement rejects overlap with Alice, another target or world geometry. Destructible-wall contacts bypass only that wall's collision hull: other walls still block the trace. Saved walls are reinserted into collision on load.

Saves retain continuous-stream history, effect clocks, wall health/melting, player ice-shell age, enemy frozen state, every jack's position/velocity, identity, target/parent, deterministic spread seed, contact/seek timers, child-spawn status and return/removal state. The shared scheduler retains the release/debit flags and reuse clocks. Existing saves without these fields load with empty effect state. Past damage and one-shot sound events are not replayed; active sound loops are reconstructed.

## Evidence

- Local `models/alice.tik` and its SKA files; `w_icewand.tik`, `fx_icewall.tik`, `fx_iceball.tik`, `w_jacks.tik`, `prj_jacks.tik`, `prj_jacks_nosound.tik`, `prj_jackball.tik`, their TANs, materials and emitter commands.
- Ice attack `0x100beea0`; Jacks primary/alternate `0x100be5a0` / `0x100be920`; child spawn `0x100912f0`; seek `0x1008fdd0` and related touch/return/trace handlers.
- Native `Weapon::UseAmmo` `0x100fa050` through player ammo handling `0x100db7f0`; freeze actor `0x10006030`; actor defaults `0x100195e0`.
- Club/diamond guard, Boojum and Ladybug `death_frozen` definitions. Machine instructions and raw constants were checked when Ghidra's decompiler lost arguments or marked live branches unreachable. Private research remains under `private/ice-jacks-research/`.

## Verification and limits

Skool1 follow-up: downward aim is flattened before the forward/down placement
traces. The previous comparison flattened upward aim instead, allowing ordinary
downward mouse aim to put the probe under the floor. The native comparison was
checked against its machine instructions. A regression covers four downward
pitches; the Skool1 captures in `--heavy-render-check` also exercise a single
right-click after the same inventory grant as `wuss`, in both camera views.
The `AECD7704` published build passes those twelve Skool1 cases, all eighteen
Ice/Jacks saved-continuation cases, and finale wall placement in both views.
A live Skool1 mouse reproduction also confirmed one wall and 90 remaining Will
at downward pitch -0.35. Logs and the isolated quick-save are retained under
`private/qlair-audit/skool-*`.

30 September placement follow-up: the downward support trace now covers the wall's full footprint, preventing false rejection when a shallow step or slope lies beneath a corner. Clearance, overlap, support, cost/refund and lifetime rules remain in force. A regression test covers the shallow-step failure. Native funded right-click checks create one solid wall for 10 Will on the actual finale floor in both camera views; first-person wall save/restore is also checked. Evidence is under `private/qlair-audit/beam-*` and `private/ice-jacks-check/finale-wall-*.png`.

The installed launcher build passed 307 unit tests, strict all-target Clippy, the release build, `--ice-jacks-check`, and the existing shared-input and Mallet/Jackbomb archive checks. Anode rendered all seventeen attack/save stages and four frozen enemy models; all seventeen continued with matching state, damage and Will after restarting the native process. Captures were inspected for the stream, first-person Wand, wall, water freeze, swarm/burst, and frozen actors. The previous executable is backed up; `private/ice-jacks-installed.json` identifies the installed binary and source hashes.

The unit suite checks stream falloff/occlusion, placement rejection, solid and destructible walls, lifetime/time stop, underwater freeze/unfreeze events, Jacks collision/targeting/return, damage totals, and serialization at 30/60/144 Hz. `--ice-jacks-check` verifies the funded action scheduler against original clips at those frame rates, exact Will totals, saved reuse and pause, frozen enemy state, and referenced sound assets.

`--ice-jacks-render-check` uses the funded Character path, original artwork and normal render pass. It captures stream, first-person, wall charge/rise/stand/melt/expiry, underwater freeze/release, Jacks spawn/swarm/return/burst/ricochet, and four frozen enemy models. It writes isolated snapshots under `private/ice-jacks-check/`; a second process reads them and compares continued effect state, damage, release counts and Will. This is a controlled combat fixture, not a full campaign play-through.

The implementation uses this engine's fixed-step swept-box collision rather than recreating every original physics rounding detail. Jacks impact sparks, ribbon geometry, terminal fade and frozen-corpse retirement are renderer approximations; the alternate's no-contact cleanup uses its declared four-second lifetime. Unsupported future enemies still require their own original reactions and immunities. Audio assets/events are connected; the automated graphics fixtures do not establish audible output. First-person presentation is inherited from the remake, not a recovered original camera animation.
