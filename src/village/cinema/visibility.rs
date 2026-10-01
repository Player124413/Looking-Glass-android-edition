//! Accelerated replay through the actual choreography, story and actor renderer.
//! No player saves/settings are read or written. Every run retains shared GPU
//! resources across scenes to catch bugs hidden by restarting the executable.
use super::*;
use crate::character::{visibility_camera, visibility_image, visible_pixels, Character};

pub async fn check(assets: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(assets, "gvillage")?;
    let village = super::super::Village::load(assets, &scene.map)?;
    let hints = crate::cheshire::Hints::load(assets, &scene.map, "gvillage")?;
    let mut art = Art::load(assets)?;
    let mut alice = Character::load(assets)?;
    let spawn = crate::interaction::spawn(&scene.map, None).0;
    let mut report = Vec::new();
    for event in ["entry"].into_iter().chain(GNOMES).chain(["knife_cat"]) {
        for skip_at in [None, Some(2.5), Some(12.)] {
            let mut c = Cinema::load(assets, &scene.map)?;
            let mut story = Story::load(assets, "gvillage");
            let mut player = Player::spawn(&scene.world, spawn).context("Visibility test spawn")?;
            if event == "entry" {
                c.begin(&mut story);
            } else if event == "knife_cat" {
                c.state.start(Beat::Knife);
                story.trigger(event);
            } else {
                story.trigger(event);
                story.tick(1. / 60., false);
                c.sync_story(&story);
            }
            let mut samples = 0;
            let mut minimum = usize::MAX;
            let mut restored = false;
            let mut last_beat = None;
            let mut last_camera = None;
            let mut last_fade = (BLACK, 0.);
            for tick in 0..18000 {
                if let Some(camera) = c.camera() {
                    last_camera = Some(camera);
                    last_fade = c.fade();
                }
                let seconds = tick as f32 / 60.;
                if c.active() && skip_at.is_some_and(|at| seconds >= at) {
                    c.skip(&mut player, &scene.world, &mut story)?;
                }
                c.advance(1. / 60., &mut player, &scene.world)?;
                if c.prepare_story(&mut story) {
                    story.tick(1. / 60., false);
                }
                c.sync_story(&story);
                for id in story.take_completed() {
                    c.completed(&id);
                }
                if seconds >= 1. && !restored {
                    let saved = serde_json::to_vec(&c.state)?;
                    let dialogue = story.snapshot();
                    c.restore(Some(&serde_json::from_slice(&saved)?))?;
                    story.restore(&dialogue, &hints)?;
                    c.advance(0., &mut player, &scene.world)?;
                    ensure!(
                        serde_json::to_vec(&c.state)? == saved,
                        "Paused/restored scene changed: {event}"
                    );
                    restored = true;
                }
                let beat = c.state.beat;
                if let Some(beat) = beat.filter(|_| tick % 30 == 0 || beat != last_beat) {
                    let camera = c.camera().context("Active scene has no camera")?;
                    clear_background(BLACK);
                    set_camera(&Camera3D {
                        position: camera.eye,
                        target: camera.target,
                        up: camera.up,
                        fovy: 75_f32.to_radians(),
                        z_near: 2.,
                        z_far: 30000.,
                        ..Default::default()
                    });
                    let transforms = village
                        .objects
                        .iter()
                        .map(|o| {
                            let mut p = super::super::sample(&o.name, o.base, seconds, 0.);
                            if o.name == "shrink_door1" {
                                p.angles.z = c.door_angle();
                            }
                            (o.model, p.origin, p.rotation())
                        })
                        .collect::<Vec<_>>();
                    scene.draw(camera.eye, seconds, false, false, &transforms);
                    art.story_pose(&story);
                    if beat == Beat::Gnome(2) && [0, 2, 4, 7].contains(&c.state.line) {
                        // Alice speaks in these shots. Check the authored view,
                        // not only the fitted diagnostic view below.
                        let before = get_screen_data();
                        art.material.atmosphere(&scene.atmosphere, camera.eye);
                        let (clip, time, looping, pose, scale) = c.alice_pose(beat);
                        art.alice.draw(clip, time, looping, pose, scale, false);
                        let pixels = visible_pixels(&before, &visibility_image());
                        ensure!(pixels > 100, "Alice offscreen while speaking in {event}, line {}, shot {:.3}s: {pixels} pixels", c.state.line, c.state.shot_time);
                    }
                    art.draw(&c, &scene.atmosphere, camera.eye, false);
                    // Flush an actual shot, then measure Alice alone at its actual
                    // pose. A fitted diagnostic camera excludes intentional cuts
                    // away from Alice and scenery occlusion from the assertion.
                    let _ = get_screen_data();
                    next_frame().await;
                    let (clip, time, looping, pose, scale) = c.alice_pose(beat);
                    ensure!(
                        scale > 0. && pose.translation.is_finite(),
                        "Invalid Alice pose: {event} {beat:?}"
                    );
                    visibility_camera(pose.translation, scale);
                    art.material
                        .atmosphere(&crate::environment::Atmosphere::default(), camera.eye);
                    let before = get_screen_data();
                    art.alice.draw(clip, time, looping, pose, scale, true);
                    let after = visibility_image();
                    let pixels = visible_pixels(&before, &after);
                    if pixels < 100 {
                        std::fs::create_dir_all("private/visibility")?;
                        crate::viewer::save_capture(std::path::Path::new(
                            "private/visibility/failure.png",
                        ))?;
                    }
                    ensure!(
                        pixels >= 100,
                        "Alice missing in {event}, {beat:?}, {time:.3}s: {pixels} pixels"
                    );
                    minimum = minimum.min(pixels);
                    samples += 1;
                    next_frame().await;
                }
                last_beat = beat;
                if !c.active() && !story.busy() {
                    break;
                }
            }
            ensure!(
                !c.active() && !story.busy() && c.camera().is_none() && player.script_motion == 0,
                "Scene did not return camera/control: {event}"
            );
            ensure!(
                c.state.done.contains(event) && scene.world.body_clear(player.feet),
                "Invalid scene endpoint: {event}"
            );
            // Player body after the final shot, then after save/restore, using
            // the same live pipeline and the same state produced by the scene.
            alice.resume_scene(&player, player.script_facing, art.alice.handoff_pose());
            for reload in [false, true] {
                if reload {
                    let snapshot = serde_json::to_vec(&alice.snapshot())?;
                    alice.restore(&serde_json::from_slice(&snapshot)?)?;
                }
                visibility_camera(player.feet, 1.);
                alice.atmosphere(&crate::environment::Atmosphere::default(), player.eye());
                let before = get_screen_data();
                alice.draw_ghost(player.feet, true, false);
                let pixels = visible_pixels(&before, &visibility_image());
                ensure!(
                    pixels >= 100,
                    "Gameplay Alice missing after {event}, skip={skip_at:?}, reload={reload}"
                );
                next_frame().await;
            }
            if skip_at.is_none() {
                return_frames(
                    &mut scene,
                    &mut alice,
                    &player,
                    last_camera.context("No final shot")?,
                    Color {
                        a: last_fade.1,
                        ..last_fade.0
                    },
                    event,
                )
                .await?;
            }
            report.push(serde_json::json!({"scene":event,"skip_at":skip_at,"samples":samples,"minimum_alice_pixels":minimum,"restored":restored,"handoff_visible":true}));
            println!("PASS visibility {event}, skip={skip_at:?}: {samples} poses, at least {minimum} Alice pixels; watched/skipped handoff and reload visible");
        }
    }
    gl_use_default_material();
    set_default_camera();
    std::fs::create_dir_all("private/visibility")?;
    std::fs::write(
        "private/visibility/village-report.json",
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(())
}

/// Render the same collision-safe camera return and gameplay pose update as the
/// viewer, using the actual endpoint produced by a watched scene.
async fn return_frames(
    scene: &mut crate::render::Scene,
    alice: &mut Character,
    player: &Player,
    from: Camera,
    fade: Color,
    event: &str,
) -> Result<()> {
    let mut follow = crate::camera::Follow::default();
    let mut handoff = crate::camera::Handoff::default();
    let direction = vec3(player.script_facing.cos(), player.script_facing.sin(), 0.);
    let (eye, target, _) = follow.update(&scene.world, player.feet, direction, 132., 0.);
    let gameplay = Camera::look(eye, target);
    handoff.update(&scene.world, Some(from), gameplay, fade, 0.);
    for frame in 0..36 {
        let view = handoff.update(&scene.world, None, gameplay, fade, 1. / 60.);
        let cover = handoff.overlay();
        ensure!(
            view.eye.is_finite() && (view.target - view.eye).length() > 0.9,
            "Invalid camera during {event} handoff"
        );
        if cover.a < 0.1 {
            ensure!(
                !scene
                    .world
                    .sweep(view.eye, view.eye, Vec3::splat(3.9))
                    .start_solid,
                "Visible handoff camera inside geometry: {event}, frame {frame}"
            );
        }
        alice.update(
            1. / 60.,
            player,
            false,
            crate::character::WeaponInput {
                dice: 1,
                first_person: false,
                selected: crate::weapons::UNARMED,
                click: None,
                aim: direction,
            },
            &crate::combat::Context {
                world: &scene.world,
                targets: &[],
            },
        );
        clear_background(BLACK);
        let camera = Camera3D {
            position: view.eye,
            target: view.target,
            up: Vec3::Z,
            fovy: 75_f32.to_radians(),
            z_near: 2.,
            z_far: 30000.,
            ..Default::default()
        };
        set_camera(&camera);
        crate::render_fx::begin_view(&camera, 0., &scene.atmosphere, false);
        scene.draw(view.eye, 0., false, false, &[]);
        alice.atmosphere(&scene.atmosphere, view.eye);
        alice.draw(player.feet, false);
        crate::render_fx::finish();
        set_default_camera();
        if cover.a > 0. {
            draw_rectangle(0., 0., screen_width(), screen_height(), cover);
        }
        if [0, 12, 35].contains(&frame) {
            get_screen_data().export_png(&format!("private/visibility/{event}-return-{frame}.png"));
        }
        next_frame().await;
    }
    ensure!(
        !handoff.was_scripted() && handoff.overlay().a == 0.,
        "Handoff stayed active: {event}"
    );
    println!("PASS {event}: 36 rendered handoff frames, clear camera and final position retained");
    Ok(())
}
