//! Real-asset scene contacts, save continuation, skip and native staging probes.
use super::*;
use crate::{assets::Assets, interaction::Interactions, story::Story};

pub(super) fn trigger(map: &Bsp, name: &str) -> Vec3 {
    let e = map
        .entities
        .iter()
        .find(|e| e.get("thread").is_some_and(|n| n == name))
        .unwrap();
    let model: usize = e["model"].trim_start_matches('*').parse().unwrap();
    vector(&e["origin"]).unwrap() + (map.models[model].min + map.models[model].max) * 0.5
        - PLAYER_CENTER
}
pub(super) fn setup(
    assets: &mut Assets,
    map: &Bsp,
    name: &str,
) -> Result<(Interactions, World, Player, Story)> {
    let mut world = World::from_bsp(map)?;
    let mut i = Interactions::load(map)?;
    i.set_entry(assets, map, "potears1", None)?;
    i.sync(&mut world);
    let mut story = Story::load(assets, "potears1");
    let mut player = Player::spawn(&world, crate::interaction::spawn(map, None).0)
        .context("Pool start obstructed")?;
    if name == cinema::ENTRY {
        i.entry_story(&mut story);
    } else {
        // Later scene fixtures inherit the real arrival's world commitment.
        i.entry_story(&mut story);
        i.skip_cinematic(map, &mut world, &mut player, &mut story)?;
        let feet = trigger(map, name);
        player = Player::spawn(&world, feet).unwrap_or_else(|| Player::new(feet));
        let e = i.triggers(0.01, feet, feet);
        for id in e.story {
            story.trigger(&id);
        }
        ensure!(
            i.pool.as_ref().unwrap().scene_id() == Some(name),
            "Pool contact did not start {name}"
        );
    }
    Ok((i, world, player, story))
}
pub(super) fn tick(
    i: &mut Interactions,
    map: &Bsp,
    world: &mut World,
    player: &mut Player,
    story: &mut Story,
    dt: f32,
) -> Result<usize> {
    i.advance_school(dt, map, world, player)?;
    let e = i.update(dt, map, world, player, Vec3::X, false)?;
    if i.prepare_story(story) {
        story.tick(dt, false);
    }
    i.sync_cinematic_story(story);
    for id in story.take_completed() {
        i.completed_dialogue(&id);
    }
    if let Some(exit) = e.transition {
        ensure!(
            exit == ("potears2".into(), Some("potears2_start1".into())),
            "Incorrect Pool exit"
        );
        Ok(1)
    } else {
        Ok(0)
    }
}
pub fn check(assets: &mut Assets, map: &Bsp) -> Result<()> {
    for name in [
        cinema::ENTRY,
        "Tears1_Boulder1",
        "Tears1_Boulder2",
        TALK,
        cinema::EXIT,
    ] {
        for skip_at in [None, Some(0.), Some(2.)] {
            let (mut i, mut world, mut player, mut story) = setup(assets, map, name)?;
            let hints = crate::cheshire::Hints::load(assets, map, "potears1")?;
            let paused = serde_json::to_value(i.snapshot())?;
            i.advance_school(0., map, &mut world, &mut player)?;
            ensure!(
                paused == serde_json::to_value(i.snapshot())?,
                "Pause advanced Pool scene"
            );
            let mut exits = 0;
            let mut skipped = false;
            let mut restored = false;
            for frame in 0..36_000 {
                let dt = 1. / 60.;
                if skip_at.is_some_and(|at| frame as f32 * dt >= at) && !skipped {
                    ensure!(
                        i.skip_cinematic(map, &mut world, &mut player, &mut story)?,
                        "Skip rejected {name}"
                    );
                    skipped = true;
                }
                exits += tick(&mut i, map, &mut world, &mut player, &mut story, dt)?;
                let p = i.pool.as_ref().unwrap();
                if let Some(camera) = p.camera() {
                    ensure!(
                        camera.eye.is_finite()
                            && camera.target.is_finite()
                            && camera.up.is_finite()
                            && camera.eye.distance(camera.target) > 1.,
                        "Invalid Pool camera"
                    );
                }
                if frame == 85 {
                    let before = p.camera();
                    let saved = serde_json::from_value(serde_json::to_value(i.snapshot())?)?;
                    i.restore(&saved, map)?;
                    i.sync(&mut world);
                    player = serde_json::from_value(serde_json::to_value(&player)?)?;
                    player.validate_world(&world)?;
                    story.restore(
                        &serde_json::from_value(serde_json::to_value(story.snapshot())?)?,
                        &hints,
                    )?;
                    if let Some(a) = before {
                        let b = i
                            .pool
                            .as_ref()
                            .unwrap()
                            .camera()
                            .context("Saved camera lost")?;
                        ensure!(
                            a.eye.distance(b.eye) < 0.001 && a.target.distance(b.target) < 0.001,
                            "Saved camera changed"
                        );
                    }
                    restored = true;
                }
                if (name == cinema::EXIT && exits > 0) || !i.scripted() {
                    break;
                }
            }
            ensure!(
                (name == cinema::EXIT && exits == 1)
                    || (name != cinema::EXIT && !i.scripted() && world.body_clear(player.feet)),
                "Pool scene failed to hand off {name}"
            );
            ensure!(
                skip_at == Some(0.) || restored,
                "Save checkpoint not reached"
            );
            if name == TALK {
                ensure!(
                    story.completed == 1 && !story.busy(),
                    "Turtle dialogue repeated or unfinished"
                );
                ensure!(
                    i.pool.as_ref().unwrap().state.talked,
                    "Turtle commitment missing"
                );
                for _ in 0..600 {
                    tick(&mut i, map, &mut world, &mut player, &mut story, 1. / 60.)?;
                }
                ensure!(
                    i.pool.as_ref().unwrap().ready(0),
                    "Turtle did not release first leaf"
                );
            }
            for _ in 0..5 {
                exits += tick(&mut i, map, &mut world, &mut player, &mut story, 1. / 60.)?;
            }
            ensure!(
                exits == usize::from(name == cinema::EXIT),
                "Duplicate scene exit"
            );
            ensure!(
                !i.skip_cinematic(map, &mut world, &mut player, &mut story)?,
                "Completed scene skipped again"
            );
            println!("PASS Pool scene {name}, skip={skip_at:?}: real contact, pause, saved camera/player/dialogue, landing, one-shot commitment");
        }
    }
    // A pre-scene v12 save keeps the current position/transport; old pending scene
    // contacts become usable, while arrival and completed dialogue do not replay.
    let (mut i, mut world, mut player, mut story) = setup(assets, map, "Tears1_Boulder1")?;
    let mut saved = serde_json::to_value(i.snapshot())?;
    saved["pool"].as_object_mut().unwrap().remove("cinema");
    i.restore(&serde_json::from_value(saved)?, map)?;
    i.sync(&mut world);
    let before = player.feet;
    i.entry_story(&mut story);
    ensure!(
        !i.scripted() && player.feet == before,
        "Old Pool save replayed arrival"
    );
    let at = trigger(map, "Tears1_Boulder1");
    i.triggers(0.01, at, at);
    ensure!(i.scripted(), "Old consumed Pool contact not rearmed");
    i.skip_cinematic(map, &mut world, &mut player, &mut story)?;
    let mut invalid = i.pool.as_ref().unwrap().snapshot();
    invalid.cinema.time = -1.;
    ensure!(
        i.pool.as_mut().unwrap().restore(&invalid, map).is_err(),
        "Bad Pool scene timer accepted"
    );
    let mut clean = Interactions::load(map)?;
    clean.set_entry(assets, map, "potears1", None)?;
    let exit = map
        .entities
        .iter()
        .find(|e| e.get("targetname").is_some_and(|s| s == "end_of_level"))
        .unwrap();
    let id: usize = exit["model"].trim_start_matches('*').parse()?;
    let at = vector(&exit["origin"]).unwrap() + (map.models[id].min + map.models[id].max) * 0.5
        - PLAYER_CENTER;
    ensure!(
        clean.triggers(0.01, at, at).transition.is_none(),
        "Exit volume bypassed jump scene"
    );
    println!("PASS Pool legacy consumed contacts, unchanged player, no arrival replay, bad-save rejection and exit volume gate");
    Ok(())
}

