use super::*;
pub fn check(a: &mut Assets) -> Result<()> {
    super::performance_check::check(a)?;
    super::presentation_check::check(a)?;
    let m = Bsp::parse(&a.read("maps/funhouse.bsp")?)?;
    let mut o = Funhouse::load(a, &m)?;
    let mut w = World::from_bsp(&m)?;
    w.set_dynamic(o.colliders());
    let mut p = Player::new(o.data.points["funhouse_start1"].translation);
    o.finish_scene(&m, &mut w, &mut p)?;
    let paused = o.snapshot();
    o.advance(0., &m, &mut w, &mut p, &[])?;
    ensure!(o.snapshot() == paused, "Paused machinery changed state");
    ensure!(
        o.saved.cells == 2 && o.saved.arrived,
        "Intro did not free cell 2"
    );
    let floor = o.objects.iter().find(|o| o.name == "break_plat").unwrap();
    ensure!(floor.solid, "Arena floor absent");
    let ground = w.sweep(
        vec3(32., -2200., 100.),
        vec3(32., -2200., -100.),
        Vec3::ZERO,
    );
    ensure!(
        ground.fraction < 1. && ground.normal.z > 0.9,
        "Arena has no physical floor"
    );
    let scene_before = o.snapshot();
    let mut replay = Funhouse::load(a, &m)?;
    replay.restore(&scene_before, &m)?;
    let mut replay_world = World::from_bsp(&m)?;
    replay_world.set_dynamic(replay.colliders());
    let mut replay_player = p.clone();
    for _ in 0..30 {
        o.advance(1. / 120., &m, &mut w, &mut p, &[])?;
        replay.advance(1. / 120., &m, &mut replay_world, &mut replay_player, &[])?;
    }
    ensure!(
        o.snapshot() == replay.snapshot() && p.feet == replay_player.feet,
        "Restored machinery diverged during continuation"
    );
    for i in [1, 3, 4, 5, 6, 7] {
        o.event(&format!("cell{i}"));
        o.event(&format!("cell{i}"));
    }
    ensure!(
        o.saved.cells.count_ones() == 7 && !o.saved.gas,
        "Duplicate clock opened gas doors"
    );
    let v = o.snapshot();
    let mut restored = Funhouse::load(a, &m)?;
    restored.restore(&v, &m)?;
    ensure!(restored.saved.cells == o.saved.cells, "Clock save mismatch");
    o.event("cell8");
    for _ in 0..130 {
        o.advance(1. / 120., &m, &mut w, &mut p, &[])?;
    }
    ensure!(
        o.saved.scene.as_ref().is_some_and(|s| s.kind == Kind::Gas),
        "All clocks did not start gas reveal"
    );
    o.finish_scene(&m, &mut w, &mut p)?;
    ensure!(o.saved.gas, "Gas reveal failed");
    o.event(Kind::Tweedles.id());
    o.finish_scene(&m, &mut w, &mut p)?;
    ensure!(w.body_clear(p.feet), "Arena handoff obstructed");
    ensure!(o.targets().len() == 2, "Paired bosses absent");
    // Check authored attack frames on the same arena collision used by play.
    for family in 0..2 {
        let mut boss = o.saved.bosses[family].clone();
        boss.at = vec3(32., -2200., 1.);
        boss.yaw = 0.;
        boss.action = battle::Action::Knife;
        let rig = &o.data.rigs[boss.model()];
        boss.time = (if family == 0 { 8. } else { 7. }) * rig.frame("knife_attack") - 0.004;
        let mut feedback = Feedback::default();
        boss.step(
            &o.data,
            &w,
            boss.at + vec3(70., 0., 56.),
            false,
            0,
            &mut feedback,
        );
        ensure!(
            feedback.damage == 10.,
            "Tweedle knife event missed its damage frame"
        );
        boss.action = battle::Action::Split;
        boss.time = 28. * rig.frame("russian_split") - 0.004;
        let child = boss.step(
            &o.data,
            &w,
            boss.at + vec3(300., 0., 56.),
            false,
            0,
            &mut feedback,
        );
        ensure!(
            child.is_some(),
            "Tweedle failed to spawn its miniature from the belly tag"
        );
        ensure!(
            boss.step(
                &o.data,
                &w,
                boss.at + vec3(300., 0., 56.),
                false,
                0,
                &mut feedback
            )
            .is_none(),
            "Miniature event duplicated"
        );
        boss.time = 28. * rig.frame("russian_split") - 0.004;
        ensure!(
            boss.step(
                &o.data,
                &w,
                boss.at + vec3(300., 0., 56.),
                false,
                family + 2,
                &mut feedback
            )
            .is_none(),
            "Miniature live cap ignored"
        );
    }
    o.hit(Hit {
        id: BASE,
        damage: 800.,
        kind: crate::combat::DamageKind::Ice,
        knockback: Vec3::ZERO,
    });
    ensure!(
        !o.saved.exit.committed && o.saved.bosses[0].frozen && o.targets().len() == 1,
        "Single boss opened exit"
    );
    o.hit(Hit {
        id: BASE + 1,
        damage: 900.,
        kind: crate::combat::DamageKind::Knife,
        knockback: Vec3::ZERO,
    });
    let mut stats = Stats::default();
    let mut story = Story::load(a, "funhouse");
    for _ in 0..601 {
        o.combat(&mut crate::level::Combat {
            dt: 1. / 120.,
            world: &w,
            player: &mut p,
            stats: &mut stats,
            story: &mut story,
            notarget: false,
            summon: None,
            threatens: &|_| false,
        });
    }
    ensure!(
        o.saved
            .scene
            .as_ref()
            .is_some_and(|s| s.kind == Kind::Hatter),
        "Pair defeat did not start Hatter"
    );
    o.finish_scene(&m, &mut w, &mut p)?;
    ensure!(o.saved.exit.committed, "Hatter did not commit exit");
    let first = o.update(&mut w, &p, Vec3::X, false).transition;
    ensure!(
        first.is_some() && o.update(&mut w, &p, Vec3::X, false).transition.is_none(),
        "Exit emitted twice"
    );
    o.restore(&o.snapshot(), &m)?;
    ensure!(
        o.update(&mut w, &p, Vec3::X, false).transition == first,
        "Restored exit did not retry"
    );
    door_contract(a, &m)?;
    pit_contract(a)?;
    fulcrum_contract(a, &m)?;
    pendulum_contract(a, &m)?;
    for (distance, expected) in [(150., 25.), (270., 5.)] {
        let mut b = battle::Boss::new(
            BASE,
            0,
            false,
            Transform {
                translation: vec3(-300., -2000., 1.03125),
                rotation: Quat::IDENTITY,
            },
        );
        b.action = battle::Action::Fall;
        b.time = 0.3;
        b.velocity = -Vec3::Z * 100.;
        let mut arena = Funhouse::load(a, &m)?;
        arena.saved.scene = None;
        w.set_dynamic(arena.colliders());
        let mut f = Feedback::default();
        b.step(
            &arena.data,
            &w,
            b.at + vec3(distance, 0., 56.),
            false,
            0,
            &mut f,
        );
        ensure!(
            (f.damage - expected).abs() < 0.1,
            "Ground-pound falloff {distance}: {}",
            f.damage
        );
    }
    println!("PASS native radial ground-pound falloff");
    println!("PASS Funhouse clock mask, physical floor, scene handoffs, boss identities and exit transaction");
    Ok(())
}
fn pendulum_contract(a: &mut Assets, m: &Bsp) -> Result<()> {
    let mut o = Funhouse::load(a, m)?;
    o.saved.scene = None;
    let mut w = World::from_bsp(m)?;
    w.set_dynamic(o.colliders());
    let mut p = Player::new(vec3(-128., -3552., 8.03125));
    p.grounded = true;
    for _ in 0..960 {
        o.advance(1. / 120., m, &mut w, &mut p, &[])?;
    }
    ensure!(
        o.saved.time > 7.99 && w.body_clear(p.feet),
        "Pendulum rider stalls the world clock or intersects support"
    );
    ensure!(
        p.feet.distance(vec3(-128., -3552., 8.03125)) < 3.,
        "Pendulum does not return its rider after a full period"
    );
    println!("PASS full pendulum period with upright rider carry");
    Ok(())
}
fn fulcrum_contract(a: &mut Assets, m: &Bsp) -> Result<()> {
    let mut o = Funhouse::load(a, m)?;
    o.saved.scene = None;
    let mut w = World::from_bsp(m)?;
    w.set_dynamic(o.colliders());
    w.set_settled_supports(o.settled_supports());
    let mut p = Player::new(vec3(1920., -3430., 0.03125));
    p.grounded = true;
    for _ in 0..120 {
        o.advance(1. / 120., m, &mut w, &mut p, &[])?;
    }
    let i = o.objects.iter().position(|o| o.id == 460).unwrap();
    ensure!(
        o.saved.fulcrums[i].x < -10. && p.feet.z < -20.,
        "Resting support copy blocked fulcrum tilt/carry"
    );
    ensure!(w.body_clear(p.feet), "Tilted rider intersects its support");
    p.tick(
        &w,
        crate::movement::Controls {
            jump: true,
            ..Default::default()
        },
    );
    ensure!(
        p.jumps == 1 && !p.grounded,
        "Cannot jump from tilted support"
    );
    println!("PASS weighted fulcrum carry and jump without a stale support plane");
    Ok(())
}
fn pit_contract(a: &mut Assets) -> Result<()> {
    use crate::powerups::Difficulty;
    for difficulty in [Difficulty::Easy, Difficulty::Normal] {
        let mut r = crate::route::Route::enter(a, "funhouse", None, Stats::default(), difficulty)?;
        r.skip_cinematics = true;
        r.wait_for_cinematic()?;
        // Staged drop through the actual pit volumes: the route itself never uses rescue.
        r.player = Player::new(vec3(2400., -3000., -500.));
        r.player.velocity = vec3(0., 0., -200.);
        for _ in 0..360 {
            if r.tick(crate::movement::Controls::default()).is_err() || r.teleports > 0 {
                break;
            }
        }
        if difficulty == Difficulty::Easy {
            ensure!(
                r.teleports == 1 && r.stats.alive(),
                "Easy pit failed to rescue Alice"
            );
        } else {
            ensure!(
                r.teleports == 0 && !r.stats.alive(),
                "Normal pit used Easy rescue or failed to kill"
            );
        }
    }
    println!("PASS authored Easy rescue and Normal fall volumes");
    Ok(())
}
fn door_contract(a: &mut Assets, m: &Bsp) -> Result<()> {
    let mut o = Funhouse::load(a, m)?;
    o.saved.scene = None;
    o.saved.arrived = true;
    let initial = o.snapshot();
    for id in [147, 161] {
        let i = o.objects.iter().position(|v| v.id == id).unwrap();
        let center = (m.models[o.objects[i].model].min + m.models[o.objects[i].model].max) * 0.5
            + o.objects[i].base.translation;
        let mut signs = vec![];
        for side in [-1., 1.] {
            o.restore(&initial, m)?;
            let mut w = World::from_bsp(m)?;
            w.set_dynamic(o.colliders());
            let mut p = Player::new(vec3(center.x, center.y + side * 65., 0.03125));
            for _ in 0..240 {
                o.move_world(1. / 120., m, &mut w, &mut p, &[])?;
            }
            ensure!(
                o.saved.doors[i] == 1.,
                "Door {id} did not open from side {side}"
            );
            ensure!(w.body_clear(p.feet), "Door {id} crushed the waiting player");
            signs.push(o.saved.door_signs[i]);
        }
        ensure!(signs[0] != signs[1], "Door {id} swings toward an activator");
    }
    println!("PASS both native door flag combinations, both approach sides and player clearance");
    Ok(())
}
pub fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        if let Ok(mode) = std::env::var("LOOKING_GLASS_FUNHOUSE_SAVES") {
            return saves::run(a, mode == "write").await;
        }
        super::presentation_check::render(a).await?;
        std::fs::create_dir_all("private/funhouse-work/captures")?;
        let mut world = crate::render::Scene::load(a, "funhouse")?;
        println!(
            "Funhouse unresolved material identifiers: {:?}",
            world.missing
        );
        let mut o = Funhouse::load(a, &world.map)?;
        let mut art = art::Art::load(a, &o)?;
        for (name, kind, t, eye, target) in [
            ("arrival", Some(Kind::Arrival), 29., Vec3::ZERO, Vec3::ZERO),
            (
                "tube",
                None,
                0.,
                vec3(32., 100., 100.),
                vec3(32., -448., 100.),
            ),
            (
                "cells",
                None,
                0.,
                vec3(-1440., -550., 136.),
                vec3(-1043., -16., 288.),
            ),
            (
                "arena",
                None,
                0.,
                vec3(500., -2600., 250.),
                vec3(32., -2016., 32.),
            ),
            (
                "fulcrums",
                None,
                0.,
                vec3(2300., -2300., 420.),
                vec3(1900., -3100., 100.),
            ),
            (
                "heads",
                None,
                0.,
                vec3(380., -4580., 230.),
                vec3(50., -4740., 140.),
            ),
            (
                "pendulums",
                None,
                0.,
                vec3(-200., -3310., 160.),
                vec3(150., -3552., 100.),
            ),
            ("gas", Some(Kind::Gas), 7.9, Vec3::ZERO, Vec3::ZERO),
            (
                "miniatures",
                None,
                0.,
                vec3(300., -2450., 150.),
                vec3(0., -2030., 80.),
            ),
            (
                "rattle",
                None,
                0.,
                vec3(300., -2450., 150.),
                vec3(0., -2030., 100.),
            ),
            (
                "flight",
                None,
                0.,
                vec3(300., -2450., 150.),
                vec3(0., -2030., 220.),
            ),
            ("tweedles", Some(Kind::Tweedles), 6., Vec3::ZERO, Vec3::ZERO),
            ("hatter", Some(Kind::Hatter), 5., Vec3::ZERO, Vec3::ZERO),
        ] {
            o.saved.fight = matches!(name, "miniatures" | "rattle" | "flight");
            o.saved.minis.clear();
            for b in &mut o.saved.bosses {
                b.action = battle::Action::Idle;
                b.time = 0.;
                b.at = o.data.points[if b.family == 0 {
                    "dee_pos1"
                } else {
                    "dum_pos2"
                }]
                .translation;
            }
            if name == "miniatures" {
                for i in 0..2 {
                    let mut b = battle::Boss::new(
                        BASE + 100 + i,
                        i,
                        true,
                        Transform {
                            translation: vec3(-96. + i as f32 * 170., -2200., 24.),
                            rotation: Quat::IDENTITY,
                        },
                    );
                    b.action = battle::Action::Idle;
                    o.saved.minis.push(b);
                    o.saved.bosses[i].action = battle::Action::Split;
                    o.saved.bosses[i].time = 1.4;
                }
            }
            if name == "rattle" {
                for b in &mut o.saved.bosses {
                    b.action = battle::Action::RattleOut;
                    b.time = 0.8;
                    b.weapon = battle::Weapon::Rattle;
                }
            }
            if name == "flight" {
                for b in &mut o.saved.bosses {
                    b.action = battle::Action::Fly;
                    b.time = 0.4;
                    b.at.z += 180.;
                }
            }
            o.saved.scene = kind.map(Scene::new);
            if let Some(s) = &mut o.saved.scene {
                s.clock.time = t;
            }
            o.saved.gas_start = (name == "gas").then_some(o.saved.time - t);
            o.rebuild(&world.map)?;
            let c = o
                .scene_camera()
                .unwrap_or(crate::cinematic::Camera::look(eye, target));
            let cam = Camera3D {
                position: c.eye,
                target: c.target,
                up: c.up,
                fovy: 75f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            };
            for frame in 0..4 {
                clear_background(BLACK);
                world.prepare_camera_portals(&cam, t, false, &o.transforms());
                set_camera(&cam);
                crate::render_fx::begin_view(&cam, t, &world.atmosphere, false);
                world.draw(c.eye, t, false, false, &o.transforms());
                art.draw(&o, &world.atmosphere, c.eye, false);
                crate::render::depth_read_only(|| {
                    world.draw(c.eye, t, false, true, &o.transforms());
                    art.effects(&o, c.eye, &world.atmosphere);
                });
                crate::render_fx::finish();
                set_default_camera();
                if frame == 3 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/funhouse-work/captures/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        println!("PASS Funhouse native captures");
        Ok(())
    })
}
