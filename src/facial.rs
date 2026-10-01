//! Original LIP envelopes and mouth/eyelid presentation. No game code is executed.
use crate::{
    assets::Assets,
    bsp::tokens,
    skeletal::{Definition, Skeleton, Transform},
    texture,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone, Default)]
pub struct LipTrack {
    duration: f32,
    samples: Vec<u8>,
}
impl LipTrack {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= 1_000_000, "LIP file too large");
        let mut words = std::str::from_utf8(bytes)?.split_whitespace();
        let duration: f32 = words.next().context("Missing LIP duration")?.parse()?;
        let count: usize = words.next().context("Missing LIP count")?.parse()?;
        ensure!(
            duration.is_finite() && duration > 0. && duration <= 600.,
            "Invalid LIP duration"
        );
        ensure!((1..=12002).contains(&count), "Invalid LIP sample count");
        let samples = words
            .take(count + 1)
            .map(str::parse)
            .collect::<Result<Vec<u8>, _>>()?;
        ensure!(samples.len() == count, "LIP sample count mismatch");
        Ok(Self { duration, samples })
    }
    /// A missing envelope is valid; a supplied corrupt envelope is a validation error.
    pub fn checked(assets: &mut Assets, voice: &str) -> Result<Self> {
        let path = format!("{}.lip", voice.trim_end_matches(".wav"));
        if !assets.contains(&path) {
            return Ok(Self::default());
        }
        Self::parse(&assets.read(&path)?).with_context(|| path)
    }
    pub fn load(assets: &mut Assets, voice: &str) -> Self {
        Self::checked(assets, voice).unwrap_or_else(|e| {
            eprintln!("Lip sync unavailable {voice}: {e:#}");
            Self::default()
        })
    }
    pub fn sample(&self, seconds: f32) -> f32 {
        if !seconds.is_finite()
            || seconds < 0.
            || seconds >= self.duration
            || self.samples.is_empty()
        {
            return 0.;
        }
        // The supplied envelopes have 20 samples/second and trailing padding.
        // Do not stretch that padding across the recording's duration.
        let frame = seconds * 20.;
        let a = frame.floor() as usize;
        let x = self.samples.get(a).copied().unwrap_or(0) as f32;
        let y = self.samples.get(a + 1).copied().unwrap_or(0) as f32;
        (x + (y - x) * frame.fract()) / 255.
    }
}

#[derive(Default)]
pub struct Settings {
    pub angle: Option<f32>,
    blink: Option<(String, f32, String)>,
}
impl Settings {
    fn parse(text: &str) -> Result<Self> {
        let t = tokens(text)?;
        let mut out = Self::default();
        let mut depth = 0;
        let mut section = "";
        let mut server = false;
        let mut skins = BTreeMap::<String, Vec<String>>::new();
        let mut blink = None;
        for (i, word) in t.iter().enumerate() {
            match word.as_str() {
                "{" => depth += 1,
                "}" => {
                    depth -= 1;
                    if depth < 2 {
                        server = false;
                    }
                }
                _ if depth == 0 => section = word,
                _ if depth == 1 && section.eq_ignore_ascii_case("setup") => {
                    if word.eq_ignore_ascii_case("surface")
                        && t.get(i + 2)
                            .is_some_and(|v| v.eq_ignore_ascii_case("shader"))
                    {
                        if let (Some(surface), Some(skin)) = (t.get(i + 1), t.get(i + 3)) {
                            skins
                                .entry(surface.to_lowercase())
                                .or_default()
                                .push(skin.clone());
                        }
                    }
                }
                _ if depth == 1 && section.eq_ignore_ascii_case("init") => {
                    server = word.eq_ignore_ascii_case("server")
                }
                _ if depth == 2 && server => {
                    if word.eq_ignore_ascii_case("maxmouthangle") {
                        let angle: f32 = t.get(i + 1).context("Missing mouth angle")?.parse()?;
                        ensure!(
                            angle.is_finite() && (0. ..=90.).contains(&angle),
                            "Invalid mouth angle"
                        );
                        out.angle = Some(angle);
                    } else if word.eq_ignore_ascii_case("blinkinfo") {
                        let surface = t
                            .get(i + 1)
                            .context("Missing blink surface")?
                            .to_lowercase();
                        let period: f32 =
                            t.get(i + 2).context("Missing blink interval")?.parse()?;
                        ensure!(
                            period.is_finite() && (0.5..=60.).contains(&period),
                            "Invalid blink interval"
                        );
                        blink = Some((surface, period));
                    }
                }
                _ => (),
            }
        }
        out.blink = blink.and_then(|(surface, period)| {
            skins
                .get(&surface)
                .and_then(|s| s.get(1))
                .map(|skin| (surface, period, skin.clone()))
        });
        Ok(out)
    }
}

