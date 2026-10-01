# Ten-toy fidelity audit

**Latest implementation update:** [Eye Staff and Blunderbuss](STAFF_BLUNDERBUSS.md) now have playable modes, native timing and damage, Will consumption and saved continuation. All ten toys have abilities. In particular the cannonball touch handler bypasses its declared direct hit and excludes the owner from its explosion. Older findings below remain the dated audit baseline.


Audited 28 September 2026 against the user's installed American McGee's Alice archives and this Rust working tree. This is a comparison and implementation backlog; it does not change gameplay balance or claim a completed remake.

**Follow-up:** [shared action handling](WEAPON_ACTIONS.md) now implements held repetition, queued switching, single event-time debits, thrown recovery and Watch release timing. Cards/Mallet costs are corrected. The comparisons below preserve the audit's original snapshot; consult the follow-up for current input/resource behaviour. See the dated implementation follow-ups below for backend progress.

**Further implementation:** [Blade/Cards fidelity](BLADE_CARDS.md) now replaces the audited prototype damage, hit volumes and projectile behaviour, and resolves the Cards reload question. The Current/Gaps bullets below remain the audit-time record.

**Mallet/Jackbomb follow-up:** [MALLET_JACKBOMB.md](MALLET_JACKBOMB.md) completes the Mallet gameplay rules and implements both Jackbomb modes, original timing, damage and saved state. Six toys now have playable abilities; the bold coverage statement below records the earlier audit date.

**Ice Wand/Jacks follow-up:** [ICE_JACKS.md](ICE_JACKS.md) implements both modes of each, native Will charges, freezing, wall collision, seeking/trace projectiles and saved continuation. Eight toys now have playable abilities. The earlier snapshot below is retained as audit history.

**Five toys have working abilities: Vorpal Blade, Cards, Croquet Mallet, Demon Dice and the Watch. Jackbomb, Ice Wand, Jacks, Eye Staff and Blunderbuss have inventory/equipment rendering but no playable attacks.** The first three working toys still use independently chosen prototype damage, costs and projectile behaviour. Loading an original model or attack clip does not establish mechanical fidelity.

## Evidence and interpretation

The primary sources are `models/w_*.tik`, their referenced projectile/effect/creature definitions, `models/alice.tik`, `global/alice_torso.st`, and the referenced SKA/TAN animation headers. Paths in backticks below are **paths inside the user's PK3 archives**, not redistributed files. Active definitions take precedence over commented examples. Alphabetically later archives override earlier ones, using case-insensitive paths.

Original compiled behaviour was inspected read-only in Ghidra where text definitions were insufficient. Native findings are identified separately below. Decompiled parameter types are imperfect: call sites, machine instructions and constant widths were checked for the Blade/Mallet damage arguments and the Jacks alternate loop. Unresolved engine behaviour is explicitly labelled, rather than filled in from memory or cheat guides.

Reproduce the asset inventory with `python tools/audit_toys.py` from the repository root, optionally adding `--data <base-folder>`. The [read-only audit tool](../tools/audit_toys.py) writes extracted definitions, animation timings, archive origins, SHA-256 hashes and compared Rust source hashes to `private/toy-audit/`. Extracted assets and native research remain private and outside source packages. Dependency traversal includes caches; a reachable model is not automatically an active projectile.

All Alice attack clips inspected here have a 0.05-second frame interval. A stated event time is `frame index × 0.05`, measured from that clip's start. **Clip duration, release time, projectile lifetime and weapon cycle time are different things.** These are nominal source times, not an original-game runtime measurement. Unless stated otherwise, damage is raw base damage before difficulty, power-ups, target immunities and splash falloff. Direct and radius-damage declarations must not simply be added together as guaranteed damage to one enemy.

## Coverage and resource comparison

Primary/alternate mean the default left/right attack buttons; rebinding should preserve their semantics.

