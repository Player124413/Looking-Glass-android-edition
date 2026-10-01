//! Item particles use authored tags and clip events, independently of item grants.
use crate::{assets::Assets, environment::Atmosphere, skeletal::Transform, texture};
use anyhow::Result;
use macroquad::prelude::*;
use std::collections::BTreeMap;

pub(crate) struct Effect {
    model: crate::tan::Model,
    scale: f32,
    events: crate::animation_events::Model,
    attached: Option<crate::particles::Attached>,
    bursts: Option<crate::particles::Attached>,
    clip: &'static str,
    duration: f32,
}
impl Effect {
    pub fn load(
        a: &mut Assets,
        name: &str,
        clip: &'static str,
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let (def, model) = crate::weapons::read_model_clip(a, name, Some(clip))?;
        let frames = model
            .surfaces
            .first()
            .map(|s| s.frames.len())
            .or_else(|| model.tags.values().next().map(Vec::len))
            .unwrap_or(1);
        let duration = frames as f32 * model.frame_time;
        let mut effect = Self {
            attached: crate::particles::Attached::load(a, name, specs)?,
            bursts: crate::particles::Attached::load_clip_bursts(
                a,
                name,
                Some(clip),
                model.frame_time,
                specs,
            )?,
            events: crate::animation_events::Model::load(a, &format!("models/{name}.tik"))?,
            model,
            scale: def.scale,
            clip,
            duration,
        };
        for p in [&mut effect.attached, &mut effect.bursts]
            .into_iter()
            .flatten()
        {
            p.use_sprite_dimensions();
        }
        Ok(effect)
    }
    pub fn draw(&self, age: f32, origin: Vec3, spin: f32, camera: Vec3, atmosphere: &Atmosphere) {
        let looping = self.clip == "idle";
        let pose = |t: f32, tag: Option<&str>| {
            let rotation = Quat::from_rotation_z(t * spin);
            let Some(points) = tag.and_then(|tag| self.model.tags.get(tag)) else {
                return Transform {
                    translation: origin,
                    rotation,
                };
            };
            let frame = t.max(0.) / self.model.frame_time;
            let frame = if looping {
                frame % points.len() as f32
            } else {
                frame.min((points.len() - 1) as f32)
            };
            let a = frame as usize;
            let b = if looping {
                (a + 1) % points.len()
            } else {
                (a + 1).min(points.len() - 1)
            };
            let axes = &self.model.tag_rotations[tag.unwrap()];
            Transform {
                translation: origin
                    + rotation * (points[a].lerp(points[b], frame.fract()) * self.scale),
                rotation: rotation * axes[a].slerp(axes[b], frame.fract()),
            }
        };
        // A template can draw many items in one frame. Never reuse another
        // item's birth positions; reconstruction also makes paused frames stable.
        for template in [&self.attached, &self.bursts].into_iter().flatten() {
            template.fork().draw(
                age,
                1.,
                pose,
                |name, t, initial| {
                    self.events
                        .visual(self.clip, t, self.duration, self.model.frame_time, looping)
                        .emitters
                        .get(name)
                        .copied()
                        .unwrap_or(initial)
                },
                camera,
                atmosphere,
            );
        }
    }
}
