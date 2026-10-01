use super::*;
use crate::movement::Controls;
fn owner(r: &crate::route::Route) -> &Encounter {
    r.interactions
        .levels
        .iter()
        .find_map(|s| s.ctl.downcast_ref::<Encounter>())
        .unwrap()
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/rchess1.bsp")?)?;
    let mut o = Encounter::load(a, &map)?;
    let mut w = World::from_bsp(&map)?;
    w.set_dynamic(o.colliders());
    println!(
        "King tags {:?}; queener {}s, exit {}s; {} chess enemies",
        o.data
            .boss()
            .skeleton
            .bones
            .iter()
            .filter(|b| b.name.starts_with("tag_"))
            .map(|b| &b.name)
            .collect::<Vec<_>>(),
        o.data.queener_time,
        o.exit_duration(),
        o.saved.pieces.len()
    );
    let mut p = Player::spawn(&w, crate::interaction::spawn(&map, None).0)
        .context("Blocked Checkmate entry")?;
    let mut stats = Stats::for_level("rchess1", None);
    let mut story = Story::load(a, "rchess1");
    ensure!(o.targets().is_empty(), "Hidden King can be hit");
    let initial = o.snapshot();
    o.event(KILLED);
    ensure!(initial == o.snapshot(), "Killed callback bypassed combat");
    o.event(BEHEAD);
    o.skip(&map, &mut w, &mut p, &mut story)?;
    ensure!(
        o.saved.beheaded && w.body_clear(p.feet),
        "Beheading skip blocked"
    );
    o.event(INTRO);
    o.skip(&map, &mut w, &mut p, &mut story)?;
    ensure!(
        o.saved.phase == Phase::Fight && o.dismiss_summons() && w.body_clear(p.feet),
        "Intro did not arm arena"
    );
    let start = o.snapshot();
    for hz in [30, 60, 144] {
        o.restore(&start, &map)?;
        for _ in 0..hz * 30 {
            o.combat(&mut Combat {
                dt: 1. / hz as f32,
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
            o.saved.boss.used & 1 != 0
                && o.saved.boss.shots.len() <= 16
                && o.saved.boss.beams.len() <= 4,
            "{hz}Hz King did not attack safely"
        );
        let saved = o.snapshot();
        o.combat(&mut Combat {
            dt: 0.,
            world: &w,
            player: &mut p,
            stats: &mut stats,
            story: &mut story,
            notarget: false,
            summon: None,
            threatens: &|_| false,
        });
        ensure!(o.snapshot() == saved, "Pause advanced King");
        o.restore(&saved, &map)?;
        println!("PASS King {hz}Hz attacks, resource limits, pause and restore");
    }
    for (hp, attack, mask) in [
        (1300., battle::Action::Beam, 1),
        (750., battle::Action::Grenade, 2),
        (300., battle::Action::Diamond, 4),
    ] {
        let mut b = battle::Boss::new(o.data.points["r_king"]);
        b.health = hp;
        ensure!(b.ranged() == attack, "Health tier mismatch");
        b.set(attack);
        let mut out = Feedback::default();
        for _ in 0..(o.data.boss().duration(b.clip()) * 120.).floor() as usize {
            b.step(&w, vec3(4100., 2944., 48.), false, false, &o.data, &mut out);
        }
        ensure!(b.used & mask != 0, "Missing ranged attack");
    }
    let mut b = battle::Boss::new(o.data.points["r_king"]);
    b.set(battle::Action::Melee);
    let mut out = Feedback::default();
    for _ in 0..(o.data.boss().duration("attack_1") * 120.).floor() as usize {
        b.step(
            &w,
            b.at + vec3(-90., 0., 48.),
            false,
            false,
            &o.data,
            &mut out,
        );
    }
    ensure!(
        out.damage == 10.,
        "Melee window hit {} times",
        out.damage / 10.
    );
    b = battle::Boss::new(o.data.points["r_king"]);
    b.step(
        &w,
        vec3(4100., 2944., 48.),
        false,
        true,
        &o.data,
        &mut Feedback::default(),
    );
    ensure!(
        b.action == battle::Action::Shield,
        "Incoming shot did not trigger counter"
    );
    println!("PASS King health tiers, damage-once melee and shield counter");
    o.restore(&start, &map)?;
    o.hit(Hit {
        id: BASE,
        damage: 2000.,
        kind: crate::combat::DamageKind::Ice,
        knockback: Vec3::X,
    });
    ensure!(
        o.saved.boss.frozen && o.targets().iter().all(|t| t.id != BASE),
        "Frozen death is still targetable"
    );
    for _ in 0..120 * 20 {
        o.combat(&mut Combat {
            dt: battle::STEP,
            world: &w,
            player: &mut p,
            stats: &mut stats,
            story: &mut story,
            notarget: true,
            summon: None,
            threatens: &|_| false,
        });
        if o.saved.phase == Phase::Killed {
            break;
        }
    }
    ensure!(
        o.saved.phase == Phase::Killed,
        "King death did not start scene"
    );
    let saved = o.snapshot();
    let mut bad = saved.clone();
    bad["boss"]["health"] = serde_json::json!(1300.);
    ensure!(
        o.restore(&bad, &map).is_err() && o.snapshot() == saved,
        "Invalid save mutated encounter"
    );
    o.skip(&map, &mut w, &mut p, &mut story)?;
    ensure!(
        o.update(&mut w, &p, Vec3::X, false)
            .transition
            .is_some_and(|e| EXIT.matches(&e)),
        "Missing Funhouse transition"
    );
    ensure!(
        o.update(&mut w, &p, Vec3::X, false).transition.is_none(),
        "Duplicate exit request"
    );
    println!("PASS frozen death, immutable rejected save, skip and one-shot Funhouse exit");
    Ok(())
}
pub(super) fn route(a: &mut Assets) -> Result<()> {
    route_impl(a, std::env::var_os("LOOKING_GLASS_KING_ARENA").is_some())
}
fn route_impl(a: &mut Assets, staged: bool) -> Result<()> {
    let mut r = crate::route::Route::new(a, "rchess1", None)?;
    r.skip_cinematics = std::env::var_os("LOOKING_GLASS_WATCH_SCENES").is_none();
    drive_inner(&mut r, staged)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.world.body_clear(next.player.feet),
        "Blocked Funhouse arrival"
    );
    println!("PASS Red King production input combat and onward arrival");
    Ok(())
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        if let Ok(mode) = std::env::var("LOOKING_GLASS_KING_SAVE") {
            return saves::run(a, mode == "write").await;
        }
        std::fs::create_dir_all("private/rchess1/captures")?;
        let mut scene = crate::render::Scene::load(a, "rchess1")?;
        let mut o = Encounter::load(a, &scene.map)?;
        let mut art = art::Art::load(a, &o)?;
        for (name, phase, time, action) in [
            ("beheading", Phase::Beheading, 15., battle::Action::Idle),
            ("bridge", Phase::Intro, 4., battle::Action::Idle),
            ("grow", Phase::Intro, 6.7, battle::Action::Idle),
            ("melee", Phase::Fight, 0.45, battle::Action::Melee),
            ("beam", Phase::Fight, 1.2, battle::Action::Beam),
            ("ball", Phase::Fight, 0.9, battle::Action::Grenade),
            ("diamond", Phase::Fight, 0.6, battle::Action::Diamond),
            ("shield", Phase::Fight, 0.35, battle::Action::Shield),
            ("frozen", Phase::Fight, 1., battle::Action::Dead),
            (
                "queen",
                Phase::Killed,
                o.pawn_time() + 4.,
                battle::Action::Dead,
            ),
            (
                "portal",
                Phase::Killed,
                o.portal_time() + 4.2,
                battle::Action::Dead,
            ),
            (
                "fall",
                Phase::Killed,
                o.fall_time() + 3.,
                battle::Action::Dead,
            ),
        ] {
            o.saved.phase = phase;
            o.saved.time = time;
            o.saved.boss.set(action);
            o.saved.boss.time = time.min(2.);
            o.saved.boss.health = if action == battle::Action::Dead {
                0.
            } else {
                1300.
            };
            o.saved.boss.frozen = name == "frozen";
            o.saved.boss.shots.clear();
            o.saved.boss.beams.clear();
            if name == "beam" {
                o.saved.boss.beams.push(battle::Beam {
                    from: o.saved.boss.at + vec3(-30., 0., 100.),
                    to: vec3(4120., 2920., 32.),
                    age: 0.15,
                });
            }
            for (label, kind) in [
                ("ball", battle::Kind::Ball),
                ("diamond", battle::Kind::Diamond),
                ("shield", battle::Kind::Seeker),
            ] {
                if name == label {
                    o.saved.boss.shots.push(battle::Shot {
                        id: 1,
                        kind,
                        at: o.saved.boss.at + vec3(-100., -20., 100.),
                        velocity: -Vec3::X * 300.,
                        age: 0.3,
                        ended: None,
                    });
                }
            }
            o.rebuild(&scene.map)?;
            let cam = o.scene_camera().unwrap_or_else(|| {
                crate::cinematic::Camera::look(
                    vec3(4170., 2680., 170.),
                    o.saved.boss.at + Vec3::Z * 64.,
                )
            });
            for frame in 0..4 {
                clear_background(BLACK);
                let camera = Camera3D {
                    position: cam.eye,
                    target: cam.target,
                    up: cam.up,
                    fovy: 75_f32.to_radians(),
                    z_near: 2.,
                    z_far: 20000.,
                    ..Default::default()
                };
                set_camera(&camera);
                crate::render_fx::begin_view(&camera, time, &scene.atmosphere, false);
                scene.draw(cam.eye, time, false, false, &o.transforms());
                art.draw(&o, &scene.atmosphere, cam.eye, false);
                crate::render::depth_read_only(|| {
                    scene.draw(cam.eye, time, false, true, &o.transforms());
                    art.effects(&o, cam.eye, &scene.atmosphere);
                });
                let (_, dropped) = crate::render_fx::finish();
                ensure!(dropped == 0, "King effects exceeded budget");
                set_default_camera();
                art.hud(&o);
                if frame == 3 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/rchess1/captures/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        println!("PASS Red King staged native captures");
        Ok(())
    })
}

pub(crate) fn drive(r: &mut crate::route::Route) -> Result<()> { drive_inner(r, false) }
fn drive_inner(r: &mut crate::route::Route, staged: bool) -> Result<()> {
    r.stop_at_exit = true;
    r.tactics = true;
    if staged {
        r.player = Player::spawn(&r.world, vec3(3300., 2944., 64.))
            .context("Arena bridge fixture blocked")?;
        println!("STAGED bridge entrance: independent boss-route fixture");
    } else {
        // Private input replays accept only movement/navigation commands: no state,
        // damage, health or inventory edits are possible through this fixture.
        let steps: Vec<serde_json::Value> =
            if let Ok(path) = std::env::var("LOOKING_GLASS_KING_ROUTE_INPUT") {
                serde_json::from_str(&std::fs::read_to_string(path)?)?
            } else {
                serde_json::from_str(super::route_input::DEFAULT)?
            };
        std::fs::create_dir_all("private/rchess1-route")?;
        for (n, step) in steps.into_iter().enumerate() {
            println!("CHECKMATE {n} {step} at {:?}, sanity {}", r.player.feet, r.stats.sanity());
            if let Some(v) = step.get("nav") {
                let xyz: [f32; 3] = serde_json::from_value(v.clone())?;
                r.navigate_until_scene(Vec3::from_array(xyz))?;
            } else if let Some(v) = step.get("input") {
                let input: [f32; 6] = serde_json::from_value(v.clone())?;
                for i in 0..(input[0] * 120.) as usize {
                    r.tick(Controls {
                        wish: vec2(input[1], input[2]),
                        swim: vec3(input[1], input[2], 0.),
                        rise: input[3],
                        use_pressed: input[4] > 0. && i == 0,
                        jump: input[5] > 0.,
                        run: true,
                        ..Default::default()
                    })?;
                }
                println!("INPUT feet {:?}, rope {:?}", r.player.feet, r.player.rope);
            } else if let Some(v) = step.get("steer") {
                let g: [f32; 4] = serde_json::from_value(v.clone())?;
                let goal = vec3(g[0], g[1], g[2]);
                for _ in 0..(g[3] * 120.) as usize {
                    let delta = goal - r.player.feet;
                    if delta.length() < 18. {
                        break;
                    }
                    r.tick(Controls {
                        wish: delta.truncate().normalize_or_zero(),
                        swim: delta.normalize_or_zero(),
                        rise: if delta.z > 18. {
                            1.
                        } else if delta.z < -18. {
                            -1.
                        } else {
                            0.
                        },
                        run: true,
                        ..Default::default()
                    })?;
                }
                ensure!(
                    r.player.feet.distance(goal) < 30.,
                    "Steer to {goal:?} stopped at {:?}",
                    r.player.feet
                );
                println!("STEER {:?}", r.player.feet);
            } else if step.get("clear").is_some() {
                r.clear(700.)?;
            } else if step.get("beheading").is_some() {
                for _ in 0..6000 {
                    r.tick(Controls::default())?;
                    if owner(&r).saved.beheaded {
                        break;
                    }
                }
                ensure!(owner(&r).saved.beheaded, "Execution was not reached");
                ensure!(r.stats.copies(6) == 3, "Third Demon Die missing");
            } else {
                anyhow::bail!("Unknown route input");
            }
            std::fs::write("private/rchess1-route/last.json", serde_json::to_vec(&r.checkpoint())?)?;
        }
    }
    r.navigate_until_scene(vec3(3936., 2944., 0.))?;
    for _ in 0..6000 {
        r.tick(Controls::default())?;
        if owner(&r).saved.phase == Phase::Fight {
            break;
        }
    }
    ensure!(
        owner(&r).saved.phase == Phase::Fight,
        "King intro was not reached at {:?}",
        r.player.feet
    );
    let corners = [
        vec2(3940., 2630.),
        vec2(4470., 2630.),
        vec2(4470., 3370.),
        vec2(3940., 3370.),
    ];
    let mut corner = 0;
    for i in 0..120 * 480 {
        if owner(&r).saved.phase != Phase::Fight {
            break;
        }
        let goal = if r.stats.sanity() < 65. && owner(&r).saved.essence_wait == 0. {
            owner(&r).essence_position().truncate()
        } else {
            if r.player.feet.truncate().distance(corners[corner]) < 55. {
                corner = (corner + 1) % 4;
            }
            corners[corner]
        };
        r.tick(Controls {
            wish: (goal - r.player.feet.truncate()).normalize_or_zero(),
            run: true,
            ..Default::default()
        })?;
        if i % 1200 == 0 {
            println!(
                "King {:.0} hp / Alice {:.0}, feet {:?}",
                owner(&r).saved.boss.health,
                r.stats.sanity(),
                r.player.feet
            );
        }
    }
    ensure!(
        owner(&r).saved.phase == Phase::Killed || r.transition.is_some(),
        "Boss did not finish"
    );
    println!(
        "King defeated: sanity {}, attacks {}, throws {}",
        r.stats.sanity(),
        owner(&r).saved.boss.used,
        r.shots
    );
    for _ in 0..18000 {
        r.tick(Controls::default())?;
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition.as_ref().is_some_and(|e| EXIT.matches(e)),
        "King scene did not exit"
    );
    Ok(())
}

