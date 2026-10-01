# Verification record â€” 28 September 2026

## Second School encounter repair (2026-09-30)

M0 item8 restores six placed Diamonds, the growth reinforcement and the second
Dice Boojum through legacy Encounters. Existing quest actors retain their owners;
legacy NPC slots preserve Club hit/loot identities while suppressing decorative
Diamonds. Floor spawner receivers are supported, but the supplied map/scripts have
no incoming activation for those two markers. No ambush was invented.

The real-map check covers all four difficulties (entity202 excluded on Easy),
dormant receivers, repeated activation, swept hits, projectile damage, partial
health, death persistence and nine legacy quest stages per difficulty. Nine
new separate-process native save futures pass, including pending Dice dialogue,
a live Diamond projectile, both Dice states and growth completion. Seven actual
older saves load with regenerated NPC ownership. Native Blade input changes
Diamond health55 to10 to0 while Alice takes three7-point hits; F9/F5 preserves
the damaged and dead states. No invisibility, god mode or notarget is enabled.

The full fresh School2 route passes35,802ticks,7jumps,12throws,3swings,126cards,
100combatdamage and86Sanity at the correct return. Combat constants were not
changed. The continuous School2-to-return check also passes both return variants.
Strict Normal campaign reruns from the previous School1 checkpoint pass visit6
but retain the watched gym-death and skipped missing-Dice-grant frontiers at7.
This is not a claim of full campaign completion. Old visit7 route checkpoints
correctly reject exact-state comparison after this intentional encounter upgrade;
the reruns start at unchanged visit6 instead.

481 unit tests, strict Clippy/fmt, event/story/friendly checks and the updated
39-visit registry pass. Only School2's fingerprint changes; old rule keys, save12,
visit numbers and Appendix-F hit ranges remain fixed. Native rendering covers
seven encounter shots plus existing School scene/mover fixtures. Audio was
disabled in the isolated desktop tests. Evidence: private/school2-encounters/.

## Second School potion creation (2026-09-30)

The greenhouse and final Gnome scenes now retain their camera and actor phases
through saves. Three local camera tracks accompany the beaker pour, lollipop
growth, Gnome mixing, condenser motion, growing potion, held Star and disappearance.
The bottle uses its authored vapor emitter. Final dialogue waits for the mixing
clip; advancing a voice cannot bypass potion growth. Skipping uses the same quest
commit as watching, leaving the two rewards separately collectible.

Twenty-eight new watching/advancing/skipping combinations pass with repeated
JSON restores, paused clocks, safe supported handoffs, matching condenser audio
phase, single ingredient consumption and distinct reward gates. The fifteen
previous Gnome scene combinations also pass. Legacy growing, final-dialogue and
mixing saves continue without replaying scenes. Ten new disk fixtures cover these
phases, skipping and collecting only the potion.

The continuous fresh route passes in 35,562 ticks: 7 jumps, 11 throws, 5 swings,
95 cards, 121 combat damage and 100 Sanity at the correct return entrance. No
recovery, resource fills or direct quest changes are used by that route. The
Normal strict campaign, resumed from the preceding build's visit-7 checkpoints,
retains its known limits: watched combat death and skipped Demon Dice provenance
failure. This change does not establish full campaign completion through visit 7.

477 unit tests, strict all-target Clippy, formatting, story/event checks and all
39 fresh snapshot/event signatures pass. Separate native processes write and
reread 103 saved futures exactly; 78 complete level replacements pass. Thirty-eight
captures cover 22 scene shots and 16 earlier school fixtures. The final bottle
emitter is checked in the final native rerun, which also rereads all 19 school
scene futures. Actual F9/P/E/held-Enter/F5 checks confirm resumed mixing cannot
advance early, growth and final watch/skip yield identical items, and collecting
only the potion remains a partial reward after reload. Audio verification decodes 330
referenced files; two already missing original references remain reported.
Native checks use isolated settings/saves with audio disabled; no listening or
running-original comparison is claimed. Binary identity, final input checks and
precise evidence are recorded in `private/school2-potion/WORKLOG.md`.


## Second School Gnome scenes (2026-09-30)

Mushroom, the post-battle laboratory reappearance and Spice Drops now have saved
camera and character staging. The live rescue fight still requires two guards;
watching or skipping the reappearance cannot satisfy that gate. Held ingredients,
Boojum entrance splines, Gnome scale effects and the cabinet reveal follow local
asset data. Completed version-12 conversations stay completed.

Fifteen watching/advancing/skipping combinations pass with repeated saves during
each timeline, stable paused cameras, supported handoffs and no premature reward.
Legacy saves without scene fields retain their quest callbacks. Native testing
caught and repaired defeated guards freezing during dialogue and the missing
viewer skip registration. Nine new save cases cover the conversation, ingredient,
Boojum, growth, walking, cabinet, returning camera and skipped exchange stages.

The complete fresh school-two route passes in 32,320 ticks with 100 Sanity,
including all three Boojums, both rescue guards, ingredients and both final rewards.
The Normal strict campaign still passes visits 1–6. Its watched run dies during
the visit-7 fight; its skipped run finishes the route but fails the separate
Demon Dice grant audit. The previous published build, run from both identical
visit-7 entrance checkpoints, dies in that fight too. Campaign completion through
visit 7 remains unproven; no combat resource fill or grant bypass was added.

All 39 fresh snapshots/event signatures match the baseline. The full 93-case
separate-process save suite and 78 native level replacements pass. The final
focused scene writer/reader also passes after the guard-animation repair; the
published binary rereads all 93 fixtures with identical continued states.
Actual F9/P/held-Enter/F5 tests verify all three scene handoffs: three live
Boojums, two live rescue guards, or Spice Drops alone. 477 unit tests, formatting,
strict all-target Clippy and source/provenance checks pass.
Twenty-eight staged captures cover the new scenes and existing second-school
presentation. Two effect WAVs decode successfully; no listening or running-original
comparison is claimed. Evidence and final binary identity are recorded in
`private/school2-gnome/WORKLOG.md`.

## School library and recipe (2026-09-30)

Saved staging now owns the shelf, book and recipe performances. Six additional
separate-process save fixtures cover shelf motion, book dialogue/push/drop/skip
and the recipe camera. Fifteen actual-contact scene combinations exercise
watching, rapid dialogue advancement, three skip points, pause and repeated
JSON restores throughout the whole timeline. Unsolved callbacks, pending old
exits and a completed legacy recipe with another queued conversation cannot
bypass the scene. Repeated completion cannot duplicate departure.

477 unit tests, formatting, strict all-target Clippy and source/provenance
checks pass. All 84 separate-process save cases and 78 native level replacements
pass; the final executable also rereads all 84 saved futures. Normal strict
chains reach visit 6 with scenes watched and skipped. Actual F5/F9/P/E/held-Enter
checks retain a supported book-scene handoff in school one and preserve the
recipe's remaining delay before exactly one transition to school two.

The continuous school route passes in 34,126 ticks with 100 Sanity and combat
active. All 39 fresh interaction snapshots and event signatures match the
existing baseline. The native capture set includes the shelf reveal/tilt,
book cameras and recipe close-up. Seven scene sound identifiers resolve to
nonempty local PCM data; no listening claim is made. Save format 12, event keys,
visit numbers and hit ranges are unchanged. Detailed final release evidence is
kept privately in `private/school-library/WORKLOG.md`.



## School theatre staging (2026-09-30)

`Theatre_Cinematic` now uses the authored camera cuts, stair and ship movement,
platform entrances, actor gestures, headwatch targets, steam, persistent pipe
and disappearance. The local native dialogue-wait handler confirms one second
after each voice; this override is limited to this scene. Guard activation uses
the existing event once at its late cue, with the existing actors approaching
through the aisles. The library still requires its separate shelf scene.

- 477 unit tests, formatting, strict all-target Clippy, source and provenance
  checks pass. All 39 event signatures and fresh interaction snapshots match.
- Watching, five skip points, ten scene restore checkpoints, pause, pending-cue
  save/skip and repeated callbacks pass. Defeated reinforcements remain defeated
  after completion/load. Sampled stair approaches clear the map geometry.
- Seventeen native school captures cover the five shots, cast entrances, steam,
  dialogue, reinforcement approach, disappearance, retained pupils and the
  existing shelf/bridge/recipe views. No listening comparison is claimed.
- All 78 separate-process save cases pass, including the voice gap, either side
  of the reinforcement cue, disappearance and skip. All 78 level replacements
  pass. The release binary rereads every saved future and actual older format-12
  active/completed school saves. The last source adjustment affects only the
  Gnome idle clip's local clock after smoking; saved simulation is unchanged.
- Interactive F9/P/held-Enter/F5 checks return to the supported home position,
  grounded with script motion cleared, two active guards, no pending duplicate
  event and the independent library puzzle still locked.
- The continuous school route passes in 29,376 ticks at 73 Sanity. Normal strict
  campaign chains reach visit 6 with scenes watched and skipped and no retries.
  This does not extend the existing visit-7 frontier.

Evidence: `private/school-theatre/WORKLOG.md`. Tests use isolated saves/settings
and disabled audio. Save 12, event keys, visit numbers and hit ranges remain.
`SCHOOL.md` records the supported handoff/floor and bounded head-turn choices.

## Beyond puzzle cameras (2026-09-30)

The note demonstration, reset, solve and arch reveal now own saved camera playback
alongside the existing machinery. Completion/skip cannot grant lever success or
open the exit. Direct inspection of the installed script corrected decimal timing
errors in the earlier private note: introduction 12.7 s, reset 7.1 s, solve 7.9 s,
arch reveal 17 s. See `BEYOND.md` for behavior and remaining presentation limits.

- 476 unit tests, formatting, strict all-target Clippy, source and provenance
  checks pass. All 39 existing event signatures/fresh interaction snapshots match.
- Four scenes, four difficulties and four save/skip points cover pause, camera
  clearance, deterministic continuation and grounded handoff. The arch fixture
  deliberately retains an unfinished two-note lever attempt throughout.
- Native captures cover three points in each puzzle shot. All four mid-scene
  saves load in the normal viewer; an actual older format-12 Fortress save loads.
  Interactive F9/P/held-Enter testing resumes/skips the arch reveal safely. Its
  resulting autosave retains notes=2, solved=false, the original lever flags,
  walkway=2 and last_open=false; Alice is grounded with zero velocity and control.
- 72 save cases pass separate writer/reader processes (69552 / 53704), including
  four new scene cases. All 78 native level replacements pass.
- Continuous Normal Beyond route: 29,817 ticks, 21 jumps, 1 throw, 87 cards,
  zero combat damage, 100 Sanity. Strict Normal chains pass visits 1-6 both
  watched and skipped; visit 4 takes 30,028 / 28,339 ticks and exits with 100
  Sanity in both. The existing visit-7 frontier remains outside this check.
- Evidence and executable provenance: `private/beyond-scenes/WORKLOG.md`.
  Desktop checks used Anode and isolated saves/settings with audio disabled.
  Save version 12, event keys, visit numbering and hit ranges are unchanged.


## Reports 9-12 and Fortress Rage pickup (2026-09-30)

First-school theatre and shelf performances now have scene-owned cameras/actors,
with the existing access/guard events delayed to completion. The entrance pupil
walks its five-node circuit. The held Ice Wand emits idle mist. The reviewed
Fortress Rage spawner now becomes an ordinary visible, collectible Rage Box with
its voice/pickup cue, delayed guard activation and lift lowering. Report 11's
existing ledge repair is retained and regression-tested. See `SCHOOL.md`,
`ITEMS.md`, `ICE_JACKS.md` and `LEDGES.md` for behavior and fidelity limits.

- 476 unit tests, formatting and strict all-target Clippy pass.
- School, items, story, Ice/Jacks, ledge, Beyond, shared event and registry
  checks pass. All 39 event signatures and fresh interaction snapshots match
  the previous release. Save format 12 and reserved identifiers are unchanged.
- The school matrix exercises both scene contacts, watched/zero/ten-second
  skips, pause, saved camera/player/dialogue, safe handoff and one-shot effects.
  Rage contact/lift/save/difficulty checks pass on all four difficulties.
- Native captures inspect both school scenes, Gnome/pipe, platform support,
  walking pupil, idle Ice mist and Rage visibility. Pupil continuation agrees
  at 30/60/144 Hz; native Fortress/Pool ledge recovery passes.
- 78 full native level replacements and 68 saves written/read in separate
  processes pass on candidate-d (writer 62208, reader 39592). Four additional
  cases cover both school scenes, the pupil and the delayed Rage callback.
- Continuous first-school route: 30,213 ticks, 63 Sanity, 17 jumps, two climbs,
  seven throws, three swings, 135 cards, 33 damage. Beyond route: 27,337 ticks,
  100 Sanity, 21 jumps, two throws, 88 cards, zero damage. Strict Normal chains
  pass visits 1-6 with scenes watched and skipped. These headless route runs
  used candidate-c; subsequent changes only fix school prop loading/camera
  presentation. This is not a whole-campaign proof or a new visit-7 completion.
- Final native school camera, old-save and restart-reader checks are recorded
  with their executable hash in `private/reports-9-12/WORKLOG.md`. Visual tests
  use the hidden Anode desktop, isolated settings/saves and disabled audio;
  sound assets are checked, but no listening comparison is claimed.

## Pool ledges and scene restoration (2026-09-30)

Pull-up landing checks now account for shelves that rise behind the handhold,
while retaining full standing-body clearance and the existing flat-ledge path.
Pool owns the Rabbit arrival, boulder cutaways, Turtle conversation/departure
and encounters, and final jump/exit. See `LEDGES.md` and `POOL.md`.

- 475 unit tests, formatting and strict all-target Clippy pass. Source and
  staged provenance audits pass; original script prose is not added.
- Six authored Pool slopes pass catch, pull, mid-pull save validation and
  grounded completion. Native Fortress and Pool fixtures show catch, hang,
  pull-up and standing completion, with one climb cue and no held weapon.
  The screenshot's exact coordinates were not supplied; these fixtures test
  the same shelf construction rather than a claimed identical camera position.
- The Pool headless matrix covers five scenes, each watched and skipped at
  zero/two seconds, pause, saved camera/dialogue/player continuation, scene-less
  migration, one exit and transport commitments. Four transport fixtures pass.
  Eight native scene captures and four transport captures pass and were
  inspected, including the Turtle's leaf departure and camera clearance.
- Pool, ledge, traversal, swim, Cheshire, story, Ladybug and registry headless
  checks pass on the final executable. All 39 event signatures match the prior
  release; 38 visit snapshots match and only Pool gains its optional scene
  state. Save format 12, event rule keys, visit numbers and hit ranges remain
  unchanged. A copied pre-change format-12 Pool save loads without replaying
  the arrival scene; the original fixture is untouched.
- The final executable passes 78 native level replacements and 64 save cases
  written and read in separate processes. Settings and saves are isolated;
  audio output is disabled, so this is not a listening test.
- The Normal campaign baseline-fill check completes six legs and stops at the
  known visit-7 `skool2` death. It ran on the preceding candidate, before the
  Pool-only exit-gate compatibility correction. This is not a completed chain
  or a full Pool route proof. Pool enemy combat, native boulder collision and
  damage, exact actor navigation/acting and later Pool visits remain open.
- Final executable SHA-256:
  `81A90369EDF3B7A1AAFB803116C571391DC156ABB1CD0C3346D098E5E437AC6B`.
  Logs, captures, legacy fixture and registry comparison: `private/pool-repair/`.

## Duchess effects and victory focus (2026-09-30)

Restores animation-gated pepper/sneeze emitters, death blood bursts and neck
smoke/Meta effects, the blood-spurt attachment, and phase silhouettes. The
victory camera follows the animated head and then the neck. Size/opacity tuning
keeps the head readable; exact native camera motion, controller swelling and
head/brain debris remain fidelity work. See `DUCHESS.md`.

- 475 unit tests pass, including clip-isolated bursts, both random-lifetime
  syntaxes, and phase trail movement/pause/expiry at 30/60/144 Hz. Strict
  all-target Clippy and formatting pass.
- The normal Duchess route remains 16,861 ticks, 20 throws, 38 cards, 44 attacks,
  13 dodges and 45.001835 Sanity, with the shell returned and the temple exit.
  All 39 fresh-visit snapshots/event signatures match the previous baseline
  byte for byte. Save format 12, event keys and reserved hit ranges are unchanged.
- Native rendering covers 15 Duchess fixtures, including a moving phase and
  three sneeze stages. Eight effect-on/off pixel comparisons pass. Paused
  frames allow at most one channel quantization step; inspected captures keep
  the head and its emission point framed. Shared animation effects also pass.
- The effects build passes 78 native level replacements and 64 writer/reader
  save cases in separate processes. Final particle-size/opacity polish repeats
  the Duchess render checks and the 64-case reader. Settings/saves are isolated;
  no new audible-output claim. The isolated headless legacy check found no
  retained fixtures and is not counted as migration evidence. The full campaign
  chain was not repeated for these cosmetic changes; visit 7 remains the frontier.
- Final executable SHA-256:
  `0E186BD14E88838B68352CA7ACEEA559F15C989F8D537F03691D32314EFF20D2`.
  Pre-polish effects build:
  `6F30A68438F1DE79D2955F1495BBA88C7F44DF048A1FD44960C3E517069D5351`.
  Logs, captures, research and deployment record: `private/duchess-effects/`.

## Aim pointer size and brightness (2026-09-30)

User playtest feedback identified an oversized, saturated marker. Pointer particle scale is now 0.2 and its own emitters use 0.1 opacity relative to the initial restoration. Other effects, aim contacts, visibility rules and saved state are unchanged.

- 473 unit tests, strict all-target Clippy and formatting pass. Native blue/red captures inspected: affected pixels reduce from 9,517/7,557 to 328/203 in the same fixture. Disabled weapon, hidden context and occlusion still produce zero marker pixels. A Garden1 opening also closes cleanly with isolated settings/saves and audio disabled.
- Candidate SHA-256: `BC73DA8F44CC93895FB16BE4802EB7D2556F45666526571C1C0090BBD8255A28`. This display-only adjustment did not repeat the full campaign/save suites; the preceding release's results remain below. Evidence: `private/aim-pointer-tuning/`.

## Rope climbing stability (2026-09-30)

The shared cosmetic rope solver keeps material spacing fixed as the hand moves between segments. The loaded upper span stays taut; the free tail retains gravity, damping, collision and release inertia. Hand positions interpolate at 120 Hz. Rope thickness/artwork and Alice's movement, animations and saved grip are unchanged.

- The regression fails before the fix with 6.7393 units of unwanted material displacement during vertical ascent/descent. Afterward the maximum is 0.4918–0.4938 at 30/60/144 Hz. A swinging climb stays continuous, and final material positions differ by less than 0.005 units across those frame rates. Existing pause, bend and release checks pass; 473 unit tests, strict all-target Clippy and formatting pass.
- Native Pandemonium checks include 300 frames of ascent, descent and hold, with inspected captures at both heights and during a separate swing. Alice moves from -32.7 to 161.3632 and back to -38.9249; all cast/mechanism fixtures pass. These are staged visual and input checks, not an entrance-to-exit playthrough of every rope map.
- Release candidate SHA-256: `E689BD06102E46FAD6A2E3CB53715B8319288924C99C823EFC2CDBE0820CFD3B`. Traversal parses all 27 ropes across seven maps; shared rope climb/swing/restore probes, animation runtime and Pandemonium state/route checks pass. All 39 visit snapshot hashes, event signatures and sizes exactly match the preceding release. Evidence: `private/rope-climb/`.
- All 78 native level replacements and 64 fresh-process save continuations pass with isolated settings/saves. The Normal strict chain passes through visit 6; the existing visit-7 frontier remains. Save format 12, event keys and reserved hit ranges are unchanged. The isolated headless legacy loader found no retained fixtures, so its empty result is not counted as save evidence.

## World-space aim pointer (2026-09-30)

Restores the supplied target-emitter artwork along Alice's aim ray, using primary weapon marker/range declarations, actor and world contacts, and the existing depth/fog pass. This is cosmetic; no weapon, movement, save or event state changes. See [TARGETING.md](TARGETING.md) for supported behaviour and remaining target-orbit/fade fidelity work.

- Final candidate SHA-256: `80A4349EEB90845A23E58C020509E12E31F4ABD83F5161F2DC096FC5E1BA56C3`. The earlier candidate `6C2C29E46271351CB61D603828108D1CDD686089DE7C74119D6E9FF736676608` has identical gameplay and pointer implementation; the final candidate only corrects the new render comparison to reset material/depth state and use separate frames.
- 471 unit tests pass, including nearest solid/actor contacts, hidden targets, clear-space range and weapon-rule parsing. Strict all-target Clippy and formatting pass. The Normal strict campaign chain reaches visit 6 on the earlier candidate. All 39 snapshot hashes, event signatures and serialized sizes are explicitly equal to the preceding published build, including registered controllers.
- Native marker comparisons on the final candidate: blue 9,517 changed pixels; red 7,557; disabled weapon, hidden context and an occluding wall each zero. Blue/red captures inspected. The initial comparison incorrectly counted background geometry; the corrected fixture isolates marker pixels. Evidence: `private/aim-pointer/`.
- All 65 headless flags pass on the earlier candidate. Final native checks pass: full actor visibility, 78 level replacements, 64 fresh-process save continuations, and 180-frame Garden1/Skool1/Fortress1 openings. Live captures confirm the gameplay marker and its suppression during arrival scenes; these short openings do not establish complete scene fidelity. Settings and saves were isolated. The existing visibility harness first needed its capture directory created, then passed.

## Ledge recovery, flexible ropes, pipe smoke and Tower water (2026-09-30)

Airborne edges can now be caught and climbed, using the original hang, shimmy and pull-up animations. Rope rendering uses narrow textured strands with constrained segments, gravity and a free tail driven by the existing checked player swing. The Pandemonium warning performance renders the pipe's own smoke tag; authored pipe removal between gestures is retained. Tower2's entity water brush now supplies rendering, immersion and three rising stages. See [LEDGES.md](LEDGES.md), [TRAVERSAL.md](TRAVERSAL.md) and [TOWER2.md](TOWER2.md).

- Final candidate SHA-256: `9ED7795F1846F1C4FFA4D01CB4213508DB09A6CEA35A672C72A2E4169A48B6CD`. Built in the separate main-agent target directory. Unit tests: 469 pass; strict all-target Clippy and formatting pass.
- All 65 headless flags exit 0. Strict Normal watched/skipped chains pass through visit 6 on the earlier candidate; Easy watched/skipped chains pass on the final candidate. The existing visit-7 frontier remains. Formatting, source/index and staged provenance audits pass, as does the Python identifier-exemption regression test.
- Native Anode: ledge catch/hang/pull-up captures, held-weapon visibility, suppressed attacks and one climb cue pass. The water-surface comparison at heights 624/880/1136/1392 changes 85,070/165,800/248,723/249,730 pixels. Full actor visibility, all 78 level replacements, and an unfiltered save writer/reader pair in separate processes pass: 64 restarted cases including hanging, shimmy, mid-pull-up and mid-rise. Original local assets are used; settings and saves are isolated and audio output is disabled.
- An earlier candidate additionally passed the Pandemonium render suite and a normal Tower2 arrival smoke run; inspected captures show the pipe mesh and smoke, a thin bent rope, and clear arrival geometry. The final candidate differs in saved-ledge validation, cancellation cooldown and test assertions, not these render paths.
- All 39 event signatures match the preceding published build. Fresh-visit snapshots match for 38 visits; only Tower2 changes, because it now has controller state. Save version remains 12, with optional player ledge fields and the existing controller slot. Older dry-tank saves use the controller migration path to restart at that visit's entrance. Event keys and reserved hit ranges are unchanged.
- The provenance scanner now excludes only an exact quoted identifier naming an asset actually mounted from the user's packs. Surrounding prose and unknown paths remain checked, with a Python regression test. This fixes a six-token false positive on the flush sound identifier; no original game prose was added.
- Limits: actual-map ledge probes are staged recovery fixtures, not proof for every edge or campaign chokepoint. Exact native root motion, hanging pain, corner turns and moving ledge supports remain unfinished. Flexible rope presentation is rebuilt on load; the saved player swing is unchanged. Tower2 still lacks its complete encounters, flusher sinking, fans and route; restoring water does not finish the visit. Evidence: `private/traversal-water-fixes/`.

## Fortress arrival camera and cast (2026-09-29)

The arrival's straight-segment paths and improvised camera offsets caused abrupt turns, obstructed views and an orbit on the wrong side of the ship. Fortress arrival now uses cubic position/speed blending, authored camera directions and a smooth clearance correction for moving cave pillars. Passenger seat tags follow the same looping animation as the ship. Other scene controllers and the return reveal retain their existing paths. See [FORTRESS.md](FORTRESS.md).

- Candidate SHA-256: `64CB6C4E8627BAC1357916834D8AD5A3B8C9D6467C0B8383C4638FE9A6CEE6A2`. Main builds use a separate target directory. The running user game and protected target executable are preserved; root Launch.cmd delegates to the current private playtest launcher so a versioned executable can be published while the old game is open.
- Static/headless: 466 unit tests, strict all-target Clippy, formatting and provenance/source audits pass. All 63 headless matrix flags exit 0. All 39 recorded fresh-visit snapshot/event hashes match the previous published build, including a direct comparison of the complete hash files. Save format remains 12.
- Camera coverage: 10,020 samples at 120 Hz, no static or moving-brush intersections; maximum non-cut camera step 4.623 units and turn 0.280 degrees. Watched/skipped endpoints, paused clocks, mid-scene resume and legacy scene-state migration pass.
- Native Anode: sixteen scene captures and individual cast visibility assertions, full actor visibility suite, all 78 level replacements, and an unfiltered save writer/reader pair in separate processes (60 restarted cases) pass. A 6,000-frame live Fortress arrival finishes the scene, returns control with Alice visible and closes cleanly. Captures were inspected. Tests use isolated settings/saves with audio disabled.
- Both Fortress routes retain their previous metrics: first visit 15,300 ticks, 6 jumps, 1 Blade throw, 26 Cards, no combat damage; return 6,990 ticks, 2 jumps, 5 throws, 42 Cards, 2 combat damage. Strict Normal and Easy campaign chains reach visit 6. Visit 7 remains the previously documented chain frontier; this is not full campaign completion.
- Evidence: `private/fortress-camera/`. One initial visibility-suite run lacked its screenshot output directory; after creating that isolated harness directory, the entire native batch passed. Guard attack choreography, projectiles and head tracking remain fidelity work; foreground scenery can still briefly occlude cast in establishing shots.

## Opening chain fixes and the airship yaw: headless results (2026-09-29)

This range (from the F3b verified point to `ca2f69e`) holds the strict-chain route-driver fixes for fortress2 (visit 4) and skool1 (visit 6), and the Pandemonium airship yaw fix. It has been checked headless only. The chain loop was stopped in round 3 for a usage handover, and one implementer edit for skool2 (visit 7) is preserved unmerged as a WIP commit on `level/skool2`.

- Build: SHA-256 `ADDDBDD480C9D6A005D6EC42104C120D1681B29A2771CD1F2706B1D06BD389CF` (copied candidate of `ca2f69e`, not the launcher exe). Unit tests: 463 pass, 0 fail. `cargo fmt --check`, clippy with `-D warnings` and `tools/check_source.py` clean.
- Headless: 64 of 64 flags exit 0 (every Appendix E-1 check, `--save-legacy-check`, `--registry-check` against the F1.0 baseline, `--route-difficulty-check`, `--campaign-legs-check`, `--campaign-graph-check`, and the strict chain to visit 6 on Normal and on Easy).
- Chain: `--campaign-route-check --campaign-strict` passes visits 1 to 6 (gvillage, pandemonium, fortress1, fortress2, fortress1 return, skool1) on Normal and Easy; the reported frontier is visit 7 `skool2$skool2_start1` (a route-driver resource-budget fix is in progress, not a gameplay defect). Route metrics of the airship fix are unchanged by design.
- Anode: NOT run for this range. Owed: `tools/test_visibility.ps1`, `tools/test_render_fx.ps1`, `--level-swap-check`, `--visibility-check`, the unfiltered `--save-check-write` then `--save-check-read` pair, `--campaign-save-chain-write` then `--campaign-save-chain-read`, `--pandemonium-render-check` with a before and after airship capture set, and smoke runs. Anode reported no audio device at the last native run; audible output is not verified.
- Limits: staged fixtures versus route proof are unchanged from earlier entries; the airship yaw is yaw-only (pitch and bank unverified natively); no whole-campaign run is claimed.
- Evidence: `private/final-merge/` and `private/final-merge-matrix/` in the integration worktree.

## Pandemonium airship yaws along its flight (2026-09-29)

