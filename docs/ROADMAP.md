# Remaining work toward a playable Rust implementation

**Version 0.27 restores shared environmental movement:** authored currents, launch volumes and updrafts; finite air and drowning; campaign rope climbing/swinging; and knockback from supported enemy attacks. Rope controls are **E** to grab/release, **Space/Ctrl** to climb and **WASD** to swing. The breathing upgrade, remaining air, grip and impulses persist in saves. See [movement controls, original evidence and verification](TRAVERSAL.md). Map-specific unfinished sequences still need their own restoration.

**Version 0.26 adds the Duchess boss in `potears3`:** expanding/sealed arena, pursuit and phase dodges, pepper/pig/melee attacks, timed defeat, Mock Turtle shell reward and gated temple transition. Saves and held-Enter cinematic skipping are supported. Use **tools/launchers/Launch-Duchess.cmd** (N for a fresh visit). The normal entrance-to-exit fight passes without cheats. Jackbomb attacks were unfinished at this version and are now implemented ([MALLET_JACKBOMB.md](MALLET_JACKBOMB.md)); the surrounding campaign journey remains unfinished. See [encounter details and verification](DUCHESS.md).

Version 0.25 restores Pandemonium's four main cinematics with original camera tracks, timed actor movement/animation, world changes, persistent scene state and hold-Enter skipping. Watched/skipped routes both reach Fortress 1. See [CINEMATICS.md](CINEMATICS.md) for scope and remaining visual/audio approximations. Fortress 1's opening remains unfinished.

Version 0.24 restores the first school return (`skool1_start2`), including its changed platforms/cast, inherited potion and star, observatory lift, globe opening, drinking/shrinking sequence and `potears1_start1` exit. First and return visit saves stay separate. See [SCHOOL-RETURN.md](SCHOOL-RETURN.md). The fortresses and later unfinished visits/maps remain outside this change.

Version 0.23 restores Pandemonium from its normal entrance through the rope descent, minecart, Cards/key loop, guarded return and airship transition. See [PANDEMONIUM.md](PANDEMONIUM.md) for the input-route proof and cinematic limits. Fortress 1 was then the earliest unfinished campaign map; a native exit test confirmed that its missing intro left Alice in the dark staging room. The fortress maps blocked a full campaign playthrough until they gained verified normal routes (see [FORTRESS.md](FORTRESS.md) and [BEYOND.md](BEYOND.md)); the first school return was restored in v0.24.

Version 0.22 implements the Pool of Tears' Ladybug acorn bombers: authored patrol groups, four paired ambushes, collision-checked navigation, timed attacks/rearming, original animation/sound presentation, damage/death and save restoration. See [LADYBUGS.md](LADYBUGS.md). Remaining enemy types in this area include Snarks, Army Ants and Bloodroses; leaf rides, progression and story still need work before a full route can be verified. Ladybugs on later maps and exact original flight/effects behavior remain pending.

Version 0.21 implements Demon Dice, the next toy in campaign order: one original summon action on either mouse button, 40 Will, rolling dice, three original animated demon types, melee/ranged damage, effects/sounds and save restoration. See [DICE.md](DICE.md). Jackbomb is the next unfinished campaign weapon. Full original demon AI, enemy retaliation against summons, the king's extra moves and exact effects remain fidelity work.

Version 0.20 adds a shared entity registry and bounded event dispatcher with activation conditions, persistent variables, enable flags, delayed events and one-shot history. Existing school interactions verify the foundation, including saves across restart and migration of v0.19 cached visits. See [EVENTS.md](EVENTS.md).

Version 0.19 adds versioned local quick/automatic saves, restored campaign visits and Continue after restart. See [SAVES.md](SAVES.md). This preserves the gameplay implemented so far; it does not complete the remaining campaign.

Version 0.18 restores village bridges/machinery through its original Pandemonium exit, first-school enemy activations, and its optional Looking Glass secret. The school route still completes the library and recipe exit with combat active, both with and without the secret detour. **The original village-to-school journey was incomplete at this version:** Pandemonium was completed for normal traversal in v0.23; both fortress maps needed traversal, puzzles and progression gates, and now have verified normal routes ([FORTRESS.md](FORTRESS.md), [BEYOND.md](BEYOND.md)). No shortcut to school has been added. See [VILLAGE.md](VILLAGE.md) for the chain and its verification limits.

Version 0.15 restores selected original dialogue across `gvillage`, `skool1` and `skool2`, starts the village at its playable arrival, and implements the gym lever/bleachers. Version 0.16 connects school-two traversal, movers, Boojum combat and its main ingredient/potion quest. Exact cinematic choreography, remaining enemy types and the first school's return visit remain campaign work. See [STORY.md](STORY.md).

Version 0.14 connects the first school visit's traversal and book-puzzle progression to the next map. It builds on the rendered environments, Alice/NPCs, basic club-guard combat, weapons, music, swimming and recovery. Cinematics, complete enemy scripting, the school's return route and the rest of the campaign still need substantial work. See [SCHOOL.md](SCHOOL.md).

## Completed milestone: move through one room

Static collision now uses BSP planes, brushes, brush sides and world-model ranges with an independently built bounding-volume hierarchy. It also includes tessellated curved-surface collision. A 120 Hz controller implements gravity, ground contact, wall sliding, stairs, slopes and jumping. Free flight remains available through F4. Synthetic fixtures and a real school-level route pass without solid overlaps. This milestone uses an upright box body rather than the originally proposed capsule.

The route walks through the school corridor and jumps onto a raised area; movement is consistent at 30, 60 and 144 rendering frames per second. Fidelity to Alice's original movement, collision on moving entities and the full campaign remain unverified.

