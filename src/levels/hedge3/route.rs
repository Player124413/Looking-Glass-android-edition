use super::*;
use crate::route::Route;
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
        std::fs::create_dir_all("private/hedge3-work")?;
        let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_HEDGE3_FROM") {
            Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?
        } else {
            let mut r = Route::new(a, "hedge3", Some("hedge3_start1"))?;
            r.enable_native_cast(a)?;
            r
        };
        if std::env::var_os("LOOKING_GLASS_HEDGE3_CAPTURE").is_some() {
            return capture(a, &mut r).await;
        }
        drive(a,&mut r)?;
        let stats=serde_json::to_value(&r.stats)?;
        let next = r.depart(a, true)?;
        ensure!(
            next.level().map == "tower3"
                && next.world.body_clear(next.player.feet)
                && serde_json::to_value(&next.stats)? == stats,
            "Bad tower3 handoff"
        );
        println!("PASS Labyrinthine Revenge: live machinery route and authored tower3 entry");
        Ok(())
    })
}
// Drive ordinary movement toward a visible point on a moving platform. The
// point is local to its authored model; no player or mechanism state is staged.
fn board(r: &mut Route, id: usize, local: Vec3) -> Result<()> {
    let mut jumped = false;
    for _ in 0..120 * 12 {
        let feet = r.player.feet;
        let m = owner(&mut r.interactions)?;
        let o = m
            .objects
            .iter()
            .find(|o| o.id == id)
            .context("Unknown boarding object")?;
        let tr = o.collider.trace(
            feet + PLAYER_CENTER,
            feet + PLAYER_CENTER - Vec3::Z * 3.,
            PLAYER_HALF,
        );
        if r.player.grounded && !tr.start_solid && tr.fraction < 1. && tr.normal.z >= 0.65 {
            return Ok(());
        }
        let goal = o.pose.translation + o.pose.rotation * local;
        let jump = !jumped && r.player.grounded;
        jumped |= jump;
        r.tick(Controls {
            wish: (goal - feet).truncate().normalize_or_zero(),
            run: true,
            jump,
            ..Default::default()
        })?;
    }
    anyhow::bail!("Failed to board machine {id} at {:?}", r.player.feet)
}
fn ride(r: &mut Route, spec: Vec3) -> Result<()> {
    for _ in 0..120 * 35 {
        let feet = r.player.feet;
        let m = owner(&mut r.interactions)?;
        let o = m
            .objects
            .iter()
            .find(|o| o.id == spec.y as usize)
            .context("Unknown ride object")?;
        let d = feet - o.pose.translation;
        let angle = d.y.atan2(d.x).to_degrees();
        let delta = (angle - spec.x + 180.).rem_euclid(360.) - 180.;
        if delta.abs() < spec.z.max(0.5) && r.player.grounded {
            return Ok(());
        }
        r.tick(Controls::default())?;
    }
    anyhow::bail!("Machine ride did not reach exit bearing")
}
fn steer(r: &mut Route, goal: Vec3, mode: &str) -> Result<()> {
    let mut last = r.player.feet;
    let mut stalled = 0;
    let mut jumped = false;
    for n in 0..120 * 30 {
        let d = goal - r.player.feet;
        if d.truncate().length() < 22.
            && d.z.abs() < 64.
            && (mode == "air" || r.player.grounded || r.player.swimming)
        {
            return Ok(());
        }
        let jump = r.player.ledge.as_ref().is_some_and(|l| !l.pulling)
            || (r.player.grounded
                && (mode == "hop" || matches!(mode, "jump" | "air") && (!jumped || d.z > 24.)));
        jumped |= jump;
        let input = Controls {
            wish: if mode == "air" && d.truncate().length() < 30. {
                (d.truncate() * 3. - r.player.velocity.truncate() * 0.7).clamp_length_max(320.)
                    / 320.
            } else {
                d.truncate().normalize_or_zero()
            },
            swim: d.normalize_or_zero(),
            rise: d.z.signum(),
            run: true,
            jump,
            ..Default::default()
        };
        r.tick(input)?;
        if r.transition.is_some() {
            return Ok(());
        }
        if n % 120 == 119 {
            println!(
                "  HEDGE {:?} velocity {:?} grounded {} sanity {}",
                r.player.feet,
                r.player.velocity,
                r.player.grounded,
                r.stats.sanity()
            );
            if r.player.feet.distance(last) < 1. {
                stalled += 1;
            } else {
                stalled = 0;
            }
            last = r.player.feet;
            ensure!(stalled < 12, "Hedge stalled toward {goal:?}");
        }
    }
    anyhow::bail!(
        "Hedge traversal timeout at {:?} toward {goal:?}",
        r.player.feet
    )
}
fn continuation(a: &mut Assets, r: &mut Route, name: &str) -> Result<()> {
    let selected_heavy = r.heavy_weapon;
    let ice_stream = r.ice_stream;
    let conserve_will = r.conserve_will;
    r.quiet();
    r.heavy_weapon = selected_heavy;
    r.tactics = true;
    let path = format!("private/hedge3-work/route-{name}.json");
    std::fs::write(&path, serde_json::to_vec(&r.checkpoint())?)?;
    let cp = serde_json::from_slice(&std::fs::read(&path)?)?;
    let audit = r.audit.clone();
    let mut saved = Route::resume(a, &cp)?;
    saved.tactics = true;
    saved.heavy_weapon = selected_heavy;
    saved.ice_stream = ice_stream;
    saved.conserve_will = conserve_will;
    saved.stop_at_exit = true;
    ensure!(
        r.state()? == saved.state()?,
        "Hedge save changed before continuation"
    );
    for tick in 0..120 {
        let live = r.tick(Controls::default());
        let loaded = saved.tick(Controls::default());
        ensure!(
            live.as_ref().err().map(ToString::to_string)
                == loaded.as_ref().err().map(ToString::to_string),
            "Hedge saved continuation outcome diverged"
        );
        ensure!(
            r.state()? == saved.state()?,
            "Hedge saved continuation diverged"
        );
        if let Err(error) = live {
            // A reload must reproduce an exposed player's death too. This idle
            // comparison does not require the actual route to wait under fire.
            ensure!(
                !r.stats.alive() && error.to_string().starts_with("Route died at"),
                "Hedge continuation failed: {error}"
            );
            println!("HEDGE continuation {name}: identical idle death at tick {tick}");
            break;
        }
    }
    *r = Route::resume(a, &cp)?;
    r.audit = audit;
    r.heavy_weapon = selected_heavy;
    r.ice_stream = ice_stream;
    r.conserve_will = conserve_will;
    r.tactics = true;
    r.stop_at_exit = true;
    println!("PASS Hedge disk continuation {name}");
    Ok(())
}
fn probe(r: &Route, spec: &str) -> Result<()> {
    let values = spec
        .split(',')
        .map(str::parse::<f32>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        values.len() == 6,
        "Probe expects xmin,xmax,ymin,ymax,zmax,spacing"
    );
    let mut points = vec![];
    let mut x = values[0];
    while x <= values[1] {
        let mut y = values[2];
        while y <= values[3] {
            let mut z = values[4];
            for _ in 0..32 {
                let from = vec3(x, y, z);
                let tr = r.world.sweep(from, vec3(x, y, -1000.), vec3(1., 1., 1.));
                if tr.start_solid {
                    z -= 32.;
                    continue;
                }
                if tr.fraction >= 1. {
                    break;
                }
                let p = from.lerp(vec3(x, y, -1000.), tr.fraction);
                points.push((p.to_array(), tr.normal.to_array(), r.world.body_clear(p)));
                z = p.z - 64.;
            }
            y += values[5];
        }
        x += values[5];
    }
    std::fs::write(
        "private/hedge3-work/floors.json",
        serde_json::to_vec(&points)?,
    )?;
    println!("HEDGE probe {} floor samples", points.len());
    Ok(())
}

