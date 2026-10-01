use super::*;
pub(super) fn owner(r: &mut crate::route::Route) -> &mut Royale {
    r.interactions
        .levels
        .iter_mut()
        .find_map(|s| s.ctl.downcast_mut::<Royale>())
        .unwrap()
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let mut map = Bsp::parse(&a.read("maps/grounds2.bsp")?)?;
    for difficulty in crate::powerups::Difficulty::ALL {
        map.difficulty = difficulty;
        for hz in [30, 60, 144] {
            let mut r = Royale::load(a, &map)?;
            let mut w = World::from_bsp(&map)?;
            w.set_dynamic(r.colliders());
            let mut p = Player::spawn(&w, interaction::spawn(&map, Some("grounds2_start1")).0)
                .context("Battlefield arrival is obstructed")?;
            ensure!(
                p.feet.z > -800. && w.body_clear(p.feet),
                "Unsafe arrival: {:?}",
                p.feet
            );
            for _ in 0..hz * 23 {
                r.advance(1. / hz as f32, &map, &mut w, &mut p, &[])?;
                ensure!(
                    w.body_clear(p.feet),
                    "Intro overlaps floor at {}: {:?}",
                    r.saved.time,
                    p.feet
                );
            }
            ensure!(
                r.saved.done && p.feet.distance(*r.data.path.last().unwrap()) < 1.,
                "Introduction missed landing"
            );
            let watched = r.snapshot();
            let mut skipped = Royale::load(a, &map)?;
            let mut sw = World::from_bsp(&map)?;
            let mut sp = Player::new(r.data.path[0]);
            skipped.saved.initialized = r.saved.initialized;
            skipped.skip(&map, &mut sw, &mut sp, &mut Story::load(a, "grounds2"))?;
            ensure!(
                watched == skipped.snapshot() && sp.feet.distance(p.feet) < 0.01,
                "Skip changes outcome"
            );
            let snap = r.snapshot();
            r.advance(0., &map, &mut w, &mut p, &[])?;
            ensure!(r.snapshot() == snap, "Pause advanced intro");
            let mut other = Royale::load(a, &map)?;
            other.restore(&snap, &map)?;
            ensure!(other.snapshot() == snap, "Restore changed scene");
            println!(
                "PASS grounds2 {difficulty:?} {hz}Hz safe arrival, watched/skip equality and pause"
            );
        }
        let mut r = Royale::load(a, &map)?;
        let mut w = World::from_bsp(&map)?;
        let mut p = Player::new(r.data.path[0]);
        let mut story = Story::load(a, "grounds2");
        r.skip(&map, &mut w, &mut p, &mut story)?;
        for n in [1, 3, 5, 7] {
            r.event(&format!(
                "Spawner{:02}",
                n + usize::from(difficulty == crate::powerups::Difficulty::Easy)
            ));
        }
        let mut stats = Stats::for_level("grounds2", None);
        for tick in 0..120 * 8 {
            r.combat(&mut crate::level::Combat {
                dt: 1. / 120.,
                world: &w,
                player: &mut p,
                stats: &mut stats,
                story: &mut story,
                notarget: true,
                summon: None,
                threatens: &|_| false,
            });
            if tick == 119 {
                ensure!(
                    r.saved.groups.iter().all(|g| !g.started || g.count == 1),
                    "Second guard spawned too early"
                );
            }
            if tick == 145 {
                ensure!(
                    r.saved.groups.iter().all(|g| !g.started || g.count == 2),
                    "Second guard spawned too late"
                );
            }
            state::State::validate(&r.saved, state::Visit { returning: false })?;
        }
        for (i, actor) in r.saved.actors.iter().enumerate().filter(|(_, v)| v.spawned) {
            ensure!(
                actor.body.health > 0. && actor.flight.is_none() && actor.body.feet.z < 400.,
                "Guard {i} failed to land: {:?}, health {}, flight {}",
                actor.body.feet,
                actor.body.health,
                actor.flight.is_some()
            );
        }
        ensure!(
            r.saved.groups.iter().all(|g| !g.started || g.count == 2),
            "Live cap failed"
        );
        let cap = r.snapshot();
        let mut restored = Royale::load(a, &map)?;
        restored.restore(&cap, &map)?;
        for _ in 0..100 {
            for o in [&mut r, &mut restored] {
                o.combat(&mut crate::level::Combat {
                    dt: 1. / 144.,
                    world: &w,
                    player: &mut p,
                    stats: &mut stats,
                    story: &mut story,
                    notarget: true,
                    summon: None,
                    threatens: &|_| false,
                });
            }
            ensure!(
                r.snapshot() == restored.snapshot(),
                "Saved company future diverged"
            );
        }
        println!("PASS grounds2 {difficulty:?} launch landings, 1.1-second cadence, per-company cap and saved future");
        for tick in 0..120 * 30 {
            r.combat(&mut crate::level::Combat {
                dt: 1. / 120.,
                world: &w,
                player: &mut p,
                stats: &mut stats,
                story: &mut story,
                notarget: true,
                summon: None,
                threatens: &|_| false,
            });
            if tick % 180 == 179 {
                for h in r.targets() {
                    r.hit(Hit {
                        id: h.id,
                        damage: 10000.,
                        kind: crate::combat::DamageKind::Other,
                        knockback: Vec3::ZERO,
                    });
                }
            }
        }
        let count: usize = r.saved.groups.iter().map(|g| g.count).sum();
        ensure!(
            count
                == if difficulty == crate::powerups::Difficulty::Easy {
                    15
                } else {
                    30
                },
            "Wrong reinforcement quota: {count}"
        );
        let good = r.snapshot();
        for n in 1..=8 {
            r.event(&format!("Spawner{n:02}"));
            r.event(&format!("SpawnDead{n:02}"));
        }
        ensure!(r.snapshot() == good, "Repeated callback resets a company");
        r.restore(&good, &map)?;
        let mut bad = good.clone();
        bad["groups"][7]["count"] = serde_json::json!(7);
        ensure!(
            r.restore(&bad, &map).is_err() && r.snapshot() == good,
            "Bad save mutated companies"
        );
        println!("PASS grounds2 {difficulty:?}: {count} finite reinforcements, bounded live slots and atomic save rejection");
    }
    let r = crate::route::Route::new(a, "grounds2", Some("grounds2_start1"))?;
    if std::env::var_os("LOOKING_GLASS_GROUNDS2_SURVEY").is_some() {
        route::survey(&r)?;
    }
    Ok(())
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        std::fs::create_dir_all("private/grounds2-work/captures")?;
        let mut scene = crate::render::Scene::load(a, "grounds2")?;
        let mut o = Royale::load(a, &scene.map)?;
        let mut art = art::Art::load(a, &o)?;
        for (name, time) in [
            ("arrival", 0.),
            ("collapse", 5.5),
            ("pawn", 10.5),
            ("duel", 17.),
        ] {
            o.saved.time = time;
            o.rebuild(&scene.map)?;
            let c = o.camera(&scene.world).unwrap();
            let camera = Camera3D {
                position: c.eye,
                target: c.target,
                up: c.up,
                fovy: 60_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            };
            for frame in 0..4 {
                clear_background(scene.atmosphere.background());
                scene.prepare_camera_portals(&camera, time, false, &o.transforms());
                set_camera(&camera);
                crate::render_fx::begin_view(&camera, time, &scene.atmosphere, false);
                scene.draw(c.eye, time, false, false, &o.transforms());
                art.draw(&o, &scene.atmosphere, c.eye, false);
                crate::render::depth_read_only(|| {
                    scene.draw(c.eye, time, false, true, &o.transforms())
                });
                crate::render_fx::finish();
                set_default_camera();
                if frame == 3 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/grounds2-work/captures/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        let mut player = Player::new(o.data.path[0]);
        let mut stats = Stats::for_level("grounds2", None);
        let mut story = Story::load(a, "grounds2");
        o.finish(&scene.map, &mut scene.world, &mut player)?;
        o.event("Spawner01");
        for _ in 0..42 {
            o.combat(&mut crate::level::Combat {
                dt: 1. / 120.,
                world: &scene.world,
                player: &mut player,
                stats: &mut stats,
                story: &mut story,
                notarget: true,
                summon: None,
                threatens: &|_| false,
            });
        }
        let mut cast = crate::npc::Npcs::load(
            a,
            &scene.map,
            "grounds2",
            Some("grounds2_start1"),
            false,
            false,
        )?;
        cast.notarget(true);
        cast.activate_levels(&[]);
        for (name, c) in [
            (
                "spade",
                Camera::look(vec3(-3020., 1710., 900.), vec3(-3650., 1340., 790.)),
            ),
            (
                "magma",
                Camera::look(vec3(-1730., 1830., -400.), vec3(-1730., 2380., -600.)),
            ),
        ] {
            if name == "magma" {
                for _ in 0..60 {
                    cast.update(1. / 120., &scene.world, vec3(-1688., 1996., -550.));
                }
            }
            let camera = Camera3D {
                position: c.eye,
                target: c.target,
                up: c.up,
                fovy: 60_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            };
            for frame in 0..4 {
                clear_background(scene.atmosphere.background());
                scene.prepare_camera_portals(&camera, 22., false, &o.transforms());
                set_camera(&camera);
                crate::render_fx::begin_view(&camera, 22., &scene.atmosphere, false);
                scene.draw(c.eye, 22., false, false, &o.transforms());
                art.draw(&o, &scene.atmosphere, c.eye, false);
                cast.draw(
                    c.eye,
                    (c.target - c.eye).normalize(),
                    &scene.atmosphere,
                    false,
                );
                crate::render::depth_read_only(|| {
                    scene.draw(c.eye, 22., false, true, &o.transforms())
                });
                crate::render_fx::finish();
                set_default_camera();
                if frame == 3 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/grounds2-work/captures/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        println!("PASS grounds2 native arrival, collapse, launch, duel, Spade and Magma renders");
        Ok(())
    })
}
