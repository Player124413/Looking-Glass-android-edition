//! Archive-backed attack ribbons. Simulation owns their endpoints and lifetimes.
use crate::{
    assets::Assets, environment::Atmosphere, particles::Attached, render_fx, skeletal::Transform,
    texture,
};
use anyhow::{Context, Result};
use macroquad::prelude::*;
use std::{collections::BTreeMap, rc::Rc};

pub struct Art {
    layers: BTreeMap<&'static str, (Texture2D, Rc<render_fx::Surface>)>,
    impact: Attached,
}
thread_local! { static CACHE: std::cell::RefCell<std::rc::Weak<Art>> = Default::default(); }
const MATERIALS: [&str; 2] = ["queenbeam1", "emap1"];

fn ribbon(from: Vec3, to: Vec3, camera: Vec3, width: f32, repeats: f32) -> Option<Mesh> {
    let axis = (to - from).try_normalize()?;
    if !from.is_finite()
        || !to.is_finite()
        || !camera.is_finite()
        || !width.is_finite()
        || width <= 0.
    {
        return None;
    }
    let side = axis
        .cross(camera - from.lerp(to, 0.5))
        .try_normalize()
        .unwrap_or_else(|| axis.any_orthonormal_vector())
        * width;
    Some(Mesh {
        vertices: [
            (from - side, vec2(0., 0.)),
            (to - side, vec2(repeats, 0.)),
            (to + side, vec2(repeats, 1.)),
            (from + side, vec2(0., 1.)),
        ]
        .into_iter()
        .map(|(p, uv)| Vertex::new(p.x, p.y, p.z, uv.x, uv.y, WHITE))
        .collect(),
        indices: vec![0, 1, 2, 0, 2, 3],
        texture: None,
    })
}
impl Art {
    pub fn load(
        a: &mut Assets,
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Rc<Self>> {
        if let Some(v) = CACHE.with(|c| c.borrow().upgrade()) {
            return Ok(v);
        }
        let mut layers = BTreeMap::new();
        for name in MATERIALS {
            let path = texture::resolve(a, name, specs)
                .with_context(|| format!("Missing attack material {name}"))?;
            let image = texture::decode(a, &path)?;
            let tex = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
            tex.set_filter(FilterMode::Linear);
            let surface = render_fx::register(a, &tex, &[name.into()], specs)?;
            layers.insert(name, (tex, surface));
        }
        let art = Rc::new(Self {
            layers,
            impact: Attached::load_clip_bursts(a, "fx_lightning_hit", Some("idle"), 0.05, specs)?
                .context("Missing lightning interception effect")?,
        });
        CACHE.with(|c| *c.borrow_mut() = Rc::downgrade(&art));
        Ok(art)
    }
    fn draw(
        &self,
        material: &str,
        from: Vec3,
        to: Vec3,
        camera: Vec3,
        width: f32,
        age: f32,
        repeat: f32,
    ) {
        let Some(mut mesh) = ribbon(from, to, camera, width, repeat) else {
            return;
        };
        mesh.texture = Some(self.layers[material].0.clone());
        // Original material stages own colour, blending and scrolling; no debug tint.
        render_fx::skin_effect(&mesh, age, 1.);
    }
    pub fn lightning(
        &self,
        from: Vec3,
        to: Vec3,
        camera: Vec3,
        age: f32,
        range: f32,
        authored: bool,
        atm: &Atmosphere,
    ) {
        self.draw(
            if authored { "queenbeam1" } else { "emap1" },
            from,
            to,
            camera,
            2.5,
            age,
            1.,
        );
        if from.distance(to) < range - 1. && age >= 0. {
            // Reconstruct bounded interception bursts from the saved beam clock.
            // No render-time damage, wall-clock timer or persistent emitter history.
            let mut effect = self.impact.fork();
            effect.draw(
                age.rem_euclid(0.125),
                1.,
                |_, _| Transform {
                    translation: to,
                    rotation: Quat::from_rotation_arc(
                        Vec3::Z,
                        (from - to).try_normalize().unwrap_or(Vec3::Z),
                    ),
                },
                |_, _, on| on,
                camera,
                atm,
            );
        }
    }
}

pub fn check(a: &mut Assets) -> Result<()> {
    let specs = texture::read_materials(a)?;
    for name in MATERIALS {
        let spec = &specs[name];
        anyhow::ensure!(
            spec.stages
                .iter()
                .any(|s| s.blend == crate::materials::Blend::Add),
            "Attack material must retain additive blending: {name}"
        );
        let path = texture::resolve(a, name, &specs).context("Missing attack texture")?;
        let image = texture::decode(a, &path)?;
        anyhow::ensure!(
            image
                .pixels
                .chunks_exact(4)
                .map(|p| (p[0], p[1], p[2]))
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                > 16,
            "Attack texture became a flat colour: {name}"
        );
    }
    for model in ["c_chess_red_bishop", "c_chess_bishop", "c_chess_red_king"] {
        let bytes = a.read(&format!("models/{model}.tik"))?;
        let text = String::from_utf8_lossy(&bytes);
        anyhow::ensure!(
            text.split_whitespace().any(|t| t == "queenbeam1"),
            "Changed original attack material for {model}"
        );
    }
    println!("PASS attack textures, additive materials and authored chess beam references");
    Ok(())
}

pub async fn render_check(a: &mut Assets) -> Result<()> {
    let specs = texture::read_materials(a)?;
    let art = Art::load(a, &specs)?;
    let atmosphere = Atmosphere::default();
    let camera = Camera3D {
        position: vec3(0., -300., 70.),
        target: vec3(0., 0., 70.),
        up: Vec3::Z,
        fovy: 60_f32.to_radians(),
        ..Default::default()
    };
    std::fs::create_dir_all("private/npc-attack-fx/captures")?;
    let mut frames = Vec::new();
    for (label, material, age, wall, enabled) in [
        ("chess-lightning", "queenbeam1", 0.03, false, true),
        ("chess-scroll", "queenbeam1", 0.09, false, true),
        ("chess-paused", "queenbeam1", 0.03, false, true),
        ("jabberspawn-lightning", "emap1", 0.03, false, true),
        ("wall-baseline", "queenbeam1", 0.03, true, false),
        ("wall-occlusion", "queenbeam1", 0.03, true, true),
    ] {
        for _ in 0..2 {
            clear_background(BLACK);
            set_camera(&camera);
            if wall {
                draw_cube(vec3(0., -25., 70.), vec3(60., 8., 50.), None, GRAY);
            }
            render_fx::begin_view(&camera, age, &atmosphere, false);
            if enabled {
                art.draw(
                    material,
                    vec3(-100., 0., 70.),
                    vec3(100., 0., 70.),
                    camera.position,
                    2.5,
                    age,
                    1.,
                );
            }
            let (_, dropped) = render_fx::finish();
            anyhow::ensure!(dropped == 0, "Attack effect dropped geometry");
            set_default_camera();
            next_frame().await;
        }
        // Capture the same composed frame before yielding another clear.
        clear_background(BLACK);
        set_camera(&camera);
        if wall {
            draw_cube(vec3(0., -25., 70.), vec3(60., 8., 50.), None, GRAY);
        }
        render_fx::begin_view(&camera, age, &atmosphere, false);
        if enabled {
            art.draw(
                material,
                vec3(-100., 0., 70.),
                vec3(100., 0., 70.),
                camera.position,
                2.5,
                age,
                1.,
            );
        }
        render_fx::finish();
        set_default_camera();
        let image = get_screen_data();
        image.export_png(&format!("private/npc-attack-fx/captures/{label}.png"));
        frames.push(image);
        next_frame().await;
    }
    anyhow::ensure!(
        frames[0].bytes == frames[2].bytes,
        "Paused attack appearance drifted"
    );
    anyhow::ensure!(
        frames[0].bytes != frames[1].bytes,
        "Original lightning scroll is not animated"
    );
    for i in [0, 3] {
        let pixels = frames[i]
            .bytes
            .chunks_exact(4)
            .filter(|p| p[0].max(p[1]).max(p[2]) > 24)
            .count();
        anyhow::ensure!(
            pixels > 100,
            "Original attack texture is invisible ({pixels})"
        );
    }
    let width = frames[4].width as usize;
    let height = frames[4].height as usize;
    for y in height * 49 / 100..height * 51 / 100 {
        for x in width * 48 / 100..width * 52 / 100 {
            let i = (y * width + x) * 4;
            anyhow::ensure!(
                frames[4].bytes[i..i + 4] == frames[5].bytes[i..i + 4],
                "Attack renders through solid cover"
            );
        }
    }
    anyhow::ensure!(
        frames[4].bytes != frames[5].bytes,
        "Occlusion fixture hid the whole beam"
    );
    println!("PASS original attack pixels, scrolling, paused identity and solid-cover occlusion");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ribbon_keeps_collision_endpoints_and_has_width_even_end_on() {
        for camera in [Vec3::X * 20., Vec3::Y * 20., Vec3::ZERO] {
            let m = ribbon(Vec3::ZERO, Vec3::X * 10., camera, 2.5, 3.).unwrap();
            assert!(m.vertices.iter().all(|v| v.position.is_finite()));
            assert!(((m.vertices[0].position + m.vertices[3].position) * 0.5).length() < 1e-5);
            assert!(
                ((m.vertices[1].position + m.vertices[2].position) * 0.5 - Vec3::X * 10.).length()
                    < 1e-5
            );
            assert!((m.vertices[0].position.distance(m.vertices[3].position) - 5.).abs() < 1e-5);
        }
        assert!(ribbon(Vec3::ZERO, Vec3::ZERO, Vec3::Y, 2., 1.).is_none());
        assert!(ribbon(Vec3::ZERO, Vec3::X, Vec3::Y, f32::NAN, 1.).is_none());
    }
}
