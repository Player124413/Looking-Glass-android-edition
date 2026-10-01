use super::*;
use crate::route::Route;
#[path = "swim_path.rs"]
mod swim_path;
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
        std::fs::create_dir_all("private/tower2-work")?;
        if let Ok(bounds) = std::env::var("LOOKING_GLASS_TOWER2_PROBE") {
            return probe(a, &bounds);
        }
        if std::env::var_os("LOOKING_GLASS_TOWER2_CAPTURE").is_some() {
            return capture(a).await;
        }
        for skip in [false, true] {
            walk(a, skip)?;
        }
        Ok(())
    })
}
fn checkpoint(r: &Route) -> Result<()> {
    std::fs::write(
        "private/tower2-work/last-checkpoint.json",
        serde_json::to_vec(&r.checkpoint())?,
    )?;
    Ok(())
}
fn walk(a: &mut Assets, skip: bool) -> Result<()> {
    let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_TOWER2_FROM") {
        Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?
    } else {
        let mut r = Route::new(a, "tower2", Some("tower2_start1"))?;
        r.enable_native_cast(a)?;
        r
    };
    r.skip_cinematics=skip;
    drive(a,&mut r)?;
    let stats = serde_json::to_value(&r.stats)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.level().map == "hedge3"
            && next.world.body_clear(next.player.feet)
            && serde_json::to_value(&next.stats)? == stats,
        "Bad hedge3 entry or changed resources"
    );
    println!("PASS Water Logged skip={skip}: three stages and live dive into hedge3");
    Ok(())
}
fn steer(r: &mut Route, goal: Vec3, mode: &str) -> Result<()> {
    let mut last = r.player.feet;
    let mut jumped = false;
    for n in 0..120 * 25 {
        let d = goal - r.player.feet;
        if d.truncate().length() < if mode == "swim" { 8. } else { 24. }
            && d.z.abs() < if mode == "swim" { 8. } else { 64. }
        {
            return Ok(());
        }
        let jump = r.player.ledge.as_ref().is_some_and(|h| !h.pulling)
            || mode == "jump" && !jumped && r.player.grounded
            || mode != "swim" && n % 120 == 119 && r.player.feet.distance(last) < 8.;
        jumped |= jump;
        if n % 120 == 119 {
            last = r.player.feet;
        }
        r.tick(Controls {
            wish: d.truncate().normalize_or_zero(),
            swim: if mode == "swim" {
                d.normalize_or_zero()
            } else {
                d.truncate().normalize_or_zero().extend(0.)
            },
            rise: if mode == "swim" {
                0.
            } else if r.player.immersion.level > 0 {
                1.
            } else {
                0.
            },
            jump,
            run: true,
            ..Default::default()
        })?;
        if r.transition.is_some() {
            return Ok(());
        }
        if n % 120 == 119 {
            println!(
                "  TOWER {:?}, v {:?}, depth {}, sanity {}",
                r.player.feet,
                r.player.velocity,
                r.player.immersion.level,
                r.stats.sanity()
            );
        }
    }
    anyhow::bail!("Tower stuck toward {goal:?} at {:?}", r.player.feet)
}
fn waterpath(r: &mut Route, goal: Vec3) -> Result<()> {
    use std::collections::{BTreeMap, VecDeque};
    const STEP: f32 = 32.;
    let at = |k: (i32, i32)| vec3(k.0 as f32 * STEP, k.1 as f32 * STEP, goal.z);
    let key = |p: Vec3| ((p.x / STEP).round() as i32, (p.y / STEP).round() as i32);
    let clear = |a: Vec3, b: Vec3| {
        let t = r.world.body_trace(a, b);
        !t.start_solid && t.fraction >= 1.
    };
    let start = key(r.player.feet);
    let end = key(goal);
    let mut parent = BTreeMap::new();
    let mut queue = VecDeque::new();
    for x in -2..=2 {
        for y in -2..=2 {
            let k = (start.0 + x, start.1 + y);
            if clear(r.player.feet, at(k)) {
                parent.insert(k, None);
                queue.push_back(k);
            }
        }
    }
    let mut found = None;
    while let Some(k) = queue.pop_front() {
        if (k.0 - end.0).abs() <= 1 && (k.1 - end.1).abs() <= 1 && clear(at(k), goal) {
            found = Some(k);
            break;
        }
        for (dx, dy) in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ] {
            let n = (k.0 + dx, k.1 + dy);
            if n.0 < 0 || n.0 > 110 || n.1 < -12 || n.1 > 88 || parent.contains_key(&n) {
                continue;
            }
            if r.world.liquid_at(at(n) + Vec3::Z * 20.) != 0 && clear(at(k), at(n)) {
                parent.insert(n, Some(k));
                queue.push_back(n);
            }
        }
    }
    let mut k = found.context("No swimming path at this water height")?;
    let mut path = vec![goal];
    loop {
        path.push(at(k));
        if let Some(n) = parent[&k] {
            k = n;
        } else {
            break;
        }
    }
    path.reverse();
    // Simplify only when the entire player's swept body clears the shortcut.
    let mut waypoints = vec![];
    let mut current = r.player.feet;
    let mut cursor = 0;
    while cursor < path.len() {
        let mut end = cursor;
        while end + 1 < path.len() && clear(current, path[end + 1]) {
            end += 1;
        }
        current = path[end];
        waypoints.push(current);
        cursor = end + 1;
    }
    println!("TOWER swimming path {waypoints:?}");
    for p in waypoints {
        steer(r, p, "swim")?;
    }
    Ok(())
}
fn probe(a: &mut Assets, bounds: &str) -> Result<()> {
    let b: [i32; 6] = serde_json::from_str(bounds)?;
    let cp: crate::route::Checkpoint =
        serde_json::from_slice(&std::fs::read("private/tower2-work/last-checkpoint.json")?)?;
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
        "private/tower2-work/physical-floors.json",
        serde_json::to_vec(&points)?,
    )?;
    println!(
        "PASS collision survey: {} supported body positions",
        points.len()
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
        "private/tower2-work/route-{}-{name}.json",
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
async fn capture(a: &mut Assets) -> Result<()> {
    let cp: crate::route::Checkpoint =
        serde_json::from_slice(&std::fs::read("private/tower2-work/last-checkpoint.json")?)?;
    let mut r = Route::resume(a, &cp)?;
    let mut scene = crate::render::Scene::load(a, "tower2")?;
    let mut art = scene::Art::load(a)?;
    let eye = std::env::var("LOOKING_GLASS_TOWER2_EYE")
        .ok()
        .and_then(|s| interaction::vector(&s))
        .unwrap_or(r.player.eye());
    let target = std::env::var("LOOKING_GLASS_TOWER2_LOOK")
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
                "private/tower2-work/route-view.png",
            ))?;
        }
        next_frame().await;
    }
    Ok(())
}

