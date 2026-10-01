use super::*;
use crate::inventory::Stats;
use crate::movement::FIXED_DT;
fn tick(
    i: &mut Interactions,
    map: &Bsp,
    w: &mut World,
    p: &mut Player,
    story: &mut Story,
    dt: f32,
) -> Result<()> {
    i.advance_school(dt, map, w, p)?;
    if i.prepare_story(story) {
        story.tick(dt, false);
    }
    i.sync_cinematic_story(story);
    for n in story.take_completed() {
        i.completed_dialogue(&n);
    }
    i.sync(w);
    Ok(())
}
fn approach(kind: Kind) -> Vec3 {
    match kind {
        Kind::North => vec3(2536., -1552., -1300.),
        Kind::South => vec3(1850., -2660., -750.),
        Kind::Second => vec3(1136., -752., -1450.),
        Kind::Cat => vec3(-215., -2863., -968.),
        _ => Vec3::ZERO,
    }
}
fn enter(i: &mut Interactions, map: &Bsp, w: &mut World, p: &mut Player, kind: Kind) -> Result<()> {
    if kind == Kind::Arrival {
        *p = Player::spawn(w, crate::interaction::spawn(map, None).0)
            .context("Garden spawn blocked")?;
        return Ok(());
    }
    let o = owner(i)?;
    o.upgraded();
    o.saved.legacy = false;
    o.rebuild(map)?;
    i.sync(w);
    // Staged approach, followed by contact with the real authored trigger volume.
    let id = match kind {
        Kind::North => 130,
        Kind::South => 129,
        Kind::Second => 114,
        Kind::Cat => 53,
        _ => unreachable!(),
    };
    let pos = data::at(&map.entities[id]).translation;
    let start = approach(kind);

    *p = Player::new(start);
    let feet = w
        .actor_footing(
            start,
            crate::collision::PLAYER_CENTER,
            crate::collision::PLAYER_HALF,
            200.,
        )
        .context("Garden approach has no support")?;
    p.feet = feet;
    p.grounded = true;
    let direction = (pos - feet).truncate().normalize_or_zero();
    for _ in 0..240 {
        let before = p.feet;
        p.tick(
            w,
            crate::movement::Controls {
                wish: direction,
                ..Default::default()
            },
        );
        i.triggers(FIXED_DT, before, p.feet);
        if i.scripted() {
            return Ok(());
        }
    }
    anyhow::bail!(
        "Missed real {kind:?} trigger at {:?}; target {:?}",
        p.feet,
        pos
    )
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let mut map = Bsp::parse(&a.read("maps/garden2.bsp")?)?;
    for difficulty in crate::powerups::Difficulty::ALL {
        map.difficulty = difficulty;
        let mut i = Interactions::load(&map)?;
        i.set_entry(a, &map, "garden2", None)?;
        let o = owner(&mut i)?;
        ensure!(
            o.objects.iter().filter(|o| o.group == 1).count() == 6
                && o.objects.iter().filter(|o| o.group == 2).count() == 12,
            "Missing bridge pieces"
        );
        let count = if matches!(
            difficulty,
            crate::powerups::Difficulty::Hard | crate::powerups::Difficulty::Nightmare
        ) {
            22
        } else {
            26
        };
        ensure!(
            o.objects.len() == count,
            "Difficulty geometry count {}",
            o.objects.len()
        );
        o.activate_named(&map, "t131");
        ensure!(
            o.saved.ants.iter().filter(|a| a.enabled).count()
                == match difficulty {
                    crate::powerups::Difficulty::Easy => 0,
                    crate::powerups::Difficulty::Normal => 1,
                    _ => 2,
                },
            "Wrong bridge Ant difficulty set"
        );
    }
    map.difficulty = crate::powerups::Difficulty::Normal;
    let hints = crate::cheshire::Hints::load(a, &map, "garden2")?;
    for hz in [30, 60, 144] {
        for kind in [
            Kind::Arrival,
            Kind::North,
            Kind::South,
            Kind::Second,
            Kind::Cat,
        ] {
            for skip in [None, Some(0.2), Some(8.)] {
                let mut i = Interactions::load(&map)?;
                i.set_entry(a, &map, "garden2", None)?;
                let mut w = World::from_bsp(&map)?;
                i.sync(&mut w);
                let mut p = Player::new(Vec3::ZERO);
                enter(&mut i, &map, &mut w, &mut p, kind)?;
                let home = p.feet;
                let mut story = Story::load(a, "garden2");
                let mut skipped = false;
                let mut restored = false;
                let mut frames = 0;
                for frame in 0..hz * 180 {
                    if !i.scripted() {
                        break;
                    }
                    frames = frame;
                    if !skipped && skip.is_some_and(|t| frame as f32 / hz as f32 >= t) {
                        i.skip_cinematic(&map, &mut w, &mut p, &mut story)?;
                        skipped = true;
                    }
                    if frame == hz * 2 && !restored {
                        let saved = i.snapshot();
                        let st = story.snapshot();
                        let before = serde_json::to_value(&saved)?;
                        let pp = serde_json::to_value(&p)?;
                        tick(&mut i, &map, &mut w, &mut p, &mut story, 0.)?;
                        ensure!(
                            before == serde_json::to_value(i.snapshot())?
                                && pp == serde_json::to_value(&p)?,
                            "Paused scene changed"
                        );
                        let camera = owner(&mut i)?.scene_camera();
                        let transforms = i.transforms();
                        let mut resumed = Interactions::load(&map)?;
                        resumed.set_entry(a, &map, "garden2", None)?;
                        resumed.restore(&saved, &map)?;
                        let mut rs = Story::load(a, "garden2");
                        rs.restore(&st, &hints)?;
                        ensure!(
                            camera
                                .zip(owner(&mut resumed)?.scene_camera())
                                .is_some_and(|(a, b)| a.eye == b.eye && a.target == b.target)
                                && transforms == resumed.transforms(),
                            "Restored scene/world diverged"
                        );
                        i = resumed;
                        story = rs;
                        i.sync(&mut w);
                        restored = true;
                    }
                    tick(&mut i, &map, &mut w, &mut p, &mut story, 1. / hz as f32)?;
                }
                ensure!(!i.scripted(), "Scene failed to end {kind:?}");
                ensure!(
                    w.body_clear(p.feet) && p.grounded && p.velocity == Vec3::ZERO,
                    "Unsafe handoff {kind:?} {:?}",
                    p.feet
                );
                let o = owner(&mut i)?;
                match kind {
                    Kind::North | Kind::South => ensure!(
                        p.feet == home
                            && o.saved.bridge1
                            && o.objects
                                .iter()
                                .filter(|o| o.group == 1)
                                .all(|o| o.pose.is_none()),
                        "First bridge changed home or retained deck"
                    ),
                    Kind::Second => ensure!(
                        p.feet
                            .distance(o.data.points["fakeplayer_bridge_pos2"].translation)
                            < 96.1
                            && o.saved.bridge2
                            && o.saved.ants.iter().any(|a| a.enabled),
                        "Second bridge endpoint/cue missing"
                    ),
                    Kind::Arrival => ensure!(
                        p.feet
                            .distance(o.data.points["alice_pos_squish2"].translation)
                            < 96.1
                            && o.saved.arrived
                            && o.saved.ladies.iter().any(|l| l.enabled),
                        "Arrival endpoint/cue missing"
                    ),
                    Kind::Cat => ensure!(p.feet == home && o.saved.cat, "Final Cat changed home"),
                }
                println!(
                    "PASS garden2 {kind:?} {hz}Hz skip={skip:?} frames={frames} feet={:?}",
                    p.feet
                );
            }
        }
    }
    for ids in [&[129usize][..], &[130][..], &[114][..], &[52][..]] {
        let mut old = Interactions::load(&map)?;
        for &id in ids {
            let contact = data::at(&map.entities[id]).translation - crate::collision::PLAYER_CENTER;
            old.triggers(FIXED_DT, contact, contact);
        }
        let mut i = Interactions::load(&map)?;
        i.set_entry(a, &map, "garden2", None)?;
        i.restore(&old.snapshot(), &map)?;
        let o = owner(&mut i)?;
        ensure!(o.saved.arrived && !o.scripted(), "Legacy arrival replayed");
        if ids[0] == 114 {
            ensure!(o.saved.bridge2, "Pending second bridge not migrated");
        } else if ids[0] == 52 {
            ensure!(o.saved.cat, "Pending final Cat not migrated");
        } else {
            ensure!(o.saved.bridge1, "Pending first bridge not migrated");
        }
        let mut w = World::from_bsp(&map)?;
        i.sync(&mut w);
        for &id in ids {
            let contact = data::at(&map.entities[id]).translation - crate::collision::PLAYER_CENTER;
            i.triggers(FIXED_DT, contact, contact);
        }
        ensure!(!i.scripted(), "Consumed legacy scene replayed");
    }
    let mut g = Garden::load(a, &map)?;
    g.upgraded();
    let p = Player::new(vec3(900., -1200., -1600.));
    g.restore_position(&p, &map)?;
    ensure!(
        g.saved.bridge1 && g.saved.bridge2 && !g.saved.legacy,
        "Legacy route position ignored"
    );
    println!("PASS legacy trigger/position migration and no replay");
    encounters(a, &map)?;
    println!("PASS Herbaceous scene/geometry contract; approaches are staged, not a full-map route proof");
    Ok(())
}
fn encounters(a: &mut Assets, map: &Bsp) -> Result<()> {
    let mut g = Garden::load(a, map)?;
    g.upgraded();
    let mut w = World::from_bsp(map)?;
    w.set_dynamic(g.colliders());
    for group in 1..=4 {
        g.spawn_ladies(group);
    }
    for l in &g.saved.ladies {
        let mut b = l.actor.clone();
        b.patrol_started = true;
        b.notarget = true;
        let home = b.feet;
        for _ in 0..120 * 30 {
            b.advance(1. / 120., &w, Vec3::ZERO, g.data.lady);
        }
        ensure!(
            b.feet.distance(home) > 24.,
            "Ladybug {} never patrols",
            l.id
        );
        b.validate_save(&l.actor)?;
    }
    let before = g.saved.ladies.len();
    for group in 1..=4 {
        g.spawn_ladies(group);
    }
    ensure!(
        before == g.saved.ladies.len() && before == 7,
        "Duplicate Ladybug spawns"
    );
    g.event("Ant_Ambush2").context("Missing Corporal ambush")?;
    let mut p = Player::new(vec3(1502., 1048., -1076.));
    let mut stats = Stats::for_level("garden2", None);
    let mut story = Story::load(a, "garden2");
    let start = g.saved.ants.iter().find(|a| a.id == 139).unwrap().feet;
    for tick in 0..120 * 9 {
        g.encounter_combat(&mut crate::level::Combat {
            dt: 1. / 120.,
            world: &w,
            player: &mut p,
            stats: &mut stats,
            story: &mut story,
            notarget: true,
            summon: None,
            threatens: &|_| false,
        });
        if tick == 60 {
            let snap = g.snapshot();
            let mut restored = Garden::load(a, map)?;
            restored.restore(&snap, map)?;
            ensure!(restored.snapshot() == snap, "Ambush restore changed state");
            g = restored;
        }
    }
    let ant = g.saved.ants.iter().find(|a| a.id == 139).unwrap();
    ensure!(
        ant.enabled && !ant.script_wait && ant.feet.distance(start) > 80.,
        "Corporal failed to run into its ambush: {:?}",
        ant.feet
    );
    let snap = g.snapshot();
    g.event("Ant_Ambush2");
    ensure!(g.snapshot() == snap, "Corporal ambush replayed");
    println!("PASS all seven Ladybug patrols, one-shot spawns and saved Corporal ambush");
    let mut g = Garden::load(a, map)?;
    g.upgraded();
    g.saved.legacy = false;
    w.set_dynamic(g.colliders());
    let high = vec3(1120., -792., -1390.);
    let low = high - Vec3::Z * 200.;
    let hit = w.body_trace(high, low);
    ensure!(!hit.start_solid && hit.fraction < 1., "No fulcrum support");
    let mut rider = Player::new(high.lerp(low, hit.fraction));
    let start = rider.feet;
    for _ in 0..240 {
        g.advance(1. / 120., map, &mut w, &mut rider, &[])?;
    }
    ensure!(
        g.saved.fulcrum.length() > 0.1 && rider.feet.z < start.z - 0.1 && w.body_clear(rider.feet),
        "Fulcrum failed to carry its rider"
    );
    let snapshot = g.snapshot();
    let mut restored = Garden::load(a, map)?;
    restored.restore(&snapshot, map)?;
    ensure!(
        g.transforms() == restored.transforms(),
        "Fulcrum restore changed pose"
    );
    println!("PASS bounded fulcrum tilt, safe rider and saved pose");
    Ok(())
}
pub fn stage(
    case: &str,
    a: &mut Assets,
    map: &Bsp,
    i: &mut Interactions,
    w: &mut World,
    p: &mut Player,
    _: &mut Stats,
) -> Result<Story> {
    let kind = if case.contains("bridge1") {
        Kind::South
    } else if case.contains("bridge2") {
        Kind::Second
    } else if case.contains("cat-") {
        Kind::Cat
    } else {
        Kind::Arrival
    };
    i.sync(w);
    enter(i, map, w, p, kind)?;
    let mut story = Story::load(a, "garden2");
    for _ in 0..120 * 180 {
        let o = owner(i)?;
        let stop = o.saved.scene.as_ref().is_some_and(|s| match case {
            "garden2-arrival-rabbit" => s.phase == Phase::Talk && s.elapsed() > 0.7,
            "garden2-arrival-hatter" => s.phase == Phase::Squish && s.elapsed() > 12.,
            "garden2-arrival-kneel" => s.phase == Phase::Kneel && s.elapsed() > 7.,
            "garden2-bridge1-fall" | "garden2-bridge2-fall" => {
                s.clock.time > o.data.collapse() + 1.3
            }
            "garden2-cat-talk" => s.phase == Phase::Talk && s.elapsed() > 0.7,
            _ => false,
        });
        if stop {
            return Ok(story);
        }
        tick(i, map, w, p, &mut story, FIXED_DT)?;
    }
    anyhow::bail!("Garden fixture failed {case}")
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let mut scene = crate::render::Scene::load(a, "garden2")?;
        let mut art = art::Art::load(a)?;
        for case in REGISTRATION.save_cases {
            let mut i = Interactions::load(&scene.map)?;
            i.set_entry(a, &scene.map, "garden2", None)?;
            let mut w = World::from_bsp(&scene.map)?;
            let mut p = Player::new(Vec3::ZERO);
            let mut stats = Stats::for_level("garden2", None);
            let story = stage(case.name, a, &scene.map, &mut i, &mut w, &mut p, &mut stats)?;
            let o = owner(&mut i)?;
            let c = o.scene_camera().context("Missing render camera")?;
            art.story_pose(&story);
            for frame in 0..3 {
                clear_background(BLACK);
                set_camera(&Camera3D {
                    position: c.eye,
                    target: c.target,
                    up: c.up,
                    fovy: 75_f32.to_radians(),
                    z_near: 2.,
                    z_far: 20000.,
                    ..Default::default()
                });
                scene.draw(c.eye, 0., false, false, &o.transforms());
                art.draw(o, &scene.atmosphere, c.eye, false);
                crate::render::depth_read_only(|| {
                    scene.draw(c.eye, 0., false, true, &o.transforms())
                });
                art.effects(o, c.eye, &scene.atmosphere);
                set_default_camera();
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/garden2-scenes/{}.png",
                        case.name
                    )))?;
                }
                next_frame().await;
            }
        }
        Ok(())
    })
}
