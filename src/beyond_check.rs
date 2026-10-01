//! Staged state/collision fixtures; traversal proof lives in beyond_route.
use crate::{
    assets::Assets, bsp::Bsp, collision::World, interaction::Interactions, movement::Player,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
pub fn check_rage(assets: &mut Assets) -> Result<()> {
    let mut map = Bsp::parse(&assets.read("maps/fortress2.bsp")?)?;
    let catalog = crate::inventory::Catalog::load(assets)?;
    for difficulty in crate::powerups::Difficulty::ALL {
        for skip_at in [None, Some(0), Some(150), Some(480)] {
            map.difficulty = difficulty;
            let mut items = crate::inventory::pickups(&map, "fortress2", &catalog);
            let item = items
                .iter()
                .find(|p| p.id == "fortress2:37")
                .context("Spawned Rage Box missing")?;
            ensure!(
                item.origin == vec3(-160., 3552., 88.),
                "Rage pickup placement changed"
            );
            let mut i = Interactions::load(&map)?;
            i.set_entry(assets, &map, "fortress2", None)?;
            let mut world = World::from_bsp(&map)?;
            i.sync(&mut world);
            let mut player = Player::spawn(&world, item.origin).context("Rage lift unreachable")?;
            for _ in 0..120 {
                player.tick(&world, Default::default());
            }
            let mut stats = crate::inventory::Stats::default();
            stats.difficulty = difficulty;
            let mut story = crate::story::Story::load(assets, "fortress2");
            let messages = crate::inventory::collect(&mut stats, &items, player.feet, &world);
            ensure!(
                stats.collected.contains("fortress2:37")
                    && stats.powers.rage > 0.
                    && !messages.is_empty(),
                "Rage box cannot be collected from lift {:?}",
                player.feet
            );
            i.sync_pickups(&stats, &mut items, &mut story);
            i.beyond
                .as_mut()
                .unwrap()
                .bind_rage_player(&mut player, 0.7);
            ensure!(i.scripted(), "Rage pickup did not start its scene");
            let paused = serde_json::to_value(i.snapshot())?;
            i.advance_school(0., &map, &mut world, &mut player)?;
            ensure!(
                paused == serde_json::to_value(i.snapshot())?,
                "Paused Rage scene advanced"
            );
            let start = player.feet;
            for frame in 0..1080 {
                if skip_at == Some(frame) {
                    let age = i.beyond.as_ref().unwrap().state.age;
                    i.skip_cinematic(&map, &mut world, &mut player, &mut story)?;
                    ensure!(
                        !i.scripted() && i.beyond.as_ref().unwrap().state.age == age,
                        "Rage skip advanced map time or retained control"
                    );
                }
                i.advance_school(1. / 60., &map, &mut world, &mut player)?;
                i.update(1. / 60., &map, &mut world, &player, Vec3::X, false)?;
                if i.scripted() {
                    ensure!(
                        player.feet.distance(start) < 0.1,
                        "Rage lift lowered during transformation: {frame} {:?}",
                        player.feet
                    );
                    // Viewer and Route suppress movement when scripted(); the
                    // owner must also retain its saved player-control marker.
                    ensure!(
                        player.script_motion == 1 && player.velocity == Vec3::ZERO,
                        "Rage movement lock lost"
                    );
                    let camera = i
                        .beyond
                        .as_ref()
                        .unwrap()
                        .scene_camera_in(&world)
                        .context("Rage camera missing")?;
                    ensure!(
                        !world
                            .sweep(camera.eye, camera.eye, Vec3::splat(2.))
                            .start_solid,
                        "Rage camera inside geometry"
                    );
                }
                if [0, 120, 300, 600, 760, 870, 1030].contains(&frame) {
                    let camera = i.beyond.as_ref().unwrap().scene_camera_in(&world);
                    let transforms = i.transforms();
                    let saved = serde_json::from_value(serde_json::to_value(i.snapshot())?)?;
                    i.restore(&saved, &map)?;
                    i.sync(&mut world);
                    ensure!(transforms == i.transforms(), "Rage restore moved lift");
                    let paused = serde_json::to_value(i.snapshot())?;
                    i.advance_school(0., &map, &mut world, &mut player)?;
                    ensure!(
                        paused == serde_json::to_value(i.snapshot())?,
                        "Paused lift moved"
                    );
                    if let Some(before) = camera {
                        let after = i.beyond.as_ref().unwrap().scene_camera_in(&world).unwrap();
                        ensure!(
                            before.eye == after.eye && before.target == after.target,
                            "Rage restore changed shot"
                        );
                    }
                }
                i.sync_pickups(&stats, &mut items, &mut story);
                stats.update(1. / 60.);
            }
            ensure!(
                !i.scripted()
                    && player.script_motion == 0
                    && player.grounded
                    && world.body_clear(player.feet),
                "Rage did not return safe gameplay"
            );
            ensure!(
                !i.beyond.as_ref().unwrap().state.solved
                    && !i.beyond.as_ref().unwrap().state.last_open,
                "Rage granted puzzle/exit progress"
            );
            ensure!(
                (player.feet.z - start.z + 48.).abs() < 1.,
                "Rage lift did not carry Alice down 48: {:?} -> {:?}",
                start,
                player.feet
            );
            ensure!(
                i.beyond.as_ref().unwrap().state.rage_wave,
                "Rage callback not completed"
            );
            let guards: Vec<_> = i
                .encounters
                .as_ref()
                .unwrap()
                .actors
                .iter()
                .filter(|a| a.name == "rage_guard2")
                .collect();
            ensure!(
                !guards.is_empty() && guards.iter().all(|a| a.active),
                "Rage guard group not activated"
            );
            ensure!(
                crate::inventory::collect(&mut stats, &items, player.feet, &world).is_empty(),
                "Rage collected twice"
            );
            println!("PASS {:?} Rage watch/skip {skip_at:?}, paused/saved camera, grant, {} guards and supported lift descent",difficulty,guards.len());
            let b = i.beyond.as_mut().unwrap();
            let mut legacy = b.snapshot();
            legacy.cinema = None;
            legacy.rage_lift = None;
            b.restore(&legacy, &map)?;
            ensure!(
                !b.pickup(&stats) && b.scene_id().is_none(),
                "Legacy Rage replayed"
            );
        }
    }
    check_legacy_rage(assets, &map)?;
    Ok(())
}

fn check_legacy_rage(assets: &mut Assets, map: &Bsp) -> Result<()> {
    use crate::beyond::cinema::Beat;
    for elapsed in [0.5, 2.5, 8., 13.] {
        let (mut i, mut world, mut player) =
            crate::beyond::cinema_check::setup(assets, map, Beat::Rage)?;
        let home = player.feet;
        let b = i.beyond.as_mut().unwrap();
        let mut json = serde_json::to_value(b.snapshot())?;
        json.as_object_mut().unwrap().remove("rage_lift");
        json["age"] = elapsed.into();
        json["cinema"]["time"] = f32::min(elapsed, 12.5).into();
        json["cinema"]["finished"] = (elapsed >= 12.5).into();
        let legacy = serde_json::from_value(json)?;
        b.restore(&legacy, map)?;
        let expected_drop = 48. * ((elapsed - 1.) / 4.).clamp(0., 1.);
        player.feet.z -= expected_drop;
        b.state
            .cinema
            .as_mut()
            .unwrap()
            .home
            .as_mut()
            .unwrap()
            .translation = player.feet;
        i.sync(&mut world);
        let hit = world.sweep(
            player.feet + crate::collision::PLAYER_CENTER,
            player.feet + crate::collision::PLAYER_CENTER - Vec3::Z * 3.,
            crate::collision::PLAYER_HALF,
        );
        ensure!(
            !hit.start_solid && hit.fraction < 1.,
            "Legacy Rage lost support at {elapsed}"
        );
        let before = i.transforms();
        let mut story = crate::story::Story::load(assets, "fortress2");
        i.skip_cinematic(map, &mut world, &mut player, &mut story)?;
        ensure!(
            before == i.transforms(),
            "Legacy skip reset lift at {elapsed}"
        );
        for _ in 0..300 {
            i.advance_school(1. / 60., map, &mut world, &mut player)?;
        }
        ensure!(
            (player.feet.z - home.z + 48.).abs() < 1.,
            "Legacy descent failed at {elapsed}"
        );
    }
    println!("PASS legacy Rage lift phases retain support and complete without rewind/replay");
    Ok(())
}
pub fn check(assets: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&assets.read("maps/fortress2.bsp")?)?;
    let mut i = Interactions::load(&map)?;
    i.set_entry(assets, &map, "fortress2", None)?;
    let old = Interactions::load(&map)?.snapshot();
    i.restore(&old, &map)?;
    let mut world = World::from_bsp(&map)?;
    i.sync(&mut world);
    let mut p = Player::spawn(&world, crate::interaction::spawn(&map, None).0).unwrap();
    let before = serde_json::to_value(i.snapshot())?;
    i.advance_school(0., &map, &mut world, &mut p)?;
    ensure!(
        before == serde_json::to_value(i.snapshot())?,
        "Paused machinery moved"
    );
    let exit = vec3(-152., 3856., 428.);
    ensure!(
        i.triggers(0.01, exit, exit).transition.is_none(),
        "Early return exit"
    );
    let b = i.beyond.as_mut().unwrap();
    b.event("Start_GetSmart");
    b.skip_scene(&map, &world, &mut p)?;
    // Operate the actual nearby levers, checking wrong order and reset ownership.
    let eye = |y| vec3(1320., y, -285.);
    b.activate(&world, eye(529.), Vec3::X).unwrap();
    b.activate(&world, eye(465.), Vec3::X).unwrap();
    b.activate(&world, eye(593.), Vec3::X).unwrap();
    ensure!(!b.state.solved, "Wrong lever order opened doors");
    ensure!(
        b.activate(&world, vec3(1200., 497., 48.), -Vec3::X)
            .is_none(),
        "Lever used facing away"
    );
    b.activate(&world, vec3(1200., 497., 48.), Vec3::X).unwrap();
    ensure!(
        b.state.notes == 0 && !b.state.lever_used.iter().any(|v| *v),
        "Reset failed"
    );
    b.skip_scene(&map, &world, &mut p)?;
    for y in [465., 593., 529.] {
        b.activate(&world, eye(y), Vec3::X).unwrap();
    }
    ensure!(b.state.solved, "Correct lever order did not open doors");
    b.skip_scene(&map, &world, &mut p)?;
    for thread in ["Long_Walk_Flip", "GS_Move", "RAISE_ROOM", "LastStart"] {
        i.beyond.as_mut().unwrap().event(thread).unwrap();
    }
    for _ in 0..24 {
        i.advance_school(1. / 120., &map, &mut world, &mut p)?;
    }
    let snapshot = i.snapshot();
    let encoded = serde_json::to_vec(&snapshot)?;
    let decoded = serde_json::from_slice(&encoded)?;
    let mut restored = Interactions::load(&map)?;
    restored.set_entry(assets, &map, "fortress2", None)?;
    restored.restore(&decoded, &map)?;
    ensure!(
        i.transforms() == restored.transforms(),
        "Restored mover poses differ"
    );
    ensure!(
        serde_json::to_value(i.snapshot())? == serde_json::to_value(restored.snapshot())?,
        "Restored puzzle state differs"
    );
    let mut w2 = World::from_bsp(&map)?;
    restored.sync(&mut w2);
    let mut p2 = p.clone();
    for _ in 0..240 {
        i.advance_school(1. / 120., &map, &mut world, &mut p)?;
        restored.advance_school(1. / 120., &map, &mut w2, &mut p2)?;
    }
    ensure!(
        i.transforms() == restored.transforms() && p.feet == p2.feet,
        "Restored machinery diverged"
    );
    let b = i.beyond.as_mut().unwrap();
    ensure!(b.state.walkway == 3, "Rolling walkway did not advance");
    b.event("GS_MoveBack");
    for _ in 0..90 {
        i.advance_school(1. / 120., &map, &mut world, &mut p)?;
    }
    ensure!(
        i.beyond.as_ref().unwrap().state.walkway == 2,
        "Rolling walkway did not reverse"
    );
    i.beyond.as_mut().unwrap().event("GS_MoveBack");
    i.advance_school(0.1, &map, &mut world, &mut p)?;
    ensure!(
        i.beyond.as_ref().unwrap().state.walkway == 2,
        "Walkway passed its lower bound"
    );
    for _ in 0..180 {
        i.advance_school(0.1, &map, &mut world, &mut p)?;
    }
    for thread in ["LastOpenB", "LastOpenC"] {
        i.beyond.as_mut().unwrap().event(thread).unwrap();
        ensure!(
            !i.beyond.as_ref().unwrap().state.last_open,
            "Wrong final door enabled exit"
        );
        for _ in 0..180 {
            i.advance_school(0.1, &map, &mut world, &mut p)?;
        }
    }
    i.beyond.as_mut().unwrap().event("LastOpenA").unwrap();
    ensure!(
        i.beyond.as_ref().unwrap().state.last_open,
        "Correct final door did not enable exit"
    );
    let valid = i.beyond.as_ref().unwrap().snapshot();
    let mut invalid = valid.clone();
    invalid.step_slots = [1, 9, 10];
    ensure!(
        i.beyond.as_mut().unwrap().restore(&invalid, &map).is_err(),
        "Accepted disconnected walkway save"
    );
    i.beyond.as_mut().unwrap().restore(&valid, &map)?;
    crate::beyond::cinema_check::check(assets)?;
    println!("PASS Beyond the Wall: legacy migration, pause, lever order/reset, rolling walkway reversal, partial-motion JSON save/restore, final-door retries and exit gate");
    Ok(())
}