async fn capture(a: &mut Assets, r: &mut Route) -> Result<()> {
    let mut scene = crate::render::Scene::load(a, "hedge3")?;
    r.interactions.presentation.apply(&mut scene);
    let eye = std::env::var("LOOKING_GLASS_HEDGE3_EYE")
        .ok()
        .and_then(|s| interaction::vector(&s))
        .unwrap_or(r.player.eye());
    let target = std::env::var("LOOKING_GLASS_HEDGE3_LOOK")
        .ok()
        .and_then(|s| interaction::vector(&s))
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
        scene.draw(
            eye,
            r.environment_clock,
            false,
            false,
            &r.interactions.transforms(),
        );
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
        set_default_camera();
        if frame == 2 {
            crate::viewer::save_capture(std::path::Path::new(
                "private/hedge3-work/route-view.png",
            ))?;
        }
        next_frame().await;
    }
    Ok(())
}

fn wait_phase(r: &mut Route, spec: Vec3) -> Result<()> {
    for n in 0..120 * 30 {
        let m = owner(&mut r.interactions)?;
        let k = m
            .objects
            .iter()
            .position(|o| o.id == spec.y as usize)
            .context("Unknown phase object")?;
        let phase = (m.saved.machines[k].time as f32).rem_euclid(spec.z);
        if n % 240 == 0 {
            println!("  HEDGE wait phase {phase} at {:?}", r.player.feet);
        }
        if (phase - spec.x).abs() < FIXED_DT * 0.75 {
            return Ok(());
        }
        r.tick(Controls::default())?;
    }
    anyhow::bail!("Timed mechanism did not reach requested phase")
}