| Toy | Original modes | Original resource requirement, primary / alternate | Current Rust cost and coverage |
| --- | --- | --- | --- |
| Vorpal Blade | Slash / thrown knife | 0 / 0 | 0 / 0; both approximate |
| Cards | Single seeker / splitting seeker volley | 3 / 20 Will | **1 / 5**; single straight card / five-card fan |
| Croquet Mallet | Melee strike / croquet ball | 0 / 8 Will | 0 / **6**; melee / bouncing ball without explosion damage |
| Jackbomb | Timed bomb / rotating flamethrower bomb | 15 / 20 Will | Neither attack implemented; no debit |
| Ice Wand | Sustained cold attack / ice wall | 0.75 per primary activation / 10 Will | Neither attack implemented; no debit |
| Jacks | Primary swarm / alternate burst | 10 / 20; original ammo type is named `jacks` | Neither attack implemented; no debit |
| Demon Dice | Summon; both buttons use the same mode | 40 Will | 40; bounded independent summon AI |
| Jabberwock's Eye Staff | Held beam / held sky attack | 0.4 / 1 declared units; **not a complete per-use price** | Neither attack implemented; no debit |
| Blunderbuss | One cannon attack; both buttons use it | **99 Will**, not a declared 100 | Neither attack implemented; no debit |
| Deadtime / Pocket Watch | Time stop; both buttons use it | 1 Will | 1; duration/recharge match, activation timing differs |

The inventory catalog reads these original requirements, but supported combat normally charges the separate hard-coded table in [combat.rs](../src/combat.rs); the Watch charges through [inventory.rs](../src/inventory.rs). Reading a correct catalog value therefore does not mean the attack uses it. Jacks' named ammo type and Eye Staff's custom class need their native debit rules followed through before treating every declared value as an ordinary one-shot Will cost.

## Shared input, animation and damage rules

**Original:** the torso state machine tests `ATTACKRIGHT` / `ATTACK_ALTERNATE`, returns to its standing state after ordinary attack animations, and has explicit held/released loops for the Staff. Native attack predicates read attack-button bits without consuming them. Together these support held-button repetition for ordinary attacks once the weapon/state gates allow another attack; the precise cadence also depends on weapon cycles and engine events. The Staff explicitly stops on release or insufficient mana. The standing state routes insufficient mana to switching to the knife. Blunderbuss and Staff alternate use animation-controlled full-body states; ordinary attacks generally retain leg movement. Both-button priority and every interrupt transition still need original runtime verification.

**Current:** [viewer.rs](../src/viewer.rs), [input.rs](../src/input.rs) and [weapons.rs](../src/weapons.rs) accept a **new button press**, equally for mouse, rebound keyboard and controller. Holding does not repeat. Simultaneous primary/alternate presses select alternate. Busy/equipping clicks are discarded; there is no input buffer. A supported attack spends Will at acceptance, before its release frame. Switching toys cancels an unfinished action without refund. Insufficient Will blocks the attack with a notice, without selecting the knife. God mode bypasses the debit. Attack aim is captured at the initial press. Movement uses a blended upper-body action rather than the full original weapon state machine.

**Current shared damage:** outgoing Easy/Normal/Hard/Nightmare multipliers are 1.25 / 1 / 0.7 / 0.55. Rage multiplies Blade by 4, Mallet by 3 and other attacks by 2, using the selected toy when damage is resolved. The original native damage routine also contains weapon-dependent Rage handling; source-weapon attribution for delayed attacks/summons needs a targeted comparison before changing it. Hit volumes are simplified boxes. General original damage-type immunity, freezing, enemy knockback, radius damage and reactions are not fully reproduced. A visually successful impact on a supported enemy does not establish behaviour against every campaign monster.

Supported actions/projectiles, Dice state and Watch timers have Rust save support; pause freezes their gameplay clocks. Original save-format compatibility is not claimed. Exact restart restoration of future charge loops, returning weapons, walls and swarms remains work for those implementations. First-person models use independently authored camera poses, without an original hand/viewmodel animation system.

