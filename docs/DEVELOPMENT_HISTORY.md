# Development history

Historical notes from development. Their status statements describe their own
milestones; see the current README and campaign audit for present coverage.

# Looking Glass

**Demon Dice now restores missing attacks and enemy retaliation.** The Watch follows the reviewed exceptions across all ten toys, including moving thrown knives and Staff spirals, frozen demons and Ice walls, and saved expiry/recharge. See [rules, verification and remaining fidelity boundaries](DICE_WATCH.md).

**Rage and Tea regain their original transformation clips and alternate skins.** Their existing effects, timers and saved state are preserved. Facial presentation now uses the original default mouth range and explicit dialogue head-watch targets where supported. First person shows only the weapon and its effects; hands, arms and transformation body parts stay out of the camera. See [presentation coverage](POWER_PRESENTATION.md) and [facial fidelity](FACIAL.md).

**Weapon input now supports held fire and queued switching.** Attacks spend Will once at their original event, the Watch activates at frame 22, and thrown-weapon recovery persists across selection changes. Cards cost 3/20 Will and the Mallet ball costs 8. Old prepaid save actions remain compatible. See [shared action rules and verification](WEAPON_ACTIONS.md).

Actor and cutscene updates have a [one-command visibility regression suite](VISIBILITY_TESTS.md), including automated Village scene replays, skip/load handoffs and GPU pixel checks.

The [ten-toy fidelity audit](TOY_AUDIT.md) compares original firing modes, controls, Will costs, animation timing, damage and special effects with the current implementation, and records remaining work and unresolved engine behaviour.

**The Load/Save camera regains its original appearance.** Saved photographs sit inside the ornate archive frames again, with animated sepia, wooden shutters, brass controls and projector sounds. The six slots, previews, backup recovery and overwrite/load confirmations keep working. Relaunch **Launch.cmd**; existing saves remain compatible. See [menus](MENUS.md).

**The village opening and Rabbit/gnome scenes are restored.** Fresh village visits now include the fall, landing, Cheshire introduction, Blade pickup and Rabbit shrinking through the door. Gnome conversations have cameras, gestures and movement. Pandemonium gains refined cart acting, track collapses, disappearance particles and sound timing. Scenes support pause, saving and safe skipping. See [cinematic changes and remaining fidelity limits](VILLAGE_CINEMATICS.md).

**Missing map decorations are restored.** The original maps now gain 1,184 omitted prop placements across 29 maps, including vegetation, lamps, classroom furniture, clocks, banners and statues. Original positions, sizes, orientations and supported room attachments are preserved; compiled duplicates and existing quest objects are excluded. Relaunch **Launch.cmd**; existing saves remain compatible. See [decoration coverage and remaining scripted props](DECORATIONS.md).

**More original animations are connected.** Alice regains weapon-specific idle routines with animated hand props, conversation gestures, damage reactions and correct unarmed movement. NPC idle/talk variations, Gnome talk transitions, the Hatter's cane, swaying school book stacks and moving sky-clock hands are also restored. Relaunch **Launch.cmd**; existing saves remain compatible. See [animation coverage and remaining contexts](ANIMATIONS.md).

**Version 0.32 adds rebindable controls, Windows Xbox/XInput controllers and four manual save slots.** Open **Escape / Start → Settings → Controls** to rebind keyboard, mouse and controller actions; Apply keeps the changes. In **Load/Save**, select an empty slot and choose Save. Existing saves and old key settings remain compatible. See [controls and controller layout](CONTROLS.md).

**Fortress sky and Pool of Tears transports:** the Fortress exterior now uses the level's animated vortex, and its help text distinguishes the first route from the return jump into the Skool. Pool of Tears gains four ride leaves, replacement leaves, wobbling lily pads and the Mock Turtle conversation. Its full route and remaining enemy/cinematic work are still unfinished. See [Fortress](FORTRESS.md) and [Pool transport progress](POOL.md).

**Facial animation is restored for the implemented dialogue.** Alice and speaking characters now use the original lip-sync tracks, including Cheshire hints and the supported cinematic cast. Alice also uses her original blinking face texture. Relaunch **Launch.cmd** or **Launch-Village.cmd**; existing saves remain compatible. See [facial animation and verification](FACIAL.md).

**Version 0.31.9 expands gameplay audio.** Missing combat and quest sounds now resolve, Alice's running/swimming/climbing/death cues follow animation frames, enemy and projectile sounds have world positions, and the village/school/minecart movers regain their sound cues. Music crossfades between levels; walls, water and enclosed rooms affect the mix. Windows playback and mute/pause/mixing checks pass. Relaunch **Launch.cmd**. See [audio coverage, verification and fidelity limits](AUDIO.md).

**Version 0.31.8 fixes the village machinery and steam.** The sawmill slats and beam tilt together, wall vents bend and spin about their proper hinges, and steam follows their moving outlets during the original burst intervals. Mushroom-roof and puff-ball steam timing is also connected. The normal village-to-Pandemonium route still passes without cheats. Relaunch **Launch.cmd** or **Launch-Village.cmd**. See [village machinery and verification](VILLAGE.md).