Playtest fix for visit 02: the departure flight moved the airship along its 15 s path but drew it, its Gnome and Alice, and placed the seat and chase camera, as if its yaw were 0, so it travelled broadside to the direction its propeller implies. The model's data agree on its nose: the bones named front and back sit at +X and -X, the propeller, its spin axis and the rudders are at the -X end, and the propeller blades, taken with the animation's spin sense, push air toward -X; the passenger seat is at +X. `Pandemonium::ship_yaw()` and `ship_rotation()` now derive a yaw about Z from the travel heading at the saved flight clock (no new saved state). The raw segment headings jump about 54, 26, 15 and 25 degrees at the interior nodes, so adjacent headings are blended with a smoothstep over 0.5 s either side of each node and renormalised, which stays under 100 degrees per second at 30, 60 and 144 Hz. The rest pose and the boarding beat keep yaw exactly 0. The rotation turns the tag offset (so the seat, `validate_player` and `player.feet` stay consistent), the ship, its Gnome and Alice, and `script_facing`. No pitch or bank. The chase camera keeps its authored bearing and distance; it frames the ship from the stern quarter. A flight save written before this change seated Alice at the unturned position and still loads: `validate_player` accepts that seat and the next tick moves her to the turned one. The black frame of the flight beat hides the yaw and 64-unit position cut at flight start.

- Build: SHA-256 `596D1628BA72CF96AB48CC50FEBAE0A4D2F57CCBB0E89FC3DC665107631745C4` (copied candidate, not the launcher exe; the unchanged base build is `F7588D4723DF7B0626D4B08017146B08649964D4611CDEB22C58DC3E1E12BD56`). Unit tests: 457 pass (14 new: yaw exactly 0 at rest and boarding, start and end headings, segment headings between blend windows, the node mean, continuity at 30, 60 and 144 Hz, determinism, vertical hops, reversals, one-segment paths, the attachment transform). Clippy and fmt clean.
- Headless: every Appendix E-1 flag plus `--save-legacy-check`, `--registry-check` (all 39 F1.0 hashes byte for byte), `--route-difficulty-check`, `--campaign-legs-check` and `--campaign-graph-check` exit 0 (63 runs). `--pandemonium-check` gains a real-data check: yaw 0 at rest and boarding, first and last segment headings, continuity, determinism, the seat against `validate_player`, clear body and no trigger at every flight tick, the chase camera's distance, framing and clear line, a mid-flight restore, and a pre-change flight save. Route pandemonium: 21,730 ticks, 6 jumps, 11 throws, 1 authored teleport, 56.6 Sanity; skipped 21,115 ticks, 55.3 Sanity, both unchanged. 59 of the 63 logs are byte-identical to the unchanged build's; `--pandemonium-check` differs by its new lines, and the two Pandemonium route logs and the legs log differ only in the printed final WAIT position, which is Alice's seat after the flight.
- Chain: `--campaign-route-check --campaign-strict --campaign-to fortress1` exit 0: gvillage 12,969 ticks, Pandemonium 19,983 (Sanity 100 to 57), fortress1 15,300, with the pre-exit save of visit 02 (inside the flight) rebuilt window-free.
- Anode: not run in this stage. Recipe 13c owes `--pandemonium-render-check` (adds the staged flight captures `pand-flight-start`, `pand-flight-turn` and `pand-flight-end`), `--pandemonium-machinery-render-check`, `tools/test_visibility.ps1`, `tools/test_render_fx.ps1`, `--level-swap-check`, the `--save-check-write` and `--save-check-read` pair, and a before and after capture set of the boarding, flight and departure shots on the unchanged build and the new one.
- Limits: the orientation is a headless derivation from data and the Fortress precedent; whether the ship visibly faces its travel is confirmed only by the native captures. The propeller's thrust sense comes from blade geometry plus the animation's spin and the Fortress precedent, not from a native run. The native chase camera's own follow settings (a distance of 256 and a yaw of 115 on the camera entity) were not reproduced or verified. Pitch and bank are not modelled. The black sky behind the platform is authored (no sky is defined and the map is enclosed by unlit void walls) and is unchanged.
- Evidence: `C:/DEV/McGee/private/p02-airship-yaw-impl/round-1/` (per-flag logs, the unchanged build's logs, model probe output, `build.json`, fmt, clippy and test logs).

## Native verification of F2, F3a and F3b (2026-09-29)

Anode runs of the integrated build after each of the three foundations that changed saves, the transition path and the chain harness. Each ran on its own copied exe, and the two save-format-sensitive ones ran the unfiltered writer/reader pair in separate processes.

- Build: F2 SHA-256 `46C89999F90BF74B3A762335CEE96AF255CA99C86CC4AC0EBB630E8C348F10F8` (merge `a6909e8`); F3a SHA-256 `C354B9EC2BD552620195F71645359B69D902712622DA6E82614292CDF9EE2FBC` (merge `7f70eb8`); F3b SHA-256 `B580FF47B5D5301478FA332DF68EDFD8C8661386F8FEC356F1B716FECEF2B219` (merge `23f2f5e`). Copied candidates, not the launcher exe, unchanged before and after every batch. Unit tests: 412 pass at F2, 421 at F3a, 445 at F3b.
- Headless: every Appendix E-1 flag exits 0 at each merge; `--registry-check` reproduces the 39 F1.0 hashes byte for byte at each. At F3b the strict chain (`--campaign-route-check --campaign-strict`) passes gvillage, pandemonium and fortress1 and reports `FRONTIER fortress2$fortress2_start1` on Normal, Easy and Hard; the skip-cinematics variant reports the same frontier; `--campaign-graph-check` passes its acceptance lists.
- Anode (seat 0.11.0, session 4, 1280x720 at 100%), all owned jobs exit 0:
  - F2: `--save-check-write` (writer 65092; the written saves carry format version 12) then a separate `--save-check-read` (reader 101572) with all nine legacy PASS lines, the format-7 Duchess line, 59 restarted cases and the final PASS; `--save-preview-check`; `tools/test_visibility.ps1` (116), `tools/test_render_fx.ps1`, `--level-swap-check` (78 full level replacements), `--visibility-check`.
  - F3a: `--level-swap-check` (78), `tools/test_visibility.ps1` (116), `tools/test_render_fx.ps1`, `--visibility-check`; writer 95200 and a separate reader 59876 with all nine legacy lines, the Duchess line and 59 restarted cases; 900-frame skool1, skool2 and gvillage `--new-game` smoke runs.
  - F3b: `tools/test_campaign_chain.ps1` headless strict chain; `--campaign-save-chain-write` (writer 50316) then a separate `--campaign-save-chain-read` (reader 97648) with restarted auto and slot1 saves for visits 01 to 03 and an auto-only save for visit 04; `--level-swap-check` (78), `tools/test_visibility.ps1` (116), `tools/test_render_fx.ps1`, `--visibility-check`; the unfiltered `--save-check-write` (writer 39644) and separate reader (21048) with all nine legacy lines, the Duchess line and the final PASS; 900-frame skool1 and gvillage smoke runs.
- Anode reported no audio device; audible output not verified.
- Limits: the visit 04 fortress2 native fixture is auto-only, because the writer stores the pre-exit manual slot only for a leg that passed and fortress2 is the chain's current frontier; it becomes a full pair when that leg is tuned to pass. The chain reaches visit 3 only. Exit codes and PASS lines are the claim; captures were kept but are not visually reviewed here. Another agent on the machine (Codex) shared the seat, and the lease queue worked without contention failures.
- Evidence: `private/F2-native/`, `private/F3a-native/`, `private/F3b-native/`, `private/campaign-chain/`, `private/campaign-save-chain/`.

## Campaign chain check, F3b (2026-09-29)

The chain harness of the plan's F3 is finished on top of F3a. `--campaign-route-check` runs `campaign_route::LEGS` back to back on one `Stats` and ledger through `campaign::arrive` and prints one `PASS` line with metrics per leg, a reward-provenance report and `FRONTIER <map$entry>`: the first failing leg with its message, or, for a visit that has no driver yet, the start thread nothing handles. `--campaign-strict` skips the baseline loadout and starts from no chapter loadout, so each of the twelve milestones has to come from its authored pickup (found in the map's own entities at run time), the temple's shell is kept and listed as the `utemple arrival grant`, and the optional Blunderbuss is reported, not required. It writes `private/campaign-chain/NN-<map>-<first|return>.json` and `report.json`, resumes from a checkpoint (and refuses one that is not proof for the run), takes `--campaign-from` and `--campaign-to`, `--difficulty`, `--campaign-skip-cinematics` and the diagnosis-only `--campaign-allow-retry` (always reported as failing), and has a variants list (today school one's secret room). `--campaign-expect frontier=<visit>` (or `no-checkpoint`) pins an honest failure: the run still prints and writes exactly what it found, but its process exits 0 only while the pinned failure is what happened (the chain stops short of its range with its frontier at that visit, or a strict chain refuses to start for want of a checkpoint) and exits 1 the day the frontier moves; it never turns a completed chain or a diagnosis run into a pass. A run that refuses to start now writes a `refused` report, so an earlier run's `report.json` cannot stand in for it. At every boundary and in the middle of every passing leg it saves, rebuilds a second route from the saved text and runs both for 600 ticks of identical input; it holds the ledger to the save limits (8 MiB, 72 cached visits, 10,000 elements per array) and round-trips the entrance and the pre-exit moment as real saves written by the save `Store` and rebuilt with `save::rebuild_headless`. `--campaign-graph-check` holds Appendix A, compiled into an identifier-only table, against the data for all 39 visits and prints the three lists. The windowed `--campaign-save-chain-write` and `--campaign-save-chain-read`, `tools/test_campaign_chain.ps1` and the completed `docs/CAMPAIGN.md` finish the item. No route driver, gameplay constant, save version or crate changed.

