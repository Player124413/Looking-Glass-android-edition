use super::*;
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/wchess2.bsp")?)?;
    let r = Castle::load(a, &map)?;
    let mut restored = Castle::load(a, &map)?;
    restored.restore(&r.snapshot(), &map)?;
    ensure!(
        r.snapshot() == restored.snapshot(),
        "Castling initial state mismatch"
    );
    println!(
        "PASS Castling load: {} actors, {} doors, pawn path {}s",
        r.saved.cast.len(),
        r.saved.doors.len(),
        r.data.pawn_duration
    );
    tests(a, &map)?;
    Ok(())
}
pub(super) fn route(a: &mut Assets) -> Result<()> {
    route_inner(a, false)
}
pub(super) fn route_skip(a: &mut Assets) -> Result<()> {
    route_inner(a, true)
}
pub(crate) fn drive(r: &mut crate::route::Route) -> Result<()> {
    std::fs::create_dir_all("private/wchess2")?;
    r.tactics = true;
    let goals: Vec<Vec3> = if let Some(path) = std::env::var_os("LOOKING_GLASS_WCHESS2_PATH") {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else {
        COURSE.iter().map(|p| vec3(p[0], p[1], p[2])).collect()
    };
    for (k, goal) in goals.iter().enumerate() {
        println!("Castling route {} {:?} -> {goal:?}", k, r.player.feet);
        let result = walk_to(r, *goal);
        let checkpoint = serde_json::to_vec(&r.checkpoint())?;
        std::fs::write(
            format!("private/wchess2/route-{k}.json"),
            checkpoint,
        )?;
        result?;
        r.wait_for_cinematic()?;
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(owner(&mut r.interactions)?.saved.king, "King not completed");
    ensure!(
        r.transition == Some(EXIT.destination()),
        "Portal not entered at {:?}",
        r.player.feet
    );
    println!(
        "Castling completion ticks={} sanity={} will={} teleports={} jumps={}",
        r.ticks,
        r.stats.sanity(),
        r.stats.will(),
        r.teleports,
        r.jumps
    );
    Ok(())
}
fn route_inner(a: &mut Assets, skip: bool) -> Result<()> {
    let mut r = if let Some(path) = std::env::var_os("LOOKING_GLASS_WCHESS2_RESUME") {
        crate::route::Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?
    } else {
        crate::route::Route::new(a, "wchess2", Some("wchess2_start1"))?
    };
    r.skip_cinematics = skip;
    drive(&mut r)?;
    let stats = serde_json::to_value(&r.stats)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.stats.alive() && serde_json::to_value(&next.stats)? == stats,
        "Arrival lost resources"
    );
    println!(
        "PASS Castling Normal traversal, skip={skip}; Checkmate entrance {:?}",
        next.player.feet
    );
    Ok(())
}
fn walk_to(r: &mut crate::route::Route, goal: Vec3) -> Result<()> {
    for t in 0..2400 {
        if r.transition.is_some() {
            return Ok(());
        }
        if r.interactions.scripted() {
            r.wait_for_cinematic()?;
            return Ok(());
        }
        let d = goal - r.player.feet;
        if d.truncate().length() < 22. && d.z.abs() < 102. && r.player.grounded {
            return Ok(());
        }
        r.tick(Controls {
            wish: d.truncate().normalize_or_zero(),
            run: d.truncate().length() > 28.,
            jump: t == 480 || t == 960,
            ..Default::default()
        })?;
        ensure!(r.stats.alive(), "Castling died at {:?}", r.player.feet);
    }
    anyhow::bail!(
        "Castling walk blocked at {:?} heading to {goal:?}",
        r.player.feet
    )
}

fn tick(
    i: &mut Interactions,
    map: &Bsp,
    w: &mut World,
    p: &mut Player,
    s: &mut Story,
    dt: f32,
) -> Result<()> {
    i.advance_school(dt, map, w, p)?;
    if i.prepare_story(s) {
        s.tick(dt, false);
    }
    i.sync_cinematic_story(s);
    for n in s.take_completed() {
        i.completed_dialogue(&n);
    }
    i.sync(w);
    Ok(())
}
fn setup(a: &mut Assets, map: &Bsp) -> Result<(Interactions, World, Player, Story)> {
    let mut i = Interactions::load(map)?;
    i.set_entry(a, map, "wchess2", Some("wchess2_start1"))?;
    let mut w = World::from_bsp(map)?;
    i.sync(&mut w);
    let p = Player::spawn(&w, vec3(-1024., -4096., -64.)).context("Queen approach has no floor")?;
    Ok((i, w, p, Story::load(a, "wchess2")))
}
fn tests(a: &mut Assets, map: &Bsp) -> Result<()> {
    use crate::movement::FIXED_DT;
    let hints = crate::cheshire::Hints::load(a, map, "wchess2")?;
    for hz in [30, 60, 120] {
        for skip in [false, true] {
            let (mut i, mut w, mut p, mut story) = setup(a, map)?;
            let mut checked = false;
            let mut seen = BTreeSet::new();
            ensure!(
                i.triggers(FIXED_DT, vec3(-1024., -64., 544.), vec3(-1024., -64., 544.))
                    .transition
                    .is_none(),
                "Premature initial exit"
            );
            // Cross the Queen strip through production movement, from a staged approach.
            for _ in 0..180 {
                let from = p.feet;
                p.tick(
                    &w,
                    Controls {
                        wish: Vec2::Y,
                        ..Default::default()
                    },
                );
                i.triggers(FIXED_DT, from, p.feet);
                if i.scripted() {
                    break;
                }
            }
            ensure!(i.scripted(), "Queen trigger did not activate");
            for phase in [scene::Kind::Queen, scene::Kind::King] {
                if phase == scene::Kind::King {
                    p = Player::spawn(&w, vec3(-1024., 1600., 192.))
                        .context("King approach floor missing")?;
                    for _ in 0..240 {
                        let from = p.feet;
                        p.tick(
                            &w,
                            Controls {
                                wish: Vec2::Y,
                                ..Default::default()
                            },
                        );
                        i.triggers(FIXED_DT, from, p.feet);
                        if i.scripted() {
                            break;
                        }
                    }
                    ensure!(i.scripted(), "King trigger did not activate");
                }
                for step in 0..(hz * 180) {
                    if skip && step == hz {
                        i.skip_cinematic(map, &mut w, &mut p, &mut story)?;
                    }
                    tick(&mut i, map, &mut w, &mut p, &mut story, 1. / hz as f32)?;
                    if let Some(v) = story.progress(TALK) {
                        seen.insert(v.0);
                    }
                    if !skip
                        && phase == scene::Kind::King
                        && !checked
                        && story.progress(TALK).is_some_and(|(n, t)| n == 3 && t > 0.2)
                    {
                        let saved = i.snapshot();
                        let saved_story = story.snapshot();
                        let old = owner(&mut i)?.snapshot();
                        for _ in 0..20 {
                            tick(&mut i, map, &mut w, &mut p, &mut story, 0.)?;
                        }
                        ensure!(old == owner(&mut i)?.snapshot(), "Pause advanced Castling");
                        let (mut j, mut v, _, mut other) = setup(a, map)?;
                        j.restore(&saved, map)?;
                        other.restore(&saved_story, &hints)?;
                        let mut q = p.clone();
                        j.sync(&mut v);
                        for _ in 0..120 {
                            tick(&mut i, map, &mut w, &mut p, &mut story, FIXED_DT)?;
                            tick(&mut j, map, &mut v, &mut q, &mut other, FIXED_DT)?;
                        }
                        ensure!(
                            owner(&mut i)?.snapshot() == owner(&mut j)?.snapshot()
                                && p.feet == q.feet,
                            "Restored royal future differs"
                        );
                        checked = true;
                    }
                    if !i.scripted() {
                        break;
                    }
                }
                ensure!(
                    !i.scripted() && w.body_clear(p.feet),
                    "Royal scene failed to return safe control at {:?}",
                    p.feet
                );
                ensure!(owner(&mut i)?.saved.queen, "Queen not committed");
            }
            let r = owner(&mut i)?;
            ensure!(r.saved.king, "King incomplete");
            if !skip {
                ensure!(
                    seen.len() == 9 && story.has_seen(LAST),
                    "King dialogue missing"
                );
            }
            // A stationary rider must follow the physical slab through a complete bob cycle.
            let king_handoff = p.clone();
            p = Player::new(vec3(-1060., -64., r.portal_pose().translation.z + 64.03125));
            p.grounded = true;
            r.rebuild(map)?;
            w.set_dynamic(r.colliders());
            ensure!(w.body_clear(p.feet), "Portal rider starts obstructed");
            let mut low = p.feet.z;
            let mut high = low;
            for _ in 0..1440 {
                r.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
                p.tick(&w, Controls::default());
                let offset = p.feet.z - r.portal_pose().translation.z;
                ensure!(
                    (offset - 64.03125).abs() < 0.2,
                    "Portal rider lost support: {offset}"
                );
                low = low.min(p.feet.z);
                high = high.max(p.feet.z);
            }
            ensure!(high - low > 25., "Portal did not move");
            p = king_handoff;
            let before = r.snapshot();
            r.event("cinema_king_thread");
            r.event("cinema_queen_abduction_thread");
            ensure!(r.snapshot() == before, "Royal replay");
            let d = EXIT.destination();
            ensure!(
                r.exit_contact(&d).unwrap().transition == Some(d.clone()),
                "Committed portal failed"
            );
            ensure!(
                r.exit_contact(&d).unwrap().transition.is_none(),
                "Duplicate portal delivery"
            );
            r.transition_failed(&d);
            r.saved.exit.advance(1.);
            ensure!(
                r.update(&mut w, &p, Vec3::X, false).transition.is_some(),
                "Portal retry lost"
            );
            let snap = r.snapshot();
            r.restore(&snap, map)?;
            ensure!(
                r.update(&mut w, &p, Vec3::X, false).transition.is_some(),
                "Reload lost committed exit"
            );
            println!(
                "PASS Castling royal scenes {hz}Hz skip={skip} at {:?}",
                p.feet
            );
        }
    }
    let mut r = Castle::load(a, map)?;
    ensure!(
        !r.objects
            .iter()
            .filter(|o| o.id == 40 || o.id == 41)
            .any(|o| o.locked),
        "4096 mistaken for lock"
    );
    for group in ["1a", "1b", "2a", "3a"] {
        r.cast_event(&format!("enemy_group{group}_thread"));
    }
    ensure!(
        !r.saved.fired.contains("enemy_group1b_thread"),
        "Alternative spawn duplicated"
    );
    r.cast_event("start_battle_group2_thread");
    ensure!(
        !r.group_ready("3b") && !r.group_ready("4a"),
        "Incorrect spawn gates"
    );
    let good = r.snapshot();
    let mut bad = good.clone();
    bad["king"] = serde_json::json!(true);
    ensure!(
        r.restore(&bad, map).is_err() && r.snapshot() == good,
        "Rejected save mutated live owner"
    );
    for consumed in [false, true] {
        let mut old = Interactions::load(map)?;
        if consumed {
            for at in [
                vec3(-1024., -3904., -64.),
                vec3(-1024., 1760., 320.),
                vec3(-1024., -64., 544.),
            ] {
                old.triggers(FIXED_DT, at, at);
            }
        }
        let (mut i, mut w, mut p, mut story) = setup(a, map)?;
        i.restore(&old.snapshot(), map)?;
        i.sync(&mut w);
        ensure!(
            !owner(&mut i)?.saved.king && !owner(&mut i)?.saved.queen,
            "Legacy save bypassed royal scenes"
        );
        ensure!(
            i.triggers(FIXED_DT, vec3(-1024., -64., 544.), vec3(-1024., -64., 544.))
                .transition
                .is_none(),
            "Legacy exit remains open"
        );
        for _ in 0..180 {
            let from = p.feet;
            p.tick(
                &w,
                Controls {
                    wish: Vec2::Y,
                    ..Default::default()
                },
            );
            i.triggers(FIXED_DT, from, p.feet);
            if i.scripted() {
                break;
            }
        }
        ensure!(i.scripted(), "Consumed Queen trigger was not migrated");
        tick(&mut i, map, &mut w, &mut p, &mut story, FIXED_DT)?;
    }
    let (mut i, mut w, mut p, mut story) = setup(a, map)?;
    let r = owner(&mut i)?;
    r.saved.queen = true;
    r.saved.pending = Some(scene::Kind::King);
    r.start_scene(&p);
    r.finish_scene(map, &mut w, &mut p, true)?;
    r.saved.age += 6.;
    r.rebuild(map)?;
    i.sync(&mut w);
    tick(&mut i, map, &mut w, &mut p, &mut story, 0.)?;
    let portal = owner(&mut i)?.portal_pose();
    let offset = portal.translation.z - 448.;
    ensure!(offset < -16., "Portal fixture has not descended");
    let old = vec3(-1024., -64., 668.);
    ensure!(
        i.triggers(FIXED_DT, old, old).transition.is_none(),
        "Editor exit remained after portal moved"
    );
    let current = vec3(-1024., -64., 668. + offset);
    ensure!(
        i.triggers(FIXED_DT, current, current).transition == Some(EXIT.destination()),
        "Bound trigger did not follow portal"
    );
    let _ = &mut story;
    println!("PASS Castling spawn alternatives, legacy migration, bound trigger and atomic save validation");
    Ok(())
}

pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let mut world = crate::render::Scene::load(a, "wchess2")?;
        let mut r = Castle::load(a, &world.map)?;
        let mut art = art::Art::load(a, &r)?;
        let mut p = Player::new(r.data.point("alice_queen_dest1").translation);
        for (name, kind, t, phase, line) in [
            (
                "queen-start",
                scene::Kind::Queen,
                2.,
                scene::Phase::Approach,
                0,
            ),
            (
                "queen-platform",
                scene::Kind::Queen,
                14.,
                scene::Phase::Approach,
                0,
            ),
            (
                "king-approach",
                scene::Kind::King,
                4.,
                scene::Phase::Approach,
                0,
            ),
            ("king-talk", scene::Kind::King, 20., scene::Phase::Talk, 0),
            ("alice-talk", scene::Kind::King, 25., scene::Phase::Talk, 1),
            ("pawn", scene::Kind::King, 60., scene::Phase::Pawn, 0),
            ("grab", scene::Kind::King, 65., scene::Phase::Grab, 0),
        ] {
            r.saved.scene = None;
            r.saved.pending = Some(kind);
            r.saved.queen = kind == scene::Kind::King;
            r.start_scene(&p);
            if kind == scene::Kind::Queen {
                let mut w = World::from_bsp(&world.map)?;
                for _ in 0..(t / crate::movement::FIXED_DT) as usize {
                    r.advance(crate::movement::FIXED_DT, &world.map, &mut w, &mut p, &[])?;
                }
                let c = r.scene_camera().unwrap();
                capture(&mut world, &mut art, &r, c, name).await?;
                continue;
            }
            let s = r.saved.scene.as_mut().unwrap();
            s.time = t;
            s.phase = phase;
            s.start = t - 0.7;
            s.shot_start = t - 0.7;
            s.line = line;
            s.king_camera = Some(18.);
            s.alice_camera = Some(24.);
            s.pawn_camera = Some(58.);
            s.shot = if line == 1 { 9 } else { 8 };
            r.rebuild(&world.map)?;
            let c = r.scene_camera().context("Missing camera")?;
            capture(&mut world, &mut art, &r, c, name).await?;
        }
        r.finish_scene(&world.map, &mut World::from_bsp(&world.map)?, &mut p, true)?;
        r.saved.age += 4.;
        r.rebuild(&world.map)?;
        capture(
            &mut world,
            &mut art,
            &r,
            crate::cinematic::Camera::look(vec3(-1024., -500., 650.), vec3(-1024., -64., 500.)),
            "portal",
        )
        .await?;
        println!("PASS eight Castling render samples");
        Ok(())
    })
}
async fn capture(
    scene: &mut crate::render::Scene,
    art: &mut art::Art,
    r: &Castle,
    c: crate::cinematic::Camera,
    name: &str,
) -> Result<()> {
    for frame in 0..3 {
        clear_background(BLACK);
        let view = Camera3D {
            position: c.eye,
            target: c.target,
            up: c.up,
            fovy: 75_f32.to_radians(),
            z_near: 2.,
            z_far: 20000.,
            ..Default::default()
        };
        set_camera(&view);
        crate::render_fx::begin_view(&view, r.saved.age, &scene.atmosphere, false);
        scene.draw(c.eye, r.saved.age, false, false, &r.transforms());
        art.draw(r, &scene.atmosphere, c.eye, false);
        crate::render::depth_read_only(|| {
            scene.draw(c.eye, r.saved.age, false, true, &r.transforms())
        });
        art.effects(r, c.eye, &scene.atmosphere);
        crate::render_fx::finish();
        set_default_camera();
        if frame == 2 {
            crate::viewer::save_capture(std::path::Path::new(&format!(
                "C:/DEV/McGee/private/wchess2/{name}.png"
            )))?;
        }
        next_frame().await;
    }
    Ok(())
}
pub(super) fn survey(a: &mut Assets) -> Result<()> {
    use std::io::Write;
    let map = Bsp::parse(&a.read("maps/wchess2.bsp")?)?;
    let w = World::from_bsp(&map)?;
    let mut file = std::io::BufWriter::new(std::fs::File::create(
        "C:/DEV/McGee/private/wchess2/floors.csv",
    )?);
    writeln!(file, "x,y,z")?;
    let mut count = 0;
    for x in (-2496..=640).step_by(32) {
        for y in (-5344..=2400).step_by(32) {
            let mut heights = BTreeSet::new();
            for z in (-160..=864).step_by(64) {
                let p = vec3(x as f32, y as f32, z as f32 + 0.1);
                if !w.body_clear(p) {
                    continue;
                }
                let t = w.body_trace(p, p - Vec3::Z * 128.);
                if !t.start_solid && t.fraction < 1. && t.normal.z > 0.65 {
                    heights.insert(((p.z - 128. * t.fraction) * 32.).round() as i32);
                }
            }
            for z in heights {
                writeln!(file, "{x},{y},{}", z as f32 / 32.)?;
                count += 1;
            }
        }
    }
    println!("Survey: {count} supported body positions");
    Ok(())
}

