mod acting;
mod ambient_animation;
mod android;
mod animation_events;
mod ant;
mod assets;
mod audio;
mod beyond;
mod beyond_check;
mod beyond_render;
mod beyond_route;
mod boojum;
mod bsp;
mod burrow;
mod camera;
mod campaign;
mod campaign_chain;
mod campaign_playtest;
mod campaign_route;
mod cards;
mod chapters;
mod character;
mod cheshire;
mod chess;
mod cinematic;
mod clockwork;
mod collision;
mod combat;
mod console;
mod decorations;
mod dice;
mod dismember;
mod duchess;
mod duchess_check;
mod electric;
mod encounters;
mod entity;
mod environment;
mod event;
mod event_check;
mod facial;
mod falling_rock;
mod fire_imp;
mod flight;
mod footing;
mod fortress;
mod fortress_route;
mod frozen;
mod gym;
mod hud;
mod input;
mod interaction;
mod inventory;
mod jabberwock;
mod ladybug;
mod ledge;
mod level;
mod levels;
mod lighting;
mod look;
mod loot;
mod loot_art;
mod magma;
mod materials;
mod menu;
mod model_detail;
mod movement;
mod movie;
mod npc;
mod pandemonium;
mod pandemonium_render;
mod pandemonium_route;
mod particles;
mod plants;
mod pool;
mod pool_check;
mod power_pose;
mod powerups;
mod pickup_effects;
mod preferences;
mod progression_check;
mod progression_render;
mod recovery;
mod registry_check;
mod render;
mod render_fx;
mod rope;
mod route;
mod save;
mod save_check;
mod save_legacy_check;
mod save_preview;
mod school;
mod school2;
mod school2_quest;
mod school2_route;
mod school_return;
mod school_return_route;
mod school_route;
mod skeletal;
mod sky;
mod sky_performance_check;
mod sky_sequence;
mod snark;
mod story;
mod tan;
mod targeting;
mod texture;
mod touch;
mod traversal;
mod ui;
mod viewer;
mod frame_profile;
mod viewmodel;
mod village;
mod village_route;
mod visibility;
mod water;
mod water_art;
mod weapon_rules;
mod weapons;
mod wildlife;

use anyhow::{bail, Context, Result};
use assets::Assets;
use bsp::Bsp;
use std::path::PathBuf;

