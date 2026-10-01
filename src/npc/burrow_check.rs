//! Original facts and safe placement checks, plus an explicitly staged larva encounter.
use super::*;
use crate::burrow::{Insect, Kind, Phase};
/// Original larvae are boss-spawned. This opt-in fixture never changes campaign placements.
pub fn larva_fixture(map: &mut Bsp) -> Result<()> {
    let mut entity = map
        .entities
        .iter()
        .find(|e| {
            e.get("model")
                .is_some_and(|m| m == "c_antlion.tik" && !e.contains_key("spawnflags"))
        })
        .context("Missing Antlion fixture anchor")?
        .clone();
    entity.insert("model".into(), "c_larva.tik".into());
    entity.insert("classname".into(), "Enemies_Larva".into());
    entity.insert("targetname".into(), "larva_playtest".into());
    map.entities.retain(|e| {
        !e.get("classname")
            .is_some_and(|c| c.to_lowercase().starts_with("enemies_"))
    });
    map.entities.push(entity);
    Ok(())
}
pub fn check(assets: &mut Assets) -> Result<()> {
    for kind in Kind::ALL {
        let data = Data::load(assets, kind.model(), &[])?;
        let source = crate::bsp::tokens(&String::from_utf8_lossy(
            &assets.read(&format!("models/{}.tik", kind.model()))?,
        ))?;
        for fact in [
            vec!["health", if kind == Kind::Larva { "7" } else { "120" }],
            vec!["visiondistance", "1000"],
        ] {
            ensure!(
                source
                    .windows(fact.len())
                    .any(|w| w.iter().map(String::as_str).eq(fact.iter().copied())),
                "Insect source fact changed: {fact:?}"
            );
        }
        for clip in kind.clips() {
            ensure!(data.clips[*clip].duration() > 0., "Empty insect clip");
        }
        if kind != Kind::Larva {
            let mut insect = Insect::new(kind, Vec3::ZERO, 0., 1., 7);
            for variant in 0..3 {
                insect.variant = variant;
                insect.set(Phase::Melee);
                for &(frame, damage) in insect.melee_events() {
                    let fact = [
                        format!("{frame:.0}"),
                        "melee".into(),
                        format!("{damage:.0}"),
                    ];
                    ensure!(
                        source.windows(3).any(|w| w == fact),
                        "Missing insect attack cue"
                    );
                    ensure!(
                        data.clips[insect.clip()].frames.len() > frame as usize,
                        "Attack cue beyond clip"
                    );
                }
            }
            for tag in ["tag_dirt01", "tag_dirt02", "tag_dirt03"] {
                ensure!(
                    data.skeleton.bones.iter().any(|b| b.name == tag),
                    "Missing dirt attachment"
                );
            }
            for clip in ["burrow_up", "burrow_down"] {
                for emitter in ["dirt1", "dirt2", "dirt3"] {
                    let c = &data.clips[clip];
                    ensure!(
                        data.events
                            .visual(clip, 0.1, c.duration(), c.frame_time, false)
                            .emitters
                            .get(emitter)
                            == Some(&true),
                        "Burrow emitter did not activate"
                    );
                }
            }
        }
        println!(
            "PASS {}: source health, clips, contacts and attachments",
            kind.model()
        );
    }
    for name in ["garden2", "centipede1"] {
        let mut map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let world = World::from_bsp(&map)?;
        for difficulty in crate::powerups::Difficulty::ALL {
            map.difficulty = difficulty;
            let spawns = placements(&map, name, None, false);
            let mut count = 0;
            for s in spawns
                .iter()
                .filter(|s| Kind::from_model(&s.model).is_some())
            {
                let mut p = ResidentForCheck::new(s, &map, name)?.unwrap();
                p.place(&world);
                p.validate(s)?;
                count += 1;
                let b = p.target(0);
                ensure!(
                    !world.sweep(b.center, b.center, b.half).start_solid,
                    "Blocked insect placement {name} {:?}",
                    s.origin
                );
            }
            ensure!(count > 0, "Missing Antlion cast");
            println!("PASS insect placement {name}/{difficulty:?}: {count}");
        }
    }
    Ok(())
}
use resident::Resident as ResidentForCheck;