## 1. Vorpal Blade

**Original sources:** `w_knife.tik`, `prj_knife.tik`; Alice aliases `knife_att_1_prim1/2/3`, `knife_att_1_alt`; native `knifeattack` handler `0x100be1a0`.

- **Primary:** three authored swing variants. Each clip lasts 1.05 s; swing sound at frame 5 (0.25 s), melee contact at frame 7 (0.35 s). Native melee passes **25 base damage** and a nominal reach parameter of **40** to the shared melee helper. The weapon's separate `minrange 50` is not this damage trace's reach. No Will cost.
- **Alternate:** throw sound at frame 6 (0.30 s), release at frame 8 (0.40 s), Alice clip 1.05 s. Weapon declares **3.5 s** thrown-weapon/cycle timing. Projectile: **45 damage**, speed **1,200 units/s**, life **1 s**, gravity multiplier **0.125**, bounding half-extents 8; authored spin, knife damage type, swipe trail, flight sound and impact/decal effects. Weapon and projectile explicitly use `ignore_deadtime 0`.
- **Current:** contact frames and Alice clip lengths match. Swings cycle deterministically, use **20 damage / 65-unit reach**; throw uses **35 damage**, speed **950**, no gravity, **2.5 s** life and radius 1. The knife reappears when the 1.05 s animation finishes, with no 3.5 s recovery gate. Selected original sounds/models and approximate trails/sparks are present.
- **Gaps:** damage, melee geometry, throw physics/hit volume/lifetime, return/recovery timing, held-input behaviour, original variant-selection semantics and time-stop exception. Original melee geometry includes more than a reach scalar; matching 40 alone is not a complete collision fix.

## 2. Cards

**Original sources:** `w_cards.tik`, `w_playingcard.tik`, `prj_cardmirv.tik`, `prj_carddrunk.tik`; Alice `cards_att_prim`, `cards_att_prim_reload`, `cards_att_1_alt`.

- **Primary:** **3 Will**, release at frame 0 of a **0.25 s** clip. Projectile: **7 damage**, speed **700**, life **1.5 s**, knockback **60**, `seeker 10`, half-extents 8. A separate **1.10 s** reload clip exists; this audit did not establish its invocation/cadence and does not append it to every shot by assumption.
- **Alternate:** **20 Will**, authored cycle **0.5 s**, Alice clip **0.85 s**. Loose card attaches at frame 3 (0.15 s); release at frame 7 (0.35 s). Carrier: speed **1,000**, **15 direct damage**, life **2 s**, knockback **60**, `seeker 25`, `drunk 1`. It declares `fragment prj_carddrunk.tik 0.1 8`: eight fragment projectiles with the 0.1 timing parameter. Fragments declare **7 damage**, speed **1,000**, life **2 s**, knockback **60**, `seeker 25`, `drunk 1`. Exact split trigger, steering and contact/splash interactions still require the generic projectile handlers to be resolved.
- **Effects:** card spin/trails, distinct flesh/world impact aliases, impact mark and card-hit effect. Seeker/drunk parameters are engine inputs, not verified degrees-per-second or accuracy percentages.
- **Current:** **1 / 5 Will**, **10 damage per card**, straight speed **950**, life **2.5 s**, radius 1. Primary releases at frame 0; alternate at frame 7. Alternate creates **five simultaneous fan shots**, separated by 0.075 radians, rather than a carrier and eight steering fragments. No knockback or homing. The single-shot/fan resource and animation gates work against supported enemies.
- **Gaps:** both costs, damage, primary/alternate projectile classes, seeking/wandering/splitting, timing and reload integration, physical hit size, knockback and exact impact effects. Five hits can total 50 current base damage; this is not evidence of an original alternate-shot total.

## 3. Croquet Mallet

**Original sources:** `w_mallet.tik`, `croquetball.tik`, `fx_croquetballexp.tik`; Alice `mallet_att_1_prim1/2`, `mallet_att_3_alt`; native `malletattack` handler `0x100bdda0`.

