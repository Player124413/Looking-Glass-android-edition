use super::*;
use crate::movement::Controls;
fn owner(r: &crate::route::Route) -> &Encounter {
    r.interactions
        .levels
        .iter()
        .find_map(|s| s.ctl.downcast_ref::<Encounter>())
        .unwrap()
}
pub(crate) fn check(a: &mut Assets, kind: Kind) -> Result<()> {
    let map = Bsp::parse(&a.read(&format!("maps/{}.bsp", kind.map()))?)?;
    let mut o = Encounter::load(a, &map, kind)?;
    fire_check::check(&o.data, kind)?;
    let mut w = World::from_bsp(&map)?;
    let mut p =
        Player::spawn(&w, crate::interaction::spawn(&map, None).0).context("Blocked entry")?;
    let mut s = Story::load(a, kind.map());
    for n in [kind.intro(), kind.outro()] {
        ensure!(
            if n == "grounds1_EndCine" {
                s.trigger_gated(n, "grounds1.survived", true)
            } else {
                s.trigger(n)
            },
            "Missing reviewed dialogue {n}"
        );
        s.finish_sequence(n);
    }
    let intro = o.snapshot();
    o.event(kind.outro());
    ensure!(intro == o.snapshot(), "Early callback bypasses battle");
    o.skip(&map, &mut w, &mut p, &mut s)?;
    ensure!(w.body_clear(p.feet), "Intro handoff embedded");
    let start = o.snapshot();
    let mut watch = o.saved.boss.clone();
    watch.freeze_spirals = true;
    watch.shots.push(battle::Shot {
        at: watch.at,
        start: watch.at,
        velocity: Vec3::X * 1000.,
        age: 0.,
        ended: None,
        spiral: true,
        bounced: false,
        id: 1,
    });
    watch.step(kind, &w, p.eye(), true, &o.data, &mut Feedback::default());
    ensure!(
        watch.time > 0. && watch.shots[0].age == 0.,
        "Pocket Watch must stop spirals, but not the boss"
    );
    let mut stats = Stats::for_level(kind.map(), None);
    stats.god = true;
    for hz in [30, 60, 144] {
        o.restore(&start, &map)?;
        for _ in 0..hz * if kind == Kind::Lair { 91 } else { 12 } {
            o.combat(&mut Combat {
                dt: 1. / hz as f32,
                world: &w,
                player: &mut p,
                stats: &mut stats,
                story: &mut s,
                notarget: true,
                summon: None,
                threatens: &|_| false,
            });
        }
        if kind == Kind::Lair {
            ensure!(
                o.saved.ticks == 10800 && o.saved.wave_count == 2 && o.saved.waves.len() == 6,
                "{hz}Hz waves/timer differ"
            );
        }
        let snap = o.snapshot();
        o.combat(&mut Combat {
            dt: 0.,
            world: &w,
            player: &mut p,
            stats: &mut stats,
            story: &mut s,
            notarget: true,
            summon: None,
            threatens: &|_| false,
        });
        ensure!(snap == o.snapshot(), "Pause advanced encounter");
        o.restore(&snap, &map)?;
        println!(
            "PASS {} {hz}Hz timing, bounded resources, pause and state restore",
            kind.map()
        );
    }
    if kind == Kind::Lair {
        o.hit(Hit {
            id: kind.base(),
            damage: 500000.,
            kind: crate::combat::DamageKind::Blunderbuss,
            knockback: Vec3::ZERO,
        });
        ensure!(o.saved.boss.health == 99999., "Survival boss can die");
        o.advance(0.1, &map, &mut w, &mut p, &[])?;
        ensure!(
            o.saved.phase == Phase::Outro,
            "Survival finale did not start"
        );
        o.skip(&map, &mut w, &mut p, &mut s)?;
        ensure!(o.saved.phase == Phase::Reward, "No eye reward");
        p.feet = o.data.points["eye_altar"].translation + Vec3::Z * 16.;
        o.prepare_player(&mut stats, &mut p);
        ensure!(o.saved.eye && stats.copies(7) == 1, "Eye Staff not granted");
        for _ in 0..25 {
            o.advance(0.1, &map, &mut w, &mut p, &[])?;
        }
        let exit = o
            .update(&mut w, &p, Vec3::X, false)
            .transition
            .context("Missing return exit")?;
        ensure!(kind.exit().matches(&exit), "Wrong return visit");
        ensure!(
            o.update(&mut w, &p, Vec3::X, false).transition.is_none(),
            "Repeated in-flight exit"
        );
        let saved = o.snapshot();
        o.restore(&saved, &map)?;
        ensure!(
            o.update(&mut w, &p, Vec3::X, false).transition.is_some(),
            "Restored exit lost"
        );
    } else {
        ensure!(o.saved.boss.flying, "Grounds boss never took off");
        o.hit(Hit {
            id: kind.base(),
            damage: 1000.,
            kind: crate::combat::DamageKind::Blunderbuss,
            knockback: Vec3::ZERO,
        });
        for _ in 0..2400 {
            o.combat(&mut Combat {
                dt: battle::STEP,
                world: &w,
                player: &mut p,
                stats: &mut stats,
                story: &mut s,
                notarget: true,
                summon: None,
                threatens: &|_| false,
            });
            if o.saved.boss.landed {
                break;
            }
        }
        ensure!(
            o.saved.boss.landed && !o.saved.boss.flying,
            "Half-health landing stuck at {:?}",
            o.saved.boss.at
        );
        o.hit(Hit {
            id: kind.base(),
            damage: 1000.,
            kind: crate::combat::DamageKind::Blunderbuss,
            knockback: Vec3::ZERO,
        });
        for _ in 0..102 {
            o.advance(0.1, &map, &mut w, &mut p, &[])?;
        }
        ensure!(
            o.saved.phase == Phase::Outro,
            "Defeat did not reach Gryphon"
        );
        o.skip(&map, &mut w, &mut p, &mut s)?;
        for _ in 0..132 {
            o.advance(0.1, &map, &mut w, &mut p, &[])?;
        }
        ensure!(o.saved.phase == Phase::Done, "Drawbridge failed");
        o.restore(&o.snapshot(), &map)?;
    }
    let good = o.snapshot();
    let mut bad = good.clone();
    bad["boss"]["health"] = serde_json::json!(-1);
    ensure!(
        o.restore(&bad, &map).is_err() && o.snapshot() == good,
        "Bad save mutated encounter"
    );
    for t in [0., 3., 12., 18., 24., 31., 38., 45., 60.] {
        o.saved.phase = Phase::Intro;
        o.saved.time = t;
        for p in o.performances() {
            ensure!(
                o.data.rigs[p.model].clips.contains_key(p.clip),
                "Missing scene clip {}/{}",
                p.model,
                p.clip
            );
            ensure!(p.pose.translation.is_finite(), "Invalid scene pose");
        }
        ensure!(o.scene_camera().is_some(), "Missing scene camera");
    }
    println!(
        "PASS {} victory, reward/bridge, source scenes, gates and corrupt-save rejection",
        kind.map()
    );
    Ok(())
}
pub(crate) fn route(a: &mut Assets, kind: Kind) -> Result<()> {
    play(a, kind).map(|_| ())
}
/// Continue a verified battle through its real campaign handoff.
pub(crate) fn play(a: &mut Assets, kind: Kind) -> Result<crate::route::Route> {
    let mut r = crate::route::Route::enter(
        a,
        kind.map(),
        None,
        Stats::for_level(kind.map(), None),
        crate::powerups::Difficulty::Normal,
    )?;
    r.skip_cinematics = std::env::var_os("JABBER_SKIP").is_some();
    drive(&mut r, kind)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.world.body_clear(next.player.feet),
        "Destination entry blocked"
    );
    println!(
        "PASS {} production input battle and {} arrival",
        kind.map(),
        kind.exit().map
    );
    Ok(next)
}
pub(crate) fn render(a: &mut Assets, kind: Kind) -> crate::levels::BoxFuture<'_> {
    Box::pin(async move {
        if let Ok(mode) = std::env::var("LOOKING_GLASS_JABBER_SAVE") {
            return saves::run(a, kind, mode == "write").await;
        }
        if kind == Kind::Lair { altar_check::render(a).await?; }
        fire_check::render(a, kind).await?;
        std::fs::create_dir_all("private/jabberwock/captures")?;
        let mut scene = crate::render::Scene::load(a, kind.map())?;
        let mut o = Encounter::load(a, &scene.map, kind)?;
        let mut art = art::Art::load(a, &o)?;
        for (name, phase, t) in [
            ("intro", Phase::Intro, 3.),
            ("air-scene", Phase::Intro, 24.),
            ("fire", Phase::Fight, 1.),
            ("wounded-or-countdown", Phase::Fight, 0.5),
            ("eye-beam", Phase::Fight, 0.25),
            ("flight", Phase::Fight, 0.5),
            ("death", Phase::Death, 1.),
            ("eye-or-farewell", Phase::Outro, 4.),
            (
                "reward-or-bridge",
                if kind == Kind::Lair {
                    Phase::Reward
                } else {
                    Phase::Done
                },
                0.,
            ),
        ] {
            if kind == Kind::Lair && matches!(name, "flight" | "death") {
                continue;
            }
            o.saved.phase = phase;
            o.saved.time = t;
            o.saved.boss = battle::Boss::new(
                o.data.points[if kind == Kind::Lair {
                    "jabber_pos3"
                } else {
                    "jabberwock"
                }],
                kind,
            );
            o.saved.boss.set(battle::Action::Fire);
            o.saved.boss.time = t;
            o.saved.ticks = if name == "wounded-or-countdown" { 5400 } else { 0 };
            if name == "wounded-or-countdown" && kind == Kind::Grounds {
                o.saved.boss.health = 1000.;
            }
            if name == "flight" {
                o.saved.boss.at.z += 350.;
                o.saved.boss.flying = true;
                o.saved.boss.set(battle::Action::FlyFire);
                o.saved.boss.time = t;
            } else if name == "death" {
                o.saved.boss.health = 0.;
                o.saved.boss.set(battle::Action::Dead);
                o.saved.boss.time = t;
            }
            if kind == Kind::Grounds {
                let sky = if name == "air-scene" {
                    "sky_manga"
                } else {
                    "sky_lava"
                };
                scene.set_sky_origin(o.data.points[sky].translation);
            }
            if name == "eye-beam" {
                let b = &mut o.saved.boss;
                b.set(battle::Action::Beam);
                b.time = t;
                b.aim = b.at + vec3(-450., 0., 80.);
                b.step(kind, &scene.world, b.aim, false, &o.data, &mut Feedback::default());
                ensure!(b.beam.is_some(), "Eye-beam fixture did not emit");
            } else if phase == Phase::Fight {
                let mut out = Feedback::default();
                for _ in 0..100 {
                    o.saved.boss.step(
                        kind,
                        &scene.world,
                        o.saved.boss.at + vec3(-450., 0., 32.),
                        false,
                        &o.data,
                        &mut out,
                    );
                }
            }
            o.rebuild(&scene.map)?;
            let camera = o.scene_camera().unwrap_or_else(|| {
                crate::cinematic::Camera::look(
                    o.saved.boss.at + vec3(-600., -250., 150.),
                    o.saved.boss.at + Vec3::Z * 130.,
                )
            });
            for frame in 0..4 {
                clear_background(BLACK);
                let cam = Camera3D {
                    position: camera.eye,
                    target: camera.target,
                    up: camera.up,
                    fovy: 75_f32.to_radians(),
                    z_near: 2.,
                    z_far: 20000.,
                    ..Default::default()
                };
                set_camera(&cam);
                crate::render_fx::begin_view(&cam, t, &scene.atmosphere, false);
                scene.draw(camera.eye, t, false, false, &o.transforms());
                art.draw(&o, &scene.atmosphere, camera.eye, false);
                crate::render::depth_read_only(|| {
                    scene.draw(camera.eye, t, false, true, &o.transforms());
                    art.effects(&o, camera.eye, &scene.atmosphere);
                });
                let (_, dropped) = crate::render_fx::finish();
                ensure!(dropped == 0, "Effect budget exceeded");
                set_default_camera();
                art.hud(&o);
                if frame == 3 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/jabberwock/captures/{}-{name}.png",
                        kind.map()
                    )))?;
                }
                next_frame().await;
            }
        }
        println!("PASS {} native scenes/combat captures", kind.map());
        Ok(())
    })
}

