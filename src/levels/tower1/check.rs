use super::*;
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/tower1.bsp")?)?;
    let mut t = Tower::load(a, &map)?;
    println!("TOWER1 Cat idle {} talk {}", t.data.idle, t.data.talk);
    for hz in [30, 60, 144] {
        for n in 0..4 {
            t.saved.faces = [6.; 4];
            t.event(&format!("Face{}Thread", n + 1));
            let mut w = World::from_bsp(&map)?;
            let mut p = Player::spawn(&w, interaction::spawn(&map, None).0)
                .context("Blocked Tower spawn")?;
            t.upgraded();
            let mut gust = 0.;
            for _ in 0..hz * 6 {
                t.advance(1. / hz as f32, &map, &mut w, &mut p, &[])?;
                if t.gust(n) {
                    gust += 1. / hz as f32;
                }
            }
            ensure!((gust - 2.).abs() < 2. / hz as f32, "Wrong gust length");
        }
    }
    for time in [0., 0.34, 0.35, 0.54, 0.55, 1., 2.54, 2.55, 2.94, 2.95, 5.9] {
        t.saved.faces.fill(time);
        let saved = t.snapshot();
        let mut q = Tower::load(a, &map)?;
        q.restore(&saved, &map)?;
        let mut w = World::from_bsp(&map)?;
        let mut p =
            Player::spawn(&w, interaction::spawn(&map, None).0).context("Blocked Tower spawn")?;
        t.advance(0., &map, &mut w, &mut p, &[])?;
        ensure!(t.snapshot() == saved, "Paused face moved");
        for _ in 0..720 {
            t.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
            q.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
            ensure!(t.snapshot() == q.snapshot(), "Face save future diverged");
        }
    }
    for hz in [30, 60, 144] {
        for skip in [None, Some(0.2), Some(3.)] {
            let mut r = crate::route::Route::new(a, "tower1", None)?;
            let home = r.player.feet;
            for n in 0..hz * 60 {
                if skip.is_some_and(|v| n as f32 / hz as f32 >= v)
                    && r.interactions.scene_id_after(None).is_some()
                {
                    r.interactions.skip_cinematic(
                        &r.map,
                        &mut r.world,
                        &mut r.player,
                        &mut r.story,
                    )?;
                }
                r.interactions.advance_school(
                    1. / hz as f32,
                    &r.map,
                    &mut r.world,
                    &mut r.player,
                )?;
                if r.interactions.prepare_story(&mut r.story) {
                    r.story.tick(1. / hz as f32, false);
                }
                r.interactions.sync_cinematic_story(&r.story);
                for name in r.story.take_completed() {
                    r.interactions.completed_dialogue(&name);
                }
                if !r.interactions.scripted() {
                    break;
                }
            }
            ensure!(
                !r.interactions.scripted()
                    && r.story.has_seen(INTRO)
                    && r.player.feet == home
                    && r.player.script_motion == 0,
                "Tower1 intro failed to release Alice"
            );
            println!("PASS Tower1 intro {hz}Hz skip {skip:?}: stable landing and completed speech");
        }
    }

    t.upgraded();
    let mut w = World::fixture(&[]);
    t.traversal(&mut w.traversal);
    for id in [8, 15, 354] {
        let pad = w
            .traversal
            .actor_pushes
            .iter()
            .find(|p| p.id.0 == id)
            .unwrap();
        let (origin, direction) = (pad.origin, pad.direction);
        let body = crate::combat::Target {
            id: 0,
            center: origin,
            half: Vec3::splat(8.),
        };
        let mut recoil = crate::combat::Recoil::default();
        let delta = recoil.step(FIXED_DT, &w, body);
        ensure!(
            delta.dot(direction) > 1.,
            "Actor pad {id} did not push the body"
        );
        let saved = serde_json::to_value(&recoil)?;
        ensure!(
            recoil.step(0., &w, body) == Vec3::ZERO && serde_json::to_value(&recoil)? == saved,
            "Paused actor pad changed velocity"
        );
        let mut q: crate::combat::Recoil = serde_json::from_value(saved)?;
        ensure!(
            recoil.step(FIXED_DT, &w, body) == q.step(FIXED_DT, &w, body),
            "Actor pad velocity lost on reload"
        );
    }
    for (point, expected) in [
        (vec3(1600., 680., 1800.), vec![0]),
        (vec3(2080., 1100., 1800.), vec![1, 2]),
        (vec3(2400., 1200., 2500.), vec![3]),
    ] {
        let mut r = crate::route::Route::new(a, "tower1", None)?;
        owner(&mut r.interactions)?.upgraded();
        r.player.feet = point;
        r.interactions
            .update(FIXED_DT, &r.map, &mut r.world, &r.player, Vec3::X, false)?;
        r.interactions.triggers(FIXED_DT, point, point);
        for n in &expected {
            ensure!(
                owner(&mut r.interactions)?.saved.faces[*n] == 0.,
                "Authored face trigger did not fire"
            );
        }
        for _ in 0..120 * 6 + 3 {
            r.interactions
                .advance_school(FIXED_DT, &r.map, &mut r.world, &mut r.player)?;
            r.interactions
                .update(FIXED_DT, &r.map, &mut r.world, &r.player, Vec3::X, false)?;
            r.interactions.triggers(FIXED_DT, point, point);
        }
        for n in &expected {
            ensure!(
                owner(&mut r.interactions)?.saved.faces[*n] < 0.1,
                "Occupied face did not repeat after six seconds"
            );
        }
    }
    println!("PASS Tower1 saved gusts, pause, frame rates and watched/skipped arrival");
    Ok(())
}
pub(super) async fn render(a: &mut Assets) -> Result<()> {
    std::fs::create_dir_all("private/tower1-work")?;
    let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_TOWER1_FROM") {
        crate::route::Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?
    } else {
        crate::route::Route::new(a, "tower1", None)?
    };
    let time = std::env::var("LOOKING_GLASS_TOWER1_TIME")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(3.);
    if std::env::var_os("LOOKING_GLASS_TOWER1_FROM").is_none() {
        let t = owner(&mut r.interactions)?;
        t.saved.faces.fill(time);
        t.saved.scene.as_mut().unwrap().clock.time = time as f32;
        t.saved.scene.as_mut().unwrap().clock.home = Some(Transform {
            translation: r.player.feet,
            rotation: Quat::IDENTITY,
        });
    }
    let mut scene = crate::render::Scene::load(a, "tower1")?;
    let mut art = scene::Art::load(a)?;
    r.interactions.presentation.apply(&mut scene);
    let eye = std::env::var("LOOKING_GLASS_TOWER1_EYE")
        .ok()
        .and_then(|s| interaction::vector(&s));
    let target = std::env::var("LOOKING_GLASS_TOWER1_LOOK")
        .ok()
        .and_then(|s| interaction::vector(&s));
    let cam = r.interactions.camera_after(None, &r.world);
    let eye = eye.unwrap_or_else(|| cam.as_ref().map_or(r.player.eye(), |c| c.eye));
    let target = target.unwrap_or_else(|| cam.as_ref().map_or(eye + Vec3::Y * 300., |c| c.target));
    for frame in 0..3 {
        clear_background(BLACK);
        set_camera(&Camera3D {
            position: eye,
            target,
            up: Vec3::Z,
            fovy: 75_f32.to_radians(),
            z_near: 2.,
            z_far: 20000.,
            ..Default::default()
        });
        scene.draw(eye, time as f32, false, false, &r.interactions.transforms());
        art.draw(owner(&mut r.interactions)?, &scene.atmosphere, eye, false);
        art.effects(owner(&mut r.interactions)?, eye, &scene.atmosphere);
        crate::render::depth_read_only(|| {
            scene.draw(eye, time as f32, false, true, &r.interactions.transforms())
        });
        set_default_camera();
        if frame == 2 {
            crate::viewer::save_capture(std::path::Path::new("private/tower1-work/view.png"))?;
        }
        next_frame().await;
    }
    Ok(())
}

