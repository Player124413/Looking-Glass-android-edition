# Shared residents and plant enemies

The shared NPC loader now gives later-map Boojums, Ladybugs and Diamond guards
their existing combat controllers, plus new Bloodrose and Evil Mushroom combat.
Opening encounter owners still control their own cast. Script models, hidden
story actors and registry-owned NPCs are not promoted into ordinary combat.

## Behaviour

Boojums fly, react, scream with swept wave contacts, freeze and fall/shrink on
death. Ladybugs follow reviewed closed patrols, release acorns, rearm, react,
freeze and crash. Their attached acorn disappears after release. Diamond guards
use the existing collision-aware pursuit and ranged projectile controller.
Each family retains its original animations and death reward size.

Bloodroses begin small and invulnerable. They grow over 0.7 seconds when the player
is visible between 100 and 768 units away, then become damageable. Their source
health is 70. The melee contact deals 25 at frame 14; a single thorn fires at frame
10; the five-shot fan fires at frames 11, 14, 17, 20 and 23. Thorns travel at 600
and deal 10. They originate at the animated barrel tag. Rose knockback is disabled
as in the supplied data. Death awards Small essence.

Evil Mushrooms wake within 256 units, have 125 health and a 25-point pain threshold,
and spit a 600-speed, 10-damage spore at frame 14 from the animated skull tag.
Spores use the supplied spore emitter rather than a visible thorn mesh. Suction
works inside 416 units with clear sight; a close grab transitions into chewing
and release. Cover, distance, pain and death interrupt it. Death awards Medium
essence. Both plants use original pain/death variants and frozen clips.

## Placement and persistence

Ordinary eligible placements respect difficulty flags. Reviewed direct touch
triggers and script-free relays can activate supported dormant actors once.
The new slots are appended after existing actors so older reward identities do
not shift. Garden1's spawn-only Ladybug callbacks and wake-up triggers are
reviewed individually. Pool2's paired callbacks start the existing Ladybug patrols.
Garden1, Garden4 and Centipede1 use the reviewed source spline routes.

Saves retain activation delays, hostility, health, attack contacts, projectiles,
patrol progress, death state and reward identity. Old decorative saves acquire a
controller; appended ambush actors absent from an old save reset dormant. Plant
position and route identity cannot be altered by the saved controller. Pausing
stops combat and trigger activation. Simulation runs in fixed 120 Hz steps.
Each plant owns at most 12 five-second projectiles; cosmetic spore caches are
removed with their projectile and cleared on restoration or level replacement.

## Approximations and remaining work

These are playable replacements, not execution of the original AI scripts.
The original Mushroom `Digest` engine implementation is unavailable: this version
uses three 5-damage bites over 1.5 seconds and a release impulse. It does not lock
or teleport Alice into the mouth. Suction is limited to two seconds. The Rose's
fan uses the authored numeric offsets as degrees; original projectile leading,
random spread and blind fire are not reproduced. Plant projectiles use centered
swept boxes with a five-second resource limit. Impacts do not yet have original
decals. Plant bodies are stationary and non-solid to Alice.

Existing Boojum, Ladybug and Diamond controller approximations remain. Later
Boojums use local pursuit, including Hatter1's scripted patrol actor. The mixed
Garden1 Ant/Ladybug ambush and initial Ladybug3 spawn, Garden2's cinematic-linked
spawns and ambiguous duplicate Ladybug5 routes still need scene-specific work.
Unreviewed script callbacks, recurring waves, encounter death targets, enemy
patrol scripts and puzzle-gated spawns are not claimed restored by this change.

`--resident-check` verifies supplied clips and attack/pause/save/freeze/retirement
at 30, 60 and 144 FPS, plus map/difficulty placements and trigger identities.
`tools/test_residents.ps1` exercises production drawing and fresh-process save
continuation. Native captures are staged in original maps; they are not proof
of natural campaign completion. Local assets, saves and visual evidence stay
under ignored `private/residents`. Native tests disable audio.

Verified locally on 30 September 2026: 535 unit tests, 76 map/difficulty audits,
37 staged native captures, and 26 exact fresh-process continuations, including
dormant actors, pending activation and reward uniqueness. The merged main project
compiles for all targets. All 63 model definitions load (357 clips), and all
39 registry visits build deterministically; no stored registry golden was present.
Ant/Chess/Fire Imp, visibility and 78 full-level replacement checks passed on
the preceding candidate of this batch. The final candidate additionally fixes
Diamond corpse retirement and expands plant/activation checks. Lint verification
uses the same four pre-existing category allowances recorded in CLOCKWORK.md.

Final candidate SHA-256: `82c5e2ef` (full identity in the private evidence manifest).
`tools/launchers/Launch-NPC-Roster.cmd` opens Garden1 in this separate candidate with isolated
saves. It includes the preceding Ant, Chess, Fire Imp and Clockwork work; ongoing
main-project work from other chats remains in the main source checkout.