**Version 0.31.7 completes the requested gameplay console commands.** Press **~ / backtick** for `god`, `noclip`, `notarget`, `wuss`, `give all`, `health`, `cg_cameradist`, `give <item.tik>` and `map`. God mode now permits weapon attacks without spending Will; Wuss grants toys without refilling resources. `itemlist` shows the 29 supported original inventory filenames. Cheat state and over-100 health survive save/load; fast console typing keeps letters in order. Relaunch **Launch.cmd**. See [console commands and examples](CONSOLE.md).

**Beyond the Wall now connects the two Fortress of Doors visits.** Launch **Launch-Beyond.cmd**, or enter normally from Fortress of Doors. Its musical levers, flipping corridor, rolling bridge, rising staircase and shuffled return doors are restored. Saves retain puzzle/machinery progress. See [controls, verification and remaining fidelity work](BEYOND.md).

**Fortress of Doors now has two playable visits.** Launch **Launch-Fortress.cmd** for the lower route through the splitting room to Beyond the Wall. Choose **Fortress of Doors - return** in Tab for the upper route into school. The arrival skips to the original cinematic endpoint; full arrival cinematics remain unfinished. Beyond the Wall now has a verified main route. See [fortress controls, sky behaviour and verification](FORTRESS.md).

**Version 0.31.6 corrects Pandemonium's wall machinery.** Arms and legs now remain joined at their authored pivots; pistons slide vertically, the shared wheel turns on its axle, and the two cams use their original staggered stroke sequences. Steam emission follows the power strokes. Existing saves retain the mechanism phase. See [Pandemonium verification](PANDEMONIUM.md).

**Local setup and release preparation:** [installation instructions](INSTALL.md), [GitHub/Windows preview plan](RELEASE_PLAN.md), and [Steam guide outline](STEAM_GUIDE_PLAN.md). Double-click **Setup.cmd** to select the original game download or an extracted folder. No public release is available yet.

**Version 0.31.5 restores Pandemonium's green slime surface.** Both animated layers now use the blending declared in the original material, making the harmful pools visible while retaining their view-dependent transparency. Relaunch **Launch.cmd**; saves and damage rules are unchanged.

**Version 0.31.4 fixes the minecart's wheels and floating flames.** Wheel assemblies remain bound to the cart through the lift, rail ride and final tilt, spinning about their actual axles. The original hanging lantern bodies now surround the existing map flames. Relaunch **Launch.cmd**; existing saves continue to work.

**Version 0.31.3 smooths the camera near walls and along the minecart route.** The follow camera returns gradually after obstructions, rail turns are continuous, and the cart camera anticipates tunnel clearance. The village now starts unarmed; collect the hallway Vorpal Blade to equip it. Existing saves keep earned weapons and can consume an uncollected village Blade. Relaunch **Launch.cmd**. See [camera behaviour](CAMERA.md).

**Version 0.31.2 adds mouse control to the Tab chapter menu.** Scroll to browse, click to select, then double-click or click **Begin** to start that visit. **Return** closes the chooser, and clicking the difficulty label changes it. Keyboard navigation and the original-art styling remain available. Relaunch **Launch.cmd**.

**Version 0.31.1 fixes shared character placement and facing.** Walking actors settle onto solid support, while flyers and swimmers retain their height. Speaking characters can turn toward Alice during dialogue, and the second-school Gnome faces Alice rather than the camera. Village and Centipede-map examples were checked in-game. Existing saves remain compatible; relaunch **Launch.cmd**. See [placement coverage and remaining embedded/scripted cases](ACTOR_PLACEMENT.md).

**Version 0.31 brings the interface closer to the original.** Menus and gameplay overlays now share the original fonts, parchment and dialogue frames, controls, cursor and pause watch. Sanity/Will meters and weapon cards use the original HUD meshes and texture coordinates. Prototype panels and permanent instruction strips are removed; **H**, **I**, **Tab** and **Esc** retain their functions. Modern display settings and saves remain compatible. Relaunch **Launch.cmd**. See [interface coverage and remaining fidelity differences](UI_FIDELITY.md).

**Version 0.30.2 adds modern display choices.** Open **Esc â†’ Settings â†’ Video** for 1920Ã—1080, 2560Ã—1440 (QHD/2K) and 3840Ã—2160 (4K), plus an explicit Windowed/Fullscreen selector. The controls use the original parchment, lettering and buttons. Fullscreen fits the desktop; Windowed uses the selected pixel size where the display allows it. Choose **Apply** to keep your settings.

**Version 0.30.1 puts the Tab level selector in story order.** All 39 visits now have level names and map IDs, including separate fortress, school and forest returns. Up/Down selects a visit, Home/End jumps to the first/last, and Enter starts it at the appropriate entrance.

**Version 0.30 restores the original Escape menus.** Press **Esc** for the main menu, then **Settings** for video, audio, controls and game options. Original artwork, hover highlights, bitmap lettering and Alice's animated mirror are read from your archives. Save/load, new-game difficulty and quit actions are connected; Escape returns through the menu and resumes play. Preferences persist separately from campaign saves. See [menu controls and coverage](MENUS.md).