pub struct Rig {
    mouth: Option<usize>,
    angle: f32,
}
impl Rig {
    pub fn new(skeleton: &Skeleton, settings: &Settings) -> Self {
        Self {
            mouth: skeleton
                .bones
                .iter()
                .position(|b| b.name.eq_ignore_ascii_case("tag_mouth")),
            // Models without an override use a restrained reconstructed default.
            // Both Actor and Sentient initialize maxmouthangle to 10 degrees
            // in the supplied gameplay DLL (constant 0x1015c1d8).
            angle: settings.angle.unwrap_or(10.).to_radians(),
        }
    }
    pub fn apply(&self, pose: &mut [Transform], amount: f32) {
        if let Some(bone) = self.mouth.and_then(|i| pose.get_mut(i)) {
            let amount = if amount.is_finite() {
                amount.clamp(0., 1.)
            } else {
                0.
            };
            // Observed mouth tags use local X as the transverse jaw hinge.
            // Compose after the authored local rotation, preserving head acting.
            bone.rotation =
                (bone.rotation * Quat::from_rotation_x(self.angle * amount)).normalize();
        }
    }
}

struct Blink {
    surface: usize,
    period: f32,
    open: Texture2D,
    closed: Texture2D,
}
pub struct Face {
    rig: Rig,
    blink: Option<Blink>,
}
impl Face {
    pub fn load(
        assets: &mut Assets,
        path: &str,
        def: &Definition,
        skeleton: &Skeleton,
        meshes: &[Option<Mesh>],
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let settings = Settings::parse(&String::from_utf8_lossy(&assets.read(path)?))?;
        let rig = Rig::new(skeleton, &settings);
        let blink = if let Some((surface, period, skin)) = settings.blink {
            let index = skeleton
                .surfaces
                .iter()
                .position(|s| s.name == surface)
                .context("Missing blink surface")?;
            let open = meshes[index]
                .as_ref()
                .and_then(|m| m.texture.clone())
                .context("Missing open eye texture")?;
            let path = texture::resolve(assets, &format!("{}/{skin}", def.path), specs)
                .or_else(|| texture::resolve(assets, &skin, specs))
                .context("Missing blink texture")?;
            let image = texture::decode(assets, &path)?;
            let closed = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
            closed.set_filter(FilterMode::Linear);
            Some(Blink {
                surface: index,
                period,
                open,
                closed,
            })
        } else {
            None
        };
        Ok(Self { rig, blink })
    }
    pub fn mouth_angle(&mut self, degrees: f32) {
        if degrees.is_finite() && (0. ..=90.).contains(&degrees) {
            self.rig.angle = degrees.to_radians();
        }
    }
    pub fn pose(&self, skeleton: &Skeleton, local: &[Transform], mouth: f32) -> Vec<Transform> {
        let mut local = local.to_vec();
        self.rig.apply(&mut local, mouth);
        skeleton.global_pose(&local)
    }
    pub fn blink(&self, meshes: &mut [Option<Mesh>], time: f32, alive: bool) {
        if let Some(b) = &self.blink {
            if let Some(mesh) = &mut meshes[b.surface] {
                mesh.texture = Some(
                    if alive && blinking(time, b.period) {
                        &b.closed
                    } else {
                        &b.open
                    }
                    .clone(),
                );
            }
        }
    }
}
fn blinking(time: f32, period: f32) -> bool {
    time.is_finite() && time >= 0. && time.rem_euclid(period) >= period - 0.14
}

/// Bounded additive attention, used only when the reviewed dialogue script
/// explicitly names a headwatch target. The authored head/eye channels survive.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Watch {
    yaw: f32,
    pitch: f32,
}
impl Watch {
    pub fn valid(&self) -> bool {
        self.yaw.is_finite()
            && self.pitch.is_finite()
            && self.yaw.abs() <= 0.79
            && self.pitch.abs() <= 0.53
    }
    pub fn update(&mut self, dt: f32, direction: Option<Vec3>) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        let (yaw, pitch) = direction
            .filter(|d| d.is_finite() && d.length_squared() > 0.01)
            .map_or((0., 0.), |d| {
                (
                    d.y.atan2(d.x).clamp(-0.78, 0.78),
                    -d.z.atan2(d.truncate().length()).clamp(-0.52, 0.52),
                )
            });
        let t = 1. - (-dt * 6.).exp();
        self.yaw += (yaw - self.yaw) * t;
        self.pitch += (pitch - self.pitch) * t;
    }
    pub fn apply(&self, skeleton: &Skeleton, local: &mut [Transform]) {
        if self.yaw.abs() + self.pitch.abs() < 0.00001 {
            return;
        }
        let Some(head) = skeleton
            .bones
            .iter()
            .position(|b| b.name.eq_ignore_ascii_case("tag_head"))
            .and_then(|tag| skeleton.bones[tag].parent)
            .or_else(|| skeleton.bones.iter().position(|b| b.name.eq_ignore_ascii_case("bip01 head")))
        else {
            return;
        };
        let global = skeleton.global_pose(local);
        let parent = skeleton.bones[head]
            .parent
            .map_or(Quat::IDENTITY, |i| global[i].rotation);
        let turn = Quat::from_rotation_z(self.yaw) * Quat::from_rotation_y(self.pitch);
        local[head].rotation =
            (parent.conjugate() * turn * parent * local[head].rotation).normalize();
    }
}

