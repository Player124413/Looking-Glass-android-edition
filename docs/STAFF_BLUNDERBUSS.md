# Eye Staff and Blunderbuss

28 September 2026. Both weapons now have playable attacks and use the shared inventory, input, animation, collision, damage and save systems. Original artwork and sounds are read from the user's local archives.

## Eye Staff

- Primary: hold to charge; the source switches to beam effects at 2.15 seconds and marks the beam ready at 2.30. Starting requires more than 20 Will. The active beam traces 2,000 units, stops at solids, follows aim, and applies 5 damage / 5 knockback per 0.05-second pulse for 0.4 Will. There is no invented fixed startup debit.
- Releasing before readiness cancels. Releasing after readiness launches one spiral projectile: speed 1,000, direct damage 150, knockback 400, lifetime 2.5 seconds. The separate explosion uses 100 maximum radial damage and radius 400, excluding the direct victim; normal self-damage rules apply. It continues under the Pocket Watch; see the [native deadtime correction](DICE_WATCH.md).
- Alternate: the original 0.60-second startup enters the held sky attack. Charging consumes 1 Will per 0.05-second tick. Release after more than 0.5 seconds of actual charging schedules comets, with a roughly five-second charge limit and automatic release on exhaustion. A brief tap does not invent an instant alternate projectile.
- Comets begin at `5 + i * 0.333` seconds after release, for positive integer `i < charge_seconds * 3`. They cycle visible enemies within the original 90-degree cone / 2,048-unit targeting range. Without a candidate, they fall near the forward aim point. A ceiling trace chooses the spawn point. Speed 1,000, direct damage 100, knockback 400, lifetime 2.5 seconds; the same separate explosion applies. Comets continue during Watch time stop.
- The alternate uses its full-body startup/hold/end clips. Primary permits movement. Switching waits for the held attack's release and ending. Death and scripted movement cancel held effects without firing a release projectile; already released attacks retain their saved lifetimes.

## Blunderbuss

There is **one attack**. Both buttons invoke it; there is no added shotgun spread, alternate blast or charge mode.

Alice starts `buss_att` (3.65 seconds). At 0.05 seconds the weapon ignites and consumes **99 Will once**. Its own firing TAN releases the cannonball 0.70 seconds later: **0.75 seconds from the click**. Releasing the button does not cancel a committed shot. Original full-body recoil and the animated weapon run through the action; first-person presentation has a corresponding weapon kick.

The cannonball travels at 2,500 units/s with half-extents 8 and a five-second lifetime. The custom native touch handler bypasses generic direct damage: the TIKI's `hitdamage 8` / `knockback 60` do not add a separate hit. Touch creates an explosion with **998 maximum damage, 800 knockback and radius 1,058** (the native default of damage + 60). Its touch event excludes Alice, not the struck enemy. Generic lifetime expiry has no excluded victim. The 600-unit impact-mark setting is a decal size, not a blast radius. Blast falloff and the existing native inner-radius visibility exceptions are shared with the other explosives.

## Artwork and saves

The build loads the original held/firing models, beam rig and materials, spiral/comet/cannonball models, smoke sprites, expanding projectile shells, charge loops and explosion sounds. Animation-owned particle bursts now preserve source counts and frame times. World effects are depth-tested. Weapon ignition and projectile motion have separate clocks; restoring cannot replay ignition, re-debit Will or release an additional cannonball.

The optional `heavy` save section retains Staff mode/age/tick/aim, pending comet count and deadlines, target cursor and random seed, moving projectiles, impact ages, particle birth histories and cannon ignition. Older saves have empty heavy-weapon state. Pause advances none of these clocks. Impact visuals and remaining trails survive restoration without applying damage again.

Native research is local and read-only in `private/staff-buss-research/`. Relevant observations: `10072780` primary start guard; `10072470`, `10072130`, `10070230` charge stages; `10078670` per-frame dispatch flags confirming that primary draining begins only after readiness; `10070f70` beam trace/debit/damage; `100706c0` release; `100707f0`, `10071e80`, `100708a0` sky charging/scheduling/targeting; `10055ab0` cannon touch; `10100110` projectile lifetime; `100ff710`, `100ff250`, `100fb970` explosion construction and radius damage. Decompiled arguments were checked against instructions and scalar widths. No original code or assets are included in source packages.

## Verification and fidelity limits

`--heavy-check` validates original assets and cannon timing/debit at 30/60/144 Hz. `--heavy-render-check` exercises complete Character/Stats snapshots in a fresh process, then compares the next update's projectile/effect state, Will and damage exactly. It captures primary charging/beam/release, alternate charging/queue/comets/automatic stop, cannon ignition/pre-release/flight/impact and first-person views. Native artifacts remain in `private/heavy-check-v2/`.

Verified for this build:

- 315 unit tests pass; strict all-target Clippy and formatting checks pass. Tests include multiple-target selection and cycling, world obstruction, time stop, empty Will, charge cancellation, projectile expiry, moving geometry overlap, and deterministic particle births after restoration.
- The original-asset `--heavy-check`, `--weapon-input-check`, `--blade-cards-check`, `--mallet-jack-check` and `--ice-jacks-check` pass against the release executable.
- Two native Anode runs completed successfully. All 16 persisted scenarios resumed in the second process with exact next-step charge, queue, projectile/effect, damage and Will state. Staff cancellation was also checked through the full Character path. Both camera views and the revised beam orientation were inspected. Logs: `private/heavy-native-run1.json`, `private/heavy-native-run2.json`.
- Source hashes, executable hashes and the previous installed build are retained under `private/staff-buss-*`. Test settings and snapshots use isolated private directories.
- Sound assets and event dispatch were checked, but the test desktop had no audio output; audible mixing was not assessed in this run.

The 30 September beam follow-up restores the two visible blue meshes and purple core, root/distal controller scaling and orbital motion. The resolved contact point limits beam length. First-person charge glow is reduced and the beam follows the visible muzzle while preserving the body-space damage ray. Native first/third-person captures and saved continuation checks are recorded under `private/qlair-audit/beam-*`.

The Skool1 follow-up reduces the charge particles' billboard size and brightness
in both camera views. It adds a camera-facing core using the original purple
texture, so the beam stays visible when its narrow cutout cone is viewed along
its length. The core terminates at the resolved contact point and remains depth
tested. Beam collision, damage and Will costs are unchanged.
`--heavy-render-check` now includes school-room and corridor
positions, downward aim, the normal 128-unit follow camera, and the inventory
grant used by `wuss`. Its Skool1 captures are in
`private/qlair-audit/skool-weapons/`.

The published follow-up is `AECD7704` (30 September). Its native checks pass all
16 heavy-weapon continuation cases and 12 Skool1 weapon/view/position cases.
The final frame captures were inspected on an isolated Windows desktop; the
earlier live Skool1 mouse reproduction and saves are retained in the same audit
folder. The gameplay change passed the 579-test unit suite before the final
cosmetic core adjustment. Both root and finale launchers use the verified build.

Fine cosmetic differences remain: beam geometry is fitted to the collision endpoint and its orbital offset tapers at the ends; impact decals, thrown rock debris and the original camera earthquake are not reproduced. First-person positioning and recoil are the remake's presentation of original props, because the archives provide no first-person arm rig. These limits do not change attack modes, resource costs or hit calculations. The existing remade enemy/boss systems retain their own documented fidelity limits.
