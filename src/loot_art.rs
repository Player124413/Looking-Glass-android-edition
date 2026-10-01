//! Meta Essence uses three authored passes: reflection, coloured filter, swirl.
//! Treating its first image as an opaque skin loses both colour and transparency.
use crate::{assets::Assets, environment::Atmosphere, loot::Grade, materials::Blend, texture};
use anyhow::{ensure, Context, Result};
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
};

struct Layer {
    spec: crate::materials::Stage,
    texture: Texture2D,
}
pub struct Art {
    props: Vec<crate::weapons::Prop>,
    layers: Vec<Vec<Layer>>,
    effects: Vec<crate::pickup_effects::Effect>,
    materials: [Rc<Material>; 2],
}
pub const GRADES: [Grade; 4] = [Grade::Small, Grade::Medium, Grade::Large, Grade::Super];
impl Art {
    pub fn load(
        assets: &mut Assets,
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let mut props = Vec::new();
        let mut layers = Vec::new();
        let mut textures = BTreeMap::<String, Texture2D>::new();
        let direct = BTreeMap::new();
        for (i, grade) in GRADES.into_iter().enumerate() {
            props.push(crate::weapons::Prop::load_animation(
                assets,
                grade.model(),
                "idle",
                &direct,
            )?);
            let shader = format!("models/weapons/metaessence/skin{:02}", 4 - i);
            let stages = &specs
                .get(&shader)
                .context("Missing Meta Essence material")?
                .stages;
            ensure!(
                stages.len() == 3
                    && stages
                        .iter()
                        .map(|s| s.blend)
                        .eq([Blend::Add, Blend::Filter, Blend::Add]),
                "Unsupported Meta Essence material {shader}"
            );
            let mut grade_layers = Vec::new();
            for stage in stages {
                let name = stage.images.first().context("Missing Meta Essence image")?;
                // Stage images are literal assets, not shader aliases. Resolving
                // skin02.tga through the shader again selects ref.tga instead.
                let path = texture::resolve(assets, name, &direct)
                    .context("Missing Meta Essence texture")?;
                if !textures.contains_key(&path) {
                    let image = texture::decode(assets, &path)?;
                    let tex = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
                    tex.set_filter(FilterMode::Linear);
                    textures.insert(path.clone(), tex);
                }
                grade_layers.push(Layer {
                    spec: stage.clone(),
                    texture: textures[&path].clone(),
                });
            }
            layers.push(grade_layers);
        }
        Ok(Self {
            props,
            layers,
            effects: GRADES
                .iter()
                .map(|g| crate::pickup_effects::Effect::load(assets, g.model(), "idle", specs))
                .collect::<Result<_>>()?,
            materials: [material(false)?, material(true)?],
        })
    }
    pub fn draw(
        &mut self,
        grade: Grade,
        origin: Vec3,
        atmosphere: &Atmosphere,
        eye: Vec3,
        clock: f32,
    ) {
        if crate::render_fx::active() {
            self.draw_inner(grade, origin, atmosphere, eye, clock);
        } else {
            crate::render::depth_read_only(|| {
                self.draw_inner(grade, origin, atmosphere, eye, clock)
            });
        }
    }
    fn draw_inner(
        &mut self,
        grade: Grade,
        origin: Vec3,
        atmosphere: &Atmosphere,
        eye: Vec3,
        clock: f32,
    ) {
        let i = GRADES.iter().position(|g| *g == grade).unwrap();
        for (filter, material) in self.materials.iter().enumerate() {
            atmosphere.apply(material, eye);
            material.set_uniform("FogMode", if filter == 1 { 2_f32 } else { 1. });
        }
        let meshes = self.props[i].meshes_at(
            crate::skeletal::Transform {
                translation: origin,
                rotation: Quat::from_rotation_z(clock * 0.4),
            },
            1.,
            true,
            clock,
            true,
        );
        for mesh in meshes {
            let uv: Vec<_> = mesh.vertices.iter().map(|v| v.uv).collect();
            let mut queued = Vec::new();
            for layer in &self.layers[i] {
                for (v, &base) in mesh.vertices.iter_mut().zip(&uv) {
                    v.uv = layer.spec.view_uv(
                        base,
                        v.position,
                        v.normal.truncate().normalize_or_zero(),
                        eye,
                        clock,
                    );
                }
                mesh.texture = Some(layer.texture.clone());
                if crate::render_fx::active() {
                    if let Ok(p) = crate::render_fx::pass(
                        crate::render_fx::copy_mesh(mesh),
                        &layer.spec,
                        true,
                        false,
                        Default::default(),
                    ) {
                        queued.push(p);
                    }
                    continue;
                }
                gl_use_material(&self.materials[usize::from(layer.spec.blend == Blend::Filter)]);
                draw_mesh(mesh);
            }
            if !queued.is_empty() {
                crate::render_fx::submit(queued, true);
            }
        }
        gl_use_default_material();
        self.effects[i].draw(clock, origin, 0.4, eye, atmosphere);
    }
}

