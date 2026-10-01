//! Actual trigger contacts, W9 motion, saved actors and post-scene hazards.
use super::cinema_check::{setup, tick, trigger};
use super::*;
use crate::{
    assets::Assets,
    combat::{DamageKind, Hit},
    interaction::Interactions,
    story::Story,
};
pub fn stage(
    case: &str,
    assets: &mut Assets,
    map: &Bsp,
    i: &mut Interactions,
    world: &mut World,
    player: &mut Player,
) -> Result<Story> {
    let (name, frames) = match case {
        "pool-boulder-one-push" => ("Tears1_Boulder1", 45),
        "pool-boulder-one-flight" => ("Tears1_Boulder1", 150),
        "pool-boulder-two-flight" => ("Tears1_Boulder2", 150),
        "pool-boulder-three-wait" => ("Tears1_Boulder3", 60),
        "pool-boulder-three-live" => ("Tears1_Boulder3", 135),
        "pool-boulder-ant-dead" => ("Tears1_Boulder3", 135),
        _ => anyhow::bail!("Unknown boulder fixture"),
    };
    let (fresh, w, p, mut story) = if name == "Tears1_Boulder3" {
        let mut i = Interactions::load(map)?;
        i.set_entry(assets, map, "potears1", None)?;
        let mut w = World::from_bsp(map)?;
        i.sync(&mut w);
        let mut story = Story::load(assets, "potears1");
        let mut entry = Player::spawn(&w, crate::interaction::spawn(map, None).0)
            .context("Blocked Pool entry")?;
        i.entry_story(&mut story);
        i.skip_cinematic(map, &mut w, &mut entry, &mut story)?;
        let feet = trigger(map, name);
        let p = Player::spawn(&w, feet).context("Blocked Boulder3 contact")?;
        i.triggers(0.01, p.feet, p.feet);
        ensure!(
            i.pool.as_ref().unwrap().state.cinema.rocks[2].is_some(),
            "Boulder3 contact failed"
        );
        (i, w, p, story)
    } else {
        setup(assets, map, name)?
    };
    *i = fresh;
    *world = w;
    *player = p;
    for _ in 0..frames {
        tick(i, map, world, player, &mut story, 1. / 60.)?;
    }
    if case == "pool-boulder-ant-dead" {
        i.hit_level(Hit {
            id: ants::IDS[0],
            damage: 100.,
            kind: DamageKind::Knife,
            knockback: Vec3::ZERO,
        });
    }
    Ok(story)
}
pub fn behavior(r: &mut crate::save::Restored) -> Result<()> {
    let before = r
        .interactions
        .pool
        .as_ref()
        .unwrap()
        .state
        .boulders
        .as_ref()
        .unwrap()
        .ants[0]
        .health;
    if r.interactions.scripted() {
        r.interactions.skip_cinematic(
            &r.scene.map,
            &mut r.scene.world,
            &mut r.game.player,
            &mut r.story,
        )?;
    }
    for _ in 0..180 {
        tick(
            &mut r.interactions,
            &r.scene.map,
            &mut r.scene.world,
            &mut r.game.player,
            &mut r.story,
            1. / 60.,
        )?;
    }
    let pool = r.interactions.pool.as_ref().unwrap();
    ensure!(
        pool.state.boulders.as_ref().unwrap().ants[0].health == before,
        "Ant resurrected after restoration"
    );
    ensure!(
        !r.interactions.scripted() && r.scene.world.body_clear(r.game.player.feet),
        "Unsafe boulder handoff"
    );
    Ok(())
}
pub fn check(assets: &mut Assets, map: &Bsp) -> Result<()> {
    for name in ["Tears1_Boulder1", "Tears1_Boulder2"] {
        let mut outcome: Option<(Vec3, Vec3, usize)> = None;
        for hz in [30, 60, 144] {
            for skip in [None, Some(0.), Some(1.5)] {
                let (mut i, mut world, mut player, mut story) = setup(assets, map, name)?;
                let index = usize::from(name == "Tears1_Boulder2");
                let mut saved = false;
                for frame in 0..hz * 20 {
                    if skip.is_some_and(|at| frame as f32 / hz as f32 >= at) {
                        i.skip_cinematic(map, &mut world, &mut player, &mut story)?;
                        break;
                    }
                    tick(
                        &mut i,
                        map,
                        &mut world,
                        &mut player,
                        &mut story,
                        1. / hz as f32,
                    )?;
                    if frame == hz {
                        let snapshot = serde_json::to_value(i.snapshot())?;
                        i.advance_school(0., map, &mut world, &mut player)?;
                        ensure!(
                            snapshot == serde_json::to_value(i.snapshot())?,
                            "Paused boulder moved"
                        );
                        i.restore(&serde_json::from_value(snapshot.clone())?, map)?;
                        ensure!(
                            snapshot == serde_json::to_value(i.snapshot())?,
                            "Boulder restore changed state"
                        );
                        saved = true;
                    }
                    if !i.scripted() {
                        break;
                    }
                }
                let p = i.pool.as_ref().unwrap();
                let b = p.state.boulders.as_ref().unwrap();
                let rock = &b.rocks[index];
                if let Some((position, velocity, node)) = outcome {
                    ensure!(
                        rock.position.distance(position) < 0.1
                            && rock.velocity.distance(velocity) < 0.1
                            && rock.node == node,
                        "Different watch/skip physical outcome for {name}: {:?} vs {position:?}",
                        rock.position
                    );
                } else {
                    outcome = Some((rock.position, rock.velocity, rock.node));
                }
                ensure!(
                    !i.scripted()
                        && p.state.cinema.done[index + 1]
                        && b.rocks[index].started
                        && b.rocks[index].solid,
                    "Incomplete boulder commitment"
                );
                ensure!(
                    world.body_clear(player.feet),
                    "Unsafe boulder landing {:?}",
                    player.feet
                );
                ensure!(
                    !b.released && i.levels_targets().is_empty(),
                    "Early pusher release"
                );
                if index == 1 {
                    ensure!(
                        player.feet.distance(p.boulder_alice().translation) < 0.01,
                        "Boulder2 ignored handoff"
                    );
                }
                ensure!(skip == Some(0.) || saved, "No boulder checkpoint");
                let consumed = serde_json::to_value(i.snapshot())?;
                i.reset_contacts();
                let at = trigger(map, name);
                i.triggers(0.01, at, at);
                ensure!(!i.scripted(), "Consumed scene replayed");
                let mut old = consumed;
                old["pool"].as_object_mut().unwrap().remove("boulders");
                i.restore(&serde_json::from_value(old)?, map)?;
                ensure!(
                    !i.scripted() && i.pool.as_ref().unwrap().state.cinema.done[index + 1],
                    "Old completion replayed"
                );
                println!("PASS boulder {name} hz={hz} skip={skip:?}: contact, pause, restore, handoff, one-shot");
            }
        }
    }
    let mut i = Interactions::load(map)?;
    let mut world = World::from_bsp(map)?;
    let mut player = Player::new(Vec3::ZERO);
    let mut story = stage(
        "pool-boulder-three-wait",
        assets,
        map,
        &mut i,
        &mut world,
        &mut player,
    )?;
    ensure!(
        !i.scripted() && i.levels_targets().is_empty(),
        "Boulder3 incorrectly cinematic or early AI"
    );
    for _ in 0..61 {
        tick(&mut i, map, &mut world, &mut player, &mut story, 1. / 60.)?;
    }
    ensure!(
        i.levels_targets().iter().map(|t| t.id).collect::<Vec<_>>() == ants::IDS,
        "Missing or duplicate ant targets"
    );
    ensure!(i.levels.is_empty() && i.has_level_combat(),
        "Pool-owned ants were omitted from the gameplay combat update");
    let mut stats = crate::inventory::Stats::default();
    let before_clocks: Vec<_> = i.pool.as_ref().unwrap().state.boulders.as_ref().unwrap()
        .ants.iter().map(|a| a.time).collect();
    for dt in [0., 1. / 60.] {
        if i.has_level_combat() {
            i.levels_step(&mut crate::level::Combat {
                dt, world: &world, player: &mut player, stats: &mut stats,
                story: &mut story, notarget: true, summon: None, threatens: &|_| false,
            });
        }
        for (ant, before) in i.pool.as_ref().unwrap().state.boulders.as_ref().unwrap()
            .ants.iter().zip(&before_clocks) {
            ensure!((ant.time - before - dt).abs() < 0.00001,
                "Gameplay did not advance the released ant (or ignored pause)");
        }
    }
    let saved = serde_json::to_value(i.snapshot())?;
    i.restore(&serde_json::from_value(saved.clone())?, map)?;
    ensure!(
        saved == serde_json::to_value(i.snapshot())?,
        "Live ant restore changed"
    );
    ensure!(
        i.hit_level(Hit {
            id: ants::IDS[0],
            damage: 25.,
            kind: DamageKind::Knife,
            knockback: Vec3::ZERO
        })
        .is_some(),
        "Pusher hit dispatch missing"
    );
    ensure!(
        i.pool
            .as_ref()
            .unwrap()
            .state
            .boulders
            .as_ref()
            .unwrap()
            .ants[0]
            .health
            == 75.,
        "Wrong pusher damage"
    );
    i.hit_level(Hit {
        id: ants::IDS[0],
        damage: 75.,
        kind: DamageKind::Knife,
        knockback: Vec3::ZERO,
    });
    let before = serde_json::to_value(i.snapshot())?;
    i.restore(&serde_json::from_value(before.clone())?, map)?;
    ensure!(
        before == serde_json::to_value(i.snapshot())? && i.levels_targets().len() == 1,
        "Dead pusher revived"
    );
    println!(
        "PASS Boulder3: real trigger, two-second release, stable identities, damage/death/save"
    );
    // Exercise the combat implementation with real model clocks in a controlled
    // arena; this complements the map contact/ownership checks above.
    let arena = World::fixture(&[(vec3(-2000., -2000., -100.), vec3(2000., 2000., 0.))]);
    for distance in [80., 300.] {
        let mut ant = ants::Ant::pusher(vec3(0., 0., 0.03125), 0., 634);
        ant.enabled = true;
        let mut victim = Player::new(vec3(distance, 0., 0.03125));
        let mut stats = crate::inventory::Stats::default();
        let data = &i.pool.as_ref().unwrap().cinema;
        let mut damage = 0.;
        for step in 0..1200 {
            let mut out = crate::combat::Feedback::default();
            ant.advance(
                &crate::level::Combat {
                    dt: 1. / 120.,
                    world: &arena,
                    player: &mut victim,
                    stats: &mut stats,
                    story: &mut story,
                    notarget: false,
                    summon: None,
                    threatens: &|_| false,
                },
                data,
                &mut out,
            );
            damage += out.damage;
            if step == 150 {
                let saved = serde_json::to_value(&ant)?;
                ant = serde_json::from_value(saved.clone())?;
                ant.validate()?;
                ensure!(
                    saved == serde_json::to_value(&ant)?,
                    "Ant combat clock changed on restore"
                );
            }
        }
        ensure!(damage >= 10., "Activated ant inert at distance {distance}");
        println!("PASS real-model pusher combat at {distance}: damage={damage}, saved attack/projectile clocks");
    }
    let spec = &i.pool.as_ref().unwrap().cinema.rocks[2];
    let mut rock = crate::falling_rock::State::new(spec, true, true);
    let mut contact_world = World::fixture(&[]);
    contact_world.set_dynamic(vec![spec.collider(&rock).unwrap()]);
    let sweep = contact_world.sweep(
        rock.position - Vec3::X * 200.,
        rock.position + Vec3::X * 200.,
        Vec3::splat(8.),
    );
    ensure!(
        !sweep.start_solid && sweep.fraction < 1.,
        "Rock hull absent from player collision"
    );
    let body = crate::combat::Target {
        id: crate::dice::ALICE,
        center: rock.position,
        half: PLAYER_HALF,
    };
    ensure!(
        spec.advance(&mut rock, 0.1, &[body]).hits.is_empty(),
        "Stationary rock damaged Alice"
    );
    spec.activate(&mut rock);
    let contacts = spec.advance(&mut rock, 0.01, &[body]);
    ensure!(
        contacts.hits.len() == 1 && contacts.hits[0].1 == 50.,
        "Pool moving contact damage missing"
    );
    println!("PASS real Pool rock dimensions, swept solid contact, damage50 while moving, harmless at rest");
    Ok(())
}