- Build: SHA-256 `E609D1FD369508333142E2C03B8B1FCDA60633220480E50B942C4BF98BEAACFC` (copied release candidate, not the launcher exe). Unit tests: 445 pass (24 new: the probe input; the exit table and its expected visits; the script fact scanner, including that comments hide a dead level change; the reward table and its failure rules; the visit parser, checkpoint names, variants, the resume guard, the exit policy, the expectation parser, the pinned-frontier gate and the typed missing-checkpoint refusal, the checkpoint round trip and the save limits at 39 and 79 visits). Clippy and fmt clean; only the files this change created or edited were formatted.
- Headless: every Appendix E-1 flag plus `--save-legacy-check`, `--registry-check`, `--route-difficulty-check`, `--campaign-legs-check` (Normal, Easy, Hard) and `--campaign-graph-check` exit 0 (66 runs). `--registry-check --registry-baseline` reproduces all 39 F1.0 hashes byte for byte. All 66 logs are byte-identical to the first F3b round's, which were byte-identical to F3a's round-2 logs, so every route metric is unchanged (school two 25,814 ticks and 100 Sanity, Beyond the Wall 24,484 and 47, school 21,033 and 73, return chain 12,134 and 82, and the rest).
- Chain (strict, from New Game state, `--campaign-route-check --campaign-strict`, exit 0 because no `--campaign-to` gates it): Normal passes gvillage (12,969 ticks), Pandemonium (19,983 ticks, Sanity 100 to 57, 460 ticks in slime, 1 authored teleport) and fortress1 (15,300 ticks, Sanity 57, 1 teleport), then `FRONTIER fortress2$fortress2_start1` (Route died at (1572.26, 581.16, -332.97)). Skipped cinematics: 8,962, 17,517 and 5,399 ticks, same frontier. Easy: 12,969, 20,013 (Sanity 77) and 14,991 ticks, then no physics-input route to (2340, 3392, -64) through the rolling walkways. Hard: 12,969, 19,983 (Sanity 19) and 15,878 ticks, then Route died at (1431.27, 380.91, -332.97). These are the frontiers F3a reported; nothing was retuned. `--campaign-to 3` passes and resuming from the visit 4 checkpoint reproduces the same failure from the same entrance state. The three failures the plan tells the harness to report and not to fix are checked with a pin, each exiting 0 while its failure holds (`tools/test_campaign_chain.ps1 -Pinned` runs them; without a pin each exits 1): `--campaign-to fortress2$fortress2_start1 --campaign-expect frontier=fortress2$fortress2_start1` (the gating run cannot pass visit 4: the FAIL line, then `FRONTIER fortress2$fortress2_start1`); `--campaign-from 4 --campaign-allow-retry --campaign-expect frontier=skool1$skool1_start1` (Sanity and Will restored once: fortress2 passes alone as recorded, 24,463 ticks, 21 jumps, 9 throws, 38 cards, Sanity 47, the fortress return passes in 7,149 ticks, and skool1 stops on the missing Croquet Mallet grant; the report says `diagnosis`, run in a scratch working directory so it never writes next to the proof checkpoints); and `--campaign-from 6 --campaign-expect no-checkpoint` (a strict chain never starts from a chapter loadout: it refuses and writes a `refused` report, in an empty scratch working directory). The viewer-style chain (no `--campaign-strict`) stops at the same place; from the staged fortress return it runs school one (19,291 ticks) and its `skool1-secret` variant (21,751 ticks), fills the Mallet in on exit, and dies in school two (F3a's chained school frontier). `tools/test_campaign_chain.ps1 -Determinism` ran the strict chain twice in separate processes: every leg's entrance and exit state hash is equal.
- Save and continue: passed at the four entrances the chain reached and in the middle of the three passing legs (checkpoints at tick 4,000, or 2,699 for the skipped fortress1; 600 ticks of identical input each), in every run above. The ledger held 0 to 3 visits and 28 to 82,456 bytes. The arrival autosave and the pre-exit manual save (one second before the exit) of each visit were written by the save `Store`, read back and rebuilt window-free with a stand-in Alice block: 22 to 134 KB each.
- Exit graph: PASS. Lists 1 and 2 name exactly {9, 10, 12, 17, 18, 22, 23, 25, 27, 28, 37, 38, 39}, each with its reason (script-only: utemple, centipede1, centipede2, rchess1, funhouse, hatter2, jlair2, wforest return, and qlair's ending film; scene-fired: potears1 #115, potears2 #68, and the fallback volumes of facade #67 and keep #71). List 3 flags all eight volumes the plan names (centipede2 #43, hatter1 #121, wchess2 #32, garden4 #6, utemple #30, potears2 #50 and #68, wforest #126 on the return visit) as live at fresh entry, plus centipede2's growth-scene trigger #34; fortress2 #43 and skool2 #29 and #75 are gated by their controllers.
- Anode: not run in this stage. Recipe 13c owes `--campaign-save-chain-write` then `--campaign-save-chain-read` as two separate processes (through `tools/test_campaign_chain.ps1 -InsideAnodeSeat`, after the headless chain has left its checkpoints), `--level-swap-check`, `tools/test_visibility.ps1`, and the unfiltered `--save-check-write` then `--save-check-read` pair with all nine legacy PASS lines. The windowed pair could not be run here and is compile-checked only; its game construction is the same `build_game` that the headless round trip exercises.
- Anode reported no audio device; audible output not verified.
- Limits: the chain reaches visit 3 and reports fortress2 as its frontier; the legs of visits 5 to 8 pass alone but were not reached by a strict chain, and the strict chain would stop at visit 6 on the missing Croquet Mallet grant. The mid-leg probe compares 600 ticks of a fixed walking input at one moment per leg, not a driven leg; a headless checkpoint has no cast and a stand-in Alice block. The exit graph is a reviewed table held against the data. This entry does not claim a whole-campaign run.
- Evidence: `C:/DEV/McGee/private/f3b-chain-check-impl/round-2/` (per-flag logs, the chain runs, the pinned runs and their reports, the PowerShell run summary, `build.json`, fmt, clippy and test logs); the first round's is in `round-1/`.

## Campaign chain core, F3a (2026-09-29)

The viewer's normal exit is now a shared function, `campaign::arrive`, that the headless routes call too, so viewer and route cannot drift: the destination loads at the resources' difficulty, the visit that was left is recorded in the ledger, the baseline loadout is filled in (a strict exit keeps only authored arrival grants, such as the temple's shell), the opening scene plays only for an uncached visit, and an obstructed entrance fails the exit instead of dropping Alice into flight. `Route` gained `enter` (carried resources and a real difficulty), `depart`, a `stop_at_exit` latch, the Cheshire hints, `Metrics` and the shared assertions; every opening check body became a `drive(r)`; school two moved from its private harness onto the shared route (with placed club guards, the quest-item sync, `dice_cat` and story-deferred exits); and `campaign_route::LEGS` lines the eight opening drivers up with the campaign route. No save format change (`VERSION` stays 12), no Cargo change.

- Build: SHA-256 `9C8175FF171D4EA3EBA98E378243A0843CC0FDD765AC2FD8F4D4AC5F61049055` (copied release candidate, not the launcher exe; round 2, after the review fixes below; every log of an earlier build of the same source, `A01D8F7F...`, is byte-identical). Unit tests: 421 pass (9 new: the 39 visit keys and the eight opening exits, the strict arrival grants, the obstructed entrance, the legs' alignment and portal counts, the metrics, and two for the route planner's tolerance). Clippy and fmt clean.
- Headless: all 58 Appendix E-1 flags plus `--save-legacy-check` and `--registry-check` exit 0, and so do the two new flags. `--registry-check --registry-baseline` reproduces all 39 F1.0 hashes byte for byte. Every recorded route log is byte-identical to the log of the unchanged branch apart from one added line (the Cheshire hint load) except school two: village 13,987 ticks and 100 Sanity, Pandemonium 21,730 (skipped 21,115), fortress 15,300 and return 6,990 with 100 Sanity, Beyond the Wall 24,484 and 47 Sanity, school 21,033 and 73, school secret 21,270 and 77, return 12,134 and 82 (`--school-return-check` and `--school-return-chain-check`), Duchess 16,861 and 45.
- Re-baseline, `--school2-route-check`: 26,915 to **25,814 ticks**, 7 to 8 jumps, 8 to 10 throws, 4 to 3 swings, 117 to 109 cards, 70 to 69 combat damage, **100 Sanity unchanged**. Cause: the private harness dropped the weapon's knockback and damage kind when it hit a Boojum or guard (`School2::hit`, `Guard::hurt_kind`); the shared route delivers the whole hit as the viewer does (`School2::hit_attack`, `Guard::hit`), so they recoil. With the old delivery restored the folded route reproduced the old log byte for byte (26,915 ticks), which proves the rest of the fold is exact. No gameplay constant changed.
- Route legs (alone, from each visit's baseline, latched at the exit; `--campaign-legs-check`, strict, Normal): gvillage 12,969 ticks, Pandemonium 19,983 (Sanity 100 to 57, 460 ticks in slime, 1 authored teleport), fortress1 15,300 (1), Beyond the Wall 24,463 (Sanity 47, 3), fortress1 return 6,990, skool1 19,831 (Sanity 77; with its secret room 19,171, Sanity 48), skool2 25,785 (2), skool1 return 12,046 (Sanity 82). Each leg passes the shared assertions (authored exit and teleports, no lava, no recovery, one transition, alive) and its exit hands over to the next visit through `Route::depart`. All 39 entrances are clear.
- Difficulty (R3): `--route-difficulty-check` builds all 39 visits at Easy, Normal and Hard; enemies or pickups differ in 15 (six of the eight opening visits: fortress1 has 5, 7 and 8 actors and 6, 5 and 4 pickups; skool1 25, 31 and 32 actors), the route built by `Route::enter` holds exactly what the loader built at every difficulty, and damage scales by the difficulty.
- Easy and Hard (reported, never gating; `--campaign-legs-check --difficulty easy|hard` exit 0): the fortress1 first visit now finds its route at both (Easy 14,991 ticks, 9 jumps, Sanity 100; Hard 15,878 ticks, 11 jumps, Sanity 100). Of the nine leg bodies alone, eight pass at Easy (all but Beyond the Wall: the recorded timed crossing walks off the Easy-only deck) and eight at Hard (all but school two, where Alice dies in the closing battle at 1.4 times the damage); both are printed as `REPORTED not passing` and counted in a closing `REPORT` line, and the run goes on to the remaining legs. Easy: gvillage 12,969, Pandemonium 20,013 (Sanity 78), fortress1 14,991, return 6,098, skool1 16,981 (83; secret room 18,871), skool2 26,535 (89), skool1 return 12,140. Hard: gvillage 12,969, Pandemonium 19,983 (Sanity 20), fortress1 15,878, Beyond the Wall 24,375 (61), return 8,127 (90), skool1 22,741 (27; secret room 23,101), skool1 return 12,046 (75).
- Chain: not passing yet, reported by `--campaign-legs-check` and not gating. Chained on carried resources at Normal the strict chain stops at Beyond the Wall (visit 4; Alice arrives with 57 Sanity and the recorded route spends 53), and school one to school two stops in school two (77 Sanity and 2 Will on arrival). The recorded drivers assume full bars at every entrance, keep walking after the exit trigger, wade 460 ticks of slime in Pandemonium and collect neither the Mallet nor the Demon Dice: route-driver retunes for F3b and M0.
- Review round 2: the first round's Easy and Hard runs stopped at the fortress1 leg ("No physics-input route to (-5, -2430, 32)", exit 1). Cause: the goal lies on a narrow ledge, and from the spots Easy and Hard bring Alice to (about 20 units north of where Normal does) no state of the 24-tick search lands standing within 22 units of it (the closest, 27.5 units away, cannot reach it in one more step, and a state 4.7 units away is still falling). `Route::navigate` now retries an unplanned goal with a 40-unit radius (`Route::plan`, printed as `Widened the tolerance`); a goal that plans at 22 keeps its plan, so all recorded logs are byte-identical to round one's. Two driver corrections: the school secret-room assertion now scales the 45-second Darkened Looking Glass by the difficulty's power-up duration (36 seconds at Hard; Normal still asserts more than 40), and the legs check reports instead of stopping at a leg that does not pass at a non-Normal difficulty.
- Anode: not run in this stage. Recipe 13c owes `--level-swap-check` (the viewer's `enter_level` now builds on `campaign::load_visit` and `Scene::from_parts`), `tools/test_visibility.ps1`, the unfiltered `--save-check-write` then `--save-check-read` pair (all nine legacy PASS lines) and a 900-frame skool1 and skool2 smoke run, plus a smoke of the viewer's new exit path.
- Anode reported no audio device; audible output not verified.
- Limits: staged only headless. The viewer's exit path (`arrive` plus the window half, on copies that are kept only when the whole destination loads) has no native proof yet. The chained runs do not pass; this entry claims legs, the shared transition and the difficulty input, not a campaign run.
- Evidence: `C:/DEV/McGee/private/f3a-chain-core-impl/round-2/` (per-flag logs, the Normal, Easy and Hard legs logs, `summary.txt`, `build.json`); round one is in `round-1/`.

## Save format 12 and the generic upgrade path, F2 (2026-09-29)

The save format goes from 11 to 12 exactly once (DG-6), in its own commit: the writer writes 12, the reader accepts 1 to 12, and a non-empty per-visit `levels` map (registered-controller state) in a file older than 12 is rejected, so older executables refuse registry-era saves with a version message. No existing event signature, rule key or controller state changed. On top of it, a save that predates a visit's registered controller is upgraded instead of refused: the visit is restored against the program it was made with (its own program minus the registry), extended to the current one (`Runtime::extend_registered_from`: `extend_gated_from` plus new writable facts and sends to the controller's own receivers), formerly pending script triggers are rearmed, ambushes and rewards are counted as run, Alice restarts at the entrance when the controller rejects her position or her body is not clear, and the saved cast is regenerated when a controller owns part of it. `levels::state` is the saved-state template for controllers, `SaveCase` gained a behaviour hook so a visit never edits `save_check.rs`, and a synthetic 39-visit save proves the 8 MiB, 72-visit and 10,000-element limits are enforced. No visit is registered yet, so all of it is proven with a synthetic controller, synthetic BSP fixtures and synthetic saves.

- Build: SHA-256 `3EAC8D35A21BBF2075494353BA38E1491ED71C30C3648489231BC3743AED14D4` (copied release candidate, not the launcher exe). Unit tests: 412 pass (43 new: the bump, the guards and the writer/reader versions; the state template; the event extension; the upgrade steps and their mutation-checked failure modes; respawn and cast regeneration; the 39-visit and limit tests; the registry save-case plumbing). `cargo fmt --check` clean (only changed files were formatted), `cargo clippy --locked --all-targets -- -D warnings` clean.
- Headless: all 58 Appendix E-1 flags plus `--save-legacy-check` and `--registry-check` exit 0. `--registry-check --registry-baseline` reproduces all 39 F1.0 hashes byte for byte. `--save-legacy-check` restores 15 of 15 retained fixtures, now also asserting that the format-7 Duchess fixture gains its controller. Route metrics are unchanged from the P0.8 record (school 21,033 ticks and 73 Sanity, school two 26,915 ticks and 100 Sanity, school return chain 12,134 ticks and 82 Sanity, Beyond the Wall 24,484 ticks and 47 Sanity).
- Limits: a full 39-visit save with 80 KB of controller state per visit is about 3.2 MB (limit 8 MiB). Forged files with valid checksums are refused for a 73rd visit or completion marker, an array of 10,001 elements, and a file of 8 MiB plus one byte; the writer refuses the same. Formats 9, 10 and 11 have no retained fixture and are listed in SAVES.md as untested against real files. The upgrade could not run against a real registered visit (none exists yet) or against the NPC models (step 7 is unit-tested through its decision and merge functions); the first registered visit exercises both.
- Anode: not run in this stage. Recipe 13c (milestone `F2`) owes the unfiltered `--save-check-write` then `--save-check-read` pair in two processes (the reader log must hold all nine legacy PASS lines, the restart cases and the new `legacy-v7-duchess` line), `--save-preview-check`, `tools/test_visibility.ps1` and `--level-swap-check`. Nothing here claims audible output.
- Evidence: `C:/DEV/McGee/private/f2-save12-impl/round-1/` (per-flag logs, `summary.txt`, `build.json`, fmt, clippy and test logs).

## Level-controller registry, F1 (2026-09-29)

A visit that no typed legacy controller owns now plugs in as one `LevelController` (`src/level.rs`) registered by a single uncommented line in `src/levels/mod.rs`, instead of arms in the chains of `interaction.rs`, `viewer.rs` and `route.rs`. Every generic hook is placed after the legacy chain it belongs to: the `Interactions` hooks (facts, gates on visits no legacy controller owns, receivers then rules, outputs, colliders, transforms, liquids, movers, prompts, updates, script threads before the pending notice, scenes, dialogue, loot), the viewer loops (art, transport, skip id, recovery entrance, movement ownership, combat step, targets, hit dispatch, camera, fade, help text, effects, HUD), the same combat step in `route.rs` through one shared `level::step_controllers`, and the consumers in `render`, `decorations`, `npc`, `audio/world`, `story`, `main` (registered flags, help, visibility fixtures), `save_check` (registered cases) and `tools/package_source.py` (launchers by glob). Appendix F is committed as code (`src/levels/reservations.rs`): hit-ID base, key prefixes, flags, save-case prefix and private path of every visit, and a registration is held to them. `Snapshot` gains `levels`, which is skipped when empty. No save `VERSION`, Cargo version, existing rule key, action, precedence or hit-ID range changed, and no crate was added. See [LEVEL_REGISTRY.md](LEVEL_REGISTRY.md).

- Build: SHA-256 `88205F67A36B778C8DD2582A559F83D951DDD554D4E7C71CB55833EFEA0825BD` (copied candidate, not the launcher exe). Unit tests: 369 pass (new ones cover the trait defaults, hit ranges, target ownership, combat aggregation, key namespaces, event-program join and restore, snapshot round trip, the legacy-first order of gate, camera, fade, scene id, skip and hit, registration limits, the reservation table and the dynamic liquid volume). Clippy and fmt clean; only files this change touched were formatted.
- Baseline (F1.0): `--registry-check --registry-record` was run on the untouched code, before any refactor, and stored 39 SHA-256 hashes (36 first entries plus `fortress1_start2`, `skool1_start2` and `wforest_start2`) of the serialized `Interactions::snapshot()`, including the event signature. A second process reproduced them, and each visit is built twice per run with identical snapshots. The baseline commit precedes the refactor commits on `system/f1-registry`.
- Headless: `--registry-check --registry-baseline` reproduces all 39 hashes byte for byte after every refactor commit and on the final build; it also validates the (empty) registration table and injects a probe controller into each of the ten legacy-owned visits, whose event program stays byte-identical and whose scene skips are taken by the legacy controller (gvillage and fortress1 open in a scene) before the registry is asked. Removing the legacy-owner guard from the gate hook makes that probe fail at gvillage, so the check is not vacuous. Every existing Appendix E-1 flag plus `--save-legacy-check` and `--registry-check` passes (60 flags, all exit 0). Route metrics are unchanged from the route-runner repair record below: school 21,033 ticks and 73 Sanity, school secret 21,270 and 77, school two 26,915 and 100, return 12,134 and 82, fortress 15,300 and 6,990, Beyond the Wall 24,484 and 47, village 13,987 and 100, Pandemonium 21,730 and skipped 21,115.
- Anode: not run in this stage. Recipe 13c (milestone `F1`) still owes `tools/test_visibility.ps1`, `tools/test_render_fx.ps1`, `--level-swap-check`, and the unfiltered `--save-check-write` then `--save-check-read` pair with all nine legacy PASS lines in the reader log.
- Limits: no visit is registered yet (only pre-seeded commented lines), so the registration path is proven with synthetic controllers and the real-data probe, not by a real visit, and `level.rs` and `levels/mod.rs` carry a module `allow(dead_code)` until the first registration. A save that names a controller the visit lacks, or omits one it has, is refused; the generic upgrade and the format bump are F2. wforest's two visits hash equally until something distinguishes them. This entry does not claim a whole-campaign run.
- Evidence: `C:/DEV/McGee/private/f1-registry-impl/round-1/` (logs, `build.json`, `summary.json`, the baseline `hashes.json`; the baseline is also at `C:/DEV/McGee/private/registry-baseline/hashes.json`).

## Native verification of the route-runner repair and the registry (2026-09-29)

Anode runs of the integrated build after the P0.8 repair and again after F1, each on its own copied exe. The P0.8 run is the first native proof that the unfiltered save reader now passes: it had stopped at the sixth legacy fixture at the baseline.

- Build: P0.8 run SHA-256 `E8EA72B3BA47B754C18144C0C0F81B8CD5F36746A06762E3A0877FEE7A596B61` (integration head `801f059`, docs-only commits after it do not change it); F1 run SHA-256 `2E0CE2CE48542B24EA8E39AB93590700E9F590E691BA2D53174DCC1DDFDA01A` (registry merge `a6735e3`). Both are copied candidates, not the launcher exe, and unchanged before and after every batch. Unit tests: 342 pass at the P0.8 head, 369 at the F1 merge. Clippy and fmt clean.
- Headless: the integrator's Appendix E-1 matrix (59 flags including `--save-legacy-check`) exits 0 at both merges; F1 also passes `--registry-check` (39 fresh-visit snapshots reproduce the F1.0 baseline byte for byte; the legacy-precedence probe passes on the 10 legacy-owned visits).
- Chain: none. `--campaign-route-check` does not exist yet (F3).
- Anode (seat 0.11.0, session 4, 1280x720 at 100%):
  - P0.8 build: `--save-check-write` (writer pid 44708, exit 0, 59 cases) then a separate `--save-check-read` (reader pid 12364, exit 0) with all nine legacy PASS lines, 59 restarted cases and the final restart PASS; `--save-preview-check`; `--dice-render-check` (21 captures) and `--dice-watch-render-check` (17 of 17); `tools/test_visibility.ps1`, `tools/test_render_fx.ps1`, `--level-swap-check`, `--visibility-check`; `--blade-cards-render-check`, `--npc-placement-render-check`, `--animation-runtime-render`, `--facial-render`, `--presentation-render`; a 900-frame skool1 `--new-game` smoke run. All exit 0.
  - F1 build: `tools/test_visibility.ps1` (116 cases), `tools/test_render_fx.ps1` (4 checks), `--level-swap-check` (78 full level replacements), `--visibility-check` (116), `--render-check` (8), `--fidelity-corpus-check` (38); `--save-check-write` (writer 65936) and a separate `--save-check-read` (reader 35116) with all nine legacy PASS lines, 59 restarted cases and the final PASS; `--save-preview-check`; 900-frame skool1 and gvillage smoke runs; the school, pandemonium, fortress, beyond and pool render checks and `--duchess-render`. All exit 0.
  - Owned jobs exited 0; no owned windows remain; leases released.
- Anode reported no audio device; audible output not verified.
- Limits: exit codes and PASS lines are the claim; captures were kept but are not visually reviewed here. The `--registry-check` baseline hashes were captured by the F1 implementer on the pre-refactor commit, not from a separately built pre-F1 exe. No route runs across levels yet, and no whole-campaign run is claimed.
- Evidence: `private/P0-8-native/`, `private/F1-native/`, `private/f1-registry/`.

## Route-runner baseline repair (2026-09-29)

Seven headless route checks were RED on the campaign baseline (integration `b0792ee`: the user's checkpoint `73e4165` plus the weapons snapshot `593845c`). Building `73e4165` alone (a detached worktree with its own target directory) and running the same flags reproduced every failure with byte-identical output, and the four route checks that passed on the baseline (`--village-route-check`, both Pandemonium routes, `--school-return-check`) also pass, byte for byte, on both trees. **The snapshot introduced none of the failures.** All seven are route-driver failures that the Blade/Cards rules of 2026-09-28 exposed: a thrown Blade is 45 damage followed by 3.5 s with the Blade out of the hand, but the driver still threw at the nearest visible enemy (idle ones included), aimed with a windup allowance that the release-time aim no longer needs, waited in place for fixed times while Boojum waves shoved Alice about, and never picked up what its kills dropped. No gameplay constant, save format or event rule changed; no route uses flight, warps, recovery, refills or console commands. The driver now lives in `src/route/fight.rs` (policy and tests) and `Route::clear`, and is described in [COMBAT.md](COMBAT.md#route-driver).

| Check | At `73e4165` | Cause (measured with `LOOKING_GLASS_ROUTE_TRACE=1`) | Repair | Route metrics now |
| --- | --- | --- | --- | --- |
| `--school-route-check` | RED, died at (202.8, 2796.4, -288.0) | Seven 7-damage bolt hits (49) and the two library drops (23 + 21 fall damage, unchanged) left 7 Sanity at the recipe exit, where more Diamond bolts hit Alice as she stood through the exit wait. Kills came one per 3.9 s and were spent on idle guards. | `clear` after every waypoint: fights awake enemies, melees adjacent ones, collects essence, returns to the waypoint. | 21,033 ticks, 12 jumps, 3 climbs, 9 throws, 5 swings, 100 cards, 62 damage, **73 Sanity** |
| `--school-secret-check` | RED, died at (-2551.8, 2992.8, -480.0) | Alice waited 5 s and then shot the theatre face for up to 1,200 ticks while Boojum waves knocked her some 230 units north of the alcove (370 by the time she died). | Fight before and after the wait, and between shots at the switch (`shoot_switch` clears threats first). | 21,270 ticks, 14 jumps, 2 climbs, 12 throws, 7 swings, 87 cards, 66 damage, **77 Sanity** |
| `--school2-route-check` | RED, Boojum battle unfinished | Three 70-health Boojums need at least six Blade hits, which is more than 23 s at one throw per 3.9 s; the route waited a fixed 20 s and asserted. The route also stepped off the 440-unit balcony (42 Sanity) afterwards. | Fight until the quest leaves its battle stage (Cards while Will lasts), fight on the later legs, and retrace the bleachers instead of the balcony drop. | 26,915 ticks, 7 jumps, 8 throws, 4 swings, 117 cards, 70 damage, **100 Sanity**; battle segment 8,465 ticks, 58 damage taken |
| `--school-return-chain-check` | RED (depends on school two) | Same as school two. | None of its own. | School two as above, then the return: 12,134 ticks, 15 jumps, 21 throws, 18 damage, **82 Sanity** (100 Sanity, about 5 Will carried in) |
| `--fortress-route-check` | RED, no physics route to (-5, -2430, 32) from (-17.0, -2184.4, 51.1) | The Boojum's large waves (300 u/s impulse each) pushed Alice more than 500 units from her waiting spot during the 12 s room split, and two of the five throws in that time went to a guard that never attacked instead of the Boojum; the next planned leg started from the wrong place. | Fight before the wait and after each leg. | First visit 15,300 ticks, 6 jumps, 1 throw, 26 cards, 0 damage, **100 Sanity**; return 6,990 ticks, 2 jumps, 5 throws, 42 cards, 2 damage, **100 Sanity** |
| `--beyond-route-check` | RED, blocked toward (1926, 732) at (1582.6, 591.5, 0.03) | An enemy impulse (player velocity 72 then 424 u/s with no input) carried Alice off the rolling walkway's step into the fall-recovery teleports; the timed legs were then out of step. | Awake-only targeting with Cards; no other change to the route. | 24,484 ticks, 21 jumps, 9 throws, 38 cards, 28 damage, **47 Sanity**, three authored portals |
| `--duchess-check` | RED, boss not dead after 47 throws, 0 dodges | The driver subtracted 0.4 s of Alice's velocity from the aim, a windup allowance from when the aim was sampled at the click. Aim is now sampled at release, so throws were off by up to 128 units: boss health stayed 600 for the whole fight. | Aim at the boss, leading its flight; Cards and throws as the policy chooses. | 16,861 ticks, 20 throws, 38 cards, 55 damage, **45 Sanity**, 44 boss attacks, 13 dodges |

- Build: SHA-256 `f45dd8a3a9bddfe6b7829a14808a8dea15cbc1e22f0491df72efce213bf8794d` (copied candidate, not the launcher exe; the `73e4165` comparison build, rebuilt in a detached worktree with its own target directory and removed afterwards, is `8cc44e818a57bd00f23e117a0ee495a39cf623d882b34e6393697c64053720c9`). Unit tests: 334 pass (four new ones cover the throw lead against the real projectile code, the tracker, the toy choice and the ledge look-ahead; three come with the NPC-validation commit below). Clippy and fmt clean; only files this change touched were formatted.
- Headless: every existing Appendix E-1 flag passes (58 flags, all exit 0). `--animation-check` and `--npc-check` were the only other REDs on the baseline ("1 unexpected NPC validation failures", byte-identical at `73e4165`, an off-norm hand-tag rotation on `c_clockwork` that no route touches); this branch carries the repair from `system/p0-8-npc-models` as a cherry-pick (`-x`, source `48ca6f6`), so the route repair can be checked against a fully green matrix. The integrator's rebase drops the duplicate when that branch lands first. The village, both Pandemonium and school-return routes are byte-identical to `73e4165`. The docs of those four had drifted (village 9,880 ticks, Pandemonium 21,792 ticks and 100 Sanity); their current numbers are 13,987 ticks/100 Sanity, 21,730 ticks/57 Sanity, 21,115 ticks/55 Sanity and 12,134 ticks/82 Sanity, and the docs now say so.
- Chain: no campaign chain exists yet; `--school-return-chain-check` is the only carried-over route and passes.
- Anode: not run in this stage (headless only). Native steps listed for the 13c batch: smoke of the candidate, the two-process save check, and `--blade-cards-render-check`.
- Limits: these remain headless input simulations. The melee swing originates at the eye rather than the animated hand; Cards use the eye as origin like the Blade; drops heal Alice, as they do in the game; the routes take 21,000 to 27,000 ticks where the earlier ones took 14,000 to 21,000 because they now stop to fight. Boojums killed over a void drop their essence out of reach; school two's Sanity is restored by the essence that later fights drop within reach (it reads 100 by the end of the rescue).
- Evidence: `C:/DEV/McGee/private/p0-8-routes-impl/round-2/` (logs of all 58 flags, `build.json`, the `73e4165` comparison logs, `summary.json`); round 1 (`round-1/`) produced byte-identical route and baseline logs.

## Campaign baseline (2026-09-29)

Phase 0 baseline of the campaign integration branch, recorded before any repair (P0.8 has not run). Every existing static, headless and native Anode check was run once against a single hashed candidate exe, with no gameplay or route-runner changes. The baseline is triaged, not green: 9 of 57 headless flags and 1 of 49 native steps exit non-zero, and each is listed under Limits with the error it printed. This entry does not claim a whole-campaign run.

- Build: SHA-256 `7B15CA259FC8479911454458EB29BDA988234074C2647BA2A87AC20BC062EED7` (copied candidate `private/campaign-baseline-native/looking-glass.exe`, not the launcher exe; recomputed before and after every native batch and unchanged; the copy used for the static and headless runs, `private/campaign-baseline/looking-glass.exe`, has the same hash). The P0.6 build and the headless matrix record source commit `b0792ee`, and the native `build.json` records `82169d3`; the hash is identical, and `git diff b0792ee HEAD` touches only docs, `README.md` and `tools/provenance_check.py` (no source, `Cargo.toml` or `Cargo.lock`). Unit tests: 327 pass, 0 fail. `cargo fmt --check` exit 0 (no baseline formatting failures to list), clippy exit 0, release build exit 0 (`private/campaign-baseline/p06-exit-codes.json`).
- Headless: 48 of 57 flags exit 0 (`private/campaign-baseline/matrix-summary.json`); the 9 that exit 1 are listed under Limits. Route village (`--village-route-check`, opening to authored Pandemonium exit): 13987 ticks, 6 jumps, 2 throws, 0 combat damage, 100 Sanity at exit; the runner reports no flight, warps or recovery. Route Pandemonium watched (`--pandemonium-route-check`): 21730 ticks, 6 jumps, 11 throws, 1 authored teleport, Sanity 56.60045; skipped (`--pandemonium-skip-route-check`): 21115 ticks, 6 jumps, 11 throws, 1 authored teleport, Sanity 55.3338. These two runners print no combat damage and no no-flight/recovery statement, and the watched and skipped runs end with different ticks and Sanity, so this entry does not claim they commit equal state. `--school-return-check`: 12134 ticks, 15 jumps, 21 throws, 18 damage, 82 Sanity (the runner prints no teleport count).
- Chain: none. `--campaign-route-check`, `--campaign-strict`, `--campaign-graph-check`, `--level-spec-check`, `--scene-check`, `--enemy-check` and `--registry-check` do not exist in this source (no match in `src/main.rs`), so no New Game to frontier chain was run on Normal or Easy.
- Anode (seat 0.11.0, session 4, 1280x720 at 100%, capture probe OK, no input delivery exercised): 48 of 49 window steps exit 0 across seven batches. Every step used cwd the integration root, `--data C:/DEV/McGee/alice_202106/Alice1/bin/base` and an isolated `LOOKING_GLASS_SETTINGS_DIR`; direct exe steps also used `--no-audio` and an isolated `--save-dir` (the PowerShell scripts choose their own flags). Steps ran through `cmd.exe /c`, except suites steps 4 to 13, which launched the exe directly and worked.
  - Suites, 13 of 13: `tools/test_visibility.ps1` (116 cases), `tools/test_render_fx.ps1` (4 checks), `tools/test_sky_performance.ps1` (6 checks), `--level-swap-check` (81 PASS, 78 full level replacements), `--render-check` (8), `--fidelity-render-check` (22), `--fidelity-corpus-check` (38), `--render-fx-check` (6), `--billboard-check` (2), `--sky-performance-check` (10), `--decorations-render-check` (6), `--loot-render-check` (9), `--visibility-check` (116).
  - Characters, 5 of 5: `--animation-render`, `--animation-runtime-render`, `--presentation-render`, `--facial-render`, `--npc-placement-render-check`.
  - Toys, 6 of 6: `--blade-cards-render-check`, `--heavy-render-check`, `--ice-jacks-render-check`, `--mallet-jack-render-check`, `--dice-render-check`, `--dice-watch-render-check`.
  - Levels, 14 of 14: `--school-render-check`, `--school2-render-check`, `--school-return-render-check`, `--village-cinematic-render-check`, `--village-machinery-render-check`, `--progression-render-check`, `--pandemonium-render-check`, `--pandemonium-machinery-render-check`, `--fortress-render-check`, `--fortress-cinematic-render-check`, `--beyond-render-check`, `--pool-render-check`, `--ladybug-render-check`, `--duchess-render`.
  - Film, 2 of 2: `--movie opening` (about 126 s) and `--movie ending` (about 66 s) played through and exited by themselves.
  - Previews, 6 of 6, each `--frames 600`: hud, weapon, npc, combat, shelf, and `--story-preview Cat_Glass_Dialog` (map skool1).
  - Saves, 2 of 3: `--save-check-write` exit 0 (59 cases, writer pid 89060) and `--save-preview-check` exit 0; `--save-check-read` exit 1 (RED, see Limits). Writer and reader ran as separate processes. The reader's own pid was not recorded, and it failed before its per-case writer-pid comparison, so a different writer pid is not confirmed.
  - No `--start-at`, `--fly`, route-replay or normal-play run, so no native per-visit protocol was executed.
  - Owned jobs exited 0 except the save reader (exit 1, a genuine result); no owned windows remain; the lease was released after every batch.
- Anode reported no audio device (no isolated Remote Audio output); audible output not verified. Direct exe runs used `--no-audio` and no `seat_audio_*` check was possible, so soundtrack, voice timing and film soundtrack sync are unverified.
- Limits: this is a baseline of existing checks, all of which are staged fixtures or single-map runners; none is a continuous production-input route across levels. Exit codes and PASS lines are the result. Capture files were kept for every window step, but only the story and shelf preview captures were visually inspected in this pass (the five film frames were captured and noted in the film batch record). `--beyond-render-check` and `--pool-render-check` print capture lines and no PASS line; `--village-cinematic-render-check`, `--fortress-render-check` and `--npc-placement-render-check` print per-item PASS lines and no final line. Nine headless checks were RED at this baseline (all repaired afterwards by P0.8; see the route-runner repair and native verification entries above):
  - Named as expected in plan P0.8 (the plan attributes them to the 2026-09-28 Blade/Cards change, `docs/BLADE_CARDS.md:58`; this baseline did not re-derive that): `--school-route-check` printed "Route died at Vec3(202.80353, 2796.3992, -287.96875)"; `--school-secret-check` printed "Route died at Vec3(-2551.8054, 2992.8438, -479.96875)"; `--school2-route-check` and `--school-return-chain-check` both printed "Boojum battle did not finish Battle".
  - Not on the plan's expected list, cause not investigated here, for P0.8 triage: `--animation-check` and `--npc-check` both printed "1 unexpected NPC validation failures", from "DEFERRED NPC c_clockwork: Invalid packed bone quaternion"; `--duchess-check` printed "Boss did not die: 47 throws 0 dodges"; `--fortress-route-check` printed "No physics-input route to Vec3(-5.0, -2430.0, 32.0) from Vec3(-16.95531, -2184.3833, 51.104256) (100001 states)"; `--beyond-route-check` printed "Walk blocked toward Vec2(1926.0, 732.0) at Vec3(1582.5646, 591.4684, 0.031249523)".
  - Native RED: `--save-check-read` exit 1 after about 7 s with "Save check failed: Restored v0.20 Alice/projectile state differs", raised by the `school` case of the original v0.20 fixture (`private/dice-legacy-v2-school.json`, `src/save_check.rs`). Five of the nine legacy PASS lines printed (actual v0.26 temple, actual v0.24 airship, actual v0.22 Pandemonium, original v0.19 school, original v0.19 battle). The original v0.20 school save failed; the original v0.20 battle, actual v0.23 return and actual v0.21 Pool of Tears saves were not reached, and none of the 59 per-case restart checks ran. All nine fixture files exist in `private/`, so the missing lines are not skipped fixtures. Plan F2 needs all nine legacy PASS lines, so this stays open.
- Evidence: `private/campaign-baseline/` (static logs, `p06-exit-codes.json`, `matrix-summary.json`, per-flag headless logs, `anode.json`) and `private/campaign-baseline-native/` (`build.json`, `suites/`, `saves/` with `save-check-read.log` and `writer.pid`, `characters/`, `toys/`, `levels/` with 181 captures, `film/`, `previews/`, `availability/`).

## Shared materials, lighting and particles (2026-09-28)

The release candidate with SHA-256 `C703B6F65D6AF05D8ABDAFDEB3754E966DA579261DDC61BE5416C38D5A9474E2` passes `--render-fx-check`, `--fidelity-corpus-check` (all 36 maps with exact resident-scene restoration), the full `--visibility-check`, existing `--fidelity-render-check`, and a normal School view with NPCs and toy art. The same source passes 265 unit tests, strict all-target Clippy and formatting checks. The original-data audit finds 85 supported emitter definitions and reports the unsupported sparkle declaration. This verified binary was installed into the local launcher's `target/release/looking-glass.exe`; its predecessor was backed up and installation hashes recorded in `private/render-fx/installed.json`.

New GPU assertions cover three submission permutations of cross-category alpha composition, opaque depth, registered Add/Filter/Add model layers, additive sprite texture alpha, dynamic-light colour and models-only masking, projected shadow height and source flare occlusion. Particle tests cover named animation events, birth-space movement, exact paused samples, restored clocks, disable/expiry, and the combined 4,096-particle/pose bounds. Shared material resources remain alive until GPU flush.

The performance fixtures exposed a large Pool of Tears sorting/upload regression; compact geometry, compatible-run merging and conservative camera clipping brought the final queue-plus-light fixtures to 16.57–16.80 ms. These are display-paced synchronized samples, not isolated GPU costs or full-game frame-rate claims. Source shader/texture references and remaining approximations (including contact shadows, receiver occlusion and cosmetic particle reconstruction) are described in [RENDER_FX.md](RENDER_FX.md). Reports, captures and per-frame timings are in `private/render-fx/`; the native runner records the binary hash and every required check's exit code. No original executable or player save slot was used.

## Cutscene continuity and camera return (2026-09-28)

Village conversations now release Alice at the final authored position/facing, with checked floor support instead of restoring the pre-scene location. The introduction retains the end of her walk. The Blade scene's Rabbit shot is a cutaway: Alice remains at `alice_knife_node`, matching the original script's disabled warp. The first traversal attempt caught the incorrect alternative destination at the distant Gnome marker; the corrected route passes to the Pandemonium exit with 13,987 ticks, six jumps, four throws and 100 Sanity, without recovery or route warps. Pandemonium's cart handoff also retains its final 225-degree facing.

The shared camera return orbits around Alice over 0.45 seconds, checks the entire path and each subsequent movement against collision, preserves authored internal cuts, and uses a 0.25-second covered reveal for distant/blocked/skipped/first-person returns. Alice's actual cinematic local pose seeds the gameplay animation blend. Camera return freezes on pause and clears on load/travel/recovery/view changes; scene art is prepared before simulation/drawing resumes.

The validation snapshot passes 274 unit tests. The full GPU visibility suite passes in 64 seconds, including exact pose retention through serialization, all six Village endpoint comparisons, 18 replay cases / 597 poses, eight authored views, 216 rendered camera-return frames and the existing 41 post-scene checks. Intermediate camera captures were inspected; this caught and removed a straight-line return that moved too close to Alice. A separate 1,500-frame native viewer run completed the second Gnome conversation and returned to ordinary visible gameplay. That smoke run used isolated settings/saves and no audio; it does not claim audio or whole-campaign playthrough verification.

Installed executable SHA256: `B2CBAE7437C0150BFFAEBC615009112F519FC4A39EF7CA1B9070A16D5D106836`. Evidence: `private/transition-installed.json`, `private/transition-sources.json`, `private/transition-tests.log`, `private/transition-route.log`, `private/transition-visibility-summary.json`, `private/transition-visibility.log`, and `private/transition-live-gnome2.png`. The prior launcher is backed up privately. Strict Clippy on the concurrently edited shared source was blocked by other in-progress rendering/combat API changes; that unverified build was not installed. See [camera behavior](CAMERA.md) and [automated checks](VISIBILITY_TESTS.md).

The frozen snapshot also passes Clippy with only dead-code warnings allowed for the other agent's unfinished visibility/LOD metadata. A concurrent agent subsequently replaced the launcher with SHA256 BC55DBE6AE09FBEA286A644883B96AFF41DBCA10017ADD4927276AAFE4F5E78A; it contains the handoff regression and endpoint/pose guards, but the recorded native run above certifies the B2CBAE74 build. Later combined builds must rerun the same suite.

## Cutscene visibility and automated regression (2026-09-28)

The shared actor material now assigns all visibility/tint state per actor, keeps its raw uniforms private, and flushes nondefault effects before clearing them. Gameplay Alice, cinematic puppets, NPCs, props and menu actors use this contract. Intentional Cat fades and Glass ghosting are retained. A separate Village defect used the Rabbit cutaway camera for the third Gnome conversation and left Alice at the prior marker for its Cat shot; both the camera/hold and Alice's shot placement are corrected without changing save formats.

The new authored-camera regression failed before correction with zero Alice pixels. After correction it measured 11,704 pixels in the Gnome shot and 10,722 in the Cat shot. The suite runs six Village scenes naturally and at two skip points, samples 597 poses in 18 cases, verifies mid-scene restoration/paused clocks and visible gameplay handoff, and checks eight representative authored camera views. Forty-one additional native fixtures check player rendering after Fortress, Pandemonium, school-return and Duchess scenes. Actual Alice/puppet images must match clean baselines after deliberately stale dissolve, ghost and power state, through both immediate and queued rendering. It also caught an in-progress render-queue material-lifetime crash; queue resources now survive GPU flush.

Run [tools/test_visibility.ps1](../tools/test_visibility.ps1) before accepting actor/cutscene/renderer updates. It builds in an isolated directory, exits nonzero on failure, and records the executable hash/status in `private/visibility/run-summary.json`, output in `private/visibility/run.log`, and scene sample counts in `private/visibility/village-report.json`. No player saves/settings are modified. See [coverage and limits](VISIBILITY_TESTS.md). The fitted pose camera deliberately excludes authored cutaways; these checks do not replace camera choreography review, traversal or audio tests.

The combined release source snapshot passes 258 unit tests and strict all-target Clippy; source hashes are retained in `private/visibility-release-sources.json`, with build/test logs in `private/visibility-release-{build,tests,clippy}.log`. The full suite also passed in 59 seconds against the shared launcher build with SHA256 `1A8A3C5214535D3E1403A491394F587E81DF4D8BF16FC201E694802410DBE09D`; its report and log are preserved as `private/visibility-launcher-1A8A3C52-*` and `private/visibility-launcher-1A8A3C52.log`. Concurrent work subsequently rebuilt the launcher, so this result certifies that recorded binary rather than arbitrary later builds. Native checks use Anode's separate desktop and local original assets; originals and private test artifacts remain excluded from source distribution.

## School pickup feedback and floating pipe (2026-09-28)

The reported library-door save had full Sanity and Will. Both drops remain available under the original full-meter rule; touching a full resource pickup now gives one ordinary HUD notice per contact. `--items-check` passes both doorway locations against real school brushes, with direct/diagonal contact, each depleted meter, full-meter feedback and exactly-once collection after serialization. All 241 unit tests at this change, strict Clippy, formatting and the release build passed (`private/pickup-pipe-{tests,clippy,release,items}.log`).

Decoration filtering now follows included cinematic scripts. The school's initially hidden, actor-attached Gnome pipe no longer appears at its editor position. The all-map audit passes with 1,251 placements and 40 deferred scripted props; the additional exclusions are the school cinematic airship and a Garden 3 attached marble. No original asset files were changed.

Three isolated native Anode runs exited 0. A copy of the reported save staged at the drop retained both drops at 100/100 and displayed the full-meter notice; the same fixture at 60 Sanity collected only the touched drop and saved 75 Sanity/100 Will. The Gnome's theatre area was visually checked without the stray pipe. Captures: `private/pickup-pipe-full.png`, `private/pickup-pipe-collected.png`, `private/pickup-pipe-gnome.png`. Tests used separate saves/settings; player saves were read-only. These fixes apply when loading existing saves after relaunch.

## Liquid, running and Village entrance audio corrections (2026-09-28)

Continuous health loss no longer restarts the hurt animation every frame or duplicates its cry through two audio producers. Pain/choke/gasp/death occupy one voice with duration-aware spacing and priority; drowning does not insert ordinary pain between choking contacts. TIKI sound commands now belong to named performances, preventing `run_jump`'s cry from contaminating ordinary unarmed running. Legacy speaker paths normalize before deduplication, removing the second entrance forcefield while preserving all other Village sources and authored sound-manager settings. Damage rates and saves are unchanged.

The shared checkout passed 242 unit tests and strict Clippy. Original-asset checks validate all 26 locomotion aliases and 296 audio files. Actual Windows-device regression playback passed seven measured intervals (liquid, pause, drowning, surfacing, death, Village ambience, running), with no overlapping player reaction sinks or missing requests and peak 0.3109. The broader 14-interval combat/music/mute/pause output check also passed. Detailed scope and private evidence are recorded in [AUDIO.md](AUDIO.md).

## Village introduction and Pandemonium cinematic refinement (2026-09-28)

The village's six scenes pass 24 watched/skipped runs through original assets, dialogue durations, safe hand-off, pause and serialized continuation. The real input route completes the introduction, actual Blade pickup/Rabbit sequence, all four gnome conversations and the Pandemonium exit in 14,175 ticks, seven jumps, three throws, no combat damage and 100 Sanity. No flight, debug warp, recovery or resource refill is used. The shrinking Rabbit stays ahead of his authored camera, using original animation travel speed integrated over his decreasing size. Logs: `private/village-cinematic-final.log`, `private/village-route-final.log`.

Pandemonium's four cinematic outcome checks and transport/progression checks pass. Its normal input routes finish in 21,765 ticks watched and 21,150 skipped, both with six jumps, 18 throws, one authored return portal and 70.60045 Sanity. These input simulations include combat and do not claim a human playthrough. Tunnel-camera checks pass at 30/60/144 Hz with clear sightlines and a maximum authored node-boundary turn of 0.028 degrees. Evidence: `private/cinematic-final.log`, `private/pandemonium-final.log`, `private/pandemonium-route-final.log`, `private/pandemonium-skip-route-final.log`, `private/camera-final.log`.

Audio checks verify both track-collapse cue sequences fire once per event, the village fall and landing recordings contain nonzero decoded samples, and loading does not replay historical impacts. The corpus check decodes 296 referenced files, 38 music cues and 901 ambient placements; two pre-existing archive references remain absent. This pass does not claim new audible output-device verification. Evidence: `private/audio-final.log`.

The native save writer and reader pass all 56 staged cases in separate processes, including four new village cases: falling, Blade conversation, Rabbit shrinking and gnome conversation. Resources, inventory, saved phases and continued simulation match after restart. Existing legacy fixtures and atomic-write/damage checks also pass. The older Pool of Tears fixture now explicitly expects its existing transport migration to the safe entrance rather than incorrectly asserting an unchanged player position; resources and actor history remain checked. Private evidence: `private/save-check/*/{quick.json,future.json,writer.pid}`.

Native village and Pandemonium render fixtures complete successfully, using original textures/models and shared depth-tested materials. In the real viewer, a loaded falling-scene save pauses correctly; P resumes, holding Enter releases Alice at the playable opening, F5 saves the completed introduction and F9 restores it without replaying the scene or losing resources/selected weapon. The viewer closes cleanly. Private evidence: `private/village-cinema-live/quick.json`, `private/village-cinema/*.png`, `private/pand-*.png`, `private/village-cinema-render-final.log`, `private/pand-cinema-render-final.log`. Anode reported no audio output device for this session.

The final shared build passes all 235 unit tests, strict Clippy and formatting. The village legacy check also serializes an upgraded cached visit before loading it, ensuring an already collected Blade does not unexpectedly replay its new scene. Logs: `private/cinema-tests.log`, `private/cinema-clippy.log`, `private/cinema-format.log`. Release compilation records the exact source hashes before and after compilation in `private/village-release-sources.json`.

See [VILLAGE_CINEMATICS.md](VILLAGE_CINEMATICS.md) for restored choreography and remaining fidelity limits. Captures and save fixtures are staged presentation/persistence checks, separate from the input routes.

## Load/Save camera restoration (2026-09-28)

- The original six `ui/load/loadA`–`loadF` layers now mask the photographs correctly. The large preview uses all five authored sepia/scratch frames and the original destination-colour/source-alpha blend, plus the 180 ms shutter motion and 350/500 ms picture-change/reopen timing. Captures at `private/save-preview-menu-0.png` through `-2.png` were inspected; native input also showed the wooden camera shutters moving over its aperture. Original PCM projector/hover/click files were validated.
- `--save-preview-check` passes with exit code 0 through the updated renderer: real screenshots, overwrite/reopen, failed-write preservation, matching backup image/state, legacy and malformed-preview fallbacks. All 235 unit tests pass, including shutter timing/rapid selection and leap-day date formatting; strict Clippy, formatting and whitespace checks pass.
- Native input in Anode at 800×600 used only `private/save-menu-native` slots and `private/save-menu-settings`. Keyboard navigation opened Load/Save; mouse input selected thumbnails and the original brass controls. Empty Slot 3 saved successfully, cancelling its overwrite preserved the file's exact hash, and a confirmed second save while the menu remained open preserved the entire gameplay payload. Attempting to load empty Slot 4 left it empty and displayed an error; loading occupied Slot 1 restored the game and left it paused. Evidence: `private/save-menu-native/evidence.json` and `private/save-menu-native.log`. Existing 1200×680 staged captures verify widescreen proportions. The shared keyboard/controller action router is unchanged; this pass did not repeat the earlier virtual-controller tests.
- The live input run requested audio capture, but the hidden desktop reported `NoDevice`. Its UI checks completed and the viewer closed normally; the process then returned 1 because no audio output was available to capture. Audible playback is not claimed from that run. Menu cues use the existing mixer and honour effects volume/mute, while gameplay audio remains paused.
- The release executable was rebuilt successfully. Player saves/settings were not used, and the running game was not stopped. Existing format-11 saves remain compatible. See [MENUS.md](MENUS.md) for remaining save-browser differences.

## Original map decorations (2026-09-28)

- `--decorations-check` and the rebuilt release executable's same audit pass all 36 maps: 1,255 accepted environmental placements, including 1,184 previously omitted placements across 29 maps; six attach to room movers. Twenty-four compiled duplicates are skipped, 37 script-dependent candidates are deferred, and all 110 referenced images decode. See [DECORATIONS.md](DECORATIONS.md) for scope.
- Native Anode `--decorations-render-check` passes five staged before/after comparisons through the normal scene renderer: school chair/table, school-two classroom furniture, garden flower/vegetation, Hatter bust and attached Fortress lamps. All comparisons change over 12,000 pixels; ten private captures were produced and representative images inspected. The initial statue inspection camera was occluded by architecture, so the attachment fixture uses the visible hanging light instead. The fixtures are not traversal evidence.
- Native `--fidelity-corpus-check` loads and draws all 36 maps and preserves the resident school's exact rendered pixels across the other scene loads. Its five existing empty/`textures/notexture` map references remain fallback images; accepted decorative skins all resolve. Logs: `private/decorations-corpus.log`, `private/decorations-native.log`, `private/decorations-release-audit.log`.
- The release build starts normal school-two play, draws 89 environmental placements with zero fallback images and exits cleanly after 180 frames. This uses `private/decorations-smoke` rather than player saves and runs without audio. It is a startup/render smoke check, not a full playthrough.
- All 231 unit tests pass, strict Clippy passes, and formatting/diff-whitespace checks pass. New unit coverage includes source offsets/visibility, three-axis orientation, attachment transforms and spatial duplicate matching. No save schema changes are introduced by this restoration.

## Controls and manual saves (v0.32.0, 2026-09-28)

- 219 unit tests pass; strict Clippy, formatting and diff whitespace checks pass. New checks cover controller radial deadzones, analog magnitude, trigger/button edges, held-input suppression, menu repeat, eight-key preference migration, binding swaps and safe, separate slot paths.
- Native Anode sessions used `private/controls-settings`, `private/controls-native` and copies of the launcher. A virtual Xbox 360 pad was isolated with HidHide (`seatOnly`, verified from the parent desktop). No player's settings or saves were changed. Tests ran without audio; no additional audio verification is claimed here.
- Controller Start/B opened and closed the original menus; D-pad/A navigated Main, Settings, Video and Load/Save. Video changes applied through the controller. Keyboard and mouse navigated the same settings and save screens. Controls pages, controller calibration, inventory selection/return and chapter navigation were exercised.
- Native bindings: Forward W → T, Interact E → middle mouse, controller Jump A → X (swapping Interact to A). Apply preserved them in the settings file and a later game process loaded those settings. W no longer moved Alice; T moved her about 71 units. Analog movement and right-stick look changed saved position/yaw/pitch; X produced one jump. Controller primary and alternate Blade attacks each produced the correct saved attack type and an emitted shot; a mouse primary attack also produced its action and shot after switching back. Controller bumpers and inventory changed the selected owned toy.
- Empty Slot 1 was created with the controller; Slots 2 and 4 were created with the mouse. Re-saving an occupied slot showed a confirmation, which B cancelled. Loading empty Slot 4 gave a message while preserving play; Save then created it successfully. Quick save and automatic save remained separate. Loading Slot 2 in a new process restored every pre-existing gameplay field exactly; the simultaneous acting upgrade added only its default empty `character.acting` state. A separate manual-menu load also restored a save successfully. Evidence: `private/controls-native/restart-comparison.json` and the saved snapshots.
- Attaching during play worked after controller discovery; detaching an active pad paused play with a disconnect notice. Keyboard P resumed and mouse combat worked afterwards. Raw-state unit tests additionally cover held buttons/sticks on reconnect and focus changes. Physical Xbox hardware and native PlayStation/DirectInput devices were not tested; native PlayStation/DirectInput support is not implemented.
- The jump test exposed a pre-existing tangent collision stall at the school entrance pillar: Alice stayed in place while downward velocity grew beyond the save validator's limit. A checked one-skin separation now applies only to zero-distance tangent wall contacts. An asset-free synthetic regression reproduces that geometry and verifies landing, clear body placement and valid save state. Reloading the captured mid-air save in the native game now lands at Z -511.96875 with zero velocity and permits a new manual save. `private/controls-native/landing-fixed-evidence.json` records it.
- The normal first-school traversal passes (15,135 ticks, 16 jumps, one climb, 27 Sanity; combat enabled, no flight/warps/recovery). Both Fortress visits pass with route steering aligned to their authored portal and raised takeoff ledge; the route fixture no longer assumes an exact endpoint within its broader navigation tolerance. Logs: `private/controls-school-route.log`, `private/fortress-final-route.log`.


## 2026-09-28 - additional original animations

The rebuilt game passes strict Clippy, formatting and all 219 unit tests. `--animation-check` passes 48 Alice performance clips / 3,903 frames with original prop commands, ten ambient bindings, and all 63 eligible NPC models / 177 loaded clips. `--character-check` also passes all 222 Alice archive clips / 8,688 frames; `--weapon-check` passes all 13 shared prop models and 12 action clips. The facial regression still passes 366 lip tracks and 79 implemented story lines. See [ANIMATIONS.md](ANIMATIONS.md) for scope and unfinished contexts.

Native `--animation-render` completed with exit code 0 in Anode, producing 18 explicitly staged captures through the actual renderer. The capture set includes Alice's toy routines and dialogue/reaction poses, Gnome and Cheshire variants, the whole Hatter with his cane, the actual school `bookstacks` model, and moving watch hands. It also restores a character snapshot with the new performance field omitted, exercising the older-save default. Representative images were inspected; these are staged presentation checks, not a full playthrough.

Normal `skool1` and unarmed `gvillage` launches also completed and captured successfully using isolated saves under `private/animation-smoke-saves`; no player saves or original assets were edited. The school retains its pre-existing empty-name image fallback; the village reports none. Local evidence is under `private/animation-*`. The release executable used by the normal launchers was rebuilt.

## 2026-09-28 - Beyond the Wall main route

The subsequent Fortress sky / Pool transport pass is recorded in [FORTRESS.md](FORTRESS.md) and [POOL.md](POOL.md). Both Fortress routes pass against v0.32 movement: first visit 10 jumps/9 Blade throws/32 combat damage, return 2 jumps/17 throws/no combat damage, no flight or recovery. The route fixture now aims into the portal and aligns precisely on the raised takeoff ledge instead of depending on a 22-unit navigation tolerance. Native GPU checks verify sky occlusion, alpha cutouts, slime blending, fog neutrality and the new Fortress tower fade into the actual sky; exterior/schoolhouse/room captures confirm the result. Pool's four staged ride/contact/save fixtures pass, and native old-save imports work both before and after the Ladybug implementation. This is a transport milestone, not a verified full Pool playthrough. Strict Clippy and 219 current unit tests pass. Evidence: `private/fortress-final-route.log`, `private/pool-final-check.log`, `private/pool-final-unit.log`.

`fortress2` now completes from its ordinary entrance to `fortress1$fortress1_start2` with production movement, collision, combat and triggers. The permanent `--beyond-route-check` passes the flipping corridor, musical levers, rolling walkway, rising staircase, final shuffled door and exit: 20 jumps, 44 Blade throws, seven combat damage, 92 Sanity, exactly three authored portals, no flight/recovery/forced warps or resource refills. Evidence: `private/beyond-final-route.log`. Both Fortress routes still pass; the return now includes active Boojums (17 Blade throws).

`--beyond-check` passes the older generic-save upgrade, paused state, wrong/correct lever sequences and reset, walkway reversal/bounds, partial-motion JSON save restoration, final-door retries and exit gate. Strict Clippy and 205 current unit tests pass. Story checks cover 69 original voice/subtitle pairs across six maps, including both Beyond conversations. Difficulty-excluded contacts and separate C-summoned hint regions are not mistaken for automatic story triggers.

Four native staged views render without missing images. Native F5, separate-process loading and F9 restore succeed using isolated saves under `private/beyond-native-saves`. Anode had no audio device during that check; audible output is covered separately by the audio work. Cinematic cameras, background flying props, optional Rage Box staging and exact encounter presentation remain fidelity work. See [BEYOND.md](BEYOND.md). Each opening map now has a main-route check; this does not claim a single uninterrupted village-to-school campaign run.

## 2026-09-28 - facial animation

Restored original lip envelopes and Alice’s blink skin in the gameplay and puppet render paths. The release build, strict Clippy and 205 unit tests pass. `--facial-check` validates 366 original tracks / 42,378 samples, eight deforming rigs and 79 supported story lines. Native close-ups confirm mouth opening and Alice’s blink. Cheshire’s 38 hint regions / 40 recordings pass their existing check. The broader story trigger check now passes through both fortress maps after the Beyond integration. See [coverage, timing approximations and evidence](FACIAL.md).

## 2026-09-28 — v0.31.9 gameplay audio

Gameplay requests now use a bounded cache with on-demand resolution, restoring previously silent combat/quest sounds. Correct weapon impact clips, positional enemy/projectile cues, projectile flight loops, Alice animation-frame events, pain/water sounds and audible death animation are integrated. The village roller, school bookcases/spiral lift and Pandemonium minecart follow their saved motion clocks. The camera drives spatial mixing with wall obstruction, underwater filtering and restrained room reflections. Map music crossfades and all named music cue metadata are supported; unsupported encounters still need their own trigger logic. See [AUDIO.md](AUDIO.md) for exact scope and remaining fidelity limits.

204 unit tests, strict Clippy and release build pass. The archive-backed audio check verifies mover start/impact, loops and save restoration, then decodes 296 unique referenced files across 38 music cues and 901 ambient definitions. Two absent original references are reported (`icw_idle.wav`, `idle_move2.wav`); the tested gameplay requests have none missing.

The final normal-session Windows output check consumed 991,457 stereo frames (22.48 seconds) and resolved 81 one-shot events. Fourteen measured playback intervals passed, including combat, machinery, dialogue, crossfades, underwater sound and a loud stress mix. Mute/pause measured zero; peak was 0.9800. Evidence is in `private/gameplay-audio-{tests,clippy,build,audit,output}.log`, `private/gameplay-audio-mixing.json` and `private/audio-output-check.wav`. This verifies the device callback and application mix, not physical speaker volume or exact original-game audio parity.

Anode's separate session had no audio device. Its native school combat run reached death after nine guard attacks, captured `private/gameplay-audio-native.png` and exited cleanly. The lease was released. The native check and audible device check are separate. Original archives and user saves/settings were untouched; the native run used an isolated settings directory. Audio observers require no save-format change. The launch executable is 0.31.9, and the source-review archive excludes all game sounds and private recordings.

## 2026-09-28 - v0.31.8 village machinery and steam

The reported village sawmill used reversed pitch signs, separating its beam from the vertical slats. Its slats/beam and cam rotations now follow the reviewed original direction. Wall vent arms compose the local bend before the parent shaft spin. Three steam emitters retain their authored offsets from those moving arms, and emit during each two-turn stroke. Three mushroom emitters bind to their roofs with the original four-unit lowering and compression bursts; roof timing repeats after a single initial delay. The two puff-ball emitters remain at fixed outlets and use the ascent/raised emission intervals. New particle placement preserves per-entity tag offsets, saved enable flags and particles already released into world space. The steam definition's world-space rising velocity is retained.

192 unit tests, strict Clippy and the release build pass (`private/village-machinery-tests.log`, `private/village-machinery-clippy.log`, `private/village-machinery-build.log`). Added tests check vent-axis motion and continuity, sawmill tilt direction, phased emission, roof dwell/release and independent motion of released puffs. `--village-machinery-check` passes three vent joint/nozzle constraints and restoration of poses and eight emission controls at 14 phases (`private/village-machinery-state.log`). `--progression-check` retains paused and 30/60/144 Hz machinery behaviour, hatch damage gating and one-time guard activation (`private/village-machinery-progression.log`).

The input-simulated normal village route still reaches `pandemonium$player_start`, now in 10,068 ticks with six jumps, three throws, zero combat damage and 100 Sanity. It collects the hallway Blade and uses no cheats, direct state edits or recovery (`private/village-machinery-route.log`). This route proof is separate from native visual inspection.

Anode rendered the sawmill through both tilts and eight seconds of the staggered vent cycles, with 31 captures across sawmill, vents and roofs. Inspected images show the sawmill beam beneath its slats and steam originating at moving nozzle mouths rather than fixed points above the wall. Inspection cameras are checked against world collision. The staged native job exits successfully with no missing images; log `private/village-machinery-native.json`, images `private/village-machinery-*.png`. Sprite size/orientation, puff-ball rotation and mover sound loops retain documented fidelity limits; this is not a full original-game visual comparison.

The launcher uses 0.31.8. Player saves/settings and original archives were untouched. Village pose/clock serialization and save format remain unchanged; the previous executable/source package were backed up and the local source-review archive refreshed without game data or private captures. See [VILLAGE.md](VILLAGE.md).

## 2026-09-28 - v0.31.7 requested gameplay console commands

The requested command set is implemented: `god`, `noclip`, `notarget`, `wuss`, `give all`, `health`, `cg_cameradist`, `give <item.tik>` and `map`. The shared energy debit now honours God even at zero Will. Wuss grants all ten toys and three Dice without changing resources; Give All also refills resources and grants the original give-all breathing shell. The archive-backed resolver accepts 29 supported inventory filenames and reports unknown or unsupported models without changing state. School grants synchronize inventory while preserving unrelated quest stages. Notarget cancels new targeting/attacks in the implemented enemy systems, including Boojum pursuit and hostile summons; already released projectiles remain live. First-/third-person selection and positive camera distances use the existing camera and weapon presentation. Save format 11 retains notarget and accepts explicit health above 100; earlier formats remain readable.

188 unit tests, strict Clippy and the release build pass (`private/console-gameplay-tests.log`, `private/console-gameplay-clippy.log`, `private/console-gameplay-build.log`). `--console-check` passes original item grants, every declared primary/alternate energy cost under God, resource preservation, invalid-item atomicity, school reward gating, camera selection and serialized cheat flags (`private/console-gameplay-state.log`). Targeted unit tests exercise zero-energy God, disabling it, over-health pickups, invalid numbers, command parsing, Boojum target cancellation and hostile demon targeting.

Native Anode testing found and fixed reversed buffered character input in Macroquad 0.4.14. Retesting at 5–10 ms per character executes command names, arguments and quoted item paths in order. The real console successfully changes from gvillage to skool1, grants Cards by filename, rejects a missing item, toggles noclip and enemy targeting, and switches between both camera modes. After card attacks, Wuss leaves 37 Sanity and 90 Will intact; God preserves that Will through another attack. Give All restores 100/100. Saving 250 Sanity, all toys, God, notarget and first-person view, cleanly quitting and launching a second process with `--load quick` restores all those values. God/notarget can then be disabled; `health 0` and `health 100` kill and revive. A third-person screenshot shows Alice again after switching back. Both owned processes exit cleanly.

Native logs: `private/console-gameplay-native.json`; console capture: `private/screenshots/skool1-4412.png`. Saves and preferences were isolated under `private/console-gameplay-save` and `private/console-gameplay-settings`; player saves/settings were not changed. These are command and state checks, not a full campaign playthrough. Commands only operate on implemented gameplay systems; granting an unfinished toy does not implement its attacks. The command reference documents filename coverage and behaviour. Wider original engine commands/cvars are outside the user's clarified scope.

## Fortress of Doors playable visits

The current fortress update supersedes the historical dark-arrival blocker recorded below. Both continuous routes pass with ordinary movement, jumps, combat and authored triggers: first visit to `fortress2` (10 jumps, nine Blade throws, one portal, 36 combat damage), and return to `skool1$skool1_start1` (two jumps, 11 throws, no teleports or combat damage). Return traversal activates the Boojum/shutter event. Neither route uses flight, recovery or direct state edits. Separate fixtures pass first/return gating, pause, partial room-split save restoration and migration from the previous generic controller.

Native Anode captures confirm the playable arrival, reconstructed splitting room and animated sky artwork. The outer void uses the map's black materials; the sky portal belongs to the splitting room. The full airship cinematic, remaining prop/fulcrum fidelity remain unfinished; Beyond the Wall now has a verified main route (above); see [FORTRESS.md](FORTRESS.md). The isolated build passes 181 unit tests, formatting and strict Clippy. Combined-source verification is recorded in `private/fortress-combined-route.log`.


## 2026-09-28 - v0.31.6 Pandemonium wall machinery

Replaced the independent arm/leg/cam oscillators with the reviewed source movement declarations for 16 BSP parts. Widget children inherit their parent's transformed attachment offsets and counter-rotate; calf and foot joints compose through the thigh. The leg startup, right-side delay, four unequal strokes, vertical piston travel, fixed wheel axle and both cams' eight unequal strokes/start delays are restored. Steam emission is gated by each leg's downstroke without overriding saved entity activation or removing existing puffs. Poses and intervals derive from the existing saved age; save format is unchanged.

`private/machinery-state.log` records real-map checks for six linked joints, fixed axle positions and axes, unrotated eight-unit pistons, continuity across motion boundaries, and serialized phase restoration at 20 sampled ages. Existing pause, cart lift/ride/landing at 30/60/144 Hz, restart, pickups, doors and one-shot exit checks pass. `private/machinery-route.log` records the normal input-simulated route: 21,792 ticks, eight jumps, 18 throws, one return teleport and 70.6 remaining Sanity. This is separate from staged visual inspection.

Native Anode inspection renders seven seconds each of the widget, legs and cams, including gated steam, with nine captures per mechanism and no missing images. Logs are in `private/machinery-native-final.json`; captures use `private/pand-machinery-*.png`. Inspection cameras are checked against collision; the cam view also checks five clear sightlines around the wheel to avoid the foreground rail. Native views and joint checks confirm that parts remain joined and the arms reciprocate instead of spinning through the wheel. These staged cameras do not establish a human campaign playthrough. This fix covers Pandemonium's wall mechanisms; mover sound loops and other maps' unfinished mechanisms remain separate work.

184 unit tests, strict Clippy, formatting and the release build pass (`private/machinery-tests.log`, `private/machinery-clippy.log`, `private/machinery-build.log`). The normal launcher uses 0.31.6. Original files and player saves were untouched; the previous source package/executable were backed up locally. The refreshed source-review package excludes original data and private artifacts.

## 2026-09-28 - v0.31.5 visible Pandemonium slime

The map's three drawn slime surfaces use two original turbulent texture stages. Source-alpha/source-colour blending is now supported instead of dropping the base stage. Its RGB/alpha fog neutral and reusable pipeline indices were added while preserving geometry, damage, source colours, transparency rules and save format.

GPU readback checks verify the exact two-stage colour equation over black and coloured backgrounds at alpha 0, 64 and 255, plus fog neutrality, cutout boundaries, sky occlusion, transparent depth and first-person depth (`private/slime-render-final.json`). Native staged views inspect both the village basin and lower minecart basin (`private/fidelity-pandemonium.png`, `private/fidelity-pandemonium-lower.png`), with zero missing images in Pandemonium. Viewpoints are selected from actual liquid triangles with clear camera bodies and unobstructed sightlines. These are presentation fixtures, not campaign traversal proof. The school-one capture still reports its existing empty texture reference; it is unrelated to this material.

All three visible surfaces were probed against actual map liquid volumes; one second of normal movement updates caused slime damage under each. The wider swimming/controller checks also pass (`private/slime-liquid-check.log`). Full level replacement passed 78 times with old/new scene resources coexisting, plus 64 simultaneous character/prop material owners (`private/slime-level-swap.log`). The extra blend does not exhaust the pipeline limit.

181 unit tests, formatting, strict Clippy and the release build pass (`private/slime-tests.log`, `private/slime-clippy.log`, `private/slime-build.log`). The normal launcher uses 0.31.5; earlier in-use executables were retained without terminating the player's session. Original files and player save slots were untouched. The refreshed source-review archive excludes original data and private artifacts.

## 2026-09-28 - v0.31.4 cart wheels and hanging lanterns

The two BSP wheel assemblies now spin around their transverse local Y axle (the source script's pitch/rotateX convention), at one revolution per second. Their original offsets are transformed by the cart in every phase, including boarding/lift and its final downward pitch. Previously the X-axis rotation swung the paired discs through the bed, and lift/landing offsets did not consistently follow the parent. Wheel angle remains derived from the existing saved ride/cinematic clocks; save format is unchanged.

The floating minecart-route flames were BSP surfaces whose separately placed `lantern2` bodies were not being drawn. The existing environmental-model path now restores those original models and materials at their authored position/scale, with difficulty filtering and no duplication of baked static copies. Pandemonium gains 24 model surfaces and has zero missing images. The shared axial billboard renderer also preserves rectangular flame dimensions and the authored long axis rather than squashing the quad to a square or positioning it from texture coordinates. A regression covers orbiting/axial cameras, tilted axes, different vertex orders and offset UVs.

`--pandemonium-check` now validates real wheel vertices, fixed mounts, transverse positions/radii and shared visibility during the complete lift/ride/landing at 30/60/144 Hz. A rail-phase save/restore check also compares reconstructed machinery transforms. All pass (`private/cart-visual-state.log`). The normal entrance-to-exit route remains valid: 21,792 ticks, 8 jumps, 18 throws, one intended return teleport, 70.6 Sanity (`private/cart-visual-route.log`).

Native Anode checks exercised the actual boarding trigger through the ride/landing, pause/resume and a clean exit, with isolated preferences and no player-save writes (`private/cart-visual-native.json`). Staged visual checks cover lift, rail-start wheel angles, later track and landing, alongside the other cinematic fixtures. Captures include `private/pand-rail-start.png` and `private/pand-wheels.png`; these are visual fixtures rather than the entrance-to-exit proof. Formatting, 180 unit tests, strict Clippy and release build pass (`private/cart-visual-tests.log`, `private/cart-visual-clippy.log`, `private/cart-visual-build.log`). The normal launcher uses 0.31.4. The local source-review package excludes original game data and private files.

## 2026-09-28 - v0.31.3 wall and minecart camera, opening Blade

The third-person camera retracts safely around obstructions and eases back after a short clear interval. The abrupt overhead lift is removed. Step/landing height changes are damped with bounded lag; direct mouse aim is retained. Pandemonium's rail heading is continuous across authored nodes, and its camera checks clearance at 12 points over the next 0.6 seconds to ease inward ahead of tunnel walls. Original rail positions, timings and camera cuts are retained. Cinematic mouse/arrow input no longer accumulates a hidden view rotation. See `CAMERA.md`.

`--camera-check` passes 51,840 orbit samples across 144 supported positions in the village and two schools, checking both camera volume and sightline against world/dynamic collision. The live cart camera passes at 30/60/144 Hz (950/1,900/4,559 rail frames); largest relative camera steps are 22.93/12.60/4.95 units respectively, with at most 0.028 degrees difference across the sampled track-node boundaries. The initial unsmoothed tunnel retraction measured about 70 units in one 60 Hz frame. The final 12-sample version passes the bounded-change check at all three frame rates. Logs: `private/camera-opening-collision.log`.

The normal-input village route starts unarmed, touches and collects the authored hallway Blade, and reaches its exit: 10,068 ticks, 6 jumps, 3 throws, 100 Sanity, no flight/warps/recovery. The full Pandemonium route passes with its intended return teleport: 21,792 ticks, 8 jumps, 18 throws, 1 teleport, 70.6 Sanity. Watched/skipped cinematic progression, intermediate saved skips and pause checks pass; 36-map/39-visit loadout checks pass. Logs: `private/camera-opening-village.log`, `private/camera-pandemonium-route.log`, `private/camera-opening-cinematic.log`, `private/camera-opening-loadouts.log`.

Native Anode checks used separate settings and save folders. A new village game displayed empty hands and no selected-weapon HUD; its unarmed quick save restored after restarting, including first-person view, with no attacks emitted by attempted clicks. A staged hallway check then used ordinary forward input to collect the Blade and verified its HUD and first-person model. That run exited 0 (`private/camera-opening-pickup-native.json`, capture `private/screenshots/gvillage-2505.png`). Starting at the real cart boarding trigger, the native cart check was observed through boarding, tunnel travel and landing, including mouse movement during the ride; its owned test process later reached the harness's three-minute lifetime limit, rather than a clean application exit (`private/camera-minecart-native.json`, capture `private/screenshots/pandemonium-931.png`). Full entrance-to-exit coverage is provided by the separate normal-input route checks, not these staged visuals.

Formatting, **179 unit tests**, strict Clippy and the release build pass (`private/camera-opening-tests.log`, `private/camera-opening-clippy.log`, `private/camera-opening-build.log`). Tests cover camera collision/return hysteresis, corners, pause/steps/reset/frame rates, unarmed save/pickup compatibility, and first equip from empty hands. Campaign save format remains 10. Older saves retain their weapons, while an uncollected village Blade can be consumed once even if it was granted early by an older build. The normal launcher uses 0.31.3; in-use older executables were retained without stopping the player's session. The refreshed source-review package excludes original assets and private data.

## 2026-09-28 - v0.31.2 mouse chapter navigation

The Tab chooser accepts wheel scrolling, single-click selection, double-click activation, clickable Begin/Return actions and a clickable difficulty label. Keyboard selection, wrapping, Home/End, Enter and D remain available. Rows do not recenter on mouse selection; scrolling clamps to the list and keeps the selected visit visible. Windows wheel events are converted from raw 120-unit deltas to notches. The original parchment, fonts and arrow marker remain in use, with shared canvas coordinates for rendering and hit testing.

Anode native input verified 800x600 windowed and 1280x720 fullscreen layouts. Checks covered one-notch scrolling (three rows), both list ends, stable clicked rows, difficulty changes, Begin, double-click, Return, keyboard wrapping and loading. Mouse Begin loaded the school return with `skool1_start2` and Blade/Cards/Mallet/Dice; double-click loaded fortress1; keyboard Enter loaded the village. A save made while paused before browsing, scrolling, selecting and returning had an identical gameplay payload afterward (only `saved_at` changed). Both native runs exited 0 and reported zero emitted weapon actions. Settings and saves used isolated `private/chapter-mouse-*` directories.

Logs: `private/chapter-mouse-native.log`, `private/chapter-mouse-wide.log`; capture: `private/chapter-mouse-wide.png`; input-isolation comparison: `private/chapter-mouse-isolation.json`. Tests cover stable selection/double-click identity, bounded/fractional scrolling, short lists, keyboard selection visibility, scaled hit regions and clicks outside the centered canvas. Click geometry was tested at 800x600, 1280x720, 1080p, 1440p and 4K; physical native display testing was limited to the Anode desktop's 1280x720 resolution.

Formatting, **173 unit tests**, strict Clippy and the release build pass (`private/chapter-mouse-tests.log`, `private/chapter-mouse-clippy.log`, `private/chapter-mouse-build.log`). The ordinary launcher uses version 0.31.2. Campaign save format remains 10, and the source-review package excludes original assets and private files.

## 2026-09-28 - v0.31.1 actor grounding and facing

Generic walking actors now resolve their editor origins against solid world/dynamic support using scaled, original `setsize` bounds. Flyers/swimmers/zero-gravity actors keep their altitude. Club/Diamond guards settle before first presentation; later falls retain gravity. Original spawn identities stay unchanged, with compatible saved footing/fall fields. Dialogue presentation now turns visible cast toward Alice independently of paused combat. The second-school Gnome faces Alice rather than the camera, and faces the condenser while mixing; saved yaw and pause are checked. See `ACTOR_PLACEMENT.md` for scope and limitations.

The read-only 36-map audit found 339 eligible grounded declarations with safe support (255 corrections over one unit), 128 declared airborne/swimming cases, and 33 placements without safe support. The latter are reported rather than force-relocated; counts include legacy declarations owned by dedicated scripted controllers. Log: `private/actor-placement-audit.log`. Native staged close-ups verified village `torchgnome1` (24 units lower) and `torchgnome3` (8 lower), `centipede1/ant_runner1` (40 lower), `centipede2/ant_guard2` (49 lower), and the first-school child (8 lower), with correct facing and preserved snapshot identity. Captures: `private/actor-gvillage-torchgnome1.png`, `private/actor-gvillage-torchgnome3.png`, `private/actor-centipede1-ant_runner1.png`, `private/actor-centipede2-ant_guard2.png`, `private/actor-skool1-return_insane2.png`. Other actors are hidden only in these inspection fixtures. A normal village launch also exercised the opening Cat dialogue (`private/actor-placement-village.log`). These are placement checks, not complete Centipede routes.

The village, first school, second school and Pandemonium normal-input route checks pass after guard placement changed. Logs: `private/actor-village-route.log`, `private/actor-school-route.log`, `private/actor-school2-route.log`, `private/actor-pandemonium-route.log`. The school-two component check also verifies support, dialogue/mixing directions, pause and saved facing (`private/actor-school2-check.log`).

Separate native writer/reader processes pass all **52 save fixtures** and the existing legacy/malformed-save checks (`private/actor-save-write.log`, `private/actor-save-read.log`); player save slots were not used. Release build, formatting, **169 unit tests** and strict Clippy pass (`private/actor-placement-build.log`, `private/actor-placement-tests.log`, `private/actor-placement-clippy.log`). The ordinary launcher has the rebuilt 0.31.1 executable. An in-use earlier executable was retained under a different filename without terminating the player's process. Save format remains 10; the source-review package excludes original data and private captures.

## 2026-09-28 - v0.31 original-art interface

Original archive fonts and decorative art now cover startup, menus, confirmations, inventory, chapters, help, pause, dialogue, notices, the console and combat overlays. The HUD uses authored bar/back/riser and folding TAN geometry/UVs; partial resource fills select/interpolate the original riser frames. Shared UI textures use existing rendering pipelines. Details and fidelity boundaries are in `UI_FIDELITY.md`.

Anode native input checked 1280x720 fullscreen and 800x600 windowed layouts, settings pages, original cursor, owned-weapon mouse selection, chapter/help overlays, pause, console input, and Cheshire subtitles. A quick save was made using the original camera button and loaded through the confirmation flow; the level reload completed without pipeline exhaustion. A new process displayed the restyled Continue screen and restored the automatic save. Tests used `private/ui-fidelity-saves` and `private/ui-fidelity-settings`, leaving player slots/preferences untouched. A separate `--hud-preview --frames 120` process captured partial Sanity/Will and exited 0.

Logs: `private/ui-fidelity-native.log`, `private/ui-fidelity-final-native.log`, `private/ui-fidelity-small.log`, `private/ui-fidelity-hud-preview.log`. Retained captures: `private/ui-controls.png`, `private/ui-video.png`, `private/ui-loadsave.png`, `private/ui-chapters.png`, `private/ui-inventory.png`, `private/ui-help.png`, `private/ui-dialogue.png`, `private/ui-console.png`, `private/ui-hud-partial.png`. The test desktop is 1280x720 at 100% DPI; physical 4K/non-100% DPI rendering is not verified. Save-browser thumbnails/deletion, HUD folding transitions, layered liquid effects and exact original perspective/timing remain unfinished.

Formatting, release build, **164 unit tests** and strict Clippy pass (`private/ui-fidelity-build-final.log`, `private/ui-fidelity-tests-final.log`, `private/ui-fidelity-clippy-final.log`). During the final build the player opened the earlier 0.31 executable. Its in-use file was renamed within `target/release` and retained, allowing the finished build to be installed at the normal launcher path without ending the player's session. No original executable/DLL was run. The source-review package includes no original assets or private data.

## 2026-09-28 - v0.30.2 modern display options

Video now offers 1920x1080, 2560x1440 (QHD/2K) and 3840x2160 (4K), appending to the existing saved size indices. Original parchment, Asrafel font, Video title, mirror and Apply/Cancel artwork surround separate Window Resolution and Display Mode selectors. Left/right mouse arrows and keyboard arrows select values. Fullscreen is explicitly borderless at the desktop resolution; Windowed restores its selected pixel size. High-DPI rendering is enabled and window requests use physical pixels rather than DPI-scaled logical dimensions.

Real Anode input selected and applied each new size, toggled fullscreen, cancelled an unapplied edit, exited and restarted. The new process restored fullscreen and the 4K window preference. Switching back to Windowed at 800x600 produced an 800x600 client (816x639 including the frame), with all controls visible. Mouse input in both directions and keyboard wrapping were checked, as were 4:3 and widescreen menu layouts. Native logs: `private/display-native.log`, `private/display-restart.log`; retained captures: `private/display-video.png`, `private/display-windowed.png`. Preferences and save data stayed in isolated `private/display-settings` and `private/display-saves` directories.

The Anode desktop is 1280x720 at 100% scaling. Windows constrained larger window requests to that desktop's maximum window bounds; fullscreen used the desktop's 1280x720 resolution. This verifies selectable/persisted modern sizes and the mode/UI paths, not actual 1080p/1440p/4K output on matching monitors or non-100% desktop DPI. No desktop display settings were changed.

Release build, formatting, **164 unit tests** and strict Clippy pass (`private/display-build.log`, `private/display-tests.log`, `private/display-clippy.log`). The ordinary `Launch.cmd` uses the rebuilt 0.30.2 executable. Existing preference indices remain compatible; campaign save format stays 10. The source-review package excludes all original art and private files.

## 2026-09-28 - v0.30.1 story-order level selector

The Tab selector uses the existing 39-visit campaign route, with numbered level names and map IDs. Fortress, school and forest returns are separate rows 5, 8 and 28, selecting their named entrances and corresponding starting loadouts. Reopening the chooser highlights the current visit. Home/End jumps to first/last while the chooser is open; ordinary Home entrance recovery remains available outside it. Extra archive maps, if any, follow the story entries.

Real Anode keyboard input loaded all three returns, confirming `fortress1_start2`, `skool1_start2` and `wforest_start2` in the runtime logs with their expected weapon inventories. Reopening the selector kept the school and forest return rows selected. Home, End and wrapping in both directions were checked. Final native screenshots confirm readable punctuation, the opening sequence, separate return rows and the finale at 39. Logs: `private/story-selector-native.log`, `private/story-selector-final-native.log`; capture: `private/story-selector.png`. These checks verify the selector and existing entry/loadout behavior, not new traversal completion.

Release build, formatting, **164 unit tests**, strict Clippy and the all-map/visit loadout audit pass (`private/story-selector-build-final.log`, `story-selector-tests.log`, `story-selector-clippy.log`, `story-selector-loadouts.log`). The first build encountered the player's running executable lock, so initial UI checks used a separately staged binary. Once the original process exited, the final normal release build succeeded and the final native check ran through the unchanged `Launch.cmd`. Player processes were not stopped. Test settings and saves used isolated `private/story-selector-*` directories; save format remains 10. Local source packaging excludes original assets and private data.

## 2026-09-28 - v0.30 original Escape menus

Original archive main/settings/new-game/load-save/quit/credits layouts and textures are connected to explicit Rust actions. Main-menu hit regions, bitmap lettering, selected highlights and Alice's animated Settings reflection are restored. Options support display size/fullscreen, sound, mouse sensitivity/inversion, collision-aware camera distance, always-run, subtitles and eight remappable keyboard actions. Apply persists preferences separately from save data; Escape/Cancel discards staged edits. See [MENUS.md](MENUS.md) for controls and remaining differences from the original engine.

Real Anode input verified Escape opening/resuming, nested back navigation, original quit confirmation, credits paging, mouse volume adjustment, keyboard settings changes, Apply/Cancel, key capture, fullscreen and resizing to an 800x600 client. A separate process restart restored fullscreen, 30/100 music volume, 1.1 sensitivity, inverted look and the J forward binding. J then moved Alice **118.125 units**, and exiting recorded **zero weapon attacks** from menu input. Logs: `private/menu-native-final.log`, `menu-restart.log`, `menu-visual-final.log`; captures: `private/menu-main.png`, `menu-settings.png`, `menu-loadsave.png`, `menu-credits.png`.

Two confirmed quick saves taken while browsing the modal menus had **exactly identical full gameplay payloads**, covering actor/player/resource state, quests, campaign data and clocks (`private/menu-pause-check.log`). Loading that save through the menu restored the school and left play paused. New Game / Easy started the village with fresh campaign state and Easy difficulty, automatically preserving the prior session first; its quick slot remained available. Save/load and the school-to-village change completed with menu resources resident and no pipeline-limit panic. These are UI/state checks, not new campaign traversal claims.

Visual inspection caught and corrected the mirror's target aspect ratio and missing depth attachment, overlapping control hints, and an original disk-full widget that must remain hidden outside its error path. Original-data comparisons use the local layouts/artwork, not an original-executable run. Native checks used `private/menu-native` and `LOOKING_GLASS_SETTINGS_DIR=private/menu-test-settings`; the player's saves and preferences were untouched.

Release build, formatting, **164 unit tests** and strict Clippy pass (`private/menu-build.log`, `menu-unit-tests.log`, `menu-clippy.log`). Save format remains 10. Existing launchers point to the rebuilt 0.30.0 executable. The local source-review package excludes original assets, captures, preferences and saves.

## 2026-09-28 â€” v0.29.1 level-change resource fix

The level chooser retained lazy art from previously visited maps, unlike campaign exits and quickload. Actor/prop pipelines were also duplicated for every art owner even after v0.29 shared the world pipelines. Map art is now one replaceable group on all switch paths, and actors/props share a weakly cached shader pipeline. Tab/console map loads now use the same complete level loader as campaign transitions, preparing the incoming scene before replacing the current one. Save format remains 10.

Native `--level-swap-check` passes **78 complete replacements**: all 36 maps plus three selectable return visits, twice. It retains the old world, Alice, NPCs, particles and map art while constructing the incoming equivalents, and includes HUD, Cheshire and power-up resources. Every resulting scene is drawn. A separate GPU check verifies that 64 actor/prop material owners share one pipeline and that Alice's power tint does not affect a subsequent ordinary draw. Log: `private/swap-stress.log`. The v0.29 corpus check covered world rendering but missed this full resource graph.

Real Anode keyboard input switched first school â†’ second school â†’ first-school return â†’ Duchess through Tab. C summoned Cheshire; F5/F9 restored both the school return with its hint and the Duchess visit, and the game exited normally. Logs: `private/swap-after.log`; saves stayed in `private/swap-after`. This verifies the ordinary UI/loading paths, not traversal of those maps. The retained v0.29 binary was also inspected through several changes; that limited manual sequence did not reproduce the reported panic, so no exact original failure sequence is claimed.

Release build, formatting, **159 unit tests** and strict Clippy pass (`private/swap-build.log`, `swap-tests.log`, `swap-clippy.log`). A separate native reader also passes all **52 retained restart fixtures** with exact state/continued simulation and actual legacy migrations (`private/swap-save-read.log`). The launchers use the rebuilt **0.29.1** executable. User save slots were untouched; source packaging continues to exclude original assets, research, saves and binaries.

## 2026-09-28 â€” v0.29 rendering fidelity

Restored original portal viewpoints/sky meshes, reflective and layered materials, authored alpha tests, selected visual deformations, garden/chess waterfall meshes, oriented multi-emitter environmental particles and blend-neutral fog. References were the local original shader/model/map declarations; no original executable or DLL was run and no original-renderer screenshot comparison is claimed. [RENDERING.md](RENDERING.md) records coverage and remaining approximations.

Release build, formatting, **159 unit tests** and strict Clippy pass (`private/fidelity-build-final.log`, `fidelity-tests-final.log`, `fidelity-clippy-final.log`). The headless reference audit reports **26 sky maps, 19 unambiguous portal viewpoints, 38 reflective map-material references and 22 supported emitter definitions** (`private/fidelity-audit-final.log`). The existing all-map world/interaction check also passes, including school entrance doors and both steam-floor variants at 30/60/144 Hz (`private/fidelity-world-final.log`). Rendering deformation does not change collision.

Native Anode GPU checks pass exact GT0/GE128/LT128 cutout boundaries, additive/filter/double-filter fog neutrality, sky-opening depth, all six existing transparent blend wall-occlusion cases, read-only transparent depth and the first-person depth layer. Six staged map views were inspected, including stars through school observatory glazing, the Wonderland Woods sky, Pool of Tears water and a clear garden-waterfall camera with spray. The garden runtime check also passes pause, saved emitter disable/re-enable, selective steam shutdown and bounded lifetime/population. Log: `private/fidelity-render-final.log`; captures: `private/fidelity-{skool1,skool2,garden1,potears1,gvillage,wforest}.png`. Staged cameras are separate from traversal proof.

All **36 maps load and draw natively**, with a second school scene kept alive. Its captured pixels remain exactly identical after drawing each other map, checking shared pipeline reuse and per-scene lightmap rebinding (`private/fidelity-corpus-final.log`). This caught and fixed a case-sensitive chess-waterfall shader lookup. Remaining fallbacks are an empty material reference in fortress1/rchess1/skool1/utemple and the source `textures/notexture` reference in hatter1.

The initial restart check exposed Macroquad's 32-pipeline limit when rich materials were allocated separately for simultaneous scenes. World pipelines are now shared through weak references, and equal-depth variants are allocated only when used. After this fix, a separate native reader passes all **52 retained restart fixtures** with exact saved state and deterministic continued simulation, plus the existing actual legacy migrations (`private/fidelity-save-read.log`). Save format remains 10. Cosmetic particles are rebuilt; shared activation and environmental clocks retain their existing saved behavior. Player save slots were not used.

A native 120-frame launch of the ordinary first school also renders Alice, NPCs and the HUD and exits cleanly, using an isolated save directory (`private/fidelity-native.log`, `private/fidelity-playable.png`). This is a launch/render smoke test, not a new full-level traversal.

The release executable used by the existing launchers is rebuilt, and the 116-file source-review archive excludes original assets, extracted references, captures, saves and binaries. Nothing was published.

## 2026-09-28 â€” v0.28 items and difficulty

Rage, Tea and Glass use recovered duration/difficulty rules, exclusive activation and original item models. The Watch has its original cost, stop/recharge clocks and action assets. Supported deaths produce persistent, decaying Meta Essence. Difficulty filters both supported pickups and enemies. Read-only archive/Ghidra research stays private; [ITEMS.md](ITEMS.md) records behavior and presentation limits.

Release build, formatting, **156 unit tests** and strict Clippy pass (`private/items-build-final.log`, `items-tests-final.log`, `items-clippy-final.log`). `--items-check` audits all 36 maps: **338 / 314 / 293 / 293** supported pickups on Easy/Normal/Hard/Nightmare, including Normal's 13 Rage, 4 Tea and 6 Glass plus the separately gated school secret. Actual second-school mover/enemy snapshots remain unchanged during Watch time at 30/60/144 Hz, frozen enemies can take damage, and simulation resumes after saved expiry (`private/items-check-final.log`). Tests cover Tea speed/jump/body clearance, damage multipliers, power exclusivity/expiry/death, Watch costs/repetition/retry/recharge, Will recovery caps, drop collection/occlusion/decay and summon/projectile freezing across save serialization.

Separate Anode writer/reader processes pass **52 staged restart cases** with exact continued simulation, nine new cases covering each temporary power, active/expiring/recharging Watch and live/collected/expired essence (`private/items-save-write.log`, `items-save-read.log`). Actual legacy save migrations pass while retaining existing actors' health and state; newly supported difficulty-specific actors can be added. Native keyboard/mouse testing in isolated save folders confirmed Rage attachments/tint/countdown and Watch activation reducing Will from 100 to 99. A separate process loaded the Hard game without a difficulty argument and a paused resave retained Hard and the exact stopped/recharge values. These fixtures used staged inventory; they do not establish a full campaign playthrough. User saves were untouched, and owned desktop windows were closed.

Normal-input regression routes pass: village **10,068 ticks / 100 Sanity**, first school **15,039 / 27**, second school **21,119 / 100**, Pandemonium with skipped scenes **21,049 / 70.60045**, and Duchess **9,571 / 75**. The additional health in school two comes from ordinary collected enemy rewards. Logs: `private/items-{village-route,school-route,school2-route,pand-route,duchess-route}.log`. The school-two-to-return chain also passes, preserving inventory and completing the return in **12,160 ticks / 82 Sanity** (`private/items-return-chain.log`). Its route runner now accepts the potion's authored control hand-off before its displacement retry logic, rather than trying to keep walking during the drinking sequence.

`--weapon-check` validates 13 prop models, 6,849 vertices, **12** Alice action clips and three weapon attachment tags (`private/items-weapon-check.log`). Audio validation fully decodes **186** referenced files including the new power/Watch/drop sounds; the preexisting missing `sound/icewand/icw_idle.wav` remains (`private/items-audio-check.log`). No audible-device claim is made for the hidden desktop. Source packaging excludes original game data, binaries, research, saves and captures.

## 2026-09-28 â€” v0.27 environmental movement

Restored shared authored currents, target launches, updrafts, five/twenty-second air with drowning, campaign rope controls and supported enemy knockback. Read-only local Ghidra analysis established the force response mask, launch/air rules and updraft gravity; original model definitions supply selected impact strengths. Build, formatting, **147 unit tests** and strict Clippy pass. See [TRAVERSAL.md](TRAVERSAL.md) for controls, source evidence, approximations and the map-specific forces whose owning sequences remain unfinished.

`--traversal-check` parses all 36 maps (156 player force volumes, 27 ropes), compares live current/updraft behavior at 30/60/144 Hz with continuous body clearance, restores disabled/enabled force state, and exercises saved rope climb/swing on six real maps. The isolated swimming/bank check passes separately. Logs: `private/traversal-check-final.log`, `private/environment-swim.log`, `private/environment-{unit,clippy,release}-final.log`.

Normal-input regression routes pass: village **10,068 ticks / 100 Sanity**; first school **15,039 / 27**; second school **21,119 / 30** with all quest rewards and gated return; Pandemonium skipped-cinematic route **21,049 / 69.33**; Duchess **9,571 / 75** through the temple reward transition. Route planning now responds to live knockback by replanning ordinary inputs, and the second-school runner reapproaches the lollipop after its greenhouse ambush. Logs: `private/environment-village-final.log`, `private/environment-school.log`, `private/environment-school2-final.log`, `private/environment-pand-final.log`, `private/environment-duchess.log`. The Pandemonium mechanics check also verifies that an enemy impulse releases its preexisting rope controller without swallowing momentum (`private/environment-pand-mechanics-final.log`). These are headless automated routes, not human playthroughs.

Native Anode testing used isolated staged saves. Real keyboard input resumed a nearly exhausted air meter through drowning/death, retried, and saved fresh air while retaining the upgrade. On a held rope, Space raised Alice, Ctrl lowered her, WASD swung, and E released with momentum; saved positions and grip state confirmed each change. The final visual pass exposed invisible climb volumes; these now receive a textured vine mesh with ordinary world depth/fog and swing transforms. Original-art captures remain private (`private/screenshots/garden1-1304.png`, `garden1-2462.png`, `potears1-3778.png`, `potears1-7822.png`, `potears1-8847.png`). Grasp/animation alignment remains approximate.

Separate native writer/reader processes pass **43 save cases** and deterministic continuation, with five new movement cases. An actual format-8 temple save receives its twenty-second breathing upgrade and retains inventory through resaving; older retained fixtures also load. Logs: `private/movement-save-write.log`, `private/movement-save-read.log`. The native GPU occlusion regression passes (`private/environment-render-final.log`). Audio validation decodes **181 files**; the one existing missing Ice Wand idle reference remains (`private/environment-audio-final.log`). The hidden desktop has no audio device, so there is no new audible playback claim. User save slots were untouched; source packaging excludes original assets, research, binaries and captures.

## 2026-09-28 â€” v0.26 Duchess

Implemented the first campaign boss in potears3. Normal input route passes at **9,571 ticks, 26 throws, 75 Sanity**, with six dodges and 11 boss attacks; the shell is returned and the gated utemple transition emitted. Full-duration and skipped scenes produce matching arena/reward outcomes. Early exit, possession-only activation, pause, save reconstruction and 30/60/144 Hz checks pass. See [DUCHESS.md](DUCHESS.md) for mechanics and scope limits. Logs: `private/duchess-check-final.log`, `private/duchess-release.log`, `private/duchess-unit.log` (139 passed), `private/duchess-clippy-final.log`.

Native Anode screenshots inspected the encounter and ten staged scenes. Separate writer/reader processes passed 38 persistent-save cases; real keyboard input skipped the intro into live combat and saved, then skipped the reward scene into an actual utemple load/autosave with inventory retained. Logs: `private/duchess-render-final.log`, `private/duchess-save-{write,read}.log`, `private/duchess-native-{intro,reward}.log`. No new audible-device verification; this seat reports NoDevice.

School route (14,822 ticks/27 Sanity) and Pandemonium skip route (21,049 ticks/100 Sanity) remain unchanged. Logs: `private/duchess-{school,pand}-regression.log`. User saves were not used for testing.

## 2026-09-28 â€” v0.25 Pandemonium cinematics

Restored the four main cinematic sequences with archived camera tracks, explicit actor choreography and one-shot completion shared by playback/skipping. Release build, **133 unit tests**, and `cargo clippy --all-targets --locked -- -D warnings` pass. Full watched dialogue and saved intermediate skips pass `--cinematic-check`, including camera validity, safe landings, identical settled critical mover poses/collision, guard activations and exit count. Existing Pandemonium gates/rope/door/transport checks also pass. Logs: `private/cinema-{check,pand-regression}-final.log`, `private/cinema-unit-final.log`, `private/cinema-clippy-final.log`, `private/cinema-release.log`.

The normal route passes with cinematics (**21,792 ticks**) and with each scene skipped (**21,049 ticks**); both use eight jumps, 18 throws and one original portal and finish at the Fortress 1 transition with **100 Sanity**. No flight, debug relocation, recovery or resource refill is used. These are normal-input simulations with automated aiming, separate from staged visual fixtures. Shared school events and the first school route retain their previous pass (**14,822 ticks, 27 Sanity**). Logs: `private/cinema-route-final.log`, `private/cinema-skip-route-final.log`, `private/cinema-events.log`, `private/cinema-school-regression.log`.

Native save writer/reader processes pass **28 cases**, including warning, vanish, return scene, landing, boarding, flight and post-skip warning/cart. An actual v0.24 airship save upgrades and resaves/reloads without replaying boarding or changing player/resources; earlier actual migration fixtures still pass. Logs: `private/cinema-save-write.log`, `private/cinema-save-read.log`.

Anode inspected native staged camera/actor captures (`private/pand-*.png`). Real P/hold-Enter/F5 input skipped a loaded warning, stopped its dialogue, restored first-person control and wrote a format-7 save with warning completion once, 37 Sanity, 41 Will and unchanged inventory. Real hold-Enter during loaded boarding loaded and autosaved `fortress1$first`, retaining selected Mallet and two Dice. Test folders `private/cinema-native-warning` and `private/cinema-native-board` are separate from player saves. Audio had no native output device, so no new audible playback claim is made. See [CINEMATICS.md](CINEMATICS.md) for approximations and the confirmed unfinished Fortress 1 opening.

## Version 0.23: Pandemonium progression

- Release build and strict Clippy pass; **128 unit tests pass**. The new rule-migration test permits only reviewed gate-condition changes and rejects changed actions.
- `--pandemonium-route-check` completes a normal-start, continuous input route through rope use, the Gnome, minecart, Cards, guards, key, upper door, return portal, house and airship exit. Result: **21,149 ticks, eight jumps, 18 Blade throws, one authored return portal, 100 Sanity**. Six guards are defeated; the other return guard is bypassed. No free flight, debug warp, recovery, resource refill or direct quest-state editing is used. The original scripted cart landing and airship attachment transfers are explicit level mechanics. The route uses automated aiming and dialogue advancement; this is not a human playthrough.
- `--pandemonium-check` passes early gate rejection, key contact/visibility checks, 30/60/144 Hz cart landing, pause, door obstruction/resumption, rope collision/release and continuation after restore, malformed state rejection, cart/return guard activation, four saved progression phases and dialogue-gated one-shot exit.
- `--story-check` decodes **65 original voice/subtitle pairs across four maps**, including Pandemonium's four sequences / 15 lines. Ordinary story contacts and stable map arrivals pass; gated departure is covered by the map/route checks.
- Native Anode staged GPU checks load the original rope/cart/key/airship models and mounted animation clips. Captures of rope hanging, cart seating, the visible key, raised return gate and the airship with Alice/Gnome on attachment tags were inspected. No fallback textures were reported (27,014 triangles / 171 batches in this map). These staged captures are separate from traversal proof.
- Native keyboard input in a staged rope approach confirms E grabs the rope, switches the pose/prompt, and Ctrl descends. Rope interaction takes priority over nearby greeting/dialogue input; hidden Cheshire actors cannot intercept E.
- The native writer and separate reader pass **18 persistence cases**, including cart, key, return and flight, with exact state/continued-simulation comparisons. Actual retained v0.19, v0.20 and v0.21 fixtures still upgrade. An actual v0.22 Pandemonium quicksave was created with the retained Rust executable, then loaded, upgraded, resaved and reloaded. Ordinary player save slots were not used.
- A native staged departure save resumes its voiced-dialogue timeline, loads `fortress1$fortress1_start1`, records Pandemonium completion and writes the new campaign autosave. **Confirmed next-map blocker:** Fortress 1's intro is not implemented, so Alice remains grounded in its dark cinematic staging room (3432, 4820, 200). Receiving the transition does not establish playable Fortress progression.
- Regression routes remain unchanged: school one **14,822 ticks / 27 Sanity**; school two **20,838 ticks / 18 Sanity**; village **9,880 ticks / 100 Sanity**. Shared school entity/event checks and the Ladybug campaign/asset/combat checks pass.
- The current release executable is rebuilt for the launchers. The local source-review package includes the new controller, tests, documentation and launcher, but no game assets, extracted scripts, saves or captures. No original executable/DLL was invoked and nothing was published.

Exact original camera cuts, spline interpolation, Gnome chase/fire staging, machine linkage and all mover sound events remain fidelity work. See [PANDEMONIUM.md](PANDEMONIUM.md) for controls and scope.


## Version 0.22: Pool of Tears Ladybug bombers

Release build, formatting and Clippy pass; **127 unit tests pass**. Eight added regressions cover bounded flight around walls beneath ceilings, water avoidance/pause, frame-rate-independent bombing/rearming and dodging, pain interruption and the single armed death drop, walls/invisibility, saved projectile continuation, saving immediately before an armed death drop, and event-program extension preserving queued work and consumed rules. Logs: `private/ladybug-build.txt`, `private/ladybug-tests.txt`, `private/ladybug-clippy.txt`.

`--ladybug-check` loads the actual `potears1` cast: **12 Ladybugs**, four initially active and eight dormant. Every spawn is clear; every actor advances along its authored node route for **30 simulated seconds** without entering solid geometry. All six original trigger volumes activate their correct groups. Four ambushes produce the first actor immediately and the second after 2.1 seconds, retaining that delay through pause and snapshot restoration. Re-entering does not replay activation or revive a killed actor. Trigger checks pass at **30/60/144 Hz**. A staged stationary target below the resident `lady3` takes **120.39 damage from three drops** over eight seconds at each rate; weapon damage causes pain and death. Log: `private/ladybug-map-check.txt`. These are isolated navigation/activation/combat checks, not a full Pool of Tears route.

Native Anode save writer **86388** and reader **8376** ran in separate processes and exited 0. All **14 cases** restore exactly and match continued simulation. New cases save a pending second ambush arrival, a falling acorn, and a spent Ladybug's death with an existing bomb. Five cached visits rebuild, now including Pool of Tears. The reader also migrates retained actual v0.19/v0.20 school/battle files and an actual v0.21 Pool of Tears quick-save created in original prototype process **93608**, then resaves/reloads format 4 without losing player/resources/NPC identities. Storage corruption, backup and schema checks still pass. Logs: `private/ladybug-save-write.txt`, `private/ladybug-save-read.txt`, `private/ladybug-legacy-write.txt`.

Native `--ladybug-render-check` exited 0 and produced seven inspected captures: patrol with held acorn, pre-release attack, detached falling acorn, blast, pain, falling death with the last acorn, and landing. This fixture stages a resident Ladybug and a stationary target, with an inspection camera. The original model and animation poses are visible; the blast is an independent line/ring approximation. These images do not establish exact original particles, map materials/sky rendering, or a native keyboard playthrough. Log: `private/ladybug-render.txt`; captures: `private/ladybug-{patrol,attack,drop,blast,pain,death,landed}.png`.

Shared school event and Demon Dice combat checks pass. Both existing continuous school routes retain their results: first school **14,822 ticks / 15 jumps / 27 Sanity**, normal `skool2` exit; second school **20,838 ticks / 7 jumps / 51 throws / 40 combat damage / 18 Sanity**, gated `skool1_start2` return. Logs: `private/ladybug-event-check.txt`, `private/ladybug-dice-check.txt`, `private/ladybug-school-route.txt`, `private/ladybug-school2-route.txt`. Audio validation decodes **180 referenced files**, including six Ladybug sounds; the existing missing Ice Wand idle reference remains. Native tests were silent, with no new audible playback claim.

All saves/captures remain private; ordinary player slots were untouched. No owned game windows remain and the Anode lease was released. The v0.21 executable/source backups are retained. The updated source-review package excludes game data, binaries, saves and captures. Nothing was published. [LADYBUGS.md](LADYBUGS.md) records the partial campaign scope and approximated flight/effects; [SAVES.md](SAVES.md) records format-4 migration.

## Version 0.21: Demon Dice

Release build, formatting and Clippy pass; **119 unit tests pass**. The seven new regressions cover the shared original primary/alternate throw and frame-seven release, one 40-Will debit, insufficient resources, busy/pause gating, one-to-three dice and tier thresholds, all three demons' melee/ranged contacts at 30/60/144 Hz, walls/blocked placement, hostile empty-room summons, death/closing sound, disappearing targets and deterministic roll/summon restart. Logs: `private/dice-build.txt`, `private/dice-tests.txt`, `private/dice-clippy.txt`.

`--weapon-check` decodes **13 props / 6,849 vertices / 11 Alice action clips**. `--dice-check` loads original demon skeletons and selected clips and defeats a staged school guard with **60 damage at each of 30/60/144 Hz**. That fixture holds the guard in place to isolate summon combat; it is not a normal-progression route. The shared event and campaign loadout checks pass. Audio validation decodes **174 referenced files**, including ten new throw/bounce/rift/demon sounds, across 36 cues and 901 emitters; the existing missing Ice Wand idle reference remains. Logs: `private/dice-weapon-check.txt`, `private/dice-dice-check.txt`, `private/dice-event-check.txt`, `private/dice-loadout-check.txt`, `private/dice-audio-check.txt`.

The independent headless school routes retain their previous results: first school **14,822 ticks / 15 jumps / 27 Sanity**, then the normal `skool2` exit; school two **20,838 ticks / 7 jumps / 51 Blade throws / 40 combat damage / 18 Sanity**, then the correctly gated `skool1_start2` return. These use the existing real movement/interaction/combat simulation with the limitations recorded below; neither is a native human playthrough using Dice. Logs: `private/dice-school-route.txt`, `private/dice-school2-route.txt`.

Anode actual-input testing used the combat preview in native game process **32072**. Selecting **7**, switching to first person and left-clicking displayed the held die, released it and summoned the lesser demon. The guard received six 10-damage strikes and died. After resetting the staged encounter, repeated right-clicks produced one summon and one debit to **60 Will**; the second click while busy did not debit again. Its demon also damaged the guard. God mode and a health refill were used to keep Alice alive for this staged weapon inspection; Will spending remained active. This is not evidence of a cheat-free campaign fight. Log: `private/dice-native.txt`; private captures include `private/screenshots/skool1-3419.png` and `skool1-7799.png`.

The native save writer and reader ran in separate processes **90944** and **26512**, both exit 0. All **eleven cases** restored exactly and matched continued simulation, including the three new cases: accepted action before release, two rolling dice, and an active hostile summon. The saved balance is 100 Sanity / 60 Will immediately after casting. Existing mover, quest, enemy, event, campaign and storage checks still pass. Retained actual v0.19 and v0.20 school/battle saves also load and survive resaving in format 3. An initial hostile-summon save exposed an out-of-range Alice target sentinel; the corrected bounded identity is included in the passing restart test. Logs: `private/dice-save-write.txt`, `private/dice-save-read.txt`.

Native `--dice-render-check` stages all three original demon models on the school-two gym floor and captures appearance, melee and ranged poses/effects. The private images are inspected separately from the automated completion flag. These explicitly staged captures verify presentation, not campaign acquisition, placement in every room or enemy AI. Early inspection-camera positions were obstructed or outside the room and were corrected before the final captures. Log: `private/dice-render.txt`; captures: `private/dice-{0,1,2}-{Appear,Melee,Ranged}.png`.

Native testing was silent: sounds are connected and decode successfully, but there is no new audible playback claim. Tests use `private/save-check` and preview modes, leaving ordinary player save slots untouched. Previous executable/source backups, analysis and original-art captures remain private. The refreshed source-review archive excludes assets, saves, binaries and captures. Nothing was published. [DICE.md](DICE.md) records the same-action mouse bindings, mechanics and remaining AI/effects fidelity; [SAVES.md](SAVES.md) records format-3 compatibility.

## Version 0.20: shared entity identities, events and puzzle conditions

Release build, formatting and Clippy pass; **112 unit tests pass**. Seven new synthetic tests cover named-group aliases and unnamed identities, composed activation conditions and counters, saved one-shot/cooldown history, delayed FIFO chains at 30/60/144 Hz, paused queues, malformed state/program rejection, missing receivers and transactional relay-cycle rollback. Logs: `private/event-build.txt`, `private/event-clippy.txt`, `private/event-tests.txt`.

Release `--event-check` uses the actual school maps. It verifies saved disabled triggers, group activation, disabled/re-enabled actors and retained death; premature library/recipe rejection and all four completed flying books opening the gate; a saved theatre completion that unlocks/opens its doors and activates both reinforcements once at 30/60/144 Hz; pause and repeated completion; version-1 import and rejection of swapped trigger identities; a disabled gym lever staying unusable after reload, then operating normally once enabled with a retained usage count; and stage-gated Elder Gnome completion preserving the Mushroom and three-Boojum encounter. These are staged interaction/state checks. Log: `private/event-school-check.txt`.

Independent continuous input simulations still pass:

| Route | Result |
| --- | --- |
| First school, normal route | 14,822 ticks, 15 jumps, 27 Sanity, normal `skool2` exit |
| First school, optional secret detour | 16,520 ticks, 17 jumps, 1 climb, 13 Sanity, normal recipe exit |
| Second school, full main quest | 20,838 ticks, 7 jumps, 51 Blade throws, 40 combat damage, 18 Sanity, gated `skool1_start2` return |
| Village, playable opening to authored exit | 9,880 ticks, 4 jumps, 3 throws, no combat damage, 100 Sanity, Pandemonium exit |

These run real movement/interactions/combat with no flight, debug warps, recovery or refills. The school-two route uses its two normal authored portals. They remain headless automated routes with the projectile/input-search limitations recorded below, not native human campaign playthroughs. The intervening Pandemonium/fortress journey is still unfinished. Logs: `private/event-school-route.txt`, `private/event-final-school-secret-check.txt`, `private/event-school2-route.txt`, `private/event-final-village-route-check.txt`. World/door/steam-floor and first/return progression/secret checks also pass: `private/event-final-world-check.txt`, `private/event-final-progression-check.txt`.

During integration, automatically forwarding an original target link to a school-two door changed paired-leaf operation and blocked the gym route. Door activation now requires an explicit binding; the complete school-two route again matches its prior result. The existing physical door controller and quest ownership remain authoritative.

The final native save writer and reader ran in separate Anode processes, **84320** and **86140**, and both exited with code 0. All eight scenarios reconstruct exactly and match the writer's subsequent simulation. The school scenario now saves a theatre-completion event due in 0.25 seconds and resumes its real door/reinforcement effects in the reader. All four cached visits rebuild. Storage corruption, backup, checksum, unsupported-version and failed-write checks still pass. Two retained **actual v0.19** files (partial school and Boojum battle) also migrate all four cached visits, retain player/resources and survive a version-2 resave/reload. Logs: `private/event-save-write.txt`, `private/event-save-read.txt`.

Native checks use staged fixtures in `private/save-check`, leaving ordinary player save slots untouched. No new audible playback or native keyboard playthrough is claimed. No owned game windows remained and the Anode lease was released. The v0.19 executable, source package and legacy fixtures are retained privately; the refreshed source-review archive excludes assets, saves and captures. Nothing was published. See [EVENTS.md](EVENTS.md) for extension points, time/order semantics and component limits, and [SAVES.md](SAVES.md) for compatibility.

## Version 0.19: persistent saves and campaign state

Release build, formatting and Clippy pass; **105 unit tests pass**. Two new regressions check fixed save/load console slots and separation of campaign first/return visits. Logs: `private/save-release-build.txt`, `private/save-clippy.txt`, `private/save-tests.txt`.

Native `--save-check-write` and `--save-check-read` ran in separate Anode game processes (writer **84172**, reader **92352**). All **eight staged cases** passed: partial first-school library/secret and enemy state, partial Boojum battle, lollipop growth, potion mixing, potion collected but Star missing, first-school return, village machinery/Cheshire, and completed-quest exit approach. Every file contains four distinct cached visits, with depleted **37 Sanity / 41 Will**, two collected pickup identities, selected Mallet, two Dice, and an active Looking Glass timer. Restored level controllers, NPCs, Alice animation/actions, dialogue/hints and cached visits match the saved state. Continued mover/combat/action/dialogue simulation matches the recorded continuation. These are staged persistence fixtures, not claims of ordinary traversal. Logs: `private/save-write-log.txt`, `private/save-read-log.txt`; fixtures: `private/save-check/`.

Behaviour checks after loading confirm that collected pickups and the secret do not refill resources or renew invisibility, seen dialogue does not enqueue twice, dead enemies are not resurrected, the Boojum battle can finish, growth/mixing continue, and both rewards are still required for the exit. Storage tests pass for replacement of an existing save, previous-copy recovery, a second damaged primary without losing the good backup, version/data/checksum rejection, truncation, ignoring an interrupted temporary write, invalid entrance rejection and preserving the committed save after an invalid write.

Actual keyboard/window tests used `private/save-ui`, separate from the user's default saves. In process **97120**, F5 saved the school state; console `give all` changed resources to 100/100 and granted all toys; F9 restored 37/41, the original four toy types (including Dice x2), position/view and the **2/4** library objective. Closing the window created `auto.json` and exited 0. A fresh process **60348** offered Continue from `auto`; Enter restored the same resources, inventory, position and objective, paused. Logs: `private/save-ui-first-process.txt`, `private/save-ui-restarted.txt`; private console capture is recorded in the restart log.

A further native test loaded the staged completed school-two quest in process **91492**, resumed and walked across its authored exit using W. The viewer entered **skool1_start2**, retained resources/items/cached visits, and wrote the separate automatic slot. After clean close, process **57628** loaded that automatic slot and reported the same return entrance and campaign cache, with 37/41 resources. Logs: `private/save-transition-log.txt`, `private/save-transition-restart.txt`. Native testing was silent; there is no new audible playback claim. All owned windows closed with exit 0 and the Anode lease was released.

The full movement regressions remain unchanged: village **9,880 ticks / 4 jumps / 3 throws / 100 Sanity**; school **14,822 ticks / 15 jumps / 27 Sanity**; second school **20,838 ticks / 7 jumps / 51 throws / 18 Sanity** ending at the gated school return. The real-map progression/secret/activation check also passes. Logs: `private/save-village-route.txt`, `private/save-school-route.txt`, `private/save-school2-route.txt`, `private/save-progression-check.txt`.

See [SAVES.md](SAVES.md) for controls, storage and exact scope. Version 1 saves retain implemented Rust gameplay and require matching game data; original save files and unimplemented campaign scripts are not supported. The previous v0.18 executable/source archive are retained privately. Saves, fixtures, recordings, original assets and screenshots remain excluded from the source archive. Nothing was published.

## Version 0.18: village machinery, school encounters and secret

Release build, formatting and Clippy pass; **103 unit tests pass**. New combat regressions exercise Diamond projectile sweeping, wall blocking, dodging, equal attack/damage results at 30/60/144 Hz, and invisibility preventing new attacks without making Alice immune to existing projectiles. Logs: `private/progression-unit-tests.txt`, `private/progression-clippy.txt`, `private/progression-release-build.txt`.

The village's normal playable opening reaches its authored **Pandemonium exit** in **9,880 ticks, four jumps, three throws, zero combat damage and 100 Sanity**. The headless route uses ordinary movement, dialogue advancement and timed Blade attacks; it does not use flight, debug warps, recovery, refills or direct progression-state changes. Log: `private/progression-village-route.txt`. **This does not verify a journey all the way to school.** The original chain passes through Pandemonium and the fortress maps, whose progression remains unfinished; see [VILLAGE.md](VILLAGE.md). Private script/entity research recorded those blockers without changing the connecting maps.

The first-school normal-start route now runs with enemy combat active and still exits to `skool2`: **14,822 ticks, 15 jumps, 27 Sanity**. The additional secret route shoots the theatre face using the shared attack/projectile path, collects the Looking Glass, completes all four flying books and exits normally: **16,520 ticks, 17 jumps, one climb, 13 Sanity**. It also checks all six first-visit trigger/dialogue activation groups. Logs: `private/progression-school-route-check.txt`, `private/progression-school-secret-check.txt`.

These are **headless input simulations**, not native human entrance-to-exit playthroughs. They run the live movement, mover, trigger, story, combat, pickup and weapon-contact code, with automated aiming and eye-height Blade release. Native attachment/presentation is checked separately. The two routes neither grant the secret directly nor bypass its wall/damage/collection gates.

`--progression-check` passes real-map component tests for 30/60/144 Hz village machinery, pause, hatch damage threshold, dialogue-triggered guard activation without resurrection, first/return school group separation, one-shot contacts, switch shot occlusion, unique 45-second secret collection/expiry, panel removal, face movement and unchanged recipe exit gating. Log: `private/progression-check.txt`.

The full second-school regression remains playable: **20,838 ticks, seven jumps, 51 throws, 40 combat damage, 18 Sanity**, ending at the gated `skool1_start2` return after the Elder Gnome, three-Boojum battle, rescue, ingredients/transformations and both rewards. Its two teleports are authored portals; no cheats, recovery or refills occur. Log: `private/progression-school2-route-check.txt`.

School and school-two mover/state checks, the all-map world checks, the **945-drop** shelf/footing regression, swim checks and club combat checks pass. NPC validation reports **452 generic eligible placements, 62/63 supported models**; the reduction comes from moving the village/school enemies into explicit encounter ownership. The pre-existing Mad Hatter attachment deferral remains. Story validation decodes **50 voice/subtitle pairs across 21 entries**. Audio validation decodes **164 referenced files**, including the new Diamond/secret effects, across **36 music cues and 901 emitters**. The existing missing Ice Wand idle reference remains. Logs: `private/progression-*-check.txt`.

Anode staged native checks rendered the restored bridge, machinery and sawmill at two motion phases, the secret picture and closed/open alcove, the original Looking Glass model, a Diamond guard and Alice with/without the active invisibility appearance. Twelve captures are stored privately; these fixtures do not prove progression. The ordinary village opening was also launched and its console/input checked. Native tests were silent; no new listening verification is claimed. Log: `private/progression-render.txt`; captures: `private/village-*.png`, `private/school-secret-*.png`, `private/school-diamond.png`, `private/school-looking-glass-*.png`. The final owned native job exited 0 and the Anode lease was released.

The v0.17 executable and source archive are retained privately. The refreshed source-review archive excludes original assets, transcripts, extracted scripts, Ghidra output, captures and executables. Nothing was published. Exact cinematic choreography, original AI/motion fidelity, the intervening village-to-school maps and the first school's return quest remain unfinished.

## Version 0.17: developer console and summoned Cheshire

Release build, formatting and Clippy pass; **100 unit tests pass**. New cases cover command parsing/rejection of chained input, bounded text/history/output and draft restoration, hint cooldown/pause/busy handling without quest completion, obstructed/unsupported Cat placement, and god mode's opt-in damage immunity while retaining Will costs and quest items. Logs: `private/console-hints-tests.txt`, `private/console-clippy.txt`, `private/console-release-build.txt`.

Release `--cheshire-check` loads supported hint data across all 36 maps: **38 authored regions and 40 unique recordings**, fully decoded to finite audio samples. It checks each of the three second-school region selections and plays their lines through the story clock without emitting quest completion. Some recordings are referenced by one map but have subtitles in another TLK; the loader resolves those from the local tables. Named regions requiring unfinished script gates are deferred. Log: `private/cheshire-assets.txt`.

Release `--story-check` retains all **49 voice/subtitle pairs**. Release `--audio-check` now decodes **158 unique referenced files**, including Cheshire's appearance/disappearance effects; the existing missing `sound/icewand/icw_idle.wav` reference remains. Logs: `private/console-story-check.txt`, `private/console-audio-check.txt`.

The full normal-start release school-two route remains unchanged: **20,838 ticks, 7 jumps, 53 throws, 40 combat damage and 18 Sanity**, reaching the gated `skool1_start2` return with no cheats, recovery or refills. This remains a headless input simulation with the limits described in the v0.16 record. Log: `private/console-school2-route.txt`.

Anode native testing opened the console with backtick, checked status, held W while typing and confirmed unchanged coordinates, recalled history, completed `no` to `noclip`, toggled flight on/off, granted toys/resources, enabled god mode and rejected a missing map. C summoned the visible, textured Cheshire with his sitting/talk animation and original subtitle; V and P allowed inspection in first person while paused. A fast opening/typing test exposed the first typed character being dropped; the input branch was fixed. The final release then accepted backtick immediately followed by `map skool1$skool1_start2`, loaded that entrance with fresh state, accepted an immediate `status`, and summoned Cheshire through the `cheshire` command. These are staged interface tests, not campaign completion. Logs: `private/console-native.txt`, `private/console-native-final.txt`. Captures include `private/screenshots/skool2-2625.png` (console), `skool2-3547.png` (C summon), `skool1-1122.png` (named entrance/status), and `skool1-3670.png` (console summon).

Both owned native jobs exited with code 0; no owned game windows remained and the Anode lease was released. Native tests used `--no-audio`; no new listening verification is claimed. The v0.16 binary and source archive are retained privately. The refreshed source archive excludes original assets, transcripts, Ghidra exports, captures and binaries. Nothing was published. See [CONSOLE.md](CONSOLE.md) for commands, hint scope and provenance.

## Version 0.16: second-school traversal, combat and potion quest

Release build, formatting and Clippy pass; **95 unit tests pass**. New synthetic tests cover ordered quest stages, ingredient consumption, pause, duplicate events, distinct reward gating, Boojum activation, deterministic timed scream attacks at 30/60/144 Hz, wall-blocked contacts, dodging, pain and death. Logs: `private/school2-final-checks.txt`, `private/school2-release-build.txt`.

The release **`--school2-route-check` completes the main quest continuously from the normal entrance** in **20,838 physics ticks, 7 jumps and 53 Blade throws**, taking **40 combat damage** and finishing alive with **18 Sanity**. It operates doors and the bleacher lever with E, reaches the Elder Gnome, advances his dialogue, defeats the three gym Boojums, climbs the broken central stairs, rescues him from two guards, collects Jumbogrow, grows and collects the lollipop, returns the ingredients, waits for mixing, collects both potion and star, and reaches the gated `skool1_start2` return. The two teleports are the ordinary authored mirror portals. No free flight, debug warp, recovery, refill or direct quest-state mutation is used. Supported placed guards also participate. Movement/body clearance and survival are checked throughout. Log: `private/school2-final-school2.txt`.

This is a **headless input simulation**, not a native human end-to-end playthrough. It runs the actual controller, interactions, quest, combat timing and shared projectile/contact path; its thrown Blade starts at eye height rather than the native rendered weapon attachment. Search selects movement inputs for difficult jumps, then replays them through the live controller. Native presentation is checked separately below.

Release `--school2-check` verifies pause and 30/60/144 Hz mover agreement, shared rigid pendulum/lamp motion, floating cabinet/books mesh collision and carried riders, premature exit rejection, repeated potion collection not unlocking the exit, both distinct rewards unlocking the correct return, and the arrival backdoor remaining disabled. These are explicitly staged component cases. Log: `private/school2-final-school2.txt`.

Release regressions pass: `--school-check`, `--school-route-check`, `--footing-check`, `--world-check`, `--swim-check`, and `--story-check`. The first-school continuous traversal remains **13,863 ticks, 15 jumps and 48 Sanity**, with combat checked separately there. The shelf regression still passes **945 landings** plus walking/jumping and both full tipping motions at 30/60/144 Hz. Story validation now fully decodes **49 original voice/subtitle pairs across 20 entries**, including eight second-school conversations/hints. Log: `private/school2-final-regressions.txt`.

Release `--audio-check` validates **36 level music cues, 901 ambient emitters and 156 fully decoded unique files**, including the new Boojum, guard, Gnome and quest effects. The one known missing original reference remains `sound/icewand/icw_idle.wav`. Log: `private/school2-final-audio.txt`. Native tests used `--no-audio`; audible mixing was not listening-verified.

Anode tests inspected the normal entrance and eight staged rendering scenes: floating movers, gym pendulums, activated Boojums, Gnome rescue, open Jumbogrow cabinet, grown lollipop, potion mixing and rewards. Sixteen captures under `private/school2-*.png` and `private/school2-render-log.txt` record those fixtures. They do not prove normal progression. A separate release native test started near the Gnome, used actual E input to finish his dialogue, showed the three activated flying Boojums and depth-tested scream rings, and let their attacks deplete Sanity to the normal death/retry panel. Enter restored the entrance; I showed the retained Mushroom above the weapon grid. This native test did not land weapon hits; combat completion is covered by the continuous simulation. Log: `private/school2-native-combat.txt`; inventory capture: `private/screenshots/skool2-9242.png`.

The native test window exited with code 0, no owned game windows remained in the seat, and the Anode lease was released. The v0.15.1 executable/source archive are retained privately. The refreshed source-only archive excludes original game data, binaries, extracts and captures. No public release was made. Exact cinematic choreography, remaining enemy types and the first school's return quest remain unfinished; see [SCHOOL2.md](SCHOOL2.md).

## Version 0.15.1: shelf collision and footing

Release build, formatting and Clippy pass; **90 unit tests pass**. New synthetic regressions cover the highest corner of rotated brushes (including the first 0.1875-degree tilt increment), grounding and jumping there, recovery from a tiny floor overlap, rejection of deep overlaps, and a thin ceiling that must not be crossed by correction. Logs: `private/footing-tests.txt`, `private/footing-clippy.txt`, `private/footing-release-build.txt`.

Before the fix, the staged real-school sweep reproduced **nine airborne stalls** along the fallen shelf crossing plus **both tipping shelves embedding a rider on their first movement tick**. The completed release `--footing-check` passes **945 drops**, eight walking directions and a jump from each, plus both full tipping motions at 30/60/144 Hz. Mover clearance is asserted before physics, so the small-overlap correction cannot hide a carry failure. Logs: `private/footing-before.txt`, `private/footing-release-check.txt`.

The continuous release school route still reaches `skool2` in **13,863 ticks, 15 jumps and 48 Sanity**, without flight, warps or recovery. This covers traversal/progression, not NPC combat. School lift/state, all-map world/door/steam-floor, swimming and the 36-map asset/collider checks also pass. Logs: `private/footing-release-route.txt`, `private/footing-school.txt`, `private/footing-world.txt`, `private/footing-swim.txt`, `private/footing-maps.txt`.

In Anode, the release `--shelf-preview` starts at the formerly stalled contact. Actual keyboard input produced a visible jump there, then a second jump with forward movement escaped the crossing and settled on the lower library floor. Final logged state: grounded, clear full body, two jumps, no recovery or flight; the owned window exited with code 0 and the lease was released. This is a staged local reproduction, not a native entrance-to-exit playthrough. The first direct GUI job opened no visible window and was cancelled; the successful test used a command wrapper. Native test was silent. Log: `private/footing-native.txt`; private captures: `footing-standing.png`, `footing-jump.png`, `footing-escaped.png`. The v0.15 source archive is retained; the refreshed package excludes all original data, binaries and captures. No public release was made.

## Version 0.15: village/school dialogue and gym bleachers

The release build, formatting and Clippy pass; **88 unit tests pass**. New synthetic cases cover comment/prototype-aware dialogue extraction, TLK encoding and multiline text, once-only queues, pause/advance, delayed exits, recovery cancelling a delayed exit, subtitle wrapping, and voice pause/mute/music reduction independent of music playback. The source package retains a v0.14 backup and excludes transcripts, original recordings, extracts, screenshots and binaries.

Release `--story-check` validates all **43 original voice/subtitle pairs across 18 entries in three maps**, fully decodes the recordings with Rodio, and compares sample duration with subtitle timing. Real BSP tests verify supported village/school-two arrivals settle on solid ground, story triggers respect pause/contact re-entry, and both premature school-two exits stay disabled. `--gym-check` validates the lever from supported footing, one-way extension, pause, a physical climb of all twelve bleacher tiers at 30/60/144 Hz, and obstruction stop/resume without crushing. These are component tests; there is no complete village or school-two route claim. Logs are `private/v015-story-check.txt` and `private/v015-gym-check.txt`.

Release regressions pass: `--school-check`, `--world-check`, `--combat-check`, `--npc-check` (472 eligible placements, 62/63 models; the known Mad Hatter attachment deferral remains), and `--school-route-check`. The first-school route still reaches its expected exit in **13,930 ticks with 16 jumps and 48 Sanity**, without flight, warps or recovery. It checks traversal/state, not the real-time dialogue player or NPC combat. Logs are `private/v015-*-check.txt`; unit results are `private/story-tests.txt`.

Anode verified village subtitle rendering, pausing and E advancing one line after resume. A staged upper-gym test triggered the original Elder Gnome conversation and showed his talk clip; a separate staged lever test used actual E input and logged one successful activation with a visible pulled lever. The first lever camera fixture was obstructed and fell back to free flight, so it was replaced with a verified standing approach at (2163, -1859, 64) before the successful interaction. These fixtures do not establish traversal from the normal entrance. Logs: `private/story-gym-native.txt`, `private/gym-lever-native-final.txt`.

The final release launchers also exited successfully with captures `private/story-village-final.png` and `private/story-gym-final.png`; logs are `private/story-village-release.txt` and `private/story-gym-release.txt`. **Anode reported NoDevice**, so the initial requested mix capture produced no audio and exited with its expected capture error. Subsequent visual tests were silent. Voice decoding/timing and independent sink controls pass, but audible native dialogue, perceived mixing and original lip/cinematic timing are not listening-verified. All owned windows were closed and the seat lease released. No public release was made.

## Version 0.14: school first-visit progression

Release build, formatting and Clippy passed; **82 tests passed**. The added synthetic checks cover unwrapped spiral rotation and safe 100-unit ordinary jumps with either walking or running input. The earlier fall, swimming, combat and weapon regressions remain passing.

The release `--school-route-check` starts at the normal entrance and reaches the recipe exit for `skool2$skool2_start1` using **13,930 physics ticks, 16 jumps and 48 remaining Sanity**. This is a continuous movement-and-puzzle test: doors, the theatre/library gate, lower book, spiral lift, middle book, tipped shelves, upper book, upper lift, top book, bridge, recipe event, descent and exit. It uses no flight, warps, recovery, resource refill or direct puzzle-state changes. Difficult crossings use a bounded search for movement inputs, which are then replayed through the actual controller and interaction code. NPC combat is excluded from this route and checked separately; this is not an original-game comparison or a human end-to-end playthrough.

`--school-check` validates actual trigger volumes, rejects early/out-of-order/paused progression, carries riders on both lifts at 30/60/144 Hz, stops/resumes a lift under a test obstruction, delays bridge solidity when Alice occupies it, and checks the named exit. Retry retains progress, while a fresh visit resets it and the return visit excludes the first-visit recipe puzzle. The existing `--physics-check`, `--world-check`, `--swim-check`, `--combat-check` and `--weapon-check` all pass. Logs are `private/school-final-school-check.txt`, `private/school-final-school-route-check.txt`, `private/school-regression-*.txt` and `private/school-tests.txt`.

The Anode native renderer diagnostic loaded and animated the closed flying book, completed book bridge and fallen/open recipe book. Three staged captures were inspected: `private/school-waiting.png`, `private/school-bridge.png` and `private/school-recipe.png`. Initial diagnostic cameras were inside/behind geometry; their positions were corrected before the final captures. These staged scenes are separate from the continuous traversal proof. The native depth regression still passes fog/effect wall occlusion, transparent layering and first-person self-occlusion. Logs: `private/school-render.txt`, `private/school-depth.txt`.

A normal release launch also exercised H help in Anode. That exposed the objective announcement being hidden by the help layout; the objective now has its own line in the temporary H panel, verified in the rebuilt release (`private/school-objective.png`). The normal test window exited with code 0; no owned game windows remained and the Anode lease was released. Native smoke tests use `--no-audio`; no new dialogue, listening comparison or full enemy-script playthrough is claimed. See [SCHOOL.md](SCHOOL.md) for gameplay and fidelity limits. The v0.13 source archive is retained, and the new source package excludes original assets, extracts, captures and decompilations. No public release was made.


## Version 0.13: club-guard encounter

Release build, formatting and Clippy passed; **80 tests passed**. Six new synthetic tests cover nearest swept weapon contacts before walls (including embedded starts and misses), fixed-step pursuit/attack timing across 30/60/144 Hz, pause, once-only and avoidable enemy strike contacts, cliff/wall avoidance, pain/death interruption and resource gating during equip/busy/unsupported actions.

The release `--combat-check` loads the real school, door collision and club-guard clips. At each of 30/60/144 Hz it verifies approach along the entrance floor, 60 outgoing damage over 12 seconds, Blade reach and defeat after three Blade hits. `--npc-check` still validates 62 of 63 model definitions across 470 eligible placements, now including the club guard's five additional behaviour clips; the Mad Hatter remains the expected skeletal-attachment deferral. `--weapon-check` passes all 12 props, 6,825 decoded vertices, ten action clips and three Alice attachments. Logs: `private/combat-route-validation.txt`, `private/combat-npc-validation.txt`, `private/combat-weapon-validation.txt`, `private/combat-test-results.txt`.

In Anode's hidden desktop, the native release preview showed the guard alert, approach and attack. Blade swings inflicted 20 damage and a third landed swing defeated the 55-health guard. One click made during an unfinished action was ignored. A card spread landed three 10-damage contacts and left Will at 95; the thrown Blade then inflicted 35 and defeated that guard. A croquet ball inflicted 25 with Will at 94, and a subsequent Mallet swing inflicted 35 and completed that encounter. Hit, pain and finished death poses were visible. Pause and first/third-person views were exercised.

Home restored the guard for each new encounter. Letting it attack drained Alice's Sanity in 12-point steps to zero, displayed her death/retry panel and played her death pose. Enter restored 100 Sanity and reset the encounter. The owned window closed with exit code 0, no owned windows remained, and the Anode lease was released. Native log: `private/combat-desktop.txt`; captures: `private/combat-blade-defeat.png`, `private/combat-card-spread.png`, `private/combat-mallet-defeat.png`, `private/combat-alice-death.png`. These runs used `--no-audio`; sound paths are connected but no new listening comparison is claimed. Subsequent changes extracted the unchanged action-readiness predicate for testing, bounded distant sight checks, and updated help/preview wording; the final release and encounter checks were rebuilt afterward.

Scope is deliberately one enemy type with local pursuit and simplified hit volumes. Other AI, body separation, general navigation, quests, loot and original balance are not established. See [COMBAT.md](COMBAT.md). The v0.12 source archive is retained and the local source-only package is refreshed. Original artwork, data and captures remain excluded and unpublished.

## Version 0.12: placed NPCs and greetings

Release build, formatting and Clippy passed; **74 tests passed**. Five new synthetic tests cover greeting distance/facing/occlusion/cooldown, paused reactions and wall-blocked turning, server-only metadata parsing, school first/return startup rules, and malformed/activation-gated placements. NPC body rendering shares Alice's existing skinning shader; attachment loading now accepts named TAN clips other than `idle`. Model directories normalize trailing slashes, and texture lookup recognizes extensionless shader aliases referenced with image extensions.

The release `--npc-check` audited 470 eligible first-visit placements across all 36 maps. It loaded **62 of 63 model definitions**, decoded their visible textures and TAN attachment textures, checked attachment bones, and skinned every frame of the selected clips to finite positions. The Mad Hatter's skeletal cane is an explicitly expected deferral; other model failures fail the audit. This does not validate every NPC animation, original AI, story visibility or scripted path. Log: `private/npc-validation.txt`.

In Anode's hidden desktop, the preview showed the Cheshire Cat, gnome, club guard and schoolchild with textured bodies, animation and attached guard/headgear props. E triggered the Cat's greeting. A normal school cast loaded 19 actors from six models; a diagnostic start near the authored entrance schoolchild allowed its E greeting to be exercised. First/third-person switching, walking, pause and the existing Blade presentation were inspected. Choosing the school's return visit through Tab/Right/Enter rebuilt the cast to 11 actors from three models and loaded the return entrance. There is no claim of NPC combat, pathfinding, spoken dialogue or full-map progression.

Both interactive test windows exited with code 0. The native depth regression also passed all six fog/effect wall-occlusion cases, transparent depth-write preservation and first-person depth layering. `--weapon-check` passed 12 prop models, 6,825 vertices, ten action clips and three Alice attachments; `--character-check` passed 222 clips/8,688 frames. Logs: `private/npc-desktop.txt`, `private/npc-school-desktop.txt`, `private/npc-depth-validation.txt`, `private/npc-weapon-validation.txt`, `private/npc-character-validation.txt`. Screenshots: `private/npc-preview-greeting.png` and `private/npc-school-greeting.png`. Tests used `--no-audio`; NPC audio is not implemented. All owned windows closed and the Anode lease was released.

The v0.11 source-review archive is retained. The refreshed local source allowlist includes the NPC preview launcher and documentation, and excludes original assets, extracts and screenshots. Nothing has been published. See [NPCS.md](NPCS.md) for limitations.

## Version 0.11: first-person weapons and effects

Release build, formatting and Clippy passed; **69 tests passed**. Three new synthetic tests cover camera-relative orientation through yaw/pitch changes, pause/reset of the view pose and knife release visibility, and projectile launch/aiming with both a nearby and distant wall. The close-wall test exposed an aiming trace that could run along the wall; aiming at the surface instead of the swept clearance plane fixed it.

The native `--render-check` passed all six existing translucent wall-occlusion cases and the transparent depth-write check. Its new first-person check verifies that clearing the weapon depth layer preserves world colour while the local weapon still occludes its own farther geometry. An initial test expectation assumed the library's named BLUE colour was pure blue; the assertion was corrected to compare the actual colour with framebuffer tolerance. Log: `private/first-person-render-validation.txt`. `--weapon-check` still validates all 12 prop models, 6,825 decoded vertices, ten action clips and three Alice attachments; log: `private/first-person-weapon-validation.txt`.

Anode's hidden desktop exercised all ten first-person equipment selections, the knife resting pose, primary swing/trail and alternate thrown blade, Cards single/spread throws, Mallet swing/trail and alternate ball, movement toward the school door, looking up while strafing, pause/resume, Home and switching back to the third-person body. The run logged eight fired actions and ten world impacts. The first visual inspection prompted a resting-angle adjustment to show the knife face and card artwork more clearly. This validates the independently authored first-person presentation, not original-game hand/animation fidelity. The separate hand mesh and combat remain unfinished.

Local captures include `private/first-person-knife.png`, `private/first-person-swing.png`, `private/first-person-throw.png`, `private/first-person-cards.png`, `private/first-person-mallet.png` and `private/first-person-ball.png`. Native log: `private/first-person-desktop.txt`; the initial framing run is retained separately. Tests used `--no-audio`, so no fresh listening claim is made. Owned native windows closed with exit code 0, the final GPU check exited successfully, no owned windows remained and the Anode lease was released. The final release build includes the corrected GPU assertion. Original-art captures remain private; the prior v0.10 archive is retained and the allowlisted source-review package is refreshed.

## Version 0.10: campaign weapon loadouts

Release build, formatting and Clippy passed; **66 tests passed**. Four new tests cover weapon-acquisition boundaries, first/return visits, cumulative Dice, fresh-start inventory reset, normal-exit preservation of resources/selection/pickup history, idempotent grants, preview inventories and the original Deadtime Watch classname alias. See [LOADOUTS.md](LOADOUTS.md) for the full progression and assumptions.

`--loadout-check` passed for **36 maps and 39 visits**, validating supplied named entrances and all weapon milestones against the installed map/script data. It also confirms that the actual `hatter2` Watch entity produces a collectible inventory item. `--hud-check` passed for **10 weapon definitions, 8 resource definitions and 219 supported placed pickups**. Logs: `private/loadout-validation.txt` and `private/loadout-hud-validation.txt`. These checks establish the configured profiles and supported data, not original-game playthrough fidelity or item reachability.

In Anode's hidden desktop, a normal `garden1` launch showed its five expected toys and one Die. The Tab chooser switched the forest to its named return entrance, showing three Dice, Jacks, the Watch and complete Eye Staff while leaving the Blunderbuss unavailable. Switching back to its first visit removed the later toys and returned to two Dice. `qlair` supplied all ten toys and three Dice, and the Blunderbuss could be selected. A subsequent school first-visit selection reset to Blade/Cards and rejected the unavailable Blunderbuss. Home retained this inventory. The school return supplied Mallet and one Die, and logged the return steam-floor setup. Normal-exit state retention/fill-in is covered by synthetic tests and code inspection this version; no new native traversal of an exit is claimed.

The native test window closed with exit code 0; no owned windows remained and the Anode lease was released. Tests used `--no-audio`. Only comment/help/notification wording changed after the native run; the final release was rebuilt and data checks rerun. Native log: `private/loadout-desktop.txt`. Captures: `private/loadout-garden.png`, `private/loadout-forest-return.png`, `private/loadout-forest-first.png`, `private/loadout-final-level.png`, `private/loadout-school-first.png` and `private/loadout-school-return.png`. Research excerpts and original-art captures remain private. The prior v0.9 source archive is retained and the allowlisted local source-review package is refreshed.

## Version 0.9: steam-floor support, surface posture and recovery

Release build, formatting and Clippy passed; **62 tests passed**. The new regression coverage includes recovery after dropping and jumping in a pit, obstructed footing/entrance fallback, edge rejection, swept fatal-fall contacts, paused triggers, death-animation reset, shallow-water wading, swimming-pose alignment and shallow entry above the float line.

`--world-check` verifies all eight authored steam-floor platforms for both school entry states and walks across the floor at 30/60/144 Hz. It checks a real school falling-death volume and its contact reset, alongside the earlier door route. `--swim-check` verifies the deeper surface position at all three render rates and both west/southwest bank exits, with eight approaches remaining outside solids. Logs: `private/recovery-world-validation.txt` and `private/surface-swim-validation.txt`. The GPU depth/transparent-write checks still pass in `private/render-depth-validation.txt`.

In the hidden Anode desktop, the native release window was exercised with keyboard input: surface treading, forward stroke, pause, Home, western bank climb, walking over the school steam floor, dropping onto the lower carpet and R returning to prior footing. A targeted test start crossed an authored fatal-fall volume, displayed zero Sanity and the death/retry panel; Enter restored resources and walking at the entrance. Faint, falling and submerged-death clips load, with the falling pose inspected. This does not claim original death-camera/timing fidelity. Test starts were supplied through the explicit `--start-at` diagnostic option; ordinary launches still use the original entrance. Logs: `private/surface-desktop.txt`, `private/steam-floor-desktop.txt`, `private/death-retry-desktop.txt`.

The first native window-close check exposed a pre-existing cursor-release panic after the graphics request queue had closed. Window-close requests now pass through normal cleanup; retesting the title-bar close while mouse capture was active exited with code 0. Escape also exited cleanly. All owned windows/jobs were closed and the Anode lease released. Tests used `--no-audio`; no new listening claim is made.

Local evidence includes `private/swimming-surface-v09.png`, `private/swimming-stroke-v09.png`, `private/swimming-bank-v09.png`, `private/steam-floor-v09.png`, `private/steam-recovery-v09.png` and `private/death-retry-v09.png`. These captures and the narrow read-only Ghidra callback inspection remain excluded from source packaging. The prior v0.8 source-review archive is retained. General platform scripting, persistence, combat and full-level progression remain unfinished; see [RECOVERY.md](RECOVERY.md).

## Version 0.8: swimming, bank exits and wall occlusion

Release build, formatting and Clippy passed; **56 tests passed**. New checks exercise convex/sloped liquid membership, hazard priority, idle flotation, diving/surfacing, bottom collision, fast water entry without a false hard landing, low-bank exits, rejection of tall walls/low ceilings, pause and fixed-rate damage, and swim animation state selection. The existing walking, door, inventory, audio and weapon tests still pass.

`--swim-check` builds **371 convex liquid volumes across 26 maps**. The real `garden1` underwater start completes float/dive/surface/swim/stop phases with matching results at 30/60/144 render Hz. Eight approaches to its banks remain outside solids; the west approach climbs to dry ground and the southwest approach walks out on a slope. A first attempt exposed a surface-height clamp that could move the body into a sloped bank; the adjusted endpoint is now swept again. Log: `private/swimming-validation.txt`.

The user's report of fog/effects showing through walls exposed a separate real defect: Miniquad 0.4.8's OpenGL implementation disables depth testing when `depth_write` is false. All custom 3-D effect pipelines now enable depth testing; a flushed transparent pass separately masks depth writes. `--render-check` passed in Anode with pixel readback: a wall hides a fogged effect for all six supported translucent blend modes, and overlapping transparent layers blend without writing depth. Log: `private/render-depth-validation.txt`. These GPU checks extend the earlier headless coverage, which could not detect the bug.

Anode native checks verified original tread/frog-stroke animations, Space ascent, Ctrl descent, look-directed swimming, a bank climb followed by standing/walking, pause, Home returning to the underwater spawn, and first-person swimming. The bank route used normal controls, with no free flight. A separate 180-frame school rendering run completed and its capture was inspected. Owned windows/jobs were closed/completed and the desktop lease released.

The 18-second, 1200 Ã— 622, 30 FPS `private/swimming-demo.mp4` shows underwater movement, surfacing and leaving the pool; sampled frames were visually inspected. Captures/logs remain local and excluded from source packaging. Native checks used `--no-audio`. All **141 referenced audio files** in the current audio check decoded, including the four new water sounds; the previously known missing original ice-wand reference remains. This is not a listening check for the new water events.

The 36-map corpus validation and three-rate school-door route also pass. Logs: `private/swimming-corpus-validation.txt`, `private/swimming-world-validation.txt`, `private/swimming-audio-validation.txt`. Swimming/climb tuning, liquid damage, underwater colour, exact original water blending, animation timing and root-motion alignment remain provisional. No full map or scripted progression is claimed. See [SWIMMING.md](SWIMMING.md).

## Version 0.7: environment and initial interactions

Release build, formatting and Clippy passed; **50 tests passed**. New checks cover material stage/animation line boundaries and transforms, malformed inline-model/fog references, literal fog settings, bounded steam definitions, interaction reach/occlusion/pause/player obstruction, swept trigger entry/reentry/cooldown, and bounded map destinations. No full-map completion is claimed.

`--validate-all` passed for all **36 maps, 1,007,917 stored vertices, 183,673 surfaces, 2,953 FTX images and 2,138 material definitions**. Log: `private/environment-corpus-validation.txt`. `--world-check` passed all maps with **149 door objects, 506 supported/pending trigger volumes, 11 fog volumes and 8 literal distance-fog settings**, and validated exit-map/named-spawn references. It reports **376 distinct pending map/script-thread pairs**. At 30, 60 and 144 Hz, the real school route verifies closed-door collision, E opening both leaves and successful walking through without solid overlap. Log: `private/world-validation.txt`.

Anode's native-window checks verified school doors from both sides, opening/closing, passing through, pause and ignored E during pause; animated fire; and the school's three steam emitters. Garden fog/colour was inspected from free flight. Its normal start is underwater and currently sinks because swimming is absent. A school exit loaded `skool2` at its named entrance, retaining resources/inventory. **Free flight was used to reach that exit**, so the check validates transition mechanics only, not the intended route or puzzles. The tests used `--no-audio`; no new listening validation of door sounds is claimed.

The first rendering check exposed a dropped lightmap-atlas texture handle, producing a black world. Retaining the handle in the scene fixed it; subsequent native captures were inspected. One direct Anode launch failed before startup with Windows error `0xc0000142`; launching through the existing command-wrapper approach then ran successfully. The failed owned process was terminated, its remaining error dialog dismissed, all owned game windows closed and the desktop lease released. Other seat applications were left alone.

`private/environment-interactions-demo.mp4` is an 18-second, 1200 Ã— 622, 30 FPS silent native-window recording. It shows E opening the school doors, walking through toward the fire, and pausing; sampled frames were inspected. Local captures include `environment-school.png`, `environment-steam.png`, `environment-garden.png` and `world-exit-preview.png`. They contain original artwork and are excluded from the source package. Nothing has been published.

Fog density, door flags/rotation/pairing and damage timing remain provisional. Script activation, sliding doors, puzzle/moving-platform logic, NPCs/combat, cinematics and swimming remain unimplemented. Read [WORLD.md](WORLD.md) for scope, evidence and the concrete school-route blockers. No original-versus-Rust comparison was performed.

## Version 0.6: held weapons and the first visual actions

Release build, formatting and Clippy passed; **41 tests passed**. New regression checks cover unsigned TAN coordinates and malformed ranges, action events across 30/60/144 Hz and pause, weapon-change cancellation, upper-body pose masking, and visual projectile collision against a thin wall with bounded ball bounces. The local weapon check validates **12 prop models, 6,825 decoded vertices, 10 action clips and 3 named Alice attachments**, including every referenced skin. Log: `private/weapon-validation.txt`.

Anode exercised all ten equipment selections, weapon-switching, both click actions on Blade/Cards/Mallet, moving and jumping with actions, pause, inventory and Home reset. The Blade's grip/trail, mallet growth and ball toss, flying-card spread, and world impact effects were visually inspected. A small shoulder offset improves projectile visibility; the existing camera collision regression still passes. An initial point-blank impact check exposed overly angular sparks; the final version uses soft radial sprites and proximity fading. Inventory explains that this is a visual preview without damage or Will spending.

`private/weapon-visuals-demo.mp4` is a **26-second, 1200 Ã— 622, 30 FPS** silent capture of the owned Rust window. It includes the first three toys' actions, all equipment selections, movement/jump and pause; sampled action and equipment frames were inspected. The final screenshot is `private/weapon-preview.png`. Startup diagnostics are recorded in `private/weapon-desktop-validation.txt`. An initial diagnostic launch through the background command runner created no visible game window; that owned job was cancelled and desktop testing used the GUI launcher instead.

The audio corpus check now fully decodes **137 unique referenced files**, including the 13 added weapon effects. Its only missing reference is the previously known original ice-wand ambient file. Log: `private/weapon-audio-validation.txt`. These desktop checks used `--no-audio`; event wiring and decoding are verified, but fresh listening/output-capture verification of the added sounds is not claimed.

The preview shortcut was checked separately. Owned test windows were closed and the Anode lease released. Data and captures remain local and excluded from source packaging; the previous v0.5 source archive is retained. Seven toys have held models only. Damage, Will consumption, enemy behaviour, exact original projectiles/particle scripts, dedicated first-person hands and the rest of the attack system remain unfinished. Details: [WEAPONS.md](WEAPONS.md).

## Version 0.5: HUD, resources, weapon selection and pickups

The release build, formatting and Clippy passed; **37 tests passed**. Five additional tests cover inventory ownership/cycling and Dice caps, resource clamping/death/reset, regeneration timing, narrow TIKI field parsing, single-use and obstructed pickups, and physics impact speed feeding fall damage once. The local HUD corpus check read **10 weapon definitions, 8 resource definitions and 218 supported static pickups across 36 maps**. Log: `private/hud-validation.txt`. See [HUD.md](HUD.md) for provisional tuning and unsupported mechanics.

Anode verified the compact original-art meters, selected-toy icon, the I inventory, mouse and number-key selection, locked-slot rejection, wheel cycling and closing inventory with Escape. Holding movement while inventory was open left Alice in place. In explicit `--hud-preview`, partial Sanity/Will displayed correctly; Will stayed at 69 while inventory was open and increased after resuming. Home restored both to 100 without losing selected/owned toys, and an unsupported attack displayed its notice without spending resources. The mallet's alternate cost is visible even though its primary attack uses no Will.

A normal exploration run collected the actual skool1 Croquet Mallet: its world marker disappeared, the notification appeared, the selected icon changed and the inventory slot unlocked. Free flight was used to reach its platform, then walking enabled the pickup. Ownership and selection remained after changing to skool2. Collection obstruction and repeated-collection prevention are additionally covered by synthetic tests; not all 218 placements have been traversed manually.

Testing garden1 exposed a lethal initial drop at its unfinished scripted start. The first landing after map load/Home is now protected; the same start retained 100 Sanity. A deliberate high drop after returning from F4 flight reached zero, displayed the Sanity-lost prompt and stopped gameplay; Home restored both resources. Out-of-bounds recovery now returns in free flight to avoid repeatedly falling from an unsupported start. The latter branch is code-checked, not a claim of a full campaign recovery playthrough.

The sample-meter screenshot is `private/screenshots/hud-preview.png`. It is explicitly a diagnostic preview, not a normal starting loadout. Anode used `--no-audio`; previous sound verification remains separate. All owned game windows were closed and the desktop lease released. Original-art screenshots stay local and out of the source archive. Combat, persistent saves, power-up/watch timers, difficulty scripting and animated TAN HUD fidelity are not implemented.

## Version 0.4: Alice's model, animation and third-person camera

Release build, formatting and Clippy passed; **32 tests passed**. Five new synthetic checks cover malformed skeletal files, hierarchy/weighted skinning, interpolation and loop boundaries, locomotion transitions and camera collision. The full local character corpus decoded and skinned **131 bones, 19 surfaces, 222 clips and 8,688 frames**, with finite vertices and a maximum 0.676-unit excursion outside stored animation bounds. Only six clips are currently wired into gameplay. Details: [CHARACTER.md](CHARACTER.md); log: `private/character-validation.txt`.

Anode verified the textured model from front and back, idle/walk/run/jump poses, pause/resume, camera orbit and shortening near walls, the raised framing at the school start, V first/third-person switching, F4 free flight and return, Home reset and loading skool2 with Alice present. A camera regression test also checks clearance from walls and hiding the skin in extremely cramped spaces. All owned windows were closed and the desktop lease was released.

`private/alice-character-demo.mp4` is a 12-second, 1200 Ã— 622, 30 FPS recording of only the Rust game window. Its movement and airborne frames were visually inspected. These Anode checks used `--no-audio`; the previous audio verification remains separate. The recording and screenshots contain original artwork, remain local and are excluded from the source package. The first recording attempt failed because its window height was odd; the final successful capture adds one padding row for H.264.

No original-versus-Rust animation comparison is claimed. Root motion, exact controller/animation timing, weapon attachment, combat and story events remain unfinished. Static collision and approximate lighting still limit fidelity.

## Version 0.3.2: mouse-look and cursor release

The release build, formatting and Clippy passed; **27 tests passed**. Mouse motion now uses relative counts with fixed sensitivity, independent of frame time and window dimensions. Three new checks cover look direction and pitch limits, equal rotation at different frame rates, and discarding absolute cursor jumps when releasing and recapturing the mouse.

Anode checks verified looking right, left, up and down without holding a button, and continued turning at the screen edge. F3 and P released the cursor; closing the sound panel or clicking to resume preserved the camera direction. Minimizing and Alt-Tab both paused the game and released the pointer outside the window. Returning kept the game paused, and clicking resumed mouse-look without a jump. The owned test window was closed and the lease released afterwards.

Focus detection uses Windows' UI-thread active-window and foreground-window handles because the current graphics libraries do not expose a focus query or automatically release the cursor. Held movement keys are suppressed after focus loss until released or pressed again. Automatic focus handling is Windows-only; other platforms remain unverified. These desktop checks used `--no-audio`; the earlier audio verification still applies.

## Version 0.3.1: unobstructed view and temporary help

The release build and Clippy passed. Anode checks confirmed a clear initial view with only a small H hint; H displayed the controls and countdown; the overlay, title banner and diagnostic footer disappeared automatically after the eight-second interval. Pressing H again dismissed it early. Walking remained active while help was visible, and P displayed a compact pause/resume reminder. F1 remains a compatibility alias for H. The owned test window was closed and the lease released.

## Version 0.3: sound and music

Build, formatting and Clippy checks passed; **24 tests passed**. The audio corpus check parsed all 36 level cues and 901 emitters, fully decoded 124 unique referenced files and reported one missing original ice-wand sound. Two stale ambient paths have explicit remaps to their existing exact-name files. See [AUDIO.md](AUDIO.md) for scope and commands.

Actual audio output was checked using the normal Windows session, because Anode returned `NoDevice`. The ten-second app-only capture verifies non-silent music/ambience, two footstep events, one jump and landing, silence during mute/pause and playback after resuming. The test also loaded skool2's different track and emitters. Anode verified the sound panel, saved volume/mute state across restart, movement and level selection without an audio device. This separation is intentional: no claim of audible Anode playback is made.

Logs and captures are under `private/audio-validation.txt`, `private/audio-output-check.txt` and `private/audio-output-check.wav`. No capture is included in source packaging. Scripted dialogue, combat/animation sounds, mood transitions, occlusion and reverb remain unimplemented.

## Version 0.2: collision and movement

- `cargo build --release --locked`, `cargo fmt --check` and `cargo clippy --locked --all-targets -- -D warnings`: passed on Rust 1.86.0, Windows x64.
- `cargo test --locked`: **17 tests passed**. Eleven new collision/movement tests cover thin walls, parallel contact, embedded starts, two-sided patch facets, falling and landing, jump height, low ceilings, wall sliding, corners, low/high steps, slopes, slope jumping, render-rate independence and retained jump presses.
- `--validate-all`: all **36 maps** passed, including constructing the static collision hulls and their acceleration structure. The original FTX/image and triangle checks still pass. Output: `private/collision-validation.txt`.
- `--physics-check --map skool1`: **900 fixed ticks passed** with finite state, no body overlap and no fall below the world bounds. The route walks more than 600 units down the corridor, strafes, jumps, lands on a raised area and comes to rest. The map uses 2,305 brush hulls and 3,840 curved patch facets. Output: `private/movement-route.txt`.

In Anode's hidden background desktop, the native release build was operated with real keyboard input. Walking advanced through the school, jumping cleared a raised edge, free flight moved upward, and F4 returned to walking. P stopped movement despite held W; the level chooser also paused movement while open. Home returned to the level start. F12 saved the new movement UI. The owned window was closed and the desktop lease released afterwards.

`private/movement-demo.mp4` is a local 18-second, 1200 Ã— 642, 30 FPS H.264 recording of the Rust window. It shows walking, strafing, jumping and looking around; sampled frames were visually inspected. It contains original game artwork and is excluded from the source package. It has not been published. No original executable was run, and no original-versus-Rust movement comparison has been performed.

The collision parameters and movement tuning are provisional. Whole-campaign traversal, moving objects, triggers, character animation, combat and gameplay are still absent. Passing the corpus check means collision structures can be built, not that every path through every map has been tested. See [MOVEMENT.md](MOVEMENT.md) for the implementation boundaries and format evidence.

## Version 0.1: renderer checks retained from the previous milestone

### Build and automatic checks

- Rust 1.86.0, Windows x64.
- `cargo build --release --locked`: passed; local executable in `target/release/looking-glass.exe`.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo test --locked`: six synthetic tests passed. They cover malformed/truncated headers, negative/out-of-file ranges, entity quoting/comments, FTX sizes and RGBA order, archive override/case behaviour and quadratic basis weights.
- `looking-glass.exe --validate-all`: passed for every mounted map and FTX image. Actual output retained in `private/validation.txt`.

Corpus totals: **36 maps, 1,007,917 stored vertices, 183,673 surfaces, 2,953 mounted FTX images and 2,138 parsed material definitions**. Patch tessellation was exercised for all maps, and each produced index was checked against its output vertices. These checks do not establish complete fidelity to the original renderer or gameplay.

### Ghidra

Official portable Ghidra 11.4.2 archive checksum verified as `795a02076af16257bd6f3f4736c4fc152ce9ff1f95df35cd47e2adc086e037a6`. Local headless analysis succeeded for the Windows executable, `cgamex86.dll` and `fgamex86.dll`. The saved project, logs and selected decompilations are in `private/`. See FORMAT.md for counts, addresses and the invalid automatic imports of the two non-Windows DLLs.

### Real desktop checks

Anode 0.9.0, background session 3, 1280 Ã— 720 desktop, native OpenGL window. The user's original executable was not run. The compiled Rust viewer was tested instead.

Interactive checks:

- Opened `skool1` and visually inspected geometry, textures, lighting and controls.
- Held W for one second; the view advanced along the corridor.
- F1 hid the help panel; F12 wrote a screenshot.
- Tab opened the level chooser; Down and Enter loaded `skool2` and visibly changed the environment.
- F2 toggled inspection lighting and F12 saved another image.
- Escape closed the owned window. No other seat applications were closed; the desktop lease was released.

Release-build render/capture checks each completed with exit code 0:

| Map | Rendered triangles | Draw batches | Missing image fallbacks |
| --- | ---: | ---: | ---: |
| skool1 | 25,207 | 99 | 1 |
| utemple | 69,672 | 79 | 1 |
| hedge3 | 128,245 | 313 | 0 |
| garden1 | 13,840 | 38 | 0 |

The two fallbacks correspond to empty material names in the input. Geometry totals differ from stored counts because curved surfaces are tessellated and non-world/hidden surfaces are omitted. Visible captures of `skool1`, `hedge3` and `garden1` were inspected; `utemple` completed its capture but has not received a detailed visual review. Screenshots are local under `private/screenshots/` and excluded from the source package.

The initial screenshots exposed a vertical row-order bug in screenshot saving. The saver now converts OpenGL's bottom-up rows to PNG's top-down rows, and the corrected `skool1.png` was visually verified. The live renderer was already the right way up.

### Limits of the renderer verification

No original-versus-Rust gameplay comparison has been performed. No full campaign, character, script, audio or save compatibility is claimed. Static collision was added in version 0.2 as described above. The initial `gvillage` view is inside its scripted opening area. Rendering on other operating systems, GPUs and language packs remains untested. The locked build and corpus checks demonstrate local milestones, not readiness for a public game release.

An actual v0.25/format-7 potears3 save was loaded and resaved in format 8 with identical player/resources and the new encounter/gates initialized. The original fixture remains in `private/duchess-legacy-v7`; logs are `private/duchess-legacy-{write,read}.log`.

## 2026-09-28 - Movement, event timeline and Club Guard damage surfaces

- 263 unit tests and strict all-target Clippy pass. Original-asset movement checks pass at 30/60/144 Hz, including render frames without a physics tick, blocked walking, jumping, ledge progress, rope/cart/airship clocks, pause and save continuation. Stationary platform passengers retain idle poses.
- Actual Alice walking samples on the bounded raised-floor fixture reduce summed stance penetration from 41.35 to 0.00 units without changing root translation or bone lengths. This measures that fixture, not every map slope or possible pose.
- Shared event checks cover chronological frame skips, repeated first/numeric frames, single entry/exit semantics, loop attachment history, optional arguments, ordered wildcard visibility, emitter state, finite lifetimes and resume without duplicate sounds.
- Native production renders pass for walking, blocked movement, jumping and climbing, plus the original Club Guard's intact/separated/falling/settled/fading/cleaned stages. Saved/restored pixel readbacks match. Intact actor readbacks remain identical after rendering severed actors sharing the model.
- Three full campaign saves (`guard-cut-before`, `guard-cut-live`, `guard-cut-expired`) pass separate writer/reader processes, exact restored state and continued simulation. These are staged persistence fixtures using the real school scene and native loader, not continuous combat traversal.
- Original animation audit: 48 Alice performances / 3,903 frames, ten ambient placements and 63 eligible NPC models / 179 clips. School combat checks pass at 30/60/144 Hz. Audio audit decodes the new death cue; its two pre-existing missing archive references remain reported.
- Native stress check passes all 78 full level replacements with overlapping scene, actor, attachment and particle resources. All desktop fixtures were isolated; their processes exited and the shared test desktop was released. Original archives and player saves remain unchanged.

The supported dismemberment recipe is lethal Vorpal Blade damage to the Club Card Guard, torso variant `death_3a` / `death_top`. Additional actors and head/limb variants remain future work. Bounds cover the authored detached animation, use swept collision and expire after five seconds; this is reconstructed debris behavior rather than the original engine's exact physics.


## 2026-09-28 - Power-up presentation, facial fidelity and weapon-only first person

- 288 unit tests, strict all-target Clippy and the release build pass. The launch executable and tested private candidate share SHA-256 `9E54CDFBBE4566ED3B151114A2E0FAEFDE6A737220CC34EB178BA4411DA7128E`.
- Original asset checks pass for the Rage (12.5 seconds) and Tea (6.1 seconds) sequences, four alternate textures, source weapon poses, interruption, repeated activation, frame skips, pause and legacy/restored state. The inventory remains authoritative for effect duration and gameplay.
- Native Rage/Tea captures cover start, growing, completed and expired appearances. Saving/restoring each staged transformation produces identical pixels; pause leaves the complete presentation state unchanged; death removes the form. Separate writer/reader processes pass the full `item-rage` and `item-tea` campaign fixtures, including exact restored state and continued simulation.
- At the user's request, first person renders only the equipped toy and its effects. The experimental third-person arm reuse was removed. Native captures pass for all ten toys in idle, attack and wall-adjacent poses, switching and view-state restoration. Blade, Mallet and wall-adjacent Staff captures were visually inspected. Unit checks retain eye-ray aiming, swept projectile launch safety and thrown-weapon visibility.
- Facial source checks pass all 366 lip envelopes / 42,378 samples and all 84 currently supported story lines. Eight character close-up renders pass, including Alice's blink. The original 10-degree default jaw range replaces the reconstructed 20-degree default; authored overrides and non-mouth animation channels remain intact. Explicit supported dialogue headwatch targets use bounded, saved, pause-aware attention.
- The full visibility regression suite passed on the preceding combined build before the weapon-only adjustment. The final candidate received the focused render/state tests above. No original-executable side-by-side or complete-campaign comparison is claimed. Blink/headwatch interpolation and first-person placement remain reconstructed presentation, as described in `FACIAL.md` and `POWER_PRESENTATION.md`.

Captures and logs are under `private/presentation-*` and `private/presentation-audit/`, outside the source distribution. Player saves and original archives were not modified by these isolated fixtures.


## 2026-09-28 - Missing characters after Tab level selection

The Tab/console loader prepared cutscene art against the outgoing level, then installed the incoming scene and dialogue. The new `Entered` package prepares and owns the destination art before any live state is replaced. Normal campaign exits use the same package.

- Reproduced the empty Village cutscene by opening the school, pressing Tab, selecting Home (first chapter), and pressing Enter in the previous executable. Repeated that actual menu sequence in the fixed executable and visually confirmed Alice during the opening. Isolated captures are `private/level-entry-audit/before-tab.png` and `after-tab.png`; they show different opening shots, not a pixel-aligned before/after comparison.
- Added a GPU regression to the regular visibility suite: school to Village, back to school, and into Village again. Both opening conversation renders contribute 41,742 character pixels. Alice and Cheshire are visible in the inspected capture. The check uses the same prepared destination package as gameplay.
- 291 unit tests, strict all-target Clippy, the release build and the full automated visibility suite pass. The suite completed in approximately 65 seconds and includes natural/skipped cutscenes, pause, saved/restored poses and gameplay handoffs.
- Tested/installed release SHA-256: `8D5C939578C48265863FB9BD948D79A5C0890512946440A42F763806CFC904F9`. Logs are retained in `private/level-entry-audit/`. The hidden desktop reported no audio output device; this pass verifies visual presentation and dialogue progression, not audible output. Player saves were not used or modified.

## 2026-09-28 - Changing skies, fog, rendering cost and Fortress arrival sky

Tested and installed release SHA-256: `CE0C89ED67185B86463C6C69CF0DA73F347E54542E90740270213C9427251715`. The previous launcher was backed up. The release build, 291 unit tests and strict all-target Clippy pass. `private/sky-performance/run-summary.json` records all six native checks passing on this exact executable: sky/performance, Fortress rendering, shared effects, all 36 maps, 78 complete level replacements and the complete actor/cutscene visibility suite.

- Reproduced the Fortress arrival's black background in the normal game path. The expanded sky aperture now includes the airship staging area. Eleven Fortress camera pairs cover the arrival, cave, exterior, landing, playable entry and interiors; only one channel differs by one 8-bit step. The normal game was also captured and visually inspected during the arrival and at the playable start. Images are `fortress-live-after.png` and `fortress-entry-after.png` under `private/sky-performance/`.
- Native source-sequence checks pass for Hedge 1/3 sky switching, Garden 2/4 fog fades and Hatter 1/2 moving miniature scenery. Pause/restoration passes with the documented bounded one-step pixel tolerance; serialized fog/sky state, trigger gates, interrupted fades, delayed motion, looping and shrink callbacks have unit coverage.
- Across twelve fixed views, layer-weighted submitted triangles fall from **935,703 to 94,865 (89.86%)**. Nine views have identical images; the other three differ at no more than 17 of 816,000 pixels, maximum RGB RMSE 0.002636 on the 0–255 scale. The portal's own visibility remains required in both variants. Fortress additionally retains required world PVS in its separate comparison because its newly opened sky shell must not reveal disconnected rooms.
- Ordinary benchmark views retained full model detail at the conservative 0.35-pixel threshold. The isolated original Pool prop keeps all near detail and removes four triangles at long range with identical pixels. Visibility culling supplies the substantial savings; these results do not establish a meaningful model-LOD speedup.
- The pickup fixture measures 1,636 contrasting model pixels without the plume and 1,641 with it. The source weapon model is visible in the inspected image. Collecting it removes its altar emitters, remaining puffs and light.

The following are the forward view of each map in the repeat profile, in milliseconds for rendering plus synchronized readback (three warmups, median of seven samples, 1200×680). All twelve repeat views improved; all twelve pairs are preserved in `profile.json`. These are local diagnostic timings, not GPU-only timings or gameplay FPS. The first pass had a Hatter timing outlier (28.41 to 51.92 ms); the same view repeated at 12.04 to 4.83 ms. Both runs are retained, so the variable wall-clock timing is not mistaken for a stable frame-rate guarantee.

| Forward view | Baseline triangles | Optimized triangles | Baseline ms | Optimized ms |
| --- | ---: | ---: | ---: | ---: |
| School | 56,071 | 17,123 | 7.64 | 5.45 |
| Pool of Tears | 113,083 | 19,012 | 35.57 | 4.98 |
| Hedge | 72,167 | 7,206 | 13.35 | 4.21 |
| Hatter | 72,664 | 14,089 | 12.95 | 5.70 |

See [SKY_PERFORMANCE.md](SKY_PERFORMANCE.md) for coverage and remaining approximations, including spline timing, fog density and portal lighting. This is representative sequence coverage, not a full campaign or original-runtime comparison. Original archives and player saves were not changed; all original-art captures remain private.

## September 28: flame orientation

The School fireplace's 72-by-70 quad selects a horizontal autosprite2 axis. The previous camera-facing basis reversed its top and bottom when viewed from the other side of its authored plane. The shared renderer now chooses the facing orientation nearest the authored edge, preserving mount points, dimensions and UVs for vertical, horizontal and tilted effects.

`--billboard-check --no-audio` passed on installed executable SHA-256 `6F93B693EBF4961B2CEC1C532F6BD84AD1EC2F43760699F9578D0962E56B06D5`. All 36 supplied maps were scanned: 272 flame surfaces across 15 maps passed 13,056 camera views. A further 24 GPU frames checked the actual School fireplace geometry from both sides and different heights, at three animation times, through both rendering paths. The diagnostic texture uses the same repeating sampler as the source material. Three captures with the original animated flames were saved; the front and angled views were visually inspected and are upright. Two focused unit tests cover ordinary tall flames plus the wider School flame, reordered vertices, offset UVs and tilted placement.

The test is included in `tools/test_render_fx.ps1`. Its report and screenshots are under `private/billboards/`; `private/fire-installed.json` records the executable and source manifest. This final build retains the preceding shared rendering and weapon changes. No original assets or player saves were changed.
## 2026-09-29 - Supported arrivals in Grounds 2 and Keep

Verified executable SHA-256: `7F606698F342F038136B67A593800D3EF9A7D0F4AF91E9732F0A970516328AAC`.
This candidate includes the preceding pickup and character fixes.

Keep now draws and collides with its arrival lift, rising 192 units over five
seconds and carrying a supported player. A blocked passenger stops its advance.
Its clock uses the existing registered-controller state slot; the outer save
version remains 12. Grounds 2 enters at `alicestop`, the authored cinematic
hand-off, because the collapsing approach sequence is still absent. This is a
bounded arrival correction; neither map's full scenes or route are complete.

- `--keep-check` passes ten seconds at both default and named entrances of both
  maps, including hazard checks, grounding, pause and a mid-arrival snapshot
  resumed into a separate world. Keep ends on its actual collision surface at
  z=120.03125; the larger BSP bounds include padding.
- Both levels were opened normally on the hidden desktop for 600 frames and
  visually inspected: Alice stays alive, and the Keep lift is visible beneath
  her. The 78 full level replacements passed.
- The complete native save writer/reader pair passes, including the added
  `keep-arrival` case and continued simulation after a process restart.
- 464 unit tests, formatting, strict all-target Clippy, source index audit and
  provenance pass. The registry baseline is unchanged on every unregistered
  visit. Keep's new controller snapshot differs as expected, while its event
  signature is identical to the previous baseline. No reserved hit range or
  existing event key changes.

Logs and captures: `private/codex-main-fixes/arrival-*`. All verification used
isolated saves/settings and disabled audio. Other late-level gaps, original
arrival staging and the wider recorded playtest backlog remain open.

## 2026-09-29 - Main playtest pickup and character corrections

Verified executable SHA-256: `0118586BC7ACAAF54C1900D4EFEAE3B5792AE7E3684B3FAF2A7C906F8A985C2C`.
This build includes the integration team's latest main merge (`f2f0176`). Root
`Launch.cmd` prefers the checked private playtest executable when no packaged
executable exists, using the separate playtest save folder.

- Placed Meta Essence now uses the same original animated materials as drops;
  health and Will pickups use their assigned original models. Native checks
  compare all four Essence grades at two times, removal after collection,
  occlusion and fog. The original-art captures remain private.
- Held Dice draws one die; ownership count and summon behavior are unchanged.
  Mesh attachments can coexist with emitters, restoring the gnome's pipe. The
  Pandemonium pipe pose was inspected; smoke fidelity remains open.
- Pandemonium now supplies resolved climb direction to Alice's animation.
  Climbing selects up/down clips; vertical travel caused by swinging retains
  the hanging clip. The transient direction is not serialized.
- 463 unit tests, strict all-target Clippy, formatting, the source index audit
  and provenance checks passed. Focused animation, inventory, loadout,
  traversal, Pandemonium and registry checks passed. Strict Normal and Easy
  chains passed through visit 6 before the final presentation-only climb
  correction; this does not establish later campaign completion.
- The final executable passed the hidden-desktop item, actor visibility,
  billboard, shared-effects, 36-map rendering and 78-level replacement suites.
  The unfiltered save writer/reader passed in separate processes. Seven real
  strict-chain boundary saves also survived fresh-process reload and resave.

Evidence is under `private/codex-main-fixes/`. Runs used isolated settings and
saves with audio disabled. Rope flexibility, later-map arrival safety, breath
sources, water-edge traversal and the other recorded playtest issues remain
separate work; these checks do not establish original-game visual parity.

## 2026-09-30 - Water Logged

Published executable SHA-256:
`30F764CF8060E6FD1F64A53D22C46CB096F45023B1F49A83F70567CD3FC6C6C7`.
Built from the frozen Crazed Clockwork A9D42A44 release plus the Tower changes,
preserving earlier published restorations. Save envelope 12 and Tower state
version 1 remain compatible with existing rising-water progress. The regular
root launcher selects this build; its isolated eight-frame smoke opened Tower2
and closed cleanly. `tools/launchers/Launch-Water-Logged.cmd` provides separate playtest saves.

- 625 unit tests passed. The Tower asset contracts cover introduction watched,
  skipped and restored at 30/60/144 Hz; supported return pose; shell and air
  sources; twelve moving objects; ordered water stages; lid sway; flusher rider
  carry and return; lethal swept fan contact; Watch exceptions; current timing;
  legacy state; pause and saved continuation. Cheshire's appearance/departure
  cues fire once at the intended time and are not replayed by loading.
- Both fresh Normal routes complete all three flushers, pipe passages, live
  fans/currents, native Snark combat, the large essence pickup and the real dive
  into `hedge3$hedge3_start1`. Neither uses god mode, notarget, teleport or staged
  progression. Both finish at 75 sanity with about 13.9 seconds of air and no
  drowning damage. The next-map arrival is collision-clear and keeps resources.
- Each route continues from ten disk saves, comparing identical 120-tick live
  futures. Four additional native Store fixtures pass in separate writer and
  reader processes: introduction, first rise, second-stage machinery and final
  opened shaft (writer 60136, reader 56772).
- All five waves independently activate the expected 3/6/5/4/2 distinct Snarks,
  preserve their saved future and do not resurrect after death/retrigger/load.
  The full route is not an exhaustive clear of every optional encounter.
- Four water heights pass visible-surface checks and three introduction frames
  were captured. The final water view and Cheshire appearance were inspected.
  Native checks used isolated desktops/settings/saves and muted audio.
- Before the final sound-only correction, A41A9841 also passed ledge, swimming,
  rope/traversal and both complete Garden2 routes. Candidate12 preserved the
  Hatter1 machinery/traversal and Hatter2 battle contracts. Shared movement and
  world-clock behavior did not change after those regression checks.

Evidence and the verified 380-file source manifest are recorded in
`private/tower2-work/release.json`. The original-text provenance scan passed for
2,574 added source lines; changed Tower/ledge files pass formatting. Normal
standalone traversal is verified; the full campaign chain, other difficulties,
manual keyboard/F5/retry interaction and audible playback remain unclaimed.
See [Water Logged](TOWER2.md) for the bounded lid-sway, sink-law resampling and
bubble-bounds fidelity limits.

## Labyrinthine Revenge machinery (2026-10-01)

Published executable SHA-256:
`D654E222596C3099B131B39968C4EFC54B2BA5B24DFCB1E3A5A73F94BCD8E784`.
The frozen 416-file source retains Battle Royale
`9a5a8a3266be6bec8d44d45ff437ddd3c5bd0487af8fb55cbaa2bacba31a540c`
and its preceding restorations. Save envelope 12 and Hedge3 controller version 1
are unchanged. The unrelated dirty checkout was not packaged.
Both `Launch.cmd` and `tools/launchers/Launch-Labyrinthine-Revenge.cmd` passed isolated
eight-frame viewer startup/render smoke checks and closed cleanly.

- 632 unit tests passed. Asset contracts cover moving brush/patch collision,
  rider carry, raised teeth, rotating-face separation, blocked crush damage,
  paired doors and actor masks, door/chain sounds, Watch freeze, malformed state,
  all 20 sky-trigger volumes and 30/60/144 Hz continuation.
- The native Normal route covers arrival, the lava gear and weighted platform,
  both bellows, vent climbs, all ride gears, crushers, hammers and the upper
  rolling wheels. It takes BSP exit #101 into `tower3$tower3_start1`, checks a
  collision-clear arrival and exact carried resources. It finishes with
  100.00 sanity and zero teleports. Native enemies, lethal hazards and ordinary
  movement/combat remain active; there are
  no edited resources, god mode or notarget.
- The proof joins a fresh arrival prefix to unedited live disk checkpoints.
  Its final baked input matches those joined segments. It includes 76 disk
  continuations with identical per-tick idle futures for up to 120 ticks, or
  identical death when waiting exposed is lethal. The real route resumes the
  checkpoint; the comparison does not force Alice to stand under fire. This
  is a checkpoint-continuation proof, not one uninterrupted process.
- Four production Store fixtures (gears, gust, slam and sink) passed on the
  final build in separate writer and reader processes, with exact state and
  continued simulation. A separate legacy viewer smoke recovered an old
  controller-less save at the entrance with its saved loadout visible.
- Native rendering captured five machinery views and all seven skies. Bellows
  emission was checked for enabled state and actual live particles; machinery
  and sky images were inspected. Original-artwork captures remain private.
- Tower2, Garden1, Jlair1, Hedge2 and Grounds2 contract regressions passed.
  Compared with the latest selected baseline, all 38 other visit snapshots and
  all 39 event signatures match. Integration also fixes that baseline's missing
  reservation for `--grounds2-save-check`; its previously failing unit test now
  passes, without changing hit-ID ranges or visit order.
- The working project compiles, owned Rust files pass formatting and the scoped
  source provenance scan has zero matches. Frozen sources, replay segments and
  hashes are recorded in `private/hedge3-work/release.json`.

Exact original pendulum integration remains approximated by a bounded
three-second sinusoid at the authored amplitude. Full campaign-chain traversal,
manual keyboard/F5/retry interaction, other-difficulty route completion and
audible playback are not claimed. See [Labyrinthine Revenge](HEDGE3.md).

## Machinations machinery (2026-10-01)

Executable SHA-256:
`AB73CC2A9E32FB5B257FE56472F08993FA4B09F8E0A3A4F7EBAFD15BE2731FA1`.
The isolated build retains the published Mirror Image `6C9850B5` release and
its preceding restorations. Only the Tower3 controller, diagnostics and registry
attachment change compiled source relative to that release. Save envelope 12
is unchanged; the new Tower3 controller uses version 1.
Both `Launch.cmd` and `tools/launchers/Launch-Machinations.cmd` passed native eight-frame
startup/render checks and closed cleanly with the selected executable.

- 634 unit tests passed. Asset contracts cover all 49 controller objects,
  duplicate gear names, full bind poses, solid/render agreement, saved group
  clocks, Watch pause, free carry versus blocked crush and malformed saves.
  Bound riders complete the entire six-second pedal cycle and 24-second arm
  cycle, including both reversals and the raised parent lip.
- Both fresh Normal routes use production movement and collision, with the
  Cheshire introduction watched and skipped. Each completes 19,263 ticks and
  34 jumps, collects three essences and uses exactly the three authored fall
  returns. Each reaches exit #17 alive at 93 sanity and 100 Will, then enters
  `grounds1$grounds1_start1` collision-clear with identical resources and the
  Royal Rage introduction active. No god mode, notarget, resource edits,
  extra teleports or recovery are used. Tower3 authors no enemy actors.
- Each route verifies 46 disk continuations, comparing up to 120 identical
  idle ticks (or identical death) before resuming the saved route point.
  Production recovery independently passes at all three room destinations,
  preserving machinery clocks and placing Alice safely above the floor.
  Watched and early/late skipped scene outcomes agree at 30/60/144 Hz.
- Four native Store fixtures pass in separate writer and reader processes on
  the final executable: lift, pedal, room three and active introduction. A real
  viewer F5/load/F5 test upgrades a controller-less legacy save, preserves every
  resource field and safely returns to the entrance. That upgrade was tested
  before the final, Tower-only arm carry correction.
- Five Tower machinery/introduction views and an ordinary Royal Rage intro
  capture render without fallback textures. These are rendering checks; the
  earned next-map transition is established by the movement routes above.
  An additional close view confirms the seated Cheshire model is visible.
  Native checks use the hidden desktop, isolated settings/saves and muted audio.
- All 38 other visit snapshots and event signatures match the selected Mirror
  Image baseline. The all-39-visit graph and Tower2, Hedge3 and Funhouse contracts
  also passed before the final Tower-only arm correction. The dirty working
  project compiles, owned source passes formatting, and the scoped original-text
  provenance scan reports zero matches in 2,845 added lines.

Frozen source, exact hashes and test evidence are recorded in
`private/tower3-work/release.json`. The two fulcrums remain solid and static.
Exact original pendulum integration and pedal-plate orientation remain fidelity
follow-ups. Manual full keyboard traversal, audible playback, other difficulties
and a full campaign-chain run are not claimed. See [Machinations](TOWER3.md).


## Airborne Terror release 52DD22B8 (2026-10-01)

Airborne Terror build 52DD22B8 restores the Cheshire introduction, animated blow faces and physical gusts, six-second occupied trigger repeats, ten native Boojum ambushes and actor launch pads. Root Launch.cmd selects it; tools/launchers/Launch-Airborne-Terror.cmd opens a fresh visit with separate saves. It retains Machinations AB73CC2A and all preceding published work.

Both fresh Normal routes, with the introduction watched and skipped, traverse the entire tower and load the actual Hedge2 entrance alive: 58 Sanity, exact carried resources, no recovery teleports, no lost or lethal-hazard ticks. The production cast fights normally; no health grants, removed enemies or reduced combat are used.

Verification: 635 unit tests, root compilation, Tower1 and retained Tower3 contracts, four separate-process native Store futures, all-difficulty activation/death/reload tests, seven staged captures and real keyboard pause/F5/process-restart/held-Enter/F9 checks pass. A prior-release native save keeps position, resources and all ten ambush actors; the intro is treated as finished and the sealed precache actor is hidden. The 39-visit exit graph passes; all38 other-visit snapshot hashes and event signatures match AB73CC2A. Save12, existing event keys, route order and hit reservations are unchanged. Scoped provenance: 2159 added code/data/documentation lines, no six-word original-text matches.

Limits: full-route proof is Normal difficulty only, not a full campaign chain or manual keyboard traversal. Actor post-pad decay, existing updraft/vent cadence and sub-frame presentation remain fidelity comparisons. Audio was unavailable. Inherited Mirror Image full combat-course certification remains open at its pendulums. The earlier broad opening chain's skool2 navigation issue is not claimed fixed here.

Details: docs/TOWER1.md; private/tower1-work/WORKLOG.md, release.json and release-source-manifest.json. Compiled inputs are frozen in private/tower1-work/release-source-52dd22b8. Shared uncommitted files and the Git index are preserved.
