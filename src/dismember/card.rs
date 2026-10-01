//! Original Spade torso recipe, sharing bounded piece physics.
use super::*;
/// The falling top clip extends below the standing feet. Lift its conservative
/// sweep box clear of the floor before releasing it; otherwise every floor
/// rejects the fragment even though the initial upper-body pose is clear.
pub fn birth_lift(recipe: &Recipe) -> f32 {
    (-recipe.min.z + 1.).max(0.)
}
pub fn recipe(assets: &mut crate::assets::Assets) -> Result<Recipe> {
    use crate::{
        animation_events::{Command, When},
        skeletal::{Animation, Definition, Skeleton},
    };
    let def = Definition::load(assets, "models/cardguard_spade.tik")?;
    let skeleton = Skeleton::parse(&assets.read(&format!("{}/{}", def.path, def.model))?)?;
    let events = crate::animation_events::Model::load(assets, "models/cardguard_spade.tik")?;
    let death = Animation::parse(
        &assets.read(&format!("{}/{}", def.path, def.animations["death_gib"]))?,
        skeleton.bones.len(),
    )?;
    let (frame, cap, name, pattern) = events.clips["death_gib"]
        .iter()
        .find_map(|e| match (&e.when, &e.command) {
            (
                When::Frame(n),
                Command::Gib {
                    cap,
                    animation,
                    surfaces,
                    ..
                },
            ) => Some((*n, cap, animation, surfaces)),
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("Missing Spade Guard cut recipe"))?;
    let top = Animation::parse(
        &assets.read(&format!("{}/{}", def.path, def.animations[name]))?,
        skeleton.bones.len(),
    )?;
    let (mut min, mut max) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
    // Include intermediate rotation samples, and padding. The conservative box
    // contains the complete authored fall instead of only the upright torso.
    for i in 0..top.frames.len() * 4 {
        let pose = skeleton.global_pose(&top.sample(i as f32 * top.frame_time / 4., false));
        for v in skeleton
            .surfaces
            .iter()
            .filter(|s| s.name == *cap || crate::animation_events::matches(pattern, &s.name))
            .flat_map(|s| &s.vertices)
        {
            let p = v.position(&pose) * def.scale;
            min = min.min(p);
            max = max.max(p);
        }
    }
    ensure!(min.is_finite() && max.is_finite(), "Empty sever surfaces");
    let recipe = Recipe {
        split: frame as f32 * death.frame_time,
        duration: death.duration(),
        frame_time: death.frame_time,
        events,
        min: min - Vec3::ONE,
        max: max + Vec3::ONE,
    };
    Ok(recipe)
}
