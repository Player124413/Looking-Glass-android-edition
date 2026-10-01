//! C1 pilot acceptance, through the legacy owner and real BSP contacts.
use super::cinema_check::{setup, tick, trigger};
use super::*;
use crate::{
    assets::Assets,
    interaction::Interactions,
    movement::{Controls, FIXED_DT},
    powerups::Difficulty,
};

pub fn stage(
    case: &str,
    assets: &mut Assets,
    map: &Bsp,
    i: &mut Interactions,
    world: &mut World,
    player: &mut Player,
) -> Result<crate::story::Story> {
    let (fresh, w, p, mut story) = setup(assets, map, cinema::EXIT)?;
    *i = fresh;
    *world = w;
    *player = p;
    let frames = match case {
        "pool-pilot-fade" => 18,
        "pool-pilot-jump" => 84,
        "pool-pilot-fall" => 165,
        "pool-pilot-late" => 285,
        "pool-pilot-complete" => 305,
        _ => anyhow::bail!("Unknown Pool pilot fixture"),
    };
    for _ in 0..frames {
        tick(i, map, world, player, &mut story, 1. / 60.)?;
    }
    Ok(story)
}

pub fn check(assets: &mut Assets, map: &Bsp) -> Result<()> {
    dependencies(assets)?;
    let mut expected = None;
    for hz in [30, 60, 144] {
        for skip in [None, Some(0.), Some(2.5), Some(4.8)] {
            let (mut i, mut w, mut p, mut story) = setup(assets, map, cinema::EXIT)?;
            let home = p.feet;
            let mut checkpoint = 0;
            let mut exits = 0;
            for frame in 0..(hz * 7) {
                let time = frame as f32 / hz as f32;
                if skip.is_some_and(|t| time >= t) && exits == 0 {
                    i.skip_cinematic(map, &mut w, &mut p, &mut story)?;
                }
                exits += tick(&mut i, map, &mut w, &mut p, &mut story, 1. / hz as f32)?;
                ensure!(
                    p.feet == home && p.velocity == Vec3::ZERO,
                    "Ending lost player ownership"
                );
                if exits > 0 {
                    break;
                }
                if checkpoint < 5 && time >= [0.1, 0.6, 1.5, 2.6, 4.5][checkpoint] {
                    let paused = serde_json::to_value(i.snapshot())?;
                    i.advance_school(0., map, &mut w, &mut p)?;
                    ensure!(
                        paused == serde_json::to_value(i.snapshot())?,
                        "Paused pilot changed"
                    );
                    let camera = i.pool.as_ref().unwrap().camera().unwrap();
                    let mut restored = Interactions::load(map)?;
                    restored.set_entry(assets, map, "potears1", None)?;
                    restored.restore(&serde_json::from_value(paused)?, map)?;
                    restored.sync(&mut w);
                    let after = restored.pool.as_ref().unwrap().camera().unwrap();
                    ensure!(
                        camera.eye.distance(after.eye) < 0.001
                            && camera.up.distance(after.up) < 0.001,
                        "Pilot restore lost shot"
                    );
                    i = restored;
                    checkpoint += 1;
                }
            }
            ensure!(exits == 1, "Pilot did not emit exactly one exit");
            let pool = i.pool.as_ref().unwrap();
            let digest = serde_json::to_value(pool.state.cinema.ending.as_ref().unwrap())?;
            if let Some(e) = &expected {
                ensure!(*e == digest, "Watch/skip completion differs");
            } else {
                expected = Some(digest);
            }
            ensure!(
                skip.is_some() || checkpoint == 5,
                "Missing pilot restore point"
            );
            for _ in 0..10 {
                ensure!(
                    tick(&mut i, map, &mut w, &mut p, &mut story, 1. / 60.)? == 0,
                    "Duplicate exit"
                );
            }
            i.transition_failed(&cinema::ENDING.end.exit.unwrap().destination());
            ensure!(
                tick(&mut i, map, &mut w, &mut p, &mut story, 0.)? == 0,
                "Paused retry"
            );
            let mut retries = 0;
            for _ in 0..120 {
                retries += tick(&mut i, map, &mut w, &mut p, &mut story, 1. / 60.)?;
            }
            ensure!(retries == 1, "Failed exit did not retry once");
            let saved = serde_json::from_value(serde_json::to_value(i.snapshot())?)?;
            i.restore(&saved, map)?;
            ensure!(
                tick(&mut i, map, &mut w, &mut p, &mut story, 1. / 60.)? == 1,
                "Saved pending exit lost"
            );
            println!("PASS Pool C1 pilot {hz}Hz skip={skip:?}: saved clocks, pause, equal completion, held player, exit retry");
        }
    }
    migration(assets, map)?;
    last_leaf(assets)?;
    Ok(())
}

