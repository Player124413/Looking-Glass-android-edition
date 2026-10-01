# Vorpal Blade and Cards fidelity

28 September 2026. This supersedes the Blade/Cards implementation columns in the historical [ten-toy audit](TOY_AUDIT.md). Values below are base damage before the existing difficulty and Rage multipliers.

## Implemented rules

| Attack | Will | Release / repeat | Damage and behaviour |
| --- | --- | --- | --- |
| Blade melee | 0 | Contact 0.35 s, sound 0.25 s, 1.05 s action | 25 damage; three original swing clips, selected with a saved pseudo-random stream. A 40-unit sweep from the animated weapon origin, with the native ±15 X/Y and 0..24 Z volume. Each contacted actor is damaged once; multiple actors can be hit. World geometry blocks the sweep. |
| Blade throw | 0 | Release 0.40 s, sound 0.30 s; 3.5 s recovery from release | 45 damage, 1,200 units/s, gravity acceleration 100 units/s², 1 s flight, 8-unit half-extents. Terminates at the first actor or wall; no boomerang flight. Pitch spin, original projectile model and distinct projectile impact sounds. |
| Cards primary | 3 | Immediate release; held repeat every 0.25 s | One animated card, 7 damage, 700 units/s, 1.5 s life, 8-unit half-extents, authored knockback 60. Seeks the acquired target in 10-degree pitch/yaw steps. |
| Cards alternate | 20 | Loose card attaches at 0.15 s, release 0.35 s; 0.85 s action / held repeat | One carrier, 15 direct damage, 1,000 units/s, 2 s life. At 0.1 s of flight it spawns eight additional 7-damage fragments and **continues flying**. Fragments live for 2 s from birth, travel at 1,000 units/s, and inherit the owner's target when spawned. Carrier and fragments have knockback 60 and 8-unit half-extents. |

The alternate's 0.5 s weapon cycle starts at release, so it ends with its 0.85 s Alice action. It is neither a five-card fan nor eight full-cost attacks. Each projectile can contact one actor. The maximum theoretical direct sum is 15 + 8×7; the spread and walls mean that is not guaranteed damage to one enemy.

The eight children start on a 16-unit ring at 45-degree intervals; each direction is the carrier's forward vector plus one quarter of its radial direction. Seeker updates begin at 0.2 s, then repeat every 0.1 s. Alternate projectiles turn at most 25 degrees per axis on each seeker update; their separate wandering event changes yaw by up to 5 degrees and pitch by up to 3 degrees every 0.15 s. Random spin and wandering state survive loading. Acquisition uses the player's native 2,048-unit, 90-degree full cone, line of sight, and angular closeness. Breakable scenery and Alice's summoned ally are excluded from assisted acquisition. Projectiles retain their acquired target rather than finding a new enemy after it disappears.

The Blade's original return is **reappearance in the hand**. The attached return effect and sound begin 3.0 s after release, and the prop becomes reusable at 3.5 s. The return effect uses the original `meta` sprite with five timed inward bursts. Switching toys, hitting something, or loading cannot shorten the recovery. The Watch leaves both the Blade projectile/return/reuse clocks and Cards active. The September 28 native-handler audit established that `ignore_deadtime 0` also clears the affected flag; see [Dice/Watch corrections](DICE_WATCH.md).

Cards retain a distinct damage type. Supported guards, Boojums and Ladybugs receive mass-based knockback: `500 × authored amount / max(mass, 50)`, followed by the Rust controller's swept, damped movement. Knife deaths continue through the existing severing rules. A conflicting summoned-demon target ID has been separated from the Duchess so player shots reach the boss damage handler.

## Reload evidence

`cards_att_prim_reload.ska` is a real 22-frame / 1.10 s clip. It is **not an active reload step in the supplied game configuration**:

- The torso state asks for `cards_att_prim`.
- The engine animation selector (`alice.exe`, `0x00434d70`) excludes a matching alias when its next character is `_`; this excludes `cards_att_prim_reload`.
- No active state/script references that reload alias directly.
- The Cards weapon declares no ammunition clip. Generic `Weapon::CheckReload` (`0x100fa5b0`) requires a configured, depleted clip.

Consequently held primary fire uses consecutive 0.25 s actions. Release finishes the current action; it does not queue another card or charge for a cosmetic reload. No manual reload control or invented magazine limit has been added.