const COURSE: &[[f32; 3]] = &[
    [160.0_f32, -4800.0_f32, 0.03125_f32],
    [160.0_f32, -4896.0_f32, 0.03125_f32],
    [-128.0_f32, -4896.0_f32, 0.03125_f32],
    [-160.0_f32, -4896.0_f32, -8.46875_f32],
    [-256.0_f32, -4896.0_f32, -56.46875_f32],
    [-288.0_f32, -4896.0_f32, -63.96875_f32],
    [-320.0_f32, -4896.0_f32, -63.96875_f32],
    [-320.0_f32, -4608.0_f32, -63.96875_f32],
    [-352.0_f32, -4608.0_f32, -63.96875_f32],
    [-352.0_f32, -4576.0_f32, -63.96875_f32],
    [-384.0_f32, -4576.0_f32, -63.96875_f32],
    [-384.0_f32, -4352.0_f32, -63.96875_f32],
    [-608.0_f32, -4352.0_f32, -63.96875_f32],
    [-608.0_f32, -4320.0_f32, -63.96875_f32],
    [-640.0_f32, -4320.0_f32, -63.96875_f32],
    [-640.0_f32, -4288.0_f32, -63.96875_f32],
    [-672.0_f32, -4288.0_f32, -63.96875_f32],
    [-672.0_f32, -4256.0_f32, -63.96875_f32],
    [-704.0_f32, -4256.0_f32, -63.96875_f32],
    [-704.0_f32, -4224.0_f32, -63.96875_f32],
    [-736.0_f32, -4224.0_f32, -63.96875_f32],
    [-736.0_f32, -4192.0_f32, -63.96875_f32],
    [-768.0_f32, -4192.0_f32, -63.96875_f32],
    [-768.0_f32, -4160.0_f32, -63.96875_f32],
    [-832.0_f32, -4160.0_f32, -63.96875_f32],
    [-832.0_f32, -4096.0_f32, -63.96875_f32],
    [-864.0_f32, -4096.0_f32, -63.96875_f32],
    [-864.0_f32, -4064.0_f32, -63.96875_f32],
    [-896.0_f32, -4064.0_f32, -63.96875_f32],
    [-896.0_f32, -4032.0_f32, -63.96875_f32],
    [-928.0_f32, -4032.0_f32, -63.96875_f32],
    [-928.0_f32, -4000.0_f32, -63.96875_f32],
    [-960.0_f32, -4000.0_f32, -63.96875_f32],
    [-960.0_f32, -3968.0_f32, -63.96875_f32],
    [-992.0_f32, -3968.0_f32, -63.96875_f32],
    [-992.0_f32, -3936.0_f32, -63.96875_f32],
    [-1024.0_f32, -3936.0_f32, -63.96875_f32],
    [-1024.0_f32, -3904.0_f32, -63.96875_f32],
    [-1024.0_f32, -3616.0_f32, -63.96875_f32],
    [-1024.0_f32, -3584.0_f32, -56.46875_f32],
    [-1024.0_f32, -3488.0_f32, -8.46875_f32],
    [-1024.0_f32, -3456.0_f32, 0.03125_f32],
    [-1024.0_f32, -2624.0_f32, 0.03125_f32],
    [-1024.0_f32, -2368.0_f32, 0.03125_f32],
    [-1024.0_f32, -2336.0_f32, 4.3125_f32],
    [-1024.0_f32, -2304.0_f32, 13.46875_f32],
    [-1024.0_f32, -2272.0_f32, 22.59375_f32],
    [-1024.0_f32, -2240.0_f32, 31.75_f32],
    [-1024.0_f32, -2208.0_f32, 40.875_f32],
    [-1024.0_f32, -2144.0_f32, 59.1875_f32],
    [-1024.0_f32, -2112.0_f32, 64.03125_f32],
    [-1024.0_f32, -1824.0_f32, 64.03125_f32],
    [-800.0_f32, -1824.0_f32, 64.03125_f32],
    [-768.0_f32, -1824.0_f32, 73.25_f32],
    [-736.0_f32, -1824.0_f32, 88.71875_f32],
    [-704.0_f32, -1824.0_f32, 104.59375_f32],
    [-672.0_f32, -1824.0_f32, 120.3125_f32],
    [-672.0_f32, -1792.0_f32, 129.21875_f32],
    [-640.0_f32, -1792.0_f32, 143.84375_f32],
    [-640.0_f32, -1760.0_f32, 155.0625_f32],
    [-608.0_f32, -1760.0_f32, 163.625_f32],
    [-608.0_f32, -1728.0_f32, 175.375_f32],
    [-608.0_f32, -1696.0_f32, 186.34375_f32],
    [-608.0_f32, -1664.0_f32, 192.15625_f32],
    [-608.0_f32, -1632.0_f32, 192.03125_f32],
    [-608.0_f32, -1568.0_f32, 192.03125_f32],
    [-480.0_f32, -1568.0_f32, 192.03125_f32],
    [-480.0_f32, -1344.0_f32, 192.03125_f32],
    [-256.0_f32, -1344.0_f32, 192.03125_f32],
    [-256.0_f32, -1312.0_f32, 192.03125_f32],
    [-128.0_f32, -1312.0_f32, 192.03125_f32],
    [-128.0_f32, -1184.0_f32, 192.03125_f32],
    [-128.0_f32, -1152.0_f32, 186.8125_f32],
    [-128.0_f32, -1088.0_f32, 167.125_f32],
    [-128.0_f32, -1056.0_f32, 157.25_f32],
    [-128.0_f32, -768.0_f32, 68.65625_f32],
    [-128.0_f32, -736.0_f32, 64.03125_f32],
    [-128.0_f32, -640.0_f32, 64.03125_f32],
    [-224.0_f32, -640.0_f32, 64.03125_f32],
    [-224.0_f32, -608.0_f32, 64.03125_f32],
    [-288.0_f32, -608.0_f32, 64.03125_f32],
    [-288.0_f32, -576.0_f32, 64.03125_f32],
    [-288.0_f32, -544.0_f32, 64.03125_f32],
    [-384.0_f32, -544.0_f32, 64.03125_f32],
    [-384.0_f32, -512.0_f32, 64.03125_f32],
    [-672.0_f32, -512.0_f32, 64.03125_f32],
    [-704.0_f32, -512.0_f32, 72.21875_f32],
    [-736.0_f32, -512.0_f32, 89.6875_f32],
    [-768.0_f32, -512.0_f32, 107.125_f32],
    [-800.0_f32, -512.0_f32, 124.59375_f32],
    [-832.0_f32, -512.0_f32, 142.03125_f32],
    [-864.0_f32, -512.0_f32, 159.5_f32],
    [-896.0_f32, -512.0_f32, 160.03125_f32],
    [-928.0_f32, -512.0_f32, 160.03125_f32],
    [-928.0_f32, -256.0_f32, 160.03125_f32],
    [-928.0_f32, -224.0_f32, 167.53125_f32],
    [-928.0_f32, -64.0_f32, 247.53125_f32],
    [-960.0_f32, -64.0_f32, 247.53125_f32],
    [-960.0_f32, 0.0_f32, 279.53125_f32],
    [-960.0_f32, 32.0_f32, 288.03125_f32],
    [-960.0_f32, 832.0_f32, 288.03125_f32],
    [-960.0_f32, 864.0_f32, 279.53125_f32],
    [-960.0_f32, 1024.0_f32, 199.53125_f32],
    [-960.0_f32, 1056.0_f32, 192.03125_f32],
    [-960.0_f32, 1120.0_f32, 192.03125_f32],
    [-992.0_f32, 1120.0_f32, 192.03125_f32],
    [-992.0_f32, 1152.0_f32, 192.03125_f32],
    [-1024.0_f32, 1152.0_f32, 192.03125_f32],
    [-1024.0_f32, 1184.0_f32, 192.03125_f32],
    [-1024_f32, 1248_f32, 192_f32],
    [-1024.0_f32, 1056.0_f32, 192.03125_f32],
    [-1024.0_f32, 1024.0_f32, 199.53125_f32],
    [-1024.0_f32, 864.0_f32, 279.53125_f32],
    [-1024.0_f32, 832.0_f32, 288.03125_f32],
    [-1024.0_f32, 128.0_f32, 288.03125_f32],
    [-1056.0_f32, 128.0_f32, 288.03125_f32],
    [-1056.0_f32, 96.0_f32, 288.03125_f32],
    [-1088.0_f32, 96.0_f32, 288.03125_f32],
    [-1088.0_f32, 32.0_f32, 288.03125_f32],
    [-1088.0_f32, 0.0_f32, 279.53125_f32],
    [-1088.0_f32, -64.0_f32, 247.53125_f32],
    [-1120.0_f32, -64.0_f32, 247.53125_f32],
    [-1120.0_f32, -224.0_f32, 167.53125_f32],
    [-1120.0_f32, -256.0_f32, 160.03125_f32],
    [-1120.0_f32, -512.0_f32, 160.03125_f32],
    [-1152.0_f32, -512.0_f32, 160.03125_f32],
    [-1184.0_f32, -512.0_f32, 151.53125_f32],
    [-1344.0_f32, -512.0_f32, 71.53125_f32],
    [-1376.0_f32, -512.0_f32, 64.03125_f32],
    [-1664.0_f32, -512.0_f32, 64.03125_f32],
    [-1664.0_f32, -544.0_f32, 64.03125_f32],
    [-1728.0_f32, -544.0_f32, 64.03125_f32],
    [-1728.0_f32, -576.0_f32, 64.03125_f32],
    [-1760.0_f32, -576.0_f32, 64.03125_f32],
    [-1760.0_f32, -608.0_f32, 64.03125_f32],
    [-1824.0_f32, -608.0_f32, 64.03125_f32],
    [-1824.0_f32, -640.0_f32, 64.03125_f32],
    [-1920.0_f32, -640.0_f32, 64.03125_f32],
    [-1920.0_f32, -736.0_f32, 64.03125_f32],
    [-1920.0_f32, -768.0_f32, 68.65625_f32],
    [-1920.0_f32, -1056.0_f32, 157.25_f32],
    [-1920.0_f32, -1088.0_f32, 167.125_f32],
    [-1920.0_f32, -1152.0_f32, 186.8125_f32],
    [-1920.0_f32, -1184.0_f32, 192.03125_f32],
    [-1920.0_f32, -1312.0_f32, 192.03125_f32],
    [-1792.0_f32, -1312.0_f32, 192.03125_f32],
    [-1792.0_f32, -1344.0_f32, 192.03125_f32],
    [-1664.0_f32, -1344.0_f32, 192.03125_f32],
    [-1664.0_f32, -1664.0_f32, 192.03125_f32],
    [-1696.0_f32, -1664.0_f32, 192.03125_f32],
    [-1696.0_f32, -1760.0_f32, 192.03125_f32],
    [-1728.0_f32, -1760.0_f32, 192.03125_f32],
    [-1728.0_f32, -1792.0_f32, 192.03125_f32],
    [-1952.0_f32, -1792.0_f32, 192.03125_f32],
    [-1952.0_f32, -2208.0_f32, 192.03125_f32],
    [-1952.0_f32, -2240.0_f32, 200.75_f32],
    [-1952.0_f32, -2272.0_f32, 215.3125_f32],
    [-1952.0_f32, -2304.0_f32, 229.84375_f32],
    [-1952.0_f32, -2336.0_f32, 244.84375_f32],
    [-1920.0_f32, -2336.0_f32, 255.375_f32],
    [-1920.0_f32, -2368.0_f32, 268.1875_f32],
    [-1888.0_f32, -2368.0_f32, 281.03125_f32],
    [-1888.0_f32, -2400.0_f32, 288.46875_f32],
    [-1856.0_f32, -2400.0_f32, 301.5_f32],
    [-1824.0_f32, -2400.0_f32, 313.6875_f32],
    [-1792.0_f32, -2400.0_f32, 322.53125_f32],
    [-1760.0_f32, -2400.0_f32, 327.875_f32],
    [-1728.0_f32, -2400.0_f32, 333.1875_f32],
    [-1696.0_f32, -2400.0_f32, 344.75_f32],
    [-1664.0_f32, -2400.0_f32, 359.3125_f32],
    [-1632.0_f32, -2400.0_f32, 373.84375_f32],
    [-1600.0_f32, -2400.0_f32, 388.84375_f32],
    [-1600.0_f32, -2368.0_f32, 399.375_f32],
    [-1568.0_f32, -2368.0_f32, 412.1875_f32],
    [-1568.0_f32, -2336.0_f32, 425.03125_f32],
    [-1536.0_f32, -2336.0_f32, 432.46875_f32],
    [-1536.0_f32, -2304.0_f32, 445.5_f32],
    [-1536.0_f32, -2272.0_f32, 457.6875_f32],
    [-1536.0_f32, -2240.0_f32, 464.15625_f32],
    [-1536.0_f32, -2208.0_f32, 464.03125_f32],
    [-1536.0_f32, -1920.0_f32, 464.03125_f32],
    [-1600.0_f32, -1920.0_f32, 464.03125_f32],
    [-1600.0_f32, -1888.0_f32, 464.03125_f32],
    [-1632.0_f32, -1888.0_f32, 464.03125_f32],
    [-1632.0_f32, -1184.0_f32, 464.03125_f32],
    [-1536.0_f32, -1184.0_f32, 464.03125_f32],
    [-1536.0_f32, -352.0_f32, 464.03125_f32],
    [-1504.0_f32, -352.0_f32, 464.03125_f32],
    [-1504.0_f32, -320.0_f32, 464.03125_f32],
    [-1472.0_f32, -320.0_f32, 464.03125_f32],
    [-1472.0_f32, -224.0_f32, 464.03125_f32],
    [-1472.0_f32, -192.0_f32, 468.4375_f32],
    [-1472.0_f32, -160.0_f32, 479.03125_f32],
    [-1472.0_f32, -128.0_f32, 487.65625_f32],
    [-1440.0_f32, -128.0_f32, 495.65625_f32],
    [-1440.0_f32, -96.0_f32, 500.71875_f32],
    [-1408.0_f32, -96.0_f32, 507.3125_f32],
    [-1376.0_f32, -96.0_f32, 512.15625_f32],
    [-1344.0_f32, -96.0_f32, 512.03125_f32],
    [-1248.0_f32, -96.0_f32, 512.03125_f32],
    [-1248.0_f32, -64.0_f32, 512.03125_f32],
    [-1216.0_f32, -64.0_f32, 512.03125_f32],
    [-1024_f32, -64_f32, 496_f32],
];
