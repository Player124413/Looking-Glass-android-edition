//! Staged transport/save fixtures, not a proof of the entire level route.
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::World,
    interaction::Interactions,
    movement::{Controls, Player, FIXED_DT},
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
fn load(assets: &mut Assets, map: &Bsp) -> Result<Interactions> {
    let mut i = Interactions::load(map)?;
    i.set_entry(assets, map, "potears1", None)?;
    Ok(i)
}
fn settle(
    i: &mut Interactions,
    map: &Bsp,
    world: &mut World,
    p: &mut Player,
    seconds: f32,
) -> Result<()> {
    for _ in 0..(seconds / FIXED_DT).ceil() as usize {
        i.advance_school(FIXED_DT, map, world, p)?;
    }
    Ok(())
}
pub fn check(assets: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&assets.read("maps/potears1.bsp")?)?;
    crate::pool::boulder_check::check(assets, &map)?;
    crate::pool::arrival_check::check(assets, &map)?;
    crate::pool::pilot_check::check(assets, &map)?;
    crate::pool::cinema_check::check(assets, &map)?;
    crate::pool::transport_check::check(assets, &map)?;
    let mut world = World::from_bsp(&map)?;
    let mut i = load(assets, &map)?;
    let legacy = Interactions::load(&map)?.snapshot();
    i.restore(&legacy, &map)?;
    let mut p = Player::spawn(&world, crate::interaction::spawn(&map, None).0).unwrap();
    i.sync(&mut world);
    let frozen = serde_json::to_value(i.snapshot())?;
    i.advance_school(0., &map, &mut world, &mut p)?;
    ensure!(
        frozen == serde_json::to_value(i.snapshot())?,
        "Paused leaves moved"
    );
    let ride1 = vec3(576., 2784., 2052.1);
    i.triggers(0.01, ride1, ride1);
    let before = i
        .pool
        .as_ref()
        .unwrap()
        .pose("rideleaf1obj", vec3(576., 2784., 2044.))
        .0;
    ensure!(before.z > 4000., "Leaf available before Turtle dialogue");
    let talk = vec3(232., 2460., 2044.);
    let e = i.triggers(0.01, talk, talk);
    ensure!(
        e.story == [crate::pool::TALK],
        "Turtle conversation missing"
    );
    i.completed_dialogue(crate::pool::TALK);
    settle(&mut i, &map, &mut world, &mut p, 40.)?;
    ensure!(
        (i.pool
            .as_ref()
            .unwrap()
            .pose("rideleaf1obj", vec3(576., 2784., 2044.))
            .0
            .z
            - 2044.)
            .abs()
            < 0.01,
        "Leaf failed to fall"
    );
    for (index, base, thread) in [
        (0, vec3(576., 2784., 2044.), "rideleaf1start"),
        (1, vec3(1840., -720., 1364.), "rideleaf2start"),
        (2, vec3(-1504., -2656., 332.), "rideleaf3start"),
        (3, vec3(-3936., 672., 716.), "rideleaf4start"),
    ] {
        if index == 1 {
            i.pool.as_mut().unwrap().event("Turtle_Encounter1");
            settle(&mut i, &map, &mut world, &mut p, 33.)?;
        }
        if index == 3 {
            i.pool.as_mut().unwrap().event("rideleaf4falldown");
            settle(&mut i, &map, &mut world, &mut p, 10.)?;
        }
        let name = format!("rideleaf{}obj", index + 1);
        i.sync(&mut world);
        p = Player::spawn(&world, base + Vec3::Z * 72.).expect("Leaf boarding volume blocked");
        for _ in 0..120 {
            p.tick(&world, Controls::default());
        }
        ensure!(
            p.grounded && (p.feet.z - base.z).abs() < 12.,
            "Boarding leaf {} failed at {:?}",
            index + 1,
            p.feet
        );
        let start = p.feet;
        i.reset_contacts();
        i.triggers(0.01, p.feet, p.feet);
        // Contact must start the actual leaf, not a manually invoked fixture event.
        for _ in 0..180 {
            i.advance_school(FIXED_DT, &map, &mut world, &mut p)?;
            p.tick(&world, Controls::default());
        }
        ensure!(p.feet.distance(start) > 50., "{thread} did not carry Alice");
        ensure!(
            p.grounded && world.body_clear(p.feet),
            "Leaf {} rider lost footing at {:?}",
            index + 1,
            p.feet
        );
        let saved = i.snapshot();
        let mut restored = load(assets, &map)?;
        restored.restore(&serde_json::from_slice(&serde_json::to_vec(&saved)?)?, &map)?;
        ensure!(
            restored.transforms() == i.transforms(),
            "Mid-ride save changed pose"
        );
        let mut other_world = World::from_bsp(&map)?;
        restored.sync(&mut other_world);
        let mut other = p.clone();
        for _ in 0..120 {
            i.advance_school(FIXED_DT, &map, &mut world, &mut p)?;
            p.tick(&world, Controls::default());
            restored.advance_school(FIXED_DT, &map, &mut other_world, &mut other)?;
            other.tick(&other_world, Controls::default());
        }
        ensure!(
            p.feet.distance(other.feet) < 0.01 && i.transforms() == restored.transforms(),
            "Restored rider diverged"
        );
        p.tick(
            &world,
            Controls {
                jump: true,
                ..Default::default()
            },
        );
        ensure!(p.velocity.z > 0., "Cannot jump off leaf");
        println!("PASS leaf {} contact, carry, jump, mid-ride save and deterministic continuation ({name})",index+1);
        // Staged fixtures return to the entrance between independent rides.
        p = Player::spawn(&world, crate::interaction::spawn(&map, None).0).unwrap();
    }
    for n in 1..=4 {
        i.pool.as_mut().unwrap().event(&format!("leaftrain{n}"));
        settle(&mut i, &map, &mut world, &mut p, 2.)?;
    }
    let mut restored = load(assets, &map)?;
    restored.restore(&i.snapshot(), &map)?;
    ensure!(
        restored.transforms() == i.transforms(),
        "Train save changed poses"
    );
    println!("PASS Pool of Tears transport gates, dialogue drop, four repeating trains, pause and legacy migration; staged fixtures are separate from --potears1-route-check");
    Ok(())
}
pub async fn render(assets: &mut Assets) -> Result<()> {
    crate::pool::cinema_check::render(assets).await?;
    let mut scene = crate::render::Scene::load(assets, "potears1")?;
    let mut i = load(assets, &scene.map)?;
    let mut art = crate::pool::Art::load(assets, i.pool.as_ref().unwrap())?;
    let mut p = Player::spawn(&scene.world, crate::interaction::spawn(&scene.map, None).0).unwrap();
    i.pool.as_mut().unwrap().event(crate::pool::TALK);
    i.completed_dialogue(crate::pool::TALK);
    i.pool.as_mut().unwrap().event("Turtle_Encounter1");
    i.pool.as_mut().unwrap().event("rideleaf4falldown");
    settle(&mut i, &scene.map, &mut scene.world, &mut p, 33.)?;
    for (name, eye, target) in [
        (
            "first-leaf",
            vec3(320., 2760., 2210.),
            vec3(700., 2750., 2030.),
        ),
        (
            "lilypads",
            vec3(2410., -440., 1550.),
            vec3(2040., -680., 1350.),
        ),
        (
            "third-leaf",
            vec3(-1730., -2530., 505.),
            vec3(-1490., -2660., 320.),
        ),
        (
            "last-leaf",
            vec3(-3660., 730., 900.),
            vec3(-3936., 672., 716.),
        ),
    ] {
        for frame in 0..3 {
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: eye,
                target,
                up: Vec3::Z,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            });
            let poses = i.transforms();
            scene.draw(eye, 33., false, false, &poses);
            art.draw(i.pool.as_ref().unwrap(), false, &scene.atmosphere, eye);
            crate::render::depth_read_only(|| scene.draw(eye, 33., false, true, &poses));
            set_default_camera();
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/pool-{name}.png"
                )))?;
            }
            next_frame().await;
        }
    }
    Ok(())
}
