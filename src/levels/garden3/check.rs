use super::*;
use crate::{
    inventory::Stats,
    movement::{Controls, FIXED_DT},
};
fn setup(a: &mut Assets, map: &Bsp) -> Result<(Interactions, World, Player)> {
    let mut i = Interactions::load(map)?;
    i.set_entry(a, map, "garden3", Some("garden3_start1"))?;
    let mut w = World::from_bsp(map)?;
    i.sync(&mut w);
    let p = Player::spawn(&w, crate::interaction::spawn(map, Some("garden3_start1")).0)
        .context("Chase spawn blocked")?;
    Ok((i, w, p))
}
fn tick(i: &mut Interactions, map: &Bsp, w: &mut World, p: &mut Player, dt: f32) -> Result<()> {
    i.advance_school(dt, map, w, p)?;
    i.sync(w);
    Ok(())
}
fn identical(a: &Garden, b: &Garden) -> Result<()> {
    ensure!(a.snapshot() == b.snapshot(), "Saved chase future differs");
    ensure!(a.transforms() == b.transforms(), "Saved geometry differs");
    let cameras = |g: &Garden| g.scene_camera().map(|c| (c.eye, c.target, c.up));
    ensure!(cameras(a) == cameras(b), "Saved camera differs");
    ensure!(a.quake_offset() == b.quake_offset(), "Saved quake differs");
    Ok(())
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/garden3.bsp")?)?;
    mushroom(a, &map)?;
    let mut expected = None;
    for hz in [30, 60, 144] {
        for skip in [None, Some(0.2), Some(18.49), Some(19.51), Some(24.8)] {
            let (mut i, mut w, mut p) = setup(a, &map)?;
            let mut story = Story::load(a, "garden3");
            let mut did_skip = false;
            for _ in 0..hz * 30 {
                if !i.scripted() {
                    break;
                }
                if !did_skip && skip.is_some_and(|s| owner(&mut i).unwrap().saved.age >= s) {
                    ensure!(
                        i.skip_cinematic(&map, &mut w, &mut p, &mut story)?,
                        "Skip not accepted"
                    );
                    did_skip = true;
                }
                tick(&mut i, &map, &mut w, &mut p, 1. / hz as f32)?;
            }
            let g = owner(&mut i)?;
            ensure!(
                g.saved.arrived
                    && !g.scripted()
                    && g.saved.rock.solid
                    && g.saved.rock.started
                    && (g.saved.rock.elapsed - 5.5).abs() < 0.001,
                "Bad chase commitment"
            );
            ensure!(
                g.saved.first_quake.is_some()
                    && g.saved.pillar.is_none()
                    && g.saved.ice.is_none()
                    && g.saved.end.is_none(),
                "Premature world callbacks"
            );
            ensure!(
                p.grounded && w.body_clear(p.feet),
                "Unsafe chase landing {:?}",
                p.feet
            );
            let state = (g.snapshot(), p.feet, p.script_facing);
            if let Some(ref before) = expected {
                ensure!(before == &state, "Watch/skip end mismatch {hz} {skip:?}");
            } else {
                println!(
                    "Handoff feet {:?}, marble {:?}, lead {:.1}",
                    p.feet,
                    g.saved.rock.position,
                    p.feet.distance(g.saved.rock.position)
                );
                expected = Some(state);
            }
            let before = g.snapshot();
            ensure!(
                !g.skip(&map, &mut w, &mut p, &mut story)?,
                "Repeated skip accepted"
            );
            ensure!(g.snapshot() == before, "Repeated skip changed chase");
        }
    }
    println!("PASS 15 watch/skip cases at 30/60/144 Hz; identical real hazard and safe handoff");
    for time in [
        0.2, 4.5, 10., 18.49, 19.49, 19.5, 19.51, 23.6, 24.49, 24.51, 25.,
    ] {
        let (mut i, mut w, mut p) = setup(a, &map)?;
        while owner(&mut i)?.saved.age + 0.0001 < time {
            let dt = (time - owner(&mut i)?.saved.age).min(FIXED_DT);
            tick(&mut i, &map, &mut w, &mut p, dt)?;
        }
        let saved = i.snapshot();
        let before = serde_json::to_value(&saved)?;
        let player = serde_json::to_value(&p)?;
        for _ in 0..30 {
            tick(&mut i, &map, &mut w, &mut p, 0.)?;
        }
        ensure!(
            before == serde_json::to_value(i.snapshot())? && player == serde_json::to_value(&p)?,
            "Pause moved chase"
        );
        let (mut r, mut rw, _) = setup(a, &map)?;
        r.restore(&saved, &map)?;
        let mut rp = p.clone();
        r.sync(&mut rw);
        identical(owner(&mut i)?, owner(&mut r)?)?;
        for k in 0..240 {
            tick(&mut i, &map, &mut w, &mut p, FIXED_DT)?;
            tick(&mut r, &map, &mut rw, &mut rp, FIXED_DT)?;
            if k % 30 == 0 {
                identical(owner(&mut i)?, owner(&mut r)?)?;
                ensure!(
                    serde_json::to_value(&p)? == serde_json::to_value(&rp)?,
                    "Restored player future differs"
                );
            }
        }
    }
    println!("PASS 11 pause/save futures including both sides of release and gravity change");
    let (mut i, mut w, mut p) = setup(a, &map)?;
    let mut story = Story::load(a, "garden3");
    i.skip_cinematic(&map, &mut w, &mut p, &mut story)?;
    tick(&mut i, &map, &mut w, &mut p, FIXED_DT)?;
    let home = p.feet;
    // Ordinary collision/movement after handoff, with the live marble chasing.
    for _ in 0..120 {
        let before = p.feet;
        p.tick(
            &w,
            Controls {
                wish: Vec2::Y,
                ..Default::default()
            },
        );
        i.triggers(FIXED_DT, before, p.feet);
        tick(&mut i, &map, &mut w, &mut p, FIXED_DT)?;
    }
    ensure!(
        p.feet.distance(home) > 150. && owner(&mut i)?.saved.damage == 0.,
        "Chase has no playable lead {:?}",
        p.feet
    );
    println!(
        "PASS live movement after handoff: {:.1} units with no marble contact",
        p.feet.distance(home)
    );
    // Contact uses the W9 swept damage path, then the same combat feedback as the viewer.
    let g = owner(&mut i)?;
    p.feet = g.saved.rock.position - crate::collision::PLAYER_CENTER;
    g.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
    ensure!(g.saved.damage == 999., "Marble is not lethal");
    let mut stats = Stats::for_level("garden3", None);
    let mut ctx = crate::level::Combat {
        dt: FIXED_DT,
        world: &w,
        player: &mut p,
        stats: &mut stats,
        story: &mut story,
        notarget: false,
        summon: None,
        threatens: &|_| false,
    };
    ensure!(
        g.combat(&mut ctx).damage == 999. && g.combat(&mut ctx).damage == 0.,
        "Contact delivered twice"
    );
    println!("PASS real lethal contact and one-time damage delivery");
    // Real target dispatch and START_OPEN parity. The staged trigger contacts do not
    // constitute a full route run; they exercise the authored receiver graph.
    let (mut i, mut w, mut p) = setup(a, &map)?;
    i.skip_cinematic(&map, &mut w, &mut p, &mut story)?;
    tick(&mut i, &map, &mut w, &mut p, FIXED_DT)?;
    ensure!(
        owner(&mut i)?
            .saved
            .doors
            .iter()
            .map(|d| d.open)
            .eq([true, false, true, false]),
        "START_OPEN lost"
    );
    for (id, expected) in [
        (41, [false, false, true, false]),
        (48, [false, true, false, true]),
        (47, [false, false, true, false]),
        (73, [false, true, false, true]),
    ] {
        let at = data::at(&map.entities[id]).translation - crate::collision::PLAYER_CENTER;
        i.triggers(FIXED_DT, at, at);
        ensure!(
            owner(&mut i)?
                .saved
                .doors
                .iter()
                .map(|d| d.open)
                .eq(expected),
            "Gate dispatch {id}"
        );
    }
    let g = owner(&mut i)?;
    p.feet = vec3(20000., 20000., 0.);
    let mut quakes = [false; 3];
    for _ in 0..120 * 120 {
        g.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
        for (seen, at) in quakes
            .iter_mut()
            .zip([g.saved.pillar, g.saved.second_quake, g.saved.ice])
        {
            if !*seen && at.is_some_and(|at| g.saved.age - at < 0.1) {
                ensure!(
                    g.quake_offset().length() > 0.01 && g.quake_offset().length() < 3.,
                    "Missing or excessive chase quake"
                );
                let mut resumed = Garden::load(a, &map)?;
                resumed.restore(&g.snapshot(), &map)?;
                identical(g, &resumed)?;
                *seen = true;
            }
        }
    }
    ensure!(
        quakes.into_iter().all(|v| v) && g.quake_offset() == Vec3::ZERO,
        "Quake event missing or never settled"
    );
    ensure!(
        g.settled_supports().len() < g.objects.len() - 4,
        "Collapsed floors remain eligible for hanging"
    );
    ensure!(
        g.saved.rock.node == 56
            && g.saved.pillar.is_some()
            && g.saved.second_quake.is_some()
            && g.saved.ice.is_some(),
        "Real rock did not complete path"
    );
    ensure!(
        g.saved.doors.iter().all(|d| d.latched && d.open),
        "Forward gates stranded player"
    );
    ensure!(
        g.objects
            .iter()
            .filter(|o| o.name == "falling_pillar1"
                || o.name.starts_with("icefloor2nd")
                || o.name.starts_with("marble_ice"))
            .all(|o| o.pose.is_none()),
        "Broken geometry remains collidable"
    );
    println!("PASS 56-node real chase, target-driven gates, one-shot callbacks and physical floor collapse");
    let (mut i, mut w, mut p) = setup(a, &map)?;
    let g = owner(&mut i)?;
    g.upgraded();
    g.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
    ensure!(
        g.saved.arrived && g.saved.scene.is_none() && g.saved.rock.solid,
        "Legacy chase was not rearmed"
    );
    let good = g.snapshot();
    let mut bad = good.clone();
    bad["gravity"] = serde_json::json!(0.);
    ensure!(
        g.restore(&bad, &map).is_err() && g.snapshot() == good,
        "Invalid save mutated live owner"
    );
    println!("PASS legacy continuation, malformed save rejection; full-map traversal remains a separate route check");
    for ids in [&[][..], &[5usize][..], &[16][..], &[41, 48, 47, 73][..]] {
        let mut old = Interactions::load(&map)?;
        for &id in ids {
            let at = data::at(&map.entities[id]).translation - crate::collision::PLAYER_CENTER;
            old.triggers(FIXED_DT, at, at);
        }
        let (mut i, mut w, mut p) = setup(a, &map)?;
        i.restore(&old.snapshot(), &map)?;
        tick(&mut i, &map, &mut w, &mut p, FIXED_DT)?;
        let g = owner(&mut i)?;
        ensure!(
            g.saved.arrived
                && g.saved.rock.solid
                && g.saved.intro_ticks == 660
                && g.saved.pillar.is_none()
                && g.saved.ice.is_none()
                && g.saved.end.is_none(),
            "Legacy runtime did not restart chase safely"
        );
        let at = data::at(&map.entities[16]).translation - crate::collision::PLAYER_CENTER;
        i.triggers(FIXED_DT, at, at);
        ensure!(
            owner(&mut i)?.saved.end.is_some(),
            "Legacy pending exit platform was not rearmed"
        );
    }
    println!("PASS actual pre-controller runtime migration, including consumed pending floor/exit triggers");
    Ok(())
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
    if let Some(name) = case.strip_prefix("garden3-live-") {
        let cp = route::fixture(a, name)?;
        i.restore(&cp.level.interactions, map)?;
        i.sync(w);
        *p = cp.player;
        *stats = cp.stats;
        p.validate_world(w)?;
        let mut story = Story::load(a, "garden3");
        let mut hints = crate::cheshire::Hints::load(a, map, "garden3")?;
        hints.restore(&cp.level.hints)?;
        story.restore(&cp.level.story, &hints)?;
        return Ok(story);
    }
    let target = match case {
        "garden3-fleet" => 3.,
        "garden3-carrier" => 12.,
        "garden3-before-release" => 19.4,
        "garden3-marble" => 21.,
        "garden3-running" => 24.3,
        "garden3-chase" => 25.,
        _ => anyhow::bail!("Unknown chase fixture"),
    };
    i.sync(w);
    *p = Player::spawn(w, crate::interaction::spawn(map, Some("garden3_start1")).0)
        .context("Fixture spawn blocked")?;
    for _ in 0..120 * 30 {
        if owner(i)?.saved.age >= target {
            return Ok(Story::load(a, "garden3"));
        }
        tick(i, map, w, p, FIXED_DT)?;
    }
    anyhow::bail!("Chase fixture timeout")
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let mut scene = crate::render::Scene::load(a, "garden3")?;
        let mut art = art::Art::load(a)?;
        mushroom_render(a, &mut scene, &mut art).await?;
        for case in REGISTRATION.save_cases {
            let (mut i, mut w, mut p) = setup(a, &scene.map)?;
            let mut stats = Stats::for_level("garden3", None);
            stage(case.name, a, &scene.map, &mut i, &mut w, &mut p, &mut stats)?;
            let g = owner(&mut i)?;
            let c = match case.name {
                "garden3-live-gates" => Some(crate::cinematic::Camera::look(
                    vec3(-1300., 2200., -300.),
                    vec3(-1856., 1744., -100.),
                )),
                "garden3-live-airborne" => Some(crate::cinematic::Camera::look(
                    p.feet + vec3(-120., -240., 180.),
                    vec3(1636., 2532., -1760.),
                )),
                "garden3-live-pillar" => Some(crate::cinematic::Camera::look(
                    vec3(2100., 2850., -1400.),
                    vec3(1636., 2532., -1900.),
                )),
                "garden3-live-ice" => Some(crate::cinematic::Camera::look(
                    p.eye() + Vec3::Z * 40.,
                    vec3(3200., 250., -3230.),
                )),
                "garden3-live-ending" => Some(crate::cinematic::Camera::look(
                    vec3(750., 1200., -2900.),
                    vec3(580., 1312., -3300.),
                )),
                _ => g.scene_camera(),
            }
            .unwrap_or_else(|| {
                crate::cinematic::Camera::look(
                    p.feet + vec3(160., -320., 200.),
                    g.saved.rock.position,
                )
            });
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
                scene.draw(c.eye, 0., false, false, &g.transforms());
                art.draw(g, &scene.atmosphere, c.eye, false);
                crate::render::depth_read_only(|| {
                    scene.draw(c.eye, 0., false, true, &g.transforms())
                });
                set_default_camera();
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/garden3-scenes/{}.png",
                        case.name
                    )))?;
                }
                next_frame().await;
            }
        }
        Ok(())
    })
}

