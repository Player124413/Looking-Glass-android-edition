//! Original Jackbomb TANs, animated mouth/fuse tags and archive effect art.
use super::{blast, bomb, Prop};
use crate::{
    assets::Assets, environment::Atmosphere, particles::Attached, skeletal::Transform, texture,
};
use anyhow::{Context, Result};
use macroquad::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Art {
    props: Vec<Prop>,
    templates: Vec<Attached>,
    emitters: BTreeMap<(usize, u32), Attached>,
    pub atmosphere: std::cell::RefCell<Atmosphere>,
}
impl Art {
    pub fn load(
        assets: &mut Assets,
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let mut props = Vec::new();
        for (name, clip) in [
            ("prj_jackbomb", "idle"),
            ("prj_jackbomb", "open"),
            ("prj_jackbomb", "spring"),
            ("prj_jackalt", "spring"),
            ("fx_shockwave", "idle"),
            ("fx_jackbombcyl", "idle"),
            ("fx_shockwave2", "idle"),
        ] {
            props.push(Prop::load_animation(assets, name, clip, specs)?);
        }
        let mut templates = Vec::new();
        for name in [
            "prj_jackbomb",
            "prj_jackalt",
            "prj_jackbomb_breath",
            "croquetball",
            "fx_jackbombbit",
            "fx_croquetbit",
        ] {
            templates.push(
                Attached::load(assets, name, specs)?
                    .with_context(|| format!("Missing {name} emitters"))?,
            );
        }
        templates[1].orient_velocity("sparkle");
        // These dense additive sprites otherwise obscure the small moving ball.
        // Change only their presentation, never projectile travel or damage.
        for index in [3, 5] {
            templates[index].resize(0.2);
            templates[index].attenuate(0.5);
        }
        templates.push(
            Attached::load_bursts(assets, "fx_sparkhit", 0.05, specs)?
                .context("Missing Mallet impact sparks")?,
        );
        Ok(Self {
            props,
            templates,
            emitters: BTreeMap::new(),
            atmosphere: std::cell::RefCell::new(Atmosphere::default()),
        })
    }
    pub fn clear(&mut self) {
        self.emitters.clear();
    }
    pub fn draw(
        &mut self,
        simulation: (&bomb::State, &bomb::Data),
        balls: &[super::Projectile],
        blasts: &[blast::Blast],
        camera: Vec3,
        fullbright: bool,
        body: &crate::character::SkinMaterial,
    ) {
        let (state, data) = simulation;
        let mut active = BTreeSet::new();
        let atmosphere = self.atmosphere.borrow();
        body.bind();
        for b in &state.bombs {
            let (stage, time, looping) = data.stage(b);
            self.props[stage].draw_frame(b.pose(), 1., fullbright, time, looping);
        }
        for b in &state.bombs {
            let kind = usize::from(b.alternate);
            let key = (kind, b.id);
            active.insert(key);
            let e = self
                .emitters
                .entry(key)
                .or_insert_with(|| self.templates[kind].fork());
            e.draw(
                b.age as f32,
                1.,
                |at, tag| {
                    let mut at_b = b.clone();
                    at_b.age = at as f64;
                    data.tag(&at_b, tag.unwrap_or("tag_fuse"))
                },
                |name, at, default| match name {
                    "fuse" | "light" => at as f64 >= data.crank,
                    "biglight" | "sparkle" => b.alternate && at as f64 >= data.crank + data.open,
                    _ => default,
                },
                camera,
                &atmosphere,
            );
        }
        for f in &state.flames {
            let key = (2, f.id);
            active.insert(key);
            self.emitters
                .entry(key)
                .or_insert_with(|| self.templates[2].fork())
                .draw(
                    f.age as f32,
                    1.,
                    |at, _| {
                        f.trail.sample(
                            at,
                            Transform {
                                translation: f.position,
                                rotation: Quat::IDENTITY,
                            },
                        )
                    },
                    |_, _, default| default,
                    camera,
                    &atmosphere,
                );
        }
        for p in balls.iter().filter(|p| p.model == 11) {
            let key = (3, p.audio_id as u32);
            active.insert(key);
            self.emitters
                .entry(key)
                .or_insert_with(|| self.templates[3].fork())
                .draw(
                    p.age as f32,
                    1.,
                    |at, _| {
                        p.trail.sample(
                            at,
                            Transform {
                                translation: p.position,
                                rotation: Quat::IDENTITY,
                            },
                        )
                    },
                    |_, _, default| default,
                    camera,
                    &atmosphere,
                );
        }
        for b in blasts
            .iter()
            .filter(|b| matches!(b.kind, blast::Kind::Jack | blast::Kind::Croquet))
        {
            body.bind();
            if b.kind == blast::Kind::Jack {
                if b.age < 1. {
                    fade_prop(
                        &mut self.props[4],
                        b.origin,
                        b.age,
                        1.5 + 4. * b.age,
                        0.5 * (1. - b.age),
                    );
                }
                let t = b.age - 0.1;
                if (0. ..0.8).contains(&t) {
                    fade_prop(&mut self.props[5], b.origin, t, 1.3, 1. - t / 0.8);
                }
                if (0. ..0.7).contains(&t) {
                    for i in 0..3 {
                        let a = i as f32 * 2.399963;
                        fade_prop(
                            &mut self.props[6],
                            b.origin + vec3(a.cos(), a.sin(), 0.) * 60.,
                            t,
                            0.4 + 1.1 * t,
                            1. - t / 0.7,
                        );
                    }
                }
            }
            let kind = if b.kind == blast::Kind::Jack { 4 } else { 5 };
            if kind == 4 && b.age < 0.1 {
                continue;
            }
            let count = if kind == 4 { 5 } else { 3 };
            for i in 0..count {
                let key = (kind, b.seed.wrapping_mul(7).wrapping_add(i));
                active.insert(key);
                let angle = (b.seed.wrapping_add(i * 31) % 1000) as f32 * 2.399963;
                let speed = if kind == 4 { 300. } else { 800. };
                let velocity = vec3(angle.cos(), angle.sin(), 0.4 + (i % 3) as f32 * 0.2) * speed;
                let gravity = if kind == 4 { 200. } else { 800. };
                let at = (b.age - if kind == 4 { 0.1 } else { 0. }).max(0.);
                let e = self
                    .emitters
                    .entry(key)
                    .or_insert_with(|| self.templates[kind].fork());
                e.draw(
                    at,
                    if kind == 4 { 0.33 } else { 1. },
                    |t, _| Transform {
                        translation: b.origin + velocity * t - Vec3::Z * (0.5 * gravity * t * t),
                        rotation: Quat::IDENTITY,
                    },
                    |_, t, default| default && t < 3. + (i % 3) as f32,
                    camera,
                    &atmosphere,
                );
            }
        }
        for b in blasts
            .iter()
            .filter(|b| b.kind == blast::Kind::ElectricHit && b.age < 0.75)
        {
            let key = (6, b.seed);
            active.insert(key);
            self.emitters
                .entry(key)
                .or_insert_with(|| self.templates[6].fork())
                .draw(
                    b.age,
                    1.,
                    |_, _| Transform {
                        translation: b.origin,
                        rotation: Quat::IDENTITY,
                    },
                    |_, _, default| default,
                    camera,
                    &atmosphere,
                );
        }
        self.emitters.retain(|k, _| active.contains(k));
        gl_use_default_material();
    }
}
fn fade_prop(prop: &mut Prop, position: Vec3, age: f32, scale: f32, alpha: f32) {
    crate::render::depth_read_only(|| {
        for mesh in prop.meshes_at(
            Transform {
                translation: position,
                rotation: Quat::IDENTITY,
            },
            scale,
            true,
            age,
            false,
        ) {
            crate::render_fx::skin_effect(mesh, age, alpha);
        }
    });
}
