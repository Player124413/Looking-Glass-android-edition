//! Linked camera apertures, with an independent PVS and a clipped remote view.
//! Original brush transforms own visibility; no gameplay or saved cast is added.
use super::*;
use crate::{
    interaction::vector,
    skeletal::{Animation, Definition, Skeleton, Transform},
    weapons::Prop,
};
use anyhow::ensure;
use macroquad::camera::Camera;

pub(super) struct Portal {
    pub surface: usize,
    model: usize,
    center: Vec3,
    normal: Vec3,
    destination: Vec3,
    forward: Vec3,
    mesh: Mesh,
    local: Vec<Vec3>,
    target: Option<RenderTarget>,
    material: Material,
    ready: bool,
    pupil: Option<Pupil>,
    mirror: bool,
    cat: Option<crate::npc::Puppet>,
    actor: Option<crate::level::ReflectionActor>,
    extra: Vec<(usize, Vec3, Quat)>,
    hide: Vec<usize>,
}
struct Pupil {
    art: Prop,
    path: Vec<Vec3>,
    speed: f32,
    scale: f32,
}
impl Pupil {
    fn load(
        assets: &mut Assets,
        map: &Bsp,
        world: &World,
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let actor = map
            .entities
            .iter()
            .find(|e| {
                e.get("classname")
                    .is_some_and(|s| s == "Characters_InsaneChild_Muzzle")
                    && e.get("target").is_some_and(|s| s == "t171")
            })
            .context("Missing school portal pupil")?;
        let mut path = Vec::new();
        let first = actor["target"].as_str();
        let mut name = first;
        loop {
            ensure!(path.len() < 16, "Portal pupil path does not close");
            let node = map
                .entities
                .iter()
                .find(|e| e.get("targetname").is_some_and(|s| s == name))
                .context("Missing portal pupil waypoint")?;
            let point = node
                .get("origin")
                .and_then(|s| vector(s))
                .context("Invalid pupil waypoint")?;
            path.push(
                world
                    .actor_footing(
                        point + Vec3::Z * 8.,
                        Vec3::Z * 30.,
                        vec3(16., 16., 30.),
                        64.,
                    )
                    .context("Portal pupil has no floor")?,
            );
            name = node.get("target").context("Broken portal pupil path")?;
            if name == first {
                break;
            }
        }
        let model = "c_insanechild_muzzle";
        let def = Definition::load(assets, &format!("models/{model}.tik"))?;
        let bones = Skeleton::parse(&assets.read(&format!("{}/{}", def.path, def.model))?)?
            .bones
            .len();
        let clip = Animation::parse(
            &assets.read(&format!("{}/{}", def.path, def.animations["walk"]))?,
            bones,
        )?;
        let scale = actor
            .get("scale")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.);
        Ok(Self {
            art: Prop::load_animation(assets, model, "walk", specs)?,
            path,
            speed: clip.distance * def.scale * scale / clip.duration(),
            scale,
        })
    }
    fn pose(&self, time: f32) -> Transform {
        let segments = self
            .path
            .iter()
            .copied()
            .zip(self.path.iter().copied().cycle().skip(1))
            .map(|(a, b)| (a, b, a.distance(b)))
            .collect::<Vec<_>>();
        let total: f32 = segments.iter().map(|s| s.2).sum();
        let mut distance = (time.max(0.) * self.speed).rem_euclid(total.max(0.001));
        for (a, b, length) in segments {
            if distance <= length {
                let direction = b - a;
                return Transform {
                    translation: a.lerp(b, distance / length.max(0.001)),
                    rotation: Quat::from_rotation_z(direction.y.atan2(direction.x)),
                };
            }
            distance -= length;
        }
        Transform {
            translation: self.path[0],
            rotation: Quat::IDENTITY,
        }
    }
}

fn frame(forward: Vec3) -> Quat {
    let right = forward.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::X);
    let up = right.cross(forward).normalize();
    Quat::from_mat3(&Mat3::from_cols(right, forward, up))
}

