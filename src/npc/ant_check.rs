//! Source-data combat and staged native Ant regression fixtures, never player saves.
use super::*;
use crate::ant::{Ant, Phase};
use crate::combat::{DamageKind, Feedback, Hit};

pub fn check(assets: &mut Assets) -> Result<()> {
    let mut placements_count = 0;
    for name in [
        "potears1",
        "potears2",
        "garden1",
        "garden2",
        "centipede1",
        "centipede2",
    ] {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let world = World::from_bsp(&map)?;
        for s in placements(&map, name, None, false)
            .into_iter()
            .filter(|s| crate::ant::is_ant(&s.model))
        {
            let d = Data::load(assets, &s.model, &[])?;
            for clip in if s.model == "c_armyantcorp" {
                crate::ant::CORPORAL
            } else {
                crate::ant::REGULAR
            } {
                ensure!(
                    d.clips.contains_key(*clip),
                    "Missing Ant clip {}/{clip}",
                    s.model
                );
            }
            let mut a = Ant::new(
                placements_count,
                s.model == "c_armyantcorp",
                s.origin,
                s.yaw,
            );
            a.scale = s.scale;
            let target = a.target(0);
            a.feet = world
                .actor_footing(a.feet, Vec3::Z * target.half.z, target.half, 1024.)
                .unwrap_or(a.feet);
            a.validate()?;
            placements_count += 1;
        }
    }
    // Actual animation data with a controlled arena isolates combat timing from terrain.
    let floor = World::fixture(&[(vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.))]);
    for corporal in [false, true] {
        let mut results = Vec::new();
        for fps in [30, 60, 144] {
            let mut a = Ant::new(11, corporal, Vec3::Z * 0.1, 0.);
            let data = Data::load(assets, a.model(), &[])?;
            let mut out = Feedback::default();
            for _ in 0..fps * 20 {
                a.update(
                    1. / fps as f32,
                    &floor,
                    vec3(450., 0., 48.),
                    &data,
                    &mut out,
                );
            }
            ensure!(
                out.damage > 0. && a.feet.x > 100.,
                "Ant did not pursue and attack with source clips"
            );
            a.validate()?;
            results.push((a.feet, out.damage));
        }
        for pair in results.windows(2) {
            ensure!(
                pair[0].0.distance(pair[1].0) < 0.1 && (pair[0].1 - pair[1].1).abs() < 0.1,
                "Ant frame rate drift"
            );
        }
    }
    println!("PASS Ant combat: {placements_count} eligible cast placements across six maps; soldier/Corporal source clips, pursuit and attacks at 30/60/144 Hz");
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Saved {
    cast: Snapshot,
    loot: crate::loot::Loot,
    expected: Snapshot,
    id: usize,
    eye: Vec3,
}

