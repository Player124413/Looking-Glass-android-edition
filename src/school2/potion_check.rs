//! Real-asset scene fixtures; these do not stand in for the continuous route.
use super::cinema::{Beat, FINAL, GROW};
use super::*;
use crate::{interaction::Interactions, story::Story};
use anyhow::ensure;

pub(crate) fn setup(
    assets: &mut Assets,
    map: &Bsp,
    beat: Beat,
) -> Result<(Interactions, World, Player, Story)> {
    let mut i = Interactions::load(map)?;
    i.set_entry(assets, map, "skool2", None)?;
    let mut world = World::from_bsp(map)?;
    i.sync(&mut world);
    let p = Player::spawn(
        &world,
        if beat == Beat::Growth {
            vec3(-2896., -2560., 504.)
        } else {
            vec3(168., -864., 280.)
        },
    )
    .context("Potion fixture spawn")?;
    let mut story = Story::load(assets, "skool2");
    let s = i.school2.as_mut().unwrap();
    for b in &mut s.boojums {
        b.active = false;
    }
    for g in &mut s.guards {
        g.hurt(1000.);
    }
    s.quest.items.mushroom = true;
    s.quest.items.spice = true;
    s.quest.items.jumbogrow = beat == Beat::Growth;
    s.quest.items.lollipop = beat == Beat::Final;
    s.quest.enter(if beat == Beat::Growth {
        Stage::Jumbogrow
    } else {
        Stage::Lollipop
    });
    for event in s
        .event(if beat == Beat::Growth { GROW } else { FINAL })
        .unwrap()
        .story
    {
        story.trigger(&event);
    }
    ensure!(
        s.cinematic() && s.scene_id().is_some(),
        "Potion scene did not register"
    );
    Ok((i, world, p, story))
}

