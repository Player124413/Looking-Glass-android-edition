//! Native input traversal with short supported pathnode legs and a close escort.
use super::*;
use crate::{movement::Controls, route::Route};
fn state(r: &Route) -> &Maze {
    r.interactions
        .levels
        .iter()
        .find_map(|s| s.ctl.downcast_ref())
        .unwrap()
}
fn child(r: &Route) -> Option<CompanionContact> {
    r.native_cast
        .as_ref()?
        .companions()
        .into_iter()
        .find(|c| c.name == "seek_kid_chase" && c.visible)
}
pub async fn check(a: &mut Assets) -> Result<()> {
    std::fs::create_dir_all("private/hedge1")?;
    let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_MAZE_FROM") {
        Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?
    } else {
        let mut r = Route::new(a, "hedge1", Some("hedge1_start1"))?;
        r.enable_native_cast(a)?;
        r
    };
    drive(&mut r)?;
    let health = r.stats.sanity();
    let will = r.stats.will();
    let next = r.depart(a, true)?;
    ensure!(
        next.stats.sanity() == health
            && next.stats.will() == will
            && next.world.body_clear(next.player.feet),
        "Unsafe tower1 arrival"
    );
    save(&next, "arrival")?;
    println!("PASS Majestic Maze ordinary escort and physical exit to tower1");
    Ok(())
}
fn save(r: &Route, name: &str) -> Result<()> {
    ensure!(
        name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'),
        "Bad route label"
    );
    std::fs::write(
        format!("private/hedge1/{name}.json"),
        serde_json::to_vec(&r.checkpoint())?,
    )?;
    Ok(())
}
fn step(r: &mut Route, s: &serde_json::Value) -> Result<()> {
    if let Some(v) = s.get("go") {
        let at: [f32; 3] = serde_json::from_value(v.clone())?;
        go(r, Vec3::from_array(at), s["lead"] == true)?;
    } else if let Some(v) = s.get("steer") {
        let at: [f32; 3] = serde_json::from_value(v.clone())?;
        steer(r, Vec3::from_array(at), s["lead"] == true)?;
    } else if let Some(v) = s.get("wait") {
        r.wait(v.as_f64().unwrap() as f32)?;
    } else if s.get("watch_ready") == Some(&serde_json::Value::Bool(true)) {
        // Recharge by spending actual play time at a chosen safe waypoint.
        r.wait(r.stats.powers.recharge + 0.1)?;
    } else if s.get("watch") == Some(&serde_json::Value::Bool(true)) {
        r.use_watch()?;
    } else if let Some(v) = s.get("clear") {
        r.clear(v.as_f64().unwrap() as f32)?;
    } else if s.get("portal") == Some(&serde_json::Value::Bool(true)) {
        go(r, vec3(3136., 4288., 1920.), false)?;
        let before = r.teleports;
        steer(r, vec3(3136., 4480., 1936.), false)?;
        ensure!(r.teleports == before + 1, "Portal did not fire");
    } else if let Some(v) = s.get("save") {
        save(r, v.as_str().unwrap())?;
    } else if let Some(v) = s.get("assert") {
        match v.as_str().unwrap() {
            "prelude" => ensure!(
                state(r).saved.doors[1] == 1. && !state(r).saved.following,
                "Prelude did not demonstrate plate"
            ),
            "following" => ensure!(
                child(r).is_some() && state(r).saved.doors == [0.; 3],
                "Chase handoff/gate closure missing"
            ),
            "holding" => ensure!(state(r).ready(), "Child did not hold plate"),
            _ => anyhow::bail!("Unknown assertion"),
        }
    } else {
        anyhow::bail!("Unknown maze input")
    };
    Ok(())
}
fn steer(r: &mut Route, goal: Vec3, lead: bool) -> Result<()> {
    let mut last = r.player.feet;
    let teleports = r.teleports;
    let mut recovery = 0;
    for tick in 0..120 * 35 {
        let d = goal - r.player.feet;
        let c = child(r);
        let reached = d.truncate().length() < 18. && d.z.abs() < 40.;
        if reached
            && (!lead
                || c.as_ref().is_none_or(|c| {
                    c.holding || c.feet.truncate().distance(r.player.feet.truncate()) < 120.
                }))
        {
            return Ok(());
        }
        let wait = lead
            && c.as_ref().is_some_and(|c| {
                !c.holding
                    && c.feet.truncate().distance(r.player.feet.truncate()) > 150.
                    // Combat may put the child ahead around a corner. Move
                    // toward him rather than waiting outside his line of sight.
                    && (c.feet - r.player.feet).truncate().dot(d.truncate()) <= 0.
            });
        let stuck = tick % 120 == 119 && r.player.feet.distance(last) < 8.;
        if lead && stuck && recovery < 3 {
            if let Some(c) = c.as_ref().filter(|c| {
                !c.holding && c.feet.truncate().distance(r.player.feet.truncate()) > 100.
            }) {
                // Regain contact after a fight or an occluded corner through
                // normal movement. Never relocate the child or latch his plate.
                recovery += 1;
                println!("MAZE regain escort contact at {:?}", c.feet);
                go(r, c.feet, false)?;
                last = r.player.feet;
                continue;
            }
        }
        if tick % 120 == 119 {
            last = r.player.feet;
        }
        r.tick(Controls {
            wish: if wait || reached {
                Vec2::ZERO
            } else {
                d.truncate().normalize_or_zero()
            },
            run: !lead,
            jump: !lead && stuck,
            ..Default::default()
        })?;
        let leaf = state(r).saved.doors[1];
        if leaf > 0. && leaf < 1. {
            save(r, "mid-slide")?;
        }
        if r.transition.is_some() || r.teleports != teleports {
            return Ok(());
        }
    }
    anyhow::bail!(
        "Maze leg {goal:?} blocked at {:?}; child {:?}",
        r.player.feet,
        child(r).map(|c| c.feet)
    )
}
fn nodes(r: &Route) -> Vec<Vec3> {
    r.map
        .entities
        .iter()
        .filter(|e| {
            e.get("classname")
                .is_some_and(|s| matches!(s.as_str(), "info_pathnode" | "info_waypoint"))
        })
        .filter_map(|e| e.get("origin").and_then(|v| crate::interaction::vector(v)))
        .filter_map(|p| {
            r.world
                .actor_footing(p + Vec3::Z * 24., PLAYER_CENTER, PLAYER_HALF, 128.)
        })
        .filter(|p| p.z > 1500. && p.y < 5800.)
        .collect()
}
fn edge(w: &World, a: Vec3, b: Vec3) -> bool {
    let distance = a.distance(b);
    if distance > 500. || (a.z - b.z).abs() > 240. {
        return false;
    }
    // Curved ramps cannot be certified by a straight chord between their endpoints.
    // Follow supported short segments, with bounded rises and swept player clearance.
    let steps = (distance / 20.).ceil().max(1.) as usize;
    let mut previous = a;
    for i in 1..=steps {
        let sample = a.lerp(b, i as f32 / steps as f32);
        let Some(foot) = w.actor_footing(sample + Vec3::Z * 64., PLAYER_CENTER, PLAYER_HALF, 128.)
        else {
            return false;
        };
        if (foot.z - previous.z).abs() > 28. {
            return false;
        }
        let tr = w.body_trace(previous + Vec3::Z * 18., foot + Vec3::Z * 18.);
        if tr.start_solid || tr.fraction < 1. {
            return false;
        }
        previous = foot;
    }
    previous.distance(b) < 12.
}
fn go(r: &mut Route, goal: Vec3, lead: bool) -> Result<()> {
    let path = plan(r, goal)?;
    println!("MAZE planned {} legs: {:?}", path.len(), path);
    for p in path {
        let before = r.teleports;
        steer(r, p, lead)?;
        if r.transition.is_some() || r.teleports != before {
            break;
        }
    }
    Ok(())
}
pub(super) fn graph_check(a: &mut Assets) -> Result<()> {
    let r = Route::new(a, "hedge1", Some("hedge1_start1"))?;
    plan(&r, vec3(3968., 4288., 1920.))?;
    Ok(())
}
fn plan(r: &Route, goal: Vec3) -> Result<Vec<Vec3>> {
    let mut points = vec![r.player.feet];
    points.extend(nodes(r));
    let goal = r
        .world
        .actor_footing(goal + Vec3::Z * 24., PLAYER_CENTER, PLAYER_HALF, 128.)
        .unwrap_or(goal);
    points.push(goal);
    let end = points.len() - 1;
    let mut costs = vec![f32::INFINITY; points.len()];
    let mut parent = vec![usize::MAX; points.len()];
    let mut done = vec![false; points.len()];
    costs[0] = 0.;
    loop {
        let at = (0..points.len())
            .filter(|i| !done[*i])
            .min_by(|a, b| costs[*a].total_cmp(&costs[*b]))
            .unwrap();
        if !costs[at].is_finite() {
            let mut edges = vec![];
            for i in 0..points.len() {
                for j in i + 1..points.len() {
                    if edge(&r.world, points[i], points[j]) {
                        edges.push([i, j]);
                    }
                }
            }
            std::fs::write(
                "private/hedge1/graph.json",
                serde_json::to_vec(
                    &serde_json::json!({"points":points.iter().map(|p|p.to_array()).collect::<Vec<_>>(),"reachable":done,"edges":edges}),
                )?,
            )?;
            anyhow::bail!(
                "No supported node route from {:?} to {goal:?}",
                r.player.feet
            );
        }
        if at == end {
            break;
        }
        done[at] = true;
        for j in 0..points.len() {
            if !done[j] && edge(&r.world, points[at], points[j]) {
                let cost = costs[at] + points[at].distance(points[j]);
                if cost < costs[j] {
                    costs[j] = cost;
                    parent[j] = at;
                }
            }
        }
    }
    let mut path = vec![];
    let mut at = end;
    while at != 0 {
        path.push(points[at]);
        at = parent[at];
    }
    path.reverse();
    Ok(path)
}