- **Primary:** no Will; two **0.80 s** clips. Electrical state starts on entry; trail starts frame 3 (0.15 s), growth frame 4 (0.20 s), contact frame 7 (0.35 s), shrink/trail-off frame 8 (0.40 s), later electrical reset frame 13 (0.65 s). Native melee passes **24 base damage** and nominal reach **50** to the common melee helper.
- **Alternate:** **8 Will**, Alice clip **1.35 s**. Ball attaches at frame 3 (0.15 s), becomes free at frame 7 (0.35 s), fires at frame 13 (0.65 s). Ball: speed **2,500**, **30 direct damage**, life **5 s**, gravity **0.5**, radius declaration **100**, half-extents 8, bounce-on-touch and electric damage type. Explosion definition adds **30 radius damage**, **30 knockback**, life **1 s**. The ball's radius declaration is distinct from its small collision bounds; damage stacking/falloff needs native verification.
- **Current:** matching Alice contact/toss frames, growth and attachment presentation. Primary **35 damage / 85-unit reach**. Alternate **6 Will / 25 damage**, speed **1,100**, gravity acceleration **200 units/s²**, life **2.5 s**, radius 3. At most three wall bounces with 0.65 damping, then expiration; enemy contact ends the ball. Trails and sparks are approximate.
- **Gaps:** damage/cost/reach, faster ball and gravity/lifetime, original bounce/impact rules, explosion/radius damage, electrical damage semantics, knockback and effect timing beyond the selected visual events.

## 4. Jackbomb

**Original sources:** `w_jackbomb.tik`, `prj_jackbomb.tik`, `prj_jackalt.tik`, `prj_jackbomb_breath.tik`, `fx_jackbombexp.tik`; Alice `jbomb_att_2_prim/alt`.

- **Input/timing:** two distinct attacks; both use the **0.90 s** throw clip and release frame 7 (**0.35 s**). Primary costs **15 Will**, with **3 s** cycle/thrown timing. Alternate costs **20 Will**, with **8 s** cycle/thrown timing.
- **Primary:** bouncing timed bomb, speed **600**, gravity **1**, life **3 s**. Its crank/open/spring animation events control the jack-in-the-box presentation. Explosion declares radius **300**, radius damage **300**, knockback **300**, life **2 s**, fire damage type.
- **Alternate:** bouncing rotating flame emitter, speed **600**, gravity **1**, life **10 s**; this is different from the weapon's 8 s reuse gate. Cranking TAN is **1.25 s**, opening **0.35 s**, rotation **4.05 s**, each at 20 fps. Rotation contains **22 flame-release events** at authored frames. Emitted flame speeds alternate between **250 and 300**, overriding the breath model's default 60; breath damage **10**, knockback **20**, life **3 s**, gravity **1**, bounce-on-touch, fire damage type. It references the same final explosion definition. Exact expiry/animation-transition ordering remains engine work; these clip lengths are not proof of an endlessly repeating flame loop.
- **Effects:** fuse smoke/fire/light, crank music, opening/pop, rotating mouth flame, explosion fragments/fire/decal and positional sound events.
- **Current/gaps:** equipped model only. Both debits, throwing, cycle gates, bounce/fuse lifecycle, rotating aim axis, flame collisions, explosion falloff, damage, all action/event effects and saves are missing. No playable Jackbomb ability is implied by its assets loading successfully.

## 5. Ice Wand

**Original sources:** `w_icewand.tik`, `fx_icewall.tik`, `ai/fx_icewall.st`; Alice `wand_att_2_prim/alt`; native `icewandattack` handler `0x100beea0`.

