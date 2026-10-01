use super::*;
use crate::{movement::Controls, route::Route};
pub(super) fn owner(r: &Route) -> &Flora {
    r.interactions
        .levels
        .iter()
        .find_map(|s| s.ctl.downcast_ref::<Flora>())
        .unwrap()
}
pub(super) fn owner_mut(r: &mut Route) -> &mut Flora {
    r.interactions
        .levels
        .iter_mut()
        .find_map(|s| s.ctl.downcast_mut::<Flora>())
        .unwrap()
}
fn step(
    o: &mut Flora,
    map: &Bsp,
    w: &mut World,
    p: &mut Player,
    s: &mut Stats,
    story: &mut Story,
    dt: f32,
) -> Result<()> {
    o.prepare_player(s, p);
    o.advance(dt, map, w, p, &[])?;
    w.set_dynamic(o.colliders());
    o.prepare_story(story);
    story.tick(dt, false);
    o.sync_story(story);
    for done in story.take_completed() {
        o.dialogue_complete(&done);
    }
    Ok(())
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/centipede1.bsp")?)?;
    let mut w = World::from_bsp(&map)?;
    let start = vec3(-280., 4700., -600.);
    let mut reference = None;
    for hz in [30, 60, 144] {
        for skip in [None, Some(2.1), Some(3.4), Some(5.)] {
            let mut o = Flora::load(a, &map)?;
            w.set_dynamic(o.colliders());
            let mut p = Player::spawn(&w, start).context("Ambush fixture blocked")?;
            let mut stats = Stats::for_level("centipede1", None);
            stats.set_health(54.)?;
            let mut story = Story::load(a, "centipede1");
            let fresh = o.snapshot();
            o.event("C1_End");
            o.dialogue_complete(BEAT);
            ensure!(
                o.snapshot() == fresh,
                "Out-of-order callback escaped the forest"
            );
            o.prepare_player(&mut stats, &mut p);
            o.event(AMBUSH);
            ensure!(
                !o.skip(&map, &mut w, &mut p, &mut story)?,
                "Skipped the unskippable rush"
            );
            let mut skipped = false;
            for tick in 0..hz * 120 {
                if skip.is_some_and(|t| tick as f32 / hz as f32 >= t) && !skipped {
                    skipped = o.skip(&map, &mut w, &mut p, &mut story)?;
                }
                if o.saved.phase == Phase::Countdown {
                    break;
                }
                step(
                    &mut o,
                    &map,
                    &mut w,
                    &mut p,
                    &mut stats,
                    &mut story,
                    1. / hz as f32,
                )?;
            }
            ensure!(
                o.saved.phase == Phase::Countdown,
                "Ambush did not hand back control"
            );
            ensure!(stats.sanity() == 54., "Scene refilled or damaged Alice");
            ensure!(story.has_seen(BEAT), "Skip lost dialogue completion");
            ensure!(
                !o.reveal() && o.saved.ants.iter().all(|s| !s.ant.script_wait),
                "Ambush left hidden AI locked"
            );
            let mut equivalent = o.snapshot();
            equivalent["clock"] = serde_json::json!(0.);
            equivalent["speech"] = serde_json::json!(0.);
            // Skips commit before the next simulation step; watched completion commits on it.
            equivalent["time"] = serde_json::json!(0.);
            if let Some(ref before) = reference {
                ensure!(
                    &equivalent == before,
                    "Watched/skipped ambush outcomes differ at {hz}Hz"
                );
            } else {
                reference = Some(equivalent);
            }
            let paused = o.snapshot();
            step(&mut o, &map, &mut w, &mut p, &mut stats, &mut story, 0.)?;
            ensure!(o.snapshot() == paused, "Paused ambush advanced");
            let saved = o.snapshot();
            o.restore(&saved, &map)?;
            o.prepare_player(&mut stats, &mut p);
            stats.damage(10000.);
            ensure!(stats.sanity() == 10., "Countdown lost minimum sanity");
            let mut exit_ticks = 0;
            while o.saved.phase != Phase::Done && exit_ticks < hz * 11 {
                step(
                    &mut o,
                    &map,
                    &mut w,
                    &mut p,
                    &mut stats,
                    &mut story,
                    1. / hz as f32,
                )?;
                exit_ticks += 1;
            }
            ensure!(
                (exit_ticks as i32 - hz as i32 * 10).abs() <= 1,
                "{hz}Hz countdown length {exit_ticks}"
            );
            ensure!(o.saved.wave == 4, "Missing timed wave attempt");
            ensure!(
                o.saved.ants.iter().any(|s| s.group == 1),
                "Spawn chain produced no ambush pressure"
            );
            let exit = o.update(&mut w, &p, Vec3::X, false).transition;
            ensure!(exit == Some(EXIT.destination()), "Wrong destination");
            ensure!(
                o.update(&mut w, &p, Vec3::X, false).transition.is_none(),
                "Exit repeated"
            );
            o.transition_failed(&EXIT.destination());
            ensure!(
                o.update(&mut w, &p, Vec3::X, false).transition.is_none(),
                "Exit retry had no delay"
            );
            for _ in 0..11 {
                o.advance(0.1, &map, &mut w, &mut p, &[])?;
            }
            ensure!(
                o.update(&mut w, &p, Vec3::X, false).transition.is_some(),
                "Failed departure cannot retry"
            );
            let good = o.snapshot();
            let mut bad = good.clone();
            bad["wave"] = serde_json::json!(50);
            ensure!(
                o.restore(&bad, &map).is_err() && o.snapshot() == good,
                "Invalid restore mutated the arena"
            );
        }
    }
    // Living counters are derived from actors: ordinary damage frees a brave slot.
    let mut o = Flora::load(a, &map)?;
    let mut p = Player::spawn(&w, start).unwrap();
    o.phase(Phase::Reveal);
    o.commit(&w, &mut p);
    let id = o
        .saved
        .ants
        .iter()
        .find(|s| s.group == 1)
        .context("No brave Ant")?
        .ant
        .id;
    o.hit(Hit {
        id: BASE + id,
        damage: 1000.,
        kind: crate::combat::DamageKind::Knife,
        knockback: Vec3::ZERO,
    });
    ensure!(
        o.saved
            .ants
            .iter()
            .find(|s| s.ant.id == id)
            .unwrap()
            .ant
            .health
            == 0.,
        "Wave enemy cannot die"
    );
    o.spawn_wave(&w, p.feet + Vec3::Z * 32.);
    ensure!(
        o.saved
            .ants
            .iter()
            .filter(|s| s.group == 1 && s.ant.health > 0.)
            .count()
            == 1,
        "Killed brave kept its live slot"
    );
    // The killer chain is fully visible from the trigger and correctly produces nothing there.
    // Moving behind the forest geometry permits it to spawn; facing never enters the decision.
    o.spawn_wave(&w, vec3(1136., -1712., -400.));
    ensure!(
        o.saved.ants.iter().any(|s| s.group == 2),
        "Hidden killer chain did not spawn"
    );
    let mut runners = Flora::load(a, &map)?;
    let runner = *runners.data.runners.keys().next().unwrap();
    runners.hit(Hit {
        id: BASE + runner,
        damage: 1000.,
        kind: crate::combat::DamageKind::Knife,
        knockback: Vec3::ZERO,
    });
    let corpse = runners
        .saved
        .ants
        .iter()
        .find(|s| s.ant.id == runner)
        .unwrap()
        .ant
        .feet;
    runners.phase(Phase::Reveal);
    runners.commit(&w, &mut p);
    let killed = &runners
        .saved
        .ants
        .iter()
        .find(|s| s.ant.id == runner)
        .unwrap()
        .ant;
    ensure!(
        killed.health == 0. && killed.feet == corpse && !killed.script_wait,
        "Ambush resurrected or moved a dead runner"
    );
    let mut dead = Stats::for_level("centipede1", None);
    dead.set_health(0.)?;
    o.prepare_player(&mut dead, &mut p);
    let before = o.snapshot();
    o.advance(1., &map, &mut w, &mut p, &[])?;
    ensure!(
        before == o.snapshot(),
        "Dead Alice completed the timed exit"
    );
    println!("PASS Flora watched/skip equivalence, 30/60/144Hz, waves, damage, pause, validation and exit retry");
    Ok(())
}
pub(super) fn route(a: &mut Assets) -> Result<()> {
    walk(a, false)
}
pub(super) fn skip_route(a: &mut Assets) -> Result<()> {
    walk(a, true)
}
fn walk(a: &mut Assets, skip: bool) -> Result<()> {
    for difficulty in [
        crate::powerups::Difficulty::Normal,
        crate::powerups::Difficulty::Hard,
    ] {
        walk_one(a, skip, difficulty)?;
    }
    Ok(())
}
pub(crate) fn drive(a: &mut Assets, r: &mut crate::route::Route) -> Result<()> {
    let skip = r.skip_cinematics;
    r.tactics = true;
    r.stop_at_exit = true;
    let goals: Vec<[f32; 3]> = if let Ok(path) = std::env::var("LOOKING_GLASS_CENTIPEDE1_ROUTE") {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else {
        vec![
            [1320., -1976., -496.],
            [857., -1653., -491.],
            [1136., -1200., -500.],
            [1136., -640., -560.],
            [576., -256., -600.],
            [-476., 84., -608.],
            [-468., 1024., -560.],
            [1244., 1588., -576.],
            [640., 2816., -560.],
            [578., 3294., -428.],
            [-256., 4100., -680.],
            [-280., 4820., -704.],
        ]
    };
    for xyz in goals {
        let guess = Vec3::from_array(xyz);
        let goal = [0., 64., 128., -64.]
            .into_iter()
            .find_map(|z| {
                r.world.actor_footing(
                    guess + Vec3::Z * z,
                    crate::collision::PLAYER_CENTER,
                    crate::collision::PLAYER_HALF,
                    512.,
                )
            })
            .with_context(|| format!("No walkable support for {guess:?}"))?;
        println!("Flora route goal {goal:?} from {:?}", r.player.feet);
        r.navigate_until_scene(goal)?;
        if owner(&r).saved.phase != Phase::Explore {
            break;
        }
    }
    ensure!(
        owner(&r).saved.phase != Phase::Explore,
        "Walking route missed ambush trigger"
    );
    ensure!(
        r.stats.copies(6) == 2 && r.stats.collected.contains("centipede1:299"),
        "Missed second Demon Die"
    );
    ensure!(r.story.has_seen(TALK), "Missed Centipede voice-over");
    ensure!(
        owner(&r)
            .saved
            .ants
            .iter()
            .filter(|s| matches!(s.ant.id, 36 | 201 | 228 | 229))
            .all(|s| s.ant.enabled),
        "ants-01 did not activate"
    );
    let mut did_skip = false;
    let mut resumed = false;
    let mut last_phase = owner(&r).saved.phase;
    for _ in 0..120 * 90 {
        if skip && !did_skip && owner(&r).skippable() {
            did_skip =
                r.interactions
                    .skip_cinematic(&r.map, &mut r.world, &mut r.player, &mut r.story)?;
        }
        r.tick(Controls::default())?;
        ensure!(r.stats.alive(), "Flora route died");
        let phase = owner(&r).saved.phase;
        if phase != last_phase {
            println!(
                "  {phase:?}, sanity {}, ants {}",
                r.stats.sanity(),
                owner(&r).saved.ants.len()
            );
            last_phase = phase;
        }
        if !resumed && phase == Phase::Countdown && owner(&r).saved.time >= 3.2 {
            // Save during real combat, then continue both timelines with identical inputs.
            r.quiet();
            let mut loaded = Route::resume(a, &r.checkpoint())?;
            loaded.quiet();
            loaded.stop_at_exit = true;
            ensure!(
                crate::route::run_identical(r, &mut loaded, 120 * 8)?.is_none(),
                "Saved ambush continuation failed"
            );
            ensure!(
                r.transition == Some(EXIT.destination())
                    && loaded.transition == r.transition
                    && r.stats.alive()
                    && r.stats.sanity() >= 10.,
                "Saved ambush lost its floor or timed exit"
            );
            println!("  PASS live countdown save/restore: identical combat, resources and exit");
            resumed = true;
        }
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition == Some(EXIT.destination()) && (!skip || did_skip) && resumed,
        "Flora timed exit did not complete"
    );
    Ok(())
}
fn walk_one(a: &mut Assets, skip: bool, difficulty: crate::powerups::Difficulty) -> Result<()> {
    let mut r = Route::enter(
        a,
        "centipede1",
        Some("centipede1_start1"),
        Stats::for_level("centipede1", None),
        difficulty,
    )?;
    r.skip_cinematics = skip;
    drive(a, &mut r)?;
    let carried = (
        r.stats.sanity(),
        r.stats.will(),
        (0..10).map(|n| r.stats.copies(n)).collect::<Vec<_>>(),
        r.stats.turtle_air,
    );
    let ticks = r.ticks;
    let mut next = r.depart(a, true)?;
    ensure!(
        next.level().map == "centipede2" && next.world.body_clear(next.player.feet),
        "Sanctum entrance obstructed"
    );
    ensure!(
        carried
            == (
                next.stats.sanity(),
                next.stats.will(),
                (0..10).map(|n| next.stats.copies(n)).collect(),
                next.stats.turtle_air
            ),
        "Departure changed carried resources"
    );
    ensure!(next.interactions.scripted(), "Sanctum intro did not start");
    next.interactions
        .prepare_player(&mut next.stats, &mut next.player);
    ensure!(
        next.stats.minimum_sanity == 0.,
        "Forest sanity floor leaked into Sanctum intro"
    );
    next.wait_for_cinematic()?;
    ensure!(
        !next.interactions.scripted() && next.world.body_clear(next.player.feet),
        "Sanctum intro failed to hand back control"
    );
    println!("PASS {difficulty:?} Flora route skip={skip}, {ticks} ticks, normal Sanctum arrival/intro, inventory and resources preserved");
    Ok(())
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        std::fs::create_dir_all("private/centipede1-work/captures")?;
        let mut scene = crate::render::Scene::load(a, "centipede1")?;
        let mut art = art::Art::load(a)?;
        let ui = crate::ui::Ui::load(a)?;
        let mut npcs = crate::npc::Npcs::load(a, &scene.map, "centipede1", None, false, false)?;
        for (name, phase, time) in [
            ("forest", Phase::Explore, 0.),
            ("rush", Phase::Rush, 1.5),
            ("reveal", Phase::Reveal, 2.),
            ("waves", Phase::Countdown, 6.3),
            ("lid", Phase::Explore, 0.),
        ] {
            let mut o = Flora::load(a, &scene.map)?;
            o.saved.phase = phase;
            o.saved.time = time;
            o.saved.alice.translation = vec3(-280., 4760., -768.);
            if phase == Phase::Rush {
                for s in &mut o.saved.ants {
                    if let Some(p) = o.data.runners.get(&s.ant.id) {
                        s.ant.feet = s.ant.feet.lerp(p.translation, 0.7);
                        s.ant.time = time as f32;
                    }
                }
            }
            if phase == Phase::Countdown {
                let mut player = Player::new(o.saved.alice.translation);
                let mut stats = Stats::for_level("centipede1", None);
                let mut story = Story::load(a, "centipede1");
                o.saved.phase = Phase::Reveal;
                o.commit(&scene.world, &mut player);
                for _ in 0..(time * 120.) as usize {
                    o.advance(1. / 120., &scene.map, &mut scene.world, &mut player, &[])?;
                    o.combat(&mut Combat {
                        dt: 1. / 120.,
                        world: &scene.world,
                        player: &mut player,
                        stats: &mut stats,
                        story: &mut story,
                        notarget: false,
                        summon: None,
                        threatens: &|_| false,
                    });
                }
            }
            let mut story = Story::load(a, "centipede1");
            if phase == Phase::Reveal {
                story.trigger(BEAT);
                story.tick(1., false);
                art.story_pose(&story);
            }
            scene.atmosphere.distance = o.scene_fog().unwrap();
            let camera = if name == "lid" {
                crate::cinematic::Camera::look(
                    vec3(-300., 6100., -360.),
                    o.data.lid_pose.translation,
                )
            } else if name == "forest" {
                crate::cinematic::Camera::look(
                    vec3(1136., -1800., -350.),
                    vec3(1100., -700., -550.),
                )
            } else {
                o.camera(&scene.world).unwrap_or_else(|| {
                    crate::cinematic::Camera::look(
                        vec3(-550., 5200., -420.),
                        vec3(-180., 5500., -620.),
                    )
                })
            };
            let mut steam = crate::particles::Steam::load(a, &scene.map)?;
            for _ in 0..120 {
                steam.update(1. / 120., camera.eye, &scene.world);
                if phase == Phase::Explore {
                    npcs.update(1. / 120., &scene.world, camera.eye);
                }
            }
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
                crate::lighting::select(steam.lights(), camera.eye, &scene.world);
                crate::render_fx::begin_view(&cam, time as f32, &scene.atmosphere, false);
                scene.draw(camera.eye, time as f32, false, false, &o.transforms());
                art.draw(&o, &scene.atmosphere, camera.eye, false);
                npcs.draw(
                    camera.eye,
                    (camera.target - camera.eye).normalize(),
                    &scene.atmosphere,
                    false,
                );
                crate::render::depth_read_only(|| {
                    scene.draw(camera.eye, time as f32, false, true, &o.transforms());
                    art.effects(&o, camera.eye, &scene.atmosphere);
                    steam.prepare_draw(camera.eye, &scene.atmosphere);
                    let direction = (camera.target - camera.eye).normalize();
                    for (id, _) in steam.depths(camera.eye, direction) {
                        steam.draw_one(id, camera.eye, direction);
                    }
                });
                let (_, dropped) = crate::render_fx::finish();
                ensure!(dropped == 0, "Flora effect budget exceeded");
                set_default_camera();
                if phase == Phase::Reveal {
                    story.draw("E", &ui);
                }
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/centipede1-work/captures/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        println!("PASS Flora native forest/rush/reveal/waves/lid captures");
        Ok(())
    })
}
