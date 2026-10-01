//! Fixtures exercise real contacts/lever inputs, independently of camera playback.
use super::*;
use crate::{assets::Assets, interaction::Interactions, story::Story};
use anyhow::Context;
pub(crate) fn setup(
    assets: &mut Assets,
    map: &Bsp,
    beat: cinema::Beat,
) -> Result<(Interactions, World, Player)> {
    let mut i = Interactions::load(map)?;
    i.set_entry(assets, map, "fortress2", None)?;
    let mut world = World::from_bsp(map)?;
    i.sync(&mut world);
    let at = match beat {
        cinema::Beat::Demonstration => vec3(960., 376., 68.),
        cinema::Beat::Reset => vec3(1200., 497., 8.),
        cinema::Beat::Solved => vec3(1320., 529., -325.),
        cinema::Beat::Arches => vec3(5280., -4332., 76.),
        cinema::Beat::Rage => vec3(-160., 3552., 88.),
    };
    let mut p = Player::spawn(&world, at).context("Beyond scene fixture support")?;
    for _ in 0..120 {
        p.tick(&world, Default::default());
    }
    match beat {
        cinema::Beat::Rage => {
            let catalog = crate::inventory::Catalog::load(assets)?;
            let mut items = crate::inventory::pickups(map, "fortress2", &catalog);
            let mut stats = crate::inventory::Stats::default();
            let mut story = Story::load(assets, "fortress2");
            crate::inventory::collect(&mut stats, &items, p.feet, &world);
            ensure!(
                stats.collected.contains("fortress2:37"),
                "Rage scene fixture collection"
            );
            i.sync_pickups(&stats, &mut items, &mut story);
            i.beyond.as_mut().unwrap().bind_rage_player(&mut p, 0.);
        }
        cinema::Beat::Demonstration | cinema::Beat::Arches => {
            if beat == cinema::Beat::Arches {
                // An independently triggered reveal must not solve even a partial attempt.
                let state = &mut i.beyond.as_mut().unwrap().state;
                state.puzzle_started = true;
                state.notes = 2;
                state.lever_used = [true, false, true];
            }
            let thread = if beat == cinema::Beat::Arches {
                "RAISE_ROOM"
            } else {
                "Start_GetSmart"
            };
            let e = map
                .entities
                .iter()
                .find(|e| e.get("thread").is_some_and(|s| s == thread))
                .unwrap();
            let m = e["model"].trim_start_matches('*').parse::<usize>()?;
            let contact = vector(&e["origin"]).unwrap()
                + (map.models[m].min + map.models[m].max) * 0.5
                - PLAYER_CENTER;
            i.triggers(0.01, contact, contact);
        }
        cinema::Beat::Reset | cinema::Beat::Solved => {
            let b = i.beyond.as_mut().unwrap();
            b.state.puzzle_started = true;
            if beat == cinema::Beat::Reset {
                b.state.notes = 1;
                b.state.lever_used[2] = true;
                b.activate(&world, p.eye(), Vec3::X)
                    .context("Reset lever fixture")?;
            } else {
                for y in [465., 593., 529.] {
                    b.activate(&world, vec3(1320., y, -285.), Vec3::X)
                        .context("Solve lever fixture")?;
                }
            }
        }
    }
    ensure!(
        i.beyond.as_ref().unwrap().scene_id() == Some(beat.id()),
        "Contact did not start {beat:?}"
    );
    Ok((i, world, p))
}
fn progress(b: &Beyond) -> (bool, bool, [bool; 3], u8, u8, [u8; 3], bool) {
    let s = &b.state;
    (
        s.puzzle_started,
        s.solved,
        s.lever_used,
        s.notes,
        s.walkway,
        s.step_slots,
        s.last_open,
    )
}
pub fn check(assets: &mut Assets) -> Result<()> {
    let mut map = Bsp::parse(&assets.read("maps/fortress2.bsp")?)?;
    for difficulty in crate::powerups::Difficulty::ALL {
        map.difficulty = difficulty;
        for beat in [
            cinema::Beat::Demonstration,
            cinema::Beat::Reset,
            cinema::Beat::Solved,
            cinema::Beat::Arches,
        ] {
            for stop in [
                0.,
                beat.duration() * 0.5,
                beat.duration() - 0.2,
                beat.duration() + 0.1,
            ] {
                let (mut i, mut w, mut p) = setup(assets, &map, beat)?;
                let initial = progress(i.beyond.as_ref().unwrap());
                let home = p.feet;
                let paused = serde_json::to_value(i.snapshot())?;
                i.advance_school(0., &map, &mut w, &mut p)?;
                ensure!(
                    paused == serde_json::to_value(i.snapshot())?,
                    "Paused camera advanced"
                );
                for frame in 0..(stop * 120.).ceil() as usize {
                    i.advance_school(1. / 120., &map, &mut w, &mut p)?;
                    i.update(1. / 120., &map, &mut w, &p, Vec3::X, false)?;
                    if frame % 60 == 0 {
                        if let Some(c) = i.beyond.as_ref().unwrap().scene_camera() {
                            ensure!(
                                c.eye.is_finite() && c.target.is_finite() && c.up.length() > 0.99,
                                "Invalid camera"
                            );
                            let hit = w.sweep(c.eye, c.eye, Vec3::splat(2.));
                            ensure!(
                                !hit.start_solid,
                                "{beat:?} camera inside geometry at frame {frame}: {:?}",
                                c.eye
                            );
                        }
                    }
                }
                let saved = serde_json::from_value(serde_json::to_value(i.snapshot())?)?;
                let mut restored = Interactions::load(&map)?;
                restored.set_entry(assets, &map, "fortress2", None)?;
                restored.restore(&saved, &map)?;
                let mut w2 = World::from_bsp(&map)?;
                restored.sync(&mut w2);
                let mut p2 = p.clone();
                let mut story = Story::load(assets, "fortress2");
                // One branch watches; a restored branch skips. Both retain earned progress.
                let before_age = restored.beyond.as_ref().unwrap().state.age;
                restored.skip_cinematic(&map, &mut w2, &mut p2, &mut story)?;
                ensure!(
                    before_age == restored.beyond.as_ref().unwrap().state.age,
                    "Skip advanced machinery"
                );
                for _ in 0..2200 {
                    if !i.scripted() {
                        break;
                    }
                    i.advance_school(1. / 120., &map, &mut w, &mut p)?;
                    i.update(1. / 120., &map, &mut w, &p, Vec3::X, false)?;
                }
                for (branch, world, player) in [(&i, &w, &p), (&restored, &w2, &p2)] {
                    ensure!(
                        !branch.scripted()
                            && player.script_motion == 0
                            && player.velocity == Vec3::ZERO,
                        "{beat:?} did not release control"
                    );
                    ensure!(
                        progress(branch.beyond.as_ref().unwrap()) == initial,
                        "{beat:?} altered earned progress"
                    );
                    ensure!(
                        world.body_clear(player.feet)
                            && player.grounded
                            && player.feet.distance(home) < 1.,
                        "{beat:?} unsafe handoff {:?} home {home:?}",
                        player.feet
                    );
                }
                // Full same-process save continuation must also be deterministic.
                restored.restore(&saved, &map)?;
                restored.sync(&mut w2);
                let mut original = Interactions::load(&map)?;
                original.set_entry(assets, &map, "fortress2", None)?;
                original.restore(&saved, &map)?;
                original.sync(&mut w);
                let s = &restored.beyond.as_ref().unwrap().state;
                p.feet = s
                    .cinema
                    .as_ref()
                    .and_then(|c| c.home)
                    .map_or(home, |h| h.translation);
                p2 = p.clone();
                for _ in 0..120 {
                    original.advance_school(1. / 120., &map, &mut w, &mut p)?;
                    restored.advance_school(1. / 120., &map, &mut w2, &mut p2)?;
                }
                ensure!(
                    serde_json::to_value(original.snapshot())?
                        == serde_json::to_value(restored.snapshot())?
                        && p.feet == p2.feet,
                    "Scene save diverged"
                );
            }
            println!("PASS {difficulty:?} {beat:?}: watched/skipped, early/mid/late saves, safe handoff, unchanged puzzle progress");
        }
    }
    Ok(())
}
