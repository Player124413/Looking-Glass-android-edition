//! Asset-backed checks of the upper hall's two ambient door leaves.
use super::*;
use anyhow::ensure;

const NAMES: [&str; 2] = ["slamming_door1", "slamming_door2"];

fn expected(seconds: f32, index: usize) -> f32 {
    let (peak, outward, inward) = [(32., 0.4, 0.3), (-26., 0.2, 0.4)][index];
    let phase = seconds.rem_euclid(outward + inward);
    peak * if phase <= outward {
        phase / outward
    } else {
        1. - (phase - outward) / inward
    }
}

pub fn check(assets: &mut Assets, map: &Bsp) -> Result<()> {
    let wav = assets.read("sound/world/mover/flapping_door_loop.wav")?;
    ensure!(
        wav.starts_with(b"RIFF"),
        "Door loop does not resolve to WAV audio"
    );
    for returning in [false, true] {
        for fps in [30, 60, 144] {
            let mut school = School::load(map, returning)?;
            let mut world = World::from_bsp(map)?;
            let mut player = Player::new(map.spawn().0);
            for tick in 0..fps * 5 {
                school.advance(1. / fps as f32, map, &mut world, &mut player, &[])?;
                for (index, name) in NAMES.iter().enumerate() {
                    let door = school.object(name);
                    ensure!(door.enabled && door.solid, "Door leaf missing or non-solid");
                    ensure!(
                        (door.pose.angles.z - expected((tick + 1) as f32 / fps as f32, index))
                            .abs()
                            < 0.02,
                        "Door timing differs at {fps} Hz: {name} tick {tick}, angle {}",
                        door.pose.angles.z
                    );
                    ensure!(
                        school.transforms().any(|(model, _, _)| model == door.model),
                        "Door has no draw transform"
                    );
                }
                if tick % 37 == 0 {
                    let saved = serde_json::to_value(school.snapshot())?;
                    school.advance(0., map, &mut world, &mut player, &[])?;
                    ensure!(
                        saved == serde_json::to_value(school.snapshot())?,
                        "Paused doors moved"
                    );
                    let mut restored = School::load(map, returning)?;
                    restored.restore(&serde_json::from_value(saved.clone())?, map)?;
                    ensure!(
                        saved == serde_json::to_value(restored.snapshot())?,
                        "Saved door phase changed"
                    );
                    school = restored;
                }
            }
            let mut loops = Vec::new();
            school.sound_state(&mut loops, &mut Vec::new());
            let door_loops: Vec<_> = loops
                .iter()
                .filter(|l| l.path.ends_with("flapping_door_loop.wav"))
                .collect();
            ensure!(
                door_loops.len() == 1 && door_loops[0].clock == Some(school.clock),
                "Missing or duplicate door loop"
            );

            // A real movement sweep must stay inside the hall even when the
            // independently moving leaves leave a visible crack between them.
            let mut school = School::load(map, returning)?;
            let mut player = Player::new(vec3(-1280., 2624., -255.96875));
            let mut clock = crate::movement::FixedClock::default();
            world.set_dynamic(school.colliders().collect());
            ensure!(
                world.body_clear(player.feet),
                "Door approach fixture obstructed"
            );
            for _ in 0..fps * 5 {
                school.advance(1. / fps as f32, map, &mut world, &mut player, &[])?;
                clock.advance(
                    1. / fps as f64,
                    &world,
                    &mut player,
                    crate::movement::Controls {
                        wish: vec2(-1., 0.),
                        ..Default::default()
                    },
                );
                ensure!(
                    world.body_clear(player.feet),
                    "Swinging door embedded Alice"
                );
                ensure!(
                    player.feet.x > -1470. && player.feet.z > -260.,
                    "Alice crossed the doors into the void: {:?}",
                    player.feet
                );
            }
            ensure!(player.feet.x < -1360., "Approach did not reach the doors");
            println!("PASS upper hall doors, return={returning}, {fps} Hz: motion, pause, save and blocked passage");
        }

        let mut school = School::load(map, returning)?;
        school.event("Skool1_OG_MoveShelf");
        let mut legacy = school.snapshot();
        legacy.objects.retain(|o| !flapping_door(&o.name));
        let before = serde_json::to_value(&legacy)?;
        let mut restored = School::load(map, returning)?;
        restored.restore(&legacy, map)?;
        ensure!(
            NAMES
                .iter()
                .all(|name| restored.object(name).motion.is_some()),
            "Older save did not restore ambient doors"
        );
        let mut after = restored.snapshot();
        after.objects.retain(|o| !flapping_door(&o.name));
        ensure!(
            before == serde_json::to_value(after)?,
            "Door migration changed saved progression"
        );
        legacy.objects.retain(|o| o.name != "secret_shelf");
        ensure!(
            restored.restore(&legacy, map).is_err(),
            "Migration accepted a missing puzzle mover"
        );
    }

    let mut school = School::load(map, false)?;
    let mut world = World::from_bsp(map)?;
    let mut player = Player::new(vec3(-1456., 2596., -255.96875));
    world.set_dynamic(school.colliders().collect());
    ensure!(
        world.body_clear(player.feet),
        "Closing-door fixture obstructed"
    );
    for _ in 0..240 {
        school.advance(1. / 120., map, &mut world, &mut player, &[])?;
        ensure!(world.body_clear(player.feet), "Ambient door crushed Alice");
    }
    let stopped = school.object(NAMES[0]).pose.angles.z;
    ensure!(
        stopped > 0. && stopped < 31.,
        "Door failed to stop for Alice: {stopped}"
    );
    school.advance(0.1, map, &mut world, &mut player, &[])?;
    ensure!(
        school.object(NAMES[0]).pose.angles.z == stopped,
        "Blocked leaf kept moving"
    );
    player.feet = map.spawn().0;
    school.advance(0.1, map, &mut world, &mut player, &[])?;
    ensure!(
        (school.object(NAMES[0]).pose.angles.z - stopped).abs() > 1.,
        "Blocked leaf did not resume"
    );
    println!("PASS ambient doors: original sound, older-save upgrade and no-crush stop/resume");
    Ok(())
}

