use super::*;
pub(super) fn check(assets: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&assets.read("maps/tower2.bsp")?)?;
    intro(assets, &map)?;
    rim(&map)?;
    machinery(assets, &map)?;
    currents(assets, &map)?;
    for hz in [30, 60, 144] {
        let mut i = Interactions::load(&map)?;
        i.set_entry(assets, &map, "tower2", None)?;
        owner(&mut i)?.upgraded();
        let mut world = World::from_bsp(&map)?;
        i.sync(&mut world);
        ensure!(
            world.liquid_at(vec3(1776., 2400., 600.)) != 0,
            "Tank dry at arrival"
        );
        ensure!(
            world.liquid_at(vec3(1776., 2400., 640.)) == 0,
            "Water above initial surface"
        );
        let mut p = Player::spawn(&world, interaction::spawn(&map, None).0)
            .context("Tank entrance blocked")?;
        let mut swimmer = Player::new(vec3(1776., 2400., 500.));
        swimmer.breath.shell = true;
        swimmer.tick(&world, Controls::default());
        ensure!(swimmer.swimming, "Tank does not support swimming");
        for stage in 0..3 {
            let t = i.levels[0].ctl.downcast_mut::<Tower>().unwrap();
            ensure!(
                t.event(&format!("MoveWater{}", stage + 1)).is_some(),
                "Flush not handled"
            );
            for _ in 0..hz * 8 {
                i.advance_school(1. / hz as f32, &map, &mut world, &mut p)?;
            }
            let t = i.levels[0].ctl.downcast_ref::<Tower>().unwrap();
            let expected = 624. + 256. * (stage + 1) as f32;
            ensure!((t.height() - expected).abs() < 0.1, "Wrong water height");
            ensure!(
                world.liquid_at(vec3(1776., 2400., expected - 2.)) != 0
                    && world.liquid_at(vec3(1776., 2400., expected + 2.)) == 0,
                "Moved water does not match surface"
            );
            let snapshot = i.snapshot();
            let mut loaded = Interactions::load(&map)?;
            loaded.set_entry(assets, &map, "tower2", None)?;
            loaded.restore(&snapshot, &map)?;
            ensure!(
                serde_json::to_value(snapshot)? == serde_json::to_value(loaded.snapshot())?,
                "Tank restore differs"
            );
        }
        println!("PASS tower2 {hz} Hz: wet arrival, swimming, three flush heights, restored state");
    }
    // Mid-rise continuation and pause, with the actual convex water brush.
    let mut a = Tower::load(assets, &map)?;
    a.upgraded();
    a.event("MoveWater1");
    a.saved.stages[0] = Some(2.5);
    a.rebuild(&map)?;
    let mut b = Tower::load(assets, &map)?;
    b.restore(&a.snapshot(), &map)?;
    let mut w = World::from_bsp(&map)?;
    let mut p = Player::new(vec3(1776., 2400., 640.));
    let saved = a.snapshot();
    a.advance(0., &map, &mut w, &mut p, &[])?;
    ensure!(a.snapshot() == saved, "Paused water moved");
    for _ in 0..600 {
        a.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
        b.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
        ensure!(
            a.snapshot() == b.snapshot(),
            "Resumed rising water diverged"
        );
    }
    println!("PASS tower2 mid-rise and pause; continuous traversal is checked separately");
    Ok(())
}

