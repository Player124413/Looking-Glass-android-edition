//! Per-actor emitter history; no shared-model clocks or render-driven damage.
use super::*;
use crate::{
    clockwork::Rig,
    wildlife::{Creature, Phase},
};
pub(super) struct Art {
    effect: Option<crate::particles::Attached>,
    templates: BTreeMap<String, crate::particles::Attached>,
    attachments: BTreeMap<String, crate::particles::Attached>,
    phase: Option<Phase>,
    time: f32,
}
impl Art {
    pub fn new(model: &Model) -> Self {
        Self {
            effect: model.own_effect.as_ref().map(|t| t.fork()),
            templates: model
                .effects
                .iter()
                .map(|(n, e)| (n.clone(), e.fork()))
                .collect(),
            attachments: BTreeMap::new(),
            phase: None,
            time: 0.,
        }
    }
    pub fn draw(&mut self, g: &Creature, model: &Model, camera: Vec3, atmosphere: &Atmosphere, beams: &attack_fx::Art) {
        let data = &model.data;
        if let Some(anchor) = g.web {
            gl_use_default_material();
            draw_line_3d(g.target(0).center, anchor, Color::new(0.8, 0.8, 0.8, 0.6));
        }
        if let Some(end) = g.beam {
            gl_use_default_material();
            let from = if g.kind.jabber() {
                g.feet
                    + Quat::from_rotation_z(g.yaw)
                        * data.tag(g.clip(), g.time, "tag_beam").translation
                        * g.scale
            } else {
                g.target(0).center
            };
            if g.kind.jabber() {
                beams.lightning(from, end, camera, (g.time-data.clips[g.clip()].frame_time).max(0.), 1000., false, atmosphere);
            } else {
                draw_line_3d(from, end, Color::new(0.45, 0.75, 1., 0.7));
            }
        }
        if self.phase != Some(g.phase) || g.time < self.time {
            if let Some(effect) = &mut self.effect {
                *effect = effect.fork();
            }
            self.attachments.clear();
        }
        self.phase = Some(g.phase);
        self.time = g.time;
        if g.frozen || g.visual_scale(data) <= 0. {
            return;
        }
        let transform = Transform {
            translation: g.feet,
            rotation: Quat::from_rotation_z(g.yaw),
        };
        let clip = &data.clips[g.clip()];
        let visual = data.events.visual(
            g.clip(),
            g.time,
            clip.duration(),
            clip.frame_time,
            g.loops(),
        );
        self.attachments
            .retain(|tag, _| visual.attachments.contains_key(tag));
        for (tag, a) in visual.attachments.iter().take(8) {
            let Some(template) = self.templates.get(&a.model) else {
                continue;
            };
            let effect = self
                .attachments
                .entry(tag.clone())
                .or_insert_with(|| template.fork());
            let born = g.time - a.age;
            effect.draw(
                a.age,
                g.scale * a.scale,
                |at, child_tag| {
                    let local = data.tag(g.clip(), born + at, tag);
                    let mut pose = Transform {
                        translation: transform.point(local.translation * g.scale),
                        rotation: transform.rotation * local.rotation,
                    };
                    if let (Some(prop), Some(child_tag)) = (model.props.get(&a.model), child_tag) {
                        pose.translation = prop.point(pose, child_tag, g.scale * a.scale);
                    }
                    pose
                },
                |_, _, default| default,
                camera,
                atmosphere,
            );
        }
        let Some(effect) = &mut self.effect else {
            return;
        };
        effect.draw(
            g.time,
            g.scale,
            |at, tag| {
                if let Some(tag) = tag {
                    let local = data.tag(
                        g.clip(),
                        if g.loops() {
                            at.rem_euclid(clip.duration())
                        } else {
                            at
                        },
                        tag,
                    );
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
                    .visual(g.clip(), at, clip.duration(), clip.frame_time, g.loops())
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
