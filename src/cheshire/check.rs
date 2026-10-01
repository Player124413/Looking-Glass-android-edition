//! C4 asset, owner, save and presentation acceptance checks.
use super::*;
use crate::{
    interaction::Interactions,
    movement::{Controls, Player},
};
use anyhow::ensure;

pub const CASES: &[&str] = &[
    "cheshire-pool-before-47",
    "cheshire-pool-before-728",
    "cheshire-pool-after-46",
    "cheshire-pool-fade-out",
    "cheshire-pool-cooldown",
    "cheshire-pool-swimming",
];

fn center(map: &Bsp, entity: usize) -> Vec3 {
    let e = &map.entities[entity];
    let model = &map.models[e["model"][1..].parse::<usize>().unwrap()];
    crate::interaction::vector(&e["origin"]).unwrap() + (model.min + model.max) * 0.5
        - PLAYER_CENTER
}
fn step(h: &mut Hints, s: &mut Story, dt: f32, advance: bool) -> bool {
    if h.prepare_story(s) {
        s.tick(dt, advance);
    }
    h.update(dt, s.hint_active())
}
fn restored(
    assets: &mut Assets,
    map: &Bsp,
    i: &Interactions,
    h: &Hints,
    s: &Story,
) -> Result<(Hints, Story)> {
    let mut hints = Hints::load(assets, map, "potears1")?;
    hints.restore(&serde_json::from_value(serde_json::to_value(
        h.snapshot(),
    )?)?)?;
    hints.sync(i);
    let mut story = Story::load(assets, "potears1");
    story.restore(
        &serde_json::from_value(serde_json::to_value(s.snapshot())?)?,
        &hints,
    )?;
    Ok((hints, story))
}

fn pool(assets: &mut Assets, map: &Bsp) -> Result<()> {
    let mut i = Interactions::load(map)?;
    i.set_entry(assets, map, "potears1", None)?;
    let mut world = World::from_bsp(map)?;
    i.sync(&mut world);
    let mut h = Hints::load(assets, map, "potears1")?;
    h.sync(&i);
    ensure!(
        h.regions.iter().map(|r| r.entity).collect::<Vec<_>>()
            == [43, 44, 45, 729, 730, 46, 47, 728],
        "Legacy Pool region indices moved"
    );
    for index in 0..8 {
        // Same index-only schema as pre-C4 format-12 saves, including duplicate voice paths.
        let saved: Snapshot = serde_json::from_value(serde_json::json!({
            "selected":index,"next":17,"cooldown":0.,"appearance":null
        }))?;
        i.pool.as_mut().unwrap().state.talking = index == 5;
        h.restore(&saved)?;
        h.sync(&i);
        ensure!(
            h.selected == Some(index) && h.next == 17,
            "Legacy hint selection lost"
        );
    }
    for entity in [47, 728, 46] {
        let after = entity == 46;
        i.pool.as_mut().unwrap().state.talking = after;
        h.sync(&i);
        h.selected = None;
        h.observe(center(map, entity), center(map, entity));
        ensure!(
            h.selected.is_some_and(|n| h.regions[n].entity == entity),
            "Wrong region #{entity}"
        );
        let path = h.regions[h.selected.unwrap()].line.path.clone();
        let before = serde_json::to_value(i.snapshot())?;
        for hz in [30, 60, 144] {
            for advance in [false, true] {
                h.appearance = None;
                h.cooldown = 0.;
                let mut story = Story::load(assets, "potears1");
                let baseline = serde_json::to_value(story.snapshot())?;
                for repeat in 0..2 {
                    h.summon(&mut story, &world, center(map, entity), 0., true)
                        .map_err(anyhow::Error::msg)?;
                    ensure!(
                        h.summon(&mut story, &world, center(map, entity), 0., true)
                            .is_err(),
                        "Double summon"
                    );
                    for frame in 0..hz * 160 {
                        let dt = 1. / hz as f32;
                        if frame == 2 {
                            let pause = serde_json::to_value(h.snapshot())?;
                            let dialogue = serde_json::to_value(story.snapshot())?;
                            step(&mut h, &mut story, 0., true);
                            ensure!(
                                pause == serde_json::to_value(h.snapshot())?
                                    && dialogue == serde_json::to_value(story.snapshot())?,
                                "Hint advanced while paused"
                            );
                            (h, story) = restored(assets, map, &i, &h, &story)?;
                        }
                        step(&mut h, &mut story, dt, advance && frame > hz);
                        if let Some(line) = story.line() {
                            ensure!(line.path == path, "Wrong saved voice");
                        }
                        ensure!(story.take_completed().is_empty(), "Hint completed a quest");
                        if !story.busy() && h.appearance.is_none() && h.cooldown == 0. {
                            break;
                        }
                    }
                    ensure!(
                        !story.busy() && h.appearance.is_none() && h.cooldown == 0.,
                        "Hint never finished"
                    );
                    ensure!(
                        serde_json::to_value(story.snapshot())? == baseline,
                        "Repeat {repeat} changed story history"
                    );
                }
            }
        }
        ensure!(
            before == serde_json::to_value(i.snapshot())?,
            "Hint changed Pool or encounters"
        );
        i.pool.as_mut().unwrap().state.talking = !after;
        h.sync(&i);
        ensure!(
            h.selected.is_none(),
            "Disabled named hint remained selected"
        );
        println!("PASS Pool #{entity}: legacy selection, watch/advance/reload/pause, two summons, no quest commits at 30/60/144Hz");
    }
    // An old selected brush may become disabled while its saved voice is still playing.
    i.pool.as_mut().unwrap().state.talking = false;
    h.sync(&i);
    h.appearance = None;
    h.cooldown = 0.;
    let mut active = Story::load(assets, "potears1");
    h.summon(&mut active, &world, center(map, 47), 0., true)
        .map_err(anyhow::Error::msg)?;
    step(&mut h, &mut active, 0.1, false);
    let path = active.line().unwrap().path.clone();
    i.pool.as_mut().unwrap().state.talking = true;
    let (migrated, active) = restored(assets, map, &i, &h, &active)?;
    ensure!(
        migrated.selected.is_none() && active.line().is_some_and(|l| l.path == path),
        "Disabled region migration lost an active recording"
    );
    // The real owner swaps the regions on entry into TALK, before dialogue completes.
    let mut player = Player::spawn(&world, crate::interaction::spawn(map, None).0).unwrap();
    let mut story = Story::load(assets, "potears1");
    i.pool.as_mut().unwrap().state.talking = false;
    for event in i
        .pool
        .as_mut()
        .unwrap()
        .event(crate::pool::TALK)
        .unwrap()
        .story
    {
        story.trigger(&event);
    }
    h.sync(&i);
    ensure!(
        !h.allowed && h.regions[5].enabled && !h.regions[6].enabled && !h.regions[7].enabled,
        "Turtle entry failed to swap/lock hints"
    );
    ensure!(
        h.summon(&mut story, &world, player.feet, 0., false)
            .is_err(),
        "Summon entered Turtle scene"
    );
    i.skip_cinematic(map, &mut world, &mut player, &mut story)?;
    h.sync(&i);
    ensure!(
        h.allowed && i.pool.as_ref().unwrap().state.talked,
        "Turtle skip did not reopen hints"
    );
    for case in CASES {
        let (hints, _) = stage(case, assets, map, &mut i, &mut world, &mut player)?;
        ensure!(world.body_clear(player.feet), "Staged hint player embedded");
        if case.ends_with("swimming") {
            ensure!(!hints.visible(), "Swimming hint placed a body");
        }
    }
    println!("PASS Pool owner entry/skip windows and six native save fixture placements");
    Ok(())
}