**Version 0.29.1 fixes rendering-resource exhaustion during level changes and loading.** Character/prop shaders are now shared, and choosing a level releases the previous map's art. Full-resource swap checks cover all 36 maps and the three selectable return visits twice. Relaunch **Launch.cmd** to use the fix; saves remain compatible.

**Version 0.29 improves rendering:** original portal skies and sky meshes, reflective water, cutout and layered materials, garden waterfalls, oriented fire/steam/bubble/spray emitters, and corrected fog blending. Transparent world surfaces and environmental particles now share a sorted pass. Relaunch the usual launcher; existing saves still load. See [rendering coverage, checks and remaining limits](RENDERING.md).

**Version 0.28 adds Rage Box, Grasshopper Tea, shared Looking Glass rules, the Pocket Watch, enemy essence drops and four difficulty settings.** Select an owned Watch with **0** and use either mouse button: 1 Will, 20 seconds stopped time, six-minute recharge. Power/drop timers and difficulty survive saving and restarting. **Tab â†’ D** selects difficulty for a fresh level start; Continue retains the saved choice. See [item rules, durations and verification](ITEMS.md).

**Version 0.27 restores shared environmental movement:** authored currents, launch volumes and updrafts; finite air and drowning; campaign rope climbing/swinging; and knockback from supported enemy attacks. Rope controls are **E** to grab/release, **Space/Ctrl** to climb and **WASD** to swing. The breathing upgrade, remaining air, grip and impulses persist in saves. See [movement controls, original evidence and verification](TRAVERSAL.md). Map-specific unfinished sequences still need their own restoration.

**Version 0.26 adds the Duchess boss in `potears3`:** expanding/sealed arena, pursuit and phase dodges, pepper/pig/melee attacks, timed defeat, Mock Turtle shell reward and gated temple transition. Saves and held-Enter cinematic skipping are supported. Use **Launch-Duchess.cmd** (N for a fresh visit). The normal entrance-to-exit fight passes without cheats. Jackbomb attacks and the surrounding campaign journey remain unfinished. See [encounter details and verification](DUCHESS.md).

A native Rust starting point for preserving the **2011 re-release of American McGee's Alice**. It opens the user's existing PK3 archives and renders original levels. It does not load the original executable or gameplay DLLs.

**Version 0.25 restores Pandemonium's four main cinematics:** the Elder Gnome warning, minecart ride, house return and airship departure. Original camera paths, timed acting and world changes now accompany the dialogue. **Hold Enter to skip a scene**; **E** still advances one line. Saving works during and after a skip, and older saves upgrade. Use **Launch-Pandemonium.cmd** (N for a fresh visit). See [cinematic controls, verification and fidelity limits](CINEMATICS.md). The playable Fortress 1 arrival is now covered by the fortress update above.

**Version 0.24 restores the first school return** (`skool1_start2`): changed cast/platforms, open library passage, Lucky Star lift, observatory globe, Drink Me animation and shrinking exit to the Pool of Tears. Enter normally from school two, or use **Launch-School-Return.cmd** and choose **N** for a fresh visit. **H** shows the current objective; **E** opens doors. Earned inventory carries through normal transitions, and mid-quest saves resume after restart. See [the return route and verification](SCHOOL-RETURN.md).

Version 0.23 restores Pandemonium's main route, the next unfinished map after the village: usable ropes, minecart lift and ride, Cards, guard activations, the Gnome's key, upper door, return portal and airship departure to Fortress 1. Original dialogue and mounted animations are connected, and progress survives saves. Start with **Launch-Pandemonium.cmd**, or enter from the village. Choose **N** for a fresh visit if Continue is offered. **E** grabs/releases a rope; **Space/Ctrl** climb; **WASD** swings. The fortress updates above now provide playable routes through both maps. See [Pandemonium controls, verification and remaining limits](PANDEMONIUM.md).

**Version 0.22 implements the Pool of Tears' Ladybug bombers.** All twelve actors now have collision-aware flight, acorn attacks, damage reactions, falling deaths and original animations/sounds. Six reviewed map triggers start patrols and paired ambushes; saving retains bombs, enemy state and delayed arrivals. **Launch-Pool-of-Tears.cmd** opens that map; choose **N** for a new visit if Continue is offered. The map's remaining enemies and full traversal are still unfinished. See [Ladybug behavior and scope](LADYBUGS.md).

**Version 0.21 implements Demon Dice, the next toy in campaign order.** Select **7** when owned. Both mouse buttons use the original summoning attack for **40 Will**; the original Dice definition has no separate alternate attack. Rolling dice summon a lesser demon, normal demon or king, with original models/animations, rift and attack effects, sounds, melee/ranged enemy damage and persistent summon state. Collected copies improve the roll. Try **Launch-Combat-Preview.cmd** for a staged encounter, or use the normal school-two pickup and return loadout. See [Demon Dice controls and scope](DICE.md).

