use super::*;
pub(super) fn owner(r: &mut crate::route::Route) -> &mut Maze {
    r.interactions
        .levels
        .iter_mut()
        .find_map(|s| s.ctl.downcast_mut::<Maze>())
        .unwrap()
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    if std::env::var_os("LOOKING_GLASS_HEDGE2_ROUTE").is_some() {
        return route::walk(a, false, false);
    }
    let map = Bsp::parse(&a.read("maps/hedge2.bsp")?)?;
    let mut w = World::from_bsp(&map)?;
    let mut p =
        Player::spawn(&w, interaction::spawn(&map, None).0).context("Blocked hedge arrival")?;
    let mut stats = Stats::default();
    let mut story = Story::load(a, "hedge2");
    for hz in [30, 60, 144] {
        for skip in [false, true] {
            let mut o = Maze::load(a, &map)?;
            o.prepare_player(&mut stats, &mut p);
            ensure!(stats.turtle_air && p.breath.shell, "Missing shell");
            ensure!(
                !o.ready() && o.saved.motion == Saved::default().motion,
                "Open initial gate"
            );
            for k in 0..2 {
                o.fire(k);
                for n in 0..hz * 16 {
                    if n == hz && skip {
                        ensure!(
                            o.skip(&map, &mut w, &mut p, &mut story)?,
                            "Cutaway not skippable"
                        );
                    }
                    o.advance(1. / hz as f32, &map, &mut w, &mut p, &[])?;
                    if n == hz / 2 {
                        let saved = o.snapshot();
                        let mut next = Maze::load(a, &map)?;
                        next.restore(&saved, &map)?;
                        ensure!(saved == next.snapshot(), "Changed saved cutaway");
                        o = next;
                    }
                }
                ensure!(
                    o.saved.finished[k] && !o.scripted(),
                    "Unfinished lever sequence"
                );
                let before = o.snapshot();
                o.event(THREADS[k]);
                ensure!(o.snapshot() == before, "Lever repeated");
            }
            ensure!(
                o.ready()
                    && o.saved.motion[0] == 0.
                    && o.saved.motion[1..].iter().all(|v| *v == 1.)
                    && o.active_npcs() == [34, 567],
                "Incomplete final machinery"
            );
            println!("PASS hedge2 {hz}Hz skip={skip}: both scenes, six brushes, robots and shell");
        }
    }
    let mut air = Maze::load(a, &map)?;
    ensure!(air.bubbles.len() == 8, "Missing authored air emitters");
    for at in air.bubbles.clone() {
        let mut swimmer = Player::new(at);
        swimmer.breath.submerged = 19.;
        air.prepare_player(&mut stats, &mut swimmer);
        ensure!(
            swimmer.breath.remaining() == 20.,
            "Air emitter does not restore breath"
        );
    }
    let mut dry = Player::new(Vec3::splat(10000.));
    dry.breath.submerged = 19.;
    air.prepare_player(&mut stats, &mut dry);
    ensure!(
        dry.breath.remaining() == 1.,
        "Air refill outside bubble volume"
    );
    println!("PASS hedge2 eight original air emitters and bounded breath contact");
    // Real interaction selection and E input, with data-owned animation duration.
    for k in 0..2 {
        let mut o = Maze::load(a, &map)?;
        w.set_dynamic(o.colliders());
        let mut p = Player::new(o.levers[k].translation);
        let aim = o.levers[k].rotation * Vec3::X;
        ensure!(w.body_clear(p.feet), "Lever standing point blocked");
        ensure!(
            o.pick(&w, p.eye(), aim) == Some(k),
            "Lever cannot be reached by E"
        );
        o.update(&mut w, &p, aim, true);
        ensure!(
            o.saved.used[k] && !o.saved.fired[k],
            "Lever fired before animation"
        );
        let mut other = Maze::load(a, &map)?;
        other.restore(&o.snapshot(), &map)?;
        for _ in 0..120 * 18 {
            let mut p2 = p.clone();
            o.advance(1. / 120., &map, &mut w, &mut p, &[])?;
            other.advance(1. / 120., &map, &mut w, &mut p2, &[])?;
            ensure!(
                o.snapshot() == other.snapshot(),
                "Resumed lever future diverged"
            );
        }
        ensure!(o.saved.finished[k], "E did not finish cutaway");
    }
    println!("PASS hedge2 real lever reach, delayed callbacks and identical restored futures");
    // Physical aperture negatives, using the restored brush collision itself.
    let mut o = Maze::load(a, &map)?;
    w.set_dynamic(o.colliders());
    ensure!(
        w.body_trace(vec3(2240., 2700., 1800.), vec3(2240., 2800., 1800.))
            .fraction
            < 1.,
        "End gate not solid"
    );
    ensure!(
        w.body_trace(vec3(-1168., 850., 560.), vec3(-1168., 1100., 560.))
            .fraction
            < 1.,
        "Water gate not solid"
    );
    let original = o.snapshot();
    let mut bad = original.clone();
    bad["motion"][3] = serde_json::json!(2.);
    ensure!(
        o.restore(&bad, &map).is_err() && o.snapshot() == original,
        "Invalid save changed controller"
    );
    let before = o.snapshot();
    o.advance(0., &map, &mut w, &mut p, &[])?;
    ensure!(before == o.snapshot(), "Pause advanced machinery");
    o.fire(1);
    o.skip(&map, &mut w, &mut p, &mut story)?;
    ensure!(!o.ready(), "End lever bypassed water route");
    o.fire(0);
    o.skip(&map, &mut w, &mut p, &mut story)?;
    for _ in 0..300 {
        o.advance(1. / 120., &map, &mut w, &mut p, &[])?;
    }
    ensure!(
        w.body_trace(vec3(2240., 2700., 1800.), vec3(2240., 2800., 1800.))
            .fraction
            == 1.,
        "Open end gate obstructs Alice"
    );
    ensure!(
        w.body_trace(vec3(-1168., 850., 480.), vec3(-1168., 1100., 480.))
            .fraction
            == 1.,
        "Open water gate obstructs swimmer"
    );
    o.saved.motion[5] = 0.5;
    ensure!(!o.ready(), "Half-open left leaf allowed exit");
    let mut r = crate::route::Route::new(a, "hedge2", Some("hedge2_start1"))?;
    let beyond = vec3(2240., 2808., 1884.);
    let event = r.interactions.triggers(1. / 120., beyond, beyond);
    ensure!(
        event.transition.is_none(),
        "Closed-gate trigger allowed tower departure"
    );
    println!(
        "PASS hedge2 physical apertures, wrong-route exit refusal, pause and atomic save rejection"
    );
    Ok(())
}
fn cast_check(a: &mut Assets) -> Result<()> {
    let mut r = crate::route::Route::new(a, "hedge2", Some("hedge2_start1"))?;
    r.enable_native_cast(a)?;
    let cast = r.native_cast.as_mut().unwrap();
    let before = serde_json::to_value(cast.snapshot())?;
    let actors = before["actors"].as_array().context("Missing cast")?;
    let ids: Vec<_> = [34, 567]
        .iter()
        .map(|id| {
            actors
                .iter()
                .position(|v| v["spawn"]["clock_spawn"] == *id)
                .context("Missing relay-owned automaton")
        })
        .collect::<Result<_>>()?;
    for &i in &ids {
        ensure!(
            actors[i]["clock"]["active"] == false,
            "Lever robot started active"
        );
    }
    owner(&mut r).fire(0);
    let cast = r.native_cast.as_mut().unwrap();
    cast.activate_levels(&r.interactions.levels);
    for &i in &ids {
        ensure!(
            serde_json::to_value(cast.snapshot())?["actors"][i]["clock"]["active"] == true,
            "Lever did not activate robot"
        );
        cast.hit(crate::combat::Hit {
            id: i,
            damage: 20.,
            kind: crate::combat::DamageKind::Other,
            knockback: Vec3::ZERO,
        });
    }
    let wounded = serde_json::to_value(cast.snapshot())?;
    cast.activate_levels(&r.interactions.levels);
    ensure!(
        serde_json::to_value(cast.snapshot())? == wounded,
        "Repeated relay healed or duplicated robots"
    );
    let saved = cast.snapshot();
    cast.restore(&saved)?;
    ensure!(
        serde_json::to_value(cast.snapshot())? == wounded,
        "Robot save changed wounded state"
    );
    println!("PASS hedge2 native robot activation, damage, idempotency and persistence");
    Ok(())
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        if let Ok(mode) = std::env::var("LOOKING_GLASS_HEDGE2_SAVE") {
            return saves::run(a, mode == "write").await;
        }
        cast_check(a)?;
        std::fs::create_dir_all("private/hedge2-work/captures")?;
        let mut world = crate::render::Scene::load(a, "hedge2")?;
        let mut o = Maze::load(a, &world.map)?;
        let mut art = art::Art::load(a)?;
        for (name, k, time) in [
            ("water-lever", 0, -1.),
            ("water-pull", 0, -2.),
            ("tunnel-closed", 0, 7.),
            ("tunnel-opening", 0, 9.),
            ("end-lever", 1, -1.),
            ("end-pull", 1, -2.),
            ("end-opening", 1, 1.2),
        ] {
            o.saved = Saved::default();
            if time == -2. {
                o.saved.used[k] = true;
                o.saved.pull = Some(Pull {
                    index: k,
                    time: 1.5,
                    pose: Transform {
                        translation: o.levers[k].point(vec3(-54., 0., 0.)),
                        rotation: o.levers[k].rotation,
                    },
                });
            }
            if time >= 0. {
                o.fire(k);
                o.saved.scene.as_mut().unwrap().time = time;
                o.saved.motion[if k == 0 { 3 } else { 4 }] = if k == 0 {
                    ((time - 8.) / 2.).clamp(0., 1.)
                } else {
                    ((time - 1.) / 0.4).clamp(0., 1.)
                };
                o.saved.motion[5] = o.saved.motion[4];
            }
            o.rebuild(&world.map)?;
            let camera = o.camera(&world.world).unwrap_or_else(|| {
                let p = o.levers[k];
                Camera::look(
                    p.point(vec3(-100., -100., 90.)),
                    p.point(vec3(30., 0., 24.)),
                )
            });
            let cam = Camera3D {
                position: camera.eye,
                target: camera.target,
                up: camera.up,
                fovy: 60_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            };
            for frame in 0..4 {
                clear_background(world.atmosphere.background());
                world.prepare_camera_portals(&cam, 0., false, &o.transforms());
                set_camera(&cam);
                crate::render_fx::begin_view(&cam, 0., &world.atmosphere, false);
                world.draw(camera.eye, 0., false, false, &o.transforms());
                art.draw(&o, &world.atmosphere, camera.eye, false);
                crate::render::depth_read_only(|| {
                    world.draw(camera.eye, 0., false, true, &o.transforms())
                });
                crate::render_fx::finish();
                set_default_camera();
                if frame == 3 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/hedge2-work/captures/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        println!("PASS hedge2 native lever and cutaway captures");
        Ok(())
    })
}
