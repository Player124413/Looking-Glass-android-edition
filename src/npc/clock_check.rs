//! Source data, bounded ambushes, production rendering and restart continuity.
use super::*;
use crate::{
    clockwork::{Automaton, Phase, MODEL},
    combat::{DamageKind, Feedback, Hit},
};

pub fn check(assets: &mut Assets) -> Result<()> {
    let data = Data::load(assets, MODEL, &[])?;
    for clip in crate::clockwork::CLIPS {
        ensure!(
            data.clips.contains_key(*clip),
            "Missing Automaton clip {clip}"
        );
    }
    crate::weapons::read_model(assets, "prj_hand")?;
    let clip = &data.clips["range_steam_fire"];
    let steam = data.events.visual(
        "range_steam_fire",
        0.2,
        clip.duration(),
        clip.frame_time,
        false,
    );
    for jet in ["steamblast1", "steamblast2"] {
        ensure!(
            steam.emitters.get(jet) == Some(&true),
            "Original steam event ignored: {jet}"
        );
    }
    let floor = World::fixture(&[(vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.))]);
    let mut results = Vec::new();
    for fps in [30, 60, 144] {
        let mut p = Automaton::new(Vec3::Z * 0.1, 0., 1., 11, true);
        let mut out = Feedback::default();
        for _ in 0..20 * fps {
            p.update(
                1. / fps as f32,
                &floor,
                vec3(450., 0., 48.),
                &data,
                &mut out,
            );
        }
        ensure!(
            out.damage > 0. && p.feet.x > 100.,
            "Automaton does not pursue/attack with supplied timing"
        );
        p.validate()?;
        results.push((p.feet, out.damage));
    }
    for pair in results.windows(2) {
        ensure!(
            pair[0].0.distance(pair[1].0) < 0.1 && pair[0].1 == pair[1].1,
            "Automaton frame rate drift"
        );
    }
    println!("PASS Clockwork source clips, hand projectile, pursuit and contact at 30/60/144 Hz");
    for name in ["funhouse", "hatter1", "hedge2", "hedge3"] {
        let mut map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        for difficulty in crate::powerups::Difficulty::ALL {
            map.difficulty = difficulty;
            let spawns = placements(&map, name, None, false);
            let count = spawns.iter().filter(|s| s.model == MODEL).count();
            let dormant = spawns.iter().filter(|s| s.clock_spawn.is_some()).count();
            ensure!(spawns.len() < 256, "Automaton cast reaches capacity");
            let ambushes = clock_spawns::ambushes(&map, name)?;
            for a in &ambushes {
                ensure!(
                    a.touches(a.center + Vec3::Z * 20.),
                    "Automaton trigger volume mismatch"
                );
                for (id, delay) in &a.receivers {
                    ensure!(
                        spawns.iter().filter(|s| s.clock_spawn == Some(*id)).count() == 1
                            && (0. ..=30.).contains(delay),
                        "Missing or duplicate Automaton receiver"
                    );
                }
            }
            println!(
            "PASS {name}/{difficulty:?}: {count} Clockworks, {dormant} dormant, {} ambush volumes",
            ambushes.len()
        );
        }
    }
    let hatter = Bsp::parse(&assets.read("maps/hatter2.bsp")?)?;
    ensure!(
        clock_spawns::placements(&hatter, "hatter2").is_empty(),
        "Boss adds spawned outside their scene"
    );
    Ok(())
}
#[derive(serde::Serialize, serde::Deserialize)]
struct Saved {
    cast: Snapshot,
    expected: Snapshot,
    loot: crate::loot::Loot,
    eye: Vec3,
    id: usize,
    dead: bool,
    damage: f32,
}
fn step(npcs: &mut Npcs, world: &World, eye: Vec3, frames: usize) -> f32 {
    (0..frames)
        .map(|_| npcs.update(1. / 60., world, eye).damage)
        .sum()
}
pub async fn native(assets: &mut Assets, mode: &str) -> Result<()> {
    std::fs::create_dir_all("private/clock")?;
    let mut scene = crate::render::Scene::load(assets, "funhouse")?;
    let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
    interactions.set_entry(assets, &scene.map, "funhouse", None)?;
    interactions.sync(&mut scene.world);
    let mut npcs = Npcs::load(assets, &scene.map, "funhouse", None, false, false)?;
    npcs.update(0., &scene.world, Vec3::splat(10000.));
    let initial = npcs.snapshot();
    let id = npcs
        .actors
        .iter()
        .position(|a| a.clock.is_some() && a.spawn.clock_spawn.is_none())
        .context("Missing placed Automaton")?;
    let (pending_id, entity) = npcs
        .actors
        .iter()
        .enumerate()
        .find_map(|(id, a)| a.spawn.clock_spawn.map(|e| (id, e)))
        .context("Missing dormant Automaton")?;
    let ambush = npcs
        .clock_ambushes
        .iter()
        .find(|a| a.receivers.iter().any(|(i, _)| *i == entity))
        .unwrap();
    let trigger_eye = ambush.center + Vec3::Z * 20.;
    ensure!(
        !npcs.targets().iter().any(|t| t.id == pending_id),
        "Dormant Automaton is targetable"
    );
    npcs.update(0., &scene.world, trigger_eye);
    ensure!(
        serde_json::to_value(&initial)? == serde_json::to_value(npcs.snapshot())?,
        "Paused Automaton trigger changed state"
    );
    step(&mut npcs, &scene.world, trigger_eye, 1);
    ensure!(
        npcs.actors[pending_id].clock.as_ref().unwrap().active,
        "Touch did not activate Automaton"
    );
    // A pending timer is saved even when this map's reviewed trigger has no delay.
    npcs.restore(&initial)?;
    npcs.actors[pending_id].clock.as_mut().unwrap().spawn_delay = Some(2.);
    let delayed = npcs.snapshot();
    step(&mut npcs, &scene.world, Vec3::splat(10000.), 180);
    ensure!(
        npcs.targets().iter().any(|t| t.id == pending_id),
        "Delayed Automaton failed to spawn away from trigger"
    );
    // Old saves omit both the optional combat field and the appended ambush actors.
    let mut legacy = initial.clone();
    legacy.actors.retain(|a| a.spawn.clock_spawn.is_none());
    for a in &mut legacy.actors {
        a.clock = None;
    }
    legacy.matches_placements(&scene.map, "funhouse", None)?;
    npcs.restore(&legacy)?;
    ensure!(
        !npcs.actors[pending_id].clock.as_ref().unwrap().active,
        "Old load retained a live new ambush"
    );
    let feet = npcs.actors[id].position();
    let target = feet + Vec3::Z * 48.;
    let eye = (0..32)
        .map(|i| {
            let angle = i as f32 * std::f32::consts::TAU / 32.;
            target + vec3(angle.cos() * 250., angle.sin() * 250., 20.)
        })
        .find(|p| {
            let t = scene.world.sweep(target, *p, Vec3::splat(2.));
            !t.start_solid && t.fraction >= 1.
        })
        .context("No clear Automaton camera")?;
    for case in [
        "punch", "contact", "fists", "steam", "pain", "death", "ice", "dormant", "delay",
    ] {
        let path = format!("private/clock/{case}-save.json");
        if mode == "clock-save-read" {
            let saved: Saved = serde_json::from_slice(&std::fs::read(path)?)?;
            saved
                .cast
                .matches_placements(&scene.map, "funhouse", None)?;
            npcs.restore(&saved.cast)?;
            let mut loot = saved.loot;
            loot.validate()?;
            let before = npcs.loot_sources();
            let damage = step(&mut npcs, &scene.world, saved.eye, 180);
            loot.defeated(&before, &npcs.loot_sources(), &scene.world);
            ensure!(
                serde_json::to_value(npcs.snapshot())? == serde_json::to_value(saved.expected)?
                    && damage == saved.damage,
                "Automaton {case} continuation differs after restart"
            );
            ensure!(
                loot.awarded.len() == usize::from(saved.dead)
                    && loot.drops.len() == usize::from(saved.dead),
                "Automaton reward duplicated on load"
            );
            if saved.dead {
                ensure!(
                    !npcs.targets().iter().any(|t| t.id == saved.id),
                    "Dead Automaton resurrected"
                );
            }
            println!(
                "PASS Automaton {case}: exact fresh-process continuation, contact and reward ledger"
            );
            continue;
        }
        npcs.restore(&initial)?;
        let mut loot = crate::loot::Loot::default();
        let before = npcs.loot_sources();
        let continuation_eye = if matches!(case, "delay" | "dormant") {
            if case == "delay" {
                npcs.restore(&delayed)?;
            }
            Vec3::splat(10000.)
        } else if case == "fists" {
            eye
        } else {
            feet + (eye - target).truncate().normalize().extend(0.) * 65. + Vec3::Z * 48.
        };
        if matches!(case, "punch" | "contact" | "fists" | "steam") {
            let p = npcs.actors[id].clock.as_mut().unwrap();
            p.phase = match case {
                "fists" => Phase::FistFire,
                "steam" => Phase::SteamFire,
                _ => Phase::Punch,
            };
            p.time = 0.;
            p.yaw = (continuation_eye.y - feet.y).atan2(continuation_eye.x - feet.x);
            if matches!(case, "contact" | "fists" | "steam") {
                let frame =
                    npcs.models[npcs.actors[id].model].data.clips["attack_punch"].frame_time;
                let frames = if case == "contact" {
                    (frame * 9. * 60.).ceil() as usize + 1
                } else {
                    8
                };
                step(&mut npcs, &scene.world, continuation_eye, frames);
            }
        } else if case == "pain" {
            npcs.hit(Hit {
                id,
                damage: 65.,
                kind: DamageKind::Knife,
                knockback: Vec3::ZERO,
            });
        } else if matches!(case, "death" | "ice") {
            npcs.hit(Hit {
                id,
                damage: 400.,
                kind: if case == "ice" {
                    DamageKind::Ice
                } else {
                    DamageKind::Knife
                },
                knockback: Vec3::ZERO,
            });
            loot.defeated(&before, &npcs.loot_sources(), &scene.world);
            ensure!(
                loot.drops.len() == 1 && loot.drops[0].grade == crate::loot::Grade::Large,
                "Wrong Automaton reward"
            );
            step(&mut npcs, &scene.world, continuation_eye, 12);
        }
        let p = npcs.actors[id].clock.as_ref().unwrap();
        let state = serde_json::to_value(p)?;
        if case == "fists" {
            ensure!(!p.fists.is_empty(), "Restart fixture lacks live fist");
        }
        if case == "contact" {
            ensure!(
                state["contacts"].as_u64().unwrap() > 0,
                "Restart fixture lacks punch contact"
            );
        }
        if case == "steam" {
            ensure!(
                state["steam_hits"].as_u64().unwrap() > 0,
                "Restart fixture lacks steam contact"
            );
        }
        let cast = npcs.snapshot();
        let damage = step(&mut npcs, &scene.world, continuation_eye, 180);
        let expected = npcs.snapshot();
        if mode == "clock-save-write" {
            std::fs::write(
                path,
                serde_json::to_vec(&Saved {
                    cast,
                    expected,
                    loot,
                    eye: continuation_eye,
                    id,
                    dead: matches!(case, "death" | "ice"),
                    damage,
                })?,
            )?;
            println!("PASS Automaton {case}: saved live state and expected continuation");
        }
    }
    if mode != "clock-render" {
        return Ok(());
    }
    npcs.restore(&initial)?;
    npcs.actors = vec![npcs.actors[id].clone()];
    let stages = [
        ("idle", Phase::Idle, 0.2, false, 0),
        ("twitch", Phase::Twitch, 0.4, false, 0),
        ("wake", Phase::Wake, 0.3, false, 0),
        ("walk", Phase::Walk, 0.3, false, 0),
        ("fast", Phase::Fast, 0.15, false, 0),
        ("punch1", Phase::Punch, 0.36, false, 0),
        ("punch2", Phase::Punch, 0.48, false, 0),
        ("punch3", Phase::Punch, 0.64, false, 0),
        ("fist-ready", Phase::FistReady, 0.3, false, 0),
        ("fist-fire", Phase::FistFire, 0.2, false, 0),
        ("fist-back", Phase::FistBack, 0.3, false, 0),
        ("steam-ready", Phase::SteamReady, 0.3, false, 0),
        ("steam-fire", Phase::SteamFire, 0.5, false, 0),
        ("steam-back", Phase::SteamBack, 0.3, false, 0),
        ("pain1", Phase::Pain, 0.2, false, 0),
        ("pain2", Phase::Pain, 0.35, false, 1),
        ("pain3", Phase::Pain, 0.35, false, 2),
        ("death1", Phase::Dead, 0.9, false, 0),
        ("death2", Phase::Dead, 0.9, false, 1),
        ("frozen", Phase::Dead, 0.6, true, 0),
        ("removed", Phase::Dead, 20., false, 0),
    ];
    let mut hashes = BTreeSet::new();
    for (label, phase, time, frozen, variant) in stages {
        let p = npcs.actors[0].clock.as_mut().unwrap();
        p.phase = phase;
        p.time = time;
        p.frozen = frozen;
        p.variant = variant;
        p.health = if phase == Phase::Dead { 0. } else { 400. };
        p.yaw = (eye.y - p.feet.y).atan2(eye.x - p.feet.x);
        hashes.insert(capture(&mut npcs, &mut scene, &interactions, eye, target, label).await?);
    }
    // Launch a real fist through the same original tag sampler used in play.
    let yaw = (eye.y - feet.y).atan2(eye.x - feet.x);
    let mut p = Automaton::new(feet, yaw, 1., 0, true);
    p.phase = Phase::FistFire;
    let floor = World::fixture(&[(
        feet + vec3(-2000., -2000., -20.),
        feet + vec3(2000., 2000., -0.1),
    )]);
    let aim = feet + vec3(yaw.cos() * 500., yaw.sin() * 500., 48.);
    let data = &npcs.models[npcs.actors[0].model].data;
    let mut out = Feedback::default();
    for _ in 0..14 {
        p.update(1. / 60., &floor, aim, data, &mut out);
    }
    ensure!(
        !p.fists.is_empty(),
        "Original hand tags failed to launch a fist"
    );
    npcs.actors[0].clock = Some(p);
    ensure!(
        npcs.lights().len() == npcs.actors[0].clock.as_ref().unwrap().fists.len(),
        "Missing fist light"
    );
    hashes.insert(
        capture(
            &mut npcs,
            &mut scene,
            &interactions,
            eye,
            target,
            "live-fist",
        )
        .await?,
    );
    ensure!(hashes.len() == 22, "Automaton visual states identical");
    println!("PASS 22 Automaton animation/steam/fist/frozen/cleanup captures; trigger, pause and legacy migration");
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
    let path = format!("private/clock/{label}.png");
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
        crate::lighting::select(npcs.lights(), eye, &scene.world);
        crate::render_fx::begin_view(&camera, 0., &scene.atmosphere, false);
        let transforms = interactions.transforms();
        scene.draw(eye, 0., false, false, &transforms);
        npcs.draw(eye, (target - eye).normalize(), &scene.atmosphere, false);
        crate::render::depth_read_only(|| scene.draw(eye, 0., false, true, &transforms));
        let (_, dropped) = crate::render_fx::finish();
        ensure!(dropped == 0, "Automaton render queue dropped geometry");
        set_default_camera();
        if frame == 2 {
            crate::viewer::save_capture(std::path::Path::new(&path))?;
        }
        next_frame().await;
    }
    use sha2::Digest;
    Ok(sha2::Sha256::digest(std::fs::read(path)?).to_vec())
}