pub(crate) fn drive(a: &mut Assets, r: &mut Route) -> Result<()> {
    std::fs::create_dir_all("private/tower2-work")?;
    let skip = r.skip_cinematics;
    r.tactics = true;
    r.stop_at_exit = true;
    r.wait_for_cinematic()?;
    checkpoint(&r)?;
    let steps: Vec<Step> = if let Ok(path) = std::env::var("LOOKING_GLASS_TOWER2_ROUTE") {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else {
        serde_json::from_str(include_str!("route_steps.json"))?
    };
    for (k, s) in steps.iter().enumerate() {
        let goal = Vec3::from_array(s.pos);
        println!(
            "TOWER goal {k} {goal:?} {} at {:?} sanity {}",
            s.mode,
            r.player.feet,
            r.stats.sanity()
        );
        let result = match s.mode.as_str() {
            "waterpath" => waterpath(r, goal),
            "pipe" => swim_path::swim(r, goal),
            "fan" => swim_path::fan(r),
            "nav" => r.navigate(goal),
            "wait" => {
                let mut out = Ok(());
                for _ in 0..s.frames {
                    if let Err(e) = r.tick(Controls::default()) {
                        out = Err(e);
                        break;
                    }
                }
                out
            }
            "input" | "inputjump" => {
                let mut out = Ok(());
                for n in 0..s.frames {
                    if let Err(e) = r.tick(Controls {
                        wish: goal.truncate(),
                        swim: goal,
                        rise: goal.z,
                        run: true,
                        jump: n == 0 && s.mode == "inputjump",
                        ..Default::default()
                    }) {
                        out = Err(e);
                        break;
                    }
                }
                out
            }
            "clear" => r.clear(goal.x),
            _ => steer(r, goal, &s.mode),
        };
        checkpoint(&r)?;
        result?;
        if let Some(name) = &s.save {
            continuation(a, r, name, skip)?;
        }
        ensure!(r.stats.alive(), "Tower route died at {:?}", r.player.feet);
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition == Some(("hedge3".into(), Some("hedge3_start1".into()))),
        "Tower route has not reached hedge3: {:?}",
        r.player.feet
    );
    ensure!(
        owner(&mut r.interactions)?.exit_ready()
            && r.stats.turtle_air
            && r.teleports == 0
            && !r.stats.god
            && !r.stats.notarget
            && r.native_cast.is_some()
            && r.damage > 0.
            && r.shots + r.cards > 0
            && r.stats.collected.contains("tower2:29")
            && r.player.breath.remaining() > 0.
            && r.player.breath.hits == 0,
        "Tower route bypassed stages or live play"
    );
    println!(
        "TOWER finished skip={skip}: sanity {}, damage {}, shots {}, air {:?}",
        r.stats.sanity(),
        r.damage,
        r.shots,
        r.player.breath
    );
    Ok(())
}
