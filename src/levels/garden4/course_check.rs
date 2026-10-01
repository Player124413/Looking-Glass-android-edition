use super::*;
pub(super) fn check(a: &mut Assets) -> Result<()> {
    for difficulty in crate::powerups::Difficulty::ALL {
        let mut map = Bsp::parse(&a.read("maps/garden4.bsp")?)?;
        map.difficulty = difficulty;
        let mut g = Garden::load(a, &map)?;
        let mut w = World::from_bsp(&map)?;
        w.set_dynamic(g.colliders());
        let mut p = Player::spawn(
            &w,
            crate::interaction::spawn(&map, Some("garden4_start1")).0,
        )
        .context("Ice entrance obstructed")?;
        ensure!(
            g.objects.len() == 36 && g.objects.iter().filter(|o| o.solid).count() == 35,
            "Missing ice geometry"
        );
        ensure!(
            g.objects
                .iter()
                .filter(|o| o.name.starts_with("brokenwall"))
                .all(|o| o.solid),
            "Wall missing at arrival"
        );
        for n in ["Ice_Fall1", "Ice_Fall2", "Ice_Fall3"] {
            g.event(n);
        }
        ensure!(
            g.course().rocks[0].started == !g.data.easy
                && g.course().rocks[1].started == !g.data.easy,
            "Wrong difficulty hazards"
        );
        for _ in 0..120 * 3 {
            g.advance(1. / 120., &map, &mut w, &mut p, &[])?;
        }
        ensure!(
            g.course().crushed[2].is_some() && g.course().crushed[3].is_some(),
            "Altar rocks did not crush ice"
        );
        ensure!(
            g.course().crushed[0].is_some() == !g.data.easy
                && g.course().crushed[1].is_some() == !g.data.easy,
            "Missing early floor collapse"
        );
        ensure!(
            g.course().rocks[2..=3].iter().all(|r| !r.solid),
            "Harmless rocks became solid"
        );
        let saved = g.snapshot();
        let transforms = g.transforms();
        g.advance(0., &map, &mut w, &mut p, &[])?;
        ensure!(
            g.snapshot() == saved && g.transforms() == transforms,
            "Pause advanced falling floor"
        );
        let mut restored = Garden::load(a, &map)?;
        restored.restore(&saved, &map)?;
        let mut rw = World::from_bsp(&map)?;
        rw.set_dynamic(restored.colliders());
        let mut rp = p.clone();
        for _ in 0..120 * 7 {
            g.advance(1. / 120., &map, &mut w, &mut p, &[])?;
            restored.advance(1. / 120., &map, &mut rw, &mut rp, &[])?;
        }
        ensure!(
            g.snapshot() == restored.snapshot() && g.transforms() == restored.transforms(),
            "Restored collapse diverged"
        );
        g.event("Garden4_Boulder_End");
        for _ in 0..120 * 30 {
            g.advance(1. / 120., &map, &mut w, &mut p, &[])?;
        }
        ensure!(
            g.course().wall.is_some()
                && g.objects
                    .iter()
                    .filter(|o| o.name.starts_with("brokenwall"))
                    .all(|o| !o.solid && o.pose.is_none()),
            "Rolling boulder did not open wall"
        );
        g.output(&crate::event::Effect::Trigger(crate::entity::Id(78)));
        g.advance(1. / 120., &map, &mut w, &mut p, &[])?;
        ensure!(
            g.objects.iter().any(|o| o.name == "block_back" && o.solid),
            "Fog contact did not seal return route"
        );
        let good = g.snapshot();
        let mut bad = good.clone();
        bad["course"]["rocks"][0]["node"] = serde_json::json!(900);
        ensure!(
            g.restore(&bad, &map).is_err() && g.snapshot() == good,
            "Invalid rock restore mutated course"
        );
        println!("PASS {difficulty:?} ice floors, rock callbacks, harmless hazards, wall, backstop, pause and restored motion");
    }
    for hz in [30, 60, 144] {
        let map = Bsp::parse(&a.read("maps/garden4.bsp")?)?;
        let mut g = Garden::load(a, &map)?;
        let mut w = World::from_bsp(&map)?;
        w.set_dynamic(g.colliders());
        let mut p = Player::spawn(
            &w,
            crate::interaction::spawn(&map, Some("garden4_start1")).0,
        )
        .unwrap();
        g.event(course::MARBLE);
        ensure!(
            g.scene_id().is_none(),
            "Original unskippable marble exposed skip"
        );
        for _ in 0..hz * 8 {
            g.advance(1. / hz as f32, &map, &mut w, &mut p, &[])?;
            if g.course().marble_done {
                break;
            }
        }
        ensure!(
            g.course().marble_done
                && w.body_clear(p.feet)
                && p.feet.distance(g.course().rocks[6].position) > 240.,
            "Unsafe marble handoff at {hz}Hz"
        );
        ensure!(
            g.objects
                .iter()
                .filter(|o| matches!(
                    o.name.as_str(),
                    "break_floor6" | "break_floor7" | "break_floor8" | "break_floor9"
                ))
                .all(|o| !o.solid),
            "Marble left lower ice present"
        );
        println!(
            "PASS {hz}Hz marble handoff at {:?}, lead {:.1}",
            p.feet,
            p.feet.distance(g.course().rocks[6].position)
        );
    }
    legacy(a)?;
    Ok(())
}

