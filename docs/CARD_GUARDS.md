# Heart and Spade guards

Heart and Spade placements now use shared combat controllers instead of decorative
animation. Original assets remain in the user's local installation. No original
AI or map script executes in this implementation.

## Implemented behavior

Hearts have 200 health, a 35-point pain threshold, four melee variants and the
begin/middle/end multi-hit combination. Their charged and ground-strike attacks
launch seeking Heart projectiles. Spades have 160 health, staff strikes, the
two-contact spinning attack, and paired ranged shots. Both detect visible targets,
turn, pursue with collision and ledge checks, react to hits and knockback, and can
fight the summoned Demon. Spades flee locally below six health. Grounds2's two
placed Heart guards retain the script's 4,000-unit sight range.

All melee/projectile contacts use the source clip frame times. The Spade muzzle
is derived from the attached staff's `tag_barrel`, transformed through the actor's
animated `tag_weapon`; it is not a fabricated bone in the guard skeleton.
Projectiles move at 850, live at most five seconds, and deal 20 (Heart) or 13
(Spade) direct damage. Swept collision stops them at actors and world geometry.
Impact splash excludes the direct victim and checks cover. Original projectile
meshes, trail emitters, Heart charge emitters and red dynamic lights are drawn.
Sound asset paths resolve locally; native automated tests disable playback.

Normal deaths play source variants, followed by Heart shrinking or Spade sinking.
Ice holds the original Heart frame 9 / Spade frame 6. Knife-killed Spades play
`death_gib`, separate the authored upper surfaces with their cap, remove the
attached weapon, and animate a swept detached torso for at most five seconds.
Heart deaths award Large essence and Spades Medium through the existing reward
ledger, once per enemy.

## Placement and persistence

Ordinary placements are supported in Wforest, Hedge1/2/3, Facade, Grounds2 and
Keep. Selection uses the supplied model, including Hedge2's Spade models labelled
as Diamond entities. Direct touch triggers and script-free relays activate their
dormant guards once and respect difficulty and delay. New ambush slots append
after the previous resident families, preserving existing save/reward identifiers.

Saves retain health, position, facing, pain, attack contacts, cooldown, hostility,
projectiles and their intended homing target, impacts, recoil, activation and
detached pieces. Old decorative guards acquire controllers. Missing appended
guards reset dormant, while existing resident families keep their state. Pause
stops the fixed 120 Hz simulation. Per actor, shots and impact records are capped
at 12; particle caches follow live shot IDs and reset on restore/level replacement.

## Deliberate approximations and outstanding work

Local collision steering replaces original pathfinding, patrols, hearing and
weapon-aware tactics. Hearts do not yet flee from the Jabberwock's Eye Staff.
The original homing `seeker 7` has no recovered angular-unit definition: turning
is bounded to two radians per second here. A disappeared Demon is not replaced
as a projectile's homing target. Trail history after a restore uses the current
projectile direction as an approximation of its earlier curved path.

Splash uses the repository's 110-unit inferred default radius and conservative
world occlusion. Short impact flares stand in for nested source explosion meshes;
original decals, weapon swipe ribbons, blood spurts, dropped staff physics and
random whole-body gib deaths remain presentation work. The conservative torso
collision box is lifted clear of the floor at birth; a piece is omitted when
nearby walls leave no safe room for its full animation bounds. Spade pain responds to
each hit outside an existing pain animation. Source sound timing/loops are
approximated by state-entry cues.

Grounds2's recurring script waves and kill-thread accounting, Keep's cinematic
waves, and unreviewed puzzle/cutscene-linked activations remain scene work. This
does not claim every authored Heart/Spade appearance or the complete NPC roster.

## Verification

`--resident-check` now covers both guards alongside the previous five resident
families: local source facts and real clips, 30/60/144 FPS combat, pause, exact
save continuation, ice and retirement, and 76 map/difficulty placement audits.
The Card unit tests cover individual contacts, dodging, walls, targeting, homing,
pain thresholds, malformed saves, projectile/death cleanup and continuation.
`tools/test_residents.ps1` adds production native captures and separate-process
save write/read checks. Captures are staged original-map views, not a natural
campaign playthrough. Evidence remains under ignored `private/cards`.

Use `tools/launchers/Launch-Card-Guards.cmd` for the separate verified candidate, starting in
Hedge3 with isolated playtest saves. Main-project source integration is checked
separately so concurrent campaign work is retained.

Verified on 30 September 2026: 546 unit tests; 76 map/difficulty audits; 56 staged
native captures; 38 exact fresh-process continuations and reward checks. The
original Spade torso also separates, restores and expires in the clear-floor
source-data fixture. The selected wall-adjacent Spade capture exercises safe
omission when the conservative animated piece cannot fit. The Heart camera has
a foreground ledge that partly hides its lower body.

All 63 models load with 392 clips. Ant, Chess, Fire Imp and Clockwork regression
checks pass, and all 39 registry visits build deterministically twice (no stored
golden comparison). The merged main project passes its all-targets build check.
Lint checks pass with the four pre-existing category allowances documented in
CLOCKWORK.md. Audible mixing and a natural campaign route were not verified.

Candidate SHA-256:
`3f9292bb071655cd64d1ab304c967c10315baeaf5c6b60df040785e059533475`.

The automated actor-visibility suite and 78 full level replacements also pass
on this candidate, including shared NPC, particle and material state.
