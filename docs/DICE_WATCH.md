# Demon Dice and Pocket Watch — September 28, 2026

This pass implements the missing demon attacks and enemy retaliation, corrects lifetime and defensive behaviour, and audits stopped time across the ten implemented toys. Rules come from the user's installed TIKI/SKA/state files and bounded, read-only inspection of the original gameplay DLL. Original assets and analysis exports remain private; the Rust implementation does not execute that DLL.

## Demon Dice

Both attack buttons use the same summon. The throw costs **40 Will** at Alice's frame 7 (0.35 seconds), has six seconds of recovery, and throws one to three dice according to owned copies. Copies are not consumed. The saved random stream determines the pips; reloading cannot reroll them. A failed placement refunds the original 40 Will once.

| Result | Creature | Health / pain threshold | Effective scale |
| --- | --- | --- | --- |
| One die, or sum below 9 | Lesser | 50 / 35 | 1.25 |
| Sum at least 9 without meeting the king condition | Normal | 100 / 40 | 1.5 |
| Three dice totalling at least 13 | King | 200 / 75 | 2.8, including the model's 1.4 scale |

**Rage with three dice forces a king.** This is recorded when throwing, so Rage expiry or loading during the roll does not change the result. All three use their original scaled bounding boxes. Placement checks support, clearance and an unobstructed path from the dice. Lesser demons fly; larger demons step over supported ground, avoid local obstacles and refuse unsupported ledges. Charges stop when blocked.

The summon chooses a nearby visible eligible enemy within its 1,200-unit vision distance. Without one at appearance it attacks Alice. A friendly summon reacquires remaining enemies and disappears after idle time and the current idle animation, rather than becoming hostile after its last enemy dies. The previous invented 30-second lifetime is removed: combat can continue beyond that. Death or an existing cinematic dismissal ends the summon. Friendly cast and puzzle targets are excluded.

| Creature / attack | Authored events and damage |
| --- | --- |
| Lesser fist | Frame 8: 10 electric damage, 20 knockback |
| Lesser lightning | Frames 9–15: seven 2-damage beams, alternating hand tags, 500-unit range |
| Normal tail | Frames 6–9: four 5-damage contacts |
| Normal claw | Frames 5–9: five 5-damage contacts |
| Normal fireball | Frame 20: speed 700, 25 direct damage, two-second lifetime |
| King first strike / follow-through | Frame 12: 20 damage and 2 knockback; follow-through frame 3: 20 damage and 100 knockback |
| King lightning | Frames 10, 16, 18, 20: four 2-damage beams |
| King ice breath | Frames 11, 12, 15, 18, 21: five projectiles, speed 800, 15 damage each, two-second lifetime |
| King charge | Start, travelling and impact clips; impact frame 1: 35 damage and 150 knockback |

Timing uses each original animation's frame interval. Melee requires reach, facing and clear sight. Projectiles and beams trace their actual path against world geometry and the nearest body. The lesser beam uses animated hand heading, target elevation and difficulty-dependent scatter; the king's beam tracks its target. Fire and ice carry their original damage categories. Ice kills use the original frozen-death pose/material.

Club/Diamond guards, Boojums, Ladybugs and the Duchess acquire and retaliate against friendly summons. Demon hits provoke retaliation. Projectiles test both Alice and the summon independently of the shooter's current target. Melee and area damage are routed to the body actually hit; Duchess attacks directed at a demon cannot grab or move Alice. Losing a summon returns acquisition to the surviving targets. Retaliation selection survives saves; live body references are rebuilt on the next update.

The normal demon's energy shield displays the original impact model while damage still reduces health. The inspected original damage handler emits its shield effect and continues damage processing; it is not blanket invulnerability. Original pain thresholds and saved impact knockback apply. Demon damage is no longer multiplied by Alice's currently selected weapon, Rage or outgoing player difficulty multiplier.

Original attack, pain and death clips, timed sound events, fireball/ice particles, summoning smoke and disappearance particles are connected. Demon sounds now have world positions. The former orange trail on ice breath and flat purple summoning disk have been removed.

## Pocket Watch policy

Both buttons activate at Alice's frame 22 (1.10 seconds), costing **1 Will**. Time stops for **20 seconds**; recharge takes **360 seconds from activation**, independent of difficulty. Switching toys does not cancel, restart or refill these clocks. Rejected uses cannot charge twice. Pause freezes both clocks. Retry cancels stopped time while retaining recharge; normal campaign transitions retain the player's timers.

