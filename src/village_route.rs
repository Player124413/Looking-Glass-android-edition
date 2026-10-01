//! Continuous village traversal using ordinary walking, jumping, combat and triggers.
use crate::{assets::Assets, route::Route};
use anyhow::Result;
use macroquad::prelude::*;
pub fn check(assets: &mut Assets) -> Result<()> {
    let mut r = Route::new(assets, "gvillage", None)?;
    drive(&mut r)?;
    println!("PASS village opening to authored Pandemonium exit: {} ticks, {} jumps, {} throws, {} combat damage, {} sanity; no flight, warps or recovery",r.ticks,r.player.jumps,r.shots,r.damage,r.stats.sanity());
    Ok(())
}
/// The village from the falling introduction's hand-off to the authored Pandemonium exit.
pub fn drive(r: &mut Route) -> Result<()> {
    anyhow::ensure!(r.stats.equipped().is_none(), "Village must start unarmed");
    r.wait(6.)?;
    for goal in [
        vec3(-4180., 5900., 16.),
        vec3(-4192., 5416., -52.),
        vec3(-4412., 5328., -52.),
        vec3(-4780., 5392., -120.),
        vec3(-4968., 5424., -120.),
    ] {
        r.walk(goal, false)?;
    }
    r.wait(6.)?;
    r.navigate(vec3(-5100., 5200., -120.))?;
    r.navigate(vec3(-5100., 4780., -56.))?;
    r.wait(6.)?;
    for goal in [
        vec3(-4784., 4784., -120.),
        vec3(-4512., 4800., -96.),
        vec3(-4288., 4808., -136.),
        vec3(-3984., 4712., -176.),
        vec3(-3972., 4332., -160.),
        vec3(-3960., 4056., -224.),
        vec3(-3968., 3840., -272.),
        vec3(-3776., 3832., -208.),
        vec3(-3544., 3856., -136.),
    ] {
        r.walk(goal, false)?;
    }
    anyhow::ensure!(
        r.stats.copies(0) == 1,
        "Hallway Blade was not collected through normal traversal"
    );
    anyhow::ensure!(
        r.pickups.iter().any(
            |p| matches!(p.kind, crate::inventory::PickupKind::Weapon(0))
                && r.stats.collected.contains(&p.id)
        ),
        "Blade grant did not come from its authored pickup"
    );
    r.wait(15.)?;
    for goal in [
        vec3(-3208., 4040., -56.),
        vec3(-2800., 3872., -96.),
        vec3(-2808., 3520., -96.),
        vec3(-2744., 3304., -96.),
    ] {
        r.navigate(goal)?;
    }
    r.navigate(vec3(-2936., 3024., 128.))?;
    r.wait(8.)?;
    r.navigate(vec3(-3112., 2584., 40.))?;
    r.walk(vec3(-3312., 2440., 48.), false)?;
    r.wait(8.)?;
    anyhow::ensure!(
        r.transition == Some(("pandemonium".into(), Some("player_start".into()))),
        "Village exit was not reached: {:?}",
        r.transition
    );
    Ok(())
}