// Drop the real player onto the visible cap, including its overhang beyond the
// pillar. Also compare against the old world (pillar only) to prove the regression.
fn mushroom(a: &mut Assets, map: &Bsp) -> Result<Vec3> {
    let mut g = Garden::load(a, map)?;
    let saved = g.snapshot();
    let support_count = g.settled_supports().len();
    let (pose, cap) = g.shroom.as_ref().context("Missing mushroom collider")?.clone();
    let (lo, hi) = g.data.shroom_triangles.iter().flatten().copied()
        .fold((Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(lo, hi), p| (lo.min(p), hi.max(p)));
    let center = (lo + hi) * 0.5;
    let mut world = World::from_bsp(map)?;
    world.set_dynamic(g.colliders());
    let mut old = World::from_bsp(map)?;
    old.set_dynamic(g.objects.iter().filter(|o| o.pose.is_some()).map(|o| o.collider.clone()).collect());
    let mut restored = None;
    for offset in [Vec2::ZERO, Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
        let xy = center.truncate() + offset * (hi - lo).truncate() * 0.28;
        let top = pose.point(vec3(xy.x, xy.y, hi.z + 80.));
        let bottom = pose.point(vec3(xy.x, xy.y, lo.z - 80.));
        let hit = cap.trace(top, bottom, Vec3::ZERO);
        ensure!(!hit.start_solid && hit.fraction < 1. && hit.normal.z > 0.65,
            "Cap probe is not a walkable visible surface: {offset:?}");
        let surface = top.lerp(bottom, hit.fraction);
        let contact = world.body_trace(top, bottom);
        let absent = old.body_trace(top, bottom);
        let mut p = Player::new(top);
        for _ in 0..240 { p.tick(&world, Controls::default()); }
        ensure!(p.grounded && world.body_clear(p.feet) && p.feet.z >= surface.z - 0.25
            && p.feet.z < pose.translation.z + hi.z + 1.,
            "Player fell through or floated above mushroom: {:?}, surface {surface:?}", p.feet);
        let standing = p.feet;
        for _ in 0..120 { p.tick(&world, Controls::default()); }
        ensure!(p.grounded && p.feet.distance(standing) < 0.1, "Standing player slid off cap");
        if absent.fraction > contact.fraction + 0.01 { restored = Some(standing); }
    }
    let standing = restored.context("Fixture did not expose missing cap collision")?;
    let mut supports = World::fixture(&[]);
    supports.set_settled_supports(g.settled_supports());
    ensure!(supports.ledge_trace(standing + Vec3::Z * 10., standing - Vec3::Z * 10., Vec3::ZERO).fraction < 1.,
        "Settled cap is absent from ledge support");
    // A round mesh must not become a full invisible box at its corners.
    let corner = vec3(hi.x - 2., hi.y - 2., hi.z + 10.);
    ensure!(cap.trace(pose.point(corner), pose.point(corner - Vec3::Z * (hi.z-lo.z+20.)), Vec3::ZERO).fraction == 1.,
        "Cap grew an invisible bounding box");
    g.saved.pillar = Some(0.);
    g.saved.age = 1.;
    g.rebuild(map)?;
    let (moved, shape) = g.shroom.as_ref().context("Cap vanished before pillar")?;
    ensure!(moved.translation.distance(pose.translation) > 1., "Cap did not follow pillar");
    let local_top = vec3(center.x, center.y, hi.z + 80.);
    let local_bottom = vec3(center.x, center.y, lo.z - 80.);
    ensure!(shape.trace(moved.point(local_top), moved.point(local_bottom), Vec3::ZERO).fraction < 1.,
        "Moved cap lost collision");
    ensure!(g.settled_supports().len() + 2 == support_count, "Moving pillar/cap still supply settled ledges");
    g.saved.age = 4.3;
    g.rebuild(map)?;
    ensure!(g.shroom.is_none(), "Removed pillar left invisible cap collision");
    g.restore(&saved, map)?;
    let (again, shape) = g.shroom.as_ref().context("Save restore lost cap collision")?;
    ensure!(again.translation == pose.translation && again.rotation == pose.rotation
        && shape.trace(pose.point(local_top), pose.point(local_bottom), Vec3::ZERO).fraction < 1.,
        "Restored cap disagrees with saved geometry");
    println!("PASS mushroom: five player landings, stable footing, mesh edge, ledge support, collapse and restore; feet {standing:?}");
    Ok(standing)
}
async fn mushroom_render(a: &mut Assets, scene: &mut crate::render::Scene, art: &mut art::Art) -> Result<()> {
    let feet = mushroom(a, &scene.map)?;
    let g = Garden::load(a, &scene.map)?;
    let player = Player::new(feet);
    let mut alice = crate::character::Character::load(a)?;
    alice.reset(&player, 0.);
    let camera = Camera3D { position: feet + vec3(170., -330., 150.), target: feet + Vec3::Z * 20.,
        up: Vec3::Z, fovy: 65_f32.to_radians(), z_near: 2., z_far: 20000., ..Default::default() };
    for frame in 0..3 {
        clear_background(BLACK);
        set_camera(&camera);
        crate::render_fx::begin_view(&camera, 0., &scene.atmosphere, false);
        scene.draw(camera.position, 0., false, false, &g.transforms());
        art.draw(&g, &scene.atmosphere, camera.position, false);
        alice.atmosphere(&scene.atmosphere, camera.position);
        alice.draw_ghost(feet, false, false);
        crate::render::depth_read_only(|| scene.draw(camera.position, 0., false, true, &g.transforms()));
        let (_, dropped) = crate::render_fx::finish();
        ensure!(dropped == 0, "Mushroom fixture dropped geometry");
        set_default_camera();
        if frame == 2 { crate::viewer::save_capture(std::path::Path::new("private/garden3-scenes/mushroom-footing.png"))?; }
        next_frame().await;
    }
    Ok(())
}
