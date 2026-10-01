# Campaign replay audit — 2026-10-01

Five automated campaign lineages have completed all 39 visits: three Easy and
two Normal, with the Normal scenes watched and skipped respectively. One Easy run
was uninterrupted from New Game on candidate 33. The other four used earned
checkpoints while defects and replay inputs were corrected.

The two gameplay repairs below were published in `C76118C3`. During final
verification, the presentation update `92CC824C` became the selected root
`Launch.cmd` build. Its parent is `C76118C3`; both repaired gameplay files match
exactly. The newer presentation build is preserved. These campaign runs test the
recorded candidates, not a fresh campaign on that later presentation build.
Candidates 26–42 change replay inputs and diagnostics, not gameplay.

## Method and results

The native automated replay uses ordinary player controls, paid weapon actions,
real enemy damage, carried inventory, actual gates and exits. Each arrival goes
through the real save store and restoration checks. No campaign run uses god mode,
notarget, resource grants, edited enemy health or forced quest completion.

| Replay | Passed visits | Evidence and scope |
| --- | --- | --- |
| Easy first | 1–39 | `fixed-easy-watch`; cumulative earned-checkpoint lineage |
| Easy second | 1–39 | `fresh-final-easy`; cumulative earned-checkpoint lineage |
| Easy fresh, candidate 33 | 1–39 | `fresh-complete-easy33.log`; one uninterrupted invocation, 1,693.733 seconds |
| Normal watched | 1–39 | Cumulative earned-checkpoint lineage; final segment `normal-keep-door42.log` |
| Normal skipped | 1–39 | Cumulative earned-checkpoint lineage; final segment `skip-keep-door42.log` |

Evidence is under `private/campaign-runs-20261001/`. `completion-index.json`
records the lineages. The fresh candidate-33 report contains all 39 ordered,
passing rows and no mid-route continuations. Checkpoint continuations retain their
original entrances and unedited saved resources. Historical replay logs sometimes
say “all 39 visits” after a late continuation; use the invocation's start visit
and report rows to establish its actual scope. The current message distinguishes
full invocations from continuations.

The first earned ending save was also loaded in the actual viewer. The ending
film played without skipping, credits appeared and the completion marker was
written. Evidence: `ending-verification.json`, `ending-credits-watched.png`,
`ending-viewer-live.log`. The earlier paused capture is not ending evidence.
The other four runs earned ending saves; their films were not separately watched.

## Gameplay repairs available to try

**Pool moving-leaf edge support.** Rider detection used a one-unit horizontal
probe although player movement supports Alice with her full collision footprint.
At a leaf edge she could stand on the leaf without receiving its motion.
`src/pool.rs` now uses the full footprint, retaining body-clearance, relative
velocity and support-normal checks. The authored leaf-3 regression fails before
and passes after at 30, 60 and 144 Hz. All four leaf rides, pause and save
continuation pass (`pool-edge-before.log`, `pool-edge-after.log`, `pool-final.log`).

**Hedge rotating-axle false crush.** An earned visit-33 run died where a rotating
`gearwithspikeaxle` pressed Alice slightly into the fixed floor. Floor projection
alone still intersected the sloping tooth. `src/levels/hedge3/motion.rs` resolves
both contact constraints within the existing 20-unit separation limit, accepting
only a fully swept path and clear endpoint. Truly blocked contacts still crush.
The actual-map regression fails before and passes after; genuine pinning, rider,
pause and saved-phase checks also pass. The unchanged earned bellows checkpoint
passes at full health with the repair.

The published executable and parent hashes, with its frozen source inventory,
are in `release.json`. Save format 12, event-rule keys, campaign visit numbers and
reserved hit-ID ranges are unchanged.

## Replay corrections

These changes improve what the automated player does; they do not weaken combat
or replace the real route with scripted success.

- Pool and Dry Landing: ordinary jumps clear two route obstructions. Whole-visit
  Easy checks and the fresh campaign pass with the final inputs.
- Return WForest, Maze and Hedge2: use real supplies, wait for actual Watch
  recharge at safe entrances, release held fire before selecting it, regain
  contact with the companion, and stay on the high bank. Escort and exit gates
  still depend on their real world state.
