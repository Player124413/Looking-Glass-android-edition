//! Focused collision regression at the second ride's bank; not route completion evidence.
use super::*;
use crate::{assets::Assets, movement::FIXED_DT};

pub fn check(a: &mut Assets, map: &Bsp) -> Result<()> {
    edge_rider(a, map)?;
    for confined in [false, true] {
        let mut pool = Pool::load(a, map)?;
        pool.state.age = 289.23477;
        pool.state.talking = true;
        pool.state.talked = true;
        pool.state.drops[1] = Some(250.);
        pool.state.rides[1] = Some(274.22177);
        pool.state.trains[1] = Some((1, 281.77826));
        pool.rebuild(map)?;
        let mut world = World::from_bsp(map)?;
        let feet = vec3(1521.5931, -2250.5078, 1316.0774);
        let mut player =
            Player::spawn(&world, feet + Vec3::Z * 48.).context("Blocked-leaf fixture spawn")?;
        player.feet = feet;
        let mut fixed = Vec::new();
        if confined {
            for axis in [Vec3::X, Vec3::Y] {
                let width = if axis == Vec3::X {
                    vec3(1., 100., 100.)
                } else {
                    vec3(100., 1., 100.)
                };
                for sign in [-1., 1.] {
                    let center = feet + Vec3::Z * 40. + axis * sign * 16.1;
                    fixed.push(Collider::box_bounds(center - width, center + width));
                }
            }
        }
        world.set_dynamic(fixed.iter().cloned().chain(pool.colliders()).collect());
        ensure!(
            world.body_clear(player.feet),
            "Blocked-leaf fixture starts obstructed"
        );
        let before = serde_json::to_value(pool.snapshot())?;
        pool.advance(0., map, &mut world, &mut player, &fixed)?;
        ensure!(
            before == serde_json::to_value(pool.snapshot())? && player.feet == feet,
            "Paused collision moved"
        );
        let age = pool.state.age;
        for _ in 0..240 {
            pool.advance(FIXED_DT, map, &mut world, &mut player, &fixed)?;
            ensure!(world.body_clear(player.feet), "Leaf embedded swimmer");
        }
        ensure!(pool.state.age > age + 1.99, "Blocked leaf froze the visit");
        if confined {
            ensure!(
                pool.state.blocked_time.values().any(|t| *t > 0.),
                "Confined mover did not wait"
            );
            ensure!(
                player.feet.distance(feet) < 0.1,
                "Mover pushed through enclosure"
            );
        } else {
            ensure!(
                player.feet.distance(feet) > 0.1,
                "Leaf did not push swimmer clear"
            );
        }
        let mut restored = Pool::load(a, map)?;
        restored.restore(
            &serde_json::from_slice(&serde_json::to_vec(&pool.snapshot())?)?,
            map,
        )?;
        let mut other = player.clone();
        let mut other_world = World::from_bsp(map)?;
        for _ in 0..120 {
            pool.advance(FIXED_DT, map, &mut world, &mut player, &fixed)?;
            restored.advance(FIXED_DT, map, &mut other_world, &mut other, &fixed)?;
        }
        ensure!(
            serde_json::to_value(pool.snapshot())? == serde_json::to_value(restored.snapshot())?
                && player.feet == other.feet,
            "Blocked mover restore diverged"
        );
        ensure!(
            pool.objects.len() == 15 && pool.state.blocked_time.len() <= 15,
            "Mover state grew"
        );
        println!("PASS Pool swimmer collision confined={confined}, global clock, pause, bounded delays and restore");
    }
    Ok(())
}

fn edge_rider(a: &mut Assets, map: &Bsp) -> Result<()> {
    for hz in [30, 60, 144] {
        let mut pool = Pool::load(a, map)?;
        pool.state.age = 397.0751;
        pool.state.talking = true;
        pool.state.talked = true;
        pool.state.rides[2] = Some(369.0736);
        pool.rebuild(map)?;
        let leaf = pool.objects.iter().find(|o| o.name == "rideleaf3obj").unwrap();
        let previous = leaf.pose;
        let m = &map.models[leaf.model];
        let center = previous.0 + previous.1 * ((m.min + m.max) * 0.5);
        let mut world = World::from_bsp(map)?;
        world.set_dynamic(pool.colliders().collect());
        let static_world = World::from_bsp(map)?;
        let mut edge = None;
        'scan: for x in (-176..=176).step_by(8) {
            for y in (-176..=176).step_by(8) {
                let above = center + vec3(x as f32, y as f32, 160.);
                let below = above - Vec3::Z * 320.;
                let hit = leaf.collider.trace(above + PLAYER_CENTER, below + PLAYER_CENTER, PLAYER_HALF);
                if hit.start_solid || hit.fraction >= 1. || hit.normal.z < 0.65 { continue; }
                let feet = above.lerp(below, hit.fraction);
                let thin = leaf.collider.trace(feet + PLAYER_CENTER,
                    feet + PLAYER_CENTER - Vec3::Z * 3., vec3(1., 1., PLAYER_HALF.z));
                if thin.fraction >= 1. && world.body_clear(feet)
                    && static_world.body_trace(feet, feet - Vec3::Z * 3.).fraction >= 1. {
                    edge = Some(feet);
                    break 'scan;
                }
            }
        }
        let feet = edge.context("Missing full-footprint leaf edge fixture")?;
        let mut player = Player::new(feet);
        player.grounded = true;
        pool.advance(1. / hz as f32, map, &mut world, &mut player, &[])?;
        let next = pool.objects.iter().find(|o| o.name == "rideleaf3obj").unwrap();
        let carried = next.pose.0 + next.pose.1 * previous.1.inverse() * (feet - previous.0);
        let expected = next.collider.rider_feet(carried).map_or(carried, |p| p.0);
        ensure!(player.feet.distance(expected) < 0.05,
            "Leaf lost edge rider at {hz} Hz: {feet:?} -> {:?}, expected {expected:?}", player.feet);
        ensure!(world.body_clear(player.feet), "Edge rider embedded after carry");
    }
    println!("PASS Pool leaf full-footprint edge carrying at 30/60/144 Hz");
    Ok(())
}