thread_local! {
    // Quickload keeps both level owners alive; do not allocate more pipelines.
    static MATERIALS: RefCell<[Weak<Material>; 2]> = RefCell::new(Default::default());
}
fn material(filter: bool) -> Result<Rc<Material>> {
    MATERIALS.with(|cache| {
        let index = usize::from(filter);
        if let Some(m) = cache.borrow()[index].upgrade() {
            return Ok(m);
        }
        let mut uniforms = crate::environment::uniforms();
        uniforms.push(UniformDesc::new("FogMode", UniformType::Float1));
        let (source, dest) = if filter {
            (
                BlendFactor::Zero,
                BlendFactor::Value(BlendValue::SourceColor),
            )
        } else {
            (BlendFactor::One, BlendFactor::One)
        };
        let m = Rc::new(
            load_material(
                ShaderSource::Glsl {
                    vertex: crate::character::VERTEX,
                    fragment: &crate::environment::fragment(FRAGMENT),
                },
                MaterialParams {
                    uniforms,
                    pipeline_params: crate::render::depth_pipeline(Some(BlendState::new(
                        Equation::Add,
                        source,
                        dest,
                    ))),
                    ..Default::default()
                },
            )
            .map_err(|e| anyhow::anyhow!("Meta Essence shader: {e:?}"))?,
        );
        cache.borrow_mut()[index] = Rc::downgrade(&m);
        Ok(m)
    })
}
const FRAGMENT: &str = r#"#version 100
precision mediump float;
uniform sampler2D Texture;
uniform float FogMode;
varying highp vec2 uv;
// FOG
void main() { gl_FragColor=vec4(fogged(texture2D(Texture,fract(uv)).rgb,FogMode),1.0); }
"#;

