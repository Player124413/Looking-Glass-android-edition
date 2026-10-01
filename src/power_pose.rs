//! Original Alice power-up performances and skins, independent of gameplay timers.
pub(crate) mod effects;
use crate::{
    assets::Assets,
    skeletal::{Animation, Definition, Skeleton, Transform},
    texture,
    weapons::Prop,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Form {
    Rage,
    Tea,
}
const RAGE: [&str; 9] = [
    "ready",
    "rage_infection_start",
    "rage_infection_pant",
    "rage_infection_pant",
    "rage_infection_rise",
    "rage_infection_grow",
    "rage_infection_stand",
    "ready",
    "ready",
];
const TEA: [&str; 1] = ["grasshoppertea"];
// Supplied SKAN durations, checked against the loaded clips in Performance::load.
pub const RAGE_SECONDS: f32 = 12.5;
const RAGE_LENGTHS: [f32; 9] = [1.55, 2.6, 1.3, 1.3, 1., 0.25, 1.4, 1.55, 1.55];

/// Player-relative bulletcam endpoints from RAGE_*; the clock belongs to the
/// map scene. Exponential settling is a port approximation of native damping,
/// evaluated analytically so pause, reload and frame rate cannot change it.
pub fn rage_camera(time: f32, at: Transform) -> crate::cinematic::Camera {
    let eyes = [
        vec3(50., -50., 0.),
        vec3(50., -50., 0.),
        vec3(50., -50., 0.),
        vec3(25., -50., 0.),
        vec3(25., -20., 0.),
        vec3(30., -5., 0.),
        vec3(35., -5., 0.),
        vec3(35., -5., 20.),
        vec3(35., 0., 20.),
    ];
    let heights = [20., 20., 0., 0., 0., 10., 20., 20., 20.];
    let (mut eye, mut target) = (eyes[0], Vec3::Z * heights[0]);
    let mut left = time.max(0.);
    for (index, duration) in RAGE_LENGTHS.iter().enumerate() {
        let elapsed = left.min(*duration);
        let weight = 1. - (-2. * elapsed).exp();
        eye = eye.lerp(eyes[index], weight);
        target = target.lerp(Vec3::Z * heights[index], weight);
        left -= elapsed;
        if left <= 0. {
            break;
        }
    }
    let center = at.translation + crate::collision::PLAYER_CENTER;
    crate::cinematic::Camera::look(center + at.rotation * eye, center + at.rotation * target)
}
fn sequence(form: Form) -> &'static [&'static str] {
    match form {
        Form::Rage => &RAGE,
        Form::Tea => &TEA,
    }
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub form: Option<Form>,
    pub age: f32,
    pub performing: bool,
    weight: f32,
    pose: Vec<Transform>,
    // Absent in old saves: apply the existing effect without repeating its pickup performance.
    initialized: bool,
}
impl State {
    pub fn fresh() -> Self {
        Self {
            initialized: true,
            ..Self::default()
        }
    }
    pub fn sync(&mut self, form: Option<Form>) {
        if self.form != form || !self.initialized {
            let perform = self.initialized && form.is_some();
            *self = Self {
                form,
                age: if perform { 0. } else { 60. },
                performing: perform,
                initialized: true,
                ..Self::default()
            };
        }
    }
    pub fn interrupt(&mut self) {
        self.performing = false;
    }
    pub fn skin(&self) -> Option<Form> {
        self.form.filter(|_| self.age >= 2.5)
    }
    pub fn hidden(&self, name: &str) -> bool {
        self.form == Some(Form::Rage)
            && match name {
                "material2" => self.age >= 8.,
                "material19" => self.age >= 7.75,
                _ => false,
            }
    }
    pub fn validate(&self, bones: usize) -> Result<()> {
        ensure!(
            self.age.is_finite()
                && (0. ..=60.).contains(&self.age)
                && (0. ..=1.).contains(&self.weight)
                && (!self.performing || self.form.is_some())
                && (self.pose.is_empty() || self.pose.len() == bones)
                && self.pose.iter().all(|p| p.translation.is_finite()
                    && p.rotation.is_finite()
                    && (p.rotation.length_squared() - 1.).abs() < 0.01),
            "Invalid saved power-up presentation"
        );
        Ok(())
    }
}

