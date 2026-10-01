# Weapons — updated for version 0.28

**Shared input update:** tap or hold to attack; switches wait for the current action, debits occur once at the authored event, and thrown-weapon recovery survives switching. The Watch activates at frame 22. See [weapon action rules and verification](WEAPON_ACTIONS.md).

For the source-by-source comparison of **all ten toys**, including original costs, input, timing, damage and remaining gaps, see the [28 September 2026 toy audit](TOY_AUDIT.md). The feature descriptions below describe the Rust implementation; they do not imply original combat balance.

The Pocket Watch now activates with either mouse button using the original `deadtime_watch` Alice clip, animated Watch prop and use sound. It costs 1 Will, stops supported world actors for 20 seconds and recharges for 360 seconds. See [ITEMS.md](ITEMS.md) for difficulty, saving and time-stop scope.

All ten inventory toys now display their original held models on Alice's animated `tag_weapon`. The equip animation shrinks the old model, switches at source frame 5, and grows the new model. Armed walk/run clips distinguish large and small weapons. The `ready` clip supplies the resting stance.

The Blade, Cards, Mallet, Jackbomb, Ice Wand, Jacks, Demon Dice, Eye Staff and Blunderbuss have implemented attacks against supported enemies; the Watch stops time. See [COMBAT.md](COMBAT.md) and [DICE.md](DICE.md). Eye Staff and Blunderbuss are now playable; see [their rules, source evidence and verification](STAFF_BLUNDERBUSS.md). `tools/launchers/Launch-Weapon-Preview.cmd` or `--weapon-preview` explicitly unlocks all toys with full resources for inspection.

Mallet/Jackbomb behaviour, native evidence and restart verification: [MALLET_JACKBOMB.md](MALLET_JACKBOMB.md).

## Actions in this pass

| Toy | Left click | Right click |
| --- | --- | --- |
| Vorpal Blade | Three original swing clips; 25 damage at 0.35 s, native melee volume | 45 damage thrown blade at frame 8; original ballistics, hit volume and 3.5 s recovery |
| Cards | One seeking card, 7 damage / 3 Will, held repeat every 0.25 s | Loose card then carrier at frame 7; eight radial fragments after 0.1 s |
| Croquet Mallet | Two original swings; 24 damage / 100 knockback at 0.35 s, native 50-unit sweep | Ball launches at 0.65 s: 8 Will, 2,500 units/s, 30 direct damage, repeated bounces, five-second lifetime and splash |
| Jackbomb | 15 Will; bouncing box with a three-second fuse and 300-damage explosion | 20 Will; rotating mouth fire, ten-second fuse and final explosion |
| Ice Wand | Held stream: 0.75 Will every 0.05 s, distance-based damage and frozen enemy deaths | 10 Will for a solid, destructible, timed ice wall; underwater use consumes remaining Will and freezes Alice |
| Jacks | 10 Will; bouncing, seeking swarm, five-second attack then return | 20 Will; sixteen spreading jacks, swept contacts and ricochets |
| Demon Dice | Original throw clip, release at frame 7, bouncing dice and a summoned demon with melee/ranged attacks; 40 Will | Same original summon; no separate alternate mode in the installed definition |
| Pocket Watch | Original watch action and animated prop; 20-second stop | Same time stop |
| Eye Staff | Charge, hold a damaging beam, release a spiral projectile | Charge and release a delayed targeted comet shower |
| Blunderbuss | One cannonball: 99 Will, ignition at 0.05 s, release at 0.75 s, radial explosion | Same original cannon attack |

Tap for one action or hold to repeat when ready. Busy taps are not buffered. Selecting another toy queues an equip animation after the attack finishes. Pause, inventory and other menus freeze poses, effect ages and projectiles; release the attack buttons before resuming held fire. Home/level changes clear pending actions and transient effects. Unsupported toys display a brief notice and spend no Will. See [WEAPON_ACTIONS.md](WEAPON_ACTIONS.md) for cancellation and save rules.

When stationary and grounded, the original full-body action is blended in and out. During movement/jumps, only the spine and its descendants receive the action layer, retaining the controller's leg animation. Attack facing and contact aim follow the current look direction; animation root motion does not move the collider. The third-person camera has a small, collision-checked shoulder offset so Alice obscures fewer projectiles.

## First-person view

Version 0.11 renders all ten held toys in first person, selected with the same controls and campaign inventory. V switches views. The independently authored camera-relative positions, rotations and small movement bob keep equipment in view while looking up/down or moving sideways. These are presentation poses using the original prop models, not recovered original first-person animations; the view intentionally renders only the weapon and its effects, without hands, arms, body surfaces or transformation attachments.

