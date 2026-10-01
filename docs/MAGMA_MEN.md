# Magma Men

Magma Men now use a shared combat controller instead of dormant/decorative
placements. The six reviewed actors appear only after their original touch
triggers: two in `wforest`, two in `jlair1`, and the two `launchmagma` spawners in
`grounds2`. The latter use their intersecting push volumes and authored target
points for a one-use, swept ballistic entrance. The dangling `magmaman03` target
in `jlair1` does not manufacture an extra enemy.

The supplied `c_magmamen` model, `ai/c_magmamen.st` and fireball declarations
provide the health (200), vision (800), pain threshold (25), unscaled server
hull, visual scale, three sets of skins, walk clips, damage frames and death
clips. Fire Sword damage is ignored. Lava is permitted by the movement checks;
water/slime and unsupported ledges remain rejected.

The molten/cooling punch extends at frames 10–16, dealing 10 fire damage once
before entering the authored recoil clip. Rock punches deal 15 at frame 11;
the separately staged rock bash has the declared frame-13 radius attack.
Fireballs leave the animated mouth at frame 10, travel at 700, deal 25 Fire
Sword damage and expire after two seconds. Sweeps prevent hits through walls
or spawning a shot through a wall. Four live shots per actor is the hard cap.
Pain interrupts an attack without leaving its future contact armed. Lethal
hits choose molten/rock or frozen animation, retire the body and use the shared
one-time large mana reward. Already released fireballs finish independently.

Skin selection and cooling alpha are supplied per actor; shared model textures
are restored after each draw. The shader parser recognizes `alphaGen fromEntity`
for the supplied skin transitions. Smoke follows the three skeletal smoke tags;
fireballs use the supplied fire/smoke particle declaration. Emitter histories are
owned by actors/shots and cleared on restore and level replacement.

Simulation uses the shared 120 Hz resident clock. Zero elapsed time does not
advance triggers, cooling, attacks, projectiles, smoke or death. Saves retain
form, cooling progress, contact consumption, random state, launch velocity,
recoil, projectiles and activation delay. Added actor slots append after earlier
resident families, preserving existing enemy/reward identities in old saves.

## Deliberate limits

The native MagmaMan class that controlled `STAGE` is absent from the supplied
scripts. Cooling over six seconds per form outside lava, with four-times-faster
reheating in lava, is an independent approximation, not verified original timing.
Form changes wait for an idle/walk boundary so they cannot replace an attack or
pain clip midway through its cues. Cooling currently occurs during active combat
and idle time. The staged bash is implemented for coverage, but the supplied AI
selects the ordinary rock punch, so regular combat does not invent bash selection.

Steering is local; linked patrol paths, original route finding and exact aim
inaccuracy are not reproduced. The push entrance uses the existing gravity/target
solver once, rather than a general monster trigger-push runtime. Source lava
handprints, earthquakes, dismemberment/gib fragments, the continuous steam sound
and walking sounds remain presentation gaps. Attack/pain/death sounds are wired;
audible output still needs a listening pass. Frozen deaths use the existing ice
overlay and bounded retirement. No original executable code is loaded or copied.

## Verification

The eight dedicated controller tests cover cue timing, cover, cooling/reheating,
immunity, pain/deaths, bounded projectiles, liquid/cliff movement, corrupted saves
and exact mid-action/launch continuation. The complete unit suite passes (578).
All-target Clippy passes with the repository's existing allowances.

`--resident-check` verifies the supplied clips/cues/tags, all 24 Magma placements
across four difficulties, clear collision hulls, both launch pits and behaviour
at 30/60/144 Hz, alongside the earlier resident families. This is deterministic
component coverage, not a natural campaign walkthrough.

`tools/test_magma.ps1` provides background-desktop rendering and fresh-process
save checks. It exercises the five original trigger groups, old-save migration,
attack/pain/death/ice/dormancy/delay/projectile continuation and staged captures
of each form and attack. These native checks, a playthrough and current-batch
level-replacement checks have not yet run: another task held the background
desktop through repeated acquisition attempts. Candidate identity and this
limitation are recorded with the private executable/evidence bundle.

Phantasmagoria, Nightmare Spiders, Jabberspawn variants, Walkrocks, remaining
allies/escorts and boss encounters are still tracked in `NPC_ROSTER.md`.