pub struct Performance {
    clips: BTreeMap<String, Animation>,
    pub state: State,
}
impl Performance {
    pub fn load(assets: &mut Assets, def: &Definition, bones: usize) -> Result<Self> {
        let mut clips = BTreeMap::new();
        for &name in RAGE.iter().chain(&TEA) {
            if !clips.contains_key(name) {
                let file = def
                    .animations
                    .get(name)
                    .with_context(|| format!("Missing power-up animation {name}"))?;
                clips.insert(
                    name.into(),
                    Animation::parse(&assets.read(&format!("{}/{file}", def.path))?, bones)?,
                );
            }
        }
        for (name, duration) in RAGE.iter().zip(RAGE_LENGTHS) {
            ensure!(
                (clips[*name].duration() - duration).abs() < 0.001,
                "Rage scene/clip clock mismatch: {name}"
            );
        }
        Ok(Self {
            clips,
            state: State::fresh(),
        })
    }
    pub fn duration(&self, form: Form) -> f32 {
        sequence(form)
            .iter()
            .map(|n| self.clips[*n].duration())
            .sum()
    }
    /// A scene supplies the sole clock, including after a paused save restore.
    pub fn rage_scene(&mut self, time: f32, finished: bool, local: &mut Vec<Transform>) {
        if self.state.form != Some(Form::Rage) {
            return;
        }
        if finished {
            self.state.age = self.state.age.max(RAGE_SECONDS);
            self.state.interrupt();
        } else {
            self.state.age = time.clamp(0., RAGE_SECONDS);
            self.state.performing = true;
            self.state.weight = 1.;
            self.state.pose = self.sample(Form::Rage, self.state.age);
            local.clone_from(&self.state.pose);
        }
    }
    fn sample(&self, form: Form, age: f32) -> Vec<Transform> {
        let mut time = age;
        let seq = sequence(form);
        for (i, name) in seq.iter().enumerate() {
            let clip = &self.clips[*name];
            if time < clip.duration() || i == seq.len() - 1 {
                let mut pose = clip.sample(time, false);
                if form == Form::Rage && i > 0 {
                    let duration = 0.15_f32.min(clip.duration() * 0.4);
                    let weight = effects::ease(time / duration);
                    if weight < 1. {
                        let previous = &self.clips[seq[i - 1]];
                        for (p, from) in pose
                            .iter_mut()
                            .zip(previous.sample(previous.duration(), false))
                        {
                            *p = from.blend(*p, weight);
                        }
                    }
                }
                return pose;
            }
            time -= clip.duration();
        }
        unreachable!()
    }
    /// Movement retains its own root and legs. Attacks and traversal interrupt
    /// the performance permanently for this activation, without consuming time twice.
    pub fn update(
        &mut self,
        dt: f32,
        allowed: bool,
        full_body: bool,
        local: &mut [Transform],
        upper: &[bool],
    ) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        if !allowed {
            self.state.interrupt();
        }
        if let Some(form) = self.state.form {
            self.state.age = (self.state.age + dt).min(60.);
            if self.state.age >= self.duration(form) {
                self.state.interrupt();
            }
            if self.state.performing {
                self.state.pose = self.sample(form, self.state.age);
            }
        }
        let change = dt / 0.15;
        self.state.weight = if self.state.performing {
            (self.state.weight + change).min(1.)
        } else {
            (self.state.weight - change).max(0.)
        };
        // A new attack/traversal must own its pose immediately, including its release tags.
        if !allowed {
            self.state.weight = 0.;
        }
        for (i, (out, source)) in local.iter_mut().zip(&self.state.pose).enumerate() {
            if full_body || upper[i] {
                *out = out.blend(*source, self.state.weight);
            }
        }
    }
}