pub(crate) fn drive(r: &mut crate::route::Route, kind: Kind) -> Result<()> {
    r.stop_at_exit = true;
    r.tactics = true;
    r.heavy_weapon = Some(7);
    for _ in 0..30000 {
        r.tick(Controls::default())?;
        if owner(&r).saved.phase == Phase::Fight {
            break;
        }
    }
    ensure!(
        owner(&r).saved.phase == Phase::Fight,
        "Intro did not hand over"
    );
    let mut corner = 0;
    let goals = if kind == Kind::Lair {
        [
            vec2(-1200., 0.),
            vec2(-1000., 1000.),
            vec2(-500., 900.),
            vec2(-550., 0.),
        ]
    } else {
        [
            vec2(-1200., -100.),
            vec2(-1200., 700.),
            vec2(-400., 700.),
            vec2(-400., -100.),
        ]
    };
    for tick in 0..72000 {
        if owner(&r).saved.phase != Phase::Fight {
            break;
        }
        let healing = r.stats.sanity() < 75. && owner(&r).saved.essence_wait == 0.;
        let goal = if healing {
            owner(&r).essence_position().truncate()
        } else {
            goals[corner]
        };
        let d = goal - r.player.feet.truncate();
        if !healing && d.length() < 70. {
            corner = (corner + 1) % 4;
        }
        r.aim_at = if kind == Kind::Grounds {
            Some(kind.base())
        } else {
            owner(&r)
                .targets()
                .iter()
                .find(|t| t.id != kind.base())
                .map(|t| t.id)
        };
        r.tick(Controls {
            wish: d.normalize_or_zero(),
            run: true,
            // The arena supplies sit on raised ground. Walk into their real
            // pickup volume by jumping the lip, including a hanging pull-up.
            jump: r.player.ledge.as_ref().is_some_and(|h| !h.pulling)
                || (healing && r.player.grounded && d.length() < 180.
                    && owner(&r).essence_position().z - r.player.feet.z > 48.),
            ..Default::default()
        })
        .with_context(|| {
            format!(
                "{} route at fight tick {tick}, boss {:?}, survival {}",
                kind.map(),
                owner(&r).saved.boss.action,
                owner(&r).saved.ticks
            )
        })?;
        ensure!(
            r.stats.alive(),
            "Combat route died at {tick}, health {}, boss {} at {:?}",
            r.stats.sanity(),
            owner(&r).saved.boss.health,
            r.player.feet
        );
    }
    ensure!(
        owner(&r).saved.phase != Phase::Fight,
        "Battle never completed (boss {}, {:?})",
        owner(&r).saved.boss.health,
        owner(&r).saved.boss.action
    );
    for _ in 0..30000 {
        r.tick(Controls::default())?;
        if matches!(owner(&r).saved.phase, Phase::Reward | Phase::Done) {
            break;
        }
    }
    if kind == Kind::Lair {
        r.navigate_until_scene(owner(&r).data.points["eye_altar"].translation)?;
        for _ in 0..500 {
            r.tick(Controls::default())?;
            if r.transition.is_some() {
                break;
            }
        }
    } else {
        for x in [0., 250., 350., 688., 1000., 1300., 1500., 1700.] {
            let top = vec3(x, 192., 1000.);
            let bottom = vec3(x, 192., -300.);
            let tr = r.world.body_trace(top, bottom);
            println!(
                "Exit floor x{x}: {:?}, solid={}",
                top.lerp(bottom, tr.fraction),
                tr.start_solid
            );
        }
        for origin in [
            vec3(150., 192., 256.),
            vec3(750., 192., 256.),
            vec3(1300., 192., 256.),
            vec3(1700., 192., 300.),
        ] {
            let tr = r.world.body_trace(origin, origin - Vec3::Z * 600.);
            let goal = origin.lerp(origin - Vec3::Z * 600., tr.fraction);
            r.navigate_until_scene(goal)?;
            if origin.x == 1300. {
                for _ in 0..250 {
                    r.tick(Controls::default())?;
                }
            }
            if r.transition.is_some() {
                break;
            }
        }
    }
    ensure!(
        r.transition
            .as_ref()
            .is_some_and(|e| kind.exit().matches(e)),
        "Missing onward route"
    );
    Ok(())
}
