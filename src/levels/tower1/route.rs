//! Ordinary movement and funded combat; no route teleports or resource grants.
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
    Box::pin(async move { run(a, false, false) })
}
pub(super) fn skipped(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move { run(a, false, true) })
}
pub(super) fn probe(a: &mut Assets) -> Result<()> {
    run(a, true, true)
}
fn run(a: &mut Assets, probe: bool, skip: bool) -> Result<()> {
    std::fs::create_dir_all("private/tower1-work")?;
    let mut r = if let Ok(f) = std::env::var("LOOKING_GLASS_TOWER1_FROM") {
        Route::resume(a, &serde_json::from_slice(&std::fs::read(f)?)?)?
    } else {
        let mut r = Route::new(a, "tower1", Some("tower1_start1"))?;
        if !probe {
            r.enable_native_cast(a)?;
        }
        r
    };
    r.skip_cinematics = skip;
    drive_inner(&mut r, probe)?;
    let stats = serde_json::to_value(&r.stats)?;
    std::fs::write(
        format!(
            "private/tower1-work/route-{}.json",
            if skip { "skipped" } else { "watched" }
        ),
        serde_json::to_vec_pretty(&serde_json::json!({"metrics":r.metrics(),"audit":r.audit}))?,
    )?;
    let next = r.depart(a, true)?;
    ensure!(
        next.level().map == "hedge2"
            && next.world.body_clear(next.player.feet)
            && serde_json::to_value(&next.stats)? == stats,
        "Hedge2 handoff changed resources or blocked Alice"
    );
    println!("PASS Tower1 full live route into Hedge2 with exact carried resources");
    Ok(())
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
                && (mode == "hop"
                    || matches!(mode, "jump" | "air" | "precise") && (!jumped || d.z > 24.)));
        jumped |= jump;
        let input = Controls {
            wish: if mode == "precise" || mode == "air" {
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
                "  TOWER1 {:?} velocity {:?} grounded {} sanity {}",
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
            ensure!(stalled < 12, "Tower stalled toward {goal:?}");
        }
    }
    anyhow::bail!(
        "Tower traversal timeout at {:?} toward {goal:?}",
        r.player.feet
    )
}
fn floors(r: &Route, spec: &str) -> Result<()> {
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
        "private/tower1-work/floors.json",
        serde_json::to_vec(&points)?,
    )?;
    println!("TOWER1 probe {} floor samples", points.len());
    Ok(())
}

fn drive_inner(r: &mut Route, probe: bool) -> Result<()> {
    r.tactics = true;
    r.stop_at_exit = true;
    r.wait_for_cinematic()?;
    if let Ok(s) = std::env::var("LOOKING_GLASS_TOWER1_PROBE") {
        return floors(&r, &s);
    }
    let data = std::env::var("LOOKING_GLASS_TOWER1_ROUTE")
        .ok()
        .map(std::fs::read)
        .transpose()?
        .unwrap_or_else(|| include_bytes!("route_steps.json").to_vec());
    let steps: Vec<Step> = serde_json::from_slice(&data)?;
    for (n, s) in steps.iter().enumerate() {
        let goal = Vec3::from_array(s.pos);
        println!(
            "TOWER1 step{n} {} {goal:?} from {:?} health {}",
            s.mode,
            r.player.feet,
            r.stats.sanity()
        );
        let result = (|| -> Result<()> {
            match s.mode.as_str() {
                "phase" => {
                    let hold = r.player.feet.truncate();
                    let mut ready = false;
                    for _ in 0..120 * 15 {
                        let t = owner(&mut r.interactions)?.saved.faces[s.frames - 1];
                        if t >= goal.x as f64 && t < goal.y as f64 {
                            ready = true;
                            break;
                        }
                        let wish = ((hold - r.player.feet.truncate()) * 3.
                            - r.player.velocity.truncate() * 0.7)
                            .clamp_length_max(320.)
                            / 320.;
                        r.tick(Controls {
                            wish,
                            run: true,
                            ..Default::default()
                        })?;
                    }
                    ensure!(ready, "Face idle window not reached");
                }
                "nav" => r.navigate(goal)?,
                "xy" => r.walk_xy(goal.truncate())?,
                "clear" => r.clear(goal.x)?,
                "wait" | "input" | "inputjump" => {
                    for k in 0..s.frames {
                        r.tick(Controls {
                            wish: if s.mode == "wait" {
                                Vec2::ZERO
                            } else {
                                goal.truncate()
                            },
                            run: true,
                            jump: s.mode == "inputjump" && k == 0,
                            ..Default::default()
                        })?;
                    }
                }
                _ => steer(r, goal, &s.mode)?,
            }
            Ok(())
        })();
        std::fs::write(
            "private/tower1-work/last-checkpoint.json",
            serde_json::to_vec(&r.checkpoint())?,
        )?;
        result?;
        if let Some(name) = &s.save {
            std::fs::write(
                format!("private/tower1-work/route-{name}.json"),
                serde_json::to_vec(&r.checkpoint())?,
            )?;
        }
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition == Some(("hedge2".into(), Some("hedge2_start1".into()))),
        "Tower course incomplete at {:?}",
        r.player.feet
    );
    ensure!(
        !probe && r.native_cast.is_some(),
        "Movement diagnostic only; native cast required for route proof"
    );
    ensure!(
        r.stats.alive()
            && !r.stats.god
            && !r.stats.notarget
            && r.teleports == 0
            && r.audit.lost_ticks == 0
            && r.audit.transitions == 1,
        "Invalid route bypass"
    );
    Ok(())
}

pub(crate) fn drive(r: &mut Route) -> Result<()> { std::fs::create_dir_all("private/tower1-work")?; drive_inner(r, false) }