pub fn stage(
    case: &str,
    assets: &mut Assets,
    map: &Bsp,
    i: &mut Interactions,
    world: &mut World,
    player: &mut Player,
) -> Result<(Hints, Story)> {
    *i = Interactions::load(map)?;
    i.set_entry(assets, map, "potears1", None)?;
    i.sync(world);
    let mut story = Story::load(assets, "potears1");
    let after = case == "cheshire-pool-after-46";
    if after {
        for event in i
            .pool
            .as_mut()
            .unwrap()
            .event(crate::pool::TALK)
            .unwrap()
            .story
        {
            story.trigger(&event);
        }
        i.skip_cinematic(map, world, player, &mut story)?;
    }
    let entity = if after {
        46
    } else if case.ends_with("728") {
        728
    } else {
        47
    };
    let at = center(map, entity);
    let mut found = None;
    // Find a supported, clear point in the actual brush for native save/input tests.
    'search: for x in [0., -64., 64., -128., 128.] {
        for y in [0., -64., 64., -128., 128.] {
            let Some(mut p) = Player::spawn(world, at + vec3(x, y, 64.)) else {
                continue;
            };
            for _ in 0..120 {
                p.tick(world, Controls::default());
            }
            let mut hints = Hints::load(assets, map, "potears1")?;
            hints.sync(i);
            hints.observe(p.feet, p.feet);
            if hints
                .selected
                .is_some_and(|n| hints.regions[n].entity == entity)
                && p.grounded
            {
                for yaw in (0..4).map(|n| n as f32 * std::f32::consts::FRAC_PI_2) {
                    if place(world, p.feet, yaw).is_some() {
                        found = Some((p, hints, yaw));
                        break 'search;
                    }
                }
            }
        }
    }
    let (p, mut hints, yaw) = found.context("No clear Pool hint fixture placement")?;
    *player = p;
    player.script_facing = yaw;
    hints
        .summon(
            &mut story,
            world,
            player.feet,
            yaw,
            case.ends_with("swimming"),
        )
        .map_err(anyhow::Error::msg)?;
    let time = if case.ends_with("fade-out") {
        4.75
    } else if case.ends_with("cooldown") {
        8.
    } else if case.ends_with("47") {
        1.
    } else {
        3.5
    };
    for frame in 0..(time * 60.) as usize {
        step(
            &mut hints,
            &mut story,
            1. / 60.,
            frame >= 180 && (case.ends_with("fade-out") || case.ends_with("cooldown")),
        );
    }
    Ok((hints, story))
}