pub fn check(assets: &mut Assets) -> Result<()> {
    let names: Vec<_> = assets
        .names()
        .filter(|n| n.ends_with(".lip"))
        .map(str::to_owned)
        .collect();
    let mut samples = 0;
    for name in &names {
        let track = LipTrack::parse(&assets.read(name)?).with_context(|| name.clone())?;
        for i in 0..track.samples.len() * 2 {
            ensure!(
                (0. ..=1.).contains(&track.sample(i as f32 / 40.)),
                "Invalid mouth envelope"
            );
        }
        ensure!(
            track.sample(track.duration) == 0.,
            "Mouth remains open after voice"
        );
        samples += track.samples.len();
    }
    for name in crate::story::speakers::models() {
        let path = format!("models/{name}.tik");
        let def = Definition::load(assets, &path)?;
        let skeleton = Skeleton::parse(&assets.read(&format!("{}/{}", def.path, def.model))?)?;
        let settings = Settings::parse(&String::from_utf8_lossy(&assets.read(&path)?))?;
        let rig = Rig::new(&skeleton, &settings);
        let neutral = crate::story::speakers::neutral_model(name);
        ensure!(
            rig.mouth.is_none() == neutral,
            "Unexpected mouth binding for {name}"
        );
        let clip = def
            .animations
            .values()
            .find(|n| n.ends_with(".ska"))
            .context("No skeletal clip")?;
        let clip = crate::skeletal::Animation::parse(
            &assets.read(&format!("{}/{clip}", def.path))?,
            skeleton.bones.len(),
        )?;
        let local = clip.sample(0., false);
        // Head and eye choreography belongs to the original skeletal clips.
        // Verify that the speech controller only changes its mouth joint.
        let authored = local.clone();
        let before = skeleton.global_pose(&local);
        let mut after = local;
        rig.apply(&mut after, 1.);
        ensure!(
            after
                .iter()
                .zip(&authored)
                .enumerate()
                .all(|(i, (a, b))| Some(i) == rig.mouth
                    || (a.rotation == b.rotation && a.translation == b.translation)),
            "Speech modified authored head/eye behaviour for {name}"
        );
        let after = skeleton.global_pose(&after);
        let mut moved = 0;
        for vertex in skeleton.surfaces.iter().flat_map(|s| &s.vertices) {
            let p = vertex.position(&after);
            ensure!(p.is_finite(), "Invalid facial skinning for {name}");
            moved += usize::from(p.distance(vertex.position(&before)) > 0.001);
        }
        ensure!(
            if neutral { moved == 0 } else { moved > 0 },
            "Invalid mouth deformation for {name}"
        );
        println!(
            "PASS facial rig {name}: {moved} vertices, {:.0} degree jaw range",
            rig.angle.to_degrees()
        );
    }
    println!(
        "PASS {} original lip tracks / {samples} envelope samples",
        names.len()
    );
    crate::story::check_faces(assets)?;
    Ok(())
}

