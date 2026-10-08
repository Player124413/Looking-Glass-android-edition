//! One transparent queue for world, skin, pickup and effect geometry.
//! Packets own their uniforms: shared pipeline state never leaks across actors.
use crate::{
    assets::Assets,
    environment::Atmosphere,
    materials::{Blend, Stage},
    texture,
};
use anyhow::Result;
use macroquad::prelude::*;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
};

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Appearance {
    pub dissolve: f32,
    pub ghost: bool,
    pub power: f32,
}
pub struct Pass {
    pub mesh: Mesh,
    pub material: Rc<Material>,
    pub lit: bool,
    pub clamp: bool,
    pub cutout: f32,
    pub cutout_reference: f32,
    pub gain: Vec4,
    pub fog: f32,
    pub model: bool,
    pub appearance: Appearance,
    pub filter_fade: f32,
}
struct Group {
    passes: Vec<Pass>,
}
#[derive(Default)]
struct Frame {
    active: bool,
    camera: Vec3,
    direction: Vec3,
    time: f32,
    atmosphere: Atmosphere,
    fullbright: bool,
    appearance: Appearance,
    groups: Vec<Group>,
    vertices: usize,
    dropped: usize,
    retained: Vec<(Rc<Material>, Option<Texture2D>)>,
    projection: Option<Mat4>,
    last_configured: std::cell::Cell<Option<(*const Material, bool, bool)>>,
}
thread_local! {
    static FRAME: RefCell<Frame> = RefCell::default();
    static SKIN_OPACITY: std::cell::Cell<f32> = const { std::cell::Cell::new(1.) };
    static SURFACES: RefCell<Vec<(macroquad::miniquad::TextureId, Weak<Surface>)>> = RefCell::default();
}
const MAX_VERTICES: usize = 1_000_000;
pub fn copy_mesh(m: &Mesh) -> Mesh {
    Mesh {
        vertices: m.vertices.clone(),
        indices: m.indices.clone(),
        texture: m.texture.clone(),
    }
}
pub fn active() -> bool {
    FRAME.with(|f| f.borrow().active)
}
pub fn time() -> f32 {
    FRAME.with(|f| f.borrow().time)
}
pub fn projection() -> Option<Mat4> {
    FRAME.with(|f| f.borrow().projection)
}
/// Use the actual view matrix for conservative geometry and effect rejection,
/// including reflected cameras with an oblique aperture plane.
pub(crate) fn set_projection(matrix: Mat4) {
    FRAME.with(|f| f.borrow_mut().projection = Some(matrix));
}
pub fn projected_error(center: Vec3, radius: f32, error: f32) -> f32 {
    FRAME.with(|f| {
        let f = f.borrow();
        let Some(m) = f.projection else {
            return f32::INFINITY;
        };
        let clip = m * center.extend(1.);
        let distance = (clip.w - radius).max(0.001);
        // Projection row magnitude also handles rolled cameras without knowing FOV.
        let row = vec3(m.x_axis.y, m.y_axis.y, m.z_axis.y).length();
        error * row * screen_height() * 0.5 / distance
    })
}
pub fn appearance(a: Appearance) {
    FRAME.with(|f| f.borrow_mut().appearance = a);
}
pub fn begin(camera: Vec3, direction: Vec3, time: f32, atmosphere: &Atmosphere, fullbright: bool) {
    unsafe {
        get_internal_gl().flush();
    }
    FRAME.with(|f| {
        *f.borrow_mut() = Frame {
            active: true,
            camera,
            direction: direction.normalize_or_zero(),
            time,
            atmosphere: atmosphere.clone(),
            fullbright,
            ..Default::default()
        }
    });
}
pub fn begin_view(camera: &Camera3D, time: f32, atmosphere: &Atmosphere, fullbright: bool) {
    use macroquad::camera::Camera;
    begin(
        camera.position,
        camera.target - camera.position,
        time,
        atmosphere,
        fullbright,
    );
    set_projection(camera.matrix());
}
fn same_style(a: &Pass, b: &Pass) -> bool {
    Rc::ptr_eq(&a.material, &b.material)
        && a.mesh.texture.as_ref().map(Texture2D::raw_miniquad_id)
            == b.mesh.texture.as_ref().map(Texture2D::raw_miniquad_id)
        && a.lit == b.lit
        && a.clamp == b.clamp
        && a.cutout == b.cutout
        && a.cutout_reference == b.cutout_reference
        && a.gain == b.gain
        && a.fog == b.fog
        && a.model == b.model
        && a.appearance == b.appearance
        && a.filter_fade == b.filter_fade
}
fn outside(points: [Vec4; 3]) -> bool {
    (0..3).any(|axis| points.iter().all(|p| p[axis] < -p.w) || points.iter().all(|p| p[axis] > p.w))
}
fn draw_pass(p: &Pass, indices: Option<&[u16]>) {
    FRAME.with(|f| {
        let f = f.borrow();
        let m = &p.material;
        let lit_active = !f.fullbright && p.lit;
        let config_key = (Rc::as_ptr(m), p.model, lit_active);
        if !crate::android::is_android() || f.last_configured.get() != Some(config_key) {
            f.atmosphere.apply(m, f.camera);
            m.set_uniform("Fullbright", if f.fullbright { 1_f32 } else { 0. });
            m.set_uniform("SkyFade", 0_f32);
            crate::lighting::apply(m, p.model, lit_active);
            f.last_configured.set(Some(config_key));
        }
        m.set_uniform("Lit", if p.lit { 1_f32 } else { 0. });
        m.set_uniform("ClampUV", if p.clamp { 1_f32 } else { 0. });
        m.set_uniform("Cutout", p.cutout);
        m.set_uniform("CutoutReference", p.cutout_reference);
        m.set_uniform("Gain", p.gain);
        m.set_uniform("FogMode", p.fog);
        m.set_uniform("FxModel", if p.model { 1_f32 } else { 0. });
        m.set_uniform(
            "FxAppearance",
            vec4(
                p.appearance.dissolve,
                if p.appearance.ghost { 1. } else { 0. },
                p.appearance.power,
                p.filter_fade,
            ),
        );
        gl_use_material(m);
        if let Some(indices) = indices {
            // Interleaved surfaces can contribute just one triangle per run.
            // Uploading the whole source mesh for each run is quadratic in mesh
            // size; compact this span and stay within the u16 index range.
            let gl = unsafe { get_internal_gl() };
            gl.quad_gl.texture(p.mesh.texture.as_ref());
            gl.quad_gl.draw_mode(DrawMode::Triangles);
            for chunk in indices.chunks(65532) {
                let vertices: Vec<_> = chunk.iter().map(|&i| p.mesh.vertices[i as usize]).collect();
                let compact: Vec<u16> = (0..vertices.len() as u16).collect();
                gl.quad_gl.geometry(&vertices, &compact);
            }
        } else {
            draw_mesh(&p.mesh);
        }
    });
}
pub fn submit(passes: Vec<Pass>, transparent: bool) {
    if passes.is_empty() {
        return;
    }
    if !transparent {
        FRAME.with(|f| f.borrow().last_configured.set(None));
        for p in &passes {
            draw_pass(p, None);
        }
        FRAME.with(|f| {
            f.borrow_mut()
                .retained
                .extend(passes.into_iter().map(|p| (p.material, p.mesh.texture)))
        });
        return;
    }
    FRAME.with(|f| {
        let mut f = f.borrow_mut();
        let count = passes.iter().map(|p| p.mesh.vertices.len()).sum::<usize>();
        if f.vertices.saturating_add(count) > MAX_VERTICES {
            f.dropped += 1;
            return;
        }
        f.vertices += count;
        f.groups.push(Group { passes });
    });
}
pub fn finish() -> (usize, usize) {
    let (groups, camera, direction, dropped, projection) = FRAME.with(|f| {
        let mut f = f.borrow_mut();
        f.last_configured.set(None);
        (
            std::mem::take(&mut f.groups),
            f.camera,
            f.direction,
            f.dropped,
            f.projection,
        )
    });
    // Sorting triangles rather than whole objects fixes effects on opposite sides
    // of one water surface. All layers of a triangle stay in authored order.
    let mut order = Vec::new();
    for (g, group) in groups.iter().enumerate() {
        let mesh = &group.passes[0].mesh;
        for (t, triangle) in mesh.indices.chunks_exact(3).enumerate() {
            if projection.is_some_and(|m| {
                outside([
                    m * mesh.vertices[triangle[0] as usize].position.extend(1.),
                    m * mesh.vertices[triangle[1] as usize].position.extend(1.),
                    m * mesh.vertices[triangle[2] as usize].position.extend(1.),
                ])
            }) {
                continue;
            }
            let center = triangle
                .iter()
                .map(|&i| mesh.vertices[i as usize].position)
                .sum::<Vec3>()
                / 3.;
            order.push(((center - camera).dot(direction), g, t));
        }
    }
    order.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    crate::render::depth_read_only(|| {
        let mut start = 0;
        while start < order.len() {
            let group = order[start].1;
            let mut end = start + 1;
            let passes = &groups[group].passes;
            if passes.len() == 1 {
                let p = &passes[0];
                while end < order.len()
                    && end - start < 20000
                    && groups[order[end].1].passes.len() == 1
                    && same_style(p, &groups[order[end].1].passes[0])
                {
                    end += 1;
                }
                let mut vertices = Vec::with_capacity((end - start) * 3);
                for &(_, g, t) in &order[start..end] {
                    let mesh = &groups[g].passes[0].mesh;
                    vertices.extend(
                        mesh.indices[t * 3..t * 3 + 3]
                            .iter()
                            .map(|&i| mesh.vertices[i as usize]),
                    );
                }
                let indices = (0..vertices.len() as u16).collect();
                draw_pass(
                    &Pass {
                        cutout_reference: p.cutout_reference,
                        mesh: Mesh {
                            vertices,
                            indices,
                            texture: p.mesh.texture.clone(),
                        },
                        material: p.material.clone(),
                        lit: p.lit,
                        clamp: p.clamp,
                        cutout: p.cutout,
                        gain: p.gain,
                        fog: p.fog,
                        model: p.model,
                        appearance: p.appearance,
                        filter_fade: p.filter_fade,
                    },
                    None,
                );
                start = end;
                continue;
            }
            while end < order.len() && order[end].1 == group {
                end += 1;
            }
            let mut indices = Vec::with_capacity((end - start) * 3);
            for &(_, _, t) in &order[start..end] {
                indices.extend_from_slice(&passes[0].mesh.indices[t * 3..t * 3 + 3]);
            }
            for p in passes {
                draw_pass(p, Some(&indices));
            }
            start = end;
        }
    });
    unsafe {
        get_internal_gl().flush();
    }
    FRAME.with(|f| {
        let mut f = f.borrow_mut();
        f.active = false;
        f.retained.clear();
    });
    gl_use_default_material();
    (order.len(), dropped)
}
pub fn fog(blend: Blend) -> f32 {
    match blend {
        Blend::Add | Blend::AlphaAdd | Blend::ColorAdd => 1.,
        Blend::Filter => 2.,
        Blend::DoubleFilter => 3.,
        Blend::AlphaColor => 4.,
        _ => 0.,
    }
}
pub fn pass(
    mesh: Mesh,
    stage: &Stage,
    model: bool,
    lit: bool,
    appearance: Appearance,
) -> Result<Pass> {
    pass_at(mesh, stage, model, lit, appearance, time())
}
fn pass_at(
    mesh: Mesh,
    stage: &Stage,
    model: bool,
    lit: bool,
    appearance: Appearance,
    clock: f32,
) -> Result<Pass> {
    let rgb = stage.constant_rgb.unwrap_or(Vec3::ONE)
        * stage
            .rgb
            .as_ref()
            .map_or(1., |w| w.sample(clock))
            .clamp(0., 2.);
    Ok(Pass {
        mesh,
        material: crate::render::world_material(
            if crate::android::is_android()
                && stage.blend == Blend::Opaque
                && !stage.depth_equal
                && stage.alpha_test == 0
                && appearance.dissolve <= 0.
                && !appearance.ghost
            {
                crate::render::SOLID_OPAQUE_MATERIAL
            } else {
                crate::render::blend_index(stage.blend)
                    + if stage.depth_equal {
                        crate::render::BLEND_COUNT
                    } else {
                        0
                    }
            },
        )?,
        lit,
        clamp: stage.clamp,
        cutout: stage.alpha_test as f32,
        cutout_reference: stage.alpha_reference,
        gain: rgb.extend(
            stage
                .alpha
                .as_ref()
                .map(|w| w.sample(clock))
                .or(stage.constant_alpha)
                .unwrap_or(1.)
                .clamp(0., 1.),
        ),
        fog: fog(stage.blend),
        model,
        appearance,
        filter_fade: 0.,
    })
}
pub fn effect(mesh: &Mesh, blend: Blend) {
    if !active() {
        draw_mesh(mesh);
        return;
    }
    let mesh = copy_mesh(mesh);
    // Source-alpha/additive is equivalent to the former shader's premultiply,
    // including the texture alpha (not only the vertex fade).
    let blend = if blend == Blend::Add {
        Blend::AlphaAdd
    } else {
        blend
    };
    if let Ok(p) = pass(
        mesh,
        &Stage {
            blend,
            ..Default::default()
        },
        true,
        false,
        Appearance::default(),
    ) {
        submit(vec![p], true);
    }
}
pub fn line(a: Vec3, b: Vec3, color: Color) {
    if !active() {
        draw_line_3d(a, b, color);
        return;
    }
    let eye = FRAME.with(|f| f.borrow().camera);
    let side = (eye - (a + b) * 0.5).cross(b - a).normalize_or_zero() * 0.5;
    let vertices = [a - side, b - side, b + side, a + side]
        .into_iter()
        .map(|position| Vertex {
            position,
            uv: Vec2::ZERO,
            normal: Vec4::ZERO,
            color: color.into(),
        })
        .collect();
    effect(
        &Mesh {
            vertices,
            indices: vec![0, 1, 2, 0, 2, 3],
            texture: None,
        },
        Blend::Alpha,
    );
}
pub fn sphere(center: Vec3, radius: f32, texture: Option<&Texture2D>, color: Color) {
    if !active() {
        draw_sphere(center, radius, texture, color);
        return;
    }
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for y in 0..=8 {
        let a = std::f32::consts::PI * y as f32 / 8.;
        for x in 0..=12 {
            let b = std::f32::consts::TAU * x as f32 / 12.;
            vertices.push(Vertex {
                position: center + vec3(a.sin() * b.cos(), a.sin() * b.sin(), a.cos()) * radius,
                uv: vec2(x as f32 / 12., y as f32 / 8.),
                normal: Vec4::ZERO,
                color: color.into(),
            });
        }
    }
    for y in 0..8 {
        for x in 0..12 {
            let i = y * 13 + x;
            indices.extend([i, i + 1, i + 13, i + 1, i + 14, i + 13]);
        }
    }
    effect(
        &Mesh {
            vertices,
            indices,
            texture: texture.cloned(),
        },
        Blend::Alpha,
    );
}
struct Layer {
    stage: Stage,
    textures: Vec<Texture2D>,
}
pub struct Surface {
    layers: Vec<Layer>,
    transparent: bool,
}
pub fn register(
    assets: &mut Assets,
    base: &Texture2D,
    names: &[String],
    specs: &BTreeMap<String, texture::MaterialSpec>,
) -> Result<Rc<Surface>> {
    let spec = names.iter().find_map(|n| {
        let key = n.to_lowercase();
        specs
            .get(&key)
            .or_else(|| key.rsplit_once('.').and_then(|(s, _)| specs.get(s)))
    });
    let stages = spec.map(|s| s.stages.clone()).unwrap_or_default();
    let transparent = spec.is_some_and(|s| s.transparent)
        || stages
            .first()
            .is_some_and(|s| s.blend != Blend::Opaque && s.alpha_test == 0 && !s.depth_write);
    let mut layers = Vec::new();
    for stage in stages.into_iter().filter(|s| {
        !s.unsupported && !s.images.is_empty() && !s.images.iter().any(|n| n == "$lightmap")
    }) {
        let mut textures = Vec::new();
        for name in &stage.images {
            if name == "$whiteimage" || name == "*white" {
                textures.push(Texture2D::from_rgba8(1, 1, &[255; 4]));
                continue;
            }
            let Some(path) = texture::resolve(assets, name, &BTreeMap::new()) else {
                continue;
            };
            let image = texture::decode(assets, &path)?;
            let tex = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
            tex.set_filter(FilterMode::Linear);
            // Model shaders tile too (notably the Queen's tentacle and base).
            // CPU UV modifiers may leave [0,1]; clamping samples one edge texel.
            if !stage.clamp {
                unsafe {
                    get_internal_gl().quad_context.texture_set_wrap(
                        tex.raw_miniquad_id(),
                        macroquad::miniquad::TextureWrap::Repeat,
                        macroquad::miniquad::TextureWrap::Repeat,
                    );
                }
            }
            textures.push(tex);
        }
        if textures.len() == stage.images.len() {
            layers.push(Layer { stage, textures });
        }
    }
    if layers.is_empty() {
        layers.push(Layer {
            stage: Stage {
                alpha_test: 4,
                ..Default::default()
            },
            textures: vec![base.clone()],
        });
    }
    let surface = Rc::new(Surface {
        layers,
        transparent,
    });
    SURFACES.with(|s| {
        let mut s = s.borrow_mut();
        s.retain(|(_, v)| v.strong_count() > 0);
        s.push((base.raw_miniquad_id(), Rc::downgrade(&surface)));
    });
    Ok(surface)
}
pub fn skin(mesh: &Mesh) {
    SKIN_OPACITY.with(|a| skin_timed(mesh, None, a.get()));
}

