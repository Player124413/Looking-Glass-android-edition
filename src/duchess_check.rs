use crate::{
    duchess::{Action, Stage},
    movement::Controls,
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
pub fn check(assets: &mut crate::assets::Assets) -> Result<()> {
    crate::duchess::check_intro(assets)?;
    if std::env::var_os("LOOKING_GLASS_DUCHESS_INTRO_ONLY").is_some() {
        return Ok(());
    }
    contracts(assets)?;
    let mut r = crate::route::Route::new(assets, "potears3", None)?;
    drive(assets, &mut r)
}

pub(crate) fn drive(assets: &mut crate::assets::Assets, r: &mut crate::route::Route) -> Result<()> {
    r.tactics = true;
    println!("Duchess normal entrance {:?}", r.player.feet);
    for goal in [
        vec3(-832., -1000., 16.),
        vec3(-536., -520., 16.),
        vec3(-280., -264., 16.),
    ] {
        println!("Navigate to {goal:?}");
        r.navigate(goal)?;
    }
    for t in 0..2400 {
        if r.stats.collected.contains("potears3:21") {
            break;
        }
        let wish = (vec2(-114., 66.) - r.player.feet.truncate()).normalize_or_zero();
        r.tick(Controls {
            wish,
            run: false,
            jump: t % 120 == 0,
            ..Default::default()
        })?;
    }
    println!(
        "Pickup approach {:?} collected={:?}",
        r.player.feet, r.stats.collected
    );
    for _ in 0..3000 {
        r.tick(Controls::default())?;
        if r.interactions.duchess.as_ref().unwrap().state.stage == Stage::Fighting {
            break;
        }
    }
    ensure!(
        r.interactions.duchess.as_ref().unwrap().state.stage == Stage::Fighting,
        "Pickup failed to start Duchess encounter"
    );
    ensure!(r.stats.copies(3) == 1, "Jackbomb pickup missing");
    println!("Fighting from {:?}", r.player.feet);
    let mut last_feet = r.player.feet;
    for t in 0..24000 {
        let d = r.interactions.duchess.as_ref().unwrap();
        if d.state.stage != Stage::Fighting {
            break;
        }

        let radial = (r.player.feet - vec3(0., 300., 0.))
            .truncate()
            .normalize_or_zero();
        let wish = (vec2(-radial.y, radial.x)
            + radial
                * if (r.player.feet - vec3(0., 300., 0.)).truncate().length() < 400. {
                    0.8
                } else {
                    -0.2
                })
        .normalize_or_zero();
        // The expanded kitchen has low furniture and uneven floor pieces.
        // Step over an obstruction instead of circling motionless into it.
        let jump = t % 120 == 119 && r.player.feet.distance(last_feet) < 16.;
        if t % 120 == 119 {
            last_feet = r.player.feet;
        }
        r.tick(Controls {
            wish,
            run: true,
            jump,
            ..Default::default()
        })?;
        if t % 1200 == 0 {
            let d = r.interactions.duchess.as_ref().unwrap();
            println!(
                "fight health={} sanity={} feet={:?} boss={:?} {:?}",
                d.state.health,
                r.stats.sanity(),
                r.player.feet,
                d.state.feet,
                d.state.action
            );
        }
    }
    ensure!(
        r.interactions.duchess.as_ref().unwrap().state.health == 0.,
        "Boss did not die: {} throws, {} swings, {} cards, {} dodges",
        r.shots,
        r.swings,
        r.cards,
        r.interactions.duchess.as_ref().unwrap().state.dodges
    );
    for _ in 0..8000 {
        r.tick(Controls::default())?;
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition.as_ref().is_some_and(|e| e.0 == "utemple"),
        "Missing gated exit"
    );
    let d = r.interactions.duchess.as_ref().unwrap();
    ensure!(d.state.shell_returned, "Shell not returned");
    println!("Duchess route: {} ticks, {} throws, {} swings, {} cards, {} damage, {} sanity, {} attacks, {} dodges, shell returned, utemple exit",r.ticks,r.shots,r.swings,r.cards,r.damage,r.stats.sanity(),d.state.attacks,d.state.dodges);
    // Restart reconstruction rejects impossible reward/death combinations.
    let state = d.snapshot();
    let mut restored = crate::duchess::Duchess::load(assets, &r.map)?;
    restored.restore(&state, &r.map)?;
    let mut invalid = state;
    invalid.health = 30.;
    ensure!(
        restored.restore(&invalid, &r.map).is_err(),
        "Accepted living defeated boss"
    );
    ensure!(d.state.action == Action::Death, "Missing death animation");
    Ok(())
}

/// Deliberately staged art review; the separate headless check proves the normal route.
pub async fn render(assets: &mut crate::assets::Assets) -> Result<()> {
    crate::duchess::render_intro(assets).await?;
    if std::env::var_os("LOOKING_GLASS_DUCHESS_INTRO_ONLY").is_some() {
        return Ok(());
    }
    let mut player_art = crate::character::Character::load(assets)?;
    let mut scene = crate::render::Scene::load(assets, "potears3")?;
    let mut i = crate::interaction::Interactions::load(&scene.map)?;
    i.set_entry(assets, &scene.map, "potears3", None)?;
    let mut art = crate::duchess::Art::load(assets, &scene.map)?;
    let mut stats = crate::inventory::Stats::for_level("potears3", None);
    let mut story = crate::story::Story::load(assets, "potears3");
    let mut p = crate::movement::Player::new(Vec3::ZERO);
    for case in [
        "duchess-expand",
        "duchess-intro",
        "duchess-pepper",
        "duchess-pig",
        "duchess-phase",
        "duchess-bite",
        "duchess-death",
        "duchess-sneeze-1",
        "duchess-sneeze-2",
        "duchess-sneeze-3",
        "duchess-blood",
        "duchess-smoke",
        "duchess-rescue",
        "duchess-reward",
        "duchess-complete",
    ] {
        i.duchess = Some(crate::duchess::Duchess::load(assets, &scene.map)?);
        let d = i.duchess.as_mut().unwrap();
        d.fixture(
            if case.starts_with("duchess-sneeze")
                || matches!(case, "duchess-blood" | "duchess-smoke")
            {
                "duchess-death"
            } else {
                case
            },
            &scene.map,
            &mut scene.world,
            &mut p,
            &mut stats,
            &mut story,
        )?;
        if let Some(frame) = match case {
            "duchess-sneeze-1" => Some(35.),
            "duchess-sneeze-2" => Some(66.),
            "duchess-sneeze-3" => Some(90.),
            "duchess-blood" => Some(112.),
            "duchess-smoke" => Some(148.),
            _ => None,
        } {
            d.state.clock = frame * d.timing.clips["death"].1;
            d.state.time = d.state.clock;
        }
        let frames = if case == "duchess-phase" { 52 } else { 4 };
        let base = d.state.feet;
        let mut without = None;
        let mut with = None;
        for frame in 0..frames {
            if case == "duchess-phase" {
                let time = frame.min(48) as f32 / 60.;
                d.state.clock = time;
                d.state.time = time;
                d.state.feet = base + Vec3::X * time * 340.;
            }
            let c = d
                .camera(&scene.world)
                .unwrap_or(crate::cinematic::Camera::look(
                    d.state.feet + vec3(185., -260., 140.),
                    d.state.feet + Vec3::Z * 64.,
                ));
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
            crate::render_fx::begin_view(&camera, 2., &scene.atmosphere, false);
            let transforms = d.transforms().collect::<Vec<_>>();
            scene.draw(c.eye, 2., false, false, &transforms);
            art.draw(d, false, &scene.atmosphere, c.eye);
            crate::render::depth_read_only(|| {
                scene.draw(c.eye, 2., false, true, &transforms);
                if frame != frames - 3 {
                    art.effects(d, c.eye, &scene.atmosphere);
                }
            });
            crate::render_fx::finish();
            set_default_camera();
            art.hud(d);
            draw_text(case, 18., screen_height() - 25., 20., WHITE);
            if frame == frames - 3 {
                without = Some(get_screen_data());
            }
            if frame == frames - 2 {
                with = Some(get_screen_data());
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/{case}-paused.png"
                )))?;
            }
            if frame == frames - 1 {
                crate::viewer::save_capture(std::path::Path::new(&format!("private/{case}.png")))?;
                let now = get_screen_data();
                // Alpha compositing can differ by one quantization step on this GPU.
                let pause_delta = with
                    .as_ref()
                    .unwrap()
                    .bytes
                    .iter()
                    .zip(&now.bytes)
                    .map(|(a, b)| a.abs_diff(*b))
                    .max()
                    .unwrap_or(0);
                ensure!(
                    pause_delta <= 1,
                    "Paused effects changed in {case}: {pause_delta}"
                );
                if matches!(
                    case,
                    "duchess-pepper"
                        | "duchess-phase"
                        | "duchess-death"
                        | "duchess-blood"
                        | "duchess-smoke"
                ) || case.starts_with("duchess-sneeze")
                {
                    let changed = now
                        .bytes
                        .chunks_exact(4)
                        .zip(without.as_ref().unwrap().bytes.chunks_exact(4))
                        .filter(|(a, b)| {
                            a[..3].iter().zip(&b[..3]).any(|(a, b)| a.abs_diff(*b) > 8)
                        })
                        .count();
                    ensure!(
                        changed > 30,
                        "No visible effect in {case}: {changed} pixels"
                    );
                    println!("PASS {case}: {changed} effect pixels; pause stable");
                }
            }
            next_frame().await;
        }
        println!("Captured {case}");
        player_art.check_visible(case).await?;
    }
    Ok(())
}

