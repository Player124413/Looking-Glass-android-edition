//! Real-data progression gates, weapon switches and secret reward regressions.
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::World,
    combat::{self, Hit},
    interaction::{vector, Interactions},
    inventory::Stats,
    movement::{Player, FIXED_DT},
};
use anyhow::{Context, Result};
use macroquad::prelude::*;
pub fn check(assets: &mut Assets) -> Result<()> {
    village(assets)?;
    let map = Bsp::parse(&assets.read("maps/skool1.bsp")?)?;
    let center = |name: &str| -> Result<Vec3> {
        map.entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|s| s == name))
            .and_then(|e| e.get("origin"))
            .and_then(|s| vector(s))
            .context("Missing trigger")
    };
    for entry in [None, Some("skool1_start2")] {
        let returning = entry.is_some();
        let mut i = Interactions::load(&map)?;
        i.set_entry(assets, &map, "skool1", entry)?;
        for (trigger, group, count, enabled) in [
            ("first_monster_trigger1", "t188", 4, !returning),
            ("lastpass_trigger", "t187", 4, returning),
            ("return_booj_trigger", "t189", 5, returning),
        ] {
            let feet = center(trigger)? - Vec3::Z * 28.;
            i.triggers(FIXED_DT, feet, feet);
            i.activate_enemies();
            let actors = &i.encounters.as_ref().unwrap().actors;
            anyhow::ensure!(
                actors
                    .iter()
                    .filter(|a| a.name == group && a.active)
                    .count()
                    == if enabled { count } else { 0 },
                "Wrong {group} activation for {entry:?}"
            );
            i.reset_contacts();
            i.triggers(FIXED_DT, feet, feet);
            anyhow::ensure!(i.take_activations().is_empty(), "One-shot {group} replayed");
        }
        if returning {
            anyhow::ensure!(
                i.switch_target("Open_Bookcase_Goodie").is_none(),
                "Return visit secret switch active"
            );
        }
    }
    let mut i = Interactions::load(&map)?;
    i.set_entry(assets, &map, "skool1", None)?;
    let mut world = World::from_bsp(&map)?;
    i.sync(&mut world);
    let switch = i
        .switch_target("Open_Bookcase_Goodie")
        .context("Missing shootable face")?;
    i.triggers(
        FIXED_DT,
        switch.center - Vec3::Z * 28.,
        switch.center - Vec3::Z * 28.,
    );
    anyhow::ensure!(
        !i.school.as_ref().unwrap().secret_open,
        "Touch opened shootable switch"
    );
    let targets = [switch];
    let ctx = combat::Context {
        world: &world,
        targets: &targets,
    };
    anyhow::ensure!(
        combat::contact(&ctx, vec3(-2554., 2624., -432.), switch.center, 1.).is_some(),
        "Face cannot be shot from theatre"
    );
    anyhow::ensure!(
        combat::contact(&ctx, vec3(-2303., 2624., -294.), switch.center, 1.).is_none(),
        "Weapon shoots switch through back wall"
    );
    let mut stats = Stats::default();
    let reward = vec3(624., 2581., -240.);
    i.school
        .as_mut()
        .unwrap()
        .collect_secret(&mut stats, reward, &world);
    anyhow::ensure!(
        stats.invisible == 0.,
        "Secret collected before opening panel"
    );
    i.shoot(Hit {
        knockback: Vec3::ZERO,
        kind: crate::combat::DamageKind::Other,
        id: switch.id,
        damage: 1.,
    });
    i.sync(&mut world);
    let event = i
        .school
        .as_mut()
        .unwrap()
        .collect_secret(&mut stats, reward, &world);
    anyhow::ensure!(
        stats.invisible == 45. && event.story == ["Cat_Glass_Dialog"],
        "Secret reward/voice missing"
    );
    stats.update(10.);
    i.school
        .as_mut()
        .unwrap()
        .collect_secret(&mut stats, reward, &world);
    anyhow::ensure!(
        stats.invisible == 35.,
        "Repeat contact renewed secret reward"
    );
    i.reset_contacts();
    anyhow::ensure!(
        i.switch_target("Open_Bookcase_Goodie").is_none(),
        "Recovery rearmed secret"
    );
    stats.update(35.);
    anyhow::ensure!(stats.invisible == 0., "Invisibility never expires");
    anyhow::ensure!(
        !i.school
            .as_ref()
            .unwrap()
            .trigger_enabled("", "", Some("skool2")),
        "Secret bypassed recipe gate"
    );
    // The solid panel is absent after opening, while the face animates east by 16 units.
    let mut p = Player::new(vec3(-2554., 2624., -479.96875));
    for _ in 0..250 {
        i.advance_school(FIXED_DT, &map, &mut world, &mut p)?;
    }
    let transforms = i.transforms();
    anyhow::ensure!(
        !transforms.iter().any(|t| t.0 == 1),
        "Secret panel still draws"
    );
    anyhow::ensure!(
        transforms
            .iter()
            .any(|t| t.0 == 4 && (t.1.x + 2460.).abs() < 0.01),
        "Secret face did not move"
    );
    println!("PASS first/return enemy gates; one-shot activations; shootable face LOS; secret panel, unique 45-second reward, dialogue and unchanged recipe gate");
    Ok(())
}

