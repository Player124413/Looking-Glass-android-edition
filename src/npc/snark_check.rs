//! Audits locally supplied facts and swimming at real placements; no scripts execute.
use super::*;
use crate::{
    clockwork::Rig,
    snark::{Kind, Snark},
};
pub fn check(assets: &mut Assets) -> Result<()> {
    for kind in Kind::ALL {
        let data = Data::load(assets, kind.model(), &[])?;
        let source = crate::bsp::tokens(&String::from_utf8_lossy(
            &assets.read(&format!("models/{}.tik", kind.model()))?,
        ))?;
        for fact in [
            vec!["health", if kind == Kind::Fire { "100" } else { "25" }],
            vec!["8", "melee", "10"],
            vec!["visiondistance", "1000"],
        ] {
            ensure!(
                source
                    .windows(fact.len())
                    .any(|w| w.iter().map(String::as_str).eq(fact.iter().copied())),
                "Snark source fact changed: {fact:?}"
            );
        }
        for clip in kind.clips() {
            ensure!(data.clips[clip].duration() > 0., "Empty Snark clip");
            let mouth = data.tag(clip, 0., "tag_mouth");
            ensure!(mouth.translation.is_finite(), "Invalid Snark mouth");
        }
        ensure!(
            data.clips["bite"].frames.len() > 8,
            "Snark bite cue beyond clip"
        );
        if kind != Kind::BiteOnly {
            let fact = [
                "8",
                "proj",
                "tag_mouth",
                &format!("{}.tik", kind.projectile()),
            ];
            ensure!(
                source
                    .windows(fact.len())
                    .any(|w| w.iter().map(String::as_str).eq(fact)),
                "Snark projectile cue changed"
            );
            let source = crate::bsp::tokens(&String::from_utf8_lossy(
                &assets.read(&format!("models/{}.tik", kind.projectile()))?,
            ))?;
            for fact in [
                ["speed", if kind == Kind::Fire { "700" } else { "600" }],
                ["hitdamage", if kind == Kind::Fire { "25" } else { "5" }],
            ] {
                ensure!(
                    source
                        .windows(2)
                        .any(|w| w.iter().map(String::as_str).eq(fact)),
                    "Snark projectile values changed"
                );
            }
        }
        if kind == Kind::Fire {
            for tag in ["tag_pipe01", "tag_pipe03", "tag_pipe06"] {
                ensure!(
                    data.skeleton.bones.iter().any(|b| b.name == tag),
                    "Fire Snark smoke tag missing"
                );
            }
        }
        if kind != Kind::BiteOnly {
            let mut world =
                World::fixture(&[(vec3(-1000., -1000., -100.), vec3(1000., 1000., -50.))]);
            world.set_dynamic_liquids(vec![crate::collision::Liquid {
                contents: if kind == Kind::Fire { 8 } else { 32 },
                volume: crate::collision::Collider::box_bounds(
                    vec3(-1000., -1000., -50.),
                    vec3(100., 1000., 100.),
                ),
            }]);
            let mut p = Snark::new(kind, Vec3::ZERO, 0., 1., 7);
            let mut feedback = combat::Feedback::default();
            let mut shot = false;
            for _ in 0..3600 {
                p.step(&world, vec3(350., 0., 180.), &data, &mut feedback);
                shot |= !p.shots.is_empty();
            }
            ensure!(
                shot && feedback.damage > 0.,
                "Original Snark clips failed to spit from a bank: {}",
                kind.model()
            );
            p.validate()?;
        }
        println!(
            "PASS {}: source health, melee/projectile cues, clips and mouth",
            kind.model()
        );
    }
    for map_name in [
        "potears1",
        "garden1",
        "centipede1",
        "hedge2",
        "hedge3",
        "jlair1",
        "wforest",
        "tower2",
        "utemple",
    ] {
        let map = Bsp::parse(&assets.read(&format!("maps/{map_name}.bsp"))?)?;
        let mut world = World::from_bsp(&map)?;
        let mut interactions = crate::interaction::Interactions::load(&map)?;
        interactions.set_entry(assets, &map, map_name, None)?;
        interactions.sync(&mut world);
        for spawn in placements(&map, map_name, None, false)
            .iter()
            .filter(|s| Kind::from_model(&s.model).is_some() && s.resident_spawn.is_none())
        {
            let kind = Kind::from_model(&spawn.model).unwrap();
            let data = Data::load(assets, &spawn.model, &[])?;
            let mut p = Snark::new(kind, spawn.origin, spawn.yaw, spawn.scale, 1);
            p.notarget = true;
            for _ in 0..360 {
                p.step(
                    &world,
                    Vec3::splat(10000.),
                    &data,
                    &mut combat::Feedback::default(),
                );
            }
            p.validate()?;
            ensure!(
                kind.wet(&world, p.feet),
                "Snark failed to settle into its liquid: {map_name} {:?}",
                spawn.origin
            );
            println!(
                "SNARK PLACEMENT {map_name} {} {:?}: liquid={} settled={:?}",
                spawn.model,
                spawn.origin,
                world.liquid_at(p.feet),
                p.feet
            );
        }
    }
    Ok(())
}