**Version 0.20 adds shared entity and puzzle-event foundations.** Entities now have stable identities, saved activation flags and history. Reusable conditions, counters, one-shot rules and delayed events support future puzzles. The existing school trigger groups, theatre reinforcements/doors, library gates, gym lever and ingredient dialogue use this foundation. F5/F9 and Continue keep the same controls; v0.19 saves migrate automatically when loaded. See [event system and extension guide](EVENTS.md).

**Version 0.19 adds persistent saves.** **F5** quick-saves and **F9** quick-loads. Choose **Continue** when reopening to resume the newest quick/automatic save. Resources, inventory, pickups, enemies, puzzles, dialogue and visited campaign levels are retained. Normal level exits and clean quit also update a separate automatic slot. Loaded games pause; press **P** or click to resume. Saves stay in `private/saves`; previews cannot overwrite them. See [saving and loading](SAVES.md).

**Version 0.18 restores village machinery, first-school enemy activations and the optional Looking Glass secret.** Start **Launch-Village.cmd** for the repaired bridges and moving objects; **Launch.cmd** starts the school. Club/Diamond guards and Boojums activate at the reviewed progression triggers. Shoot the theatre's face picture to open the library secret; its original Looking Glass grants 45 seconds of invisibility. The school route, including the secret, still completes the four-book puzzle and exits normally with combat active. See [village scope](VILLAGE.md) and [school guide](SCHOOL.md).

**The opening campaign maps now each have verified main routes:** Village → Pandemonium → Fortress of Doors → Beyond the Wall → Fortress of Doors return → school. A single uninterrupted campaign playthrough remains a separate validation task, and cinematic/combat fidelity still needs work.

**Version 0.17 restores the developer console and summoned Cheshire hints.** Press **~ / backtick** for the console and type `help` to see supported commands. Press **C** during play to summon Cheshire with his original model, talk animation, recording and subtitles. He uses the latest supported hint region, or one of his original general replies. **H** remains the controls/objective overlay. Relaunch the usual launcher. See [console commands and hint scope](CONSOLE.md).

**Version 0.16 connects the second school's main quest.** Start **Launch-School2.cmd**. The floating books/cabinet and bound gym pendulums now draw, collide and move. Reach the Elder Gnome, fight the three Boojums, rescue him in the laboratory, collect Jumbogrow and grow the greenhouse lollipop, then return for the Drink Me potion and Lucky Star. Both rewards are required for the return portal. **H** shows the current objective; **I** lists quest items; **E** operates doors/the lever and advances dialogue. See [the second school guide and verification scope](SCHOOL2.md).

**Version 0.15.1 fixes sticking at the fallen school bookshelves.** Collision now follows tilted corners correctly, so Alice can stand and jump at the crossing. Tipping shelves keep her upright body clear of their surface, and tiny floor overlaps receive a checked local correction. Relaunch **Launch.cmd** to use the fix. The existing **R** recovery remains available for larger traps. See [collision and recovery notes](RECOVERY.md).

**Version 0.15 restores story dialogue across three maps.** Original voices and subtitles now accompany the village opening and four gnome conversations, the first school's theatre and recipe scenes, and the second school's gym conversation and selected Cat hints. **E** talks to a nearby character or advances the current line. Try **Launch-Village.cmd** for the opening conversation, **Launch-School2.cmd** for the next school area, or continue using **Launch.cmd**. The second school's lever now extends solid, climbable bleachers. Version 0.16 extends the second school through its main quest; village progression and original cinematic staging remain partial. See [story controls and exact scope](STORY.md).

**Version 0.14 connects the first school visit.** The theatre unlocks the library, lifts carry Alice, the four animated flying books form a bridge, and the recipe book unlocks the transition to `skool2`. **H** briefly shows the current objective. The ordinary jump now reaches the school's ledges; **Shift + Space** gives more horizontal distance. Start with **Launch.cmd**. See the [school route guide and remaining limits](SCHOOL.md). Cinematics, complete enemy scripting and the return visit remain unfinished.

**Version 0.13 adds the first playable NPC encounter.** Club card guards notice Alice, approach along clear supported ground, attack, react to damage and play a death animation when defeated. The Blade, Cards and Mallet now damage these guards; Cards and croquet balls spend Will. **Launch-Combat-Preview.cmd** puts one guard at the school entrance and supplies all toys for testing. Use **1**, **2**, **3**, both mouse buttons, and **V** for first person. **Home** resets the encounter; **Enter** retries after death. See [combat scope and controls](COMBAT.md).

Version 0.12 adds placed, animated NPCs. Characters use their original models, idle clips and attached props, and turn toward nearby visible Alice. Approach a friendly character and press **E** when the greeting prompt appears. **Launch-NPC-Preview.cmd** remains a peaceful showcase of the Cheshire Cat, a gnome, a card guard and a schoolchild. Version 0.15 adds selected voiced conversations; other greetings remain animation reactions and scripted paths remain pending. See [NPC scope and controls](NPCS.md).