pub(crate) fn drive(r: &mut Route) -> Result<()> {
    std::fs::create_dir_all("private/hedge1")?;
    r.tactics = true;
    r.conserve_will = false;
    r.ice_stream = true;
    r.heavy_weapon = Some(7);
    r.stop_at_exit = true;
    save(&r, "entry")?;
    if std::env::var_os("LOOKING_GLASS_MAZE_NODES").is_some() {
        let nodes = nodes(&r);
        std::fs::write(
            "private/hedge1/nodes.json",
            serde_json::to_vec(&nodes.iter().map(|p| p.to_array()).collect::<Vec<_>>())?,
        )?;
        println!("{} supported nodes", nodes.len());
        return Ok(());
    }
    let text = std::env::var("LOOKING_GLASS_MAZE_INPUT")
        .ok()
        .map(std::fs::read_to_string)
        .transpose()?
        .unwrap_or_else(|| include!("route_input.rs").into());
    let steps: Vec<serde_json::Value> = serde_json::from_str(&text)?;
    for (n, s) in steps.iter().enumerate() {
        println!(
            "MAZE {n} {s} from {:?} health {} child {:?} gates {:?}",
            r.player.feet,
            r.stats.sanity(),
            child(&r).map(|c| (c.feet, c.holding)),
            state(&r).saved.doors
        );
        let result = step(r, s);
        save(&r, "last")?;
        result?;
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition == Some(EXIT.destination()),
        "Maze route incomplete at {:?}",
        r.player.feet
    );
    ensure!(
        state(&r).ready() && state(&r).saved.end_seen && state(&r).saved.start_seen,
        "Escort progression incomplete"
    );
    r.assert_clean(1, 0)?;
    ensure!(
        !r.stats.god && !r.stats.notarget && r.stats.minimum_sanity == 0.,
        "Route assistance enabled"
    );
    ensure!(
        serde_json::to_value(r.interactions.snapshot())?["triggers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["reported"] == false),
        "Pending maze thread"
    );
    println!("Maze metrics {}", r.metrics());
    save(&r, "exit")?;
    std::fs::write(
        "private/hedge1/report.json",
        serde_json::to_vec_pretty(
            &serde_json::json!({"metrics":r.metrics(),"audit":r.audit,"exit":r.transition}),
        )?,
    )?;
    Ok(())
}
