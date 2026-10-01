# Duchess encounter — version 0.26

## Jackbomb pickup camera repair (2026-10-01)

The expansion camera previously retracted from the boss's final position while
the fireplace was still moving. That could put the view through the chimney,
showing missing surfaces and abrupt camera motion. The introduction now plays
the four local camera paths, with dialogue cuts and close-up framing for the
port's standing actors. The camera clock pauses and is saved with the encounter;
older saves default this optional presentation state. Save format remains 12.

The room's moving walls, beams and doors now reuse prepared brush shapes.
They retain the previous collision geometry, including the solid-only mask.
The focused check compares 512 sweeps at translated and rotated poses against
the former construction path. All 1,677 samples of the watched introduction
keep the camera clear of geometry, with no movement over 6.51 units per frame
at 60 Hz within an individual shot. Pause and camera restoration are checked.

The full encounter route still collects the Jackbomb, fights the living boss,
returns the shell and reaches the temple exit. Native evidence includes the
continuous introduction and existing attack/victory captures. Logs, the old
clipped view and corrected captures are in `private/duchess-intro-fix/`.
This repair retains the existing simplified actor performance and encounter
timing; it does not claim an exact reconstruction of the original cinematic.

## Main update: effects and victory focus (2026-09-30)

The original Duchess emitters now run on her animation clock: pepper attacks,
all three defeat sneezes, the blood burst, and the dark pepper smoke/purple
Meta effects from the neck. The timed blood-spurt attachment also renders.
Pig creation/impact and pepper projectile effects use their local asset
declarations. Clip-specific burst selection prevents death events from being
sampled during an attack; prefix-random particle lifetimes are supported.
For readability, this actor's billboards use 0.4 of the renderer's default
size and 0.65 opacity. Emission timing, tag positions and travel are unchanged;
this tuning is not a claim of exact native particle dimensions.

Phasing now uses `fx_duchess_trail` silhouettes and the peppermill emitter.
Afterimages follow actual movement at 0.1-second intervals, with at most ten
copies and a one-second fade. They replace the blinking body and sphere cloud;
their opacity is an approximation. The current body is translucent throughout
the existing phase window. Gameplay movement, damage and immunity are unchanged.

The victory camera tracks the animated head, then blends to the neck when the
head surfaces disappear. Its 300-unit orbit and collision avoidance keep the
emission point framed. Exact native orbit interpretation, head swelling and
thrown head/brain geometry remain fidelity work. The existing 12-second scene,
skip result, rewards and exit are unchanged. Cosmetic history is not saved;
save format remains 12, with no event-key or hit-range changes.

`--duchess-render` includes three sneeze stages, blood, late smoke and a moving
phase trail, with visible-effect pixel comparisons and paused-frame checks.
See `VALIDATION.md` for current results. The version-0.26 figures below are
historical milestones.

Current facial support: original lip-sync envelopes and Alice’s blink texture are now connected for implemented dialogue. See [facial animation, verification and remaining limits](FACIAL.md). Earlier facial limitations below describe previous milestones.

The first missing campaign boss is restored in `potears3`. Open **tools/launchers/Launch-Duchess.cmd**, choose **N** for a fresh visit, walk down the entrance ramp, enter the house and jump onto the table to collect the Jackbomb. Collection, rather than weapon ownership or touching the high editor trigger, starts the encounter. The original level-entry resource refill is applied once. The earlier fortress/Pool of Tears journey and the temple's own progression still need separate work.

## Fight and arena

Five wall assemblies, ceiling, four beams, the entry door, fireplace bars, secret door, well lever and well lids use the map's original geometry and named small/large positions. Decorations and the shell follow their parents. The room expands over three seconds, the Duchess grows from the chimney, the entrance seals, and the original introductory dialogue plays. The staged player position is the script's arena mark. Active combat recovery stays inside the sealed arena.

The Duchess has 600 health. She pursues on supported ground with collision-checked local detours, runs beyond 350 units, and uses close attacks inside 110 units. All currently supported damaging toys can hit her. Both knife attacks, Cards, Mallet and Dice retain their existing costs. The Jackbomb is collected and carried onward, and its attacks are implemented ([MALLET_JACKBOMB.md](MALLET_JACKBOMB.md)), but the fight still selects the Blade after the introduction, until her first attack, if the new Jackbomb is selected. Use 1–3 or 7 to choose another toy.

Her states are pursuit, pepper shot, pig shot, mill smack, grab/bite/toss, phasing dodge, pain and death. There are no invented health-threshold phases. Incoming swept projectiles can cause a temporary nonsolid/untargetable phase; translucent afterimages and pepper emission show that window. She is vulnerable outside it. Accumulated 50 damage causes pain, preventing every small hit from restarting the pain clip. Her 120 Hz state machine saves its substep remainder and event cursor.