/// Replace the near plane by the destination aperture. This prevents the back
/// of the remote room from covering the view when the virtual eye is outside it.
fn clipped_projection(projection: Mat4, view: Mat4, plane: Vec4) -> Mat4 {
    let plane = view.inverse().transpose() * plane;
    let inverse = projection.inverse();
    // Choose the far corner in clip space. A mirror reverses the projection's
    // horizontal axis, so view-space signs select the opposite corner and can
    // invert the clipping volume at grazing angles.
    let clip_plane = inverse.transpose() * plane;
    let corner = inverse * vec4(clip_plane.x.signum(), clip_plane.y.signum(), 1., 1.);
    let scale = 2. / plane.dot(corner);
    let clip = plane * scale;
    let mut p = projection;
    p.x_axis.z = clip.x - p.x_axis.w;
    p.y_axis.z = clip.y - p.y_axis.w;
    p.z_axis.z = clip.z - p.z_axis.w;
    p.w_axis.z = clip.w - p.w_axis.w;
    p
}
struct View {
    matrix: Mat4,
    target: RenderTarget,
}
impl Camera for View {
    fn matrix(&self) -> Mat4 {
        self.matrix
    }
    fn depth_enabled(&self) -> bool {
        true
    }
    fn render_pass(&self) -> Option<RenderPass> {
        Some(self.target.render_pass.clone())
    }
    fn viewport(&self) -> Option<(i32, i32, i32, i32)> {
        None
    }
}

pub(super) fn load(
    assets: &mut Assets,
    name: &str,
    map: &Bsp,
    world: &World,
    specs: &BTreeMap<String, texture::MaterialSpec>,
) -> Result<Vec<Portal>> {
    let mut portals = Vec::new();
    for (model, m) in map.models.iter().enumerate() {
        let owner = map
            .entities
            .iter()
            .find(|e| e.get("model").is_some_and(|s| s == &format!("*{model}")));
        let base = owner
            .and_then(|e| e.get("origin"))
            .and_then(|s| vector(s))
            .unwrap_or(Vec3::ZERO);
        for surface in m.surfaces.clone() {
            let s = &map.surfaces[surface];
            if !specs
                .get(&map.shaders[s.shader].name.to_lowercase())
                .is_some_and(|s| s.camera_portal)
            {
                continue;
            }
            let (vertices, indices) = map.triangulate(s);
            if vertices.is_empty() || indices.is_empty() {
                continue;
            }
            let center = vertices.iter().map(|v| v.position).sum::<Vec3>() / vertices.len() as f32;
            let normal = vertices[0].normal.normalize_or_zero();
            let marker = map
                .entities
                .iter()
                .filter(|e| e.get("classname").is_some_and(|s| s == "portal_surface"))
                .filter_map(|e| Some((e, vector(e.get("origin")?)?.distance(center + base))))
                .filter(|(_, d)| *d < 64.)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|p| p.0);
            let mirror = (name == "keep" && owner.and_then(|e| e.get("targetname")).is_some_and(|n| n == "mirror"))
                || (name == "hatter1" && marker.is_some_and(|e| !e.contains_key("target")));
            let (destination, forward) = if mirror { (Vec3::ZERO, normal) } else {
            let Some(link) = marker.and_then(|e| e.get("target")) else {
                continue;
            };
            let Some(camera) = map.entities.iter().find(|e| {
                e.get("classname").is_some_and(|s| s == "portal_camera")
                    && e.get("targetname") == Some(link)
            }) else {
                continue;
            };
            let Some(destination) = camera.get("origin").and_then(|s| vector(s)) else {
                continue;
            };
            let Some(aim) = map
                .entities
                .iter()
                .find(|e| {
                    e.get("targetname")
                        .is_some_and(|s| Some(s) == camera.get("target"))
                })
                .and_then(|e| e.get("origin"))
                .and_then(|s| vector(s))
            else {
                continue;
            };
            let forward = (aim - destination).normalize_or_zero();
            (destination, forward) };
            if normal.length_squared() < 0.9 || forward.length_squared() < 0.9 {
                continue;
            }
            let material = load_material(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: FRAGMENT,
                },
                MaterialParams {
                    pipeline_params: PipelineParams {
                        depth_test: Comparison::LessOrEqual,
                        depth_write: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )?;
            let pupil = if name == "fortress1"
                && owner
                    .and_then(|e| e.get("targetname"))
                    .is_some_and(|s| s == "window_portal")
            {
                Some(Pupil::load(assets, map, world, specs)?)
            } else {
                None
            };
            let cat = if name == "hatter1" && mirror { Some(crate::npc::Puppet::load(assets,"c_cheshire", &["sit_talk1"],specs)?) } else {None};
            portals.push(Portal {
                cat, actor: None,
                surface,
                model,
                center,
                normal,
                destination,
                forward,
                local: vertices.iter().map(|v| v.position).collect(),
                mesh: Mesh {
                    vertices: vertices
                        .iter()
                        .map(|v| {
                            Vertex::new(v.position.x, v.position.y, v.position.z, 0., 0., WHITE)
                        })
                        .collect(),
                    indices,
                    texture: None,
                },
                target: None,
                material,
                ready: false,
                pupil, mirror, extra: vec![], hide: vec![],
            });
        }
    }
    Ok(portals)
}

