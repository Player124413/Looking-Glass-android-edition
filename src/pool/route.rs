//! Ordinary entrance-to-exit movement; diagnostic inputs never edit player/world state.
use crate::{assets::Assets, movement::Controls, route::Route};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
#[path = "route_input.rs"]
mod route_input;

pub fn check(a: &mut Assets, skip: bool, native: bool) -> Result<()> {
    std::fs::create_dir_all("private/potears1")?;
    let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_POOL_FROM") {
        Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?
    } else {
        let mut r = Route::new(a, "potears1", Some("potears1_start1"))?;
        if native {
            r.enable_native_cast(a)?;
        }
        r
    };
    r.skip_cinematics = skip;
    drive(&mut r)?;
    println!("Pool metrics: {}", r.metrics());
    checkpoint(&r, "exit")?;
    std::fs::write(
        "private/potears1/report.json",
        serde_json::to_vec_pretty(&serde_json::json!({
            "skip": skip, "native": native, "resumed": std::env::var_os("LOOKING_GLASS_POOL_FROM").is_some(),
            "metrics": r.metrics(), "audit": r.audit, "exit": r.transition,
        }))?,
    )?;
    let carried = serde_json::to_value(&r.stats)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.world.body_clear(next.player.feet) && serde_json::to_value(&next.stats)? == carried,
        "Unsafe Hollow Hideaway arrival"
    );
    println!(
        "PASS Pool {} to Hollow Hideaway skip={skip} native={native}",
        if std::env::var_os("LOOKING_GLASS_POOL_FROM").is_some() {
            "resumed route"
        } else {
            "ordinary entrance"
        }
    );
    Ok(())
}
pub(crate) fn drive(r: &mut Route) -> Result<()> {
    std::fs::create_dir_all("private/potears1")?;
    r.stop_at_exit = true;
    r.tactics = true;
    r.conserve_will = true;
    r.wait_for_cinematic()?;
    if let Ok(bounds) = std::env::var("LOOKING_GLASS_POOL_PROBE") {
        return probe(&r, &bounds);
    }
    let steps: Vec<serde_json::Value> = if let Ok(path) = std::env::var("LOOKING_GLASS_POOL_INPUT")
    {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else {
        serde_json::from_str(route_input::STEPS)?
    };
    checkpoint(&r, "entry")?;
    for (index, step) in steps.iter().enumerate() {
        println!(
            "POOL {index}: {step} from {:?}, sanity {}",
            r.player.feet,
            r.stats.sanity()
        );
        let result = input(r, step);
        checkpoint(&r, "last")?;
        result?;
        r.wait_for_cinematic()?;
        ensure!(r.stats.alive(), "Pool died at {:?}", r.player.feet);
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition == Some(("potears2".into(), Some("potears2_start1".into()))),
        "Pool route unfinished at {:?}",
        r.player.feet
    );
    r.assert_clean(0, 0)?;
    ensure!(
        !r.stats.god && !r.stats.notarget && r.stats.minimum_sanity == 0.,
        "Route used assists"
    );
    let pool = r.interactions.pool.as_ref().unwrap();
    ensure!(
        pool.state.rides.iter().all(Option::is_some)
            && pool.state.turtle.iter().all(Option::is_some)
            && pool.state.cinema.done.iter().all(|v| *v)
            && pool.state.cinema.exit_sent,
        "Pool missed a transport or scene"
    );
    let state = serde_json::to_value(r.interactions.snapshot())?;
    for id in [
        70, 667, 69, 110, 128, 62, 58, 109, 82, 50, 56, 63, 67, 116, 118,
    ] {
        ensure!(
            state["triggers"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["id"] == id && t["fired"] == true),
            "Pool missed route trigger {id}"
        );
    }
    ensure!(
        state["triggers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["reported"] == false),
        "Pool left an unsupported contact"
    );
    Ok(())
}
fn checkpoint(r: &Route, label: &str) -> Result<()> {
    ensure!(
        !label.is_empty()
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-'),
        "Invalid checkpoint label"
    );
    std::fs::write(
        format!("private/potears1/{label}.json"),
        serde_json::to_vec(&r.checkpoint())?,
    )?;
    Ok(())
}
fn input(r: &mut Route, s: &serde_json::Value) -> Result<()> {
    // Difficulty-specific collision can put the rider on the branch rather
    // than above it. Jump from actual support; never change the saved pose.
    if let Some(v) = s.get("jump_if_grounded") {
        if r.player.grounded {
            let g: [f32; 3] = serde_json::from_value(v.clone())?;
            return input(r, &serde_json::json!({"input":[g[0],g[1],g[2],0,1,0]}));
        }
        return Ok(());
    }
    if let Some(v) = s.get("cards") {
        r.conserve_will = !v
            .as_bool()
            .ok_or_else(|| anyhow::anyhow!("cards needs a boolean"))?;
    } else if let Some(v) = s.get("board") {
        let i = v.as_u64().unwrap() as usize - 1;
        let name = format!("rideleaf{}obj", i + 1);
        for _ in 0..120 * 30 {
            let p = r.interactions.pool.as_ref().unwrap();
            let o = p.objects.iter().find(|o| o.name == name).unwrap();
            let d = o.pose.0 + Vec3::Z * 8. - r.player.feet;
            if p.state.rides[i].is_some()
                && r.player.grounded
                && d.truncate().length() < 75.
                && d.z.abs() < 20.
            {
                return Ok(());
            }
            r.tick(Controls {
                wish: (d.truncate() / 100.).clamp_length_max(1.),
                swim: d.normalize_or_zero(),
                rise: if r.player.immersion.level > 0 { 1. } else { 0. },
                run: true,
                ..Default::default()
            })?;
        }
        anyhow::bail!("Cannot board leaf {} at {:?}", i + 1, r.player.feet);
    } else if let Some(v) = s.get("align") {
        let g: [f32; 3] = serde_json::from_value(v.clone())?;
        let goal = Vec3::from_array(g);
        for _ in 0..120 * 12 {
            let d = goal - r.player.feet;
            if d.truncate().length() < 2.
                && d.z.abs() < 24.
                && r.player.grounded
                && r.player.velocity.truncate().length() < 10.
            {
                return Ok(());
            }
            r.tick(Controls {
                wish: (d.truncate() / 80.).clamp_length_max(1.),
                swim: d.normalize_or_zero(),
                rise: if r.player.immersion.level > 0 { 1. } else { 0. },
                run: true,
                ..Default::default()
            })?;
        }
        anyhow::bail!("Align {goal:?} stopped at {:?}", r.player.feet);
    } else if let Some(v) = s.get("ride") {
        let g: [f32; 2] = serde_json::from_value(v.clone())?;
        let i = g[0] as usize - 1;
        ensure!(
            r.interactions.pool.as_ref().unwrap().state.rides[i].is_some(),
            "Leaf {} not boarded",
            i + 1
        );
        for _ in 0..120 * 120 {
            let p = r.interactions.pool.as_ref().unwrap();
            let name = format!("rideleaf{}obj", i + 1);
            if p.state.age
                - p.state.rides[i].unwrap()
                - p.state.blocked_time.get(&name).copied().unwrap_or(0.)
                >= g[1]
                || r.exited()
            {
                return Ok(());
            }
            let center = p.objects.iter().find(|o| o.name == name).unwrap().pose.0;
            let d = center - r.player.feet;
            r.tick(Controls {
                wish: (d.truncate() / 60.).clamp_length_max(1.),
                swim: d.normalize_or_zero(),
                rise: if r.player.immersion.level > 0 { 1. } else { 0. },
                run: true,
                ..Default::default()
            })?;
        }
        anyhow::bail!("Ride timer did not advance");
    } else if let Some(v) = s.get("ready") {
        let i = v.as_u64().unwrap() as usize - 1;
        for _ in 0..120 * 60 {
            if r.interactions.pool.as_ref().unwrap().ready(i) {
                return Ok(());
            }
            r.tick(Controls::default())?;
        }
        anyhow::bail!("Leaf {} did not become ready", i + 1);
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
            ensure!(r.stats.alive(), "Pool died at {:?}", r.player.feet);
        }
    } else if let Some(v) = s.get("clear") {
        r.clear(v.as_f64().unwrap() as f32)?;
    } else if let Some(v) = s.get("save") {
        let label = v.as_str().unwrap();
        if matches!(label, "rope-grip" | "rope-top") {
            ensure!(
                r.player.rope.as_ref().is_some_and(|g| g.id == crate::entity::Id(57)),
                "{label}: Alice has not caught the rope"
            );
            ensure!(
                r.player.feet.z > if label == "rope-top" { 380. } else { 200. },
                "{label}: Alice has not climbed the rope"
            );
        }
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
        ensure!(r.stats.alive(), "Pool died at {:?}", r.player.feet);
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
    std::fs::write("private/potears1/floors.json", serde_json::to_vec(&points)?)?;
    println!("Surveyed {} supported points", points.len());
    Ok(())
}
pub async fn render(a: &mut Assets) -> Result<()> {
    if let Ok(path) = std::env::var("LOOKING_GLASS_POOL_CAPTURE") {
        let cp: crate::route::Checkpoint = serde_json::from_slice(&std::fs::read(path)?)?;
        let mut r = Route::resume(a, &cp)?;
        let mut scene = crate::render::Scene::load(a, "potears1")?;
        let mut art = super::Art::load(a, r.interactions.pool.as_ref().unwrap())?;
        let mut foes = crate::encounters::Art::load(a)?;
        let eye = r.player.eye();
        let target = std::env::var("LOOKING_GLASS_POOL_LOOK")
            .ok()
            .and_then(|s| crate::interaction::vector(&s))
            .unwrap_or(eye + Vec3::Y * 300.);
        for frame in 0..3 {
            clear_background(scene.atmosphere.background());
            let camera = Camera3D {
                position: eye,
                target,
                up: Vec3::Z,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            };
            set_camera(&camera);
            scene.prepare_camera_portals(
                &camera,
                r.environment_clock,
                false,
                &r.interactions.transforms(),
            );
            scene.update_ropes(
                0.,
                r.player.rope.as_ref().and_then(|g| {
                    r.world
                        .traversal
                        .rope(g.id)
                        .map(|rope| (rope.model, r.player.feet + Vec3::Z * 40.))
                }),
                &r.interactions.transforms(),
            );
            crate::render_fx::begin_view(&camera, r.environment_clock, &scene.atmosphere, false);
            scene.draw(
                eye,
                r.environment_clock,
                false,
                false,
                &r.interactions.transforms(),
            );
            art.draw(
                r.interactions.pool.as_ref().unwrap(),
                false,
                &scene.atmosphere,
                eye,
            );
            if let Some(e) = &r.interactions.encounters {
                foes.draw(e, false, &scene.atmosphere, eye);
            }
            if let Some(n) = &mut r.native_cast {
                n.draw(
                    eye,
                    (target - eye).normalize_or_zero(),
                    &scene.atmosphere,
                    false,
                );
            }
            crate::render::depth_read_only(|| {
                scene.draw(
                    eye,
                    r.environment_clock,
                    false,
                    true,
                    &r.interactions.transforms(),
                )
            });
            crate::render_fx::finish();
            set_default_camera();
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(
                    "private/potears1/route-view.png",
                ))?;
            }
            next_frame().await;
        }
        return Ok(());
    }
    check(
        a,
        std::env::var_os("LOOKING_GLASS_WATCH_SCENES").is_none(),
        true,
    )
}
