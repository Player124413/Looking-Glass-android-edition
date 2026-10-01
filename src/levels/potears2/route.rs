//! Ordinary controls from the real entrance; no warps, stat grants or recovery.
use super::*;
use crate::{movement::Controls, route::Route};
fn state(r: &Route) -> &PoolTwo {
    r.interactions
        .levels
        .iter()
        .find_map(|l| l.ctl.downcast_ref())
        .unwrap()
}
pub fn check(a: &mut Assets) -> Result<()> {
    run(a, false)
}
pub async fn render(a: &mut Assets) -> Result<()> {
    if let Ok(path) = std::env::var("LOOKING_GLASS_HOLLOW_CAPTURE") {
        let r = Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?;
        let mut scene = crate::render::Scene::load(a, "potears2")?;
        let mut art = scene::Art::load(a)?;
        let o = state(&r);
        let eye = r.player.eye();
        let target = std::env::var("LOOKING_GLASS_HOLLOW_LOOK")
            .ok()
            .and_then(|s| crate::interaction::vector(&s))
            .unwrap_or(eye + Vec3::Y * 300.);
        let camera = o.camera(&r.world).unwrap_or(crate::cinematic::Camera {
            eye,
            target,
            up: Vec3::Z,
        });
        for frame in 0..3 {
            clear_background(BLACK);
            let view = Camera3D {
                position: camera.eye,
                target: camera.target,
                up: camera.up,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            };
            set_camera(&view);
            scene.prepare_camera_portals(
                &view,
                r.environment_clock,
                false,
                &r.interactions.transforms(),
            );
            crate::render_fx::begin_view(&view, r.environment_clock, &scene.atmosphere, false);
            let poses = r.interactions.transforms();
            scene.draw(camera.eye, r.environment_clock, false, false, &poses);
            art.draw(o, &scene.atmosphere, camera.eye, false);
            crate::render::depth_read_only(|| {
                scene.draw(camera.eye, r.environment_clock, false, true, &poses)
            });
            art.effects(o, camera.eye, &scene.atmosphere);
            set_default_camera();
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new("private/potears2/capture.png"))?;
            }
            next_frame().await;
        }
        return Ok(());
    }
    run(a, true)
}
fn run(a: &mut Assets, native: bool) -> Result<()> {
    std::fs::create_dir_all("private/potears2")?;
    let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_HOLLOW_FROM") {
        Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?
    } else {
        let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_HOLLOW_CARRY") {
            let cp: crate::route::Checkpoint = serde_json::from_slice(&std::fs::read(path)?)?;
            ensure!(
                cp.level.map == "potears1" && cp.stats.alive(),
                "Not a Pool carry"
            );
            Route::enter_carrying(
                a,
                "potears2",
                Some("potears2_start1"),
                cp.stats,
                cp.ledger,
                cp.difficulty,
            )?
        } else {
            Route::new(a, "potears2", Some("potears2_start1"))?
        };
        if native {
            r.enable_native_cast(a)?;
        }
        r
    };
    r.skip_cinematics = std::env::var_os("LOOKING_GLASS_HOLLOW_WATCH").is_none();
    drive(&mut r)?;
    println!("Hollow metrics: {}", r.metrics());
    checkpoint(&r, "exit")?;
    std::fs::write(
        "private/potears2/report.json",
        serde_json::to_vec_pretty(
            &serde_json::json!({"native":native,"metrics":r.metrics(),"audit":r.audit,"exit":r.transition}),
        )?,
    )?;
    let health = r.stats.sanity();
    let will = r.stats.will();
    let next = r.depart(a, true)?;
    ensure!(
        next.world.body_clear(next.player.feet) && next.stats.alive(),
        "Unsafe Just Desserts arrival"
    );
    ensure!(
        next.stats.sanity() == health && next.stats.will() == will,
        "Just Desserts changed carried health or Will"
    );
    checkpoint(&next, "arrival")?;
    println!("PASS Hollow Hideaway ordinary entrance to Just Desserts native={native}");
    Ok(())
}
pub(crate) fn drive(r: &mut Route) -> Result<()> {
    std::fs::create_dir_all("private/potears2")?;
    r.stop_at_exit = true;
    r.tactics = true;
    if let Ok(b) = std::env::var("LOOKING_GLASS_HOLLOW_PROBE") {
        return probe(&r, &b);
    }
    let steps: Vec<serde_json::Value> = if let Ok(p) = std::env::var("LOOKING_GLASS_HOLLOW_INPUT") {
        serde_json::from_slice(&std::fs::read(p)?)?
    } else {
        serde_json::from_str(include!("route_input.rs"))?
    };
    checkpoint(&r, "entry")?;
    for (i, s) in steps.iter().enumerate() {
        println!(
            "HOLLOW {i}: {s} from {:?} health {} fish {}",
            r.player.feet,
            r.stats.sanity(),
            state(&r).saved.fish.exposure
        );
        let result = input(r, s);
        checkpoint(&r, "last")?;
        result?;
        r.wait_for_cinematic()?;
        ensure!(r.stats.alive(), "Hollow route died at {:?}", r.player.feet);
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition == Some(EXIT.destination()),
        "Hollow route unfinished at {:?}",
        r.player.feet
    );
    r.assert_clean(0, 0)?;
    ensure!(
        !r.stats.god && !r.stats.notarget && r.stats.minimum_sanity == 0.,
        "Route assists"
    );
    let o = state(&r);
    ensure!(
        o.saved.pond.leaves.iter().all(Option::is_some)
            && o.saved.pond.ladies
            && o.saved.fish.attack.is_none()
            && o.ready()
            && o.saved.scene.phase == Phase::Done,
        "Incomplete pond route"
    );
    ensure!(
        serde_json::to_value(r.interactions.snapshot())?["triggers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["reported"] == false),
        "Pending pond contact"
    );
    Ok(())
}
fn checkpoint(r: &Route, label: &str) -> Result<()> {
    ensure!(
        !label.is_empty()
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-'),
        "Invalid checkpoint"
    );
    std::fs::write(
        format!("private/potears2/{label}.json"),
        serde_json::to_vec(&r.checkpoint())?,
    )?;
    Ok(())
}
fn input(r: &mut Route, s: &serde_json::Value) -> Result<()> {
    if let Some(v) = s.get("ride") {
        let g: [f64; 2] = serde_json::from_value(v.clone())?;
        let i = g[0] as usize - 1;
        ensure!(state(r).saved.pond.leaves[i].is_some(), "Leaf not boarded");
        let offset: [f32; 2] = s
            .get("offset")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?
            .unwrap_or([0., 0.]);
        let mut aim = vec2(offset[0], offset[1]);
        for tick in 0..120 * 70 {
            if state(r).saved.pond.clocks[5 + i] >= g[1] {
                return Ok(());
            }
            if s.get("auto").and_then(|v| v.as_bool()) == Some(true) && tick % 15 == 0 {
                let o = state(r);
                let pad = &o.pond.pads[5 + i];
                let mut best = None;
                for x in [-64., -32., 0., 32., 64.] {
                    for y in [-40., -20., 0., 20., 40.] {
                        let off = vec3(x, y, 2.03125);
                        let at = pad.pose.translation + off;
                        if !r.world.body_clear(at) || pad.collider.rider_feet(at).is_none() {
                            continue;
                        }
                        let tr = r.world.body_trace(r.player.feet, at);
                        if tr.start_solid || tr.fraction < 1. {
                            continue;
                        }
                        let mut safe = 0;
                        for step in 0..=8 {
                            let future = o
                                .pond
                                .future(5 + i, o.saved.pond.clocks[5 + i] + step as f64 * 0.2)
                                + off;
                            if !r.world.body_clear(future) {
                                break;
                            }
                            safe += 1;
                        }
                        let score = safe as f32 * 1000. - at.distance(r.player.feet);
                        if best.is_none_or(|(_, v)| score > v) {
                            best = Some((vec2(x, y), score));
                        }
                    }
                }
                if let Some((next, _)) = best {
                    aim = next;
                }
            }
            let d = state(r).pond.pads[5 + i].pose.translation + aim.extend(0.) - r.player.feet;
            r.tick(Controls {
                wish: (d.truncate() / 50.).clamp_length_max(0.7),
                swim: d.normalize_or_zero(),
                run: true,
                ..Default::default()
            })?;
        }
        anyhow::bail!("Leaf ride stalled");
    } else if s.get("shore").and_then(|v| v.as_bool()) == Some(true) {
        // Wait on the moving pad until the ordinary jump can reach the east bank.
        // A fixed delay fails when earlier combat changes the lily's loop phase.
        for _ in 0..120 * 30 {
            let at = r.player.feet;
            if r.player.grounded && at.x > -335. && (-1070. ..=-960.).contains(&at.y) {
                return Ok(());
            }
            r.tick(Controls::default())?;
        }
        anyhow::bail!("Lily five did not reach its shore landing");
    } else if let Some(v) = s.get("board") {
        let name = v.as_str().unwrap();
        for tick in 0..120 * 20 {
            let p = state(r)
                .pond
                .pads
                .iter()
                .find(|p| p.name == name)
                .context("Unknown pad")?;
            let d = p.pose.translation + Vec3::Z * 8. - r.player.feet;
            if r.player.grounded && d.truncate().length() < 45. && d.z.abs() < 20. {
                return Ok(());
            }
            r.tick(Controls {
                wish: (d.truncate() / 70.).clamp_length_max(1.),
                swim: d.normalize_or_zero(),
                rise: if r.player.immersion.level > 0 { 1. } else { 0. },
                jump: tick % 90 == 0,
                run: true,
                ..Default::default()
            })?;
        }
        anyhow::bail!("Cannot board {name}");
    } else if let Some(v) = s.get("nav") {
        let g: [f32; 3] = serde_json::from_value(v.clone())?;
        r.navigate(Vec3::from_array(g))?;
    } else if let Some(v) = s.get("steer") {
        let g: [f32; 3] = serde_json::from_value(v.clone())?;
        steer(r, Vec3::from_array(g))?;
    } else if let Some(v) = s.get("input") {
        let g: [f32; 6] = serde_json::from_value(v.clone())?;
        for k in 0..(g[0] * 120.) as usize {
            r.tick(Controls {
                wish: vec2(g[1], g[2]),
                swim: vec3(g[1], g[2], 0.),
                rise: g[3],
                jump: k == 0 && g[4] > 0.,
                use_pressed: k == 0 && g[5] > 0.,
                run: true,
                ..Default::default()
            })?;
            ensure!(
                r.stats.alive(),
                "Hollow Hideaway died at {:?}",
                r.player.feet
            );
        }
    } else if let Some(v) = s.get("clear") {
        r.clear(v.as_f64().unwrap() as f32)?;
    } else if let Some(v) = s.get("save") {
        let label = v.as_str().unwrap();
        checkpoint(r, label)?;
    } else {
        anyhow::bail!("Unknown route command {s}");
    }
    Ok(())
}
fn steer(r: &mut Route, goal: Vec3) -> Result<()> {
    let mut last = r.player.feet;
    for t in 0..120 * 18 {
        if r.interactions.scripted() {
            r.wait_for_cinematic()?;
        }
        let d = goal - r.player.feet;
        if d.truncate().length() < 20. && d.z.abs() < 48. {
            return Ok(());
        }
        let stuck = t % 120 == 119 && r.player.feet.distance(last) < 18.;
        if t % 120 == 119 {
            last = r.player.feet;
        }
        r.tick(Controls {
            wish: d.truncate().normalize_or_zero(),
            swim: d.normalize_or_zero(),
            rise: if r.player.immersion.level > 0 { 1. } else { 0. },
            jump: stuck,
            run: true,
            ..Default::default()
        })?;
        ensure!(
            r.stats.alive(),
            "Hollow Hideaway died at {:?}",
            r.player.feet
        );
        if r.transition.is_some() {
            return Ok(());
        }
    }
    anyhow::bail!(
        "Steer {goal:?} blocked at {:?} ledge {:?}",
        r.player.feet,
        r.player.ledge
    )
}
fn probe(r: &Route, bounds: &str) -> Result<()> {
    let b: [i32; 6] = serde_json::from_str(bounds)?;
    let mut points = vec![];
    for x in (b[0]..=b[1]).step_by(32) {
        for y in (b[2]..=b[3]).step_by(32) {
            let mut z = b[5] as f32;
            while z > b[4] as f32 {
                let high = vec3(x as f32, y as f32, z);
                if r.world.body_clear(high) {
                    let low = high - Vec3::Z * 32.;
                    let h = r.world.body_trace(high, low);
                    if !h.start_solid && h.fraction < 1. && h.normal.z >= 0.65 {
                        let p = high.lerp(low, h.fraction);
                        points.push([p.x, p.y, p.z]);
                    }
                }
                z -= 32.;
            }
        }
    }
    std::fs::write("private/potears2/floors.json", serde_json::to_vec(&points)?)?;
    println!("Surveyed {} supported points", points.len());
    Ok(())
}
