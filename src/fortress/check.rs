//! Real-data regression for both restored Fortress sequences.
use super::{
    cinema::{Beat, ARRIVAL, BOOJUM},
    Fortress,
};
use crate::{assets::Assets, interaction::Interactions, movement::Controls, route::Route};
use anyhow::{ensure, Result};
use macroquad::prelude::*;

pub fn check(assets: &mut Assets) -> Result<()> {
    let mut r = Route::new(assets, "fortress1", None)?;
    camera_coverage(&r)?;
    ensure!(r.interactions.scripted(), "Fresh arrival did not start");
    let mut saved = false;
    for _ in 0..12000 {
        if !r.interactions.scripted() {
            break;
        }
        r.tick(Controls::default())?;
        let f = r.interactions.fortress.as_ref().unwrap();
        if !saved && f.state.cinema.time >= 24. {
            let before = serde_json::to_value(r.interactions.snapshot())?;
            r.interactions
                .advance_school(0., &r.map, &mut r.world, &mut r.player)?;
            ensure!(
                before == serde_json::to_value(r.interactions.snapshot())?,
                "Pause advanced Fortress cinema"
            );
            let camera = r
                .interactions
                .fortress
                .as_ref()
                .unwrap()
                .cinema
                .camera(&r.interactions.fortress.as_ref().unwrap().state.cinema)
                .unwrap();
            let snapshot = r.interactions.snapshot();
            let mut restored = Interactions::load(&r.map)?;
            restored.set_entry(assets, &r.map, "fortress1", None)?;
            restored.restore(&snapshot, &r.map)?;
            let f = restored.fortress.as_ref().unwrap();
            let next = f.cinema.camera(&f.state.cinema).unwrap();
            ensure!(
                camera.eye == next.eye && camera.target == next.target,
                "Saved shot changed"
            );
            let dialogue = r.story.snapshot();
            let hints = crate::cheshire::Hints::load(assets, &r.map, "fortress1")?;
            r.story.restore(&dialogue, &hints)?;
            r.interactions = restored;
            saved = true;
        }
    }
    ensure!(
        saved && !r.interactions.scripted() && r.story.has_seen(ARRIVAL) && !r.story.busy(),
        "Arrival never finished"
    );
    let watched = r
        .interactions
        .fortress
        .as_ref()
        .unwrap()
        .state
        .cinema
        .clone();
    let landing = r.player.feet;
    let mut skipped = Route::new(assets, "fortress1", Some("fortress1_start1"))?;
    ensure!(
        skipped.interactions.skip_cinematic(
            &skipped.map,
            &mut skipped.world,
            &mut skipped.player,
            &mut skipped.story
        )?,
        "Skip rejected"
    );
    ensure!(
        serde_json::to_value(watched)?
            == serde_json::to_value(&skipped.interactions.fortress.as_ref().unwrap().state.cinema)?
            && landing.distance(skipped.player.feet) < 1.
            && skipped.world.body_clear(skipped.player.feet),
        "Watch/skip arrival endpoints differ"
    );
    ensure!(
        !skipped.interactions.skip_cinematic(
            &skipped.map,
            &mut skipped.world,
            &mut skipped.player,
            &mut skipped.story
        )?,
        "Arrival skip replayed"
    );
    // Saves predating this controller must stay playable instead of seizing the camera.
    let f = skipped.interactions.fortress.as_mut().unwrap();
    let mut legacy = serde_json::to_value(f.snapshot())?;
    legacy.as_object_mut().unwrap().remove("cinema");
    f.restore(&serde_json::from_value(legacy)?, &skipped.map)?;
    f.cinema.begin(&mut f.state.cinema, &mut skipped.story);
    ensure!(!f.state.cinema.active(), "Legacy save replayed arrival");
    let mut return_route = Route::new(assets, "fortress1", Some("fortress1_start2"))?;
    ensure!(
        !return_route.interactions.scripted(),
        "Return visit played arrival"
    );
    return_route
        .interactions
        .fortress
        .as_mut()
        .unwrap()
        .event(BOOJUM);
    return_route.interactions.activate_enemies();
    let mut before = return_route.interactions.snapshot();
    let home = return_route.player.feet;
    for _ in 0..60 {
        return_route.interactions.advance_school(
            0.1,
            &return_route.map,
            &mut return_route.world,
            &mut return_route.player,
        )?;
        if !return_route.interactions.scripted() {
            break;
        }
        if return_route
            .interactions
            .fortress
            .as_ref()
            .unwrap()
            .state
            .cinema
            .time
            >= 2.
        {
            before = return_route.interactions.snapshot();
        }
    }
    let actors = serde_json::to_value(
        return_route
            .interactions
            .encounters
            .as_ref()
            .unwrap()
            .snapshot(),
    )?;
    ensure!(
        !return_route.interactions.scripted() && return_route.player.feet == home,
        "Boojum reveal moved/locked Alice"
    );
    return_route
        .interactions
        .restore(&before, &return_route.map)?;
    ensure!(
        return_route.interactions.scripted(),
        "Mid-reveal save did not resume"
    );
    return_route.interactions.skip_cinematic(
        &return_route.map,
        &mut return_route.world,
        &mut return_route.player,
        &mut return_route.story,
    )?;
    ensure!(
        actors
            == serde_json::to_value(
                return_route
                    .interactions
                    .encounters
                    .as_ref()
                    .unwrap()
                    .snapshot()
            )?,
        "Boojum skip changed ambush endpoint"
    );
    return_route
        .interactions
        .fortress
        .as_mut()
        .unwrap()
        .event(BOOJUM);
    ensure!(
        !return_route.interactions.scripted(),
        "Boojum scene replayed"
    );
    println!("PASS Fortress arrival/return: watched and skipped outcomes, voiced story, paused clocks, mid-scene resume, legacy saves, one-shot ambush");
    Ok(())
}

