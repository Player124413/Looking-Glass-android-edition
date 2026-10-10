use crate::{
    assets::Assets,
    audio::Audio,
    character::{self, Character},
    hud::Hud,
    interaction::{self, Interactions},
    inventory::{self, Catalog, Stats},
    look::{window_focused, MouseLook},
    movement::{Controls, FixedClock, Player, EYE_HEIGHT},
    recovery::{self, Recovery},
    Options,
};
use anyhow::{Context, Result};
use macroquad::prelude::*;

pub fn config() -> macroquad::conf::Conf {
    let android = crate::android::is_android();
    macroquad::conf::Conf {
        miniquad_conf: Conf {
            window_title: "Looking Glass".into(),
            window_width: 1200,
            window_height: 680,
            fullscreen: false,
            high_dpi: true,
            sample_count: if android { 0 } else { 1 },
            ..Default::default()
        },
        draw_call_vertex_capacity: 60000,
        draw_call_index_capacity: 180000,
        ..Default::default()
    }
}

use crate::render::Scene;
use crate::save::{self, Campaign, Game, Level, Slot, Store, View};
fn load_scene(
    assets: &mut Assets,
    name: &str,
    difficulty: crate::powerups::Difficulty,
) -> Result<Scene> {
    Scene::load_for(assets, name, difficulty)
}
struct Entered {
    art: LevelArt,
    scene: Scene,
    interactions: Interactions,
    story: crate::story::Story,
    hints: crate::cheshire::Hints,
    npcs: crate::npc::Npcs,
    steam: crate::particles::Steam,
    environment_clock: f32,
    pickup_clock: f32,
}
/// Art owns GPU resources and may contain map-specific geometry. Replace this
/// whole group on every kind of level change, including the Tab/console chooser.
#[derive(Default)]
struct LevelArt {
    fortress: Option<crate::fortress::cinema::Art>,
    village: Option<crate::village::cinema::Art>,
    school: Option<crate::school::Art>,
    gym: Option<crate::gym::Art>,
    beyond: Option<crate::beyond::Art>,
    pool: Option<crate::pool::Art>,
    school2: Option<crate::school2::Art>,
    duchess: Option<crate::duchess::Art>,
    pandemonium: Option<crate::pandemonium::Art>,
    encounters: Option<crate::encounters::Art>,
    cheshire: Option<crate::cheshire::Art>,
    /// Registered controllers' art, aligned with `Interactions::levels` (`None`: no art).
    levels: Vec<Option<Box<dyn crate::level::LevelArt>>>,
}
impl LevelArt {
    fn handoff_pose(&self) -> Option<&crate::cinematic::ActorPose> {
        self.village
            .as_ref()
            .and_then(|a| a.alice.handoff_pose())
            .or_else(|| self.fortress.as_ref().and_then(|a| a.alice.handoff_pose()))
            .or_else(|| {
                self.pandemonium
                    .as_ref()
                    .and_then(|a| a.alice.handoff_pose())
            })
            .or_else(|| self.duchess.as_ref().and_then(|a| a.alice.handoff_pose()))
            .or_else(|| {
                self.pool
                    .as_ref()
                    .and_then(|a| a.cinema.alice.handoff_pose())
            })
            .or_else(|| {
                self.school
                    .as_ref()
                    .and_then(|a| a.returning.alice.handoff_pose())
            })
            .or_else(|| self.levels.iter().flatten().find_map(|a| a.handoff_pose()))
    }
    fn prepare(
        &mut self,
        assets: &mut Assets,
        scene: &Scene,
        i: &Interactions,
        cat: bool,
    ) -> Result<()> {
        if i.fortress.is_some() && self.fortress.is_none() {
            self.fortress = Some(crate::fortress::cinema::Art::load(assets)?);
        }
        if cat && self.cheshire.is_none() {
            self.cheshire = Some(crate::cheshire::Art::load(assets)?);
        }
        if i.school.is_some() && self.school.is_none() {
            let specs = crate::texture::read_materials(assets)?;
            self.school = Some(crate::school::Art::load(assets, &specs)?);
        }
        if i.beyond.is_some() && self.beyond.is_none() {
            self.beyond = Some(crate::beyond::Art::load(assets)?);
        }
        if let Some(p) = &i.pool {
            if self.pool.is_none() {
                self.pool = Some(crate::pool::Art::load(assets, p)?);
            }
        }
        if i.gym.is_some() && self.gym.is_none() {
            self.gym = Some(crate::gym::Art::load(assets)?);
        }
        if let Some(s) = &i.school2 {
            if self.school2.is_none() {
                self.school2 = Some(crate::school2::Art::load(assets, s)?);
            }
        }
        if i.duchess.is_some() && self.duchess.is_none() {
            self.duchess = Some(crate::duchess::Art::load(assets, &scene.map)?);
        }
        if i.village.is_some() && self.village.is_none() {
            self.village = Some(crate::village::cinema::Art::load(assets)?);
        }
        if i.pandemonium.is_some() && self.pandemonium.is_none() {
            self.pandemonium = Some(crate::pandemonium::Art::load(assets)?);
        }
        if i.encounters.is_some() && self.encounters.is_none() {
            self.encounters = Some(crate::encounters::Art::load(assets)?);
        }
        if self.levels.is_empty() {
            for slot in &i.levels {
                self.levels.push(
                    slot.reg
                        .art
                        .map(|load| load(assets, &scene.map, &*slot.ctl))
                        .transpose()?,
                );
            }
        }
        Ok(())
    }
    fn story_pose(&mut self, story: &crate::story::Story) {
        if let Some(a) = &mut self.school {
            a.first.story_pose(story);
        }
        if let Some(a) = &mut self.pool {
            a.cinema.story_pose(story);
        }
        if let Some(a) = &mut self.fortress {
            a.story_pose(story);
        }
        if let Some(a) = &mut self.village {
            a.story_pose(story);
        }
        if let Some(art) = &mut self.cheshire {
            art.story_pose(story);
        }
        if let Some(art) = &mut self.pandemonium {
            art.story_pose(story);
        }
        if let Some(art) = &mut self.duchess {
            art.story_pose(story);
        }
        if let Some(art) = &mut self.school2 {
            art.story_pose(story);
        }
        for art in self.levels.iter_mut().flatten() {
            art.story_pose(story);
        }
    }
    fn draw(
        &mut self,
        i: &Interactions,
        hints: &crate::cheshire::Hints,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
        fullbright: bool,
    ) {
        if let (Some(a), Some(f)) = (&mut self.fortress, &i.fortress) {
            a.draw(f, atmosphere, camera, fullbright);
        }
        if let Some(a) = &mut self.cheshire {
            a.draw(hints, atmosphere, camera, fullbright);
        }
        if let (Some(a), Some(s)) = (&mut self.school, &i.school) {
            a.draw(s, fullbright, atmosphere, camera);
        }
        if let (Some(a), Some(v)) = (&mut self.village, &i.village) {
            a.draw(&v.cinema, atmosphere, camera, fullbright);
        }
        if let (Some(a), Some(s)) = (&mut self.pandemonium, &i.pandemonium) {
            a.draw(s, fullbright, atmosphere, camera);
        }
        if let (Some(a), Some(s)) = (&mut self.beyond, &i.beyond) {
            a.draw(s, fullbright, atmosphere, camera);
        }
        if let (Some(a), Some(s)) = (&mut self.pool, &i.pool) {
            a.draw(s, fullbright, atmosphere, camera);
        }
        if let (Some(a), Some(s)) = (&mut self.gym, &i.gym) {
            a.draw(s, fullbright, atmosphere, camera);
        }
        if let (Some(a), Some(s)) = (&mut self.duchess, &i.duchess) {
            a.draw(s, fullbright, atmosphere, camera);
        }
        if let (Some(a), Some(s)) = (&mut self.school2, &i.school2) {
            a.draw(s, fullbright, atmosphere, camera);
        }
        if let (Some(a), Some(s)) = (&mut self.encounters, &i.encounters) {
            a.school_staging = i
                .school
                .as_ref()
                .is_some_and(|s| s.scene_id() == Some(crate::school::cinema::THEATRE));
            a.draw(s, fullbright, atmosphere, camera);
        }
        for (art, slot) in self.levels.iter_mut().zip(&i.levels) {
            if let Some(a) = art {
                a.draw(&*slot.ctl, atmosphere, camera, fullbright);
            }
        }
    }
    /// Registered controllers' effects, drawn with the particle pass.
    fn effects(
        &mut self,
        i: &Interactions,
        camera: Vec3,
        atmosphere: &crate::environment::Atmosphere,
    ) {
        for (art, slot) in self.levels.iter_mut().zip(&i.levels) {
            if let Some(a) = art {
                a.effects(&*slot.ctl, camera, atmosphere);
            }
        }
    }
    /// Registered controllers' HUD elements.
    fn hud(&mut self, i: &Interactions) {
        for (art, slot) in self.levels.iter_mut().zip(&i.levels) {
            if let Some(a) = art {
                a.hud(&*slot.ctl);
            }
        }
    }
}
fn enter_level(
    assets: &mut Assets,
    name: &str,
    entry: Option<&str>,
    cached: Option<&Level>,
    options: &Options,
    play_entry: bool,
) -> Result<Entered> {
    let loaded =
        crate::campaign::load_visit(assets, name, entry, cached, options.difficulty, play_entry)?;
    finish_entered(assets, loaded, name, entry, cached, options)
}
/// The window half of entering a level: the scene, the cast, the particles and the art, built on
/// logic that `campaign::load_visit` or `campaign::arrive` already prepared.
fn finish_entered(
    assets: &mut Assets,
    loaded: crate::campaign::Loaded,
    name: &str,
    entry: Option<&str>,
    cached: Option<&Level>,
    options: &Options,
) -> Result<Entered> {
    let crate::campaign::Loaded {
        map,
        world,
        mut interactions,
        story,
        hints,
    } = loaded;
    let scene = Scene::from_parts(assets, name, map, world)?;
    if options.npc_preview || options.combat_preview || options.larva_preview {
        interactions.encounters = None;
    }
    let mut npcs = crate::npc::Npcs::load(
        assets,
        &scene.map,
        name,
        entry,
        options.npc_preview,
        options.combat_preview,
    )?;
    if options.larva_preview && name == "centipede1" {
        npcs = crate::npc::Npcs::larva_preview(assets)?;
    }
    if let Some(saved) = cached {
        npcs.restore(&saved.npcs)?;
    }
    let steam = crate::particles::Steam::load(assets, &scene.map)?;
    // Prepare the destination cast before replacing any live level state.
    // Tab, console maps and campaign exits must carry this exact scene/art pair.
    let mut art = LevelArt::default();
    art.prepare(assets, &scene, &interactions, true)?;
    Ok(Entered {
        art,
        scene,
        interactions,
        story,
        hints,
        npcs,
        steam,
        environment_clock: cached.map_or(0., |s| s.environment_clock),
        pickup_clock: cached.map_or(0., |s| s.pickup_clock),
    })
}

pub(crate) fn save_capture(path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let frame = get_screen_data();
    // OpenGL reads rows from the bottom; PNG stores rows from the top.
    let mut pixels = frame.bytes;
    let stride = frame.width as usize * 4;
    for y in 0..frame.height as usize / 2 {
        let opposite = frame.height as usize - 1 - y;
        let (top, bottom) = pixels.split_at_mut(opposite * stride);
        top[y * stride..(y + 1) * stride].swap_with_slice(&mut bottom[..stride]);
    }
    image::save_buffer(
        path,
        &pixels,
        frame.width as u32,
        frame.height as u32,
        image::ColorType::Rgba8,
    )?;
    println!("Capture: {}", path.display());
    Ok(())
}

/// Exercise the complete resource graph, including two live players/scenes
/// during replacement. Map-only tests miss lazily loaded encounter/prop materials.
pub async fn check_level_swaps(assets: &mut Assets, options: &Options) -> Result<()> {
    crate::character::check_shared_material().await?;
    let catalog = Catalog::load(assets)?;
    let hud = Hud::load(assets)?;
    let mut power_art = crate::powerups::Art::load(assets)?;
    let mut alice = Character::load(assets)?;
    let mut active = enter_level(assets, "skool1", None, None, options, true)?;
    anyhow::ensure!(active.art.school.is_some(), "Missing starting school cast");
    let names = assets.maps();
    let mut visits = names.iter().map(|n| (n.as_str(), None)).collect::<Vec<_>>();
    visits.extend(
        names
            .iter()
            .filter_map(|n| crate::campaign::return_entry(n).map(|e| (n.as_str(), Some(e)))),
    );
    let mut count = 0;
    for _ in 0..2 {
        for &(name, entry) in &visits {
            // Keep the old full scene/art alive until the new graph is ready.
            let next = enter_level(assets, name, entry, None, options, true)?;
            let mut next_alice = Character::load(assets)?;
            active = next;
            std::mem::swap(&mut alice, &mut next_alice);
            drop(next_alice);
            let (eye, yaw) = interaction::spawn(&active.scene.map, entry);
            let player = Player::spawn(&active.scene.world, eye)
                .unwrap_or_else(|| Player::new(eye - Vec3::Z * EYE_HEIGHT));
            alice.reset(&player, yaw);
            let direction = vec3(yaw.cos(), yaw.sin(), 0.);
            let (camera, target, _) =
                character::follow_camera(&active.scene.world, player.feet, direction);
            let transforms = active.interactions.transforms();
            clear_background(active.scene.atmosphere.background());
            set_camera(&Camera3D {
                position: camera,
                target,
                up: Vec3::Z,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 30000.,
                ..Default::default()
            });
            active.scene.draw(camera, 1., false, false, &transforms);
            alice.atmosphere(&active.scene.atmosphere, camera);
            alice.draw_ghost(player.feet, false, false);
            active.npcs.draw(
                camera,
                (target - camera).normalize_or_zero(),
                &active.scene.atmosphere,
                false,
            );
            active.art.draw(
                &active.interactions,
                &active.hints,
                &active.scene.atmosphere,
                camera,
                false,
            );
            let stats = starting_stats(options, name, entry);
            let pickups = inventory::pickups_for_visit(&active.scene.map, name, entry, &catalog);
            power_art.draw(
                &pickups,
                &stats,
                &save::visit_key(name, entry),
                &active.scene.atmosphere,
                camera,
                1.,
            );
            crate::render::depth_read_only(|| {
                active.scene.draw_with_particles(
                    camera,
                    (target - camera).normalize_or_zero(),
                    1.,
                    false,
                    &transforms,
                    &active.steam,
                );
                alice.draw_effects(camera, false, true);
            });
            set_default_camera();
            hud.draw(
                &stats,
                &catalog,
                false,
                &crate::input::Input::default(),
                &crate::preferences::Preferences::default(),
            );
            next_frame().await;
            count += 1;
            println!("PASS full level replacement {count}: {name} {entry:?}");
        }
    }
    println!("PASS {count} full level replacements with overlapping worlds, Alice, NPCs, particles, level art, Cheshire and power-up art");
    Ok(())
}