pub async fn render(assets: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(assets, "skool1")?;
    let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
    interactions.set_entry(assets, &scene.map, "skool1", None)?;
    interactions.sync(&mut scene.world);
    let mut player = Player::new(scene.map.spawn().0);
    let eye = vec3(-1200., 2624., -163.);
    let target = vec3(-1424., 2624., -185.);
    let mut captures = Vec::new();
    for (name, seconds) in [("closed", 0.), ("swing", 0.2), ("opposed", 0.4)] {
        while interactions.school.as_ref().unwrap().clock + 0.00001 < seconds {
            interactions.advance_school(1. / 120., &scene.map, &mut scene.world, &mut player)?;
        }
        for frame in 0..3 {
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: eye,
                target,
                up: Vec3::Z,
                fovy: 65_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            });
            crate::render_fx::begin(eye, target - eye, seconds, &scene.atmosphere, false);
            let transforms = interactions.transforms();
            scene.draw(eye, seconds, false, false, &transforms);
            crate::render::depth_read_only(|| scene.draw(eye, seconds, false, true, &transforms));
            crate::render_fx::finish();
            set_default_camera();
            if frame == 2 {
                captures.push(get_screen_data());
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/school-door-{name}.png"
                )))?;
            }
            next_frame().await;
        }
    }
    for image in &captures[1..] {
        let changed = captures[0]
            .bytes
            .chunks_exact(4)
            .zip(image.bytes.chunks_exact(4))
            .filter(|(a, b)| {
                a[..3]
                    .iter()
                    .zip(&b[..3])
                    .any(|(&a, &b)| a.abs_diff(b) > 20)
            })
            .count();
        ensure!(
            changed > 500,
            "Door swing is not visibly rendered: {changed} changed pixels"
        );
    }
    println!("PASS native upper hall doors: distinct closed and animated views");
    Ok(())
}
