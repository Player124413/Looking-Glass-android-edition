use super::*;
use crate::{
    interaction::Interactions,
    movement::{Controls, FIXED_DT},
};
fn owner(i: &mut Interactions) -> Result<&mut Keep> {
    i.levels
        .iter_mut()
        .find_map(|s| s.ctl.downcast_mut::<Keep>())
        .context("Missing Keep controller")
}
fn setup(a: &mut Assets, map: &Bsp) -> Result<(Interactions, World, Player, Story)> {
    let mut i = Interactions::load(map)?;
    i.set_entry(a, map, "keep", Some("keep_start1"))?;
    let mut w = World::from_bsp(map)?;
    i.sync(&mut w);
    let p = Player::spawn(&w, interaction::spawn(map, Some("keep_start1")).0)
        .context("Keep spawn blocked")?;
    Ok((i, w, p, Story::load(a, "keep")))
}
fn tick(
    i: &mut Interactions,
    map: &Bsp,
    w: &mut World,
    p: &mut Player,
    story: &mut Story,
    dt: f32,
) -> Result<Events> {
    let e = i.update(dt, map, w, p, Vec3::Y, false)?;
    i.advance_school(dt, map, w, p)?;
    if i.prepare_story(story) {
        story.tick(dt, false);
    }
    i.sync_cinematic_story(story);
    for n in story.take_completed() {
        i.completed_dialogue(&n);
    }
    if !i.scripted() {
        p.tick(w, Controls::default());
    }
    i.sync(w);
    Ok(e)
}
fn presentation(a: &mut Assets, map: &Bsp) -> Result<()> {
    let mut k = Keep::load(a, map)?;
    let actors = World::actor_world(map)?;
    let placed = data::at(&map.entities[3]);
    k.begin(Kind::Arrival);
    for time in [3.1, 5., 7., 10.] {
        k.saved.scene.as_mut().unwrap().time = time;
        let (at, _, _, _, _) = k.cat().context("Missing arrival Cheshire")?;
        ensure!(
            at.translation.truncate() == placed.translation.truncate()
                && at.rotation == placed.rotation,
            "Arrival Cheshire left his authored placement"
        );
        let center = at.translation + Vec3::Z * 24.;
        let support = actors.sweep(center, center - Vec3::Z, vec3(32., 32., 24.));
        ensure!(
            !support.start_solid && support.fraction < 0.3 && support.normal.z > 0.65,
            "Arrival Cheshire is unsupported at {:?}",
            at.translation
        );
    }
    println!("PASS Keep arrival Cheshire: placed actor, grounded body and seated performance");
    for hz in [30, 60, 144] {
        let mut w = World::from_bsp(map)?;
        let feet = w
            .actor_footing(vec3(512., 1200., 160.), PLAYER_CENTER, PLAYER_HALF, 128.)
            .context("Hallway approach has no support")?;
        let mut p = Player::new(feet);
        k.saved = Saved {
            elapsed: 5.,
            initialized: true,
            arrival: true,
            ..Default::default()
        };
        k.motion.rebuild(map, &k.saved)?;
        let closed = k.transforms();
        k.update(&mut w, &p, Vec3::Y, false);
        ensure!(
            k.saved.hall == 2. && k.transforms() == closed,
            "Door contact snapped the mesh"
        );
        let dt = 1. / hz as f32;
        for n in 1..=hz {
            k.update(&mut w, &p, Vec3::Y, false);
            k.motion
                .advance(dt, map, &mut w, &mut p, &[], &mut k.saved)?;
            ensure!(
                (k.saved.hall_open - n as f32 / hz as f32).abs() < 0.0001,
                "Hallway door did not slide over its authored second at {hz}Hz"
            );
            ensure!(w.body_clear(p.feet), "Door opening embedded Alice");
        }
        let open = k.transforms();
        for model in [46, 47] {
            let at =
                |poses: &Vec<(usize, Vec3, Quat)>| poses.iter().find(|t| t.0 == model).unwrap().1;
            ensure!(
                at(&open).distance(at(&closed)) > 50.,
                "Hallway leaf stayed closed"
            );
        }
        // Leave the contact region: hold, then close smoothly. Restore in flight
        // and require the actual rendered poses to have identical futures.
        for _ in 0..hz * 2 {
            k.motion
                .advance(dt, map, &mut w, &mut p, &[], &mut k.saved)?;
        }
        ensure!(k.saved.hall_open > 0.9, "Hallway door lost its hold time");
        for _ in 0..hz / 2 {
            k.motion
                .advance(dt, map, &mut w, &mut p, &[], &mut k.saved)?;
        }
        ensure!(
            (0.4..0.6).contains(&k.saved.hall_open),
            "Door closing snapped"
        );
        let saved = k.snapshot();
        let mut loaded = Keep::load(a, map)?;
        loaded.restore(&saved, map)?;
        let mut other = World::from_bsp(map)?;
        let mut q = p.clone();
        for _ in 0..hz {
            k.motion
                .advance(dt, map, &mut w, &mut p, &[], &mut k.saved)?;
            loaded
                .motion
                .advance(dt, map, &mut other, &mut q, &[], &mut loaded.saved)?;
            ensure!(
                k.snapshot() == loaded.snapshot() && k.transforms() == loaded.transforms(),
                "Saved hallway movement diverged"
            );
        }
        ensure!(
            k.saved.hall_open == 0. && k.transforms() == closed,
            "Door did not close"
        );
        loaded.restore(&saved, map)?;
        loaded
            .motion
            .advance(0., map, &mut other, &mut q, &[], &mut loaded.saved)?;
        ensure!(loaded.snapshot() == saved, "Paused hallway door moved");
        for hold in [0., 0.25, 2.] {
            let mut legacy = saved.clone();
            legacy.as_object_mut().unwrap().remove("hall_open");
            legacy["hall"] = serde_json::json!(hold);
            loaded.restore(&legacy, map)?;
            ensure!(
                loaded.saved.hall_open == (hold / 0.5_f32).min(1.),
                "Legacy door pose changed"
            );
        }
        println!("PASS Keep hallway {hz}Hz: contact, timed mesh travel, hold, close, saved future and pause");
    }
    Ok(())
}
fn play(a: &mut Assets, map: &Bsp, kind: Kind, hz: u32, skip: bool) -> Result<()> {
    let (mut i, mut w, mut p, mut story) = setup(a, map)?;
    if kind != Kind::Arrival {
        for _ in 0..720 {
            tick(&mut i, map, &mut w, &mut p, &mut story, FIXED_DT)?;
        }
    }
    let k = owner(&mut i)?;
    if matches!(kind, Kind::Heart | Kind::Death) {
        k.saved.won = [true; 3];
        k.saved.heart_scene = true;
        k.saved.heart_open = true;
        k.saved.death_started = kind == Kind::Death;
    }
    if kind == Kind::Mirror {
        k.rotate();
    } else {
        k.begin(kind);
    }
    let mut lines = std::collections::BTreeSet::new();
    let mut saw_strike = false;
    let mut saw_weep = false;
    for n in 0..hz * 180 {
        if skip && n == hz {
            i.skip_cinematic(map, &mut w, &mut p, &mut story)?;
        }
        tick(&mut i, map, &mut w, &mut p, &mut story, 1. / hz as f32)?;
        let k = owner(&mut i)?;
        if let Some(s) = &k.saved.scene {
            lines.insert(s.line);
            saw_strike |= k.strike_time().is_some_and(|t| t >= 0.);
            saw_weep |= k.alice_act().0 == "weep_loop";
        }
        if n == hz * 2 {
            let before = k.snapshot();
            k.restore(&before, map)?;
            ensure!(before == k.snapshot(), "Scene restore changed cursor");
        }
        if !i.scripted() {
            break;
        }
    }
    ensure!(!i.scripted(), "Keep scene stalled {kind:?}");
    ensure!(w.body_clear(p.feet), "Keep handoff embedded Alice");
    if kind == Kind::Death {
        ensure!(owner(&mut i)?.saved.exit.committed, "Death lost exit");
        if !skip {
            ensure!(
                lines.len() == 4 && saw_strike && saw_weep,
                "Missing death performance: lines={lines:?}, strike={saw_strike}, weep={saw_weep}"
            );
        }
    }
    println!("PASS Keep {kind:?} {hz}Hz skip={skip}");
    Ok(())
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/keep.bsp")?)?;
    presentation(a, &map)?;
    for hz in [30, 60, 144] {
        for kind in [
            Kind::Arrival,
            Kind::Hint,
            Kind::Mirror,
            Kind::Heart,
            Kind::Death,
        ] {
            for skip in [false, true] {
                play(a, &map, kind, hz, skip)?;
            }
        }
    }
    let (mut i, mut w, mut p, mut story) = setup(a, &map)?;
    for _ in 0..1200 {
        tick(&mut i, &map, &mut w, &mut p, &mut story, FIXED_DT)?;
    }
    ensure!((p.feet.z - 120.).abs() < 1., "Lift failed {:?}", p.feet);
    for round in 0..3 {
        owner(&mut i)?.rotate();
        i.skip_cinematic(&map, &mut w, &mut p, &mut story)?;
        ensure!(
            owner(&mut i)?.saved.selected == Some(round),
            "Wrong mirror cycle"
        );
        owner(&mut i)?.enter_room(round);
        for _ in 0..125 {
            tick(&mut i, &map, &mut w, &mut p, &mut story, FIXED_DT)?;
        }
        ensure!(
            i.shot_targets().len() == 3,
            "Room did not arm exactly 3 portraits"
        );
        if round == 0 {
            for _ in 0..2 {
                let t = i
                    .switch_target("Hatter_Club_Lose")
                    .context("Wrong portrait missing")?;
                i.shoot(Hit {
                    id: t.id,
                    damage: 1.,
                    kind: crate::combat::DamageKind::Knife,
                    knockback: Vec3::ZERO,
                });
                ensure!(i.shot_targets().is_empty(), "Wrong room stayed armed");
                let k = owner(&mut i)?;
                ensure!(
                    k.saved.spawned.len() == 5 && !k.saved.won[0],
                    "Loss spawn state"
                );
                k.saved.loss_time = 1.5;
                k.saved.next = 0;
                k.rotate();
                i.skip_cinematic(&map, &mut w, &mut p, &mut story)?;
                owner(&mut i)?.enter_room(0);
                for _ in 0..125 {
                    tick(&mut i, &map, &mut w, &mut p, &mut story, FIXED_DT)?;
                }
            }
        }
        let thread = match round {
            0 => "Tweedle_Club_Win",
            1 => "Jabber_Diamond_Win",
            _ => "Hatter_Spade_Win",
        };
        let t = i
            .switch_target(thread)
            .context("Correct portrait missing")?;
        let h = Hit {
            id: t.id,
            damage: 1.,
            kind: crate::combat::DamageKind::Knife,
            knockback: Vec3::ZERO,
        };
        i.shoot(h);
        i.shoot(h);
        ensure!(
            owner(&mut i)?.saved.won.iter().filter(|v| **v).count() == round + 1,
            "Duplicate win counted"
        );
        let before = i.snapshot();
        let (mut other, _, _, _) = setup(a, &map)?;
        other.restore(&before, &map)?;
        ensure!(
            serde_json::to_value(&before)? == serde_json::to_value(other.snapshot())?,
            "Puzzle restore differs"
        );
        ensure!(
            !i.exit_volumes()
                .iter()
                .find(|v| v.index == 71)
                .unwrap()
                .live,
            "Premature physical exit"
        );
        for _ in 0..260 {
            tick(&mut i, &map, &mut w, &mut p, &mut story, FIXED_DT)?;
        }
        if i.scripted() {
            i.skip_cinematic(&map, &mut w, &mut p, &mut story)?;
        }
    }
    let k = owner(&mut i)?;
    ensure!(
        k.saved.heart_scene && k.saved.heart_open,
        "Heart gate did not open"
    );
    k.event("Keep_Cheshire_Dead");
    i.skip_cinematic(&map, &mut w, &mut p, &mut story)?;
    let mut requests = 0;
    for _ in 0..180 {
        requests += usize::from(
            tick(&mut i, &map, &mut w, &mut p, &mut story, FIXED_DT)?
                .transition
                .is_some(),
        );
    }
    ensure!(requests == 1, "Exit requested {requests} times");
    let k = owner(&mut i)?;
    ensure!(
        k.exit_contact(&EXIT.destination())
            .unwrap()
            .transition
            .is_none(),
        "Fallback duplicated exit"
    );
    k.transition_failed(&EXIT.destination());
    for _ in 0..125 {
        k.saved.exit.advance(FIXED_DT);
    }
    ensure!(
        k.saved.exit.request(EXIT).is_some(),
        "Failed transition cannot retry"
    );
    let saved = k.snapshot();
    k.restore(&saved, &map)?;
    ensure!(
        k.saved.exit.request(EXIT).is_some(),
        "Saved commitment cannot redeliver"
    );
    let mut bad = saved.clone();
    bad["won"] = serde_json::json!([false, false, false]);
    ensure!(
        k.restore(&bad, &map).is_err() && k.snapshot() == saved,
        "Invalid save changed live state"
    );
    // Exercise arrival-only event program migration with consumed old trigger history.
    let mut old = serde_json::to_value(Interactions::load(&map)?.snapshot())?;
    old["levels"] = serde_json::json!({"keep":{"version":1,"elapsed":2.5}});
    let old = serde_json::from_value(old)?;
    let (mut migrated, _, _, _) = setup(a, &map)?;
    migrated.restore(&old, &map)?;
    ensure!(
        owner(&mut migrated)?.saved.elapsed == 2.5,
        "Legacy lift lost progress"
    );
    println!("PASS Keep repeat loss, all suits, saved puzzle, gated/de-duplicated exit, retry and legacy migration");
    Ok(())
}
fn stage(i: &mut Interactions, map: &Bsp, n: usize) -> Result<()> {
    let k = owner(i)?;
    k.saved = Saved::default();
    k.saved.elapsed = 2.5;
    match n {
        1 => {
            k.rotate();
            k.saved.scene.as_mut().unwrap().time = 3.;
            k.saved.mirror_time = 1.5;
        }
        2 => {
            k.saved.lost = Some(0);
            k.saved.loss_time = 1.;
            k.spawn_group("club");
        }
        3 => {
            k.saved.won = [true, true, false];
            k.saved.selected = Some(1);
            k.saved.next = 2;
        }
        4 => {
            k.saved.won = [true; 3];
            k.saved.heart_scene = true;
            k.saved.death_started = true;
            k.begin(Kind::Death);
            k.saved.scene.as_mut().unwrap().time = 0.8;
        }
        5 | 6 => {
            k.saved.won = [true; 3];
            k.saved.heart_scene = true;
            k.saved.heart_open = true;
            k.saved.death_started = true;
            k.saved.queen_open = true;
            if n == 6 {
                k.saved.death_done = true;
                k.saved.exit.committed = true;
            } else {
                k.begin(Kind::Death);
                let t = k.strike_offset() + 1.15;
                let s = k.saved.scene.as_mut().unwrap();
                s.time = t;
                s.line = 2;
                s.starts[2] = 1.;
                s.line_time = 0.15;
                let at = k.data.at("cat_pace2");
                k.saved.head.update(
                    FIXED_DT,
                    0.15,
                    at.translation,
                    at.rotation.to_euler(EulerRot::ZYX).0,
                    1.,
                    &World::from_bsp(map)?,
                    Some(&k.data.sever),
                );
            }
        }
        _ => {}
    }
    k.motion.rebuild(map, &k.saved)
}
pub(super) const SAVES: &[super::super::SaveCase] = &[
    super::super::SaveCase {
        name: "keep-arrival",
        visit: "keep$first",
        stage: Some(|i, m| stage(i, m, 0)),
        behavior: None,
    },
    super::super::SaveCase {
        name: "keep-mirror-mid",
        visit: "keep$first",
        stage: Some(|i, m| stage(i, m, 1)),
        behavior: None,
    },
    super::super::SaveCase {
        name: "keep-room-lost",
        visit: "keep$first",
        stage: Some(|i, m| stage(i, m, 2)),
        behavior: None,
    },
    super::super::SaveCase {
        name: "keep-two-wins",
        visit: "keep$first",
        stage: Some(|i, m| stage(i, m, 3)),
        behavior: None,
    },
    super::super::SaveCase {
        name: "keep-cheshire-mid",
        visit: "keep$first",
        stage: Some(|i, m| stage(i, m, 4)),
        behavior: None,
    },
    super::super::SaveCase {
        name: "keep-cheshire-strike",
        visit: "keep$first",
        stage: Some(|i, m| stage(i, m, 5)),
        behavior: None,
    },
    super::super::SaveCase {
        name: "keep-exit-committed",
        visit: "keep$first",
        stage: Some(|i, m| stage(i, m, 6)),
        behavior: None,
    },
];
pub(super) fn route(a: &mut Assets) -> Result<()> {
    route_inner(a, false)
}
pub(super) fn skip_route(a: &mut Assets) -> Result<()> {
    route_inner(a, true)
}
fn route_inner(a: &mut Assets, skip: bool) -> Result<()> {
    let mut r = crate::route::Route::new(a, "keep", Some("keep_start1"))?;
    r.stats.notarget = true;
    r.stats.god = true; // Isolate traversal and puzzle checks from repeated fall/combat damage.
    r.aim_at = Some(usize::MAX);
    r.skip_cinematics = skip;
    drive(&mut r)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.world.body_clear(next.player.feet),
        "Qlair arrival blocked"
    );
    Ok(())
}
fn approach_lever(r: &mut crate::route::Route) -> Result<()> {
    for p in [
        vec3(512., 1100., 128.),
        vec3(512., 1300., 128.),
        vec3(512., 1600., 128.),
        vec3(0., 1750., 256.),
        vec3(0., 1930., 256.),
        vec3(-180., 2470., 256.),
        vec3(32., 3000., 256.),
        vec3(512., 3208., 272.),
    ] {
        nav(r, p)?;
        if p == vec3(512., 1300., 128.) && r.stats.copies(9) > 0
            && r.stats.powers.recharge <= 0. && r.stats.will() >= 1.5
        {
            // Let the contact door finish opening before stopping world time.
            r.wait(1.1)?;
            r.use_watch()?;
            r.heavy_weapon = None;
            r.ice_stream = false;
            r.conserve_will = true;
        }
        if p == vec3(512., 1600., 128.) || p == vec3(512., 3208., 272.) {
            r.clear(350.)?;
            r.heavy_weapon = Some(7);
            r.ice_stream = true;
            r.conserve_will = false;
            r.will_reserve = 0.;
            r.clear(900.)?;
        }
    }
    Ok(())
}
fn return_to_lobby(r: &mut crate::route::Route, _suit: usize) -> Result<()> {
    nav(r, vec3(456., -544., 512.))?;
    for p in [
        vec3(376., -544., 144.),
        vec3(80., -700., 144.),
        vec3(48., -640., 144.),
        vec3(48., -480., 144.),
        vec3(96., -370., 144.),
        vec3(340., -240., 144.),
        vec3(512., 0., 144.),
        vec3(512., 500., 144.),
    ] {
        nav(r, p)?;
    }
    Ok(())
}
fn nav(r: &mut crate::route::Route, p: Vec3) -> Result<()> {
    let goal = [0., 16., 32., 64., 128., 256., 512.]
        .into_iter()
        .find_map(|z| {
            r.world
                .actor_footing(p + Vec3::Z * z, PLAYER_CENTER, PLAYER_HALF, 512.)
        })
        .with_context(|| format!("Keep route footing {p:?}"))?;
    println!("Keep walk {:?} -> {goal:?}", r.player.feet);
    r.navigate_until_scene(goal)?;
    r.wait_for_cinematic()?;
    Ok(())
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let mut renderer = crate::render::Scene::load(a, "keep")?;
        let mut art = art::Art::load(a)?;
        for (name, kind, time, line) in [
            ("arrival", Kind::Arrival, 7., 0),
            ("mirror-club", Kind::Mirror, 7., 0),
            ("mirror-diamond", Kind::Mirror, 7., 1),
            ("mirror-spade", Kind::Mirror, 7., 2),
            ("death-cat", Kind::Death, 8., 0),
            ("death-strike", Kind::Death, 40., 2),
            ("death-weep", Kind::Death, 60., 3),
        ] {
            let (mut i, mut w, mut p, mut story) = setup(a, &renderer.map)?;
            let k = owner(&mut i)?;
            k.saved.elapsed = 5.;
            k.saved.won = if kind == Kind::Death {
                [true; 3]
            } else {
                [false; 3]
            };
            k.saved.heart_scene = kind == Kind::Death;
            k.saved.death_started = kind == Kind::Death;
            k.saved.queen_open = kind == Kind::Death;
            k.saved.doors[4] = 1.;
            if kind == Kind::Mirror {
                k.saved.next = line;
                k.rotate();
                k.saved.mirror_time = 5.;
            } else {
                k.begin(kind);
            }
            let offset = k.strike_offset();
            let s = k.saved.scene.as_mut().unwrap();
            s.time = time;
            s.home = Some(Transform {
                translation: p.feet,
                rotation: Quat::IDENTITY,
            });
            if kind == Kind::Death {
                s.line = line;
                s.starts[2] = time - 0.15 - offset;
                if name == "death-weep" {
                    k.saved.scene.as_mut().unwrap().starts[2] -= 18.;
                }
            }
            k.motion.rebuild(&renderer.map, &k.saved)?;
            let mut c = k.scene_camera().unwrap();
            if kind == Kind::Mirror {
                let target = [
                    vec3(-17., 2700., 96.),
                    vec3(512., 3005., 128.),
                    vec3(1040., 2699., 96.),
                ][line];
                let source = vec3(512., 2456., 100.);
                let normal = Quat::from_rotation_z(ANGLES[line].to_radians()) * Vec3::Y;
                let d = target - source;
                c.eye = source - (d - 2. * normal * d.dot(normal)).normalize() * 400.;
                c.target = source;
                c.up = Vec3::Z;
            }
            if kind == Kind::Death && name == "death-strike" {
                let at = k.data.at("cat_pace2");
                k.saved.head.update(
                    1. / 120.,
                    0.15,
                    at.translation,
                    at.rotation.to_euler(EulerRot::ZYX).0,
                    1.,
                    &w,
                    Some(&k.data.sever),
                );
            }
            let camera = Camera3D {
                position: c.eye,
                target: c.target,
                up: c.up,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            };
            renderer.set_sky_origin(k.sky_origin().unwrap());
            let (extra, hide) = k.reflection();
            renderer.set_reflection(extra, hide);
            for frame in 0..3 {
                let transforms = k.transforms();
                renderer.prepare_camera_portals(&camera, time, false, &transforms);
                clear_background(BLACK);
                set_camera(&camera);
                crate::render_fx::begin_view(&camera, time, &renderer.atmosphere, false);
                renderer.draw(c.eye, time, false, false, &transforms);
                art.draw(k, &renderer.atmosphere, c.eye, false);
                renderer.draw(c.eye, time, false, true, &transforms);
                art.effects(k, c.eye, &renderer.atmosphere);
                crate::render_fx::finish();
                set_default_camera();
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/keep-puzzle/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
            let _ = (&mut w, &mut p, &mut story);
        }
        let mut k = Keep::load(a, &renderer.map)?;
        k.saved.elapsed = 5.;
        k.saved.hall = 2.;
        k.motion.rebuild(&renderer.map, &k.saved)?;
        let mut w = World::from_bsp(&renderer.map)?;
        let feet = w
            .actor_footing(vec3(512., 1200., 160.), PLAYER_CENTER, PLAYER_HALF, 128.)
            .context("Missing hall capture footing")?;
        let mut p = Player::new(feet);
        let camera = Camera3D {
            position: vec3(512., 1100., 195.),
            target: vec3(512., 1380., 220.),
            up: Vec3::Z,
            fovy: 75_f32.to_radians(),
            z_near: 2.,
            z_far: 20000.,
            ..Default::default()
        };
        renderer.set_sky_origin(k.sky_origin().unwrap());
        renderer.set_reflection(vec![], vec![]);
        for n in 0..4 {
            let dt = match n {
                0 => 0.,
                3 => 0.5,
                _ => 0.25,
            };
            k.motion
                .advance(dt, &renderer.map, &mut w, &mut p, &[], &mut k.saved)?;
            let expected = if n == 3 { 1. } else { n as f32 * 0.25 };
            ensure!(
                k.saved.hall_open == expected,
                "Hall capture blocked movement"
            );
            let transforms = k.transforms();
            for frame in 0..3 {
                renderer.prepare_camera_portals(&camera, 0., false, &transforms);
                clear_background(BLACK);
                set_camera(&camera);
                crate::render_fx::begin_view(&camera, 0., &renderer.atmosphere, false);
                renderer.draw(camera.position, 0., false, false, &transforms);
                renderer.draw(camera.position, 0., false, true, &transforms);
                crate::render_fx::finish();
                set_default_camera();
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/keep-puzzle/hall-{n}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        Ok(())
    })
}

pub(crate) fn drive(r: &mut crate::route::Route) -> Result<()> {
    let skip = r.skip_cinematics;
    r.tactics = true;
    r.ice_stream = true;
    r.heavy_weapon = Some(7);
    r.will_reserve = 2.;
    r.stop_at_exit = true;
    r.wait_for_cinematic()?;
    if r.stats.powers.recharge > 0. {
        r.wait(r.stats.powers.recharge + 0.1)?;
    }
    r.wait(5.)?;
    println!("Keep route starts {:?}", r.player.feet);
    for i in 0..3 {
        if i > 0 {
            return_to_lobby(r, i - 1)?;
        }
        approach_lever(r)?;
        r.wait_for_cinematic()?;
        nav(r, vec3(512., 3138., 272.))?;
        // Navigation may finish within 24 units; interaction needs the final approach.
        r.walk(vec3(512., 3138., 280.), false)?;
        let at = owner(&mut r.interactions)?
            .data
            .at("mirror_lever")
            .point(vec3(36., 0., 26.));
        let wish = (at - r.player.eye()).truncate().normalize_or_zero();
        ensure!(
            r.interactions
                .prompt(&r.world, r.player.eye(), wish.extend(0.))
                .is_some(),
            "Keep lever unreachable at {:?}",
            r.player.feet
        );
        r.tick(Controls {
            wish,
            use_pressed: true,
            ..Default::default()
        })?;
        r.wait_for_cinematic()?;
        ensure!(
            owner(&mut r.interactions)?.saved.selected == Some(i),
            "Lever selected wrong suit"
        );
        for p in [
            vec3(32., 3000., 256.),
            vec3(-180., 2470., 256.),
            vec3(0., 1930., 256.),
            vec3(0., 1750., 256.),
            vec3(512., 1600., 128.),
            vec3(512., 1450., 128.),
            vec3(512., 1100., 128.),
            vec3(512., 500., 128.),
            vec3(376., -544., 144.),
        ] {
            nav(r, p)?;
        }
        nav(r, vec3(376., -384., 144.))?;
        r.clear(900.)?;
        r.walk(vec3(374., -376., 128.), false)?;
        let mut steam_seen = false;
        for _ in 0..1800 {
            steam_seen |= r.player.steam;
            if r.player.feet.z > 548. {
                break;
            }
            let delta = vec2(512., -384.) - r.player.feet.truncate();
            r.tick(Controls {
                wish: (delta * 2.5 / 210.).clamp_length_max(1.),
                jump: r.player.grounded,
                ..Default::default()
            })?;
        }
        ensure!(
            steam_seen && r.player.feet.z > 548.,
            "Keep steam did not lift Alice: {:?}",
            r.player.feet
        );
        r.walk(vec3(456., -544., 512.), false)?;
        let path: &[Vec3] = match i {
            0 => &[
                vec3(512., -850., 512.),
                vec3(512., -1100., 512.),
                vec3(512., -2210., 512.),
            ],
            1 => &[
                vec3(-530., 256., 512.),
                vec3(-730., 256., 512.),
                vec3(-950., 256., 512.),
                vec3(-2060., 256., 512.),
            ],
            _ => &[
                vec3(1560., 256., 512.),
                vec3(1752., 256., 512.),
                vec3(1950., 256., 512.),
                vec3(3100., 256., 512.),
            ],
        };
        for &p in path {
            nav(r, p)?;
        }
        r.wait(1.1)?;
        let n = match i {
            0 => "Tweedle_Club_Win",
            1 => "Jabber_Diamond_Win",
            _ => "Hatter_Spade_Win",
        };
        r.shoot_switch(n)?;
        r.aim_at = None;
        r.wait(2.2)?;
        r.wait_for_cinematic()?;
        ensure!(
            owner(&mut r.interactions)?.saved.won[i],
            "Portrait shot missed"
        );
        for &p in path.iter().rev().skip(1).take(1) {
            nav(r, p)?;
        }
        nav(
            r,
            match i {
                0 => vec3(512., -1000., 512.),
                1 => vec3(-872., 256., 512.),
                _ => vec3(1896., 256., 512.),
            },
        )?;
        r.wait(0.3)?;
        nav(r, path[0])?;
    }
    return_to_lobby(r, 2)?;
    approach_lever(r)?;
    nav(r, vec3(512., 3528., 272.))?;
    nav(r, vec3(512., 4100., 272.))?;
    nav(r, vec3(512., 4600., 272.))?;
    r.wait_for_cinematic()?;
    r.wait(0.1)?;
    ensure!(
        r.transition.as_ref().is_some_and(|d| EXIT.matches(d)),
        "Keep exit missing"
    );
    ensure!(
        r.teleports == 0 && r.stats.alive(),
        "Keep route lost continuity"
    );
    println!(
        "PASS Keep continuous route skip={skip}, ticks={}, shots={}, teleports={}",
        r.ticks, r.shots, r.teleports
    );
    Ok(())
}