- **Primary:** a **one-frame / 0.05 s** Alice clip with `every fire` and `every icewandattack`; this is a sustained/repeated attack, not a conventional long swing. Declared requirement **0.75 Will**, weapon maximum-range hint **400**. At nominal 20 activations/s the declaration suggests **15 Will/s**, but this derived rate is not a verified native drain measurement. Native contact damage is **5 × (1 − trace fraction)** before shared modifiers, so treating it as a flat 5-damage projectile would be incorrect.
- **Alternate:** **10 Will**, **1.60 s** clip. Ice effects turn on at frame 1 (0.05 s), wall placement attack at frame 9 (**0.45 s**), turn off at frame 22 (1.10 s). The wall is an Actor, with bounds ±24 horizontally and height **118**, disabled ordinary AI, no knockback, and ice/frozen immunities. Its state graph rises, stands for **10 s**, then suicides and shrinks; the ten seconds start in the standing state, not necessarily at Alice's button press.
- **Special rules/effects:** cold spray, cubes/smoke, looping spray sound, placement geometry and a destructible/expiring wall. Native code has a submerged branch that empties Will and dispatches extra effects/events. Its complete gameplay consequence, freeze accumulation, shattering and target-specific thresholds are **unresolved** here; no underwater kill rule is assumed.
- **Current/gaps:** equipped model only. Continuous input/debit, contact damage, freezing/thaw/shatter, underwater special case, wall placement/collision/health/lifecycle, original effects/sound loop cleanup and saving are missing.

## 6. Jacks

**Original sources:** `w_jacks.tik`, `prj_jacks.tik`, `prj_jacks_nosound.tik`, `prj_jackball.tik`; Alice `jacks_att_1`, `jacks_att_2`; native handlers `0x100be5a0`, `0x100be920`.

- **Primary:** requirement **10**, cycle/thrown timing **6.5 s**. Alice throw clip **0.90 s**, sound frame 4 (0.20 s), release frame 7 (**0.35 s**). `prj_jacks` declares speed **600**, damage **7**, knockback **30**, life **4 s**, bounce-on-touch, half-extents 8. Native primary spawning overrides initial motion (forward component **200**, upward **250**) and schedules a follow-up event after **0.1 s**. The weapon also caches a zero-damage jackball with speed 600 and life 5 s.
- **Alternate:** requirement **20**, cycle/thrown timing **2.5 s**. Alice clip **1.05 s**, sound frame 4 (0.20 s), release frame 8 (**0.40 s**). `prj_jacks_nosound` declares speed **600**, damage **4**, knockback **30**, life **4 s**, bounce-on-touch. The native alternate handler has a **16-iteration projectile-spawn loop**, with different first/subsequent model branches. This is not evidence that all sixteen necessarily hit or all share one damage value.
- **Special rules/effects:** bouncing, spinning/lit jacks with trails and bounce audio; a primary swarm managed by native follow-up behaviour. Homing/target allocation, primary swarm count, return rules, exact alternate layout/model mix and named-`jacks` ammo accounting remain to be fully decoded. No invented total damage or guaranteed hit count is assigned.
- **Current/gaps:** equipped model only. Both attacks, swarm/burst controller, debit/reuse rules, target steering, wall bounces/returns, damage/knockback, original events and saved swarm state are missing.

## 7. Demon Dice

**Original sources:** `w_demondice.tik`, `w_throwndice.tik`, `c_demon_lesser.tik`, `c_demon_normal.tik`, `c_demon_king.tik`, `prj_fireball.tik`, `fx_demonking_icebreath.tik`; Alice `dice_att`.