fn last_leaf(assets: &mut Assets) -> Result<()> {
    for skip in [false, true] {
        let map = Bsp::parse(&assets.read("maps/potears1.bsp")?)?;
        let mut i = Interactions::load(&map)?;
        i.set_entry(assets, &map, "potears1", None)?;
        let mut w = World::from_bsp(&map)?;
        i.sync(&mut w);
        let mut p = Player::spawn(&w, crate::interaction::spawn(&map, None).0).unwrap();
        // Stage only the preceding authored leaf-drop event; boarding onward is physical.
        i.pool.as_mut().unwrap().event("rideleaf4falldown");
        for _ in 0..1200 {
            i.advance_school(FIXED_DT, &map, &mut w, &mut p)?;
        }
        p = Player::spawn(&w, vec3(-3936., 672., 788.)).context("Last leaf boarding blocked")?;
        for _ in 0..120 {
            p.tick(&w, Controls::default());
        }
        i.triggers(FIXED_DT, p.feet, p.feet);
        let mut story = crate::story::Story::load(assets, "potears1");
        let mut entered = None;
        let mut exited = false;
        for frame in 0..3600 {
            let before = p.feet;
            i.advance_school(FIXED_DT, &map, &mut w, &mut p)?;
            if !i.scripted() {
                p.tick(&w, Controls::default());
            }
            let contact = i.triggers(FIXED_DT, before, p.feet);
            ensure!(
                contact.damage == 0. && contact.transition.is_none(),
                "Last leaf hit a hazard or early exit"
            );
            if i.scripted() {
                entered.get_or_insert(frame);
                if skip {
                    i.skip_cinematic(&map, &mut w, &mut p, &mut story)?;
                }
            }
            let out = i.update(FIXED_DT, &map, &mut w, &p, Vec3::NEG_Y, false)?;
            if let Some(exit) = out.transition {
                ensure!(
                    entered.is_some() && (skip || frame - entered.unwrap() >= 599),
                    "Premature scene exit"
                );
                let mut stats = crate::inventory::Stats::for_level("potears1", None);
                stats.damage(39.);
                stats.spend_will(17.);
                let before_stats = serde_json::to_value(&stats)?;
                let leaving = crate::save::Level {
                    map: "potears1".into(),
                    entry: None,
                    interactions: i.snapshot(),
                    npcs: Default::default(),
                    story: story.snapshot(),
                    hints: crate::cheshire::Hints::load(assets, &map, "potears1")?.snapshot(),
                    environment_clock: 0.,
                    pickup_clock: 0.,
                };
                let mut ledger = crate::save::Campaign::default();
                let next =
                    crate::campaign::arrive(assets, &mut stats, &mut ledger, leaving, &exit, true)?;
                ensure!(
                    next.map == "potears2"
                        && next.entry.as_deref() == Some("potears2_start1")
                        && next.loaded.world.body_clear(next.player.feet)
                        && ledger.completed.contains("potears1$first")
                        && before_stats == serde_json::to_value(stats)?,
                    "Pool transition altered resources or entrance"
                );
                exited = true;
                break;
            }
        }
        ensure!(exited, "Last leaf did not reach scene/exit");
        println!("PASS Pool last-leaf physical boarding/contact/held exit, skip={skip}, strict arrival preserves resources");
    }
    Ok(())
}