// The generic writer has already advanced the entrance. Stage every saved clock
// together, including when this fixture moves back into the opening fade.
fn stage_intro(i: &mut Interactions, time: f32) -> Result<()> {
    let scene = owner(i)?.saved.scene.as_mut().context("Missing Tower introduction")?;
    scene.clock.time = time;
    scene.clock.shot_time = time;
    scene.clock.cast_time = time;
    scene.clock.line_time = time;
    scene.validate()
}

pub(super) const SAVES: &[super::super::SaveCase] = &[
    super::super::SaveCase {
        name: "tower1-early",
        visit: "tower1$first",
        stage: Some(|i, _| {
            stage_intro(i, 0.25)?;
            Ok(())
        }),
        behavior: None,
    },
    super::super::SaveCase {
        name: "tower1-ending",
        visit: "tower1$first",
        stage: Some(|i, _| {
            stage_intro(i, 8.8)?;
            Ok(())
        }),
        behavior: None,
    },
    super::super::SaveCase {
        name: "tower1-intro",
        visit: "tower1$first",
        stage: Some(|i, _| {
            stage_intro(i, 5.)?;
            Ok(())
        }),
        behavior: None,
    },
    super::super::SaveCase {
        name: "tower1-gust",
        visit: "tower1$first",
        stage: Some(|i, _| {
            let t = owner(i)?;
            t.upgraded();
            t.saved.faces = [0.1, 0.6, 2.54, 2.8];
            Ok(())
        }),
        behavior: None,
    },
];

