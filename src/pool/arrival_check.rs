//! Real entrance, geometry, saved clocks and watched/skipped arrival acceptance.
use super::cinema_check::{setup, tick};
use super::*;
use crate::{assets::Assets, interaction::Interactions, story::Story};

pub fn stage(
    case: &str,
    assets: &mut Assets,
    map: &Bsp,
    i: &mut Interactions,
    world: &mut World,
    player: &mut Player,
) -> Result<Story> {
    let (fresh, w, p, mut story) = setup(assets, map, cinema::ENTRY)?;
    *i = fresh;
    *world = w;
    *player = p;
    let frames = match case {
        "pool-arrival-ready" => 0,
        "pool-arrival-fade" => 45,
        "pool-arrival-rock" => 260,
        "pool-arrival-rabbit" => 390,
        "pool-arrival-walk" => 570,
        "pool-arrival-complete" => 720,
        _ => anyhow::bail!("Unknown arrival fixture"),
    };
    for _ in 0..frames {
        tick(i, map, world, player, &mut story, 1. / 60.)?;
    }
    Ok(story)
}
fn committed(i: &Interactions, w: &World, p: &Player) -> Result<()> {
    let pool = i.pool.as_ref().unwrap();
    let s = &pool.state;
    ensure!(
        s.cinema.done[0]
            && !i.scripted()
            && pool.camera().is_none()
            && pool.actor("alice").is_none()
            && pool.actor("rabbit_actor1").is_none(),
        "Arrival cast/camera still owned"
    );
    ensure!(
        (p.feet.truncate() - vec2(-3536., 2912.)).length() < 0.001
            && p.script_facing.abs() < 0.001
            && p.grounded
            && w.body_clear(p.feet)
            && w.body_trace(p.feet, p.feet - Vec3::Z * 1.).fraction < 1.,
        "Unsafe arrival landing {:?}",
        p.feet
    );
    ensure!(
        pool.rock_pose(3).unwrap().0.translation == pool.points["t495"],
        "Arrival rock not at rest"
    );
    ensure!(
        !s.talking && !s.talked && s.drops.iter().all(Option::is_none) && !s.cinema.exit_sent,
        "Arrival committed unrelated progression"
    );
    Ok(())
}
pub fn behavior(r: &mut crate::save::Restored) -> Result<()> {
    if r.interactions.scripted() {
        ensure!(
            r.interactions.skip_cinematic(
                &r.scene.map,
                &mut r.scene.world,
                &mut r.game.player,
                &mut r.story
            )?,
            "Restored arrival skip rejected"
        );
    }
    committed(&r.interactions, &r.scene.world, &r.game.player)?;
    r.interactions.entry_story(&mut r.story);
    ensure!(
        !r.interactions.scripted(),
        "Completed arrival replayed after restart"
    );
    Ok(())
}
pub fn check(assets: &mut Assets, map: &Bsp) -> Result<()> {
    // The default and named entrance both read #284's actual thread. An invalid
    // destination, or an entrance without that thread, cannot start the scene.
    for entry in [None, Some("potears1_start1"), Some("absent")] {
        let mut i = Interactions::load(map)?;
        i.set_entry(assets, map, "potears1", entry)?;
        let mut story = Story::load(assets, "potears1");
        i.entry_story(&mut story);
        ensure!(
            i.scripted() == (entry != Some("absent")),
            "Wrong entrance scene binding"
        );
    }
    let mut no_thread = Bsp::parse(&assets.read("maps/potears1.bsp")?)?;
    no_thread.entities[284].remove("thread");
    let mut i = Interactions::load(&no_thread)?;
    i.set_entry(assets, &no_thread, "potears1", None)?;
    let mut story = Story::load(assets, "potears1");
    i.entry_story(&mut story);
    ensure!(
        !i.scripted(),
        "Arrival guessed without an authored entrance thread"
    );

    for hz in [30, 60, 144] {
        for skip in [None, Some(0.), Some(4.4), Some(6.5), Some(9.5), Some(11.5)] {
            let (mut i, mut w, mut p, mut story) = setup(assets, map, cinema::ENTRY)?;
            let start = i
                .pool
                .as_ref()
                .unwrap()
                .actor("alice")
                .context("Missing first-frame Alice")?
                .2;
            ensure!(
                (start.translation.truncate() - vec2(-3728., 2912.)).length() < 0.001,
                "Wrong Alice start"
            );
            let rabbit = i
                .pool
                .as_ref()
                .unwrap()
                .actor("rabbit_actor1")
                .context("Missing first-frame Rabbit")?
                .2;
            ensure!(
                (rabbit.rotation * Vec3::X).distance(Vec3::NEG_Y) < 0.001,
                "Wrong Rabbit idle facing"
            );
            let mut checkpoint = 0;
            for frame in 0..hz * 14 {
                let t = frame as f32 / hz as f32;
                if skip.is_some_and(|at| t >= at) {
                    i.skip_cinematic(map, &mut w, &mut p, &mut story)?;
                    break;
                }
                ensure!(
                    tick(&mut i, map, &mut w, &mut p, &mut story, 1. / hz as f32)? == 0,
                    "Arrival requested exit"
                );
                if !i.scripted() {
                    break;
                }
                let pool = i.pool.as_ref().unwrap();
                let s = &pool.state.cinema;
                ensure!(
                    s.rocks[3].is_some() == (s.time >= 4.),
                    "Early/missing rock release"
                );
                let a = pool.actor("alice").unwrap();
                let r = pool.actor("rabbit_actor1").unwrap();
                ensure!(
                    a.0 == if s.time < 8. { "idle_stand" } else { "walk" },
                    "Alice walk timing"
                );
                ensure!(
                    r.0 == if s.time < 5. { "idle" } else { "run" },
                    "Rabbit run timing"
                );
                ensure!(
                    w.body_clear(a.2.translation),
                    "Alice intersects arrival bank"
                );
                if s.time >= 8. {
                    ensure!(
                        w.body_trace(a.2.translation, a.2.translation - Vec3::Z * 2.)
                            .fraction
                            < 1.,
                        "Alice floats on bank"
                    );
                }
                let half = pool.arrival_rabbit_half();
                let base = r.2.translation + Vec3::Z * half.z;
                let support =
                    pool.cinema
                        .arrival
                        .rabbit_world
                        .sweep(base, base - Vec3::Z * 2., half);
                ensure!(
                    !support.start_solid && support.fraction < 1.,
                    "Rabbit unsupported/embedded at {}: {:?}",
                    s.time,
                    r.2.translation
                );
                let camera = pool.camera().unwrap();
                let original = pool.cinema.tracks["tears1_path4"].camera(s.time);
                ensure!(
                    camera.eye.distance(original.eye) < 0.001
                        && camera.target.distance(original.target) < 0.001,
                    "Wrong authored camera"
                );
                if checkpoint < 6 && t >= [0., 2., 4.3, 5.8, 8.7, 11.4][checkpoint] {
                    let saved = serde_json::to_value(i.snapshot())?;
                    i.advance_school(0., map, &mut w, &mut p)?;
                    ensure!(
                        saved == serde_json::to_value(i.snapshot())?,
                        "Paused arrival advanced"
                    );
                    let mut restored = Interactions::load(map)?;
                    restored.set_entry(assets, map, "potears1", Some("potears1_start1"))?;
                    restored.restore(&serde_json::from_value(saved)?, map)?;
                    let pool = restored.pool.as_ref().unwrap();
                    let c = pool.camera().unwrap();
                    ensure!(
                        c.eye.distance(camera.eye) < 0.001
                            && c.target.distance(camera.target) < 0.001,
                        "Saved arrival camera changed"
                    );
                    for (name, before) in [("alice", a), ("rabbit_actor1", r)] {
                        let after = pool.actor(name).unwrap();
                        ensure!(
                            before.0 == after.0
                                && before.1 == after.1
                                && before.2.translation.distance(after.2.translation) < 0.001
                                && before.2.rotation.dot(after.2.rotation).abs() > 0.9999,
                            "Saved arrival cast changed"
                        );
                    }
                    i = restored;
                    i.sync(&mut w);
                    checkpoint += 1;
                }
            }
            committed(&i, &w, &p)?;
            let before = serde_json::to_value(i.snapshot())?;
            i.entry_story(&mut story);
            ensure!(
                !i.skip_cinematic(map, &mut w, &mut p, &mut story)?
                    && before == serde_json::to_value(i.snapshot())?,
                "Duplicate arrival commitment"
            );
            ensure!(
                !story.busy() && story.completed == 0,
                "Invented arrival dialogue"
            );
            // A later camera may replace the legacy clock/home without corrupting
            // the already completed arrival's independently saved clocks.
            i.pool.as_mut().unwrap().scene_event("Tears1_Boulder1");
            tick(&mut i, map, &mut w, &mut p, &mut story, 1. / hz as f32)?;
            i.restore(&i.snapshot(), map)?;
            println!("PASS Pool arrival {hz} Hz skip={skip:?}: entrance, cast, cues, saved poses, grounded handoff, no rewards/exit/replay");
        }
    }
    // The previous ad-hoc scene becomes a runner at the same clock; consumed
    // legacy entries remain consumed. No migration teleports the gameplay body.
    let (mut i, mut w, mut p, mut story) = setup(assets, map, cinema::ENTRY)?;
    for _ in 0..570 {
        tick(&mut i, map, &mut w, &mut p, &mut story, 1. / 60.)?;
    }
    let before = p.feet;
    let time = i.pool.as_ref().unwrap().state.cinema.time;
    let mut saved = serde_json::to_value(i.snapshot())?;
    saved["pool"]["cinema"]["version"] = 2.into();
    saved["pool"]["cinema"]
        .as_object_mut()
        .unwrap()
        .remove("arrival");
    i.restore(&serde_json::from_value(saved.clone())?, map)?;
    ensure!(
        p.feet == before && i.pool.as_ref().unwrap().state.cinema.time == time,
        "Legacy active arrival restarted"
    );
    i.skip_cinematic(map, &mut w, &mut p, &mut story)?;
    committed(&i, &w, &p)?;
    saved["pool"]["cinema"]["beat"] = serde_json::Value::Null;
    saved["pool"]["cinema"]["done"][0] = true.into();
    i.restore(&serde_json::from_value(saved)?, map)?;
    let feet = p.feet;
    i.entry_story(&mut story);
    ensure!(
        !i.scripted() && p.feet == feet,
        "Legacy completed entry replayed"
    );
    println!(
        "PASS Pool arrival: active version2 clock migration and completed old-save continuation"
    );
    Ok(())
}
