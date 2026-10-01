//! Real BSP binding/quest fixtures; separate from campaign traversal proof.
use super::*;
use crate::{assets::Assets, cheshire::Hints, school2_quest::Stage};
use anyhow::{ensure, Context};

fn placed(map: &Bsp, actor: &str) -> Option<Vec3> {
    let e = map
        .entities
        .iter()
        .find(|e| e.get("targetname").is_some_and(|n| n == actor))?;
    Some(vector(e.get("origin")?)? + Vec3::Z * (32. * value(e, "scale", 1.)).clamp(12., 100.))
}
fn tick(
    i: &mut Interactions,
    map: &Bsp,
    world: &mut World,
    player: &mut Player,
    story: &mut Story,
    advance: bool,
) -> Result<()> {
    let dt = 1. / 60.;
    i.advance_school(dt, map, world, player)?;
    let e = i.update(dt, map, world, player, Vec3::X, false)?;
    ensure!(e.transition.is_none(), "Talk unexpectedly changed level");
    for id in e.story {
        story.trigger(&id);
    }
    if i.prepare_story(story) {
        story.tick(dt, advance);
    }
    i.sync_cinematic_story(story);
    for id in story.take_completed() {
        i.completed_dialogue(&id);
    }
    if let Some(s) = &mut i.school2 {
        let (e, _) = s.update(dt, world, player.feet, story.busy());
        ensure!(
            e.transition.is_none(),
            "Gnome talk unexpectedly changed level"
        );
        for id in e.story {
            story.trigger(&id);
        }
    }
    Ok(())
}
pub fn check(assets: &mut Assets) -> Result<()> {
    let mut report = Vec::new();
    for &binding in BINDINGS {
        for skip in [false, true] {
            let map = Bsp::parse(&assets.read(&format!("maps/{}.bsp", binding.map))?)?;
            let mut i = Interactions::load(&map)?;
            i.set_entry(assets, &map, binding.map, None)?;
            let mut world = World::from_bsp(&map)?;
            let mut story = Story::load(assets, binding.map);
            let hints = Hints::load(assets, &map, binding.map)?;
            if binding.thread == crate::school2::cinema::FINAL {
                let s = i.school2.as_mut().unwrap();
                s.quest.enter(Stage::Lollipop);
                s.quest.items.lollipop = true;
                s.quest.items.mushroom = true;
                s.quest.items.spice = true;
                for g in &mut s.guards {
                    g.hurt(1000.);
                }
            }
            i.sync(&mut world);
            let target = i
                .controller_talk_target(binding)
                .or_else(|| placed(&map, binding.actor))
                .context("Missing bound actor")?;
            let mut approach = None;
            for angle in 0..24 {
                let yaw = angle as f32 * std::f32::consts::TAU / 24.;
                let eye = target + vec3(yaw.cos() * 80., yaw.sin() * 80., 32.);
                if let Some(player) = Player::spawn(&world, eye) {
                    let aim = (target - player.eye()).normalize_or_zero();
                    if let Some(c) =
                        i.resolve_conversation(&world, player.eye(), aim, &story, |n| {
                            placed(&map, n)
                        })
                    {
                        if c.actor() == binding.actor {
                            approach = Some((player, aim, c));
                            break;
                        }
                    }
                }
            }
            let (mut player, aim, c) = approach
                .with_context(|| format!("No clear real-map approach to {}", binding.actor))?;
            let id = c
                .trigger
                .context("First talk must use an authored trigger")?;
            let usage = i.event_world.snapshot().usage;
            i.event_world.set_enabled(id, false)?;
            ensure!(
                i.talk_activation(binding, &story).is_none()
                    || binding.thread == "Torchgnome2_Dialog",
                "Disabled trigger still selectable"
            );
            i.event_world.set_enabled(id, true)?;
            let event = i
                .start_conversation(c, &mut story)
                .context("Eligible talk rejected")?;
            ensure!(
                event.story == vec![binding.thread.to_owned()],
                "Wrong actor's conversation"
            );
            for id in event.story {
                story.trigger(&id);
            }
            ensure!(
                i.start_conversation(c, &mut story).is_none(),
                "Repeated E started duplicate conversation"
            );
            let used = i.event_world.snapshot().usage;
            ensure!(
                used[&format!("trigger/{}", id.0)].count
                    == usage[&format!("trigger/{}", id.0)].count + 1,
                "Talk did not consume existing trigger exactly once"
            );
            i.dispatch(Event::Entity(id, Input::Activate));
            ensure!(
                i.event_world.snapshot().usage == used,
                "Contact after E duplicated trigger effects"
            );
            for frame in 0..18000 {
                tick(&mut i, &map, &mut world, &mut player, &mut story, true)?;
                if frame == 10 {
                    i.restore(
                        &serde_json::from_value(serde_json::to_value(i.snapshot())?)?,
                        &map,
                    )?;
                    story.restore(
                        &serde_json::from_value(serde_json::to_value(story.snapshot())?)?,
                        &hints,
                    )?;
                    i.sync(&mut world);
                    ensure!(
                        i.resolve_conversation(&world, player.eye(), aim, &story, |n| placed(
                            &map, n
                        ))
                        .is_none(),
                        "Active restored scene offered Talk"
                    );
                }
                if skip && frame == 20 {
                    ensure!(
                        i.skip_cinematic(&map, &mut world, &mut player, &mut story)?,
                        "Scene could not skip"
                    );
                }
                if !i.scripted() && !story.busy() {
                    break;
                }
            }
            ensure!(
                !i.scripted() && !story.busy(),
                "Conversation did not finish"
            );
            ensure!(
                world.body_clear(player.feet),
                "Conversation returned player into a wall"
            );
            if let Some(s) = &i.school2 {
                if binding.thread == crate::school2::cinema::MUSHROOM {
                    ensure!(
                        s.quest.stage == Stage::Battle
                            && s.quest.items.mushroom
                            && !s.quest.items.spice,
                        "Mushroom talk bypassed rescue or lost reward"
                    );
                } else {
                    ensure!(
                        s.quest.stage == Stage::Rewards
                            && !s.quest.items.potion
                            && !s.quest.items.star
                            && !s.quest.items.mushroom
                            && !s.quest.items.spice
                            && !s.quest.items.lollipop,
                        "Final talk lost one-time consumption/reward"
                    );
                }
                ensure!(
                    i.talk_activation(binding, &story).is_none(),
                    "Consumed Gnome conversation still selectable"
                );
            }
            if i.pool.is_some() {
                // The released leaf takes nine seconds to settle after departure.
                for _ in 0..600 {
                    tick(&mut i, &map, &mut world, &mut player, &mut story, false)?;
                }
                let p = i.pool.as_ref().unwrap();
                ensure!(
                    p.state.talked
                        && Condition::flag("pool.ready0").test(&p.facts())
                        && p.conversation_target().is_none(),
                    "Turtle talk did not commit leaf/departure exactly once"
                );
            }
            if binding.map == "gvillage" {
                let before = serde_json::to_value(i.snapshot())?;
                let count = story.completed;
                let repeat = Conversation {
                    binding,
                    trigger: None,
                };
                i.start_conversation(repeat, &mut story)
                    .context("Completed village speech not repeatable")?;
                story.tick(0.1, false);
                story.restore(
                    &serde_json::from_value(serde_json::to_value(story.snapshot())?)?,
                    &hints,
                )?;
                // Repeat speech must not drive the scene/event owner at all.
                for _ in 0..100 {
                    story.tick(0.25, true);
                    i.sync_cinematic_story(&story);
                }
                ensure!(
                    !story.busy() && story.take_completed().is_empty() && story.completed == count,
                    "Repeat emitted quest completion"
                );
                ensure!(
                    serde_json::to_value(i.snapshot())? == before,
                    "Repeat changed quest or scene state"
                );
            }
            report.push(
                serde_json::json!({"map":binding.map,"actor":binding.actor,"thread":binding.thread,
                "trigger":id.0,"skip":skip,"feet":player.feet,"safe":true}),
            );
            println!(
                "PASS friendly {} {} skip={skip}: authored trigger, restore, single commitment",
                binding.map, binding.actor
            );
        }
    }
    // All other opening/return actors stay automatic or unavailable, even when
    // an arbitrary friendly-looking placement is directly in front of Alice.
    for (map_name, entry) in [
        ("pandemonium", None),
        ("fortress1", None),
        ("fortress1", Some("fortress1_start2")),
        ("fortress2", None),
        ("skool1", None),
        ("skool1", Some("skool1_start2")),
        ("potears2", None),
        ("potears3", None),
    ] {
        let map = Bsp::parse(&assets.read(&format!("maps/{map_name}.bsp"))?)?;
        let mut i = Interactions::load(&map)?;
        i.set_entry(assets, &map, map_name, entry)?;
        let story = Story::load(assets, map_name);
        ensure!(
            i.resolve_conversation(
                &World::fixture(&[]),
                Vec3::X * 50.,
                -Vec3::X,
                &story,
                |_| Some(Vec3::ZERO)
            )
            .is_none(),
            "Unreviewed/return-visit actor gained a conversation"
        );
    }
    let map = Bsp::parse(&assets.read("maps/skool2.bsp")?)?;
    let mut i = Interactions::load(&map)?;
    i.set_entry(assets, &map, "skool2", None)?;
    let story = Story::load(assets, "skool2");
    let s = i.school2.as_mut().unwrap();
    for stage in [
        Stage::Battle,
        Stage::Laboratory,
        Stage::Rescue,
        Stage::SpiceDialogue,
        Stage::Jumbogrow,
        Stage::Growing,
        Stage::Mixing,
        Stage::Rewards,
        Stage::Complete,
    ] {
        s.quest.enter(stage);
        ensure!(
            s.conversation_target(crate::school2::cinema::MUSHROOM)
                .is_none()
                && s.conversation_target(crate::school2::cinema::FINAL)
                    .is_none(),
            "Unavailable Gnome offered Talk"
        );
    }
    s.quest.enter(Stage::Lollipop);
    for mask in 0..8 {
        s.quest.items.mushroom = mask & 1 != 0;
        s.quest.items.spice = mask & 2 != 0;
        s.quest.items.lollipop = mask & 4 != 0;
        ensure!(
            s.conversation_target(crate::school2::cinema::FINAL)
                .is_some()
                == (mask == 7),
            "Ingredient gate bypassed"
        );
    }
    ensure!(!story.busy(), "Availability queries started speech");
    std::fs::create_dir_all("private/friendly-npcs")?;
    std::fs::write(
        "private/friendly-npcs/check.json",
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("PASS friendly audit exclusions, wrong visits, quest stages and all ingredient combinations");
    Ok(())
}

/// Isolated native save/input fixtures. They stage quest prerequisites, not a route.
pub fn stage(
    case: &str,
    assets: &mut Assets,
    map: &Bsp,
    i: &mut Interactions,
    world: &mut World,
    player: &mut Player,
) -> Result<(Story, Vec3)> {
    let binding = BINDINGS[if case.contains("mushroom") {
        4
    } else if case.contains("final") {
        5
    } else if case.contains("pool") {
        6
    } else {
        1
    }];
    *i = Interactions::load(map)?;
    i.set_entry(assets, map, binding.map, None)?;
    let mut story = Story::load(assets, binding.map);
    if let Some(v) = &mut i.village {
        v.cinema.state.done.insert("entry".into());
    }
    if let Some(p) = &mut i.pool {
        p.state.cinema.done[0] = true;
    }
    if binding.thread == crate::school2::cinema::FINAL {
        let s = i.school2.as_mut().unwrap();
        s.quest.enter(Stage::Lollipop);
        s.quest.items.mushroom = true;
        s.quest.items.spice = true;
        s.quest.items.lollipop = true;
        for g in &mut s.guards {
            g.hurt(1000.);
        }
    }
    i.sync(world);
    let target = i
        .controller_talk_target(binding)
        .or_else(|| placed(map, binding.actor))
        .context("Fixture actor missing")?;
    let mut selected = None;
    for angle in 0..24 {
        let yaw = angle as f32 * std::f32::consts::TAU / 24.;
        if let Some(p) = Player::spawn(world, target + vec3(yaw.cos() * 80., yaw.sin() * 80., 32.))
        {
            let aim = (target - p.eye()).normalize_or_zero();
            if let Some(c) = i.resolve_conversation(world, p.eye(), aim, &story, |n| placed(map, n))
            {
                if c.actor() == binding.actor {
                    selected = Some((p, aim, c));
                    break;
                }
            }
        }
    }
    let (p, aim, c) = selected.context("Fixture has no clear approach")?;
    *player = p.clone();
    if case.contains("repeat") {
        for event in i
            .start_conversation(c, &mut story)
            .context("Fixture talk failed")?
            .story
        {
            story.trigger(&event);
        }
        tick(i, map, world, player, &mut story, true)?;
        i.skip_cinematic(map, world, player, &mut story)?;
        for _ in 0..18000 {
            tick(i, map, world, player, &mut story, true)?;
            if !i.scripted() && !story.busy() {
                break;
            }
        }
        ensure!(
            !i.scripted() && !story.busy(),
            "Fixture first conversation stuck"
        );
        story.take_completed();
        if !case.ends_with("ready") {
            i.start_conversation(
                Conversation {
                    binding,
                    trigger: None,
                },
                &mut story,
            )
            .context("Fixture repeat rejected")?;
            if case.ends_with("active") {
                story.tick(0.1, false);
            }
        }
    }
    // The cinematic has an authored handoff away from this actor. Stage the
    // input fixture back at its verified approach, so its repeat prompt is testable.
    if case.contains("repeat") {
        *player = p;
    }
    player.script_facing = aim.y.atan2(aim.x);
    Ok((story, aim))
}