struct Skin {
    index: usize,
    normal: Texture2D,
    rage: Texture2D,
    tea: Texture2D,
}
fn skin_pixels(name: &str, image: &mut texture::RgbaImage) {
    // This opaque alternate arm skin has no authored alpha-test/blend shader.
    // Its stored alpha is not coverage: the palm/fingers contain zero alpha.
    // The generic skin fallback would cut those parts out of the intact mesh.
    if name == "rage_arm" {
        for pixel in image.pixels.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
    }
}
struct Attachment {
    form: Form,
    tag: usize,
    start: f32,
    growth: f32,
    scale: f32,
    prop: Prop,
}
pub struct Art {
    skins: Vec<Skin>,
    props: Vec<Attachment>,
    cup: (usize, Prop),
    aura: effects::Aura,
    _layers: Vec<std::rc::Rc<crate::render_fx::Surface>>,
}
impl Art {
    pub fn load(
        assets: &mut Assets,
        skeleton: &Skeleton,
        meshes: &[Option<Mesh>],
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let mut layers = Vec::new();
        let mut textures = BTreeMap::new();
        for name in ["rage_face", "rage_arm", "g_face", "g_arm"] {
            let skin = format!("models/alice/{name}");
            let path = texture::resolve(assets, &skin, specs)
                .with_context(|| format!("Missing original skin {skin}"))?;
            let mut image = texture::decode(assets, &path)?;
            skin_pixels(name, &mut image);
            let tex = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
            tex.set_filter(FilterMode::Linear);
            layers.push(crate::render_fx::register(assets, &tex, &[skin], specs)?);
            textures.insert(name, tex);
        }
        let mut skins = Vec::new();
        for (index, surface) in skeleton.surfaces.iter().enumerate() {
            let pair = match surface.name.as_str() {
                "material1" => Some(("rage_face", "g_face")),
                "material6" | "material7" | "material8" | "material19" => {
                    Some(("rage_arm", "g_arm"))
                }
                _ => None,
            };
            if let Some((rage, tea)) = pair {
                skins.push(Skin {
                    index,
                    normal: meshes[index]
                        .as_ref()
                        .and_then(|m| m.texture.clone())
                        .context("Power-up surface missing")?,
                    rage: textures[rage].clone(),
                    tea: textures[tea].clone(),
                });
            }
        }
        // Attachments and growth delays are from the supplied gameplay DLL.
        // The original effect's timer remains owned by inventory::Stats.
        let mut props = Vec::new();
        for (form, model, clip, bone, start, growth, scale) in [
            (
                Form::Rage,
                "a_rage_back_left",
                "rage_back_left",
                "tag_back",
                3.5,
                4.,
                1.,
            ),
            (
                Form::Rage,
                "a_rage_back_right",
                "rage_back_right",
                "tag_back",
                3.2,
                4.,
                1.,
            ),
            (
                Form::Rage,
                "a_rage_hair",
                "rage_hair",
                "tag_head",
                3.5,
                4.,
                1.,
            ),
            (
                Form::Rage,
                "a_rage_claw",
                "rage_claw",
                "tag_leftforearm",
                3.5,
                4.,
                1.,
            ),
            (
                Form::Tea,
                "wings",
                "wings_forward",
                "tag_back",
                3.,
                1.5,
                1.25,
            ),
            (Form::Tea, "headgear", "headgear", "tag_head", 3.5, 1., 1.),
        ] {
            let tag = skeleton
                .bones
                .iter()
                .position(|b| b.name == bone)
                .context("Missing power-up attachment bone")?;
            props.push(Attachment {
                form,
                tag,
                start,
                growth,
                scale,
                prop: Prop::load_animation(assets, model, clip, specs)?,
            });
        }
        Ok(Self {
            skins,
            props,
            aura: effects::Aura::load(assets, specs)?,
            cup: (
                skeleton
                    .bones
                    .iter()
                    .position(|b| b.name == "tag_03")
                    .context("Missing Tea pickup tag")?,
                Prop::load_animation(assets, "w_grasshopper_pickup", "idle", specs)?,
            ),
            _layers: layers,
        })
    }
    pub fn skin(&self, meshes: &mut [Option<Mesh>], state: &State) {
        for skin in &self.skins {
            if let Some(mesh) = &mut meshes[skin.index] {
                mesh.texture = Some(
                    match state.skin() {
                        Some(Form::Rage) if state.age < 2.85 => &skin.normal,
                        Some(Form::Rage) => &skin.rage,
                        Some(Form::Tea) => &skin.tea,
                        None => &skin.normal,
                    }
                    .clone(),
                );
            }
        }
    }
    pub fn atmosphere(&self, camera: Vec3) {
        self.aura.camera.set(camera);
    }
    pub fn remaining(&self, remaining: f32) {
        self.aura.remaining.set(remaining);
    }
    pub fn surface_effects(&self, meshes: &[Option<Mesh>], state: &State) {
        if state.form != Some(Form::Rage) {
            return;
        }
        for skin in &self.skins {
            if let Some(mesh) = &meshes[skin.index] {
                effects::blend_skin(mesh, &skin.rage, effects::ease((state.age - 2.5) / 0.35));
            }
        }
        for mesh in meshes.iter().flatten() {
            self.aura.draw(mesh, state);
        }
    }
    pub fn draw(
        &mut self,
        state: &State,
        pose: &[Transform],
        root: Transform,
        scale: f32,
        fullbright: bool,
    ) {
        if state.form == Some(Form::Tea) && state.performing && state.age < 4.75 {
            let tag = pose[self.cup.0];
            self.cup.1.draw_frame(
                Transform {
                    translation: root.point(tag.translation * scale),
                    rotation: root.rotation * tag.rotation,
                },
                scale,
                fullbright,
                state.age,
                false,
            );
        }
        for a in &mut self.props {
            if state.form != Some(a.form) || state.age < a.start {
                continue;
            }
            let growth = if a.form == Form::Rage {
                effects::ease((state.age - a.start) / a.growth)
            } else {
                ((state.age - a.start) / a.growth).clamp(0., 1.)
            };
            let anchor = Transform {
                translation: root.point(pose[a.tag].translation * scale),
                rotation: root.rotation * pose[a.tag].rotation,
            };
            for mesh in a.prop.meshes_at(
                anchor,
                scale * a.scale * growth,
                fullbright,
                state.age - a.start,
                a.form == Form::Tea,
            ) {
                crate::render_fx::skin(mesh);
                if a.form == Form::Rage {
                    self.aura.draw(mesh, state);
                }
            }
        }
    }
}

