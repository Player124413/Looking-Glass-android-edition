# Heart of Darkness — Final: content audit

Audit date: 30 September 2026. Scope: qlair, both boss encounters, attached actors, attacks, effects, cinematics, ending and saved-state handoffs. This does not approve preceding visits or the full campaign milestone.

The fixes below are implemented and packaged in the regular and dedicated launchers. Normal and Hard watched combat routes reach the ending. One verification gap remains: the automated Hard route with every scene skipped fails to recover from combat knockback near an essence pedestal. The separate skipped-scene lifecycle checks pass; a complete Hard skipped playthrough is not claimed. Completion of a combat route alone is not evidence that every original engine command has been reproduced.

## Actor inventory

| Actor | Role and coverage |
|---|---|
| Queen1 | Normal/wounded attacks, ordinary pain variants, laser preparation, popup and telekinesis |
| Queen1 big tentacle | Supporting animated actor; not a separate regular enemy |
| Queen2 body | Slit attacks, both variants, relocation, transformation and death |
| Queen2 halo | Idle/stiff/strobe/death, attached flames and dynamic lights |
| Tentacle 1 | Claw, preparation, swept melee contact, swipe trail, collapse and dismemberment |
| Tentacle 2 | Centipede, Jabberwock and Hatter; independent attack clock, growth/return scales, bounded head targeting |
| Tentacle 3 | Passive damageable part; the original state file declares idle/death behavior |
| Tentacle 4 | Club, preparation, swept melee contact, swipe trail, collapse and dismemberment |

No additional ordinary enemy spawn was found in this map. The original dependency inventory resolved 61 TIKI definitions from 13 roots without missing definitions. That inventory is retained in private/qlair-audit/asset-inventory.json; asset presence alone does not prove execution.

## Corrections

| Finding | Implemented behavior | Verification |
|---|---|---|
| Tab loaded geometry without the mission | Root Launch.cmd detects a selected playtest build missing the finale controller and falls back to the dedicated tested build. The audited build is also selected by the regular playtest launcher. | Exact packaged build: native root Launch.cmd, Tab from skool1, final map, introduction, skip, movement and quick save |
| Seeking, splash and impacts absent | Typed projectile properties; fixed-cadence, bounded steering; swept boxes; direct-hit exclusion; explosion radius/falloff/occlusion; impact sounds, decals and saved effects | Asset-backed moving-target, 30/60/144 Hz, wall, direct/splash and saved continuation checks |
| Fixed probability bands and coupled parts | Ordered independent chance checks, range/visibility gates, independently busy tentacles, randomized recovery, ordinary pain variants | Staged attack and decision checks; full combat routes rerun after balance changes |
| Telekinesis and popup approximations | Escape window and target lock; delayed lateral slam; pull/arrival, world-second squeeze pulses, survival floor and release; floor-projected popup target and visible travel | Native handler constants reviewed privately; route and saved-state checks |
| Missing second attack variants and invisible wave | Both slit variants selected and saved; canonical model keys accept live and legacy names | All ten projectile types emitted in 14 staged attack cases |
| Tentacles fought past collapse | Below-1030 collapse to 25 health; stop attacking; death/blood presentation; finishing hit removes mesh and hit boxes and spawns fragments | Four-part boundary, finishing-hit and legacy-save checks |
| Missing particle/light/trail commands | 280 Hz ice sparks admitted within bounded budgets; attached crown/head lights; popup burst; material-based beams/impacts; claw/club ribbons; recursive model and sprite explosion bursts | Dedicated native capture cases, including each impact family |
| Missing final destruction | Intermediate tagged explosions; final body hide, nested explosion effects and 59 debris fragments; effects persist independently of the hidden body | Saved latches, pause clocks, skip behavior and native final-burst capture |
| Alice could fall after winning | Cinematic control remains active throughout the ten-second ending delay, including the skipped death scene | Ending-control contract and continuous combat completion |
| Head controllers and simplified audio | Authored scale tracks bound to native controller bones; bounded head attention; frame-crossed attack/preparation/growth/return sounds; projectile and death loops | Controller binding, saved clocks, asset resolution and native presentation checks |

