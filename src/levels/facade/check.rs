use super::*;
use crate::{
    interaction::Interactions,
    movement::{Controls, FIXED_DT},
    route::Route,
};
fn owner(i: &mut Interactions) -> Result<&mut Facade> {
    i.levels
        .iter_mut()
        .find_map(|s| s.ctl.downcast_mut::<Facade>())
        .context("Missing Ascension owner")
}
fn setup(a: &mut Assets, map: &Bsp) -> Result<(Interactions, World, Player, Story)> {
    let mut i = Interactions::load(map)?;
    i.set_entry(a, map, "facade", Some("facade_start1"))?;
    let mut w = World::from_bsp(map)?;
    i.sync(&mut w);
    let p = Player::spawn(&w, interaction::spawn(map, Some("facade_start1")).0)
        .context("Ascension entrance blocked")?;
    Ok((i, w, p, Story::load(a, "facade")))
}
fn tick(i: &mut Interactions, map: &Bsp, w: &mut World, p: &mut Player, dt: f32) -> Result<Events> {
    let e = i.update(dt, map, w, p, Vec3::Y, false)?;
    i.advance_school(dt, map, w, p)?;
    i.sync(w);
    Ok(e)
}
fn deck(w: &World) -> Result<Vec3> {
    w.actor_footing(vec3(320., 2976., -1500.), PLAYER_CENTER, PLAYER_HALF, 200.)
        .context("Lift lacks solid deck")
}
fn begin(i: &mut Interactions, w: &World, p: &mut Player) -> Result<()> {
    p.feet = deck(w)?;
    p.velocity = Vec3::ZERO;
    p.script_facing = 90_f32.to_radians();
    p.grounded = true;
    i.triggers(FIXED_DT, p.feet - Vec3::Y * 100., p.feet);
    ensure!(owner(i)?.scripted(), "Lift contact did not start its scene");
    Ok(())
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/facade.bsp")?)?;
    let (mut i, mut w, mut p, _) = setup(a, &map)?;
    ensure!(
        owner(&mut i)?.transforms().len() == 3 && w.body_clear(p.feet),
        "Missing gate/lift geometry"
    );
    ensure!(
        !w.traversal.pushes.iter().any(|v| v.id == Id(39)),
        "Monster pad launches Alice"
    );
    for id in [74, 176] {
        let force = w
            .traversal
            .pushes
            .iter()
            .find(|v| v.id == Id(id))
            .context("Missing updraft")?;
        ensure!(
            force.enabled && force.accelerate && force.speed < 0.,
            "Updraft not armed"
        );
        let mut probe = Player::new(force.origin);
        let start = probe.feet.z;
        for _ in 0..45 {
            probe.tick(&w, Controls::default());
        }
        ensure!(
            probe.feet.z > start + 8. && probe.steam,
            "Updraft {id} did not lift Alice"
        );
    }
    ensure!(
        i.triggers(FIXED_DT, vec3(320., 2976., -812.), vec3(320., 2976., -812.))
            .transition
            .is_none(),
        "Early fallback exit"
    );
    p.feet = w
        .actor_footing(vec3(320., 300., -1470.), PLAYER_CENTER, PLAYER_HALF, 300.)
        .context("Gate approach blocked")?;
    let e = i.triggers(FIXED_DT, p.feet, p.feet + Vec3::Y * 110.);
    ensure!(e.transition.is_none(), "Door caused exit");
    ensure!(
        owner(&mut i)?.saved.gate_opening,
        "Gate target was not dispatched"
    );
    for _ in 0..420 {
        tick(&mut i, &map, &mut w, &mut p, FIXED_DT)?;
    }
    ensure!(owner(&mut i)?.saved.gate == 1., "Gate did not open");
    let saved = i.snapshot();
    i.restore(&saved, &map)?;
    i.sync(&mut w);
    for _ in 0..900 {
        tick(&mut i, &map, &mut w, &mut p, FIXED_DT)?;
    }
    ensure!(owner(&mut i)?.saved.gate == 0., "Gate did not close");
    i.restore(&saved, &map)?;
    i.sync(&mut w);
    p.feet = w
        .actor_footing(vec3(320., 464., -1450.), PLAYER_CENTER, PLAYER_HALF, 300.)
        .context("Doorway floor absent")?;
    let mut closing = false;
    let mut reversed = false;
    for _ in 0..1200 {
        tick(&mut i, &map, &mut w, &mut p, FIXED_DT)?;
        let s = &owner(&mut i)?.saved;
        closing |= !s.gate_opening;
        reversed |= closing && s.gate_opening;
        ensure!(w.body_clear(p.feet), "Closing gate embedded Alice");
    }
    ensure!(reversed, "Closing gate failed to reverse at Alice");
    println!("PASS Ascension targeted doors, saved motion, safe closing, both updrafts and monster-only pad");
    for hz in [30, 60, 144] {
        for skip in [false, true] {
            let (mut i, mut w, mut p, mut story) = setup(a, &map)?;
            begin(&mut i, &w, &mut p)?;
            let mut exit = None;
            let mut end = 0.;
            let mut last = p.feet;
            for frame in 0..hz * 7 {
                if skip && frame == hz {
                    ensure!(
                        i.skip_cinematic(&map, &mut w, &mut p, &mut story)?,
                        "Cannot skip lift"
                    );
                }
                let e = tick(&mut i, &map, &mut w, &mut p, 1. / hz as f32)?;
                ensure!(w.body_clear(p.feet), "Lift embedded rider at {hz}Hz");
                ensure!(p.feet.distance(last) < 30., "Discontinuous rider");
                last = p.feet;
                if frame == hz {
                    let s = i.snapshot();
                    i.advance_school(0., &map, &mut w, &mut p)?;
                    ensure!(
                        serde_json::to_value(&s)? == serde_json::to_value(i.snapshot())?,
                        "Paused lift moved"
                    );
                    i.restore(&s, &map)?;
                    i.sync(&mut w);
                }
                if e.transition.is_some() {
                    exit = e.transition;
                    end = (frame + 1) as f32 / hz as f32;
                    break;
                }
            }
            ensure!(
                exit.as_ref().is_some_and(|d| EXIT.matches(d)),
                "Lift transfer missing"
            );
            let expected = if skip { 1.5 } else { DEPART };
            ensure!(
                (end - expected).abs() <= 2.1 / hz as f32,
                "Wrong departure timing {end} instead of {expected}"
            );
            ensure!(
                tick(&mut i, &map, &mut w, &mut p, 0.1)?
                    .transition
                    .is_none(),
                "Duplicate script exit"
            );
            ensure!(
                owner(&mut i)?
                    .exit_contact(&EXIT.destination())
                    .unwrap()
                    .transition
                    .is_none(),
                "Duplicate physical exit"
            );
            i.transition_failed(&EXIT.destination());
            let mut retried = false;
            for _ in 0..12 {
                retried |= tick(&mut i, &map, &mut w, &mut p, 0.1)?
                    .transition
                    .is_some();
            }
            ensure!(retried, "Failed load cannot retry");
            let s = i.snapshot();
            i.restore(&s, &map)?;
            ensure!(
                tick(&mut i, &map, &mut w, &mut p, 0.01)?
                    .transition
                    .is_some(),
                "Committed save lost exit"
            );
            transfer(a, &i)?;
            println!("PASS Ascension {hz}Hz skip={skip}: transfer at {end:.3}s, saved exit and carried Keep arrival");
        }
    }
    let (mut i, _, _, _) = setup(a, &map)?;
    let prior = Interactions::load(&map)?.snapshot();
    i.restore(&prior, &map)?;
    ensure!(
        owner(&mut i)?.saved.ride.is_none(),
        "Legacy load started lift"
    );
    let f = owner(&mut i)?;
    let good = f.snapshot();
    let mut bad = good.clone();
    bad["gate"] = serde_json::json!(2.);
    ensure!(
        f.restore(&bad, &map).is_err() && f.snapshot() == good,
        "Bad save mutated Ascension"
    );
    println!("PASS Ascension legacy migration and atomic invalid-state rejection");
    Ok(())
}
fn transfer(a: &mut Assets, i: &Interactions) -> Result<()> {
    for strict in [false, true] {
        let mut r = Route::enter(
            a,
            "facade",
            Some("facade_start1"),
            crate::inventory::Stats::for_level("facade", None),
            crate::powerups::Difficulty::Normal,
        )?;
        r.interactions.restore(&i.snapshot(), &r.map)?;
        r.transition = Some(EXIT.destination());
        r.stats.damage(63.);
        r.stats.spend_will(59.);
        r.stats.select(7);
        let stats = serde_json::to_value(&r.stats)?;
        let mut next = r.depart(a, strict)?;
        ensure!(
            next.player.feet.distance(vec3(512., 288., -56.)) < 0.2,
            "Wrong Keep arrival {:?}",
            next.player.feet
        );
        ensure!(
            next.world.body_clear(next.player.feet)
                && next.ledger.completed.contains("facade$first"),
            "Keep arrival/ledger failed"
        );
        next.interactions
            .prepare_player(&mut next.stats, &mut next.player);
        ensure!(
            serde_json::to_value(&next.stats)? == stats,
            "Keep reset carried inventory/resources"
        );
        ensure!(next.interactions.scripted(), "Keep arrival scene absent");
    }
    Ok(())
}
pub(super) fn route(a: &mut Assets) -> Result<()> {
    run_route(a, false)
}
pub(super) fn skip_route(a: &mut Assets) -> Result<()> {
    run_route(a, true)
}
fn run_route(a: &mut Assets, skip: bool) -> Result<()> {
    // Three independent component fixtures. Only their initial position is staged;
    // ascent, landing, gate crossing and lift boarding use production inputs.
    for (start, height, landing) in [
        (
            vec3(-1232., -2608., -2580.),
            -2310.,
            vec3(-1224., -2768., -2303.9688),
        ),
        (
            vec3(-496., -2872., -2544.),
            -1740.,
            vec3(-328., -2856., -1743.9688),
        ),
    ] {
        let mut r = fixture(a, start, skip)?;
        rise(&mut r, start.truncate(), height, landing)?;
        ensure!(
            r.teleports == 0 && r.player.immersion.level == 0 && r.stats.sanity() == 100.,
            "Unsafe air-column traversal"
        );
    }
    let mut r = fixture(a, vec3(320., 280., -1599.9688), skip)?;
    let beyond_gate = r
        .world
        .actor_footing(vec3(320., 650., -1400.), PLAYER_CENTER, PLAYER_HALF, 300.)
        .context("Gate landing missing")?;
    r.walk(beyond_gate, false)?;
    ensure!(
        owner(&mut r.interactions)?.saved.gate > 0.2,
        "Gate crossing bypassed leaves"
    );
    println!("PASS targeted gate crossed with ordinary movement");
    let mut r = fixture(a, vec3(320., 2780., -1563.9688), skip)?;
    for _ in 0..600 {
        r.tick(Controls {
            wish: Vec2::Y,
            jump: r.player.grounded,
            ..Default::default()
        })?;
        if r.interactions.scripted() {
            break;
        }
    }
    ensure!(
        r.interactions.scripted(),
        "Boarding the lift did not start departure at {:?}",
        r.player.feet
    );
    r.wait_for_cinematic()?;
    r.wait(0.1)?;
    ensure!(
        r.transition.as_ref().is_some_and(|d| EXIT.matches(d))
            && r.teleports == 0
            && r.stats.alive(),
        "Ascension route did not finish alive"
    );
    let f = owner(&mut r.interactions)?;
    // A skipped scene can depart under full white before the short fall ends.
    // On the watched ride Alice must settle onto the actual deck surface.
    if !skip {
        ensure!(
            (r.player.feet.z
                - f.lift_pose().translation.z
                - f.saved.ride.as_ref().unwrap().floor.unwrap())
            .abs()
                < 0.1,
            "Jump boarding left Alice floating over the deck"
        );
    }
    let before = serde_json::to_value(&r.stats)?;
    let ticks = r.ticks;
    let mut next = r.depart(a, true)?;
    next.interactions
        .prepare_player(&mut next.stats, &mut next.player);
    ensure!(
        serde_json::to_value(&next.stats)? == before && next.world.body_clear(next.player.feet),
        "Live carried state/arrival mismatch"
    );
    println!(
        "PASS Ascension updraft/gate/lift fixture skip={skip}, ticks={ticks}, Keep feet={:?}",
        next.player.feet
    );
    Ok(())
}
fn fixture(a: &mut Assets, feet: Vec3, skip: bool) -> Result<Route> {
    let mut r = Route::new(a, "facade", Some("facade_start1"))?;
    ensure!(
        r.world.body_clear(feet),
        "Obstructed fixture start {feet:?}"
    );
    r.player = Player::new(feet);
    r.skip_cinematics = skip;
    r.stop_at_exit = true;
    r.stats.notarget = true;
    r.aim_at = Some(usize::MAX);
    Ok(r)
}
pub(super) fn rise(r: &mut Route, center: Vec2, height: f32, landing: Vec3) -> Result<()> {
    let start = r.player.feet;
    let mut rising = false;
    for n in 0..3000 {
        let goal = if rising { landing.truncate() } else if r.player.steam {
            // Keep moving within the real air column while ranged enemies aim.
            // This is ordinary steering, bounded inside the authored volume.
            let angle = n as f32 / 120. * 3.;
            center + vec2(angle.cos(), angle.sin()) * 36.
        } else { center };
        let delta = goal - r.player.feet.truncate();
        r.tick(Controls {
            wish: (delta * 0.06 - r.player.velocity.truncate() * 0.015).clamp_length_max(1.),
            jump: !rising && n % 150 == 0,
            ..Default::default()
        })?;
        if n % 120 == 0 {
            println!(
                "  RISE {:?} steam={} ground={}",
                r.player.feet, r.player.steam, r.player.grounded
            );
        }
        rising |= r.player.feet.z >= height;
        ensure!(
            r.stats.alive() && r.world.body_clear(r.player.feet),
            "Updraft unsafe at {:?}",
            r.player.feet
        );
        if rising && r.player.grounded && r.player.feet.distance(landing) < 45. {
            ensure!(
                r.player.feet.z > start.z + 250.,
                "Insufficient updraft rise"
            );
            println!("PASS updraft landing {:?}", r.player.feet);
            return Ok(());
        }
    }
    anyhow::bail!(
        "Updraft failed to reach {landing:?}, stopped {:?}",
        r.player.feet
    )
}
pub(super) const SAVES: &[super::super::SaveCase] = &[
    super::super::SaveCase {
        name: "facade-gates",
        visit: "facade$first",
        stage: None,
        behavior: None,
    },
    super::super::SaveCase {
        name: "facade-lift-fade",
        visit: "facade$first",
        stage: None,
        behavior: None,
    },
    super::super::SaveCase {
        name: "facade-lift-mid",
        visit: "facade$first",
        stage: None,
        behavior: None,
    },
    super::super::SaveCase {
        name: "facade-lift-skip",
        visit: "facade$first",
        stage: None,
        behavior: None,
    },
    super::super::SaveCase {
        name: "facade-exit-committed",
        visit: "facade$first",
        stage: None,
        behavior: None,
    },
];
pub(crate) fn stage_native(
    case: &str,
    a: &mut Assets,
    map: &Bsp,
    i: &mut Interactions,
    w: &mut World,
    p: &mut Player,
) -> Result<Story> {
    let mut story = Story::load(a, "facade");
    if case == "facade-gates" {
        i.dispatch(Event::Entity(Id(19), Input::Activate));
        for _ in 0..100 {
            tick(i, map, w, p, FIXED_DT)?;
        }
    } else {
        begin(i, w, p)?;
        let t = match case {
            "facade-lift-fade" => 0.25,
            "facade-lift-mid" => 2.,
            "facade-lift-skip" => 1.,
            _ => DEPART,
        };
        for _ in 0..(t * 120.) as usize {
            tick(i, map, w, p, FIXED_DT)?;
        }
        if case == "facade-lift-skip" {
            i.skip_cinematic(map, w, p, &mut story)?;
            for _ in 0..24 {
                tick(i, map, w, p, FIXED_DT)?;
            }
        }
        if case == "facade-exit-committed" {
            for _ in 0..2 {
                tick(i, map, w, p, FIXED_DT)?;
            }
        }
    }
    Ok(story)
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let mut scene = crate::render::Scene::load(a, "facade")?;
        let mut art = Art::load(a)?;
        for (name, t) in [
            ("gates-closed", -1_f32),
            ("gates-open", -2.),
            ("lift-start", 0.6),
            ("lift-mid", 2.5),
            ("lift-end", 4.4),
        ] {
            let (mut i, mut w, mut p, _) = setup(a, &scene.map)?;
            let camera = if t < 0. {
                if t == -2. {
                    let f = owner(&mut i)?;
                    f.saved.gate = 1.;
                    f.rebuild(&scene.map)?;
                }
                Camera::look(vec3(320., 100., -1450.), vec3(320., 464., -1420.))
            } else {
                begin(&mut i, &w, &mut p)?;
                for _ in 0..(t * 120.) as usize {
                    tick(&mut i, &scene.map, &mut w, &mut p, FIXED_DT)?;
                }
                owner(&mut i)?.camera(&w).unwrap()
            };
            let f = owner(&mut i)?;
            let c = Camera3D {
                position: camera.eye,
                target: camera.target,
                up: camera.up,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            };
            for n in 0..3 {
                clear_background(BLACK);
                set_camera(&c);
                crate::render_fx::begin_view(&c, t.max(0.), &scene.atmosphere, false);
                scene.draw(camera.eye, t.max(0.), false, false, &f.transforms());
                art.draw(f, &scene.atmosphere, camera.eye, false);
                scene.draw(camera.eye, t.max(0.), false, true, &f.transforms());
                crate::render_fx::finish();
                set_default_camera();
                if n == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/facade-work/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        Ok(())
    })
}
