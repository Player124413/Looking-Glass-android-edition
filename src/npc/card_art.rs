//! Per-actor particle state; shared props never retain another guard's trail.
use super::*;
use crate::cards::{Guard, Kind, Phase};
use crate::clockwork::Rig;
pub(super) struct Projectiles {
    prop: Prop,
    trail: Option<crate::particles::Attached>,
}
impl Projectiles {
    pub fn load(
        assets: &mut Assets,
        actors: &[Actor],
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<BTreeMap<Kind, Self>> {
        let mut result = BTreeMap::new();
        for kind in [Kind::Heart, Kind::Spade] {
            if actors.iter().any(|a| a.spawn.model == kind.model()) {
                result.insert(
                    kind,
                    Self {
                        prop: Prop::load_animation(assets, kind.projectile(), "idle", specs)?,
                        trail: crate::particles::Attached::load(assets, kind.projectile(), specs)?,
                    },
                );
            }
        }
        Ok(result)
    }
}
pub(super) struct Art {
    charge: Option<crate::particles::Attached>,
    trails: BTreeMap<u32, crate::particles::Attached>,
    phase: Option<Phase>,
    time: f32,
}
impl Art {
    pub fn new(charge: Option<&crate::particles::Attached>) -> Self {
        Self {
            charge: charge.map(|s| s.fork()),
            trails: BTreeMap::new(),
            phase: None,
            time: 0.,
        }
    }
    pub fn shots(
        &mut self,
        g: &Guard,
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
            props
                .prop
                .draw_frame(transform, 1., fullbright, s.age, true);
            if let Some(template) = &props.trail {
                self.trails
                    .entry(s.serial)
                    .or_insert_with(|| template.fork())
                    .draw(
                        s.age,
                        1.,
                        |at, tag| {
                            let mut pose = transform;
                            pose.translation -= s.direction * 850. * (s.age - at);
                            if let Some(tag) = tag {
                                pose.translation = props.prop.point(pose, tag, 1.);
                            }
                            pose
                        },
                        |_, _, default| default,
                        camera,
                        atmosphere,
                    );
            }
        }
        // Bounded impact flare while nested source explosion meshes remain unsupported.
        gl_use_default_material();
        for i in &g.impacts {
            if i.age < 0.3 {
                draw_sphere(
                    i.position,
                    12. * (1. - i.age / 0.3),
                    None,
                    Color::new(1., 0.2, 0.1, 1.),
                );
            }
        }
        material.bind();
    }
    pub fn charge(&mut self, g: &Guard, data: &Data, camera: Vec3, atmosphere: &Atmosphere) {
        let Some(effect) = &mut self.charge else {
            return;
        };
        if self.phase != Some(g.phase) || g.time < self.time {
            *effect = effect.fork();
        }
        self.phase = Some(g.phase);
        self.time = g.time;
        if g.kind != Kind::Heart || !matches!(g.phase, Phase::Charge | Phase::Slam) {
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
impl Model {
    pub(super) fn draw_card(
        &mut self,
        g: &Guard,
        fullbright: bool,
        material: &crate::character::SkinMaterial,
    ) {
        let time = g.sample_time(&self.data);
        let fade = g.fade(&self.data);
        let clip = &self.data.clips[g.clip()];
        let local = clip.sample(time, g.loops());
        let visual =
            self.data
                .events
                .visual(g.clip(), time, clip.duration(), clip.frame_time, g.loops());
        if fade > 0. {
            let mut feet = g.feet;
            let scale = if g.kind == Kind::Heart || g.frozen {
                g.scale * fade
            } else {
                feet.z -= (1. - fade) * 100. * g.scale;
                g.scale
            };
            material.bind();
            self.draw_pose(
                &local,
                0.,
                time,
                g.health > 0.,
                Transform {
                    translation: feet,
                    rotation: Quat::from_rotation_z(g.yaw),
                },
                scale,
                fullbright,
                !g.dismember.severed,
                &visual,
                g.dismember.severed.then_some(("top*", "cap_body", false)),
            );
            if g.frozen {
                self.draw_frozen();
            }
        }
        if let Some(fragment) = &g.dismember.fragment {
            let clip = &self.data.clips["death_top"];
            let local = clip.sample(g.dismember.age, false);
            let visual = self.data.events.visual(
                "death_top",
                g.dismember.age,
                clip.duration(),
                clip.frame_time,
                false,
            );
            material.draw_dissolving(g.dismember.fade(), || {
                self.draw_pose(
                    &local,
                    0.,
                    g.dismember.age,
                    false,
                    fragment.transform(),
                    g.scale,
                    fullbright,
                    false,
                    &visual,
                    Some(("top*", "cap_body", true)),
                )
            });
        }
    }
}
