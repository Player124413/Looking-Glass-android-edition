# Shared weapon input and action handling

28 September 2026. The playable toys now use a shared, funded action scheduler. The original torso state graph, Alice animation events and weapon definitions determine input, release timing and recovery. The [toy audit](TOY_AUDIT.md) remains the baseline for projectile, damage and effect fidelity.

## Player behaviour

- Tap for one attack; hold to repeat when the current attack and that toy's recovery allow it. Releasing a normal swing or throw does not abort its already-started animation. Busy taps are not saved up for later.
- Primary wins when both buttons are held, matching the ordering in the original standing state. Dice, Blunderbuss and Watch map both buttons to one mode.
- Selecting a toy during an attack queues the switch until the current action finishes. Changing the selection again replaces that pending selection. Equip animations also finish before another switch; their source frame 5 changes the visible prop.
- Will is checked before starting, and committed once at the authored weapon event. Cancelling before that event through death, scripted movement, a conversation/cinematic or recovery does not spend Will. Once an attack has fired, cancellation cannot refund it or recall its projectile. Switching alone does not interrupt a committed swing/throw.
- Insufficient Will selects an owned Vorpal Blade. Unsupported toy attacks and unowned toys never spend Will. Dice additionally require no existing roll/summon and their recovery to have ended.
- Pause and menus freeze action/recovery clocks. Menu, focus, controller-disconnect and scripted-control boundaries require the attack buttons to be released before another held attack starts. A click used to close/resume an interface cannot also fire.

Mouse, rebound keyboard keys and Xbox/XInput triggers feed the same held-input path. The controller's existing trigger threshold/deadzone still applies. Wheel bindings remain impulse inputs rather than pretending to be held buttons.

Eye Staff and Blunderbuss are now enabled; the Staff custom channel owns periodic Will consumption and exact release events. See [STAFF_BLUNDERBUSS.md](STAFF_BLUNDERBUSS.md).

## Rules and availability

Times are measured from the Alice action's start. Weapon recovery starts at its firing event and persists while another toy is equipped.

| Toy | Action/release rules | Availability in this build |
| --- | --- | --- |
| Blade | Primary contact 0.35 s; throw 0.40 s; 3.5 s thrown recovery blocks reuse and keeps the knife out of the hand | Both attacks playable with original damage, sweep, flight and reappearance rules; see [Blade/Cards fidelity](BLADE_CARDS.md) |
| Cards | Primary immediately; alternate 0.35 s; 0.25 / 0.85 s clips; alternate cycle 0.5 s | Both playable; 3 / 20 Will, original carrier plus eight fragments, seeking and wandering; no active reload pause in the supplied configuration |
| Mallet | Contact 0.35 s; ball 0.65 s; 0.8 / 1.35 s clips | Both playable with original melee, ball physics, five-second life and splash; alternate 8 Will; see [Mallet/Jackbomb fidelity](MALLET_JACKBOMB.md) |
| Jackbomb | Both release 0.35 s; primary/alternate reuse 3 / 8 s | Both playable; 15 / 20 Will; three-second explosion or ten-second rotating fire, saved fuses and flame events |
| Ice Wand | Primary repeats its 0.05 s clip while held; wall event 0.45 s in a 1.6 s clip | Playable stream, wall and underwater freeze; 0.75 per pulse / 10 Will per successful wall |
| Jacks | Primary release 0.35 s, 6.5 s recovery; alternate 0.40 s, 2.5 s recovery | Playable seeking swarm / sixteen-piece burst; 10 / 20 Will; saved targets, trajectories and return events |
| Dice | Both buttons release once at 0.35 s; 40 Will then; 6 s recovery plus active-summon gate | Playable |
| Eye Staff | Primary charge/middle/sustain/end states; charge stage 2.3 s, ending 0.9 s. Alternate startup 0.6 s, held middle, ending 0.8 s. Release ends the held effect; primary can end during charge, alternate finishes startup. Middle-loop events cannot create duplicate effect instances | Playable charged beam/release spiral and delayed comet shower; saved periodic debit/targets/effects |
| Blunderbuss | Both buttons use the same 3.65 s action; 99 Will at Alice's 0.05 s weapon-start event; projectile event 0.70 s later | Playable original cannonball, radial damage and saved ignition/discharge |
| Watch | Both buttons activate and spend 1 Will at frame 22, 1.10 s; 20 s stop, 360 s recovery | Playable; no immediate click-time activation |

