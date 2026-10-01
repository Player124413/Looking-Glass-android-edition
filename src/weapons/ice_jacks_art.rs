//! Archive artwork shared by every Ice/Jacks instance; no per-shot GPU pipeline.
use super::{ice, jacks, Prop};
use crate::{
    assets::Assets, environment::Atmosphere, particles::Attached, skeletal::Transform, texture,
};
use anyhow::{Context, Result};
use macroquad::prelude::*;
use std::collections::BTreeMap;
fn shell_frame(age: f32) -> (usize, f32) {
    if age >= 8. {
        (2, age - 8.)
    } else if age >= 6.5 {
        (2, age - 6.5)
    } else if age >= 0.1 {
        (1, age - 0.1)
    } else {
        (0, 0.)
    }
}
pub(super) struct Art {
    walls: Vec<Prop>,
    jacks: Vec<Prop>,
    wand: Prop,
    shell: Vec<Prop>,
    shell_emit: Attached,
    frozen: std::rc::Rc<crate::frozen::Art>,
    stream: Attached,
    idle: Attached,
    swipe: Texture2D,
    pub atmosphere: std::cell::RefCell<Atmosphere>,
}
impl Art {
    pub async fn check_idle(assets: &mut Assets) -> Result<()> {
        let specs = texture::read_materials(assets)?;
        let mut art = Self::load(assets, &specs)?;
        let pose = Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        };
        let tip = art.wand.point(pose, "tag_barrel", 1.);
        let eye = tip + vec3(45., -75., 35.);
        let mut images = Vec::new();
        for on in [false, true, true, false] {
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: eye,
                target: tip,
                up: Vec3::Z,
                fovy: 50_f32.to_radians(),
                z_near: 1.,
                ..Default::default()
            });
            let material = crate::character::skin_material()?;
            material.bind();
            art.wand(pose, 1., 2., true);
            if on {
                art.held_mist(pose, 1., 2., eye);
            }
            gl_use_default_material();
            set_default_camera();
            images.push(get_screen_data());
            if on {
                crate::viewer::save_capture(std::path::Path::new("private/ice-idle-mist.png"))?;
            }
            next_frame().await;
        }
        let changed = images[0]
            .bytes
            .chunks_exact(4)
            .zip(images[1].bytes.chunks_exact(4))
            .filter(|(a, b)| a != b)
            .count();
        anyhow::ensure!(changed > 20, "Ice idle mist did not render: {changed}");
        anyhow::ensure!(
            images[1].bytes == images[2].bytes && images[0].bytes == images[3].bytes,
            "Paused/hidden Ice mist differs"
        );
        println!(
            "PASS idle Ice Wand: {changed} effect pixels; paused and unequipped frames stable"
        );
        Ok(())
    }
    pub fn frozen_body(&self, meshes: &[Option<Mesh>]) {
        for mesh in meshes.iter().flatten() {
            self.frozen.draw(mesh);
        }
    }
    pub fn load(a: &mut Assets, specs: &BTreeMap<String, texture::MaterialSpec>) -> Result<Self> {
        let mut walls = vec![];
        for clip in ["aidle", "rise", "stand", "death"] {
            let (mut def, model) = super::read_model_clip(a, "fx_icewall", Some(clip))?;
            // The original TAN has an additional unassigned base surface.
            // Reuse the supplied wall material instead of an error texture.
            def.skins.insert("all".into(), "skin01.tga".into());
            walls.push(Prop::build(a, def, model, specs)?);
        }
        let mut jacks = vec![];
        for name in ["prj_jacks", "prj_jacks_nosound", "prj_jackball"] {
            jacks.push(Prop::load(a, name, specs)?);
        }
        let mut stream =
            Attached::load(a, "w_icewand", specs)?.context("Ice Wand emitters missing")?;
        for name in [
            "cubes",
            "smoker",
            "smoker2",
            "wallcubes",
            "wallsmoker",
            "wallsmoker2",
        ] {
            stream.orient_velocity(name);
        }
        let path = texture::resolve(a, "textures/special/swipe_jacks", specs)
            .context("Jacks swipe missing")?;
        let image = texture::decode(a, &path)?;
        let swipe = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
        swipe.set_filter(FilterMode::Linear);
        let mut shell = vec![];
        for clip in ["idle", "up", "down"] {
            let (mut def, model) = super::read_model_clip(a, "fx_iceball", Some(clip))?;
            def.skins.insert("all".into(), "powerups/iceblock2".into());
            shell.push(Prop::build(a, def, model, specs)?);
        }
        Ok(Self {
            walls,
            jacks,
            shell,
            shell_emit: Attached::load(a, "fx_iceball", specs)?.context("Water ice emitters")?,
            frozen: crate::frozen::Art::load(a, specs)?,
            wand: Prop::load_animation(a, "w_icewand", "fire", specs)?,
            idle: stream.fork(),
            stream,
            swipe,
            atmosphere: Default::default(),
        })
    }
    pub fn clear(&mut self) {
        self.stream = self.stream.fork();
        self.idle = self.idle.fork();
        self.shell_emit = self.shell_emit.fork();
    }
    pub fn wand(&mut self, pose: Transform, scale: f32, time: f32, fullbright: bool) {
        self.wand.draw_frame(pose, scale, fullbright, time, true);
    }
    pub fn held_mist(&mut self, pose: Transform, scale: f32, time: f32, eye: Vec3) {
        self.idle.draw(
            time,
            scale,
            |_, tag| Transform {
                translation: self.wand.point(pose, tag.unwrap_or("tag_barrel"), scale),
                rotation: pose.rotation,
            },
            |name, _, _| name == "ice_effect",
            eye,
            &self.atmosphere.borrow(),
        );
    }
    pub fn draw(
        &mut self,
        ice: &ice::State,
        data: &ice::Data,
        jacks: &jacks::State,
        camera: Vec3,
        fullbright: bool,
        body: &crate::character::SkinMaterial,
    ) {
        body.bind();
        for w in &ice.walls {
            let (stage, time, looping) = data.stage(w);
            self.walls[stage].draw_frame(w.pose(), 1., fullbright, time, looping);
        }
        for p in &jacks.pieces {
            let index = if p.kind == jacks::Kind::Ball {
                2
            } else {
                usize::from(p.silent)
            };
            let scale = if p.kind == jacks::Kind::Burst {
                0.5 / 0.75
            } else {
                1.
            };
            for m in self.jacks[index].meshes_at(p.pose(), scale, fullbright, p.age as f32, true) {
                crate::render_fx::skin_effect(m, p.age as f32, p.alpha());
            }
            if p.kind == jacks::Kind::Ball {
                continue;
            }
            let mut vertices = vec![];
            for i in 0..12 {
                let a = p.age as f32 - i as f32 / 24.;
                let b = a - 1. / 24.;
                if b < 0. {
                    break;
                }
                let a = p.trail.sample(a, p.pose());
                let b = p.trail.sample(b, p.pose());
                if a.translation.distance_squared(b.translation) > 1000. * 1000. {
                    continue;
                }
                let side = (camera - a.translation)
                    .cross(b.translation - a.translation)
                    .normalize_or_zero()
                    * 2.;
                let color = Color::new(1., 1., 1., (1. - i as f32 / 12.) * p.alpha()).into();
                let corners = [
                    a.translation - side,
                    a.translation + side,
                    b.translation + side,
                    b.translation - side,
                ];
                let uv = [vec2(0., 0.), vec2(1., 0.), vec2(1., 1.), vec2(0., 1.)];
                for j in [0, 1, 2, 0, 2, 3] {
                    vertices.push(Vertex {
                        position: corners[j],
                        uv: uv[j],
                        color,
                        normal: Vec4::ZERO,
                    });
                }
            }
            if !vertices.is_empty() {
                crate::render_fx::effect(
                    &Mesh {
                        indices: (0..vertices.len() as u16).collect(),
                        vertices,
                        texture: Some(self.swipe.clone()),
                    },
                    crate::materials::Blend::AlphaAdd,
                );
            }
        }
        if let Some(s) = &ice.shell {
            let age = s.age as f32;
            let (index, time) = shell_frame(age);
            let pose = Transform {
                translation: s.origin,
                rotation: Quat::from_rotation_z(s.yaw),
            };
            let prop = &mut self.shell[index];
            for mesh in prop.meshes_at(pose, 1., true, time, false) {
                self.frozen.draw(mesh);
            }
            self.shell_emit.draw(
                age,
                1.,
                |birth, tag| {
                    let (i, t) = shell_frame(birth);
                    Transform {
                        translation: self.shell[i].point_at(pose, tag.unwrap_or("tag_1"), t),
                        rotation: pose.rotation,
                    }
                },
                |name, birth, _| {
                    let (index, t) = shell_frame(birth);
                    let frame = (t / self.shell[index].model.frame_time) as usize;
                    let large = name
                        .strip_prefix("smoke")
                        .and_then(|s| s.parse::<u8>().ok())
                        .is_some_and(|i| i >= 5);
                    index != 0
                        && if large {
                            (8..19).contains(&frame)
                        } else {
                            frame < 15
                        }
                },
                camera,
                &self.atmosphere.borrow(),
            );
        }
        let at = ice.clock as f32;
        let fallback = Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        };
        self.stream.draw(
            at,
            1.,
            |t, _| ice.trail.sample(t, fallback),
            |name, t, _| match name {
                "cubes" | "smoker" | "smoker2" => ice.flowing(t, false),
                "wallcubes" | "wallsmoker" | "wallsmoker2" => ice.flowing(t, true),
                _ => false,
            },
            camera,
            &self.atmosphere.borrow(),
        );
    }
}
