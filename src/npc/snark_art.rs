//! Per-Snark emitter histories and per-projectile trails, cleared on restore.
use super::*;
use crate::{
    clockwork::Rig,
    snark::{Kind, Phase, Snark},
};
pub(super) struct Projectiles {
    prop: Option<Prop>,
    trail: Option<crate::particles::Attached>,
}
impl Projectiles {
    pub fn load(
        assets: &mut Assets,
        actors: &[Actor],
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<BTreeMap<Kind, Self>> {
        let mut result = BTreeMap::new();
        for kind in [Kind::Water, Kind::Fire] {
            if actors.iter().any(|a| a.spawn.model == kind.model()) {
                result.insert(
                    kind,
                    Self {
                        prop: if kind == Kind::Water {
                            Some(Prop::load_animation(
                                assets,
                                kind.projectile(),
                                "idle",
                                specs,
                            )?)
                        } else {
                            None
                        },
                        trail: crate::particles::Attached::load(assets, kind.projectile(), specs)?,
                    },
                );
            }
        }
        Ok(result)
    }
}
pub(super) struct Art {
    smoke: Option<crate::particles::Attached>,
    trails: BTreeMap<u32, crate::particles::Attached>,
    phase: Option<Phase>,
    time: f32,
}
impl Art {
    pub fn new(smoke: Option<&crate::particles::Attached>) -> Self {
        Self {
            smoke: smoke.map(|s| s.fork()),
            trails: BTreeMap::new(),
            phase: None,
            time: 0.,
        }
    }
    pub fn shots(
        &mut self,
        g: &Snark,
        props: &mut Projectiles,
        camera: Vec3,
        atmosphere: &Atmosphere,
        fullbright: bool,
        material: &crate::character::SkinMaterial,
    ) {
        self.trails
            .retain(|id, _| g.shots.iter().any(|s| s.serial == *id));
        for s in &g.shots {
            let transform = Transform {
                translation: s.position,
                rotation: Quat::from_rotation_arc(Vec3::X, s.direction),
            };
            material.bind();
            if let Some(prop) = &mut props.prop {
                prop.draw_frame(transform, 1., fullbright, s.age, true);
            }
            if let Some(template) = &props.trail {
                self.trails
                    .entry(s.serial)
                    .or_insert_with(|| template.fork())
                    .draw(
                        s.age,
                        1.,
                        |at, tag| {
                            let mut pose = transform;
                            pose.translation -= s.direction * g.kind.speed() * (s.age - at);
                            if let (Some(tag), Some(prop)) = (tag, props.prop.as_ref()) {
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
    }
    pub fn tongue(&self, g: &Snark, data: &Data) {
        if let Some(end) = g.tongue {
            gl_use_default_material();
            let start = g.mouth(data, g.time);
            for z in [-1., 0., 1.] {
                draw_line_3d(
                    start + Vec3::Z * z,
                    end + Vec3::Z * z,
                    Color::new(0.65, 0.1, 0.15, 1.),
                );
            }
        }
    }
    pub fn smoke(&mut self, g: &Snark, data: &Data, camera: Vec3, atmosphere: &Atmosphere) {
        let Some(effect) = &mut self.smoke else {
            return;
        };
        if self.phase != Some(g.phase) || g.time < self.time {
            *effect = effect.fork();
        }
        self.phase = Some(g.phase);
        self.time = g.time;
        if g.kind != Kind::Fire || g.health <= 0. {
            return;
        }
        let transform = Transform {
            translation: g.feet,
            rotation: Quat::from_rotation_z(g.yaw),
        };
        effect.draw(
            g.time,
            g.scale,
            |at, tag| {
                if let Some(tag) = tag {
                    let at = if g.loops() {
                        at.rem_euclid(data.clips[g.clip()].duration())
                    } else {
                        at
                    };
                    let local = data.tag(g.clip(), at, tag);
                    Transform {
                        translation: transform.point(local.translation * g.scale),
                        rotation: transform.rotation * local.rotation,
                    }
                } else {
                    transform
                }
            },
            |_, _, default| default,
            camera,
            atmosphere,
        );
    }
}
