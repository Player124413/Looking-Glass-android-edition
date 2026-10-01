//! Continuous production movement; native checks include the real resident cast.
use super::*;
use crate::{movement::Controls, route::Route};
#[derive(Deserialize)]
struct Step {
    pos: [f32; 3],
    #[serde(default)]
    mode: String,
    #[serde(default)]
    frames: usize,
}
pub(super) fn check(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let skip = std::env::var_os("LOOKING_GLASS_JLAIR1_SKIP").is_some();
        walk(a, true, skip)
    })
}
pub(super) fn traversal(a: &mut Assets) -> Result<()> {
    walk(a, false, true)
}
pub(super) fn walk(a: &mut Assets, native: bool, skip: bool) -> Result<()> {
    std::fs::create_dir_all("private/jlair1-work")?;
    let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_JLAIR1_FROM") {
        Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?
    } else {
        let mut r = Route::new(a, "jlair1", Some("jlair1_start1"))?;
        if native {
            r.enable_native_cast(a)?;
        }
        r
    };
    r.skip_cinematics = skip;
    drive_inner(&mut r,native)?;
    let carried = r.stats.clone();
    let mut next = r.depart(a, true)?;
    ensure!(
        next.world.body_clear(next.player.feet)
            && next
                .interactions
                .levels
                .iter()
                .any(|s| s.ctl.id() == "jlair2"),
        "Missing Jabberwock encounter"
    );
    ensure!(
        (0..10).all(|i| next.stats.copies(i) == carried.copies(i))
            && next.stats.staff_component == carried.staff_component
            && next.stats.sanity() == carried.sanity()
            && next.stats.will() == carried.will(),
        "Carried state changed"
    );
    next.skip_cinematics = skip;
    next.wait_for_cinematic()?;
    next.wait(0.1)?;
    ensure!(
        !crate::level::targets(&next.interactions.levels).is_empty(),
        "Survival boss not active"
    );
    println!(
        "PASS ordinary transfer to Jabberwock survival at {:?}",
        next.player.feet
    );
    Ok(())
}
fn steer(r: &mut Route, goal: Vec3, jump: bool) -> Result<()> {
    let mut jumped = false;
    for _ in 0..120 * 16 {
        if r.interactions.scripted() {
            r.wait_for_cinematic()?;
        }
        let d = goal.truncate() - r.player.feet.truncate();
        if d.length() < 22. && (r.player.feet.z - goal.z).abs() < 70. && r.player.grounded {
            return Ok(());
        }
        let leap = jump && !jumped && r.player.grounded;
        jumped |= leap;
        r.tick(Controls {
            wish: d.normalize_or_zero(),
            run: true,
            jump: leap || r.player.ledge.as_ref().is_some_and(|h| !h.pulling),
            ..Default::default()
        })?;
        if r.transition.is_some() {
            return Ok(());
        }
    }
    anyhow::bail!("Route stuck toward {goal:?} at {:?}", r.player.feet)
}