- **Input/resources:** **one summon mode**, reached by either attack button. **40 Will**; `maxamount 3`; cycle/thrown timing **6 s**. Alice clip **0.90 s**, release frame 7 (**0.35 s**). Throw/roll, summon, attacks and disappearance have distinct creature/projectile events. Demon health is **50 / 100 / 200** for lesser/normal/king.
- **Original attack declarations:** lesser melee frame 8 gives **10** electric damage; ranged frames 9–15 each issue a beam with damage argument **2**. Normal melee has four **5-damage** contacts at frames 6–9, plus another five-contact variant at frames 5–9; ranged fireball releases frame 20, damage **25**, speed **700**, life **2 s**. Normal demon also declares shield/bounce-off behaviour. King has **20-damage** melee variants (frame 12 or frame 3), ice-breath events at frames 11/12/15/18/21 (projectile damage **15**, speed **800**, life **2 s**), and beam events at frames 10/16/18/20 with damage argument **2**. Other run/attack variants exist. Beam event parameters and target exposure must be resolved before claiming aggregate DPS.
- **Current:** 40 Will at action acceptance; release frame and six-second cooldown are implemented, with cooldown starting at release. Only one rolling batch/demon may be active. One to three dice select a tier using independent roll thresholds (normal at sum ≥9, king with three dice and sum ≥13). Health values match. Lesser uses the listed melee/seven beam contacts; normal uses four melee contacts or a **25-damage** fireball; king uses frame-12 melee or four **5-damage** beam contacts. Current fireball speed/life are **600 / 3 s**. Navigation, attack selection, hostility when no eligible enemy exists at summoning, and a **30 s** demon lifetime are independently implemented. Rolls, cooldown, demon and projectile state persist in Rust saves.
- **Gaps:** original roll probabilities/tier thresholds and hostility/lifetime parity are not established. Missing normal shield and extra melee variant, king ice breath and other attack variants; king's current beam value **5** differs from the original event argument **2**. Fireball speed/life differ. Enemy retaliation against summons, full navigation/target choice, native beam semantics, damage types and complete audiovisual events remain unfinished. The existing school combat check verifies the implemented subset, not every demon tier/move. See [DICE.md](DICE.md).

## 8. Jabberwock's Eye Staff

**Original sources:** `w_eyestaff.tik`, `prj_eyestaff_comet.tik`, referenced charge/beam/spiral/launcher effects; Alice `staff_att_*`; torso Staff states; native Eyestaff handlers.

- **Primary:** held beam/charge sequence. Declared requirement **0.4**, range hints **50–6,000**. Middle clip **0.25 s**, end clip **0.90 s**. Alice's middle alias invokes `fire` on entry and frame 0; the native start handler has a guard against duplicate activation. The original **does not** imply two separate beams per animation cycle. State exits on button release or insufficient mana and calls the beam-stop event.
- **Alternate:** held sky attack. Declared requirement **1**, range hints **300–6,000**. Full-body startup **0.60 s**, repeating middle **0.25 s** with sky-attack event at frame 0, ending **0.80 s**; release stops the sky attack. The installed alternate state checks the primary mana predicate; this is recorded as source behaviour rather than silently replaced with a presumed correction.
- **Native/resource caveat:** beam-start handler `0x10072780` also tests Will **greater than 20**, while setup `0x10072470` schedules stages with **2.15 s / 2.30 s** delay constants. These establish additional gating/staging beyond the TIKI's 0.4 requirement. Exact charge completion, damage pulses, depletion/refund and alternate drain/launch cadence are **not fully resolved**. Do not implement these as a 0.4-Will instant hitscan and a 1-Will instant explosion.
- **Projectile/effects:** active comet definition has speed **1,000**, life **2.5 s**, **100 direct damage**, **100 radius damage**, radius **100**, knockback **400**, eye-beam damage type. Charge/spiral/beam/launcher effects, lights, smoke, impact effects and stop-event cleanup matter. These declarations do not supply the primary beam's complete damage formula. Similarly named `prj_eyestaff.tik` is an obsolete Jackbomb-like definition and is **not referenced as this weapon's active projectile**.
- **Current/gaps:** equipped model only. Both held modes, charge and release state machines, Will gates/drain, beam/sky targeting, damage and arena interactions, original animations/effects/audio and mid-charge saving are missing. Primary beam damage and total Will use remain native-research blockers, explicitly distinct from known comet values.

## 9. Blunderbuss

