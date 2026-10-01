//! Per-actor cosmetic caches. Meshes/materials are shared; particle birth positions are not.
use super::*;
use crate::clockwork::{Automaton, Phase, Rig};
pub(super) struct Art {
    steam: Option<crate::particles::Attached>,
    trail: Option<crate::particles::Attached>,
    trails: BTreeMap<u32, crate::particles::Attached>,
    phase: Option<Phase>,
    time: f32,
}
impl Art {
    pub fn new(
        steam: Option<&crate::particles::Attached>,
        trail: Option<&crate::particles::Attached>,
    ) -> Self {
        let mut steam = steam.map(|s| s.fork());
        if let Some(s) = &mut steam {
            s.orient_velocity("steamblast1");
            s.orient_velocity("steamblast2");
        }
        Self {
            steam,
            trail: trail.map(|s| s.fork()),
            trails: BTreeMap::new(),
            phase: None,
            time: 0.,
        }
    }
    pub fn shots(
        &mut self,
        p: &Automaton,
        prop: &mut Prop,
        camera: Vec3,
        atmosphere: &Atmosphere,
        fullbright: bool,
        material: &crate::character::SkinMaterial,
    ) {
        self.trails
            .retain(|id, _| p.fists.iter().any(|f| f.serial == *id));
        for fist in &p.fists {
            let transform = Transform {
                translation: fist.position,
                rotation: Quat::from_rotation_arc(Vec3::X, fist.direction),
            };
            material.bind();
            prop.draw(transform, 1., fullbright);
            if let Some(template) = &self.trail {
                let effect = self
                    .trails
                    .entry(fist.serial)
                    .or_insert_with(|| template.fork());
                effect.draw(
                    fist.age,
                    1.,
                    |at, tag| {
                        let mut pose = transform;
                        pose.translation -= fist.direction * 500. * (fist.age - at);
                        if let Some(tag) = tag {
                            pose.translation = prop.point(pose, tag, 1.);
                        }
                        pose
                    },
                    |_, _, default| default,
                    camera,
                    atmosphere,
                );
            }
        }
        gl_use_default_material();
        for impact in &p.impacts {
            draw_sphere(
                impact.position,
                8. * (1. - impact.age / 0.25),
                None,
                Color::new(1., 0.65, 0.1, 1.),
            );
        }
        material.bind();
    }
    pub fn steam(&mut self, p: &Automaton, data: &Data, camera: Vec3, atmosphere: &Atmosphere) {
        let Some(effect) = &mut self.steam else {
            return;
        };
        if self.phase != Some(p.phase) || p.time < self.time {
            *effect = effect.fork();
        }
        self.phase = Some(p.phase);
        self.time = p.time;
        if !p.active || p.health <= 0. {
            return;
        }
        let transform = Transform {
            translation: p.feet,
            rotation: Quat::from_rotation_z(p.yaw),
        };
        let clip = &data.clips[p.clip()];
        effect.draw(
            p.time,
            p.scale,
            |at, tag| {
                let local = data.tag(p.clip(), at, tag.unwrap_or("tag_smoke"));
                Transform {
                    translation: transform.point(local.translation * p.scale),
                    rotation: transform.rotation * local.rotation,
                }
            },
            |name, at, default| {
                data.events
                    .visual(p.clip(), at, clip.duration(), clip.frame_time, p.loops())
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