fn migration(assets: &mut Assets, map: &Bsp) -> Result<()> {
    for seconds in [0_f32, 0.75, 2.5, 4.5, 5.2] {
        let (mut i, mut w, mut p, mut story) = setup(assets, map, cinema::EXIT)?;
        for _ in 0..(seconds * 60.) as usize {
            tick(&mut i, map, &mut w, &mut p, &mut story, 1. / 60.)?;
        }
        let mut old = serde_json::to_value(i.snapshot())?;
        old["pool"]["cinema"]["version"] = 1.into();
        old["pool"]["cinema"]
            .as_object_mut()
            .unwrap()
            .remove("ending");
        i.restore(&serde_json::from_value(old)?, map)?;
        let s = i
            .pool
            .as_ref()
            .unwrap()
            .state
            .cinema
            .ending
            .as_ref()
            .context("Unmigrated pilot")?;
        ensure!(
            (s.time - seconds.min(5.)).abs() < 0.02,
            "Legacy pilot clock lost"
        );
        let mut exits = 0;
        for _ in 0..360 {
            exits += tick(&mut i, map, &mut w, &mut p, &mut story, 1. / 60.)?;
        }
        ensure!(exits == 1, "Legacy active/completed pilot failed exit");
    }
    let (mut i, _, _, _) = setup(assets, map, cinema::EXIT)?;
    let mut old = serde_json::to_value(i.snapshot())?;
    old["pool"]["cinema"]["version"] = 1.into();
    old["pool"]["cinema"]["beat"] = serde_json::Value::Null;
    old["pool"]["cinema"]
        .as_object_mut()
        .unwrap()
        .remove("ending");
    i.restore(&serde_json::from_value(old)?, map)?;
    let at = trigger(map, cinema::EXIT);
    i.triggers(0.01, at, at);
    ensure!(
        i.pool.as_ref().unwrap().scene_id() == Some(cinema::EXIT),
        "Consumed pending pilot did not rearm"
    );
    println!("PASS Pool legacy active/complete/pending ending migration, no entry replay");
    Ok(())
}

fn dependencies(assets: &mut Assets) -> Result<()> {
    for difficulty in Difficulty::ALL {
        let mut map = Bsp::parse(&assets.read("maps/potears1.bsp")?)?;
        map.difficulty = difficulty;
        let mut i = Interactions::load(&map)?;
        i.set_entry(assets, &map, "potears1", None)?;
        let pool = i.pool.as_ref().unwrap();
        ensure!(
            pool.clips.len() == [0, 2, 6, 6][difficulty as usize],
            "Wrong Pool difficulty clips"
        );
        for id in 1..=6 {
            let e = &map.entities[id];
            ensure!(!supported(e), "Clip was submitted for drawing");
            ensure!(
                map.models[id]
                    .brushes
                    .clone()
                    .all(|b| map.shaders[map.brushes[b].shader].contents & 0x10000 != 0),
                "Not a player clip"
            );
        }
        let e = &map.entities[127];
        let id: usize = e["model"].trim_start_matches('*').parse()?;
        let at = vector(&e["origin"]).unwrap() + (map.models[id].min + map.models[id].max) * 0.5
            - PLAYER_CENTER;
        ensure!(
            i.triggers(0.01, at, at).damage == 9999.,
            "Pool kill volume lost setdamage"
        );
        println!(
            "PASS Pool {:?}: collision-only clip set and 9999 damage",
            difficulty
        );
    }
    normal_posts(assets)?;
    Ok(())
}

fn normal_posts(assets: &mut Assets) -> Result<()> {
    let mut map = Bsp::parse(&assets.read("maps/potears1.bsp")?)?;
    map.difficulty = Difficulty::Normal;
    let mut i = Interactions::load(&map)?;
    i.set_entry(assets, &map, "potears1", None)?;
    let mut w = World::from_bsp(&map)?;
    i.sync(&mut w);
    let mut p = Player::spawn(&w, vec3(-1504., -2656., 404.)).context("Leaf3 boarding blocked")?;
    for _ in 0..120 {
        p.tick(&w, Controls::default());
    }
    i.triggers(FIXED_DT, p.feet, p.feet);
    for frame in 0..1680 {
        i.advance_school(FIXED_DT, &map, &mut w, &mut p)?;
        let jump = frame == 1038 || frame == 1398;
        let time = frame as f32 * FIXED_DT;
        let mut target = i.pool.as_ref().unwrap().paths[2].pose(time + 0.35).0;
        if (8.3..9.6).contains(&time) {
            target.x = -2025.;
        }
        if (11.3..12.7).contains(&time) {
            target.x = -2040.;
        }
        let wish =
            if !p.grounded || jump || (8.3..9.6).contains(&time) || (11.3..12.7).contains(&time) {
                ((target - p.feet).truncate() / 60.).clamp_length_max(1.)
            } else {
                Vec2::ZERO
            };
        p.tick(
            &w,
            Controls {
                jump,
                wish,
                ..Default::default()
            },
        );
        ensure!(
            w.body_clear(p.feet),
            "Normal clip route entered a solid post"
        );
    }
    ensure!(
        p.grounded && p.feet.y > -2500. && p.feet.z > 320.,
        "Normal clip route lost its leaf at {:?}",
        p.feet
    );
    println!(
        "PASS Pool Normal route past posts4/3 with ordinary steering/jump inputs: {:?}, {} jumps",
        p.feet, p.jumps
    );
    Ok(())
}
