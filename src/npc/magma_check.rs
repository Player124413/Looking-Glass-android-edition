//! Source checks and launch-pad interpretation for the six reviewed Magma placements.
use super::*;
pub(super) fn launch(map: &Bsp, spawn: &Spawn) -> Result<Option<Vec3>> {
    let Some(id) = spawn.resident_spawn else {
        return Ok(None);
    };
    if map.entities[id]
        .get("classname")
        .is_none_or(|s| s != "func_spawn")
    {
        return Ok(None);
    }
    for e in &map.entities {
        if e.get("classname").is_none_or(|s| s != "trigger_push")
            || number(e, "spawnflags", 0.) as u32 & 8 != 0
        {
            continue;
        }
        let Some(index) = e
            .get("model")
            .and_then(|s| s.strip_prefix('*'))
            .and_then(|s| s.parse::<usize>().ok())
        else {
            continue;
        };
        let origin = e
            .get("origin")
            .and_then(|s| vector(s))
            .unwrap_or(Vec3::ZERO);
        let volume = crate::collision::Collider::model(map, index, origin, Quat::IDENTITY, false)?;
        if !volume
            .trace(
                spawn.origin,
                spawn.origin,
                vec3(44., 44., 32.) * spawn.scale,
            )
            .start_solid
        {
            continue;
        }
        let target = e
            .get("target")
            .and_then(|name| {
                map.entities
                    .iter()
                    .find(|p| p.get("targetname") == Some(name))
            })
            .and_then(|e| e.get("origin"))
            .and_then(|s| vector(s));
        if let Some(target) = target {
            return Ok(crate::traversal::launch_velocity(spawn.origin, target));
        }
    }
    Ok(None)
}
pub(super) fn check(assets: &mut Assets) -> Result<()> {
    use crate::magma::{CLIPS, MODEL};
    let data = Data::load(assets, MODEL, &[])?;
    let fireball_model = "models/prj_fireball.tik";
    let source = crate::bsp::tokens(&String::from_utf8_lossy(
        &assets.read("models/c_magmamen.tik")?,
    ))?;
    for fact in [
        vec!["health", "200"],
        vec!["visiondistance", "800"],
        vec!["painthreshold", "25"],
        vec!["11", "melee", "15"],
        vec!["13", "radiusattack", "145", "40"],
        vec![
            "10",
            "proj",
            "tag_mouth",
            fireball_model.strip_prefix("models/").unwrap(),
        ],
    ] {
        ensure!(
            source
                .windows(fact.len())
                .any(|w| w.iter().map(String::as_str).eq(fact.iter().copied())),
            "Magma source fact changed: {fact:?}"
        );
    }
    for clip in CLIPS {
        ensure!(data.clips[*clip].duration() > 0., "Missing Magma clip");
    }
    for tag in ["tag_mouth", "tag_smoke01", "tag_smoke02", "tag_smoke03"] {
        ensure!(
            data.skeleton.bones.iter().any(|b| b.name == tag),
            "Magma tag missing"
        );
    }
    let specs = texture::read_materials(assets)?;
    for skin in [
        "skin01",
        "skin01a",
        "skin01b",
        "skin02",
        "magmaskin03",
        "skin03a",
        "skin03b",
    ] {
        let shader = format!("{}/{skin}", data.def.path);
        let spec = specs
            .get(&shader)
            .with_context(|| format!("Missing Magma shader {shader}"))?;
        let path = texture::resolve(assets, &format!("{shader}.tga"), &specs)
            .context("Magma base skin missing")?;
        texture::decode(assets, &path)?;
        for image in spec.stages.iter().flat_map(|s| &s.images) {
            let path = texture::resolve(assets, image, &specs)
                .with_context(|| format!("Magma layer {image}"))?;
            texture::decode(assets, &path)?;
        }
        if matches!(skin, "skin01" | "skin01a" | "magmaskin03" | "skin03a") {
            let layer = spec.stages.last().context("Missing transition layer")?;
            ensure!(
                layer.vertex_alpha
                    && layer.opacity(Vec3::Z, Vec3::Z, 0) == 0.
                    && layer.opacity(Vec3::Z, Vec3::Z, 255) == 1.,
                "Magma cooling layer ignores actor alpha"
            );
        }
    }
    println!("PASS Magma: all three skin forms/layers decode and cooling alpha is respected");
    let mut total = 0;
    for name in ["wforest", "jlair1", "grounds2"] {
        let mut map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let world = World::from_bsp(&map)?;
        for difficulty in crate::powerups::Difficulty::ALL {
            map.difficulty = difficulty;
            let spawns = placements(&map, name, None, false);
            let spawns: Vec<_> = spawns.iter().filter(|s| s.model == MODEL).collect();
            ensure!(spawns.len() == 2, "Magma placement count changed: {name}");
            for s in spawns {
                let p = resident::Resident::new(s, &map, name)?.unwrap();
                ensure!(
                    !p.active && s.resident_spawn.is_some(),
                    "Magma ambush starts active"
                );
                let mut g = match p.body {
                    resident::Body::Magma(g) => g,
                    _ => unreachable!(),
                };
                ensure!(
                    g.launch.is_some() == (name == "grounds2"),
                    "Missing/unexpected Magma launch"
                );
                g.place(&world);
                let b = g.target(0);
                ensure!(
                    !world.sweep(b.center, b.center, b.half).start_solid,
                    "Blocked Magma placement {name} {:?}",
                    g.feet
                );
                let start = g.feet;
                g.notarget = true;
                let mut peak = start.z;
                for _ in 0..120 * 5 {
                    g.step(
                        &world,
                        Vec3::splat(10000.),
                        &data,
                        &mut combat::Feedback::default(),
                    );
                    peak = peak.max(g.feet.z);
                }
                g.validate()?;
                if name == "grounds2" {
                    ensure!(
                        peak - start.z > 350.,
                        "Magma failed to leave launch pit: {start:?} -> peak {peak} end {:?}",
                        g.feet
                    );
                }
                total += 1;
            }
            println!("PASS Magma {name}/{difficulty:?}: two dormant placements, solid clearance, launch/settling");
        }
    }
    println!("PASS Magma source clips, cues, tags and {total} placements");
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct AmbushSave {
    cast: Snapshot,
    expected: Snapshot,
    eye: Vec3,
}
pub(super) async fn native(assets: &mut Assets, mode: &str) -> Result<()> {
    for name in ["wforest", "jlair1", "grounds2"] {
        let scene = crate::render::Scene::load(assets, name)?;
        let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
        interactions.set_entry(assets, &scene.map, name, None)?;
        let mut world = scene.world;
        interactions.sync(&mut world);
        let mut npcs = Npcs::load(assets, &scene.map, name, None, false, false)?;
        npcs.notarget(true);
        npcs.update(0., &world, Vec3::splat(10000.));
        let initial = npcs.snapshot();
        let mut previous = initial.clone();
        previous
            .actors
            .retain(|a| a.spawn.model != crate::magma::MODEL);
        previous.matches_placements(&scene.map, name, None)?;
        npcs.restore(&previous)?;
        for (i, a) in npcs.actors.iter().enumerate() {
            if a.spawn.model == crate::magma::MODEL {
                let p = a.resident.as_ref().unwrap();
                ensure!(
                    !p.active && p.delay.is_none(),
                    "Migration activated Magma ambush"
                );
                p.validate(&a.spawn)?;
                continue;
            }
            ensure!(
                serde_json::to_value(a)? == serde_json::to_value(&initial.actors[i])?,
                "Magma migration changed cast"
            );
        }
        let triggers: Vec<_> = npcs
            .resident_ambushes
            .iter()
            .filter_map(|t| {
                let ids: Vec<_> = t
                    .receivers
                    .iter()
                    .filter(|(id, _)| {
                        let e = &scene.map.entities[*id];
                        e.get("model")
                            .or_else(|| e.get("modelname"))
                            .is_some_and(|m| m == "c_magmamen.tik")
                    })
                    .map(|(id, _)| *id)
                    .collect();
                (!ids.is_empty()).then_some((t.center + Vec3::Z * 20., ids))
            })
            .collect();
        ensure!(
            triggers.len() == if name == "grounds2" { 1 } else { 2 },
            "Missing Magma trigger"
        );
        for (i, (eye, ids)) in triggers.into_iter().enumerate() {
            let path = format!("private/residents/{name}-magma-trigger-{i}.json");
            if mode == "resident-save-read" {
                let saved: AmbushSave = serde_json::from_slice(&std::fs::read(path)?)?;
                saved.cast.matches_placements(&scene.map, name, None)?;
                npcs.restore(&saved.cast)?;
                for _ in 0..240 {
                    npcs.update(1. / 60., &world, saved.eye);
                }
                ensure!(
                    serde_json::to_value(npcs.snapshot())? == serde_json::to_value(saved.expected)?,
                    "Magma activation/flight restart differs"
                );
            } else {
                npcs.restore(&initial)?;
                npcs.update(0., &world, eye);
                ensure!(
                    serde_json::to_value(npcs.snapshot())? == serde_json::to_value(&initial)?,
                    "Paused Magma trigger fired"
                );
                for _ in 0..40 {
                    npcs.update(1. / 60., &world, eye);
                }
                for id in &ids {
                    let a = npcs
                        .actors
                        .iter()
                        .find(|a| a.spawn.resident_spawn == Some(*id))
                        .unwrap();
                    ensure!(
                        a.resident.as_ref().unwrap().active,
                        "Magma touch did not activate receiver {id}"
                    );
                }
                let cast = npcs.snapshot();
                for _ in 0..240 {
                    npcs.update(1. / 60., &world, eye);
                }
                if mode == "resident-save-write" {
                    std::fs::write(
                        path,
                        serde_json::to_vec(&AmbushSave {
                            cast,
                            expected: npcs.snapshot(),
                            eye,
                        })?,
                    )?;
                }
            }
            println!("PASS {name} Magma trigger {i}: pause, activation, migration and fresh-process continuation");
        }
    }
    Ok(())
}
