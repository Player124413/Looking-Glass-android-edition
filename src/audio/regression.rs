//! Regression checks using the user's mounted assets and the production mixer.
use super::*;

pub fn check(assets: &mut Assets) -> Result<()> {
    let model = events::Model::load(assets, "models/alice.tik")?;
    let def = crate::skeletal::Definition::alice(assets)?;
    for file in crate::character::CLIPS {
        let alias = crate::character::sound_alias(file);
        ensure!(
            def.animations
                .get(alias)
                .is_some_and(|f| f == &format!("{file}.ska")),
            "Incorrect audio alias for {file}: {alias}"
        );
    }
    let span = events::Span {
        start: 0.,
        end: 1.,
        duration: 1.,
        frame_time: 0.05,
        looping: false,
        entered: true,
    };
    for alias in ["run_nowep", "run_smallwep", "run_bigwep"] {
        let cues = model.between(alias, span);
        ensure!(
            cues.len() == 2 && cues.iter().all(|c| c.path.contains("/footstep")),
            "Running contains a non-footstep cue: {alias}: {cues:?}"
        );
    }
    ensure!(
        model
            .between("run_jump", span)
            .iter()
            .any(|c| c.path.ends_with("/jump2.wav")),
        "Running-jump cry was lost"
    );
    for alias in [
        "death_falling1",
        "death_drown1",
        "climb_48",
        "swim_forward_frog",
    ] {
        ensure!(
            !model.between(alias, span).is_empty(),
            "Alias {alias} lost its audio"
        );
    }
    let map = Bsp::parse(&assets.read("maps/gvillage.bsp")?)?;
    let spec = level_spec(assets, "gvillage", &map)?;
    let portals: Vec<_> = spec
        .speakers
        .iter()
        .filter(|s| s.path.ends_with("/ambience_forcefield.wav"))
        .collect();
    ensure!(
        portals.len() == 1 && portals[0].origin.distance(vec3(-4150.15, 6147.33, 311.70)) < 0.1,
        "Village portal is duplicated or lost its sound-manager position"
    );
    ensure!(
        spec.speakers.len() == 20,
        "Unrelated village ambience changed"
    );
    println!("PASS original running/jump aliases, swimming/climbing/death cues, one village portal and 19 other ambient sources");
    Ok(())
}