/// Close-up comparison through the same skinning/texture path used in the game.
pub async fn render(assets: &mut Assets) -> Result<()> {
    let specs = texture::read_materials(assets)?;
    let material = crate::character::skin_material()?;
    for name in crate::story::speakers::models() {
        let mut actor = crate::npc::Puppet::load(assets, name, &["idle_base_01"], &specs)?;
        actor.show_attachments(crate::story::speakers::neutral_model(name));
        let clip = if name == "alice" {
            "idle_base_01"
        } else {
            actor.idle_clip()
        }
        .to_owned();
        let transform = Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        };
        let target = actor
            .tag(
                crate::story::speakers::focus(name),
                &clip,
                0.,
                transform,
                1.,
            )
            .context("Missing facial focus")?
            .translation
            + Vec3::Z * 3.;
        for (label, mouth, time) in [("closed", 0., 0.), ("open", 1., 0.), ("blink", 0., 3.9)] {
            if label == "blink" && name != "alice" {
                continue;
            }
            for frame in 0..3 {
                clear_background(Color::new(0.10, 0.12, 0.15, 1.));
                set_camera(&Camera3D {
                    position: target
                        + vec3(1., -0.3, 0.07) * crate::story::speakers::portrait_distance(name),
                    target,
                    up: Vec3::Z,
                    fovy: 27_f32.to_radians(),
                    z_near: 0.5,
                    z_far: 1000.,
                    ..Default::default()
                });
                material.bind();
                actor.mouth(mouth);
                actor.draw(
                    &clip,
                    time,
                    true,
                    Transform {
                        translation: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    1.,
                    true,
                );
                gl_use_default_material();
                set_default_camera();
                draw_text(&format!("{name}: {label}"), 20., 30., 24., WHITE);
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/facial-{name}-{label}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
    }
    println!(
        "PASS facial close-up render: registered speaker rigs, open/closed mouths and Alice blink"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_default_and_headwatch_are_bounded_pauseable_and_saved() {
        let sk = Skeleton {
            surfaces: vec![],
            bones: vec![],
        };
        assert!((Rig::new(&sk, &Settings::default()).angle.to_degrees() - 10.).abs() < 0.001);
        let mut w = Watch::default();
        w.update(1., Some(vec3(-10., 100., 100.)));
        assert!(w.valid());
        let saved = serde_json::to_vec(&w).unwrap();
        w.update(0., None);
        assert_eq!(saved, serde_json::to_vec(&w).unwrap());
        let mut restored: Watch = serde_json::from_slice(&saved).unwrap();
        w.update(0.2, None);
        restored.update(0.2, None);
        assert_eq!(
            serde_json::to_vec(&w).unwrap(),
            serde_json::to_vec(&restored).unwrap()
        );
    }
    #[test]
    fn lip_track_interpolates_at_source_rate_and_closes() {
        let track = LipTrack::parse(b"0.15\r\n5\r\n0 255 0 0 0").unwrap();
        assert_eq!(track.sample(0.), 0.);
        assert!((track.sample(0.025) - 0.5).abs() < 0.001);
        assert_eq!(track.sample(0.05), 1.);
        for time in [-1., 0.15, 100., f32::NAN] {
            assert_eq!(track.sample(time), 0.);
        }
    }
    #[test]
    fn rejects_invalid_and_truncated_tracks() {
        for bytes in [
            "",
            "NaN 1 0",
            "0 1 0",
            "1 0",
            "1 2 0",
            "1 1 0 2",
            "1 1 -1",
            "1 1 256",
            "1 999999999",
        ] {
            assert!(LipTrack::parse(bytes.as_bytes()).is_err(), "{bytes}");
        }
    }
    #[test]
    fn metadata_ignores_comments_clients_and_animation_commands() {
        let s = Settings::parse("setup { surface Face shader open.tga surface Face shader closed.tga } init { server { blinkinfo Face 4 maxmouthangle 30 } client { maxmouthangle 80 } } animations { idle test.ska { server { maxmouthangle 90 } } }").unwrap();
        assert_eq!(s.angle, Some(30.));
        assert_eq!(s.blink, Some(("face".into(), 4., "closed.tga".into())));
        assert!(
            Settings::parse("// maxmouthangle 90\ninit { server { maxmouthangle NaN } }").is_err()
        );
        assert!(!blinking(0., 4.));
        assert!(blinking(3.9, 4.));
        assert!(!blinking(4., 4.));
    }
    #[test]
    fn jaw_layer_preserves_head_and_body_and_moves_its_children() {
        use crate::skeletal::Bone;
        let sk = Skeleton {
            surfaces: vec![],
            bones: vec![
                Bone {
                    parent: None,
                    name: "head".into(),
                },
                Bone {
                    parent: Some(0),
                    name: "tag_mouth".into(),
                },
                Bone {
                    parent: Some(1),
                    name: "lip".into(),
                },
            ],
        };
        let rig = Rig::new(
            &sk,
            &Settings {
                angle: Some(30.),
                ..Default::default()
            },
        );
        let mut pose = vec![
            Transform {
                rotation: Quat::IDENTITY,
                translation: Vec3::ZERO
            };
            3
        ];
        pose[0].rotation = Quat::from_rotation_z(0.6);
        pose[2].translation = -Vec3::Y;
        let before = pose.clone();
        rig.apply(&mut pose, 1.);
        assert_eq!(pose[0].rotation, before[0].rotation);
        assert_eq!(pose[1].translation, before[1].translation);
        assert!(sk.global_pose(&pose)[2].translation.z < -0.49);
    }
}