pub(super) async fn actors(a: &mut Assets) -> Result<()> {
    for difficulty in crate::powerups::Difficulty::ALL {
        let mut r = crate::route::Route::enter(
            a,
            "tower1",
            None,
            crate::inventory::Stats::for_level("tower1", None),
            difficulty,
        )?;
        owner(&mut r.interactions)?.upgraded();
        r.interactions
            .advance_school(FIXED_DT, &r.map, &mut r.world, &mut r.player)?;
        r.enable_native_cast(a)?;
        let npcs = r.native_cast.as_mut().unwrap();
        let json = serde_json::to_value(npcs.snapshot())?;
        let cast = json["actors"].as_array().unwrap();
        ensure!(
            cast.iter()
                .filter(|x| x["spawn"]["resident_spawn"].is_number())
                .count()
                == 10,
            "Tower spawn count changed"
        );
        ensure!(
            npcs.targets().is_empty(),
            "Tower dormant or precache cast active on entry"
        );
        for id in [16, 342, 344, 345, 347, 10, 350, 352, 356, 400] {
            let eye = interaction::vector(&r.map.entities[id]["origin"]).unwrap() + Vec3::Z * 20.;
            for _ in 0..3 {
                npcs.update(FIXED_DT, &r.world, eye);
            }
        }
        ensure!(
            npcs.targets().len() == 10,
            "Missing native Boojum activation"
        );
        let snapshot = npcs.snapshot();
        npcs.restore(&snapshot)?;
        ensure!(npcs.targets().len() == 10, "Active enemies lost on restore");
        let targets = npcs.targets();
        for target in targets {
            npcs.hit(crate::combat::Hit {
                id: target.id,
                damage: 1000.,
                knockback: Vec3::ZERO,
                kind: crate::combat::DamageKind::Other,
            });
        }
        ensure!(
            npcs.targets().is_empty(),
            "Boojum death did not retire combat target"
        );
        let dead = npcs.snapshot();
        npcs.restore(&dead)?;
        for id in [16, 342, 344, 345, 347, 10, 350, 352, 356, 400] {
            npcs.update(
                FIXED_DT,
                &r.world,
                interaction::vector(&r.map.entities[id]["origin"]).unwrap() + Vec3::Z * 20.,
            );
        }
        ensure!(
            npcs.targets().is_empty(),
            "Consumed ambush respawned after death/load"
        );
        let mut old = json.clone();
        let precache = old["actors"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|x| x["spawn"]["origin"][1].as_f64().is_some_and(|y| y > 6000.))
            .unwrap();
        ensure!(
            precache["spawn"]["hidden"] == true,
            "Precache actor visible"
        );
        precache["spawn"]["hidden"] = serde_json::json!(false);
        let old = serde_json::from_value(old)?;
        npcs.restore(&old)?;
        ensure!(
            npcs.targets().is_empty(),
            "Precache migration activated an enemy"
        );
        println!("PASS Tower1 {difficulty:?}: ten native activations, damage/death, no replay and precache migration");
    }
    Ok(())
}
