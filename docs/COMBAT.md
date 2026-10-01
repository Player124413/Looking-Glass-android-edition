# Combat — updated for version 0.27

Version 0.27 applies player knockback from Boojum waves, Diamond projectiles, Ladybug acorn explosions and the Duchess. Impulses detach generic rope grips, interrupt ledge climbing and pass through ordinary swept collision; brief input suppression preserves the push. Pause and saves preserve that state. Weapon impulses against enemies remain a separate fidelity gap. See [TRAVERSAL.md](TRAVERSAL.md).

Version 0.26 adds the Duchess boss, arena and progression reward. See [DUCHESS.md](DUCHESS.md) for attacks, vulnerabilities, saves and fidelity limits.

Version 0.22 adds all twelve Ladybugs in `potears1`: authored patrols and paired ambushes, collision-checked flight detours, timed acorn drops and rearming, pain interruption, armed death drops and falling/landing death. Original models, animations and sounds are connected. Their bombs, navigation and delayed activations persist in saves. See [LADYBUGS.md](LADYBUGS.md) for mechanics, tests and fidelity limits.

Version 0.21 adds Demon Dice on **7**, the next toy in campaign order. Either mouse button spends **40 Will** on the original summon attack. Collected dice determine the roll and possible demon tier; original creature clips drive melee and ranged enemy damage. See [DICE.md](DICE.md) for controls, contact damage, hostile empty-room summons, saves and fidelity limits.

Version 0.18 adds the reviewed village/first-school activation groups, including the first school's return-entry groups. Diamond guards use their original model, staff, attack/pain/death clips, projectile and sounds. A frame-five release fires a swept projectile at 700 units/second, with radius 8, 7 damage and a maximum five-second life, measured from the local definitions. Walls block the projectile and movement can dodge it. The simplified sight/pursuit behaviour is independent Rust, not the original AI. School Boojums use the existing flight/scream implementation.

The optional Looking Glass suppresses sight acquisition for 45 seconds. Previously fired attacks and hazards can still hurt Alice. These explicit encounter groups retain activation, damage and death through R/Home/death recovery alongside puzzle progress; a fresh Tab/console map load rebuilds them. Duplicate dialogue completion cannot respawn killed enemies. Other maps' older generic club guards and the combat preview still use the reset behaviour described below.

Version 0.16 adds the second school's flying Boojums and scripted laboratory guards. Boojums activate at the quest's encounter triggers, approach/orbit, emit timed scream projectiles, take weapon damage, interrupt attacks with pain and fall/shrink when killed. Scream contacts are swept against world geometry and Alice. The main three-Boojum battle must finish before the laboratory quest advances. The rescue guards threaten the Gnome until Alice hits them. See [SCHOOL2.md](SCHOOL2.md) for the full route and verification boundaries. The club-guard and weapon rules below still apply.

**tools/launchers/Launch-Combat-Preview.cmd** opens the school entrance with one club card guard and all toys available. Use **1** for the Blade, **2** for Cards and **3** for the Mallet. Tap for one attack or hold to repeat when ready; busy taps are not buffered and switches wait for the current action. **V** switches views, **P** pauses, **Home** restores the entrance and resets the guard. **Enter** retries after death. The usual **R** recovery also resets the current map's club guards. Friendly interaction stays on **E**.

Ordinary launches enable combat for supported placed club guards and the village/schools' explicit encounter groups. Normal weapon ownership remains campaign-dependent. **tools/launchers/Launch-NPC-Preview.cmd** disables the new village/first-school encounter groups as well as generic club combat throughout its peaceful showcase session. Other character and enemy types retain the previous idle/reaction behaviour.

## Encounter behaviour

Club guards use their original idle, alert, walk, attack, pain and death clips and attached staff. They notice a visible Alice within 420 units, turn, alert and walk toward her. Local movement checks the body against world geometry and current doors, permits small steps, checks support under all four corners, and refuses ledges and liquid. This is direct local pursuit with a distance limit, not a navigation mesh: a guard stops when sight is blocked and cannot plan a route around a corner.

The guard has 55 health, matching its model declaration. Its selected attack uses the declaration's frame-14 strike and 12 damage. The strike occurs once, only with clear sight, nearby horizontal/vertical range and Alice in the forward attack cone. The guard does not turn during the swing, allowing Alice to retreat or circle away. A short recovery follows. Pain interrupts the current attack, repeated pellets do not restart the same pain pose, and lethal damage stops combat and leaves the finished death pose. Selected alert/attack/pain/death sounds use the existing effects channel; they are not a general animation-event interpreter or spatial voice system.

NPC logic advances in fixed 120 Hz steps. Pause, menus, free flight and Alice's death freeze its clock. The headless school route produces the same pursuit position and outgoing damage at 30/60/144 render Hz. Home, R and death retry reset generic guards and clear Alice's pending attacks/projectiles, including Dice summons; explicit encounter groups retain their progress as described above. Since v0.19, cached visits and persistent saves retain enemy state. Returning through normal exits restores that visit; starting a fresh map rebuilds it.

## Weapon rules