Version 0.11 adds visible first-person weapons and attack effects. Press **V** to switch views. All ten equipped toys have a camera-relative model; Blade, Cards and Mallet retain their primary/alternate visual actions, including swings, throws and trails. These view poses are independently authored and do not include a separate hand mesh. See [weapon visuals](WEAPONS.md).

Version 0.10 gives every level its earlier campaign weapons. Starting or selecting a map assumes that previous areas were fully explored, including optional weapons, while toys first found in the current visit remain pickups. The **Tab** chooser lists first and return visits separately in story order for the fortress, school and forest. Demon Dice counts and the later Eye Staff/Blunderbuss are included. See [loadout rules and progression](LOADOUTS.md).

Version 0.9 fixes the school steam-floor gap and improves swimming and recovery. The eight authored stage platforms are visible and solid, using the appropriate first/return-visit set. Alice floats at shoulder depth, blends between swimming poses at a consistent waterline, and wades when supported in shallow water. **R** returns to previous footing, **Enter** retries after death, and **Home** restores the current entrance. Authored fatal-fall volumes kill instead of leaving Alice stranded. See [recovery and platform notes](RECOVERY.md).

The earlier fire/water animation, steam and fog remain available. Press **E** near a supported rotating door to open or close it. Basic teleport, damage and level-exit volumes work, including named entrances. The first school visit now has a tested traversal and puzzle route. Other maps still need their progression logic; full story and combat fidelity remain incomplete. See [world effects](WORLD.md) and the [school milestone](SCHOOL.md).

The earlier equipment, weapon visuals, Sanity/Will, pickups, music and ambience remain available. Combat supports club guards, selected Diamond guards, school Boojums and Pool of Tears Ladybugs. Movement, fog, door and damage behaviour remain provisional.

## Try it

Double-click **Launch.cmd** in this folder. A release executable has been built locally. The launcher starts `skool1`, which provides a useful view without an opening cinematic. **Launch-Village.cmd** starts a fresh `gvillage` visit with the falling introduction and its opening dialogue.

The default data location is `alice_202106/Alice1/bin/base`, relative to this folder. The original files are opened read-only. No original binary needs to run.

**Launch-Swimming-Preview.cmd** opens the underwater start of `garden1`. Hold Space to surface; use Ctrl to dive and look/WASD to swim. At a low bank, keep moving toward it and hold Space to climb out. Space also climbs supported low ledges on land. E remains the interaction key.

**Launch-Weapon-Preview.cmd** starts with all toys unlocked and full resources. Use **1** for the Blade, **2** for Cards, **3** for the Mallet, **4** for Jackbomb, **5** for Ice Wand, **6** for Jacks, **7** for Demon Dice and **0** for the Watch; try both mouse buttons. Use **8** for Eye Staff (hold/release either mode) and **9** for Blunderbuss (one cannon attack on either button). This preview does not change the normal starting inventory. **V** switches between Alice's third-person equipment and the first-person weapon view. See [weapon visuals](WEAPONS.md).

A local 18-second door/effects demo is saved at `private/environment-interactions-demo.mp4`. It shows the native Rust window opening and walking through the school doors, then pausing. Earlier movement and weapon recordings remain in `private/`. These recordings contain original game artwork, are excluded from source packaging, and have not been published.

| Control | Action |
| --- | --- |
| W / A / S / D | Move forward / left / back / right |
| Space | Jump / climb a low ledge; rise / climb out while swimming; climb up a rope |
| Left Ctrl | Dive while swimming / climb down a rope |
| E while walking | Grab/release a rope, request a low-ledge climb, advance dialogue, talk to a nearby friendly NPC, use the gym lever, or open / close a supported door; follow the prompt |
| Q / E | Descend / ascend in free flight |
| Mouse (no button needed) or arrow keys | Look around |
| V | Switch third-person / first-person view |
| I | Open / close the inventory and pause gameplay |
| 1â€“9, 0 | Select an owned toy; inventory slots show the bindings |
| Mouse wheel or [ / ] | Cycle through owned toys |
| Left click in the inventory | Select an owned toy |
| Left / right click during play | Primary / alternate attack for equipped toys; Dice, Blunderbuss and Watch share one mode on both buttons |
| Left Shift | Run / swim / fly faster |
| F4 | Switch between walking and free flight |
| P | Pause and release the mouse / resume |
| Left click while paused | Resume and capture the mouse |
| R | Recover footing before a jump/fall, or the entrance if none is usable; restore resources |
| Enter after death | Retry using the same recovery route |
| Home | Return to the current entrance with full Sanity and Will |
| Tab, then wheel / click or Up / Down | Browse 39 story-ordered visits; double-click, Begin or Enter starts the selected visit |
| Home / End in the level chooser | Jump to the first / last story visit |
| H (or F1) | Brief controls and current school objective; press again to dismiss |
| C | Summon Cheshire for a hint; E advances his line |
| ~ / backtick | Open / close the developer console; type help for commands |
| F2 | Toggle full lighting for inspection |
| F3 | Sound settings: Up / Down select, Left / Right adjust |
| F5 / F9 | Quick-save / quick-load |
| M | Mute / restore all sound |
| F12 | Save a screenshot under `private/screenshots` |
| Escape | Open the original main menu; go back through its pages; resume from the main menu |