impl Portal {
    fn transform(&self, transforms: &[(usize, Vec3, Quat)]) -> Option<(Vec3, Quat)> {
        if self.model == 0 {
            Some((Vec3::ZERO, Quat::IDENTITY))
        } else {
            transforms
                .iter()
                .find(|t| t.0 == self.model)
                .map(|t| (t.1, t.2))
        }
    }
    fn visible(&self, camera: &Camera3D, origin: Vec3, rotation: Quat) -> bool {
        let center = origin + rotation * self.center;
        if (camera.position - center).dot(rotation * self.normal) <= 0.5 {
            return false;
        }
        let matrix = camera.matrix();
        let points: Vec<_> = self
            .local
            .iter()
            .map(|p| matrix * (origin + rotation * *p).extend(1.))
            .collect();
        !(0..3).any(|axis| {
            points.iter().all(|p| p[axis] < -p.w) || points.iter().all(|p| p[axis] > p.w)
        })
    }
    pub(super) fn draw(&mut self, camera: Vec3, transforms: &[(usize, Vec3, Quat)]) {
        if !self.ready {
            return;
        }
        let Some((origin, rotation)) = self.transform(transforms) else {
            return;
        };
        if (camera - origin - rotation * self.center).dot(rotation * self.normal) <= 0. {
            return;
        }
        for (v, p) in self.mesh.vertices.iter_mut().zip(&self.local) {
            v.position = origin + rotation * *p;
        }
        self.mesh.texture = self.target.as_ref().map(|t| t.texture.clone());
        gl_use_material(&self.material);
        draw_mesh(&self.mesh);
        gl_use_default_material();
    }
}

