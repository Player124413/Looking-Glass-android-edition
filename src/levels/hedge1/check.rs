use super::*;
pub fn check(a: &mut Assets) -> Result<()> {
    if std::env::var_os("LOOKING_GLASS_MAZE_GRAPH").is_some() {
        return super::route::graph_check(a);
    }
    let map = Bsp::parse(&a.read("maps/hedge1.bsp")?)?;
    let script = String::from_utf8_lossy(&a.read("maps/hedge1.scr")?).into_owned();
    let start = script
        .find("void SeekCine()")
        .context("Missing historical exit evidence")?;
    ensure!(
        script[..start].rfind("/*") > script[..start].rfind("*/"),
        "SeekCine became executable; re-review exit ownership"
    );
    ensure!(
        script[start..]
            .split("*/")
            .next()
            .unwrap()
            .contains("tower1$tower1_start1"),
        "Inactive exit evidence changed"
    );
    for hz in [30, 60, 144] {
        let mut m = Maze::load(&map)?;
        let mut w = World::from_bsp(&map)?;
        let mut p = Player::new(vec3(3200., 4288., 1928.));
        let dt = 1. / hz as f32;
        w.set_dynamic(m.colliders());
        ensure!(
            w.body_trace(vec3(3008., 4288., 1920.1), vec3(2816., 4288., 1920.1))
                .fraction
                < 1.,
            "Closed leaves do not block Alice"
        );
        m.event("SeekEnd");
        m.companion_contacts(&[CompanionContact {
            name: "seek_kid".into(),
            feet: vec3(3136., 4096., 1944.),
            half: vec3(16., 16., 30.),
            visible: true,
            holding: false,
        }]);
        for _ in 0..hz {
            m.advance(dt, &map, &mut w, &mut p, &[])?;
        }
        ensure!(
            m.saved.doors.iter().all(|p| *p == 1.) && !m.ready(),
            "Prelude plate did not open without completing escort"
        );
        m.event("SeekStart");
        m.companion_contacts(&[]);
        for _ in 0..hz {
            m.advance(dt, &map, &mut w, &mut p, &[])?;
        }
        ensure!(m.saved.doors == [0.; 3], "Empty plate failed to close");
        p = Player::new(vec3(3136., 4096., 1944.));
        for _ in 0..hz {
            m.advance(dt, &map, &mut w, &mut p, &[])?;
        }
        ensure!(
            m.saved.doors[1] == 1.
                && !m.ready()
                && m.exit_contact(&EXIT.destination())
                    .unwrap()
                    .transition
                    .is_none(),
            "Alice alone bypassed the escort"
        );
        if hz == 60 {
            // A real sprint from the occupied plate must meet the closing leaves.
            for _ in 0..480 {
                m.advance(1. / 120., &map, &mut w, &mut p, &[])?;
                p.tick(
                    &w,
                    crate::movement::Controls {
                        wish: (vec2(2800., 4288.) - p.feet.truncate()).normalize_or_zero(),
                        run: true,
                        ..Default::default()
                    },
                );
            }
            ensure!(
                p.feet.x > 2912. && !m.ready(),
                "Alice outran the plate gate without the child"
            );
        }
        p = Player::new(vec3(3200., 4288., 1920.1));
        m.companion_contacts(&[CompanionContact {
            name: "seek_kid_chase".into(),
            feet: vec3(3136., 4096., 1944.),
            half: vec3(16., 16., 30.),
            visible: true,
            holding: true,
        }]);
        for _ in 0..hz * 12 {
            m.advance(dt, &map, &mut w, &mut p, &[])?;
        }
        ensure!(m.ready(), "Held plate timed out");
        ensure!(
            w.body_trace(vec3(3008., 4288., 1920.1), vec3(2816., 4288., 1920.1))
                .fraction
                == 1.,
            "Open leaves block the exit corridor"
        );
        let saved = m.snapshot();
        m.advance(0., &map, &mut w, &mut p, &[])?;
        ensure!(saved == m.snapshot(), "Paused gate moved");
        let mut restored = Maze::load(&map)?;
        restored.restore(&saved, &map)?;
        restored.companion_contacts(&m.companions);
        let mut q = p.clone();
        let mut other = World::from_bsp(&map)?;
        for _ in 0..hz {
            m.advance(dt, &map, &mut w, &mut p, &[])?;
            restored.advance(dt, &map, &mut other, &mut q, &[])?;
        }
        ensure!(
            m.snapshot() == restored.snapshot(),
            "Restored gate diverged"
        );
        ensure!(
            m.exit_contact(&EXIT.destination()).unwrap().transition == Some(EXIT.destination()),
            "Physical exit failed"
        );
        ensure!(
            m.update(&mut w, &p, Vec3::ZERO, false).transition.is_none(),
            "Exit delivered twice"
        );
        m.transition_failed(&EXIT.destination());
        for _ in 0..hz + 3 {
            m.advance(dt, &map, &mut w, &mut p, &[])?;
        }
        ensure!(
            m.update(&mut w, &p, Vec3::ZERO, false).transition == Some(EXIT.destination()),
            "Failed exit cannot retry"
        );
        println!("PASS Majestic Maze plate, child gate, pause/restore and exit retry at {hz} Hz");
    }
    crate::npc::companion_check(a)?;
    Ok(())
}
pub async fn render(a: &mut Assets) -> Result<()> {
    let path = std::env::var("LOOKING_GLASS_MAZE_CAPTURE")
        .context("Set LOOKING_GLASS_MAZE_CAPTURE to an ordinary route checkpoint")?;
    let cp: crate::route::Checkpoint = serde_json::from_slice(&std::fs::read(path)?)?;
    let mut r = crate::route::Route::resume(a, &cp)?;
    let mut scene = crate::render::Scene::load(a, "hedge1")?;
    r.interactions.presentation.apply(&mut scene);
    let target = std::env::var("LOOKING_GLASS_MAZE_LOOK")
        .ok()
        .and_then(|v| crate::interaction::vector(&v))
        .unwrap_or(vec3(3136., 4096., 1970.));
    for frame in 0..3 {
        clear_background(BLACK);
        let view = Camera3D {
            position: r.player.eye(),
            target,
            up: Vec3::Z,
            fovy: 75_f32.to_radians(),
            z_near: 2.,
            z_far: 20000.,
            ..Default::default()
        };
        set_camera(&view);
        let poses = r.interactions.transforms();
        scene.prepare_camera_portals(&view, r.environment_clock, false, &poses);
        crate::render_fx::begin_view(&view, r.environment_clock, &scene.atmosphere, false);
        scene.draw(view.position, r.environment_clock, false, false, &poses);
        if let Some(n) = &mut r.native_cast {
            n.draw(
                view.position,
                (target - view.position).normalize_or_zero(),
                &scene.atmosphere,
                false,
            );
        }
        crate::render::depth_read_only(|| {
            scene.draw(view.position, r.environment_clock, false, true, &poses)
        });
        set_default_camera();
        if frame == 2 {
            crate::viewer::save_capture(std::path::Path::new("private/hedge1/capture.png"))?;
        }
        next_frame().await;
    }
    Ok(())
}