fn drive_inner(r: &mut Route, native: bool) -> Result<()> {
    let skip = r.skip_cinematics;
    r.stop_at_exit = true;
    r.tactics = native;
    if let Ok(bounds) = std::env::var("LOOKING_GLASS_JLAIR1_PROBE") {
        let b: [i32; 6] = serde_json::from_str(&bounds)?;
        let mut points = vec![];
        for x in (b[0]..=b[1]).step_by(32) {
            for y in (b[2]..=b[3]).step_by(32) {
                let mut z = b[5] as f32;
                while z > b[4] as f32 {
                    let high = vec3(x as f32, y as f32, z);
                    let low = high - Vec3::Z * 16.;
                    if r.world.body_clear(high) {
                        let tr = r.world.body_trace(high, low);
                        if !tr.start_solid && tr.fraction < 1. && tr.normal.z >= 0.65 {
                            let feet = high.lerp(low, tr.fraction);
                            points.push(feet.to_array());
                        }
                    }
                    z -= 16.;
                }
            }
        }
        std::fs::write(
            "private/jlair1-work/floors.json",
            serde_json::to_vec(&points)?,
        )?;
        println!("PASS {} collision-supported floor points", points.len());
        return Ok(());
    }
    r.wait_for_cinematic()?;
    let steps: Vec<Step> = if let Ok(path) = std::env::var("LOOKING_GLASS_JLAIR1_ROUTE") {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else {
        serde_json::from_str(include_str!("route_steps.json"))?
    };
    for (k, s) in steps.iter().enumerate() {
        std::fs::write(
            "private/jlair1-work/last-checkpoint.json",
            serde_json::to_vec(&r.checkpoint())?,
        )?;
        let goal = Vec3::from_array(s.pos);
        println!(
            "CURIOSITY goal {k} {goal:?} {} from {:?} sanity {}",
            s.mode,
            r.player.feet,
            r.stats.sanity()
        );
        match s.mode.as_str() {
            "nav" => r.navigate(goal)?,
            "align" => {
                let mut reached = false;
                for _ in 0..1200 {
                    let delta = (goal - r.player.feet).truncate();
                    if delta.length() < 2. && r.player.velocity.truncate().length() < 5.
                        && r.player.grounded && (goal.z-r.player.feet.z).abs() < 8. {
                        reached = true; break;
                    }
                    r.tick(Controls { wish: (delta * (4. / 210.)).clamp_length_max(1.),
                        ..Default::default() })?;
                }
                ensure!(reached, "Could not line up at {goal:?}");
            }
            "clear" => r.clear(goal.x)?,
            "pickup" => {
                let e = r.map.entities.get(goal.x as usize).context("Missing pickup")?;
                ensure!(e.get("classname").is_some_and(|c| c.starts_with("Item_")), "Not a pickup");
                let at = e.get("origin").and_then(|s| crate::interaction::vector(s))
                    .context("Missing pickup origin")?;
                let floor = r.world.actor_footing(at + Vec3::Z * 32.,
                    crate::collision::PLAYER_CENTER, crate::collision::PLAYER_HALF, 256.)
                    .context("Unsupported pickup")?;
                r.navigate(floor)?;
            }
            "watch" => r.use_watch()?,
            "wait" => r.wait(goal.x)?,
            "input" | "inputjump" => {
                for n in 0..s.frames {
                    r.tick(Controls {
                        wish: goal.truncate(),
                        run: true,
                        jump: s.mode == "inputjump" && n == 0,
                        ..Default::default()
                    })?;
                }
            }
            _ => steer(r, goal, s.mode == "jump")?,
        }
        if r.interactions.scripted() {
            r.wait_for_cinematic()?;
        }
        std::fs::write(
            format!("private/jlair1-work/step-{k}.json"),
            serde_json::to_vec(&r.checkpoint())?,
        )?;
        if r.transition.is_some() {
            break;
        }
    }
    std::fs::write(
        "private/jlair1-work/end-checkpoint.json",
        serde_json::to_vec(&r.checkpoint())?,
    )?;
    ensure!(
        r.transition == Some(("jlair2".into(), Some("jlair2_start1".into()))),
        "Route not at exit: {:?}",
        r.player.feet
    );
    ensure!(
        check::owner(&r).saved.caterpillar
            && r.audit.lost_ticks == 0
            && r.stats.alive()
            && !r.stats.god
            && !r.stats.notarget
            && r.teleports == 0
            && r.audit.transitions == 1,
        "Route missed scene, used a cheat or required recovery"
    );
    ensure!(
        (!native
            || r.pickups.iter().any(
                |p| matches!(p.kind, crate::inventory::PickupKind::Weapon(5))
                    && r.stats.collected.contains(&p.id)
            ))
            && r.stats.copies(5) == 1,
        "Jacks altar was missed or added a duplicate toy"
    );
    if native {
        let cast = serde_json::to_value(
            r.native_cast
                .as_ref()
                .context("Missing native encounters")?
                .snapshot(),
        )?;
        std::fs::write(
            "private/jlair1-work/completed-cast.json",
            serde_json::to_vec_pretty(&cast)?,
        )?;
    }
    println!(
        "PASS Burning Curiosity continuous route native={native} skip={skip}: {}",
        r.metrics()
    );
    Ok(())
}

pub(crate) fn drive(r: &mut Route) -> Result<()> { std::fs::create_dir_all("private/jlair1-work")?; drive_inner(r,true) }