## Completed character foundation

Version 0.4 displays Alice's original textured model and six locomotion clips with skeletal interpolation and state blending. A collision-aware third-person camera and V view toggle are available. All 222 Alice clips pass decoding and skinning validation, but most are not connected to gameplay. Version 0.6 added weapon attachments and selected action clips. Root-motion fidelity, general animation events, facial changes and cinematics remain. See [CHARACTER.md](CHARACTER.md).

## Completed audio foundation

Version 0.3 reads the normal music cue for all 36 levels and loads active looping/random ambient emitters. Independent WAV/MP3 decoding, stereo positioning, distance attenuation, movement effects, saved volume controls, mute and pause are implemented. Actual output was verified on the user's normal Windows output device, and the controls in Anode. See [AUDIO.md](AUDIO.md). Sound events still need to be connected to future gameplay, animation and script systems.

## Completed HUD and inventory foundation

Version 0.5 connects original-art Sanity/Will meters and ten toy slots to player state. Ownership, selection, static resource/weapon pickups, per-run collection tracking, provisional fall damage and Will recovery are implemented. I pauses gameplay for inventory selection. Combat consumption, exact difficulty rules, power-up/watch timers, original TAN HUD/pickup animations remain; persistent saves were added in version 0.19. See [HUD.md](HUD.md).

Version 0.10 adds earlier-campaign weapon profiles for all 36 maps and three return visits. Fresh starts, normal-exit preservation, cumulative Dice counts and optional late weapons are handled separately. This does not simulate script completion. See [LOADOUTS.md](LOADOUTS.md).

## Subsequent milestones

Version 0.6 completed an initial **weapon presentation** layer: all ten held models, armed movement/equipping, and primary/alternate Blade, Cards and Mallet visuals with static-world effects and sounds. Those three toys now damage supported enemies; v0.21 adds Demon Dice. The other six toys' actions and exact particle/return/charge behaviour remain pending. See [WEAPONS.md](WEAPONS.md).

Version 0.11 adds camera-relative first-person equipment and action presentation, using the same release clock with visible swing ribbons and world-colliding projectiles. Separate hand geometry, original-view fidelity and the existing combat gaps remain.

Version 0.7 adds selected material stages/animation, approximate authored fog, steam emitters, rotating-door rendering/collision and E interaction, basic teleport/damage/exit volumes and named map entries. A school-door route passes at 30/60/144 Hz. A native map transition has been verified after reaching the exit with free flight; this does not establish intended progression. See [WORLD.md](WORLD.md).

Version 0.8 makes the garden underwater start usable: swimming, diving/surfacing, original swim clips, low-bank climbing and slope exits, water sounds and camera tint, and provisional slime/lava damage. It also fixes transparent effects bypassing wall depth checks, with a native GPU regression check. See [SWIMMING.md](SWIMMING.md).

1. **Complete the school experience and continue the campaign.** The first-visit route, moving supports and book exit now work. Add the missing cinematic/story presentation, full enemy activations, optional secret and return-visit observatory progression; then continue beyond the second school's completed main quest.
2. **Gameplay interface and entities.** Shared dispatch, conditions and saved entity state are implemented in v0.20. Bind additional reviewed level puzzles and component types through these interfaces; remaining movers and quest choreography still use dedicated controllers. Original script execution, arbitrary spawn flags, general spawning/removal and cinematic timelines are not implemented. Use the local Ghidra project for specific unresolved behaviours; define independent Rust interfaces and behavioural tests.
3. **Model and animation completion.** Version 0.12 adds placed NPCs, original idle/talk clips, TAN props, proximity turning and E greetings. Add skeletal attachments (the Mad Hatter's cane), scripted activation/movement, combat clips and animation events; improve root-motion fidelity. See [NPCS.md](NPCS.md).
4. **Interaction and combat.** Version 0.13 verifies a bounded club-guard encounter with pursuit, timed melee, health, pain/death, Blade/Cards/Mallet damage and Will costs. Extend this to the other enemies, enemy projectiles, body separation, pathfinding, loot and additional weapon abilities. See [COMBAT.md](COMBAT.md).
5. **Audio completion and cinematics.** Extend the implemented WAV/MP3 backend with full story coverage, cinematic timing, script/animation events, combat effects, music mood transitions, occlusion/reverb and cutscene controls. No dependency on redistributing the proprietary Miles sound binaries.
6. **Persistence and campaign.** Versioned Rust saves and visited-map persistence are implemented. Remaining work: further scripted transitions, difficulty, original checkpoints, remaining water/current/drowning rules and a complete beginning-to-end playthrough.
7. **Rendering fidelity.** Complete the remaining material stages, transparency sorting, fog conventions, skyboxes, moving brush models, particle families, visibility culling and reference-image comparisons.
8. **Accessibility and portability.** Rebindable controls, controller navigation, scalable high-contrast UI, subtitle controls, camera/motion settings, pause behaviour, and an accessibility API approach for menus. Verify Linux/macOS separately before claiming support.

## Release work

Establish input-copy entitlement and have the UK/international release approach reviewed. Maintain an evidence/provenance record without publishing protected dumps. Audit source, dependency licences, executable notices and branding. Retain a packaging allowlist and synthetic fixtures so automated builds need no proprietary data. Consider a separately isolated clean-room team if advised; this first developer has already inspected the original binary.

## Deliberate current limits

The source package contains no automatic game downloader, original binaries or replacement asset licence. The game-data path is supplied by the user. Public hosting, a public repository and a release announcement have not been created. Those decisions remain separate from producing and testing the local prototype.
