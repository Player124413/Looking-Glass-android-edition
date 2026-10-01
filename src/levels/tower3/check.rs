use super::*;
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/tower3.bsp")?)?;
    let t = Tower::load(a, &map)?;
    ensure!(
        t.objects.len() == 49 && t.transforms().len() == 42 && t.colliders().len() == 41,
        "Missing Tower geometry"
    );
    ensure!(
        t.objects.iter().filter(|o| o.name == "gear01").count() == 2,
        "Duplicate gear lost"
    );
    arm_clearance(a, &map)?;
    for o in &t.objects {
        if let Some(m) = o.model {
            println!(
                "TOWER part {} {} bounds {:?} {:?}",
                o.id,
                o.name,
                o.base + map.models[m].min,
                o.base + map.models[m].max
            );
        }
    }
    for time in [0., 0.2, 2., 4., 10., 12., 15., 24., 32., 64.] {
        let mut t = Tower::load(a, &map)?;
        t.upgraded();
        t.saved.clocks.fill(time);
        t.rebuild();
        for o in &t.objects {
            if let Some(p) = o.parent {
                let parent = &t.objects[p];
                let expected =
                    parent.pose.translation + parent.pose.rotation * (o.base - parent.base);
                ensure!(
                    o.pose.translation.distance(expected) < 0.002,
                    "Broken bind {}",
                    o.id
                );
                if o.local_angles {
                    ensure!(
                        o.pose.rotation.angle_between(Quat::IDENTITY).abs() < 0.001,
                        "Pedal plate rotated off level"
                    );
                }
            }
        }
        let s = t.snapshot();
        let mut q = Tower::load(a, &map)?;
        q.restore(&s, &map)?;
        ensure!(
            s == q.snapshot() && t.transforms() == q.transforms(),
            "Tower saved pose differs"
        );
    }
    for hz in [30, 60, 144] {
        for skip in [None, Some(0.2), Some(3.)] {
            let mut r = crate::route::Route::new(a, "tower3", None)?;
            let home = r.player.feet;
            for n in 0..hz * 60 {
                if skip.is_some_and(|v| n as f32 / hz as f32 >= v)
                    && r.interactions.scene_id_after(None).is_some()
                {
                    r.interactions.skip_cinematic(
                        &r.map,
                        &mut r.world,
                        &mut r.player,
                        &mut r.story,
                    )?;
                }
                r.interactions.advance_school(
                    1. / hz as f32,
                    &r.map,
                    &mut r.world,
                    &mut r.player,
                )?;
                if r.interactions.prepare_story(&mut r.story) {
                    r.story.tick(1. / hz as f32, false);
                }
                r.interactions.sync_cinematic_story(&r.story);
                for name in r.story.take_completed() {
                    r.interactions.completed_dialogue(&name);
                }
                if !r.interactions.scripted() {
                    break;
                }
            }
            ensure!(
                !r.interactions.scripted()
                    && r.story.has_seen(INTRO)
                    && r.player.feet == home
                    && r.player.script_motion == 0,
                "Tower intro failed to release Alice"
            );
            println!("PASS Tower intro {hz}Hz skip {skip:?}: stable landing and completed speech");
        }
    }
    for room in 1..=3 {
        let mut r = crate::route::Route::new(a, "tower3", None)?;
        owner(&mut r.interactions)?.upgraded();
        owner(&mut r.interactions)?.event(&format!("Room{room}Tele"));
        for (n, id) in [33, 35, 426].into_iter().enumerate() {
            let info = TriggerInfo {
                id: crate::entity::Id(id),
                name: &format!("room{}_tele", n + 1),
                class: crate::level::TriggerClass::Teleport,
                thread: "",
                exit: None,
                target: None,
            };
            let t = owner(&mut r.interactions)?;
            ensure!(
                t.gate(&info).unwrap().test(&t.facts()) == (n + 1 == room),
                "Checkpoint overlap enabled"
            );
        }
        let s = owner(&mut r.interactions)?.snapshot();
        let mut t = Tower::load(a, &map)?;
        t.restore(&s, &map)?;
        ensure!(t.saved.room as usize == room, "Lost checkpoint");
        let mut bad = s.clone();
        bad["room"] = serde_json::json!(4);
        ensure!(
            t.restore(&bad, &map).is_err(),
            "Invalid checkpoint accepted"
        );
    }
    rider(a, &map)?;
    cycles(a, &map)?;
    recovery(a, &map)?;
    crush(a, &map)?;
    println!("PASS Tower bind poses, duplicate names, scene outcomes, saved phases and exclusive checkpoints");
    Ok(())
}
fn arm_clearance(a: &mut Assets, map: &Bsp) -> Result<()> {
    let mut t = Tower::load(a, map)?;
    // These are the shaft centres and rooms the arm incorrectly swept through.
    // Small core probes exclude the intentional shallow contact of meshing teeth.
    let shafts = [vec3(2368., 1120., 992.), vec3(1600., 544., 992.)];
    for step in 0..=24 * 120 {
        let time = step as f64 / 120.;
        let root = t.objects.iter().position(|o| o.id == 31).unwrap();
        for (i, pose) in motion::poses(&t.objects, root, time) {
            t.objects[i].set(pose);
        }
        let arm = t.objects.iter().find(|o| o.id == 31).unwrap();
        let wheel = t.objects.iter().find(|o| o.id == 138).unwrap();
        ensure!(wheel.pose.translation.y >= 1343.99,
            "Arm entered the southern pedal room at {time}s");
        ensure!((wheel.pose.translation.distance(arm.pose.translation) -
            wheel.base.distance(arm.base)).abs() < 0.002, "Arm attachment drifted");
        for point in shafts {
            for o in [arm, wheel] {
                ensure!(!o.collider.as_ref().unwrap().touches(point, point, Vec3::splat(4.)),
                    "Arm part {} intersects a lift shaft at {time}s", o.id);
            }
        }
        if step == 10 * 120 || step == 12 * 120 {
            ensure!(wheel.pose.translation.distance(vec3(1748.9519, 2456.596, 976.)) < 0.01,
                "Arm did not reach its authored northern endpoint");
            let right = wheel.pose.rotation * Vec3::X;
            let expected = vec3((-630_f32).to_radians().cos(), (-630_f32).to_radians().sin(), 0.);
            ensure!(right.distance(expected) < 0.001, "Bound wheel lost its counter-rotation");
        }
    }
    println!("PASS Tower arm: complete 24-second northern sweep, clear lift cores and counter-rotating attachment");
    Ok(())
}
fn cycles(a: &mut Assets, map: &Bsp) -> Result<()> {
    // Isolate two authored bind groups so a complete ride measures attachment
    // across both turnarounds, independently of nearby walls and dismount timing.
    for (id, group, seconds) in [(49, 51, 6), (138, 31, 24)] {
        let mut t = Tower::load(a, map)?;
        t.upgraded();
        for o in &mut t.objects {
            o.solid = o.id == id || o.id == group;
        }
        let o = t.objects.iter().find(|o| o.id == id).unwrap();
        let c = o.collider.as_ref().unwrap();
        let mut start = None;
        for x in [50., -50., 90., -90., 0.] {
            let from = o.pose.translation + vec3(x, 0., 120.);
            let to = from - Vec3::Z * 240.;
            let tr = c.trace(from + PLAYER_CENTER, to + PLAYER_CENTER, PLAYER_HALF);
            if !tr.start_solid && tr.fraction < 1. && tr.normal.z > 0.65 {
                start = Some(from.lerp(to, tr.fraction));
                break;
            }
        }
        let start = start.context("No full-cycle ride fixture")?;
        let mut p = Player::new(start);
        p.grounded = true;
        let mut w = World::fixture(&[]);
        w.set_dynamic(t.colliders());
        let mut farthest = 0_f32;
        for tick in 0..seconds * 120 {
            t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
            p.tick(&w, Controls::default());
            farthest = farthest.max(p.feet.distance(start));
            let wheel = t.objects.iter().find(|o| o.id == id).unwrap();
            let offset = p.feet - wheel.pose.translation;
            // Leaving the arm's 16-unit lip briefly goes airborne before landing
            // on the wheel. Stay above the deck and inside its inner rim.
            let on_arm_deck = id == 138 && offset.truncate().length() < 100. && offset.z >= 31.;
            if id == 138 {
                ensure!(
                    on_arm_deck,
                    "Arm ride drifted off its deck at tick {tick}: {offset:?}"
                );
            }
            ensure!(
                w.body_clear(p.feet) && t.saved.damage == 0. && (on_arm_deck || p.grounded || t.objects.iter().filter(|o|o.solid).filter_map(|o|o.collider.as_ref()).any(|c|motion::support(c,&p))),
                "Full cycle dropped rider {id} tick {tick}: start {start:?} feet {:?} velocity {:?} grounded {} clear {} damage {}",
                p.feet,p.velocity,p.grounded,w.body_clear(p.feet),t.saved.damage
            );
        }
        ensure!(
            farthest > 100. && p.feet.distance(start) < if id == 138 { 100. } else { 2. },
            "Full cycle detached rider {id}: {:?}",
            p.feet - start
        );
        println!("PASS Tower bound rider {id}: complete {seconds}-second cycle and return");
    }
    Ok(())
}
fn recovery(a: &mut Assets, map: &Bsp) -> Result<()> {
    for (id, room) in [(4, 1), (428, 2), (429, 2), (430, 3)] {
        let mut r = crate::route::Route::new(a, "tower3", None)?;
        owner(&mut r.interactions)?.upgraded();
        let point = interaction::vector(&map.entities[id]["origin"]).unwrap() - PLAYER_CENTER;
        r.interactions.triggers(FIXED_DT, point, point);
        ensure!(
            owner(&mut r.interactions)?.saved.room == room,
            "Checkpoint trigger {id} failed"
        );
        let entry = r
            .interactions
            .levels_recovery_entry(interaction::spawn(map, None));
        let before = serde_json::to_value(r.interactions.snapshot())?;
        let (mut p, _, nearby) = crate::recovery::Recovery::default()
            .restore(&r.world, &r.interactions, Vec3::ZERO, entry)
            .context("Tower checkpoint recovery blocked")?;
        let expected = entry.0 - Vec3::Z * crate::movement::EYE_HEIGHT;
        ensure!(
            !nearby && p.feet.distance(expected) < 0.01,
            "Recovery used feet as eye position"
        );
        for _ in 0..120 {
            p.tick(&r.world, Controls::default());
        }
        ensure!(
            p.grounded && r.world.body_clear(p.feet) && p.feet.distance(expected) < 20.,
            "Unsafe room {room} recovery"
        );
        ensure!(
            serde_json::to_value(r.interactions.snapshot())? == before,
            "Recovery reset machinery"
        );
    }
    println!("PASS Tower authored checkpoint triggers and production recovery at all three rooms");
    Ok(())
}
fn crush(a: &mut Assets, map: &Bsp) -> Result<()> {
    for blocked in [false, true] {
        let mut t = Tower::load(a, map)?;
        t.upgraded();
        for o in &mut t.objects {
            o.solid = o.id == 14;
        }
        let o = t.objects.iter().find(|o| o.id == 14).unwrap();
        let from = o.base + vec3(144., 60., 350.);
        let to = from - Vec3::Z * 240.;
        let hit = o.collider.as_ref().unwrap().trace(
            from + PLAYER_CENTER,
            to + PLAYER_CENTER,
            PLAYER_HALF,
        );
        ensure!(
            !hit.start_solid && hit.fraction < 1.,
            "Missing wheel crush fixture"
        );
        let start = from.lerp(to, hit.fraction);
        let mut p = Player::new(start);
        p.grounded = true;
        let ceiling = if blocked {
            vec![(
                start + vec3(-500., -500., 56.2),
                start + vec3(500., 500., 100.),
            )]
        } else {
            vec![]
        };
        let mut w = World::fixture(&ceiling);
        w.set_dynamic(t.colliders());
        for _ in 0..120 {
            t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
            if t.saved.damage > 0. {
                break;
            }
        }
        ensure!(
            (t.saved.damage == 1000.) == blocked,
            "Wheel blocked/free crush differs: {}",
            t.saved.damage
        );
        ensure!(w.body_clear(p.feet), "Wheel left Alice embedded");
        if !blocked {
            ensure!(
                p.feet.distance(start) > 10.,
                "Free wheel did not carry Alice"
            );
        }
    }
    println!("PASS Tower wheel free carry versus blocked crush without interpenetration");
    Ok(())
}
fn rider(a: &mut Assets, map: &Bsp) -> Result<()> {
    for (id, time) in [(48, 1.), (49, 0.), (139, 3.)] {
        let mut t = Tower::load(a, map)?;
        t.upgraded();
        t.saved.clocks.fill(time);
        t.rebuild();
        let o = t.objects.iter().find(|o| o.id == id).unwrap();
        let c = o.collider.as_ref().unwrap();
        let mut footing = None;
        for radius in [0., 50., 100., 130.] {
            for n in 0..16 {
                let angle = n as f32 * std::f32::consts::TAU / 16.;
                let from =
                    o.pose.translation + vec3(radius * angle.cos(), radius * angle.sin(), 120.);
                let to = from - Vec3::Z * 240.;
                let tr = c.trace(from + PLAYER_CENTER, to + PLAYER_CENTER, PLAYER_HALF);
                if !tr.start_solid && tr.fraction < 1. && tr.normal.z > 0.65 {
                    footing = Some(from.lerp(to, tr.fraction));
                    break;
                }
            }
            if footing.is_some() {
                break;
            }
        }
        let mut p = Player::new(footing.with_context(|| format!("No ride surface {id}"))?);
        p.grounded = true;
        let mut w = World::from_bsp(map)?;
        w.set_dynamic(t.colliders());
        ensure!(w.body_clear(p.feet), "Ride fixture blocked {id}");
        let mut q = p.clone();
        let mut other = Tower::load(a, map)?;
        other.restore(&t.snapshot(), map)?;
        let mut v = World::from_bsp(map)?;
        v.set_dynamic(other.colliders());
        let start = p.feet;
        for _ in 0..120 {
            t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
            other.advance(FIXED_DT, map, &mut v, &mut q, &[])?;
            p.tick(&w, Controls::default());
            q.tick(&v, Controls::default());
            ensure!(
                p.feet == q.feet && t.snapshot() == other.snapshot(),
                "Tower rider continuation diverged {id}"
            );
            ensure!(
                w.body_clear(p.feet) && t.saved.damage == 0.,
                "Tower ride trapped player {id}"
            );
        }
        ensure!(
            p.feet.distance(start) > 20.,
            "Tower did not carry rider {id}"
        );
        let state = t.snapshot();
        t.advance(0., map, &mut w, &mut p, &[])?;
        ensure!(t.snapshot() == state, "Paused machine moved");
        println!("PASS Tower rider {id}: carried, clear, exact restored future and pause");
    }
    Ok(())
}
pub(super) async fn render(a: &mut Assets) -> Result<()> {
    std::fs::create_dir_all("private/tower3-work")?;
    let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_TOWER3_FROM") {
        crate::route::Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?
    } else {
        crate::route::Route::new(a, "tower3", None)?
    };
    let time = std::env::var("LOOKING_GLASS_TOWER3_TIME")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(3.);
    if std::env::var_os("LOOKING_GLASS_TOWER3_FROM").is_none() {
        let t = owner(&mut r.interactions)?;
        t.saved.clocks.fill(time);
        t.saved.scene.as_mut().unwrap().clock.time = time as f32;
        t.saved.scene.as_mut().unwrap().clock.home = Some(Transform {
            translation: r.player.feet,
            rotation: Quat::IDENTITY,
        });
        t.rebuild();
    }
    let mut scene = crate::render::Scene::load(a, "tower3")?;
    let mut art = scene::Art::load(a)?;
    r.interactions.presentation.apply(&mut scene);
    let eye = std::env::var("LOOKING_GLASS_TOWER3_EYE")
        .ok()
        .and_then(|s| interaction::vector(&s));
    let target = std::env::var("LOOKING_GLASS_TOWER3_LOOK")
        .ok()
        .and_then(|s| interaction::vector(&s));
    let cam = r.interactions.camera_after(None, &r.world);
    let eye = eye.unwrap_or_else(|| cam.as_ref().map_or(r.player.eye(), |c| c.eye));
    let target = target.unwrap_or_else(|| cam.as_ref().map_or(eye + Vec3::Y * 300., |c| c.target));
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
        scene.draw(eye, time as f32, false, false, &r.interactions.transforms());
        art.draw(owner(&mut r.interactions)?, &scene.atmosphere, eye, false);
        crate::render::depth_read_only(|| {
            scene.draw(eye, time as f32, false, true, &r.interactions.transforms())
        });
        set_default_camera();
        if frame == 2 {
            crate::viewer::save_capture(std::path::Path::new("private/tower3-work/view.png"))?;
        }
        next_frame().await;
    }
    Ok(())
}
fn stage(i: &mut Interactions, time: f64, room: u8) -> Result<()> {
    let t = owner(i)?;
    t.upgraded();
    t.saved.clocks.fill(time);
    t.saved.room = room;
    t.rebuild();
    Ok(())
}
pub(super) const SAVES: &[super::super::SaveCase] = &[
    super::super::SaveCase {
        name: "tower3-lift",
        visit: "tower3$first",
        stage: Some(|i, _| stage(i, 2., 1)),
        behavior: None,
    },
    super::super::SaveCase {
        name: "tower3-pedal",
        visit: "tower3$first",
        stage: Some(|i, _| stage(i, 1.5, 2)),
        behavior: None,
    },
    super::super::SaveCase {
        name: "tower3-room3",
        visit: "tower3$first",
        stage: Some(|i, _| stage(i, 20., 3)),
        behavior: None,
    },
    super::super::SaveCase {
        name: "tower3-intro",
        visit: "tower3$first",
        stage: Some(|i, _| {
            let t = owner(i)?;
            t.saved.scene.as_mut().unwrap().clock.time = 3.;
            Ok(())
        }),
        behavior: None,
    },
];
