# Demon Dice — version 0.21

Select Demon Dice with **7** once owned. **Left or right click** performs the same original summon: the installed weapon definition has one firing mode, and both Alice action states select `dice_att`. There is no distinct alternate Dice attack to restore. The summoned creature chooses melee or ranged attacks according to distance.

An accepted throw costs **40 Will**, committed once at the frame-7 release (0.35 seconds). Holding either attack button repeats only after the six-second recovery and the existing roll/summon have ended. Busy taps do not queue another summon. Insufficient Will selects an owned Blade. A weapon switch waits for the current throw to finish; death or scripted-control cancellation before release costs nothing. Released dice and demons continue while another toy is equipped. See [shared action and save rules](WEAPON_ACTIONS.md).

## Summoning and combat

Alice throws one die for each copy owned, up to three; casting never consumes those inventory copies. The dice bounce against the world and settle before opening a rift. Their saved roll determines the creature:

| Owned dice and roll | Summon | Health |
| --- | --- | ---: |
| One die, or a total below 9 | Lesser demon | 50 |
| Two or three dice totalling 9–12 | Normal demon | 100 |
| Three dice totalling 13 or more | Demon king | 200 |

The normal tier is simply any total of 9 or more that does not meet the king condition. More dice improve the chance of a stronger summon; three dice do not guarantee a king unless Rage was active when throwing. Floor support and body clearance are checked before appearance. A cramped or obstructed landing may fail to produce a demon, refunding the cast once.

The creature targets nearby visible supported enemies. The lesser demon can fly; the larger demons use supported ground movement. If no eligible enemy is visible when it appears, it turns on Alice. A friendly summon fades after the enemies disappear instead of then attacking Alice. Alice can damage the summon with her other supported weapons. Death, inactivity or cinematic dismissal ends the summon. Combat has no arbitrary 30-second expiry. Only one summon is allowed at a time.

| Creature | Melee | Ranged |
| --- | --- | --- |
| Lesser | 10 damage at the strike | Seven lightning pulses, 2 damage each |
| Normal | Four tail contacts or five claw contacts, 5 damage each | Swept fireball, 25 damage on impact |
| King | 20 damage plus a 20-damage follow-through; 35-damage charge impact | Four 2-damage lightning pulses or five 15-damage ice projectiles |

Walls block attacks. Supported guards, Boojums, Ladybugs and the Duchess retaliate against friendly demons. Demon damage uses its own authored values rather than Alice's weapon/Rage multipliers. The normal shield emits an impact effect while health damage continues. See the current update for the attack event table, saved state and exact scope.

## Presentation and saves

The held and thrown dice and all three demons use the installed original models and skins. Alice uses her original throwing clip; demons use original idle, movement, melee, ranged and disappearance clips. Throw, bounce, rift and attack sounds use the existing effects volume. The summoning smoke, disappearance, fireball and ice-breath effects now use original definitions. World effects respect depth; first-person equipment uses the existing local weapon pass. **V** switches views.

**F5/F9 and Continue** preserve an unreleased action, thrown positions/velocities, roll and random state, Will balance, recovery clock, active demon, target, health, animation/attack progress and damaging fireballs. Active lightning, shield and recoil clocks are saved; ambient cosmetic clouds rebuild. Pause and menus freeze the simulation. Recovery and level changes clear active weapon effects, including summons, while retaining owned dice. Save format 3 also reads formats 1 and 2; see [SAVES.md](SAVES.md).

The first die is available in `skool2`; the existing campaign profiles give one die to the school return and later starts, then add the second and third at their existing milestones. Pickup and inventory rules are unchanged. For a staged demonstration, open **tools/launchers/Launch-Combat-Preview.cmd**, select **7** and cast at the guard. That preview supplies one die and does not overwrite ordinary saves.

## Verification and remaining fidelity

`--dice-check` validates original models/clips and runs a staged real-school guard encounter at 30/60/144 Hz. Unit checks cover both mouse buttons, resource gating, all three demons' melee/ranged damage, blocked placement, walls, pause, hostile summons, death and deterministic restart. Native input checks cover first-person casting, guard damage/death and one debit despite repeated clicks. Native save writer/reader checks add unreleased, rolling and active-summon cases. `--dice-render-check` captures explicitly staged original models and effects; it is not a progression test. See [VALIDATION.md](VALIDATION.md).

See [current fidelity boundaries](DICE_WATCH.md#remaining-fidelity-boundary) for original Actor/navigation and cosmetic differences. Assets, private analysis and original-art captures remain local and excluded from source packaging.
