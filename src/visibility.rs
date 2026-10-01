//! Checked FAKK leaf/PVS data. Unknown or solid viewpoints deliberately fail open.
use crate::bsp::Plane;
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;

#[derive(Debug)]
struct Node {
    plane: usize,
    children: [i32; 2],
}
#[derive(Debug)]
struct Leaf {
    cluster: i32,
    min: Vec3,
    max: Vec3,
    faces: Vec<usize>,
}
#[derive(Debug, Default)]
pub struct Visibility {
    nodes: Vec<Node>,
    leaves: Vec<Leaf>,
    rows: Vec<u8>,
    stride: usize,
    clusters: usize,
    face_count: usize,
}
impl Visibility {
    pub fn parse(
        nodes: &[u8],
        leaves: &[u8],
        faces: &[u8],
        vis: &[u8],
        plane_count: usize,
        face_count: usize,
    ) -> Result<Self> {
        let int =
            |b: &[u8], i: usize| -> i32 { i32::from_le_bytes(b[i..i + 4].try_into().unwrap()) };
        ensure!(
            nodes.len() % 36 == 0 && leaves.len() % 48 == 0 && faces.len() % 4 == 0,
            "Invalid visibility record size"
        );
        if nodes.is_empty() || leaves.is_empty() || vis.is_empty() {
            return Ok(Self::default());
        }
        ensure!(vis.len() >= 8, "Truncated visibility header");
        let clusters = usize::try_from(int(vis, 0))?;
        let stride = usize::try_from(int(vis, 4))?;
        ensure!(
            clusters <= 1_000_000
                && stride >= clusters.div_ceil(8)
                && stride.checked_mul(clusters).context("PVS overflow")? == vis.len() - 8,
            "Invalid PVS dimensions"
        );
        let nodes = nodes
            .chunks_exact(36)
            .map(|b| {
                let plane = usize::try_from(int(b, 0))?;
                ensure!(plane < plane_count, "Invalid BSP node plane");
                Ok(Node {
                    plane,
                    children: [int(b, 4), int(b, 8)],
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let leaves = leaves
            .chunks_exact(48)
            .map(|b| {
                let cluster = int(b, 0);
                ensure!(
                    cluster >= -1 && cluster < clusters as i32,
                    "Invalid leaf cluster"
                );
                let min = vec3(int(b, 8) as f32, int(b, 12) as f32, int(b, 16) as f32);
                let max = vec3(int(b, 20) as f32, int(b, 24) as f32, int(b, 28) as f32);
                ensure!(min.cmple(max).all(), "Inverted leaf bounds");
                let first = usize::try_from(int(b, 32))?;
                let count = usize::try_from(int(b, 36))?;
                ensure!(
                    first
                        .checked_add(count)
                        .is_some_and(|v| v <= faces.len() / 4),
                    "Invalid leaf face span"
                );
                let faces = (first..first + count)
                    .map(|i| {
                        let f = usize::try_from(int(faces, i * 4))?;
                        ensure!(f < face_count, "Invalid leaf face");
                        Ok(f)
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(Leaf {
                    cluster,
                    min,
                    max,
                    faces,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        for n in &nodes {
            for &child in &n.children {
                ensure!(
                    if child >= 0 {
                        (child as usize) < nodes.len()
                    } else {
                        (-1_i64 - child as i64) < leaves.len() as i64
                    },
                    "Invalid BSP child"
                );
            }
        }
        Ok(Self {
            nodes,
            leaves,
            rows: vis[8..].to_vec(),
            stride,
            clusters,
            face_count,
        })
    }
    pub fn cluster(&self, eye: Vec3, planes: &[Plane]) -> Option<usize> {
        if self.nodes.is_empty() || !eye.is_finite() {
            return None;
        }
        let mut index = 0_i32;
        for _ in 0..self.nodes.len() + 1 {
            if index < 0 {
                let leaf = &self.leaves[(-1_i64 - index as i64) as usize];
                // Also catches noclip outside the compiled world bounds.
                return (eye.cmpge(leaf.min - Vec3::ONE).all()
                    && eye.cmple(leaf.max + Vec3::ONE).all()
                    && leaf.cluster >= 0)
                    .then_some(leaf.cluster as usize);
            }
            let n = &self.nodes[index as usize];
            let p = planes[n.plane];
            index = n.children[usize::from(eye.dot(p.normal) < p.distance)];
        }
        None
    }
    pub fn faces(&self, cluster: Option<usize>) -> Option<Vec<bool>> {
        let c = cluster.filter(|&c| c < self.clusters)?;
        let mut visible = vec![false; self.face_count];
        for leaf in &self.leaves {
            if leaf.cluster >= 0 && self.sees(c, leaf.cluster as usize) {
                for &f in &leaf.faces {
                    visible[f] = true;
                }
            }
        }
        Some(visible)
    }
    fn sees(&self, from: usize, to: usize) -> bool {
        from == to || self.rows[from * self.stride + to / 8] & (1 << (to % 8)) != 0
    }
    pub fn bounds(&self, cluster: Option<usize>, min: Vec3, max: Vec3) -> bool {
        let Some(c) = cluster.filter(|&c| c < self.clusters) else {
            return true;
        };
        let mut found = false;
        for leaf in &self.leaves {
            if leaf.cluster < 0 || !min.cmple(leaf.max).all() || !max.cmpge(leaf.min).all() {
                continue;
            }
            found = true;
            if self.sees(c, leaf.cluster as usize) {
                return true;
            }
        }
        !found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn two_rooms() -> Visibility {
        Visibility {
            nodes: vec![Node {
                plane: 0,
                children: [-1, -2],
            }],
            leaves: vec![
                Leaf {
                    cluster: 0,
                    min: vec3(0., -10., -10.),
                    max: Vec3::splat(10.),
                    faces: vec![0],
                },
                Leaf {
                    cluster: 1,
                    min: Vec3::splat(-10.),
                    max: vec3(0., 10., 10.),
                    faces: vec![1],
                },
            ],
            rows: vec![1, 2],
            stride: 1,
            clusters: 2,
            face_count: 2,
        }
    }
    #[test]
    fn rooms_solid_or_unknown_views_and_moving_bounds() {
        let v = two_rooms();
        let planes = [Plane {
            normal: Vec3::X,
            distance: 0.,
        }];
        let c = v.cluster(Vec3::X, &planes);
        assert_eq!(c, Some(0));
        assert_eq!(v.faces(c), Some(vec![true, false]));
        assert!(!v.bounds(c, vec3(-9., -1., -1.), vec3(-2., 1., 1.)));
        assert!(v.bounds(c, vec3(-2., -1., -1.), vec3(2., 1., 1.)));
        assert_eq!(v.cluster(Vec3::X * 50., &planes), None);
        assert!(v.bounds(None, Vec3::splat(-9.), Vec3::splat(-2.)));
        let mut cycle = two_rooms();
        cycle.nodes[0].children = [0, 0];
        assert_eq!(cycle.cluster(Vec3::X, &planes), None);
        let mut solid = two_rooms();
        solid.leaves[0].cluster = -1;
        assert_eq!(solid.cluster(Vec3::X, &planes), None);
    }
    #[test]
    fn pvs_rejects_bad_ranges_and_fails_open_without_data() {
        assert!(Visibility::parse(&[0; 35], &[], &[], &[], 0, 0).is_err());
        let v = Visibility::default();
        assert!(v.faces(None).is_none());
        assert!(v.bounds(None, Vec3::ZERO, Vec3::ONE));
    }
}
