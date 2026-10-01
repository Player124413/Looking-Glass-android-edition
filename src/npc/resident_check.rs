//! Source-data, shared residency and fresh-process continuity checks.
use super::*;
use resident::{Body, Resident};
const CASES: &[(&str, &str)] = &[
    ("c_boojum", "funhouse"),
    ("c_ladybug", "potears2"),
    ("cardguard_diamond", "wforest"),
    ("c_bloodrose", "garden1"),
    ("c_evilmushroom", "garden1"),
    ("cardguard_heart", "hedge3"),
    ("cardguard_spade", "hedge2"),
    ("c_snark", "hedge2"),
    ("c_snark_biteonly", "potears1"),
    ("c_firesnark", "wforest"),
    ("c_antlion", "centipede1"),
    ("c_antlion-underground", "centipede1"),
    ("c_larva", "centipede1"),
    ("c_magmamen", "wforest"),
];
pub fn check(assets: &mut Assets) -> Result<()> {
    card_check::check(assets)?;
    snark_check::check(assets)?;
    burrow_check::check(assets)?;
    magma_check::check(assets)?;
    wildlife_check::check(assets)?;
    for &(model, map_name) in CASES {
        let mut map = Bsp::parse(&assets.read(&format!("maps/{map_name}.bsp"))?)?;
        if model == "c_larva" {
            burrow_check::larva_fixture(&mut map)?;
        }
        let data = Data::load(assets, model, &[])?;
        let mut spawn = placements(&map, map_name, None, false)
            .into_iter()
            .find(|s| {
                s.model == model && (s.resident_spawn.is_none() || model == crate::magma::MODEL)
            })
            .context("Missing resident placement")?;
        spawn.origin = vec3(
            0.,
            0.,
            match model {
                "c_ladybug" => 240.,
                "c_boojum" => 80.,
                "c_snark" | "c_snark_biteonly" | "c_firesnark" => 90.,
                _ => 0.1,
            },
        );
        spawn.resident_spawn = None;
        spawn.yaw = 0.;
        spawn.name = String::new();
        let mut world = World::fixture(&[(vec3(-4000., -4000., -20.), vec3(4000., 4000., 0.))]);
        let eye = if let Some(kind) = crate::snark::Kind::from_model(model) {
            world.set_dynamic_liquids(vec![crate::collision::Liquid {
                contents: if kind == crate::snark::Kind::Fire {
                    8
                } else {
                    32
                },
                volume: crate::collision::Collider::box_bounds(
                    vec3(-1000., -1000., 0.),
                    vec3(1000., 1000., 200.),
                ),
            }]);
            vec3(180., 0., 100.)
        } else {
            vec3(180., 0., 48.)
        };
        let mut results = Vec::new();
        for fps in [30, 60, 144] {
            let mut p = Resident::new(&spawn, &map, "fixture")?.unwrap();
            let paused = serde_json::to_value(&p)?;
            let mut f = combat::Feedback::default();
            p.update(0., &world, eye, &data, &mut f);
            ensure!(
                paused == serde_json::to_value(&p)?,
                "Paused resident changed"
            );
            for _ in 0..fps * 20 {
                p.update(1. / fps as f32, &world, eye, &data, &mut f);
            }
            ensure!(f.damage > 0., "{model} did not attack");
            p.validate(&spawn)?;
            let mut restored: Resident = serde_json::from_value(serde_json::to_value(&p)?)?;
            for _ in 0..30 {
                p.update(1. / 60., &world, eye, &data, &mut f);
                restored.update(
                    1. / 60.,
                    &world,
                    eye,
                    &data,
                    &mut combat::Feedback::default(),
                );
            }
            ensure!(
                serde_json::to_value(&p)? == serde_json::to_value(&restored)?,
                "Resident continuation changed"
            );
            results.push((p.position(), f.damage));
            if let Body::Insect(g) = &mut p.body {
                if g.health > 0. {
                    g.set(crate::burrow::Phase::Idle);
                }
            }
            p.hit(combat::Hit {
                id: 0,
                damage: 1000.,
                kind: combat::DamageKind::Ice,
                knockback: Vec3::ZERO,
            });
            ensure!(p.health() == 0., "Resident did not die");
            for _ in 0..fps * 20 {
                p.update(1. / fps as f32, &world, eye, &data, &mut f);
            }
            let at = p.position();
            for _ in 0..fps * 2 {
                p.update(1. / fps as f32, &world, eye, &data, &mut f);
            }
            ensure!(
                p.position() == at,
                "Retired resident still simulates falling"
            );
        }
        for pair in results.windows(2) {
            ensure!(
                pair[0].0.distance(pair[1].0) < 0.05 && pair[0].1 == pair[1].1,
                "Resident frame-rate drift"
            );
        }
        println!("PASS {model}: source clips, attack, pause, restart, ice and bounded retirement at 30/60/144 Hz");
    }
    for map_name in [
        "potears1",
        "potears2",
        "garden1",
        "garden2",
        "garden3",
        "garden4",
        "centipede1",
        "wforest",
        "funhouse",
        "hatter1",
        "jlair1",
        "hedge1",
        "hedge2",
        "hedge3",
        "tower1",
        "facade",
        "grounds1",
        "grounds2",
        "keep",
        "tower2",
        "utemple",
    ] {
        let mut map = Bsp::parse(&assets.read(&format!("maps/{map_name}.bsp"))?)?;
        for difficulty in crate::powerups::Difficulty::ALL {
            map.difficulty = difficulty;
            let spawns = placements(&map, map_name, None, false);
            let mut counts = BTreeMap::<String, usize>::new();
            for spawn in &spawns {
                if resident::supported(&spawn.model) {
                    let p = Resident::new(spawn, &map, map_name)?.unwrap();
                    p.validate(spawn)?;
                    *counts.entry(spawn.model.clone()).or_default() += 1;
                }
            }
            for a in resident_spawns::ambushes(&map, map_name)? {
                ensure!(
                    a.touches(a.center + Vec3::Z * 20.),
                    "Resident trigger mismatch: {map_name}/{difficulty:?} at {:?}",
                    a.center
                );
                for (id, _) in a.receivers {
                    ensure!(
                        spawns
                            .iter()
                            .filter(|s| s.resident_spawn == Some(id))
                            .count()
                            == 1,
                        "Missing resident receiver"
                    );
                }
            }
            ensure!(spawns.len() < 256, "NPC cast capacity reached");
            println!("PASS resident cast {map_name}/{difficulty:?}: {counts:?}");
        }
    }
    Ok(())
}
#[derive(serde::Serialize, serde::Deserialize)]
struct Saved {
    cast: Snapshot,
    expected: Snapshot,
    eye: Vec3,
    damage: f32,
    loot: crate::loot::Loot,
    awards: usize,
}
fn step(npcs: &mut Npcs, world: &World, eye: Vec3, frames: usize) -> f32 {
    (0..frames)
        .map(|_| npcs.update(1. / 60., world, eye).damage)
        .sum()
}
pub async fn native(assets: &mut Assets, mode: &str) -> Result<()> {
    let magma_only = mode.starts_with("magma-");
    let insects_only = mode.starts_with("burrow-");
    let snarks_only = mode.starts_with("snark-");
    let mapped = mode
        .replacen("snark-", "resident-", 1)
        .replacen("burrow-", "resident-", 1)
        .replacen("magma-", "resident-", 1);
    let mode = mapped.as_str();
    std::fs::create_dir_all("private/residents")?;
    if magma_only {
        magma_check::native(assets, mode).await?;
    }
    if insects_only {
        burrow_check::ambush_native(assets, mode).await?;
    }
    for &(family, map_name) in CASES {
        if magma_only && family != crate::magma::MODEL {
            continue;
        }
        if insects_only && crate::burrow::Kind::from_model(family).is_none() {
            continue;
        }
        if snarks_only && crate::snark::Kind::from_model(family).is_none() {
            continue;
        }
        let mut scene = crate::render::Scene::load(assets, map_name)?;
        if family == "c_larva" {
            burrow_check::larva_fixture(&mut scene.map)?;
        }
        let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
        interactions.set_entry(assets, &scene.map, map_name, None)?;
        interactions.sync(&mut scene.world);
        let mut npcs = Npcs::load(assets, &scene.map, map_name, None, false, false)?;
        npcs.update(0., &scene.world, Vec3::splat(10000.));
        for a in &mut npcs.actors {
            if let Some(resident::Resident {
                body: Body::Snark(p),
                ..
            }) = &mut a.resident
            {
                p.notarget = true;
                for _ in 0..360 {
                    p.step(
                        &scene.world,
                        Vec3::splat(10000.),
                        &npcs.models[a.model].data,
                        &mut combat::Feedback::default(),
                    );
                }
                p.notarget = false;
                // Frame Fire Snarks at the surface of their actual pool. Their
                // authored resting origin can be hidden below opaque lava.
                if p.kind == crate::snark::Kind::Fire {
                    for _ in 0..128 {
                        let next = p.feet + Vec3::Z * 2.;
                        let body = p.target(0);
                        let trace =
                            scene
                                .world
                                .sweep(body.center, body.center + Vec3::Z * 2., body.half);
                        if !p.kind.wet(&scene.world, next)
                            || trace.start_solid
                            || trace.fraction < 1.
                        {
                            break;
                        }
                        p.feet = next;
                    }
                }
            }
        }
        let initial = npcs.snapshot();
        let id = npcs
            .actors
            .iter()
            .position(|a| {
                a.resident.as_ref().is_some_and(|p| p.model() == family)
                    && (a.spawn.resident_spawn.is_none() || family == crate::magma::MODEL)
            })
            .context("Missing active resident")?;
        let mut framed = npcs.actors[id].resident.clone().unwrap();
        framed.active = true;
        if let Body::Insect(p) = &mut framed.body {
            p.set(crate::burrow::Phase::Idle);
        }
        if let Body::Plant(p) = &mut framed.body {
            p.grow_for_check();
        }
        let bounds = framed.target(id);
        let target = bounds.center;
        let radius =
            (bounds.half.length() * 2.8).max(if family == "c_larva" { 120. } else { 240. });
        let eye = (0..32)
            .map(|i| {
                let a = i as f32 * std::f32::consts::TAU / 32.;
                target + vec3(a.cos() * radius, a.sin() * radius, 24.)
            })
            .find(|p| {
                let t = scene.world.sweep(target, *p, Vec3::splat(2.));
                !t.start_solid && t.fraction >= 1.
            })
            .context("No clear resident camera")?;
        let mut legacy = initial.clone();
        legacy.actors.retain(|a| a.spawn.resident_spawn.is_none());
        for a in &mut legacy.actors {
            a.resident = None;
        }
        legacy.matches_placements(&scene.map, map_name, None)?;
        npcs.restore(&legacy)?;
        ensure!(
            npcs.actors[id].resident.is_some(),
            "Legacy resident did not migrate"
        );
        // Upgrade the immediately preceding resident release without disturbing live neighbors.
        let mut previous = initial.clone();
        previous.actors.retain(|a| {
            a.spawn.resident_spawn.is_none()
                || crate::burrow::Kind::from_model(&a.spawn.model).is_none()
        });
        for a in &mut previous.actors {
            if crate::burrow::Kind::from_model(&a.spawn.model).is_some() {
                a.resident = None;
            }
        }
        previous.matches_placements(&scene.map, map_name, None)?;
        npcs.restore(&previous)?;
        for (index, a) in npcs.actors.iter().enumerate() {
            if crate::burrow::Kind::from_model(&a.spawn.model).is_none() {
                ensure!(
                    serde_json::to_value(&a.resident)?
                        == serde_json::to_value(&initial.actors[index].resident)?,
                    "Existing resident changed on insect upgrade"
                );
            }
        }
        let pending = npcs.actors.iter().position(|a| {
            a.spawn.resident_spawn.is_some()
                && a.resident.as_ref().is_some_and(|p| p.model() == family)
        });
        if let Some(pending) = pending {
            let entity = npcs.actors[pending].spawn.resident_spawn.unwrap();
            let trigger = npcs
                .resident_ambushes
                .iter()
                .find(|a| a.receivers.iter().any(|(i, _)| *i == entity))
                .unwrap();
            let trigger_eye = trigger.center + Vec3::Z * 20.;
            npcs.restore(&initial)?;
            ensure!(
                !npcs.targets().iter().any(|t| t.id == pending),
                "Dormant resident targetable"
            );
            npcs.update(0., &scene.world, trigger_eye);
            ensure!(
                serde_json::to_value(npcs.snapshot())? == serde_json::to_value(&initial)?,
                "Paused trigger changed state"
            );
            step(&mut npcs, &scene.world, trigger_eye, 1);
            let p = npcs.actors[pending].resident.as_ref().unwrap();
            ensure!(
                p.active || p.delay.is_some(),
                "Touch did not activate resident"
            );
            step(&mut npcs, &scene.world, Vec3::splat(10000.), 1900);
            ensure!(
                npcs.actors[pending].resident.as_ref().unwrap().active,
                "Pending resident did not activate"
            );
            npcs.restore(&legacy)?;
            ensure!(
                !npcs.actors[pending].resident.as_ref().unwrap().active,
                "Legacy save retained spawned resident"
            );
        }
        for case in [
            "attack",
            "pain",
            "death",
            "ice",
            "dormant",
            "delay",
            "projectile",
            "tongue",
            "burrow-up",
            "burrow-down",
            "tunnel",
            "attached",
            "draining",
        ] {
            if matches!(case, "burrow-up" | "burrow-down" | "tunnel")
                && !matches!(
                    crate::burrow::Kind::from_model(family),
                    Some(crate::burrow::Kind::Antlion | crate::burrow::Kind::Underground)
                )
            {
                continue;
            }
            if matches!(case, "attached" | "draining") && family != "c_larva" {
                continue;
            }
            if ((case == "tongue") || (case == "projectile" && family != crate::magma::MODEL))
                && !matches!(
                    crate::snark::Kind::from_model(family),
                    Some(crate::snark::Kind::Water | crate::snark::Kind::Fire)
                )
            {
                continue;
            }
            let delayed = matches!(case, "dormant" | "delay");
            let id = if delayed {
                let Some(pending) = pending else { continue };
                pending
            } else {
                id
            };
            let eye = if delayed {
                Vec3::splat(10000.)
            } else if matches!(case, "attached" | "draining") {
                target + (eye - target).with_z(0.).normalize() * 75. + Vec3::Z * 36.
            } else {
                eye
            };
            let path = format!("private/residents/{family}-{case}-save.json");
            if mode == "resident-save-read" {
                let saved: Saved = serde_json::from_slice(&std::fs::read(path)?)?;
                saved.cast.matches_placements(&scene.map, map_name, None)?;
                npcs.restore(&saved.cast)?;
                let mut loot = saved.loot;
                loot.validate()?;
                let before = npcs.loot_sources();
                let damage = step(&mut npcs, &scene.world, saved.eye, 120);
                loot.defeated(&before, &npcs.loot_sources(), &scene.world);
                ensure!(
                    serde_json::to_value(npcs.snapshot())? == serde_json::to_value(saved.expected)?
                        && damage == saved.damage,
                    "Resident restart differs"
                );
                ensure!(
                    loot.awarded.len() == saved.awards,
                    "Resident reward duplicated"
                );
                println!("PASS {family}/{case}: exact fresh-process continuation and reward");
                continue;
            }
            npcs.restore(&initial)?;
            if !delayed {
                npcs.actors[id].resident.as_mut().unwrap().active = true;
            }
            let mut loot = crate::loot::Loot::default();
            let before = npcs.loot_sources();
            if let Some(resident::Resident {
                body: Body::Plant(p),
                ..
            }) = npcs.actors[id].resident.as_mut()
            {
                p.set(crate::plants::Phase::Ready);
            }
            if matches!(
                case,
                "burrow-up" | "burrow-down" | "tunnel" | "attached" | "draining"
            ) {
                let actor = &mut npcs.actors[id];
                let Some(Resident {
                    body: Body::Insect(g),
                    ..
                }) = &mut actor.resident
                else {
                    unreachable!();
                };
                use crate::burrow::Phase;
                g.set(match case {
                    "burrow-up" => Phase::Rise,
                    "burrow-down" => Phase::Dive,
                    "tunnel" => Phase::Tunnel,
                    _ => Phase::Run,
                });
                if matches!(case, "attached" | "draining") {
                    let desired = if case == "attached" {
                        Phase::Attach
                    } else {
                        Phase::Suck
                    };
                    for _ in 0..1800 {
                        g.step(
                            &scene.world,
                            eye,
                            &npcs.models[actor.model].data,
                            &mut combat::Feedback::default(),
                        );
                        if g.phase == desired {
                            break;
                        }
                    }
                    ensure!(
                        g.phase == desired,
                        "Larva did not reach {desired:?} in original scene: {:?} {:?}",
                        g.phase,
                        g.feet
                    );
                }
            } else if delayed {
                if case == "delay" {
                    npcs.actors[id].resident.as_mut().unwrap().delay = Some(1.35);
                }
            } else if matches!(case, "attack" | "projectile" | "tongue") {
                let p = npcs.actors[id].resident.as_mut().unwrap();
                match &mut p.body {
                    Body::Wildlife(_) => unreachable!(),
                    Body::Magma(g) => {
                        g.set(if case == "projectile" {
                            crate::magma::Phase::Spit
                        } else {
                            crate::magma::Phase::Punch
                        });
                        if case == "projectile" {
                            g.shots.push(crate::snark::Shot {
                                serial: 99,
                                position: target + (eye - target) * 0.2,
                                direction: (eye - target).normalize(),
                                age: 0.,
                            });
                        }
                    }
                    Body::Insect(g) => {
                        g.set(if g.kind == crate::burrow::Kind::Larva {
                            crate::burrow::Phase::Run
                        } else {
                            crate::burrow::Phase::Melee
                        });
                    }
                    Body::Snark(g) => {
                        g.set(if case == "tongue" {
                            crate::snark::Phase::Tongue
                        } else if g.kind == crate::snark::Kind::BiteOnly {
                            crate::snark::Phase::Bite
                        } else {
                            crate::snark::Phase::Spit
                        });
                        if case == "projectile" {
                            g.shots.push(crate::snark::Shot {
                                serial: 99,
                                position: target + (eye - target) * 0.2,
                                direction: (eye - target).normalize(),
                                age: 0.,
                            });
                        }
                    }
                    Body::Card(g) => {
                        g.set(crate::cards::Phase::Fire);
                    }
                    Body::Plant(g) => {
                        g.set(crate::plants::Phase::Spit);
                    }
                    Body::Diamond(g) => {
                        g.state = combat::State::Attack;
                        g.time = 0.;
                    }
                    Body::Boojum(b) => {
                        b.state = crate::boojum::State::Scream;
                        b.time = 0.;
                    }
                    Body::Ladybug(b) => {
                        b.state = crate::ladybug::State::Attack;
                        b.time = 0.;
                    }
                }
            } else {
                if let Some(Resident {
                    body: Body::Insect(g),
                    ..
                }) = npcs.actors[id].resident.as_mut()
                {
                    g.set(crate::burrow::Phase::Idle);
                }
                npcs.hit(combat::Hit {
                    id,
                    damage: if case == "pain" {
                        if crate::snark::Kind::from_model(family).is_some()
                            || crate::burrow::Kind::from_model(family).is_some()
                        {
                            7.
                        } else if family == "cardguard_heart" {
                            40.
                        } else {
                            30.
                        }
                    } else {
                        1000.
                    },
                    kind: if case == "ice" {
                        combat::DamageKind::Ice
                    } else {
                        combat::DamageKind::Knife
                    },
                    knockback: Vec3::ZERO,
                });
            }
            loot.defeated(&before, &npcs.loot_sources(), &scene.world);
            step(&mut npcs, &scene.world, eye, 8);
            let cast = npcs.snapshot();
            let future_before = npcs.loot_sources();
            let damage = step(&mut npcs, &scene.world, eye, 120);
            let expected = npcs.snapshot();
            let mut future_loot = loot.clone();
            future_loot.defeated(&future_before, &npcs.loot_sources(), &scene.world);
            if mode == "resident-save-write" {
                std::fs::write(
                    path,
                    serde_json::to_vec(&Saved {
                        cast,
                        expected,
                        eye,
                        damage,
                        loot,
                        awards: future_loot.awarded.len(),
                    })?,
                )?;
            }
        }
        if mode != "resident-render" {
            continue;
        }
        npcs.restore(&initial)?;
        npcs.actors = vec![npcs.actors[id].clone()];
        let mut base = npcs.actors[0].resident.clone().unwrap();
        base.active = true;
        let mut idle_pixels: Option<Vec<u8>> = None;
        for pose in [
            "idle",
            "attack",
            "pain",
            "death",
            "ice",
            "retired",
            "dormant",
            "fan",
            "suction",
            "digest",
            "projectile",
            "charge",
            "slam",
            "spin",
            "combo",
            "cut",
            "swim",
            "rise",
            "dive",
            "tongue",
            "burrow-up",
            "burrow-down",
            "hidden",
            "pincers",
            "sting",
            "leap",
            "attach",
            "drain",
            "detach",
            "cooling",
            "rock",
            "rock-attack",
            "rock-death",
            "bash",
        ] {
            let mut p = base.clone();
            if matches!(
                pose,
                "cooling" | "rock" | "rock-attack" | "rock-death" | "bash"
            ) && family != crate::magma::MODEL
            {
                continue;
            }
            if matches!(
                pose,
                "burrow-up" | "burrow-down" | "hidden" | "pincers" | "sting"
            ) && !matches!(&p.body,Body::Insect(g) if g.kind!=crate::burrow::Kind::Larva)
            {
                continue;
            }
            if matches!(pose, "leap" | "attach" | "drain" | "detach")
                && !matches!(&p.body,Body::Insect(g) if g.kind==crate::burrow::Kind::Larva)
            {
                continue;
            }
            if matches!(pose, "swim" | "rise" | "dive" | "tongue") {
                let Body::Snark(g) = &p.body else {
                    continue;
                };
                if pose != "swim" && g.kind == crate::snark::Kind::BiteOnly {
                    continue;
                }
            }
            if pose == "projectile"
                && matches!(&p.body,Body::Snark(g) if g.kind==crate::snark::Kind::BiteOnly)
            {
                continue;
            }
            if matches!(pose, "charge" | "slam" | "spin" | "combo" | "cut") {
                let Body::Card(g) = &p.body else {
                    continue;
                };
                if (matches!(pose, "charge" | "slam" | "combo")
                    && g.kind != crate::cards::Kind::Heart)
                    || (matches!(pose, "spin" | "cut") && g.kind != crate::cards::Kind::Spade)
                {
                    continue;
                }
            }
            if matches!(
                pose,
                "dormant" | "fan" | "suction" | "digest" | "projectile"
            ) {
                if pose == "projectile"
                    && matches!(&p.body, Body::Card(_) | Body::Snark(_) | Body::Magma(_))
                {
                } else {
                    let Body::Plant(plant) = &p.body else {
                        continue;
                    };
                    if (pose == "fan" && plant.kind != crate::plants::Kind::Rose)
                        || (matches!(pose, "suction" | "digest")
                            && plant.kind != crate::plants::Kind::Mushroom)
                    {
                        continue;
                    }
                }
            }
            let yaw = (eye.y - target.y).atan2(eye.x - target.x);
            match &mut p.body {
                Body::Wildlife(_) => unreachable!(),
                Body::Magma(g) => {
                    use crate::magma::Phase;
                    g.yaw = yaw;
                    g.form = match pose {
                        "rock" | "rock-attack" | "rock-death" | "bash" => 2,
                        "cooling" => 1,
                        _ => 0,
                    };
                    g.cooling = g.form as f32 * 6.;
                    g.set(match pose {
                        "attack" => Phase::Punch,
                        "rock-attack" => Phase::Rock,
                        "bash" => Phase::Bash,
                        "projectile" => Phase::Spit,
                        "pain" => Phase::Pain,
                        "death" | "ice" | "retired" | "rock-death" => Phase::Dead,
                        _ => Phase::Idle,
                    });
                    g.health = if g.phase == Phase::Dead { 0. } else { 200. };
                    g.frozen = pose == "ice";
                    g.time = if pose == "retired" { 20. } else { 0.4 };
                    if pose == "projectile" {
                        g.shots.push(crate::snark::Shot {
                            serial: 1,
                            position: target + (eye - target) * 0.3,
                            direction: (eye - target).normalize(),
                            age: 0.2,
                        });
                    }
                }
                Body::Insect(g) => {
                    use crate::burrow::{Kind, Phase};
                    g.yaw = yaw;
                    g.set(match pose {
                        "attack" if g.kind == Kind::Larva => Phase::Leap,
                        "attack" | "pincers" | "sting" => Phase::Melee,
                        "pain" if g.kind == Kind::Larva => Phase::Dead,
                        "pain" => Phase::Pain,
                        "death" | "ice" | "retired" => Phase::Dead,
                        "burrow-up" => Phase::Rise,
                        "burrow-down" => Phase::Dive,
                        "hidden" => Phase::Tunnel,
                        "leap" => Phase::Leap,
                        "attach" => Phase::Attach,
                        "drain" => Phase::Suck,
                        "detach" => Phase::Detach,
                        _ => Phase::Idle,
                    });
                    g.variant = match pose {
                        "pincers" => 1,
                        "sting" => 2,
                        _ => 0,
                    };
                    g.time = if pose == "retired" { 20. } else { 0.4 };
                    g.health = if g.phase == Phase::Dead {
                        0.
                    } else {
                        g.kind.health()
                    };
                    g.frozen = pose == "ice";
                }
                Body::Snark(g) => {
                    use crate::snark::{Phase, Shot};
                    g.yaw = yaw;
                    g.set(match pose {
                        "attack" => Phase::Bite,
                        "projectile" => Phase::Spit,
                        "swim" => Phase::Swim,
                        "rise" => Phase::Rise,
                        "dive" => Phase::Dive,
                        "tongue" => Phase::Tongue,
                        "pain" => Phase::Pain,
                        "death" | "ice" | "retired" => Phase::Dead,
                        _ => Phase::Idle,
                    });
                    g.time = if pose == "retired" { 20. } else { 0.4 };
                    g.health = if matches!(pose, "death" | "ice" | "retired") {
                        0.
                    } else {
                        g.kind.health()
                    };
                    g.frozen = pose == "ice";
                    if pose == "projectile" {
                        g.shots.push(Shot {
                            serial: 1,
                            position: target + (eye - target) * 0.3,
                            direction: (eye - target).normalize(),
                            age: 0.2,
                        });
                    }
                    if pose == "tongue" {
                        g.tongue = Some(target + (eye - target) * 0.5);
                    }
                }
                Body::Card(g) => {
                    use crate::cards::{Kind, Phase, Shot};
                    g.yaw = yaw;
                    g.set(match pose {
                        "attack" | "projectile" => Phase::Fire,
                        "charge" => Phase::Charge,
                        "slam" => Phase::Slam,
                        "spin" => Phase::Melee,
                        "combo" => Phase::Combo,
                        "pain" => Phase::Pain,
                        "death" | "ice" | "retired" | "cut" => Phase::Dead,
                        _ => Phase::Idle,
                    });
                    g.variant = 0;
                    g.time = match pose {
                        "retired" => 20.,
                        "attack" => 0.8,
                        "slam" => 1.5,
                        _ => 0.4,
                    };
                    g.health = if matches!(pose, "death" | "ice" | "retired" | "cut") {
                        0.
                    } else {
                        g.kind.health()
                    };
                    g.frozen = pose == "ice";
                    if pose == "cut" {
                        g.cut = true;
                        g.time = 0.;
                        let data = &npcs.models[npcs.actors[0].model].data;
                        for _ in 0..48 {
                            g.step(&scene.world, eye, data, &mut combat::Feedback::default());
                        }
                    }
                    if pose == "projectile" {
                        g.shots.push(Shot {
                            serial: 1,
                            position: target + (eye - target) * 0.3,
                            direction: (eye - target).normalize(),
                            age: 0.2,
                            victim: crate::dice::ALICE,
                        });
                    }
                    ensure!(
                        g.kind == Kind::Heart
                            || !matches!(g.phase, Phase::Charge | Phase::Slam | Phase::Combo),
                        "Invalid staged guard"
                    );
                }
                Body::Plant(g) => {
                    g.yaw = yaw;
                    g.set(match pose {
                        "attack" | "projectile" => crate::plants::Phase::Spit,
                        "dormant" => crate::plants::Phase::Dormant,
                        "fan" => crate::plants::Phase::Fan,
                        "suction" => crate::plants::Phase::Suck,
                        "digest" => crate::plants::Phase::Digest,
                        "pain" => crate::plants::Phase::Pain,
                        "death" | "ice" | "retired" => crate::plants::Phase::Dead,
                        _ => crate::plants::Phase::Ready,
                    });
                    g.time = if pose == "retired" { 20. } else { 0.4 };
                    g.health = if matches!(pose, "death" | "ice" | "retired") {
                        0.
                    } else {
                        g.kind.health()
                    };
                    g.frozen = pose == "ice";
                    if pose != "dormant" {
                        g.grow_for_check();
                    }
                    if pose == "projectile" {
                        g.shots.push(crate::plants::Shot {
                            serial: 1,
                            position: target + (eye - target) * 0.3,
                            direction: (eye - target).normalize(),
                            age: 0.2,
                        });
                    }
                }
                Body::Diamond(g) => {
                    g.yaw = yaw;
                    g.state = match pose {
                        "attack" => combat::State::Attack,
                        "pain" => combat::State::Pain,
                        "death" | "ice" | "retired" => combat::State::Dead,
                        _ => combat::State::Idle,
                    };
                    g.time = if pose == "retired" { 20. } else { 0.4 };
                    g.health = if matches!(pose, "death" | "ice" | "retired") {
                        0.
                    } else {
                        55.
                    };
                    g.frozen = pose == "ice";
                }

                Body::Boojum(b) => {
                    b.yaw = yaw;
                    b.state = match pose {
                        "attack" => crate::boojum::State::Scream,
                        "pain" => crate::boojum::State::Pain,
                        "death" | "ice" | "retired" => crate::boojum::State::Dead,
                        _ => crate::boojum::State::Fly,
                    };
                    b.time = if pose == "retired" { 20. } else { 0.8 };
                    b.health = if matches!(pose, "death" | "ice" | "retired") {
                        0.
                    } else {
                        70.
                    };
                    b.frozen = pose == "ice";
                }
                Body::Ladybug(b) => {
                    b.yaw = yaw;
                    b.state = match pose {
                        "attack" => crate::ladybug::State::Attack,
                        "pain" => crate::ladybug::State::Pain,
                        "death" | "ice" | "retired" => crate::ladybug::State::Dead,
                        _ => crate::ladybug::State::Patrol,
                    };
                    b.time = if pose == "retired" { 20. } else { 0.4 };
                    b.health = if matches!(pose, "death" | "ice" | "retired") {
                        0.
                    } else {
                        crate::ladybug::HEALTH
                    };
                    b.frozen = pose == "ice";
                }
            }
            npcs.actors[0].resident = Some(p);
            let pixels = capture(
                &mut npcs,
                &mut scene,
                &interactions,
                eye,
                target,
                &format!("{family}-{pose}"),
            )
            .await?;
            if crate::snark::Kind::from_model(family).is_some()
                || crate::burrow::Kind::from_model(family).is_some()
                || family == crate::magma::MODEL
            {
                if pose == "ice" {
                    ensure!(
                        npcs.models[npcs.actors[0].model].frozen.is_some(),
                        "Snark frozen material missing"
                    );
                    if family == "c_firesnark" {
                        let ice = pixels
                            .chunks_exact(4)
                            .filter(|p| p[2] > 180 && p[1] > 120 && p[2].saturating_sub(p[0]) > 20)
                            .count();
                        ensure!(
                            ice > 20,
                            "Fire Snark ice coating absent from capture ({ice} pixels)"
                        );
                    }
                }
                if pose == "idle" {
                    idle_pixels = Some(pixels);
                } else if pose == "retired" {
                    let idle = idle_pixels
                        .as_ref()
                        .context("Missing visible Snark frame")?;
                    let changed = idle
                        .chunks_exact(4)
                        .zip(pixels.chunks_exact(4))
                        .filter(|(a, b)| (0..3).any(|i| a[i].abs_diff(b[i]) > 8))
                        .count();
                    ensure!(changed>100, "Snark body is not visible in its native capture: {family} ({changed} pixels)");
                    println!("PASS visible {family}: {changed} body/effect pixels retire");
                }
            }
        }
    }
    Ok(())
}
async fn capture(
    npcs: &mut Npcs,
    scene: &mut crate::render::Scene,
    interactions: &crate::interaction::Interactions,
    eye: Vec3,
    target: Vec3,
    label: &str,
) -> Result<Vec<u8>> {
    let mut captured = Vec::new();
    for frame in 0..3 {
        clear_background(BLACK);
        let camera = Camera3D {
            position: eye,
            target,
            up: Vec3::Z,
            fovy: 65_f32.to_radians(),
            z_near: 2.,
            z_far: 30000.,
            ..Default::default()
        };
        set_camera(&camera);
        crate::render_fx::begin_view(&camera, 0., &scene.atmosphere, false);
        let transforms = interactions.transforms();
        scene.draw(eye, 0., false, false, &transforms);
        npcs.draw(eye, (target - eye).normalize(), &scene.atmosphere, false);
        crate::render::depth_read_only(|| scene.draw(eye, 0., false, true, &transforms));
        let (_, dropped) = crate::render_fx::finish();
        ensure!(dropped == 0, "Resident geometry dropped");
        set_default_camera();
        if frame == 2 {
            captured = get_screen_data().bytes;
            if label.starts_with("cardguard_") && label.ends_with("retired") {
                let pixels = get_screen_data();
                let lit = pixels
                    .bytes
                    .chunks_exact(4)
                    .filter(|p| p[0].max(p[1]).max(p[2]) > 8)
                    .count();
                ensure!(
                    lit > pixels.bytes.len() / 40,
                    "Card capture does not show representative map scenery"
                );
            }
            crate::viewer::save_capture(std::path::Path::new(&format!(
                "private/residents/{label}.png"
            )))?;
        }
        next_frame().await;
    }
    Ok(captured)
}
