//! Continuous input through the actual world, with the production resident cast.
use super::*;
use crate::{movement::Controls, route::Route};

#[derive(Deserialize)]
struct Step {
    pos: [f32; 3],
    #[serde(default)]
    mode: String,
    #[serde(default)]
    frames: usize,
    #[serde(default)]
    save: Option<String>,
}
pub(super) fn check(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        if let Ok(bounds) = std::env::var("LOOKING_GLASS_GARDEN2_PROBE") {
            return probe(a, &bounds);
        }
        if std::env::var_os("LOOKING_GLASS_GARDEN2_CAPTURE").is_some() {
            return capture(a).await;
        }
        for skip in [false, true] {
            walk(a, skip)?;
        }
        Ok(())
    })
}
fn probe(a: &mut Assets, bounds: &str) -> Result<()> {
    let b: [i32; 6] = serde_json::from_str(bounds)?;
    let cp: crate::route::Checkpoint =
        serde_json::from_slice(&std::fs::read("private/garden2-work/last-checkpoint.json")?)?;
    let r = Route::resume(a, &cp)?;
    let mut points = vec![];
    for x in (b[0]..=b[1]).step_by(24) {
        for y in (b[2]..=b[3]).step_by(24) {
            let mut z = b[5] as f32;
            while z > b[4] as f32 {
                let high = vec3(x as f32, y as f32, z);
                if r.world.body_clear(high) {
                    let low = high - Vec3::Z * 16.;
                    let hit = r.world.body_trace(high, low);
                    if !hit.start_solid && hit.fraction < 1. && hit.normal.z >= 0.65 {
                        let feet = high.lerp(low, hit.fraction);
                        points.push([feet.x, feet.y, feet.z, hit.normal.z]);
                    }
                }
                z -= 16.;
            }
        }
    }
    std::fs::write(
        "private/garden2-work/physical-floors.json",
        serde_json::to_vec(&points)?,
    )?;
    println!(
        "PASS collision survey: {} supported body positions",
        points.len()
    );
    Ok(())
}
fn walk(a: &mut Assets, skip: bool) -> Result<()> {
    std::fs::create_dir_all("private/garden2-work")?;
    let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_GARDEN2_FROM") {
        Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?
    } else {
        let mut r = Route::new(a, "garden2", Some("garden2_start1"))?;
        r.enable_native_cast(a)?;
        r
    };
    r.skip_cinematics = skip;
    drive(a, &mut r)?;
    let carried = serde_json::to_value(&r.stats)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.level().map == "garden3"
            && next.world.body_clear(next.player.feet)
            && serde_json::to_value(&next.stats)? == carried,
        "Bad Rolling Stones entry"
    );
    println!("PASS Herbaceous Border skip={skip}: native encounters and full underworld into Rolling Stones");
    Ok(())
}
pub(crate) fn drive(a: &mut Assets, r: &mut Route) -> Result<()> {
    let skip = r.skip_cinematics;
    std::fs::create_dir_all("private/garden2-work")?;
    r.stop_at_exit = true;
    r.wait_for_cinematic()?;
    let steps: Vec<Step> = if let Ok(path) = std::env::var("LOOKING_GLASS_GARDEN2_ROUTE") {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else {
        serde_json::from_str(include_str!("route_steps.json"))?
    };
    println!(
        "UNDERWORLD handoff {:?} sanity {}",
        r.player.feet,
        r.stats.sanity()
    );
    for (k, step) in steps.iter().enumerate() {
        let goal = Vec3::from_array(step.pos);
        println!(
            "UNDERWORLD goal {k} {goal:?} {} from {:?} sanity {}",
            step.mode,
            r.player.feet,
            r.stats.sanity()
        );
        match step.mode.as_str() {
            "input" | "inputjump" | "release" => {
                for n in 0..step.frames {
                    r.tick(Controls {
                        wish: goal.truncate(),
                        rise: goal.z,
                        run: true,
                        jump: step.mode == "inputjump" && n == 0,
                        use_pressed: step.mode == "release" && n == 0,
                        ..Default::default()
                    })?;
                }
            }
            "nav" => r.navigate(goal)?,
            "clear" => {
                r.clear(goal.x)?;
            }
            "rope" => {
                for n in 0..120 * 15 {
                    let d = goal.z - r.player.feet.z;
                    if r.player.rope.is_some() && d.abs() < 10. {
                        break;
                    }
                    r.tick(Controls {
                        rise: d.signum(),
                        use_pressed: n == 0 && r.player.rope.is_none(),
                        ..Default::default()
                    })?;
                }
                ensure!(r.player.rope.is_some(), "No rope at {:?}", r.player.feet);
            }
            "wait" => {
                for _ in 0..(goal.x * 120.) as usize {
                    r.tick(Controls::default())?;
                }
            }
            _ => steer(r, goal, &step.mode)?,
        }
        r.wait_for_cinematic()?;
        if let Some(name) = &step.save {
            continuation(a, r, name, skip)?;
        }
        std::fs::write(
            "private/garden2-work/last-checkpoint.json",
            serde_json::to_vec(&r.checkpoint())?,
        )?;
        ensure!(r.stats.alive(), "Underworld died at {:?}", r.player.feet);
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition == Some(("garden3".into(), Some("garden3_start1".into()))),
        "Underworld not yet at exit: {:?}",
        r.player.feet
    );
    let g = owner(&mut r.interactions)?;
    ensure!(
        g.saved.arrived
            && g.saved.bridge1
            && g.saved.bridge2
            && g.saved.cat
            && g.saved.scene.is_none()
            && g.objects
                .iter()
                .filter(|o| o.group != 0)
                .all(|o| o.pose.is_none()),
        "The underworld route omitted a scene or retained a fallen bridge"
    );
    ensure!(
        r.native_cast.is_some()
            && r.stats.alive()
            && !r.stats.god
            && !r.stats.notarget
            && r.teleports == 0
            && r.shots > 0
            && r.damage > 0.
            && r.stats.collected.contains("garden2:57"),
        "Route bypassed live play or the Mallet"
    );
    println!(
        "UNDERWORLD finished: Sanity {}, shots {}, damage {}, collected {:?}",
        r.stats.sanity(),
        r.shots,
        r.damage,
        r.stats.collected
    );
    Ok(())
}
fn continuation(a: &mut Assets, r: &mut Route, name: &str, skip: bool) -> Result<()> {
    // Aim smoothing is test-driver bookkeeping, not saved game state. Reset it
    // on both branches so their combat decisions receive the same history.
    let tactics = r.tactics;
    r.quiet();
    r.tactics = tactics;
    let path = format!(
        "private/garden2-work/route-{}-{name}.json",
        if skip { "skipped" } else { "watched" }
    );
    std::fs::write(&path, serde_json::to_vec(&r.checkpoint())?)?;
    let cp = serde_json::from_slice(&std::fs::read(&path)?)?;
    let mut loaded = Route::resume(a, &cp)?;
    loaded.tactics = tactics;
    loaded.stop_at_exit = true;
    loaded.skip_cinematics = skip;
    ensure!(
        r.state()? == loaded.state()?,
        "Save changed {name} before continuation"
    );
    for n in 0..120 {
        let first = r.tick(Controls::default());
        let second = loaded.tick(Controls::default());
        ensure!(
            r.state()? == loaded.state()?,
            "Saved {name} diverged after {n} ticks"
        );
        match (first, second) {
            (Ok(()), Ok(())) => {}
            (Err(x), Err(y)) if x.to_string() == y.to_string() => break,
            _ => anyhow::bail!("Saved {name} changed its continuation result"),
        }
    }
    // The main route itself continues from the disk save, at the marked moment.
    *r = Route::resume(a, &cp)?;
    r.tactics = tactics;
    r.stop_at_exit = true;
    r.skip_cinematics = skip;
    println!("PASS saved {name}: same live future and disk-restored route continuation");
    Ok(())
}
fn steer(r: &mut Route, goal: Vec3, mode: &str) -> Result<()> {
    let mut last = r.player.feet;
    let mut jumped = false;
    for t in 0..120 * 14 {
        if r.interactions.scripted() {
            r.wait_for_cinematic()?;
        }
        let d = goal.truncate() - r.player.feet.truncate();
        if mode == "grab" && r.player.rope.is_some() {
            return Ok(());
        }
        if mode == "rise"
            && d.length() < 35.
            && r.player.feet.z >= goal.z
            && r.player.velocity.z > 0.
        {
            return Ok(());
        }
        if mode == "vent"
            && d.length() < 60.
            && (r.player.updraft_time > 0. || r.player.knockback_time > 0.)
            && r.player.velocity.z > 100.
            && r.player.feet.z > goal.z - 70.
        {
            return Ok(());
        }
        if d.length() < 28.
            && (r.player.feet.z - goal.z).abs() < 90.
            && (r.player.grounded || r.player.immersion.level > 0)
        {
            return Ok(());
        }
        let jump = (matches!(mode, "jump" | "vent" | "leap")
            && !jumped
            && (r.player.grounded || mode == "leap" && r.player.rope.is_some()))
            || t % 120 == 119 && r.player.feet.distance(last) < 18.;
        jumped |= jump;
        if t % 120 == 119 {
            last = r.player.feet;
        }
        r.tick(Controls {
            wish: if mode == "rise" {
                (d / 100.).clamp_length_max(1.)
            } else {
                d.normalize_or_zero()
            },
            swim: d.normalize_or_zero().extend(0.),
            rise: if r.player.immersion.level > 0 || mode == "leap" && t == 0 {
                1.
            } else {
                0.
            },
            run: true,
            jump,
            use_pressed: mode == "grab" || mode == "leap" && t == 0,
            ..Default::default()
        })?;
        ensure!(
            r.stats.alive(),
            "Underworld died at {:?}, damage {}",
            r.player.feet,
            r.damage
        );
        if r.transition.is_some() {
            return Ok(());
        }
        if t % 120 == 119 {
            println!(
                "  UNDERWORLD {:?} velocity {:?}, rope {:?}, sanity {}",
                r.player.feet,
                r.player.velocity,
                r.player.rope,
                r.stats.sanity()
            );
        }
    }
    anyhow::bail!("Underworld stuck toward {goal:?} at {:?}", r.player.feet)
}
async fn capture(a: &mut Assets) -> Result<()> {
    let cp: crate::route::Checkpoint =
        serde_json::from_slice(&std::fs::read("private/garden2-work/last-checkpoint.json")?)?;
    let mut r = Route::resume(a, &cp)?;
    let mut scene = crate::render::Scene::load(a, "garden2")?;
    let mut art = art::Art::load(a)?;
    let eye = r.player.eye();
    let target = std::env::var("LOOKING_GLASS_GARDEN2_LOOK")
        .ok()
        .and_then(|s| crate::interaction::vector(&s))
        .unwrap_or(eye + Vec3::Y * 300.);
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
        scene.draw(eye, 0., false, false, &r.interactions.transforms());
        let o = owner(&mut r.interactions)?;
        art.draw(o, &scene.atmosphere, eye, false);
        if let Some(n) = &mut r.native_cast {
            n.draw(
                eye,
                (target - eye).normalize_or_zero(),
                &scene.atmosphere,
                false,
            );
        }
        crate::render::depth_read_only(|| {
            scene.draw(eye, 0., false, true, &r.interactions.transforms())
        });
        set_default_camera();
        if frame == 2 {
            crate::viewer::save_capture(std::path::Path::new(
                "private/garden2-work/route-view.png",
            ))?;
        }
        next_frame().await;
    }
    Ok(())
}