pub async fn render(assets: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(assets, "potears1")?;
    let pool = Pool::load(assets, &scene.map)?;
    let mut art = Art::load(assets, &pool)?;
    for (name, mut seconds) in [
        (cinema::ENTRY, 0.),
        (cinema::ENTRY, 4.5),
        (cinema::ENTRY, 6.),
        (cinema::ENTRY, 9.),
        (cinema::ENTRY, 11.5),
        ("Tears1_Boulder1", 1.5),
        ("Tears1_Boulder2", 1.5),
        (TALK, 4.),
        (TALK, 35.),
        (TALK, 75.),
        ("turtle_departure", 0.),
        (cinema::EXIT, 1.5),
        (cinema::EXIT, 0.25),
        (cinema::EXIT, 2.75),
        (cinema::EXIT, 4.75),
    ] {
        let (mut i, mut world, mut player, mut story) = setup(
            assets,
            &scene.map,
            if name == "turtle_departure" {
                TALK
            } else {
                name
            },
        )?;
        if name == "turtle_departure" {
            story.finish_sequence(TALK);
            i.completed_dialogue(TALK);
            seconds = i.pool.as_ref().unwrap().depart_ride() + 2.;
        }
        for _ in 0..(seconds * 60.) as usize {
            tick(
                &mut i,
                &scene.map,
                &mut world,
                &mut player,
                &mut story,
                1. / 60.,
            )?;
        }
        art.cinema.story_pose(&story);
        let camera = i
            .pool
            .as_ref()
            .unwrap()
            .camera()
            .context("Missing Pool render camera")?;
        for frame in 0..3 {
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: camera.eye,
                target: camera.target,
                up: camera.up,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            });
            let poses = i.transforms();
            scene.draw(camera.eye, seconds, false, false, &poses);
            art.draw(
                i.pool.as_ref().unwrap(),
                false,
                &scene.atmosphere,
                camera.eye,
            );
            crate::render::depth_read_only(|| scene.draw(camera.eye, seconds, false, true, &poses));
            set_default_camera();
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/pool-scene-{name}-{seconds}.png"
                )))?;
            }
            next_frame().await;
        }
        println!("PASS Pool rendered {name} at {seconds}");
    }
    Ok(())
}