**Original sources:** `w_blunderbuss.tik`, `prj_blunderbuss.tik`, `fx_blunderboom_wall.tik`; Alice `buss_att`; weapon `firing.tan`.

- **Input/resources:** both buttons enter the same full-body cannon attack. Declared requirement is **99 Will**. Alice's clip lasts **3.65 s**; frame 1 (**0.05 s**) invokes weapon `fire`, with a later recovery vocal at frame 25 (1.25 s).
- **Important two-stage timing:** weapon `firing.tan` lasts **1.05 s**, interval 0.05 s, and issues `shoot` at its frame **14 (0.70 s)**. Nominal projectile release is therefore approximately **0.75 s after Alice's attack starts**, subject to engine event scheduling. The Alice frame-1 event starts the weapon animation; it is not an immediate projectile release. Weapon ignition sound is frame 0, strikeplate smoke frame 4 and muzzle smoke frame 15.
- **Damage/physics:** custom `cannonball` projectile, speed **2,500**, direct damage **8**, knockback **60**, life **5 s**. Referenced wall explosion declares **998 radius damage**, knockback **800**, life **2 s**, Blunderbuss damage type. Its definition has no explicit explosion radius; the projectile's **600 impact-mark radius is a decal size**, not evidence of a 600-unit lethal radius. Native cannonball world/actor impact rules and splash falloff need inspection before assigning a total or guaranteed kill.
- **Current/gaps:** equipped model only. Will gate, movement/recovery state, delayed firing, projectile and custom impact logic, splash damage/knockback, recoil/presentation and ignition/muzzle/explosion events are missing. The necessary clips exist in the archives but are not loaded as a playable action.

## 10. Deadtime / Pocket Watch

**Original sources:** `w_watch.tik`; Alice `deadtime_watch`; native `deadtimewatch` handlers `0x100bf770`, `0x100f7130`.

- **Input/resources/timing:** both buttons use one time-stop action. **1 Will**, declared cycle **360 s**; native stop duration **20 s**. Alice clip **2.15 s**; particle setup at frame 20 (**1.00 s**), weapon fire at frame 22 (**1.10 s**). Weapon fire then invokes the native stop event and use sound at frame 0 of its own animation.
- **Special rules:** original entities/projectiles declare whether they ignore deadtime. The result is selective time suspension, not a blanket freeze of every animation and projectile.
- **Current:** 1 Will / 20 s stop / 360 s recharge, ownership/alive/recharge checks, original Alice/Watch art and use sound. Both buttons activate the same effect. Supported enemies and movers stop; Alice and her attacks/projectiles can continue. Timers persist in saves and pause correctly. However, [inventory.rs](../src/inventory.rs) starts stop/recharge **immediately on accepting the click**, and the action's fire/sound frame is 0, rather than original frame 22.
- **Gaps:** delay gameplay activation/sound until the authored release, define cancellation before release and debit timing, restore the source particle sequence, and audit each class's deadtime exceptions. In particular the original knife opts into deadtime while current shared player projectile motion continues. Existing Watch checks establish the current supported world freeze and saved expiry, not full per-class parity.

## Implementation priorities and unresolved research

| Priority | Work | Completion evidence needed |
| --- | --- | --- |
| 1 | Shared held/pressed/released input, per-mode attack states and resource/cycle handling | Mouse, rebound key and controller tests for hold/release, busy/equip input, insufficient Will, cancellation, pause and reload |
| 1 | Correct the three prototype toys' numeric and projectile differences; Watch release timing | Per-contact/resource assertions plus observable release/recovery timing; original source values must drive gameplay, not just the catalog |
| 2 | Jackbomb: first currently missing toy by inventory/campaign rank | Both modes, clear/tight rooms, flame sweep axis, timed explosion/Will, enemy and self/environment interactions, save/restart |
| 2 | Ice Wand and Jacks native rules, then implementations | Freezing/wall placement and expiry; swarm targeting/burst/return; proper debit and saved entities |
| 2 | Complete Dice's attack repertoire and native parity research | All tiers/variants, roll boundaries, hostility, shield and retaliation, lifetime and restarts |
| 3 | Staff and Blunderbuss | Resolve Staff beam damage/drain and cannonball explosion rules first; then held/charged/delayed attack and cancellation tests |
| Across all | Damage typing, target-specific reactions, knockback, deadtime flags and event-driven effects/audio | Tests against each supported enemy family, water and geometry, plus frame-by-frame and audible comparisons |

