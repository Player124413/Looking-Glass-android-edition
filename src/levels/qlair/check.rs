use super::*;
use crate::movement::{Controls, FIXED_DT};
fn owner(i: &crate::interaction::Interactions) -> &Queen {
    i.levels
        .iter()
        .find_map(|s| s.ctl.downcast_ref::<Queen>())
        .unwrap()
}
pub(super) fn route(a: &mut Assets) -> Result<()> {
    fight_route(a, false, false)
}
pub(super) fn skip_route(a: &mut Assets) -> Result<()> {
    fight_route(a, false, true)
}
pub(super) fn second_route(a: &mut Assets) -> Result<()> {
    fight_route(a, true, true)
}
fn arena_camp(world: &World, origin: Vec3, axis: Vec2) -> Result<Vec3> {
    let floor = |at: Vec3| {
        let high = vec3(at.x, at.y, 1200.);
        let low = vec3(at.x, at.y, 800.);
        let hit = world.body_trace(high, low);
        (!hit.start_solid && hit.fraction < 1.).then(|| high.lerp(low, hit.fraction))
    };
    let mut spots = Vec::new();
    for x in -7..=7 {
        for y in -7..=7 {
            let Some(p) = floor(origin + vec3(x as f32 * 48., y as f32 * 48., 0.)) else {
                continue;
            };
            let normal = vec2(-axis.y, axis.x);
            if (-7..=7).all(|i| {
                (-2..=2).all(|j| {
                    floor(p + (axis * i as f32 * 16. + normal * j as f32 * 16.).extend(0.))
                        .is_some_and(|f| (f.z - p.z).abs() < 32.)
                })
            }) {
                spots.push(p);
            }
        }
    }
    spots
        .into_iter()
        .min_by(|a, b| {
            a.distance_squared(vec3(-16., 4800., 960.))
                .total_cmp(&b.distance_squared(vec3(-16., 4800., 960.)))
        })
        .or_else(|| floor(origin))
        .context("No broad fighting platform near essence")
}
fn dodge(r: &crate::route::Route, camp: Vec3, axis: Vec2, preferred: Vec2, jump: bool) -> Vec2 {
    let q = owner(&r.interactions);
    let normal = vec2(-axis.y, axis.x);
    let choices = [
        preferred,
        axis,
        -axis,
        normal,
        -normal,
        (axis + normal).normalize(),
        (axis - normal).normalize(),
        (-axis + normal).normalize(),
        (-axis - normal).normalize(),
        Vec2::ZERO,
    ];
    choices
        .into_iter()
        .map(|wish| {
            let mut p = r.player.clone();
            let mut predicted = q
                .saved
                .projectiles
                .iter()
                .map(|s| (s.at, s.velocity, s.seek_at))
                .collect::<Vec<_>>();
            let mut cost = (wish - preferred).length_squared() * 2.;
            for tick in 0..84 {
                p.tick(
                    &r.world,
                    Controls {
                        wish,
                        jump: jump && tick == 0,
                        ..Default::default()
                    },
                );
                let dt = (tick + 1) as f32 * FIXED_DT;
                if p.feet.z < camp.z - 35. {
                    cost += 1e8;
                    break;
                }
                let from = p.feet.truncate() - camp.truncate();
                cost += from.length_squared() * 0.0002;
                // Staying near the camp is a preference. Predicted falls above
                // are the hard boundary; a projectile warrants using the whole
                // available platform rather than remaining inside a tiny box.
                cost += (from.dot(normal).abs() - 24.).max(0.).powi(2);
                cost += (from.dot(axis).abs() - 64.).max(0.).powi(2);
                for (shot, predicted) in q.saved.projectiles.iter().zip(&mut predicted) {
                    if shot.end.is_some() || shot.age + dt >= shot.life {
                        continue;
                    }
                    let spec = &q.data.projectiles[&format!("{}.tik", shot.model_key())];
                    if spec.seeker > 0. && shot.age + dt >= predicted.2 {
                        predicted.1 = projectile::steer(
                            predicted.1,
                            p.feet + Vec3::Z * 32. - predicted.0,
                            spec.seeker,
                        );
                        predicted.2 += 0.1;
                    }
                    predicted.0 += predicted.1 * FIXED_DT;
                    let at = predicted.0;
                    let d = (at - (p.feet + Vec3::Z * 32.)).abs() - vec3(24., 24., 40.);
                    let separation = d.max(Vec3::ZERO).length();
                    let margin = if spec.splash > 0. {
                        spec.radius + 40.
                    } else {
                        85.
                    };
                    if separation < margin {
                        cost += (margin - separation).powi(2) * 30.;
                    }
                }
            }
            (wish, cost)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap()
        .0
}
fn fight_route(a: &mut Assets, second: bool, skip: bool) -> Result<()> {
    for difficulty in [
        crate::powerups::Difficulty::Normal,
        crate::powerups::Difficulty::Hard,
    ] {
        let mut r = crate::route::Route::enter(
            a,
            "qlair",
            None,
            Stats::for_level("qlair", None),
            difficulty,
        )?;
        r.skip_cinematics = skip;
        drive_inner(&mut r,second)?;
    }
    Ok(())
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    audit::check(a)?;
    let map = Bsp::parse(&a.read("maps/qlair.bsp")?)?;
    let mut q = Queen::load(a, &map)?;
    for hp in [2500., 1000.] {
        q.saved.queen1 = hp;
        for attack in [
            battle::Attack::Idle,
            battle::Attack::Pain,
            battle::Attack::Wound,
            battle::Attack::Popup,
            battle::Attack::Spit,
            battle::Attack::Combo,
            battle::Attack::Sweep,
            battle::Attack::SlamLeft,
            battle::Attack::SlamRight,
            battle::Attack::Grab,
            battle::Attack::Ice,
            battle::Attack::Claw,
            battle::Attack::Club,
            battle::Attack::Centipede,
            battle::Attack::JabberEye,
            battle::Attack::JabberSpit,
            battle::Attack::Hatter,
            battle::Attack::Spore,
            battle::Attack::Scream,
        ] {
            q.saved.attack = attack;
            for name in [
                "c_queen1",
                "c_q2_body",
                "c_q2_t01",
                "c_q2_t02",
                "c_q2_t03",
                "c_q2_t04",
            ] {
                for (clip, _) in q.sequence(name) {
                    ensure!(
                        q.data.rigs[name].clips.contains_key(clip),
                        "Missing {name}/{clip}"
                    );
                }
            }
        }
    }
    q.saved.queen1 = 2500.;
    q.saved.attack = battle::Attack::Idle;
    ensure!(
        q.data.rigs["c_queen1"]
            .attacks
            .values()
            .map(Vec::len)
            .sum::<usize>()
            > 50,
        "Queen1 attack events absent"
    );
    ensure!(
        q.data.rigs["c_q2_body"]
            .attacks
            .values()
            .map(Vec::len)
            .sum::<usize>()
            > 10,
        "Queen2 attack events absent"
    );
    let mut world = World::from_bsp(&map)?;
    world.set_dynamic(q.colliders());
    let mut p = Player::spawn(&world, q.data.points["qlair_start1"].translation)
        .context("Finale entrance blocked")?;
    let mut stats = Stats::for_level("qlair", None);
    q.prepare_player(&mut stats, &mut p);
    ensure!(q.transforms().len() == 14, "Missing finale floor/stairs");
    q.event(INTRO);
    let mut story = Story::load(a, "qlair");
    ensure!(
        q.skip(&map, &mut world, &mut p, &mut story)?,
        "Intro skip failed"
    );
    ensure!(
        q.saved.phase == Phase::Queen1 && q.saved.collapsed == 20. && world.body_clear(p.feet),
        "Intro handoff failed"
    );
    q.hit(Hit {
        id: BASE,
        damage: 2500.,
        kind: crate::combat::DamageKind::Knife,
        knockback: Vec3::ZERO,
    });
    let threats = |_: &Target| false;
    q.combat(&mut Combat {
        dt: 0.01,
        world: &world,
        player: &mut p,
        stats: &mut stats,
        story: &mut story,
        notarget: false,
        summon: None,
        threatens: &threats,
    });
    ensure!(q.saved.phase == Phase::Birth, "Queen1 death failed");
    q.skip(&map, &mut world, &mut p, &mut story)?;
    q.prepare_player(&mut stats, &mut p);
    ensure!(
        q.checkpoint_requested() && q.saved.powered && stats.sanity() == 100.,
        "Birth checkpoint/refill failed"
    );
    q.checkpoint_written();
    ensure!(!q.checkpoint_requested(), "Duplicate checkpoint");
    q.saved.position = 4;
    q.saved.middle = 0.;
    q.saved.motion = 0;
    q.event("MoveQueen4");
    ensure!(
        q.saved.platform == -4 && q.saved.middle == 17. && q.saved.motion == 1,
        "Leaving platform failed to send Queen to middle"
    );
    q.event("MoveQueen1");
    ensure!(
        q.saved.platform == 1,
        "Arrival did not select next platform"
    );
    q.saved.motion = 0;
    let old = q.saved.parts[0];
    q.hit(Hit {
        id: BASE + 2,
        damage: 900.,
        kind: crate::combat::DamageKind::Blunderbuss,
        knockback: Vec3::ZERO,
    });
    ensure!(old == q.saved.parts[0], "Tentacle cannon immunity failed");
    q.hit(Hit {
        id: BASE + 1,
        damage: 3501.,
        kind: crate::combat::DamageKind::Knife,
        knockback: Vec3::ZERO,
    });
    q.combat(&mut Combat {
        dt: 0.01,
        world: &world,
        player: &mut p,
        stats: &mut stats,
        story: &mut story,
        notarget: false,
        summon: None,
        threatens: &threats,
    });
    ensure!(q.saved.phase == Phase::Death, "Queen2 death failed");
    q.skip(&map, &mut world, &mut p, &mut story)?;
    ensure!(
        q.controlled() && q.camera(&world).is_some(),
        "Ending delay released Alice over a ledge"
    );
    for _ in 0..99 {
        q.advance(0.1, &map, &mut world, &mut p, &[])?;
    }
    ensure!(!q.ending_ready(), "Ending fired early");
    for _ in 0..2 {
        q.advance(0.1, &map, &mut world, &mut p, &[])?;
    }
    ensure!(q.ending_ready(), "Ending absent");
    let saved = q.snapshot();
    q.restore(&saved, &map)?;
    for phase in [Phase::Queen1, Phase::Queen2, Phase::Ending] {
        let mut dead = Queen::load(a, &map)?;
        dead.saved.phase = phase;
        dead.saved.queen1 = 0.;
        dead.saved.queen2 = 0.;
        dead.saved.time = 9.99;
        let mut dead_stats = Stats::for_level("qlair", None);
        dead.saved.initialized = true;
        dead_stats.damage(10000.);
        dead.prepare_player(&mut dead_stats, &mut p);
        dead.combat(&mut Combat {
            dt: 0.1,
            world: &world,
            player: &mut p,
            stats: &mut dead_stats,
            story: &mut story,
            notarget: false,
            summon: None,
            threatens: &threats,
        });
        dead.advance(0.1, &map, &mut world, &mut p, &[])?;
        ensure!(dead.saved.phase == phase, "Dead Alice advanced {phase:?}");
    }
    // Watched scenes, early/middle/late skip and a restored saved clock must
    // all commit the same reachable arena and one request, independently of FPS.
    for hz in [30, 60, 144] {
        for (phase, skip_at) in [
            (Phase::Intro, None),
            (Phase::Birth, None),
            (Phase::Intro, Some(0.)),
            (Phase::Intro, Some(8.)),
            (Phase::Intro, Some(17.)),
            (Phase::Birth, Some(0.)),
            (Phase::Birth, Some(25.)),
            (Phase::Birth, Some(75.)),
        ] {
            let mut q = Queen::load(a, &map)?;
            let mut w = World::from_bsp(&map)?;
            w.set_dynamic(q.colliders());
            let mut p = Player::spawn(&w, q.data.points["alice_start_pos1"].translation)
                .context("Scene test entry")?;
            q.start(phase);
            if phase == Phase::Birth {
                q.saved.queen1 = 0.;
                q.saved.collapsed = 20.;
            }
            let mut story = Story::load(a, "qlair");
            let dt = 1. / hz as f32;
            let mut restored = false;
            for _ in 0..hz * 240 {
                if !q.scripted() {
                    break;
                }
                if skip_at.is_some_and(|t| q.saved.time >= t) {
                    q.skip(&map, &mut w, &mut p, &mut story)?;
                    continue;
                }
                q.advance(dt, &map, &mut w, &mut p, &[])?;
                story.line_limit = None;
                q.prepare_story(&mut story);
                story.tick(dt, false);
                for id in story.take_completed() {
                    q.dialogue_complete(&id);
                }
                if !restored && q.saved.time >= 3. {
                    let state = q.snapshot();
                    q.advance(0., &map, &mut w, &mut p, &[])?;
                    ensure!(state == q.snapshot(), "Paused scene advanced");
                    q.restore(&state, &map)?;
                    ensure!(state == q.snapshot(), "Scene restore drift");
                    restored = true;
                }
            }
            ensure!(
                q.saved.phase
                    == if phase == Phase::Intro {
                        Phase::Queen1
                    } else {
                        Phase::Queen2
                    },
                "Scene failed {phase:?} at {hz} Hz, time {}",
                q.saved.time
            );
            ensure!(w.body_clear(p.feet), "Scene endpoint blocked");
            ensure!(
                q.checkpoint_requested() == (phase == Phase::Birth),
                "Wrong checkpoint count"
            );
        }
    }
    println!(
        "PASS watched/early-middle-late skipped scenes, pause and saved clocks at 30/60/144 Hz"
    );
    for phase in [Phase::Queen1, Phase::Birth, Phase::Queen2, Phase::Ending] {
        let mut r = crate::route::Route::new(a, "qlair", None)?;
        let q = r
            .interactions
            .levels
            .iter_mut()
            .find_map(|s| s.ctl.downcast_mut::<Queen>())
            .unwrap();
        q.start(phase);
        q.saved.initialized = true;
        q.saved.collapsed = 20.;
        q.saved.time = 2.;
        if phase != Phase::Queen1 {
            q.saved.queen1 = 0.;
        }
        if matches!(phase, Phase::Queen2 | Phase::Ending) {
            q.saved.powered = true;
            q.saved.refilled = true;
            q.saved.checkpoint = true;
            q.saved.checkpoint_saved = true;
            r.player = Player::spawn(&r.world, q.data.points["alice_fight_queen"].translation)
                .context("Checkpoint arena blocked")?;
        }
        if phase == Phase::Queen2 {
            q.saved.motion = 1;
            q.saved.body.translation.z = -900.;
            q.saved.motion_time = 1.;
        }
        if phase == Phase::Ending {
            q.saved.queen2 = 900.;
        }
        r.interactions.sync(&mut r.world);
        let cp = r.checkpoint();
        let mut resumed = crate::route::Route::resume(a, &cp)?;
        r.quiet();
        resumed.quiet();
        for _ in 0..120 {
            r.tick(Controls::default())?;
            resumed.tick(Controls::default())?;
        }
        ensure!(r.state()? == resumed.state()?, "Saved {phase:?} diverged");
        ensure!(r.transition.is_none(), "Saved finale requested a map");
        println!(
            "PASS {phase:?} snapshot and identical continued simulation ({} seconds)",
            120. * FIXED_DT
        );
    }
    println!("PASS finale lifecycle: collapse, two defeat gates, power, one checkpoint, tentacle immunity, ten-second ending; ENDING (scripted contract fixture, not a combat route)");
    Ok(())
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let mut scene = crate::render::Scene::load(a, "qlair")?;
        let mut q = Queen::load(a, &scene.map)?;
        let mut art = art::Art::load(a, &q)?;
        let base_state = q.saved.clone();
        std::fs::create_dir_all("private/qlair-work/captures")?;
        let mut cases: Vec<(String, Phase, f32)> = [
            ("intro", Phase::Intro, 8.),
            ("queen1", Phase::Queen1, 1.),
            ("queen1-wounded", Phase::Queen1, 1.),
            ("collapse", Phase::Intro, 20.),
            ("birth", Phase::Birth, 25.),
            ("body-talk", Phase::Birth, q.data.speech_start() + 20.),
            ("queen2", Phase::Queen2, 12.),
            ("queen2-hatter", Phase::Queen2, 12.),
            ("queen2-scream", Phase::Queen2, 12.),
            ("queen2-wounded", Phase::Queen2, 12.),
            ("queen2-submerged", Phase::Queen2, 12.),
            ("queen2-collapsed-tentacles", Phase::Queen2, 12.),
            ("queen2-destroyed-tentacles", Phase::Queen2, 12.),
            ("death", Phase::Death, 2.),
            ("death-final", Phase::Death, 8.),
            (
                "death-explosions",
                Phase::Death,
                q.data.rigs["c_q2_body"].duration("death_start") + 5.,
            ),
            ("death-burst", Phase::Ending, 0.25),
            ("queen2-growth", Phase::Queen2, 12.),
            ("queen2-claw", Phase::Queen2, 12.),
            ("queen1-popup", Phase::Queen1, 12.),
            ("queen1-beam", Phase::Queen1, 12.),
            ("queen1-wounded-beam", Phase::Queen1, 12.),
            ("queen2-eye-beam", Phase::Queen2, 12.),
        ]
        .into_iter()
        .map(|(name, phase, time)| (name.into(), phase, time))
        .collect();
        cases.extend(q.data.projectiles.keys().map(|model| {
            (
                format!("projectile-{}", model.trim_end_matches(".tik")),
                Phase::Queen2,
                12.,
            )
        }));
        let explosions = q
            .data
            .projectiles
            .values()
            .filter_map(|p| p.explosion.clone())
            .collect::<std::collections::BTreeSet<_>>();
        cases.extend(explosions.into_iter().map(|model| {
            (
                format!("impact-{}", model.trim_end_matches(".tik")),
                Phase::Queen2,
                12.,
            )
        }));
        for (name, phase, time) in cases {
            q.saved = base_state.clone();
            q.saved.phase = phase;
            q.saved.time = time;
            if name == "queen1-wounded" {
                q.saved.queen1 = 1000.;
                q.saved.attack = battle::Attack::Wound;
                q.saved.attack_time = 0.9;
            }
            if name == "queen2-wounded" {
                q.saved.queen2 = 2750.;
            }
            if name == "queen2-hatter" {
                q.saved.attack = battle::Attack::Hatter;
                q.saved.attack_time = 2.;
            }
            if name == "queen2-growth" {
                q.saved.part_attack[1] = Some(battle::Attack::Centipede);
                q.saved.part_time[1] = 0.55;
            }
            if name == "queen2-claw" {
                q.saved.part_attack[0] = Some(battle::Attack::Claw);
                q.saved.part_time[0] = 1.5;
            }
            if name == "queen1-popup" {
                q.saved.attack = battle::Attack::Popup;
                q.saved.attack_time = 2.4;
                q.saved.attack_target =
                    q.data.points["alice_start_pos1"].translation + Vec3::Z * 32.;
            }
            if name == "death-explosions" {
                q.death_explosions();
            }
            if name == "death-burst" {
                q.final_burst();
                q.advance_effects(0.25, &scene.world);
            }
            if name == "queen2-scream" {
                q.saved.attack = battle::Attack::Scream;
                q.saved.attack_time =
                    q.data.rigs["c_q2_body"].duration("attack_slit_scream_open") + 1.;
            }
            if name == "queen2-submerged" {
                q.saved.body.translation.z = -700.;
                q.saved.motion = 2;
                q.saved.motion_time = 1.;
            }
            if name == "queen2-collapsed-tentacles" {
                q.saved.parts = [25.; 4];
                q.saved.part_death = [1.2; 4];
            }
            if name == "queen2-destroyed-tentacles" {
                q.saved.parts = [0.; 4];
            }
            q.saved.collapsed = if matches!(phase, Phase::Corridor | Phase::Intro) {
                (time - q.data.intro_break()).max(0.)
            } else {
                20.
            };
            q.saved.alice =
                q.data.points[if matches!(phase, Phase::Queen2 | Phase::Death | Phase::Ending) {
                    "alice_fight_queen"
                } else {
                    "alice_start_pos1"
                }];
            if name.ends_with("beam") {
                let attack = if phase == Phase::Queen1 {
                    battle::Attack::Sweep
                } else {
                    battle::Attack::JabberEye
                };
                q.saved.attack = attack;
                q.saved.attack_time = 0.;
                q.saved.platform = -4;
                if name == "queen1-wounded-beam" {
                    q.saved.queen1 = 1000.;
                }
                if let Some(n) = Queen::attack_part(attack) {
                    q.saved.part_attack[n] = Some(attack);
                }
                let mut player = Player::new(q.saved.alice.translation);
                let mut stats = Stats::for_level("qlair", None);
                let mut story = Story::load(a, "qlair");
                for _ in 0..2400 {
                    q.saved.clock += FIXED_DT;
                    q.battle(&mut Combat {
                        dt: FIXED_DT,
                        world: &scene.world,
                        player: &mut player,
                        stats: &mut stats,
                        story: &mut story,
                        notarget: false,
                        summon: None,
                        threatens: &|_| false,
                    });
                    if q.saved.projectiles.iter().any(|s| s.end.is_some()) {
                        break;
                    }
                }
                let beams = q
                    .saved
                    .projectiles
                    .iter()
                    .filter_map(|s| s.end.map(|end| (&s.model, s.at, end)))
                    .collect::<Vec<_>>();
                ensure!(!beams.is_empty(), "No live beams for {name}");
                println!("PASS actual attack capture {name}: {beams:?}");
            }
            let camera = q.scene_camera().unwrap_or_else(|| {
                crate::cinematic::Camera::look(
                    q.saved.alice.translation + vec3(0., -200., 130.),
                    if phase == Phase::Queen1 {
                        q.queen1_pose().translation
                    } else {
                        q.saved.body.translation + Vec3::Z * 1050.
                    },
                )
            });
            if let Some(model) = name.strip_prefix("impact-") {
                let toward = (camera.target - camera.eye).normalize();
                q.saved.impacts.push(projectile::Impact {
                    model: model.into(),
                    at: camera.eye + toward * 200.,
                    normal: -toward,
                    age: 0.15,
                    mark: Some("boojum_impact_decal".into()),
                    mark_radius: 24.,
                });
            }
            if let Some(model) = name.strip_prefix("projectile-") {
                let model = format!("{model}.tik");
                let spec = &q.data.projectiles[&model];
                let toward = (camera.target - camera.eye).normalize();
                q.saved.projectiles.push(battle::Shot {
                    model: model.trim_end_matches(".tik").into(),
                    at: camera.eye + toward * 200.,
                    velocity: -toward * spec.speed,
                    age: 0.4,
                    damage: spec.damage,
                    force: spec.force,
                    life: spec.life,
                    end: None,
                    seek_at: projectile::first_seek(),
                    victim: Some(crate::dice::ALICE),
                    trail: Vec::new(),
                });
            }
            for frame in 0..3 {
                clear_background(BLACK);
                let cam = Camera3D {
                    position: camera.eye,
                    target: camera.target,
                    up: camera.up,
                    fovy: 75f32.to_radians(),
                    z_near: 2.,
                    z_far: 20000.,
                    ..Default::default()
                };
                set_camera(&cam);
                crate::lighting::select(q.lights(), camera.eye, &scene.world);
                crate::render_fx::begin_view(&cam, time, &scene.atmosphere, false);
                scene.draw(camera.eye, time, false, false, &q.transforms());
                art.draw(&q, &scene.atmosphere, camera.eye, false);
                crate::render::depth_read_only(|| {
                    scene.draw(camera.eye, time, false, true, &q.transforms());
                    art.effects(&q, camera.eye, &scene.atmosphere);
                });
                let (_, dropped) = crate::render_fx::finish();
                ensure!(
                    dropped == 0,
                    "Finale capture {name} exceeded the effect budget"
                );
                set_default_camera();
                art.hud(&q);
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/qlair-work/captures/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        println!("PASS finale staged render captures");
        Ok(())
    })
}

pub(crate) fn drive(r: &mut crate::route::Route) -> Result<()> { drive_inner(r,false) }
fn drive_inner(r: &mut crate::route::Route, second: bool) -> Result<()> {
    std::fs::create_dir_all("private/qlair-work")?;
    let difficulty = r.difficulty;
        r.tactics = true;
        r.heavy_weapon = Some(7);
        ensure!(
            !r.stats.god && !r.stats.notarget,
            "Combat proof cannot use cheats"
        );
        if second {
            let q = r
                .interactions
                .levels
                .iter_mut()
                .find_map(|s| s.ctl.downcast_mut::<Queen>())
                .unwrap();
            q.start(Phase::Birth);
            q.saved.queen1 = 0.;
            q.saved.collapsed = 20.;
            q.saved.initialized = true;
            q.skip(&r.map, &mut r.world, &mut r.player, &mut r.story)?;
            q.prepare_player(&mut r.stats, &mut r.player);
            println!("STAGED second-arena combat diagnostic");
        }
        for _ in 0..if second { 0 } else { 1200 } {
            r.tick(Controls {
                wish: Vec2::Y,
                ..Default::default()
            })?;
            if owner(&r.interactions).saved.phase == Phase::Queen1 {
                break;
            }
        }
        r.wait_for_cinematic()?;
        ensure!(
            owner(&r.interactions).saved.phase
                == if second { Phase::Queen2 } else { Phase::Queen1 },
            "Actual corridor contact failed at {:?}",
            r.player.feet
        );
        let mut corner = 0;
        let mut before_phase = Phase::Queen1;
        for step in 0..120000 {
            ensure!(
                r.ticks < 240000,
                "Finale route exceeded its time budget: phase {:?}, Queens {}/{}, sanity {}",
                owner(&r.interactions).saved.phase,
                owner(&r.interactions).saved.queen1,
                owner(&r.interactions).saved.queen2,
                r.stats.sanity()
            );
            let q = owner(&r.interactions);
            let phase = q.saved.phase;
            if phase == Phase::Done {
                ensure!(r.transition.is_none(), "Finale made a map transition");
                println!(
                    "PASS {} ENDING normal input route: {} throws, sanity {}, {} ticks",
                    difficulty.name(),
                    r.shots,
                    r.stats.sanity(),
                    r.ticks
                );
                break;
            }
            if phase == Phase::Queen1
                && q.saved.essence_wait == 0.
                && (r.stats.sanity() < 65. || r.stats.will() < 25.)
            {
                let item = q.essence_pose().translation;
                let high = item + Vec3::Z * 64.;
                let low = item - Vec3::Z * 128.;
                let floor = r.world.body_trace(high, low);
                let essence = high.lerp(low, floor.fraction);
                for goal in [
                    vec3(0., -850., 0.),
                    vec3(0., -1408., 0.),
                    essence,
                    vec3(0., -1408., 0.),
                    vec3(0., -850., 0.),
                ] {
                    if owner(&r.interactions).saved.phase != Phase::Queen1 {
                        break;
                    }
                    r.navigate_until_scene(goal)
                        .with_context(|| format!("Finale essence route to {goal:?}"))?;
                }
                println!(
                    "  collected essence: sanity {}, Will {}, Queen {}",
                    r.stats.sanity(),
                    r.stats.will(),
                    owner(&r.interactions).saved.queen1
                );
                corner = 0;
                continue;
            }
            if phase != before_phase {
                println!(
                    "  {:?}: {:?}, sanity {}",
                    phase,
                    r.player.feet,
                    r.stats.sanity()
                );
                before_phase = phase;
                corner = 0;
                if phase == Phase::Queen2 {
                    std::fs::write(
                        format!(
                            "private/qlair-work/{}-{}queen2.json",
                            difficulty.name(),
                            if second { "staged-" } else { "" }
                        ),
                        serde_json::to_vec(&r.checkpoint())?,
                    )?;
                }
            }
            if phase == Phase::Queen2 {
                r.aim_at = Some(BASE + 1);
                r.heavy_weapon = Some(7);
                corner = owner(&r.interactions).saved.essence as usize;
                let goals = [
                    vec3(-120., 3784., 984.),
                    vec3(960., 4896., 992.),
                    vec3(-16., 5768., 984.),
                    vec3(-1048., 4832., 928.),
                ];
                let origin = goals[corner % 4];
                let floor = r
                    .world
                    .body_trace(origin + Vec3::Z * 64., origin - Vec3::Z * 256.);
                let goal = (origin + Vec3::Z * 64.).lerp(origin - Vec3::Z * 256., floor.fraction);
                println!(
                    "  platform {}: goal {:?}, Queen {}, sanity {}",
                    corner % 4 + 1,
                    goal,
                    owner(&r.interactions).saved.queen2,
                    r.stats.sanity()
                );
                r.navigate_until_scene(goal)
                    .context("Second-arena platform traversal")?;
                if owner(&r.interactions).saved.phase != Phase::Queen2 {
                    continue;
                }
                if r.stats.sanity() < 100. || r.stats.will() < 100. {
                    for _ in 0..480 {
                        if owner(&r.interactions).saved.essence as usize != corner {
                            break;
                        }
                        if owner(&r.interactions).saved.phase != Phase::Queen2 {
                            break;
                        }
                        // The path can arrive at full running speed. Brake on the
                        // pedestal before jumping; air steering cannot cancel that
                        // momentum before Alice leaves the narrow landing.
                        let jump = r.player.grounded
                            && r.player.velocity.truncate().length() < 40.
                            && (r.player.feet.truncate() - goal.truncate())
                                .abs()
                                .max_element()
                                < 30.;
                        let wish = dodge(
                            &r,
                            goal,
                            Vec2::X,
                            ((goal.truncate() - r.player.feet.truncate()) / 30.
                                - r.player.velocity.truncate() / 320.)
                                .clamp_length_max(1.),
                            jump,
                        );
                        r.tick(Controls {
                            jump,
                            wish,
                            ..Default::default()
                        })?;
                    }
                }
                // Picking up floating essence can happen on the way up. Finish
                // the jump before asking the grounded route planner for a path.
                for _ in 0..240 {
                    if r.player.grounded || owner(&r.interactions).saved.phase != Phase::Queen2 {
                        break;
                    }
                    let wish = dodge(
                        &r,
                        goal,
                        Vec2::X,
                        ((goal.truncate() - r.player.feet.truncate()) / 30.
                            - r.player.velocity.truncate() / 320.)
                            .clamp_length_max(1.),
                        false,
                    );
                    r.tick(Controls {
                        wish,
                        ..Default::default()
                    })?;
                }
                if owner(&r.interactions).saved.phase != Phase::Queen2 {
                    continue;
                }
                // The western station has a narrow, exposed firing ledge. Take
                // its essence and keep circling instead of trading homing volleys
                // there; the southern platform provides room to finish the fight.
                if corner % 4 == 3 {
                    continue;
                }
                let axis = if corner % 2 == 0 { Vec2::X } else { Vec2::Y };
                let camp = arena_camp(&r.world, origin, axis)?;
                println!("  Fighting platform {camp:?}");
                r.navigate_until_scene(camp)
                    .context("Leave essence pedestal for arena platform")?;
                if owner(&r.interactions).saved.phase != Phase::Queen2 {
                    continue;
                }
                let mut strafe = 1.;
                let mut wish = Vec2::ZERO;
                for tick in 0..1800 {
                    if owner(&r.interactions).saved.phase != Phase::Queen2 {
                        break;
                    }
                    let from = r.player.feet.truncate() - camp.truncate();
                    if from.dot(axis) * strafe > 35. {
                        strafe = -strafe;
                    }
                    let radial = from - axis * from.dot(axis);
                    if tick % 12 == 0 {
                        wish = dodge(
                            &r,
                            camp,
                            axis,
                            (axis * strafe - radial / 40.).clamp_length_max(1.),
                            false,
                        );
                    }
                    r.tick(Controls {
                        wish,
                        run: false,
                        ..Default::default()
                    })
                    .with_context(|| {
                        format!(
                            "Queen2 dodge failed: sanity {}, Queen {}, attack {:?}, body {:?}",
                            r.stats.sanity(),
                            owner(&r.interactions).saved.queen2,
                            owner(&r.interactions).saved.attack,
                            owner(&r.interactions).saved.body.translation
                        )
                    })?;
                    // Refill once a volley clears, unless the remaining Staff
                    // charge can finish the body. Crossing a bridge at low health
                    // is riskier than landing those last few hits from the camp.
                    let can_finish = owner(&r.interactions).saved.queen2 - 995.
                        <= r.stats.will() * 12.5 * difficulty.outgoing();
                    if tick > 180
                        && !can_finish
                        && (r.stats.will() < 10. || r.stats.sanity() < 65.)
                        && owner(&r.interactions).saved.projectiles.is_empty()
                        && owner(&r.interactions).saved.attack == battle::Attack::Idle
                        && owner(&r.interactions)
                            .saved
                            .part_attack
                            .iter()
                            .all(Option::is_none)
                    {
                        break;
                    }
                }
                println!(
                    "  platform fight: Queen {}, parts {:?}, sanity {}, Will {}",
                    owner(&r.interactions).saved.queen2,
                    owner(&r.interactions).saved.parts,
                    r.stats.sanity(),
                    r.stats.will()
                );
                continue;
            }
            let goals = if phase == Phase::Queen1 {
                vec![
                    vec2(-160., -850.),
                    vec2(160., -850.),
                    vec2(160., -600.),
                    vec2(-160., -600.),
                ]
            } else {
                vec![
                    vec2(-1048., 4832.),
                    vec2(-1040., 5400.),
                    vec2(-16., 5768.),
                    vec2(850., 5480.),
                    vec2(960., 4896.),
                    vec2(800., 4070.),
                    vec2(-120., 3784.),
                    vec2(-1080., 3928.),
                ]
            };
            let goal = goals[corner % goals.len()];
            let delta = goal - r.player.feet.truncate();
            if delta.length() < 45. {
                corner += 1;
            }
            r.aim_at = None;
            r.tick(Controls {
                jump: step % 180 == 0,
                run: true,
                wish: if matches!(phase, Phase::Queen1 | Phase::Queen2) {
                    delta.normalize_or_zero()
                } else {
                    Vec2::ZERO
                },
                ..Default::default()
            })
            .with_context(|| {
                format!(
                    "{} {:?} at {:?}, queens {}/{}, attack {:?}",
                    difficulty.name(),
                    phase,
                    r.player.feet,
                    owner(&r.interactions).saved.queen1,
                    owner(&r.interactions).saved.queen2,
                    owner(&r.interactions).saved.attack
                )
            })?;
            ensure!(
                r.stats.alive(),
                "{} route died at {:?}, phase {:?}, queens {}/{}",
                difficulty.name(),
                r.player.feet,
                phase,
                owner(&r.interactions).saved.queen1,
                owner(&r.interactions).saved.queen2
            );
            if step % 12000 == 0 {
                println!(
                    "  fight {:?} {:?} health {}/{} sanity {}",
                    phase,
                    r.player.feet,
                    owner(&r.interactions).saved.queen1,
                    owner(&r.interactions).saved.queen2,
                    r.stats.sanity()
                );
            }
        }
        ensure!(
            owner(&r.interactions).ending_ready(),
            "Finale route timed out"
        );
    Ok(())
}