Blade, Cards, Mallet, Jackbomb and Dice use the existing action clock and release/sound events. Primary swings move across the lower view with trails; the alternate knife disappears at release and reappears when its thrown recovery expires. Cards launch singly or as a carrier that sheds eight fragments; the Mallet grows for its swing and shows the ball before launch. Dice disappear at release, become visible world projectiles and return to the hand after the action. Held dice reflect the owned count. Jackbomb stays hidden through its three/eight-second reuse delay. Ice Wand and Jacks also use the action clock, original models and effects; Jacks stay hidden during their reuse delay. Eye Staff has charging/beam/comet effects, and Blunderbuss has its firing animation, smoke and recoil. Pause/menus freeze the presentation; recovery clears actions and bob. Death and free flight hide the local weapon.

Rendering finishes the world, projectiles, sparks and pickups first, with normal wall depth tests. Only then is depth cleared, preserving the world colour, for the local held model and swing ribbon. This prevents a nearby wall from cutting the held weapon in half while keeping distant effects occluded. The HUD is drawn last. Switching views clears the old swing ribbon without removing projectiles already in flight.

First-person throws originate at the displayed attachment, constrained by a sweep from the eye using the projectile's radius, and converge toward the sightline surface. Version 0.13 first checks for a living club guard on that sightline, so close shots aim at the guard rather than a wall behind it. The aiming trace uses a point rather than the projectile clearance plane so a hand blocked against a wall cannot launch parallel to that wall. Melee impacts use the eye's aim ray. Swept weapon contacts now apply damage once per melee release or projectile, stopping at the nearest guard before solid geometry.

## Original data and independent presentation

Model paths, scales, surface skins and idle TAN files come from `models/w_*.tik`; the loose card and ball use `w_card_loose.tik` and `croquetball.tik`. Attachment indices are resolved by bone names in Alice's SKB, not hard-coded numeric indices. Alice's model scale applies to tag position; each prop uses its own TIKI scale. Original SKA clips drive the hand/arm motion. The supported action mappings and release/sound frame numbers were read from `models/alice.tik`. No original script or executable runs.

Blade, Cards, Mallet and Jackbomb projectile speeds, lifetime, damage and collision rules follow the original definitions and inspected native handlers. Some swipe and impact visuals remain approximations. Collision uses the existing static-world sweeps; visuals cannot tunnel through a thin wall in a single frame. Spawn positions are traced from Alice's eye so an attachment reaching through a wall cannot spawn a visual beyond it. Projectile, trail and particle counts are bounded. Effects test depth but do not write it. Close-to-camera sparks fade to reduce point-blank glare.

Source sounds for switching, swings, throws and impacts use the existing audio engine and effects volume. Sounds are triggered at the corresponding action event, once even when a frame is skipped. This narrow event mapping is not a general TIKI event interpreter. The knife stays hidden through its 3.5-second thrown recovery. Blade return and Cards sequencing now follow the evidence in [BLADE_CARDS.md](BLADE_CARDS.md). Some particle and decal details remain future work; shared charge/input state is described in WEAPON_ACTIONS.md.

## Observed TAN version 2 layout

The new reader accepts `TAN ` magic and version 2. The 176-byte header stores frame/tag/surface counts at 72/76/80, total duration at 84, frame/surface offsets at 100/104, up to 16 tag offsets from 108 and file end at 172. Each 68-byte frame stores bounds, quantisation scale, translation bias, unused motion delta, radius and frame interval. Some supplied stationary files contain NaNs in the unused delta fields; these are ignored, while all positions/scales used for rendering must be finite.

Tags have a 64-byte name followed by 48 bytes per frame (position and three axes). A 104-byte surface header contains its name, frame/vertex/triangle counts and relative triangle/collapse/UV/packed-vertex/end offsets. A vertex stores three **unsigned** 16-bit coordinates and a packed normal; position is `packed * frame_scale + frame_bias`. Normals for lighting are reconstructed from triangles. Bounds, ranges, counts, timing and indices are checked before use. All frames are decoded and validated, but held props currently draw frame 0; Alice's skeletal attachment provides their movement.

## Checks

`--weapon-check` validates seventeen prop models (ten toys, three attachments, Blade projectile and three Cards projectile classes), their textures, twenty action clips and the three required Alice attachment names. The supplied props contain 7,898 decoded vertices across their frames. `--weapon-input-check` checks the funded action scheduler against real clips. Synthetic tests cover malformed TAN ranges, unsigned coordinates, event timing across frame rates, pause/cancellation, pose masking and high-speed projectile wall collision/bounce limits. Dice combat, placement, resource and restart checks are described in [DICE.md](DICE.md).

Original assets and demonstration captures remain local under `private/`; source packaging includes no game data. See [VALIDATION.md](VALIDATION.md) for the performed desktop checks and their limits.

Ice Wand/Jacks mechanics, native evidence and save verification: [ICE_JACKS.md](ICE_JACKS.md).
