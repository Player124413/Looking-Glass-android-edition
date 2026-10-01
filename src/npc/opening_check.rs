//! Opening-map regression audit using shipped maps, dialogue and actor rendering.
//! Staged probes complement the ordinary village/Pandemonium route checks.
use super::*;
use crate::{cheshire::Hints, movement::Player, story::Story, village::cinema};

fn marker(map: &Bsp, name: &str) -> Vec3 {
    map.entities
        .iter()
        .find(|e| e.get("targetname").is_some_and(|s| s == name))
        .and_then(|e| e.get("origin"))
        .and_then(|s| vector(s))
        .unwrap()
}

/// Repeated activation, paused clocks and a serialized mid-line continuation
/// must not introduce another start-to-finish copy of any recording.
pub fn dialogue(assets: &mut Assets) -> Result<()> {
    for map_name in ["gvillage", "pandemonium"] {
        let map = Bsp::parse(&assets.read(&format!("maps/{map_name}.bsp"))?)?;
        let hints = Hints::load(assets, &map, map_name)?;
        for hz in [30, 60, 144] {
            let mut story = Story::load(assets, map_name);
            for event in crate::story::registry::events(map_name) {
                let expected: usize = crate::story::registry::for_map(map_name)
                    .iter()
                    .filter(|b| b.event == event)
                    .map(|b| b.line_count())
                    .sum();
                ensure!(story.trigger(event), "Missing {map_name}/{event}");
                let before = story.completed;
                let mut voices = Vec::new();
                let mut restored = false;
                for frame in 0..hz * 300 {
                    story.trigger(event);
                    if story.tick(1. / hz as f32, false) {
                        voices.push(story.line().context("Missing started line")?.path.clone());
                    }
                    if frame == hz {
                        let progress = story.progress(event).context("Missing restore line")?;
                        let path = story.line().unwrap().path.clone();
                        let snapshot = serde_json::to_vec(&story.snapshot())?;
                        ensure!(!story.tick(0., true), "Paused dialogue advanced");
                        ensure!(
                            snapshot == serde_json::to_vec(&story.snapshot())?,
                            "Pause changed dialogue"
                        );
                        story.restore(&serde_json::from_slice(&snapshot)?, &hints)?;
                        ensure!(
                            story.progress(event) == Some(progress),
                            "Restored line restarted"
                        );
                        // Audio resumes the interrupted recording at its saved offset.
                        // This is a resume, not a second initial playback.
                        ensure!(
                            story.tick(1. / hz as f32, false) && story.line().unwrap().path == path,
                            "Wrong recording resumed"
                        );
                        ensure!(
                            story.progress(event).unwrap().1 >= progress.1,
                            "Voice offset reset"
                        );
                        restored = true;
                    }
                    if !story.busy() {
                        break;
                    }
                }
                ensure!(
                    restored && !story.busy() && voices.len() == expected,
                    "{map_name}/{event}: {} starts, expected {expected}",
                    voices.len()
                );
                ensure!(
                    voices.iter().collect::<BTreeSet<_>>().len() == expected,
                    "Repeated recording in {map_name}/{event}"
                );
                ensure!(
                    story.completed == before + 1 && story.take_completed() == vec![event],
                    "Duplicate completion {map_name}/{event}"
                );
                story.trigger(event);
                ensure!(!story.busy(), "Completed trigger restarted {event}");
                println!("PASS opening dialogue {map_name}/{event} at {hz} Hz: {expected} unique lines, one completion, pause/restore/recontact");
            }
        }
    }
    Ok(())
}

async fn actor_pixels(npcs: &mut Npcs, name: &str, label: &str) -> Result<usize> {
    let all = npcs.actors.clone();
    let a = all
        .iter()
        .find(|a| a.spawn.name == name)
        .context("Missing cast probe")?
        .clone();
    let target = a.target();
    let eye = target + vec3(145., -180., 70.);
    npcs.actors = vec![a];
    clear_background(BLACK);
    set_camera(&Camera3D {
        position: eye,
        target,
        up: Vec3::Z,
        fovy: 60_f32.to_radians(),
        z_near: 2.,
        z_far: 3000.,
        ..Default::default()
    });
    let before = get_screen_data();
    npcs.draw(
        eye,
        (target - eye).normalize(),
        &Atmosphere::default(),
        true,
    );
    let after = crate::character::visibility_image();
    let pixels = crate::character::visible_pixels(&before, &after);
    after.export_png(&format!("private/opening-audit/{label}.png"));
    npcs.actors = all;
    next_frame().await;
    Ok(pixels)
}

