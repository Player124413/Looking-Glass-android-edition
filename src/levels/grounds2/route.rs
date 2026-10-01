use super::*;
use crate::route::Route;
pub(super) fn survey(r: &Route) -> Result<()> {
    let mut points = vec![];
    for (id, e) in r.map.entities.iter().enumerate() {
        if e.get("classname")
            .is_some_and(|s| s == "info_pathnode" || s.starts_with("Item_"))
        {
            let p = data::pose(e).translation;
            if let Some(at) = r.world.actor_footing(p, PLAYER_CENTER, PLAYER_HALF, 180.) {
                points.push((id, at));
            }
        }
    }
    std::fs::create_dir_all("private/grounds2-work")?;
    std::fs::write(
        "private/grounds2-work/points.json",
        serde_json::to_vec_pretty(&points)?,
    )?;
    println!("Surveyed {} battlefield points", points.len());
    Ok(())
}
pub(super) fn check(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_GROUNDS2_FROM") {
            Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?
        } else if std::env::var_os("LOOKING_GLASS_GROUNDS2_ONLY").is_some() {
            Route::new(a, "grounds2", Some("grounds2_start1"))?
        } else {
            crate::jabberwock::check::play(a, crate::jabberwock::Kind::Grounds)?
        };
        if r.native_cast.is_none() {
            r.enable_native_cast(a)?;
        }
        drive(&mut r)?;
        let stats = serde_json::to_value(&r.stats)?;
        let next = r.depart(a, true)?;
        ensure!(
            next.level().map == "facade"
                && next.level().entry.as_deref() == Some("facade_start1")
                && serde_json::to_value(&next.stats)? == stats
                && next.world.body_clear(next.player.feet),
            "Bad Ascension handoff"
        );
        let expected = Player::spawn(
            &next.world,
            interaction::spawn(&next.map, Some("facade_start1")).0,
        )
        .context("Ascension entrance is obstructed")?;
        ensure!(
            next.player.feet.distance(expected.feet) < 0.01,
            "Ascension arrival moved from its entrance"
        );
        std::fs::write(
            "private/grounds2-work/handoff.json",
            serde_json::to_vec(&next.checkpoint())?,
        )?;
        println!("PASS grounds2 complete combat route and Ascension arrival");
        Ok(())
    })
}

pub(crate) fn drive(r: &mut Route) -> Result<()> {
        r.tactics = true;
        r.heavy_weapon = Some(7);
        r.ice_stream = true;
        r.conserve_will = false;
        r.stop_at_exit = true;
        r.wait_for_cinematic()?;
        survey(&r)?;
        std::fs::write(
            "private/grounds2-work/arrival.json",
            serde_json::to_vec(&r.checkpoint())?,
        )?;
        if let Ok(path) = std::env::var("LOOKING_GLASS_GROUNDS2_ROUTE") {
            let steps: Vec<(String, [f32; 3])> = serde_json::from_slice(&std::fs::read(path)?)?;
            for (mode, xyz) in steps {
                let goal = Vec3::from_array(xyz);
                println!(
                    "GROUNDS {mode} {goal:?} from {:?} health {}",
                    r.player.feet,
                    r.stats.sanity()
                );
                std::fs::write(
                    "private/grounds2-work/last-checkpoint.json",
                    serde_json::to_vec(&r.checkpoint())?,
                )?;
                match mode.as_str() {
                    "clear" => r.clear(goal.x)?,
                    "wait" => r.wait(goal.x)?,
                    "watch" => r.use_watch()?,
                    "blade" => {
                        r.heavy_weapon = None;
                        r.ice_stream = false;
                        r.conserve_will = true;
                    }
                    "staff" => {
                        r.heavy_weapon = Some(7);
                        r.ice_stream = true;
                        r.conserve_will = false;
                    }
                    _ => r.navigate(goal)?,
                };
                ensure!(
                    r.player.immersion.kind != crate::water::Liquid::Lava,
                    "Route touched lava"
                );
            }
        } else {
            let nodes = [267, 273, 275]
                .into_iter()
                .chain(183..=240)
                .chain(351..=372);
            for id in nodes.flat_map(|id| {
                [
                    Some(id),
                    match id {
                        215 => Some(3),
                        231 => Some(388),
                        360 => Some(389),
                        _ => None,
                    },
                ]
                .into_iter()
                .flatten()
            }) {
                let raw = data::pose(&r.map.entities[id]).translation;
                let goal = r
                    .world
                    .actor_footing(raw, PLAYER_CENTER, PLAYER_HALF, 180.)
                    .with_context(|| format!("Unsupported route marker {id}"))?;
                println!("GROUNDS node {id} to {goal:?} sanity {}", r.stats.sanity());
                std::fs::write(
                    "private/grounds2-work/last-checkpoint.json",
                    serde_json::to_vec(&r.checkpoint())?,
                )?;
                r.navigate(goal)?;
                if id == 275 && r.stats.copies(9) > 0 && r.stats.powers.recharge <= 0. {
                    // Equip while still out of reach, before spending the last
                    // Will in the company. Close Blade attacks preserve supplies.
                    r.use_watch()?;
                    r.heavy_weapon = None;
                    r.ice_stream = false;
                    r.conserve_will = true;
                }
                if id == 183 {
                    r.clear(450.)?;
                    r.heavy_weapon = Some(7);
                    r.ice_stream = true;
                    r.conserve_will = false;
                }
                if [183, 204, 225, 351].contains(&id) {
                    // Clear nearby guards and collect their real drops between
                    // replacements, instead of engaging the whole distant company.
                    for _ in 0..4 {
                        r.clear(450.)?;
                        r.wait(3.)?;
                    }
                }
            }
        }
        ensure!(
            r.transition == Some(("facade".into(), Some("facade_start1".into())))
                && r.stats.alive()
                && r.teleports == 0
                && r.audit.lava_ticks == 0
                && r.audit.lost_ticks == 0,
            "Incomplete battlefield route"
        );
        let o = check::owner(r);
        let counts: Vec<_> = o.saved.groups.iter().map(|g| g.count).collect();
        println!("Battle Royale companies: {counts:?}");
        let expected: Vec<_> = TOTAL.into_iter().enumerate()
            .map(|(k, count)| if o.data.easy && k % 2 == 0 { 0 } else { count })
            .collect();
        ensure!(counts == expected, "Not all difficulty-enabled recurring guards were encountered");
        ensure!(
            o.targets().is_empty(),
            "Recurring guards still alive at exit"
        );
        ensure!(
            r.native_cast.as_ref().unwrap().targets().is_empty(),
            "Placed enemies still alive at exit"
        );
        r.assert_clean(0, 0)?;
        println!("Battle Royale: {}", r.metrics());
    Ok(())
}
