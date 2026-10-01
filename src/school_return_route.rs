use crate::{assets::Assets, route::Route};
use anyhow::Result;
use macroquad::prelude::*;
pub fn check(assets: &mut Assets) -> Result<()> {
    crate::school_return::check_gates(assets)?;
    for skip in [false, true] {
        traverse(
            assets,
            crate::inventory::Stats::for_level("skool1", Some("skool1_start2")),
            skip,
        )?;
    }
    Ok(())
}
pub fn chain(assets: &mut Assets) -> Result<()> {
    let stats = crate::school2_route::complete(assets)?;
    anyhow::ensure!(
        stats.school_items.star && stats.school_items.potion,
        "School-two rewards missing"
    );
    for skip in [false, true] {
        traverse(assets, stats.clone(), skip)?;
    }
    Ok(())
}
fn traverse(assets: &mut Assets, mut stats: crate::inventory::Stats, skip: bool) -> Result<()> {
    let before = stats.clone();
    stats.ensure_level_weapons("skool1", Some("skool1_start2"));
    anyhow::ensure!(
        stats.sanity() == before.sanity()
            && stats.will() == before.will()
            && stats.selected() == before.selected()
            && stats.collected == before.collected
            && stats.school_items == before.school_items,
        "Return entry altered resources, selection, pickups or quest rewards"
    );
    for w in 0..10 {
        anyhow::ensure!(stats.copies(w) >= before.copies(w), "Lost owned weapon");
    }
    println!(
        "PASS carryover: {} sanity, {} will, selection {}, {} collected pickups, potion and star",
        stats.sanity(),
        stats.will(),
        stats.selected(),
        stats.collected.len()
    );
    let mut r = Route::enter(
        assets,
        "skool1",
        Some("skool1_start2"),
        stats,
        crate::powerups::Difficulty::Normal,
    )?;
    r.stop_at_exit = true;
    r.skip_cinematics = skip;
    drive(&mut r)?;
    // Leaving for the Pool of Tears must neither reset nor top up what Alice carries.
    let departing = serde_json::to_value(&r.stats)?;
    r.stats
        .ensure_level_weapons("potears1", Some("potears1_start1"));
    anyhow::ensure!(
        serde_json::to_value(&r.stats)? == departing,
        "Onward exit reset inventory/resources"
    );
    anyhow::ensure!(r.teleports == 0, "Unintended traversal teleport");
    println!(
        "Return combat: {} throws, {} damage; {} jumps; pickups {:?}",
        r.shots, r.damage, r.jumps, r.stats.collected
    );
    println!("PASS RETURN: {} ticks {} sanity", r.ticks, r.stats.sanity());
    Ok(())
}
/// The return visit from the school entrance to the observatory and the exit to the Pool of
/// Tears. Alice starts with whatever she carried out of school two.
pub fn drive(r: &mut Route) -> Result<()> {
    r.wait(1.)?;
    println!("RETURN ENTRANCE {:?}", r.player.feet);
    r.walk_xy(vec2(-1888., 3290.))?;
    r.wait(5.)?;
    r.navigate(vec3(-1888., 2890., -480.))?;
    r.walk_xy(vec2(-1888., 2290.))?;
    for goal in [vec3(-2340., 2368., -480.), vec3(-2256., 2544., -480.)] {
        r.walk(goal, true)?;
    }
    for goal in [vec3(-2304., 2624., -342.), vec3(-2304., 2800., -256.)] {
        r.navigate(goal)?;
    }
    r.navigate(vec3(-2272., 2440., -256.))?;
    r.navigate(vec3(-2304., 2800., -256.))?;
    for goal in [vec3(-2304., 3248., -256.), vec3(-1664., 3248., -256.)] {
        r.walk(goal, false)?;
    }
    for goal in [vec3(-1664., 3008., -256.), vec3(-1480., 3008., -256.)] {
        r.navigate(goal)?;
    }
    for goal in [
        vec3(-208., 3008., -256.),
        vec3(-208., 3968., -256.),
        vec3(208., 3968., -288.),
        vec3(208., 4160., -264.),
        vec3(144., 4230., -288.),
        vec3(144., 4304., -264.),
    ] {
        r.walk(goal, goal.y == 4304.)?;
    }
    r.ride_to(64.)?;
    for goal in [vec3(272., 4160., 64.), vec3(0., 4096., 64.)] {
        r.walk(goal, false)?;
    }
    r.navigate(vec3(0., 3500., 64.))?;
    r.walk(vec3(-140., 3300., 64.), false)?;
    r.navigate(vec3(208., 3300., 400.))?;
    r.navigate(vec3(400., 3520., 384.))?;
    r.walk(vec3(692., 4096., 384.), false)?;
    r.walk(vec3(692., 4170., 384.), false)?;
    r.wait(1.5)?;
    r.walk(vec3(692., 4360., 416.), false)?;
    r.wait_for_cinematic()?;
    r.wait(6.)?;
    println!("AT OBSERVATORY {:?}", r.player.feet);
    r.walk(vec3(692., 4160., 1024.), false)?;
    r.walk(vec3(692., 3860., 1024.), false)?;
    for goal in [vec2(692., 3500.), vec2(-280., 3500.), vec2(-280., 3860.)] {
        r.walk_xy(goal)?;
    }
    r.navigate(vec3(-280., 4078., 1204.))?;
    r.walk_xy(vec2(-280., 4210.))?;
    r.navigate(vec3(208., 4240., 1280.))?;
    r.navigate(vec3(340., 4396., 1344.))?;
    // Let the final jump land inside the exit volume before waiting on its scene.
    r.wait(1.)?;
    r.wait_for_cinematic()?;
    anyhow::ensure!(
        r.transition == Some(("potears1".into(), Some("potears1_start1".into()))),
        "Return exit failed"
    );
    anyhow::ensure!(
        !r.stats.school_items.star && !r.stats.school_items.potion,
        "Return did not spend the two quest rewards"
    );
    Ok(())
}