pub struct Options {
    pub difficulty: powerups::Difficulty,
    pub save_dir: PathBuf,
    pub load: Option<save::Slot>,
    pub new_game: bool,
    pub data: PathBuf,
    pub map: String,
    pub direct_map: bool,
    pub entry: Option<String>,
    pub capture: Option<PathBuf>,
    pub frames: Option<u64>,
    pub fly: bool,
    pub no_audio: bool,
    pub audio_capture: Option<PathBuf>,
    pub hud_preview: bool,
    pub weapon_preview: bool,
    pub npc_preview: bool,
    pub combat_preview: bool,
    pub larva_preview: bool,
    pub start_at: Option<macroquad::prelude::Vec3>,
    pub story_preview: Option<String>,
    pub shelf_preview: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Looking Glass: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn quad_main() {
    android::init_runtime();
    main();
}

fn run() -> Result<()> {
    android::init_runtime();
    let mut o = Options {
        difficulty: Default::default(),
        save_dir: android::default_save_dir(),
        load: None,
        new_game: false,
        data: android::default_data_dir(),
        map: "skool1".into(),
        direct_map: false,
        entry: None,
        capture: None,
        frames: None,
        fly: false,
        no_audio: false,
        audio_capture: None,
        hud_preview: false,
        weapon_preview: false,
        npc_preview: false,
        combat_preview: false,
        larva_preview: false,
        start_at: None,
        story_preview: None,
        shelf_preview: false,
    };
    if android::is_android() {
        macroquad::Window::from_config(viewer::config(), async move {
            android::install_panic_hook();
            let assets = match android::wait_for_data(o.data.clone()).await {
                Ok(assets) => assets,
                Err(e) => {
                    eprintln!("Setup aborted: {e:#}");
                    macroquad::miniquad::window::request_quit();
                    return;
                }
            };
            if let Err(e) = viewer::run(assets, o).await {
                eprintln!("Viewer failed: {e:#}");
            }
            macroquad::miniquad::window::request_quit();
            // On Android `request_quit()` only stops the render loop; it does
            // not always tear the GL surface down, leaving a black window when
            // the player picks "Quit" from the menu. Explicitly finish the
            // Activity so we return to the home/launcher screen.
            crate::android::finish_activity();
        });
        android::install_panic_hook();
        return Ok(());
    }
    let mut args = std::env::args().skip(1);
    let mut mode = "view";
    let mut registry = registry_check::Args::default();
    let mut chain = campaign_chain::Args::default();
    let mut level_check: Option<&'static levels::Check> = None;
    let mut movie_name = String::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--difficulty" => {
                o.difficulty = powerups::Difficulty::parse(
                    &args.next().context("--difficulty requires a name")?,
                )?
            }
            "--save-dir" => {
                o.save_dir = args.next().context("--save-dir requires a folder")?.into()
            }
            "--load" => {
                o.load = Some(save::Slot::parse(
                    &args
                        .next()
                        .context("--load requires quick, auto or slot1-slot4")?,
                )?)
            }
            "--entry" => o.entry = Some(args.next().context("--entry requires a named entrance")?),
            "--new-game" => o.new_game = true,
            "--save-check-write" => mode = "save-write",
            "--save-check-read" => mode = "save-read",
            "--save-legacy-check" => mode = "save-legacy",
            "--registry-check" => mode = "registry",
            "--campaign-legs-check" => mode = "campaign-legs",
            "--campaign-route-check" => mode = "campaign-route",
            "--campaign-playtest" => mode = "campaign-playtest",
            "--campaign-graph-check" => mode = "campaign-graph",
            "--campaign-save-chain-write" => mode = "campaign-save-chain-write",
            "--campaign-save-chain-read" => mode = "campaign-save-chain-read",
            "--campaign-from" => {
                chain.from = Some(
                    args.next()
                        .context("--campaign-from requires a visit number or map[$entrance]")?,
                )
            }
            "--campaign-to" => {
                chain.to = Some(
                    args.next()
                        .context("--campaign-to requires a visit number or map[$entrance]")?,
                )
            }
            "--campaign-skip-cinematics" => chain.skip_cinematics = true,
            "--campaign-strict" => chain.strict = true,
            "--campaign-allow-retry" => chain.allow_retry = true,
            "--campaign-expect" => {
                chain.expect = Some(args.next().context(
                    "--campaign-expect requires frontier=<n|map[$entrance]> or no-checkpoint",
                )?)
            }
            "--campaign-log" => {
                chain.log = Some(args.next().context("--campaign-log requires a file name")?)
            }
            "--route-difficulty-check" => mode = "route-difficulty",
            "--registry-baseline" => {
                registry.baseline = Some(
                    args.next()
                        .context("--registry-baseline requires a hashes file")?
                        .into(),
                )
            }
            "--registry-record" => {
                registry.record = Some(
                    args.next()
                        .context("--registry-record requires an output file")?
                        .into(),
                )
            }
            "--save-preview-check" => mode = "save-preview",
            "--data" => o.data = args.next().context("--data requires a base folder")?.into(),
            "--map" => {
                o.map = args.next().context("--map requires a map name")?;
                o.direct_map = true;
            }
            "--start-at" => {
                o.start_at = Some(
                    interaction::vector(
                        &args
                            .next()
                            .context("--start-at requires quoted feet coordinates x y z")?,
                    )
                    .context("Invalid starting coordinates")?,
                );
            }
            "--capture" => {
                o.capture = Some(args.next().context("--capture requires a PNG path")?.into())
            }
            "--frames" => {
                let n: u64 = args
                    .next()
                    .context("--frames requires a positive count")?
                    .parse()?;
                if n == 0 {
                    bail!("--frames must be positive");
                }
                o.frames = Some(n);
            }
            "--presentation-check" => mode = "presentation",
            "--presentation-render" => mode = "presentation-render",
            "--facial-check" => mode = "facial",
            "--animation-check" => mode = "animations",
            "--animation-runtime-check" => mode = "animation-runtime",
            "--animation-runtime-render" => mode = "animation-runtime-render",
            "--decorations-check" => mode = "decorations",
            "--decorations-render-check" => mode = "decorations-render",
            "--animation-render" => mode = "animation-render",
            "--facial-render" => mode = "facial-render",
            "--list" => mode = "list",
            "--duchess-render" => mode = "duchess-render",
            "--duchess-check" => mode = "duchess",
            "--inspect" => mode = "inspect",
            "--validate-all" => mode = "validate",
            "--physics-check" => mode = "physics",
            "--fly" => o.fly = true,
            "--no-audio" => o.no_audio = true,
            "--audio-check" => mode = "audio",
            "--character-check" => mode = "character",
            "--hud-check" => mode = "hud",
            "--loadout-check" => mode = "loadout",
            "--camera-check" => mode = "camera",
            "--npc-check" => mode = "npc",
            "--npc-placement-check" => mode = "npc-placement",
            "--npc-placement-render-check" => mode = "npc-placement-render",
            "--combat-check" => mode = "combat",
            "--ant-check" => mode = "ant",
            "--magma-render-check" => mode = "magma-render",
            "--magma-save-write" => mode = "magma-save-write",
            "--magma-save-read" => mode = "magma-save-read",
            "--burrow-render-check" => mode = "burrow-render",
            "--burrow-save-write" => mode = "burrow-save-write",
            "--burrow-save-read" => mode = "burrow-save-read",
            "--larva-preview" => {
                o.larva_preview = true;
                o.map = "centipede1".into();
            }
            "--snark-render-check" => mode = "snark-render",
            "--snark-save-write" => mode = "snark-save-write",
            "--snark-save-read" => mode = "snark-save-read",
            "--resident-check" => mode = "resident",
            "--wildlife-check" => mode = "wildlife",
            "--wildlife-save-write" => mode = "wildlife-save-write",
            "--wildlife-save-read" => mode = "wildlife-save-read",
            "--wildlife-render-check" => mode = "wildlife-render",
            "--resident-render-check" => mode = "resident-render",
            "--resident-save-write" => mode = "resident-save-write",
            "--resident-save-read" => mode = "resident-save-read",
            "--clock-check" => mode = "clock",
            "--clock-render-check" => mode = "clock-render",
            "--clock-save-write" => mode = "clock-save-write",
            "--clock-save-read" => mode = "clock-save-read",
            "--imp-check" => mode = "imp",
            "--imp-render-check" => mode = "imp-render",
            "--imp-save-write" => mode = "imp-save-write",
            "--imp-save-read" => mode = "imp-save-read",
            "--chess-check" => mode = "chess",
            "--chess-render-check" => mode = "chess-render",
            "--chess-save-write" => mode = "chess-save-write",
            "--chess-save-read" => mode = "chess-save-read",
            "--ant-render-check" => mode = "ant-render",
            "--ant-save-write" => mode = "ant-save-write",
            "--ant-save-read" => mode = "ant-save-read",
            "--npc-preview" => o.npc_preview = true,
            "--combat-preview" => o.combat_preview = true,
            "--hud-preview" => o.hud_preview = true,
            "--weapon-preview" => o.weapon_preview = true,
            "--weapon-check" => mode = "weapon",
            "--blade-cards-check" => mode = "blade-cards",
            "--blade-cards-render-check" => mode = "blade-cards-render",
            "--heavy-check" => mode = "heavy",
            "--heavy-render-check" => mode = "heavy-render",
            "--ice-jacks-check" => mode = "ice-jacks",
            "--ice-jacks-render-check" => mode = "ice-jacks-render",
            "--mallet-jack-check" => mode = "mallet-jack",
            "--mallet-jack-render-check" => mode = "mallet-jack-render",
            "--weapon-input-check" => mode = "weapon-input",
            "--world-check" => mode = "world",
            "--school-check" => mode = "school",
            "--footing-check" => mode = "footing",
            "--shelf-preview" => o.shelf_preview = true,
            "--story-check" => mode = "story",
            "--cheshire-check" => mode = "cheshire",
            "--cheshire-render-check" => mode = "cheshire-render",
            "--console-check" => mode = "console",
            "--gym-check" => mode = "gym",
            "--story-preview" => {
                o.story_preview = Some(args.next().context("--story-preview needs an event name")?)
            }
            "--school-return-render-check" => mode = "school-return-render",
            "--school-return-chain-check" => mode = "school-return-chain",
            "--school-return-check" => mode = "school-return",
            "--school-route-check" => mode = "school-route",
            "--school-secret-check" => mode = "school-secret",
            "--fortress-render-check" => mode = "fortress-render",
            "--fortress-route-check" => mode = "fortress-route",
            "--fortress-cinematic-check" => mode = "fortress-cinematic",
            "--movie-check" => mode = "movie",
            "--movie" => {
                movie_name = args.next().context("--movie needs opening or ending")?;
                anyhow::ensure!(
                    matches!(movie_name.as_str(), "opening" | "ending"),
                    "--movie needs opening or ending"
                );
                mode = "story-movie";
            }
            "--fortress-cinematic-render-check" => mode = "fortress-cinematic-render",
            "--beyond-route-check" => mode = "beyond-route",
            "--beyond-check" => mode = "beyond",
            "--pool-check" => mode = "pool",
            "--potears1-save-write" => mode = "potears1-save-write",
            "--potears1-save-read" => mode = "potears1-save-read",
            "--potears1-route-check" => mode = "potears1-route",
            "--potears1-skip-route-check" => mode = "potears1-skip-route",
            "--potears1-render-check" => mode = "potears1-render",
            "--pool-render-check" => mode = "pool-render",
            "--beyond-render-check" => mode = "beyond-render",
            "--village-route-check" => mode = "village-route",
            "--village-cinematic-check" => mode = "village-cinematic",
            "--village-cinematic-render-check" => mode = "village-cinematic-render",
            "--village-gesture-render-check" => mode = "village-gesture-render",
            "--visibility-check" => mode = "visibility-check",
            "--village-machinery-check" => mode = "village-machinery",
            "--village-machinery-render-check" => mode = "village-machinery-render",
            "--progression-render-check" => mode = "progression-render",
            "--progression-check" => mode = "progression",
            "--event-check" => mode = "events",
            "--friendly-check" => mode = "friendly",
            "--opening-audit-check" => mode = "opening-audit",
            "--opening-dialogue-check" => mode = "opening-dialogue",
            "--dice-watch-render-check" => mode = "dice-watch-render",
            "--dice-check" => mode = "dice",
            "--ladybug-check" => mode = "ladybug",
            "--pandemonium-route-check" => mode = "pandemonium-route",
            "--pandemonium-skip-route-check" => mode = "pandemonium-skip-route",
            "--cinematic-check" => mode = "cinematic",
            "--pandemonium-check" => mode = "pandemonium",
            "--pandemonium-render-check" => mode = "pandemonium-render",
            "--pandemonium-machinery-render-check" => mode = "pandemonium-machinery-render",
            "--ladybug-render-check" => mode = "ladybug-render",
            "--dice-render-check" => mode = "dice-render",
            "--school2-render-check" => mode = "school2-render",
            "--school2-check" => mode = "school2",
            "--school2-route-check" => mode = "school2-route",
            "--school-render-check" => mode = "school-render",
            "--swim-check" => mode = "swim",
            "--swim-render-check" => mode = "swim-render",
            "--items-check" => mode = "items",
            "--loot-render-check" => mode = "loot-render",
            "--traversal-check" => mode = "traversal",
            "--ledge-check" => mode = "ledge",
            "--ledge-render-check" => mode = "ledge-render",
            "--targeting-render-check" => mode = "targeting-render",
            "--render-check" => mode = "render",
            "--fidelity-check" => mode = "fidelity",
            "--fidelity-render-check" => mode = "fidelity-render",
            "--water-render-check" => mode = "water-render",
            "--render-fx-check" => mode = "render-fx",
            "--billboard-check" => mode = "billboard",
            "--sky-performance-check" => mode = "sky-performance",
            "--fidelity-corpus-check" => mode = "fidelity-corpus",
            "--level-swap-check" => mode = "level-swap",
            "--death-retry-check" => mode = "death-retry",
            "--audio-test" => mode = "audio-test",
            "--audio-regression-test" => mode = "audio-regression-test",
            "--audio-capture" => {
                o.audio_capture = Some(
                    args.next()
                        .context("--audio-capture requires a WAV path")?
                        .into(),
                )
            }
            "--help" | "-h" => {
                println!("Decorations: original map props, vegetation and furnishings use their authored placements. --decorations-check audits accepted/deferred models on every map; --decorations-render-check captures staged before/after views. See docs/DECORATIONS.md for remaining scripted props.");
                println!("--level-swap-check stress-tests full level/actor/effect replacement and character material state on the GPU without using player saves.");
                println!("Rendering: authored portal skies, reflective/animated materials, cutouts, oriented environmental emitters and blend-aware fog are supported. --fidelity-check audits local references; --fidelity-render-check runs staged GPU checks/captures; --fidelity-corpus-check loads and draws all maps. These checks do not use player saves. See docs/RENDERING.md for fidelity limits.");
                println!("Power-ups: Rage Box, Grasshopper Tea and Darkened Looking Glass activate on contact and cannot stack. Their remaining time appears on the HUD. 0 selects an owned Pocket Watch; either mouse button spends 1 Will to stop supported enemies and movers for 20 seconds, with a 360-second recharge. --items-check verifies rules and real school clock behavior. --difficulty easy|normal|hard|nightmare selects a new game's rules; Tab then D changes the next fresh level start. Saves retain difficulty, active effects, Watch recharge and enemy drops (format 12, reads formats 1-11).");
                println!("Pandemonium cinematics: hold Enter to skip a scene; E advances one line; P pauses. F5/F9 retain cinematic clocks and completed skips. --cinematic-check compares watched/skipped progression; --pandemonium-skip-route-check verifies the skipped entrance-to-exit route. Save format 7 also reads formats 1-6.");
                println!("Pandemonium: --map pandemonium restores ropes, the minecart, key/return gates and airship exit. E grabs/releases ropes; Space/Ctrl climb; WASD swings. --pandemonium-check, --pandemonium-route-check and --pandemonium-render-check verify state, normal inputs and staged visuals. Fortress of Doors supports both playable visits; Beyond the Wall supports the connecting main route.");
                println!("Ladybugs: --map potears1 includes flying acorn bombers and scripted patrol/ambush triggers. --ladybug-check validates combat, navigation and activations; --ladybug-render-check captures staged poses. The map's remaining enemy types and full route are unfinished.");
                println!("Shared world events: --event-check validates school activation conditions, entity identities, one-shot/delayed events and saved-state migration. Save format v7 also loads v0.19 through v0.24 saves.");
                println!("Demon Dice: 7 selects owned dice; either mouse button uses the original summon attack for 40 Will. More collected dice can summon stronger demons with melee/ranged attacks. --dice-check validates assets and combat; --dice-render-check captures staged models/effects. Rolls and active summons persist in saves.");
                println!("Saves: F5 quick save; F9 quick load. Launch opens the original main menu; Load/Save resumes a saved game. Automatic saves occur on normal exits and clean quit; loaded games pause. --load quick|auto, --new-game, --save-dir <folder> (default private/saves). Previews cannot save. --save-check-write then --save-check-read verify staged state in separate processes under private/save-check. Original saves are not compatible.");
                println!("Console and hints: ~ / backtick opens the developer console; type help for supported commands. C summons Cheshire using original recordings and hint regions; H remains controls/objectives. --cheshire-check validates all supported hint data and school-two selection. Gameplay cheats: god, noclip, notarget, wuss, give, health and cg_cameradist are available; itemlist lists usable filenames. --console-check validates gameplay command behavior. Wider engine cvars and later scripted hint gates remain partial.");
                println!("Shelf collision: --footing-check tests landings, walking, jumping and tipping riders at 30/60/144 Hz. --shelf-preview stages the skool1 crossing for interactive testing.");
                println!("Fortress of Doors: --map fortress1 plays the airship arrival; --entry fortress1_start2 selects the return and Boojum reveal. Hold Enter/A to skip. --fortress-cinematic-check and --fortress-cinematic-render-check verify scenes; --fortress-route-check verifies both playable routes.");
                println!("Story films: New Game plays the original opening before the village. --movie opening or --movie ending plays a film directly; --movie-check validates all frames and soundtracks. P/Start pauses, hold Enter/A skips.");
                println!("Beyond the Wall: --map fortress2 restores the musical levers, flipping corridor, rolling bridge, raised staircase and shuffled return doors. --beyond-check verifies state/saves; --beyond-route-check verifies normal traversal; --beyond-render-check captures staged visuals. Full cinematic and decorative fidelity remains unfinished.");
                println!("Story: original voices/subtitles in gvillage, skool1 and skool2. E talks or advances a line; P pauses. --story-check validates recordings, subtitles and triggers. --story-preview <event> stages a supported conversation for inspection. --gym-check verifies the skool2 lever and bleachers. Village arrival is now walkable; full cinematic staging remains incomplete.");
                println!("School two: --school2-check validates movers and quest gates; --school2-route-check runs a continuous entrance-to-return route with combat; --school2-render-check saves explicitly staged private captures. H shows objectives; I shows ingredients and rewards.");
                println!("School first visit: theatre/library gate, two lifts, four flying books and recipe exit. H shows the current objective. --school-check tests state/movers; --school-route-check walks the route; --school-render-check saves staged private captures. The return visit restores the observatory lift, globe and potion exit; --map skool1 --entry skool1_start2 starts it fresh. --school-return-check verifies traversal; --school-return-chain-check carries an actual school-two completion into that return; --school-return-render-check stages native visuals.");
                println!("NPCs: placed characters animate and turn toward nearby Alice. E greets friendly characters. --npc-preview stages four characters at the school entrance; --npc-check validates models and placements. Club guards now pursue, attack and take damage; other enemy AI and full story scripting remain pending. --combat-preview stages one guard; --combat-check validates the encounter.");
                println!("Swimming: --swim-check validates liquid volumes and isolated garden swimming; --traversal-check validates currents, updrafts and ropes. --render-check tests occlusion on the GPU.\nWASD swim toward the view; Space rise/climb out; Ctrl dive; Shift swim faster.\nRopes: E grab/release, Space/Ctrl climb, WASD swing. Surface before the air meter runs out.\n");
                println!("World: E opens/closes nearby doors while walking. --world-check validates fog, doors and exits, and walks through the school entrance at 30/60/144 Hz.\nAuthored material effects, swimming and basic triggers are active; scripts and full-map progression remain unfinished.");
                println!("HUD: --hud-check validates weapon/pickup data; --hud-preview shows sample resources and all toys.\nI inventory; 1-0 select an owned toy; wheel or [ / ] cycle; R recovers previous footing while alive; after death Enter/R/Home restores the last save or visit start. --death-retry-check verifies full-world rollback. Home restores the entrance while alive.\nResources, inventory and visited level state carry through normal exits and persistent saves.\nLevel starts include earlier campaign weapons; --loadout-check validates all map/visit profiles.\nTab opens the 39-visit story-order chooser, including separate return visits. Up/Down selects; Home/End jumps to first/last. Enter starts that visit fresh.\nWeapons: --weapon-preview equips all toys; --weapon-check validates models, skins and action clips.\nLeft/right click: Blade, Cards, Mallet and Demon Dice attack supported guards and Boojums. Cards, croquet balls and Dice use Will. Both Dice buttons summon.");
                println!("Audio: --no-audio disables playback; --audio-check validates level sounds.\n--audio-capture <file.wav> captures up to 30s of this app's mix (requires --frames).\nF3 sound settings; M mute; settings are saved locally.\n");
                for line in levels::help() {
                    println!("{line}");
                }
                println!("Registry: --registry-check fingerprints a fresh visit of all 39 campaign entries (snapshot plus event signature) without a window. --registry-record <file> stores the hashes; --registry-baseline <file> fails on any difference. See docs/LEVEL_REGISTRY.md.");
                println!("Campaign chain: --campaign-legs-check runs the eight opening route legs under the chain's assertions and hands each exit to the next visit through the shared transition (--difficulty easy|normal|hard|nightmare); --route-difficulty-check proves that a route's difficulty reaches its world. Both are headless. See docs/CAMPAIGN.md.");
                println!("Campaign chain, whole: --campaign-route-check runs the route legs back to back on one Stats and ledger and prints one PASS line per leg, the reward provenance and FRONTIER <map$entry> (--campaign-from/--campaign-to <n|map[$entry]>, --campaign-strict, --campaign-skip-cinematics, --campaign-allow-retry, --difficulty, and --campaign-expect frontier=<visit>|no-checkpoint, which pins an honest failure so that its check exits 0 only while it holds); it writes private/campaign-chain/NN-<map>-<first|return>.json and report.json and resumes from a checkpoint. --campaign-graph-check checks the exit graph of all 39 visits. Both are headless. --campaign-save-chain-write then --campaign-save-chain-read (two processes) write and reload real saves at each boundary and open a window: they run only in the Anode seat.");
                println!("Looking Glass - Rust movement prototype\n\n  --data <base folder>    Folder containing your PK3 files\n  --map <name>            Level to display; default skool1\n  --list                 List levels without opening a window\n  --inspect              Inspect the selected level\n  --validate-all         Validate every map, collider and FTX image\n  --physics-check        Run a deterministic walking/jump route (skool1)\n  --character-check      Validate Alice mesh and all animation clips\n  --fly                  Start in free flight instead of walking\n  --capture <file.png>    Save a screenshot when the frame limit is reached\n  --frames <count>        Exit after this many frames (for testing)\n\nControls: WASD move; Space jump/climb; Shift run; mouse/arrows look (no button needed).\nV first/third person; F4 walk/fly; Q/E vertical in flight; P pause/release cursor; click to resume; R recover; Enter retry; Home restart.\nTab level chooser in the main menu or game; H brief help; F2 fullbright; F12 screenshot; Esc original main menu; F3 sound settings.\nAlice locomotion and static-world movement. Combat and story are unfinished.");
                return Ok(());
            }
            // Flags a registered visit contributes (`--<id>-check`, `--<id>-route-check`, ...).
            _ => match levels::find_check(&arg) {
                Some(check) => level_check = Some(check),
                None => bail!("Unknown option: {arg}; use --help"),
            },
        }
    }
    if o.capture.is_some() && o.frames.is_none() {
        bail!("--capture requires --frames");
    }
    let mut assets = match Assets::open(&o.data) {
        Ok(assets) => assets,
        Err(_err) if mode == "view" && android::is_android() => {
            macroquad::Window::from_config(viewer::config(), async move {
                match android::wait_for_data(o.data.clone()).await {
                    Ok(assets) => {
                        if let Err(e) = viewer::run(assets, o).await {
                            eprintln!("Viewer failed: {e:#}");
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("Setup aborted: {e:#}");
                        std::process::exit(1);
                    }
                }
            });
            return Ok(());
        }
        Err(err) => return Err(err),
    };
    if mode == "campaign-playtest" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = campaign_playtest::run(&mut assets, chain, o.difficulty).await {
                eprintln!("Full campaign playtest failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if let Some(check) = level_check {
        return match check.run {
            levels::Run::Headless(run) => run(&mut assets),
            levels::Run::Windowed(run) => {
                macroquad::Window::from_config(viewer::config(), async move {
                    if let Err(e) = run(&mut assets).await {
                        eprintln!("{} failed: {e:#}", check.flag);
                        std::process::exit(1);
                    }
                });
                Ok(())
            }
        };
    }
    if mode == "blade-cards" {
        return weapons::check::check(&mut assets);
    }
    if mode == "heavy" {
        return weapons::heavy_check::check(&mut assets);
    }
    if mode == "ice-jacks" {
        return weapons::ice_jacks_check::check(&mut assets);
    }
    if mode == "mallet-jack" {
        return weapons::mallet_jack_check::check(&mut assets);
    }
    if mode == "weapon-input" {
        return weapons::check_actions(&mut assets);
    }
    if mode == "camera" {
        return camera::check(&mut assets);
    }
    if mode == "duchess" {
        return duchess_check::check(&mut assets);
    }
    if mode == "progression" {
        return progression_check::check(&mut assets);
    }
    if mode == "village-cinematic" {
        return village::cinema::check(&mut assets);
    }
    if mode == "fortress-cinematic" {
        return fortress::check::check(&mut assets);
    }
    if mode == "movie" {
        return movie::check(&mut assets);
    }
    if mode == "village-machinery" {
        return village::check(&mut assets);
    }
    if mode == "opening-dialogue" {
        return npc::opening_check::dialogue(&mut assets);
    }
    if mode == "friendly" {
        return interaction::friendly::check::check(&mut assets);
    }
    if mode == "events" {
        return event_check::check(&mut assets);
    }
    if mode == "dice" {
        return dice::check(&mut assets);
    }
    if mode == "save-legacy" {
        return save_legacy_check::check(&mut assets);
    }
    if mode == "registry" {
        return registry_check::check(&mut assets, &registry);
    }
    if mode == "campaign-legs" {
        return campaign_route::check(&mut assets, o.difficulty, true);
    }
    if mode == "route-difficulty" {
        return campaign_route::difficulty_check(&mut assets);
    }
    if mode == "campaign-route" {
        return campaign_chain::check(&mut assets, o.difficulty, &chain);
    }
    if mode == "campaign-graph" {
        return campaign_chain::graph_check(&mut assets);
    }
    if mode == "cinematic" {
        return pandemonium::cinema::check(&mut assets);
    }
    if mode == "items" {
        return powerups::check(&mut assets);
    }
    if mode == "traversal" {
        return traversal::check(&mut assets);
    }
    if mode == "ledge" {
        return ledge::check(&mut assets);
    }
    if mode == "targeting-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = targeting::render_check(&mut assets).await {
                eprintln!("Aim marker render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "ledge-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = ledge::render_check(&mut assets).await {
                eprintln!("Ledge render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "pandemonium" {
        return pandemonium::check(&mut assets);
    }
    if mode == "pandemonium-route" || mode == "pandemonium-skip-route" {
        return pandemonium_route::check(&mut assets, mode == "pandemonium-skip-route");
    }
    if mode == "ladybug" {
        return ladybug::check(&mut assets);
    }
    if mode == "story" {
        return story::check(&mut assets);
    }
    if mode == "cheshire" {
        return cheshire::check::assets(&mut assets);
    }
    if mode == "gym" {
        return gym::check(&mut assets);
    }
    if mode == "school" {
        return school::check(&mut assets);
    }
    if mode == "footing" {
        return school::check_footing(&mut assets);
    }
    if mode == "school-return-chain" {
        return school_return_route::chain(&mut assets);
    }
    if mode == "school-return" {
        return school_return_route::check(&mut assets);
    }
    if mode == "school-route" {
        return school_route::check(&mut assets, false);
    }
    if mode == "beyond" {
        return beyond_check::check(&mut assets);
    }
    if mode == "pool" {
        return pool_check::check(&mut assets);
    }
    if mode == "potears1-route" || mode == "potears1-skip-route" {
        return pool::route::check(&mut assets, mode == "potears1-skip-route", false);
    }
    if matches!(
        mode,
        "potears1-render" | "potears1-save-write" | "potears1-save-read"
    ) {
        macroquad::Window::from_config(viewer::config(), async move {
            let result = if mode == "potears1-render" {
                pool::route::render(&mut assets).await
            } else {
                pool::save_check::run(&mut assets, mode == "potears1-save-write").await
            };
            if let Err(e) = result {
                eprintln!("Pool route failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "beyond-route" {
        return beyond_route::check(&mut assets);
    }
    if mode == "fortress-route" {
        return fortress_route::check(&mut assets);
    }
    if mode == "village-route" {
        return village_route::check(&mut assets);
    }
    if mode == "school-secret" {
        return school_route::check(&mut assets, true);
    }
    if mode == "school2" {
        return school2::check(&mut assets);
    }
    if mode == "school2-route" {
        return school2_route::check(&mut assets);
    }
    if o.audio_capture.is_some() && (o.frames.is_none() || o.no_audio) {
        bail!("--audio-capture requires --frames and enabled audio");
    }
    if mode == "audio" {
        return audio::check(&mut assets);
    }
    if mode == "presentation" {
        power_pose::check(&mut assets)?;
        return facial::check(&mut assets);
    }
    if mode == "facial" {
        return facial::check(&mut assets);
    }
    if mode == "animation-runtime" {
        character::check_movement(&mut assets)?;
        return dismember::check(&mut assets);
    }
    if mode == "animation-runtime-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            let result = async {
                character::render_movement(&mut assets).await?;
                dismember::render_check(&mut assets).await?;
                Ok::<_, anyhow::Error>(())
            }
            .await;
            if let Err(e) = result {
                eprintln!("Animation runtime failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "animations" {
        acting::check(&mut assets)?;
        ambient_animation::check(&mut assets)?;
        return npc::check(&mut assets);
    }
    if mode == "decorations" {
        return decorations::check(&mut assets);
    }
    if mode == "character" {
        return skeletal::check(&mut assets);
    }
    if mode == "hud" {
        return inventory::check(&mut assets);
    }
    if mode == "loadout" {
        return campaign::check(&mut assets);
    }
    if mode == "npc" {
        return npc::check(&mut assets);
    }
    if mode == "npc-placement" {
        return npc::check_placement(&mut assets);
    }
    if mode == "ant" {
        return npc::ant_check::check(&mut assets);
    }
    if matches!(mode, "wildlife-save-write" | "wildlife-save-read") {
        return npc::wildlife_check::fresh(&mut assets, mode == "wildlife-save-read");
    }
    if mode == "wildlife" {
        return npc::wildlife_check::check(&mut assets);
    }
    if mode == "wildlife-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = npc::wildlife_check::native(&mut assets).await {
                eprintln!("Creature check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "resident" {
        return npc::resident_check::check(&mut assets);
    }
    if matches!(
        mode,
        "resident-render"
            | "resident-save-write"
            | "resident-save-read"
            | "magma-render"
            | "magma-save-write"
            | "magma-save-read"
            | "burrow-render"
            | "burrow-save-write"
            | "burrow-save-read"
            | "snark-render"
            | "snark-save-write"
            | "snark-save-read"
    ) {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = npc::resident_check::native(&mut assets, mode).await {
                eprintln!("Resident check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "clock" {
        return npc::clock_check::check(&mut assets);
    }
    if matches!(
        mode,
        "clock-render" | "clock-save-write" | "clock-save-read"
    ) {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = npc::clock_check::native(&mut assets, mode).await {
                eprintln!("Clockwork check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "imp" {
        return npc::imp_check::check(&mut assets);
    }
    if matches!(mode, "imp-render" | "imp-save-write" | "imp-save-read") {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = npc::imp_check::native(&mut assets, mode).await {
                eprintln!("Fire Imp check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "chess" {
        return npc::chess_check::check(&mut assets);
    }
    if matches!(
        mode,
        "chess-render" | "chess-save-write" | "chess-save-read"
    ) {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = npc::chess_check::native(&mut assets, mode).await {
                eprintln!("Chess check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if matches!(mode, "ant-render" | "ant-save-write" | "ant-save-read") {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = npc::ant_check::native(&mut assets, mode).await {
                eprintln!("Ant check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "combat" {
        return npc::check_combat(&mut assets);
    }
    if mode == "blade-cards-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = weapons::check::render(&mut assets).await {
                eprintln!("Blade/Cards render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "dice-watch-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = weapons::dice_watch_check::render(&mut assets).await {
                eprintln!("Dice/Watch render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "heavy-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = weapons::heavy_check::render(&mut assets).await {
                eprintln!("Staff/Blunderbuss render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "swim-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = water_art::check(&mut assets).await {
                eprintln!("Swimming presentation check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "ice-jacks-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = weapons::ice_jacks_check::render(&mut assets).await {
                eprintln!("Ice/Jacks render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "mallet-jack-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = weapons::mallet_jack_check::render(&mut assets).await {
                eprintln!("Mallet/Jackbomb render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "weapon" {
        return weapons::check(&mut assets);
    }
    if mode == "audio-test" {
        return audio::test_output(&mut assets, &o.map);
    }
    if mode == "audio-regression-test" {
        return audio::regression::output(&mut assets);
    }
    if mode == "world" {
        return interaction::check(&mut assets);
    }
    if mode == "fidelity" {
        return sky::check(&mut assets);
    }
    if mode == "swim" {
        return water::check(&mut assets);
    }
    let maps = assets.maps();
    if mode == "save-preview" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = save_preview::check(&mut assets).await {
                eprintln!("Save preview check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "animation-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = character::render_animations(&mut assets).await {
                eprintln!("Animation render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "presentation-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = character::render_presentation(&mut assets).await {
                eprintln!("Presentation render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "facial-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = facial::render(&mut assets).await {
                eprintln!("Facial render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "village-gesture-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = village::cinema::gnome3::render(&mut assets).await {
                eprintln!("Village gesture check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "opening-audit" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = npc::opening_check::check(&mut assets).await {
                eprintln!("Opening audit failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "npc-placement-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = npc::render_placement(&mut assets).await {
                eprintln!("Actor placement render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "duchess-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = duchess_check::render(&mut assets).await {
                eprintln!("Duchess render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "console" {
        return console::gameplay::check(&mut assets);
    }
    if mode == "pandemonium-machinery-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = pandemonium_render::check_machinery(&mut assets).await {
                eprintln!("Pandemonium machinery render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "beyond-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = beyond_render::check(&mut assets).await {
                eprintln!("Beyond render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "pool-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = pool_check::render(&mut assets).await {
                eprintln!("Pool render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "fortress-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = render::Scene::fortress_check(&mut assets).await {
                eprintln!("Fortress render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "pandemonium-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = pandemonium_render::check(&mut assets).await {
                eprintln!("Pandemonium render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "ladybug-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = ladybug::render_check(&mut assets).await {
                eprintln!("Ladybug render check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "dice-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = dice::render_check(&mut assets).await {
                eprintln!("Dice render check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if matches!(
        mode,
        "campaign-save-chain-write" | "campaign-save-chain-read"
    ) {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) =
                campaign_chain::save_chain_check(&mut assets, mode == "campaign-save-chain-write")
                    .await
            {
                eprintln!("Campaign save chain failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if matches!(mode, "save-write" | "save-read") {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = save_check::check(&mut assets, mode == "save-write").await {
                eprintln!("Save check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "visibility-check" {
        macroquad::Window::from_config(viewer::config(), async move {
            let result = async {
                character::check_visibility(&mut assets).await?;
                viewer::check_level_entry(&mut assets, &o).await?;
                village::cinema::check(&mut assets)?;
                village::cinema::visibility::check(&mut assets).await?;
                village::cinema::render_check(&mut assets).await?;
                fortress::check::check(&mut assets)?;
                pandemonium::cinema::check(&mut assets)?;
                fortress::check::render_check(&mut assets).await?;
                pandemonium_render::check_cast(&mut assets).await?;
                school_return::render_check(&mut assets).await?;
                duchess_check::render(&mut assets).await?;
                // Registered visits add their own fixtures after the legacy list.
                for fixture in levels::visibility() {
                    println!("Visibility fixture {}", fixture.name);
                    (fixture.run)(&mut assets).await?;
                }
                Ok::<_, anyhow::Error>(())
            }
            .await;
            if let Err(e) = result {
                eprintln!("Visibility regression failed: {e:#}");
                std::process::exit(1);
            }
            println!("PASS automated actor visibility suite");
        });
        return Ok(());
    }
    if mode == "cheshire-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = cheshire::check::render(&mut assets).await {
                eprintln!("Cheshire render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "village-cinematic-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = village::cinema::render_check(&mut assets).await {
                eprintln!("Village cinema render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "story-movie" {
        macroquad::Window::from_config(viewer::config(), async move {
            let result = async {
                macroquad::prelude::prevent_quit();
                let ui = ui::Ui::load(&mut assets)?;
                let mut audio = audio::Audio::new(o.no_audio, o.audio_capture.clone());
                let mut input = input::Input::default();
                let preferences = preferences::Preferences::load();
                movie::play(
                    &mut assets,
                    &mut audio,
                    &ui,
                    &mut input,
                    &preferences,
                    &movie_name,
                )
                .await
            }
            .await;
            if let Err(e) = result {
                eprintln!("Movie playback failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "fortress-cinematic-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = fortress::check::render_check(&mut assets).await {
                eprintln!("Fortress cinema render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "village-machinery-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = village::machinery_render::check(&mut assets).await {
                eprintln!("Village machinery render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "progression-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = progression_render::check(&mut assets).await {
                eprintln!("Progression render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "school2-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = school2::check_render(&mut assets).await {
                eprintln!("School two render check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "school-return-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = school_return::render_check(&mut assets).await {
                eprintln!("Return render failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "school-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = school::check_render(&mut assets).await {
                eprintln!("School render check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "death-retry" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = recovery::retry_check::check(&mut assets).await {
                eprintln!("Death retry check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "level-swap" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = viewer::check_level_swaps(&mut assets, &o).await {
                eprintln!("Level swap check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if matches!(
        mode,
        "fidelity-render"
            | "water-render"
            | "fidelity-corpus"
            | "decorations-render"
            | "render-fx"
            | "billboard"
            | "sky-performance"
    ) {
        let mut config = viewer::config();
        if mode == "sky-performance" {
            config.miniquad_conf.platform.swap_interval = Some(0);
        }
        macroquad::Window::from_config(config, async move {
            let result = if mode == "sky-performance" {
                sky_performance_check::check(&mut assets).await
            } else if mode == "water-render" {
                render::check_water(&mut assets).await
            } else if mode == "billboard" {
                render::check_billboards(&mut assets).await
            } else if mode == "render-fx" {
                render_fx::check(&mut assets).await
            } else if mode == "decorations-render" {
                render::check_decorations(&mut assets).await
            } else if mode == "fidelity-corpus" {
                sky::corpus_check(&mut assets).await
            } else {
                sky::render_check(&mut assets).await
            };
            if let Err(e) = result {
                eprintln!("Fidelity render check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "render" {
        macroquad::Window::from_config(viewer::config(), async {
            if let Err(e) = render::check_depth().await {
                eprintln!("Render check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "loot-render" {
        macroquad::Window::from_config(viewer::config(), async move {
            if let Err(e) = loot_art::check(&mut assets).await {
                eprintln!("Dropped item render check failed: {e:#}");
                std::process::exit(1);
            }
        });
        return Ok(());
    }
    if mode == "list" {
        for map in maps {
            println!("{map}");
        }
        return Ok(());
    }
    if mode == "validate" {
        let mut vertices = 0;
        let mut surfaces = 0;
        for name in &maps {
            let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)
                .with_context(|| name.clone())?;
            let world =
                collision::World::from_bsp(&map).with_context(|| format!("Collision in {name}"))?;
            for surface in &map.surfaces {
                if [1, 2, 3].contains(&surface.kind) {
                    let (v, i) = map.triangulate(surface);
                    anyhow::ensure!(
                        i.iter().all(|&i| (i as usize) < v.len()),
                        "Invalid generated index in {name}"
                    );
                }
            }
            println!(
                "OK {name:16} {:6} vertices {:5} surfaces {:4} entities | {} solid brushes, {} patch facets",
                map.vertices.len(),
                map.surfaces.len(),
                map.entities.len(),world.brush_count,world.patch_count
            );
            vertices += map.vertices.len();
            surfaces += map.surfaces.len();
        }
        let textures = assets
            .names()
            .filter(|n| n.ends_with(".ftx"))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        for name in &textures {
            texture::decode_ftx(&assets.read(name)?).with_context(|| name.to_owned())?;
        }
        let materials = texture::read_materials(&mut assets)?;
        println!(
            "PASS: {} maps, {vertices} vertices, {surfaces} surfaces, {} FTX images, {} materials.",
            maps.len(),
            textures.len(),
            materials.len()
        );
        return Ok(());
    }
    if !maps.contains(&o.map) {
        bail!("Unknown map '{}'; use --list", o.map);
    }
    if mode == "physics" {
        let map = Bsp::parse(&assets.read(&format!("maps/{}.bsp", o.map))?)?;
        return movement::check_map(&map);
    }
    if mode == "inspect" {
        let map = Bsp::parse(&assets.read(&format!("maps/{}.bsp", o.map))?)?;
        println!("Map: {}\nFormat: FAKK 42\nArchives: {}\nVertices: {}\nSurfaces: {} (world: {})\nEntities: {}\nLightmaps: {}\nSpawn: {:?}",o.map,assets.pack_count(),map.vertices.len(),map.surfaces.len(),map.world_surfaces.len(),map.entities.len(),map.lightmaps.len()/49152,map.spawn());
        return Ok(());
    }
    macroquad::Window::from_config(viewer::config(), async move {
        if let Err(e) = viewer::run(assets, o).await {
            eprintln!("Viewer failed: {e:#}");
            std::process::exit(1);
        }
    });
    Ok(())
}
