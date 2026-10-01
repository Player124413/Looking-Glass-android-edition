//! Supplied assets, authored ambushes, renderer and fresh-process Imp persistence.
use super::*;
use crate::{
    combat::{DamageKind, Feedback, Hit},
    fire_imp::{Imp, Phase, MODEL},
};

pub fn check(assets: &mut Assets) -> Result<()> {
    let data = Data::load(assets, MODEL, &[])?;
    for clip in crate::fire_imp::CLIPS {
        ensure!(data.clips.contains_key(*clip), "Missing Imp clip {clip}");
    }
    for name in [
        "gb_meatbone1",
        "gb_meatbone2",
        "gb_meatbone3",
        "gb_ribs",
        "w_fork",
    ] {
        crate::weapons::read_model(assets, name)?;
    }
    let floor = World::fixture(&[(vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.))]);
    let mut results = Vec::new();
    for fps in [30, 60, 144] {
        let mut p = Imp::new(Vec3::Z * 0.1, 0., 1., 11, true);
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
            "Imp does not pursue/attack with supplied timing"
        );
        p.validate()?;
        results.push((p.feet, out.damage));
    }
    for pair in results.windows(2) {
        ensure!(
            pair[0].0.distance(pair[1].0) < 0.1 && pair[0].1 == pair[1].1,
            "Imp frame rate drift"
        );
    }
    println!("PASS Fire Imp source clips, fork/chunks, pursuit and contact at 30/60/144 Hz");
    for name in ["wforest", "jlair1", "hedge3", "facade"] {
        let mut map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        for difficulty in crate::powerups::Difficulty::ALL {
            map.difficulty = difficulty;
            let spawns = placements(&map, name, None, false);
            let imps = spawns.iter().filter(|s| s.model == MODEL).count();
            let dormant = spawns.iter().filter(|s| s.imp_spawn.is_some()).count();
            ensure!(spawns.len() < 256, "Imp cast reaches capacity");
            let ambushes = imp_spawns::ambushes(&map, name)?;
            for a in &ambushes {
                ensure!(
                    a.touches(a.center + Vec3::Z * 20.),
                    "Imp trigger volume mismatch"
                );
                for (id, delay) in &a.receivers {
                    ensure!(
                        spawns.iter().filter(|s| s.imp_spawn == Some(*id)).count() == 1
                            && (0. ..=30.).contains(delay),
                        "Missing or duplicate Imp receiver"
                    );
                }
            }
            println!(
            "PASS {name}/{difficulty:?}: {imps} Fire Imps, {dormant} dormant, {} ambush volumes",
            ambushes.len()
        );
        }
    }
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
    std::fs::create_dir_all("private/imp")?;
    let mut scene = crate::render::Scene::load(assets, "wforest")?;
    let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
    interactions.set_entry(assets, &scene.map, "wforest", None)?;
    interactions.sync(&mut scene.world);
    let mut npcs = Npcs::load(assets, &scene.map, "wforest", None, false, false)?;
    npcs.update(0., &scene.world, Vec3::splat(10000.));
    let initial = npcs.snapshot();
    let id = npcs
        .actors
        .iter()
        .position(|a| a.imp.is_some() && a.spawn.imp_spawn.is_none())
        .context("Missing placed Imp")?;
    let (pending_id, entity) = npcs
        .actors
        .iter()
        .enumerate()
        .find_map(|(id, a)| {
            a.spawn
                .imp_spawn
                .filter(|e| {
                    npcs.imp_ambushes
                        .iter()
                        .any(|b| b.receivers.iter().any(|(i, d)| i == e && *d > 1.))
                })
                .map(|e| (id, e))
        })
        .context("Missing delayed Imp")?;
    let ambush = npcs
        .imp_ambushes
        .iter()
        .find(|a| a.receivers.iter().any(|(i, _)| *i == entity))
        .unwrap();
    let trigger_eye = ambush.center + Vec3::Z * 20.;
    ensure!(
        !npcs.targets().iter().any(|t| t.id == pending_id),
        "Dormant Imp is targetable"
    );
    npcs.update(0., &scene.world, trigger_eye);
    ensure!(
        serde_json::to_value(&initial)? == serde_json::to_value(npcs.snapshot())?,
        "Paused Imp trigger changed state"
    );
    step(&mut npcs, &scene.world, trigger_eye, 1);
    ensure!(
        npcs.actors[pending_id]
            .imp
            .as_ref()
            .unwrap()
            .spawn_delay
            .is_some(),
        "Relay failed to schedule Imp"
    );
    let delayed = npcs.snapshot();
    step(&mut npcs, &scene.world, Vec3::splat(10000.), 180);
    ensure!(
        npcs.targets().iter().any(|t| t.id == pending_id),
        "Delayed Imp failed to spawn away from trigger"
    );
    // Old saves omit both the optional combat field and the appended ambush actors.
    let mut legacy = initial.clone();
    legacy.actors.retain(|a| a.spawn.imp_spawn.is_none());
    for a in &mut legacy.actors {
        a.imp = None;
    }
    legacy.matches_placements(&scene.map, "wforest", None)?;
    npcs.restore(&legacy)?;
    ensure!(
        !npcs.actors[pending_id].imp.as_ref().unwrap().active,
        "Old load retained a live new ambush"
    );
    let feet = npcs.actors[id].position();
    let target = feet + Vec3::Z * 25.;
    let eye = (0..32)
        .map(|i| {
            let angle = i as f32 * std::f32::consts::TAU / 32.;
            target + vec3(angle.cos() * 175., angle.sin() * 175., 25.)
        })
        .find(|p| {
            let t = scene.world.sweep(target, *p, Vec3::splat(2.));
            !t.start_solid && t.fraction >= 1.
        })
        .context("No clear Imp camera")?;
    for case in ["attack", "impact", "pain", "death", "ice", "gib", "delay"] {
        let path = format!("private/imp/{case}-save.json");
        if mode == "imp-save-read" {
            let saved: Saved = serde_json::from_slice(&std::fs::read(path)?)?;
            saved.cast.matches_placements(&scene.map, "wforest", None)?;
            npcs.restore(&saved.cast)?;
            let mut loot = saved.loot;
            loot.validate()?;
            let before = npcs.loot_sources();
            let damage = step(&mut npcs, &scene.world, saved.eye, 180);
            loot.defeated(&before, &npcs.loot_sources(), &scene.world);
            ensure!(
                serde_json::to_value(npcs.snapshot())? == serde_json::to_value(saved.expected)?
                    && damage == saved.damage,
                "Imp {case} continuation differs after restart"
            );
            ensure!(
                loot.awarded.len() == usize::from(saved.dead)
                    && loot.drops.len() == usize::from(saved.dead),
                "Imp reward duplicated on load"
            );
            if saved.dead {
                ensure!(
                    !npcs.targets().iter().any(|t| t.id == saved.id),
                    "Dead Imp resurrected"
                );
            }
            println!(
                "PASS Imp {case}: exact fresh-process continuation, contact and reward ledger"
            );
            continue;
        }
        npcs.restore(&initial)?;
        let mut loot = crate::loot::Loot::default();
        let before = npcs.loot_sources();
        let continuation_eye = if case == "delay" {
            npcs.restore(&delayed)?;
            Vec3::splat(10000.)
        } else {
            feet + vec3(65., 0., 48.)
        };
        if matches!(case, "attack" | "impact") {
            let time = npcs.models[npcs.actors[id].model].data.clips["attack01"].frame_time * 11.;
            let p = npcs.actors[id].imp.as_mut().unwrap();
            p.phase = Phase::Attack;
            p.time = time;
            if case == "impact" {
                step(&mut npcs, &scene.world, continuation_eye, 5);
            }
        } else if case == "pain" {
            npcs.hit(Hit {
                id,
                damage: 25.,
                kind: DamageKind::Knife,
                knockback: Vec3::ZERO,
            });
        } else if matches!(case, "death" | "ice" | "gib") {
            if case == "gib" {
                // Exercise the production hit path, including the fork attachment.
                let seed = (0..1000)
                    .find(|seed| {
                        let mut q = Imp::new(feet, 0., 1., *seed, true);
                        q.hit(Hit {
                            id,
                            damage: 100.,
                            kind: DamageKind::Other,
                            knockback: Vec3::ZERO,
                        });
                        q.gibbed
                    })
                    .unwrap();
                npcs.actors[id].imp = Some(Imp::new(feet, 0., 1., seed, true));
            }
            npcs.hit(Hit {
                id,
                damage: 100.,
                kind: if case == "ice" {
                    DamageKind::Ice
                } else if case == "gib" {
                    DamageKind::Other
                } else {
                    DamageKind::Knife
                },
                knockback: Vec3::ZERO,
            });
            loot.defeated(&before, &npcs.loot_sources(), &scene.world);
            ensure!(
                loot.drops.len() == 1 && loot.drops[0].grade == crate::loot::Grade::Small,
                "Wrong Imp reward"
            );
            step(&mut npcs, &scene.world, continuation_eye, 12);
        }
        let cast = npcs.snapshot();
        let damage = step(&mut npcs, &scene.world, continuation_eye, 180);
        let expected = npcs.snapshot();
        if mode == "imp-save-write" {
            std::fs::write(
                path,
                serde_json::to_vec(&Saved {
                    cast,
                    expected,
                    loot,
                    eye: continuation_eye,
                    id,
                    dead: matches!(case, "death" | "ice" | "gib"),
                    damage,
                })?,
            )?;
            println!("PASS Imp {case}: saved live state and expected continuation");
        }
    }
    if mode != "imp-render" {
        return Ok(());
    }
    npcs.restore(&initial)?;
    npcs.actors = vec![npcs.actors[id].clone()];
    let stages = [
        ("idle", Phase::Idle, 0.2, false, 0),
        ("twitch", Phase::Twitch, 0.4, false, 0),
        ("alert", Phase::Alert, 0.3, false, 1),
        ("walk", Phase::Walk, 0.3, false, 0),
        ("run", Phase::Run, 0.15, false, 0),
        ("attack", Phase::Attack, 0.5, false, 0),
        ("pain1", Phase::Pain, 0.2, false, 0),
        ("pain2", Phase::Pain, 0.35, false, 1),
        ("death1", Phase::Dead, 0.9, false, 0),
        ("death2", Phase::Dead, 0.9, false, 1),
        ("frozen", Phase::Dead, 0.6, true, 0),
        ("removed", Phase::Dead, 20., false, 0),
    ];
    let mut hashes = BTreeSet::new();
    for (label, phase, time, frozen, variant) in stages {
        let p = npcs.actors[0].imp.as_mut().unwrap();
        p.phase = phase;
        p.time = time;
        p.frozen = frozen;
        p.variant = variant;
        p.health = if phase == Phase::Dead { 0. } else { 35. };
        p.yaw = (eye.y - p.feet.y).atan2(eye.x - p.feet.x);
        hashes.insert(capture(&mut npcs, &mut scene, &interactions, eye, target, label).await?);
    }
    // Render real detached parts from the saved production lethal-hit state.
    let seed = (0..1000)
        .find(|seed| {
            let mut p = Imp::new(feet, 0., 1., *seed, true);
            p.hit(Hit {
                id: 0,
                damage: 100.,
                kind: DamageKind::Other,
                knockback: Vec3::ZERO,
            });
            p.gibbed
        })
        .unwrap();
    npcs.actors[0].imp = Some(Imp::new(feet, 0., 1., seed, true));
    npcs.hit(Hit {
        id: 0,
        damage: 100.,
        kind: DamageKind::Other,
        knockback: Vec3::ZERO,
    });
    step(&mut npcs, &scene.world, eye, 8);
    hashes.insert(capture(&mut npcs, &mut scene, &interactions, eye, target, "gib").await?);
    ensure!(hashes.len() == 13, "Imp visual states identical");
    println!("PASS 13 Imp animation/fork/frozen/gib/cleanup captures; authored relay, pause and legacy migration");
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
    let path = format!("private/imp/{label}.png");
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
        ensure!(dropped == 0, "Imp render queue dropped geometry");
        set_default_camera();
        if frame == 2 {
            crate::viewer::save_capture(std::path::Path::new(&path))?;
        }
        next_frame().await;
    }
    use sha2::Digest;
    Ok(sha2::Sha256::digest(std::fs::read(path)?).to_vec())
}