/// Scope entity translucency to one actor, including its attachment meshes.
pub fn with_skin_opacity(alpha: f32, draw: impl FnOnce()) {
    struct Restore(f32);
    impl Drop for Restore {
        fn drop(&mut self) {
            SKIN_OPACITY.with(|a| a.set(self.0));
        }
    }
    let _restore = Restore(SKIN_OPACITY.with(|a| a.replace(alpha.clamp(0., 1.))));
    draw();
}

/// Spawned TAN effects animate from their birth clock and carry entity alpha.
pub(crate) fn skin_effect(mesh: &Mesh, age: f32, alpha: f32) {
    skin_timed(mesh, Some(age), alpha.clamp(0., 1.));
}

fn skin_timed(mesh: &Mesh, clock: Option<f32>, alpha: f32) {
    if !active() {
        draw_mesh(mesh);
        return;
    }
    let surface = mesh.texture.as_ref().and_then(|t| {
        SURFACES.with(|s| {
            s.borrow()
                .iter()
                .rev()
                .find(|(id, _)| *id == t.raw_miniquad_id())
                .and_then(|(_, v)| v.upgrade())
        })
    });
    let (camera, time, appearance) = FRAME.with(|f| {
        let f = f.borrow();
        (f.camera, f.time, f.appearance)
    });
    let time = clock.unwrap_or(time);
    let Some(surface) = surface else {
        let mut copy = copy_mesh(mesh);
        for v in &mut copy.vertices {
            v.color[3] = (v.color[3] as f32 * alpha) as u8;
        }
        if let Ok(p) = pass(
            copy,
            &Stage {
                alpha_test: if alpha < 1. { 0 } else { 4 },
                blend: if alpha < 1. {
                    Blend::Alpha
                } else {
                    Blend::Opaque
                },
                ..Default::default()
            },
            true,
            true,
            appearance,
        ) {
            submit(vec![p], alpha < 1.);
        }
        return;
    };
    let mut passes = Vec::new();
    for layer in &surface.layers {
        let mut copy = copy_mesh(mesh);
        copy.texture = Some(
            layer.textures[if layer.stage.images.is_empty() {
                0
            } else {
                layer.stage.frame(time)
            }]
            .clone(),
        );
        let needs_normal = layer.stage.environment || layer.stage.dot_alpha.is_some();
        let needs_eye_dir = layer.stage.dot_alpha.is_some();
        let static_uv = layer.stage.vector_uv.is_none()
            && !layer.stage.environment
            && layer.stage.mods.is_empty();
        if static_uv && !needs_eye_dir {
            if alpha < 1. || !layer.stage.vertex_alpha {
                for v in &mut copy.vertices {
                    let base_a = if layer.stage.vertex_alpha {
                        v.color[3] as f32 / 255.
                    } else {
                        1.
                    };
                    v.color[3] = (255. * (alpha * base_a).clamp(0., 1.)) as u8;
                }
            }
        } else {
            for v in &mut copy.vertices {
                let n = if needs_normal {
                    v.normal.truncate().normalize_or_zero()
                } else {
                    Vec3::ZERO
                };
                if !static_uv {
                    v.uv = layer.stage.view_uv(v.uv, v.position, n, camera, time);
                }
                let eye_dir = if needs_eye_dir {
                    (camera - v.position).normalize_or_zero()
                } else {
                    Vec3::ZERO
                };
                v.color[3] = (255. * alpha * layer.stage.opacity(n, eye_dir, v.color[3])) as u8;
            }
        }
        let mut stage = layer.stage.clone();
        if clock.is_none() && alpha < 1. {
            if stage.blend == Blend::Opaque {
                stage.blend = Blend::Alpha;
            }
            stage.alpha_test = 0;
            stage.depth_write = false;
            stage.depth_equal = false;
        }
        if let Ok(mut p) = pass_at(copy, &stage, true, !stage.identity, appearance, time) {
            // Multiplicative layers ignore source alpha. Fade their colour to
            // the blend's neutral value so a hidden ghost leaves no silhouette.
            p.filter_fade = match stage.blend {
                Blend::Filter | Blend::Factors(0, 2) | Blend::Factors(4, 0) => 1. - alpha,
                Blend::DoubleFilter | Blend::Factors(4, 2) => alpha - 1.,
                _ => 0.,
            };
            passes.push(p);
        }
    }
    submit(
        passes,
        surface.transparent || (clock.is_none() && alpha < 1.),
    );
}