All ten backends are enabled. The Staff retains its native primary greater-than-20 start requirement and alternate one-unit requirement; its custom state owns periodic 0.4/1 Will pulses. Alternate Staff and Blunderbuss use full-body actions and suppress locomotion input. Stop events carry subframe offsets so release cannot create duplicate projectiles or extend resource consumption to the next display frame.

## Implementation and save safety

[weapon_rules.rs](../src/weapon_rules.rs) centralizes mode aliases, release/debit/sound times, costs, cycles, phase types and device-neutral button transitions. [weapons.rs](../src/weapons.rs) advances an action across exact event/end boundaries, retains unused frame time for the next action, and records which debit, sound and fire events have already occurred. Effects receive captured action/weapon records, so a same-frame switch cannot dispatch an old shot as the new toy. Equipped/charging/ending phases and cooldowns are serializable.

The interactive viewer passes the actual player wallet to the scheduler through `Character::update_funded`. The old viewer-side click debit is removed. The unmetered route/render-fixture wrapper runs the same scheduler; it is not used for interactive gameplay. Cosmetic windup sounds may occur before the release event, as in the original.

Old save actions default to **already paid**, because previous builds charged at the initial click. Loading an unfinished old attack therefore cannot charge its new cost a second time. New saves retain an explicit unpaid/paid state and per-toy recovery timers. Restoring a post-release action cannot emit its projectile twice. Existing optional fields remain backward-readable by this build; downgrading new saves to older binaries is not supported.

Source evidence for the debit boundary is the original native `Weapon::Fire` (`0x100fa480`), which calls `UseAmmo` (`0x100fa050`) before starting the weapon's fire animation. The Blunderbuss's later TAN `shoot` is consequently separate from its debit. Torso attack states do not accept `NEW_WEAPON` until returning to the standing state. The Staff has explicit exit stop events and native duplicate-start guards. Private bounded Ghidra notes are under `private/weapon-action-research/`; no original code or assets are included in this document.

## Original shared-input verification

The following records the earlier shared-input build. Current Staff/Blunderbuss verification and the later installed build are recorded in [STAFF_BLUNDERBUSS.md](STAFF_BLUNDERBUSS.md).

- The shared checkout passes 265 unit tests and strict all-target Clippy. Logs are `private/weapon-input-tests.log` and `private/weapon-input-clippy.log`; the release source hashes are retained in `private/weapon-input-build-sources.json`.
- Unit checks cover all ten mode mappings, repeated fire at 30/60/144 Hz, tap/release, queued switching, pre/post-release cancellation, mode aliases, charge/stop transitions, Blunderbuss's separate debit/discharge, Watch timing, cooldowns across switching, pause, insufficient funds, unsupported toys, and new/legacy saved actions.
- `--weapon-input-check` runs the funded scheduler against real original clips at 30/60/144 Hz. Four seconds of held Cards gives 16 primary releases and exactly 48 Will spent; alternate Cards, Mallet and Watch have their own count/resource assertions.
- Existing `--weapon-check`, `--dice-check` and `--items-check` pass. The action asset check now loads 20 clips.
- The combined build installed at that checkpoint passed all four checks, including `--weapon-input-check`. Its SHA256 was `C703B6F65D6AF05D8ABDAFDEB3754E966DA579261DDC61BE5416C38D5A9474E2`; `private/weapon-input-launcher-validation.json` records that executable. It was verified in place to preserve the concurrent rendering update.
- Anode desktop input exercised a rebound keyboard attack key and an isolated Xbox controller trigger. The paused quick-save recorded ten card attacks, ten projectile releases and **70 Will remaining from 100**, with God mode off. Saves/settings were isolated under `private/weapon-input-desktop`; the user's settings and saves were not changed.

Those checks established shared action handling. Weapon-specific behaviour is covered by the later fidelity documents linked above; neither set of checks certifies every original enemy reaction or every cosmetic engine event.