fn camera_coverage(r: &Route) -> Result<()> {
    let f = r.interactions.fortress.as_ref().unwrap();
    let mut state = f.state.cinema.clone();
    let mut previous: Option<crate::cinematic::Camera> = None;
    let cuts = [10.5, 34.5, 41.5, 44.5, 51.3, 64.5, 72.5, 82.5];
    let mut max_move = 0_f32;
    let mut max_turn = 0_f32;
    let mut solid = Vec::new();
    let cinematic_brushes = f
        .objects
        .iter()
        .filter(|o| o.name == "drawbridge" || o.name.starts_with("stal"))
        .map(|o| {
            Ok((
                o,
                crate::collision::Collider::model(
                    &r.map,
                    o.model,
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    true,
                )?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut moving_solid = Vec::new();
    for frame in 0..10020 {
        let time = frame as f32 / 120.;
        state.time = time;
        let c = f.cinema.camera(&state).unwrap();
        ensure!(
            c.eye.is_finite() && c.target.is_finite() && c.up.is_finite(),
            "Nonfinite Fortress camera at {time}"
        );
        if f.cinema.fade(&state).1 < 0.99 {
            let trace = r.world.sweep_geometry(c.eye, c.eye, Vec3::splat(2.));
            if trace.start_solid {
                solid.push(time);
            }
            for (object, collider) in &cinematic_brushes {
                if let Some((p, q)) = f.cinema.brush(&object.name, object.base, &state) {
                    let eye = q.inverse() * (c.eye - p);
                    if collider.trace(eye, eye, Vec3::splat(2.)).start_solid {
                        moving_solid.push((time, object.name.as_str()));
                    }
                }
            }
        }
        if let Some(p) = previous {
            if !cuts.iter().any(|cut| (time - cut).abs() < 0.01) {
                max_move = max_move.max(c.eye.distance(p.eye));
                max_turn = max_turn.max(
                    (c.target - c.eye)
                        .normalize()
                        .angle_between((p.target - p.eye).normalize())
                        .to_degrees(),
                );
            }
        }
        previous = Some(c);
    }
    println!("Fortress camera coverage: max step {max_move:.3}, turn {max_turn:.3}, solid samples {} first {:?}",solid.len(), &solid[..solid.len().min(12)]);
    println!(
        "Fortress moving brush camera coverage: {} first {:?}",
        moving_solid.len(),
        &moving_solid[..moving_solid.len().min(12)]
    );
    ensure!(solid.is_empty(), "Fortress camera entered static geometry");
    ensure!(
        moving_solid.is_empty(),
        "Fortress camera entered moving scenery"
    );
    ensure!(
        max_move < 8. && max_turn < 1.,
        "Fortress camera jumped outside a shot cut"
    );
    Ok(())
}

pub async fn render_check(assets: &mut Assets) -> Result<()> {
    let mut player_art = crate::character::Character::load(assets)?;
    let mut scene = crate::render::Scene::load(assets, "fortress1")?;
    let mut f = Fortress::load(assets, &scene.map, false)?;
    let mut art = super::cinema::Art::load(assets)?;
    let mut enemies = crate::encounters::Encounters::load(
        assets,
        &scene.map,
        "fortress1",
        Some("fortress1_start2"),
    )?;
    let mut enemy_art = crate::encounters::Art::load(assets)?;
    for (name, beat, time) in [
        ("approach", Beat::Arrival, 4.),
        ("cave", Beat::Arrival, 23.),
        ("cave-early", Beat::Arrival, 17.),
        ("cave-late", Beat::Arrival, 31.),
        ("exterior", Beat::Arrival, 38.),
        ("spotter", Beat::Arrival, 42.5),
        ("bridge", Beat::Arrival, 47.),
        ("bridge-open", Beat::Arrival, 50.5),
        ("chase", Beat::Arrival, 56.),
        ("chase-late", Beat::Arrival, 62.),
        ("tower", Beat::Arrival, 67.),
        ("tower-wall", Beat::Arrival, 71.),
        ("escape", Beat::Arrival, 77.),
        ("escape-late", Beat::Arrival, 80.),
        ("landing", Beat::Arrival, 83.2),
        ("boojums", Beat::Boojum, 2.),
    ] {
        f.state.cinema.beat = Some(beat);
        f.state.cinema.time = time;
        f.state.returning = beat == Beat::Boojum;
        f.state.shutters = f.state.returning;
        enemies.activate("s1_booj1");
        enemies.activate("s1_booj2");
        f.cinema.place_boojums(time, &mut enemies);
        for frame in 0..3 {
            clear_background(BLACK);
            let c = f.cinema.camera(&f.state.cinema).unwrap();
            set_camera(&Camera3D {
                position: c.eye,
                target: c.target,
                up: c.up,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 30000.,
                ..Default::default()
            });
            let transforms = f.transforms().collect::<Vec<_>>();
            scene.draw(c.eye, time, false, false, &transforms);
            art.draw(&f, &scene.atmosphere, c.eye, false);
            if beat == Beat::Boojum {
                enemy_art.draw(&enemies, false, &scene.atmosphere, c.eye);
            }
            crate::render::depth_read_only(|| scene.draw(c.eye, time, false, true, &transforms));
            set_default_camera();
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/fortress-cinema-{name}.png"
                )))?;
            }
            next_frame().await;
        }
        player_art
            .check_visible(&format!("fortress/{name}"))
            .await?;
        if beat == Beat::Arrival {
            for actor in match name {
                "approach" => &["child", "ship"][..],
                "spotter" => &["spotter"][..],
                "bridge" | "bridge-open" => &["bridge_guard1", "bridge_guard2"][..],
                "tower-wall" => &["tower_shooter1", "tower_shooter2"][..],
                "landing" => &["tag_alice"][..],
                _ => &["ship", "tag_alice", "tag_gnome"][..],
            } {
                let mut samples = Vec::new();
                for omit in [Some(*actor), None] {
                    clear_background(BLACK);
                    let c = f.cinema.camera(&f.state.cinema).unwrap();
                    set_camera(&Camera3D {
                        position: c.eye,
                        target: c.target,
                        up: c.up,
                        fovy: 75_f32.to_radians(),
                        z_near: 2.,
                        z_far: 30000.,
                        ..Default::default()
                    });
                    let transforms = f.transforms().collect::<Vec<_>>();
                    scene.draw(c.eye, time, false, false, &transforms);
                    art.draw_omitting(&f, &scene.atmosphere, c.eye, false, omit);
                    crate::render::depth_read_only(|| {
                        scene.draw(c.eye, time, false, true, &transforms)
                    });
                    set_default_camera();
                    samples.push(get_screen_data());
                    next_frame().await;
                }
                let pixels = crate::character::visible_pixels(&samples[0], &samples[1]);
                let required = match (name, *actor) {
                    ("approach", _) => 100,
                    ("cave" | "cave-late" | "chase" | "chase-late", _) => 300,
                    ("spotter" | "landing", _) => 500,
                    ("exterior" | "tower" | "escape" | "escape-late", "ship") => 500,
                    ("bridge", "bridge_guard1") => 100,
                    ("bridge-open" | "tower-wall", _) => 20,
                    _ => 0, // Establishing/entry shots may intentionally occlude a passenger.
                };
                ensure!(
                    pixels >= required,
                    "Fortress {name}/{actor} hidden: {pixels} pixels, expected {required}"
                );
                if required > 0 {
                    println!("PASS Fortress visibility {name}/{actor}: {pixels} pixels (minimum {required})");
                } else {
                    println!("INFO Fortress {name}/{actor}: {pixels} pixels; no visibility assertion in this establishing shot");
                }
            }
        }
    }
    println!("PASS Fortress cinematic cast/cameras rendered; staged captures written to private/fortress-cinema-*.png");
    Ok(())
}
