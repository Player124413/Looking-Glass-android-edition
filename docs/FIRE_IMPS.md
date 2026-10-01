# Fire Imps

The Fire Imp is a supported combat enemy in wforest, jlair1, hedge3 and facade.
The supplied model retains its fork attachment and materials. It turns toward
Alice or a hostile Demon Dice summon, runs from a distance, walks on approach
and strikes with its fork. Contact checks cover, facing, height and range at
frame 12 of the original attack clip; a missed swing cannot damage later.

Health is 35 and fork damage is 10 before the shared difficulty adjustment.
Accumulated damage reaching 25 triggers one of two pain clips; further hits do
not repeatedly rewind that reaction. Knockback still responds to lighter hits.
Fire Sword damage (including the fire demon's corresponding means of damage)
is ignored; ordinary fire/explosion damage remains effective.

Two normal death clips, ice death held at the supplied fifth frame, and gib
outcomes are supported. Eligible non-blade/non-ice lethal hits have a 15 percent
gib chance. That outcome removes the body and releases four original gib props
and the fork from its current attachment pose. Five saved fragments at most
survive for five seconds. Normal corpses finish, hold five seconds, then shrink
over two. Dead actors cease attacking/being weapon targets immediately and award
one Small essence through the shared reward ledger. Deadtime Watch and pause
stop combat, pending ambush clocks, corpses and debris with gameplay time.

## Activation and saves

Existing NPC slots keep their identities. New dormant slots are appended after
all existing Chess slots and identified by the BSP entity number. Reviewed
one-shot and multiple touch triggers resolve Fire Imp spawn receivers through
bounded relay chains. Each receiver activates once; revisiting a trigger cannot
resurrect a defeated Imp. The three wforest relay delays are retained individually.
On Normal this covers 62 Imps: 10 in wforest (6 dormant), 41 in jlair1
(31 dormant), 2 in hedge3 and 9 in facade (6 dormant). Two extra jlair1
trigger-spawned records named t157 have no linked activation and remain deferred.
Difficulty restrictions apply to triggers, relays and receivers. Script callbacks,
shoot triggers and unsupported activation paths are not guessed.

Optional format-12 NPC state preserves activation, delay, health, action, strike
consumption, random sequence, reaction, knockback, pursuit and corpse/debris state.
Older records initialize their former decorative Imps at the saved placement;
missing new ambush actors reset dormant, even when loading into a played map.
The existing shared level cache/reward ledger handles return visits. Generic Card
Guard resets do not revive these enemies.

## Limits

This is a bounded local combat controller, not the original navigation/AI engine.
It can steer around small obstacles but cannot follow every authored patrol or
find a route through a complex map. Authored patrol paths and monster-only leap
pads, including the jlair1 lava-gap routes, remain unsupported. NPC bodies remain non-solid to Alice and one
another, as with the Ant/Chess implementation. Hearing/noise investigation and
some ambient sound timing remain approximations. The shared movement rule avoids
liquid edges; generic NPC environmental damage is not implemented, so the source
lava immunity does not require a damage override. This does not implement lava
swimming or all original drowning behavior. Gib trajectories, chunk selection,
contact volume and corpse hold/shrink timing are approximations. No original
scripts or game binaries execute and no original assets are added to source.

## Verification

`--imp-check` loads source clips and props, checks combat at 30/60/144 Hz and
audits placement/activation for all four difficulty settings. `tools/test_imp.ps1`
adds native pose captures and separate executable runs to save/reload attack,
consumed contact, pain, ordinary death, ice death, gibs and delayed activation.
These are focused fixtures, not a claim to have completed each campaign map.

Local candidate: `tools/launchers/Launch-Fire-Imps.cmd`, starting in wforest with isolated saves.
Verification results and binary identity are recorded privately with the build.

Verified locally on 30 September 2026: 519 unit tests; 16 map/difficulty audits;
13 distinct native captures; seven fresh-process continuation cases; Ant/Chess
combat regressions; the shared actor-visibility suite; and 78 complete level
replacements. The 39-visit registry built deterministically twice per visit;
no stored golden baseline was compared. The merged main project also compiled
for all targets. Native captures used disabled audio, so audible mixing was not
verified. Compiler lint checking found no new findings after allowing the four
pre-existing Rust 1.98 lint categories recorded for the Chess build.

Candidate binary SHA-256:
`43d004772c630d013e6ed894b85be8e815c834a4c4e61b2e417062836eb5f3e2`.
This isolated playtest includes the preceding Ant/Chess work; the shared source
also retains ongoing changes from the other development chats.