impl Scene {
    pub(crate) fn check_reflection_culled(&self) -> Result<()> {
        ensure!(self.camera_portals.iter().all(|p| !p.mirror || !p.ready),
            "Hidden mirror still prepared a reflection");
        println!("PASS hidden mirror skipped");
        Ok(())
    }
    pub(crate) fn check_reflection_pixels(&self) -> Result<()> {
        unsafe { get_internal_gl().flush(); }
        let mirrors: Vec<_> = self.camera_portals.iter().filter(|p| p.mirror && p.ready).collect();
        ensure!(!mirrors.is_empty(), "Visible mirror was not prepared");
        for p in mirrors {
            let frame = p.target.as_ref().unwrap().texture.get_texture_data();
            let lit = frame.bytes.chunks_exact(4).filter(|p| p[..3].iter().any(|&v| v > 8)).count();
            ensure!(lit > 3000, "Reflected room disappeared: {lit} lit pixels");
            println!("PASS mirror reflected room: {lit} lit pixels");
        }
        Ok(())
    }
    pub fn set_reflection_actor(&mut self, actor: Option<crate::level::ReflectionActor>) {
        for p in &mut self.camera_portals { p.actor = actor; }
    }
    pub fn set_reflection(&mut self, extra: Vec<(usize, Vec3, Quat)>, hide: Vec<usize>) {
        for portal in self.camera_portals.iter_mut().filter(|p| p.mirror) { portal.extra=extra.clone(); portal.hide=hide.clone(); }
    }
    /// Prepare visible, one-level portal views before beginning the main frame's
    /// transparency queue. No main-view sky, lights or camera state is inherited.
    pub fn prepare_camera_portals(
        &mut self,
        camera: &Camera3D,
        time: f32,
        fullbright: bool,
        transforms: &[(usize, Vec3, Quat)],
    ) {
        if self.camera_portals.is_empty() {
            return;
        }
        let cluster = self.map.visibility.cluster(camera.position, &self.map.planes);
        let faces = self.map.visibility.faces(cluster);
        let mut portals = std::mem::take(&mut self.camera_portals);
        for portal in &mut portals {
            portal.ready = false;
            // Only stationary world apertures have reliable leaf membership.
            // Moving mirrors keep their transformed visibility checks; absent
            // PVS data and viewpoints outside the map deliberately fail open.
            if portal.model == 0 && faces.as_ref().is_some_and(|f| !f[portal.surface]) {
                continue;
            }
            let Some((origin, rotation)) = portal.transform(transforms) else {
                continue;
            };
            if !portal.visible(camera, origin, rotation) {
                continue;
            }
            let aspect = camera.aspect.unwrap_or(screen_width() / screen_height());
            let width = screen_width().min(1024.) as u32;
            let height = (width as f32 / aspect).round().max(1.) as u32;
            if portal.target.as_ref().is_none_or(|t| {
                t.texture.width() as u32 != width || t.texture.height() as u32 != height
            }) {
                let target = render_target_ex(
                    width,
                    height,
                    RenderTargetParams {
                        depth: true,
                        ..Default::default()
                    },
                );
                target.texture.set_filter(FilterMode::Linear);
                portal.target = Some(target);
            }
            let source = origin + rotation * portal.center;
            let (eye, direction, up, plane, pvs) = if portal.mirror {
                let normal = rotation * portal.normal;
                let reflect = |v: Vec3| v - 2. * normal * v.dot(normal);
                (source + reflect(camera.position-source), reflect(camera.target-camera.position), reflect(camera.up), normal.extend(-normal.dot(source)), source+normal*2.)
            } else {
                let mapping=frame(portal.forward)*(rotation*frame(-portal.normal)).inverse();
                (portal.destination+mapping*(camera.position-source),mapping*(camera.target-camera.position),mapping*camera.up,portal.forward.extend(-portal.forward.dot(portal.destination)),portal.destination+portal.forward)
            };
            let view=Mat4::look_at_rh(eye,eye+direction,up);
            let mut projection=Mat4::perspective_rh_gl(camera.fovy,aspect,0.1,camera.z_far);
            if portal.mirror { projection.x_axis.x = -projection.x_axis.x; }
            let view = View {
                matrix: clipped_projection(projection, view, plane) * view,
                target: portal.target.as_ref().unwrap().clone(),
            };
            set_camera(&view);
            clear_background(BLACK);
            let atmosphere = Atmosphere::default();
            crate::lighting::select(Vec::new(), eye, &self.world);
            crate::render_fx::begin(eye, direction, time, &atmosphere, fullbright);
            crate::render_fx::set_projection(view.matrix);
            self.bind_atlas();
            for m in &self.bound_materials {
                atmosphere.apply(m, eye);
                m.set_uniform("Fullbright", if fullbright { 1_f32 } else { 0. });
                m.set_uniform("SkyFade", 0_f32);
            }
            let cluster = self
                .map
                .visibility
                .cluster(pvs, &self.map.planes);
            let mut context = DrawContext {
                enabled: self.optimize,
                required_pvs: true,
                faces: self.map.visibility.faces(cluster),
                cluster,
                visibility: &self.map.visibility,
                stats: DrawStats::default(),
            };
            let reflected:Vec<_> = transforms.iter().filter(|t| !portal.mirror || !portal.hide.contains(&t.0)).cloned().chain(portal.extra.iter().cloned()).collect();
            for b in self.batches.iter_mut().filter(|b| !b.sky) {
                let t = b.model.and_then(|model| reflected.iter().find(|t| t.0 == model));
                if b.model.is_some() && t.is_none() {
                    continue;
                }
                draw_batch(
                    b,
                    &self.materials,
                    eye,
                    time,
                    t,
                    Vec3::ZERO,
                    false,
                    &mut context,
                );
            }
            if let Some(pupil) = &mut portal.pupil {
                pupil
                    .art
                    .draw_frame(pupil.pose(time), pupil.scale, fullbright, time, true);
            }
            if let (Some(cat), Some(actor)) = (&mut portal.cat, portal.actor) {
                cat.atmosphere(&atmosphere,eye);
                cat.draw_dissolving("sit_talk1",actor.time,true,actor.pose,1.,fullbright,1.-actor.alpha);
            }
            crate::render_fx::finish();
            gl_use_default_material();
            set_camera(camera);
            portal.ready = true;
        }
        self.camera_portals = portals;
    }
}