fn intro(a: &mut Assets, map: &Bsp) -> Result<()> {
    for hz in [30, 60, 144] {
        for skip in [None, Some(0.2), Some(3.)] {
            let mut i = Interactions::load(map)?;
            i.set_entry(a, map, "tower2", None)?;
            let mut w = World::from_bsp(map)?;
            i.sync(&mut w);
            let mut p = Player::spawn(&w, interaction::spawn(map, None).0)
                .context("Blocked tower arrival")?;
            let home = p.feet;
            let mut stats = Stats::default();
            i.prepare_player(&mut stats, &mut p);
            ensure!(
                stats.turtle_air && p.breath.shell,
                "Missing tower shell grant"
            );
            let mut story = Story::load(a, "tower2");
            let mut skipped = false;
            let mut sounds = crate::audio::world::State::default();
            sounds.update(Vec::new());
            let mut heard = Vec::new();
            let leave_at = 2.5 + owner(&mut i)?.data.idle + owner(&mut i)?.data.smile;
            for n in 0..hz * 60 {
                if !i.scripted() {
                    break;
                }
                if !skipped && skip.is_some_and(|s| n as f32 / hz as f32 >= s) {
                    i.skip_cinematic(map, &mut w, &mut p, &mut story)?;
                    skipped = true;
                }
                i.advance_school(1. / hz as f32, map, &mut w, &mut p)?;
                if i.prepare_story(&mut story) {
                    story.tick(1. / hz as f32, false);
                }
                i.sync_cinematic_story(&story);
                for s in story.take_completed() {
                    i.completed_dialogue(&s);
                }
                let mut clocks = Vec::new();
                owner(&mut i)?.sounds(&mut clocks);
                for (path, _) in sounds.update(clocks) {
                    ensure!(
                        !path.ends_with("disappear.wav")
                            || (n + 1) as f32 / hz as f32 >= leave_at - 0.001,
                        "Tower departure sound fired before the Cat left"
                    );
                    heard.push(path);
                }
                if n == hz * 3 {
                    let saved = i.snapshot();
                    let mut other = Interactions::load(map)?;
                    other.set_entry(a, map, "tower2", None)?;
                    other.restore(&saved, map)?;
                    ensure!(
                        serde_json::to_value(other.snapshot())? == serde_json::to_value(saved)?,
                        "Tower scene restore differs"
                    );
                    i = other;
                    i.sync(&mut w);
                    let mut clocks = Vec::new();
                    owner(&mut i)?.sounds(&mut clocks);
                    ensure!(
                        crate::audio::world::State::default()
                            .update(clocks)
                            .is_empty(),
                        "Loading Tower intro replayed a past sound"
                    );
                }
            }
            ensure!(
                heard.iter().filter(|s| s.ends_with("/appear.wav")).count()
                    == usize::from(skip.is_none_or(|t| t >= 2.5))
                    && heard
                        .iter()
                        .filter(|s| s.ends_with("/disappear.wav"))
                        .count()
                        == usize::from(skip.is_none()),
                "Tower intro sound sequence missing or repeated"
            );
            ensure!(
                !i.scripted()
                    && owner(&mut i)?.saved.arrived
                    && story.has_seen(INTRO)
                    && p.feet == home
                    && p.script_motion == 0
                    && ((p.script_facing - owner(&mut i)?.data.yaw + std::f32::consts::PI)
                        .rem_euclid(std::f32::consts::TAU)
                        - std::f32::consts::PI)
                        .abs()
                        < 0.001,
                "Tower intro did not hand back control"
            );
            println!("PASS Tower intro {hz}Hz skip={skip:?}: same home, shell, Cat removal and restored clock");
        }
    }
    Ok(())
}
fn machinery(a: &mut Assets, map: &Bsp) -> Result<()> {
    let mut t = Tower::load(a, map)?;
    t.upgraded();
    ensure!(
        t.objects.len() == 12,
        "Missing tank machinery: {}",
        t.objects.len()
    );
    let mut w = World::from_bsp(map)?;
    w.set_dynamic(t.colliders());
    w.set_dynamic_liquids(t.liquids());
    let mut p =
        Player::spawn(&w, interaction::spawn(map, None).0).context("Blocked tower start")?;
    t.event("MoveWater3");
    ensure!(t.saved.stages == [None; 3], "Out-of-order flush accepted");
    ensure!(!t.exit_ready(), "Premature shaft exit");
    // Original rising-water saves keep their stage and do not replay the new introduction.
    t.restore(
        &serde_json::json!({"version":1,"stages":[8.,2.5,null]}),
        map,
    )?;
    ensure!(
        t.saved.arrived && !t.scripted() && (t.height() - 1008.).abs() < 0.01,
        "Legacy water state lost"
    );
    t = Tower::load(a, map)?;
    t.upgraded();
    for k in 0..3 {
        let o = t
            .objects
            .iter()
            .find(|o| o.name == format!("flusher0{}", k + 1))
            .unwrap();
        // The modeled tray is far below its authored origin; use actual collision support.
        let mut tray = None;
        for x in [-48., -32., 32., 48.] {
            for y in [-48., -32., 32., 48.] {
                let high = o.base + vec3(x, y, map.models[o.model].min.z + 180.);
                let low = high - Vec3::Z * 200.;
                let h = o.collider.trace(
                    high + crate::collision::PLAYER_CENTER,
                    low + crate::collision::PLAYER_CENTER,
                    crate::collision::PLAYER_HALF,
                );
                if !h.start_solid && h.fraction < 1. && h.normal.z >= 0.65 {
                    tray = Some(high.lerp(low, h.fraction));
                }
            }
        }
        let feet = tray.context("Flusher has no tray")?;
        p = Player::new(feet);
        p.grounded = true;
        t.stopped = true;
        w.set_dynamic(t.colliders());
        let start = p.feet;
        for _ in 0..360 {
            t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
        }
        ensure!(
            (t.machine().sink[k] - 64.).abs() < 0.01
                && (p.feet.z - start.z + 64.).abs() < 0.1
                && w.body_clear(p.feet),
            "Flusher {k} does not carry/sink during Watch: {:?} -> {:?}, depth {}",
            start,
            p.feet,
            t.machine().sink[k]
        );
        ensure!(t.machine().fans == [0.; 5], "Watch did not freeze fans");
        let saved = t.snapshot();
        let mut loaded = Tower::load(a, map)?;
        loaded.restore(&saved, map)?;
        loaded.stopped = true;
        let mut q = p.clone();
        let mut v = World::from_bsp(map)?;
        t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
        loaded.advance(FIXED_DT, map, &mut v, &mut q, &[])?;
        ensure!(
            t.snapshot() == loaded.snapshot() && p.feet == q.feet,
            "Saved sunk flusher changed"
        );
        p = Player::new(interaction::spawn(map, None).0);
        for _ in 0..120 * 8 {
            t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
        }
        ensure!(t.machine().sink[k] == 0., "Unloaded flusher did not return");
    }
    t.stopped = false;
    let mut stats = Stats::default();
    let mut swimmer = Player::new(t.data.bubbles[0] - crate::collision::PLAYER_CENTER);
    swimmer.breath.submerged = 12.;
    t.prepare_player(&mut stats, &mut swimmer);
    ensure!(
        swimmer.breath.remaining() == 20.,
        "Air bubbles do not refresh the shell's breath budget"
    );
    for (k, thread) in FLUSH.iter().enumerate() {
        t.event(thread);
        for _ in 0..120 * 8 {
            t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
        }
        let angle = t.machine().lids[k];
        let expected = if k == 2 { -35. } else { 35. };
        ensure!(
            (angle - expected).abs() < 2.1,
            "Lid {k} did not open: {angle}"
        );
        let water = t.height();
        let state = t.snapshot();
        t.event(thread);
        ensure!(t.snapshot() == state, "Flush repeated");
        for _ in 0..120 {
            t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
        }
        ensure!(
            t.machine().lids[k] != angle && t.height() == water,
            "Missing independent lid sway"
        );
        if k > 0 {
            ensure!(
                (t.machine().lids[k - 1] - 90.).abs() < 0.1,
                "Prior lid did not close upright"
            );
        }
    }
    ensure!(
        t.exit_ready(),
        "Completed water stages did not open dive exit"
    );
    let mut caught = false;
    // Find a clear approach swept by a real blade, rather than treating the fan's
    // entire bounding box as a damage volume.
    t = Tower::load(a, map)?;
    t.upgraded();
    w.set_dynamic(t.colliders());
    let o = t.objects.iter().find(|o| o.name == "fan01").unwrap();
    let ahead = Collider::model(
        map,
        o.model,
        o.base,
        Quat::from_rotation_x(8_f32.to_radians()),
        true,
    )?;
    'scan: for x in (-160..=160).step_by(16) {
        for y in (-192..=192).step_by(16) {
            for z in (-192..=192).step_by(16) {
                let feet = o.base + vec3(x as f32, y as f32, z as f32);
                let c = feet + crate::collision::PLAYER_CENTER;
                if w.body_clear(feet)
                    && ahead.trace(c, c, crate::collision::PLAYER_HALF).start_solid
                {
                    p = Player::new(feet);
                    caught = true;
                    break 'scan;
                }
            }
        }
    }
    ensure!(caught, "No clear blade approach found");
    let at = p.feet;
    for _ in 0..8 {
        t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
    }
    ensure!(
        t.machine().crush && w.body_clear(p.feet),
        "Spinning fan neither crushed nor safely stopped at {at:?}"
    );
    let mut stats = Stats::default();
    let mut story = Story::default();
    let never = |_: &crate::combat::Target| false;
    let f = t.combat(&mut crate::level::Combat {
        dt: FIXED_DT,
        world: &w,
        player: &mut p,
        stats: &mut stats,
        story: &mut story,
        notarget: false,
        summon: None,
        threatens: &never,
    });
    stats.damage(f.damage);
    ensure!(!stats.alive(), "Drain fan contact is not lethal");
    println!("PASS Tower: Watch-exempt weighted trays, saved carry, lid sway, ordered stages, shaft gate, real blade death and legacy water");
    Ok(())
}

