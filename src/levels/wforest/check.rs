use super::*;
use crate::{
    interaction::Interactions,
    movement::{Controls, FIXED_DT},
};
fn setup(
    a: &mut Assets,
    map: &Bsp,
    returning: bool,
) -> Result<(Interactions, World, Player, Story, Stats)> {
    let entry = Some(if returning {
        "wforest_start2"
    } else {
        "wforest_start1"
    });
    let mut i = Interactions::load(map)?;
    i.set_entry(a, map, "wforest", entry)?;
    let mut w = World::from_bsp(map)?;
    i.sync(&mut w);
    let p = Player::spawn(&w, crate::interaction::spawn(map, entry).0)
        .context("WForest entrance blocked")?;
    Ok((
        i,
        w,
        p,
        Story::load(a, "wforest"),
        Stats::for_level("wforest", entry),
    ))
}
fn owner(i: &mut Interactions) -> Result<&mut Forest> {
    i.levels
        .iter_mut()
        .find_map(|s| s.ctl.downcast_mut::<Forest>())
        .context("WForest owner missing")
}
#[allow(clippy::too_many_arguments)]
fn tick(
    i: &mut Interactions,
    map: &Bsp,
    w: &mut World,
    p: &mut Player,
    story: &mut Story,
    stats: &mut Stats,
    dt: f32,
) -> Result<()> {
    if dt > 0. {
        owner(i)?.prepare_player(stats, p);
    }
    i.advance_school(dt, map, w, p)?;
    if i.prepare_story(story) {
        story.tick(dt, false);
    }
    i.sync_cinematic_story(story);
    for n in story.take_completed() {
        i.completed_dialogue(&n);
    }
    i.sync(w);
    Ok(())
}
fn play(a: &mut Assets, map: &Bsp, kind: Kind, hz: u32, skip: bool) -> Result<()> {
    let (mut i, mut w, mut p, mut story, mut stats) = setup(a, map, kind.returning())?;
    let f = owner(&mut i)?;
    f.saved.staff = matches!(kind, Kind::Staff | Kind::Caterpillar | Kind::Chess);
    f.saved.armed = f.saved.staff;
    if kind == Kind::Chess {
        f.saved.done[2] = true;
        f.saved.chess = 4.2;
    }
    f.begin(kind);
    let mut saw = std::collections::BTreeSet::new();
    let mut restored = false;
    for n in 0..hz * 180 {
        if let Some((line, _)) = story.progress(kind.dialogue()) {
            saw.insert(line);
        }
        if skip && n == hz {
            ensure!(
                i.skip_cinematic(map, &mut w, &mut p, &mut story)?,
                "WForest skip refused"
            );
        }
        tick(
            &mut i,
            map,
            &mut w,
            &mut p,
            &mut story,
            &mut stats,
            1. / hz as f32,
        )?;
        if !restored && n > hz * 2 && i.scripted() {
            let before = owner(&mut i)?.snapshot();
            tick(&mut i, map, &mut w, &mut p, &mut story, &mut stats, 0.)?;
            ensure!(
                owner(&mut i)?.snapshot() == before,
                "Pause advanced WForest"
            );
            owner(&mut i)?.restore(&before, map)?;
            ensure!(
                owner(&mut i)?.snapshot() == before,
                "WForest scene cursor changed on restore"
            );
            restored = true;
        }
        if !i.scripted() {
            break;
        }
    }
    ensure!(
        !i.scripted() && owner(&mut i)?.saved.done[kind.index()],
        "WForest scene stalled: {:?}",
        kind
    );
    ensure!(w.body_clear(p.feet), "WForest scene embedded Alice");
    if !skip {
        ensure!(
            saw.len()
                == match kind {
                    Kind::Caterpillar => 5,
                    Kind::Chess => 3,
                    _ => 1,
                },
            "WForest dialogue missing: {:?} {:?}",
            kind,
            saw
        );
    }
    let f = owner(&mut i)?;
    let before = f.snapshot();
    f.begin(kind);
    ensure!(before == f.snapshot(), "WForest scene replayed");
    let mut bad = before.clone();
    bad["returning"] = serde_json::json!(!kind.returning());
    ensure!(
        f.restore(&bad, map).is_err() && f.snapshot() == before,
        "Cross-visit restore changed state"
    );
    println!(
        "PASS WForest {:?}: {} Hz, skip={}, {} lines, clear handoff {:?}",
        kind,
        hz,
        skip,
        saw.len(),
        p.feet
    );
    Ok(())
}
pub(super) fn first(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/wforest.bsp")?)?;
    let catalog = crate::inventory::Catalog::load(a)?;
    ensure!(
        !crate::inventory::pickups_for_visit(&map, "wforest", None, &catalog)
            .iter()
            .any(|p| p.id == "wforest:73"),
        "Blunderbuss leaked into first visit"
    );
    for hz in [30, 60, 144] {
        for kind in [Kind::Arrival, Kind::Staff, Kind::Caterpillar, Kind::Chess] {
            for skip in [false, true] {
                play(a, &map, kind, hz, skip)?;
            }
        }
    }
    let (mut i, mut w, _, mut s, mut stats) = setup(a, &map, false)?;
    ensure!(i.levels.len() == 1, "Duplicate WForest owner");
    let f = owner(&mut i)?;
    ensure!(
        !f.first_ready() && !f.hedge_ready() && f.targets().is_empty(),
        "Premature first route"
    );
    ensure!(
        f.event("Hedge_Maze_Entrance").unwrap().transition.is_none(),
        "Wrong first exit"
    );
    let mut p = Player::spawn(&w, f.data.at("first_eyestaff").translation)
        .context("Staff pillar blocked")?;
    tick(&mut i, &map, &mut w, &mut p, &mut s, &mut stats, FIXED_DT)?;
    ensure!(
        stats.staff_component && stats.copies(7) == 0 && i.scripted(),
        "Staff component not collected independently"
    );
    println!("PASS first-visit altar, scene ownership and wrong-route rejection");
    Ok(())
}
pub(super) fn returning(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/wforest.bsp")?)?;
    for hz in [30, 60, 144] {
        for skip in [false, true] {
            play(a, &map, Kind::Wall, hz, skip)?;
        }
    }
    let (mut i, mut w, mut p, mut story, mut stats) = setup(a, &map, true)?;
    owner(&mut i)?.prepare_player(&mut stats, &mut p);
    ensure!(
        stats.copies(7) == 1 && !owner(&mut i)?.altar(),
        "Wrong return inventory"
    );
    let wrong = vec3(1528., 4352., 612.);
    ensure!(
        i.triggers(FIXED_DT, wrong, wrong).transition.is_none(),
        "Return visit allows chess exit"
    );
    let f = owner(&mut i)?;
    for kind in [
        DamageKind::Other,
        DamageKind::Knife,
        DamageKind::Cards,
        DamageKind::Electric,
        DamageKind::Fire,
        DamageKind::Ice,
        DamageKind::Blunderbuss,
    ] {
        f.hit(Hit {
            id: RETURN_BASE,
            damage: 200.,
            kind,
            knockback: Vec3::ZERO,
        });
        ensure!(
            f.saved.wall_health == 100.,
            "Wrong weapon breaks Eye Staff wall"
        );
    }
    f.event("Open_Humpty_Door");
    for _ in 0..360 {
        tick(
            &mut i,
            &map,
            &mut w,
            &mut p,
            &mut story,
            &mut stats,
            1. / 60.,
        )?;
    }
    ensure!(
        owner(&mut i)?.saved.secret == Some(6.),
        "Humpty door stalled"
    );
    let f = owner(&mut i)?;
    f.hit(Hit {
        id: RETURN_BASE,
        damage: 40.,
        kind: DamageKind::EyeStaff,
        knockback: Vec3::ZERO,
    });
    let partial = f.snapshot();
    f.restore(&partial, &map)?;
    ensure!(f.saved.wall_health == 60., "Partial wall damage lost");
    f.hit(Hit {
        id: RETURN_BASE,
        damage: 60.,
        kind: DamageKind::EyeStaff,
        knockback: Vec3::ZERO,
    });
    f.event("Alice_Destroy_Wall");
    ensure!(!f.scripted(), "Destroyed wall replays warning");
    for _ in 0..340 {
        tick(
            &mut i,
            &map,
            &mut w,
            &mut p,
            &mut story,
            &mut stats,
            1. / 60.,
        )?;
    }
    let f = owner(&mut i)?;
    ensure!(
        f.hedge_ready() && f.targets().is_empty(),
        "Hedge doors did not finish"
    );
    let e = f
        .event("Hedge_Maze_Entrance")
        .unwrap()
        .transition
        .context("Missing hedge exit")?;
    ensure!(HEDGE.matches(&e), "Wrong destination");
    f.transition_failed(&e);
    for _ in 0..61 {
        tick(
            &mut i,
            &map,
            &mut w,
            &mut p,
            &mut story,
            &mut stats,
            1. / 60.,
        )?;
    }
    ensure!(
        owner(&mut i)?
            .update(&mut w, &p, Vec3::Y, false)
            .transition
            .is_some(),
        "Failed transition cannot retry"
    );
    println!("PASS return visit: Staff-only damage, partial-health restore, Humpty passage, early break, moving hedge doors and exit retry");
    Ok(())
}
pub(crate) fn drive(r: &mut crate::route::Route) -> Result<()> {
    std::fs::create_dir_all("private/wforest-route")?;
    let returning = r.level().entry.as_deref() == Some("wforest_start2");
    r.tactics = true;
    // Retain the earned Staff charge needed at the return wall.
    r.conserve_will = false;
    r.ice_stream = false;
    r.will_reserve = if returning { 25. } else { 0. };
    r.stop_at_exit = true;
    r.wait_for_cinematic()?;
    if returning && r.native_cast.is_some() && r.stats.copies(9) > 0 {
        // The carried Watch may still be cooling down from the lava crossing.
        // Wait at the quiet entrance; use its ordinary paid action below.
        r.wait(r.stats.powers.recharge + 0.1)?;
    }
    let goals: Vec<Vec3> = if let Ok(path) = std::env::var("LOOKING_GLASS_WFOREST_ROUTE") {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else if returning {
        vec![
            vec3(5352., 1240., 216.),
            vec3(5080., 3072., 288.),
            vec3(5328., 3696., 512.),
            vec3(5360., 3728., 512.),
            vec3(5804., 3788., 464.),
            vec3(6464., 4896., 592.),
        ]
    } else {
        vec![
            vec3(5358., 1182., 216.),
            vec3(4824., 1240., 96.),
            vec3(2776., 1301., 0.),
            vec3(2248., 1304., 168.),
            vec3(1288., 936., 368.),
            vec3(176., 56., 400.),
            vec3(1288., 936., 368.),
            vec3(2248., 1304., 168.),
            vec3(3000., 1304., 0.),
            vec3(3232., 1168., 24.),
            vec3(3312., 1440., 112.),
            vec3(4100., 646., 320.),
            vec3(4240., 464., 336.),
            vec3(4408., 2392., 144.),
            vec3(3928., 2392., 48.),
            vec3(3336., 2728., -16.),
            vec3(3816., 3496., -80.),
            vec3(3880., 3992., -48.),
            vec3(3512., 4432., 128.),
            vec3(2408., 4504., 400.),
            vec3(2160., 4624., 432.),
            vec3(2128., 4592., 432.),
            vec3(2408., 4504., 400.),
            vec3(2216., 4280., 464.),
            vec3(1800., 4288., 592.),
            // Placed supplies on the last shelf, before the one-way chess exit.
            vec3(2336., 4000., 560.),
            vec3(2400., 4064., 560.),
            vec3(1800., 4288., 592.),
        ]
    };
    for goal in goals {
        let goal = r
            .world
            .actor_footing(goal, PLAYER_CENTER, PLAYER_HALF, 1024.)
            .with_context(|| format!("Unsupported route waypoint {goal:?}"))?;
        println!("WForest route {:?} -> {:?}", r.player.feet, goal);
        if goal.distance(r.player.feet) < 80. && (goal.z - r.player.feet.z).abs() < 20. {
            r.walk(goal, false)?;
        } else {
            r.navigate_until_scene(goal)?;
        }
        let interrupted = r.interactions.scripted();
        r.wait_for_cinematic()?;
        if interrupted {
            r.navigate(goal)?;
        }
        if !returning && (goal.x - 2408.).abs() < 1. && (goal.y - 4504.).abs() < 1. {
            r.clear(1000.)?;
        }
        std::fs::write("private/wforest-route/last.json", serde_json::to_vec(&r.checkpoint())?)?;
    }
    if returning {
        if r.native_cast.is_some() && r.stats.copies(9) > 0 {
            r.use_watch()?;
        }
        r.ice_stream = false;
        r.tactics = true;
        r.heavy_weapon = Some(7);
        r.aim_at = Some(RETURN_BASE);
        if std::env::var_os("LOOKING_GLASS_ROUTE_TRACE").is_some() {
            let targets = owner(&mut r.interactions)?.targets();
            let ctx = crate::combat::Context { world: &r.world, targets: &targets };
            for t in &targets {
                println!("Wall aim diagnostic: eye {:?}, box {:?}/{:?}, contact {:?}, aim {:?}", r.player.eye(), t.center, t.half,
                    crate::combat::contact(&ctx, r.player.eye(), t.center, 1.), crate::route::fight::aim_point(&ctx, r.player.eye(), t));
            }
        }
        for _ in 0..1200 {
            r.tick(Controls::default())?;
            if owner(&mut r.interactions)?.saved.wall_health == 0. {
                break;
            }
        }
        ensure!(
            owner(&mut r.interactions)?.saved.wall_health == 0.,
            "Actual Eye Staff input failed to break wall"
        );
        r.heavy_weapon = None;
        r.aim_at = None;
        // The Staff charge is no longer needed once the wall is broken.
        // Keep fighting with the remaining earned Will while the doors move.
        r.will_reserve = 0.;
        r.ice_stream = true;
        std::fs::write("private/wforest-route/broken.json", serde_json::to_vec(&r.checkpoint())?)?;
        if r.native_cast.is_some() {
            r.clear(1000.)?;
        }
        // Destruction is live gameplay: move through the opening while the
        // remaining debris/door clocks finish instead of standing under fire.
        // A static route search still sees the unopened doors at this instant.
        // Ordinary forward/jump input advances both Alice and the real movers.
        for n in 0..120 * 60 {
            if r.transition.is_some() || owner(&mut r.interactions)?.hedge_ready() { break; }
            let goal = vec3(6504., 5432., 768.03125);
            let offset = goal - r.player.feet;
            r.tick(Controls {
                wish: if offset.truncate().length() > 18. { offset.truncate().normalize_or_zero() } else { Vec2::ZERO },
                run: true,
                jump: (r.player.grounded && n % 90 == 0 && offset.z > 16.)
                    || r.player.ledge.as_ref().is_some_and(|h| !h.pulling),
                ..Default::default()
            })?;
        }
        ensure!(r.transition.is_some() || owner(&mut r.interactions)?.hedge_ready(),
            "Wall doors did not finish after the paid Watch expired");
        // Enter the trigger's eight-unit half-depth; a navigation tolerance
        // beyond its far face can stop without ever making contact.
        r.walk(vec3(6504., 5432., 768.03125), false)?;
        r.wait(0.1)?;
    } else {
        ensure!(
            owner(&mut r.interactions)?.first_ready(),
            "First visit failed to open chess route"
        );
        // The exit trigger abuts the back wall; its centre cannot contain Alice's full
        // body. Walk into the trigger's near face, as the normal transition requires.
        r.navigate(vec3(1528., 4352., 576.03125))?;
    }
    ensure!(
        r.stats.alive() && r.teleports == 0,
        "WForest route failed survival/continuity"
    );
    ensure!(
        r.transition.as_ref().is_some_and(|e| if returning {
            HEDGE.matches(e)
        } else {
            CHESS.matches(e)
        }),
        "WForest route missed exit at {:?}",
        r.player.feet
    );
    println!(
        "PASS continuous WForest route: ticks={}, jumps={}, damage={}, sanity={}",
        r.ticks,
        r.jumps,
        r.damage,
        r.stats.sanity()
    );
    Ok(())
}
fn route(a: &mut Assets, returning: bool) -> Result<()> {
    let mut r = crate::route::Route::new(
        a,
        "wforest",
        Some(if returning {
            "wforest_start2"
        } else {
            "wforest_start1"
        }),
    )?;
    r.stats.notarget = true;
    r.skip_cinematics = true;
    drive(&mut r)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.world.body_clear(next.player.feet),
        "Destination blocked"
    );
    Ok(())
}
pub(super) fn route_first(a: &mut Assets) -> Result<()> {
    route(a, false)
}
pub(super) fn route_return(a: &mut Assets) -> Result<()> {
    route_secret(a)?;
    route(a, true)
}
pub(super) fn route_secret(a: &mut Assets) -> Result<()> {
    let mut r = crate::route::Route::new(a, "wforest", Some("wforest_start2"))?;
    r.stats.notarget = true;
    for goal in [
        vec3(5352., 1240., 216.),
        vec3(4072., 1192., 368.),
        vec3(3679., 1094., 400.),
    ] {
        let goal = r
            .world
            .actor_footing(goal, PLAYER_CENTER, PLAYER_HALF, 1024.)
            .with_context(|| format!("Unsupported secret waypoint {goal:?}"))?;
        println!("Secret route {:?} -> {:?}", r.player.feet, goal);
        r.navigate(goal)?;
    }
    ensure!(
        owner(&mut r.interactions)?.saved.secret.is_some(),
        "Humpty button not touched"
    );
    for _ in 0..(6.2 / FIXED_DT) as usize {
        r.tick(Controls::default())?;
    }
    ensure!(
        owner(&mut r.interactions)?.saved.secret == Some(6.),
        "Secret door failed to open: {:?}",
        owner(&mut r.interactions)?.saved.secret
    );
    for goal in [
        vec3(3160., 1240., 16.),
        vec3(2886., 1160., -64.),
        vec3(2886., 1090., -72.),
        vec3(2814., 574., -64.),
    ] {
        // Editor nodes in this tunnel overlap the sloping floor. Search for a clear
        // navigation goal above it; Alice still reaches it only through normal input.
        let goal = [0., 16., 32., 64., 96., 128.]
            .into_iter()
            .find_map(|lift| {
                r.world
                    .actor_footing(goal + Vec3::Z * lift, PLAYER_CENTER, PLAYER_HALF, 1024.)
            })
            .with_context(|| format!("Unsupported secret passage waypoint {goal:?}"))?;
        println!("Secret route {:?} -> {:?}", r.player.feet, goal);
        r.navigate(goal)?;
    }
    for _ in 0..(60. / FIXED_DT) as usize {
        r.tick(Controls::default())?;
        if owner(&mut r.interactions)?.saved.blunder_done {
            break;
        }
    }
    ensure!(
        r.stats.copies(8) == 1 && owner(&mut r.interactions)?.saved.blunder_done,
        "Secret reward/dialogue missing"
    );
    ensure!(
        r.stats.alive() && r.teleports == 0,
        "Secret route failed survival/continuity"
    );
    println!(
        "PASS continuous Humpty secret: {} ticks, reward and dialogue, no teleports",
        r.ticks
    );
    Ok(())
}
pub(super) fn stage_first(i: &mut Interactions, map: &Bsp) -> Result<()> {
    let f = owner(i)?;
    f.saved.initialized = true;
    f.saved.done[0] = true;
    f.saved.staff = true;
    f.saved.armed = true;
    f.begin(Kind::Staff);
    f.saved.scene.as_mut().unwrap().time = 7.;
    f.saved.scene.as_mut().unwrap().ending = Some(5.);
    f.saved.cave = 0.5;
    f.motion.rebuild(map, &f.saved)
}
pub(super) fn stage_secret(i: &mut Interactions, map: &Bsp) -> Result<()> {
    let f = owner(i)?;
    f.saved.initialized = true;
    f.saved.secret = Some(3.);
    f.saved.wall_health = 60.;
    f.motion.rebuild(map, &f.saved)
}
pub(super) fn stage_debris(i: &mut Interactions, map: &Bsp) -> Result<()> {
    let f = owner(i)?;
    f.saved.initialized = true;
    f.saved.wall_health = 0.;
    f.saved.destruction = Some(0.8);
    f.motion.rebuild(map, &f.saved)
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let mut renderer = crate::render::Scene::load(a, "wforest")?;
        let mut art = art::Art::load(a)?;
        for (name, returning, kind) in [
            ("altar", false, None),
            ("arrival", false, Some(Kind::Arrival)),
            ("staff", false, Some(Kind::Staff)),
            ("caterpillar", false, Some(Kind::Caterpillar)),
            ("chess", false, Some(Kind::Chess)),
            ("wall", true, Some(Kind::Wall)),
            ("humpty", true, None),
            ("essence", true, None),
            ("wall-breaking", true, None),
            ("hedge-open", true, None),
        ] {
            let (mut i, mut w, mut p, mut story, mut stats) = setup(a, &renderer.map, returning)?;
            let mut steam = crate::particles::Steam::load(a, &renderer.map)?;
            let f = owner(&mut i)?;
            if let Some(k) = kind {
                f.saved.staff = matches!(k, Kind::Staff | Kind::Caterpillar | Kind::Chess);
                f.saved.armed = f.saved.staff;
                if k == Kind::Wall {
                    f.saved.wall_visible = true;
                }
                f.begin(k);
            }
            if name == "hedge-open" {
                f.saved.wall_health = 0.;
                f.saved.destruction = Some(5.5);
            }
            for n in 0..180 {
                tick(
                    &mut i,
                    &renderer.map,
                    &mut w,
                    &mut p,
                    &mut story,
                    &mut stats,
                    1. / 60.,
                )?;
                steam.animate(n as f32 / 60.);
                steam.sync(&i.event_world);
                owner(&mut i)?.particles(&mut steam);
                steam.update(1. / 60., p.eye(), &w);
            }
            art.story_pose(&story);
            let f = owner(&mut i)?;
            if name == "wall-breaking" {
                f.saved.wall_health = 0.;
                f.saved.destruction = Some(0.75);
                f.motion.rebuild(&renderer.map, &f.saved)?;
            }
            let c = f.scene_camera().unwrap_or_else(|| {
                if name == "essence" {
                    let target = f.data.at("get_me1").translation + Vec3::Z * 36.;
                    let eye = [Vec3::Y, -Vec3::Y, Vec3::X, -Vec3::X].into_iter()
                        .map(|d| target + d * 140. + Vec3::Z * 40.)
                        .find(|eye| { let hit=w.sweep(target, *eye, Vec3::ZERO); !hit.start_solid && hit.fraction == 1. })
                        .unwrap_or(target + Vec3::Z * 120.);
                    crate::cinematic::Camera::look(eye, target)
                } else if name == "altar" {
                    crate::cinematic::Camera::look(
                        vec3(5520., 1050., 350.),
                        f.data.at("first_eyestaff").translation + Vec3::Z * 45.,
                    )
                } else if name == "humpty" {
                    crate::cinematic::Camera::look(
                        vec3(4020., 1150., 540.),
                        f.data.at("humpty_actor1").translation + Vec3::Z * 60.,
                    )
                } else {
                    crate::cinematic::Camera::look(
                        vec3(6504., 5000., 870.),
                        vec3(6504., 5650., 900.),
                    )
                }
            });
            ensure!(
                !w.sweep(c.eye, c.eye, Vec3::ZERO).start_solid,
                "WForest capture camera in solid: {name}"
            );
            for frame in 0..3 {
                clear_background(BLACK);
                set_camera(&Camera3D {
                    position: c.eye,
                    target: c.target,
                    up: c.up,
                    fovy: 75_f32.to_radians(),
                    z_near: 2.,
                    z_far: 20000.,
                    ..Default::default()
                });
                renderer.draw(c.eye, 0., false, false, &f.transforms());
                art.draw(f, &renderer.atmosphere, c.eye, false);
                crate::render::depth_read_only(|| {
                    renderer.draw_with_particles(
                        c.eye,
                        (c.target - c.eye).normalize(),
                        3.,
                        false,
                        &f.transforms(),
                        &steam,
                    )
                });
                art.effects(f, c.eye, &renderer.atmosphere);
                set_default_camera();
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/wforest-visits/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        Ok(())
    })
}
