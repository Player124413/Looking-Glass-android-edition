//! Route-test navigation only: choose swim inputs through the loaded collision.
use super::*;
use std::collections::{BinaryHeap, HashMap};
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
            if n.0 < -48
                || n.0 > 90
                || n.1 < -24
                || n.1 > 96
                || n.2 < 6
                || n.2 > 68
                || costs.get(&n).is_some_and(|c| *c <= cost + 1)
            {
                continue;
            }
            if r.world.liquid_at(at(n) + Vec3::Z * 52.) != 0 && clear(at(k), at(n)) {
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
    println!("HEDGE swim path ({attempts} nodes): {points:?}");
    for p in points {
        super::steer(r, p, "swim")?;
    }
    Ok(())
}