/// Native regression using the same saved drops and item renderer as gameplay.
pub async fn check(assets: &mut Assets) -> Result<()> {
    use crate::{
        inventory::Stats,
        loot::{Drop, Loot},
    };
    let mut art = crate::powerups::Art::load(assets)?;
    let mut stats = Stats::default();
    stats.loot.insert(
        "render".into(),
        Loot {
            awarded: (0..4).collect(),
            drops: GRADES
                .into_iter()
                .enumerate()
                .map(|(i, grade)| Drop {
                    id: i,
                    origin: vec3(-75. + i as f32 * 50., 0., 0.),
                    grade,
                    remaining: 5.,
                })
                .collect(),
        },
    );
    let stats: Stats = serde_json::from_slice(&serde_json::to_vec(&stats)?)?;
    stats.validate_save()?;
    let placed = stats.loot["render"].pickups("placed");
    let eye = vec3(0., -220., 48.);
    let camera = Camera3D {
        position: eye,
        target: vec3(0., 0., 48.),
        up: Vec3::Z,
        projection: Projection::Orthographics,
        fovy: 120.,
        z_near: 1.,
        z_far: 500.,
        ..Default::default()
    };
    let atmosphere = Atmosphere::default();
    let background = Color::from_hex(0x18212b);
    let mut first = None;
    for time in [0_f32, 2.] {
        clear_background(background);
        set_camera(&camera);
        art.draw(&[], &stats, "render", &atmosphere, eye, time);
        let image = get_screen_data();
        clear_background(background);
        art.draw(&placed, &Stats::default(), "placed", &atmosphere, eye, time);
        ensure!(
            image.bytes == get_screen_data().bytes,
            "Placed Meta Essence does not match the original drop materials"
        );
        for (i, grade) in GRADES.iter().enumerate() {
            let x = image.width as f32 * 0.5 + (-75. + i as f32 * 50.) * image.height as f32 / 120.;
            let radius = image.height as f32 * 22. / 120.;
            let mut red = 0;
            for row in image.bytes.chunks_exact(image.width as usize * 4) {
                for px in row
                    .chunks_exact(4)
                    .skip((x - radius).max(0.) as usize)
                    .take((radius * 2.) as usize)
                {
                    if px[0] > 50
                        && px[0] as f32 > px[1] as f32 * 1.4
                        && px[0] as f32 > px[2] as f32 * 1.4
                    {
                        red += 1;
                    }
                }
            }
            ensure!(
                red > 100,
                "Missing coloured texture for {:?}: {red} red pixels",
                grade
            );
            println!(
                "PASS {:?} coloured Meta Essence at {time}s: {red} pixels",
                grade
            );
        }
        if let Some(before) = &first {
            ensure!(
                *before != image.bytes,
                "Meta Essence animation did not advance"
            );
        } else {
            first = Some(image.bytes);
        }
        set_default_camera();
        crate::viewer::save_capture(std::path::Path::new(&format!(
            "private/loot-textures-{time:.0}.png"
        )))?;
        next_frame().await;
    }
    let mut collected = Stats::default();
    collected
        .collected
        .extend(placed.iter().map(|p| p.id.clone()));
    clear_background(background);
    set_camera(&camera);
    let before = get_screen_data();
    art.draw(&placed, &collected, "placed", &atmosphere, eye, 2.);
    ensure!(
        before.bytes == get_screen_data().bytes,
        "Collected item remains visible"
    );
    // A wall must reject every pass; full fog must leave the background alone.
    for wall in [true, false] {
        clear_background(background);
        set_camera(&camera);
        if wall {
            let material = crate::character::skin_material()?;
            material.atmosphere(&atmosphere, eye);
            material.bind();
            draw_cube(vec3(0., -20., 48.), vec3(250., 2., 200.), None, WHITE);
            gl_use_default_material();
        }
        let before = get_screen_data();
        let fog = Atmosphere {
            distance: vec4(0.4, 0.3, 0.2, 1.),
            ..Default::default()
        };
        art.draw(
            &[],
            &stats,
            "render",
            if wall { &atmosphere } else { &fog },
            eye,
            0.,
        );
        ensure!(
            before.bytes == get_screen_data().bytes,
            "Meta Essence ignored {}",
            if wall { "wall depth" } else { "fog" }
        );
        set_default_camera();
        next_frame().await;
    }
    // Restoration creates another owner before the old scene is released.
    let specs = texture::read_materials(assets)?;
    let a = Art::load(assets, &specs)?;
    let b = Art::load(assets, &specs)?;
    ensure!(
        a.materials
            .iter()
            .zip(&b.materials)
            .all(|(x, y)| Rc::ptr_eq(x, y)),
        "Drop materials duplicated on reload"
    );
    println!("PASS dropped item save restore, animation, wall occlusion, additive/filter fog and shared GPU materials");
    // Exercise real collection notifications, rather than infer pickups from a
    // loaded inventory. Both entry bursts and frame-gated emitters are covered.
    let models: Vec<_> = GRADES
        .iter()
        .map(|g| g.model())
        .chain(["p_h1", "p_h2", "p_m1", "p_m2"])
        .collect();
    for model in models {
        let item = crate::inventory::Pickup {
            id: "effect:0".into(),
            model: model.into(),
            origin: Vec3::ZERO,
            kind: crate::inventory::PickupKind::Essence,
            amount: 25.,
        };
        art.clear_effects();
        art.collected(&item, 10.);
        clear_background(background);
        set_camera(&camera);
        let empty = get_screen_data();
        art.draw(&[], &Stats::default(), "effect", &atmosphere, eye, 10.15);
        let burst = get_screen_data();
        burst.export_png(&format!("private/item-fx-audit-20261001/{model}-burst.png"));
        ensure!(
            burst.bytes != empty.bytes,
            "Missing collection particles: {model}"
        );
        // Readback temporarily binds its capture texture. Compare actual paused
        // frames, letting the backend retire that binding before drawing again.
        set_default_camera();
        next_frame().await;
        clear_background(background);
        set_camera(&camera);
        art.draw(&[], &Stats::default(), "effect", &atmosphere, eye, 10.15);
        let paused = get_screen_data();
        paused.export_png(&format!(
            "private/item-fx-audit-20261001/{model}-paused.png"
        ));
        let delta = burst
            .bytes
            .iter()
            .zip(&paused.bytes)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0);
        ensure!(
            delta <= 1,
            "Pause changed item particles: {model} (max channel difference {delta})"
        );
        set_default_camera();
        crate::viewer::save_capture(std::path::Path::new(&format!(
            "private/item-fx-audit-20261001/{model}-pickup.png"
        )))?;
        next_frame().await;
        clear_background(background);
        set_camera(&camera);
        art.draw(&[], &Stats::default(), "effect", &atmosphere, eye, 14.1);
        ensure!(
            empty.bytes == get_screen_data().bytes,
            "Expired item particles remain: {model}"
        );
        art.collected(&item, 20.);
        art.clear_effects(); // The viewer calls this on every load and map switch.
        art.draw(&[], &Stats::default(), "effect", &atmosphere, eye, 20.15);
        ensure!(
            empty.bytes == get_screen_data().bytes,
            "Item effect survived restore reset: {model}"
        );
        println!("PASS {model} collection burst, paused clock, expiry and restore reset");
        set_default_camera();
        next_frame().await;
    }
    Ok(())
}
