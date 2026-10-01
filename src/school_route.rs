//! Normal first-school input route, with live combat and progression.
use crate::{assets::Assets, route::Route};
use anyhow::Result;
use macroquad::prelude::*;
pub fn check(assets: &mut Assets, secret: bool) -> Result<()> {
    let mut r = crate::route::Route::new(assets, "skool1", None)?;
    run(&mut r, secret)?;
    println!("PASS continuous school traversal in {} ticks, sanity {}, {} jumps, {} climbs, {} throws, {} swings, {} cards, {} combat damage; no flight, warps or recovery. Combat active throughout.", r.ticks, r.stats.sanity(), r.player.jumps, r.player.climbs, r.shots, r.swings, r.cards, r.damage);
    Ok(())
}
/// The first school visit to the recipe exit.
pub fn drive(r: &mut Route) -> Result<()> {
    run(r, false)
}
/// The same route through the Looking Glass secret (an optional reward).
pub fn drive_secret(r: &mut Route) -> Result<()> {
    run(r, true)
}
/// The Croquet Mallet altar (`mallet_skool1`, first visit only) stands on the theatre's upper
/// landing, off the line of the route's goals. A strict chain has no baseline fill, so the route
/// has to collect the authored pickup before the recipe exit; it does so from the library passage.
const MALLET_LANDING_FROM: Vec3 = vec3(-2304., 2800., -256.);
const MALLET_ALTAR: Vec3 = vec3(-2274., 2334., -256.);
/// Walk to the altar and back to the passage. The chain's reward provenance is what asserts that
/// the pickup was collected (`campaign_chain::provenance`); a run from the baseline already holds it.
fn fetch_mallet(r: &mut Route) -> Result<()> {
    r.navigate(MALLET_ALTAR)?;
    r.clear(700.)?;
    r.navigate(MALLET_LANDING_FROM)?;
    r.clear(700.)
}
fn run(r: &mut Route, secret: bool) -> Result<()> {
    r.tactics = true;
    r.wait(1.)?;
    for goal in [
        vec3(-1888., 2290., -512.),
        vec3(-2340., 2368., -480.),
        vec3(-2560., 2368., -480.),
        vec3(-2340., 2368., -480.),
        vec3(-2256., 2544., -480.),
        vec3(-2304., 2624., -342.),
        vec3(-2304., 2800., -256.),
        vec3(-2304., 3248., -256.),
        vec3(-1664., 3248., -256.),
        vec3(-1664., 3008., -256.),
        vec3(-1480., 3008., -256.),
        vec3(-208., 3008., -256.),
        vec3(-208., 3456., -256.),
    ] {
        if (goal.x == -2304. && goal.y < 3000.)
            || (goal.x == -1664. && goal.y == 3008.)
            || goal.x == -1480.
        {
            r.navigate(goal)?;
        } else {
            r.walk(goal, goal.x == -2340.)?;
        }
        r.clear(700.)?;
        if goal == MALLET_LANDING_FROM {
            fetch_mallet(r)?;
        }
        if secret && goal == vec3(-2560., 2368., -480.) {
            r.navigate(vec3(-2554., 2624., -480.))?;
            r.clear(700.)?;
            r.wait(5.)?;
            r.clear(700.)?;
            r.shoot_switch("Open_Bookcase_Goodie")?;
            anyhow::ensure!(
                r.interactions.school.as_ref().unwrap().secret_open,
                "Secret switch did not open its panel"
            );
            r.navigate(goal)?;
        }
    }
    for goal in [
        vec3(-208., 3968., -256.),
        vec3(208., 3968., -288.),
        vec3(208., 3520., -288.),
        vec3(700., 3520., -264.),
        vec3(1000., 3360., -256.),
        vec3(944., 3440., -256.),
        vec3(700., 3440., -256.),
        vec3(208., 3440., -288.),
        vec3(208., 4160., -264.),
        vec3(144., 4230., -288.),
        vec3(144., 4304., -264.),
    ] {
        if secret && goal == vec3(944., 3440., -256.) {
            r.navigate(vec3(624., 2810., -256.))?;
            r.navigate(vec3(624., 2581., -240.))?;
            r.wait(0.5)?;
            // The Darkened Looking Glass lasts 45 seconds; a harder difficulty shortens every
            // power-up (`Difficulty::duration`), and five seconds of the run are already spent.
            anyhow::ensure!(
                r.stats.invisible
                    > r.difficulty
                        .duration(crate::powerups::Kind::Glass.duration())
                        - 5.,
                "Secret reward was not collected"
            );
            r.navigate(vec3(624., 2810., -256.))?;
            r.navigate(goal)?;
            continue;
        }
        if goal.x == 700. && (goal.y == 3520. || secret) {
            r.navigate(goal)?;
        } else {
            r.walk(goal, goal.y == 4304.)?;
        }
        r.clear(700.)?;
    }
    r.ride_to(64.)?;
    for goal in [
        vec3(272., 4160., 64.),
        vec3(700., 4096., 64.),
        vec3(884., 4056., 64.),
        vec3(0., 4096., 64.),
        vec3(0., 3500., 64.),
        vec3(-140., 3300., 64.),
    ] {
        r.walk(goal, false)?;
        r.clear(700.)?;
    }
    r.navigate(vec3(208., 3300., 400.))?;
    r.navigate(vec3(-32., 3496., 384.))?;
    for goal in [
        vec3(-464., 3520., 384.),
        vec3(-464., 3800., 384.),
        vec3(-464., 4096., 384.),
        vec3(-276., 4160., 384.),
        vec3(-276., 4280., 400.),
    ] {
        r.walk(goal, false)?;
        r.clear(700.)?;
    }
    r.ride_to(696.)?;
    for goal in [
        vec3(-276., 4160., 704.),
        vec3(-212., 3860., 704.),
        vec3(-64., 3968., 704.),
        vec3(-64., 3520., 704.),
        vec3(-160., 3520., 704.),
    ] {
        r.walk(goal, false)?;
        r.clear(700.)?;
    }
    r.wait(8.5)?;
    for goal in [
        vec3(-150., 3396., 708.),
        vec3(-40., 3186., 724.),
        vec3(136., 3014., 716.),
        vec3(224., 2796., 716.),
        vec3(204., 2450., 704.),
    ] {
        r.navigate(goal)?;
        r.clear(700.)?;
    }
    r.wait(4.)?;
    for goal in [
        vec3(104., 2736., 624.),
        vec3(0., 2824., 564.),
        vec3(-32., 2780., 384.),
        vec3(-32., 2780., 64.),
        vec3(200., 2776., -288.),
    ] {
        r.navigate(goal)?;
        r.clear(700.)?;
    }
    r.wait(8.)?;
    let enemies = r.interactions.encounters.as_ref().unwrap();
    for group in [
        "t188",
        "t186",
        "play_guard1",
        "play_guard2",
        "library_guard6",
        "library_guard10",
    ] {
        anyhow::ensure!(
            enemies.activated.contains(group),
            "School route missed activation {group}"
        );
    }
    anyhow::ensure!(
        r.shots + r.swings + r.cards > 0 && r.damage > 0.,
        "School route did not exercise live combat"
    );
    if secret {
        anyhow::ensure!(
            r.stats.collected.contains("skool1:lookingglass"),
            "Secret pickup was lost"
        );
    }
    anyhow::ensure!(
        r.transition == Some(("skool2".into(), Some("skool2_start1".into()))),
        "School route did not exit"
    );
    Ok(())
}
