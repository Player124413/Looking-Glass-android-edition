//! Checked readers for the observed SKL/SKAN version 2 and 3 files. See docs/CHARACTER.md.
use crate::{assets::Assets, bsp::tokens};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Bytes<'a>(pub(crate) &'a [u8]);
impl Bytes<'_> {
    pub(crate) fn span(&self, start: usize, size: usize) -> Result<&[u8]> {
        self.0
            .get(start..start.checked_add(size).context("Model range overflow")?)
            .context("Truncated model data")
    }
    pub(crate) fn int(&self, p: usize) -> Result<i32> {
        Ok(i32::from_le_bytes(self.span(p, 4)?.try_into()?))
    }
    pub(crate) fn count(&self, p: usize, max: usize) -> Result<usize> {
        let v = self.int(p)?;
        ensure!(
            v >= 0 && v as usize <= max,
            "Invalid model count/offset at {p}: {v}"
        );
        Ok(v as usize)
    }
    pub(crate) fn float(&self, p: usize) -> Result<f32> {
        let v = f32::from_le_bytes(self.span(p, 4)?.try_into()?);
        ensure!(v.is_finite(), "Non-finite model value at {p}");
        Ok(v)
    }
    pub(crate) fn vec3(&self, p: usize) -> Result<Vec3> {
        Ok(vec3(self.float(p)?, self.float(p + 4)?, self.float(p + 8)?))
    }
    pub(crate) fn name(&self, p: usize, len: usize) -> Result<String> {
        let bytes = self.span(p, len)?;
        let end = bytes
            .iter()
            .position(|&c| c == 0)
            .context("Unterminated model name")?;
        Ok(std::str::from_utf8(&bytes[..end])?.to_owned())
    }
}

pub struct Bone {
    pub parent: Option<usize>,
    pub name: String,
}
pub struct Weight {
    pub bone: usize,
    pub amount: f32,
    pub offset: Vec3,
}
pub struct SkinVertex {
    pub uv: Vec2,
    pub weights: Vec<Weight>,
}
pub struct Surface {
    pub name: String,
    pub vertices: Vec<SkinVertex>,
    pub indices: Vec<u16>,
}
pub struct Skeleton {
    pub bones: Vec<Bone>,
    pub surfaces: Vec<Surface>,
}

