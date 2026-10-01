//! Observed TAN version 2 vertex animation format; bounded, read-only decoding.
use crate::skeletal::Bytes;
use anyhow::{ensure, Result};
use macroquad::prelude::*;
use std::collections::BTreeMap;

pub struct Surface {
    pub detail: Vec<crate::model_detail::Level>,
    pub name: String,
    pub uv: Vec<Vec2>,
    pub indices: Vec<u16>,
    pub frames: Vec<Vec<Vec3>>,
}
pub struct Model {
    pub surfaces: Vec<Surface>,
    pub tags: BTreeMap<String, Vec<Vec3>>,
    pub tag_rotations: BTreeMap<String, Vec<Quat>>,
    pub frame_time: f32,
}
impl Model {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let b = Bytes(data);
        ensure!(
            b.span(0, 4)? == b"TAN " && b.int(4)? == 2,
            "Expected TAN version 2"
        );
        b.span(0, 176)?;
        let nf = b.count(72, 10000)?;
        let nt = b.count(76, 16)?;
        let ns = b.count(80, 256)?;
        // Authored emitter rigs (fx_aliceice) contain animated tags but no
        // render surfaces. An entirely empty file is still invalid.
        ensure!(nf > 0 && (ns > 0 || nt > 0), "Empty TAN model");
        let duration = b.float(84)?;
        let frame_time = duration / nf as f32;
        ensure!((0.001..=1.).contains(&frame_time), "Invalid TAN timing");
        let of = b.count(100, data.len())?;
        let mut os = b.count(104, data.len())?;
        ensure!(b.count(172, data.len())? == data.len(), "TAN end mismatch");
        ensure!(of >= 176 && of + nf * 68 <= os, "Invalid TAN frame range");
        let mut frames = Vec::new();
        for f in 0..nf {
            let p = of + f * 68;
            // With no geometry the exporter writes reversed sentinel bounds
            // and an unused negative vertex scale. Tags have separate data.
            let (min, max, scale, origin) = if ns == 0 {
                (Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO)
            } else {
                (
                    b.vec3(p)?,
                    b.vec3(p + 12)?,
                    b.vec3(p + 24)?,
                    b.vec3(p + 36)?,
                )
            };
            ensure!(
                min.cmple(max).all() && scale.cmpge(Vec3::ZERO).all(),
                "Invalid TAN frame bounds/scale"
            );
            // Some stationary source files have NaN delta-motion fields at +48; unused.
            ensure!(
                (b.float(p + 64)? - frame_time).abs() < 0.001,
                "TAN interval mismatch"
            );
            frames.push((min, max, scale, origin));
        }
        let mut tags = BTreeMap::new();
        let mut tag_rotations = BTreeMap::new();
        let mut tag_end = of + nf * 68;
        for i in 0..nt {
            let p = b.count(108 + i * 4, data.len())?;
            ensure!(
                p >= tag_end && p + 64 + nf * 48 <= os,
                "Invalid TAN tag range"
            );
            tag_end = p + 64 + nf * 48;
            let name = b.name(p, 64)?.to_ascii_lowercase();
            let mut points = Vec::new();
            let mut rotations = Vec::new();
            for f in 0..nf {
                let p = p + 64 + f * 48;
                points.push(b.vec3(p)?);
                let axes = Mat3::from_cols(b.vec3(p + 12)?, b.vec3(p + 24)?, b.vec3(p + 36)?);
                rotations.push(Quat::from_mat3(&axes).normalize());
            }
            tag_rotations.insert(name.clone(), rotations);
            ensure!(tags.insert(name, points).is_none(), "Duplicate TAN tag");
        }
        let mut surfaces = Vec::new();
        for _ in 0..ns {
            let p = os;
            b.span(p, 104)?;
            ensure!(
                b.span(p, 4)? == b"TAN " && b.count(p + 68, 10000)? == nf,
                "TAN surface mismatch"
            );
            let name = b.name(p + 4, 64)?.to_ascii_lowercase();
            let nv = b.count(p + 72, 30000)?;
            let triangles = b.count(p + 80, 60000)?;
            let ot = b.count(p + 84, data.len())?;
            let oc = b.count(p + 88, data.len())?;
            let uv = b.count(p + 92, data.len())?;
            let ov = b.count(p + 96, data.len())?;
            let end = b.count(p + 100, data.len())?;
            ensure!(
                nv > 0
                    && triangles > 0
                    && ot >= 104
                    && ot + triangles * 12 <= oc
                    && oc + nv * 4 <= uv
                    && uv + nv * 8 <= ov
                    && ov + nv * nf * 8 <= end,
                "Invalid TAN surface ranges"
            );
            b.span(p, end)?;
            let indices = (0..triangles * 3)
                .map(|i| b.count(p + ot + i * 4, nv - 1).map(|v| v as u16))
                .collect::<Result<Vec<_>>>()?;
            let uv = (0..nv)
                .map(|i| Ok(vec2(b.float(p + uv + i * 8)?, b.float(p + uv + i * 8 + 4)?)))
                .collect::<Result<Vec<_>>>()?;
            let mut positions = Vec::new();
            for (f, &(min, max, scale, origin)) in frames.iter().enumerate() {
                let mut vertices = Vec::new();
                for i in 0..nv {
                    let packed = b.span(p + ov + (f * nv + i) * 8, 8)?;
                    let xyz = |j| u16::from_le_bytes([packed[j], packed[j + 1]]) as f32;
                    let v = vec3(xyz(0), xyz(2), xyz(4)) * scale + origin;
                    ensure!(
                        v.is_finite()
                            && v.cmpge(min - Vec3::ONE).all()
                            && v.cmple(max + Vec3::ONE).all(),
                        "TAN vertex outside bounds"
                    );
                    vertices.push(v);
                }
                positions.push(vertices);
            }
            surfaces.push(Surface {
                detail: crate::model_detail::levels(
                    &indices,
                    &positions,
                    &(0..nv)
                        .map(|i| b.count(p + oc + i * 4, nv - 1).map(|v| v as u16))
                        .collect::<Result<Vec<_>>>()?,
                    b.count(p + 76, nv)?,
                ),
                name,
                uv,
                indices,
                frames: positions,
            });
            os += end;
        }
        ensure!(os == data.len(), "TAN surface end mismatch");
        Ok(Self {
            surfaces,
            tags,
            tag_rotations,
            frame_time,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut b = vec![0u8; 432];
        b[..4].copy_from_slice(b"TAN ");
        for (p, v) in [
            (4, 2i32),
            (72, 1),
            (80, 1),
            (100, 176),
            (104, 244),
            (172, 432),
            (244 + 68, 1),
            (244 + 72, 3),
            (244 + 80, 1),
            (244 + 84, 104),
            (244 + 88, 116),
            (244 + 92, 128),
            (244 + 96, 152),
            (244 + 100, 188),
            (352, 1),
            (356, 2),
        ] {
            b[p..p + 4].copy_from_slice(&v.to_le_bytes());
        }
        b[244..248].copy_from_slice(b"TAN ");
        for p in [84, 240] {
            b[p..p + 4].copy_from_slice(&0.05f32.to_le_bytes());
        }
        for p in [188, 192, 196] {
            b[p..p + 4].copy_from_slice(&1f32.to_le_bytes());
        }
        for p in [200, 204, 208] {
            b[p..p + 4].copy_from_slice(&(1f32 / 65535.).to_le_bytes());
        }
        b[244 + 152 + 8..244 + 152 + 10].copy_from_slice(&65535u16.to_le_bytes());
        b
    }
    #[test]
    fn tag_only_emitter_rig_is_valid_but_an_empty_model_is_not() {
        let mut b = fixture();
        b.resize(356, 0);
        b[244..].fill(0);
        for (p, v) in [(76, 1i32), (80, 0), (104, 356), (108, 244), (172, 356)] {
            b[p..p + 4].copy_from_slice(&v.to_le_bytes());
        }
        b[244..249].copy_from_slice(b"tag_1");
        for p in [320, 336, 352] {
            b[p..p + 4].copy_from_slice(&1f32.to_le_bytes());
        }
        let m = Model::parse(&b).unwrap();
        assert!(m.surfaces.is_empty());
        assert_eq!(m.tags["tag_1"], [Vec3::ZERO]);
        b[76..80].copy_from_slice(&0i32.to_le_bytes());
        assert!(Model::parse(&b).is_err());
    }
    #[test]
    fn unsigned_vertices_and_checked_tan_ranges() {
        let b = fixture();
        let m = Model::parse(&b).unwrap();
        assert_eq!(m.surfaces[0].frames[0][1], Vec3::X);
        for end in [0, 175, 243, 431] {
            assert!(Model::parse(&b[..end]).is_err());
        }
        for (p, v) in [
            (4, 3i32),
            (76, 17),
            (104, -1),
            (244 + 72, 30001),
            (244 + 84, 0),
            (348, 99),
        ] {
            let mut bad = b.clone();
            bad[p..p + 4].copy_from_slice(&v.to_le_bytes());
            assert!(Model::parse(&bad).is_err(), "offset {p}");
        }
    }
}
