//! Scene commitments, save continuation and native cast/camera fixtures.
use super::*;
use crate::{interaction::Interactions, story::Story};
use anyhow::ensure;
pub(crate) fn setup(
    assets: &mut Assets,
    map: &Bsp,
    name: &str,
) -> Result<(Interactions, World, Player, Story)> {
    let mut i = Interactions::load(map)?;
    i.set_entry(assets, map, "skool1", None)?;
    let mut world = World::from_bsp(map)?;
    i.sync(&mut world);
    let mut player = Player::spawn(&world, vec3(-2560., 2368., -480.))
        .context("School scene fixture blocked")?;
    let mut story = Story::load(assets, "skool1");
    if name == cinema::BOOK_WIN {
        let s = i.school.as_mut().unwrap();
        s.theatre = true;
        s.library = true;
        player = Player::spawn(&world, vec3(940., 3324., -256.)).context("Book reveal fixture blocked")?;
        for at in [vec3(-212.,3820.,704.), vec3(-468.,3690.,384.), vec3(884.,3960.,64.), player.feet] {
            i.triggers(0.01, at, at);
        }
        return Ok((i, world, player, story));
    }
    if name == cinema::SHELF {
        i.school.as_mut().unwrap().theatre = true;
        i.completed_dialogue(cinema::THEATRE);
    }
    if matches!(
        name,
        cinema::library::SHELVES | cinema::library::BOOK | cinema::library::RECIPE
    ) {
        let s = i.school.as_mut().unwrap();
        s.theatre = true;
        s.library = true;
        let feet = if name == cinema::library::SHELVES {
            vec3(56., 3496., -256.)
        } else if name == cinema::library::BOOK {
            vec3(204., 2450., 704.)
        } else {
            vec3(244., 2684., -288.)
        };
        if name != cinema::library::SHELVES {
            s.books = [Book::Bridge; 4];
            for n in 0..4 {
                let points = s.paths[n].clone();
                s.follow_book(n, &points, 1., 0.);
                let o = s.object_mut(&format!("flyingsupport{}", n + 1));
                o.pose = o.motion.as_ref().unwrap().keys.last().unwrap().1;
                o.motion = None;
                o.collider = Collider::model(map, o.model, o.pose.origin, o.pose.rotation(), true)?;
            }
        }
        if name == cinema::library::RECIPE {
            s.recipe = true;
            s.recipe_time = 3.;
        }
        i.sync(&mut world);
        player = Player::spawn(&world, feet).context("Library fixture blocked")?;
    }
    let e = map
        .entities
        .iter()
        .find(|e| e.get("thread").is_some_and(|n| n == name))
        .context("Missing scene trigger")?;
    let model = e["model"].trim_start_matches('*').parse::<usize>()?;
    let at = vector(&e["origin"]).unwrap() + (map.models[model].min + map.models[model].max) * 0.5
        - PLAYER_CENTER;
    for id in i.triggers(0.01, at, at).story {
        story.trigger(&id);
    }
    ensure!(
        i.school.as_ref().unwrap().scene_id() == Some(name),
        "School contact did not start {name}"
    );
    Ok((i, world, player, story))
}
pub(crate) fn tick(
    i: &mut Interactions,
    map: &Bsp,
    world: &mut World,
    p: &mut Player,
    story: &mut Story,
    dt: f32,
) -> Result<()> {
    i.advance_school(dt, map, world, p)?;
    i.update(dt, map, world, p, Vec3::X, false)?;
    if i.prepare_story(story) {
        story.tick(dt, false);
    }
    i.sync_cinematic_story(story);
    for id in story.take_completed() {
        i.completed_dialogue(&id);
    }
    i.activate_enemies();
    Ok(())
}
fn book_win(assets: &mut Assets, map: &Bsp) -> Result<()> {
    for skip in [None, Some(1.), Some(5.)] {
        let mut i = Interactions::load(map)?;
        i.set_entry(assets, map, "skool1", None)?;
        let s = i.school.as_mut().unwrap();
        s.theatre = true;
        s.library = true;
        let mut world = World::from_bsp(map)?;
        i.sync(&mut world);
        let mut p = Player::spawn(&world, vec3(940., 3324., -256.)).context("Fourth book contact blocked")?;
        let mut story = Story::load(assets, "skool1");
        for at in [vec3(-212.,3820.,704.), vec3(-468.,3690.,384.), vec3(884.,3960.,64.), p.feet] {
            i.triggers(0.01, at, at);
        }
        ensure!(i.school.as_ref().unwrap().books == [Book::Flying; 4], "Four real book contacts not bound");
        let home = p.feet;
        for _ in 0..20 { tick(&mut i, map, &mut world, &mut p, &mut story, 1. / 60.)?; }
        ensure!(!i.scripted(), "Book reveal started before opening animation");
        for _ in 0..15 { tick(&mut i, map, &mut world, &mut p, &mut story, 1. / 60.)?; }
        ensure!(i.school.as_ref().unwrap().scene_id() == Some(cinema::BOOK_WIN), "Fourth book did not start reveal");
        let mut skipped = false;
        for frame in 0..800 {
            if i.scripted() {
                if !skipped && skip.is_some_and(|at| frame as f32 / 60. >= at) {
                    skipped = i.skip_cinematic(map, &mut world, &mut p, &mut story)?;
                } else if frame % 79 == 0 {
                    let camera = i.school.as_ref().unwrap().scene_camera().unwrap();
                    let before = serde_json::to_value(i.snapshot())?;
                    i.advance_school(0., map, &mut world, &mut p)?;
                    ensure!(before == serde_json::to_value(i.snapshot())?, "Book reveal advanced while paused");
                    i.restore(&serde_json::from_value(before)?, map)?;
                    i.sync(&mut world);
                    let next = i.school.as_ref().unwrap().scene_camera().unwrap();
                    ensure!(camera.eye.distance(next.eye) < 0.001, "Book reveal camera changed after load");
                }
            }
            tick(&mut i, map, &mut world, &mut p, &mut story, 1. / 60.)?;
        }
        let s = i.school.as_ref().unwrap();
        ensure!(!i.scripted() && s.books == [Book::Bridge; 4] && !s.recipe && world.body_clear(p.feet)
            && p.feet.distance(home) < 1. && i.take_story_exit(&mut story).is_none(), "Book reveal changed recipe gate or landing");
        let before = serde_json::to_value(i.snapshot())?;
        i.restore(&serde_json::from_value(before)?, map)?;
        for n in 1..=4 { i.school.as_mut().unwrap().event(&format!("start_book{n}")); }
        tick(&mut i, map, &mut world, &mut p, &mut story, 1. / 60.)?;
        ensure!(!i.scripted(), "Consumed books replayed reveal after load");
        println!("PASS four-book reveal skip={skip:?}: real contacts, opening delay, pause, restoration, same bridge and no early recipe exit");
    }
    Ok(())
}

