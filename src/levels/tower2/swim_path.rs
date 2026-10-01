//! Route-test navigation only: choose swim inputs through the loaded collision.
use super::*;
use std::collections::{BinaryHeap, HashMap};
pub(super) fn fan(r: &mut Route) -> Result<()> {
    // This is test-driver timing, like waiting for a visible blade gap. Predict
    // only ordinary upward swim acceleration; never alter the fan or Alice.
    for _ in 0..60 {
        r.tick(Controls::default())?;
    }
    for _ in 0..600 {
        let (model, base, angle) = {
            let t = owner(&mut r.interactions)?;
            let o = t.objects.iter().find(|o| o.name == "fan05").unwrap();
            (o.model, o.base, t.machine().fans[4])
        };
        let mut p = r.player.feet;
        let mut velocity = r.player.velocity;
        let mut clear = true;
        for n in 1..=110 {
            velocity += (Vec3::Z * 220. - velocity).clamp_length_max(700. * FIXED_DT);
            let next = p + velocity * FIXED_DT;
            let c = Collider::model(
                &r.map,
                model,
                base,
                Quat::from_rotation_z((angle + 100. * FIXED_DT * n as f32).to_radians()),
                true,
            )?;
            let hit = c.trace(
                p + crate::collision::PLAYER_CENTER,
                next + crate::collision::PLAYER_CENTER,
                crate::collision::PLAYER_HALF,
            );
            if hit.start_solid || hit.fraction < 1. {
                clear = false;
                break;
            }
            p = next;
        }
        if clear {
            println!(
                "TOWER taking fan gap at {angle} degrees from {:?}",
                r.player.feet
            );
            for _ in 0..110 {
                r.tick(Controls {
                    swim: Vec3::Z,
                    run: true,
                    ..Default::default()
                })?;
            }
            ensure!(
                r.player.feet.z > 280.,
                "Fan-gap ascent did not clear the blade plane"
            );
            return Ok(());
        }
        r.tick(Controls::default())?;
    }
    anyhow::bail!("No safe fan-gap timing at {:?}", r.player.feet)
}
pub(super) fn swim(r: &mut Route, goal: Vec3) -> Result<()> {
    const GRID: f32 = 32.;
    type Key = (i32, i32, i32);
    let at = |k: Key| vec3(k.0 as f32, k.1 as f32, k.2 as f32) * GRID;
    let key = |p: Vec3| {
        (
            (p.x / GRID).round() as i32,
            (p.y / GRID).round() as i32,
            (p.z / GRID).round() as i32,
        )
    };
    let clear = |a: Vec3, b: Vec3| {
        let t = r.world.body_trace(a, b);
        !t.start_solid && t.fraction >= 1.
    };
    let end = key(goal);
    let distance = |k: Key| (k.0 - end.0).abs() + (k.1 - end.1).abs() + (k.2 - end.2).abs();
    let start = key(r.player.feet);
    let mut queue = BinaryHeap::new();
    let mut parents = HashMap::new();
    let mut costs = HashMap::new();
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                let k = (start.0 + x, start.1 + y, start.2 + z);
                if clear(r.player.feet, at(k)) {
                    parents.insert(k, None);
                    costs.insert(k, 0);
                    queue.push((-distance(k), 0, k));
                }
            }
        }
    }
    let mut found = None;
    let mut attempts = 0;
    while let Some((_, neg, k)) = queue.pop() {
        let cost = -neg;
        if costs.get(&k).is_some_and(|c| *c < cost) {
            continue;
        }
        attempts += 1;
        ensure!(attempts < 250_000, "Pipe planner exhausted reachable water");
        if distance(k) <= 3 && clear(at(k), goal) {
            found = Some(k);
            break;
        }
        for (x, y, z) in [
            (1, 0, 0),
            (-1, 0, 0),
            (0, 1, 0),
            (0, -1, 0),
            (0, 0, 1),
            (0, 0, -1),
        ] {
            let n = (k.0 + x, k.1 + y, k.2 + z);
            if n.0 < 0
                || n.0 > 112
                || n.1 < -16
                || n.1 > 90
                || n.2 < -24
                || n.2 > 55
                || costs.get(&n).is_some_and(|c| *c <= cost + 1)
            {
                continue;
            }
            if r.world.liquid_at(at(n) + Vec3::Z * 20.) != 0 && clear(at(k), at(n)) {
                costs.insert(n, cost + 1);
                parents.insert(n, Some(k));
                queue.push((-(cost + 1 + distance(n)), -(cost + 1), n));
            }
        }
    }
    let mut k = found.context("No water-connected pipe route")?;
    let mut path = vec![goal];
    loop {
        path.push(at(k));
        if let Some(n) = parents[&k] {
            k = n;
        } else {
            break;
        }
    }
    path.reverse();
    let mut points = vec![];
    let mut current = r.player.feet;
    let mut cursor = 0;
    while cursor < path.len() {
        let mut last = cursor;
        while last + 1 < path.len() && clear(current, path[last + 1]) {
            last += 1;
        }
        current = path[last];
        points.push(current);
        cursor = last + 1;
    }
    println!("TOWER pipe path ({attempts} nodes): {points:?}");
    for p in points {
        super::steer(r, p, "swim")?;
    }
    Ok(())
}
