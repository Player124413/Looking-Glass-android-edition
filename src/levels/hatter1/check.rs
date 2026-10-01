use super::*;
use crate::movement::Controls;
fn owner(r: &crate::route::Route) -> &Clockwork {
    r.interactions
        .levels
        .iter()
        .find_map(|s| s.ctl.downcast_ref::<Clockwork>())
        .unwrap()
}
fn own(r: &mut crate::route::Route) -> &mut Clockwork {
    r.interactions
        .levels
        .iter_mut()
        .find_map(|s| s.ctl.downcast_mut::<Clockwork>())
        .unwrap()
}
fn tick(
    o: &mut Clockwork,
    m: &Bsp,
    w: &mut World,
    p: &mut Player,
    seconds: f32,
    hz: u32,
) -> Result<()> {
    for _ in 0..(seconds * hz as f32).ceil() as usize {
        o.advance(1. / hz as f32, m, w, p, &[])?;
    }
    Ok(())
}
fn prerequisites(o: &mut Clockwork) {
    o.saved.age = 100.;
    o.saved.levers = [Some(0.), Some(1.), Some(4.), Some(40.), None];
    o.saved.hare = Some(30.);
    o.saved.port = Some(60.);
    o.saved.gryphon = Some(70.);
    o.saved.extendo = Some(80.);
}
pub(super) fn traversal(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/hatter1.bsp")?)?;
    let mut o = Clockwork::load(a, &map)?;
    let mut w = World::from_bsp(&map)?;
    w.set_dynamic(o.colliders());
    for k in [0, 1] {
        let lever = o.levers[k].translation;
        let feet = w
            .actor_footing(
                lever + vec3(64., 0., 100.),
                PLAYER_CENTER,
                PLAYER_HALF,
                120.,
            )
            .context("Gear lever approach lacks floor")?;
        let mut p = Player::new(feet);
        p.grounded = true;
        let aim = (lever + Vec3::Z * 20. - p.eye()).normalize();
        ensure!(
            o.prompt(&w, p.eye(), aim) == Some("E  Pull lever"),
            "Gear lever {k} not usable from {feet:?}"
        );
        o.update(&mut w, &p, aim, true);
        tick(&mut o, &map, &mut w, &mut p, 6., 120)?;
        ensure!(o.saved.levers[k].is_some(), "Lever E did not activate");
    }
    let mut p = Player::spawn(&w, interaction::spawn(&map, None).0).unwrap();
    tick(&mut o, &map, &mut w, &mut p, 3.2, 120)?;
    for id in 803..=815 {
        let k = o.objects.iter().position(|o| o.id == id).unwrap();
        let at = o.objects[k].base.translation;
        let feet = w
            .actor_footing(vec3(at.x, at.y, 340.), PLAYER_CENTER, PLAYER_HALF, 60.)
            .with_context(|| format!("Sink {id} has no walkable top"))?;
        p = Player::new(feet);
        p.grounded = true;
        tick(&mut o, &map, &mut w, &mut p, 0.05, 120)?;
        ensure!(w.body_clear(p.feet), "Sink embedded Alice");
        ensure!(
            if id <= 807 {
                o.saved.sinks[k] == 0.
            } else {
                o.saved.sinks[k] > 50.
            },
            "Sink {id} has wrong response {}",
            o.saved.sinks[k]
        );
        p = Player::spawn(&w, interaction::spawn(&map, None).0).unwrap();
        tick(&mut o, &map, &mut w, &mut p, 2.1, 120)?;
        ensure!(o.saved.sinks[k] == 0., "Sink {id} did not reset");
    }
    p = Player::new(
        w.actor_footing(vec3(-4784., 2400., 340.), PLAYER_CENTER, PLAYER_HALF, 70.)
            .context("Lift top missing")?,
    );
    p.grounded = true;
    let z = p.feet.z;
    o.event("Lift_Down");
    tick(&mut o, &map, &mut w, &mut p, 0.25, 120)?;
    ensure!(
        (p.feet.z - z + 170.).abs() < 1. && w.body_clear(p.feet),
        "Lift rider did not descend safely {:?}",
        p.feet
    );
    o.event("Lift_Up");
    tick(&mut o, &map, &mut w, &mut p, 0.25, 120)?;
    ensure!((p.feet.z - z).abs() < 1., "Lift rider did not return");
    let mut r = crate::route::Route::new(a, "hatter1", None)?;
    prerequisites(own(&mut r));
    own(&mut r).rebuild(&map)?;
    r.interactions.sync(&mut r.world);
    for (x, y) in [
        (-928., 2264.),
        (-928., 3048.),
        (-608., 3056.),
        (-608., 2272.),
    ] {
        r.player = Player::new(
            r.world
                .actor_footing(vec3(x, y, 260.), PLAYER_CENTER, PLAYER_HALF, 65.)
                .context("Chair cushion support missing")?,
        );
        r.player.grounded = true;
        r.tick(Controls::default())?;
    }
    // Independently staged at each authored pad; all travel and landing use ordinary movement.
    let mut pads = crate::route::Route::new(a, "hatter1", None)?;
    prerequisites(own(&mut pads));
    own(&mut pads).rebuild(&map)?;
    pads.interactions.sync(&mut pads.world);
    for (id, x, y) in [
        (152, -928., 2264.),
        (153, -928., 3048.),
        (151, -608., 3056.),
        (129, -608., 2272.),
    ] {
        let origin = pads
            .world
            .traversal
            .pushes
            .iter()
            .find(|v| v.id.0 == id)
            .unwrap()
            .origin;
        pads.player = Player::new(origin + Vec3::Z * 6.);
        let goal = pads
            .world
            .actor_footing(vec3(x, y, 260.), PLAYER_CENTER, PLAYER_HALF, 65.)
            .context("Pad landing lacks cushion")?;
        pads.navigate(goal)?;
    }
    ensure!(
        owner(&pads).saved.clock.is_some(),
        "Pad traversal missed a chair trigger"
    );
    ensure!(
        owner(&r).saved.cubes == [true; 4] && owner(&r).saved.clock.is_some(),
        "Chair touch triggers did not lower clock"
    );
    println!("PASS Clockwork production E input, 5 safe/8 sinking pillars, lift rider and four cushion contacts");
    Ok(())
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    super::lever::check(a)?;
    let map = Bsp::parse(&a.read("maps/hatter1.bsp")?)?;
    for hz in [30, 60, 144] {
        let mut o = Clockwork::load(a, &map)?;
        let mut w = World::from_bsp(&map)?;
        w.set_dynamic(o.colliders());
        let mut p = Player::spawn(&w, interaction::spawn(&map, None).0)
            .context("Blocked Clockwork arrival")?;
        for n in [
            "stop_clock",
            "Open_Port",
            "Hatter1_Cinema2",
            "extendo",
            "cube_1",
        ] {
            o.event(n);
        }
        ensure!(
            o.saved.scene.is_none() && o.saved.levers[4].is_none() && o.saved.clock.is_none(),
            "Out of order event accepted"
        );
        let before = o.snapshot();
        o.advance(0., &map, &mut w, &mut p, &[])?;
        ensure!(o.snapshot() == before, "Pause advanced Clockwork");
        o.press(1);
        o.press(1);
        tick(&mut o, &map, &mut w, &mut p, 2.5, hz)?;
        ensure!(o.saved.both().is_none(), "One lever completed both");
        o.press(0);
        tick(&mut o, &map, &mut w, &mut p, 3.1, hz)?;
        let gear = o.objects.iter().find(|o| o.name == "astralmonkey").unwrap();
        ensure!(
            (gear.pose.translation.z - gear.base.translation.z).abs() < 0.1,
            "Gear did not descend"
        );
        o.event("NORETURN");
        tick(&mut o, &map, &mut w, &mut p, 0.3, hz)?;
        let barrier = o.objects.iter().find(|o| o.name == "no_return").unwrap();
        ensure!(
            (barrier.pose.translation - barrier.base.translation).length() < 0.1,
            "One way barrier did not close"
        );
        let saved = o.snapshot();
        o.restore(&saved, &map)?;
        ensure!(o.snapshot() == saved, "Machinery restore changed state");
        let mut invalid = saved.clone();
        invalid["stopped"] = true.into();
        ensure!(
            o.restore(&invalid, &map).is_err() && o.snapshot() == saved,
            "Invalid state accepted or mutated live owner"
        );
        prerequisites(&mut o);
        o.rebuild(&map)?;
        for n in ["cube_3", "cube_1", "cube_3", "cube_4"] {
            o.event(n);
        }
        ensure!(o.saved.clock.is_none(), "Duplicate chair unlocked clock");
        o.event("cube_2");
        w.set_dynamic(o.colliders());
        // The real route can land with only the edge of Alice's footprint on
        // the cushion. A centre-only support probe misses that valid rider.
        p = Player::new(w.actor_footing(vec3(-635.062, 2261.3633, 260.), PLAYER_CENTER,
            PLAYER_HALF, 65.).context("Last cushion support missing")?);
        p.grounded = true;
        let chair_start = o.saved.age;
        tick(&mut o, &map, &mut w, &mut p, 5.2, hz)?;
        ensure!(o.saved.age - chair_start >= 5.19 && w.body_clear(p.feet),
            "Occupied cushion stopped the clock or embedded its rider at {hz} Hz");
        let (tr, _) = o
            .trigger_pose("clock_teleport", vec3(-768., 1892., 544.))
            .unwrap();
        ensure!(
            tr.distance(vec3(-768., 1962., 184.)) < 0.1,
            "Clock teleport did not follow clock"
        );
        let final_state = o.snapshot();
        for skip in [false, true] {
            o.restore(&final_state, &map)?;
            o.press(4);
            if skip {
                let mut s = Story::load(a, "hatter1");
                o.skip(&map, &mut w, &mut p, &mut s)?;
            }
            tick(
                &mut o,
                &map,
                &mut w,
                &mut p,
                if skip { 0.6 } else { 30.2 },
                hz,
            )?;
            ensure!(
                o.saved.stopped && o.saved.scene.is_none(),
                "Clock stop did not complete hz={hz} skip={skip}"
            );
            let hand = o.objects.iter().find(|o| o.name == "clockhr").unwrap();
            ensure!(
                hand.pose
                    .rotation
                    .dot(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2))
                    .abs()
                    > 0.999,
                "Clock hour hand not stopped"
            );
            let saved = o.snapshot();
            o.event("stop_clock");
            ensure!(o.snapshot() == saved, "Clock stop replayed");
        }
        println!(
            "PASS Clockwork {hz} Hz: levers, chairs, pause, restore, watched/skipped clock stop"
        );
    }
    for kind in [Kind::Hare, Kind::Port, Kind::Gryphon] {
        for skip in [false, true] {
            let mut r = crate::route::Route::new(a, "hatter1", None)?;
            r.skip_cinematics = skip;
            prerequisites(own(&mut r));
            own(&mut r).saved.hare = None;
            own(&mut r).saved.port = None;
            own(&mut r).saved.gryphon = None;
            own(&mut r).saved.extendo = None;
            if kind != Kind::Hare {
                own(&mut r).saved.hare = Some(30.);
            }
            if kind == Kind::Gryphon {
                own(&mut r).saved.port = Some(60.);
            }
            own(&mut r).begin(kind);
            r.wait_for_cinematic()?;
            ensure!(own(&mut r).saved.scene.is_none(), "Scene {kind:?} stuck");
            ensure!(
                r.world.body_clear(r.player.feet),
                "Scene {kind:?} blocked handoff"
            );
            ensure!(
                match kind {
                    Kind::Hare => owner(&r).saved.hare.is_some(),
                    Kind::Port => owner(&r).saved.port.is_some(),
                    _ => owner(&r).saved.gryphon.is_some(),
                },
                "Scene {kind:?} did not commit"
            );
            println!(
                "PASS Clockwork actual dialogue/camera clock {kind:?} skip={skip} feet={:?}",
                r.player.feet
            );
        }
    }
    let mut r = crate::route::Route::new(a, "hatter1", None)?;
    r.stop_at_exit = true;
    for at in [
        vec3(-2528., 1764., 220.),
        vec3(-2652., 2168., 52.),
        vec3(-2940., 2292., 64.),
        vec3(-768., 1892., 500.),
    ] {
        let e = r.interactions.triggers(1. / 120., at, at);
        ensure!(
            e.transition.is_none() && e.teleport.is_none(),
            "Early exit/rescue bypass at {at:?}"
        );
    }
    prerequisites(own(&mut r));
    for n in ["cube_1", "cube_2", "cube_3", "cube_4"] {
        own(&mut r).event(n);
    }
    own(&mut r).saved.age += 6.;
    own(&mut r).saved.levers[4] = Some(100.);
    own(&mut r).saved.stopped = true;
    let map = Bsp::parse(&a.read("maps/hatter1.bsp")?)?;
    own(&mut r).rebuild(&map)?;
    r.interactions.sync(&mut r.world);
    r.player = Player::new(
        r.world
            .actor_footing(vec3(-2528., 1848., 320.), PLAYER_CENTER, PLAYER_HALF, 180.)
            .context("Exit approach has no floor")?,
    );
    r.wait(2.)?;
    r.walk(vec3(-2528., 1780., r.player.feet.z), false)?;
    for _ in 0..240 {
        r.tick(Controls {
            wish: -Vec2::Y,
            ..Default::default()
        })?;
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition == Some(("hatter2".into(), None)),
        "Wrong battle exit {:?}",
        r.transition
    );
    let mut next = r.depart(a, true)?;
    next.interactions
        .prepare_player(&mut next.stats, &mut next.player);
    ensure!(
        next.interactions
            .levels
            .iter()
            .any(|s| s.ctl.id() == "hatter2")
            && next.world.body_clear(next.player.feet),
        "Battle owner or solid arrival missing"
    );
    println!(
        "PASS Clockwork gated exit walked into implemented Hatter arena at {:?}",
        next.player.feet
    );
    Ok(())
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        if let Ok(mode) = std::env::var("LOOKING_GLASS_CLOCKWORK_SAVE") {
            return super::saves::run(a, mode == "write").await;
        }
        std::fs::create_dir_all("private/hatter1-work/captures")?;
        let mut world = crate::render::Scene::load(a, "hatter1")?;
        let mut o = Clockwork::load(a, &world.map)?;
        let mut art = art::Art::load(a)?;
        super::lever::render(a, &mut world, &mut art).await?;
        for (name, kind, t, eye, target) in [
            (
                "gear",
                None,
                0.,
                vec3(-4050., 3008., 430.),
                vec3(-4352., 3008., 360.),
            ),
            (
                "mirror",
                None,
                0.,
                vec3(-5100., 2610., 440.),
                vec3(-5200., 2300., 440.),
            ),
            (
                "mirror-cat",
                Some(Kind::Mirror),
                5.,
                vec3(-5100., 2610., 440.),
                vec3(-5200., 2300., 440.),
            ),
            ("mirror-side", Some(Kind::Mirror), 5., vec3(-5490., 2340., 390.), vec3(-5200., 2530., 380.)),
            ("mirror-close", None, 0., vec3(-5480., 2330., 380.), vec3(-5204., 2306., 400.)),
            (
                "chairs",
                None,
                0.,
                vec3(-768., 3200., 480.),
                vec3(-768., 2320., 250.),
            ),
            ("hare", Some(Kind::Hare), 36., Vec3::ZERO, Vec3::ZERO),
            ("port", Some(Kind::Port), 11., Vec3::ZERO, Vec3::ZERO),
            ("gryphon", Some(Kind::Gryphon), 8., Vec3::ZERO, Vec3::ZERO),
            ("stop", Some(Kind::Stop), 16., Vec3::ZERO, Vec3::ZERO),
        ] {
            o.saved = Saved::new(o.objects.len());
            if let Some(k) = kind {
                prerequisites(&mut o);
                o.begin(k);
                o.saved.scene.as_mut().unwrap().time = t;
            }
            o.rebuild(&world.map)?;
            let camera = o.scene_camera().unwrap_or(Camera::look(eye, target));
            let cam = Camera3D {
                position: camera.eye,
                target: camera.target,
                up: camera.up,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            };
            let (extra, hide) = o.reflection();
            world.set_reflection(extra, hide);
            world.set_reflection_actor(o.reflection_actor());
            for frame in 0..4 {
                clear_background(BLACK);
                world.prepare_camera_portals(&cam, t, false, &o.transforms());
                set_camera(&cam);
                crate::render_fx::begin_view(&cam, t, &world.atmosphere, false);
                world.draw(camera.eye, t, false, false, &o.transforms());
                art.draw(&o, &world.atmosphere, camera.eye, false);
                crate::render::depth_read_only(|| {
                    world.draw(camera.eye, t, false, true, &o.transforms());
                    art.effects(&o, camera.eye, &world.atmosphere);
                });
                crate::render_fx::finish();
                set_default_camera();
                if frame == 3 {
                    if matches!(name, "gear" | "hare") {
                        world.check_reflection_culled()?;
                    }
                    if name.starts_with("mirror") {
                        world.check_reflection_pixels()?;
                    }
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/hatter1-work/captures/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        println!("PASS Clockwork native captures");
        Ok(())
    })
}
