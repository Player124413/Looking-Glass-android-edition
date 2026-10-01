//! Real-map lifecycle and contact checks, separate from the continuous swimming route.
use super::*;
use crate::movement::{Controls, FIXED_DT};

pub(super) fn check(a: &mut Assets, map: &Bsp, gate: &serde_json::Value) -> Result<()> {
    let mut t = Temple::load(a, map)?;
    let mut w = World::from_bsp(map)?;
    w.set_dynamic(t.colliders());
    w.set_dynamic_liquids(t.liquids());
    let mut p = Player::new(data::origin(&map.entities[17]));
    let mut stats = Stats::for_level("utemple", None);
    stats.damage(63.);
    stats.spend_will(59.);
    t.prepare_player(&mut stats, &mut p);
    ensure!(
        stats.sanity() == 100. && stats.will() == 100. && p.breath.shell,
        "Fresh temple grant missing"
    );
    stats.damage(63.);
    stats.spend_will(59.);
    t.prepare_player(&mut stats, &mut p);
    ensure!(
        stats.sanity() == 37. && stats.will() == 41.,
        "Arrival resources refilled twice"
    );
    t.event(ID);
    ensure!(
        !t.scripted() && t.event("breakwall").is_none(),
        "Raw event bypassed guide gate"
    );
    t.event("startturtle");
    let start = t.saved.guide;
    t.event("startturtle");
    ensure!(t.saved.guide == start, "Guide restarted");

    // Waiting protection must end. Use the actual Player update and its breath damage.
    for _ in 0..120 * 22 {
        t.prepare_player(&mut stats, &mut p);
        p.tick(&w, Controls::default());
    }
    ensure!(p.breath.hits > 0, "Leaving the guide gave unlimited air");
    t.emit();
    let hidden = t
        .data
        .emitters
        .iter()
        .find(|(_, n, _, _)| n == "panel11_bubbles")
        .unwrap();
    ensure!(
        !t.emitter_visible(&hidden.1) && hidden.3,
        "Hidden air fixture changed"
    );
    p.feet = hidden.2 - PLAYER_CENTER;
    p.breath.submerged = 19.;
    t.breathe(&mut p);
    ensure!(
        p.breath.remaining() == 20.,
        "Hidden launcher lost server breath"
    );
    let no_air = t.data.emitters.iter().find(|(_, _, _, air)| !*air).unwrap();
    p.feet = no_air.2 - PLAYER_CENTER;
    p.breath.submerged = 19.;
    t.breathe(&mut p);
    ensure!(
        p.breath.remaining() == 1.,
        "Visual-only emitter supplied air"
    );
    let first = t.saved.bubbles[0].clone();
    t.saved.clock = 0.9;
    t.emit();
    ensure!(
        t.saved.bubbles[0].at == first.at,
        "Old breath point followed the emitter"
    );
    t.saved.clock = 4.;
    t.emit();
    ensure!(
        t.saved.bubbles.iter().all(|b| b.born > 0.) && t.saved.bubbles.len() == 20,
        "Breath cadence or expiration changed"
    );

    // The added brush is actual swimming water, never a hidden solid stand-in.
    let at = t
        .data
        .guide
        .nodes
        .iter()
        .find(|n| n.0 == "tp105")
        .unwrap()
        .2;
    p = Player::new(at);
    p.breath.shell = true;
    let before = p.feet;
    for _ in 0..30 {
        p.tick(
            &w,
            Controls {
                swim: Vec3::Z,
                ..Default::default()
            },
        );
    }
    ensure!(
        p.immersion.level == 3 && p.feet.z > before.z + 1. && p.breath.submerged > 0.,
        "Brush did not drive real swimming/breath"
    );
    let mut dry = World::from_bsp(map)?;
    dry.set_dynamic(t.colliders());
    ensure!(
        crate::water::Immersion::sample(&dry, at).level != 3,
        "Brush-water fixture already wet without adapter"
    );

    // F2 re-arms an actually reported pending exit, but keeps its new guide gate shut.
    let feet = vec3(-2144., 3352., 480.);
    let mut old = Interactions::load(map)?;
    old.triggers(0.01, feet, feet);
    let old_json = serde_json::to_value(old.snapshot())?;
    ensure!(
        old_json["triggers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == 30 && s["reported"] == true),
        "Legacy exit fixture not pending"
    );
    let mut current = Interactions::load(map)?;
    current.set_entry(a, map, "utemple", None)?;
    current.restore(&old.snapshot(), map)?;
    let now = serde_json::to_value(current.snapshot())?;
    ensure!(
        now["triggers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == 30 && s["reported"] == false && s["fired"] == false),
        "Pending temple exit not rearmed"
    );
    current.triggers(0.01, feet, feet);
    ensure!(!current.scripted(), "Migrated exit bypassed guide");
    current.prepare_player(&mut stats, &mut p);
    ensure!(
        stats.sanity() == 37. && stats.will() == 41.,
        "Upgrade refilled old resources"
    );

    // Authored contact, clocks, cameras, restoration and one completion path at three rates.
    for hz in [30, 60, 144] {
        for skip in [None, Some(0.), Some(3.5), Some(6.9)] {
            let mut i = Interactions::load(map)?;
            i.set_entry(a, map, "utemple", None)?;
            let owner = i.levels[0].ctl.downcast_mut::<Temple>().unwrap();
            owner.restore(gate, map)?;
            i.sync(&mut w);
            p = Player::new(feet);
            i.triggers(FIXED_DT, feet + Vec3::X * 128., feet);
            ensure!(i.scripted(), "Real temple exit contact failed");
            let mut story = Story::load(a, "utemple");
            for tick in 0..hz * 8 {
                let t = i.levels[0].ctl.downcast_mut::<Temple>().unwrap();
                let s = t.saved.scene.as_ref().unwrap();
                let time = s.time;
                ensure!(
                    t.camera(&w)
                        .is_some_and(|c| c.eye.is_finite() && c.target.is_finite()),
                    "Invalid temple camera"
                );
                if tick % (hz / 2) == 0 {
                    let saved = t.snapshot();
                    t.advance(0., map, &mut w, &mut p, &[])?;
                    ensure!(
                        saved == t.snapshot(),
                        "Paused scene moved water/actors/breath"
                    );
                    t.restore(&saved, map)?;
                }
                if skip.is_some_and(|at| time >= at) {
                    t.skip(map, &mut w, &mut p, &mut story)?;
                }
                t.advance(1. / hz as f32, map, &mut w, &mut p, &[])?;
                if t.saved.scene.as_ref().unwrap().finished {
                    break;
                }
            }
            let t = i.levels[0].ctl.downcast_mut::<Temple>().unwrap();
            ensure!(
                t.saved.scene.as_ref().unwrap().finished && t.scripted(),
                "Ending released control early"
            );
            ensure!(
                t.update(&mut w, &p, Vec3::X, false).transition == Some(EXIT.destination()),
                "Ending lost garden exit"
            );
            ensure!(
                t.update(&mut w, &p, Vec3::X, false).transition.is_none(),
                "Ending duplicated exit"
            );
            t.transition_failed(&EXIT.destination());
            for _ in 0..132 {
                t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
            }
            ensure!(
                t.update(&mut w, &p, Vec3::X, false).transition == Some(EXIT.destination()),
                "Failed exit did not retry"
            );
            let saved = t.snapshot();
            t.restore(&saved, map)?;
            ensure!(
                t.update(&mut w, &p, Vec3::X, false).transition == Some(EXIT.destination()),
                "Restored commit lost delivery"
            );
            ensure!(
                !t.skip(map, &mut w, &mut p, &mut story)?,
                "Completed scene skipped twice"
            );
        }
    }
    println!("PASS temple real water/breath contacts, air expiry, pending-trigger migration, resources, 12 scene outcomes and failed-exit retry");
    Ok(())
}
