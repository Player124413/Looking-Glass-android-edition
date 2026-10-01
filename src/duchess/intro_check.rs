//! Pickup-scene camera clearance, collider equivalence and native presentation checks.
use super::*;
use std::time::{Duration, Instant};

fn begin(a: &mut Assets, map: &Bsp, world: &mut World) -> Result<(Duchess, Player, Stats, Story)> {
    let mut d = Duchess::load(a, map)?;
    let mut p = Player::new(vec3(-114., 66., 48.));
    let mut stats = Stats::for_level("potears3", None);
    let mut story = Story::load(a, "potears3");
    stats.collected.insert(d.pickup_id.clone());
    d.update(1. / 60., world, &mut p, &mut stats, &mut story);
    world.set_dynamic(d.colliders().collect());
    ensure!(
        d.state.stage == Stage::Expanding,
        "Pickup did not start room expansion"
    );
    Ok((d, p, stats, story))
}

fn step(
    d: &mut Duchess,
    map: &Bsp,
    world: &mut World,
    p: &mut Player,
    stats: &mut Stats,
    story: &mut Story,
) -> Result<()> {
    d.advance(1. / 60., map, world, p)?;
    story.tick(1. / 60., false);
    for event in story.take_completed() {
        d.dialogue_complete(&event);
    }
    d.update(1. / 60., world, p, stats, story);
    Ok(())
}

pub fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/potears3.bsp")?)?;
    let mut world = World::from_bsp(&map)?;
    let (mut d, mut p, mut stats, mut story) = begin(a, &map, &mut world)?;
    let mut frames = 0;
    let mut shots = std::collections::BTreeSet::new();
    let mut previous = None;
    let mut worst_step = 0_f32;
    let mut camera_bad = Vec::new();
    while d.cinematic() && frames < 9000 {
        let c = d.camera(&world).context("Missing intro camera")?;
        let shot = d.state.intro_camera.map_or(0, |v| v.0);
        shots.insert(shot);
        if world.sweep(c.eye, c.eye, Vec3::splat(4.)).start_solid {
            camera_bad.push((frames, shot, c.eye));
        }
        if let Some((last_shot, last_eye)) = previous {
            if last_shot == shot {
                worst_step = worst_step.max(c.eye.distance(last_eye));
            }
        }
        previous = Some((shot, c.eye));
        if frames == 90 || frames == 240 || shots.len() > 1 && frames % 180 == 0 {
            let before = serde_json::to_value(d.snapshot())?;
            d.advance(0., &map, &mut world, &mut p)?;
            d.update(0., &world, &mut p, &mut stats, &mut story);
            ensure!(
                before == serde_json::to_value(d.snapshot())?,
                "Paused intro moved"
            );
            let restored: State = serde_json::from_value(before)?;
            d.restore(&restored, &map)?;
            world.set_dynamic(d.colliders().collect());
            let resumed = d.camera(&world).unwrap();
            ensure!(
                resumed.eye == c.eye && resumed.target == c.target,
                "Camera changed after save restoration"
            );
        }
        step(&mut d, &map, &mut world, &mut p, &mut stats, &mut story)?;
        frames += 1;
    }
    ensure!(
        camera_bad.is_empty(),
        "Intro camera entered geometry in {} frames: {:?}",
        camera_bad.len(),
        &camera_bad[..camera_bad.len().min(8)]
    );
    ensure!(
        shots.len() == 4 && d.state.stage == Stage::Fighting,
        "Introduction did not play four shots and return to combat"
    );
    ensure!(
        worst_step < 16.,
        "Camera jumped {worst_step} units within a shot"
    );
    println!("PASS Duchess intro: {frames} clear camera samples, four shots, pause/save restoration, max frame travel {worst_step:.3}");

    // Compare actual transformed collision to the prior brush builder at moving
    // and rotating poses. Include the cost of a first query on the lazy cache.
    let mut old_time = Duration::ZERO;
    let mut cached_time = Duration::ZERO;
    let mut comparisons = 0;
    for (stage, time) in [
        (Stage::Expanding, 0.2),
        (Stage::Expanding, 1.5),
        (Stage::Expanding, 2.9),
        (Stage::Well, 1.5),
    ] {
        d.state.stage = stage;
        d.state.time = time;
        for object in &d.objects {
            let pose = d.object_pose(object);
            let now = Instant::now();
            let old = Collider::model(&map, object.model, pose.translation, pose.rotation, true)?;
            old_time += now.elapsed();
            let center = pose.translation
                + pose.rotation
                    * ((map.models[object.model].min + map.models[object.model].max) * 0.5);
            let now = Instant::now();
            let cached = object.shape.at(pose.translation, pose.rotation);
            let _ = cached.trace(center, center + Vec3::Z, Vec3::splat(4.));
            cached_time += now.elapsed();
            for axis in [Vec3::X, Vec3::Y, Vec3::Z, vec3(1., 1., 1.).normalize()] {
                for half in [Vec3::splat(4.), crate::collision::PLAYER_HALF] {
                    let start = center - axis * 2048.;
                    let end = center + axis * 2048.;
                    let a = old.trace(start, end, half);
                    let b = cached.trace(start, end, half);
                    ensure!(
                        a.start_solid == b.start_solid
                            && (a.fraction - b.fraction).abs() < 0.0001
                            && a.normal.distance(b.normal) < 0.001,
                        "Changed collider {}",
                        object.name
                    );
                    comparisons += 1;
                }
            }
        }
    }
    println!("PASS Duchess cached collision: {comparisons} equivalent sweeps; old construction {old_time:?}, cached placement plus first query {cached_time:?}");
    Ok(())
}