## Event, collision and save handling

Damage/projectile creation uses the current aim and the animated hand pose sampled at the exact release event. First-person cosmetic hand placement does not change the world-space attack origin. The eye-to-hand trace prevents an attachment reaching through a wall from spawning a projectile or melee hit on the other side. High-speed movement uses swept boxes; actor contacts cannot beat an earlier world contact.

Projectiles advance for the remaining part of their birth frame. Fragment children advance only for the time remaining after their split. The action ledger commits one debit and one release per attack, including through low frame rates, queued switches, cancellation and loading. Blade recovery, target identity, fragment generation, wandering random state and enemy recoil are saved. Legacy projectile saves migrate without manufacturing another carrier or replaying a debit. Flight loops have stable identities while other projectiles disappear.

## Source evidence

Read-only inspection of the user's original definitions, animation files and native handlers informed this independent implementation. Original code, binaries, models, sound and extracted scripts are not included in source releases.

- `models/w_knife.tik`, `models/prj_knife.tik`, `models/w_cards.tik`, `models/w_playingcard.tik`, `models/prj_cardmirv.tik`, `models/prj_carddrunk.tik`, `models/fx_thrownweapon.tik`, `models/alice.tik`, `global/alice_torso.st`.
- Native Blade attack and volume helper: `0x100be1a0`, `0x100fe6e0`.
- Thrown-weapon scheduling: `0x100fa170`.
- Projectile construction / split / seek / wander: `0x10100110`, `0x10100b10`, `0x100fb4c0`, `0x100fb340`.
- Player target acquisition: `0x100b12e0`, `0x100bd6e0`; this is distinct from the generic projectile acquisition helper.
- Sentient knockback: `0x100e0b20`; assembly confirms the 500 factor, minimum mass 50, and default knockback multiplier 1.

Private bounded research is under `private/blade-cards-research/`. Source addresses are evidence references, not callable dependencies.

## Verification and limits

- `cargo test --locked`: 291 tests passed. These exercise sweeps, nearest contacts, wall rejection, the melee volume, ballistic lifetime, radial splitting, steering, 30/60/144 Hz agreement, save continuation, return timing, time-stop behaviour and one-time spending. Strict all-target Clippy and the release build also passed.
- `--blade-cards-check` reads the actual original models, textures, sounds and clips; checks all three melee hands against a nearby guard; verifies both toys' exact release/repeat timing and resource totals at 30/60/144 Hz. `--weapon-input-check` retains the wider input regression checks.
- `--blade-cards-render-check` drives the actual funded Character/weapon path on the GPU. It captures nine private stages, verifies one primary card versus carrier-plus-eight, and restores saved state. Running it again loads the previous process's snapshots and checks that loading does not fire, split or spend again. It also tests first/third-person melee against an actual guard volume.
- Anode live school input exercised a knife throw followed by alternate Cards: two releases, ten contacts, normal shutdown. Settings/saves were isolated. Anode reported `NoDevice` for audio; sound assets and event counts were checked, but audible mixing was not verified in this session.
- All nine native stages passed again after a process restart, including normal floor impacts after loading the split volley. The combat, Dice and item headless checks passed. The existing full-school route replays did **not** pass: school one ran out of Sanity near the recipe exit, and school two did not finish the Boojum battle. A trial of ballistic/movement compensation in the automatic test driver did not resolve both failures and was removed. These remained route-regression blockers; this change did not certify either full school play-through. **Resolved 2026-09-29:** they were route-driver failures (the drivers threw at the nearest visible enemy and stood still in fights), not gameplay regressions, and no Blade or Cards constant changed. See the route-runner repair in [VALIDATION.md](VALIDATION.md). Logs are private (`blade-cards-school-route*.log`, `blade-cards-school2-route*.log`).

These checks establish the requested attack mechanics against currently supported combat targets. They are not a complete original-game combat simulation: enemy-specific armour/deflection/immunity for unsupported enemy types remains with those enemy implementations. The Rust controller's recoil damping, first-person presentation, swipe/impact particles, wall decals and the knife's short material fade are not certified pixel-for-pixel matches to the original. Recovery visibility, gameplay timing, collision dimensions, damage and Will costs are independent of those cosmetic limitations.