pub async fn check(assets: &mut Assets) -> Result<()> {
    use anyhow::ensure;
    std::fs::create_dir_all("private/render-fx")?;
    let mut scene = crate::render::Scene::load(assets, "skool1")?;
    let white = Texture2D::from_rgba8(1, 1, &[255; 4]);
    let quad = |z: f32, color: Color| Mesh {
        vertices: [
            vec3(-1., -1., z),
            vec3(1., -1., z),
            vec3(1., 1., z),
            vec3(-1., 1., z),
        ]
        .into_iter()
        .zip([vec2(0., 1.), vec2(1., 1.), vec2(1., 0.), vec2(0., 0.)])
        .map(|(position, uv)| Vertex {
            position,
            uv,
            color: color.into(),
            normal: Vec4::ZERO,
        })
        .collect(),
        indices: vec![0, 1, 2, 0, 2, 3],
        texture: Some(white.clone()),
    };
    let camera = Camera3D {
        position: vec3(0., 0., 10.),
        target: Vec3::ZERO,
        up: Vec3::Y,
        projection: Projection::Orthographics,
        fovy: 4.,
        z_near: 0.1,
        z_far: 50.,
        ..Default::default()
    };
    crate::lighting::select(vec![], camera.position, &scene.world);
    for order in [[0, 1, 2], [2, 0, 1], [1, 2, 0]] {
        clear_background(BLACK);
        set_camera(&camera);
        begin(camera.position, -Vec3::Z, 0., &Atmosphere::default(), false);
        submit(
            vec![pass(
                quad(0., WHITE),
                &Stage::default(),
                true,
                false,
                Default::default(),
            )?],
            false,
        );
        for i in order {
            let color = [
                Color::new(0., 0., 1., 0.5),
                Color::new(1., 0., 0., 0.5),
                Color::new(0., 1., 0., 0.5),
            ][i];
            effect(&quad(i as f32 + 1., color), Blend::Alpha);
        }
        let (_, dropped) = finish();
        ensure!(dropped == 0, "Transparent queue dropped geometry");
        let frame = get_screen_data();
        let pixel = frame.get_pixel(frame.width as u32 / 2, frame.height as u32 / 2);
        ensure!(
            (pixel.r - 0.375).abs() < 0.025
                && (pixel.g - 0.625).abs() < 0.025
                && (pixel.b - 0.25).abs() < 0.025,
            "Cross-category order changed compositing: {order:?} {pixel:?}"
        );
        next_frame().await;
    }
    println!("PASS triangle-sorted water/actor/pickup/effect alpha ordering across three submission permutations");
    let mut blue = quad(0., WHITE);
    blue.texture = Some(Texture2D::from_rgba8(1, 1, &[0, 0, 255, 255]));
    let mut red = quad(1., WHITE);
    red.texture = Some(Texture2D::from_rgba8(1, 1, &[255, 0, 0, 255]));
    for alpha in [0_f32, 0.12, 0.75, 1.] {
        clear_background(BLACK);
        set_camera(&camera);
        begin(camera.position, -Vec3::Z, 0., &Atmosphere::default(), true);
        skin(&blue);
        with_skin_opacity(alpha, || skin(&red));
        finish();
        let frame = get_screen_data();
        let p = frame.get_pixel(frame.width as u32 / 2, frame.height as u32 / 2);
        ensure!(
            (p.r - alpha).abs() < 0.025 && (p.b - (1. - alpha)).abs() < 0.025,
            "Entity alpha was discarded or ignored: {alpha}/{p:?}"
        );
        next_frame().await;
        clear_background(BLACK);
        set_camera(&camera);
        begin(camera.position, -Vec3::Z, 0., &Atmosphere::default(), true);
        skin(&blue);
        finish();
        let frame = get_screen_data();
        let p = frame.get_pixel(frame.width as u32 / 2, frame.height as u32 / 2);
        ensure!(p.b > 0.98, "Entity opacity leaked into another draw");
        next_frame().await;
    }
    let layers = crate::materials::parse("ghost_fixture\n{\nsurfaceparm trans\n{\nmap $whiteimage\nblendfunc blend\nrgbgen const 1 0 0\n}\n{\nmap $whiteimage\nblendfunc filter\nrgbgen const .5 .5 .5\n}\n}")?;
    let ghost_texture = Texture2D::from_rgba8(1, 1, &[255; 4]);
    let owner = register(assets, &ghost_texture, &["ghost_fixture".into()], &layers)?;
    let mut ghost = quad(1., WHITE);
    ghost.texture = Some(ghost_texture);
    for alpha in [0_f32, 0.12, 0.75, 1.] {
        clear_background(BLACK);
        set_camera(&camera);
        begin(camera.position, -Vec3::Z, 0., &Atmosphere::default(), true);
        skin(&blue);
        with_skin_opacity(alpha, || skin(&ghost));
        finish();
        let frame = get_screen_data();
        let p = frame.get_pixel(frame.width as u32 / 2, frame.height as u32 / 2);
        let filter = 1. - 0.5 * alpha;
        ensure!(
            (p.r - alpha * filter).abs() < 0.025 && (p.b - (1. - alpha) * filter).abs() < 0.025,
            "Multiplicative ghost layer ignored entity fade: {alpha}/{p:?}"
        );
        next_frame().await;
    }
    drop(owner);
    println!("PASS Phantasmagoria opacity, low-alpha visibility and independent actor state");
    clear_background(BLACK);
    set_camera(&camera);
    begin(camera.position, -Vec3::Z, 0., &Atmosphere::default(), false);
    submit(
        vec![pass(
            quad(4., Color::new(1., 0., 0., 1.)),
            &Stage::default(),
            true,
            false,
            Default::default(),
        )?],
        false,
    );
    effect(&quad(1., BLUE), Blend::Alpha);
    finish();
    let frame = get_screen_data();
    let pixel = frame.get_pixel(frame.width as u32 / 2, frame.height as u32 / 2);
    ensure!(
        pixel.r > 0.98 && pixel.b < 0.02,
        "Transparent geometry bypassed opaque depth: {pixel:?}"
    );
    next_frame().await;
    println!("PASS queued transparency respects solid depth");
    let layered = crate::materials::parse("fixture\n{\nsurfaceparm trans\n{\nmap $whiteimage\nblendfunc add\nrgbgen const .2 0 0\n}\n{\nmap $whiteimage\nblendfunc filter\nrgbgen const .5 1 1\n}\n{\nmap $whiteimage\nblendfunc add\nrgbgen const 0 0 .3\n}\n}")?;
    let owner = register(assets, &white, &["fixture".into()], &layered)?;
    clear_background(Color::new(0.2, 0.2, 0.2, 1.));
    set_camera(&camera);
    begin(camera.position, -Vec3::Z, 0., &Atmosphere::default(), false);
    skin(&quad(1., WHITE));
    finish();
    let frame = get_screen_data();
    let pixel = frame.get_pixel(frame.width as u32 / 2, frame.height as u32 / 2);
    ensure!(
        (pixel.r - 0.2).abs() < 0.025
            && (pixel.g - 0.2).abs() < 0.025
            && (pixel.b - 0.5).abs() < 0.025,
        "Skin/pickup layer order changed: {pixel:?}"
    );
    drop(owner);
    next_frame().await;
    let mut translucent = quad(1., WHITE);
    translucent.texture = Some(Texture2D::from_rgba8(1, 1, &[255, 0, 0, 64]));
    clear_background(BLACK);
    set_camera(&camera);
    begin(camera.position, -Vec3::Z, 0., &Atmosphere::default(), false);
    effect(&translucent, Blend::Add);
    finish();
    let frame = get_screen_data();
    let pixel = frame.get_pixel(frame.width as u32 / 2, frame.height as u32 / 2);
    ensure!(
        (pixel.r - 0.25).abs() < 0.025 && pixel.g < 0.02,
        "Additive sprite lost texture alpha: {pixel:?}"
    );
    next_frame().await;
    println!("PASS registered model layers and additive sprite texture alpha");
    // Real light declarations and GPU contribution, including the models-only mask.
    let lights = vec![crate::lighting::Light {
        position: vec3(0., 0., 2.),
        color: vec3(0.4, 0., 0.),
        radius: 10.,
        only_models: false,
        flare: false,
    }];
    crate::lighting::fixture_lights(lights);
    clear_background(BLACK);
    set_camera(&camera);
    begin(camera.position, -Vec3::Z, 0., &Atmosphere::default(), false);
    submit(
        vec![pass(
            quad(0., Color::new(0.2, 0.2, 0.2, 1.)),
            &Stage::default(),
            true,
            true,
            Default::default(),
        )?],
        false,
    );
    finish();
    let image = get_screen_data();
    let pixel = image.get_pixel(image.width as u32 / 2, image.height as u32 / 2);
    ensure!(
        pixel.r > pixel.g + 0.25 && pixel.g > 0.15,
        "Dynamic light did not reach GPU: {pixel:?}"
    );
    crate::viewer::save_capture(std::path::Path::new(
        "private/render-fx/light-regression.png",
    ))?;
    next_frame().await;
    println!("PASS dynamic-light colour/radius reaches actual world shader");
    let authored = crate::particles::declared_lights(assets, "croquetball")?;
    ensure!(
        !authored.is_empty(),
        "Missing supplied croquet light declaration"
    );
    for model in [false, true] {
        crate::lighting::fixture_lights(vec![crate::lighting::Light {
            position: vec3(0., 0., 2.),
            color: vec3(0.4, 0., 0.),
            radius: 10.,
            only_models: true,
            flare: false,
        }]);
        clear_background(BLACK);
        set_camera(&camera);
        begin(camera.position, -Vec3::Z, 0., &Atmosphere::default(), false);
        submit(
            vec![pass(
                quad(0., Color::new(0.2, 0.2, 0.2, 1.)),
                &Stage::default(),
                model,
                true,
                Default::default(),
            )?],
            false,
        );
        finish();
        let f = get_screen_data();
        let p = f.get_pixel(f.width as u32 / 2, f.height as u32 / 2);
        ensure!(
            if model {
                p.r > p.g + 0.25
            } else {
                (p.r - p.g).abs() < 0.025
            },
            "models-only light mask: {model} {p:?}"
        );
        next_frame().await;
    }
    let floor =
        crate::collision::World::fixture(&[(vec3(-200., -200., -10.), vec3(200., 200., 0.))]);
    let shadow_camera = Camera3D {
        position: Vec3::Z * 100.,
        target: Vec3::ZERO,
        up: Vec3::Y,
        projection: Projection::Orthographics,
        fovy: 80.,
        z_near: 0.1,
        z_far: 200.,
        ..Default::default()
    };
    crate::lighting::fixture_lights(vec![]);
    for height in [2., 200.] {
        clear_background(WHITE);
        set_camera(&shadow_camera);
        begin(
            shadow_camera.position,
            -Vec3::Z,
            0.,
            &Atmosphere::default(),
            false,
        );
        crate::lighting::shadow(&floor, Vec3::Z * height, 22.);
        finish();
        let f = get_screen_data();
        let p = f.get_pixel(f.width as u32 / 2, f.height as u32 / 2);
        ensure!(
            if height < 10. { p.r < 0.8 } else { p.r > 0.98 },
            "Projected source shadow height: {height} {p:?}"
        );
        next_frame().await;
    }
    let flare = crate::particles::declared_lights(assets, "fx_emitter_lensflare")?;
    ensure!(
        flare.len() == 1 && flare[0].flare,
        "Supplied lens flare declaration missing"
    );
    for occluded in [false, true] {
        let wall =
            crate::collision::World::fixture(&[(vec3(-20., -20., 20.), vec3(20., 20., 21.))]);
        crate::lighting::select(
            flare.clone(),
            shadow_camera.position,
            if occluded { &wall } else { &floor },
        );
        clear_background(BLACK);
        set_camera(&shadow_camera);
        begin(
            shadow_camera.position,
            -Vec3::Z,
            0.,
            &Atmosphere::default(),
            false,
        );
        crate::lighting::flares(shadow_camera.position, -Vec3::Z);
        finish();
        let f = get_screen_data();
        let p = f.get_pixel(f.width as u32 / 2, f.height as u32 / 2);
        ensure!(
            if occluded { p.b < 0.02 } else { p.b > 0.1 },
            "Source flare occlusion: {occluded} {p:?}"
        );
        next_frame().await;
    }
    println!("PASS supplied light declarations, models-only mask, projected shadow height and flare wall occlusion");
    let mut timings = Vec::new();
    for name in ["skool1", "potears1", "pandemonium"] {
        scene = crate::render::Scene::load(assets, name)?;
        let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
        interactions.set_entry(assets, &scene.map, name, None)?;
        let mut steam = crate::particles::Steam::load(assets, &scene.map)?;
        steam.animate(1.);
        steam.sync(&interactions.event_world);
        let eye = scene.map.spawn().0 + vec3(0., 0., 50.);
        for _ in 0..60 {
            steam.update(1. / 60., eye, &scene.world);
        }
        let direction = Vec3::Y;
        for mode in ["legacy", "queue", "queue_and_light"] {
            let enabled = mode == "queue_and_light";
            let mut total = 0.;
            let mut samples = Vec::new();
            let mut visible_lights = 0;
            for i in 0..8 {
                clear_background(scene.atmosphere.background());
                let view = Camera3D {
                    position: eye,
                    target: eye + direction,
                    up: Vec3::Z,
                    z_near: 2.,
                    z_far: 30000.,
                    fovy: 75f32.to_radians(),
                    ..Default::default()
                };
                set_camera(&view);
                let start = std::time::Instant::now();
                let mut lights = steam.lights();
                // A named fixture adds an actual authored projectile light ahead of
                // this fixed camera, so the on/off cost is measured even in unlit maps.
                lights.extend(crate::lighting::croquet(eye + direction * 32.));
                crate::lighting::select(if enabled { lights } else { vec![] }, eye, &scene.world);
                visible_lights = crate::lighting::count();
                if mode != "legacy" {
                    begin_view(&view, 1., &scene.atmosphere, false);
                }
                let transforms = interactions.transforms();
                scene.draw(eye, 1., false, false, &transforms);
                scene.draw_with_particles(eye, direction, 1., false, &transforms, &steam);
                let (_, dropped) = if mode != "legacy" { finish() } else { (0, 0) };
                ensure!(dropped == 0, "Queue budget exceeded in {name}");
                unsafe {
                    get_internal_gl().flush();
                    macroquad::miniquad::gl::glFinish();
                }
                if i > 1 {
                    let ms = start.elapsed().as_secs_f64() * 1000.;
                    total += ms;
                    samples.push(ms);
                }
                if enabled && i == 7 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/render-fx/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
            timings.push(
                serde_json::json!({"map":name,"mode":mode,"authored_projectile_fixture":enabled,"visible_lights":visible_lights,"mean_cpu_gpu_ms":total/6.,"samples_ms":samples}),
            );
        }
    }
    std::fs::write(
        "private/render-fx/performance.json",
        serde_json::to_vec_pretty(&timings)?,
    )?;
    println!("PASS three real-map particle/transparency fixtures; synchronized CPU+GPU timings: {timings:?}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clip_rejection_preserves_crossing_triangles() {
        assert!(outside([
            vec4(2., 0., 0., 1.),
            vec4(3., 0., 0., 1.),
            vec4(2., 1., 0., 1.)
        ]));
        assert!(!outside([
            vec4(-2., 0., 0., 1.),
            vec4(2., 0., 0., 1.),
            vec4(0., 2., 0., 1.)
        ]));
        assert!(!outside([
            vec4(0., 0., -2., 1.),
            vec4(0., 0., 0., 1.),
            vec4(1., 0., 0., 1.)
        ]));
    }
}
