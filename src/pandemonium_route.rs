//! Live normal-start input route; authored cinematic transfers are counted explicitly.
use crate::{assets::Assets, route::Route};
use anyhow::Result;
use macroquad::prelude::*;
pub fn check(assets: &mut Assets, skip: bool) -> Result<()> {
    let mut r = Route::new(assets, "pandemonium", None)?;
    r.skip_cinematics = skip;
    drive(&mut r)?;
    let next = Route::new(assets, "fortress1", Some("fortress1_start1"))?;
    anyhow::ensure!(
        next.world.body_clear(next.player.feet),
        "Fortress entrance obstructed"
    );
    println!("Fortress 1 normal arrival {:?}", next.player.feet);
    for a in &r.interactions.encounters.as_ref().unwrap().actors {
        if let crate::encounters::Enemy::Guard(g) = &a.enemy {
            println!("Guard {} active={} health={}", a.name, a.active, g.health);
        }
    }
    println!("Cinematic skipping: {skip}");
    println!(
        "PASS Pandemonium normal route: {} ticks, {} jumps, {} throws, {} teleports, sanity {}",
        r.ticks,
        r.jumps,
        r.shots,
        r.teleports,
        r.stats.sanity()
    );
    Ok(())
}
/// The normal start to the departure exit. The caller sets `skip_cinematics` for the skipped run.
pub fn drive(r: &mut Route) -> Result<()> {
    r.wait(0.5)?;
    println!("Pandemonium normal start {:?}", r.player.feet);
    r.walk(vec3(-3400., 2190., -32.), false)?;
    for tick in 0..400 {
        let can_grab = r
            .interactions
            .pandemonium
            .as_ref()
            .unwrap()
            .prompt(&r.world, r.player.eye())
            .is_some();
        r.use_pressed = can_grab;
        r.tick(crate::movement::Controls {
            wish: (vec2(-3552., 2032.) - r.player.feet.truncate()).normalize_or_zero(),
            run: true,
            jump: tick == 0,
            ..Default::default()
        })?;
        if r.interactions.pandemonium.as_ref().unwrap().controlled() {
            println!("ROPE GRABBED {:?}", r.player.feet);
            break;
        }
    }
    anyhow::ensure!(
        r.interactions.pandemonium.as_ref().unwrap().controlled(),
        "Rope not grabbed"
    );
    for _ in 0..120 {
        r.tick(crate::movement::Controls {
            rise: -1.,
            ..Default::default()
        })?;
    }
    r.use_pressed = true;
    r.tick(crate::movement::Controls::default())?;
    r.navigate(vec3(-3920., 2300., -280.))?;
    r.navigate(vec3(-4152., 2544., -280.))?;
    r.wait(10.)?;
    r.walk(vec3(-4500., 2560., -264.), false)?;
    r.walk(vec3(-5000., 2560., -240.), false)?;
    r.navigate(vec3(-5024., 2200., -160.))?;
    r.navigate(vec3(-5024., 2080., -133.))?;
    r.navigate(vec3(-5024., 1840., -92.))?;
    r.navigate(vec3(-5024., 1600., -25.))?;
    r.walk(vec3(-5024., 1368., 32.), false)?;
    r.wait(65.)?;
    println!("CART END {:?}", r.player.feet);
    anyhow::ensure!(
        r.interactions.pandemonium.as_ref().unwrap().state.rides == 1,
        "Cart did not finish"
    );
    r.walk(vec3(-3264., -560., 440.), false)?;
    r.wait(8.)?;
    r.navigate(vec3(-3378., -1040., 424.))?;
    r.walk(vec3(-3378., -1234., 448.), false)?;
    r.wait(3.)?;
    r.walk(vec3(-3412., -1036., 440.), false)?;
    for id in [598, 601, 599, 600, 602, 603, 604, 605, 606, 607, 608, 609] {
        let point = crate::interaction::vector(&r.map.entities[id]["origin"]).unwrap();
        r.walk(point, false)?;
    }
    for _ in 0..240 {
        if r.interactions.pandemonium.as_ref().unwrap().state.key {
            break;
        }
        r.tick(crate::movement::Controls {
            wish: (vec2(-4312., -2528.) - r.player.feet.truncate()).normalize_or_zero(),
            ..Default::default()
        })?;
    }
    r.wait(3.)?;
    println!(
        "KEY {} at {:?}",
        r.interactions.pandemonium.as_ref().unwrap().state.key,
        r.player.feet
    );
    anyhow::ensure!(
        r.interactions.pandemonium.as_ref().unwrap().state.key,
        "Key was not collected"
    );
    for id in [610, 611, 612, 613, 614, 615, 616, 617, 618, 619] {
        let point = crate::interaction::vector(&r.map.entities[id]["origin"]).unwrap();
        r.walk(point, false)?;
    }
    for _ in 0..600 {
        if r.teleports > 0 {
            break;
        }
        r.tick(crate::movement::Controls {
            wish: (vec2(-3674., -2276.) - r.player.feet.truncate()).normalize_or_zero(),
            ..Default::default()
        })?;
    }
    anyhow::ensure!(r.teleports == 1, "Return portal not reached");
    r.wait(3.)?;
    println!("RETURN {:?} teleports {}", r.player.feet, r.teleports);
    r.walk_xy(vec2(-3600., 1100.))?;
    r.walk_xy(vec2(-3600., 1700.))?;
    r.navigate(vec3(-3800., 1900., -384.))?;
    r.navigate(vec3(-3920., 2300., -280.))?;
    r.walk(vec3(-3968., 2560., -232.), false)?;
    r.walk(vec3(-3968., 2624., -232.), false)?;
    for _ in 0..600 {
        if r.interactions.pandemonium.as_ref().unwrap().state.departure {
            break;
        }
        r.tick(crate::movement::Controls {
            wish: Vec2::Y,
            ..Default::default()
        })?;
    }
    r.wait(30.)?;
    anyhow::ensure!(
        r.transition == Some(("fortress1".into(), Some("fortress1_start1".into()))),
        "Departure transition missing"
    );
    anyhow::ensure!(
        r.stats.copies(1) > 0 && r.shots > 0,
        "Route missed Cards or combat"
    );
    Ok(())
}