fn village(assets: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&assets.read("maps/gvillage.bsp")?)?;
    let mut results = Vec::new();
    for fps in [30, 60, 144] {
        let mut i = Interactions::load(&map)?;
        i.set_entry(assets, &map, "gvillage", None)?;
        let mut world = World::from_bsp(&map)?;
        i.sync(&mut world);
        let mut p = Player::spawn(&world, crate::interaction::spawn(&map, None).0)
            .context("Village arrival blocked")?;
        let start = i.transforms();
        i.advance_school(0., &map, &mut world, &mut p)?;
        anyhow::ensure!(
            start == i.transforms(),
            "Village movers changed while paused"
        );
        for _ in 0..fps * 4 {
            i.advance_school(1. / fps as f32, &map, &mut world, &mut p)?;
        }
        let end = i.transforms();
        anyhow::ensure!(end.len() >= 30, "Village machinery missing");
        anyhow::ensure!(
            start.iter().zip(&end).filter(|(a, b)| a != b).count() >= 10,
            "Village machinery failed to move"
        );
        anyhow::ensure!(
            world.body_clear(p.feet),
            "Village machinery trapped arrival"
        );
        results.push(end);
        let hatch = i
            .switch_target("openthis")
            .context("Village weapon hatch missing")?;
        let feet = hatch.center - Vec3::Z * 28.;
        i.triggers(FIXED_DT, feet, feet);
        anyhow::ensure!(
            i.switch_target("openthis").is_some(),
            "Walking opened weapon hatch"
        );
        for _ in 0..2 {
            i.shoot(Hit {
                knockback: Vec3::ZERO,
                kind: crate::combat::DamageKind::Other,
                id: hatch.id,
                damage: 35.,
            });
        }
        anyhow::ensure!(
            i.switch_target("openthis").is_some(),
            "Hatch opened below authored health"
        );
        i.shoot(Hit {
            knockback: Vec3::ZERO,
            kind: crate::combat::DamageKind::Other,
            id: hatch.id,
            damage: 35.,
        });
        anyhow::ensure!(
            i.switch_target("openthis").is_none(),
            "Hatch did not open after damage"
        );
        i.completed_dialogue("Torchgnome3_Dialog_part2");
        i.activate_enemies();
        let enemies = i.encounters.as_mut().unwrap();
        let guard = enemies
            .actors
            .iter_mut()
            .find(|a| a.name == "guard_rabbit")
            .context("Missing village guard")?;
        anyhow::ensure!(guard.active, "Village dialogue did not activate guard");
        if let crate::encounters::Enemy::Guard(g) = &mut guard.enemy {
            g.hurt(1000.);
        }
        i.completed_dialogue("Torchgnome3_Dialog_part2");
        i.activate_enemies();
        anyhow::ensure!(
            i.encounters.as_ref().unwrap().targets().is_empty(),
            "Replayed dialogue revived guard"
        );
    }
    for r in &results[1..] {
        for (a, b) in results[0].iter().zip(r) {
            anyhow::ensure!(
                a.0 == b.0 && a.1.distance(b.1) < 0.1 && a.2.dot(b.2).abs() > 0.9999,
                "Village mover {} depends on display rate",
                a.0
            );
        }
    }
    println!("PASS village machinery pause and 30/60/144 Hz transforms; weapon-only hatch threshold; dialogue guard activation/death without replay respawn");
    Ok(())
}