pub async fn native(assets: &mut Assets, mode: &str) -> Result<()> {
    std::fs::create_dir_all("private/ants")?;
    for (map, corporal) in [("potears1", false), ("potears2", true)] {
        let mut scene = crate::render::Scene::load(assets, map)?;
        let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
        interactions.set_entry(assets, &scene.map, map, None)?;
        interactions.sync(&mut scene.world);
        let mut npcs = Npcs::load(assets, &scene.map, map, None, false, false)?;
        npcs.update(0., &scene.world, Vec3::splat(10000.));
        ensure!(
            npcs.actors
                .iter()
                .filter(|a| npcs.registry_owns(a))
                .all(|a| a.ant.is_none()),
            "Scripted Ant acquired duplicate combat ownership"
        );
        let id = npcs
            .actors
            .iter()
            .position(|a| a.ant.as_ref().is_some_and(|g| g.corporal == corporal))
            .context("Missing generic Ant fixture")?;
        let initial = npcs.snapshot();
        let a = npcs.actors[id].ant.as_ref().unwrap();
        let target = a.target(id).center;
        let eye = (0..32)
            .filter_map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / 32.;
                let p = target + vec3(angle.cos() * 230., angle.sin() * 230., 32.);
                let trace = scene.world.sweep(target, p, Vec3::splat(2.));
                (!trace.start_solid && trace.fraction >= 1.).then_some(p)
            })
            .next()
            .context("No clear Ant camera")?;
        let path = format!("private/ants/{map}-save.json");
        if mode == "ant-save-read" {
            let saved: Saved = serde_json::from_slice(&std::fs::read(path)?)?;
            saved.cast.matches_placements(&scene.map, map, None)?;
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
                "Fresh-process Ant continuation differs"
            );
            ensure!(
                loot.awarded.len() == 1
                    && loot.drops.len() == 1
                    && !npcs.targets().iter().any(|t| t.id == saved.id),
                "Ant death/reward changed after load"
            );
            println!("PASS {map} Ant fresh-process save, continued state, dead target and exactly one reward");
            continue;
        }
        // Migration from decorative NPCs preserves the authored cast and placement.
        let mut old = initial.clone();
        for a in &mut old.actors {
            a.ant = None;
        }
        npcs.restore(&old)?;
        ensure!(
            npcs.actors[id].ant.is_some()
                && npcs.actors[id].position() == initial.actors[id].position(),
            "Legacy Ant migration failed"
        );
        npcs.restore(&initial)?;
        let before = npcs.loot_sources();
        npcs.hit(Hit {
            id,
            damage: 1000.,
            kind: DamageKind::Other,
            knockback: Vec3::ZERO,
        })
        .context("Ant did not accept weapon hit")?;
        let mut loot = crate::loot::Loot::default();
        loot.defeated(&before, &npcs.loot_sources(), &scene.world);
        ensure!(
            loot.drops.len() == 1
                && loot.drops[0].grade
                    == if corporal {
                        crate::loot::Grade::Large
                    } else {
                        crate::loot::Grade::Medium
                    },
            "Wrong Ant death reward"
        );
        for _ in 0..20 {
            npcs.update(1. / 60., &scene.world, eye);
        }
        let cast = npcs.snapshot();
        for _ in 0..120 {
            npcs.update(1. / 60., &scene.world, eye);
        }
        let expected = npcs.snapshot();
        if mode == "ant-save-write" {
            std::fs::write(
                path,
                serde_json::to_vec(&Saved {
                    cast,
                    loot,
                    expected,
                    id,
                    eye,
                })?,
            )?;
            println!("PASS wrote {map} Ant death and live-combat continuation fixture");
            continue;
        }
        npcs.restore(&initial)?;
        // Hide neighbours only in the staged visual fixture; saved cast checks use all actors.
        let actor = npcs.actors[id].clone();
        npcs.actors = vec![actor];
        let mut hashes = BTreeSet::new();
        for (label, phase, time, frozen, variant) in [
            ("idle", Phase::Idle, 0.2, false, 0),
            ("melee", Phase::Melee, 0.4, false, 0),
            ("fire", Phase::Fire, 0.5, false, 0),
            ("pain", Phase::Pain, 0.3, false, 1),
            ("death1", Phase::Dead, 0.6, false, 0),
            ("death2", Phase::Dead, 1.2, false, 1),
            ("death3", Phase::Dead, 1.2, false, 2),
            ("frozen", Phase::Dead, 0.5, true, 0),
            ("removed", Phase::Dead, 30., false, 0),
        ] {
            let a = npcs.actors[0].ant.as_mut().unwrap();
            a.phase = phase;
            a.health = if phase == Phase::Dead { 0. } else { 50. };
            a.time = time;
            a.frozen = frozen;
            a.variant = variant;
            a.yaw = (eye.y - a.feet.y).atan2(eye.x - a.feet.x);
            let output = format!("private/ants/{map}-{label}.png");
            for frame in 0..3 {
                clear_background(BLACK);
                set_camera(&Camera3D {
                    position: eye,
                    target,
                    up: Vec3::Z,
                    fovy: 65_f32.to_radians(),
                    z_near: 2.,
                    z_far: 30000.,
                    ..Default::default()
                });
                let transforms = interactions.transforms();
                scene.draw(eye, 0., false, false, &transforms);
                npcs.draw(eye, (target - eye).normalize(), &scene.atmosphere, false);
                crate::render::depth_read_only(|| scene.draw(eye, 0., false, true, &transforms));
                set_default_camera();
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&output))?;
                }
                next_frame().await;
            }
            use sha2::Digest;
            hashes.insert(sha2::Sha256::digest(std::fs::read(output)?).to_vec());
        }
        ensure!(
            hashes.len() == 9,
            "Ant animation/frozen/death views are identical"
        );
        println!("PASS {map}: staged soldier/Corporal idle, melee, ranged, pain, three deaths, freeze and cleanup; legacy NPC migration and essence reward");
    }
    Ok(())
}