The supplied Queen definition references a missing wounded laser-preparation recording. The existing normal preparation recording is used for that event. Sound assets are checked without audible playback, in accordance with this workspace's testing rule.

## Verification record

The initial audit logs use the gaps- prefix under private/qlair-audit/. The beam/Ice follow-up uses beam- logs and beam-evidence.json in the same directory. The asset-backed contracts distinguish staged attack/scene fixtures from continuous combat completion. Native checks use Anode, isolated saves/settings and --no-audio.

Initial audit SHA-256: D767AD5DC10EC80305ADA8A97A37E23D0E332576FF0FEA83EE034079051A54E3. Its preserved copy is private/playtest/looking-glass-d767ad5d.exe. The launcher copies are subsequently updated by the beam/Ice follow-up; its exact hash and verification provenance are in beam-evidence.json.

- 565 unit tests passed, zero failed. Asset-backed finale contracts passed, including grab escape/release, projectile physics and saved continuation at 30/60/144 Hz.
- Continuous watched Normal and Hard routes reached ENDING in 35,877 and 44,854 ticks. Alice retained 100 and 98.4052 sanity respectively. Both use production inputs, damage and weapon costs without cheats or injected boss damage.
- Normal with all scenes skipped reached ENDING in 17,060 ticks. The Hard skipped pilot failed while replanning after knockback near the first essence station, with Queen2 at 1,479.5 health. This diagnostic exits unsuccessfully and remains an explicit verification gap, rather than a passing result.
- 34 native captures passed with no dropped effect passes. Visual inspection included the final destruction, nested impact explosion, ice sparks, popup dust, textured beam, swipe and growth cases.
- All five scene saves were written and restored in separate native processes. Completed-campaign Continue restored the preserved second encounter. Combined native verification job: j_7962e43199b54a5e9ca0c9a509dbc51b, exit 0.
- The exact published build passed root-launch Tab entry and movement. Loading the staged ending fixture then resuming showed the ending film; holding Enter reached Credits, and Escape returned to Main. Native interactive job: j_5460c9531356413b9f4d0839bc07c50e. This is a presentation/handoff check, not a manual boss victory.

Current evidence is private/qlair-audit/gaps-evidence.json and published-root-tab-proof.json. Earlier evidence.json and final-* logs refer to the previous packaged release, SHA-256 7213EC10A72863E3754B46A17D03E7234835E88CEAA3BA7778B3055578DA18F5.

## Beam and Ice Wand follow-up, 30 September 2026

- Staff primary now renders the two orbiting blue meshes and purple core with the original beam rig, materials and root/distal controller scales. Geometry ends at the resolved collision point. First-person charge glow is reduced and its visible source follows the held model's muzzle; the damage ray remains in body space.
- Queen beams now load all four authored materials, including the previously absent queenbeam1. Tracking events aim at the current opponent; sweep events retain their animated horizontal direction and use the opponent's elevation. Damage and beam lifetime are read from the event instead of hard-coded values.
- Ice wall placement probes the whole 48-by-48 footprint before checking full-height clearance. A shallow step under a corner no longer rejects an otherwise clear placement. Ceiling, actor, existing-wall, steep-surface and unsupported placements remain rejected, with the usual Will refund.
- 578 unit tests pass. Heavy, Ice/Jacks and finale asset contracts pass. The revised combat candidate completes watched Normal and Hard routes. Native captures now trigger actual normal/wounded Queen1 and Queen2 eye attacks instead of inserting an artificial beam in front of the camera. Staff views and finale Ice wall placement are inspected in both camera modes; saved weapon states continue with matching damage, event counts and Will.

The prior Hard skipped-route navigation limitation remains open. These beam captures are staged attack tests, not manual boss victories.

## Implementation limits

This is an independently implemented, bounded finale controller, not the original DLL or script VM. Body movement, popup travel, head attention and cosmetic fragment simulation use this engine's integration and rendering conventions. Model-child bursts have explicit dependency, recursion and particle limits. These implementation choices must not be presented as bit-for-bit original-engine parity.

Manual native boss victories and audible audio quality are not claimed. Continuous routes use production movement, weapons, resource costs, collision and damage with no god mode, free ammunition or injected boss damage. Earlier native film/Credits/Main checks remain labelled with their original binary provenance.