Animation-frame cues come from local model declarations: pepper releases at frame 7 (750 speed, 25 direct damage); pig at frame 21 (600 speed, 70 direct damage); the smack hits once at frame 9 for 15 damage; bite deals three 5-damage hits at frames 34/44/54 and releases/tosses Alice at 64/65. Pig impacts have the declared 400-unit explosion radius, 15 maximum splash damage and 400 knockback, with distance falloff and world occlusion. Projectiles cannot hit through solid walls. Interrupted grabs release control, and physics resumes for the toss. Four original medium-essence locations restore 25 resources and replenish ten seconds after a successful collection.

## Defeat and reward

Death stops attacks and plays the original death animation, weapon removal, head-surface change and timed sneeze/fall sounds. Bill and the Mock Turtle emerge through the secret door. The shell disappears from its display and is restored to the Turtle's attachment. Original farewell voices/subtitles accompany this change. The lever and well lids open; the staged Turtle/Alice departure leads to `utemple`. The physical changelevel trigger is also gated, so entering the well early cannot bypass the boss. Inventory, selected toy and current resources carry through the normal campaign transition.

Hold **Enter** for about 0.65 seconds to skip either cinematic; **E** advances one dialogue line. Skip commits the same room, shell and exit state as watching. It does not defeat a living boss. A fresh key press is required for another scene. F5/F9 work through the encounter and the result; loaded games start paused (P resumes).

## Saves and verification

Save format 8 retains the boss state, health, action/cue clocks, shots, explosion effects, essence cooldowns, shell reward and exit latch. Older formats remain readable. Cached old `potears3` visits gain the controller and gated triggers. An obstructed position from an older unsupported arena is moved to the normal entrance. Saves claiming a living defeated boss, an early shell reward or malformed projectile state are rejected.

- `--duchess-check`: normal entrance, actual pickup/jump, live combat and original weapon animation releases; 16,861 physics ticks (re-baselined 2026-09-29), 20 thrown knives, 38 cards, 44 boss attacks, 13 phase dodges, 55 damage and **45 Sanity** at the temple transition. No flight, warps, invulnerability, direct health edits or recovery during this route. The action scheduler samples the aim at release, so the driver aims at the boss, leading it for the projectile flight, with no allowance for Alice's own movement; the earlier windup allowance put throws up to 128 units behind the boss (47 throws, none landed). The map's authored cinematic reposition is used.
- The same command checks early-exit refusal, ownership-only activation refusal, pause, 30/60/144 Hz consistency, saved phase reconstruction, and full-duration versus skipped cinematic outcomes, including identical settled arena transforms.
- Five new boss unit tests cover temporary immunity and pain threshold, wall-blocked/once-only melee, swept pepper/pig contacts, grab interruption/release, and mid-attack deterministic continuation. A pickup regression also prevents an already-owned Jackbomb at full Will from blocking activation. All **139** project unit tests pass, as does strict Clippy.
- Separate native writer/reader processes pass **38** cases, including ten Duchess states (expansion, introduction, phase, pepper, pig, bite, death, rescue, reward and completed encounter). Existing legacy-save cases also pass.
- Anode rendered all ten staged scenes. Real P/held-Enter/F5 input skipped the introduction into a living fight and saved it; a separate farewell skip loaded and autosaved `utemple` with the shell restored and Jackbomb/Dice inventory retained. These native fixtures are separate from the normal-route proof.
- The first school route passes at 21,033 ticks/73 Sanity, and the Pandemonium skip route at 21,115 ticks/55 Sanity (both re-measured 2026-09-29).

## Fidelity limits

This is an independently authored encounter, not execution of original AI or cinematic scripts. Attack selection is deterministic rather than the original random chances. Steering, phase duration/cooldown, grab attachment, explosion falloff and cinematic camera/actor interpolation are approximations. The original body-controller stretching, thrown head/brain debris and exact translucency are not reproduced. Sprite particle declarations are connected as described above; native particle collision, mixed sprite/model spawn lists and exact distribution remain approximations. Enemy bodies retain the prototype's general non-solid interaction with Alice. Walking-rock background animation is omitted in this arena. Jackbomb use against her is not yet covered by a check (the automatic Blade selection also remains) and the surrounding campaign maps are separate unfinished work. Native tests used Anode's no-audio-device session; voices/sounds were loaded and timed, but this update does not claim a fresh audible-output verification.

Private test logs and screenshots live under `private/duchess-*`; user save slots were not used. The local source-review archive excludes game data, executables, screenshots and saves.

An actual v0.25/format-7 potears3 save was loaded and resaved in format 8 with identical player/resources and the new encounter/gates initialized. The original fixture remains in `private/duchess-legacy-v7`; logs are `private/duchess-legacy-{write,read}.log`.
