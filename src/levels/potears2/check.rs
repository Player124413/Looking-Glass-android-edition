use super::scene::Art;
use super::*;
pub(super) fn saved_behavior(
    i: &mut Interactions,
    _: &mut crate::inventory::Stats,
    story: &mut Story,
) -> Result<()> {
    let bill = owner(i)?;
    ensure!(story.take_exit().is_none(), "Bill dialogue opened an exit");
    ensure!(
        !bill.begin_dialogue(story),
        "Bill dialogue replayed after restoration"
    );
    Ok(())
}
pub fn stage(case: &str, assets: &mut Assets, i: &mut Interactions) -> Result<Story> {
    let mut story = Story::load(assets, "potears2");
    ensure!(
        owner(i)?.begin_dialogue(&mut story),
        "Could not start Bill dialogue fixture"
    );
    let line = match case {
        "potears2-dialogue-first" => 0,
        "potears2-dialogue-middle" => 6,
        "potears2-dialogue-last" => 12,
        "potears2-dialogue-read" => 13,
        _ => anyhow::bail!("Unknown Bill fixture"),
    };
    for _ in 0..5000 {
        story.tick(0.05, false);
        if story
            .progress(DIALOGUE)
            .is_some_and(|(n, t)| n == line && t >= 0.5)
        {
            break;
        }
        if !story.busy() {
            break;
        }
    }
    for id in story.take_completed() {
        i.completed_dialogue(&id);
    }
    ensure!(
        if line == 13 {
            !story.busy()
        } else {
            story.progress(DIALOGUE).is_some_and(|(n, _)| n == line)
        },
        "Bill fixture missed requested line"
    );
    Ok(story)
}
pub(super) fn dialogue_check(assets: &mut Assets) -> Result<()> {
    crate::story::registry::validate_animations(assets, &animation_refs())?;
    let map = Bsp::parse(&assets.read("maps/potears2.bsp")?)?;
    let hints = crate::cheshire::Hints::load(assets, &map, "potears2")?;
    // Before this owner, potears2 used the plain BSP interaction program.
    let mut old = Interactions::load(&map)?;
    let feet = vec3(-160., -32., 160.) - crate::collision::PLAYER_CENTER;
    old.triggers(0.01, feet, feet);
    let before = serde_json::to_value(old.snapshot())?;
    ensure!(
        before["triggers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["id"] == 50 && t["reported"] == true),
        "Legacy Bill fixture did not consume the pending trigger"
    );
    let mut upgraded = Interactions::load(&map)?;
    upgraded.set_entry(assets, &map, "potears2", None)?;
    upgraded.restore(&old.snapshot(), &map)?;
    let after = serde_json::to_value(upgraded.snapshot())?;
    ensure!(
        after["triggers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["id"] == 50 && t["reported"] == false && t["fired"] == false),
        "Bill pending trigger migration failed"
    );
    ensure!(
        upgraded.triggers(0.01, feet, feet).story.is_empty(),
        "Migrated pending trigger bypassed the combat gate"
    );
    for hz in [30, 60, 144] {
        for skip in [false, true] {
            let mut i = Interactions::load(&map)?;
            i.set_entry(assets, &map, "potears2", None)?;
            let mut story = stage("potears2-dialogue-first", assets, &mut i)?;
            let mut visited = std::collections::BTreeSet::new();
            let mut restored = std::collections::BTreeSet::new();
            for _ in 0..hz * 240 {
                if let Some((n, time)) = story.progress(DIALOGUE) {
                    visited.insert(n);
                    let (actor, path) = if n % 2 == 0 {
                        (
                            "billthelizard",
                            format!("sound/character/bill/vo/bliz{:03}.wav", n / 2 + 1),
                        )
                    } else {
                        (
                            "alice",
                            format!("sound/character/alice/vo/alcz{}.wav", 2009 + n / 2),
                        )
                    };
                    let line = story.line().context("Missing active Bill line")?;
                    ensure!(
                        line.path == path && line.actor == actor,
                        "Bill line/speaker order changed at {n}"
                    );
                    let silent = if actor == "alice" {
                        "billthelizard"
                    } else {
                        "fakeplayer"
                    };
                    ensure!(
                        story.mouth(&[silent]) == 0.,
                        "Speech leaked to the listener"
                    );
                    if time > 0.6 && restored.insert(n) {
                        let before = serde_json::to_value(story.snapshot())?;
                        let mouth = [story.mouth(&["alice"]), story.mouth(&["billthelizard"])];
                        story.tick(0., true);
                        ensure!(
                            before == serde_json::to_value(story.snapshot())?,
                            "Paused Bill line moved"
                        );
                        let mut copy = Story::load(assets, "potears2");
                        copy.restore(&serde_json::from_value(before)?, &hints)?;
                        ensure!(
                            mouth == [copy.mouth(&["fakeplayer"]), copy.mouth(&["billthelizard"])],
                            "Restored speaker/lip clock differs"
                        );
                        story = copy;
                        let saved = owner(&mut i)?.snapshot();
                        owner(&mut i)?.restore(&saved, &map)?;
                    }
                }
                story.tick(1. / hz as f32, false);
                if skip && visited.len() == 7 {
                    story.finish_sequence(DIALOGUE);
                }
                for id in story.take_completed() {
                    i.completed_dialogue(&id);
                }
                if !story.busy() {
                    break;
                }
            }
            ensure!(
                !story.busy() && story.completed == 1,
                "Bill dialogue did not finish exactly once"
            );
            ensure!(
                skip || (visited.len() == 13 && restored.len() == 13),
                "Bill fixture omitted a line/restore"
            );
            ensure!(
                owner(&mut i)?.saved.dialogue == Dialogue::Read,
                "Owner missed dialogue completion"
            );
            ensure!(
                !owner(&mut i)?.begin_dialogue(&mut story),
                "Bill dialogue duplicated"
            );
            ensure!(
                story.take_exit().is_none(),
                "Dialogue completed the full scene"
            );
            let gates = owner(&mut i)?;
            let t = TriggerInfo {
                id: crate::entity::Id(50),
                name: "",
                class: TriggerClass::Script,
                thread: DIALOGUE,
                exit: None,
                target: None,
            };
            ensure!(
                !gates.gate(&t).unwrap().test(&Default::default()),
                "Unimplemented combat gate released"
            );
        }
    }
    println!("PASS Bill C3: 13 lines, owner callbacks, pause/restore/skip, explicit animation fallback; dialogue component alone never releases the exit");
    Ok(())
}

pub(super) fn render(assets: &mut Assets) -> crate::levels::BoxFuture<'_> {
    Box::pin(async move {
        let mut scene = crate::render::Scene::load(assets, "potears2")?;
        let mut art = Art::load(assets)?;
        for case in ["approach", "first", "middle", "last", "doors", "suction"] {
            let mut i = Interactions::load(&scene.map)?;
            i.set_entry(assets, &scene.map, "potears2", None)?;
            let mut world = World::from_bsp(&scene.map)?;
            let mut player = Player::spawn(&world, vec3(-160., -32., 128.))
                .context("House render position blocked")?;
            let story = stage_scene(
                &format!("potears2-scene-{case}"),
                assets,
                &scene.map,
                &mut i,
                &mut world,
                &mut player,
            )?;
            art.story_pose(&story);
            let owner = owner(&mut i)?;
            let camera = owner.scene_camera().context("Bill scene has no camera")?;
            let poses = owner.transforms();
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
                scene.draw(camera.eye, 0., false, false, &poses);
                art.draw(owner, &scene.atmosphere, camera.eye, false);
                crate::render::depth_read_only(|| scene.draw(camera.eye, 0., false, true, &poses));
                set_default_camera();
                draw_text(
                    "Staged Bill scene and door sequence",
                    16.,
                    28.,
                    20.,
                    WHITE,
                );
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/hollow-hideaway/{case}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        Ok(())
    })
}
pub fn stage_scene(
    case: &str,
    assets: &mut Assets,
    map: &Bsp,
    i: &mut Interactions,
    world: &mut World,
    player: &mut Player,
) -> Result<Story> {
    i.sync(world);
    *player = Player::spawn(world, vec3(-160., -32., 128.)).context("House fixture obstructed")?;
    let mut story = Story::load(assets, "potears2");
    if case.ends_with("guards") {
        return Ok(story);
    }
    defeat(i)?;
    for _ in 0..121 {
        tick(i, map, world, player, &mut story, 1. / 60.)?;
    }
    i.triggers(0.01, player.feet, player.feet);
    ensure!(i.scripted(), "House fixture failed real trigger");
    for _ in 0..60 * 240 {
        let s = &owner(i)?.saved.scene;
        let stop = match case {
            "potears2-scene-approach" => s.phase == Phase::Approach && s.time >= 0.8,
            "potears2-scene-first" => {
                s.phase == Phase::Conversation && s.line == 0 && s.line_time >= 0.7
            }
            "potears2-scene-middle" => {
                s.phase == Phase::Conversation && s.line == 6 && s.line_time >= 0.7
            }
            "potears2-scene-last" => {
                s.phase == Phase::Conversation && s.line == 12 && s.line_time >= 0.7
            }
            "potears2-scene-doors" => s.ending.as_ref().is_some_and(|e| e.time >= 4.6),
            "potears2-scene-suction" => s.ending.as_ref().is_some_and(|e| e.time >= 6.3),
            "potears2-scene-done" => s.phase == Phase::Done,
            _ => anyhow::bail!("Unknown house fixture {case}"),
        };
        if stop {
            return Ok(story);
        }
        tick(i, map, world, player, &mut story, 1. / 60.)?;
    }
    anyhow::bail!("House fixture never reached {case}")
}

fn tick(
    i: &mut Interactions,
    map: &Bsp,
    world: &mut World,
    player: &mut Player,
    story: &mut Story,
    dt: f32,
) -> Result<Events> {
    i.advance_school(dt, map, world, player)?;
    let events = i.update(dt, map, world, player, Vec3::Y, false)?;
    if i.prepare_story(story) {
        story.tick(dt, false);
    }
    i.sync_cinematic_story(story);
    for name in story.take_completed() {
        i.completed_dialogue(&name);
    }
    Ok(events)
}
fn defeat(i: &mut Interactions) -> Result<()> {
    let ids = i.levels_targets();
    ensure!(ids.len() == 2, "Wrong house guard targets");
    for t in ids {
        i.hit_level(Hit {
            id: t.id,
            damage: 160.,
            kind: crate::combat::DamageKind::Knife,
            knockback: Vec3::ZERO,
        });
    }
    ensure!(
        i.levels_targets().is_empty(),
        "Dead guards still targetable"
    );
    Ok(())
}
pub(super) fn check(assets: &mut Assets) -> Result<()> {
    dialogue_check(assets)?;
    let mut map = Bsp::parse(&assets.read("maps/potears2.bsp")?)?;
    let hints = crate::cheshire::Hints::load(assets, &map, "potears2")?;
    for difficulty in [
        crate::powerups::Difficulty::Easy,
        crate::powerups::Difficulty::Normal,
        crate::powerups::Difficulty::Hard,
        crate::powerups::Difficulty::Nightmare,
    ] {
        map.difficulty = difficulty;
        let mut i = Interactions::load(&map)?;
        i.set_entry(assets, &map, "potears2", None)?;
        let guards = &owner(&mut i)?.saved.guards;
        let ids: Vec<_> = guards.iter().map(|g| g.id).collect();
        ensure!(
            ids == if matches!(
                difficulty,
                crate::powerups::Difficulty::Easy | crate::powerups::Difficulty::Normal
            ) {
                vec![71, 337]
            } else {
                vec![5, 554]
            },
            "Difficulty guard selection differs"
        );
        println!("  {:?} antguards {:?}", difficulty, ids);
    }
    map.difficulty = crate::powerups::Difficulty::Normal;
    // Actual retained C3 saves exercise the old event-program signature as well as
    // active and cached controller migration. Nothing is rewritten in place.
    for name in ["first", "middle", "last", "read"] {
        let dir = std::path::PathBuf::from(format!("private/hollow-hideaway/c3-v1-{name}"));
        if !dir.join("quick.json").exists() {
            continue;
        }
        let old = crate::save::Store::new(dir, assets.fingerprint()?)
            .read(crate::save::Slot::Quick)?
            .game;
        let before = serde_json::to_value(old.level()?.story.clone())?;
        let mut upgraded = crate::save::rebuild_headless(assets, old.clone())?;
        ensure!(
            before == serde_json::to_value(upgraded.logic.story.snapshot())?,
            "C3 line progress changed"
        );
        let o = owner(&mut upgraded.logic.interactions)?;
        ensure!(
            !o.ready() && o.saved.guards.iter().all(|g| g.health > 0.),
            "C3 dialogue bypassed guard gate"
        );
        ensure!(
            o.saved.dialogue
                == if name == "read" {
                    Dialogue::Read
                } else {
                    Dialogue::Playing
                },
            "C3 dialogue status changed"
        );
        println!("  migrated actual C3 v1 {name}");
    }
    // A staged house combat approach uses real Blade projectiles and the normal
    // route simulation. The fixture does not claim transport from the entrance.
    let mut route = crate::route::Route::new(assets, "potears2", None)?;
    route.player = Player::spawn(&route.world, vec3(-160., -440., 128.))
        .context("House combat fixture blocked")?;
    for _ in 0..120 * 45 {
        let targets = route.interactions.levels_targets();
        if targets.is_empty() {
            break;
        }
        route.aim_at = targets
            .iter()
            .min_by(|a, b| {
                a.center
                    .distance_squared(route.player.eye())
                    .total_cmp(&b.center.distance_squared(route.player.eye()))
            })
            .map(|t| t.id);
        route.tick(crate::movement::Controls::default())?;
        if !route.stats.alive() {
            break;
        }
    }
    println!(
        "  staged Blade combat: shots={} damage={} remaining={} alive={}",
        route.shots,
        route.damage,
        route.interactions.levels_targets().len(),
        route.stats.alive()
    );
    ensure!(
        route.interactions.levels_targets().is_empty() && route.stats.alive(),
        "Staged real house fight failed"
    );
    for hz in [30, 60, 144] {
        for skip_at in [None, Some(0.2), Some(15.), Some(60.)] {
            let mut i = Interactions::load(&map)?;
            i.set_entry(assets, &map, "potears2", None)?;
            let mut world = World::from_bsp(&map)?;
            i.sync(&mut world);
            let at = vec3(-160., -32., 128.);
            let mut p = Player::spawn(&world, at).context("House contact obstructed")?;
            let mut story = Story::load(assets, "potears2");
            i.triggers(0.01, p.feet, p.feet);
            ensure!(!i.scripted(), "Live guards allowed ending");
            defeat(&mut i)?;
            for _ in 0..hz {
                tick(&mut i, &map, &mut world, &mut p, &mut story, 1. / hz as f32)?;
            }
            i.triggers(0.01, p.feet, p.feet);
            ensure!(!i.scripted(), "Guard delay shorter than two seconds");
            for _ in 0..hz + 1 {
                tick(&mut i, &map, &mut world, &mut p, &mut story, 1. / hz as f32)?;
            }
            i.triggers(0.01, p.feet, p.feet);
            ensure!(i.scripted(), "Real house trigger failed after guard deaths");
            let mut lines = std::collections::BTreeSet::new();
            let mut checkpoints = std::collections::BTreeSet::new();
            let mut exit = None;
            let mut skipped = false;
            for frame in 0..hz * 240 {
                let time = frame as f32 / hz as f32;
                if !skipped && skip_at.is_some_and(|t| time >= t) {
                    ensure!(
                        i.skip_cinematic(&map, &mut world, &mut p, &mut story)?,
                        "Active Bill skip refused"
                    );
                    skipped = true;
                }
                if let Some((n, t)) = story.progress(DIALOGUE) {
                    if t > 0.2 {
                        lines.insert(n);
                    }
                }
                let scene = &owner(&mut i)?.saved.scene;
                let phase = format!("{:?}-{}", scene.phase, scene.line);
                if checkpoints.insert(phase) {
                    let saved = i.snapshot();
                    let before = serde_json::to_value(&saved)?;
                    let st = story.snapshot();
                    tick(&mut i, &map, &mut world, &mut p, &mut story, 0.)?;
                    ensure!(
                        before == serde_json::to_value(i.snapshot())?,
                        "Paused house moved"
                    );
                    i.restore(&saved, &map)?;
                    story.restore(&st, &hints)?;
                    i.sync(&mut world);
                }
                let e = tick(&mut i, &map, &mut world, &mut p, &mut story, 1. / hz as f32)?;
                if e.transition.is_some() {
                    exit = e.transition;
                    break;
                }
            }
            ensure!(
                exit.as_ref().is_some_and(|e| EXIT.matches(e)),
                "Bill ending failed at {hz} Hz"
            );
            ensure!(
                skip_at.is_some() || lines.len() == 13,
                "Watched scene omitted dialogue"
            );
            ensure!(
                story.has_seen(DIALOGUE) && story.completed == 1,
                "Watch/skip committed different dialogue outcomes"
            );
            ensure!(
                owner(&mut i)?.saved.scene.phase == Phase::Done
                    && owner(&mut i)?
                        .door_poses()
                        .iter()
                        .all(|(_, _, r)| *r == Quat::IDENTITY),
                "Ending did not reset doors"
            );
            ensure!(
                !i.skip_cinematic(&map, &mut world, &mut p, &mut story)?,
                "Completed scene skipped twice"
            );
            ensure!(
                i.update(0.01, &map, &mut world, &p, Vec3::Y, false)?
                    .transition
                    .is_none(),
                "Exit delivered twice"
            );
            i.transition_failed(&EXIT.destination());
            for _ in 0..hz + 1 {
                tick(&mut i, &map, &mut world, &mut p, &mut story, 1. / hz as f32)?;
            }
            // A fresh process must retry the durable exit, regardless of its old delivery latch.
            let saved = i.snapshot();
            i.restore(&saved, &map)?;
            ensure!(
                i.update(0.01, &map, &mut world, &p, Vec3::Y, false)?
                    .transition
                    .is_some(),
                "Restored committed exit lost"
            );
            println!(
                "  Bill staged watch/skip {hz}Hz {:?}: {} lines, {} restored phases",
                skip_at,
                lines.len(),
                checkpoints.len()
            );
        }
    }
    println!("PASS staged house scene: actual contact, two guard deaths/delay, 13 lines, camera/cast/door clocks, pause, skip, restore and one-shot Duchess delivery. Transport route is checked separately.");
    Ok(())
}
