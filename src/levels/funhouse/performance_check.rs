//! Check cached machinery against the original collision builder and time both.
use super::*;
use std::time::Instant;

pub fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/funhouse.bsp")?)?;
    let mut owner = Funhouse::load(a, &map)?;
    let mut comparisons = 0;
    for time in [0.125, 1.5, 7.25] {
        owner.saved.time = time;
        owner.rebuild(&map)?;
        for o in &owner.objects {
            let old = Collider::model(&map, o.model, o.pose.translation, o.pose.rotation, true)?;
            let m = &map.models[o.model];
            for x in [-0.8, 0., 0.8] {
                for y in [-0.8, 0., 0.8] {
                    let mid = o.pose.point((m.min + m.max) * 0.5
                        + (m.max - m.min) * vec3(x, y, 0.) * 0.5);
                    let start = mid + Vec3::Z * 1024.;
                    let end = mid - Vec3::Z * 1024.;
                    for half in [Vec3::ZERO, PLAYER_HALF] {
                        let p = old.trace(start, end, half);
                        let q = o.collider.trace(start, end, half);
                        ensure!(p.start_solid == q.start_solid && (p.fraction-q.fraction).abs() < 0.0001,
                            "Cached collider changed model {} at {time}: {p:?} / {q:?}", o.model);
                        comparisons += 1;
                    }
                }
            }
        }
    }
    // The constantly turning machinery, with the exact same changing poses.
    let mut poses = Vec::new();
    for _ in 0..30 {
        let before: Vec<_> = owner.objects.iter().map(|o| o.pose).collect();
        owner.saved.time += 1. / 60.;
        owner.rebuild(&map)?;
        poses.extend(owner.objects.iter().zip(before).filter(|(o, p)|
            o.pose.translation != p.translation || o.pose.rotation != p.rotation)
            .map(|(o, _)| (o.model, o.pose)));
    }
    let start = Instant::now();
    for &(model, pose) in &poses {
        std::hint::black_box(Collider::model(&map, model, pose.translation, pose.rotation, true)?);
    }
    let legacy = start.elapsed().as_secs_f64() * 1000. / 30.;
    let start = Instant::now();
    for _ in 0..300 {
        owner.saved.time += 1. / 60.;
        owner.rebuild(&map)?;
    }
    let cached = start.elapsed().as_secs_f64() * 1000. / 300.;
    println!("PASS Mirror Image: {comparisons} collision comparisons; old changing-shape construction {legacy:.3} ms/frame; complete cached rebuild {cached:.3} ms/frame (30/300 samples)");
    Ok(())
}