pub fn check(assets: &mut Assets, map: &Bsp) -> Result<()> {
    book_win(assets, map)?;
    for name in [cinema::THEATRE, cinema::SHELF] {
        for skip in [
            None,
            Some(0.),
            Some(10.),
            Some(33.35),
            Some(42.7),
            Some(45.),
        ] {
            let (mut i, mut world, mut p, mut story) = setup(assets, map, name)?;
            let home = p.feet;
            let hints = crate::cheshire::Hints::load(assets, map, "skool1")?;
            let paused = serde_json::to_value(i.snapshot())?;
            i.advance_school(0., map, &mut world, &mut p)?;
            ensure!(
                paused == serde_json::to_value(i.snapshot())?,
                "School scene advanced during pause"
            );
            let mut skipped = false;
            for frame in 0..12000 {
                if !skipped && skip.is_some_and(|at| frame as f32 / 60. >= at) {
                    ensure!(
                        i.skip_cinematic(map, &mut world, &mut p, &mut story)?,
                        "School skip rejected"
                    );
                    skipped = true;
                }
                tick(&mut i, map, &mut world, &mut p, &mut story, 1. / 60.)?;
                if [90, 834, 1199, 1365, 1820, 2001, 2554, 2560, 2620, 2750].contains(&frame) {
                    let camera = i.school.as_ref().unwrap().scene_camera();
                    let saved = serde_json::from_value(serde_json::to_value(i.snapshot())?)?;
                    i.restore(&saved, map)?;
                    i.sync(&mut world);
                    p = serde_json::from_value(serde_json::to_value(&p)?)?;
                    p.validate_world(&world)?;
                    story.restore(
                        &serde_json::from_value(serde_json::to_value(story.snapshot())?)?,
                        &hints,
                    )?;
                    if let Some(a) = camera {
                        ensure!(
                            i.school
                                .as_ref()
                                .unwrap()
                                .scene_camera()
                                .is_some_and(|b| a.eye.distance(b.eye) < 0.001),
                            "Saved school camera changed"
                        );
                    }
                }
                if !i.scripted() {
                    break;
                }
            }
            let downstairs = name == cinema::THEATRE;
            ensure!(
                !i.scripted()
                    && world.body_clear(p.feet)
                    && if downstairs {
                        p.grounded
                            && p.feet.truncate().distance(vec2(-3336., 2664.)) < 0.1
                            && p.feet.z < -600.
                            && (p.script_facing.abs() - std::f32::consts::PI).abs() < 0.001
                    } else {
                        p.feet.distance(home) < 1.
                    },
                "School scene handoff failed: {name} at {:?}",
                p.feet
            );
            if name == cinema::THEATRE {
                ensure!(
                    story.completed == 1,
                    "Theatre dialogue completion count changed"
                );
                ensure!(
                    i.encounters
                        .as_ref()
                        .unwrap()
                        .actors
                        .iter()
                        .filter(|a| a.name.starts_with("play_guard"))
                        .all(|a| a.active),
                    "Theatre guards not committed"
                );
                ensure!(
                    !i.school.as_ref().unwrap().library,
                    "Theatre completed the independent shelf puzzle"
                );
                // A consumed reinforcement must not be resurrected by a stale
                // dialogue callback, a completed-scene load or a second skip.
                for a in &mut i.encounters.as_mut().unwrap().actors {
                    if a.name.starts_with("play_guard") {
                        if let crate::encounters::Enemy::Guard(g) = &mut a.enemy {
                            g.hurt(1000.);
                        }
                    }
                }
                let saved = i.snapshot();
                i.restore(&saved, map)?;
                i.sync(&mut world);
                i.completed_dialogue(cinema::THEATRE);
                i.activate_enemies();
                ensure!(
                    serde_json::to_value(&saved)? == serde_json::to_value(i.snapshot())?,
                    "Completed theatre changed after repeated activation/load"
                );
            } else {
                ensure!(
                    i.school.as_ref().unwrap().library,
                    "Shelf access not committed"
                );
            }
            ensure!(
                !i.skip_cinematic(map, &mut world, &mut p, &mut story)?,
                "Completed scene skipped again"
            );
            println!("PASS school scene {name} skip {skip:?}: contacts, pause, save, safe handoff and one-shot commitments");
        }
    }
    // Save between queuing the reinforcement cue and dispatching its event.
    // This boundary must also survive a skip before the next ordinary update.
    for skip in [false, true] {
        let (mut i, mut world, mut p, mut story) = setup(assets, map, cinema::THEATRE)?;
        for _ in 0..2555 {
            tick(&mut i, map, &mut world, &mut p, &mut story, 1. / 60.)?;
        }
        i.advance_school(1. / 30., map, &mut world, &mut p)?;
        ensure!(
            i.school
                .as_ref()
                .unwrap()
                .first_cinema
                .as_ref()
                .unwrap()
                .completion_pending,
            "Reinforcement save fixture missed pending cue"
        );
        ensure!(
            i.encounters
                .as_ref()
                .unwrap()
                .actors
                .iter()
                .filter(|a| a.name.starts_with("play_guard"))
                .all(|a| !a.active),
            "Reinforcements appeared before cue dispatch"
        );
        let saved = serde_json::from_value(serde_json::to_value(i.snapshot())?)?;
        i.restore(&saved, map)?;
        i.sync(&mut world);
        if skip {
            i.skip_cinematic(map, &mut world, &mut p, &mut story)?;
        }
        tick(&mut i, map, &mut world, &mut p, &mut story, 1. / 60.)?;
        ensure!(
            i.encounters
                .as_ref()
                .unwrap()
                .actors
                .iter()
                .filter(|a| a.name.starts_with("play_guard") && a.active)
                .count()
                == 2,
            "Pending cue lost or duplicated reinforcements"
        );
        ensure!(
            !i.school.as_ref().unwrap().library,
            "Pending theatre cue opened shelf"
        );
        println!("PASS school pending reinforcement cue save, skip {skip}");
    }
    let mut s = School::load(map, false)?;
    s.configure_cinema(assets, map)?;
    s.check_scene_walks()?;
    let mut old = s.snapshot();
    old.theatre = true;
    old.library = true;
    s.restore(&old, map)?;
    ensure!(
        !s.cinematic() && s.event(cinema::THEATRE).is_none() && s.event(cinema::SHELF).is_none(),
        "Legacy completed school scenes replayed"
    );
    let mut returning = School::load(map, true)?;
    returning.configure_cinema(assets, map)?;
    ensure!(
        returning.cinema_data.is_none(),
        "First-school cinema installed on return visit"
    );
    super::library_check::check(assets, map)?;
    check_scenery(assets, map)?;
    Ok(())
}