- Pale Realm and Hatter: select already-owned Ice for the close fights.
- Funhouse: use difficulty-appropriate bank timing, defend with ordinary inputs
  and jump from the final bank. No Staff or Watch is granted before acquisition.
- Hedge3: approach the bellows at the appropriate phase, jump directly from the
  moving gear into actual steam, and keep Ice active across the shorter upper
  gear arc and pull-up. A diagnostic trace established enemy knockback, not
  broken rider carry, at the later upper-gear fall. Both older and new builds
  reproduced that fall. No new physics repair was justified there.
- Royal Rage: jump/pull up to the existing health supply instead of walking into
  its lip. Both low-health Normal entrances now pass the whole encounter.
- Battle Royale: use the owned Watch before the opening guards, fight nearby
  enemies and collect their real drops before further reinforcement waits.
  Normal still requires 30 guards; Easy uses its authored 15-guard quota.
  Whole Normal and Easy default checks pass (`verify-battle-normal37.log`,
  `verify-battle-easy37.log`). Both Normal lineages finish with 60 Sanity and
  approximately 68 Will; no lava exposure or recovery teleport occurs.
- Ascension: wait for the carried Watch to recharge and reserve 2 Will by
  withholding weapon spending. Both complete Normal visits pass with 57 damage
  taken, 84 Sanity remaining and roughly 3 Will. The final Easy late-route check
  also passes (`verify-late-easy39.log`, visits 36–39).
- Castle Keep: the Normal entrance carries 84 Sanity and roughly 3 Will. The
  latest replay recharges the earned Watch before combat and lets the contact
  door open before stopping world time. Both Normal watched/skipped whole visits
  and the Easy check now pass, followed by the real final encounter. Evidence:
  `normal-keep-door42.log`, `skip-keep-door42.log`, `verify-keep-easy42.log`.
  The old isolated puzzle test's invulnerability is not used by the campaign driver.

## Waterside rope investigation

The user's rope could not be grabbed from water, possibly in a Turtle level.
The static inventory covers 27 placed ropes in seven maps; the live-world probe
checks 18 in `potears1`, `garden1` and `garden2` using the player-facing water and
grab code. These staged probes are diagnostic evidence, not campaign traversal.

A plausible match is `garden1` entity 482: anchor `(-2412,-636,168)`, length
456, resting tip Z -288, sampled water surface Z -384. A stationary swimmer's
eye is about 92 units below that tip, beyond the current manual reach of 76.
The campaign can grab it from its land/current approach. The visible strand uses
the complete authored length, and the original native endpoint construction
agrees. No shortening bug is established.

The original state data uses airborne rope contact; its swim state has no direct
rope transition. Native contact bounds and the precise emergence behavior still
need location-specific confirmation. No rope length, grab reach or water-jump
rule has been changed on this evidence. The exact reported rope remains
unconfirmed. Paraphrased research with claim confidence and native addresses is
in `WATERSIDE_ROPE.md`; measurements are in `rope-archive-inventory.json`,
`rope-water-live.log` and `private/traversal-check/water-ropes.json`.

## Verification and limits

The fresh run retains material fallback diagnostics for empty identifiers in
`fortress1`, `skool1`, `utemple` and `rchess1`, and `textures/notexture` in
`funhouse` and `hatter1`. Those diagnostics need surface-level visual triage; a
route pass does not establish that these materials are visually correct.

Candidate 42 passes 665 release unit tests, with one ignored. Its scoped
added-line provenance scan covers the 26 owned source/data files and this audit,
with zero matches to original-game prose (`provenance42.json`). All 39 registry fingerprints and the 39-visit
exit graph also pass; those static checks alone do not prove campaign completion.
Frozen source and executable hashes identify each tested candidate.

The automated routes and scene-owner checks are not a complete visual, camera,
facial-animation or sound audit. Native runs and the ending check used
`--no-audio`; no listening was performed. Save/pause and watched/skipped evidence
must be attributed to the specific checks, not generalized to every scene.