The [ten-toy audit](TOY_AUDIT.md) compares these implemented rules with the original definitions and targeted native handlers, including cost/damage differences and unsupported attacks.

The first three toys' damage below remains prototype tuning. Resource costs now follow the original declarations. The shared scheduler checks affordability before starting and commits a single debit at the authored weapon event; unsupported toys never debit. Insufficient Will selects an owned Blade. Hard cancellation before release is free; switching queues until the attack finishes. See [WEAPON_ACTIONS.md](WEAPON_ACTIONS.md) for exact timing, recovery and saved-action migration.

| Attack | Damage per contact | Will cost |
| --- | ---: | ---: |
| Blade swing | 20 | 0 |
| Thrown Blade | 35 | 0 |
| Single Card | 10 | 3 |
| Five-card spread | 10 per card that hits | 20 total |
| Mallet swing | 35 | 0 |
| Croquet ball | 25 | 8 |
| Demon Dice, either mouse button | Summon-dependent melee/ranged attacks; see DICE.md | 40 per summon |

Melee uses the existing release clock with a short swept reach. Projectiles use their visible swept trajectory; the nearest living guard contact before a wall wins. A projectile stops on that guard and cannot damage it repeatedly. The croquet ball retains its bounded world bounces until it hits a guard or expires. First-person shots converge on a living guard under the eye ray. A brief impact effect and HUD acknowledgement show hits/defeats; Alice's existing Sanity meter and death/retry panel reflect guard damage.

## Route driver

The headless route checks drive Alice through the same weapon scheduler as the game, but choose what to press with a small combat policy (`src/route/fight.rs`), which had to change when the Blade/Cards rules made a thrown Blade a 3.9 s commitment (release at 0.40 s, then 3.5 s with the Blade out of the hand):

- **Targets.** Only enemies that are awake and visible: Guards that have left Idle, active Boojums and Ladybugs, and the Duchess. Idle guards are not shot at.
- **Toy.** The primary Blade swing when an enemy is inside its melee volume (no recovery, no Will); otherwise Cards while Will is at least 3, which seek their target and are paid through the same Will ledger as the game; otherwise a thrown Blade that leads the target for the 1,200 u/s flight and 100 u/s² drop.
- **`Route::clear(radius)`.** Stands and fights: strafes across the line of fire (waves and bolts are aimed at where Alice is, not where she is going) inside a 120-unit leash and only where a 72-tick look-ahead, including coasting to a stop, keeps her on the floor; walks up to melee-only guards; then walks over the essence they dropped (which restores Sanity and Will) and returns to where the fight began, so straight route legs still start where they were written.
- **Progress watchdog and dry foes.** A hit can knock Alice off a walkway onto a ledge where every heading fails the look-ahead, so the fight would stand still until its 90-second limit (the first school's library passage did, once the chain handed it Alice with 25 Will). If she has neither moved 8 units nor wished to move for 240 ticks while foes remain, and she stands more than 24 units below where the fight began or beyond the leash, `clear` plans a way back to that spot (`Route::navigate`) and re-enters the fight from there, up to six times per call. A focus that has taken no damage for 2,400 ticks of being the focus is left to the route's own timing, the way a melee-only guard that cannot be reached already was; the later goals of a route that pass it call `clear` again with a fresh list. The last landed hit of each target is kept by the fight tracker (`Tracker::last_hit`, fed from the route tick's hit loop). No gameplay value is involved.
- **Opt-in.** `Route::tactics` is off unless a check sets it or calls `clear`. The village, both Pandemonium and first-school-return checks keep the original throw-at-the-nearest driver and their recorded metrics byte for byte.

`LOOKING_GLASS_ROUTE_TRACE=1` prints every hit Alice takes with its cause and position, falls, each throw, swing and card, and what they hit. This is how the drivers were retuned; use it before touching a gameplay constant.

## Limits

Combat supports Club and selected Diamond guards, supported Boojums, Pool of Tears Ladybugs, Dice summons and the Duchess in potears3. Other bosses and monsters remain unfinished. Actor hit volumes are simplified boxes; bodies remain non-solid to Alice and other NPCs. Ladybugs have bounded 3D flight routing; general ground pathfinding, coordinated crowd avoidance, knockback, loot drops and original difficulty tuning remain incomplete. Enemies retain their existing Alice/scripted-victim targeting and do not yet retaliate against summons. Quest consequences and selected scripted spawns are connected in the schools; unsupported activation flags elsewhere remain deferred. The Watch also has a supported time-stop ability; the other five toys have held models only. Both school first visits and the first-school return quest have tested routes.

## Verification

`--combat-check` loads the real school collision, doors and club-guard clips, runs the entrance encounter at 30/60/144 Hz, checks that the guard approaches and lands attacks, verifies Blade reach, and defeats it with three hits. Synthetic tests exercise walls, nearest contacts, ledges, pause, deterministic timing, dodging, dead-state behaviour and attack/resource gating. Native Anode tests exercised Blade swings/throws, card spread damage/Will, Mallet swing/ball damage/Will, guard defeat, Alice's death and Enter retry. See [VALIDATION.md](VALIDATION.md). These establish this encounter, not campaign completion or original-AI fidelity.
