//! Dirt histories belong to one actor and one burrow transition, never a shared model.
use super::*;
use crate::{
    burrow::{Insect, Kind, Phase},
    clockwork::Rig,
};
pub(super) struct Art {
    dust: Option<crate::particles::Attached>,
    phase: Option<Phase>,
    time: f32,
}
impl Art {
    pub fn new(template: Option<&crate::particles::Attached>) -> Self {
        Self {
            dust: template.map(|t| t.fork()),
            phase: None,
            time: 0.,
        }
    }
    pub fn draw(&mut self, g: &Insect, data: &Data, camera: Vec3, atmosphere: &Atmosphere) {
        let Some(effect) = &mut self.dust else {
            return;
        };
        if self.phase != Some(g.phase) || g.time < self.time {
            *effect = effect.fork();
        }
        self.phase = Some(g.phase);
        self.time = g.time;
        if g.kind == Kind::Larva || !matches!(g.phase, Phase::Rise | Phase::Dive) {
            return;
        }
        let transform = Transform {
            translation: g.feet,
            rotation: Quat::from_rotation_z(g.yaw),
        };
        let clip = &data.clips[g.clip()];
        effect.draw(
            g.time,
            g.scale,
            |at, tag| {
                if let Some(tag) = tag {
                    let local = data.tag(g.clip(), at, tag);
                    Transform {
                        translation: transform.point(local.translation * g.scale),
                        rotation: transform.rotation * local.rotation,
                    }
                } else {
                    transform
                }
            },
            |name, at, default| {
                data.events
                    .visual(g.clip(), at, clip.duration(), clip.frame_time, false)
                    .emitters
                    .get(name)
                    .copied()
                    .unwrap_or(default)
            },
            camera,
            atmosphere,
        );
    }
}
