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
    let map = Bsp::parse(&a.read("maps/hatter2.bsp")?)?;
    let mut o = Encounter::load(a, &map)?;
    // The recessed clock artwork lies below its walkable cover at z=0.
    for (name, floor) in [("hatter_cat", 0.), ("watch_cat", 396.)] {
        let p = o.data.points[name].translation;
        ensure!((p.z - floor).abs() < 0.2, "Cheshire floats at {name}: {p:?}");
        println!("PASS {name} grounded at {p:?}");
    }
    let mut world = World::from_bsp(&map)?;
    world.set_dynamic(o.colliders());
    let mut player = Player::spawn(&world, crate::interaction::spawn(&map, None).0)
        .context("Blocked Hatter entry")?;
    let mut stats = Stats::for_level("hatter2", None);
    let mut story = Story::load(a, "hatter2");
    for n in ["hatter_cat", "ExitTest", GRYPHON] {
        ensure!(
            if n == "ExitTest" {
                story.trigger_gated(n, "hatter2.rewards", true)
            } else {
                story.trigger(n)
            },
            "Missing reviewed Hatter dialogue {n}"
        );
        story.finish_sequence(n);
    }
    let timer = a.read("models/fx/dummy/timer.tan")?;
    let bytes = crate::skeletal::Bytes(&timer);
    ensure!(
        bytes.count(72, 10000)? == 100 && bytes.float(84)? == 5.,
        "Tea timer contract changed"
    );
    for behind in [false, true] {
        let mut b = battle::Boss::new(Transform {
            translation: vec3(5664., 160., 0.),
            rotation: Quat::IDENTITY,
        });
        b.set(battle::Action::Cane);
        let eye = b.at + vec3(if behind { -110. } else { 110. }, 0., 48.);
        let mut out = Feedback::default();
        for _ in 0..(o.data.boss().duration("attack_cane") * 120.).floor() as usize {
            b.step(&world, eye, true, false, false, &o.data, &mut out);
        }
        ensure!(
            out.damage == if behind { 0. } else { 15. },
            "Cane contact must be directional and fire only once: {}",
            out.damage
        );
    }
    let mut b = o.saved.boss.clone();
    for syringe in [false, true] {
        b.shots.push(battle::Shot {
            at: vec3(5664., 160., 300.),
            velocity: Vec3::X * 500.,
            age: 0.,
            syringe,
            id: u32::from(syringe),
            ended: None,
        });
    }
    b.projectile_step(
        battle::STEP,
        &world,
        vec3(6000., 160., 300.),
        true,
        &mut Feedback::default(),
    );
    ensure!(
        b.shots[0].age == 0. && b.shots[1].age > 0.,
        "Watch must stop cups, but not syringes"
    );
    println!("PASS source attack frame, directional dodge and Watch projectile behavior");
    let initial = o.snapshot();
    o.event("EndPlats");
    o.event("ExitTest");
    o.event(GRYPHON);
    ensure!(
        o.snapshot() == initial,
        "Early callback bypasses boss/rewards"
    );
    for b in &o.data.brushes {
        if matches!(b.name.as_str(), "end_plat1" | "end_plat2" | "drawbridge") {
            println!(
                "Support {} local {:?}..{:?}, pivot {:?}",
                b.name, b.min, b.max, b.pose.translation
            );
        }
    }
    o.start_fight();
    let start = o.snapshot();
    for hz in [30, 60, 144] {
        o.restore(&start, &map)?;
        let mut waves = false;
        let mut up = false;
        for _ in 0..hz * 98 {
            o.combat(&mut Combat {
                dt: 1. / hz as f32,
                world: &world,
                player: &mut player,
                stats: &mut stats,
                story: &mut story,
                notarget: true,
                summon: None,
                threatens: &|_| false,
            });
            waves |= o.saved.waves.len() == 2;
            up |= o.saved.cycle > 6500 && o.saved.boss.at == o.data.points["tower_up"].translation;
        }
        ensure!(
            waves && up && o.saved.cycle.abs_diff(240) <= 1,
            "{hz}Hz Hatter cycle differs: {}",
            o.saved.cycle
        );
        let saved = o.snapshot();
        o.combat(&mut Combat {
            dt: 0.,
            world: &world,
            player: &mut player,
            stats: &mut stats,
            story: &mut story,
            notarget: false,
            summon: None,
            threatens: &|_| false,
        });
        ensure!(saved == o.snapshot(), "Pause advanced combat");
        o.restore(&saved, &map)?;
        println!("PASS Hatter {hz}Hz clock, two bounded reinforcements, pause and restore");
    }
    // A lethal hit during the tower phase must still finish all three malfunctions.
    o.saved.cycle = 6600;
    o.saved.boss.at = o.data.points["tower_up"].translation;
    o.hit(Hit {
        id: BASE,
        damage: 3000.,
        kind: crate::combat::DamageKind::Knife,
        knockback: Vec3::ZERO,
    });
    for _ in 0..120 * 40 {
        o.combat(&mut Combat {
            dt: battle::STEP,
            world: &world,
            player: &mut player,
            stats: &mut stats,
            story: &mut story,
            notarget: true,
            summon: None,
            threatens: &|_| false,
        });
        if o.won() {
            break;
        }
    }
    ensure!(
        o.won() && o.saved.boss.malfunctions == 3,
        "Hatter death did not unlock platforms"
    );
    let saved = o.snapshot();
    for path in ["cycle", "door", "boss.health"] {
        let mut v = saved.clone();
        if path == "boss.health" {
            v["boss"]["health"] = serde_json::json!(2600.);
        } else {
            v[path] = serde_json::json!(-1);
        }
        ensure!(
            o.restore(&v, &map).is_err() && o.snapshot() == saved,
            "Invalid state mutated live encounter"
        );
    }
    println!("PASS Hatter tower death, three malfunctions, victory and atomic rejected saves");
    o.saved.lifts = 6.2;
    o.rebuild(&map)?;
    world.set_dynamic(o.colliders());
    player =
        Player::spawn(&world, vec3(5568., 988., 80.)).context("Disc support fixture blocked")?;
    for _ in 0..480 {
        o.advance(battle::STEP, &map, &mut world, &mut player, &[])?;
        player.tick(&world, Controls::default());
    }
    ensure!(
        player.feet.z > 390.,
        "Rider was not carried: {:?}, phase {}",
        player.feet,
        o.saved.lifts
    );
    let before = o.snapshot();
    let feet = player.feet;
    o.advance(0., &map, &mut world, &mut player, &[])?;
    ensure!(
        o.snapshot() == before && player.feet == feet,
        "Paused lift moved rider"
    );
    println!("PASS Hatter platform support, rider carry and pause");

    Ok(())
}
pub(super) fn route(a: &mut Assets) -> Result<()> {
    let mut r = crate::route::Route::new(a, "hatter2", None)?;
    r.skip_cinematics = std::env::var_os("LOOKING_GLASS_WATCH_SCENES").is_none();
    drive(&mut r)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.world.body_clear(next.player.feet),
        "Blocked jlair1 arrival"
    );
    println!("PASS About Face production input route and onward arrival");
    Ok(())
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        if let Ok(mode) = std::env::var("LOOKING_GLASS_HATTER_SAVE") {
            return saves::run(a, mode == "write").await;
        }
        std::fs::create_dir_all("private/hatter2/captures")?;
        let mut scene = crate::render::Scene::load(a, "hatter2")?;
        let mut o = Encounter::load(a, &scene.map)?;
        let mut art = art::Art::load(a, &o)?;
        for (name, phase, t, action) in [
            ("arrival", Phase::Arrival, 0., battle::Action::Idle),
            ("clock-cat", Phase::Cat, 3., battle::Action::Idle),
            ("clock-cat-close", Phase::Cat, 3., battle::Action::Idle),
            ("watch-cat", Phase::WatchCat, 3., battle::Action::Gone),
            ("cane", Phase::Fight, 0.6, battle::Action::Cane),
            ("syringe", Phase::Fight, 0.8, battle::Action::Rocket),
            ("cups", Phase::Fight, 0.8, battle::Action::Cup),
            ("clockworks", Phase::Fight, 51., battle::Action::Idle),
            ("tea", Phase::Fight, 74.6, battle::Action::Idle),
            (
                "malfunction",
                Phase::Fight,
                0.8,
                battle::Action::Malfunction,
            ),
            ("death", Phase::Fight, 3., battle::Action::DeathEnd),
            ("platforms", Phase::Victory, 1., battle::Action::Gone),
            ("gryphon", Phase::Gryphon, 9.5, battle::Action::Gone),
            ("riding", Phase::Gryphon, 20., battle::Action::Gone),
        ] {
            o.saved.phase = phase;
            o.saved.time = t;
            o.saved.boss.set(action);
            o.saved.boss.health = match action {
                battle::Action::DeathEnd | battle::Action::Gone => 0.,
                battle::Action::Malfunction => 100.,
                _ => 2600.,
            };
            o.saved.boss.time = if t > 5. { 0. } else { t };
            o.saved.boss.at = o.data.points[if matches!(name, "arrival" | "clockworks" | "tea") {
                "tower_up"
            } else {
                "tower_down"
            }]
            .translation;
            o.saved.boss.yaw = std::f32::consts::PI;
            o.saved.cycle = if t > 5. { (t * 120.) as u32 } else { 1000 };
            o.saved.lifts = 1.;
            o.saved.talk_done = name == "riding";
            o.saved.after_talk = if name == "riding" { 4. } else { 0. };
            o.saved.scene_from = o.data.points["end_dest"];
            o.saved.waves.clear();
            if name == "clockworks" {
                o.spawn_wave();
                for w in &mut o.saved.waves {
                    w.launch = 1.5;
                    w.actor.feet = w.to;
                }
            }
            o.rebuild(&scene.map)?;
            let camera = o.scene_camera().unwrap_or_else(|| {
                crate::cinematic::Camera::look(
                    if name == "clock-cat" {
                        vec3(5500., -40., 245.)
                    } else if name == "clock-cat-close" {
                        vec3(5630., 130., 95.)
                    } else if name == "watch-cat" {
                        vec3(5664., 1040., 475.)
                    } else if name == "platforms" {
                        vec3(5664., 650., 260.)
                    } else if name == "tea" {
                        vec3(6050., -220., 180.)
                    } else {
                        vec3(5290., 150., 155.)
                    },
                    if name == "clock-cat" {
                        vec3(5664., 205., 0.)
                    } else if name == "clock-cat-close" {
                        o.data.points["hatter_cat"].translation + Vec3::Z * 18.
                    } else if name == "watch-cat" {
                        o.data.points["watch_cat"].translation + Vec3::Z * 20.
                    } else if name == "platforms" {
                        vec3(5568., 988., 220.)
                    } else if name == "tea" {
                        vec3(5664., 160., 150.)
                    } else {
                        o.saved.boss.at + Vec3::Z * 130.
                    },
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
                ensure!(dropped == 0, "Hatter effect budget exceeded");
                set_default_camera();
                art.hud(&o);
                if frame == 3 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/hatter2/captures/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        println!("PASS Hatter native combat and Gryphon scene captures");
        Ok(())
    })
}

pub(crate) fn drive(r: &mut crate::route::Route) -> Result<()> {
    r.stop_at_exit = true;
    r.tactics = true;
    r.navigate_until_scene(vec3(5664., 160., 0.))?;
    for _ in 0..24000 {
        if owner(&r).saved.phase == Phase::Fight {
            break;
        }
        r.tick(Controls::default())?;
    }
    ensure!(
        owner(&r).saved.phase == Phase::Fight,
        "Clock never started at {:?}",
        r.player.feet
    );
    let goals = [
        vec2(6080., -240.),
        vec2(6080., 600.),
        vec2(5260., 600.),
        vec2(5260., -240.),
    ];
    let mut corner = 0;
    for tick in 0..120 * 600 {
        if owner(&r).won() {
            break;
        }
        let heal = r.stats.sanity() < 75. && owner(&r).saved.essence_live;
        let goal = if heal {
            owner(&r).essence_position().truncate()
        } else {
            goals[corner]
        };
        let d = goal - r.player.feet.truncate();
        if !heal && d.length() < 70. {
            corner = (corner + 1) % 4;
        }
        let targets = owner(&r).targets();
        r.aim_at = targets
            .iter()
            .find(|t| t.id == BASE)
            .or_else(|| targets.first())
            .map(|t| t.id);
        r.tick(Controls {
            wish: d.normalize_or_zero(),
            run: true,
            ..Default::default()
        })?;
        ensure!(
            r.stats.alive(),
            "Hatter route died at {tick}, Alice {:?}, boss {}",
            r.player.feet,
            owner(&r).saved.boss.health
        );
    }
    ensure!(
        owner(&r).won(),
        "Hatter survived route: {}",
        owner(&r).saved.boss.health
    );
    println!(
        "Hatter defeated: sanity {}, shots {}, feet {:?}",
        r.stats.sanity(),
        r.shots,
        r.player.feet
    );
    r.navigate_until_scene(vec3(5664., 160., 0.))?;
    // The route walks onto a low disc, waits for its rise, then jumps to the Watch ledge.
    let b = owner(&r)
        .data
        .brushes
        .iter()
        .find(|b| b.name == "end_plat1")
        .unwrap();
    let centre = (b.min + b.max) * 0.5 + b.pose.translation;
    r.navigate_until_scene(vec3(centre.x, centre.y - 160., 0.))?;
    for _ in 0..2400 {
        let top = 8. + 392. * super::scene::lift(owner(&r).saved.lifts);
        if top < 9. && owner(&r).saved.lifts >= 6. && owner(&r).saved.lifts < 6.5 {
            break;
        }
        r.tick(Controls::default())?;
    }
    for tick in 0..720 {
        let d = centre.truncate() - r.player.feet.truncate();
        if tick % 60 == 0 {
            println!(
                "Boarding {tick}: {:?}, lift {}",
                r.player.feet,
                owner(&r).saved.lifts
            );
        }
        r.tick(Controls {
            jump: tick == 0,
            wish: if d.length() > 8. {
                d.normalize_or_zero()
            } else {
                Vec2::ZERO
            },
            ..Default::default()
        })?;
        if r.player.feet.z > 350. {
            break;
        }
    }
    ensure!(
        r.player.feet.z > 350.,
        "Platform ride failed at {:?}",
        r.player.feet
    );
    for _ in 0..150 {
        r.tick(Controls {
            wish: (vec2(5664., 1140.) - r.player.feet.truncate()).normalize_or_zero(),
            jump: true,
            run: true,
            ..Default::default()
        })?;
        if r.player.feet.y > 1100. {
            break;
        }
    }
    r.navigate_until_scene(vec3(5664., 1152., 392.))?;
    for _ in 0..18000 {
        r.tick(Controls::default())?;
        if owner(&r).saved.phase == Phase::ExitReady && owner(&r).saved.door >= 1.4 {
            break;
        }
    }
    ensure!(
        owner(&r).saved.watch,
        "Watch was not collected at {:?}",
        r.player.feet
    );
    r.navigate_until_scene(vec3(5664., 1234., 392.))?;
    for _ in 0..120 {
        if owner(&r).scripted() {
            break;
        }
        r.tick(Controls {
            wish: Vec2::Y,
            run: true,
            ..Default::default()
        })?;
    }
    for _ in 0..24000 {
        r.tick(Controls::default())?;
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition.as_ref().is_some_and(|e| EXIT.matches(e)),
        "Gryphon exit did not finish: {:?} at {:?}",
        owner(&r).saved.phase,
        r.player.feet
    );
    Ok(())
}
