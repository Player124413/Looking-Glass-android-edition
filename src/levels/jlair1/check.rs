use super::*;
use crate::movement::Controls;
pub(super) fn owner(r: &crate::route::Route) -> &Curiosity {
    r.interactions
        .levels
        .iter()
        .find_map(|s| s.ctl.downcast_ref())
        .unwrap()
}
pub(super) fn own(r: &mut crate::route::Route) -> &mut Curiosity {
    r.interactions
        .levels
        .iter_mut()
        .find_map(|s| s.ctl.downcast_mut())
        .unwrap()
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/jlair1.bsp")?)?;
    for hz in [30, 60, 144] {
        let mut o = Curiosity::load(a, &map)?;
        o.saved.arrived = true;
        o.saved.scene = None;
        let mut w = World::from_bsp(&map)?;
        w.set_dynamic(o.colliders());
        let feet = w
            .actor_footing(vec3(-7048., 716., 1640.), PLAYER_CENTER, PLAYER_HALF, 20.)
            .context("Missing lava platform support")?;
        let mut p = Player::new(feet);
        p.grounded = true;
        for _ in 0..hz / 2 {
            o.advance(1. / hz as f32, &map, &mut w, &mut p, &[])?;
        }
        ensure!(
            (o.saved.sink - 50.).abs() < 0.1
                && (p.feet.z - feet.z + 50.).abs() < 0.1
                && w.body_clear(p.feet),
            "Platform failed to carry rider at {hz} Hz: sink {}, from {feet:?} to {:?}",
            o.saved.sink,
            p.feet
        );
        let saved = o.snapshot();
        o.advance(0., &map, &mut w, &mut p, &[])?;
        ensure!(o.snapshot() == saved, "Paused platform moved");
        o.restore(&saved, &map)?;
        ensure!(o.snapshot() == saved, "Platform save changed");
        let mut bad = saved.clone();
        bad["sink"] = 129.into();
        ensure!(
            o.restore(&bad, &map).is_err() && o.snapshot() == saved,
            "Corrupt platform save mutated live state"
        );
        p = Player::new(o.point("alice_pos1").translation);
        for _ in 0..hz * 2 {
            o.advance(1. / hz as f32, &map, &mut w, &mut p, &[])?;
        }
        ensure!(o.saved.sink == 0., "Unoccupied platform did not recover");
        println!("PASS Burning Curiosity platform rider, pause, reset and save {hz} Hz");
    }
    let mut endings = vec![];
    for skip in [false, true] {
        let mut o = Curiosity::load(a, &map)?;
        let mut w = World::from_bsp(&map)?;
        w.set_dynamic(o.colliders());
        let mut p = Player::spawn(&w, interaction::spawn(&map, Some("jlair1_start1")).0)
            .context("Blocked arrival")?;
        let mut story = Story::load(a, "jlair1");
        for kind in [Kind::Arrival, Kind::Caterpillar] {
            if kind == Kind::Caterpillar {
                o.event(CATERPILLAR);
            }
            if skip {
                o.skip(&map, &mut w, &mut p, &mut story)?;
            }
            for _ in 0..120 * 300 {
                o.advance(1. / 120., &map, &mut w, &mut p, &[])?;
                story.line_limit = None;
                if o.prepare_story(&mut story) {
                    story.tick(1. / 120., false);
                }
                o.sync_story(&story);
                for n in story.take_completed() {
                    o.dialogue_complete(&n);
                }
                if !o.scripted() {
                    break;
                }
            }
            ensure!(
                !o.scripted() && w.body_clear(p.feet),
                "Scene failed handoff"
            );
            endings.push(p.feet);
        }
        ensure!(
            o.saved.arrived && o.saved.caterpillar,
            "Missing completion latches"
        );
        o.event(CATERPILLAR);
        ensure!(!o.scripted(), "Caterpillar replayed");
        println!(
            "PASS Burning Curiosity full dialogue clock, scenes skip={skip}, landing {:?}",
            p.feet
        );
    }
    ensure!(
        endings[0] == endings[2] && endings[1] == endings[3],
        "Watch/skip handoffs differ"
    );
    // Both live and freshly reconstructed visits must continue identically through
    // the riding clock, the Cat reveal, and the one-shot essence pickups.
    for phase in 0..3 {
        let mut live = crate::route::Route::new(a, "jlair1", Some("jlair1_start1"))?;
        if phase == 0 {
            live.wait(3.)?;
        } else {
            live.skip_cinematics = true;
            live.wait_for_cinematic()?;
            live.skip_cinematics = false;
            if phase == 1 {
                own(&mut live).event(CATERPILLAR);
                live.wait(2.)?;
            } else {
                let o = own(&mut live);
                o.saved.essence_live = true;
                o.saved.essence_taken[0] = true;
            }
        }
        let mut resumed = crate::route::Route::resume(a, &live.checkpoint())?;
        for _ in 0..480 {
            live.tick(Controls::default())?;
            resumed.tick(Controls::default())?;
        }
        ensure!(
            live.state()? == resumed.state()?,
            "Saved scene/essence future diverged, phase {phase}"
        );
    }
    let mut stats = Stats::for_level("jlair1", None);
    stats.spend_will(75.);
    ensure!(
        stats.apply(crate::inventory::PickupKind::Weapon(5), 100.)
            && stats.copies(5) == 1
            && stats.will() == 100.,
        "Duplicate Jacks did not refill Will"
    );
    println!("PASS saved scene/essence continuation and duplicate Jacks refill");
    let mut r = crate::route::Route::new(a, "jlair1", Some("jlair1_start1"))?;
    r.skip_cinematics = true;
    r.wait_for_cinematic()?;
    r.player = Player::new(vec3(-3520., 3600., 3024.));
    r.player = Player::spawn(&r.world, r.player.feet).context("Missing exit approach")?;
    r.stop_at_exit = true;
    for _ in 0..240 {
        r.tick(Controls {
            wish: Vec2::Y,
            run: true,
            ..Default::default()
        })?;
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition == Some(("jlair2".into(), Some("jlair2_start1".into()))),
        "Wrong exit {:?}",
        r.transition
    );
    let next = r.depart(a, true)?;
    ensure!(
        next.interactions
            .levels
            .iter()
            .any(|s| s.ctl.id() == "jlair2")
            && next.world.body_clear(next.player.feet),
        "Missing survival encounter arrival"
    );
    println!(
        "PASS staged exit contact into implemented Jabberwock encounter {:?}",
        next.player.feet
    );
    Ok(())
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        if let Ok(mode) = std::env::var("LOOKING_GLASS_JLAIR1_SAVE") {
            return saves::run(a, mode == "write").await;
        }
        std::fs::create_dir_all("private/jlair1-work/captures")?;
        let mut world = crate::render::Scene::load(a, "jlair1")?;
        let mut o = Curiosity::load(a, &world.map)?;
        let mut art = art::Art::load(a)?;
        for (name, kind, t, line) in [
            ("ride", Some(Kind::Arrival), 3., 0),
            ("landing", Some(Kind::Arrival), 9., 0),
            ("depart", Some(Kind::Arrival), 20., 0),
            ("cat", Some(Kind::Caterpillar), 9., 2),
            ("reveal", Some(Kind::Caterpillar), 15., 3),
            ("caterpillar", Some(Kind::Caterpillar), 24., 4),
            ("sink", None, 0., 0),
            ("sunk", None, 1., 0),
        ] {
            o.saved.scene = kind.map(Scene::new);
            if let Some(s) = &mut o.saved.scene {
                s.time = t;
                s.line = line;
                s.starts[1] = 4.;
                s.starts[3] = 12.;
                s.starts[4] = 22.;
                if line >= 3 {
                    s.reveal = Some(12.);
                }
                if name == "depart" {
                    s.ending = Some(3.);
                }
            }
            o.saved.sink = if name == "sunk" { 100. } else { 0. };
            o.rebuild(&world.map)?;
            world.atmosphere.distance = o.scene_fog().unwrap();
            let camera = o.scene_camera().unwrap_or(Camera::look(
                vec3(-6750., 400., 1820.),
                vec3(-6968., 698., 1570.),
            ));
            if name == "ride" {
                let at = art.rider(&o, t)?;
                ensure!(
                    at.translation
                        .distance(o.paths["gryphon_path1"].sample(t, true).translation)
                        < 200.,
                    "Detached riding Alice"
                );
            }
            let cam = Camera3D {
                position: camera.eye,
                target: camera.target,
                up: camera.up,
                fovy: o
                    .fovy(screen_width() / screen_height())
                    .unwrap_or(75_f32.to_radians()),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            };
            for frame in 0..4 {
                clear_background(world.atmosphere.background());
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
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/jlair1-work/captures/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        println!("PASS Burning Curiosity native scene and platform captures");
        Ok(())
    })
}
