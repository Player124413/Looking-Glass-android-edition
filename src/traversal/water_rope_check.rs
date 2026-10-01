//! Authored rope endpoints and explicit grabs from the real water surface.
use super::*;
use crate::movement::{Controls, FLOAT_DEPTH};
use anyhow::ensure;

pub(super) fn check(assets: &mut crate::assets::Assets) -> Result<()> {
    let mut rows = Vec::new();
    for name in ["potears1", "garden1", "garden2"] {
        // Use the player's loaded world, including controller-owned water and
        // movers. Raw BSP water alone misses the raised Pool surface.
        let route = crate::route::Route::new(assets, name, None)?;
        let world = route.world;
        for rope in &world.traversal.ropes {
            let bottom = rope.anchor - Vec3::Z * rope.length;
            let strand = crate::rope::Strand::new(rope);
            ensure!(strand.points.last().unwrap().distance(bottom) < 0.001,
                "Visible rope endpoint differs from authored bounds");
            let mut samples = Vec::new();
            for offset in [Vec2::ZERO, Vec2::X * 24., -Vec2::X * 24.,
                Vec2::Y * 24., -Vec2::Y * 24., Vec2::X * 48., -Vec2::X * 48.,
                Vec2::Y * 48., -Vec2::Y * 48.] {
                let xy = bottom.truncate() + offset;
                let surface = (0..400).map(|i| rope.anchor.z - i as f32 * 4.)
                    .find(|&z| world.liquid_at(xy.extend(z)) & 32 != 0
                        && world.liquid_at(xy.extend(z + 4.)) & 32 == 0);
                let Some(mut surface) = surface else { continue; };
                for _ in 0..40 {
                    if world.liquid_at(xy.extend(surface + 0.1)) & 32 == 0 { break; }
                    surface += 0.1;
                }
                let feet = xy.extend(surface - FLOAT_DEPTH);
                if !world.body_clear(feet) { continue; }
                let mut p = Player::new(feet);
                p.immersion = crate::water::Immersion::sample(&world, feet);
                p.swimming = true;
                let available = world.traversal.nearest_rope(&world, p.eye(), 76.)
                    .is_some_and(|r| r.id == rope.id);
                p.tick(&world, Controls { use_pressed: true, rise: 1., ..Default::default() });
                let grabbed = p.rope.as_ref().is_some_and(|r| r.id == rope.id);
                let start = p.feet;
                if grabbed {
                    for _ in 0..120 {
                        p.tick(&world, Controls { rise: 1., ..Default::default() });
                        ensure!(world.body_clear(p.feet), "Water rope climb enters a wall");
                    }
                    p.validate_world(&world)?;
                }
                samples.push(serde_json::json!({"feet":feet.to_array(),"surface":surface,
                    "visible_reach":available,"grabbed":grabbed,"climbed":p.feet.z-start.z}));
            }
            println!("WATER ROPE {name} #{} bottom {} length {} samples {} grabs {}",
                rope.id.0,bottom.z,rope.length,samples.len(),
                samples.iter().filter(|s|s["grabbed"]==true).count());
            rows.push(serde_json::json!({"map":name,"entity":rope.id.0,
                "anchor":rope.anchor.to_array(),"bottom":bottom.to_array(),
                "length":rope.length,"samples":samples}));
        }
    }
    std::fs::create_dir_all("private/traversal-check")?;
    std::fs::write("private/traversal-check/water-ropes.json",serde_json::to_vec_pretty(&rows)?)?;
    Ok(())
}