pub async fn check(assets: &mut Assets) -> Result<()> {
    dialogue(assets)?;
    std::fs::create_dir_all("private/opening-audit")?;
    let map = Bsp::parse(&assets.read("maps/gvillage.bsp")?)?;
    let world = World::from_bsp(&map)?;
    let hints = Hints::load(assets, &map, "gvillage")?;
    let mut npcs = Npcs::load(assets, &map, "gvillage", None, false, false)?;
    npcs.update(0., &world, Vec3::splat(100000.));
    let old = npcs.snapshot();
    let mut cinema = cinema::Cinema::load(assets, &map)?;
    cinema.apply_npcs(&mut npcs);
    for name in ["essence_cat", "bridge_cat"] {
        ensure!(
            actor_pixels(&mut npcs, name, name).await? == 0,
            "Offstage {name} visible"
        );
        // Demonstrate that this probe sees the reported defect when ownership is absent.
        npcs.owns_village = false;
        ensure!(
            actor_pixels(&mut npcs, name, &format!("before-{name}")).await? > 100,
            "Probe cannot see old offstage placement"
        );
        npcs.owns_village = true;
    }
    let mut story = Story::load(assets, "gvillage");
    story.trigger("Torchgnome3_Dialog_part2");
    story.tick(1. / 60., false);
    cinema.sync_story(&story);
    npcs.story_pose(&story, 1. / 60., &world, Vec3::ZERO);
    cinema.apply_npcs(&mut npcs);
    ensure!(
        actor_pixels(&mut npcs, "cat_shrink1", "gnome3-no-early-cat").await? == 0,
        "Third Gnome's Cat appeared before its shot"
    );
    npcs.owns_village = false;
    ensure!(
        actor_pixels(&mut npcs, "cat_shrink1", "before-early-cat").await? > 100,
        "Early-Cat probe cannot detect old cast-wide visibility"
    );
    npcs.owns_village = true;
    println!("PASS village offstage actors: measured zero pixels; old ownership reproduces all three unwanted Cats");

    for (event, actor, point) in [
        ("Climb_Cat_Thread", "climb_cat", "climb_cat_node"),
        ("Exit_Cat_Thread", "exit_cat", "exit_cat_node"),
    ] {
        npcs.restore(&old)?;
        let mut cinema = cinema::Cinema::load(assets, &map)?;
        let mut story = Story::load(assets, "gvillage");
        let mut player = Player::new(Vec3::ZERO);
        cinema.skip(&mut player, &world, &mut story)?;
        cinema.apply_npcs(&mut npcs);
        story.trigger(event);
        story.tick(0.01, false);
        npcs.story_pose(&story, 0.01, &world, player.eye());
        let index = npcs
            .actors
            .iter()
            .position(|a| a.spawn.name == actor)
            .unwrap();
        ensure!(
            npcs.actors[index].position() == marker(&map, point),
            "Hint at storage location"
        );
        ensure!(
            npcs.actors[index].spawn == old.actors[index].spawn,
            "Changed saved hint identity"
        );
        ensure!(
            actor_pixels(&mut npcs, actor, actor).await? > 100,
            "Hint hidden during its speech"
        );
        npcs.restore(&old)?; // An older save retains the old runtime footing.
        cinema.apply_npcs(&mut npcs);
        ensure!(
            npcs.actors[index].position() == marker(&map, point),
            "Old save retained wrong hint position"
        );
        story.finish_sequence(event);
        npcs.story_pose(&story, 0.01, &world, player.eye());
        ensure!(
            actor_pixels(&mut npcs, actor, &format!("after-{actor}")).await? == 0,
            "Completed hint remained visible"
        );
    }

    for (event, actor, point) in [
        ("Torchgnome1_Dialog", "torchgnome2", "torchgnome2_pos1"),
        ("Torchgnome2_Dialog", "torchgnome1", "torchgnome1_talk"),
        (
            "Torchgnome3_Dialog_part2",
            "torchgnome3",
            "torchgnome3_talk",
        ),
        ("Torchgnome4_Dialog", "torchgnome4", "torchgnome4_talk"),
    ] {
        for skip in [false, true] {
            npcs.restore(&old)?;
            let mut cinema = cinema::Cinema::load(assets, &map)?;
            let mut story = Story::load(assets, "gvillage");
            let mut player = Player::new(Vec3::ZERO);
            story.trigger(event);
            for frame in 0..12000 {
                cinema.advance(1. / 60., &mut player, &world)?;
                if cinema.prepare_story(&mut story) {
                    story.tick(1. / 60., false);
                }
                cinema.sync_story(&story);
                npcs.story_pose(&story, 1. / 60., &world, player.eye());
                for id in story.take_completed() {
                    cinema.completed(&id);
                }
                if skip && frame == 5 {
                    cinema.skip(&mut player, &world, &mut story)?;
                }
                cinema.apply_npcs(&mut npcs);
                if !cinema.active() && !story.busy() {
                    break;
                }
            }
            ensure!(cinema.state.done.contains(event), "Scene did not finish");
            let index = npcs
                .actors
                .iter()
                .position(|a| a.spawn.name == actor)
                .unwrap();
            let feet = npcs.actors[index].position();
            let node = marker(&map, point);
            // Authored path markers sit above the floor. The scene
            // correctly settles feet onto support instead of keeping that offset.
            ensure!(
                feet.truncate().distance(node.truncate()) < 0.1
                    && (node.z - 80.1..=node.z + 0.1).contains(&feet.z),
                "Gnome snapped back: {actor}: {feet:?} versus {node:?}"
            );
            let support = world.sweep(
                feet + Vec3::Z * 24.,
                feet + Vec3::Z * 23.,
                vec3(8., 8., 24.),
            );
            ensure!(
                !support.start_solid && support.fraction < 1. && support.normal.z >= 0.65,
                "Gnome handoff lacks floor support: {actor}"
            );
            // Use the actual drawn target and reachable close exception, independent of camera.
            let target = npcs.actors[index].target();
            ensure!(
                npcs.talk_target(actor, &world, target, Vec3::X).is_none(),
                "An immediate extra E can replay {event}"
            );
            let saved = npcs.snapshot();
            npcs.restore(&serde_json::from_slice(&serde_json::to_vec(&saved)?)?)?;
            story.restore(&story.snapshot(), &hints)?;
            cinema.restore(Some(&cinema.state.clone()))?;
            cinema.apply_npcs(&mut npcs);
            ensure!(
                npcs.actors[index].spawn == old.actors[index].spawn
                    && npcs.actors[index].cooldown > 0.,
                "Save lost replay cooldown or actor identity"
            );
            for _ in 0..90 {
                npcs.update(1. / 60., &world, target);
            }
            cinema.apply_npcs(&mut npcs);
            ensure!(
                npcs.talk_target(actor, &world, target, Vec3::X).is_some(),
                "Deliberate talk unavailable: {actor}"
            );
            npcs.actors[index].yaw += 0.4;
            let facing = npcs.actors[index].yaw;
            cinema.apply_npcs(&mut npcs);
            ensure!(
                npcs.actors[index].yaw == facing,
                "Handoff reset later Gnome facing"
            );
            ensure!(
                actor_pixels(&mut npcs, actor, &format!("{actor}-{skip}")).await? > 100,
                "Gnome disappeared after handoff"
            );
            println!("PASS {actor} skip={skip}: final marker, saved identity, immediate replay blocked, later Talk available");
        }
    }
    // Pandemonium's specialized actor owner already hides the stored duplicates.
    let map = Bsp::parse(&assets.read("maps/pandemonium.bsp")?)?;
    let world = World::from_bsp(&map)?;
    let mut npcs = Npcs::load(assets, &map, "pandemonium", None, false, false)?;
    for name in [
        "elder_gnome1",
        "leave_gnome",
        "airship_gnome",
        "minecart_torchgnome1",
        "minecart_torchgnome2",
        "minecart_torchgnome3",
        "rope_cat",
        "cards_cat",
    ] {
        ensure!(
            actor_pixels(&mut npcs, name, &format!("pand-hidden-{name}")).await? == 0,
            "Stored Pandemonium actor visible: {name}"
        );
    }
    for (event, actor) in [("Rope_Cat_Thread", "rope_cat"), ("cards_cat", "cards_cat")] {
        let mut story = Story::load(assets, "pandemonium");
        story.trigger(event);
        story.tick(0.01, false);
        npcs.story_pose(&story, 0.01, &world, Vec3::ZERO);
        ensure!(
            actor_pixels(&mut npcs, actor, &format!("pand-active-{actor}")).await? > 100,
            "Pandemonium hint missing"
        );
        story.finish_sequence(event);
        npcs.story_pose(&story, 0.01, &world, Vec3::ZERO);
        ensure!(
            actor_pixels(&mut npcs, actor, &format!("pand-finished-{actor}")).await? == 0,
            "Pandemonium hint remained visible"
        );
    }
    println!("PASS opening actor audit: offstage cast, authored hint/gnome locations, watched/skipped/reloaded handoffs, repeat guard, Pandemonium ownership");
    Ok(())
}