impl Skeleton {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let b = Bytes(data);
        ensure!(
            b.span(0, 4)? == b"SKL " && matches!(b.int(4)?, 2 | 3),
            "Expected SKL version 2 or 3"
        );
        b.span(0, 92)?;
        let ns = b.count(72, 256)?;
        let nb = b.count(76, 200)?;
        ensure!(ns > 0 && nb > 0, "Empty skeleton");
        let ob = b.count(80, data.len())?;
        let os = b.count(84, data.len())?;
        ensure!(
            b.count(88, data.len())? == data.len(),
            "SKL end offset mismatch"
        );
        ensure!(
            ob >= 92 && ob + nb * 72 <= os,
            "Overlapping skeleton ranges"
        );
        b.span(ob, nb * 72)?;
        let mut bones = Vec::with_capacity(nb);
        for i in 0..nb {
            let parent = b.int(ob + i * 72)?;
            ensure!(
                parent >= -1 && parent < i as i32,
                "Invalid/cyclic bone parent {parent} for {i}"
            );
            bones.push(Bone {
                parent: (parent >= 0).then_some(parent as usize),
                name: b.name(ob + i * 72 + 8, 64)?,
            });
        }
        let mut surfaces = Vec::with_capacity(ns);
        let mut p = os;
        for _ in 0..ns {
            b.span(p, 96)?;
            ensure!(b.span(p, 4)? == b"SKL ", "Invalid surface magic");
            let name = b.name(p + 4, 64)?.to_ascii_lowercase();
            let nt = b.count(p + 68, 60000)?;
            let nv = b.count(p + 72, 30000)?;
            let ot = b.count(p + 80, data.len())?;
            let ov = b.count(p + 84, data.len())?;
            let oc = b.count(p + 88, data.len())?;
            let end = b.count(p + 92, data.len())?;
            ensure!(
                nv > 0
                    && nt > 0
                    && ot >= 96
                    && ot + nt * 12 <= ov
                    && ov <= oc
                    && oc + nv * 4 <= end,
                "Invalid surface ranges for {name}"
            );
            b.span(p, end)?;
            let mut indices = Vec::with_capacity(nt * 3);
            for j in 0..nt * 3 {
                indices.push(b.count(p + ot + j * 4, nv - 1)? as u16);
            }
            let mut vertices = Vec::with_capacity(nv);
            let mut v = p + ov;
            for _ in 0..nv {
                b.vec3(v)?; // Stored normals are not needed: animated normals are rebuilt.
                let uv = vec2(b.float(v + 12)?, b.float(v + 16)?);
                let nw = b.count(v + 20, nb)?;
                ensure!(
                    nw > 0 && v + 24 + nw * 20 <= p + oc,
                    "Invalid vertex weights"
                );
                v += 24;
                let mut weights = Vec::with_capacity(nw);
                let mut total = 0.;
                for _ in 0..nw {
                    let bone = b.count(v, nb - 1)?;
                    let amount = b.float(v + 4)?;
                    let offset = b.vec3(v + 8)?;
                    ensure!(
                        (0.0..=1.001).contains(&amount) && offset.abs().max_element() < 10000.,
                        "Invalid skin weight"
                    );
                    total += amount;
                    weights.push(Weight {
                        bone,
                        amount,
                        offset,
                    });
                    v += 20;
                }
                ensure!(
                    (total - 1.).abs() < 0.01,
                    "Skin weights do not sum to one: {total}"
                );
                for w in &mut weights {
                    w.amount /= total;
                }
                vertices.push(SkinVertex { uv, weights });
            }
            ensure!(v == p + oc, "Unexpected vertex payload size");
            surfaces.push(Surface {
                name,
                vertices,
                indices,
            });
            p += end;
        }
        ensure!(p == data.len(), "Unexpected trailing skeleton data");
        Ok(Self { bones, surfaces })
    }
    /// Flags bones that are unskinned attachment tags: named `tag_*` and carrying
    /// no skin weight. Only these may store a non-unit packed rotation
    /// (see `Animation::parse_tags`); every skinned bone stays strictly checked.
    pub fn unskinned_tags(&self) -> Vec<bool> {
        let mut skinned = vec![false; self.bones.len()];
        for w in self
            .surfaces
            .iter()
            .flat_map(|s| &s.vertices)
            .flat_map(|v| &v.weights)
        {
            skinned[w.bone] |= w.amount > 0.;
        }
        self.bones
            .iter()
            .zip(skinned)
            .map(|(b, skinned)| !skinned && b.name.to_ascii_lowercase().starts_with("tag_"))
            .collect()
    }
    pub fn global_pose(&self, local: &[Transform]) -> Vec<Transform> {
        let mut world: Vec<Transform> = Vec::with_capacity(self.bones.len());
        for (bone, &pose) in self.bones.iter().zip(local) {
            world.push(match bone.parent {
                Some(p) => Transform {
                    rotation: (world[p].rotation * pose.rotation).normalize(),
                    translation: world[p].point(pose.translation),
                },
                None => pose,
            });
        }
        world
    }
}
impl SkinVertex {
    pub fn position(&self, pose: &[Transform]) -> Vec3 {
        self.weights
            .iter()
            .map(|w| pose[w.bone].point(w.offset) * w.amount)
            .sum()
    }
}
/// Uniform scale of selected bone subtrees, retaining the root pivot and propagating
/// the changed spacing to child attachments. Skin weights use the returned scales.
pub(crate) fn scale_pose(skeleton: &Skeleton, pose: &mut [Transform], roots: &[(usize, f32)]) -> Vec<f32> {
    let original = pose.to_vec();
    let mut scales = vec![1.; pose.len()];
    for (i, bone) in skeleton.bones.iter().enumerate() {
        let inherited = bone.parent.map_or(1., |p| scales[p]);
        scales[i] = inherited * roots.iter().find(|(n, _)| *n == i).map_or(1., |(_, s)| *s);
        if let Some(parent) = bone.parent {
            pose[i].translation = pose[parent].translation + (original[i].translation - original[parent].translation) * inherited;
        }
    }
    scales
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Transform {
    pub rotation: Quat,
    pub translation: Vec3,
}
impl Transform {
    pub fn point(self, p: Vec3) -> Vec3 {
        self.rotation * p + self.translation
    }
    pub fn blend(self, other: Self, t: f32) -> Self {
        Self {
            rotation: self.rotation.slerp(other.rotation, t).normalize(),
            translation: self.translation.lerp(other.translation, t),
        }
    }
}
pub struct Frame {
    pub pose: Vec<Transform>,
    pub delta: Vec3,
    pub min: Vec3,
    pub max: Vec3,
}
pub struct Animation {
    pub frames: Vec<Frame>,
    pub frame_time: f32,
    pub distance: f32,
}
/// Accepted squared length of a packed rotation on an unskinned attachment tag
/// bone. Every skinned bone must be unit length within 0.02. The observed
/// exception is the Clockwork Automaton's two hand tags, stored at squared
/// lengths of about 0.60 to 0.75. The band is wide enough for that authoring
/// and still far below what a wrong stride, order or convention would produce.
const TAG_NORM_BAND: std::ops::RangeInclusive<f32> = 0.5..=1.5;

impl Animation {
    /// Strict reader: every bone's packed rotation must be unit length.
    pub fn parse(data: &[u8], bones: usize) -> Result<Self> {
        Ok(Self::parse_tags(data, &vec![false; bones])?.0)
    }
    /// Like `parse`, but `tags[i]` marks bone `i` as an unskinned attachment tag
    /// (see `Skeleton::unskinned_tags`). Such a bone may store a non-unit packed
    /// rotation inside `TAG_NORM_BAND`; it is renormalised and its index is
    /// returned so the caller can report or allow-list the model explicitly.
    pub fn parse_tags(data: &[u8], tags: &[bool]) -> Result<(Self, BTreeSet<usize>)> {
        let bones = tags.len();
        let mut renormalised = BTreeSet::new();
        let b = Bytes(data);
        let version = b.int(4)?;
        ensure!(
            b.span(0, 4)? == b"SKAN" && matches!(version, 2 | 3),
            "Expected SKAN version 2 or 3"
        );
        // Queen second form uses v2: no flags word before the frame count,
        // and seven float components per bone instead of eight packed shorts.
        // Its obsolete export-size words are not trusted; bound the real payload.
        let shift = if version == 2 { 4 } else { 0 };
        let bone_stride = if version == 2 { 28 } else { 16 };
        let nf = b.count(76 - shift, 10000)?;
        ensure!(
            nf > 0 && b.count(80 - shift, 200)? == bones && bones > 0,
            "Animation skeleton/frame mismatch"
        );
        let frame_time = b.float(88 - shift)?;
        ensure!(
            (0.001..=1.).contains(&frame_time),
            "Invalid animation frame time"
        );
        let duration = b.float(84 - shift)?;
        ensure!(
            (duration - nf as f32 * frame_time).abs() < 0.02,
            "Animation duration mismatch"
        );
        let distance = b.vec3(92 - shift)?.truncate().length();
        let start = b.count(104, data.len())?;
        let stride = 40 + bones * bone_stride;
        ensure!(
            start >= if version == 2 { 112 } else { 108 } && start + nf * stride == data.len(),
            "Animation payload size mismatch"
        );
        let mut frames = Vec::with_capacity(nf);
        for f in 0..nf {
            let p = start + f * stride;
            let min = b.vec3(p)?;
            let max = b.vec3(p + 12)?;
            ensure!(min.cmple(max).all(), "Invalid animation bounds");
            b.float(p + 24)?;
            let delta = b.vec3(p + 28)?;
            let mut pose = Vec::with_capacity(bones);
            for (i, &tag) in tags.iter().enumerate() {
                let offset = p + 40 + i * bone_stride;
                let (q, translation) = if version == 2 {
                    (
                        Quat::from_xyzw(
                            -b.float(offset)?,
                            -b.float(offset + 4)?,
                            -b.float(offset + 8)?,
                            b.float(offset + 12)?,
                        ),
                        b.vec3(offset + 16)?,
                    )
                } else {
                    let bytes = b.span(offset, 16)?;
                    let v = (0..8)
                        .map(|j| i16::from_le_bytes([bytes[j * 2], bytes[j * 2 + 1]]) as f32)
                        .collect::<Vec<_>>();
                    (
                        Quat::from_xyzw(-v[0], -v[1], -v[2], v[3]) / 32767.,
                        vec3(v[4], v[5], v[6]) / 64.,
                    )
                };
                // Both versions store the conjugate of glam's rotation.
                let unit = (q.length_squared() - 1.).abs() < 0.02;
                if !unit {
                    ensure!(
                        tag && TAG_NORM_BAND.contains(&q.length_squared()),
                        "Invalid packed bone quaternion"
                    );
                    renormalised.insert(i);
                }
                pose.push(Transform {
                    rotation: q.normalize(),
                    translation,
                });
            }
            frames.push(Frame {
                pose,
                delta,
                min,
                max,
            });
        }
        Ok((
            Self {
                frames,
                frame_time,
                distance,
            },
            renormalised,
        ))
    }
    pub fn duration(&self) -> f32 {
        self.frames.len() as f32 * self.frame_time
    }
    /// Match animation travel to collision-resolved displacement. Per-frame
    /// authored travel weights preserve the cadence within each stride.
    pub fn advance_distance(&self, time: f32, distance: f32, vertical: bool) -> f32 {
        if distance <= 0. || !distance.is_finite() {
            return time;
        }
        let weights: Vec<f32> = self
            .frames
            .iter()
            .map(|f| {
                if vertical {
                    f.delta.z.abs()
                } else {
                    f.delta.x.max(0.)
                }
            })
            .collect();
        let sum: f32 = weights.iter().sum();
        let travel = if vertical { sum } else { self.distance };
        if sum < 0.001 || travel < 0.001 {
            return time;
        }
        let frame = time.rem_euclid(self.duration()) / self.frame_time;
        let index = (frame.floor() as usize).min(weights.len() - 1);
        let before: f32 = weights[..index].iter().sum();
        let start = before + weights[index] * frame.fract();
        let end = start + distance * sum / travel;
        let cycles = (end / sum).floor();
        let mut within = end.rem_euclid(sum);
        let mut end_frame = 0.;
        for (i, weight) in weights.iter().enumerate() {
            if within < *weight {
                end_frame = i as f32 + within / weight;
                break;
            }
            within -= weight;
        }
        time + ((cycles * self.frames.len() as f32 + end_frame) - frame).max(0.) * self.frame_time
    }
    pub fn sample(&self, time: f32, looping: bool) -> Vec<Transform> {
        let frame = if looping {
            time.max(0.).rem_euclid(self.duration()) / self.frame_time
        } else {
            (time.max(0.) / self.frame_time).min((self.frames.len() - 1) as f32)
        };
        // Division can round a remainder just below the duration up to the
        // frame count (for example an 18-frame clip at 0.9 seconds).
        let a = frame.floor() as usize % self.frames.len();
        let b = if looping {
            (a + 1) % self.frames.len()
        } else {
            (a + 1).min(self.frames.len() - 1)
        };
        self.frames[a]
            .pose
            .iter()
            .zip(&self.frames[b].pose)
            .map(|(&x, &y)| x.blend(y, frame.fract()))
            .collect()
    }
}

pub struct Definition {
    pub path: String,
    pub model: String,
    pub scale: f32,
    pub skins: BTreeMap<String, String>,
    pub animations: BTreeMap<String, String>,
}
impl Definition {
    pub fn alice(assets: &mut Assets) -> Result<Self> {
        let def = Self::load(assets, "models/alice.tik")?;
        ensure!(!def.model.is_empty(), "Missing Alice skeleton");
        Ok(def)
    }
    pub fn load(assets: &mut Assets, path: &str) -> Result<Self> {
        let data = assets.read(path)?;
        let text = std::str::from_utf8(&data)?;
        let t = tokens(text)?;
        let start = t
            .iter()
            .position(|s| s.eq_ignore_ascii_case("setup"))
            .context("Missing Alice setup")?;
        ensure!(
            t.get(start + 1).is_some_and(|s| s == "{"),
            "Invalid Alice setup"
        );
        let mut result = Self {
            path: String::new(),
            model: String::new(),
            scale: 1.,
            skins: BTreeMap::new(),
            animations: BTreeMap::new(),
        };
        let mut i = start + 2;
        while i < t.len() && t[i] != "}" {
            let get = |n| t.get(i + n).context("Truncated Alice setup");
            match t[i].to_ascii_lowercase().as_str() {
                "path" => {
                    result.path = get(1)?.replace('\\', "/").trim_end_matches('/').to_owned();
                    i += 2;
                }
                "skelmodel" => {
                    result.model = get(1)?.clone();
                    i += 2;
                }
                "scale" => {
                    result.scale = get(1)?.parse()?;
                    i += 2;
                }
                "surface" if get(2)?.eq_ignore_ascii_case("shader") => {
                    result
                        .skins
                        .entry(get(1)?.to_ascii_lowercase())
                        .or_insert(get(3)?.clone());
                    i += 4;
                }
                _ => i += 1,
            }
        }
        ensure!(
            !result.path.is_empty()
                && result.scale.is_finite()
                && (0.01..=10.).contains(&result.scale),
            "Incomplete Alice definition"
        );
        if let Some(start) = t.iter().position(|s| s.eq_ignore_ascii_case("animations")) {
            let mut depth = 0;
            for i in start + 1..t.len() {
                match t[i].as_str() {
                    "{" => depth += 1,
                    "}" => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ if depth == 1 => {
                        if let Some(file) = t
                            .get(i + 1)
                            .filter(|s| s.ends_with(".tan") || s.ends_with(".ska"))
                        {
                            result
                                .animations
                                .entry(t[i].to_ascii_lowercase())
                                .or_insert(file.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(result)
    }
}

pub fn check(assets: &mut Assets) -> Result<()> {
    let def = Definition::alice(assets)?;
    let skeleton = Skeleton::parse(&assets.read(&format!("{}/{}", def.path, def.model))?)?;
    let names = assets
        .names()
        .filter(|n| n.starts_with("models/alice/") && n.ends_with(".ska"))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut total = 0;
    let mut worst_error = 0.0f32;
    for name in &names {
        let anim = Animation::parse(&assets.read(name)?, skeleton.bones.len())
            .with_context(|| name.clone())?;
        for frame in &anim.frames {
            let world = skeleton.global_pose(&frame.pose);
            for surface in &skeleton.surfaces {
                for vertex in &surface.vertices {
                    let p = vertex.position(&world);
                    ensure!(p.is_finite(), "Non-finite skinned position in {name}");
                    let error = (frame.min - p).max(p - frame.max).max_element().max(0.);
                    worst_error = worst_error.max(error);
                    ensure!(
                        error < 4.,
                        "Skin exceeds stored bounds by {error} in {name}"
                    );
                }
            }
        }
        println!("OK {name}: {} frames", anim.frames.len());
        total += anim.frames.len();
    }
    println!("PASS: {} bones, {} surfaces, {} clips, {total} frames; worst bounds error {worst_error:.3} units",skeleton.bones.len(),skeleton.surfaces.len(),names.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn put(b: &mut [u8], p: usize, n: i32) {
        b[p..p + 4].copy_from_slice(&n.to_le_bytes());
    }
    fn float(b: &mut [u8], p: usize, n: f32) {
        b[p..p + 4].copy_from_slice(&n.to_le_bytes());
    }
    fn mesh_fixture() -> Vec<u8> {
        let mut b = vec![0; 392];
        b[..4].copy_from_slice(b"SKL ");
        put(&mut b, 4, 3);
        for (p, n) in [(72, 1), (76, 2), (80, 92), (84, 236), (88, 392), (92, -1)] {
            put(&mut b, p, n);
        }
        b[100..105].copy_from_slice(b"root\0");
        b[172..178].copy_from_slice(b"child\0");
        b[236..240].copy_from_slice(b"SKL ");
        b[240..242].copy_from_slice(b"m\0");
        for (p, n) in [
            (68, 1),
            (72, 1),
            (80, 96),
            (84, 108),
            (88, 152),
            (92, 156),
            (128, 1),
            (132, 1),
        ] {
            put(&mut b, 236 + p, n);
        }
        float(&mut b, 236 + 136, 1.);
        b
    }
    fn anim_fixture() -> Vec<u8> {
        let mut b = vec![0; 252];
        b[..4].copy_from_slice(b"SKAN");
        put(&mut b, 4, 3);
        put(&mut b, 76, 2);
        put(&mut b, 80, 2);
        put(&mut b, 104, 108);
        float(&mut b, 84, 0.1);
        float(&mut b, 88, 0.05);
        for f in 0..2 {
            for i in 0..2 {
                let p = 108 + 72 * f + 40 + 16 * i;
                b[p + 6..p + 8].copy_from_slice(&32767i16.to_le_bytes());
                b[p + 8..p + 10].copy_from_slice(&((f * 128) as i16).to_le_bytes());
            }
        }
        b
    }
    #[test]
    fn version_two_float_poses_match_packed_poses_and_remain_checked() {
        let packed = Animation::parse(&anim_fixture(), 2).unwrap();
        let mut b = vec![0; 112 + 2 * (40 + 2 * 28)];
        b[..4].copy_from_slice(b"SKAN");
        put(&mut b, 4, 2);
        put(&mut b, 72, 2);
        put(&mut b, 76, 2);
        put(&mut b, 104, 112);
        float(&mut b, 80, 0.1);
        float(&mut b, 84, 0.05);
        for frame in 0..2 {
            for bone in 0..2 {
                let p = 112 + frame * 96 + 40 + bone * 28;
                float(&mut b, p + 12, 1.);
                float(&mut b, p + 16, frame as f32 * 2.);
            }
        }
        let parsed = Animation::parse(&b, 2).unwrap();
        for time in [0., 0.025, 0.05, 0.099] {
            for (a, b) in parsed
                .sample(time, false)
                .iter()
                .zip(packed.sample(time, false))
            {
                assert_eq!(a.translation, b.translation);
                assert_eq!(a.rotation, b.rotation);
            }
        }
        for n in 0..b.len() {
            assert!(Animation::parse(&b[..n], 2).is_err());
        }
        let mut bad = b.clone();
        float(&mut bad, 152, f32::NAN);
        assert!(Animation::parse(&bad, 2).is_err());
        let mut bad = b;
        float(&mut bad, 164, 0.);
        assert!(Animation::parse(&bad, 2).is_err());
        let mut mesh = mesh_fixture();
        put(&mut mesh, 4, 2);
        assert!(Skeleton::parse(&mesh).is_ok());
        put(&mut mesh, 236 + 132, 2);
        assert!(Skeleton::parse(&mesh).is_err());
    }
    #[test]
    fn checked_readers_reject_truncation_bad_ranges_indices_and_bone_cycles() {
        let mesh = mesh_fixture();
        assert!(Skeleton::parse(&mesh).is_ok());
        for n in 0..mesh.len() {
            assert!(Skeleton::parse(&mesh[..n]).is_err());
        }
        for (p, n) in [
            (4, 1),
            (80, -1),
            (164, 1),
            (236 + 96, 1),
            (236 + 132, 2),
            (236 + 92, 0),
        ] {
            let mut bad = mesh.clone();
            put(&mut bad, p, n);
            assert!(Skeleton::parse(&bad).is_err(), "offset {p}");
        }
        let anim = anim_fixture();
        assert!(Animation::parse(&anim, 2).is_ok());
        for n in 0..anim.len() {
            assert!(Animation::parse(&anim[..n], 2).is_err());
        }
        for (p, n) in [(4, 1), (76, 0), (80, 3), (104, -1), (88, 0)] {
            let mut bad = anim.clone();
            put(&mut bad, p, n);
            assert!(Animation::parse(&bad, 2).is_err());
        }
        let mut bad = mesh;
        float(&mut bad, 372, 0.25);
        assert!(Skeleton::parse(&bad).is_err());
    }
    #[test]
    fn parent_rotation_affects_child_position_and_weighted_skinning() {
        let skel = Skeleton::parse(&mesh_fixture()).unwrap();
        let pose = [
            Transform {
                rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                translation: vec3(10., 0., 0.),
            },
            Transform {
                rotation: Quat::IDENTITY,
                translation: vec3(0., 5., 0.),
            },
        ];
        let global = skel.global_pose(&pose);
        assert!(
            skel.surfaces[0].vertices[0]
                .position(&global)
                .distance(vec3(5., 0., 0.))
                < 1e-5
        );
    }
    fn set_rotation(b: &mut [u8], frame: usize, bone: usize, q: [i16; 4]) {
        let p = 108 + 72 * frame + 40 + 16 * bone;
        for (j, v) in q.into_iter().enumerate() {
            b[p + j * 2..p + j * 2 + 2].copy_from_slice(&v.to_le_bytes());
        }
    }
    #[test]
    fn off_norm_rotation_is_tolerated_only_on_an_unskinned_tag_inside_the_band() {
        let mut anim = anim_fixture();
        // Squared length about 0.60: a real hand-tag authoring, not a skinned bone.
        set_rotation(&mut anim, 1, 1, [0, 25456, 0, 0]);
        for tags in [[false, false], [true, false]] {
            let err = Animation::parse_tags(&anim, &tags).err().unwrap();
            assert!(format!("{err:#}").contains("Invalid packed bone quaternion"));
        }
        assert!(Animation::parse(&anim, 2).is_err());
        let (clip, repaired) = Animation::parse_tags(&anim, &[false, true]).unwrap();
        assert_eq!(repaired.into_iter().collect::<Vec<_>>(), [1]);
        let q = clip.frames[1].pose[1].rotation;
        assert!((q.length() - 1.).abs() < 1e-5 && (q.y + 1.).abs() < 1e-5);
        // Far outside the band even a tag is rejected (wrong stride or convention).
        set_rotation(&mut anim, 1, 1, [0, 8000, 0, 0]);
        assert!(Animation::parse_tags(&anim, &[false, true]).is_err());
        // Unit rotations never report a repair, tag or not.
        let (_, none) = Animation::parse_tags(&anim_fixture(), &[true, true]).unwrap();
        assert!(none.is_empty());
    }
    #[test]
    fn unskinned_tags_need_a_tag_name_and_no_skin_weight() {
        let bone = |name: &str, parent| Bone {
            parent,
            name: name.into(),
        };
        let weight = |bone, amount| Weight {
            bone,
            amount,
            offset: Vec3::ZERO,
        };
        let skeleton = Skeleton {
            bones: vec![
                bone("Bone01", None),
                bone("tag_hand", Some(0)),
                bone("TAG_Skinned", Some(0)),
                bone("tag_zero_weight", Some(0)),
                bone("hand", Some(0)),
            ],
            surfaces: vec![Surface {
                name: "m".into(),
                vertices: vec![SkinVertex {
                    uv: Vec2::ZERO,
                    weights: vec![weight(0, 0.5), weight(2, 0.5), weight(3, 0.)],
                }],
                indices: Vec::new(),
            }],
        };
        assert_eq!(skeleton.unskinned_tags(), [false, true, false, true, false]);
    }
    #[test]
    fn animation_interpolates_wraps_and_clamps_one_shots() {
        let anim = Animation::parse(&anim_fixture(), 2).unwrap();
        assert!((anim.sample(0.025, true)[0].translation.x - 1.).abs() < 1e-5);
        assert!((anim.sample(0.075, true)[0].translation.x - 1.).abs() < 1e-5);
        assert!(anim.sample(0.1, true)[0].translation.length() < 1e-5);
        assert!((anim.sample(100., false)[0].translation.x - 2.).abs() < 1e-5);
        let a = Transform {
            rotation: Quat::IDENTITY,
            translation: Vec3::ZERO,
        };
        let b = Transform {
            rotation: -Quat::IDENTITY,
            ..a
        };
        assert!(a.blend(b, 0.5).rotation.is_finite());
    }
    #[test]
    fn loop_rounding_at_the_last_frame_wraps_instead_of_indexing_past_the_clip() {
        let anim = Animation {
            frames: (0..18).map(|i| Frame {
                pose: vec![Transform { translation: Vec3::X * i as f32, rotation: Quat::IDENTITY }],
                delta: Vec3::ZERO, min: Vec3::ZERO, max: Vec3::ZERO,
            }).collect(),
            frame_time: 0.05,
            distance: 0.,
        };
        let edge = 0.9_f32;
        assert!(edge < anim.duration());
        assert_eq!(edge / anim.frame_time, 18.);
        assert_eq!(anim.sample(edge, true)[0].translation, Vec3::ZERO);
        assert_eq!(anim.sample(edge, false)[0].translation, Vec3::X * 17.);
    }
}