const VERTEX: &str = r#"#version 100
attribute vec3 position;
uniform mat4 Model;
uniform mat4 Projection;
varying highp vec4 screenPosition;
void main() { gl_Position=Projection*Model*vec4(position,1.0); screenPosition=gl_Position; }
"#;
const FRAGMENT: &str = r#"#version 100
precision mediump float;
uniform sampler2D Texture;
varying highp vec4 screenPosition;
void main() { gl_FragColor=vec4(texture2D(Texture,screenPosition.xy/screenPosition.w*0.5+0.5).rgb,1.0); }
"#;

pub async fn check(assets: &mut Assets) -> Result<()> {
    let mut scene = Scene::load(assets, "fortress1")?;
    ensure!(
        scene
            .camera_portals
            .iter()
            .any(|p| p.model == 10 && p.pupil.is_some()),
        "School camera portal missing"
    );
    let mut fortress = crate::fortress::Fortress::load(assets, &scene.map, true)?;
    fortress.state.shutters = true;
    for (label, dx, time) in [("center", 0., 0.), ("left", -140., 3.), ("right", 140., 6.)] {
        let mut state = serde_json::to_value(fortress.snapshot())?;
        state["age"] = time.into();
        fortress.restore(&serde_json::from_value(state)?, &scene.map)?;
        let transforms: Vec<_> = fortress.transforms().collect();
        let (_, origin, rotation) = transforms
            .iter()
            .find(|t| t.0 == 10)
            .context("Window hidden")?;
        let center = *origin + *rotation * vec3(0., 6., 0.);
        let camera = Camera3D {
            position: center + *rotation * vec3(dx, 420., 24.),
            target: center,
            up: Vec3::Z,
            fovy: 60_f32.to_radians(),
            z_near: 2.,
            z_far: 30000.,
            ..Default::default()
        };
        let portal = scene.camera_portals.iter().find(|p| p.model == 10).unwrap();
        let bounds = portal
            .local
            .iter()
            .map(|p| {
                let v = camera.matrix() * (*origin + *rotation * *p).extend(1.);
                vec2(
                    (v.x / v.w * 0.5 + 0.5) * screen_width(),
                    (v.y / v.w * 0.5 + 0.5) * screen_height(),
                )
            })
            .fold(
                (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
                |(lo, hi), p| (lo.min(p), hi.max(p)),
            );
        let mut samples = Vec::new();
        for mode in 0..5 {
            if mode == 4 {
                let state = serde_json::to_vec(&fortress.snapshot())?;
                scene = Scene::load(assets, "fortress1")?;
                fortress.restore(&serde_json::from_slice(&state)?, &scene.map)?;
            }
            let pupils: Vec<_> = if mode == 1 {
                scene
                    .camera_portals
                    .iter_mut()
                    .map(|p| p.pupil.take())
                    .collect()
            } else {
                Vec::new()
            };
            set_camera(&camera);
            clear_background(BLACK);
            if mode > 0 {
                scene.prepare_camera_portals(&camera, time, false, &transforms);
            } else {
                for p in &mut scene.camera_portals {
                    p.ready = false;
                }
            }
            crate::render_fx::begin_view(&camera, time, &scene.atmosphere, false);
            scene.draw(camera.position, time, false, false, &transforms);
            scene.draw(camera.position, time, false, true, &transforms);
            crate::render_fx::finish();
            set_default_camera();
            if mode == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/fortress-window-{label}.png"
                )))?;
            }
            samples.push(get_screen_data());
            if mode == 1 {
                for (portal, pupil) in scene.camera_portals.iter_mut().zip(pupils) {
                    portal.pupil = pupil;
                }
            }
            next_frame().await;
        }
        let pixels = crate::character::visible_pixels(&samples[0], &samples[2]);
        ensure!(
            pixels > 1000,
            "School portal view missing at {label}: {pixels} pixels"
        );
        ensure!(
            samples[2].bytes == samples[3].bytes && samples[2].bytes == samples[4].bytes,
            "Paused or rebuilt portal changed (pause={}, rebuild={})",
            crate::character::visible_pixels(&samples[2], &samples[3]),
            crate::character::visible_pixels(&samples[2], &samples[4])
        );
        let pupil_pixels = crate::character::visible_pixels(&samples[1], &samples[2]);
        if label == "center" {
            ensure!(pupil_pixels > 50, "Portal pupil is missing: {pupil_pixels}");
        }
        for (i, (a, b)) in samples[0]
            .bytes
            .chunks_exact(4)
            .zip(samples[2].bytes.chunks_exact(4))
            .enumerate()
        {
            let point = vec2(
                (i % samples[0].width as usize) as f32,
                (i / samples[0].width as usize) as f32,
            );
            if point.cmplt(bounds.0 - Vec2::splat(2.)).any()
                || point.cmpgt(bounds.1 + Vec2::splat(2.)).any()
            {
                ensure!(a == b, "Portal leaked outside its aperture at {point:?}");
            }
        }
        println!("PASS school window {label}: {pixels} interior pixels, {pupil_pixels} pupil pixels, no exterior changes, identical pause/rebuild");
    }
    // The same doorway stays closed on the first visit and before the return
    // reveal. The portal must remain behind its shutters and ordinary scenery.
    for returning in [false, true] {
        let fortress = crate::fortress::Fortress::load(assets, &scene.map, returning)?;
        let transforms: Vec<_> = fortress.transforms().collect();
        let camera = Camera3D {
            position: vec3(-3840., 4100., 570.),
            target: vec3(-3840., 3500., 530.),
            up: Vec3::Z,
            fovy: 60_f32.to_radians(),
            z_near: 2.,
            z_far: 30000.,
            ..Default::default()
        };
        let mut samples = Vec::new();
        for enabled in [false, true] {
            set_camera(&camera);
            clear_background(BLACK);
            // Keep the separate lower-route portal identical in both images.
            scene.prepare_camera_portals(&camera, 0., false, &transforms);
            for p in &mut scene.camera_portals {
                if p.model == 10 {
                    ensure!(returning || !p.ready, "School portal active on first visit");
                    if !enabled {
                        p.ready = false;
                    }
                }
            }
            crate::render_fx::begin_view(&camera, 0., &scene.atmosphere, false);
            scene.draw(camera.position, 0., false, false, &transforms);
            scene.draw(camera.position, 0., false, true, &transforms);
            crate::render_fx::finish();
            set_default_camera();
            if enabled {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/fortress-window-closed-{returning}.png"
                )))?;
            }
            samples.push(get_screen_data());
            next_frame().await;
        }
        ensure!(
            samples[0].bytes == samples[1].bytes,
            "Closed shutters reveal the portal (return={returning}, pixels={}, prepared={:?})",
            crate::character::visible_pixels(&samples[0], &samples[1]),
            scene
                .camera_portals
                .iter()
                .filter(|p| p.ready)
                .map(|p| p.model)
                .collect::<Vec<_>>()
        );
        println!(
            "PASS closed school window (return={returning}): portal hidden by visit state/shutters"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mirror_flip_preserves_clipping_at_oblique_angles() {
        let flip = Mat4::from_scale(vec3(-1., 1., 1.));
        for aspect in [4. / 3., 16. / 9., 21. / 9.] {
            let projection = Mat4::perspective_rh_gl(75_f32.to_radians(), aspect, 0.1, 20000.);
            for degrees in [-110_f32, -70., -30., 0., 30., 70., 110.] {
                let angle = degrees.to_radians();
                let eye = vec3(0., -34., 390.);
                let direction = vec3(angle.sin(), angle.cos(), -0.03).normalize();
                let view = Mat4::look_at_rh(eye, eye + direction, Vec3::Z);
                let plane = Vec3::Y.extend(0.);
                let ordinary = clipped_projection(projection, view, plane);
                let reflected = clipped_projection(flip * projection, view, plane);
                let difference = (reflected - flip * ordinary).to_cols_array();
                assert!(difference.iter().all(|v| v.abs() < 0.001),
                    "Mirroring changed the clipping volume at {degrees} degrees, aspect {aspect}");
                // The aperture still forms the near plane on both projections.
                let point = vec3(80., 0., 380.).extend(1.);
                let clipped = reflected * view * point;
                assert!((clipped.z + clipped.w).abs() < 0.001);
            }
        }
    }
    #[test]
    fn oblique_plane_clips_remote_back_wall_and_keeps_room() {
        let eye = vec3(40., -400., 24.);
        let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Z);
        let projection = Mat4::perspective_rh_gl(1.2, 1.5, 0.1, 30000.);
        let matrix = clipped_projection(projection, view, Vec3::Y.extend(0.)) * view;
        for (p, visible) in [(vec3(0., -20., 0.), false), (vec3(0., 20., 0.), true)] {
            let clip = matrix * p.extend(1.);
            assert_eq!(clip.z >= -clip.w, visible);
        }
    }
}