Music and effects/voice volumes are saved in `private/audio-settings.txt`. Defaults are 50% music and 70% effects. The level chooser pauses audio; the sound-settings panel pauses movement while allowing volume adjustments to be heard. `--no-audio` starts silently. If no output device is available, the viewer continues and reports that in F3's panel. Restart after changing your output device.

Version 0.3.1 starts with a clear view and a small **H help** hint. Instructions, the title banner and diagnostic footer stay hidden during normal play. Press H for an eight-second help overlay, or press it again to dismiss early. Opening a menu clears the temporary help; pausing still shows a small resume reminder.

Version 0.3.2 adds continuous mouse-look: move the mouse without holding a button. The cursor is hidden and captured during play, and released for pause or menus. On Windows, switching to another application automatically pauses and releases the cursor; click the game or press P to resume. Sensitivity is independent of frame rate and window dimensions, and returning from a menu does not apply the cursor movement made while it was open.

Version 0.4 adds a following camera that shortens its distance near walls and frames Alice from above in tight doorways. Her body turns toward movement, animations blend between states, and pausing freezes her pose. Free flight stays first-person. See [character implementation and validation](CHARACTER.md) for supported animations and remaining fidelity limits.

You start with 100 Sanity and 100 Will and the weapons appropriate to that visit; the default school first visit has the Vorpal Blade and Cards. Walk into supported pickups to collect them: red restores Sanity, blue restores Will, purple restores both, and weapon icons unlock toys. Normal exits retain inventory, collected pickups and resources, filling in missing earlier weapons. The level chooser starts a fresh visit with its baseline inventory and full resources; **Continue** restores saved progress after restarting. R, Enter retries and Home restore resources without removing collected toys or rewinding world state. I and P pause regeneration, movement and weapon effects. Blade, Cards, Mallet and Demon Dice damage supported guards, Boojums and Ladybugs; Cards, croquet balls and Dice spend Will. Other weapon abilities and the watch's time-freeze remain unfinished. See [HUD and inventory notes](HUD.md).

## Build and inspect

Tested on Windows x64 with Rust 1.86 and an OpenGL context supplied through Anode. Other platforms are not yet tested. Dependencies are locked; `fontdue` is pinned for compatibility with the installed compiler.

```powershell
cargo build --release --locked
cargo test --locked
cargo run --release --locked -- --school-check
cargo run --release --locked -- --school-route-check
cargo run --release --locked -- --school2-check
cargo run --release --locked -- --school2-route-check
cargo run --release --locked -- --campaign-legs-check
cargo run --release --locked -- --route-difficulty-check
cargo run --release --locked -- --campaign-route-check --campaign-strict
cargo run --release --locked -- --campaign-graph-check
cargo run --release --locked -- --story-check
cargo clippy --locked --all-targets -- -D warnings
cargo run --release --locked -- --list
cargo run --release --locked -- --inspect --map skool1
cargo run --release --locked -- --validate-all
cargo run --release --locked -- --physics-check --map skool1
cargo run --release --locked -- --audio-check
cargo run --release --locked -- --character-check
cargo run --release --locked -- --hud-check
cargo run --release --locked -- --loadout-check
cargo run --release --locked -- --hud-preview
cargo run --release --locked -- --weapon-check
cargo run --release --locked -- --dice-check
cargo run --release --locked -- --world-check
cargo run --release --locked -- --swim-check
cargo run --release --locked -- --traversal-check
cargo run --release --locked -- --render-check
cargo run --release --locked -- --weapon-preview
cargo run --release --locked -- --audio-test --map skool1
cargo run --release --locked -- --fly --map gvillage
cargo run --release --locked -- --data "D:\My Alice\bin\base" --map garden1
```

For a targeted test start, `--start-at "x y z"` accepts feet coordinates. Home still returns to the authored entrance. For example, `--map skool1 --start-at "-3580 2432 -590"` tests the repaired steam floor.

Rendering smoke test (requires a desktop/OpenGL context):

```powershell
.\target\release\looking-glass.exe --map skool1 --frames 90 --capture private/screenshots/skool1.png
```

## Implemented