/// Deliberately stages hazardous-water feedback without changing saves/settings.
/// Captures only this application's output stream, never a microphone/loopback.
pub fn output(assets: &mut Assets) -> Result<()> {
    check(assets)?;
    let map = Bsp::parse(&assets.read("maps/gvillage.bsp")?)?;
    let world = World::from_bsp(&map)?;
    let model = events::Model::load(assets, "models/alice.tik")?;
    let eye = vec3(-4144., 6120., 280.);
    let mut audio = Audio::new(false, Some("private/audio-regression-output.wav".into()));
    ensure!(audio.playback.is_some(), "No usable Windows output device");
    audio.settings = Settings::default();
    // Start with isolated player feedback so silence/pause and repetition are measurable.
    audio.player_state(assets, 100., false, false, false);
    let mut sanity = 100.;
    let mut marks = vec![];
    let mut starts = vec![];
    let mut max_ordinary = 0;
    for tick in 0..360 {
        let label = match tick {
            0 => Some("continuous-liquid"),
            80 => Some("pause"),
            100 => Some("drowning"),
            140 => Some("surface"),
            180 => Some("death"),
            230 => Some("village-entrance"),
            300 => Some("running"),
            _ => None,
        };
        if let Some(label) = label {
            marks.push((
                label,
                audio.monitor.count.load(Ordering::Relaxed) as usize / 2,
            ));
        }
        let paused = (80..100).contains(&tick);
        if tick < 80 || (100..140).contains(&tick) {
            sanity -= 0.2;
            // Reproduce both feedback producers, including the old animation restart flood.
            audio.animation(events::Cue {
                path: "sound/character/alice/pain1.wav".into(),
                volume: 0.7,
            });
            if tick >= 100 && tick % 10 == 0 {
                audio.world_effect("sound/character/alice/choke1.wav");
            }
            audio.player_state(assets, sanity, false, true, tick >= 100);
        }
        if tick == 140 {
            audio.water(0.05, 3, 2, 0.);
            audio.player_state(assets, sanity, false, false, false);
        }
        if tick == 180 {
            audio.world_effect("sound/character/alice/death_faint.wav");
            audio.player_state(assets, 0., false, false, false);
        }
        if tick == 230 {
            audio.load(assets, "gvillage", &map);
            ensure!(
                audio.reaction.is_none(),
                "Map load retained an old reaction"
            );
            audio.music = None;
        }
        if tick == 300 {
            audio.emitters.clear();
        }
        if tick >= 300 {
            let time = (tick - 300) as f32 * 0.05;
            for cue in model.between(
                "run_nowep",
                events::Span {
                    start: time % 0.6,
                    end: time % 0.6 + 0.05,
                    duration: 0.6,
                    frame_time: 0.05,
                    looping: true,
                    entered: tick == 300,
                },
            ) {
                audio.animation(cue);
            }
        }
        let before = audio.resolved;
        audio.prepare(
            assets,
            &world,
            eye,
            tick < 140,
            if paused { 0. } else { 0.05 },
        );
        if audio.resolved != before && tick < 230 {
            starts.push((tick, audio.resolved - before));
        }
        max_ordinary = max_ordinary.max(if tick < 230 { audio.effects.len() } else { 0 });
        audio.update(0.05, eye, 0., paused, false);
        std::thread::sleep(Duration::from_millis(50));
    }
    marks.push((
        "end",
        audio.monitor.count.load(Ordering::Relaxed) as usize / 2,
    ));
    ensure!(
        max_ordinary == 0,
        "Player reactions escaped their exclusive voice: {max_ordinary}"
    );
    let pain: Vec<_> = starts.iter().filter(|(tick, _)| *tick < 80).collect();
    ensure!(
        (3..=4).contains(&pain.len()) && pain.windows(2).all(|p| p[1].0 - p[0].0 >= 24),
        "Liquid voices restarted too rapidly: {pain:?}"
    );
    ensure!(
        starts.iter().all(|(_, count)| *count == 1),
        "Simultaneous reactions: {starts:?}"
    );
    let choking: Vec<_> = starts
        .iter()
        .filter(|(tick, _)| (100..140).contains(tick))
        .collect();
    ensure!(
        choking.len() == 2 && choking[1].0 - choking[0].0 >= 24,
        "Drowning interleaved a pain cry: {choking:?}"
    );
    ensure!(
        audio.missing.is_empty(),
        "Missing requested audio: {:?}",
        audio.missing
    );
    audio.finish()?;
    let samples = audio.monitor.samples.lock().unwrap();
    let mut report = vec![];
    for pair in marks.windows(2) {
        let begin = (pair[0].1 + 11025).min(samples.len() / 2);
        let end = pair[1].1.saturating_sub(2205).min(samples.len() / 2);
        ensure!(end > begin, "Empty output interval");
        let slice = &samples[begin * 2..end * 2];
        let rms = (slice.iter().map(|s| s * s).sum::<f32>() / slice.len() as f32).sqrt();
        let peak = slice.iter().fold(0f32, |p, s| p.max(s.abs()));
        ensure!(
            if pair[0].0 == "pause" {
                peak < 0.00001
            } else {
                rms > 0.00001
            },
            "Unexpected {} playback: rms={rms} peak={peak}",
            pair[0].0
        );
        ensure!(peak < 0.99, "Clipped {} mix", pair[0].0);
        report.push(serde_json::json!({"phase":pair[0].0,"rms":rms,"peak":peak}));
    }
    std::fs::write(
        "private/audio-regression-mixing.json",
        serde_json::to_string_pretty(
            &serde_json::json!({"reaction_starts_at_20hz":starts,"intervals":report}),
        )?,
    )?;
    println!("PASS liquid/choking voice separation, pause, surfacing, death, map reset, village ambience and running playback: {} intervals, {} resolved sounds", report.len(), audio.resolved);
    Ok(())
}