pub fn check(assets: &mut Assets, map: &Bsp) -> Result<()> {
    for beat in [Beat::Growth, Beat::Final] {
        let mut outcomes = Vec::new();
        for advance in [false, true] {
            for skip in [
                None,
                Some(0.),
                Some(2.),
                Some(5.),
                Some(8.),
                Some(15.),
                Some(25.),
            ] {
                let (mut i, mut world, mut player, mut story) = setup(assets, map, beat)?;
                let home = player.feet;
                let hints = crate::cheshire::Hints::load(assets, map, "skool2")?;
                let mut frames = 0;
                while i.scripted() && frames < 9000 {
                    if skip.is_some_and(|at| frames as f32 / 60. >= at) {
                        ensure!(
                            i.skip_cinematic(map, &mut world, &mut player, &mut story)?,
                            "Potion skip rejected"
                        );
                        break;
                    }
                    let s = i.school2.as_ref().unwrap();
                    ensure!(
                        s.targets().is_empty()
                            && !s.quest.items.jumbogrow
                            && !s.quest.items.potion
                            && !s.quest.items.star,
                        "Early combat or reward"
                    );
                    ensure!(
                        s.quest.items.mushroom
                            && s.quest.items.spice
                            && s.quest.items.lollipop == (beat == Beat::Final),
                        "Ingredients consumed during staging"
                    );
                    ensure!(
                        !s.trigger_enabled("exit_trigger", "", true),
                        "Scene unlocked exit"
                    );
                    if frames % 53 == 0 {
                        let old = serde_json::to_value(i.snapshot())?;
                        let camera = s.scene_camera().unwrap();
                        let condenser = s.condenser_time();
                        let mut loops = Vec::new();
                        s.potion_sound(&mut loops);
                        i.advance_school(0., map, &mut world, &mut player)?;
                        ensure!(
                            old == serde_json::to_value(i.snapshot())?,
                            "Paused potion scene advanced"
                        );
                        i.restore(&serde_json::from_value(old.clone())?, map)?;
                        i.sync(&mut world);
                        story.restore(
                            &serde_json::from_value(serde_json::to_value(story.snapshot())?)?,
                            &hints,
                        )?;
                        let restored = i.school2.as_ref().unwrap();
                        let mut after = Vec::new();
                        restored.potion_sound(&mut after);
                        ensure!(
                            loops.len() == after.len()
                                && loops.iter().zip(after).all(|(a, b)| a.id == b.id
                                    && a.clock == b.clock
                                    && a.path == b.path
                                    && a.origin == b.origin),
                            "Restored condenser loop changed phase"
                        );
                        ensure!(
                            old == serde_json::to_value(i.snapshot())?
                                && camera.eye.distance(restored.scene_camera().unwrap().eye)
                                    < 0.001
                                && condenser == restored.condenser_time(),
                            "Potion restoration changed staging"
                        );
                        let s = i.school2.as_mut().unwrap();
                        s.event(GROW);
                        s.event(FINAL);
                        ensure!(
                            old == serde_json::to_value(i.snapshot())?,
                            "Repeated trigger consumed ingredients"
                        );
                    }
                    cinema_check::tick(
                        &mut i,
                        map,
                        &mut world,
                        &mut player,
                        &mut story,
                        1. / 60.,
                        advance,
                    )?;
                    frames += 1;
                }
                ensure!(
                    !i.scripted()
                        && player.script_motion == 0
                        && world.body_clear(player.feet)
                        && player.feet.distance(home) < 1.,
                    "Potion scene stuck/unsafe {beat:?}"
                );
                let s = i.school2.as_mut().unwrap();
                s.cinema.as_ref().unwrap().validate(s.quest.stage)?;
                let expected = if beat == Beat::Growth {
                    Stage::Lollipop
                } else {
                    Stage::Rewards
                };
                ensure!(
                    s.quest.stage == expected && !s.quest.items.potion && !s.quest.items.star,
                    "Incorrect potion commitment"
                );
                ensure!(
                    s.quest.items.mushroom == (beat == Beat::Growth)
                        && s.quest.items.spice == (beat == Beat::Growth)
                        && !s.quest.items.lollipop,
                    "Ingredient handover mismatch"
                );
                if beat == Beat::Growth {
                    ensure!(
                        s.boojums[8..10].iter().all(|b| b.active && b.health > 0.),
                        "Growth lost live ambush"
                    );
                }
                let mut loops = Vec::new();
                s.potion_sound(&mut loops);
                ensure!(
                    loops.is_empty(),
                    "Completed/skipped condenser audio still running"
                );
                if skip.is_none() {
                    ensure!(
                        frames >= if beat == Beat::Growth { 590 } else { 930 },
                        "Dialogue advance bypassed authored actions"
                    );
                }
                outcomes.push((
                    s.quest.stage,
                    s.quest.items.clone(),
                    s.boojums.iter().map(|b| b.active).collect::<Vec<_>>(),
                ));
                let saved = serde_json::to_value(i.snapshot())?;
                i.restore(&serde_json::from_value(saved.clone())?, map)?;
                i.completed_dialogue(FINAL);
                ensure!(
                    !i.skip_cinematic(map, &mut world, &mut player, &mut story)?
                        && saved == serde_json::to_value(i.snapshot())?,
                    "Completed scene replayed"
                );
                let s = i.school2.as_mut().unwrap();
                if beat == Beat::Final {
                    ensure!(
                        s.quest.collect("shrink_potion")
                            && !s.quest.collect("shrink_potion")
                            && !s.trigger_enabled("exit_trigger", "", true),
                        "Potion not one-time or exit early"
                    );
                    let saved = s.snapshot();
                    s.restore(&saved, map)?;
                    ensure!(
                        s.quest.collect("lucky_star")
                            && !s.quest.collect("lucky_star")
                            && s.trigger_enabled("exit_trigger", "", true),
                        "Star/save/exit commitment wrong"
                    );
                } else {
                    ensure!(
                        s.quest.collect("ig_lollypop") && !s.quest.collect("ig_lollypop"),
                        "Lollipop pickup duplicated"
                    );
                }
                println!("PASS potion {beat:?} advance={advance} skip={skip:?} frames={frames}: restored staging, single consumption, distinct rewards");
            }
        }
        ensure!(
            outcomes.iter().all(|o| o == &outcomes[0]),
            "Watch/advance/skip outcomes differ"
        );
    }
    // Legacy saves keep their already-running timers, without replaying a new scene.
    for stage in [Stage::Growing, Stage::FinalDialogue, Stage::Mixing] {
        let (i, world, player, _) = setup(assets, map, Beat::Final)?;
        let mut value = serde_json::to_value(i.school2.unwrap().snapshot())?;
        value.as_object_mut().unwrap().remove("cinema");
        value["quest"]["stage"] = serde_json::to_value(stage)?;
        value["quest"]["time"] = serde_json::json!(2.);
        let mut s = School2::load(assets, map)?;
        s.restore(&serde_json::from_value(value)?, map)?;
        if stage == Stage::FinalDialogue {
            ensure!(s.completed_dialogue(FINAL), "Legacy final dialogue stuck");
        }
        for _ in 0..61 {
            s.update(0.1, &world, player.feet, false);
        }
        ensure!(
            !s.cinematic()
                && s.quest.stage
                    == if stage == Stage::Growing {
                        Stage::Lollipop
                    } else {
                        Stage::Rewards
                    },
            "Legacy potion save changed gate"
        );
    }
    println!(
        "PASS potion legacy growing/dialogue/mixing saves; equivalent watched/skipped outcomes"
    );
    Ok(())
}
