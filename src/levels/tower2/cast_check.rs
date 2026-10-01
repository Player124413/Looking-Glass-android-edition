//! Windowed native-cast contracts, independent of the traversal driver's aims.
use super::*;
use crate::npc::Npcs;
fn actors(n: &Npcs) -> Result<Vec<serde_json::Value>> {
    Ok(serde_json::to_value(n.snapshot())?["actors"]
        .as_array()
        .context("Missing native cast")?
        .clone())
}
pub(super) fn check(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let map = Bsp::parse(&a.read("maps/tower2.bsp")?)?;
        let mut interactions = Interactions::load(&map)?;
        interactions.set_entry(a, &map, "tower2", None)?;
        owner(&mut interactions)?.upgraded();
        let mut world = World::from_bsp(&map)?;
        interactions.sync(&mut world);
        let fresh =
            |a: &mut Assets| Npcs::load(a, &map, "tower2", Some("tower2_start1"), false, false);
        let n = fresh(a)?;
        let initial = actors(&n)?;
        ensure!(
            initial
                .iter()
                .filter(|a| a["spawn"]["model"] == "c_snark")
                .count()
                == 27,
            "Missing placed/spawned Snarks"
        );
        ensure!(
            initial
                .iter()
                .filter(|a| a["resident"]["active"] == true)
                .count()
                == 7,
            "Inactive wave appeared before its trigger"
        );
        let waves: &[(usize, &[usize])] = &[
            (406, &[407, 408, 409]),
            (14, &[13, 356, 357, 358, 410, 411]),
            (417, &[412, 413, 414, 415, 416]),
            (422, &[418, 419, 420, 421]),
            (423, &[424, 425]),
        ];
        for (trigger, ids) in waves {
            let e = &map.entities[*trigger];
            let m: usize = e["model"].trim_start_matches('*').parse()?;
            let origin = e
                .get("origin")
                .and_then(|s| interaction::vector(s))
                .unwrap_or_default();
            let center = origin + (map.models[m].min + map.models[m].max) * 0.5;
            let eye = center + Vec3::Z * 20.;
            let mut n = fresh(a)?;
            n.update(FIXED_DT, &world, eye);
            let current = actors(&n)?;
            let active: Vec<_> = current
                .iter()
                .filter(|a| a["resident"]["active"] == true)
                .filter_map(|a| a["spawn"]["resident_spawn"].as_u64().map(|n| n as usize))
                .collect();
            ensure!(
                ids.iter().all(|id| active.contains(id)),
                "Wave {trigger} missing receivers: {active:?}"
            );
            ensure!(
                active.len() == ids.len(),
                "Wave {trigger} activated unrelated actors: {active:?}"
            );
            let saved = n.snapshot();
            let mut restored = fresh(a)?;
            restored.restore(&saved)?;
            for tick in 0..120 {
                let f = n.update(FIXED_DT, &world, eye);
                let g = restored.update(FIXED_DT, &world, eye);
                ensure!(
                    serde_json::to_value(n.snapshot())?
                        == serde_json::to_value(restored.snapshot())?
                        && f.damage == g.damage
                        && f.impulse == g.impulse,
                    "Wave {trigger} save differs at tick {tick}"
                );
            }
            let current = actors(&n)?;
            for (id, a) in current.iter().enumerate().filter(|(_, a)| {
                a["spawn"]["resident_spawn"]
                    .as_u64()
                    .is_some_and(|id| ids.contains(&(id as usize)))
            }) {
                ensure!(a["resident"]["active"] == true, "Wave went dormant");
                n.hit(crate::combat::Hit {
                    id,
                    damage: 10000.,
                    kind: crate::combat::DamageKind::Other,
                    knockback: Vec3::ZERO,
                });
            }
            let saved = n.snapshot();
            restored.restore(&saved)?;
            for _ in 0..240 {
                restored.update(FIXED_DT, &world, eye);
            }
            for a in actors(&restored)? {
                if a["spawn"]["resident_spawn"]
                    .as_u64()
                    .is_some_and(|id| ids.contains(&(id as usize)))
                {
                    ensure!(
                        a["resident"]["body"]["Snark"]["health"]
                            .as_f64()
                            .unwrap_or(1.)
                            <= 0.,
                        "Wave {trigger} respawned after death/load"
                    );
                }
            }
            println!(
                "PASS tower2 wave {trigger}: {} distinct Snarks, exact saved future, no respawn",
                ids.len()
            );
        }
        Ok(())
    })
}