Still **unknown**, not zero or absent in the original: exact native Cards fragmentation/steering and reload cadence; complete Jacks swarm/return/ammo semantics; Ice Wand freeze/shatter/underwater effects and effective tick rate; Dice roll/AI/beam semantics; Staff beam damage and complete charge/drain schedule; cannonball radius/impact rules; general splash stacking/falloff, original held-repeat cadence and interruption ordering. These are scoped research tasks, not reasons to treat placeholder mechanics as faithful.

## Audit verification and provenance

- Re-read all ten weapon definitions and recursively referenced active TIKI dependencies; indexed **74 reachable definitions**, **27 Alice attack/equip aliases**, four selected prop animation headers and the ice-wall state graph. Recorded **103 distinct archive-asset hashes**. Values above come from this local version, not an assumed release or online summary.
- Original `fgamex86.dll` SHA-256: `2ead219ac0ff3afcbda8b68d0b8fc917bfd90468072a97f6fbc6581c8f60b5b5`. Ghidra used the existing project with `-readOnly -noanalysis`; temporary function definitions were discarded. Twenty targeted native functions were inspected/exported privately. Common melee helper: `0x100fe6e0`; common damage handler: `0x1006cf00`; attack predicates: `0x100a5a30` / `0x100a5aa0`.
- Ran the available release executable's `--weapon-check`, `--dice-check` and `--items-check`: all passed. These validate 13 prop models/12 currently supported action clips, implemented Dice combat at 30/60/144 Hz, resource rules and supported Watch freeze/resume/save expiry. **They do not exercise the five missing abilities or certify original fidelity.**
- Checked executable SHA-256: `a1a57fa9b36fbbc833506c6c2a12aa089db0649cdeaca2fe8a7b4a92ac441ed5`. Verified all document links and named model references, evidence counts and the compared Rust source hashes; no gameplay-source drift was detected during the audit.
- No original executable was run, no new audible/visual side-by-side original playthrough was performed, and no gameplay/build/installation changes were made for this audit. Original runtime confirmation remains necessary for the explicitly unresolved items.

Implementation entry points: [weapons.rs](../src/weapons.rs) (support gate, action frames, props, projectiles); [combat.rs](../src/combat.rs) (prototype costs/damage, swept contacts); [dice.rs](../src/dice.rs) (summons); [inventory.rs](../src/inventory.rs) (catalog, resources, Watch and damage scaling); [powerups.rs](../src/powerups.rs) (difficulty and timers); [viewer.rs](../src/viewer.rs) and [input.rs](../src/input.rs) (input acceptance and world clock integration). Existing feature notes: [WEAPONS.md](WEAPONS.md), [COMBAT.md](COMBAT.md), [DICE.md](DICE.md), [ITEMS.md](ITEMS.md).

## September 28: Demon Dice and Pocket Watch follow-up

[Demon Dice and Pocket Watch](DICE_WATCH.md) records restored normal claw/king follow-through, ice and charge attacks, enemy retaliation, shield/pain/death rules, Rage rolls, failed-summon refunds, saved combat effects and selective time suspension. **Correction to the historical audit above:** the native `ignore_deadtime` handler clears the affected flag for both omitted arguments and explicit zero. The Blade and Staff spiral therefore continue during the Watch. All 17 distinct modes across ten implemented toys have native active-Watch save/restart checks. See the follow-up for fidelity limits rather than treating the historical snapshot as current.