pub fn behavior(r: &mut crate::save::Restored) -> Result<()> {
    let owner = serde_json::to_value(r.interactions.snapshot())?;
    let stats = serde_json::to_value(&r.game.stats)?;
    let completed = r.story.completed;
    for _ in 0..1200 {
        step(&mut r.hints, &mut r.story, 1. / 60., true);
    }
    r.hints.sync(&r.interactions);
    for _ in 0..2 {
        r.hints
            .summon(&mut r.story, &r.scene.world, r.game.player.feet, 0., true)
            .map_err(anyhow::Error::msg)?;
        for _ in 0..1200 {
            step(&mut r.hints, &mut r.story, 1. / 60., true);
        }
    }
    ensure!(
        r.story.completed == completed
            && r.story.take_completed().is_empty()
            && r.story.take_exit().is_none()
            && owner == serde_json::to_value(r.interactions.snapshot())?
            && stats == serde_json::to_value(&r.game.stats)?,
        "Restored hints changed quest/rewards/exit"
    );
    Ok(())
}

pub async fn render(assets: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(assets, "potears1")?;
    let mut i = Interactions::load(&scene.map)?;
    let mut player = Player::new(Vec3::ZERO);
    let mut art = Art::load(assets)?;
    i.set_entry(assets, &scene.map, "potears1", None)?;
    let mut pool_art = crate::pool::Art::load(assets, i.pool.as_ref().unwrap())?;
    for case in CASES {
        let (hints, story) = stage(
            case,
            assets,
            &scene.map,
            &mut i,
            &mut scene.world,
            &mut player,
        )?;
        let camera = player.eye();
        let aim = vec3(
            player.script_facing.cos(),
            player.script_facing.sin(),
            -0.14,
        )
        .normalize();
        art.story_pose(&story);
        for frame in 0..3 {
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: camera,
                target: camera + aim,
                up: Vec3::Z,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            });
            let poses = i.transforms();
            scene.draw(camera, 0., false, false, &poses);
            pool_art.draw(i.pool.as_ref().unwrap(), false, &scene.atmosphere, camera);
            art.draw(&hints, &scene.atmosphere, camera, false);
            crate::render::depth_read_only(|| scene.draw(camera, 0., false, true, &poses));
            set_default_camera();
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/cheshire-bindings/{case}.png"
                )))?;
            }
            next_frame().await;
        }
        println!("PASS rendered {case}");
    }
    Ok(())
}
pub fn assets(assets: &mut Assets) -> Result<()> {
    use rodio::Source;
    let mut files = BTreeSet::new();
    let mut count = 0;
    for name in assets.maps() {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let mut h = Hints::load(assets, &map, &name).with_context(|| format!("Cheshire {name}"))?;
        count += h.regions.len();
        for line in h.fallback.iter().chain(h.regions.iter().map(|r| &r.line)) {
            files.insert(line.path.clone());
        }
        if name == "potears1" {
            pool(assets, &map)?;
        }
        if name == "skool2" {
            let world = World::from_bsp(&map)?;
            for e in &map.entities {
                if e.get("classname").map(String::as_str) != Some("trigger_catmessage") {
                    continue;
                }
                let at = e
                    .get("origin")
                    .and_then(|s| crate::interaction::vector(s))
                    .context("Hint origin missing")?
                    - PLAYER_CENTER;
                h.observe(at, at);
                let selected = h
                    .selected
                    .context("School hint region did not select a hint")?;
                anyhow::ensure!(
                    Some(&h.regions[selected].line.path) == e.get("target"),
                    "Wrong contextual school hint"
                );
                // Replaying a hint uses its own dialogue ID, never a cinematic/quest event.
                let mut story = Story::default();
                h.appearance = None;
                h.cooldown = 0.;
                h.summon(&mut story, &world, at, 0., true)
                    .map_err(anyhow::Error::msg)?;
                story.tick(0.1, false);
                anyhow::ensure!(
                    story
                        .line()
                        .is_some_and(|l| &l.path == e.get("target").unwrap()),
                    "Wrong summoned line"
                );
                story.tick(0.1, true);
                anyhow::ensure!(
                    story.take_completed().is_empty(),
                    "Hint emitted quest completion"
                );
            }
        }
    }
    files.insert("sound/character/cheshire_cat/appear.wav".into());
    files.insert("sound/character/cheshire_cat/disappear.wav".into());
    for path in &files {
        let decoder = rodio::Decoder::new(std::io::Cursor::new(assets.read(path)?))?;
        let mut peak = 0f32;
        for sample in decoder.convert_samples::<f32>() {
            anyhow::ensure!(sample.is_finite(), "Nonfinite hint sample: {path}");
            peak = peak.max(sample.abs());
        }
        anyhow::ensure!(peak > 0.001, "Silent hint {path}");
    }
    println!("PASS Cheshire: {count} authored regions across all maps; {} unique voice/effect recordings decoded",files.len());
    Ok(())
}