pub(crate) fn drive(a: &mut Assets, r: &mut Route) -> Result<()> {
    std::fs::create_dir_all("private/hedge3-work")?;
        let initial_stats = r.stats.clone();
        let fresh = std::env::var_os("LOOKING_GLASS_HEDGE3_FROM").is_none();
        r.tactics = true;
        r.stop_at_exit = true;
        if std::env::var_os("LOOKING_GLASS_HEDGE3_BENCH").is_some() {
            let cp = r.checkpoint();
            for mode in 0..4 {
                let mut b = Route::resume(a, &cp)?;
                b.tactics = true;
                let start = std::time::Instant::now();
                for _ in 0..120 {
                    if mode == 3 {
                        b.tick(Controls::default())?;
                        continue;
                    }
                    if mode != 1 {
                        b.interactions
                            .actors_for_movers(&b.native_cast.as_ref().unwrap().targets());
                        b.interactions.advance_school(
                            FIXED_DT,
                            &b.map,
                            &mut b.world,
                            &mut b.player,
                        )?;
                    }
                    if mode != 0 {
                        b.native_cast
                            .as_mut()
                            .unwrap()
                            .update(FIXED_DT, &b.world, b.player.eye());
                    }
                }
                println!("HEDGE bench {mode}: {:?}", start.elapsed());
            }
            return Ok(());
        }
        if let Ok(spec) = std::env::var("LOOKING_GLASS_HEDGE3_PROBE") {
            return probe(&r, &spec);
        }
        let text = std::env::var("LOOKING_GLASS_HEDGE3_ROUTE")
            .ok()
            .map(std::fs::read_to_string)
            .transpose()?
            .unwrap_or_else(|| include_str!("route_steps.json").into());
        let steps: Vec<Step> = serde_json::from_str(&text)?;
        for (n, s) in steps.iter().enumerate() {
            let goal = Vec3::from_array(s.pos);
            println!(
                "HEDGE goal {n} {goal:?} {} from {:?} sanity {}",
                s.mode,
                r.player.feet,
                r.stats.sanity()
            );
            let result = match s.mode.as_str() {
                "staff" => {
                    ensure!(r.stats.copies(7) > 0, "Route does not own the Eye Staff");
                    r.heavy_weapon = Some(7);
                    Ok(())
                }
                "ice" => {
                    ensure!(r.stats.copies(4) > 0, "Route does not own the Ice Wand");
                    r.heavy_weapon = None;
                    r.ice_stream = true;
                    r.conserve_will = false;
                    Ok(())
                }
                "board" => board(r, s.frames, goal),
                "ride" => ride(r, goal),
                "phase" => wait_phase(r, goal),
                "nav" => r.navigate(goal),
                "clear" => r.clear(goal.x),
                "wait" | "input" | "inputjump" => {
                    let mut out = Ok(());
                    for tick in 0..s.frames {
                        out = r.tick(Controls {
                            wish: if s.mode == "wait" {
                                Vec2::ZERO
                            } else {
                                goal.truncate()
                            },
                            run: true,
                            jump: s.mode == "inputjump" && tick == 0,
                            ..Default::default()
                        });
                        if out.is_err() {
                            break;
                        }
                    }
                    out
                }
                _ => steer(r, goal, &s.mode),
            };
            std::fs::write(
                "private/hedge3-work/last-checkpoint.json",
                serde_json::to_vec(&r.checkpoint())?,
            )?;
            result?;
            if let Some(name) = &s.save {
                continuation(a, r, name)?;
            }
            ensure!(r.stats.alive(), "Hedge route died");
            if r.transition.is_some() {
                break;
            }
        }
        ensure!(
            r.transition == Some(("tower3".into(), Some("tower3_start1".into()))),
            "Hedge route has not reached tower3: {:?}",
            r.player.feet
        );
        ensure!(
            r.teleports == 0 && !r.stats.god && !r.stats.notarget && r.native_cast.is_some(),
            "Hedge route bypassed live play"
        );
        let _stats = serde_json::to_value(&r.stats)?;
        ensure!(
            r.audit.lava_ticks == 0 && r.audit.lost_ticks == 0,
            "Hedge route left the playable path"
        );
        ensure!(
            r.audit.transitions == 1,
            "Hedge route must take exactly one exit"
        );
        let mut metrics = r.metrics();
        metrics.sanity_in = initial_stats.sanity();
        metrics.will_in = initial_stats.will();
        let collected = r.stats.collected.difference(&initial_stats.collected);
        metrics.pickups = collected
            .clone()
            .filter(|key| !key.starts_with("drop:"))
            .count();
        metrics.loot = collected.filter(|key| key.starts_with("drop:")).count();
        println!("HEDGE completed {metrics}");
        std::fs::write(
            "private/hedge3-work/route-result.json",
            serde_json::to_vec_pretty(
                &serde_json::json!({"fresh_arrival":fresh,"metrics":metrics,"audit":r.audit}),
            )?,
        )?;
    Ok(())
}