fn legacy(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/garden4.bsp")?)?;
    for consumed in [false, true] {
        let mut i = Interactions::load(&map)?;
        i.set_entry(a, &map, "garden4", Some("garden4_start1"))?;
        if consumed {
            // The marble takes control, so contact the wall trigger before it in this fixture.
            for id in [254, 37, 110] {
                let at = data::at(&map.entities[id]).translation - crate::collision::PLAYER_CENTER;
                i.triggers(1. / 120., at, at);
            }
        }
        let mut old = serde_json::to_value(i.snapshot())?;
        old["levels"]["garden4"]
            .as_object_mut()
            .unwrap()
            .remove("course");
        let snapshot = serde_json::from_value(old)?;
        let mut restored = Interactions::load(&map)?;
        restored.set_entry(a, &map, "garden4", Some("garden4_start1"))?;
        restored.restore(&snapshot, &map)?;
        let g = owner(&mut restored)?;
        ensure!(
            g.course().wall.is_some() == consumed
                && g.course().marble_done == consumed
                && !g.ready(),
            "Old course history lost or granted premature portal"
        );
        state::State::validate(&g.saved, state::Visit { returning: false })?;
        for (d, rock) in g.data.rocks.iter().zip(&g.course().rocks) {
            d.validate(rock)?;
        }
    }
    println!("PASS scene-only save migration preserves fresh and consumed cave events without opening the portal");
    Ok(())
}
pub(super) fn stage(
    case: &str,
    a: &mut Assets,
    map: &Bsp,
    i: &mut Interactions,
    w: &mut World,
    p: &mut Player,
) -> Result<Story> {
    *p = Player::spawn(w, crate::interaction::spawn(map, Some("garden4_start1")).0)
        .context("Ice save entrance obstructed")?;
    let (event, time) = match case {
        "garden4-ice-fall" => ("Ice_Fall1", 0.4),
        "garden4-ice-collapse" => ("Ice_Fall3", 2.3),
        "garden4-marble" => (course::MARBLE, 4.),
        "garden4-wall" => ("Garden4_Boulder_End", 60.),
        _ => anyhow::bail!("Unknown ice fixture"),
    };
    owner(i)?.event(event);
    for _ in 0..(time * 120.) as usize {
        i.advance_school(1. / 120., map, w, p)?;
        if case == "garden4-wall" {
            let g = owner(i)?;
            if g.course().wall.is_some_and(|t| g.course().age - t > 0.3) {
                break;
            }
        }
    }
    Ok(Story::load(a, "garden4"))
}
