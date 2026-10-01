use super::*;
use crate::movement::{Controls, FIXED_DT};
use std::collections::BTreeSet;

fn enter_rabbit(i: &mut Interactions, w: &World, p: &mut Player) -> Result<()> {
    // Stage only the approach; contact itself uses ordinary player movement.
    *p = Player::spawn(w, vec3(-1280., -316., 736.)).context("Rabbit approach blocked")?;
    for _ in 0..240 {
        let before = p.feet;
        p.tick(
            w,
            Controls {
                wish: Vec2::X,
                ..Default::default()
            },
        );
        i.triggers(FIXED_DT, before, p.feet);
        if i.scene_id_after(None) == Some(RABBIT) {
            return Ok(());
        }
    }
    anyhow::bail!("Normal movement missed Rabbit trigger at {:?}", p.feet)
}

fn tick(
    i: &mut Interactions,
    map: &Bsp,
    w: &mut World,
    p: &mut Player,
    story: &mut Story,
    stats: &mut Stats,
    dt: f32,
) -> Result<()> {
    i.advance_school(dt, map, w, p)?;
    i.prepare_player(stats, p);
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
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let refs: Vec<_> = [
        ("alice", ALICE_CLIPS),
        ("c_mockturtle", TURTLE_CLIPS),
        ("c_whiterabbit", RABBIT_CLIPS),
    ]
    .into_iter()
    .flat_map(|(model, names)| {
        names
            .iter()
            .map(move |&requested| crate::story::registry::AnimationRef {
                model,
                requested,
                fallback: None,
            })
    })
    .collect();
    crate::story::registry::validate_animations(a, &refs)?;
    let map = Bsp::parse(&a.read("maps/garden1.bsp")?)?;
    let hints = crate::cheshire::Hints::load(a, &map, "garden1")?;
    for hz in [30, 60, 144] {
        for skip_at in [None, Some(0.2), Some(8.), Some(22.)] {
            let mut i = Interactions::load(&map)?;
            i.set_entry(a, &map, "garden1", None)?;
            let mut w = World::from_bsp(&map)?;
            i.sync(&mut w);
            let mut p = Player::spawn(&w, crate::interaction::spawn(&map, None).0)
                .context("Garden spawn blocked")?;
            let mut stats = Stats::for_level("garden1", None);
            stats.damage(50.);
            stats.spend_will(50.);
            i.prepare_player(&mut stats, &mut p);
            ensure!(
                stats.sanity() == 100. && stats.will() == 100.,
                "Fresh garden refill missing"
            );
            stats.damage(1.);
            stats.spend_will(1.);
            let mut story = Story::load(a, "garden1");
            i.entry_story(&mut story);
            ensure!(
                i.scripted()
                    && !w
                        .traversal
                        .pushes
                        .iter()
                        .find(|p| p.id.0 == 117)
                        .unwrap()
                        .enabled,
                "Fresh entry/launch ownership missing"
            );
            for kind in [Kind::Arrival, Kind::Rabbit] {
                if kind == Kind::Rabbit {
                    // Stage the approach for this focused scene contract; the separate
                    // ordinary-input course verifies complete traversal.
                    enter_rabbit(&mut i, &w, &mut p)?;
                    ensure!(
                        i.scene_id_after(None) == Some(RABBIT),
                        "Real Rabbit trigger did not start scene at {:?}",
                        p.feet
                    );
                }
                let mut lines = BTreeSet::new();
                let mut checkpoints = BTreeSet::new();
                let mut skipped = false;
                for frame in 0..hz * 180 {
                    let Some(s) = owner(&mut i)?.saved.scene.clone() else {
                        break;
                    };
                    let o = owner(&mut i)?;
                    if s.phase == Phase::Talk {
                        ensure!(
                            w.body_clear(o.alice_pose().translation),
                            "Scene Alice embedded in {:?} at {:?}",
                            s.kind,
                            o.alice_pose().translation
                        );
                    }
                    if !skipped && skip_at.is_some_and(|t| frame as f32 / hz as f32 >= t) {
                        ensure!(
                            i.skip_cinematic(&map, &mut w, &mut p, &mut story)?,
                            "Scene refused skip"
                        );
                        skipped = true;
                    }
                    if let Some((n, t)) = story.progress(kind.dialogue()) {
                        if t > 0.1 {
                            lines.insert(n);
                        }
                    }
                    let key = format!("{:?}-{}", s.phase, s.clock.line);
                    if s.elapsed() > 0.2 && checkpoints.insert(key) {
                        let saved = i.snapshot();
                        let st = story.snapshot();
                        let pp = serde_json::to_value(&p)?;
                        let before = serde_json::to_value(&saved)?;
                        tick(&mut i, &map, &mut w, &mut p, &mut story, &mut stats, 0.)?;
                        ensure!(
                            before == serde_json::to_value(i.snapshot())?
                                && pp == serde_json::to_value(&p)?,
                            "Pause changed scene/player"
                        );
                        let camera = owner(&mut i)?.scene_camera();
                        let mut resumed = Interactions::load(&map)?;
                        resumed.set_entry(a, &map, "garden1", None)?;
                        resumed.restore(&saved, &map)?;
                        let mut rs = Story::load(a, "garden1");
                        rs.restore(&st, &hints)?;
                        let rc = owner(&mut resumed)?.scene_camera();
                        ensure!(
                            camera
                                .zip(rc)
                                .is_some_and(|(a, b)| a.eye == b.eye && a.target == b.target),
                            "Restored camera moved"
                        );
                        i = resumed;
                        story = rs;
                        i.sync(&mut w);
                    }
                    tick(
                        &mut i,
                        &map,
                        &mut w,
                        &mut p,
                        &mut story,
                        &mut stats,
                        1. / hz as f32,
                    )?;
                }
                ensure!(!i.scripted(), "Garden scene failed to finish {kind:?}");
                let o = owner(&mut i)?;
                ensure!(
                    p.feet.distance(o.data.points[kind.landing()].translation) < 8.1
                        && w.body_clear(p.feet),
                    "Unsafe/wrong garden landing {kind:?} {:?}",
                    p.feet
                );
                ensure!(
                    skip_at.is_some() || lines.len() == 5,
                    "Scene missed lines {kind:?}: {lines:?}"
                );
                ensure!(
                    story.has_seen(kind.dialogue()),
                    "Scene dialogue uncommitted"
                );
                ensure!(
                    stats.sanity() == 99.
                        && stats.will() == 99.
                        && stats.turtle_air
                        && p.breath.shell,
                    "Garden refilled resources again or lost shell"
                );
                ensure!(
                    !i.skip_cinematic(&map, &mut w, &mut p, &mut story)?,
                    "Completed scene skipped twice"
                );
                ensure!(
                    !w.traversal
                        .pushes
                        .iter()
                        .find(|p| p.id.0 == 117)
                        .unwrap()
                        .enabled,
                    "Arrival launch remained active"
                );
                let before = p.feet;
                p.tick(&w, Controls::default());
                ensure!(
                    w.body_clear(p.feet) && p.feet.distance(before) < 5.,
                    "Gameplay handoff launched/embedded Alice"
                );
                println!(
                    "PASS garden {kind:?} {hz} Hz skip={skip_at:?}, {} saved phases, landing {:?}",
                    checkpoints.len(),
                    p.feet
                );
            }
            let done = owner(&mut i)?.snapshot();
            owner(&mut i)?.event(RABBIT);
            ensure!(done == owner(&mut i)?.snapshot(), "Rabbit replayed");
            ensure!(
                !owner(&mut i)?.saved.portals,
                "Conversation unlocked recovery portal"
            );
            let portal = vec3(-1760., 700., 303.) - crate::collision::PLAYER_CENTER;
            ensure!(
                i.triggers(FIXED_DT, portal, portal).teleport.is_none(),
                "Closed recovery shortcut was usable"
            );
            let open = vec3(-672., 1284., 1204.) - crate::collision::PLAYER_CENTER;
            i.triggers(FIXED_DT, open, open);
            i.sync(&mut w);
            ensure!(
                owner(&mut i)?.saved.portals && owner(&mut i)?.colliders().len() == 13,
                "Recovery portal omitted its frames"
            );
            ensure!(
                i.triggers(FIXED_DT, portal, portal).teleport.is_some(),
                "Open recovery shortcut stayed closed"
            );
            let exit = vec3(-1560., 3768., 1360.) - crate::collision::PLAYER_CENTER;
            ensure!(
                i.triggers(FIXED_DT, exit, exit)
                    .transition
                    .is_some_and(|e| e.0 == "garden2" && e.1.as_deref() == Some("garden2_start1")),
                "Authored exit changed"
            );
        }
    }
    let mut old = Interactions::load(&map)?;
    let contact = vec3(-1230., -350., 680.);
    old.triggers(0.01, contact, contact);
    let mut i = Interactions::load(&map)?;
    i.set_entry(a, &map, "garden1", None)?;
    i.restore(&old.snapshot(), &map)?;
    let mut stats = Stats::for_level("garden1", None);
    stats.damage(63.);
    stats.spend_will(59.);
    stats.turtle_air = false;
    let mut p = Player::new(contact);
    i.prepare_player(&mut stats, &mut p);
    ensure!(
        !i.scripted() && stats.sanity() == 37. && stats.will() == 41. && !stats.turtle_air,
        "Legacy garden replayed/refilled/granted"
    );
    i.triggers(0.01, contact, contact);
    ensure!(
        i.scene_id_after(None) == Some(RABBIT),
        "Legacy pending Rabbit trigger not rearmed"
    );
    println!("PASS garden legacy continuation and pending-trigger migration");
    Ok(())
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let mut scene = crate::render::Scene::load(a, "garden1")?;
        let mut art = art::Art::load(a)?;
        for case in [
            "garden1-arrival-launch",
            "garden1-arrival-talk",
            "garden1-arrival-shell",
            "garden1-arrival-swim",
            "garden1-rabbit-talk",
            "garden1-rabbit-run",
        ] {
            let mut i = Interactions::load(&scene.map)?;
            i.set_entry(a, &scene.map, "garden1", None)?;
            let mut w = World::from_bsp(&scene.map)?;
            let mut p = Player::new(Vec3::ZERO);
            let mut stats = Stats::for_level("garden1", None);
            let story = stage(case, a, &scene.map, &mut i, &mut w, &mut p, &mut stats)?;
            let o = owner(&mut i)?;
            let camera = o
                .scene_camera()
                .with_context(|| format!("Scene already ended at render {case}"))?;
            let target = o.saved.scene.as_ref().unwrap().clock.time;
            art.story_pose(&story);
            for frame in 0..3 {
                clear_background(BLACK);
                set_camera(&Camera3D {
                    position: camera.eye,
                    target: camera.target,
                    up: camera.up,
                    fovy: 75_f32.to_radians(),
                    z_near: 2.,
                    z_far: 20000.,
                    ..Default::default()
                });
                scene.draw(camera.eye, target, false, false, &o.transforms());
                art.draw(o, &scene.atmosphere, camera.eye, false);
                crate::render::depth_read_only(|| {
                    scene.draw(camera.eye, target, false, true, &o.transforms())
                });
                art.effects(o, camera.eye, &scene.atmosphere);
                set_default_camera();
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/garden1-scenes/{case}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        for (case, eye, target) in [
            (
                "lily",
                vec3(-1950., -1200., -60.),
                vec3(-2380., -750., -340.),
            ),
            (
                "bridge",
                vec3(2544., -784., 150.),
                vec3(2170., -630., -100.),
            ),
            (
                "deadtree",
                vec3(1856., -2624., 180.),
                vec3(1740., -2460., 100.),
            ),
            (
                "current",
                vec3(-1100., 1900., 1300.),
                vec3(-440., 2100., 920.),
            ),
            ("mushrooms", vec3(450., 720., 400.), vec3(100., 500., 200.)),
        ] {
            let mut o = Garden::load(a, &scene.map)?;
            o.upgraded();
            o.event("Open_Portals");
            if case == "bridge" {
                o.event("Bridge_Drop");
            }
            if case == "deadtree" {
                o.event("Ant_Deadtree_Ambush");
            }
            let mut p = Player::new(Vec3::ZERO);
            for _ in 0..240 {
                o.advance(1. / 120., &scene.map, &mut scene.world, &mut p, &[])?;
            }
            for frame in 0..3 {
                clear_background(BLACK);
                set_camera(&Camera3D {
                    position: eye,
                    target,
                    up: Vec3::Z,
                    fovy: 75_f32.to_radians(),
                    z_near: 2.,
                    z_far: 20000.,
                    ..Default::default()
                });
                scene.draw(eye, o.saved.world.time, false, false, &o.transforms());
                art.draw(&o, &scene.atmosphere, eye, false);
                crate::render::depth_read_only(|| {
                    scene.draw(eye, o.saved.world.time, false, true, &o.transforms())
                });
                set_default_camera();
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/garden1-world/{case}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        Ok(())
    })
}

pub fn stage(
    case: &str,
    a: &mut Assets,
    map: &Bsp,
    i: &mut Interactions,
    w: &mut World,
    p: &mut Player,
    stats: &mut Stats,
) -> Result<Story> {
    if case.starts_with("garden1-world-") {
        owner(i)?.upgraded();
        stats.turtle_air = true;
        *p = Player::new(vec3(-450., 1900., 860.));
        if case == "garden1-world-bridge" {
            owner(i)?.event("Bridge_Drop");
        }
        if case == "garden1-world-deadtree" {
            owner(i)?.event("Ant_Deadtree_Ambush");
        }
        let mut story = Story::load(a, "garden1");
        for _ in 0..240 {
            tick(i, map, w, p, &mut story, stats, 1. / 120.)?;
        }
        if case == "garden1-world-lily" {
            let pad = &owner(i)?.movers.pads[0];
            let top = pad.pose.translation + Vec3::Z * 128.;
            let hit = pad.collider.trace(
                top + crate::collision::PLAYER_CENTER,
                top - Vec3::Z * 256. + crate::collision::PLAYER_CENTER,
                crate::collision::PLAYER_HALF,
            );
            *p = Player::new(top - Vec3::Z * 256. * hit.fraction);
        }
        ensure!(w.body_clear(p.feet), "Garden world save fixture blocked");
        return Ok(story);
    }
    let rabbit = case.starts_with("garden1-rabbit-");
    if rabbit {
        owner(i)?.upgraded();
    }
    i.sync(w);
    *p = Player::spawn(
        w,
        if rabbit {
            vec3(-1230., -350., 736.)
        } else {
            crate::interaction::spawn(map, None).0
        },
    )
    .context("Garden saved fixture blocked")?;
    if rabbit {
        enter_rabbit(i, w, p)?;
        ensure!(i.scripted(), "Rabbit save fixture missed trigger");
    }
    let mut story = Story::load(a, "garden1");
    for _ in 0..120 * 180 {
        let o = owner(i)?;
        let stop = match (case, o.saved.scene.as_ref()) {
            ("garden1-arrival-done" | "garden1-rabbit-done", None) => true,
            ("garden1-arrival-launch", Some(s)) => s.phase == Phase::Launch && s.elapsed() >= 2.7,
            ("garden1-arrival-talk" | "garden1-rabbit-talk", Some(s)) => {
                s.phase == Phase::Talk && s.clock.line == 1 && s.clock.line_time >= 0.7
            }
            ("garden1-arrival-shell", Some(s)) => {
                s.shell_at.is_some() && s.clock.line == 3 && s.clock.line_time >= 0.7
            }
            ("garden1-arrival-swim", Some(s)) => s.phase == Phase::Leave && s.elapsed() >= 2.1,
            ("garden1-rabbit-run", Some(s)) => s.phase == Phase::Leave && s.elapsed() >= 0.2,
            _ => false,
        };
        if stop {
            return Ok(story);
        }
        tick(i, map, w, p, &mut story, stats, FIXED_DT)?;
    }
    anyhow::bail!("Garden fixture did not reach {case}")
}