pub(super) async fn render_check(assets: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(assets, "tower2")?;
    let mut t = Tower::load(assets, &scene.map)?;
    let water = t.objects.iter().find(|o| o.name == "water").unwrap().model;
    for stage in 0..=3 {
        t.saved.stages = std::array::from_fn(|i| (i < stage).then_some(8.));
        t.saved.machine.as_mut().unwrap().lids = std::array::from_fn(|i| {
            if i >= stage {
                0.
            } else if i + 1 < stage {
                90.
            } else if i == 2 {
                -35.
            } else {
                35.
            }
        });
        t.rebuild(&scene.map)?;
        let height = t.height();
        let eye = vec3(1776., 2400., height + 180.);
        let target = vec3(1300., 1350., height);
        let poses = t.transforms();
        let dry: Vec<_> = poses.iter().copied().filter(|p| p.0 != water).collect();
        let mut draw = |poses: &[(usize, Vec3, Quat)]| {
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: eye,
                target,
                up: Vec3::Z,
                z_near: 2.,
                z_far: 30000.,
                fovy: 75_f32.to_radians(),
                ..Default::default()
            });
            scene.draw(eye, 1., false, false, poses);
            crate::render::depth_read_only(|| scene.draw(eye, 1., false, true, poses));
        };
        draw(&dry);
        let before = crate::character::visibility_image();
        draw(&poses);
        let after = crate::character::visibility_image();
        let pixels = crate::character::visible_pixels(&before, &after);
        ensure!(
            pixels > 500,
            "Water is not visible at stage {stage}: {pixels}"
        );
        crate::viewer::save_capture(std::path::Path::new(&format!(
            "private/tower2-water-{stage}.png"
        )))?;
        println!("PASS visible tower2 water stage {stage}, surface {height}, {pixels} pixels");
        set_default_camera();
        next_frame().await;
    }
    let mut art = scene::Art::load(assets)?;
    t = Tower::load(assets, &scene.map)?;
    let arrival = Player::spawn(&scene.world, interaction::spawn(&scene.map, None).0)
        .context("Render arrival lacks support")?;
    t.saved.scene.as_mut().unwrap().clock.home = Some(Transform {
        translation: arrival.feet,
        rotation: Quat::from_rotation_z(t.data.yaw),
    });
    for time in [1., 3.5, 5.] {
        t.saved.scene.as_mut().unwrap().clock.time = time;
        let cam = t.camera(&scene.world).unwrap();
        clear_background(BLACK);
        set_camera(&Camera3D {
            position: cam.eye,
            target: cam.target,
            up: Vec3::Z,
            fovy: 75_f32.to_radians(),
            z_near: 2.,
            z_far: 20000.,
            ..Default::default()
        });
        scene.draw(cam.eye, time, false, false, &t.transforms());
        art.draw(&t, &scene.atmosphere, cam.eye, false);
        crate::render::depth_read_only(|| scene.draw(cam.eye, time, false, true, &t.transforms()));
        set_default_camera();
        crate::viewer::save_capture(std::path::Path::new(&format!(
            "private/tower2-work/intro-{time}.png"
        )))?;
        next_frame().await;
    }
    Ok(())
}

