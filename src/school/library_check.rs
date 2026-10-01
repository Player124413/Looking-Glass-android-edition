//! Actual trigger contacts and saved scene futures, including premature exit attempts.
use super::*;
use crate::{interaction::Interactions, story::Story};
use anyhow::ensure;
use cinema::library::{BOOK, RECIPE, SHELVES};

pub fn check(assets: &mut Assets, map: &Bsp) -> Result<()> {
    let hints = crate::cheshire::Hints::load(assets, map, "skool1")?;
    for name in [SHELVES, BOOK, RECIPE] {
        for (advance, skip) in [
            (false, None),
            (true, None),
            (false, Some(0.)),
            (false, Some(8.)),
            (false, Some(18.)),
        ] {
            let (mut i, mut world, mut p, mut story) = cinema_check::setup(assets, map, name)?;
            let mut exits = 0;
            let mut skipped = false;
            let mut finished_at = None;
            let mut last_camera = None;
            for frame in 0..9000 {
                let time = frame as f32 / 60.;
                if i.scripted() && !skipped && skip.is_some_and(|t| time >= t) {
                    i.skip_cinematic(map, &mut world, &mut p, &mut story)?;
                    skipped = true;
                }
                if frame % 53 == 0 {
                    let saved = serde_json::to_value(i.snapshot())?;
                    i.advance_school(0., map, &mut world, &mut p)?;
                    ensure!(
                        saved == serde_json::to_value(i.snapshot())?,
                        "Library pause changed state"
                    );
                    i.restore(&serde_json::from_value(saved)?, map)?;
                    i.sync(&mut world);
                    story.restore(
                        &serde_json::from_value(serde_json::to_value(story.snapshot())?)?,
                        &hints,
                    )?;
                    p = serde_json::from_value(serde_json::to_value(&p)?)?;
                    p.validate_world(&world)?;
                    if let Some((a, b)) = last_camera.zip(i.school.as_ref().unwrap().scene_camera())
                    {
                        let _: crate::cinematic::Camera = a;
                        ensure!(
                            a.eye.distance(b.eye) < 0.001 && a.up.distance(b.up) < 0.001,
                            "Library restored camera changed"
                        );
                    }
                }
                i.advance_school(1. / 60., map, &mut world, &mut p)?;
                let e = i.update(1. / 60., map, &mut world, &p, Vec3::X, false)?;
                if let Some(exit) = e.transition {
                    story.defer_exit(exit);
                }
                if i.prepare_story(&mut story) {
                    story.tick(1. / 60., advance);
                }
                i.sync_cinematic_story(&story);
                for id in story.take_completed() {
                    i.completed_dialogue(&id);
                }
                if let Some(exit) = i.take_story_exit(&mut story) {
                    ensure!(
                        name == RECIPE && !i.scripted() && exit.0 == "skool2",
                        "Early library exit"
                    );
                    exits += 1;
                }
                let s = i.school.as_ref().unwrap();
                if name == BOOK && i.scripted() {
                    ensure!(
                        !s.recipe_ready() && s.books == [Book::Bridge; 4],
                        "Book scene granted early recipe/descent"
                    );
                }
                if !i.scripted() && finished_at.is_none() {
                    finished_at = Some(frame);
                }
                last_camera = s.scene_camera();
                if finished_at.is_some_and(|at| frame > at + 250) {
                    break;
                }
            }
            ensure!(
                !i.scripted() && world.body_clear(p.feet),
                "Library scene stuck or unsafe: {name}"
            );
            ensure!(
                exits == usize::from(name == RECIPE),
                "Library exit count {exits}: {name}"
            );
            let saved = serde_json::to_value(i.snapshot())?;
            i.completed_dialogue(name);
            ensure!(
                !i.skip_cinematic(map, &mut world, &mut p, &mut story)?,
                "Completed library skip restarted"
            );
            ensure!(
                saved == serde_json::to_value(i.snapshot())?,
                "Library completion replay changed state"
            );
            if name == BOOK {
                ensure!(
                    i.school.as_ref().unwrap().recipe_ready(),
                    "Book did not settle"
                );
            }
            println!("PASS library {name}: advance={advance} skip={skip:?}, repeated restores, safe handoff, exits={exits}");
        }
    }
    // Neither an out-of-order callback nor an older deferred Story exit solves the puzzle.
    let mut i = Interactions::load(map)?;
    i.set_entry(assets, map, "skool1", None)?;
    let mut story = Story::load(assets, "skool1");
    for name in [BOOK, RECIPE] {
        i.school.as_mut().unwrap().event(name);
        i.completed_dialogue(name);
    }
    story.defer_exit(("skool2".into(), Some("skool2_start1".into())));
    ensure!(
        i.take_story_exit(&mut story).is_none()
            && !i.scripted()
            && !i.school.as_ref().unwrap().recipe,
        "Unsolved school escaped through deferred exit"
    );
    let (mut i, _, _, mut story) = cinema_check::setup(assets, map, RECIPE)?;
    i.school.as_mut().unwrap().first_cinema = None;
    story.finish_sequence(RECIPE);
    story.take_completed();
    // A different queued conversation must not make an already completed
    // legacy recipe line look unfinished forever.
    ensure!(
        story.trigger("Cat_Glass_Dialog") && story.busy(),
        "Legacy queued-dialogue fixture missing"
    );
    story.defer_exit(("skool2".into(), Some("skool2_start1".into())));
    ensure!(
        i.take_story_exit(&mut story).is_none()
            && i.school.as_ref().unwrap().scene_id() == Some(RECIPE)
            && i.school
                .as_ref()
                .unwrap()
                .first_cinema
                .as_ref()
                .unwrap()
                .dialogue_done,
        "Legacy deferred recipe exit bypassed scene"
    );
    println!("PASS library unsolved callbacks and legacy deferred-exit adoption");
    Ok(())
}