pub async fn render(a: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(a, "potears3")?;
    let (mut d, mut p, mut stats, mut story) = begin(a, &scene.map, &mut scene.world)?;
    let mut art = Art::load(a, &scene.map)?;
    let mut captures = std::collections::BTreeSet::new();
    let mut frames = 0;
    let mut updates = Vec::new();
    while d.cinematic() && frames < 9000 {
        let c = d.camera(&scene.world).context("Missing intro shot")?;
        ensure!(
            !scene.world.sweep(c.eye, c.eye, Vec3::splat(4.)).start_solid,
            "Native intro camera inside a wall"
        );
        clear_background(BLACK);
        let camera = Camera3D {
            position: c.eye,
            target: c.target,
            up: c.up,
            fovy: 75_f32.to_radians(),
            z_near: 2.,
            z_far: 5000.,
            ..Default::default()
        };
        set_camera(&camera);
        crate::lighting::select(vec![], c.eye, &scene.world);
        crate::render_fx::begin_view(&camera, frames as f32 / 60., &scene.atmosphere, false);
        let transforms = d.transforms().collect::<Vec<_>>();
        scene.draw(c.eye, frames as f32 / 60., false, false, &transforms);
        art.story_pose(&story);
        art.draw(&d, false, &scene.atmosphere, c.eye);
        crate::render::depth_read_only(|| {
            scene.draw(c.eye, frames as f32 / 60., false, true, &transforms);
            art.effects(&d, c.eye, &scene.atmosphere);
        });
        crate::render_fx::finish();
        set_default_camera();
        let shot = d.state.intro_camera.map_or(0, |v| v.0);
        let label = if frames <= 252 && frames % 30 == 0 {
            Some(format!("expansion-{frames:03}"))
        } else if shot > 0 && d.state.intro_camera.unwrap().1 > 1. {
            Some(format!("dialogue-{shot}"))
        } else {
            None
        };
        if let Some(label) = label.filter(|label| captures.insert(label.clone())) {
            crate::viewer::save_capture(std::path::Path::new(&format!(
                "private/intro-{label}.png"
            )))?;
        }
        next_frame().await;
        let now = Instant::now();
        step(
            &mut d,
            &scene.map,
            &mut scene.world,
            &mut p,
            &mut stats,
            &mut story,
        )?;
        updates.push(now.elapsed().as_secs_f64() * 1000.);
        frames += 1;
    }
    ensure!(
        d.state.stage == Stage::Fighting && captures.contains("dialogue-3"),
        "Native intro failed to complete"
    );
    updates.sort_by(|a, b| a.total_cmp(b));
    println!("PASS native Duchess pickup scene: {frames} frames, {} captures, update p95 {:.3} ms, max {:.3} ms", captures.len(), updates[updates.len()*95/100], updates[updates.len()-1]);
    Ok(())
}
