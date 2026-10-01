use super::*;
use crate::movement::{Controls, FIXED_DT};
fn owner(r: &crate::route::Route) -> &Centipede {
    r.interactions
        .levels
        .iter()
        .find_map(|s| s.ctl.downcast_ref::<Centipede>())
        .unwrap()
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/centipede2.bsp")?)?;
    let mut o = Centipede::load(a, &map)?;
    let exit_trigger = TriggerInfo {
        id: crate::entity::Id(43),
        name: "c2_changelevel",
        class: TriggerClass::Exit,
        thread: "",
        exit: Some("wforest"),
        target: None,
    };
    let intro = o.snapshot();
    o.event(GROW);
    o.event("Centipede2_DropSpike");
    o.dialogue_complete(TALK1);
    ensure!(
        o.snapshot() == intro,
        "An out-of-order callback bypassed the boss"
    );
    ensure!(
        !o.gate(&exit_trigger).unwrap().test(&o.facts()),
        "Old bypass enabled"
    );
    println!(
        "Centipede floor {:?}, intro end {}, brushes {}",
        o.saved.battle.pose.translation,
        o.slide_start() + 3.5,
        o.data.brushes.len()
    );
    for clip in [
        "walk_fast",
        "walk",
        "pain_front",
        "attack_crush",
        "attack_grab_strike",
        "attack_grab_shake",
        "death_start",
        "death_thrash",
    ] {
        println!(
            "  {clip}: {}s, frame {}s",
            o.data.boss().duration(clip),
            o.data.boss().frame(clip)
        );
    }
    let mut w = World::from_bsp(&map)?;
    let mut p = Player::spawn(&w, o.data.points["centipede2_start1"].translation)
        .context("Entry obstructed")?;
    o.finish_intro(&w, &mut p)?;
    o.event("Centipede2_MinHealth_Reset");
    let mut stats = Stats::for_level("centipede2", None);
    stats.god = true;
    let mut story = Story::load(a, "centipede2");
    let base = o.snapshot();
    for hz in [30, 60, 144] {
        for action in [
            battle::Action::Spit,
            battle::Action::Larvae,
            battle::Action::Shake,
        ] {
            o.restore(&base, &map)?;
            o.saved.battle.set(action);
            p.feet = o.saved.battle.pose.translation + vec3(-300., 0., 0.);
            if action == battle::Action::Shake {
                o.saved.battle.grabbed = true;
                o.saved.battle.grab_feet = p.feet;
                o.saved.battle.victim_feet = p.feet;
            }
            for _ in 0..hz {
                o.combat(&mut Combat {
                    dt: 1. / hz as f32,
                    world: &w,
                    player: &mut p,
                    stats: &mut stats,
                    story: &mut story,
                    notarget: true,
                    summon: None,
                    threatens: &|_| false,
                });
            }
            if action == battle::Action::Larvae {
                ensure!(
                    o.saved.battle.larvae.len() == 2,
                    "{hz}Hz lost/duplicated larvae"
                );
            }
            if action == battle::Action::Spit {
                ensure!(!o.saved.battle.shots.is_empty(), "{hz}Hz lost acid shots");
            }
            if action == battle::Action::Shake {
                ensure!(
                    !o.saved.battle.grabbed && p.velocity.length() > 500.,
                    "{hz}Hz failed grab release"
                );
            }
            let snapshot = o.snapshot();
            o.combat(&mut Combat {
                dt: 0.,
                world: &w,
                player: &mut p,
                stats: &mut stats,
                story: &mut story,
                notarget: true,
                summon: None,
                threatens: &|_| false,
            });
            ensure!(snapshot == o.snapshot(), "Paused boss advanced");
            o.restore(&snapshot, &map)?;
        }
    }
    o.restore(&base, &map)?;
    o.saved.battle.set(battle::Action::Crush);
    o.saved.battle.time = 25. * o.data.boss().frame("attack_crush");
    o.hit(Hit {
        id: BASE,
        damage: 1000.,
        kind: crate::combat::DamageKind::Blunderbuss,
        knockback: Vec3::ZERO,
    });
    ensure!(
        o.saved.battle.hits == 0,
        "Late splash bypassed the closed weak point"
    );
    for hit in 0..6 {
        let b = &mut o.saved.battle;
        b.set(battle::Action::Crush);
        let h = Hit {
            id: BASE,
            damage: 45.,
            kind: crate::combat::DamageKind::Knife,
            knockback: Vec3::ZERO,
        };
        b.hit(
            Hit {
                kind: crate::combat::DamageKind::Fire,
                ..h
            },
            &o.data,
        );
        ensure!(b.hits == hit, "Fire bypassed immunity");
        b.hit(Hit { id: BASE + 1, ..h }, &o.data);
        ensure!(b.hits == hit, "Armor accepted damage");
        b.hit(h, &o.data);
        b.hit(h, &o.data);
        ensure!(b.hits == hit + 1, "Weak spot hit counted twice");
        for _ in 0..3000 {
            let mut c = Combat {
                dt: FIXED_DT,
                world: &w,
                player: &mut p,
                stats: &mut stats,
                story: &mut story,
                notarget: true,
                summon: None,
                threatens: &|_| false,
            };
            o.combat(&mut c);
            if matches!(
                o.saved.battle.action,
                battle::Action::Walk | battle::Action::Dead
            ) {
                break;
            }
        }
        let v = o.snapshot();
        o.restore(&v, &map)?;
        ensure!(
            o.saved.battle.stage == ((hit + 1) / 2 + 1).min(3),
            "Stage progression failed"
        );
    }
    o.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
    ensure!(o.saved.phase == Phase::Drop, "No defeat scene");
    for _ in 0..1000 {
        o.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
    }
    ensure!(o.saved.phase == Phase::Climb, "No climbing route");
    ensure!(
        !o.gate(&exit_trigger).unwrap().test(&o.facts()),
        "Defeat enabled old bypass"
    );
    ensure!(
        o.poses.iter().any(|(m, _, _)| *m == 2),
        "Missing climbing collider"
    );
    o.event(GROW);
    o.dialogue_complete(TALK1);
    for _ in 0..5000 {
        o.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
        if o.saved.phase == Phase::Done {
            break;
        }
    }
    ensure!(
        o.update(&mut w, &p, Vec3::X, false).transition == Some(EXIT.destination()),
        "Mushroom exit failed"
    );
    ensure!(
        o.update(&mut w, &p, Vec3::X, false).transition.is_none(),
        "Duplicate exit"
    );
    o.transition_failed(&EXIT.destination());
    for _ in 0..121 {
        o.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
    }
    ensure!(
        o.update(&mut w, &p, Vec3::X, false).transition.is_some(),
        "Exit retry failed"
    );
    o.restore(&base, &map)?;
    let mut dead = stats.clone();
    dead.god = false;
    dead.damage(100000.);
    o.prepare_player(&mut dead, &mut p);
    o.saved.battle.hits = 6;
    o.saved.battle.stage = 3;
    o.saved.battle.set(battle::Action::Dead);
    let before = o.snapshot();
    o.advance(0.1, &map, &mut w, &mut p, &[])?;
    ensure!(before == o.snapshot(), "Dead Alice started victory");
    // Intro playback and every skip point produce the same slide handoff at different FPS.
    for hz in [30, 60, 144] {
        for skip in [None, Some(0.2), Some(9.), Some(16.)] {
            o.restore(&intro, &map)?;
            o.alive = true;
            for _ in 0..hz * 30 {
                if skip.is_some_and(|t| o.saved.time >= t) {
                    o.skip(&map, &mut w, &mut p, &mut story)?;
                } else {
                    o.advance(1. / hz as f32, &map, &mut w, &mut p, &[])?;
                }
                if o.saved.phase == Phase::Slide {
                    break;
                }
            }
            ensure!(
                o.saved.phase == Phase::Slide
                    && p.feet
                        .distance(o.data.points["alice_slide_pos1"].translation)
                        < 8.,
                "{hz}Hz intro handoff failed"
            );
            ensure!(
                o.poses.iter().any(|(m, _, _)| *m == 1) && !o.poses.iter().any(|(m, _, _)| *m == 2),
                "Intro geometry differs"
            );
        }
    }
    println!("PASS Centipede six weak hits, fire/armor immunity, stages, collapse, climb and transactional mushroom exit");
    Ok(())
}
pub(super) fn route(a: &mut Assets) -> Result<()> {
    for (difficulty, skip) in [
        (crate::powerups::Difficulty::Normal, false),
        (crate::powerups::Difficulty::Hard, true),
    ] {
        route_one(a, difficulty, skip)?;
    }
    Ok(())
}
pub(crate) fn drive(_a: &mut Assets, r: &mut crate::route::Route) -> Result<()> {
    let skip = r.skip_cinematics;
    let difficulty = r.difficulty;
    r.tactics = true;
    r.aim_at = Some(BASE);
    r.stop_at_exit = true;
    for _ in 0..10000 {
        r.tick(Controls {
            wish: Vec2::X,
            ..Default::default()
        })?;
        if owner(&r).saved.phase == Phase::Fight {
            break;
        }
    }
    println!(
        "Arena {:?}, sanity {}, phase {:?}",
        r.player.feet,
        r.stats.sanity(),
        owner(&r).saved.phase
    );
    ensure!(
        owner(&r).saved.phase == Phase::Fight,
        "Slide did not reach arena"
    );
    let mut hits = 0;
    for tick in 0..100000 {
        let o = owner(&r);
        let b = &o.saved.battle;
        if o.saved.phase != Phase::Fight {
            break;
        }
        let delta = (b.pose.translation - r.player.feet).truncate();
        let wish = if delta.length() > 390. {
            delta.normalize_or_zero()
        } else if delta.length() < 350. {
            -delta.normalize_or_zero()
        } else {
            Vec2::ZERO
        };
        r.aim_at = if b.weak(&o.data) {
            Some(BASE)
        } else {
            b.larvae
                .iter()
                .enumerate()
                .filter(|(_, l)| l.vulnerable())
                .min_by(|(_, a), (_, b)| {
                    a.feet
                        .distance_squared(r.player.feet)
                        .total_cmp(&b.feet.distance_squared(r.player.feet))
                })
                .map(|(i, _)| BASE + 100 + i)
                .or(Some(BASE))
        };
        r.tick(Controls {
            wish,
            ..Default::default()
        })?;
        let b = &owner(&r).saved.battle;
        if b.hits != hits {
            hits = b.hits;
            println!(
                "  hit {hits}, stage {}, sanity {}, at {:?}",
                b.stage,
                r.stats.sanity(),
                r.player.feet
            );
        }
        if tick % 3600 == 0 {
            println!(
                "  fight {tick}, {:?} {}, boss {:?}, Alice {:?}",
                b.action, b.time, b.pose.translation, r.player.feet
            );
        }
        ensure!(r.stats.alive(), "Route died after {hits} weak hits");
    }
    ensure!(
        owner(&r).saved.battle.hits == 6,
        "Route could not defeat Centipede"
    );
    for _ in 0..2000 {
        r.tick(Controls::default())?;
        if owner(&r).saved.phase == Phase::Climb {
            break;
        }
    }
    println!("Climb begins at {:?}", r.player.feet);
    let goals = if let Ok(path) = std::env::var("LOOKING_GLASS_CENTIPEDE2_ROUTE") {
        serde_json::from_slice::<Vec<Vec3>>(&std::fs::read(path)?)?
    } else {
        vec![
            vec3(-2880., 4080., 2848.),
            vec3(-2760., 4080., 2956.),
            vec3(-2728., 4120., 2960.),
            vec3(-2600., 4320., 3012.),
            vec3(-2568., 4320., 3016.),
            vec3(-2328., 4296., 3148.),
            vec3(-2328., 4210., 3152.),
            vec3(-2290., 3980., 3268.),
            vec3(-2184., 3904., 3268.),
        ]
    };
    for goal in goals {
        println!("  climb goal {goal:?}");
        walk_goal(r, goal)?;
        println!("  reached {:?}", r.player.feet);
    }
    for _ in 0..10000 {
        r.tick(Controls::default())?;
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition == Some(EXIT.destination()),
        "Route did not reach mushroom exit"
    );
    println!(
        "PASS {} Centipede route (skip={skip}), {} throws / {} cards / {} swings, sanity {}, ticks {}",
        difficulty.name(),
        r.shots,
        r.cards,
        r.swings,
        r.stats.sanity(),
        r.ticks
    );
    Ok(())
}
fn route_one(a: &mut Assets, difficulty: crate::powerups::Difficulty, skip: bool) -> Result<()> {
    let mut r = crate::route::Route::enter(
        a,
        "centipede2",
        Some("centipede2_start1"),
        Stats::for_level("centipede2", None),
        difficulty,
    )?;
    r.skip_cinematics = skip;
    drive(a, &mut r)?;
    let inventory = (
        r.stats.sanity(),
        r.stats.will(),
        (0..10).map(|n| r.stats.copies(n)).collect::<Vec<_>>(),
        r.stats.turtle_air,
    );
    let next = r.depart(a, true)?;
    ensure!(
        next.level().map == "wforest" && next.world.body_clear(next.player.feet),
        "Mushroom destination did not load"
    );
    ensure!(
        inventory
            == (
                next.stats.sanity(),
                next.stats.will(),
                (0..10).map(|n| next.stats.copies(n)).collect::<Vec<_>>(),
                next.stats.turtle_air
            ),
        "Departure changed carried resources"
    );
    println!("PASS wforest arrival with carried resources and inventory");
    Ok(())
}
fn walk_goal(r: &mut crate::route::Route, goal: Vec3) -> Result<()> {
    for tick in 0..2400 {
        if r.interactions.scripted() {
            return Ok(());
        }
        let d = goal - r.player.feet;
        if d.truncate().length() < 10. && d.z.abs() < 18. && r.player.grounded {
            return Ok(());
        }
        let wish = d.truncate().normalize_or_zero() * (d.truncate().length() / 60.).min(1.);
        let jump = r.player.ledge.is_some()
            || r.player.grounded && d.z > 24. && d.truncate().length() < 330.;
        r.tick(Controls {
            wish,
            jump,
            run: true,
            ..Default::default()
        })?;
        if tick % 240 == 0 {
            println!(
                "    walk {:?} ledge={} grounded={}",
                r.player.feet,
                r.player.ledge.is_some(),
                r.player.grounded
            );
        }
    }
    anyhow::bail!(
        "Climb input did not reach {goal:?} from {:?}",
        r.player.feet
    )
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        std::fs::create_dir_all("private/centipede2-work/captures")?;
        let mut scene = crate::render::Scene::load(a, "centipede2")?;
        let mut o = Centipede::load(a, &scene.map)?;
        let mut art = art::Art::load(a, &o)?;
        for (name, phase, action, t) in [
            ("arrival", Phase::Intro, battle::Action::Walk, 2.),
            ("weakpoint", Phase::Fight, battle::Action::Crush, 0.6),
            ("wounded", Phase::Fight, battle::Action::Walk, 0.5),
            ("acid", Phase::Fight, battle::Action::Spit, 0.8),
            ("larvae", Phase::Fight, battle::Action::Larvae, 1.6),
            ("grab", Phase::Fight, battle::Action::Strike, 0.8),
            ("spikes", Phase::Drop, battle::Action::Dead, 2.),
            ("climb", Phase::Climb, battle::Action::Dead, 0.),
            ("growth", Phase::Grow, battle::Action::Dead, 4.),
        ] {
            o.saved.phase = phase;
            o.saved.time = t;
            o.saved.battle.set(action);
            o.saved.battle.time = t;
            o.saved.battle.hits = if name == "wounded" { 3 } else { 0 };
            o.saved.battle.stage = if name == "wounded" { 2 } else { 1 };
            o.saved.battle.shots.clear();
            o.saved.battle.larvae.clear();
            o.saved.battle.grabbed = false;
            if matches!(name, "acid" | "larvae" | "grab") {
                o.saved.battle.set(action);
                let mut player = Player::new(
                    o.saved.battle.pose.translation
                        + if name == "grab" {
                            vec3(-280., 0., 0.)
                        } else {
                            vec3(-800., -800., 0.)
                        },
                );
                let mut stats = Stats::default();
                let mut story = Story::load(a, "centipede2");
                o.saved.battle.grab_camera = player.feet + vec3(80., 280., 200.);
                for _ in 0..(t / FIXED_DT).round() as usize {
                    o.combat(&mut Combat {
                        dt: FIXED_DT,
                        world: &scene.world,
                        player: &mut player,
                        stats: &mut stats,
                        story: &mut story,
                        notarget: false,
                        summon: None,
                        threatens: &|_| false,
                    });
                }
                if name == "larvae" {
                    ensure!(o.saved.battle.larvae.len() == 2, "Missing rendered Larvae");
                }
            }
            o.rebuild(&scene.map)?;
            let camera = o.camera(&scene.world).unwrap_or_else(|| {
                crate::cinematic::Camera::look(
                    o.saved.battle.pose.translation + vec3(-480., -480., 140.),
                    o.saved.battle.pose.translation + Vec3::Z * 150.,
                )
            });
            for frame in 0..3 {
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
                crate::lighting::select(o.lights(), camera.eye, &scene.world);
                crate::render_fx::begin_view(&cam, t, &scene.atmosphere, false);
                scene.draw(camera.eye, t, false, false, &o.transforms());
                art.draw(&o, &scene.atmosphere, camera.eye, false);
                crate::render::depth_read_only(|| {
                    scene.draw(camera.eye, t, false, true, &o.transforms());
                    art.effects(&o, camera.eye, &scene.atmosphere);
                });
                let (_, dropped) = crate::render_fx::finish();
                ensure!(dropped == 0, "Centipede effect budget exceeded");
                set_default_camera();
                art.hud(&o);
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/centipede2-work/captures/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        println!("PASS Centipede native render captures");
        Ok(())
    })
}