fn contracts(assets: &mut crate::assets::Assets) -> Result<()> {
    use crate::{
        duchess::Duchess, interaction::Interactions, inventory::Stats, movement::Player,
        story::Story,
    };
    let map = crate::bsp::Bsp::parse(&assets.read("maps/potears3.bsp")?)?;
    let mut i = Interactions::load(&map)?;
    i.set_entry(assets, &map, "potears3", None)?;
    let locked = i.triggers(0.01, vec3(320., 224., -430.), vec3(320., 224., -430.));
    ensure!(locked.transition.is_none(), "Exit bypassed living Duchess");
    let mut world = crate::collision::World::from_bsp(&map)?;
    i.sync(&mut world);
    let mut p = Player::new(vec3(64., 48., 16.));
    let mut stats = Stats::for_level("potears3", None);
    let mut story = Story::load(assets, "potears3");
    stats.apply(crate::inventory::PickupKind::Weapon(3), 1.);
    let d = i.duchess.as_mut().unwrap();
    d.update(0.01, &world, &mut p, &mut stats, &mut story);
    ensure!(
        d.state.stage == Stage::Waiting,
        "Weapon ownership incorrectly activated boss"
    );
    for case in [
        "duchess-expand",
        "duchess-intro",
        "duchess-phase",
        "duchess-pepper",
        "duchess-pig",
        "duchess-bite",
        "duchess-death",
        "duchess-rescue",
        "duchess-reward",
    ] {
        let mut d = Duchess::load(assets, &map)?;
        let mut st = Story::load(assets, "potears3");
        d.fixture(case, &map, &mut world, &mut p, &mut stats, &mut st)?;
        let before = serde_json::to_value(d.snapshot())?;
        d.update(0., &world, &mut p, &mut stats, &mut st);
        d.advance(0., &map, &mut world, &mut p)?;
        ensure!(
            before == serde_json::to_value(d.snapshot())?,
            "Pause changed {case}"
        );
        let mut resumed = Duchess::load(assets, &map)?;
        resumed.restore(&serde_json::from_value(before)?, &map)?;
        ensure!(
            d.transforms().collect::<Vec<_>>() == resumed.transforms().collect::<Vec<_>>(),
            "Restored room differs"
        );
        if d.cinematic() {
            let mut ps = p.clone();
            let mut sts = Story::load(assets, "potears3");
            resumed.skip(&map, &mut world, &mut ps, &mut sts)?;
            let end = if case.ends_with("expand") || case.ends_with("intro") {
                Stage::Fighting
            } else {
                Stage::Complete
            };
            for _ in 0..18000 {
                d.advance(1. / 120., &map, &mut world, &mut p)?;
                d.update(1. / 120., &world, &mut p, &mut stats, &mut st);
                st.tick(1. / 120., false);
                for name in st.take_completed() {
                    d.dialogue_complete(&name);
                }
                if d.state.stage == end {
                    break;
                }
            }
            ensure!(
                d.state.stage == end && d.state.shell_returned == resumed.state.shell_returned,
                "Watched/skip progression differs for {case}"
            );
            ensure!(
                d.transforms().collect::<Vec<_>>() == resumed.transforms().collect::<Vec<_>>(),
                "Watched/skip arena differs for {case}"
            );
            ensure!(
                !resumed.skip(&map, &mut world, &mut ps, &mut sts)?,
                "Skip repeated settled scene"
            );
        }
    }

    let mut results = Vec::new();
    for fps in [30, 60, 144] {
        let mut d = Duchess::load(assets, &map)?;
        let mut st = Story::load(assets, "potears3");
        let mut stats = Stats::for_level("potears3", None);
        let mut p = Player::new(vec3(64., 48., 16.));
        d.fixture(
            "duchess-pepper",
            &map,
            &mut world,
            &mut p,
            &mut stats,
            &mut st,
        )?;
        d.state.notarget = true;
        for _ in 0..fps * 2 {
            d.update(1. / fps as f32, &world, &mut p, &mut stats, &mut st);
        }
        results.push((d.state.clock, d.state.shots.len(), p.feet));
    }
    ensure!(
        results
            .windows(2)
            .all(|v| (v[0].0 - v[1].0).abs() < 0.0001 && v[0].1 == v[1].1 && v[0].2 == v[1].2),
        "Duchess frame-rate drift: {results:?}"
    );
    println!("PASS Duchess gates, pickup-only activation, paused state, phase restore, full-duration/skip outcomes");
    Ok(())
}
