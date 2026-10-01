//! Saved observatory performance and physical handoff checks.
use super::*;
use anyhow::ensure;
pub fn check(assets: &mut Assets, map: &Bsp) -> Result<()> {
    for (fixture, skips) in [
        (
            "return-star-start",
            &[None, Some(0.), Some(1.), Some(4.8), Some(6.2)][..],
        ),
        (
            "return-exit-start",
            &[None, Some(0.), Some(3.7), Some(8.4), Some(11.4), Some(13.)][..],
        ),
    ] {
        for skip in skips {
            let mut i = crate::interaction::Interactions::load(map)?;
            i.set_entry(assets, map, "skool1", Some("skool1_start2"))?;
            let mut world = World::from_bsp(map)?;
            let mut player = Player::new(Vec3::ZERO);
            let mut stats = crate::inventory::Stats::for_level("skool1", Some("skool1_start2"));
            i.school.as_mut().unwrap().return_fixture(
                fixture,
                map,
                &mut world,
                &mut player,
                &mut stats,
            )?;
            i.sync(&mut world);
            let mut story = crate::story::Story::load(assets, "skool1");
            let before = serde_json::to_value(i.snapshot())?;
            i.advance_school(0., map, &mut world, &mut player)?;
            ensure!(
                before == serde_json::to_value(i.snapshot())?,
                "Paused return advanced"
            );
            let mut skipped = false;
            let star = fixture == "return-star-start";
            let mut transition = None;
            for frame in 0..1200 {
                if !skipped && skip.is_some_and(|t| frame as f32 / 60. >= t) {
                    ensure!(
                        i.skip_cinematic(map, &mut world, &mut player, &mut story)?,
                        "Return skip rejected"
                    );
                    ensure!(
                        !i.skip_cinematic(map, &mut world, &mut player, &mut story)?,
                        "Return skip restarted"
                    );
                    ensure!(
                        i.scripted() && i.school.as_ref().unwrap().scene_camera().is_some(),
                        "Skipped tail released scene ownership"
                    );
                    skipped = true;
                }
                i.advance_school(1. / 60., map, &mut world, &mut player)?;
                let mut e = i.update(1. / 60., map, &mut world, &player, Vec3::X, false)?;
                e.merge(i.triggers(1. / 60., player.feet, player.feet));
                i.school.as_mut().unwrap().sync_inventory(&mut stats);
                let s = i.school.as_ref().unwrap();
                let r = s.return_visit.as_ref().unwrap();
                let phase = r.phase;
                let time = r.scene.as_ref().unwrap().time;
                ensure!(
                    !stats.school_items.star && (star || time >= 7.5 || stats.school_items.potion),
                    "Premature item consumption"
                );
                if star {
                    ensure!(
                        stats.school_items.potion && e.transition.is_none(),
                        "Star used the potion or exited"
                    );
                    if time < 6.5 {
                        ensure!(
                            s.object("observatory_lift").pose.origin
                                == s.object("observatory_lift").base.origin,
                            "Lift moved during Star reveal"
                        );
                    }
                } else {
                    ensure!(
                        e.transition.is_none() || phase == crate::school_return::Phase::Complete,
                        "Early Pool exit"
                    );
                }
                if frame % 43 == 0 {
                    let camera = s.scene_camera();
                    let saved = serde_json::from_value(serde_json::to_value(i.snapshot())?)?;
                    i.restore(&saved, map)?;
                    i.sync(&mut world);
                    player = serde_json::from_value(serde_json::to_value(&player)?)?;
                    player.validate_world(&world)?;
                    if let Some(a) = camera {
                        let b = i.school.as_ref().unwrap().scene_camera().unwrap();
                        ensure!(
                            a.eye.distance(b.eye) < 0.001
                                && a.target.distance(b.target) < 0.001
                                && a.up.distance(b.up) < 0.001,
                            "Saved return shot changed"
                        );
                    }
                }
                if star && phase == crate::school_return::Phase::Observatory {
                    break;
                }
                if e.transition.is_some() {
                    transition = e.transition;
                    break;
                }
            }
            let s = i.school.as_ref().unwrap();
            if star {
                ensure!(
                    s.return_visit.as_ref().unwrap().phase
                        == crate::school_return::Phase::Observatory
                        && !s.cinematic(),
                    "Lift did not release"
                );
                ensure!(
                    world.body_clear(player.feet) && player.feet.z > 1000.,
                    "Unsafe lift handoff {:?}",
                    player.feet
                );
            } else {
                ensure!(
                    transition == Some(("potears1".into(), Some("potears1_start1".into())))
                        && !stats.school_items.potion,
                    "Wrong return outcome"
                );
            }
            let snapshot = s.snapshot();
            i.school.as_mut().unwrap().sync_inventory(&mut stats);
            i.school.as_mut().unwrap().restore(&snapshot, map)?;
            ensure!(
                i.school
                    .as_mut()
                    .unwrap()
                    .event(if star {
                        "Skool1_OLift_Up"
                    } else {
                        "observatory_exit_cinematic"
                    })
                    .is_none(),
                "Completed return replayed"
            );
            println!("PASS return scene {fixture} skip {skip:?}: pause, saved shots, one-time items, physical lift and Pool entrance");
        }
    }
    Ok(())
}
