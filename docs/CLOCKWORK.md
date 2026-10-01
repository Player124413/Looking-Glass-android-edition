# Clockwork Automata

Ordinary Clockwork Automata now fight through the shared NPC system. Their original
clips drive wake-up, turning, pursuit, three punch contacts, ranged preparation,
flying fists, steam, pain and two directional deaths. The shared animation reader also recognises the original `enter` event spelling
used to enable both steam jets. Source health is 400, pain
threshold 65 and mass 500. Punch contacts deal 5 each; fists deal 10 with knockback;
a steam burst deals 12 once per target. Ice holds the original frozen pose.
Deaths award one Large essence drop and the body sinks after its final pose.

Flying fists originate at the animated left/right hand tags, travel at 500 units
per second, collide against world geometry and targets, expire after five seconds,
and carry their original model, flame trail and orange dynamic light. They retain
the selected target; a vanished Demon is not replaced with Alice mid-flight.
Already launched fists can finish after their owner dies. Both Alice and a hostile
Demon can be hit by the same steam burst, once each.

## Map coverage

Placed combat actors and reviewed direct touch/relay activations work in funhouse,
hatter1, hedge2 and hedge3, respecting difficulty filters. New ambush placements
are appended so pre-existing actor/reward identities remain stable. On Normal difficulty this covers 20 enemies: funhouse 10, hatter1 3, hedge2 2 and
hedge3 5, including three dormant actors. The scene
prop named child_clockwork in hatter1 remains owned by its scripted scene.

Still deferred: funhouse's six glass-wall entrances, hedge2's puzzle-controlled
robot spawns, and hatter2's recurring clock/boss waves. Those require the owning
puzzle/scene to operate the corresponding barriers and timing. A supported
trigger_multiple can activate its actor once; it does not create replacement
Automata on repeated touches.

## Persistence and limits

Pause stops combat, projectiles and emissions. Saves retain attack contacts,
steam victims, timers, activation, health, random decisions, frozen/dead state,
projectiles and reward history. Older decorative records acquire a live controller;
new ambush slots absent from an older save reset dormant. Cosmetic particle caches
are rebuilt from the restored time. Each enemy has independent emitter state;
projectile/trail/impact resources are capped and discarded on level replacement.

Navigation uses local collision-aware pursuit and ledge checks, not the full
original path/hearing AI. Bodies remain non-solid to Alice. Seeking fists use a
bounded turn rate approximation (120 degrees/second); steam uses an approximated
forward cone. The original ranged-repeat chance is capped to one repeat per
sequence. Impacts use a short flash rather than the original explosion/decal.
Steam history resets on animation changes; restored curved trails approximate
recent travel. One-shot attack/pain/death sounds are connected; ambient movement
and sustained steam loops remain unimplemented. This is not a full campaign or
Hatter boss restoration.

## Verification

`--clock-check` checks original clips, the hand model, actual-animation combat at
30/60/144 Hz, and map/difficulty trigger identities. `tools/test_clock.ps1` runs
native rendering and save-write/read in separate processes on the background
desktop. It covers punch contacts, live fists, consumed steam damage, pain, death,
freezing, dormant actors and pending activation, plus original map trigger,
legacy-save migration, reward uniqueness and 22 visual states. Local evidence
and build identity are kept under private/clock.

The separate `tools/launchers/Launch-Clockwork.cmd` playtest opens funhouse with isolated saves.
Original assets remain read-only and are not included in source control.

Verified locally on 30 September 2026: 526 unit tests, all 16 map/difficulty audits,
22 distinct native captures, nine fresh-process continuations, and all 63 model
definitions (316 loaded clips). The 39-visit registry built deterministically
without a stored golden comparison. The merged main project compiled for all
targets. Lint checking found no new issues with the four pre-existing Rust 1.98
lint categories allowed, as recorded for the previous Chess/Fire Imp builds.
Native checks disabled audio, so audible mixing was not verified.

Candidate SHA-256:
`8c0d908b64a7620bd760f530734ae56149467fbb65ec74d84019277b5e6e2750`.
This separate build includes the preceding Ant, Chess and Fire Imp work. The main
source also retains ongoing changes from other development chats.

Ant, Chess and Fire Imp combat regressions, the actor-visibility suite, and 78 full
level replacements also passed on this candidate.
