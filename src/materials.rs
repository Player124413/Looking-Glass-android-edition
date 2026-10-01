//! Bounded, declarative subset of the shipped material language. Never executes scripts.
use crate::texture::MaterialSpec;
use anyhow::{ensure, Result};
use macroquad::prelude::*;
use std::collections::BTreeMap;

/// Sides retained by a material's `cull` declaration. TAN front winding is clockwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FaceSide {
    #[default]
    Front,
    Back,
    Both,
}
impl FaceSide {
    pub fn visible(self, [a, b, c]: [Vec3; 3], eye: Vec3) -> bool {
        let facing = (b - a).cross(c - a).dot(eye - a);
        match self {
            Self::Front => facing < 0.,
            Self::Back => facing > 0.,
            Self::Both => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Blend {
    #[default]
    Opaque,
    Alpha,
    Add,
    Filter,
    DoubleFilter,
    AlphaAdd,
    ColorAdd,
    AlphaColor,
    Invisible,
    /// Declarative GL source/destination factors used by less common original layers.
    Factors(u8, u8),
}
fn blend_factor(s: &str) -> Option<u8> {
    [
        "gl_zero",
        "gl_one",
        "gl_src_color",
        "gl_one_minus_src_color",
        "gl_dst_color",
        "gl_one_minus_dst_color",
        "gl_src_alpha",
        "gl_one_minus_src_alpha",
        "gl_dst_alpha",
        "gl_one_minus_dst_alpha",
    ]
    .iter()
    .position(|v| *v == s)
    .map(|i| i as u8)
}
#[derive(Clone, Debug)]
pub struct Wave {
    pub kind: String,
    pub values: [f32; 4],
}
impl Wave {
    pub fn sample(&self, time: f32) -> f32 {
        let [base, amplitude, phase, frequency] = self.values;
        let x = (phase + time * frequency).rem_euclid(1.);
        let wave = match self.kind.as_str() {
            "sin" => (x * std::f32::consts::TAU).sin(),
            "triangle" => 1. - 4. * (x - 0.5).abs(),
            "square" => {
                if x < 0.5 {
                    1.
                } else {
                    -1.
                }
            }
            "sawtooth" => x,
            "inversesawtooth" => 1. - x,
            "noise" => {
                // Continuous deterministic noise: stable across pause/restart and frame rates.
                let p = phase + time * frequency;
                let noise =
                    |x: f32| ((x * 12.9898 + 78.233).sin() * 43758.547).rem_euclid(1.) * 2. - 1.;
                let f = p.rem_euclid(1.);
                noise(p.floor()) * (1. - f) + noise(p.floor() + 1.) * f
            }
            _ => 0.,
        };
        base + amplitude * wave
    }
}
#[derive(Clone, Debug)]
pub enum TcMod {
    Transform([f32; 6]),
    Scale(Vec2),
    Scroll(Vec2),
    Rotate(f32),
    Turb([f32; 4]),
    Stretch(Wave),
}
#[derive(Clone, Debug)]
pub enum Deform {
    Wave(f32, Wave),
    WaveNormal(f32, Vec3, Wave),
    Move(Vec3, Wave),
    Bulge([f32; 3]),
    Sprite(bool),
}
impl Deform {
    /// Normal ripples do not move a surface or invalidate its visibility bounds.
    pub fn moves_vertices(&self) -> bool {
        !matches!(self, Self::WaveNormal(..))
    }
    pub fn normal(&self, p: Vec3, n: Vec3, time: f32) -> Vec3 {
        if let Self::WaveNormal(spread, direction, wave) = self {
            let mut wave = wave.clone();
            wave.values[2] += (p.x + p.y + p.z) / spread;
            (n + *direction * wave.sample(time)).try_normalize().unwrap_or(n)
        } else {
            n
        }
    }
    pub fn position(&self, p: Vec3, n: Vec3, uv: Vec2, time: f32) -> Vec3 {
        match self {
            Self::Wave(spread, w) => {
                let mut w = w.clone();
                w.values[2] += (p.x + p.y + p.z) / spread.max(0.001);
                p + n * w.sample(time)
            }
            Self::Move(v, w) => p + *v * w.sample(time),
            Self::Bulge([width, height, speed]) => {
                p + n * (uv.x * width + time * speed).sin() * *height
            }
            Self::Sprite(_) | Self::WaveNormal(..) => p,
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Stage {
    pub images: Vec<String>,
    pub fps: f32,
    pub blend: Blend,
    pub clamp: bool,
    pub mods: Vec<TcMod>,
    pub rgb: Option<Wave>,
    pub alpha: Option<Wave>,
    pub constant_alpha: Option<f32>,
    pub identity: bool,
    pub detail: bool,
    pub unsupported: bool,
    pub environment: bool,
    pub vector_uv: Option<(Vec3, Vec3)>,
    pub vertex_alpha: bool,
    pub dot_alpha: Option<Vec2>,
    pub inverse_dot: bool,
    pub constant_rgb: Option<Vec3>,
    /// 0 none, 1 GT0, 2 GE128, 3 LT128, 4 legacy cutout, 5..8 explicit comparisons.
    pub alpha_test: u8,
    pub alpha_reference: f32,
    pub depth_write: bool,
    pub depth_equal: bool,
}
impl Stage {
    pub fn view_uv(&self, uv: Vec2, position: Vec3, normal: Vec3, eye: Vec3, time: f32) -> Vec2 {
        let uv = if let Some((s, t)) = self.vector_uv {
            vec2(position.dot(s), position.dot(t))
        } else if self.environment {
            let view = (eye - position).normalize_or_zero();
            let reflected = normal * (2. * normal.dot(view)) - view;
            vec2(0.5 + reflected.y * 0.5, 0.5 - reflected.z * 0.5)
        } else {
            uv
        };
        self.uv(uv, position, time)
    }
    pub fn opacity(&self, normal: Vec3, eye_direction: Vec3, vertex: u8) -> f32 {
        let mut alpha = if self.vertex_alpha {
            vertex as f32 / 255.
        } else {
            1.
        };
        if let Some(range) = self.dot_alpha {
            let dot = normal.dot(eye_direction).abs().clamp(0., 1.);
            let t = if self.inverse_dot { 1. - dot } else { dot };
            alpha *= range.x + (range.y - range.x) * t;
        }
        alpha.clamp(0., 1.)
    }
    pub fn frame(&self, time: f32) -> usize {
        if self.images.is_empty() {
            0
        } else {
            ((time.max(0.) * self.fps) as usize) % self.images.len()
        }
    }
    pub fn uv(&self, mut uv: Vec2, position: Vec3, time: f32) -> Vec2 {
        for m in &self.mods {
            match m {
                TcMod::Transform([a, b, c, d, x, y]) => {
                    uv = vec2(a * uv.x + b * uv.y + x, c * uv.x + d * uv.y + y)
                }
                TcMod::Scale(v) => uv *= *v,
                TcMod::Scroll(v) => uv += *v * time,
                TcMod::Rotate(deg) => {
                    let a = (-deg * time).to_radians();
                    let p = uv - Vec2::splat(0.5);
                    uv = vec2(a.cos() * p.x - a.sin() * p.y, a.sin() * p.x + a.cos() * p.y)
                        + Vec2::splat(0.5);
                }
                TcMod::Turb([_, amp, phase, freq]) => {
                    let cycles = phase + time * freq;
                    uv += vec2(
                        (((position.x + position.z) / 1024. + cycles) * std::f32::consts::TAU).sin(),
                        ((position.y / 1024. + cycles) * std::f32::consts::TAU).sin(),
                    ) * *amp;
                }
                TcMod::Stretch(w) => {
                    let v = w.sample(time).abs().max(0.01);
                    uv = (uv - Vec2::splat(0.5)) / v + Vec2::splat(0.5);
                }
            }
        }
        uv
    }
}

/// Keep newlines: animMap is terminated by the end of its authored line.
pub fn lines(text: &str) -> Vec<Vec<String>> {
    let mut clean = String::new();
    let mut chars = text.chars().peekable();
    let mut block = false;
    let mut line = false;
    let mut quote = false;
    while let Some(c) = chars.next() {
        if c == '\n' || c == '\r' {
            clean.push('\n');
            line = false;
            continue;
        }
        if block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                block = false;
                clean.push(' ');
            }
            continue;
        }
        if line {
            continue;
        }
        if !quote && c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            block = true;
            continue;
        }
        if !quote && c == '/' && chars.peek() == Some(&'/') {
            chars.next();
            line = true;
            continue;
        }
        if c == '"' {
            quote = !quote;
            clean.push(c);
            continue;
        }
        if !quote && (c == '{' || c == '}') {
            clean.push('\n');
            clean.push(c);
            clean.push('\n');
        } else {
            clean.push(c);
        }
    }
    clean
        .lines()
        .filter_map(|l| crate::bsp::tokens(l).ok())
        .filter(|l| !l.is_empty())
        .collect()
}
fn number(s: &str) -> Option<f32> {
    s.parse::<f32>()
        .ok()
        .filter(|f| f.is_finite() && f.abs() < 1e6)
}
fn four(t: &[String]) -> Option<[f32; 4]> {
    Some([
        number(t.first()?)?,
        number(t.get(1)?)?,
        number(t.get(2)?)?,
        number(t.get(3)?)?,
    ])
}
fn wave(t: &[String]) -> Option<Wave> {
    Some(Wave {
        kind: t.first()?.to_ascii_lowercase(),
        values: four(t.get(1..)?)?,
    })
}

pub fn parse(text: &str) -> Result<BTreeMap<String, MaterialSpec>> {
    let lines = lines(text);
    let mut out = BTreeMap::new();
    let mut name = String::new();
    let mut spec = MaterialSpec::default();
    let mut stage = Stage::default();
    let mut depth = 0;
    for original in lines {
        let t = original
            .iter()
            .map(|s| s.to_ascii_lowercase())
            .collect::<Vec<_>>();
        let key = t[0].as_str();
        if key == "{" {
            depth += 1;
            ensure!(depth <= 2, "Material nesting too deep");
            continue;
        }
        if key == "}" {
            if depth == 2 {
                ensure!(spec.stages.len() < 32, "Too many material stages");
                if !stage.images.is_empty() {
                    spec.stages.push(std::mem::take(&mut stage));
                } else {
                    stage = Stage::default();
                }
            }
            if depth == 1 {
                out.insert(name.clone(), std::mem::take(&mut spec));
            }
            depth = (depth - 1).max(0);
            continue;
        }
        if depth == 0 {
            name = t[0].clone();
            continue;
        }
        let arg = t.get(1).map(String::as_str).unwrap_or("");
        if depth == 1 {
            match key {
                "qer_editorimage" => spec.candidates.push(arg.into()),
                "cull" => {
                    spec.face_side = match arg {
                        "none" | "disable" | "twosided" => FaceSide::Both,
                        "back" | "backsided" => FaceSide::Back,
                        _ => FaceSide::Front,
                    }
                }
                "surfaceparm" => match arg {
                    "nodraw" => spec.hidden = true,
                    "sky" => spec.sky = true,
                    "trans" => spec.transparent = true,
                    "nolightmap" => spec.unlit = true,
                    "fog" => spec.hidden = true,
                    _ => {}
                },
                "fogonly" => spec.hidden = true,
                "portalsky" => {
                    spec.sky = true;
                    spec.portal_sky = true;
                }
                "portal" => spec.camera_portal = true,
                "skyparms" => {
                    spec.sky = true;
                    spec.cloud_height = t.get(2).and_then(|s| number(s)).unwrap_or(512.);
                }
                "deformvertexes" => {
                    let d = match arg {
                        "wave" if t.len() >= 3 => t
                            .get(2)
                            .and_then(|s| number(s))
                            .zip(wave(&t[3..]))
                            .map(|(s, w)| Deform::Wave(s, w)),
                        "wavenormal" if t.len() >= 11 => {
                            let values = t[2..6].iter().map(|s| number(s)).collect::<Option<Vec<_>>>();
                            values.zip(wave(&t[6..])).map(|(v, w)| {
                                // The original substitutes a reciprocal spread of 100 for zero.
                                Deform::WaveNormal(if v[0] == 0. { 0.01 } else { v[0] }, vec3(v[1], v[2], v[3]), w)
                            })
                        }
                        "move" if t.len() >= 9 => {
                            let v = t[2..5]
                                .iter()
                                .map(|s| number(s))
                                .collect::<Option<Vec<_>>>();
                            v.zip(wave(&t[5..]))
                                .map(|(v, w)| Deform::Move(vec3(v[0], v[1], v[2]), w))
                        }
                        "bulge" if t.len() >= 5 => t[2..5]
                            .iter()
                            .map(|s| number(s))
                            .collect::<Option<Vec<_>>>()
                            .map(|v| Deform::Bulge([v[0], v[1], v[2]])),
                        "autosprite" | "autosprite2" => Some(Deform::Sprite(arg == "autosprite2")),
                        _ => None,
                    };
                    if let Some(d) = d {
                        spec.deforms.push(d);
                    }
                }
                "fogparms" => {
                    let nums = t[1..]
                        .iter()
                        .filter(|s| s.as_str() != "(" && s.as_str() != ")")
                        .cloned()
                        .collect::<Vec<_>>();
                    spec.fog = four(&nums).filter(|v| v[3] > 0.);
                }
                _ => {}
            }
            continue;
        }
        match key {
            "map" | "clampmap" => {
                stage.images = vec![arg.into()];
                stage.clamp = key == "clampmap";
            }
            "animmap" => {
                stage.fps = number(arg).unwrap_or(0.).clamp(0., 1000.);
                stage.images = t[2..].to_vec();
                ensure!(stage.images.len() <= 256, "Too many animation frames");
            }
            "blendfunc" => {
                stage.blend = match t[1..].join(" ").as_str() {
                    "add" | "gl_one gl_one" => Blend::Add,
                    "blend" | "gl_src_alpha gl_one_minus_src_alpha" => Blend::Alpha,
                    "filter" | "gl_dst_color gl_zero" | "gl_zero gl_src_color" => Blend::Filter,
                    "gl_dst_color gl_src_color" => Blend::DoubleFilter,
                    "gl_src_alpha gl_one" => Blend::AlphaAdd,
                    "gl_dst_color gl_one" => Blend::ColorAdd,
                    "gl_src_alpha gl_src_color" => Blend::AlphaColor,
                    "gl_one gl_zero" => Blend::Opaque,
                    "gl_zero gl_one" => Blend::Invisible,
                    _ => match t
                        .get(1)
                        .and_then(|s| blend_factor(s))
                        .zip(t.get(2).and_then(|s| blend_factor(s)))
                    {
                        Some((s, d)) => Blend::Factors(s, d),
                        None => {
                            stage.unsupported = true;
                            Blend::Alpha
                        }
                    },
                }
            }
            "rgbgen" => {
                stage.identity = arg == "identity" || arg == "identitylighting";
                if arg == "wave" {
                    stage.rgb = wave(&t[2..]);
                }
                if matches!(arg, "const" | "constant") {
                    let n = t[2..]
                        .iter()
                        .filter(|s| s.as_str() != "(" && s.as_str() != ")")
                        .filter_map(|s| number(s))
                        .collect::<Vec<_>>();
                    if n.len() == 3 {
                        stage.constant_rgb = Some(vec3(n[0], n[1], n[2]));
                    }
                }
            }
            "alphagen" => match arg {
                "wave" => stage.alpha = wave(&t[2..]),
                "const" | "constant" => stage.constant_alpha = t.get(2).and_then(|s| number(s)),
                "vertex" | "fromentity" => stage.vertex_alpha = true,
                "dot" | "oneminusdot" => {
                    stage.dot_alpha = t
                        .get(2)
                        .and_then(|s| number(s))
                        .zip(t.get(3).and_then(|s| number(s)))
                        .map(|(a, b)| vec2(a, b));
                    stage.inverse_dot = arg == "oneminusdot";
                }
                _ => {}
            },
            "tcgen" => {
                stage.environment = arg == "environment";
                if arg == "vector" {
                    let n = t[2..].iter().filter_map(|s| number(s)).collect::<Vec<_>>();
                    if n.len() == 6 {
                        stage.vector_uv = Some((vec3(n[0], n[1], n[2]), vec3(n[3], n[4], n[5])));
                    } else {
                        stage.unsupported = true;
                    }
                } else if !["base", "texture", "environment", "lightmap"].contains(&arg) {
                    stage.unsupported = true;
                }
            }
            "alphafunc" => {
                stage.alpha_test = match arg {
                    "gt0" => 1,
                    "ge128" => 2,
                    "lt128" => 3,
                    _ => 0,
                }
            }
            "alphatest" => {
                stage.alpha_test = match arg {
                    "greaterequal" => 5,
                    "greater" => 6,
                    "less" => 7,
                    "lessequal" => 8,
                    _ => 0,
                };
                stage.alpha_reference =
                    t.get(2).and_then(|s| number(s)).unwrap_or(0.).clamp(0., 1.);
            }
            "depthwrite" => stage.depth_write = true,
            "depthfunc" => stage.depth_equal = arg == "equal",
            "detail" => stage.detail = true,
            "tcmod" => {
                let n = |i: usize| t.get(i).and_then(|s| number(s.as_str()));
                let m = match arg {
                    "transform" => t[2..]
                        .iter()
                        .map(|s| number(s))
                        .collect::<Option<Vec<_>>>()
                        .and_then(|v| v.try_into().ok())
                        .map(TcMod::Transform),
                    "scroll" => n(2).zip(n(3)).map(|(x, y)| TcMod::Scroll(vec2(x, y))),
                    "scale" => n(2).zip(n(3)).map(|(x, y)| TcMod::Scale(vec2(x, y))),
                    "rotate" => n(2).map(TcMod::Rotate),
                    "turb" => four(&t[2..]).map(TcMod::Turb),
                    "stretch" => wave(&t[2..]).map(TcMod::Stretch),
                    _ => None,
                };
                if let Some(m) = m {
                    ensure!(stage.mods.len() < 16, "Too many texture transforms");
                    stage.mods.push(m);
                } else {
                    stage.unsupported = true;
                }
            }
            _ => {}
        }
    }
    ensure!(depth == 0, "Unclosed material");
    for spec in out.values_mut() {
        let mut candidates = spec
            .stages
            .iter()
            .flat_map(|s| s.images.iter())
            .filter(|s| !s.starts_with('$') && !s.starts_with('*'))
            .cloned()
            .collect::<Vec<_>>();
        candidates.append(&mut spec.candidates);
        spec.candidates = candidates;
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sky_material_sides_and_clockwise_tan_faces() {
        for (directive, expected) in [
            ("", FaceSide::Front),
            ("cull front", FaceSide::Front),
            ("cull back", FaceSide::Back),
            ("cull none", FaceSide::Both),
            ("cull disable", FaceSide::Both),
            ("cull twosided", FaceSide::Both),
        ] {
            let spec = parse(&format!("test\n{{\n{directive}\n}}\n")).unwrap();
            assert_eq!(spec["test"].face_side, expected);
        }
        let clockwise = [Vec3::ZERO, Vec3::Y, Vec3::X];
        assert!(FaceSide::Front.visible(clockwise, Vec3::Z));
        assert!(!FaceSide::Front.visible(clockwise, -Vec3::Z));
        assert!(FaceSide::Back.visible(clockwise, -Vec3::Z));
        assert!(!FaceSide::Back.visible(clockwise, Vec3::Z));
        assert!(FaceSide::Both.visible(clockwise, Vec3::Z));
        assert!(FaceSide::Both.visible(clockwise, -Vec3::Z));
        let rotation = Quat::from_rotation_y(1.2);
        let shifted = clockwise.map(|p| vec3(100., 200., 300.) + rotation * p);
        assert!(FaceSide::Front.visible(shifted, vec3(100., 200., 300.) + rotation * Vec3::Z));
    }
    #[test]
    fn uncommon_blend_and_vector_transform_are_preserved() {
        let m = parse("reflect\n{\n{\nmap image\nblendfunc GL_ONE_MINUS_DST_COLOR GL_ONE\ntcgen vector ( 1 0 0 ) ( 0 2 0 )\ntcmod transform 2 0 0 3 .5 .25\n}\n}").unwrap();
        let s = &m["reflect"].stages[0];
        assert_eq!(s.blend, Blend::Factors(5, 1));
        assert!(!s.unsupported);
        assert_eq!(
            s.view_uv(Vec2::ZERO, vec3(2., 3., 4.), Vec3::Z, Vec3::Z, 0.),
            vec2(4.5, 18.25)
        );
        assert!(
            parse("bad\n{\n{\nmap x\nblendfunc GL_FAKE GL_ONE\n}\n}").unwrap()["bad"].stages[0]
                .unsupported
        );
    }
    #[test]
    fn slime_retains_its_base_blend_and_view_alpha() {
        let m = parse("slime\n{\nsurfaceparm trans\n{\nmap base\nblendFunc GL_SRC_ALPHA GL_SRC_COLOR\nalphaGen dot 1 0\ntcMod turb 0 .1 .75 .2\n}\n{\nmap detail\nblendFunc GL_DST_COLOR GL_ONE\n}\n}").unwrap();
        let s = &m["slime"].stages;
        assert_eq!(s.len(), 2);
        assert!(s.iter().all(|s| !s.unsupported));
        assert_eq!(s[0].blend, Blend::AlphaColor);
        assert_eq!(s[1].blend, Blend::ColorAdd);
        assert_eq!(s[0].opacity(Vec3::Z, Vec3::Z, 255), 0.);
        assert_eq!(s[0].opacity(Vec3::Z, Vec3::X, 255), 1.);
        assert_ne!(
            s[0].uv(Vec2::ZERO, Vec3::ZERO, 0.),
            s[0].uv(Vec2::ZERO, Vec3::ZERO, 1.)
        );
    }
    #[test]
    fn camera_portals_remain_distinct_from_sky_portals() {
        let m = parse("door\n{\nportal\nsurfaceparm nolightmap\n{\nmap textures/common/portal.tga\nblendFunc GL_ZERO GL_ONE\n}\n}\nsky\n{\nportalSky\n}\n").unwrap();
        assert!(m["door"].camera_portal && !m["door"].sky && !m["door"].portal_sky);
        assert_eq!(m["door"].stages[0].blend, Blend::Invisible);
        assert!(!m["sky"].camera_portal && m["sky"].portal_sky);
    }
    #[test]
    fn authored_sky_cutout_reflection_and_vertex_alpha_are_preserved() {
        let m=parse("sky\n{\nportalSky\nsurfaceparm sky\n}\nwater\n{\nsurfaceparm trans\n{\nmap water\nblendFunc blend\ntcGen environment\nalphaGen dot .2 .8\nalphaFunc GE128\ndepthWrite\n}\n}\nleaf\n{\n{\nmap leaf\nalphaGen vertex\nalphaFunc GT0\n}\n}").unwrap();
        assert!(m["sky"].sky && m["sky"].portal_sky && !m["sky"].hidden);
        let s = &m["water"].stages[0];
        assert!(!s.unsupported && s.environment && s.depth_write && s.alpha_test == 2);
        assert!((s.opacity(Vec3::Z, Vec3::Z, 255) - 0.8).abs() < 0.001);
        assert!((s.opacity(Vec3::Z, Vec3::X, 255) - 0.2).abs() < 0.001);
        assert_ne!(
            s.view_uv(Vec2::ZERO, Vec3::ZERO, Vec3::Z, Vec3::Z, 0.),
            s.view_uv(Vec2::ZERO, Vec3::ZERO, Vec3::Z, Vec3::X, 0.)
        );
        assert!((m["leaf"].stages[0].opacity(Vec3::Z, Vec3::Z, 64) - 64. / 255.).abs() < 0.001);
    }
    #[test]
    fn material_deformation_and_noise_are_repeatable_without_mutating_collision() {
        let m = parse("w\n{\ndeformVertexes wave 128 sin 0 4 0 1\n{\nmap x\n}\n}").unwrap();
        let p = Vec3::ZERO;
        let d = &m["w"].deforms[0];
        assert!((d.position(p, Vec3::Z, Vec2::ZERO, 0.25).z - 4.).abs() < 0.001);
        assert_eq!(p, Vec3::ZERO);
        let wave = Wave {
            kind: "noise".into(),
            values: [0.5, 0.5, 0., 10.],
        };
        for i in -100..100 {
            let t = i as f32 / 37.;
            let v = wave.sample(t);
            assert!((0.0..=1.).contains(&v));
            assert_eq!(v, wave.sample(t));
        }
        assert!(parse("w\n{\ndeformVertexes wave\n}").is_ok());
    }
    #[test]
    fn water_normal_waves_animate_reflection_without_moving_the_surface() {
        let m = parse("water\n{\ndeformVertexes wavenormal 512 .04 .03 0 sin 0 1 0 .3\n{\nmap reflection\ntcGen environment\nalphaGen dot .5 0\n}\n}").unwrap();
        let d = &m["water"].deforms[0];
        let p = Vec3::X * 128.;
        let expected = vec3(0.04, 0.03, 1.).normalize();
        assert!(d.normal(p, Vec3::Z, 0.).distance(expected) < 0.00001);
        assert!(d.normal(p, Vec3::Z, 1. / 0.6).distance(vec3(-0.04, -0.03, 1.).normalize()) < 0.00001);
        assert!(!d.moves_vertices());
        let stage = &m["water"].stages[0];
        let eye = p + vec3(100., 40., 30.);
        assert_ne!(stage.view_uv(Vec2::ZERO, p, Vec3::Z, eye, 0.), stage.view_uv(Vec2::ZERO, p, expected, eye, 0.));
        for time in [0., 0.25, 1., 100.] {
            assert_eq!(d.position(p, Vec3::Z, Vec2::ZERO, time), p);
            assert!((d.normal(p, Vec3::Z, time).length() - 1.).abs() < 0.00001);
            assert_eq!(d.normal(p, Vec3::Z, time), d.normal(p, Vec3::Z, time));
        }
        assert!(parse("w\n{\ndeformVertexes wavenormal 512 .04\n}").unwrap()["w"].deforms.is_empty());
        let zero = parse("w\n{\ndeformVertexes wavenormal 0 .04 .03 0 sin 0 1 0 .3\n}").unwrap();
        assert!(zero["w"].deforms[0].normal(p, Vec3::Z, 0.).is_finite());
    }
    #[test]
    fn water_turbulence_has_the_original_world_space_period() {
        let s = Stage { mods: vec![TcMod::Turb([0., 0.1, 0., 0.25])], ..Default::default() };
        let uv = vec2(0.25, 0.4);
        let quarter = vec3(128., 256., 128.);
        assert!(s.uv(uv, quarter, 0.).distance(uv + Vec2::splat(0.1)) < 0.00001);
        assert!(s.uv(uv, Vec3::splat(1024.), 0.).distance(uv) < 0.00001);
        assert!(s.uv(uv, Vec3::ZERO, 1.).distance(uv + Vec2::splat(0.1)) < 0.00001);
        assert!(s.uv(uv, quarter, 4.).distance(s.uv(uv, quarter, 0.)) < 0.00001);
    }
    #[test]
    fn material_stages_preserve_animation_blend_and_comments() {
        let m=parse("flame\n{\n// disabled map evil\n{\nanimMap 10 a.tga b.tga\nblendFunc GL_ONE GL_ONE\nrgbGen wave sawtooth 0 1 0 10\n}\n}\nfog\n{\nfogParms 0.1 0.2 0.3 200\n}").unwrap();
        let s = &m["flame"].stages[0];
        assert_eq!(s.images.len(), 2);
        assert_eq!(s.frame(0.11), 1);
        assert_eq!(s.blend, Blend::Add);
        assert!((s.rgb.as_ref().unwrap().sample(0.025) - 0.25).abs() < 1e-5);
        assert_eq!(m["fog"].fog.unwrap()[3], 200.);
    }
    #[test]
    fn ordered_uv_modifiers_and_invalid_input() {
        let s = Stage {
            mods: vec![TcMod::Scale(vec2(2., 3.)), TcMod::Scroll(vec2(1., -1.))],
            ..Default::default()
        };
        assert_eq!(s.uv(vec2(0.5, 0.5), Vec3::ZERO, 2.), vec2(3., -0.5));
        assert!(parse("bad\n{").is_err());
        assert_eq!(lines("/* ignored { */ good\n{\n}")[0], vec!["good"]);
    }
    #[test]
    fn explicit_alpha_thresholds_survive_material_parsing() {
        for (comparison, mode) in [
            ("greaterequal", 5),
            ("greater", 6),
            ("less", 7),
            ("lessequal", 8),
        ] {
            let source =
                format!("test\n{{\n{{\nmap sample.tga\nalphatest {comparison} 0.3\n}}\n}}");
            let material = parse(&source).unwrap();
            assert_eq!(material["test"].stages[0].alpha_test, mode);
            assert_eq!(material["test"].stages[0].alpha_reference, 0.3);
        }
    }
}