| Runtime system | During stopped time |
| --- | --- |
| Alice, movement, aiming, weapon actions, cooldowns, Will expenditure and return/reuse clocks | Continue |
| Blade and Cards projectiles, Mallet ball, Jackbomb fuse/fire/explosion, Ice spray, Jacks, Staff charge/beam/spiral/comets, Blunderbuss | Continue |
| Thrown Dice | Continue rolling and resolving; a newly created demon waits |
| Summoned demons, their attack clocks, bolts, lightning lifetime, shield and recoil | Stop |
| Ice Wand wall lifetime | Stops; placement and damage remain possible |
| Alice's underwater Ice Wand shell | Continues on Alice's clock |
| Implemented enemy AI, attack/death animations and projectiles | Stop; damage can still change health/state |
| Implemented school/village/fortress/minecart/pool autonomous movers, scheduled encounters and essence decay | Stop |
| Environmental particles | Stop unless their emitter declares `no_deadtime` |
| Existing sound samples, looping ambience, music, dialogue and Alice's sounds | Continue; ordinary menu pause still pauses playback |
| New random environmental sound scheduling and frozen actors' future animation sound events | Wait for their world clock |
| Player-driven interactions and story playback | Retain their existing controls; the Watch does not disable dialogue or skipping |

The important native finding is `ignore_deadtime`: **no argument and an explicit `0` both clear the internal “affected by deadtime” flag**. Earlier notes misinterpreted `ignore_deadtime 0` on the Blade and Staff spiral. Both now continue, as do their return/reuse clocks. This correction supersedes the earlier Blade/Staff notes and the audit's historical statement that the knife opted into freezing.

Updates use the unfrozen fraction of the expiry frame, including demon attacks, Ice walls and ordinary environmental particles. This avoids advancing a whole frame of world simulation when only part of it follows expiry.

## Saving and verification

Saved state includes rolls and random state, summon target/hostility/health/phase, individual attack events, recoil, shield, bolts, live lightning segments, real/world clock remainders, player resources, weapon actions, Watch duration and recharge. Existing saves receive defaults for added fields. A restored attack neither re-emits completed events nor spends its Will again. Cosmetic ambient particle clouds are rebuilt; gameplay timers and damaging effects are preserved.

- Unit regressions cover every attack, all three tiers, hostile/friendly target rules, walls, step/ledge/obstacle movement, failed-placement refunds, Rage, shield/pain/death, combat beyond 30 seconds, four enemy families' retaliation, partial expiry, sound scheduling and particle exceptions.
- `--dice-check` loads original clips, verifies the attack event damage, and defeats a real first-school guard at 30/60/144 Hz.
- `--dice-render-check` renders 21 staged original demon poses, including the normal claw and king follow-through, breath and charge. These are visual fixtures, not campaign playthroughs.
- `--dice-watch-render-check` uses actual funded player actions for all **17 distinct attack modes across ten toys**, saves each active state, and verifies exact subsequent action/effect state and Will in another native process. It explicitly checks moving thrown knives, frozen newly summoned demons, frozen Ice walls, Staff release, cannon discharge and Watch activation.
- `--items-check` verifies Watch expiry/resumption with real second-school movers and enemies, including saving and different update rates. Separate tests switch through all ten toys while preserving Watch state, and verify expiry, retry and recharge on every difficulty.

Native checks run on Anode with isolated test files, without using player save slots. This seat has no isolated audio output. Sound files, event routing and pause scheduling are checked; **audible playback/mixing was not verified in this pass**.

Final validation: **327 unit tests passed**, strict all-target Clippy passed, and the release build passed. The eight archive-backed input, Blade/Cards, Mallet/Jackbomb, Ice/Jacks, Staff/Blunderbuss, Dice, item and audio checks passed. Audio decoded 328 referenced files; two pre-existing missing archive references remain (`idle_move2.wav` and `sound/icewand/icw_idle.wav`). The final native process restored all 17 toy scenarios, and all 21 demon captures completed.

## Remaining fidelity boundary

These are independently implemented combat controllers, not a port of the entire original Actor/pathfinding engine. Long routes around complex obstacles, original projectile-leading probabilities, exact melee-volume geometry, death shrinking and every cosmetic light/particle detail are not certified matches. Existing environmental shader/sky animation uses the frozen world clock; exact original renderer clock parity has not been established. Later level work implements the explicit `tower2` flusher exception: trays remain active while water, lids, fans and enemies stop (see [Water Logged](TOWER2.md)). Other map-script and unsupported projectile exceptions remain outside this weapon pass. The table above describes the implemented systems, not every actor or puzzle in the full campaign.
