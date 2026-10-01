//! Supplied-asset, renderer and separate-process persistence checks for chess enemies.
use super::*;
use crate::{
    chess::{Kind, Phase, Piece},
    combat::{DamageKind, Feedback, Hit},
};

pub fn check(assets: &mut Assets) -> Result<()> {
    attack_fx::check(assets)?;
    let floor = World::fixture(&[(vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.))]);
    for kind in Kind::ALL {
        let data = Data::load(assets, kind.model(), &[])?;
        for clip in kind.clips() {
            ensure!(data.clips.contains_key(*clip), "Missing {kind:?}/{clip}");
        }
        let mut results = Vec::new();
        for fps in [30, 60, 144] {
            let mut p = Piece::new(kind, Vec3::Z * 0.1, 0., 1., 11);
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
                out.damage > 0.,
                "{kind:?} did not attack with source timing"
            );
            p.validate()?;
            results.push((p.feet, out.damage));
        }
        for pair in results.windows(2) {
            ensure!(
                pair[0].0.distance(pair[1].0) < 0.1 && (pair[0].1 - pair[1].1).abs() < 0.1,
                "{kind:?} frame rate drift"
            );
        }
        println!("PASS {kind:?}: original clips and combat at 30/60/144 Hz");
    }
    for name in ["wforest", "wchess1", "wchess2", "rchess1"] {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let spawns = placements(&map, name, None, false);
        let reds = spawns
            .iter()
            .filter(|s| Kind::from_model(&s.model).is_some())
            .count();
        let delayed = spawns.iter().filter(|s| s.chess_spawn.is_some()).count();
        ensure!(spawns.len() < 256, "Chess cast hits NPC capacity");
        let ambushes = chess_spawns::ambushes(&map, name)?;
        for a in &ambushes {
            ensure!(
                a.touches(a.center + Vec3::Z * 20.),
                "Ambush volume mismatch"
            );
            for id in &a.entities {
                ensure!(
                    spawns.iter().any(|s| s.chess_spawn == Some(*id)),
                    "Missing chess receiver"
                );
            }
        }
        println!("PASS {name}: {reds} red pieces, {delayed} dormant receivers, {} reviewed ambush volumes; original white/royal cast retained",ambushes.len());
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
}

