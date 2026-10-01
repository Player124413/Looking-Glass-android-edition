//! The third Gnome's gestures share the recording's saved line clock.
//! Reference waits are reconstructed from local assets, never from frame counts.
use super::*;

pub const PAUSES: [f32; 6] = [1., 1., 1., 0.2, 1., 0.2];

pub(super) fn line_starts(assets: &mut Assets) -> Result<[f32; 6]> {
    let mut starts = [0.; 6];
    for (i, path) in [
        "sound/character/alice/vo/alcz1007.wav",
        "sound/character/gnome/torch/vo/gt3001.wav",
        "sound/character/alice/vo/alcz1008.wav",
        "sound/character/gnome/torch/vo/gt3002.wav",
        "sound/character/alice/vo/alcz1009.wav",
    ]
    .into_iter()
    .enumerate()
    {
        let wav = hound::WavReader::new(std::io::Cursor::new(assets.read(path)?))?;
        let duration = wav.duration() as f32 / wav.spec().sample_rate as f32;
        ensure!(
            duration.is_finite() && (0. ..=120.).contains(&duration),
            "Invalid Gnome speech length"
        );
        starts[i + 1] = starts[i] + duration + PAUSES[i];
    }
    Ok(starts)
}

pub(super) fn check(
    assets: &mut Assets,
    map: &Bsp,
    world: &World,
    hints: &crate::cheshire::Hints,
) -> Result<()> {
    const EVENT: &str = "Torchgnome3_Dialog_part2";
    for hz in [30, 60, 144] {
        for advance_intro in [false, true] {
            let mut c = Cinema::load(assets, map)?;
            let mut story = Story::load(assets, "gvillage");
            let mut player = Player::new(Vec3::ZERO);
            story.trigger(EVENT);
            let mut crouch = None;
            let mut second = None;
            let mut restored = false;
            for _ in 0..hz * 180 {
                let advance = advance_intro && story.progress(EVENT).is_some_and(|p| p.0 < 2);
                c.advance(1. / hz as f32, &mut player, world)?;
                if c.prepare_story(&mut story) {
                    story.tick(1. / hz as f32, advance);
                }
                c.sync_story(&story);
                if let Some((line, time)) = story.progress(EVENT) {
                    let pose = c.alice_pose(Beat::Gnome(2));
                    if pose.0 == "talk_gnomes_01" {
                        ensure!(line == 2, "Crouch played over the wrong speaker/line");
                        crouch.get_or_insert(time);
                    }
                    if pose.0 == "talk_gnomes_03" {
                        ensure!(line == 4, "Second size gesture played over the wrong line");
                        second.get_or_insert(time);
                    }
                    if line == 2 && time >= 5.5 && !restored {
                        let state = serde_json::to_vec(&c.state)?;
                        let dialogue = serde_json::to_vec(&story.snapshot())?;
                        c.advance(0., &mut player, world)?;
                        story.tick(0., true);
                        ensure!(
                            state == serde_json::to_vec(&c.state)?
                                && dialogue == serde_json::to_vec(&story.snapshot())?,
                            "Pause advanced the gesture or speech"
                        );
                        c.restore(Some(&serde_json::from_slice(&state)?))?;
                        story.restore(&serde_json::from_slice(&dialogue)?, hints)?;
                        c.sync_story(&story);
                        let loaded = c.alice_pose(Beat::Gnome(2));
                        ensure!(
                            pose.0 == loaded.0 && pose.1 == loaded.1 && pose.2 == loaded.2,
                            "Reload lost the exact speaking gesture"
                        );
                        // The old scene timer must not govern the restored pose.
                        let age = c.state.time;
                        c.state.time = 0.1;
                        ensure!(
                            c.alice_pose(Beat::Gnome(2)).1 == loaded.1,
                            "Gesture still follows scene age"
                        );
                        c.state.time = age;
                        restored = true;
                    }
                }
                for id in story.take_completed() {
                    c.completed(&id);
                }
                if c.state.done.contains(EVENT) {
                    break;
                }
            }
            ensure!(
                restored && c.state.done.contains(EVENT) && story.completed == 1,
                "Gnome gesture check did not complete exactly once"
            );
            ensure!(
                (crouch.context("Crouch missing")? - 4.9834).abs() <= 1. / hz as f32 + 0.003,
                "Crouch missed the phrase: {crouch:?}"
            );
            ensure!(
                (second.context("Second gesture missing")? - 3.045).abs() <= 1. / hz as f32 + 0.003,
                "Second size gesture missed its phrase: {second:?}"
            );
            println!("PASS Gnome gesture {hz} Hz advance={advance_intro}: crouch {crouch:?}, second {second:?}, pause/reload and one completion");
        }
    }
    Ok(())
}

pub async fn render(assets: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(assets, "gvillage")?;
    let village = super::super::Village::load(assets, &scene.map)?;
    let mut c = Cinema::load(assets, &scene.map)?;
    let mut art = Art::load(assets)?;
    let hints = crate::cheshire::Hints::load(assets, &scene.map, "gvillage")?;
    check(assets, &scene.map, &scene.world, &hints)?;
    std::fs::create_dir_all("private/village-gesture")?;
    for (line, times) in [
        (2, &[0.2, 4.5, 5.25, 5.85, 6.65, 7.1][..]),
        (4, &[2.5, 3.6, 4.8][..]),
    ] {
        for &time in times {
            let mut story = Story::load(assets, "gvillage");
            story.trigger(GNOMES[2]);
            story.tick(0.01, false);
            for _ in 0..line {
                story.tick(0.01, true);
            }
            for _ in 0..(time * 60.) as usize {
                story.tick(1. / 60., false);
            }
            c.state.start(Beat::Gnome(2));
            c.sync_story(&story);
            // First shot holds until Alice has finished the crouch line.
            c.state.shot_time = time;
            let camera = c.camera().unwrap();
            let transforms = village
                .objects
                .iter()
                .map(|o| {
                    let pose = super::super::sample(&o.name, o.base, time, 0.);
                    (o.model, pose.origin, pose.rotation())
                })
                .collect::<Vec<_>>();
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
            scene.draw(camera.eye, time, false, false, &transforms);
            art.story_pose(&story);
            art.draw(&c, &scene.atmosphere, camera.eye, false);
            get_screen_data()
                .export_png(&format!("private/village-gesture/line{line}-{time:.2}.png"));
            let pose = c.alice_pose(Beat::Gnome(2));
            println!("CAPTURE line {line} at {time:.2}: {} {:.3}", pose.0, pose.1);
            next_frame().await;
        }
    }
    Ok(())
}
