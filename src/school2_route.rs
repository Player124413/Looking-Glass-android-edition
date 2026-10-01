//! Continuous school-two input route, including its ordinary authored teleports.
//!
//! It runs on the shared [`Route`], so school two's controller, its placed guards, its quest items
//! and its exit go through the same gating and story deferral as the viewer, and the route can
//! start from the resources an earlier visit left Alice with (`complete_from`, the chain).
use crate::{
    assets::Assets,
    inventory::Stats,
    movement::{Controls, FIXED_DT},
    route::Route,
};
use anyhow::Result;
use macroquad::prelude::*;

pub fn check(assets: &mut Assets) -> Result<()> {
    complete(assets).map(|_| ())
}
/// The route from a fresh entrance with school two's own baseline resources.
pub fn complete(assets: &mut Assets) -> Result<Stats> {
    complete_from(assets, Stats::for_level("skool2", None))
}
/// The route from the school-two entrance with Alice carrying `stats` from an earlier visit.
/// Returns the resources she leaves with, including the two quest rewards.
pub fn complete_from(assets: &mut Assets, stats: Stats) -> Result<Stats> {
    let mut r = Route::enter(
        assets,
        "skool2",
        None,
        stats,
        crate::powerups::Difficulty::Normal,
    )?;
    drive(&mut r)?;
    summary(&r);
    Ok(r.stats)
}
fn summary(r: &Route) {
    println!("PASS FULL SCHOOL TWO: {} ticks, {} jumps, {} throws, {} swings, {} cards, {} combat damage, {} sanity; normal entrance, Elder Gnome, 3 Boojums, rescue, Jumbogrow, growth, lollipop, potion + star, gated return to skool1_start2; 2 authored portals, no cheats/recovery/refills",r.ticks,r.jumps,r.shots,r.swings,r.cards,r.damage,r.stats.sanity());
}
/// Walk each waypoint in turn, fighting whatever wakes up on the way.
fn stroll(r: &mut Route, goals: &[Vec3]) -> Result<()> {
    for &goal in goals {
        r.walk(goal, false)?;
        r.clear(700.)?;
    }
    Ok(())
}
fn portal(r: &mut Route, goal: Vec3) -> Result<()> {
    let count = r.teleports;
    for _ in 0..2400 {
        let delta = goal - r.player.feet;
        r.tick(Controls {
            wish: delta.truncate().normalize_or_zero(),
            ..Default::default()
        })?;
        if r.teleports > count {
            return Ok(());
        }
    }
    anyhow::bail!("Authored portal unreachable at {:?}", r.player.feet)
}
/// Entrance to the gated return exit; the route ends when the exit is taken.
pub fn drive(r: &mut Route) -> Result<()> {
    r.wait(1.)?;
    r.walk(vec3(-64., -3100., 0.), false)?;
    // The basement altar is an authored weapon pickup, required by the strict
    // campaign. Visit it with ordinary movement before climbing into the gym.
    if !r.stats.collected.contains("skool2:44") {
        let approach: Vec<[f32;3]> = if let Ok(path) = std::env::var("LOOKING_GLASS_SCHOOL2_DICE_APPROACH") {
            serde_json::from_slice(&std::fs::read(path)?)?
        } else { vec![[-320.,-2800.,0.],[-600.,-2560.,0.],[-800.,-2560.,0.],[-872.,-2496.,0.],[-1000.,-2496.,-48.],[-1120.,-2496.,-80.],[-1120.,-2608.,-112.],[-1120.,-2736.,-176.],[-1120.,-2816.,-192.],[-992.,-2816.,-192.]] };
        for &goal in &approach { r.walk(Vec3::from_array(goal),false)?; }
        // The altar sits across a real floor gap; the physics planner must jump it.
        r.navigate(vec3(-906.,-3170.,-192.))?;
        r.wait_for_cinematic()?;
        r.clear(700.)?;
        anyhow::ensure!(r.stats.collected.contains("skool2:44") && r.stats.copies(6) > 0,
            "Demon Dice altar was not collected");
        r.navigate(vec3(-992.,-2816.,-192.))?;
        for &goal in approach.iter().rev().skip(1) { r.walk(Vec3::from_array(goal),false)?; }
        r.walk(vec3(-64.,-3100.,0.),false)?;
    }
    for goal in [
        vec3(-64., -3100., 0.),
        vec3(250., -2800., 0.),
        vec3(480., -2560., 0.),
        vec3(800., -2560., 0.),
        vec3(1150., -2560., 0.),
        vec3(1030., -2560., 0.),
        vec3(1030., -2880., 0.),
        vec3(1200., -2880., 0.),
        vec3(1470., -2880., 0.),
    ] {
        r.walk(goal, false)?;
        r.clear(700.)?;
    }
    stroll(
        r,
        &[
            vec3(1470., -3040., 0.),
            vec3(2150., -3040., 0.),
            vec3(2150., -2000., 0.),
        ],
    )?;
    r.navigate(vec3(2163., -1859., 64.))?;
    let lever = vec3(2244., -1859., 90.);
    r.interactions.update(
        FIXED_DT,
        &r.map,
        &mut r.world,
        &r.player,
        (lever - r.player.eye()).normalize(),
        true,
    )?;
    anyhow::ensure!(
        r.interactions.gym.as_ref().unwrap().used,
        "Lever not reachable at {:?}",
        r.player.feet
    );
    r.wait(7.)?;
    r.navigate(vec3(2150., -2080., 0.))?;
    r.walk(vec3(2400., -2150., 160.), false)?;
    r.walk(vec3(2440., -2150., 192.), false)?;
    r.navigate(vec3(2270., -2040., 250.))?;
    r.walk(vec3(1950., -2016., 400.), false)?;
    r.walk(vec3(1710., -2000., 448.), false)?;
    r.navigate(vec3(1600., -2275., 440.))?;
    for goal in [
        vec3(1750., -2275., 440.),
        vec3(1472., -2560., 440.),
        vec3(1710., -2800., 440.),
    ] {
        r.walk(goal, false)?;
    }
    r.navigate(vec3(1880., -2940., 440.))?;
    r.walk(vec3(2077., -3090., 440.), false)?;
    // The three Boojums have to be defeated before the laboratory opens: fight them, then let
    // the quest notice.
    for _ in 0..40 {
        r.clear(1400.)?;
        if r.interactions.school2.as_ref().unwrap().quest.stage
            == crate::school2_quest::Stage::Laboratory
        {
            break;
        }
        r.wait(1.)?;
    }
    anyhow::ensure!(
        r.interactions.school2.as_ref().unwrap().quest.stage
            == crate::school2_quest::Stage::Laboratory,
        "Boojum battle did not finish {:?}",
        r.interactions.school2.as_ref().unwrap().quest.stage
    );
    println!(
        "PASS entrance, lever, upper gym and three Boojums: {} ticks, {} shots, {} swings, {} cards, {} damage taken",
        r.ticks, r.shots, r.swings, r.cards, r.damage
    );
    // Leave the way Alice came up. Stepping off the balcony saves a walk but costs 42 Sanity
    // (a 440-unit fall), which the fights that follow cannot spare.
    r.walk(vec3(1880., -2940., 440.), false)?;
    for goal in [
        vec3(1710., -2800., 440.),
        vec3(1472., -2560., 440.),
        vec3(1750., -2275., 440.),
        vec3(1600., -2275., 440.),
        vec3(1710., -2000., 448.),
        vec3(1950., -2016., 400.),
        vec3(2270., -2040., 250.),
        vec3(2440., -2150., 192.),
        vec3(2400., -2150., 160.),
        vec3(2150., -2080., 0.),
    ] {
        r.navigate(goal)?;
        r.clear(700.)?;
    }
    stroll(
        r,
        &[
            vec3(2150., -3040., 0.),
            vec3(1470., -3040., 0.),
            vec3(1470., -2880., 0.),
            vec3(1440., -2880., 0.),
            vec3(1040., -2880., 0.),
            vec3(1040., -2560., 0.),
            vec3(480., -2560., 0.),
            vec3(250., -2290., 0.),
            vec3(-64., -2060., 0.),
        ],
    )?;
    r.navigate(vec3(-64., -1960., 256.))?;
    r.clear(700.)?;
    r.walk(vec3(-64., -1270., 256.), false)?;
    r.clear(700.)?;
    r.wait(8.)?;
    r.clear(700.)?;
    r.walk(vec3(-64., -1020., 256.), false)?;
    r.clear(700.)?;
    r.wait(18.)?;
    r.clear(700.)?;
    anyhow::ensure!(
        r.interactions.school2.as_ref().unwrap().quest.stage
            == crate::school2_quest::Stage::Jumbogrow,
        "Rescue did not complete {:?}",
        r.interactions.school2.as_ref().unwrap().quest.stage
    );
    r.walk(vec3(40., -930., 256.), false)?;
    r.walk(vec3(150., -600., 256.), false)?;
    r.walk(vec3(36., -452., 256.), false)?;
    r.wait(1.)?;
    anyhow::ensure!(
        r.interactions
            .school2
            .as_ref()
            .unwrap()
            .quest
            .items
            .jumbogrow,
        "Jumbogrow pickup unreachable"
    );
    println!(
        "PASS rescue, Spice Drops and Jumbogrow: sanity {}",
        r.stats.sanity()
    );
    for goal in [
        vec3(150., -600., 256.),
        vec3(40., -930., 256.),
        vec3(-64., -1270., 256.),
        vec3(-64., -1504., 256.),
    ] {
        r.walk(goal, false)?;
    }
    portal(r, vec3(-312., -1504., 256.))?;
    for goal in [
        vec3(-1120., -2540., 512.),
        vec3(-1470., -2560., 512.),
        vec3(-1740., -2560., 512.),
        vec3(-2070., -2560., 512.),
        vec3(-2320., -2560., 512.),
        vec3(-2640., -2560., 512.),
        vec3(-2780., -2560., 512.),
    ] {
        r.walk(goal, false)?;
    }
    r.navigate(vec3(-3010., -2550., 488.))?;
    r.wait(7.)?;
    // The greenhouse Boojums can push Alice away during the growth animation.
    // Approach the now-grown pickup again using ordinary movement.
    if !r
        .interactions
        .school2
        .as_ref()
        .unwrap()
        .quest
        .items
        .lollipop
    {
        r.navigate(vec3(-3010., -2550., 504.))?;
        r.wait(1.)?;
    }
    anyhow::ensure!(
        r.interactions
            .school2
            .as_ref()
            .unwrap()
            .quest
            .items
            .lollipop,
        "Growth or grown lollipop pickup failed"
    );
    println!(
        "PASS greenhouse growth and lollipop collection: sanity {}",
        r.stats.sanity()
    );
    r.navigate(vec3(-2780., -2560., 480.))?;
    r.navigate(vec3(-2680., -2560., 512.))?;
    for goal in [
        vec3(-2320., -2560., 512.),
        vec3(-2070., -2560., 512.),
        vec3(-1740., -2560., 512.),
        vec3(-1470., -2560., 512.),
        vec3(-1120., -2540., 512.),
    ] {
        r.walk(goal, false)?;
    }
    portal(r, vec3(-1112., -2280., 512.))?;
    for goal in [
        vec3(-64., -1504., 256.),
        vec3(-64., -1020., 256.),
        vec3(0., -820., 256.),
    ] {
        r.walk(goal, false)?;
    }
    r.wait(9.)?;
    anyhow::ensure!(
        matches!(
            r.interactions.school2.as_ref().unwrap().quest.stage,
            crate::school2_quest::Stage::Rewards | crate::school2_quest::Stage::Complete
        ),
        "Potion preparation failed"
    );
    r.walk(vec3(-28., -695., 256.), false)?;
    r.walk(vec3(0., -760., 256.), false)?;
    r.wait(1.)?;
    anyhow::ensure!(
        r.interactions.school2.as_ref().unwrap().quest.stage
            == crate::school2_quest::Stage::Complete,
        "Both rewards were not collected"
    );
    r.walk(vec3(-64., -1270., 256.), false)?;
    anyhow::ensure!(
        r.transition == Some(("skool1".into(), Some("skool1_start2".into()))),
        "Wrong final exit"
    );
    anyhow::ensure!(r.teleports == 2, "Unexpected shortcut");
    r.stats.school_items = r.interactions.school2.as_ref().unwrap().quest.items.clone();
    Ok(())
}