/// Exercise the same destination package used by Tab and console map changes.
/// The old level remains alive until the new scene and its cutscene cast are ready.
pub async fn check_level_entry(assets: &mut Assets, options: &Options) -> Result<()> {
    use crate::character::{visibility_image, visible_pixels};
    use crate::village::cinema::Beat;
    let mut active = enter_level(assets, "skool1", None, None, options, false)?;
    anyhow::ensure!(active.art.school.is_some(), "Missing starting school cast");
    for (visit, name) in ["gvillage", "skool1", "gvillage"].into_iter().enumerate() {
        let next = enter_level(assets, name, None, None, options, true)?;
        active = next;
        anyhow::ensure!(
            active.art.village.is_some() == active.interactions.village.is_some()
                && active.art.school.is_some() == active.interactions.school.is_some(),
            "Destination cutscene cast does not match {name}"
        );
        if name != "gvillage" {
            continue;
        }
        let cinema = &mut active.interactions.village.as_mut().unwrap().cinema;
        anyhow::ensure!(
            cinema.state.beat == Some(Beat::Fall),
            "Village opening did not start"
        );
        // Use an authored opening conversation shot; the falling Alice starts
        // above the camera at time zero and cannot prove visible geometry.
        cinema.state.beat = Some(Beat::Cat);
        cinema.state.time = 17.;
        let shot = cinema.camera().context("Missing Village opening camera")?;
        let camera = Camera3D {
            position: shot.eye,
            target: shot.target,
            up: shot.up,
            fovy: 75_f32.to_radians(),
            z_near: 2.,
            z_far: 30000.,
            ..Default::default()
        };
        clear_background(BLACK);
        set_camera(&camera);
        crate::render_fx::begin_view(&camera, 17., &active.scene.atmosphere, true);
        active.scene.draw(
            shot.eye,
            17.,
            true,
            false,
            &active.interactions.transforms(),
        );
        let before = visibility_image();
        crate::render_fx::begin_view(&camera, 17., &active.scene.atmosphere, true);
        active.art.story_pose(&active.story);
        active.art.draw(
            &active.interactions,
            &active.hints,
            &active.scene.atmosphere,
            shot.eye,
            true,
        );
        let after = visibility_image();
        let pixels = visible_pixels(&before, &after);
        after.export_png(&format!("private/visibility/level-entry-{visit}.png"));
        anyhow::ensure!(
            pixels > 200,
            "Missing Village cast after level switch: {pixels} pixels"
        );
        println!("PASS Tab/console destination cast after school-to-Village switch {visit}: {pixels} pixels");
        next_frame().await;
    }
    Ok(())
}

fn starting_stats(options: &Options, map: &str, entry: Option<&str>) -> Stats {
    let mut stats = if options.weapon_preview {
        Stats::weapon_preview()
    } else if options.hud_preview {
        Stats::preview()
    } else {
        Stats::for_level(map, entry)
    };
    stats.difficulty = options.difficulty;
    stats
}

mod start;