pub async fn native(assets: &mut Assets, mode: &str) -> Result<()> {
    std::fs::create_dir_all("private/chess")?;
    let mut scene = crate::render::Scene::load(assets, "rchess1")?;
    let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
    interactions.set_entry(assets, &scene.map, "rchess1", None)?;
    interactions.sync(&mut scene.world);
    for kind in Kind::ALL {
        let mut npcs = Npcs::load(assets, &scene.map, "rchess1", None, false, false)?;
        npcs.update(0., &scene.world, Vec3::splat(10000.));
        let id = npcs
            .actors
            .iter()
            .position(|a| {
                a.chess.as_ref().is_some_and(|p| p.kind == kind) && a.spawn.chess_spawn.is_some()
            })
            .context("Missing spawned chess type")?;
        let entity = npcs.actors[id].spawn.chess_spawn.unwrap();
        let trigger_eye = npcs
            .chess_ambushes
            .iter()
            .find(|a| a.entities.contains(&entity))
            .unwrap()
            .center
            + Vec3::Z * 20.;
        ensure!(
            !npcs.targets().iter().any(|t| t.id == id),
            "Dormant chess is targetable"
        );
        let old = npcs.snapshot();
        npcs.update(0., &scene.world, trigger_eye);
        ensure!(
            serde_json::to_value(&old)? == serde_json::to_value(npcs.snapshot())?,
            "Paused trigger changed NPCs"
        );
        for _ in 0..60 {
            npcs.update(1. / 60., &scene.world, trigger_eye);
        }
        ensure!(
            npcs.targets().iter().any(|t| t.id == id),
            "Authored ambush failed to activate"
        );
        let initial = npcs.snapshot();
        let target = npcs.actors[id].chess.as_ref().unwrap().target(id).center;
        let eye = (0..16)
            .map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / 16.;
                target + vec3(angle.cos() * 220., angle.sin() * 220., 25.)
            })
            .find(|p| {
                let t = scene.world.sweep(target, *p, Vec3::splat(2.));
                !t.start_solid && t.fraction >= 1.
            })
            .context("No clear chess camera")?;
        let path = format!("private/chess/{kind:?}-save.json");
        if mode == "chess-save-read" {
            let saved: Saved = serde_json::from_slice(&std::fs::read(path)?)?;
            saved.cast.matches_placements(&scene.map, "rchess1", None)?;
            npcs.restore(&saved.cast)?;
            let mut loot = saved.loot;
            loot.validate()?;
            let before = npcs.loot_sources();
            for _ in 0..120 {
                npcs.update(1. / 60., &scene.world, saved.eye);
            }
            loot.defeated(&before, &npcs.loot_sources(), &scene.world);
            ensure!(
                serde_json::to_value(npcs.snapshot())? == serde_json::to_value(saved.expected)?,
                "Chess continuation differs after process restart"
            );
            ensure!(
                loot.awarded.len() == 1
                    && loot.drops.len() == 1
                    && !npcs.targets().iter().any(|t| t.id == saved.id),
                "Chess death/reward changed on load"
            );
            println!(
                "PASS {kind:?}: fresh-process continuation, no resurrection and one essence drop"
            );
            continue;
        }
        // Genuine old cast: new ambush slots and combat fields did not exist.
        let mut legacy = old.clone();
        legacy.actors.retain(|a| a.spawn.chess_spawn.is_none());
        for a in &mut legacy.actors {
            a.chess = None;
        }
        legacy.matches_placements(&scene.map, "rchess1", None)?;
        let mut migrated = Npcs::load(assets, &scene.map, "rchess1", None, false, false)?;
        migrated.restore(&legacy)?;
        ensure!(
            !migrated.actors[id].chess.as_ref().unwrap().active,
            "Old cast wakes dormant ambushes"
        );
        npcs.restore(&initial)?;
        let before = npcs.loot_sources();
        npcs.hit(Hit {
            id,
            damage: 1000.,
            kind: DamageKind::Other,
            knockback: Vec3::ZERO,
        })
        .context("Chess did not accept weapon damage")?;
        let mut loot = crate::loot::Loot::default();
        loot.defeated(&before, &npcs.loot_sources(), &scene.world);
        ensure!(
            loot.drops.len() == 1 && loot.drops[0].grade == kind.grade(),
            "Wrong chess reward"
        );
        for _ in 0..20 {
            npcs.update(1. / 60., &scene.world, eye);
        }
        let cast = npcs.snapshot();
        for _ in 0..120 {
            npcs.update(1. / 60., &scene.world, eye);
        }
        let expected = npcs.snapshot();
        if mode == "chess-save-write" {
            std::fs::write(
                path,
                serde_json::to_vec(&Saved {
                    cast,
                    expected,
                    loot,
                    eye,
                    id,
                })?,
            )?;
            println!("PASS {kind:?}: wrote death and live encounter continuation fixture");
            continue;
        }
        npcs.restore(&initial)?;
        npcs.actors = vec![npcs.actors[id].clone()];
        let mut stages = vec![
            ("idle", Phase::Idle, 0.2, false, 0),
            ("walk", Phase::Chase, 0.3, false, 0),
            ("attack", Phase::Melee, 0.35, false, 0),
            ("pain", Phase::Pain, 0.3, false, 1),
            ("death1", Phase::Dead, 0.6, false, 0),
            ("death2", Phase::Dead, 1.2, false, 1),
            ("frozen", Phase::Dead, 0.5, true, 0),
            ("removed", Phase::Dead, 30., false, 0),
        ];
        if kind == Kind::Knight {
            stages.push(("block", Phase::Block, 0.1, false, 0));
        }
        if kind == Kind::Rook {
            stages.push(("charge", Phase::Charge, 0.1, false, 0));
        }
        if kind == Kind::Bishop {
            stages.push(("beam", Phase::Beam, 0.35, false, 0));
        }
        let count = stages.len();
        let mut hashes = BTreeSet::new();
        for (label, phase, time, frozen, variant) in stages {
            let p = npcs.actors[0].chess.as_mut().unwrap();
            p.phase = phase;
            p.health = if phase == Phase::Dead { 0. } else { 50. };
            p.time = time;
            p.frozen = frozen;
            p.variant = variant;
            p.yaw = (eye.y - p.feet.y).atan2(eye.x - p.feet.x);
            p.beam = (phase == Phase::Beam).then_some(crate::chess::Beam {
                from: p.feet + Vec3::Z * 72.,
                to: eye - Vec3::Z * 20.,
                age: 0.02,
            });
            let output = format!("private/chess/{kind:?}-{label}.png");
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
                ensure!(dropped == 0, "Chess render queue dropped geometry");
                set_default_camera();
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&output))?;
                }
                next_frame().await;
            }
            use sha2::Digest;
            hashes.insert(sha2::Sha256::digest(std::fs::read(output)?).to_vec());
        }
        ensure!(hashes.len() == count, "Chess render states are identical");
        println!("PASS {kind:?}: {count} distinct source-animation/frozen/cleanup captures, original ambush activation and legacy migration");
    }
    Ok(())
}
