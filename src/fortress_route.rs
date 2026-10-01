//! Continuous movement and combat through both Fortress of Doors visits.
use crate::{assets::Assets, interaction::Interactions, movement::Controls, route::Route};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
/// Plan and walk to `goal`, then deal with whatever woke up on the way.
fn go(r: &mut Route, goal: Vec3) -> Result<()> {
    r.navigate(goal)?;
    r.clear(900.)
}
fn enter_portal(r: &mut Route) -> Result<()> {
    let before = r.teleports;
    for _ in 0..300 {
        r.tick(Controls {
            // Navigation stops within a small radius of its waypoint. Aim into
            // the actual doorway instead of assuming that leaves Y aligned.
            wish: (vec2(-3098., 3068.) - r.player.feet.truncate()).normalize_or_zero(),
            ..Default::default()
        })?;
        if r.teleports > before {
            break;
        }
    }
    ensure!(r.teleports == before + 1, "Fortress portal was not reached");
    Ok(())
}
fn finish_exit(r: &mut Route) -> Result<()> {
    for t in 0..600 {
        let f = r.interactions.fortress.as_ref().unwrap();
        let goal = if f.state.returning {
            f.trigger_pose("s1_changelevel", vec3(-3872., 3598., 488.))
                .unwrap()
                .0
        } else {
            vec3(-360., -2680., 132.)
        };
        r.tick(Controls {
            wish: (goal - r.player.feet).truncate().normalize_or_zero(),
            run: true,
            jump: t % 120 == 0,
            ..Default::default()
        })?;
        if r.transition.is_some() {
            return Ok(());
        }
    }
    anyhow::bail!("Fortress exit not reached at {:?}", r.player.feet)
}
fn state_checks(assets: &mut Assets) -> Result<()> {
    for entry in [None, Some("fortress1_start2")] {
        let mut r = Route::new(assets, "fortress1", entry)?;
        // Upgrade a pre-controller snapshot using the old generic event registry.
        let old = Interactions::load(&r.map)?.snapshot();
        r.interactions.restore(&old, &r.map)?;
        let before = serde_json::to_value(r.interactions.snapshot())?;
        r.interactions
            .advance_school(0., &r.map, &mut r.world, &mut r.player)?;
        ensure!(
            before == serde_json::to_value(r.interactions.snapshot())?,
            "Paused fortress advanced"
        );
        let school = vec3(-3872., 3598., 460.);
        let teleport = vec3(-3098., 3068., 118.);
        if entry.is_none() {
            ensure!(
                r.interactions
                    .triggers(0.01, school, school)
                    .transition
                    .is_none(),
                "Early school exit"
            );
            r.interactions
                .fortress
                .as_mut()
                .unwrap()
                .event("Fortress1_Start_Split");
            for _ in 0..20 {
                r.interactions
                    .advance_school(0.1, &r.map, &mut r.world, &mut r.player)?;
            }
            ensure!(
                r.interactions.fortress.as_ref().unwrap().state.split > 0.19,
                "Room did not move"
            );
        } else {
            ensure!(
                r.interactions
                    .triggers(0.01, teleport, teleport)
                    .teleport
                    .is_none(),
                "Return portal remained open"
            );
        }
        let saved = r.interactions.snapshot();
        let poses = r.interactions.transforms();
        let mut restored = Interactions::load(&r.map)?;
        restored.set_entry(assets, &r.map, "fortress1", entry)?;
        restored.restore(&saved, &r.map)?;
        ensure!(
            poses == restored.transforms(),
            "Fortress pose changed after restore"
        );
        ensure!(
            serde_json::to_value(saved)? == serde_json::to_value(restored.snapshot())?,
            "Fortress state changed after restore"
        );
    }
    println!(
        "PASS fortress visit gates, pause, partial split save/restore and pre-controller migration"
    );
    Ok(())
}
pub fn check(assets: &mut Assets) -> Result<()> {
    state_checks(assets)?;
    let mut r = Route::new(assets, "fortress1", None)?;
    drive_first(&mut r)?;
    println!("PASS fortress arrival to Beyond the Wall: {} ticks, {} jumps, {} blade throws, {} swings, {} cards, {} combat damage, {} Sanity; no flight or recovery",r.ticks,r.player.jumps,r.shots,r.swings,r.cards,r.damage,r.stats.sanity());
    let mut r = Route::new(assets, "fortress1", Some("fortress1_start2"))?;
    drive_return(&mut r)?;
    println!("PASS fortress return to school: {} ticks, {} jumps, {} blade throws, {} swings, {} cards, {} combat damage, {} Sanity; no flight or recovery",r.ticks,r.player.jumps,r.shots,r.swings,r.cards,r.damage,r.stats.sanity());
    Ok(())
}
/// The first visit: arrival, the portal, the split room and the exit to Beyond the Wall.
pub fn drive_first(r: &mut Route) -> Result<()> {
    r.tactics = true;
    r.wait(1.)?;
    go(r, vec3(-3500.0, 2140.0, 0.0))?;
    go(r, vec3(-3600.0, 2430.0, 0.0))?;
    go(r, vec3(-3500.0, 2550.0, 48.0))?;
    go(r, vec3(-3450.0, 2600.0, 48.0))?;
    go(r, vec3(-3380.0, 2700.0, 0.0))?;
    go(r, vec3(-3230.0, 2730.0, 0.0))?;
    go(r, vec3(-3230.0, 3050.0, 0.0))?;
    enter_portal(r)?;
    r.wait(1.0)?;
    go(r, vec3(-360.0, -2410.0, -192.0))?;
    go(r, vec3(-400.0, -2240.0, -192.0))?;
    r.clear(900.)?;
    r.wait(12.0)?;
    go(r, vec3(-448.0, -1810.0, 32.0))?;
    go(r, vec3(-30.0, -1900.0, 36.0))?;
    go(r, vec3(-20.0, -2190.0, 52.0))?;
    go(r, vec3(-5.0, -2430.0, 32.0))?;
    go(r, vec3(-150.0, -2630.0, 32.0))?;
    go(r, vec3(-300.0, -2610.0, 32.0))?;
    finish_exit(r)?;
    ensure!(
        r.transition == Some(("fortress2".into(), None))
            && r.teleports == 1
            && r.interactions.fortress.as_ref().unwrap().state.split == 1.,
        "First fortress visit did not complete"
    );
    Ok(())
}
/// The return visit: the Boojum reveal and the jump to the school.
pub fn drive_return(r: &mut Route) -> Result<()> {
    r.tactics = true;
    r.wait(1.)?;
    go(r, vec3(-4300.0, 2250.0, 510.0))?;
    go(r, vec3(-3800.0, 2250.0, 405.0))?;
    go(r, vec3(-3350.0, 2250.0, 390.0))?;
    go(r, vec3(-3060.0, 2250.0, 376.0))?;
    go(r, vec3(-3060.0, 2520.0, 376.0))?;
    r.walk(vec3(-3060.0, 2660.0, 376.0), true)?;
    go(r, vec3(-3060.0, 2780.0, 376.0))?;
    go(r, vec3(-3060.0, 3340.0, 376.0))?;
    go(r, vec3(-3060.0, 3890.0, 376.0))?;
    go(r, vec3(-3620.0, 3900.0, 398.0))?;
    go(r, vec3(-3810.0, 3850.0, 407.0))?;
    go(r, vec3(-3840.0, 3788.0, 452.0))?;
    r.walk_xy(vec2(-3840., 3788.))?;
    finish_exit(r)?;
    ensure!(
        r.transition == Some(("skool1".into(), Some("skool1_start1".into()))) && r.teleports == 0,
        "Fortress return did not reach school"
    );
    ensure!(
        r.interactions.fortress.as_ref().unwrap().state.shutters,
        "Return route missed the Boojum event"
    );
    ensure!(
        r.interactions
            .encounters
            .as_ref()
            .unwrap()
            .actors
            .iter()
            .filter(|a| a.name.starts_with("s1_booj"))
            .all(|a| a.active),
        "School ambush never activated"
    );
    Ok(())
}
