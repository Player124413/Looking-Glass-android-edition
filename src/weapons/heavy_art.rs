//! Original archive materials and animation-owned smoke; shared GPU resources.
use super::{
    heavy::{Kind, State},
    Prop,
};
use crate::{
    assets::Assets, environment::Atmosphere, particles::Attached, skeletal::Transform, texture,
};
use anyhow::{Context, Result};
use macroquad::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
struct BeamRig {
    skeleton: crate::skeletal::Skeleton,
    pose: Vec<Transform>,
    root: usize,
    tip: usize,
    core: Prop,
}
impl BeamRig {
    fn load(a: &mut Assets, specs: &BTreeMap<String, texture::MaterialSpec>) -> Result<Self> {
        let (mut def, model) = super::read_model(a, "fx_beam")?;
        let skeleton =
            crate::skeletal::Skeleton::parse(&a.read(&format!("{}/{}", def.path, def.model))?)?;
        let animation = crate::skeletal::Animation::parse(
            &a.read(&format!("{}/{}", def.path, def.animations["idle"]))?,
            skeleton.bones.len(),
        )?;
        let bone = |name| {
            skeleton
                .bones
                .iter()
                .position(|b| b.name == name)
                .context("Missing beam controller")
        };
        let root = bone("Bone01")?;
        let tip = bone("Bone03")?;
        let pose = skeleton.global_pose(&animation.sample(0., false));
        def.skins.insert("all".into(), "fx/eyebeam5".into());
        let core = Prop::build(a, def, model, specs)?;
        // The core ribbon uses the base texture directly, with scrolling UVs.
        for mesh in &core.meshes {
            if let Some(texture) = &mesh.texture {
                unsafe {
                    get_internal_gl().quad_context.texture_set_wrap(
                        texture.raw_miniquad_id(),
                        macroquad::miniquad::TextureWrap::Repeat,
                        macroquad::miniquad::TextureWrap::Repeat,
                    );
                }
            }
        }
        Ok(Self {
            skeleton,
            pose,
            root,
            tip,
            core,
        })
    }
    fn draw(&mut self, outer: &mut Prop, source: Transform, end: Vec3, time: f32, camera: Vec3) {
        let length = source.translation.distance(end);
        if length < 0.01 {
            return;
        }
        let rotation = Quat::from_rotation_arc(Vec3::Z, (end - source.translation) / length);
        // Two orbiting blue beams and the purple core are visible in the
        // original setup. Root scaling stretches the body; the distal
        // controller cancels that scale so the end cap keeps its size.
        for lane in 0..3 {
            let stretch = (length / 200.).max(0.0001);
            let mut pose = self.pose.clone();
            let scales = crate::skeletal::scale_pose(
                &self.skeleton,
                &mut pose,
                &[
                    (self.root, stretch),
                    (self.tip, if lane == 2 { 2. } else { 0.5 } / stretch),
                ],
            );
            let points: Vec<Vec<Vec3>> = self
                .skeleton
                .surfaces
                .iter()
                .map(|s| {
                    s.vertices
                        .iter()
                        .map(|v| {
                            v.weights
                                .iter()
                                .map(|w| pose[w.bone].point(w.offset * scales[w.bone]) * w.amount)
                                .sum()
                        })
                        .collect()
                })
                .collect();
            let (low, high) = points
                .iter()
                .flatten()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), p| {
                    (low.min(p.z), high.max(p.z))
                });
            let phase = time * 6. + lane as f32 * 3.1;
            let orbit = if lane < 2 {
                vec3(phase.sin(), phase.cos(), 0.) * 5.
            } else {
                Vec3::ZERO
            };
            let prop = if lane == 2 {
                &mut self.core
            } else {
                &mut *outer
            };
            for (mesh, points) in prop
                .meshes_at(
                    Transform {
                        translation: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    1.,
                    true,
                    0.,
                    false,
                )
                .iter_mut()
                .zip(points)
            {
                for (v, p) in mesh.vertices.iter_mut().zip(points) {
                    let along = ((p.z - low) / (high - low).max(0.001)).clamp(0., 1.);
                    let p = vec3(p.x, p.y, along * length)
                        + orbit * (std::f32::consts::PI * along).sin();
                    v.position = source.translation + rotation * p;
                    v.normal = Vec4::ZERO;
                }
                crate::render_fx::skin_effect(mesh, time, 1.);
            }
        }
        // From the shoulder camera the narrow, cutout cone is viewed nearly
        // end-on. Keep a continuous, camera-facing core between the same two
        // collision endpoints, using the original purple beam texture.
        let axis = (end - source.translation) / length;
        let side = axis
            .cross(camera - source.translation.lerp(end, 0.5))
            .try_normalize()
            .unwrap_or(rotation * Vec3::X)
            * 2.;
        let scroll = time * -2.;
        let vertices = [
            (source.translation - side, vec2(0., scroll)),
            (end - side, vec2(0., scroll + length / 128.)),
            (end + side, vec2(1., scroll + length / 128.)),
            (source.translation + side, vec2(1., scroll)),
        ]
        .into_iter()
        .map(|(position, uv)| Vertex {
            position, uv, normal: Vec4::ZERO, color: [255; 4],
        })
        .collect();
        crate::render_fx::effect(&Mesh {
            vertices,
            indices: vec![0, 1, 2, 0, 2, 3],
            texture: self.core.meshes.first().and_then(|m| m.texture.clone()),
        }, crate::materials::Blend::AlphaAdd);
    }
}
pub(super) struct Art {
    props: Vec<Prop>,
    beam: BeamRig,
    pub view_muzzle: Option<Vec3>,
    templates: Vec<Attached>,
    emitters: BTreeMap<(usize, u32), Attached>,
    pub atmosphere: std::cell::RefCell<Atmosphere>,
}
impl Art {
    pub fn clear(&mut self) {
        self.emitters.clear();
    }
    pub fn load(a: &mut Assets, specs: &BTreeMap<String, texture::MaterialSpec>) -> Result<Self> {
        let mut props = vec![];
        for (name, clip) in [
            ("w_blunderbuss", "fire"),
            ("prj_blunderbuss", "idle"),
            ("fx_eyestaff_spiral", "idle"),
            ("prj_eyestaff_comet", "idle"),
            ("fx_beam", "idle"),
            ("fx_blunderproj", "idle"),
            ("fx_blundersplode_wall", "idle"),
            ("fx_shockwave", "idle"),
        ] {
            props.push(
                Prop::load_animation(a, name, clip, specs).with_context(|| name.to_string())?,
            );
        }
        let mut templates = vec![];
        for (name, burst) in [
            ("fx_eyestaff_charge", false),
            ("fx_eyestaff_spiral", false),
            ("prj_eyestaff_comet", false),
            ("fx_eyestaff_splode", true),
            ("w_blunderbuss", true),
            ("fx_blundersplode_wall", false),
            ("fx_blundersplode_stage2", true),
            ("fx_blundersplode_person", true),
            ("fx_eyestaff_launcher", false),
            ("fx_eyestaff_comet", false),
        ] {
            templates.push(
                if burst {
                    Attached::load_bursts(a, name, 0.05, specs)?
                } else {
                    Attached::load(a, name, specs)?
                }
                .with_context(|| format!("Missing {name} particles"))?,
            );
        }
        templates[0].orient_velocity("spiral");
        templates[0].orient_velocity("pulsetest");
        // Hundreds of overlapping charge sprites otherwise cover the entire
        // beam at the gameplay camera distance. Keep their travel and cadence,
        // reducing only the billboard size and accumulated brightness.
        templates[0].resize(0.2);
        templates[0].attenuate(0.45);
        templates[4].orient_velocity("burst0");
        templates[4].orient_velocity("burst1");
        Ok(Self {
            beam: BeamRig::load(a, specs)?,
            view_muzzle: None,
            props,
            templates,
            emitters: Default::default(),
            atmosphere: Default::default(),
        })
    }
    pub fn cannon(&mut self, pose: Transform, scale: f32, time: f32, fullbright: bool) {
        self.props[0].draw_frame(pose, scale, fullbright, time, false);
    }
    pub fn draw(
        &mut self,
        s: &State,
        camera: Vec3,
        fullbright: bool,
        first_person: bool,
        body: &crate::character::SkinMaterial,
    ) {
        let mut active = BTreeSet::new();
        let atmosphere = self.atmosphere.borrow();
        if let Some(c) = &s.charge {
            let source = Transform {
                translation: self.view_muzzle.unwrap_or(c.pose.translation),
                ..c.pose
            };
            let view_offset = source.translation - c.pose.translation;
            let key = (0, c.id);
            active.insert(key);
            self.emitters
                .entry(key)
                .or_insert_with(|| self.templates[0].fork())
                .draw(
                    c.age as f32,
                    // The hand is much closer to the eye in first person.
                    // Keep the charge glow from obscuring the beam and aim.
                    if first_person { 0.65 } else { 1. },
                    |t, _| {
                        let mut pose = c.trail.sample(t, c.pose);
                        pose.translation += view_offset;
                        pose
                    },
                    |name, t, _| match name {
                        "sparkle" => c.alternate || t < 2.15,
                        "sparkle2" | "spiral" | "pulsetest" => !c.alternate && t >= 2.15,
                        _ => false,
                    },
                    camera,
                    &atmosphere,
                );
            if !c.alternate && c.age >= 2.3 {
                body.bind();
                self.beam
                    .draw(&mut self.props[4], source, c.end, c.age as f32, camera);
            }
        }
        if let Some((age, pose)) = s.ignition {
            let key = (4, 0);
            active.insert(key);
            let prop = &self.props[0];
            self.emitters
                .entry(key)
                .or_insert_with(|| self.templates[4].fork())
                .draw(
                    age as f32,
                    1.,
                    |t, tag| Transform {
                        translation: prop.point_at(pose, tag.unwrap_or("tag_barrel"), t),
                        rotation: pose.rotation,
                    },
                    |_, _, _| true,
                    camera,
                    &atmosphere,
                );
        }
        for (p, age, stop) in
            s.shots
                .iter()
                .map(|p| (p, p.age as f32, None))
                .chain(s.impacts.iter().filter_map(|i| {
                    i.source
                        .as_ref()
                        .map(|p| (p, (p.age + i.age) as f32, Some(p.age as f32)))
                }))
        {
            let index = match p.kind {
                Kind::Cannon => 1,
                Kind::Spiral => 2,
                Kind::Comet => 3,
            };
            body.bind();
            if stop.is_none() {
                self.props[index].draw_frame(p.pose(), 1., fullbright, p.age as f32, true);
            }
            if p.kind == Kind::Cannon {
                let last = (p.age as f32 * 50.).floor() as u32;
                for i in 0..200 {
                    let t = last.saturating_sub(i) as f32 / 50.;
                    let elapsed = age - t;
                    if elapsed >= 4. || i > last {
                        break;
                    }
                    if t < 0. {
                        break;
                    }
                    let pose = p.trail.sample(t, p.pose());
                    for m in
                        self.props[5].meshes_at(pose, (5. - elapsed).max(0.), true, elapsed, false)
                    {
                        crate::render_fx::skin_effect(m, elapsed, 1.);
                    }
                }
            } else {
                let k = if p.kind == Kind::Spiral { 1 } else { 2 };
                let key = (k, p.id);
                active.insert(key);
                let prop = &self.props[index];
                self.emitters
                    .entry(key)
                    .or_insert_with(|| self.templates[k].fork())
                    .draw(
                        age,
                        1.,
                        |t, tag| {
                            let pose = p.trail.sample(t, p.pose());
                            Transform {
                                translation: prop.point_at(pose, tag.unwrap_or("tag_origin"), t),
                                rotation: pose.rotation,
                            }
                        },
                        |_, t, on| on && stop.is_none_or(|stop| t <= stop),
                        camera,
                        &atmosphere,
                    );
            }
        }
        for v in &s.volleys {
            let key = (8, v.id);
            active.insert(key);
            self.emitters
                .entry(key)
                .or_insert_with(|| self.templates[8].fork())
                .draw(
                    v.age as f32,
                    1.,
                    |_, _| Transform {
                        translation: v.origin,
                        rotation: Quat::from_rotation_arc(Vec3::X, Vec3::Z),
                    },
                    |_, t, on| on && (t as f64) < v.duration,
                    camera,
                    &atmosphere,
                );
            // The launcher emits upward cosmetic comets at 3/s. The native
            // attack schedules separate damaging comets from the ceiling.
            for i in 0..16_u32 {
                let birth = i as f32 / 3.;
                let age = v.age as f32 - birth;
                if birth as f64 >= v.duration || !(0. ..7.5).contains(&age) {
                    continue;
                }
                let mut seed = v.id.wrapping_mul(7919).wrapping_add(i);
                let mut random = || {
                    seed = seed.wrapping_mul(214013).wrapping_add(2531011);
                    ((seed >> 16) & 32767) as f32 / 16384. - 1.
                };
                let origin = v.origin + vec3(random() * 100., random() * 100., 0.);
                let velocity = vec3(random() * 50., random() * 50., 1300.);
                let key = (9, v.id.wrapping_mul(16).wrapping_add(i));
                active.insert(key);
                self.emitters
                    .entry(key)
                    .or_insert_with(|| self.templates[9].fork())
                    .draw(
                        age,
                        1.,
                        |t, _| Transform {
                            translation: origin + velocity * t,
                            rotation: Quat::from_rotation_arc(Vec3::X, Vec3::Z),
                        },
                        |_, t, on| on && t < 5.,
                        camera,
                        &atmosphere,
                    );
            }
        }
        for p in &s.impacts {
            let pose = Transform {
                translation: p.origin,
                rotation: Quat::from_rotation_arc(Vec3::X, p.normal),
            };
            if p.kind != Kind::Cannon {
                let key = (3, p.id);
                active.insert(key);
                self.emitters
                    .entry(key)
                    .or_insert_with(|| self.templates[3].fork())
                    .draw(
                        p.age as f32,
                        1.,
                        |_, _| pose,
                        |_, _, _| true,
                        camera,
                        &atmosphere,
                    );
                if p.age < 0.5 {
                    body.bind();
                    for m in self.props[7].meshes_at(
                        pose,
                        1. + p.age as f32 * 5.,
                        true,
                        p.age as f32,
                        false,
                    ) {
                        crate::render_fx::skin_effect(m, p.age as f32, 1. - p.age as f32 * 2.);
                    }
                }
            } else {
                for k in [5, if p.actor { 7 } else { 6 }] {
                    let key = (k, p.id);
                    active.insert(key);
                    let prop = &self.props[6];
                    self.emitters
                        .entry(key)
                        .or_insert_with(|| self.templates[k].fork())
                        .draw(
                            p.age as f32,
                            1.,
                            |t, tag| {
                                let pose = Transform {
                                    rotation: pose.rotation
                                        * Quat::from_rotation_x(t * 1300_f32.to_radians()),
                                    ..pose
                                };
                                Transform {
                                    translation: prop.point_at(
                                        pose,
                                        tag.unwrap_or("tag_origin"),
                                        t,
                                    ) * 1.,
                                    rotation: pose.rotation,
                                }
                            },
                            |_, t, on| k != 5 || (on && t < 0.7),
                            camera,
                            &atmosphere,
                        );
                }
            }
        }
        self.emitters.retain(|key, _| active.contains(key));
    }
}

/// Actor eye attacks use the same native effect rig as the Eye Staff.
pub(crate) struct EnemyBeam { rig: BeamRig, outer: Prop }
impl EnemyBeam {
    pub(crate) fn load(a: &mut Assets, specs: &BTreeMap<String, texture::MaterialSpec>) -> Result<Self> {
        Ok(Self { rig: BeamRig::load(a, specs)?, outer: Prop::load(a, "fx_beam", specs)? })
    }
    pub(crate) fn draw(&mut self, from: Vec3, to: Vec3, time: f32, camera: Vec3) {
        self.rig.draw(&mut self.outer, Transform { translation: from, rotation: Quat::IDENTITY }, to, time, camera);
    }
}
