//! Cached local geometry for opt-in moving brush and patch collision.
use super::*;
#[derive(Debug)]
struct Convex {
    source: Vec<Plane>,
    vertices: Vec<Vec3>,
    edges: Vec<Vec3>,
}
impl Convex {
    fn new(source: &[Plane]) -> Result<Self> {
        let mut vertices = Vec::<Vec3>::new();
        for (i, a) in source.iter().enumerate() {
            for (j, b) in source.iter().enumerate().skip(i + 1) {
                for c in &source[j + 1..] {
                    let determinant = a.normal.dot(b.normal.cross(c.normal));
                    if determinant.abs() < 0.00001 {
                        continue;
                    }
                    let v = (a.distance * b.normal.cross(c.normal)
                        + b.distance * c.normal.cross(a.normal)
                        + c.distance * a.normal.cross(b.normal))
                        / determinant;
                    if v.is_finite()
                        && source.iter().all(|p| p.normal.dot(v) - p.distance <= 0.01)
                        && !vertices.iter().any(|p| p.distance_squared(v) < 0.0001)
                    {
                        vertices.push(v);
                    }
                }
            }
        }
        ensure!(
            vertices.len() >= 4,
            "Inline collision brush has no closed volume"
        );
        let mut edges = Vec::<Vec3>::new();
        for (i, a) in vertices.iter().enumerate() {
            for b in &vertices[i + 1..] {
                // Two shared supporting planes identify a hull edge. Do not add
                // diagonals joining vertices across a face or the brush interior.
                if source
                    .iter()
                    .filter(|p| {
                        (p.normal.dot(*a) - p.distance).abs() <= 0.01
                            && (p.normal.dot(*b) - p.distance).abs() <= 0.01
                    })
                    .count()
                    >= 2
                {
                    let edge = (*b - *a).normalize();
                    if !edges
                        .iter()
                        .any(|e| e.distance_squared(edge).min(e.distance_squared(-edge)) < 1e-10)
                    {
                        edges.push(edge);
                    }
                }
            }
        }

        Ok(Self {
            source: source.to_vec(),
            vertices,
            edges,
        })
    }
    fn at(&self, origin: Vec3, rotation: Quat) -> Hull {
        let points = self
            .vertices
            .iter()
            .map(|v| origin + rotation * *v)
            .collect::<Vec<_>>();
        let mut planes = self
            .source
            .iter()
            .map(|p| {
                let normal = rotation * p.normal;
                Plane {
                    normal,
                    distance: p.distance + normal.dot(origin),
                }
            })
            .collect::<Vec<_>>();
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            add_support(&mut planes, &points, axis);
            add_support(&mut planes, &points, -axis);
        }
        for edge in &self.edges {
            for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                let n = (rotation * *edge).cross(axis);
                if n.length_squared() > 0.000001 {
                    add_support(&mut planes, &points, n.normalize());
                    add_support(&mut planes, &points, -n.normalize());
                }
            }
        }
        Hull {
            planes,
            bounds: Bounds {
                min: points.iter().copied().reduce(Vec3::min).unwrap(),
                max: points.iter().copied().reduce(Vec3::max).unwrap(),
            },
        }
    }
}
#[derive(Debug)]
struct Geometry {
    brushes: Vec<Convex>,
    patches: Vec<[Vec3; 3]>,
    bounds: Option<Bounds>,
}
#[derive(Debug)]
pub struct Template {
    geometry: std::sync::Arc<Geometry>,
}
#[derive(Debug)]
pub(super) struct Placed {
    geometry: std::sync::Arc<Geometry>,
    origin: Vec3,
    rotation: Quat,
    hulls: std::sync::OnceLock<Vec<Hull>>,
}
impl Placed {
    pub(super) fn hulls(&self) -> &[Hull] {
        self.hulls
            .get_or_init(|| self.geometry.at(self.origin, self.rotation))
    }
}
impl Geometry {
    fn at(&self, origin: Vec3, rotation: Quat) -> Vec<Hull> {
        let mut hulls = self
            .brushes
            .iter()
            .map(|s| s.at(origin, rotation))
            .collect::<Vec<_>>();
        hulls.extend(self.patches.iter().filter_map(|p| {
            triangle_hull(
                origin + rotation * p[0],
                origin + rotation * p[1],
                origin + rotation * p[2],
            )
        }));
        hulls
    }
}
impl Template {
    pub fn model(map: &Bsp, index: usize) -> Result<Self> {
        Self::load(map, index, true)
    }
    /// Cache the same solid brushes used by Collider::model, without adding patches.
    pub fn brush_model(map: &Bsp, index: usize) -> Result<Self> {
        Self::load(map, index, false)
    }
    fn load(map: &Bsp, index: usize, include_patches: bool) -> Result<Self> {
        let model = map
            .models
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("Invalid moving model"))?;
        let mut brushes = Vec::new();
        let mut patches = Vec::new();
        for i in model.brushes.clone() {
            let b = &map.brushes[i];
            if map.shaders[b.shader].contents & PLAYER_MASK == 0 {
                continue;
            }
            brushes.push(Convex::new(
                &map.side_planes[b.sides.clone()]
                    .iter()
                    .map(|p| map.planes[*p])
                    .collect::<Vec<_>>(),
            )?);
        }
        for i in model.surfaces.clone().filter(|_| include_patches) {
            let s = &map.surfaces[i];
            let shader = &map.shaders[s.shader];
            if s.kind != 2 || shader.contents & PLAYER_MASK == 0 || shader.flags & 0x4000 != 0 {
                continue;
            }
            let (v, indices) = map.triangulate(s);
            patches.extend(indices.chunks_exact(3).map(|i| {
                [
                    v[i[0] as usize].position,
                    v[i[1] as usize].position,
                    v[i[2] as usize].position,
                ]
            }));
        }
        Ok(Self::from_parts(brushes, patches))
    }
    fn from_parts(brushes: Vec<Convex>, patches: Vec<[Vec3; 3]>) -> Self {
        let points = brushes
            .iter()
            .flat_map(|b| b.vertices.iter().copied())
            .chain(patches.iter().flat_map(|p| p.iter().copied()))
            .collect::<Vec<_>>();
        let bounds = points
            .iter()
            .copied()
            .reduce(Vec3::min)
            .zip(points.iter().copied().reduce(Vec3::max))
            .map(|(min, max)| Bounds {
                min: min - Vec3::splat(0.125),
                max: max + Vec3::splat(0.125),
            });
        Self {
            geometry: std::sync::Arc::new(Geometry {
                brushes,
                patches,
                bounds,
            }),
        }
    }
    pub fn at(&self, origin: Vec3, rotation: Quat) -> Collider {
        // Conservative transformed bounds reject distant contacts without building
        // thousands of patch hulls. The exact hulls are cached on the first query.
        let bounds = self.geometry.bounds.map(|b| {
            let mut min = Vec3::splat(f32::INFINITY);
            let mut max = -min;
            for x in [b.min.x, b.max.x] {
                for y in [b.min.y, b.max.y] {
                    for z in [b.min.z, b.max.z] {
                        let p = origin + rotation * Vec3::new(x, y, z);
                        min = min.min(p);
                        max = max.max(p);
                    }
                }
            }
            Bounds { min, max }
        });
        Collider {
            hulls: std::sync::Arc::new(Vec::new()),
            bounds,
            lazy: Some(std::sync::Arc::new(Placed {
                geometry: self.geometry.clone(),
                origin,
                rotation,
                hulls: std::sync::OnceLock::new(),
            })),
        }
    }
    pub fn counts(&self) -> (usize, usize) {
        (self.geometry.brushes.len(), self.geometry.patches.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_convex_keeps_separating_planes_after_rotation() {
        let source = World::fixture(&[(vec3(-80., -12., -5.), vec3(80., 12., 5.))])
            .hulls
            .remove(0)
            .planes;
        let cached = Convex::new(&source).unwrap();
        for angle in [0., 0.31, 1.2, 2.5] {
            let rotation = Quat::from_rotation_z(angle) * Quat::from_rotation_y(0.2);
            let origin = vec3(400., -200., 50.);
            let a = Collider::from_hulls(vec![cached.at(origin, rotation)]);
            let b =
                Collider::from_hulls(vec![Hull::transformed(&source, origin, rotation).unwrap()]);
            for x in -12..=12 {
                for y in -12..=12 {
                    let start = origin + vec3(x as f32 * 10., y as f32 * 10., 100.);
                    let end = start - Vec3::Z * 200.;
                    let p = a.trace(start, end, PLAYER_HALF);
                    let q = b.trace(start, end, PLAYER_HALF);
                    assert_eq!(p.start_solid, q.start_solid);
                    assert!((p.fraction - q.fraction).abs() < 1e-5);
                }
            }
        }
    }
    #[test]
    fn patch_only_template_has_rotated_support_and_empty_outside() {
        let t = Template::from_parts(
            vec![],
            vec![[vec3(-60., -60., 0.), vec3(60., -60., 0.), vec3(0., 60., 0.)]],
        );
        let c = t.at(vec3(20., 30., 40.), Quat::from_rotation_y(0.3));
        assert!(c.lazy.as_ref().unwrap().hulls.get().is_none());
        assert!(!c.touches(vec3(300., 30., 40.), vec3(300., 30., 40.), PLAYER_HALF));
        assert!(c.lazy.as_ref().unwrap().hulls.get().is_none());
        let hit = c.trace(vec3(20., 30., 140.), vec3(20., 30., -60.), Vec3::ONE);
        assert!(hit.fraction < 1. && hit.normal.z > 0.9);
        assert!(!c.touches(vec3(300., 30., 40.), vec3(300., 30., 40.), PLAYER_HALF));
    }
    #[test]
    fn lazy_shapes_preserve_exact_sweeps_membership_and_clone_cache() {
        let source = World::fixture(&[(vec3(-70., -15., -8.), vec3(70., 15., 8.))])
            .hulls
            .remove(0)
            .planes;
        let t = Template::from_parts(
            vec![Convex::new(&source).unwrap()],
            vec![[
                vec3(-60., -80., 20.),
                vec3(60., -80., 20.),
                vec3(0., -20., 20.),
            ]],
        );
        for angle in [0., 0.3, 1.2, 2.5] {
            let rotation = Quat::from_rotation_z(angle) * Quat::from_rotation_y(0.2);
            let origin = vec3(400., -200., 50.);
            let lazy = t.at(origin, rotation);
            let clone = lazy.clone();
            let eager = Collider::from_hulls(t.geometry.at(origin, rotation));
            assert!(clone.lazy.as_ref().unwrap().hulls.get().is_none());
            for x in -12..=12 {
                for y in -12..=12 {
                    let start = origin + vec3(x as f32 * 10., y as f32 * 10., 100.);
                    let end = start - Vec3::Z * 200.;
                    let a = lazy.trace(start, end, PLAYER_HALF);
                    let b = eager.trace(start, end, PLAYER_HALF);
                    assert_eq!(a.start_solid, b.start_solid);
                    assert_eq!(a.fraction, b.fraction);
                    assert_eq!(
                        lazy.contains(start - Vec3::Z * 100.),
                        eager.contains(start - Vec3::Z * 100.)
                    );
                }
            }
            assert!(clone.lazy.as_ref().unwrap().hulls.get().is_some());
            assert_eq!(lazy.interior_point(), eager.interior_point());
        }
    }
}