impl Npcs {
    pub fn larva_preview(assets: &mut Assets) -> Result<Self> {
        let mut map = Bsp::parse(&assets.read("maps/centipede1.bsp")?)?;
        larva_fixture(&mut map)?;
        let mut npcs = Self::load(assets, &map, "centipede1", None, false, false)?;
        npcs.actors.retain(|a| a.spawn.model == "c_larva");
        npcs.resident_ambushes.clear();
        Ok(npcs)
    }
    pub fn larva_preview_start(&self, world: &World) -> Result<(Vec3, f32)> {
        let actor = self
            .actors
            .iter()
            .find(|a| a.spawn.model == "c_larva")
            .context("Larva preview actor missing")?;
        let target = actor.spawn.origin;
        for i in 0..32 {
            let angle = i as f32 * std::f32::consts::TAU / 32.;
            let candidate = target + vec3(angle.cos() * 180., angle.sin() * 180., 48.);
            if let Some(player) = crate::movement::Player::spawn(world, candidate) {
                let eye = player.feet + Vec3::Z * 48.;
                let sight = world.sweep(target + Vec3::Z * 16., eye, Vec3::splat(1.));
                if !sight.start_solid && sight.fraction >= 1. {
                    return Ok((eye, (target.y - eye.y).atan2(target.x - eye.x)));
                }
            }
        }
        anyhow::bail!("No safe larva preview start")
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct AmbushSave {
    cast: Snapshot,
    expected: Snapshot,
    eye: Vec3,
}
pub async fn ambush_native(assets: &mut Assets, mode: &str) -> Result<()> {
    let scene = crate::render::Scene::load(assets, "garden2")?;
    let mut npcs = Npcs::load(assets, &scene.map, "garden2", None, false, false)?;
    npcs.notarget(true);
    npcs.update(0., &scene.world, Vec3::splat(10000.));
    let initial = npcs.snapshot();
    let mut previous = initial.clone();
    previous
        .actors
        .retain(|a| Kind::from_model(&a.spawn.model).is_none());
    previous.matches_placements(&scene.map, "garden2", None)?;
    npcs.restore(&previous)?;
    for (i, a) in npcs.actors.iter().enumerate() {
        if Kind::from_model(&a.spawn.model).is_some() {
            ensure!(
                !a.resident.as_ref().unwrap().active,
                "Upgraded ambush active"
            );
            continue;
        }
        ensure!(
            serde_json::to_value(a)? == serde_json::to_value(&initial.actors[i])?,
            "Garden migration changed an existing actor or activated an ambush"
        );
    }
    let eyes: Vec<_> = npcs
        .resident_ambushes
        .iter()
        .filter(|t| {
            t.receivers.iter().any(|(id, _)| {
                scene.map.entities[*id]
                    .get("model")
                    .is_some_and(|m| m.starts_with("c_antlion"))
            })
        })
        .map(|t| t.center + Vec3::Z * 20.)
        .collect();
    ensure!(eyes.len() == 4, "Missing Garden2 Antlion trigger branches");
    for (i, eye) in eyes.into_iter().enumerate() {
        let path = format!("private/residents/garden2-antlion-trigger-{i}.json");
        if mode == "resident-save-read" {
            let saved: AmbushSave = serde_json::from_slice(&std::fs::read(path)?)?;
            saved.cast.matches_placements(&scene.map, "garden2", None)?;
            npcs.restore(&saved.cast)?;
            for _ in 0..120 {
                npcs.update(1. / 60., &scene.world, saved.eye);
            }
            ensure!(
                serde_json::to_value(npcs.snapshot())? == serde_json::to_value(saved.expected)?,
                "Garden activation differs after fresh-process restore"
            );
        } else {
            npcs.restore(&initial)?;
            npcs.update(0., &scene.world, eye);
            ensure!(
                serde_json::to_value(npcs.snapshot())? == serde_json::to_value(&initial)?,
                "Paused Garden trigger changed state"
            );
            let cast = npcs.snapshot();
            for _ in 0..120 {
                npcs.update(1. / 60., &scene.world, eye);
            }
            ensure!(
                npcs.actors.iter().any(|a| matches!(
                    &a.resident,
                    Some(resident::Resident {
                        body: resident::Body::Insect(_),
                        active: true,
                        ..
                    })
                )),
                "Garden touch did not activate Antlions"
            );
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
        println!("PASS Garden2 Antlion trigger {i}: pause, activation, migration and continuation");
    }
    Ok(())
}