pub async fn run(mut assets: Assets, mut options: Options) -> Result<()> {
    // Release cursor/audio before Miniquad destroys its native request queue.
    prevent_quit();
    let ui = crate::ui::Ui::load(&mut assets)?;
    // Explicit native layout diagnostic; never opens or modifies player saves.
    if options.frames.is_some() && std::env::var_os("LOOKING_GLASS_PRESENTATION_CHECK").is_some() {
        return crate::hud::check_presentation(&mut assets).await;
    }
    let mut escape_menu = crate::menu::Menu::load(&mut assets).context("Loading original menus")?;
    let mut preferences = crate::preferences::Preferences::load();
    let mut input = crate::input::Input::default();
    let mut weapon_buttons = crate::weapon_rules::Buttons::default();
    weapon_buttons.block();
    preferences.display();
    escape_menu.backdrop();
    ui.toast("Opening Wonderland...", screen_height() - 70.);
    next_frame().await;
    let saves_enabled = !(options.npc_preview
        || options.combat_preview
        || options.larva_preview
        || options.weapon_preview
        || options.hud_preview
        || options.shelf_preview
        || options.start_at.is_some()
        || options.story_preview.is_some());
    let store = Store::new(options.save_dir.clone(), assets.fingerprint()?);
    let maps = assets.maps();
    let mut audio = Audio::new(options.no_audio, options.audio_capture.clone());
    let mut opening_requested = options.new_game && options.map == "gvillage";
    let resume = if let Some(slot) = options.load {
        anyhow::ensure!(
            saves_enabled,
            "Save loading is unavailable in a staged preview"
        );
        Some(store.read(slot)?)
    } else if saves_enabled && !options.new_game && !options.direct_map && options.frames.is_none() {
        match start::choose(&mut options, &mut escape_menu, &mut preferences,
            &mut input, &mut audio, &store, &maps).await {
            start::Start::Chapter => { opening_requested = false; None }
            start::Start::NewGame => { opening_requested = true; None }
            start::Start::Load(loaded) => Some(*loaded),
            start::Start::Quit => return Ok(()),
        }
    } else {
        None
    };
    escape_menu.frontend = false;
    escape_menu.close();
    if let Some(loaded) = &resume {
        options.map = loaded.game.level()?.map.clone();
        options.entry = loaded.game.level()?.entry.clone();
        options.difficulty = loaded.game.stats.difficulty;
    }
    escape_menu.backdrop();
    ui.toast("Opening Wonderland...", screen_height() - 70.);
    next_frame().await;
    let level_choices = crate::campaign::level_choices(&maps);
    let mut current = maps
        .iter()
        .position(|m| m == &options.map)
        .context("Map not found")?;
    let mut scene = load_scene(&mut assets, &maps[current], options.difficulty)?;
    let mut level_art = LevelArt::default();
    let mut skip_scene = crate::cinematic::Skip::default();
    let mut power_art = crate::powerups::Art::load(&mut assets)?;
    let mut hints = crate::cheshire::Hints::load(&mut assets, &scene.map, &maps[current])?;
    let mut console = crate::console::Console::new();
    let mut steam = crate::particles::Steam::load(&mut assets, &scene.map)?;
    let mut interactions = Interactions::load(&scene.map)?;
    interactions.set_entry(
        &mut assets,
        &scene.map,
        &maps[current],
        options.entry.as_deref(),
    )?;
    if options.npc_preview || options.combat_preview || options.larva_preview {
        interactions.encounters = None;
    }
    interactions.sync(&mut scene.world);
    save::validate_entry(&scene.map, options.entry.as_deref())?;
    let mut entry_spawn = options.entry.clone();
    let mut npcs = crate::npc::Npcs::load(
        &mut assets,
        &scene.map,
        &maps[current],
        entry_spawn.as_deref(),
        options.npc_preview,
        options.combat_preview,
    )?;
    if options.larva_preview && maps[current] == "centipede1" {
        npcs = crate::npc::Npcs::larva_preview(&mut assets)?;
    }
    let catalog = Catalog::load(&mut assets)?;
    let mut stats = starting_stats(&options, &maps[current], entry_spawn.as_deref());
    println!(
        "Level loadout {}: {}",
        maps[current],
        stats.weapon_summary()
    );
    let mut hud = Hud::load(&mut assets)?;
    if options.hud_preview {
        hud.announce("HUD preview / sample resources and all toys");
    }
    if options.weapon_preview {
        hud.announce("Weapon preview / all toys equipped with 1-0");
    }
    if options.combat_preview {
        hud.announce("Combat preview / 1 Blade, 2 Cards, 3 Mallet / Home retry");
    }
    let mut pickups =
        inventory::pickups_for_visit(&scene.map, &maps[current], entry_spawn.as_deref(), &catalog);
    let mut pickup_clock = 0.;
    let mut pickup_feedback = inventory::PickupFeedback::default();
    let mut environment_clock = 0.;
    audio.load(&mut assets, &maps[current], &scene.map);
    let mut story = crate::story::Story::load(&mut assets, &maps[current]);
    if options.shelf_preview {
        anyhow::ensure!(maps[current] == "skool1", "--shelf-preview requires skool1");
        let mut staging = Player::spawn(&scene.world, scene.map.spawn().0)
            .context("School staging entrance blocked")?;
        let school = interactions
            .school
            .as_mut()
            .context("School staging unavailable")?;
        for event in [
            "Theatre_Cinematic",
            "Skool1_OG_MoveShelf",
            "shelf_cinematic",
        ] {
            school.event(event);
        }
        for _ in 0..1250 {
            interactions.advance_school(
                crate::movement::FIXED_DT,
                &scene.map,
                &mut scene.world,
                &mut staging,
            )?;
        }
        hud.announce("Shelf crossing preview / WASD move, Space jump");
    }
    if let Some(event) = &options.story_preview {
        anyhow::ensure!(story.trigger(event), "Unknown story preview {event}");
    } else if options.start_at.is_none() && !options.fly && !options.shelf_preview {
        interactions.entry_story(&mut story);
    }
    let (mut pos, mut yaw) = interaction::spawn(&scene.map, entry_spawn.as_deref());
    if let Some(feet) = options.start_at {
        pos = feet + Vec3::Z * EYE_HEIGHT;
    }
    if options.larva_preview && maps[current] == "centipede1" {
        (pos, yaw) = npcs.larva_preview_start(&scene.world)?;
        hud.announce("Larva encounter / WASD move, mouse attack / Home retry");
    }
    if options.shelf_preview {
        // The exact formerly stalled contact from the real-data drop regression.
        pos = vec3(220., 3300., 413.13885) + Vec3::Z * EYE_HEIGHT;
        yaw = std::f32::consts::FRAC_PI_4 * 3.;
    }
    let spawn = Player::spawn(&scene.world, pos);
    let mut flying = options.fly || spawn.is_none();
    // Some map starts still expect a cinematic or scripted moving support.
    let mut spawn_landing = true;
    let mut player = spawn.unwrap_or_else(|| Player::new(pos - Vec3::Z * EYE_HEIGHT));
    let mut alice = Character::load(&mut assets).context("Loading Alice's character")?;
    let mut aim_pointer = crate::targeting::Pointer::load(&mut assets)?;
    alice.reset(&player, yaw);
    level_art.prepare(&mut assets, &scene, &interactions, true)?;
    let mut third_person = true;
    let mut follow_camera = crate::camera::Follow::default();
    let mut camera_handoff = crate::camera::Handoff::default();
    let mut rail_camera = crate::camera::Follow::default();
    let mut clock = FixedClock::default();
    let mut recovery = Recovery::default();
    let mut retry = recovery::RetryPoint::default();
    let mut paused = false;
    let mut pitch = 0.0f32;
    let mut frame = 0;
    let mut menu = false;
    let mut inventory_menu = false;
    let mut help_until = 0.0;
    let mut fullbright = false;
    let mut selected = current;
    let mut chapters = crate::chapters::Chapters::default();
    let mut selected_entry: Option<String> = None;
    let mut notice = String::new();
    let mut new_game_requested = None;
    let mut chapter_requested = None;
    let mut mouse_look = MouseLook::default();
    let mut was_focused = true;
    let mut campaign = Campaign::default();
    let mut played = resume.is_none();
    let mut post_game = false;
    let mut save_frame = crate::save_preview::Frame::default();
    // Filled by Action::Save in the modal menu loop so the save runs next
    // frame after the menu closes and the 3D world re-renders for a fresh
    // thumbnail (instead of saving a menu-stilled or black frame).
    let mut menu_save_pending: Option<Slot> = None;
    // Overlay grace: time-based, 400 ms after opening MAP/inventory. During
    // this window we ignore Escape/pointer clicks so the opening finger can't
    // immediately dismiss the newly opened overlay. Wall-clock based because
    // Android/miniquad sometimes skip Stationary touch events (touches() looks
    // empty while the finger is still held), which makes frame-count-based or
    // finger-lift-based grace end one frame too early.
    let mut overlay_grace_until: f64 = 0.;
    macro_rules! level_snapshot {
        () => {
            Level {
                map: maps[current].clone(),
                entry: entry_spawn.clone(),
                interactions: interactions.snapshot(),
                npcs: npcs.snapshot(),
                story: story.snapshot(),
                hints: hints.snapshot(),
                environment_clock,
                pickup_clock,
            }
        };
    }
    macro_rules! game_snapshot {
        () => {{
            let level = level_snapshot!();
            let key = level.key();
            let mut saved_campaign = campaign.clone();
            saved_campaign.levels.insert(key.clone(), level);
            Game {
                current: key,
                campaign: saved_campaign,
                stats: stats.clone(),
                player: player.clone(),
                view: View {
                    position: pos,
                    yaw,
                    pitch,
                    third_person,
                    flying,
                    fullbright,
                    spawn_landing,
                },
                recovery: recovery.clone(),
                character: alice.snapshot(),
            }
        }};
    }
    macro_rules! apply_loaded {
        ($restored:expr) => { apply_loaded!($restored, true) };
        ($restored:expr, $pause:expr) => {{
            let r = $restored;
            let level = r.game.level()?.clone();
            current = maps
                .iter()
                .position(|m| m == &level.map)
                .context("Saved map unavailable")?;
            selected = current;
            retry.remember(r.game.clone());
            entry_spawn = level.entry;
            selected_entry = entry_spawn.clone();
            scene = r.scene;
            interactions = r.interactions;
            story = r.story;
            hints = r.hints;
            npcs = r.npcs;
            alice = r.alice;
            steam = r.steam;
            campaign = r.game.campaign;
            stats = r.game.stats;
            options.difficulty = stats.difficulty;
            player = r.game.player;
            recovery = r.game.recovery;
            pos = r.game.view.position;
            yaw = r.game.view.yaw;
            pitch = r.game.view.pitch;
            flying = r.game.view.flying;
            third_person = r.game.view.third_person;
            follow_camera.reset();
            camera_handoff.reset();
            rail_camera.reset();
            fullbright = r.game.view.fullbright;
            spawn_landing = r.game.view.spawn_landing;
            environment_clock = level.environment_clock;
            pickup_clock = level.pickup_clock;
            power_art.clear_effects();
            pickups = inventory::pickups_for_visit(
                &scene.map,
                &maps[current],
                entry_spawn.as_deref(),
                &catalog,
            );
            level_art = LevelArt::default();
            level_art.prepare(&mut assets, &scene, &interactions, true)?;
            skip_scene = crate::cinematic::Skip::default();
            audio.load(&mut assets, &maps[current], &scene.map);
            clock.pause();
            paused = $pause;
            menu = false;
            inventory_menu = false;
            console.open = false;
            help_until = 0.;
            input.suppress();
            mouse_look.release();
            notice.clear();
            played = true;
            post_game = false;
            escape_menu.post_game = false;
            save_frame.clear();
        }};
    }
    if resume.is_none()
        && saves_enabled
        && opening_requested
        && options.frames.is_none()
    {
        match crate::movie::play(
            &mut assets,
            &mut audio,
            &ui,
            &mut input,
            &preferences,
            "opening",
        )
        .await
        {
            Ok(false) => return Ok(()),
            Err(e) => {
                eprintln!("Opening film unavailable: {e:#}");
                hud.announce("Opening film unavailable / continuing to Wonderland");
            }
            Ok(true) => (),
        }
        clock.pause();
    }
    if let Some(loaded) = resume {
        let backup = loaded.backup;
        match save::Restored::build(&mut assets, loaded.game) {
            Ok(restored) => {
                apply_loaded!(restored);
                hud.announce(if backup {
                    "Previous save restored / P or click to resume"
                } else {
                    "Game restored / P or click to resume"
                });
            }
            Err(e) => {
                paused = true;
                notice = format!("Could not restore save: {e:#}");
            }
        }
    } else if saves_enabled
        && !options.new_game
        && store.latest().is_none()
        && (store.exists(Slot::Quick) || store.exists(Slot::Auto))
    {
        hud.announce("A saved game could not be read. F9 retries quick load.");
    }
    retry.remember(game_snapshot!());
    loop {
        let mut frame_profile = crate::frame_profile::span("frame_cpu");
        let current_aim = vec3(
            yaw.cos() * pitch.cos(),
            yaw.sin() * pitch.cos(),
            pitch.sin(),
        );
        let can_use = player.rope_prompt(&scene.world).is_some()
            || interactions
                .pandemonium
                .as_ref()
                .is_some_and(|p| p.prompt(&scene.world, player.eye()).is_some())
            || interactions
                .conversation(&scene.world, player.eye(), current_aim, &npcs, &story)
                .is_some()
            || interactions
                .prompt(&scene.world, player.eye(), current_aim)
                .is_some();
        let cutscene_active = interactions
            .village
            .as_ref()
            .is_some_and(|v| v.cinema.active())
            || interactions
                .fortress
                .as_ref()
                .is_some_and(|f| f.state.cinema.active())
            || interactions
                .duchess
                .as_ref()
                .and_then(|d| d.scene_id())
                .is_some()
            || interactions
                .pandemonium
                .as_ref()
                .and_then(|p| p.scene_id())
                .is_some()
            || interactions
                .beyond
                .as_ref()
                .is_some_and(|b| b.scene_id().is_some())
            || interactions.scene_id_after(None).is_some()
            || interactions
                .pool
                .as_ref()
                .is_some_and(|p| p.scene_id().is_some())
            || interactions
                .school
                .as_ref()
                .is_some_and(|s| s.scene_id().is_some());
        input.set_touch_context(crate::touch::TouchContext {
            in_gameplay: !menu && !inventory_menu && !console.open && !paused,
            paused,
            inventory_open: inventory_menu,
            help_open: get_time() < help_until,
            alive: stats.alive(),
            swimming: player.immersion.level >= 2,
            climbing_rope: player.rope.is_some()
                || player.script_motion == 1
                || interactions
                    .pandemonium
                    .as_ref()
                    .is_some_and(|p| p.rope_hand().is_some()),
            flying,
            dialogue_active: story.busy(),
            cinematic_active: cutscene_active,
            can_interact: can_use,
        });
        input.update(&preferences, window_focused());
        let overlay_before = menu || inventory_menu || console.open || paused;
        if input.disconnected {
            paused = true;
            hud.announce("Controller disconnected / reconnect or use keyboard and mouse");
        }
        if is_quit_requested() {
            break;
        }
        if window_focused() && !console.open && !escape_menu.page.is_some() {
            if (is_key_pressed(KeyCode::Escape)
                || is_key_pressed(KeyCode::Back)
                || input.pad_pressed("Start")
                || input.touch.menu_pressed())
                && !menu
                && !inventory_menu
            {
                escape_menu.open(crate::menu::Page::Main, &preferences, audio.settings);
            } else if is_key_pressed(KeyCode::F3) {
                menu = false;
                inventory_menu = false;
                escape_menu.open(crate::menu::Page::Audio, &preferences, audio.settings);
            }
        }
        if escape_menu.page.is_some() {
            weapon_buttons.block();
            input.suppress();
            mouse_look.release();
            clock.pause();
            help_until = 0.;
            // Drain the opening MENU/MAP finger across the await so it cannot
            // re-fire a click inside the modal loop and immediately dismiss it.
            next_frame().await;
            input.suppress();
            crate::touch::drain_active_touches();
            let action = escape_menu
                .run(&mut preferences, &mut audio, &store, pos, yaw, &mut input)
                .await;
            use crate::menu::Action;
            match action {
                Action::Resume => {
                    paused = false;
                }
                Action::Quit => break,
                Action::Chapters => {
                    match escape_menu.choose_chapter(&maps, current, entry_spawn.as_deref(),
                        &mut options.difficulty, &preferences, &mut audio, &mut input).await {
                        crate::menu::ChapterChoice::Begin(choice) => {
                            chapter_requested = Some(choice);
                            escape_menu.close();
                        }
                        crate::menu::ChapterChoice::Quit => break,
                        crate::menu::ChapterChoice::Back => (),
                    }
                }
                Action::Save(slot) => {
                    escape_menu.message = if !saves_enabled {
                        "Saving is unavailable in a staged preview".into()
                    } else if !stats.alive() {
                        "Retry or load a game before saving".into()
                    } else {
                        // Defer to the next frame so the 3D world renders once
                        // more before we grab_screen() — otherwise the thumbnail
                        // shows the menu (or is black on Android where per-frame
                        // capture is disabled). The modal menu frame loop cannot
                        // set `pending_save_slot` directly because that variable
                        // lives in the per-frame scope below, so we carry the
                        // request out through `menu_save_pending`.
                        menu_save_pending = Some(slot);
                        escape_menu.close();
                        paused = false;
                        format!("Saving to {}...", slot.title())
                    };
                }
                Action::Load(slot) => {
                    if saves_enabled {
                        match store
                            .read(slot)
                            .and_then(|loaded| save::Restored::build(&mut assets, loaded.game))
                        {
                            Ok(restored) => {
                                apply_loaded!(restored);
                                escape_menu.close();
                                hud.announce("Game restored / P or click to resume");
                            }
                            Err(e) => escape_menu.message = format!("Load failed: {e:#}"),
                        }
                    } else {
                        escape_menu.message = "Loading is unavailable in a staged preview".into();
                    }
                }
                Action::NewGame(difficulty) => {
                    // Preserve the current session before a new campaign replaces it.
                    let saved = if saves_enabled && stats.alive() && !post_game {
                        store.write_with_preview(
                            Slot::Auto,
                            &game_snapshot!(),
                            save_frame.preview().as_ref(),
                        )
                    } else {
                        Ok(())
                    };
                    if !saves_enabled {
                        escape_menu.open(crate::menu::Page::Main, &preferences, audio.settings);
                        escape_menu.message = "New Game is unavailable in a staged preview".into();
                    } else if let Err(e) = saved {
                        escape_menu.open(crate::menu::Page::Main, &preferences, audio.settings);
                        escape_menu.message = format!("Could not preserve the current game: {e:#}");
                    } else {
                        post_game = false;
                        escape_menu.post_game = false;
                        new_game_requested = Some(difficulty);
                    }
                }
            }
            clock.pause();
            input.suppress();
            next_frame().await;
            continue;
        }
        let elapsed = get_frame_time();
        let mut weapon_click;
        let mut weapon_input_enabled = false;
        let mut pending_transition = None;
        let mut advance_dialogue = false;
        let dt = elapsed.clamp(0., 0.05);
        let world_dt = stats.powers.world_dt(dt);
        let power_before = stats.active_power().map(|(k, _)| k);
        let loot_visit = save::visit_key(&maps[current], entry_spawn.as_deref());
        let mut loot_before = interactions.loot_sources();
        loot_before.extend(npcs.loot_sources());
        let focused = window_focused();
        if !focused && was_focused {
            paused = true;
            clock.pause();
            // A key released in another app may never send this window a key-up.
            // Ignore it until released here or explicitly pressed again.
            input.suppress();
        }
        was_focused = focused;
        // Macroquad 0.4.14 pops its character Vec from the end. Restore event
        // order when typing/pasting delivers several characters in one frame.
        let mut chars = std::iter::from_fn(get_char_pressed).collect::<Vec<_>>();
        chars.reverse();
        let toggle_console = focused
            && (is_key_pressed(KeyCode::GraveAccent)
                || chars.iter().any(|c| matches!(c, '`' | '~' | '\u{00ac}')));
        let console_input = console.open || toggle_console;
        let mut console_map = if let Some(choice) = chapter_requested.take() {
            selected = choice.map;
            selected_entry = choice.entry.map(str::to_owned);
            post_game = false;
            escape_menu.post_game = false;
            true
        } else { false };
        if let Some(difficulty) = new_game_requested.take() {
            match crate::movie::play(
                &mut assets,
                &mut audio,
                &ui,
                &mut input,
                &preferences,
                "opening",
            )
            .await
            {
                Ok(false) => break,
                Err(e) => {
                    eprintln!("Opening film unavailable: {e:#}");
                    hud.announce("Opening film unavailable / continuing to Wonderland");
                }
                Ok(true) => (),
            }
            clock.pause();
            selected = maps
                .iter()
                .position(|name| name == "gvillage")
                .context("Campaign opening map missing")?;
            selected_entry = None;
            options.difficulty = difficulty;
            options.fly = false;
            console_map = true;
        }
        let mut console_restart = false;
        let mut console_fly = false;
        let mut summon_cat = false;
        // pending_save_slot collects every save trigger that must fire at end-of-frame
        // (after the world has been re-rendered so the thumbnail is fresh). This covers:
        //   - F5 quick save
        //   - autosave transitions (level load / checkpoint)
        //   - the pause menu's "Save to slot N" (carried over from the modal loop
        //     via `menu_save_pending` so we capture a clean in-game frame)
        let mut pending_save_slot: Option<Slot> = menu_save_pending.take();
        if focused && !console_input && input.key(&preferences, KeyCode::F5, true) {
            pending_save_slot = Some(Slot::Quick);
        }
        let mut load_requested =
            (focused && !console_input && input.key(&preferences, KeyCode::F9, true))
                .then_some(Slot::Quick);
        if toggle_console || (console.open && focused && input.ui(KeyCode::Escape)) {
            console.open = !console.open;
            menu = false;
            inventory_menu = false;
            help_until = 0.;
            clock.pause();
            input.suppress();
        }
        if console.open && focused {
            if let Some(command) = console.input(&chars) {
                use crate::console::Command;
                match command {
                    Err(message) => console.print(message),
                    Ok(Command::Help) => {
                        console.print("help/cmdlist  clear  version  status/getpos  maplist");
                        console.print(
                            "map <level[$entrance]>  restart  noclip  god  notarget  fullbright",
                        );
                        console.print("wuss  give all|weapons|health|will|<item.tik>  itemlist");
                        console.print("health <number>  cg_cameradist -45 (first person) / 128 (third person)");
                        console.print("cheshire  quit");
                        console
                            .print("save / F5  quick save    load / F9  quick load    load auto");
                        console.print("Map starts a fresh visit. While alive, restart recovers the entrance. After death, retry restores the last save or visit start.");
                        console.print("Saves retain cheat settings. Unsupported original commands are not executed.");
                    }
                    Ok(Command::Clear) => console.clear(),
                    Ok(Command::Version) => console.print(format!(
                        "Looking Glass {} / independent Rust runtime",
                        env!("CARGO_PKG_VERSION")
                    )),
                    Ok(Command::Status) => {
                        console.print(format!(
                            "{} {:?} / feet {:.2} {:.2} {:.2} / yaw {:.1} pitch {:.1}",
                            maps[current],
                            entry_spawn,
                            player.feet.x,
                            player.feet.y,
                            player.feet.z,
                            yaw.to_degrees(),
                            pitch.to_degrees()
                        ));
                        console.print(format!(
                            "Sanity {:.0} / Will {:.0} / god {} / notarget {} / noclip {} / grounded {}",
                            stats.sanity(),
                            stats.will(),
                            stats.god,
                            stats.notarget,
                            flying,
                            player.grounded
                        ));
                        console.print(stats.weapon_summary());
                        console.print(format!(
                            "cg_cameradist {} / {}",
                            if third_person {
                                preferences.camera_distance
                            } else {
                                -45.
                            },
                            if third_person {
                                "third person"
                            } else {
                                "first person"
                            }
                        ));
                        console.print(format!(
                            "Difficulty {} / power {:?} / Watch {:.2}s / recharge {:.2}s",
                            stats.difficulty.name(),
                            stats.active_power(),
                            stats.powers.stopped,
                            stats.powers.recharge
                        ));
                        console.print(format!(
                            "Campaign: {} completed visits / {} cached visits",
                            campaign.completed.len(),
                            campaign.levels.len()
                        ));
                        if let Some(q) = &interactions.school2 {
                            console.print(q.quest.objective());
                        }
                        if let Some(q) = &interactions.school {
                            console.print(q.objective());
                        }
                    }
                    Ok(Command::Maps) => {
                        for chunk in maps.chunks(6) {
                            console.print(chunk.join("  "));
                        }
                    }
                    Ok(Command::Map(name, entry)) => {
                        if let Some(index) = maps.iter().position(|m| m == &name) {
                            let valid = if let Some(entrance) = &entry {
                                assets
                                    .read(&format!("maps/{name}.bsp"))
                                    .and_then(|b| crate::bsp::Bsp::parse(&b))
                                    .is_ok_and(|m| {
                                        m.entities
                                            .iter()
                                            .any(|e| e.get("targetname") == Some(entrance))
                                    })
                            } else {
                                true
                            };
                            if valid {
                                selected = index;
                                selected_entry = entry;
                                console_map = true;
                            } else {
                                console.print("That named entrance does not exist in this level.");
                            }
                        } else {
                            console.print("Unknown level. Use maplist.");
                        }
                    }
                    Ok(Command::Restart) => {
                        console_restart = true;
                        console.open = false;
                    }
                    Ok(Command::Save) => {
                        if pending_save_slot.is_none() {
                            pending_save_slot = Some(Slot::Quick);
                        }
                    }
                    Ok(Command::Load(slot)) => load_requested = Some(slot),
                    Ok(Command::Noclip) => {
                        console_fly = true;
                    }
                    Ok(Command::God) => {
                        stats.god = !stats.god;
                        console.print(format!("God mode {}", if stats.god { "ON" } else { "OFF" }));
                    }
                    Ok(Command::Notarget) => {
                        stats.notarget = !stats.notarget;
                        console.print(format!(
                            "Notarget {}",
                            if stats.notarget { "ON" } else { "OFF" }
                        ));
                    }
                    Ok(Command::Health(value)) => {
                        let dead = !stats.alive();
                        match stats.set_health(value) {
                            Ok(()) => {
                                if dead && stats.alive() {
                                    alice.reset(&player, yaw);
                                    follow_camera.reset();
                                    camera_handoff.reset();
                                }
                                console.print(format!("Sanity {}", stats.sanity()));
                            }
                            Err(e) => console.print(e.to_string()),
                        }
                    }
                    Ok(Command::CameraDistance(value)) => {
                        if let Some(value) = value {
                            crate::console::gameplay::camera(
                                value,
                                &mut third_person,
                                &mut preferences,
                            );
                            follow_camera.reset();
                            camera_handoff.reset();
                            if let Err(e) = preferences.save() {
                                console.print(format!(
                                    "Camera applied; settings could not be saved: {e}"
                                ));
                            }
                        }
                        console.print(format!(
                            "cg_cameradist {} / {}",
                            if third_person {
                                preferences.camera_distance
                            } else {
                                -45.
                            },
                            if third_person {
                                "third person"
                            } else {
                                "first person"
                            }
                        ));
                    }
                    Ok(Command::Wuss) => {
                        match crate::console::gameplay::give(
                            "weapons", &assets, &catalog, &mut stats,
                        ) {
                            Ok(message) => console.print(message),
                            Err(e) => console.print(e.to_string()),
                        }
                    }
                    Ok(Command::Give(item)) => {
                        match crate::console::gameplay::give(&item, &assets, &catalog, &mut stats) {
                            Ok(message) => {
                                crate::console::gameplay::sync_inventory(&stats, &mut interactions);
                                stats.prepare_player(&mut player);
                                console.print(message);
                            }
                            Err(e) => console.print(e.to_string()),
                        }
                    }
                    Ok(Command::Items) => {
                        for row in crate::console::gameplay::items(&assets).chunks(3) {
                            console.print(row.join("  "));
                        }
                        console.print("Use give <filename>. Power-ups activate immediately; story items do not finish unrelated quest stages.");
                    }
                    Ok(Command::Cheshire) => {
                        summon_cat = true;
                    }
                    Ok(Command::Fullbright) => {
                        fullbright = !fullbright;
                        console.print(format!("Fullbright {fullbright}"));
                    }
                    Ok(Command::Quit) => break,
                }
            }
        }
        if let Some(slot) = load_requested {
            if saves_enabled {
                match store.read(slot).and_then(|loaded| {
                    let backup = loaded.backup;
                    save::Restored::build(&mut assets, loaded.game).map(|r| (r, backup))
                }) {
                    Ok((restored, backup)) => {
                        apply_loaded!(restored);
                        let message = if backup {
                            "Previous save restored / P or click to resume"
                        } else {
                            "Game restored / P or click to resume"
                        };
                        hud.announce(message);
                        console.print(message);
                    }
                    Err(e) => {
                        clock.pause();
                        paused = true;
                        let message = format!("Load failed: {e:#}");
                        hud.announce(&message);
                        console.print(&message);
                    }
                }
            } else {
                hud.announce("Save/load is unavailable in a staged preview");
            }
            next_frame().await;
            continue;
        }
        // Overlay toggles (MAP / inventory / Escape-back) are handled BEFORE
        // defining the `pressed`/`held` closures so we can mutably call
        // input.suppress()+drain here without fighting the borrow checker.
        let edge = |key: KeyCode| -> bool { focused && !console_input && input.key(&preferences, key, true) };
        let mut opening_map = false;
        let mut opening_inv = false;
        // Ignore Escape-close of MAP/inventory during the opening grace period
        // (time-based: 400 ms).
        let in_overlay_grace = get_time() < overlay_grace_until;
        let esc_pressed = input.ui(KeyCode::Escape)
            || is_key_pressed(KeyCode::Back);
        if esc_pressed && !console_input && !in_overlay_grace {
            if inventory_menu {
                inventory_menu = false;
            } else if menu {
                menu = false;
            }
        }
        if edge(KeyCode::Tab) {
            menu = !menu;
            inventory_menu = false;
            help_until = 0.0;
            if menu {
                opening_map = true;
                clock.pause();
                overlay_grace_until = get_time() + 0.40;
            }
            chapters.open(
                crate::campaign::choice_position(&level_choices, current, entry_spawn.as_deref()),
                level_choices.len(),
            );
        }
        if edge(KeyCode::I) && focused {
            inventory_menu = !inventory_menu;
            menu = false;
            help_until = 0.;
            clock.pause();
            if inventory_menu {
                opening_inv = true;
                overlay_grace_until = get_time() + 0.40;
            }
        }
        // Drain the finger that just opened MAP/inventory so it can't click
        // through onto the new overlay on its first frame.
        if opening_map || opening_inv {
            input.suppress();
            crate::touch::drain_active_touches();
        }

        // Edge/held helpers for the rest of the frame. These closures borrow
        // `input` immutably but are only used after the suppress() block above.
        let pressed = |key: KeyCode| -> bool { focused && !console_input && input.key(&preferences, key, true) };
        let held    = |key: KeyCode| -> bool { focused && !console_input && input.key(&preferences, key, false) };

        let mut audio_changed = false;
        if pressed(KeyCode::M) {
            audio.settings.muted = !audio.settings.muted;
            audio_changed = true;
        }
        if audio_changed {
            if let Err(e) = audio.settings.save() {
                notice = format!("Could not save sound settings: {e}");
            }
        }
        let mut help_toggle = false;
        if (pressed(KeyCode::H) || pressed(KeyCode::F1))
            && !menu
            && !inventory_menu
            && !console_input
        {
            help_toggle = true;
        }
        if help_toggle {
            help_until = if get_time() < help_until {
                0.0
            } else {
                get_time() + 8.0
            };
        }
        if pressed(KeyCode::F2) {
            fullbright = !fullbright;
        }
        if pressed(KeyCode::V) && !menu && !inventory_menu && !console_input {
            third_person = !third_person;
            follow_camera.reset();
            camera_handoff.reset();
        }
        if pressed(KeyCode::P) && focused {
            paused = !paused;
            clock.pause();
        }
        let was_paused = paused;
        let (pointer_pos, pointer_pressed, _) = crate::touch::pointer_state();
        if paused
            && focused
            && !menu
            && !inventory_menu
            && !console_input
            && get_time() >= help_until
            && (pointer_pressed || input.pad_pressed("A"))
        {
            if pointer_pos.x >= 0.
                && pointer_pos.y >= 0.
                && pointer_pos.x < screen_width()
                && pointer_pos.y < screen_height()
            {
                paused = false;
                clock.pause();
            }
        }
        mouse_look.sensitivity = preferences.sensitivity;
        mouse_look.inverted = preferences.invert_mouse;
        let scripted_view = interactions.scripted()
            || interactions.duchess.as_ref().is_some_and(|d| d.cinematic())
            || interactions
                .school
                .as_ref()
                .and_then(|s| s.return_visit.as_ref())
                .is_some_and(|s| s.camera().is_some());
        if !scripted_view {
            player.script_facing = yaw;
        }
        mouse_look.update(
            focused
                && !paused
                && !menu
                && !inventory_menu
                && !console_input
                && !scripted_view
                && !input.using_touch
                && stats.alive(),
            &mut yaw,
            &mut pitch,
        );
        hud.update(dt);
        let inv_grace = in_overlay_grace && inventory_menu;
        if focused && !console_input && stats.alive() && !menu && (inventory_menu || !paused) {
            let keys = [
                KeyCode::Key1,
                KeyCode::Key2,
                KeyCode::Key3,
                KeyCode::Key4,
                KeyCode::Key5,
                KeyCode::Key6,
                KeyCode::Key7,
                KeyCode::Key8,
                KeyCode::Key9,
                KeyCode::Key0,
            ];
            let selected = if inv_grace {
                None
            } else {
                keys.iter().position(|&k| pressed(k)).or_else(|| {
                    if inventory_menu {
                        hud.inventory_hit()
                    } else {
                        None
                    }
                })
            };
            if let Some(i) = selected {
                if stats.select(i) {
                    hud.selected(&stats);
                    if inventory_menu && input.using_touch {
                        inventory_menu = false;
                    }
                } else {
                    hud.announce(format!("Not yet available: {}", inventory::WEAPONS[i].1));
                }
            } else if inventory_menu && !inv_grace && hud.inventory_close_hit() {
                inventory_menu = false;
            }
            let previous = input.action(&preferences, "Wheel Up", true)
                || (inventory_menu && !inv_grace && (input.ui(KeyCode::Up) || input.ui(KeyCode::Left)));
            let next = input.action(&preferences, "Wheel Down", true)
                || (inventory_menu && !inv_grace && (input.ui(KeyCode::Down) || input.ui(KeyCode::Right)));
            if previous || next {
                stats.cycle(if next { 1 } else { -1 });
                hud.selected(&stats);
            }
            weapon_input_enabled = !inventory_menu
                && stats.equipped().is_some()
                && !was_paused
                && !overlay_before
                && stats.alive()
                && !flying
                && player.script_motion == 0
                && !interactions.level_blocks_weapons();
            if weapon_input_enabled
                && (input.action(&preferences, "Mouse 1", true)
                    || input.action(&preferences, "Mouse 2", true))
            {
                if !crate::weapons::supported(stats.selected()) {
                    hud.announce("This toy's attacks are not yet available");
                } else if stats.selected() == 6 && !alice.ready_to_attack(6) {
                    hud.announce("Demon Dice are still active or recovering");
                } else if stats.selected() == 9 && stats.powers.recharge > 0. {
                    hud.announce("Pocket Watch is recharging");
                }
            }
        }
        weapon_click = weapon_buttons.sample(
            [
                input.action(&preferences, "Mouse 1", false),
                input.action(&preferences, "Mouse 2", false),
            ],
            weapon_input_enabled,
        );
        if inventory_menu && input.ui(KeyCode::Enter) {
            inventory_menu = false;
        }
        let recover = focused
            && !menu
            && !inventory_menu
            && !console_input
            && ((!stats.alive() && (pressed(KeyCode::Enter) || input.pad_pressed("A")))
                || (!paused && pressed(KeyCode::R)));
        let restart =
            focused && !menu && !inventory_menu && !console_input && pressed(KeyCode::Home);
        if let Some(s) = &mut interactions.school {
            s.sync_inventory(&mut stats);
        }
        let in_transport = interactions.duchess.as_ref().is_some_and(|d| d.cinematic())
            || interactions.scripted()
            || interactions.school.as_ref().is_some_and(|s| {
                s.cinematic()
                    || s.return_visit
                        .as_ref()
                        .is_some_and(|r| r.phase == crate::school_return::Phase::Rising)
            })
            || interactions.levels_transport();
        let skip_id = interactions.scene_id_after(
            interactions
                .duchess
                .as_ref()
                .and_then(|d| d.scene_id())
                .or_else(|| interactions.pandemonium.as_ref().and_then(|p| p.scene_id()))
                .or_else(|| interactions.pool.as_ref().and_then(|p| p.scene_id()))
                .or_else(|| interactions.school.as_ref().and_then(|s| s.scene_id()))
                .or_else(|| interactions.school2.as_ref().and_then(|s| s.scene_id()))
                .or_else(|| interactions.beyond.as_ref().and_then(|b| b.scene_id()))
                .or_else(|| {
                    interactions
                        .fortress
                        .as_ref()
                        .and_then(|f| f.state.cinema.scene_id())
                })
                .or_else(|| {
                    interactions
                        .village
                        .as_ref()
                        .and_then(|v| v.cinema.scene_id())
                }),
        );
        let skip_requested = skip_scene.update(
            skip_id,
            !paused
                && !menu
                && !inventory_menu
                && !console_input
                && focused
                && !flying
                && stats.alive(),
            held(KeyCode::Enter)
                || input.pad_held("A")
                || (skip_id.is_some() && input.touch.any_touch_down()),
            dt,
        );
        if skip_requested {
            let speaking = skip_id.is_some_and(|id| story.progress(id).is_some());
            if interactions.skip_cinematic(&scene.map, &mut scene.world, &mut player, &mut story)? {
                camera_handoff.skip();
                if speaking {
                    audio.stop_voice();
                }
                clock.pause();
                pos = player.eye();
                yaw = player.script_facing;
                pitch = 0.;
                weapon_click = None;
                weapon_buttons.block();
                recovery.clear();
                spawn_landing = true;
                hud.announce("Scene skipped");
            }
        }
        // A failed attempt must roll the entire world back with Alice. Do this
        // before the live transport guard: dying during a ride must remain retryable.
        if saves_enabled && !stats.alive() && (recover || restart || console_restart) {
            match retry.game_for(&save::visit_key(&maps[current], entry_spawn.as_deref()))
                .and_then(|game| save::Restored::build(&mut assets, game))
            {
                Ok(restored) => {
                    apply_loaded!(restored, false);
                    weapon_buttons.block();
                    hud.announce("Checkpoint restored");
                    println!("Death retry: restored complete checkpoint for {}", maps[current]);
                }
                Err(e) => {
                    paused = true;
                    hud.announce(format!("Could not restore checkpoint: {e:#}"));
                    eprintln!("Death retry failed: {e:#}");
                }
            }
            // All per-frame clocks, targets and inputs above belong to the old
            // attempt. Begin a fresh frame before advancing the restored one.
            next_frame().await;
            continue;
        }
        if (recover || restart || console_restart) && in_transport {
            hud.announce(if interactions.scripted() {
                "A scene is playing / hold Enter to skip"
            } else {
                "The ride is in progress / recovery is available after landing"
            });
        }
        if (recover || restart || console_restart) && !in_transport {
            let entry = interaction::spawn(&scene.map, entry_spawn.as_deref());
            let entry = if options.larva_preview && maps[current] == "centipede1" {
                npcs.larva_preview_start(&scene.world)?
            } else {
                entry
            };
            let entry = interactions
                .pandemonium
                .as_ref()
                .map_or(entry, |p| p.recovery_entry(entry));
            let entry = interactions
                .duchess
                .as_ref()
                .map_or(entry, |d| d.recovery_entry(entry));
            let entry = interactions.levels_recovery_entry(entry);
            let checkpoint = interactions
                .school
                .as_ref()
                .and_then(|s| s.return_visit.as_ref())
                .is_some_and(|r| r.phase == crate::school_return::Phase::Observatory);
            let entry = if checkpoint {
                (vec3(692., 4100., 1072.), -std::f32::consts::FRAC_PI_2)
            } else {
                entry
            };
            if restart || console_restart || (checkpoint && player.feet.z < 1000.) {
                recovery.clear();
            }
            if let Some((restored, angle, nearby)) =
                recovery.restore(&scene.world, &interactions, player.feet, entry)
            {
                if let Some(p) = &mut interactions.pandemonium {
                    p.release_rope();
                }
                player = restored;
                yaw = angle;
                pos = player.eye();
                pitch = 0.;
                flying = false;
                paused = false;
                stats.restore();
                npcs.reset_combat();
                if options.larva_preview && maps[current] == "centipede1" {
                    npcs = crate::npc::Npcs::larva_preview(&mut assets)?;
                    npcs.update(0., &scene.world, player.eye());
                    stats.loot.remove(&loot_visit);
                    let prefix = format!("drop:{loot_visit}:");
                    stats.collected.retain(|key| !key.starts_with(&prefix));
                }
                story.recover(&mut audio);
                hints.recover();
                interactions.reset_contacts();
                spawn_landing = true;
                alice.reset(&player, yaw);
                follow_camera.reset();
                camera_handoff.reset();
                clock.pause();
                weapon_click = None;
                weapon_buttons.block();
                notice.clear();
                hud.announce(if nearby {
                    "Restored to footing before the fall"
                } else {
                    "Restored at the level entrance"
                });
                println!(
                    "Recovery: feet={:?}, nearby={nearby}, sanity={}",
                    player.feet,
                    stats.sanity()
                );
            } else {
                hud.announce("Entrance is obstructed / choose another level with Tab");
            }
        }
        if (console_fly || (pressed(KeyCode::F4) && !menu && !inventory_menu)) && stats.alive() {
            if flying {
                let feet = pos - Vec3::Z * EYE_HEIGHT;
                if scene.world.body_clear(feet) {
                    player = Player::new(feet);
                    spawn_landing = false;
                    alice.reset(&player, yaw);
                    follow_camera.reset();
                    camera_handoff.reset();
                    flying = false;
                    notice.clear();
                } else {
                    notice = "Move clear of the wall before switching to walking.".into();
                }
            } else {
                flying = true;
                pos = player.eye();
            }
            if console_fly {
                console.print(format!("Noclip {}", if flying { "ON" } else { "OFF" }));
                if !notice.is_empty() {
                    console.print(notice.clone());
                }
            }
            clock.pause();
        }
        if menu || console_map {
            clock.pause();
            let mut chapter_action = None;
            if menu && !console_map {
                if focused && !console_input {
                    if in_overlay_grace {
                        // During grace, ignore pointer/tab-originated clicks in
                        // the chapter chooser so the opening finger doesn't
                        // immediately select a chapter or hit Back. Keyboard
                        // navigation is OK, but we already suppress input above.
                        chapters.update_ignore_pointer(&input, level_choices.len(), &mut options.difficulty);
                    } else {
                        chapter_action = chapters.update(&input, level_choices.len(), &mut options.difficulty);
                    }
                }
                let choice = &level_choices[chapters.selected];
                selected = choice.map;
                selected_entry = choice.entry.map(str::to_owned);
            }
            if chapter_action == Some(crate::chapters::Hit::Back) {
                menu = false;
            } else if (input.ui(KeyCode::Enter) && !in_overlay_grace)
                || chapter_action == Some(crate::chapters::Hit::Begin)
                || console_map
            {
                match enter_level(
                    &mut assets,
                    &maps[selected],
                    selected_entry.as_deref(),
                    None,
                    &options,
                    !options.fly,
                ) {
                    Ok(next) => {
                        level_art = next.art;
                        skip_scene = crate::cinematic::Skip::default();
                        scene = next.scene;
                        hints = next.hints;
                        steam = next.steam;
                        npcs = next.npcs;
                        interactions = next.interactions;
                        story = next.story;
                        if console_map {
                            console.print(format!("Opened {}", maps[selected]));
                            console.open = false;
                        }
                        entry_spawn = selected_entry.clone();
                        recovery.clear();
                        stats = starting_stats(&options, &maps[selected], entry_spawn.as_deref());
                        println!(
                            "Level loadout {} {:?}: {}",
                            maps[selected],
                            entry_spawn,
                            stats.weapon_summary()
                        );
                        interactions.sync(&mut scene.world);
                        environment_clock = 0.;
                        audio.load(&mut assets, &maps[selected], &scene.map);
                        current = selected;
                        campaign = Campaign::default();
                        played = true;
                        pickup_clock = 0.;
                        power_art.clear_effects();
                        pickups = inventory::pickups_for_visit(
                            &scene.map,
                            &maps[current],
                            entry_spawn.as_deref(),
                            &catalog,
                        );
                        (pos, yaw) = interaction::spawn(&scene.map, entry_spawn.as_deref());
                        let spawn = Player::spawn(&scene.world, pos);
                        flying = options.fly || spawn.is_none();
                        spawn_landing = true;
                        player = spawn.unwrap_or_else(|| Player::new(pos - Vec3::Z * EYE_HEIGHT));
                        alice.reset(&player, yaw);
                        follow_camera.reset();
                        camera_handoff.reset();
                        clock.pause();
                        paused = false;
                        pitch = 0.;
                        notice.clear();
                        menu = false;
                        retry.remember(game_snapshot!());
                    }
                    Err(e) => {
                        notice = format!("Could not open level: {e:#}");
                        if console_map {
                            console.print(notice.clone());
                        }
                    }
                }
            }
        } else if !overlay_before
            && !paused
            && !inventory_menu
            && !console_input
            && focused
            && stats.alive()
        {
            if !scripted_view {
                let look = input.look(&preferences);
                yaw -= look.x * dt * 2.5;
                pitch += look.y * dt * 2.;
                let touch_look = input.touch_look(&preferences);
                yaw -= touch_look.x;
                pitch += touch_look.y;
            }
            if !scripted_view && held(KeyCode::Left) {
                yaw += dt * 1.3;
            }
            if !scripted_view && held(KeyCode::Right) {
                yaw -= dt * 1.3;
            }
            if !scripted_view && held(KeyCode::Up) {
                pitch += dt;
            }
            if !scripted_view && held(KeyCode::Down) {
                pitch -= dt;
            }
            pitch = pitch.clamp(-1.5, 1.5);
            let forward = vec3(
                yaw.cos() * pitch.cos(),
                yaw.sin() * pitch.cos(),
                pitch.sin(),
            );
            let right = vec3(yaw.sin(), -yaw.cos(), 0.);
            let mut movement = forward * input.movement().y + right * input.movement().x;
            if held(KeyCode::W) {
                movement += forward;
            }
            if held(KeyCode::S) {
                movement -= forward;
            }
            if held(KeyCode::D) {
                movement += right;
            }
            if held(KeyCode::A) {
                movement -= right;
            }
            if flying {
                player.release_rope();
            }
            if flying && (held(KeyCode::E) || (input.using_touch && held(KeyCode::Space))) {
                movement.z += 1.;
            }
            if flying && (held(KeyCode::Q) || (input.using_touch && held(KeyCode::LeftControl))) {
                movement.z -= 1.;
            }
            let speed = if held(KeyCode::LeftShift) { 800. } else { 220. };
            if flying {
                pos += movement.normalize_or_zero() * speed * dt;
                clock.pause();
            } else {
                // Looking up/down must not change walking speed or produce flight.
                let aim = vec3(
                    yaw.cos() * pitch.cos(),
                    yaw.sin() * pitch.cos(),
                    pitch.sin(),
                );
                use crate::interaction::friendly::{use_owner, UseOwner};
                let shared_rope = player.rope_prompt(&scene.world).is_some();
                let world_rope = interactions
                    .pandemonium
                    .as_ref()
                    .is_some_and(|p| p.prompt(&scene.world, player.eye()).is_some());
                let talk =
                    interactions.conversation(&scene.world, player.eye(), aim, &npcs, &story);
                let owner = use_owner(
                    story.busy(),
                    interactions.scripted(),
                    shared_rope,
                    world_rope,
                    talk.is_some(),
                    interactions
                        .prompt(&scene.world, player.eye(), aim)
                        .is_some(),
                );
                let use_pressed = pressed(KeyCode::E)
                    || (owner == UseOwner::Dialogue
                        && input.using_touch
                        && pointer_pressed
                        && pointer_pos.y > screen_height() * 0.72);
                advance_dialogue = use_pressed && owner == UseOwner::Dialogue;
                let mut events = interaction::Events::default();
                if use_pressed && owner == UseOwner::Talk {
                    if let Some(talk) = talk {
                        if let Some(result) = interactions.start_conversation(talk, &mut story) {
                            npcs.begin_talk(talk.actor());
                            events.merge(result);
                        }
                    }
                }
                events.merge(interactions.update(
                    world_dt,
                    &scene.map,
                    &mut scene.world,
                    &player,
                    aim,
                    use_pressed && owner == UseOwner::World,
                )?);
                if events.damage > 0. {
                    stats.damage(events.damage);
                    hud.announce("Hazard / Sanity lost");
                }
                for event in events.story {
                    story.trigger(&event);
                }
                pending_transition = events.transition.or(pending_transition);
                if let Some(message) = events.message {
                    hud.announce(message);
                }
                if let Some(sound) = events.sound {
                    audio.world_effect(&sound);
                }
                let was_cinematic = interactions.scripted();
                let mover_dt = if interactions.levels.iter().any(|s| s.ctl.ignores_watch()) { dt } else { world_dt };
                interactions.companions_for_movers(&npcs.companions());
                interactions.actors_for_movers(&npcs.targets());
                interactions.advance_school(mover_dt, &scene.map, &mut scene.world, &mut player)?;
                if was_cinematic && !interactions.scripted() {
                    yaw = player.script_facing;
                    pitch = 0.;
                    clock.pause();
                    spawn_landing = true;
                }
                let before_feet = player.feet;
                let before_water = player.immersion;
                let before_drowning = player.breath.hits;
                let before_air_warning = player.breath.warning_age().is_some();
                stats.prepare_player(&mut player);
                let before_grounded = player.grounded;
                // A supported spawn has already landed before its first tick.
                // Its first later fall must receive ordinary fall damage.
                if before_grounded {
                    spawn_landing = false;
                }
                interactions.prepare_player(&mut stats, &mut player);
                let before_jumps = player.jumps;
                let before_landings = player.landings;
                let before_fall = player.velocity.z;
                let flat = vec2(yaw.cos(), yaw.sin());
                let right = vec2(yaw.sin(), -yaw.cos());
                let mut wish = flat * input.movement().y + right * input.movement().x;
                if held(KeyCode::W) {
                    wish += flat;
                }
                if held(KeyCode::S) {
                    wish -= flat;
                }
                if held(KeyCode::D) {
                    wish += right;
                }
                if held(KeyCode::A) {
                    wish -= right;
                }
                let mut control = Controls {
                    use_pressed: use_pressed
                        && matches!(owner, UseOwner::SharedRope | UseOwner::Traversal),
                    wish: wish.clamp_length_max(1.),
                    swim: movement.clamp_length_max(1.),
                    rise: f32::from(held(KeyCode::Space)) - f32::from(held(KeyCode::LeftControl)),
                    jump: !overlay_before && pressed(KeyCode::Space),
                    run: preferences.run(held(KeyCode::LeftShift)),
                };
                interactions.filter_level_controls(&mut control);
                let controlled = interactions
                    .pandemonium
                    .as_mut()
                    .is_some_and(|p| p.control(world_dt, &scene.world, &mut player, control));
                if !controlled
                    && !scripted_view
                    && !interactions.scripted()
                    && !interactions
                        .duchess
                        .as_ref()
                        .is_some_and(|d| d.controlled())
                    && !interactions.school.as_ref().is_some_and(|s| s.cinematic())
                    && !interactions.levels_controlled()
                {
                    if alice.ice_locked() {
                        control = Controls::default();
                        player.velocity = Vec3::ZERO;
                    }
                    if alice.attack_movement_locked() {
                        control = Controls::default();
                    }
                    scene.world.set_weapon_obstacles(alice.ice_targets());
                    clock.advance(elapsed as f64, &scene.world, &mut player, control);
                }
                pos = player.eye();
                audio.movement(
                    (player.feet - before_feet).truncate().length(),
                    player.grounded
                        && before_grounded
                        && player.immersion.level == 0
                        && !preferences.run(held(KeyCode::LeftShift)),
                    player.jumps > before_jumps,
                    player.landings > before_landings && before_fall < -90.,
                );
                audio.water(
                    dt,
                    before_water.level,
                    player.immersion.level,
                    player.velocity.length(),
                );
                if before_water.level < 2 && player.immersion.level >= 2 {
                    hud.announce("Swimming / Space rise / Ctrl dive / look and W to swim");
                }
                let liquid_damage = std::mem::take(&mut player.liquid_damage);
                stats.damage(liquid_damage);
                if player.breath.hits != before_drowning {
                    audio.world_effect("sound/character/alice/choke1.wav");
                }
                if !before_air_warning && player.breath.warning_age().is_some()
                    && player.immersion.level == 3 && stats.alive() {
                    audio.world_effect("sound/character/alice/choke2.wav");
                }
                if liquid_damage > 0. && before_water.kind != player.immersion.kind {
                    hud.announce("Harmful liquid / Sanity lost");
                }
                if player.landings > before_landings {
                    let lost = if spawn_landing {
                        0.
                    } else {
                        stats.fall(player.landing_speed)
                    };
                    spawn_landing = false;
                    if lost > 0. {
                        hud.announce(format!("Hard landing / -{lost:.0} Sanity"));
                    }
                }
                hints.sync(&interactions);
                hints.observe(before_feet, player.feet);
                let events = interactions.triggers(dt, before_feet, player.feet);
                for event in &events.story {
                    story.trigger(event);
                }
                if events.damage > 0. {
                    stats.damage(events.damage);
                    hud.announce("Hazard / Sanity lost");
                }
                if let Some(message) = events.message {
                    hud.announce(message);
                }
                if let Some(sound) = events.sound {
                    audio.world_effect(&sound);
                }
                if stats.alive() {
                    pending_transition = events.transition.or(pending_transition);
                    if let Some((feet, angle)) = events.teleport {
                        if scene.world.body_clear(feet) {
                            player = Player::new(feet);
                            yaw = angle;
                            pos = player.eye();
                            alice.reset(&player, yaw);
                            follow_camera.reset();
                            camera_handoff.reset();
                            clock.pause();
                            spawn_landing = true;
                            recovery.clear();
                            hud.announce("Teleported");
                        } else {
                            hud.announce("Teleport destination is obstructed");
                        }
                    }
                }
                if !in_transport && recovery::outside_world(&scene.world, player.feet) {
                    stats.damage(10000.);
                    pending_transition = None;
                    hud.announce("Lost beyond the level / Enter to retry");
                }
                if stats.alive() {
                    let dice_before = stats.copies(6);
                    let rage_before = stats.powers.rage;
                    if let Some(s) = &mut interactions.school {
                        let e = s.collect_secret(&mut stats, player.feet, &scene.world);
                        if let Some(m) = e.message {
                            hud.announce(m);
                        }
                        for event in e.story {
                            story.trigger(&event);
                        }
                    }
                    for message in
                        inventory::collect_with(&mut stats, &pickups, player.feet, &scene.world,
                            |item| power_art.collected(item, pickup_clock))
                    {
                        hud.announce(message);
                    }
                    if stats.copies(6) > dice_before {
                        story.trigger("dice_cat");
                    }
                    if stats.powers.rage > rage_before {
                        audio.world_effect("sound/item/ragebox/ragebox_pickup.wav");
                    }
                    if player.script_motion == 0 {
                        recovery.observe(&scene.world, &interactions, &player, yaw);
                    }
                }
            }
        } else {
            clock.pause();
        }
        let story_active = !paused
            && !menu
            && !inventory_menu
            && !console_input
            && focused
            && !flying
            && stats.alive();
        hints.sync(&interactions);
        if (summon_cat || (story_active && pressed(KeyCode::C))) && !in_transport {
            let outcome = if !stats.alive() || flying {
                Err("Return to walking and retry before summoning Cheshire.")
            } else {
                hints.summon(&mut story, &scene.world, player.feet, yaw, player.swimming)
            };
            match outcome {
                Ok(()) => {
                    if hints.visible() {
                        audio.world_effect("sound/character/cheshire_cat/appear.wav");
                    }
                    console.open = false;
                }
                Err(message) => {
                    hud.announce(message);
                    if summon_cat {
                        console.print(message);
                    }
                }
            }
        }
        interactions.sync_pickups(&stats, &mut pickups, &mut story);
        if let Some(b) = &mut interactions.beyond {
            b.bind_rage_player(&mut player, alice.facing());
        }
        let story_ready = interactions.prepare_story(&mut story);
        story.update(
            if story_active && story_ready && hints.prepare_story(&story) {
                elapsed
            } else {
                0.
            },
            advance_dialogue,
            &mut assets,
            &mut audio,
        );
        interactions.sync_cinematic_story(&story);
        hints.sync(&interactions);
        if hints.update(if story_active { dt } else { 0. }, story.hint_active()) {
            audio.world_effect("sound/character/cheshire_cat/disappear.wav");
        }
        for event in story.take_completed() {
            let effects = interactions.completed_dialogue(&event);
            if let Some(message) = effects.message {
                hud.announce(message);
            }
            if let Some(sound) = effects.sound {
                audio.world_effect(&sound);
            }
        }
        if let Some(s) = &mut interactions.school2 {
            s.summon(alice.ally_target());
            s.notarget(
                stats.ignores_alice()
                    || interactions.school.as_ref().is_some_and(|s| s.cinematic()),
            );
            let (events, feedback) = s.update(
                if story_active { world_dt } else { 0. },
                &scene.world,
                player.feet,
                story.busy(),
            );
            stats.school_items = s.quest.items.clone();
            for &hit in &feedback.summon_hits {
                alice.hit_summon(hit);
            }
            stats.damage(feedback.damage);
            if feedback.will_drain > 0. {
                stats.spend_will(feedback.will_drain.min(stats.will()));
            }
            player.knockback(feedback.impulse);
            for event in events.story {
                story.trigger(&event);
            }
            audio.feedback(&feedback);
            if let Some(sound) = events.sound {
                audio.world_effect(&sound);
            }
            if let Some(message) = events.message {
                hud.announce(message);
            }
        }
        if let Some(s) = &mut interactions.school {
            s.sync_inventory(&mut stats);
        }
        if let Some(d) = &mut interactions.duchess {
            if story_active && stats.alive() {
                if let Some(target) = d.target() {
                    d.threatened(alice.threatens(&scene.world, target));
                }
            }
            let before_stage = d.state.stage;
            d.state.notarget = stats.ignores_alice();
            d.state.opponents.summon = alice.ally_target();
            let f = d.update(
                if story_active { world_dt } else { 0. },
                &scene.world,
                &mut player,
                &mut stats,
                &mut story,
            );
            for &hit in &f.summon_hits {
                alice.hit_summon(hit);
            }
            stats.damage(f.damage);
            player.knockback(f.impulse);
            audio.feedback(&f);
            if before_stage == crate::duchess::Stage::Waiting && d.cinematic() {
                recovery.clear();
            }
            if in_transport && !d.cinematic() {
                yaw = player.script_facing;
                pitch = 0.;
                clock.pause();
                spawn_landing = true;
            }
            pos = player.eye();
        }
        if story_active && saves_enabled && interactions.levels.iter().any(|s| s.ctl.checkpoint_requested()) {
            match store.write_finale_checkpoint(&game_snapshot!()) {
                Ok(()) => {
                    for slot in &mut interactions.levels { slot.ctl.checkpoint_written(); }
                    retry.remember(game_snapshot!());
                    hud.announce("Checkpoint saved");
                }
                Err(e) => { hud.announce(&format!("Checkpoint save failed: {e:#}")); }
            }
        }
        if story_active && interactions.levels.iter().any(|s| s.ctl.ending_ready()) && !post_game {
            post_game = true;
            campaign.completed.insert(save::visit_key(&maps[current], entry_spawn.as_deref()));
            if saves_enabled { if let Err(e) = store.mark_complete() { eprintln!("Completion marker failed: {e:#}"); } }
            mouse_look.release();
            input.suppress();
            match crate::movie::play(&mut assets, &mut audio, &ui, &mut input, &preferences, "ending").await {
                Ok(false) => break,
                Err(e) => eprintln!("Ending film failed: {e:#}"),
                Ok(true) => (),
            }
            escape_menu.post_game = true;
            escape_menu.open(crate::menu::Page::Credits, &preferences, audio.settings);
            escape_menu.message = "Campaign complete / Continue replays the final encounter".into();
            clock.pause();
            continue;
        }
        if let Some(exit) = pending_transition.take() {
            story.defer_exit(exit);
        }
        if story_active {
            pending_transition = interactions.take_story_exit(&mut story);
        }
        if let Some(exit) = pending_transition {
            options.difficulty = stats.difficulty;
            let (name, target_spawn) = exit.clone();
            let leaving = level_snapshot!();
            // The shared transition (`campaign::arrive`) decides everything the headless routes
            // also decide. The exit is all or nothing, so it works on copies of the resources and
            // the ledger until the destination's window half has loaded too.
            let (mut next_stats, mut next_campaign) = (stats.clone(), campaign.clone());
            let arrived = crate::campaign::arrive(
                &mut assets,
                &mut next_stats,
                &mut next_campaign,
                leaving,
                &exit,
                false,
            )
            .and_then(|visit| {
                let crate::campaign::NextVisit {
                    map,
                    entry,
                    cached,
                    loaded,
                    player,
                    eye,
                    yaw,
                    ..
                } = visit;
                let next = finish_entered(
                    &mut assets,
                    loaded,
                    &map,
                    entry.as_deref(),
                    cached.as_ref(),
                    &options,
                )?;
                Ok((next, player, eye, yaw))
            });
            match arrived {
                Ok((next, arrived_player, arrived_eye, arrived_yaw)) => {
                    stats = next_stats;
                    campaign = next_campaign;
                    scene = next.scene;
                    hints = next.hints;
                    steam = next.steam;
                    npcs = next.npcs;
                    interactions = next.interactions;
                    story = next.story;
                    environment_clock = next.environment_clock;
                    pickup_clock = next.pickup_clock;
                    power_art.clear_effects();
                    current = maps
                        .iter()
                        .position(|m| m == &name)
                        .context("Exit map unavailable")?;
                    selected = current;
                    entry_spawn = target_spawn;
                    println!(
                        "Exit loadout {name} {:?}: {}",
                        entry_spawn,
                        stats.weapon_summary()
                    );
                    recovery.clear();
                    interactions.sync(&mut scene.world);
                    pickups = inventory::pickups_for_visit(
                        &scene.map,
                        &name,
                        entry_spawn.as_deref(),
                        &catalog,
                    );
                    audio.load(&mut assets, &name, &scene.map);
                    // A blocked entrance already failed the exit, so there is no flight fallback.
                    (pos, yaw) = (arrived_eye, arrived_yaw);
                    flying = false;
                    player = arrived_player;
                    alice.reset(&player, yaw);
                    follow_camera.reset();
                    camera_handoff.reset();
                    clock.pause();
                    pitch = 0.;
                    spawn_landing = true;
                    weapon_click = None;
                    weapon_buttons.block();
                    level_art = next.art;
                    skip_scene = crate::cinematic::Skip::default();
                    if options.frames.is_none() {
                        pending_save_slot = Some(Slot::Auto);
                    }
                    retry.remember(game_snapshot!());
                    let visit = crate::campaign::choice_position(&level_choices, current, entry_spawn.as_deref());
                    hud.announce(&level_choices[visit].title);
                }
                Err(e) => {
                    interactions.transition_failed(&exit);
                    hud.announce(format!("Could not enter next map: {e}"));
                }
            }
        }
        if !paused && !menu && !inventory_menu && !console_input && focused && stats.alive() {
            environment_clock += world_dt;
            steam.animate(environment_clock);
            if interactions
                .school
                .as_ref()
                .is_some_and(|s| s.theatre_finished())
            {
                steam.stop();
            }
            steam.sync(&interactions.event_world);
            if let Some(v) = &interactions.village {
                v.place_particles(&mut steam);
            }
            if let Some(p) = &interactions.pandemonium {
                p.gate_particles(&mut steam);
            }
            interactions.place_level_particles(&mut steam);
            steam.update_clocks(dt, world_dt, pos, &scene.world);
        }
        // Scene controllers may finish during movement, dialogue or boss updates.
        // Resolve the handoff once, after all three, before animation/aim/rendering.
        let still_scripted = interactions.scripted()
            || interactions.duchess.as_ref().is_some_and(|d| d.cinematic())
            || interactions
                .school
                .as_ref()
                .and_then(|s| s.return_visit.as_ref())
                .is_some_and(|s| s.camera().is_some());
        if camera_handoff.was_scripted() && !still_scripted {
            if !third_person {
                camera_handoff.skip();
            }
            yaw = player.script_facing;
            pitch = 0.;
            pos = player.eye();
            clock.pause();
            spawn_landing = true;
            weapon_click = None;
            weapon_buttons.block();
            alice.resume_scene(&player, yaw, level_art.handoff_pose());
        }
        let direction = vec3(
            yaw.cos() * pitch.cos(),
            yaw.sin() * pitch.cos(),
            pitch.sin(),
        );
        let active = !paused
            && !menu
            && !inventory_menu
            && !console_input
            && focused
            && !flying
            && stats.alive();
        if active {
            played = true;
            stats.update(dt);
            pickup_clock += dt;
        }
        // Keep conversation participants safe while listening. Exploration continues;
        // enemies and attacks resume after the conversation.
        let pand_cinematic =
            interactions.duchess.as_ref().is_some_and(|d| d.cinematic()) || interactions.scripted();
        if story.busy()
            || pand_cinematic
            || interactions.school.as_ref().is_some_and(|s| s.cinematic())
        {
            weapon_click = None;
            weapon_buttons.block();
            alice.cancel_weapon_action();
            alice.dismiss_summon();
        }
        if flying {
            alice.cancel_weapon_action();
        }
        if interactions.levels.iter().any(|s| s.ctl.dismiss_summons()) {
            alice.dismiss_summon();
        }
        interactions.activate_enemies();
        let scripted_motion = interactions.scripted();
        if let Some(s) = &mut interactions.encounters {
            s.summon(alice.ally_target());
            s.notarget(
                stats.ignores_alice()
                    || interactions.school.as_ref().is_some_and(|s| s.cinematic()),
            );
            let f = s.update(
                if active
                    && !story.busy()
                    && !interactions.school.as_ref().is_some_and(|s| s.cinematic())
                    && !scripted_motion
                {
                    world_dt
                } else {
                    0.
                },
                &scene.world,
                player.eye(),
            );
            for &hit in &f.summon_hits {
                alice.hit_summon(hit);
            }
            stats.damage(f.damage);
            player.knockback(f.impulse);
            audio.feedback(&f);
        }
        if interactions.has_level_combat() {
            // Pool's scene-owned ants and registered controllers share the route's combat hook.
            let notarget = stats.ignores_alice();
            let summon = alice.ally_target();
            let dt = if active
                && !story.busy()
                && !interactions.school.as_ref().is_some_and(|s| s.cinematic())
                && !scripted_motion
            {
                if interactions.levels.iter().any(|s| s.ctl.ignores_watch()) { dt } else { world_dt }
            } else {
                0.
            };
            let threatens = |t: &crate::combat::Target| alice.threatens(&scene.world, *t);
            let f = interactions.levels_step(&mut crate::level::Combat {
                dt,
                world: &scene.world,
                player: &mut player,
                stats: &mut stats,
                story: &mut story,
                notarget,
                summon,
                threatens: &threatens,
            });
            for &hit in &f.summon_hits {
                alice.hit_summon(hit);
            }
            stats.damage(f.damage);
            player.knockback(f.impulse);
            audio.feedback(&f);
        }
        npcs.summon(alice.ally_target());
        npcs.activate_levels(&interactions.levels);
        npcs.notarget(stats.ignores_alice());
        if active && !story.busy() && !interactions.scripted() && world_dt > 0. {
            npcs.chess_threats(|t| alice.threatens(&scene.world, *t));
        }
        npcs.level_patrols(&interactions.levels);
        let feedback = npcs.update(
            if active
                && !story.busy()
                && !interactions.school.as_ref().is_some_and(|s| s.cinematic())
                && !interactions.scripted()
            {
                world_dt
            } else {
                0.
            },
            &scene.world,
            player.eye(),
        );
        alice.story_pose(&story, if story_active { dt } else { 0. });
        npcs.story_pose(
            &story,
            if active { world_dt } else { 0. },
            &scene.world,
            player.eye(),
        );
        if let Some(v) = &interactions.village {
            v.cinema.apply_npcs(&mut npcs);
        }
        for &hit in &feedback.summon_hits {
            alice.hit_summon(hit);
        }
        stats.damage(feedback.damage);
        if feedback.will_drain > 0. {
            stats.spend_will(feedback.will_drain.min(stats.will()));
        }
        player.knockback(feedback.impulse);
        if feedback.damage > 0. {
            println!(
                "Guard struck Alice: {} damage, {} sanity",
                feedback.damage,
                stats.sanity()
            );
            hud.announce("Card guard struck you");
        }
        audio.feedback(&feedback);
        if player.script_motion > 0 || player.ledge.is_some() || interactions.level_blocks_weapons() {
            weapon_click = None;
            weapon_buttons.block();
        }
        if !stats.alive() {
            alice.die();
        }
        let death_animation =
            !stats.alive() && focused && !paused && !menu && !inventory_menu && !console_input;
        let mut targets = if story.busy() {
            Vec::new()
        } else {
            npcs.targets()
        };
        if !story.busy() {
            if stats.equipped() == Some(2) || alice.croquet_contacts() {
                targets.extend(npcs.mallet_contacts());
            }
            if let Some(t) = interactions.duchess.as_ref().and_then(|d| d.target()) {
                targets.push(t);
            }
            if let Some(summon) = alice.summon_target() {
                targets.push(summon);
            }
            targets.extend(interactions.shot_targets());
            if let Some(s) = &interactions.encounters {
                targets.extend(s.targets());
            }
            if let Some(s) = &interactions.school2 {
                targets.extend(s.targets());
            }
            targets.extend(interactions.levels_targets());
        }
        targets.extend(alice.ice_targets());
        alice.world_time(dt, world_dt);
        alice.notarget(stats.ignores_alice());
        let weapon_sounds = alice.update_funded(
            if (active && !pand_cinematic) || death_animation {
                dt
            } else {
                0.
            },
            &player,
            preferences.run(held(KeyCode::LeftShift)),
            character::WeaponInput {
                dice: stats.copies(6),
                first_person: !third_person && !flying,
                selected: stats.equipped().unwrap_or(crate::weapons::UNARMED),
                click: weapon_click,
                aim: direction,
            },
            &crate::combat::Context {
                world: &scene.world,
                targets: &targets,
            },
            Some(&mut stats),
        );
        if let Some(message) = alice.weapon_notice.take() {
            hud.announce(message);
        }
        for mut hit in alice.take_hits() {
            if alice.hit_ice(hit) {
                continue;
            }
            let hit_origin = targets
                .iter()
                .find(|t| t.id == hit.id)
                .map_or(player.eye(), |t| t.center);
            if hit.id == crate::dice::ALICE {
                stats.damage(hit.damage);
                player.knockback(hit.knockback * 2.5);
                continue;
            }
            if hit.id == crate::dice::SUMMON {
                alice.hit_summon(hit);
                continue;
            }
            if hit.kind.is_demon() {
                npcs.provoke_summon(hit.id);
                if let Some(s) = &mut interactions.encounters {
                    s.provoke_summon(hit.id);
                }
                if let Some(s) = &mut interactions.school2 {
                    s.provoke_summon(hit.id);
                }
                interactions.levels_provoke_summon(hit.id);
                if hit.id == crate::duchess::ID {
                    if let Some(d) = &mut interactions.duchess {
                        d.state.opponents.demon = true;
                    }
                }
                hit.kind = hit.kind.means();
            } else {
                hit.damage = stats.attack_damage(hit.damage);
            }
            if hit.id == crate::duchess::ID {
                // A projectile still in flight after Alice dies must not lock retry in a defeat cinematic.
                if !stats.alive() {
                    continue;
                }
                if let Some(sound) = interactions
                    .duchess
                    .as_mut()
                    .and_then(|d| d.hit(hit.damage))
                {
                    audio.world_effect_at(sound, hit_origin);
                }
                continue;
            }
            // Each registration's exact hit range comes before the `>= encounters::BASE`
            // catch-all below, which would otherwise swallow every registry id.
            if let Some(sound) = interactions.hit_level(hit) {
                if let Some(sound) = sound {
                    audio.world_effect_at(sound, hit_origin);
                }
                continue;
            }
            if hit.id >= crate::encounters::BASE {
                if let Some(sound) = interactions.encounters.as_mut().and_then(|s| s.hit(hit)) {
                    audio.world_effect_at(sound, hit_origin);
                }
                continue;
            }
            if hit.id >= crate::interaction::SHOT_BASE {
                let e = interactions.shoot(hit);
                if let Some(message) = e.message {
                    hud.announce(message);
                }
                if let Some(sound) = e.sound {
                    audio.world_effect_at(&sound, hit_origin);
                }
                interactions.sync(&mut scene.world);
                continue;
            }
            if hit.id >= crate::school2::ENEMY_BASE {
                if let Some(sound) = interactions
                    .school2
                    .as_mut()
                    .and_then(|s| s.hit_attack(hit))
                {
                    audio.world_effect_at(sound, hit_origin);
                }
                continue;
            }
            if let Some((message, sound)) = npcs.hit(hit) {
                hud.announce(message);
                audio.world_effect_at(sound, hit_origin);
            }
        }
        scene.world.set_weapon_obstacles(alice.ice_targets());
        if active && loot_visit == save::visit_key(&maps[current], entry_spawn.as_deref()) {
            let mut after = interactions.loot_sources();
            after.extend(npcs.loot_sources());
            for message in crate::loot::tick_with(
                &mut stats,
                &loot_visit,
                world_dt,
                &loot_before,
                &after,
                player.feet,
                &scene.world,
                |item| power_art.collected(item, pickup_clock),
            ) {
                hud.announce(message);
                audio.world_effect("sound/item/essence/small/mtespu.wav");
            }
            if let Some(message) =
                pickup_feedback.update(&stats, &pickups, &loot_visit, player.feet, &scene.world)
            {
                hud.announce(message);
            }
        }
        if stats.active_power().map(|(k, _)| k) != power_before {
            if let Some((kind, _)) = stats.active_power() {
                audio.world_effect(match kind {
                    crate::powerups::Kind::Rage => "sound/item/ragebox/ragebox_power.wav",
                    crate::powerups::Kind::Tea => "sound/item/tea/powerup.wav",
                    crate::powerups::Kind::Glass => "sound/item/glass/powerup.wav",
                });
            }
        }
        for sound in weapon_sounds {
            audio.weapon_effect(sound);
        }
        for (path, origin) in alice.take_world_audio() {
            audio.world_effect_at(path, origin);
        }
        audio.world(&mut assets, &interactions, alice.audio_loops());
        for cue in alice.take_audio() {
            audio.animation(cue);
        }
        audio.player_state(
            &mut assets,
            stats.sanity() as f32,
            player.swimming,
            player.immersion.level == 3,
            player.immersion.level == 3 && player.breath.remaining() <= 0.,
        );
        alice.power_appearance(&stats);
        alice.water_appearance(&player);
        let rage_scene = interactions.beyond.as_ref().and_then(|b| b.rage_scene());
        if let Some((time, finished)) = rage_scene {
            alice.rage_scene(time, finished, &player);
        }
        let mut pand_camera = interactions.camera_after(
            interactions
                .duchess
                .as_ref()
                .and_then(|d| d.camera(&scene.world))
                .or_else(|| interactions.pandemonium.as_ref().and_then(|p| p.camera()))
                .or_else(|| interactions.pool.as_ref().and_then(|p| p.camera()))
                .or_else(|| interactions.school.as_ref().and_then(|s| s.scene_camera()))
                .or_else(|| interactions.school2.as_ref().and_then(|s| s.scene_camera()))
                .or_else(|| {
                    interactions
                        .beyond
                        .as_ref()
                        .and_then(|b| b.scene_camera_in(&scene.world))
                })
                .or_else(|| {
                    interactions
                        .fortress
                        .as_ref()
                        .and_then(|f| f.cinema.camera(&f.state.cinema))
                })
                .or_else(|| {
                    interactions
                        .village
                        .as_ref()
                        .and_then(|v| v.cinema.camera())
                }),
            &scene.world,
        );
        if let Some(camera) = pand_camera.filter(|_| {
            interactions
                .pandemonium
                .as_ref()
                .is_some_and(|p| p.rail_camera_active())
        }) {
            pand_camera = Some(
                rail_camera.rail(
                    &scene.world,
                    camera,
                    &interactions
                        .pandemonium
                        .as_ref()
                        .unwrap()
                        .rail_camera_ahead(),
                    if active { dt } else { 0. },
                ),
            );
        } else {
            rail_camera.reset();
        }
        let cinematic_camera = interactions
            .school
            .as_ref()
            .and_then(|s| s.return_visit.as_ref())
            .and_then(|s| s.camera());
        let scripted_camera = pand_camera.or_else(|| {
            cinematic_camera.map(|(eye, target, _)| crate::cinematic::Camera::look(eye, target))
        });
        let (eye, aim, gameplay_alice) = if third_person && !flying {
            follow_camera.update(
                &scene.world,
                player.feet,
                direction,
                preferences.camera_distance,
                if active { dt } else { 0. },
            )
        } else {
            follow_camera.reset();
            (pos, pos + direction, false)
        };
        let fade = interactions
            .fade_after(
                interactions
                    .village
                    .as_ref()
                    .map(|v| v.cinema.fade())
                    .or_else(|| {
                        interactions
                            .fortress
                            .as_ref()
                            .map(|f| f.cinema.fade(&f.state.cinema))
                    })
                    .or_else(|| {
                        interactions
                            .pandemonium
                            .as_ref()
                            .map(|p| (p.fade_color(), p.fade()))
                    })
                    .or_else(|| interactions.pool.as_ref().map(|p| p.fade()))
                    .or_else(|| interactions.beyond.as_ref().map(|b| b.scene_fade()))
                    .or_else(|| interactions.school.as_ref().map(|s| s.scene_fade()))
                    .or_else(|| interactions.school2.as_ref().map(|s| s.scene_fade())),
            )
            .map_or(Color::new(0., 0., 0., 0.), |(c, a)| Color { a, ..c });
        if scripted_camera.is_some() {
            camera_handoff.return_duration(
                if interactions
                    .pool
                    .as_ref()
                    .is_some_and(|p| p.scene_id() == Some(crate::pool::cinema::ENTRY))
                {
                    2.
                } else {
                    0.45
                },
            );
        }
        let view = camera_handoff.update(
            &scene.world,
            scripted_camera,
            crate::cinematic::Camera::look(eye, aim),
            fade,
            if active { dt } else { 0. },
        );
        let quake = interactions
            .pool
            .as_ref()
            .map_or(Vec3::ZERO, |p| p.quake_offset())
            + interactions.levels.iter().map(|s| s.ctl.quake_offset()).sum::<Vec3>();
        let (camera, target) = (view.eye + quake, view.target + quake);
        let show_alice = !interactions.level_hides_player() && if scripted_camera.is_some() {
            rage_scene.is_some_and(|(_, finished)| !finished)
                || (cinematic_camera.is_some_and(|(_, _, show)| show) && pand_camera.is_none())
        } else {
            gameplay_alice && camera.distance(player.feet + Vec3::Z * 32.) > 30.
        };
        audio.prepare(
            &mut assets,
            &scene.world,
            camera,
            scene.world.liquid_at(camera) != 0,
            elapsed,
        );
        let listen_direction = target - camera;
        audio.update_clocks(
            elapsed,
            world_dt,
            camera,
            listen_direction.y.atan2(listen_direction.x),
            console_input || menu || paused || inventory_menu || !focused,
            !story_active,
        );
        scene.atmosphere.liquid =
            crate::water::Liquid::from_contents(scene.world.liquid_at(camera)).tint();
        interactions.presentation.apply(&mut scene);
        if let Some(origin) = interactions.levels.iter().find_map(|s| s.ctl.sky_origin()) { scene.set_sky_origin(origin); }
        let (mut mirror_extra, mut mirror_hide) = (Vec::new(), Vec::new());
        for slot in &interactions.levels { let (extra, hide) = slot.ctl.reflection(); mirror_extra.extend(extra); mirror_hide.extend(hide); }
        scene.set_reflection(mirror_extra, mirror_hide);
        scene.set_reflection_actor(interactions.levels.iter().find_map(|s| s.ctl.reflection_actor()));
        if let Some(fog) = interactions.levels.iter().find_map(|s| s.ctl.scene_fog()) {
            scene.atmosphere.distance = fog;
        }
        if let Some(distance) = interactions.levels.iter().find_map(|s| s.ctl.fog_distance()) {
            scene.atmosphere.distance.w = distance;
        }
        clear_background(scene.atmosphere.background());
        let world_camera = Camera3D {
            position: camera,
            target,
            up: pand_camera.map_or(Vec3::Z, |c| c.up),
            fovy: interactions
                .beyond
                .as_ref()
                .and_then(|b| b.scene_fovy(screen_width() / screen_height()))
                .or_else(|| interactions.levels.iter().find_map(|s| s.ctl.scene_fovy(screen_width() / screen_height())))
                .unwrap_or(75.0f32.to_radians()),
            z_near: 2.,
            z_far: 30000.,
            ..Default::default()
        };
        set_camera(&world_camera);
        let mut transforms = interactions.transforms();
        transforms.extend(scene.world.traversal.transforms(&player));
        let rope_hand = interactions
            .pandemonium
            .as_ref()
            .and_then(|p| p.rope_hand())
            .or_else(|| {
                player
                    .rope
                    .as_ref()
                    .and_then(|g| scene.world.traversal.rope(g.id))
                    .map(|r| (r.model, player.feet + Vec3::Z * 40.))
            });
        scene.update_ropes(world_dt, rope_hand, &transforms);
        steam.collected(&pickups, &stats);
        if paused || menu || inventory_menu || console_input || !focused || !stats.alive() {
            steam.animate(environment_clock);
            steam.sync(&interactions.event_world);
            if interactions
                .school
                .as_ref()
                .is_some_and(|s| s.theatre_finished())
            {
                steam.stop();
            }
            if let Some(v) = &interactions.village {
                v.place_particles(&mut steam);
            }
            if let Some(p) = &interactions.pandemonium {
                p.gate_particles(&mut steam);
            }
            interactions.place_level_particles(&mut steam);
        }
        let mut lights = steam.lights();
        lights.extend(alice.lights());
        lights.extend(npcs.lights());
        for slot in &interactions.levels { lights.extend(slot.ctl.lights()); }
        scene.prepare_camera_portals(&world_camera, environment_clock, fullbright, &transforms);
        crate::lighting::select(lights, camera, &scene.world);
        crate::render_fx::begin_view(
            &world_camera,
            environment_clock,
            &scene.atmosphere,
            fullbright,
        );
        // If a save is pending, flag the preview grabber so the next update()
        // (called right after the world finishes drawing this frame) captures a
        // fresh screenshot instead of using a stale one or returning None on
        // Android where per-frame capture is throttled.
        if saves_enabled && pending_save_slot.is_some() {
            save_frame.request();
        }
        scene.draw(camera, environment_clock, fullbright, false, &transforms);
        if show_alice && stats.invisible <= 0. {
            crate::lighting::shadow(&scene.world, player.feet, 22.);
        }
        alice.atmosphere(&scene.atmosphere, camera);
        if show_alice && !interactions.school.as_ref().is_some_and(|s| s.cinematic()) {
            alice.draw_ghost(player.feet, fullbright, stats.invisible > 0.);
        }
        npcs.draw(
            camera,
            (target - camera).normalize_or_zero(),
            &scene.atmosphere,
            fullbright,
        );
        level_art.story_pose(&story);
        level_art.draw(&interactions, &hints, &scene.atmosphere, camera, fullbright);
        power_art.draw(
            &pickups,
            &stats,
            &save::visit_key(&maps[current], entry_spawn.as_deref()),
            &scene.atmosphere,
            camera,
            pickup_clock,
        );
        {
            scene.draw_with_particles(
                camera,
                (target - camera).normalize_or_zero(),
                environment_clock,
                fullbright,
                &transforms,
                &steam,
            );
            if let Some(s) = &interactions.encounters {
                s.draw_waves();
            }
            if let Some(s) = &interactions.school2 {
                for b in &s.boojums {
                    b.draw_waves();
                }
            }
            if let (Some(p), Some(art)) = (&interactions.pandemonium, &mut level_art.pandemonium) {
                art.draw_effects(p, camera, &scene.atmosphere);
            }
            if let (Some(d), Some(art)) = (&interactions.duchess, &mut level_art.duchess) {
                art.effects(d, camera, &scene.atmosphere);
            }
            level_art.effects(&interactions, camera, &scene.atmosphere);
            alice.draw_effects(camera, fullbright, show_alice);
            let can_aim = !interactions.level_blocks_weapons() && !flying
                && !pand_cinematic
                && scripted_camera.is_none()
                && !story.busy()
                && !menu
                && !inventory_menu
                && !console.open
                && stats.alive()
                && player.script_motion == 0
                && player.ledge.is_none()
                && !player.climbing();
            aim_pointer.draw(
                &crate::combat::Context {
                    world: &scene.world,
                    targets: &targets,
                },
                crate::targeting::Sight {
                    eye: player.eye(),
                    direction,
                    weapon: can_aim.then(|| stats.equipped()).flatten(),
                },
                if active { dt } else { 0. },
                camera,
                &scene.atmosphere,
            );
            hud.draw_pickups(&pickups, &stats, camera, pickup_clock);
        }
        crate::lighting::flares(camera, (target - camera).normalize_or_zero());
        crate::render_fx::finish();
        if !third_person
            && !pand_cinematic
            && !flying
            && stats.alive()
            && player.script_motion == 0
            && !interactions.school.as_ref().is_some_and(|s| s.cinematic())
        {
            crate::render::clear_view_depth();
            alice.draw_first_person(fullbright);
        }
        if saves_enabled {
            save_frame.update();
        }
        set_default_camera();
        if active {
            use crate::interaction::friendly::{use_owner, UseOwner};
            let rope = player.rope_prompt(&scene.world);
            let world_rope = interactions
                .pandemonium
                .as_ref()
                .is_some_and(|p| p.prompt(&scene.world, player.eye()).is_some());
            let talk =
                interactions.conversation(&scene.world, player.eye(), direction, &npcs, &story);
            let world = interactions.prompt(&scene.world, player.eye(), direction);
            let owner = use_owner(
                story.busy(),
                interactions.scripted(),
                rope.is_some(),
                world_rope,
                talk.is_some(),
                world.is_some(),
            );
            let prompt = match owner {
                UseOwner::Talk => talk.map(|c| c.prompt()),
                UseOwner::SharedRope => rope.map(str::to_owned),
                UseOwner::World => world.map(str::to_owned),
                _ => None,
            };
            if let Some(prompt) = prompt {
                let prompt = prompt
                    .replace("E  ", &format!("{}  ", input.label(&preferences, "E")))
                    .replace("E /", &format!("{} /", input.label(&preferences, "E")));
                ui.toast(&prompt, screen_height() - 112.);
            }
        }
        let help = get_time() < help_until;
        if player.immersion.level == 3
            && stats.alive()
            && !menu
            && !inventory_menu
            && !console_input
        {
            crate::hud::air(&player.breath, &ui);
        }
        let objective = interactions
            .school
            .as_ref()
            .filter(|_| help)
            .map(|s| s.objective())
            .or_else(|| {
                interactions
                    .school2
                    .as_ref()
                    .filter(|_| help)
                    .map(|s| s.quest.objective())
            })
            .or_else(|| {
                interactions
                    .pandemonium
                    .as_ref()
                    .filter(|_| help)
                    .map(|p| p.objective())
            })
            .or_else(|| {
                interactions
                    .fortress
                    .as_ref()
                    .filter(|_| help)
                    .map(|f| f.objective())
            })
            .or_else(|| {
                interactions
                    .pool
                    .as_ref()
                    .filter(|_| help)
                    .map(|p| p.objective())
            })
            .or_else(|| interactions.levels_objective().filter(|_| help))
            .or_else(|| help.then(|| story.objective().map(str::to_owned)).flatten());
        let show_details = help || menu;
        if paused && !menu && !inventory_menu && !console_input && !help {
            ui.paused(&if input.using_touch {
                "Tap screen to resume".into()
            } else if input.using_pad {
                "A or Start to resume".into()
            } else {
                format!("{} or click to resume", input.label(&preferences, "P"))
            });
        }
        if help && !menu && !inventory_menu && !console_input {
            let canvas = crate::ui::Canvas::new(screen_width(), screen_height());
            ui.panel(canvas.rect(Rect::new(16., 118., 608., 343.)));
            ui.font.draw(
                canvas,
                "A little guidance",
                Rect::new(38., 133., 564., 32.),
                28.,
                crate::ui::INK,
            );
            if let Some(objective) = &objective {
                let rows = crate::story::wrap(objective, 548., |t| ui.body.width(t, 14.));
                for (i, line) in rows.iter().take(3).enumerate() {
                    ui.body.draw_left(
                        canvas,
                        line,
                        Rect::new(46., 174. + i as f32 * 18., 548., 18.),
                        14.,
                        crate::ui::INK,
                    );
                }
            }
            let movement = if input.using_touch {
                "Left stick: move   Jump/Rise: jump or swim up   Dive: crouch or dive".into()
            } else if input.using_pad {
                format!(
                    "Left stick: move   {}: jump / rise   {}: dive",
                    input.label(&preferences, "Space"),
                    input.label(&preferences, "Ctrl")
                )
            } else {
                preferences.controls_text(if flying {
                    "W A S D move   Q / E down / up   Shift faster"
                } else if player.script_motion == 1 {
                    "W A S D swing   Space / Ctrl climb   E release"
                } else if player.swimming {
                    "W A S D swim   Space rise   Ctrl dive   E use"
                } else {
                    "W A S D move   Space jump   Shift run   E interact"
                })
            };
            let lines = [
                movement,
                format!(
                    "{}: look   {}: change view",
                    if input.using_touch {
                        "Right side drag"
                    } else if input.using_pad {
                        "Right stick"
                    } else {
                        "Mouse / look keys"
                    },
                    input.label(&preferences, "V")
                ),
                format!(
                    "{} / {}: primary / alternate attack (drag to aim)",
                    input.label(&preferences, "Mouse 1"),
                    input.label(&preferences, "Mouse 2")
                ),
                format!(
                    "{} / {}: toy   {}: inventory",
                    input.label(&preferences, "Wheel Up"),
                    input.label(&preferences, "Wheel Down"),
                    input.label(&preferences, "I")
                ),
                format!(
                    "{}: Cheshire hints    {}: advance dialogue",
                    input.label(&preferences, "C"),
                    input.label(&preferences, "E")
                ),
                format!(
                    "{}: recover    Save / Load in the main menu",
                    input.label(&preferences, "R")
                ),
                format!(
                    "{}: menus   {}: chapters   {}: help",
                    if input.using_touch {
                        "Menu"
                    } else if input.using_pad {
                        "Start"
                    } else {
                        "Esc"
                    },
                    input.label(&preferences, "Tab"),
                    input.label(&preferences, "H")
                ),
            ];
            for (i, line) in lines.iter().enumerate() {
                ui.font.draw_left(
                    canvas,
                    line,
                    Rect::new(46., 239. + i as f32 * 27., 548., 24.),
                    21.,
                    crate::ui::INK,
                );
            }
        }
        if menu {
            chapters.draw(
                &ui,
                &level_choices,
                &maps,
                options.difficulty,
                focused && !console_input,
                input.using_pad,
            );
        }
        if (!show_details || inventory_menu) && !interactions.scripted() {
            hud.draw(&stats, &catalog, inventory_menu, &input, &preferences);
            if !paused && !inventory_menu && !console_input && stats.alive() {
                hud.draw_notice();
            }
        }
        if input.using_touch && !menu && !console_input {
            input.touch.draw(&ui, &preferences);
        }
        if !menu && !inventory_menu && !console_input && stats.alive() {
            let cover = camera_handoff.overlay();
            if cover.a > 0. {
                draw_rectangle(0., 0., screen_width(), screen_height(), cover);
            }
            if let Some(f) = &interactions.fortress {
                let (mut color, alpha) = f.cinema.fade(&f.state.cinema);
                color.a = alpha;
                if alpha > 0. {
                    draw_rectangle(0., 0., screen_width(), screen_height(), color);
                }
            }
            if let Some(p) = &interactions.pandemonium {
                let fade = p.fade();
                if fade > 0. {
                    draw_rectangle(
                        0.,
                        0.,
                        screen_width(),
                        screen_height(),
                        Color {
                            a: fade,
                            ..p.fade_color()
                        },
                    );
                }
            }
            if let Some(v) = &interactions.village {
                let (mut color, alpha) = v.cinema.fade();
                color.a = alpha;
                if alpha > 0. {
                    draw_rectangle(0., 0., screen_width(), screen_height(), color);
                }
            }
            if let Some(p) = &interactions.pool {
                let (mut color, alpha) = p.fade();
                color.a = alpha;
                if alpha > 0. {
                    draw_rectangle(0., 0., screen_width(), screen_height(), color);
                }
            }
            if let Some(s) = &interactions.school {
                let (mut color, alpha) = s.scene_fade();
                color.a = alpha;
                if alpha > 0. {
                    draw_rectangle(0., 0., screen_width(), screen_height(), color);
                }
            }
            if let Some(b) = &interactions.beyond {
                let (mut color, alpha) = b.scene_fade();
                color.a = alpha;
                if alpha > 0. {
                    draw_rectangle(0., 0., screen_width(), screen_height(), color);
                }
            }
            if let Some((mut color, alpha)) = interactions.fade_after(None) {
                color.a = alpha;
                if alpha > 0. {
                    draw_rectangle(0., 0., screen_width(), screen_height(), color);
                }
            }
            if preferences.subtitles {
                story.draw(&input.label(&preferences, "E"), &ui);
            }
            if let (Some(d), Some(art)) = (&interactions.duchess, &level_art.duchess) {
                art.hud(d);
            }
            level_art.hud(&interactions);
            if interactions
                .village
                .as_ref()
                .is_some_and(|v| v.cinema.active())
                || interactions
                    .fortress
                    .as_ref()
                    .is_some_and(|f| f.state.cinema.active())
                || interactions
                    .duchess
                    .as_ref()
                    .and_then(|d| d.scene_id())
                    .is_some()
                || interactions
                    .pandemonium
                    .as_ref()
                    .and_then(|p| p.scene_id())
                    .is_some()
                || interactions
                    .beyond
                    .as_ref()
                    .is_some_and(|b| b.scene_id().is_some())
                || interactions.scene_id_after(None).is_some()
                || interactions
                    .pool
                    .as_ref()
                    .is_some_and(|p| p.scene_id().is_some())
                || interactions
                    .school
                    .as_ref()
                    .is_some_and(|s| s.scene_id().is_some())
            {
                skip_scene.draw(
                    &ui,
                    &if input.using_touch {
                        "Screen".into()
                    } else if input.using_pad {
                        "A".into()
                    } else {
                        input.label(&preferences, "Enter")
                    },
                );
            }
        }
        if audio.settings.muted && help && !inventory_menu && !console_input {
            ui.toast("Sound muted - M to restore", 18.);
        }
        if !notice.is_empty() {
            ui.toast(&notice, 74.);
        }
        show_mouse(!focused);
        if focused
            && (menu || inventory_menu || paused)
            && !input.using_touch
            && !crate::android::is_android()
        {
            ui.cursor();
        }
        console.draw(&ui);
        if let Some(slot) = pending_save_slot {
            let save_result: Result<(), String> = if !saves_enabled {
                Err("Saving is unavailable in a staged preview".into())
            } else if !stats.alive() {
                Err("Retry or load a game before saving".into())
            } else {
                store
                    .write_with_preview(
                        slot,
                        &game_snapshot!(),
                        save_frame.preview().as_ref(),
                    )
                    .map_err(|e| format!("{e:#}"))
            };
            match save_result {
                Ok(()) => {
                    retry.remember(game_snapshot!());
                    played = true;
                    let msg = match slot {
                        Slot::Quick => "Game saved / F9 to load",
                        Slot::Auto => "Progress saved",
                        _ => "Game saved",
                    };
                    hud.announce(msg);
                    if slot == Slot::Quick {
                        console.print("Game saved / F9 to load");
                    } else if matches!(slot, Slot::One | Slot::Two | Slot::Three | Slot::Four) {
                        console.print(format!("Game saved to {}", slot.title()));
                    }
                }
                Err(message) => {
                    hud.announce(&format!("Save failed: {message}"));
                    console.print(format!("Save failed: {message}"));
                }
            }
        }
        if !active { frame_profile.cancel(); }
        drop(frame_profile);
        frame += 1;
        if is_key_pressed(KeyCode::F12) {
            save_capture(&std::path::PathBuf::from(format!(
                "private/screenshots/{}-{frame}.png",
                maps[current]
            )))?;
        }
        if options.frames.is_some_and(|limit| frame >= limit) {
            crate::frame_profile::report(&maps[current])?;
            if let Some(path) = &options.capture {
                save_capture(path)?;
            }
            break;
        }
        // NOTE: we intentionally do NOT call input.suppress() here when the
        // overlay state changes. Suppressing here (at END of the same frame
        // where the MENU/MAP press edge was just raised) clears `self.pressed`
        // before the next input.update() even sees the edge, so short taps
        // never register — the player has to hold the button long enough for
        // a second frame of press to leak through. Each transition point that
        // needs to drain the opening finger (escape menu / chapters / inv)
        // suppresses+drain_active_touches explicitly at the right moment.
        next_frame().await;
    }
    if saves_enabled && played && stats.alive() && options.frames.is_none() && !post_game {
        if let Err(e) =
            store.write_with_preview(Slot::Auto, &game_snapshot!(), save_frame.preview().as_ref())
        {
            eprintln!("Automatic save failed: {e:#}");
        }
    }
    println!(
        "World interactions: {} uses, {} exit/teleport contacts",
        interactions.used, interactions.touched
    );
    mouse_look.release();
    println!("NPC greetings: {}", npcs.greetings);
    if options.shelf_preview {
        println!(
            "Shelf preview ended: feet={:?}, grounded={}, body_clear={}, jumps={}",
            player.feet,
            player.grounded,
            scene.world.body_clear(player.feet),
            player.jumps
        );
    }
    println!(
        "Weapon actions: {} emitted, {} impacts",
        alice.visual_counts().0,
        alice.visual_counts().1
    );
    println!(
        "Viewer closed cleanly. Data folder: {}",
        assets.base.display()
    );
    audio.finish()?;
    Ok(())
}
