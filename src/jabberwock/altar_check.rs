//! Check map ambience alongside scene-owned art, as the real viewer draws it.
use super::*;
use crate::{entity::Id, particles::Steam};

async fn audit(a: &mut Assets) -> Result<()> {
    let catalog = crate::inventory::Catalog::load(a)?;
    let maps = a.names().filter(|n| n.starts_with("maps/") && n.ends_with(".bsp"))
        .map(str::to_owned).collect::<Vec<_>>();
    let mut ordinary = 0;
    let mut scripted = Vec::new();
    for file in maps {
        let name = file.trim_start_matches("maps/").trim_end_matches(".bsp");
        let map = Bsp::parse(&a.read(&file)?)?;
        let altars = map.entities.iter().enumerate().filter(|(_, e)| {
            e.get("model").is_some_and(|m| m.trim_start_matches("models/").starts_with("altar_"))
        }).collect::<Vec<_>>();
        if altars.is_empty() { continue; }
        let mut steam = Steam::load(a, &map)?;
        let items = crate::inventory::pickups_for_visit(&map, name, None, &catalog);
        let mut stats = Stats::for_level(name, None);
        steam.collected(&items, &stats);
        for &(id, e) in &altars {
            let key = format!("{name}:{id}");
            if e.get("classname").is_some_and(|c| c.to_ascii_lowercase().starts_with("item_")) {
                let present = items.iter().any(|p| p.id == key);
                ensure!(steam.emission(Id(id)).map(|s| s.0) == Some(present),
                    "Altar does not follow pickup availability: {key}");
                ordinary += 1;
            } else {
                scripted.push((name.to_owned(), id));
            }
        }
        stats.collected.extend(items.iter().map(|p| p.id.clone()));
        steam.collected(&items, &stats);
        for &(id, e) in &altars {
            if e.get("classname").is_some_and(|c| c.to_ascii_lowercase().starts_with("item_")) {
                ensure!(steam.emission(Id(id)) == Some((false, 0)),
                    "Collected weapon left an altar effect: {name}:{id}");
            }
        }
        next_frame().await;
    }
    scripted.sort();
    ensure!(ordinary == 24 && scripted == vec![("hatter2".into(),11), ("jlair2".into(),3), ("wforest".into(),96)],
        "Altar audit coverage changed: {ordinary}, {scripted:?}");

    for (name, id, entry, active) in [
        ("hatter2", 11, None, false),
        ("jlair2", 3, None, false),
        ("wforest", 96, None, true),
        ("wforest", 96, Some("wforest_start2"), false),
    ] {
        let map = Bsp::parse(&a.read(&format!("maps/{name}.bsp"))?)?;
        let mut interactions = crate::interaction::Interactions::load(&map)?;
        interactions.set_entry(a, &map, name, entry)?;
        let mut steam = Steam::load(a, &map)?;
        let world = World::from_bsp(&map)?;
        let at = data::pose(&map.entities[id]).translation;
        let snapshot = interactions.snapshot();
        for restored in [false, true] {
            if restored { interactions.restore(&snapshot, &map)?; }
            for _ in 0..60 {
                steam.sync(&interactions.event_world);
                interactions.place_level_particles(&mut steam);
                steam.update(1. / 60., at, &world);
            }
            let (enabled, count) = steam.emission(Id(id)).context("Missing scripted altar emitter")?;
            ensure!(enabled == active && (active || count == 0),
                "Script-owned altar leaked or disappeared: {name}/{entry:?}, restored={restored}");
        }
        if name == "wforest" && active {
            let mut p = Player::spawn(&world, crate::interaction::spawn(&map, None).0).context("Blocked forest spawn")?;
            p.feet = at + Vec3::Z * 16.;
            let mut stats = Stats::for_level(name, None);
            interactions.prepare_player(&mut stats, &mut p);
            ensure!(stats.staff_component, "Staff pickup fixture missed the altar");
            steam.sync(&interactions.event_world);
            interactions.place_level_particles(&mut steam);
            ensure!(steam.emission(Id(id)).is_some_and(|s| !s.0), "Staff altar kept emitting after pickup");
        }
    }
    println!("PASS all 27 placed weapon altars: pickup availability/collection; three scripted owners; forest return and restored visibility");
    Ok(())
}

pub(super) async fn render(a: &mut Assets) -> Result<()> {
    audit(a).await?;
    std::fs::create_dir_all("private/jabberwock/captures")?;
    let mut scene = crate::render::Scene::load(a, "jlair2")?;
    let mut o = Encounter::load(a, &scene.map, Kind::Lair)?;
    let mut art = art::Art::load(a, &o)?;
    let interactions = crate::interaction::Interactions::load(&scene.map)?;
    let id = o.data.eye_altar.context("Missing Eye altar")?;
    let at = o.data.points["eye_altar"].translation;
    let camera = crate::cinematic::Camera::look(at + vec3(-460., -260., 170.), at + Vec3::Z * 105.);
    let cam = Camera3D { position: camera.eye, target: camera.target, up: Vec3::Z,
        fovy: 65_f32.to_radians(), z_near: 2., z_far: 20000., ..Default::default() };
    for (label, phase, t, collected, visible, gate) in [
        ("before-ambient", Phase::Fight, 0., false, false, false),
        ("intro", Phase::Intro, 0., false, false, true),
        ("fight", Phase::Fight, 0., false, false, true),
        ("before-reveal", Phase::Outro, 4.54, false, false, true),
        ("reveal", Phase::Outro, 4.55, false, true, true),
        ("reward", Phase::Reward, 0., false, true, true),
        ("collected", Phase::Reward, 0., true, false, true),
    ] {
        o.saved.phase = phase; o.saved.time = t; o.saved.eye = collected; o.saved.clock = 2.;
        ensure!(o.eye_visible() == visible, "Wrong Eye reward visibility: {label}");
        let mut steam = Steam::load(a, &scene.map)?;
        for _ in 0..120 {
            steam.sync(&interactions.event_world);
            if gate { o.particles(&mut steam); }
            steam.update(1. / 60., camera.eye, &scene.world);
        }
        let state = steam.emission(id).unwrap();
        ensure!(if gate { state == (false,0) } else { state.0 && state.1 > 0 },
            "Placed altar emission differs: {label}: {state:?}");
        for frame in 0..3 {
            clear_background(BLACK); set_camera(&cam);
            crate::lighting::select(steam.lights(), camera.eye, &scene.world);
            crate::render_fx::begin_view(&cam, 2., &scene.atmosphere, false);
            scene.draw(camera.eye, 2., false, false, &o.transforms());
            art.draw(&o, &scene.atmosphere, camera.eye, false);
            crate::render::depth_read_only(|| {
                scene.draw_with_particles(camera.eye, (camera.target-camera.eye).normalize(), 2., false, &o.transforms(), &steam);
                art.effects(&o, camera.eye, &scene.atmosphere);
            });
            let (_, dropped) = crate::render_fx::finish(); ensure!(dropped == 0, "Altar render overflow");
            set_default_camera();
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!("private/jabberwock/captures/jlair2-altar-{label}.png")))?;
            }
            next_frame().await;
        }
    }
    println!("PASS Eye altar world particles and scene art: hidden during intro/fight, one reward at reveal, absent after collection");
    Ok(())
}