fn rim(map: &Bsp) -> Result<()> {
    let w = World::from_bsp(map)?;
    let mut p = Player::new(vec3(1590., 2176.96875, 896.03125));
    p.grounded = true;
    for n in 0..600 {
        p.tick(
            &w,
            Controls {
                wish: Vec2::Y,
                jump: n == 0,
                run: true,
                ..Default::default()
            },
        );
        if p.ledge.is_some() {
            p.tick(
                &w,
                Controls {
                    jump: true,
                    ..Default::default()
                },
            );
            break;
        }
    }
    ensure!(
        p.ledge.as_ref().is_some_and(|h| h.pulling),
        "Cannot grab the Tower's beveled pipe rim"
    );
    for n in 0..400 {
        p.tick(&w, Controls::default());
        ensure!(w.body_clear(p.feet), "Pipe pull-up entered collision");
        if n == 120 {
            p = serde_json::from_value(serde_json::to_value(&p)?)?;
            p.validate_world(&w)?;
        }
    }
    ensure!(
        p.grounded && p.feet.z >= 1024.,
        "Pipe rim did not land on the walkway"
    );
    println!(
        "PASS Tower beveled rim: real jump, two-hand catch, saved pull-up, supported landing {:?}",
        p.feet
    );
    Ok(())
}