pub fn check(assets: &mut Assets) -> Result<()> {
    let def = Definition::alice(assets)?;
    let skeleton = Skeleton::parse(&assets.read(&format!("{}/{}", def.path, def.model))?)?;
    let mut performance = Performance::load(assets, &def, skeleton.bones.len())?;
    let mut boundary = 0.;
    for duration in RAGE_LENGTHS.iter().take(8) {
        boundary += duration;
        let before = skeleton.global_pose(&performance.sample(Form::Rage, boundary - 0.0001));
        let after = skeleton.global_pose(&performance.sample(Form::Rage, boundary + 0.0001));
        let largest = skeleton
            .surfaces
            .iter()
            .flat_map(|s| &s.vertices)
            .map(|v| v.position(&before).distance(v.position(&after)))
            .fold(0_f32, f32::max);
        ensure!(
            largest < 0.1,
            "Rage clip boundary jumps at {boundary}: {largest}"
        );
    }
    println!("PASS all eight Rage clip boundaries remain continuous on the original skin");
    let upper: Vec<_> = skeleton.bones.iter().map(|_| true).collect();
    for form in [Form::Rage, Form::Tea] {
        performance.state = State::fresh();
        performance.state.sync(Some(form));
        let mut pose = performance.clips["ready"].sample(0., false);
        performance.update(0.8, true, true, &mut pose, &upper);
        let state = serde_json::to_vec(&performance.state)?;
        performance.update(0., true, true, &mut pose, &upper);
        ensure!(
            state == serde_json::to_vec(&performance.state)?,
            "Paused transformation moved"
        );
        performance.state.validate(skeleton.bones.len())?;
        performance.update(0.1, false, false, &mut pose, &upper);
        ensure!(
            !performance.state.performing,
            "Interrupted transformation resumed"
        );
        performance.state.sync(Some(form));
        ensure!(!performance.state.performing, "Same effect retriggered");
        performance.state = serde_json::from_slice(&state)?;
        performance.update(30., true, true, &mut pose, &upper);
        ensure!(
            !performance.state.performing && performance.state.skin() == Some(form),
            "Skipped transformation did not finish"
        );
        performance.state.sync(None);
        ensure!(
            performance.state.skin().is_none(),
            "Expired power skin survived"
        );
        println!("PASS original {form:?} transformation: {:.2}s; pause, interruption, saved phase, frame skip and expiry",performance.duration(form));
    }
    let specs = texture::read_materials(assets)?;
    for skin in ["rage_face", "rage_arm", "g_face", "g_arm"] {
        let path = texture::resolve(assets, &format!("models/alice/{skin}"), &specs)
            .context("Missing power skin")?;
        let mut image = texture::decode(assets, &path)?;
        if skin == "rage_arm" {
            ensure!(
                image.pixels.chunks_exact(4).any(|p| p[3] == 0),
                "Rage arm coverage regression fixture changed"
            );
            skin_pixels(skin, &mut image);
            ensure!(
                image.pixels.chunks_exact(4).all(|p| p[3] == 255),
                "Rage hand can be cut out"
            );
        }
    }
    for weapon in crate::weapons::ACTION_CLIPS {
        let clip = Animation::parse(
            &assets.read(&format!("models/alice/{weapon}.ska"))?,
            skeleton.bones.len(),
        )?;
        for frame in &clip.frames {
            let pose = skeleton.global_pose(&frame.pose);
            ensure!(
                skeleton
                    .surfaces
                    .iter()
                    .flat_map(|s| &s.vertices)
                    .all(|v| v.position(&pose).is_finite()),
                "Invalid original action pose"
            );
        }
    }
    println!("PASS four original power skins and all original weapon action poses");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rage_arm_keeps_rgb_and_all_hand_coverage() {
        let pixels = vec![17, 23, 42, 0, 90, 40, 30, 122];
        let mut image = texture::RgbaImage {
            width: 2,
            height: 1,
            pixels: pixels.clone(),
        };
        skin_pixels("g_arm", &mut image);
        assert_eq!(image.pixels, pixels);
        skin_pixels("rage_arm", &mut image);
        assert_eq!(image.pixels, [17, 23, 42, 255, 90, 40, 30, 255]);
    }
    #[test]
    fn rage_camera_is_continuous_and_follows_the_carried_body() {
        let at = Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        };
        let mut time = 0.;
        for duration in RAGE_LENGTHS {
            time += duration;
            let before = rage_camera(time - 0.0001, at);
            let after = rage_camera(time + 0.0001, at);
            assert!(before.eye.distance(after.eye) < 0.02);
            assert!(before.target.distance(after.target) < 0.02);
        }
        assert!((time - RAGE_SECONDS).abs() < 0.001);
        let moved = Transform {
            translation: vec3(10., 20., -48.),
            rotation: Quat::from_rotation_z(0.7),
        };
        for time in [0., 3., 7.9, 11.] {
            let a = rage_camera(time, at);
            let b = rage_camera(time, moved);
            assert!(b.eye.distance(moved.translation + moved.rotation * a.eye) < 0.001);
            assert!(
                b.target
                    .distance(moved.translation + moved.rotation * a.target)
                    < 0.001
            );
        }
    }
    #[test]
    fn effects_are_idempotent_interruptible_and_old_saves_do_not_replay() {
        let mut s = State::fresh();
        s.sync(Some(Form::Rage));
        assert!(s.performing);
        s.age = 4.;
        s.interrupt();
        s.sync(Some(Form::Rage));
        assert!(!s.performing);
        assert_eq!(s.age, 4.);
        let mut restored: State =
            serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        restored.sync(Some(Form::Rage));
        assert_eq!(restored.age, 4.);
        assert!(!restored.performing);
        restored.sync(None);
        assert_eq!(restored.skin(), None);
        assert!(!restored.hidden("material2"));
        restored.sync(Some(Form::Tea));
        assert!(restored.performing);
        assert_eq!(restored.age, 0.);
        let mut legacy = State::default();
        legacy.sync(Some(Form::Tea));
        assert!(!legacy.performing);
        assert_eq!(legacy.skin(), Some(Form::Tea));
    }
}
