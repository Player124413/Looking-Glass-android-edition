# Snarks

Water Snarks, bite-only Snarks and Fire Snarks now have shared combat controllers.
Original assets are read locally; original AI and map scripts are not executed.

## Behaviour

Water/bite-only variants have 25 health; Fire Snarks have 100. They detect visible
opponents within 1,000 units, turn and swim toward them at 240 units per second.
Pursuit and knockback stay in connected liquid and sweep the actor's box against
solid geometry. Water Snarks use water; Fire Snarks accept water and lava. Dry
placements fall under gravity rather than flying across a bank. Dynamic liquid
membership is checked each step, including when a pool moves or disappears.

Bites deal 10 damage once at source frame 8 within 120 units. Water and Fire
Snarks can rise, spit at a dry target within 550 units, and dive back. Within 250
units they can also use the tongue clip and a bounded, interruptible pull.
Bite-only Snarks never gain these ranged attacks. The animated mouth supplies
the projectile origin; a swept muzzle check prevents shots appearing past walls.
Acid globs travel at 600, deal five direct damage and three five-point delayed
ticks. Fireballs travel at 700, deal 25 and expire after two seconds. World/actor
contacts are swept and shots can hit Alice or the summoned Demon.

Nonlethal hits interrupt attacks with pain and knockback. Death plays an original
clip and shrinks away; ice holds frame zero and uses the shared frozen material.
Each defeated Snark awards Medium essence once through the existing ledger.
Original acid meshes/trails, fireball particles, Fire Snark smoke attachments and
fireball lights are supported. Sound paths are resolved by the shared audio system.

## Saves and placement

Ordinary placements cover Pool of Tears, Garden1, Centipede1, Hedge2/3,
Jabberwock's Lair, Wforest, Tower2 and Utemple. Direct touch triggers and
script-free relays activate one-shot dormant spawns with difficulty/delay checks.
New spawn slots append after the previously shipped residents and Card guards,
preserving existing actor/reward IDs. Garden4 script-model fish remain scenery.

The shared 120 Hz simulation pauses with the game. Saves retain phases, contact
flags, cooldowns, recoil, rise/dive state, projectiles, acid ticks and activation.
Old decorative actors acquire controllers; existing neighbours retain their
saved state. Shots are capped at four and acid records at two per Snark. Dead
clocks stop, shots/ticks expire, and per-actor particle histories are pruned or
cleared on restore and level replacement.

## Approximations and remaining work

Swimming uses local steering and axis sliding, without original wander/path
search. Surface attacks rise vertically by at most 192 units and then fall back;
this replaces the unavailable engine TongueGrab behaviour. Tongue contact uses a
short pull and a simple strand; it does not imprison or teleport Alice. The
original acid overtime command's scheduler is unavailable: three one-second
ticks are the explicit replacement. Acid direct collision uses a conservative
centred hull. Acid decals, blood sprays and random gib variants remain deferred.

Water Snark source pain thresholds exceed their entire health pool; this build
deliberately responds to nonlethal hits with a 0.4-second debounce. Fire Snarks
have no authored frozen alias; their first death pose supplies the ice result.
Fire Snarks receive Medium essence through the shared fallback (their source
manatype declaration is commented out). Sound timing is approximated by attack
and reaction cues. Fire smoke histories restart at phase boundaries; trails after
loading reconstruct from the current position/direction. Unreviewed scripted
waves and scene callbacks are still separate campaign work.

## Verification

`--resident-check` covers local source facts, actual models/clips, combat at
30/60/144 FPS, pause, exact continuation, deaths and bounded retirement. It also
audits resident placements and trigger identities at every difficulty.
Snark unit checks exercise liquid boundaries/removal, walls, bite cues, ranged
restrictions, rise/dive, tongue cancellation, acid ticks, Demon targeting, saves,
malformed state and cleanup.

`tools/test_snarks.ps1` runs the native render and fresh-process save checks in
the background desktop. Captures are staged views of original maps, not a
natural campaign playthrough. Automated native checks disable audio. Evidence
and the separate candidate remain under ignored `private/snarks`.

`tools/launchers/Launch-Snarks.cmd` starts the separate candidate in Hedge2 with isolated saves.
The working main project's concurrent changes are integrated separately.

The Fire Snark capture is framed at the surface of its actual lava pool; its
original deep idle placement is hidden by the opaque lava. Captures compare the
visible idle body/effects against retirement, in addition to manual inspection.

Verified on 30 September 2026: 558 unit tests; 84 map/difficulty resident audits;
57 ordinary Snark placements settle in the correct liquid; all 63 NPC models
load with 421 clips. Original-data bank fixtures also verify acid/fire shot
release and damage using the real animation durations and animated muzzle.
Ant, Chess, Fire Imp and Clockwork regressions pass. All 39 registry visits build
deterministically twice (no stored golden comparison). The integrated main
project passes its all-targets build check. Lint passes with the four existing
category allowances documented in CLOCKWORK.md.

Native verification also includes 29 staged captures, 18 exact fresh-process
continuations/reward checks, the actor visibility suite and 78 full level
replacements. Fire Snark ice captures explicitly require the frozen material
and visible ice-coloured pixels. Audio playback and a natural campaign route
were not verified.

The final candidate is
`0adbd1b6751df22b2d41c59791e3b6f22264018dbca82bbf72d8a7c218cbc79f`.
The final native Snark checks include the corrected Fire ice coating. The broader
visibility/78-replacement run used the preceding build; the evidence manifest
records its hash separately. The only later runtime change enables the existing
shared frozen material for Fire Snarks, which have no source frozen clip alias.