- Read-only, case-insensitive PK3 mounting with later alphabetically sorted packs overriding earlier packs.
- Checked FAKK v42 BSP reader: lump boundaries, index bounds, finite coordinates, material references and patch dimensions.
- Polygon and triangle surfaces, quadratic Bezier patch tessellation, world-model selection.
- Native FTX texture decoding, plus TGA/JPEG/PNG, basic material image lookup, and stored lightmaps.
- Mouse and keyboard navigation, level chooser, fullbright inspection and screenshots.
- Headless archive inspection and corpus validation, plus reproducible local Ghidra analysis tools.
- Continuous collision sweeps against solid/player-clip brushes, accelerated by a bounding-volume hierarchy.
- Two-sided collision facets for tessellated curved patches, including box bevel planes.
- 120 Hz movement: gravity, ground contact, wall sliding, slopes, 18-unit steps and edge-triggered jumping. Menu and pause stop the simulation.
- A deterministic real-map walking and jump route, plus synthetic physics regression tests.
- Original per-level MP3 music with normal-cue selection, looping, volume and fade-in settings.
- WAV ambience from sound-manager files and map entities, with stereo positioning, distance attenuation and timed random emitters.
- Distance-driven alternating footsteps, jump and landing effects from the supplied Alice sounds.
- Separate music/effects volume, global mute, pause/resume and graceful operation without an audio device.
- Checked SKB/SKA skeletal readers, weighted skinning, interpolated and blended locomotion, Alice's normal textures and a collision-aware third-person camera.
- Original-art Sanity/Will meters, selected-toy indicator and keyboard/mouse inventory with ten slots, ownership and Demon Dice counts.
- Per-run player resources, provisional fall damage and Will regeneration, and 219 supported static pickups across the 36 supplied levels, with single-use tracking and wall checks.
- Campaign weapon profiles for all 36 levels and three return visits, cumulative Demon Dice and earlier optional weapons; fresh menu starts and retained inventory through normal exits.
- Checked TAN v2 model reader, original held weapon models attached to Alice's animated tags, weapon-switch and armed locomotion clips.
- Blade swings/throw, single/fanned card throws, mallet swings/ball toss, trails, bounded visual projectiles, static-world impact effects and timed original sounds. Actions blend over the upper body while moving or jumping.
- Checked inline brush models and fog records; selected material animation, blending, texture transforms and detail layers, with approximate distance/volume fog.
- Original steam/hot-steam particle definitions and textures, bounded emission, growth, fade and world clipping.
- E-operated rotating doors with matching animated collision, paired leaves, reach/visibility checks, pause and player-obstruction handling.
- Swept contacts for basic level exits, named entry points, teleports and damage volumes. Unsupported script events show a brief notice and remain unfinished.
- Convex liquid-volume sampling, wading/swimming, neutral buoyancy, ascent/descent, surface control and provisional slime/lava damage. Deep water cushions falls.
- Original frog-stroke/tread-water animations, splash/swim sounds, camera-dependent underwater tint and collision-checked low-ledge climbing with three original climb clips.
- A native GPU regression check for effect occlusion and transparent depth-write behaviour.
- First/return-visit school theatre-floor support, fatal-fall volumes, death poses and R/Enter recovery (also retained in persistent saves).
- A level-controller registry: a new visit plugs in as one `LevelController` through a single uncommented line in `src/levels/mod.rs`, its generic hooks run after every legacy controller, and the headless `--registry-check` proves against a recorded baseline that no existing visit's event program or snapshot changed (`--registry-record <file>` records it, `--registry-baseline <file>` compares). See [docs/LEVEL_REGISTRY.md](LEVEL_REGISTRY.md).
- Save format 12 and the generic upgrade path: the campaign's one save-format bump adds only the optional per-controller `levels` state (older executables refuse such files with a version message, and the reader rejects that state in any file older than 12). A save made before a visit's controller existed is upgraded on load instead of refused: it is restored against the program it was made with, formerly pending script triggers are rearmed, ambushes and rewards never replay, Alice restarts at the entrance if the controller rejects her position or her body is not clear, and the saved cast is regenerated if the controller owns part of it. `levels::state` is the saved-state template for controllers, and a synthetic 39-visit save proves the 8 MiB, 72-visit and 10,000-element limits are enforced. See [docs/SAVES.md](SAVES.md).
- The campaign chain harness (F3a): `campaign::arrive` is the one transition that the viewer and the headless routes share (the destination loads at the resources' difficulty, the visit that was left is recorded in the ledger, the baseline loadout is filled in or, when strict, only authored arrival grants apply, the entrance must be clear or the exit fails instead of dropping Alice into flight, and the opening scene plays only for an uncached visit). `Route::enter` and `Route::depart` carry resources and the ledger from leg to leg, `stop_at_exit`, `Metrics` and `assert_clean` make a route a chain leg, every opening check body is a `drive(r)` collected in `campaign_route::LEGS`, and school two now runs on the shared route and can start from carried resources (its recorded route was re-baselined to 25,814 ticks, still 100 Sanity). The headless `--campaign-legs-check` and `--route-difficulty-check` prove it on the real data; at Easy and Hard the legs check reports the leg bodies that do not pass alone and never gates on them (the route planner retries a goal on a narrow ledge at a wider radius, which is what lets fortress1's first visit pass at both). See [docs/CAMPAIGN.md](CAMPAIGN.md).
- The campaign chain check (F3b): the headless `--campaign-route-check` runs the recorded route legs back to back on one `Stats` and ledger and prints a `PASS` line with metrics for each leg, a reward-provenance report (`--campaign-strict` skips the baseline loadout, so every toy has to come from its authored pickup or grant, and the temple's shell is listed as the `utemple arrival grant`), and `FRONTIER <map$entry>` with the first failing leg or the start thread nothing handles yet. It takes `--campaign-from` and `--campaign-to` (a visit number or `map[$entry]`), `--difficulty`, `--campaign-skip-cinematics` and the diagnosis-only `--campaign-allow-retry`, writes `private/campaign-chain/NN-<map>-<first|return>.json` and `report.json`, and resumes from a checkpoint. `--campaign-expect frontier=<visit>` (or `no-checkpoint`) pins an honest failure, so the check of a frontier the plan says to report and not to fix (the run gated on visit 4, the retry diagnosis, the strict chain that refuses to start without a checkpoint) exits 0 exactly while that failure holds and exits 1 the day the frontier moves; it never turns a completed chain or a diagnosis run into a pass. At every boundary and in the middle of every leg it saves, rebuilds a second route from the saved text and runs both for 600 ticks of identical input, holds the ledger to the save limits, and round-trips the entrance and the pre-exit moment as real saves. Today the strict chain passes gvillage, Pandemonium and fortress1 and stops at Beyond the Wall (a route-driver frontier, reported and not retuned here). The headless `--campaign-graph-check` holds a table of all 39 exits against the data and lists the script-only exits without an adapter, the exits that only a scene fires, and the volumes the data closes that the engine still leaves live. `--campaign-save-chain-write` then `--campaign-save-chain-read` (two processes) write and reload real saves at each boundary and open a window, so they run only in the Anode seat; `tools/test_campaign_chain.ps1` runs it all. See [docs/CAMPAIGN.md](CAMPAIGN.md).

## Fidelity limits

Static world geometry, Alice, equipped toys, supported rotating doors, village machinery and both schools' supported movers are drawn and collide. Other characters have selected placed/speaking/combat behaviour. Sliding doors, general scripted platforms outside the reviewed maps, skyboxes, material deformations and many particle effects are missing. Material support covers a bounded subset; unsupported stages are skipped, and transparency sorting is by batch rather than individual triangle. Lighting and fog density remain approximate. The spawn selector uses entity metadata and explicit reviewed setup, not arbitrary start scripts. Loose-file overrides and locale packs are not mounted. No rendering visibility culling is implemented.

Curved collision follows a fixed tessellation and is not the original engine's curve collision algorithm. Swimming, authored currents, breath/drowning, ropes and supported enemy knockback now work. Crouching, arbitrary moving supports and unfinished map-specific sequences remain pending. Low-ledge climbing is provisional and is not general wall climbing. Basic damage triggers reduce Sanity, but their timing and activation rules are provisional. The player body is 30 Ã— 30 Ã— 56 units with an eye height of 48; these are prototype choices. An obstructed spawn may be lifted at most 128 units; if no clear body fits, the viewer starts in free flight. F4 refuses to start walking inside a wall. Home returns to the current entry point and restores resources. The first landing after a map start/Home is protected. Falling outside level bounds now ends the attempt and shows the retry screen. R/Enter recovery rechecks remembered footing for support, obstruction and hazards, with the current entrance as fallback. This is exploration recovery with history retained in Rust saves; it does not emulate original checkpoints. At zero Sanity, Enter retries from previous footing; Home restores the entrance.

Purple checks identify missing material images. Some maps contain an empty material name; this is reported instead of silently claimed as supported. Scalable text, accessibility APIs, controller support and remapping need additional work: keyboard operation alone is not full accessibility support.

Audio positioning and footstep cadence are provisional. Scripted dialogue, combat sounds, animation-timed events, dynamic music moods, reverb and sound occlusion still need their corresponding game systems. One original ice-wand ambient reference has no matching file and is skipped with a diagnostic. See [audio notes and tests](AUDIO.md).

## Research and publication

Read [the legal baseline](../LEGAL.md), [format/provenance notes](FORMAT.md), [verification record](VALIDATION.md) and [remaining work](ROADMAP.md).

Ghidra projects, reports, decompiled research, screenshots, original game files and downloaded tools are excluded from the prospective source package. The Rust code is newly written; this is **not a certified clean-room process** and is not a legal clearance for release. Nothing has been published.

```powershell
python tools/inventory.py
python tools/setup_ghidra.py
.\tools\analyze.ps1
python tools/package_source.py
```

`setup_ghidra.py` downloads a checksum-verified official portable Ghidra 11.4.2 release into `.tools`, compatible with the installed JDK 21. The user's `C:\DEV\ghidra` source checkout is preserved. `analyze.ps1 -GhidraHome <release-folder>` can use another runnable installation.

MIT licensing applies only to this project's newly authored source and documentation. It conveys no rights to Alice, its data, the original binaries or third-party dependencies. Review redistribution rights and dependency notices before any public release.

Blade/Cards fidelity and verification: [BLADE_CARDS.md](BLADE_CARDS.md).

Mallet/Jackbomb fidelity and verification: [MALLET_JACKBOMB.md](MALLET_JACKBOMB.md).

Eye Staff/Blunderbuss implementation and verification: [STAFF_BLUNDERBUSS.md](STAFF_BLUNDERBUSS.md).

Ice Wand/Jacks implementation and verification: [ICE_JACKS.md](ICE_JACKS.md).