fn currents(a: &mut Assets, map: &Bsp) -> Result<()> {
    let mut t = Tower::load(a, map)?;
    t.upgraded();
    let mut w = World::from_bsp(map)?;
    t.traversal(&mut w.traversal);
    let mut p = Player::new(vec3(2930., 400., -220.));
    p.swimming = true;
    p.breath.shell = true;
    for n in 0..240 {
        t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
        p.tick(
            &w,
            Controls {
                swim: Vec3::Y,
                run: true,
                ..Default::default()
            },
        );
        ensure!(
            w.body_clear(p.feet) && !t.machine().crush,
            "Current swept Alice into a fan"
        );
        if n == 80 {
            let snapshot = t.snapshot();
            let mut restored = Tower::load(a, map)?;
            restored.restore(&snapshot, map)?;
            let mut v = World::from_bsp(map)?;
            restored.traversal(&mut v.traversal);
            let mut q = p.clone();
            t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
            restored.advance(FIXED_DT, map, &mut v, &mut q, &[])?;
            ensure!(
                t.snapshot() == restored.snapshot()
                    && serde_json::to_value(&p)? == serde_json::to_value(&q)?,
                "Current cooldown changed after load"
            );
        }
    }
    ensure!(
        p.feet.y > 740.,
        "Native-wait current blocks the pipe crossing: {:?}",
        p.feet
    );
    println!(
        "PASS Tower current: 0.2 s pulse, saved cooldown, live fan crossing {:?}",
        p.feet
    );
    Ok(())
}