fn check_scenery(assets: &mut Assets, map: &Bsp) -> Result<()> {
    for returning in [false, true] {
        let mut school = School::load(map, returning)?;
        let original = school.snapshot();
        // Initialized scenery is rebuilt from data on both entries, including old saves.
        school.restore(&original, map)?;
        for (name, offset, visible) in [
            ("theatre_pic1", vec3(628., -20., 0.), true),
            ("theatre_pic2", vec3(628., 36., 0.), true),
            ("theatre_pic3", Vec3::ZERO, false),
            ("theatre_pic4", Vec3::ZERO, true),
        ] {
            let e = map
                .entities
                .iter()
                .find(|e| e.get("targetname").is_some_and(|n| n == name))
                .unwrap();
            let model = e["model"].trim_start_matches('*').parse::<usize>()?;
            let drawn = school.transforms().find(|p| p.0 == model);
            ensure!(
                drawn.is_some() == visible,
                "Theatre picture visibility: {name}"
            );
            if let Some((_, origin, _)) = drawn {
                ensure!(
                    origin.distance(vector(&e["origin"]).unwrap() + offset) < 0.01,
                    "Theatre picture placement: {name}"
                );
                let bounds = &map.models[model];
                let mut center = origin + (bounds.min + bounds.max) * 0.5;
                ensure!(
                    (0..=16).any(|j| {
                        // Bounds include the hanging chains and the air between them.
                        center.z = origin.z + bounds.min.lerp(bounds.max, j as f32 / 16.).z;
                        school.colliders().any(|c| {
                            c.touches(
                                center + Vec3::X * 32.,
                                center - Vec3::X * 32.,
                                Vec3::splat(1.),
                            )
                        })
                    }),
                    "Theatre picture lost collision: {name}"
                );
            }
        }
        ensure!(
            serde_json::to_value(original)? == serde_json::to_value(school.snapshot())?,
            "Derived scenery altered saved puzzle progress"
        );
    }
    for skip in [false, true] {
        let (mut i, mut world, mut p, mut story) = setup(assets, map, cinema::SHELF)?;
        if skip {
            i.skip_cinematic(map, &mut world, &mut p, &mut story)?;
        }
        for frame in 0..3600 {
            tick(&mut i, map, &mut world, &mut p, &mut story, 1. / 60.)?;
            let school = i.school.as_ref().unwrap();
            let shelf = school.object("secret_shelf");
            let book = school.object("shelf_book");
            let (_, drawn, _) = school.transforms().find(|p| p.0 == book.model).unwrap();
            let expected = book.base.origin + shelf.pose.origin - shelf.base.origin
                + Vec3::Y * (book.pose.origin.y - book.base.origin.y).clamp(-16., 0.);
            ensure!(
                drawn.distance(expected) < 0.01,
                "Shelf switch detached at {frame}"
            );
            let collider = Collider::model(map, book.model, drawn, Quat::IDENTITY, true)?;
            let center = drawn + (map.models[book.model].min + map.models[book.model].max) * 0.5;
            for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                let a = book.collider.trace(
                    center + axis * 512.,
                    center - axis * 512.,
                    Vec3::splat(1.),
                );
                let b = collider.trace(center + axis * 512., center - axis * 512., Vec3::splat(1.));
                ensure!(
                    b.fraction < 1.
                        && (a.fraction - b.fraction).abs() < 0.00001
                        && a.start_solid == b.start_solid,
                    "Shelf switch art/collision disagree at {frame}"
                );
            }
            if frame % 71 == 0 {
                let saved = serde_json::from_value(serde_json::to_value(i.snapshot())?)?;
                i.restore(&saved, map)?;
                i.sync(&mut world);
            }
        }
        let school = i.school.as_ref().unwrap();
        let book = school.object("shelf_book");
        let drawn = school.transforms().find(|p| p.0 == book.model).unwrap().1;
        ensure!(
            drawn.distance(book.base.origin + Vec3::Y * 648.) < 0.01,
            "Completed switch did not follow shelf"
        );
        ensure!(
            school.library && !school.recipe,
            "Shelf completion changed recipe gate"
        );
        println!("PASS shelf binding: skip={skip}, repeated old-shape saves, local press and completed art/collision");
    }
    println!("PASS theatre pictures: first/return entries, initialized placement, hidden third picture, unchanged saves");
    Ok(())
}
pub async fn render(assets: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(assets, "skool1")?;
    let specs = texture::read_materials(assets)?;
    let mut art = Art::load(assets, &specs)?;
    let mut enemies = crate::encounters::Art::load(assets)?;
    // Use both presentation paths, as the live viewer does. Testing only the
    // scene puppets misses map NPCs reappearing when a dialogue names them.
    let mut npcs = crate::npc::Npcs::load(assets, &scene.map, "skool1", None, false, false)?;
    let fresh_npcs = npcs.snapshot();
    let mut speaking_cats = std::collections::BTreeSet::new();
    for (name, seconds) in [
        (cinema::BOOK_WIN, 3.),
        (cinema::BOOK_WIN, 6.),
        (cinema::THEATRE, 3.),
        (cinema::THEATRE, 12.),
        (cinema::THEATRE, 15.),
        (cinema::THEATRE, 17.5),
        (cinema::THEATRE, 20.),
        (cinema::THEATRE, 27.),
        (cinema::THEATRE, 35.),
        (cinema::THEATRE, 42.),
        (cinema::THEATRE, 45.5),
        (cinema::THEATRE, 47.8),
        (cinema::THEATRE, 51.),
        (cinema::SHELF, 2.),
        (cinema::SHELF, 7.),
        (cinema::SHELF, 12.),
        (cinema::SHELF, 60.),
        (cinema::library::SHELVES, 5.),
        (cinema::library::SHELVES, 8.5),
        (cinema::library::SHELVES, 11.),
        (cinema::library::SHELVES, 11.6),
        (cinema::library::BOOK, 3.),
        (cinema::library::BOOK, 10.),
        (cinema::library::BOOK, 20.),
        (cinema::library::BOOK, 24.),
        (cinema::library::BOOK, 28.),
        (cinema::library::BOOK, 33.),
        (cinema::library::RECIPE, 3.),
    ] {
        let (mut i, mut world, mut p, mut story) = setup(assets, &scene.map, name)?;
        npcs.restore(&fresh_npcs)?;
        npcs.scene_hidden(&[]);
        for _ in 0..(seconds * 60.) as usize {
            tick(&mut i, &scene.map, &mut world, &mut p, &mut story, 1. / 60.)?;
            npcs.story_pose(&story, 1. / 60., &world, p.eye());
        }
        // Old saves can contain the map actor's active speaking flag and its
        // independent animation clock. Restoring it must not revive a second Cat.
        npcs.restore(&serde_json::from_value(serde_json::to_value(
            npcs.snapshot(),
        )?)?)?;
        for actor in story.cast() {
            if matches!(actor, "shelf_cat" | "book_cat") {
                speaking_cats.insert(actor.to_string());
            }
        }
        let c = i
            .school
            .as_ref()
            .unwrap()
            .scene_camera()
            .unwrap_or(if name == cinema::SHELF {
                crate::cinematic::Camera {
                    eye: vec3(-106., 3926., -170.),
                    target: vec3(-226., 4176., -220.),
                    up: Vec3::Z,
                }
            } else {
                crate::cinematic::Camera {
                    eye: vec3(-3100., 2650., -530.),
                    target: vec3(-3520., 2650., -590.),
                    up: Vec3::Z,
                }
            });
        art.first.story_pose(&story);
        let check_cat = matches!(name, cinema::library::SHELVES | cinema::library::BOOK);
        let mut normal = None;
        for frame in 0..if check_cat { 6 } else { 3 } {
            if frame == 3 {
                npcs.scene_hidden(&["shelf_cat", "book_cat"]);
            }
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: c.eye,
                target: c.target,
                up: c.up,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            });
            crate::render_fx::begin(c.eye, c.target - c.eye, seconds, &scene.atmosphere, false);
            let transforms = i.transforms();
            scene.draw(c.eye, seconds, false, false, &transforms);
            npcs.draw(
                c.eye,
                (c.target - c.eye).normalize_or_zero(),
                &scene.atmosphere,
                false,
            );
            art.draw(i.school.as_ref().unwrap(), false, &scene.atmosphere, c.eye);
            enemies.school_staging = i.school.as_ref().unwrap().scene_id() == Some(cinema::THEATRE);
            enemies.draw(
                i.encounters.as_ref().unwrap(),
                false,
                &scene.atmosphere,
                c.eye,
            );
            crate::render::depth_read_only(|| scene.draw(c.eye, seconds, false, true, &transforms));
            crate::render_fx::finish();
            set_default_camera();
            if frame == 2 {
                normal = Some(get_screen_data().bytes);
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/school-scene-{name}-{seconds}.png"
                )))?;
            }
            if frame == 5 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/school-cat-reference-{name}-{seconds}.png"
                )))?;
                ensure!(
                    normal.as_ref() == Some(&get_screen_data().bytes),
                    "Map Cat was drawn alongside the scene Cat in {name} at {seconds}s"
                );
            }
            next_frame().await;
        }
    }
    ensure!(
        speaking_cats.len() == 2,
        "Duplicate-Cat check did not exercise both talking map actors"
    );
    println!("PASS school Cat ownership: live NPC plus scene rendering matches scene-only Cats, including restored speech and fade-out");
    Ok(())
}
