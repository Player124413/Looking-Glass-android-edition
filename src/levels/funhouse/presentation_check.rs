//! Entrance fixtures and the camera returning through the passable screen.
use super::*;

fn sync(o: &Funhouse, w: &mut World) {
    w.set_dynamic(o.colliders());
    w.set_camera_obstacles(o.camera_colliders());
}

pub fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/funhouse.bsp")?)?;
    let catalog = crate::decorations::Catalog::load(a, &map, "funhouse")?;
    let lamps: Vec<_> = map
        .entities
        .iter()
        .enumerate()
        .filter(|(_, e)| e.get("targetname").is_some_and(|n| n == "begin_lamps"))
        .collect();
    ensure!(lamps.len() == 6, "Entrance lamp fixture changed");
    for (id, e) in lamps {
        let prop = catalog
            .placements
            .iter()
            .find(|p| p.entity == id)
            .context("Entrance flame has no supporting lamp fixture")?;
        ensure!(
            prop.pose.translation == data::pose(e).translation
                && (prop.pose.rotation * Vec3::Z).distance(Vec3::Y) < 0.001,
            "Entrance fixture did not receive the authored roll"
        );
    }
    let mut o = Funhouse::load(a, &map)?;
    let initial = o.snapshot();
    let mut w = World::from_bsp(&map)?;
    // Exercise the production synchronization path as well as the scene owner.
    let mut interactions = crate::interaction::Interactions::load(&map)?;
    interactions.set_entry(a, &map, "funhouse", None)?;
    interactions.sync(&mut w);
    let feet = o.data.points["funhouse_start1"].translation;
    let pivot = feet + Vec3::Z * 36.;
    let desired = pivot - Vec3::Y * 132. + vec3(24., 0., 12.);
    let ordinary = w.sweep(pivot, desired, Vec3::splat(4.));
    let camera = w.camera_sweep(pivot, desired, Vec3::splat(4.));
    ensure!(
        camera.fraction < ordinary.fraction && w.body_clear(feet),
        "Entrance screen must occlude only the camera"
    );
    for hz in [30, 60, 144] {
        for skip in [false, true] {
            for restore in [false, true] {
                o.restore(&initial, &map)?;
                let s = o.saved.scene.as_mut().unwrap();
                s.clock.time = 29.;
                s.ending = Some(29.);
                if restore {
                    let saved = o.snapshot();
                    o.restore(&saved, &map)?;
                }
                sync(&o, &mut w);
                let mut p = Player::new(feet);
                let mut follow = crate::camera::Follow::default();
                let mut handoff = crate::camera::Handoff::default();
                let mut frames = 0;
                for _ in 0..hz * 2 {
                    let scripted = o.scene_camera();
                    let (eye, aim, _) = follow.update(&w, p.feet, Vec3::Y, 132., 1. / hz as f32);
                    let view = handoff.update(
                        &w,
                        scripted,
                        crate::cinematic::Camera::look(eye, aim),
                        Color::new(0., 0., 0., 0.),
                        1. / hz as f32,
                    );
                    if scripted.is_none() && handoff.overlay().a < 0.99 {
                        ensure!(
                            !w.camera_sweep(view.eye, view.eye, Vec3::splat(3.99))
                                .start_solid,
                            "Visible camera inside the entrance screen"
                        );
                        let sight = w.camera_sweep(pivot, view.eye, Vec3::splat(3.99));
                        ensure!(
                            !sight.start_solid && sight.fraction >= 0.999,
                            "Entrance wall hides Alice after the cutscene"
                        );
                        frames += 1;
                    }
                    if skip {
                        handoff.skip();
                        o.finish_scene(&map, &mut w, &mut p)?;
                    } else {
                        o.advance_scene(1. / hz as f32, &map, &mut w, &mut p)?;
                    }
                    sync(&o, &mut w);
                }
                ensure!(
                    frames > hz && w.body_clear(p.feet),
                    "Entrance did not return to clear gameplay"
                );
            }
        }
    }
    println!("PASS six restored entrance lamps; watched/skipped/restored camera returns at 30/60/144 Hz; player collision unchanged");
    Ok(())
}

pub async fn render(a: &mut Assets) -> Result<()> {
    let dir = std::path::Path::new("private/mirror-image-captures");
    std::fs::create_dir_all(dir)?;
    let mut world = crate::render::Scene::load(a, "funhouse")?;
    let mut o = Funhouse::load(a, &world.map)?;
    let mut p = Player::new(o.data.points["funhouse_start1"].translation);
    sync(&o, &mut world.world);
    let mut handoff = crate::camera::Handoff::default();
    o.saved.scene.as_mut().unwrap().clock.time = 29.;
    let from = o.scene_camera().unwrap();
    handoff.update(
        &world.world,
        Some(from),
        from,
        Color::new(0., 0., 0., 0.),
        0.,
    );
    o.finish_scene(&world.map, &mut world.world, &mut p)?;
    sync(&o, &mut world.world);
    let mut follow = crate::camera::Follow::default();
    for frame in 0..36 {
        let (eye, aim, _) = follow.update(&world.world, p.feet, Vec3::Y, 132., 1. / 60.);
        let view = handoff.update(
            &world.world,
            None,
            crate::cinematic::Camera::look(eye, aim),
            Color::new(0., 0., 0., 0.),
            1. / 60.,
        );
        draw(&mut world, &o, view, handoff.overlay());
        if [0, 8, 20, 35].contains(&frame) {
            crate::viewer::save_capture(&dir.join(format!("return-{frame}.png")))?;
        }
        next_frame().await;
    }
    // The same entrance mirror and suspended flames shown in the report.
    for (name, eye, aim) in [
        ("lamps", vec3(170., 3030., 75.), vec3(25., 3280., 410.)),
        (
            "chandelier",
            vec3(110., 3090., 160.),
            vec3(32., 3275., 256.),
        ),
    ] {
        for frame in 0..4 {
            draw(
                &mut world,
                &o,
                crate::cinematic::Camera::look(eye, aim),
                Color::new(0., 0., 0., 0.),
            );
            if frame == 3 {
                crate::viewer::save_capture(&dir.join(format!("{name}.png")))?;
            }
            next_frame().await;
        }
    }
    println!("PASS native Mirror Image entrance fixtures and 36 camera return frames");
    Ok(())
}

fn draw(world: &mut crate::render::Scene, o: &Funhouse, c: crate::cinematic::Camera, cover: Color) {
    let cam = Camera3D {
        position: c.eye,
        target: c.target,
        up: c.up,
        fovy: 75f32.to_radians(),
        z_near: 2.,
        z_far: 20000.,
        ..Default::default()
    };
    clear_background(BLACK);
    world.prepare_camera_portals(&cam, 1., false, &o.transforms());
    set_camera(&cam);
    crate::render_fx::begin_view(&cam, 1., &world.atmosphere, false);
    world.draw(c.eye, 1., false, false, &o.transforms());
    crate::render::depth_read_only(|| world.draw(c.eye, 1., false, true, &o.transforms()));
    crate::render_fx::finish();
    set_default_camera();
    draw_rectangle(0., 0., screen_width(), screen_height(), cover);
}
