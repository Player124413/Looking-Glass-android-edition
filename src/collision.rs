//! Independent continuous box sweeps against static convex brush and patch hulls.
//! No original executable or gameplay code is called by this module.
pub mod model_shape;
use crate::bsp::{Bsp, Plane};
use anyhow::{ensure, Result};
#[cfg(test)]
use macroquad::prelude::vec3;
use macroquad::prelude::{Quat, Vec3};

pub const SKIN: f32 = 0.03125;
pub const PLAYER_HALF: Vec3 = Vec3::new(15., 15., 28.);
pub const PLAYER_CENTER: Vec3 = Vec3::new(0., 0., 28.);
// Solid plus player-clip. Water, triggers, and monster-only clips are not solid.
const PLAYER_MASK: i32 = 1 | 0x10000;
pub const LIQUID_MASK: i32 = 0x08 | 0x10 | 0x20;

#[derive(Clone, Copy, Debug)]
struct Bounds {
    min: Vec3,
    max: Vec3,
}
impl Bounds {
    fn overlaps(self, other: Self) -> bool {
        self.min.cmple(other.max).all() && self.max.cmpge(other.min).all()
    }
    fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}
/// Conservative segment/expanded-box broad phase. A long diagonal sight line
/// must not test every brush in the entire rectangle between its endpoints.
struct SweepBounds {
    bounds: Bounds,
    start: Vec3,
    inverse: Vec3,
    padding: Vec3,
}
impl SweepBounds {
    fn new(start: Vec3, end: Vec3, half: Vec3) -> Self {
        let delta = end - start;
        let mut inverse = Vec3::ZERO;
        for axis in 0..3 {
            if delta[axis] != 0. { inverse[axis] = 1. / delta[axis]; }
        }
        let padding = half + Vec3::splat(SKIN);
        Self { bounds: Bounds { min: start.min(end) - padding, max: start.max(end) + padding }, start, inverse, padding }
    }
    fn overlaps(&self, bounds: Bounds) -> bool {
        if !bounds.overlaps(self.bounds) { return false; }
        let mut enter = 0_f32;
        let mut leave = 1_f32;
        for axis in 0..3 {
            // The enclosing box above already checks parallel/zero-length axes.
            if self.inverse[axis] == 0. { continue; }
            let a = (bounds.min[axis] - self.padding[axis] - self.start[axis]) * self.inverse[axis];
            let b = (bounds.max[axis] + self.padding[axis] - self.start[axis]) * self.inverse[axis];
            enter = enter.max(a.min(b));
            leave = leave.min(a.max(b));
            if enter > leave { return false; }
        }
        true
    }
}
#[derive(Clone, Debug)]
struct Hull {
    planes: Vec<Plane>,
    bounds: Bounds,
}
impl Hull {
    /// Exact convex-volume membership, the same test `World::liquid_at` applies to static liquids.
    fn contains(&self, point: Vec3) -> bool {
        point.cmpge(self.bounds.min).all()
            && point.cmple(self.bounds.max).all()
            && self
                .planes
                .iter()
                .all(|p| p.normal.dot(point) <= p.distance)
    }
}
impl Hull {
    /// Box sweeps also need world-axis and edge/axis separating planes. Rotating
    /// the BSP's precomputed bevels alone leaves phantom solid corners outside
    /// the model bounds, so a downward trace can report start-solid at clear feet.
    fn transformed(source: &[Plane], origin: Vec3, rotation: Quat) -> Result<Self> {
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
                    let edge = (rotation * (*b - *a)).normalize();
                    if !edges
                        .iter()
                        .any(|e| e.distance_squared(edge).min(e.distance_squared(-edge)) < 1e-10)
                    {
                        edges.push(edge);
                    }
                }
            }
        }
        let points = vertices
            .iter()
            .map(|v| origin + rotation * *v)
            .collect::<Vec<_>>();
        let mut planes = source
            .iter()
            .map(|p| {
                let normal = rotation * p.normal;
                Plane {
                    normal,
                    distance: p.distance + normal.dot(origin),
                }
            })
            .collect::<Vec<_>>();
        let axes = [Vec3::X, Vec3::Y, Vec3::Z];
        for axis in axes {
            add_support(&mut planes, &points, axis);
            add_support(&mut planes, &points, -axis);
        }
        for edge in edges {
            for axis in axes {
                let bevel = edge.cross(axis);
                if bevel.length_squared() > 0.000001 {
                    add_support(&mut planes, &points, bevel.normalize());
                    add_support(&mut planes, &points, -bevel.normalize());
                }
            }
        }
        Ok(Self {
            planes,
            bounds: Bounds {
                min: points.iter().copied().reduce(Vec3::min).unwrap(),
                max: points.iter().copied().reduce(Vec3::max).unwrap(),
            },
        })
    }
}
fn add_support(planes: &mut Vec<Plane>, points: &[Vec3], normal: Vec3) {
    // Near-parallel is insufficient here: even the first fraction of a degree
    // of shelf rotation needs its own world-axis bevel to agree with the bounds.
    if planes
        .iter()
        .any(|p| p.normal.distance_squared(normal) < 1e-10)
    {
        return;
    }
    let distance = points
        .iter()
        .map(|p| normal.dot(*p))
        .fold(f32::NEG_INFINITY, f32::max);
    planes.push(Plane { normal, distance });
}
/// A checked inline BSP model, transformed independently of the static BVH.
#[derive(Clone, Debug)]
pub struct Collider {
    hulls: std::sync::Arc<Vec<Hull>>,
    bounds: Option<Bounds>,
    lazy: Option<std::sync::Arc<model_shape::Placed>>,
}
/// A brush-entity or moving liquid volume, published by a level controller each frame.
/// `contents` carries the liquid bits (`LIQUID_MASK`); the volume may move between frames.
#[derive(Clone, Debug)]
pub struct Liquid {
    pub contents: i32,
    pub volume: Collider,
}
impl Collider {
    fn hulls(&self) -> &[Hull] {
        self.lazy.as_ref().map_or(self.hulls.as_slice(), |p|p.hulls())
    }
    /// A representative point in one convex brush, even when the union's
    /// bounding-box centre lies in a gap between disconnected brushes.
    pub(crate) fn interior_point(&self) -> Option<Vec3> {
        for hull in self.hulls().iter() {
            let center = (hull.bounds.min + hull.bounds.max) * 0.5;
            if hull.contains(center) {
                return Some(center);
            }
            let mut sum = Vec3::ZERO;
            let mut count = 0;
            for (i, a) in hull.planes.iter().enumerate() {
                for (j, b) in hull.planes.iter().enumerate().skip(i + 1) {
                    for c in &hull.planes[j + 1..] {
                        let det = a.normal.dot(b.normal.cross(c.normal));
                        if det.abs() < 0.00001 {
                            continue;
                        }
                        let v = (a.distance * b.normal.cross(c.normal)
                            + b.distance * c.normal.cross(a.normal)
                            + c.distance * a.normal.cross(b.normal))
                            / det;
                        if v.is_finite()
                            && hull
                                .planes
                                .iter()
                                .all(|p| p.normal.dot(v) <= p.distance + 0.01)
                        {
                            sum += v;
                            count += 1;
                        }
                    }
                }
            }
            if count > 0 && hull.contains(sum / count as f32) {
                return Some(sum / count as f32);
            }
        }
        None
    }
    /// Whether the point lies inside any of this collider's convex hulls.
    pub fn contains(&self, point: Vec3) -> bool {
        self.hulls().iter().any(|h| h.contains(point))
    }
    #[cfg(test)]
    pub fn fixture(min: Vec3, max: Vec3) -> Self {
        Self::box_bounds(min, max)
    }
    /// An axis-aligned entity hull (model rotation does not rotate its bounds).
    pub fn box_bounds(min: Vec3, max: Vec3) -> Self {
        Self::from_hulls(World::fixture(&[(min, max)]).hulls)
    }
    fn from_hulls(hulls: Vec<Hull>) -> Self {
        let bounds = hulls.iter().map(|h| h.bounds).reduce(Bounds::union);
        Self {
            hulls: std::sync::Arc::new(hulls),
            bounds,
            lazy: None,
        }
    }
    pub fn triangles(triangles: &[[Vec3; 3]], origin: Vec3, rotation: Quat) -> Self {
        Self::from_hulls(
            triangles
                .iter()
                .filter_map(|t| {
                    triangle_hull(
                        origin + rotation * t[0],
                        origin + rotation * t[1],
                        origin + rotation * t[2],
                    )
                })
                .collect(),
        )
    }
    pub fn model(
        map: &Bsp,
        index: usize,
        origin: Vec3,
        rotation: Quat,
        solids_only: bool,
    ) -> Result<Self> {
        let model = map
            .models
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("Invalid inline model {index}"))?;
        let mut hulls = Vec::new();
        for i in model.brushes.clone() {
            let brush = &map.brushes[i];
            if solids_only && map.shaders[brush.shader].contents & PLAYER_MASK == 0 {
                continue;
            }
            let planes = map.side_planes[brush.sides.clone()]
                .iter()
                .map(|&p| map.planes[p])
                .collect::<Vec<_>>();
            hulls.push(Hull::transformed(&planes, origin, rotation)?);
        }
        Ok(Self::from_hulls(hulls))
    }
    pub fn trace(&self, start: Vec3, end: Vec3, half: Vec3) -> Trace {
        let mut out = Trace::default();
        let query = SweepBounds::new(start, end, half);
        if self.bounds.is_some_and(|b| query.overlaps(b)) {
            for h in self.hulls().iter() {
                if query.overlaps(h.bounds) {
                    clip_hull(h, start, end, half, &mut out);
                }
            }
        }
        out
    }
    pub fn touches(&self, start: Vec3, end: Vec3, half: Vec3) -> bool {
        let t = self.trace(start, end, half);
        t.start_solid || t.fraction < 1.
    }
    /// Small contact correction for opt-in rotating pushers. Their passenger's
    /// axis-aligned box does not rotate with the model, so carrying its centre
    /// alone may leave a corner inside a neighbouring face. World obstruction
    /// must still be checked by the caller before committing this displacement.
    pub fn separate_body(&self, feet: Vec3, limit: f32) -> Option<Vec3> {
        let start=feet+PLAYER_CENTER;
        let mut point=start;
        for _ in 0..32 {
            let correction=self.hulls().iter().find_map(|h| {
                if point.cmplt(h.bounds.min-PLAYER_HALF).any() || point.cmpgt(h.bounds.max+PLAYER_HALF).any() {return None;}
                let mut nearest=(f32::INFINITY,Vec3::ZERO);
                for p in &h.planes {
                    let depth=p.distance+p.normal.abs().dot(PLAYER_HALF)-p.normal.dot(point);
                    if depth<0. {return None;}
                    if depth<nearest.0 {nearest=(depth,p.normal);}
                }
                nearest.0.is_finite().then_some(nearest.1*(nearest.0+SKIN))
            });
            let Some(delta)=correction else {
                return (!self.touches(point,point,PLAYER_HALF)).then_some(point-PLAYER_CENTER);
            };
            point+=delta;
            if point.distance(start)>limit {return None;}
        }
        None
    }
    /// A rider stays upright when a platform tilts. Rotating the feet alone can
    /// embed the bottom corner of the body in the newly tilted support.
    pub fn rider_feet(&self, carried: Vec3) -> Option<(Vec3, Vec3)> {
        let above = carried + Vec3::Z * 2.;
        let below = carried - Vec3::Z * 2.;
        let t = self.trace(above + PLAYER_CENTER, below + PLAYER_CENTER, PLAYER_HALF);
        let feet = above.lerp(below, t.fraction);
        (!t.start_solid
            && t.fraction < 1.
            && t.normal.z >= 0.65
            && !self.touches(feet + PLAYER_CENTER, feet + PLAYER_CENTER, PLAYER_HALF))
        .then_some((feet, t.normal))
    }
}
#[derive(Debug)]
enum Node {
    Leaf {
        bounds: Bounds,
        hulls: Vec<usize>,
    },
    Branch {
        bounds: Bounds,
        left: Box<Node>,
        right: Box<Node>,
    },
}
impl Node {
    fn build(hulls: &[Hull], indices: &mut [usize]) -> Self {
        let bounds = indices
            .iter()
            .map(|&i| hulls[i].bounds)
            .reduce(Bounds::union)
            .unwrap();
        if indices.len() <= 8 {
            return Self::Leaf {
                bounds,
                hulls: indices.to_vec(),
            };
        }
        let extent = bounds.max - bounds.min;
        let axis = if extent.x >= extent.y && extent.x >= extent.z {
            0
        } else if extent.y >= extent.z {
            1
        } else {
            2
        };
        indices.sort_unstable_by(|&a, &b| {
            (hulls[a].bounds.min[axis] + hulls[a].bounds.max[axis])
                .total_cmp(&(hulls[b].bounds.min[axis] + hulls[b].bounds.max[axis]))
        });
        let middle = indices.len() / 2;
        let (a, b) = indices.split_at_mut(middle);
        Self::Branch {
            bounds,
            left: Box::new(Self::build(hulls, a)),
            right: Box::new(Self::build(hulls, b)),
        }
    }
    fn visit(&self, query: &SweepBounds, callback: &mut impl FnMut(usize)) {
        match self {
            Self::Leaf { bounds, hulls } => {
                if query.overlaps(*bounds) {
                    for &h in hulls {
                        callback(h);
                    }
                }
            }
            Self::Branch {
                bounds,
                left,
                right,
            } => {
                if query.overlaps(*bounds) {
                    left.visit(query, callback);
                    right.visit(query, callback);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Trace {
    pub fraction: f32,
    pub normal: Vec3,
    pub start_solid: bool,
    pub all_solid: bool,
}
impl Default for Trace {
    fn default() -> Self {
        Self {
            fraction: 1.,
            normal: Vec3::ZERO,
            start_solid: false,
            all_solid: false,
        }
    }
}
#[derive(Debug)]
pub struct World {
    pub traversal: crate::traversal::Traversal,
    dynamic: Vec<Collider>,
    settled_supports: Vec<Collider>,
    weapon_obstacles: Vec<(usize, Collider)>,
    camera_obstacles: Vec<Collider>,
    liquids: Vec<(i32, Hull)>,
    dynamic_liquids: Vec<Liquid>,
    hulls: Vec<Hull>,
    tree: Option<Node>,
    pub brush_count: usize,
    pub patch_count: usize,
    pub min: Vec3,
    pub max: Vec3,
}

impl World {
    pub fn from_bsp(map: &Bsp) -> Result<Self> {
        Self::from_bsp_mask(map, PLAYER_MASK)
    }
    /// Scripted actors use solid and monster-clip, not Alice's player-only clips.
    pub fn actor_world(map: &Bsp) -> Result<Self> {
        Self::from_bsp_mask(map, 1 | 0x20000)
    }
    fn from_bsp_mask(map: &Bsp, mask: i32) -> Result<Self> {
        let mut hulls = Vec::new();
        let mut liquids = Vec::new();
        for index in map.world_brushes.clone() {
            let brush = &map.brushes[index];
            let contents = map.shaders[brush.shader].contents;
            if contents & (mask | LIQUID_MASK) == 0 {
                continue;
            }
            let planes = map.side_planes[brush.sides.clone()]
                .iter()
                .map(|&p| map.planes[p])
                .collect::<Vec<_>>();
            // The shipped brushes include six axial bounding planes. Require those rather than
            // guessing finite bounds and accidentally culling a collidable wall.
            let mut min = Vec3::splat(f32::NEG_INFINITY);
            let mut max = Vec3::splat(f32::INFINITY);
            for p in &planes {
                for axis in 0..3 {
                    let other = (axis + 1) % 3;
                    let last = (axis + 2) % 3;
                    if p.normal[other].abs() < 1e-6 && p.normal[last].abs() < 1e-6 {
                        if p.normal[axis] > 0.99999 {
                            max[axis] = max[axis].min(p.distance / p.normal[axis]);
                        }
                        if p.normal[axis] < -0.99999 {
                            min[axis] = min[axis].max(p.distance / p.normal[axis]);
                        }
                    }
                }
            }
            ensure!(
                min.is_finite() && max.is_finite() && min.cmple(max + Vec3::splat(0.01)).all(),
                "Brush {index} lacks valid axial bounds"
            );
            let hull = Hull {
                planes,
                bounds: Bounds { min, max },
            };
            if contents & LIQUID_MASK != 0 {
                liquids.push((contents & LIQUID_MASK, hull.clone()));
            }
            if contents & mask != 0 {
                hulls.push(hull);
            }
        }
        let brush_count = hulls.len();
        for index in map.world_surfaces.clone() {
            let surface = &map.surfaces[index];
            let shader = &map.shaders[surface.shader];
            if surface.kind != 2 || shader.contents & mask == 0 || shader.flags & 0x4000 != 0 {
                continue;
            }
            let (v, i) = map.triangulate(surface);
            for t in i.chunks_exact(3) {
                if let Some(hull) = triangle_hull(
                    v[t[0] as usize].position,
                    v[t[1] as usize].position,
                    v[t[2] as usize].position,
                ) {
                    hulls.push(hull);
                }
            }
        }
        let patch_count = hulls.len() - brush_count;
        let tree = if hulls.is_empty() {
            None
        } else {
            Some(Node::build(
                &hulls,
                &mut (0..hulls.len()).collect::<Vec<_>>(),
            ))
        };
        Ok(Self {
            traversal: crate::traversal::Traversal::load(map)?,
            dynamic: Vec::new(),
            settled_supports: Vec::new(),
            weapon_obstacles: Vec::new(),
            camera_obstacles: Vec::new(),
            liquids,
            dynamic_liquids: Vec::new(),
            hulls,
            tree,
            brush_count,
            patch_count,
            min: map.world_min,
            max: map.world_max,
        })
    }

    /// Trace a box centre from start to end. Zero extents give a point trace.
    pub fn sweep(&self, start: Vec3, end: Vec3, half: Vec3) -> Trace {
        self.sweep_except(start, end, half, None)
    }
    /// Damage traces may contact a destructible obstacle itself, but must still
    /// respect every other obstacle. Movement never supplies an exception.
    pub fn sweep_except(&self, start: Vec3, end: Vec3, half: Vec3, ignore: Option<usize>) -> Trace {
        let mut result = self.sweep_geometry(start, end, half);
        for (id, collider) in &self.weapon_obstacles {
            if Some(*id) == ignore {
                continue;
            }
            for h in collider.hulls().iter() {
                clip_hull(h, start, end, half, &mut result);
            }
        }
        result
    }
    /// Visible scenery can block the view without changing player or weapon collision.
    pub fn set_camera_obstacles(&mut self, obstacles: Vec<Collider>) {
        self.camera_obstacles = obstacles;
    }
    pub fn camera_sweep(&self, start: Vec3, end: Vec3, half: Vec3) -> Trace {
        let mut result = self.sweep(start, end, half);
        for collider in &self.camera_obstacles {
            let trace = collider.trace(start, end, half);
            result.start_solid |= trace.start_solid;
            result.all_solid |= trace.all_solid;
            if trace.fraction < result.fraction {
                result.fraction = trace.fraction;
                result.normal = trace.normal;
            }
        }
        result
    }
    pub fn set_weapon_obstacles(&mut self, obstacles: Vec<crate::combat::Target>) {
        self.weapon_obstacles = obstacles
            .into_iter()
            .map(|t| {
                (
                    t.id,
                    Collider::from_hulls(
                        Self::fixture(&[(t.center - t.half, t.center + t.half)]).hulls,
                    ),
                )
            })
            .collect();
    }
    pub fn sweep_geometry(&self, start: Vec3, end: Vec3, half: Vec3) -> Trace {
        let mut result = self.ledge_trace(start, end, half);
        for collider in &self.dynamic {
            let t = collider.trace(start, end, half);
            result.start_solid |= t.start_solid;
            result.all_solid |= t.all_solid;
            if t.fraction < result.fraction {
                result.fraction = t.fraction;
                result.normal = t.normal;
            }
        }
        result
    }
    /// Static support probes for hanging; moving/rotating entities are excluded.
    pub fn ledge_trace(&self, start: Vec3, end: Vec3, half: Vec3) -> Trace {
        let query = SweepBounds::new(start, end, half);
        let mut result = Trace::default();
        if let Some(tree) = &self.tree {
            tree.visit(&query, &mut |index| {
                let hull = &self.hulls[index];
                if query.overlaps(hull.bounds) {
                    clip_hull(hull, start, end, half, &mut result);
                }
            });
        }
        for c in &self.settled_supports {
            for hull in c.hulls().iter() { clip_hull(hull, start, end, half, &mut result); }
        }
        result
    }
    /// Keep movers, doors and live weapon obstacles when using an actor collision mask.
    pub fn copy_dynamic_from(&mut self, other: &Self) {
        self.dynamic.clone_from(&other.dynamic);
        self.traversal.actor_pushes.clone_from(&other.traversal.actor_pushes);
        self.settled_supports.clone_from(&other.settled_supports);
        self.weapon_obstacles.clone_from(&other.weapon_obstacles);
        self.dynamic_liquids.clone_from(&other.dynamic_liquids);
    }
    pub fn set_dynamic(&mut self, colliders: Vec<Collider>) {
        self.dynamic = colliders;
    }
    /// Geometry a controller has permanently settled may support hanging and pull-ups.
    /// Active movers are deliberately excluded.
    pub fn set_settled_supports(&mut self, colliders: Vec<Collider>) {
        self.settled_supports = colliders;
    }
    /// Replace the brush-entity and moving liquid volumes. They add to the map's own liquids.
    pub fn set_dynamic_liquids(&mut self, liquids: Vec<Liquid>) {
        self.dynamic_liquids = liquids;
    }
    /// Exact convex-volume membership, including sloped liquid faces. Liquids are not solids.
    pub fn liquid_at(&self, point: Vec3) -> i32 {
        let map_liquid = self.liquids.iter().fold(0, |mask, (contents, hull)| {
            if point.cmpge(hull.bounds.min).all()
                && point.cmple(hull.bounds.max).all()
                && hull
                    .planes
                    .iter()
                    .all(|p| p.normal.dot(point) <= p.distance)
            {
                mask | contents
            } else {
                mask
            }
        });
        self.dynamic_liquids
            .iter()
            .fold(map_liquid, |mask, liquid| {
                if liquid.volume.contains(point) {
                    mask | (liquid.contents & LIQUID_MASK)
                } else {
                    mask
                }
            })
    }
    pub fn liquid_count(&self) -> usize {
        self.liquids.len()
    }
    pub fn body_trace(&self, feet: Vec3, end: Vec3) -> Trace {
        self.sweep(feet + PLAYER_CENTER, end + PLAYER_CENTER, PLAYER_HALF)
    }
    pub fn body_clear(&self, feet: Vec3) -> bool {
        !self.body_trace(feet, feet).start_solid
    }
    /// Resolve an editor actor origin onto the first walkable support below it.
    /// Check the whole declared body, including dynamic platforms. A small lift
    /// permits editor placements that overlap the floor; walls and ceilings are
    /// never crossed to search for another room's floor.
    pub fn actor_footing(&self, origin: Vec3, center: Vec3, half: Vec3, drop: f32) -> Option<Vec3> {
        if !origin.is_finite()
            || !center.is_finite()
            || !half.is_finite()
            || half.min_element() <= 0.
            || !drop.is_finite()
            || drop < 0.
        {
            return None;
        }
        for lift in [0., 1., 2., 4., 8.] {
            let start = origin + Vec3::Z * lift;
            if self.sweep(start + center, start + center, half).start_solid {
                continue;
            }
            let end = origin - Vec3::Z * drop;
            let trace = self.sweep(start + center, end + center, half);
            if !trace.start_solid && trace.fraction < 1. && trace.normal.z >= 0.65 {
                let landed = start.lerp(end, trace.fraction);
                if !self
                    .sweep(landed + center, landed + center, half)
                    .start_solid
                {
                    return Some(landed);
                }
            }
            // The first clear body determines the accessible space. Raising it
            // farther could jump over a thin obstruction or change floors.
            return None;
        }
        None
    }
    /// Correct only tiny existing overlaps. The entire escape must avoid any
    /// newly encountered solid, and its endpoint must fit the full player box.
    /// Deep obstructions are left for explicit recovery, never a large teleport.
    pub fn correct_footing(&self, feet: Vec3) -> Option<Vec3> {
        if self.body_clear(feet) {
            return None;
        }
        for distance in [0.125, 0.25, 0.5, 1.] {
            // Prefer lifting off a floor, then check equally small side/diagonal escapes.
            for z in [1., 0., -1.] {
                for x in [0., -1., 1.] {
                    for y in [0., -1., 1.] {
                        let direction = Vec3::new(x, y, z).normalize_or_zero();
                        if direction == Vec3::ZERO {
                            continue;
                        }
                        let next = feet + direction * distance;
                        let trace = self.body_trace(feet, next);
                        if !trace.all_solid && trace.fraction >= 1. && self.body_clear(next) {
                            return Some(next);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn fixture(boxes: &[(Vec3, Vec3)]) -> Self {
        let hulls = boxes
            .iter()
            .map(|&(min, max)| {
                let mut planes = Vec::new();
                for axis in 0..3 {
                    let mut normal = Vec3::ZERO;
                    normal[axis] = 1.;
                    planes.push(Plane {
                        normal,
                        distance: max[axis],
                    });
                    planes.push(Plane {
                        normal: -normal,
                        distance: -min[axis],
                    });
                }
                Hull {
                    planes,
                    bounds: Bounds { min, max },
                }
            })
            .collect::<Vec<_>>();
        let tree = if hulls.is_empty() {
            None
        } else {
            Some(Node::build(
                &hulls,
                &mut (0..hulls.len()).collect::<Vec<_>>(),
            ))
        };
        Self {
            dynamic: Vec::new(),
            settled_supports: Vec::new(),
            weapon_obstacles: Vec::new(),
            camera_obstacles: Vec::new(),
            liquids: Vec::new(),
            dynamic_liquids: Vec::new(),
            traversal: Default::default(),
            brush_count: hulls.len(),
            patch_count: 0,
            hulls,
            tree,
            min: Vec3::splat(-10000.),
            max: Vec3::splat(10000.),
        }
    }
    #[cfg(test)]
    pub fn add_liquid(&mut self, min: Vec3, max: Vec3, contents: i32, slope: Option<Plane>) {
        let mut hull = Self::fixture(&[(min, max)]).hulls.remove(0);
        if let Some(p) = slope {
            hull.planes.push(p);
        }
        self.liquids.push((contents, hull));
    }
    #[cfg(test)]
    pub fn ramp_fixture() -> Self {
        let mut world = Self::fixture(&[(vec3(-200., -300., -1000.), vec3(500., 300., 1000.))]);
        world.hulls[0].planes.push(Plane {
            normal: vec3(-0.5, 0., 1.).normalize(),
            distance: 0.,
        });
        world
    }
}

fn clip_hull(hull: &Hull, start: Vec3, end: Vec3, half: Vec3, result: &mut Trace) {
    let mut enter = f32::NEG_INFINITY;
    let mut leave = 1.0f32;
    let mut normal = Vec3::ZERO;
    let mut starts_out = false;
    let mut ends_out = false;
    for p in &hull.planes {
        let expanded = p.distance + p.normal.abs().dot(half);
        let a = p.normal.dot(start) - expanded;
        let b = p.normal.dot(end) - expanded;
        starts_out |= a >= 0.;
        ends_out |= b >= 0.;
        // A start exactly on a face is allowed to move along it or out of it.
        if a >= 0. && b >= a {
            return;
        }
        if a < 0. && b < 0. {
            continue;
        }
        if a > b {
            let fraction = (a - SKIN) / (a - b);
            if fraction > enter {
                enter = fraction;
                normal = p.normal;
            }
        } else if b > a {
            leave = leave.min((a + SKIN) / (a - b));
        }
        if enter > leave {
            return;
        }
    }
    if !starts_out {
        result.start_solid = true;
        if !ends_out {
            result.all_solid = true;
            result.fraction = 0.;
        }
        return;
    }
    if enter.is_finite() && enter < leave && enter < result.fraction {
        result.fraction = enter.max(0.);
        result.normal = normal;
    }
}

// A thin, two-sided triangular prism. Face and edge/axis bevels describe the
// Minkowski expansion for a box. Tessellation is an approximation of curved patches.
fn triangle_hull(a: Vec3, b: Vec3, c: Vec3) -> Option<Hull> {
    let cross = (b - a).cross(c - a);
    if cross.length_squared() < 1e-6 {
        return None;
    }
    let n = cross.normalize();
    let thickness = 0.125;
    let points = [
        a + n * thickness,
        b + n * thickness,
        c + n * thickness,
        a - n * thickness,
        b - n * thickness,
        c - n * thickness,
    ];
    let mut normals = vec![n, -n];
    let axes = [Vec3::X, Vec3::Y, Vec3::Z];
    for axis in axes {
        normals.push(axis);
        normals.push(-axis);
    }
    for edge in [b - a, c - b, a - c, n] {
        let edge = edge.normalize();
        let side = edge.cross(n);
        if side.length_squared() > 1e-6 {
            normals.push(side.normalize());
        }
        for axis in axes {
            let bevel = edge.cross(axis);
            if bevel.length_squared() > 1e-6 {
                normals.push(bevel.normalize());
                normals.push(-bevel.normalize());
            }
        }
    }
    let mut planes: Vec<Plane> = Vec::new();
    for normal in normals {
        if planes.iter().any(|p| p.normal.dot(normal) > 0.99999) {
            continue;
        }
        let distance = points
            .iter()
            .map(|p| normal.dot(*p))
            .fold(f32::NEG_INFINITY, f32::max);
        planes.push(Plane { normal, distance });
    }
    let min = points.iter().copied().reduce(Vec3::min)?;
    let max = points.iter().copied().reduce(Vec3::max)?;
    Some(Hull {
        planes,
        bounds: Bounds { min, max },
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn trigger_probe_is_inside_a_brush_in_a_disconnected_volume() {
        let world = super::World::fixture(&[
            (
                super::Vec3::new(-100., -10., -10.),
                super::Vec3::new(-60., 10., 10.),
            ),
            (
                super::Vec3::new(60., -10., -10.),
                super::Vec3::new(100., 10., 10.),
            ),
        ]);
        let collider = super::Collider::from_hulls(world.hulls);
        assert!(!collider.contains(super::Vec3::ZERO));
        assert!(collider.contains(collider.interior_point().unwrap()));
    }
    use super::*;
    #[test]
    fn only_settled_controller_geometry_can_support_a_ledge() {
        let mut w=World::fixture(&[]);
        let ledge=Collider::box_bounds(vec3(-64.,-64.,0.),vec3(64.,64.,128.));
        let high=vec3(0.,0.,160.);let low=vec3(0.,0.,80.);
        w.set_dynamic(vec![ledge.clone()]);
        assert!(w.sweep(high,low,Vec3::ZERO).fraction<1.);
        assert_eq!(w.ledge_trace(high,low,Vec3::ZERO).fraction,1.);
        w.set_settled_supports(vec![ledge]);
        assert!(w.ledge_trace(high,low,Vec3::ZERO).fraction<1.);
        let mut actor=World::fixture(&[]);actor.copy_dynamic_from(&w);
        assert!(actor.ledge_trace(high,low,Vec3::ZERO).fraction<1.);
        w.set_settled_supports(vec![]);
        assert_eq!(w.ledge_trace(high,low,Vec3::ZERO).fraction,1.);
    }
    #[test]
    fn controller_liquids_add_to_the_map_liquids_and_can_move_or_vanish() {
        let mut world = World::fixture(&[(vec3(-100., -100., -10.), vec3(100., 100., 0.))]);
        world.add_liquid(vec3(-100., -100., 0.), vec3(0., 100., 50.), 0x08, None);
        let in_map = vec3(-50., 0., 10.);
        let in_pool = vec3(50., 0., 10.);
        assert_eq!(world.liquid_at(in_map), 0x08);
        assert_eq!(world.liquid_at(in_pool), 0);
        let pool = |x: f32| Liquid {
            contents: 0x10 | 0x1000,
            volume: Collider::fixture(vec3(x, -100., 0.), vec3(x + 100., 100., 50.)),
        };
        world.set_dynamic_liquids(vec![pool(0.)]);
        // The controller's volume adds its liquid bits (only LIQUID_MASK bits survive).
        assert_eq!(world.liquid_at(in_pool), 0x10);
        assert_eq!(world.liquid_at(in_map), 0x08);
        // A moving liquid is replaced wholesale each frame.
        world.set_dynamic_liquids(vec![pool(100.)]);
        assert_eq!(world.liquid_at(in_pool), 0);
        world.set_dynamic_liquids(Vec::new());
        assert_eq!(world.liquid_at(in_map), 0x08);
    }
    #[test]
    fn tangent_landing_at_large_coordinates_does_not_pin_player_or_break_saving() {
        // Synthetic floor and oblique wall, reproducing the rounded tangent
        // contact encountered beside a school entrance pillar. No game assets.
        let mut world = World::fixture(&[(vec3(-2200., 1400., -600.), vec3(-1300., 2400., -512.))]);
        let mut wall = World::fixture(&[(vec3(-1800., 1800., -600.), vec3(-1400., 2200., 0.))])
            .hulls
            .remove(0);
        wall.planes.push(Plane {
            normal: vec3(-0.65079135, 0.7592566, 0.),
            distance: 2543.2927,
        });
        world.set_dynamic(vec![Collider::from_hulls(vec![wall])]);
        let mut player = crate::movement::Player::new(vec3(-1761.25, 1867.965, -508.66));
        player.velocity = vec3(97.96471, 83.96974, -200.);
        let trace = world.body_trace(player.feet, player.feet + player.velocity / 120.);
        assert_eq!(trace.fraction, 0.);
        assert!(!trace.start_solid && !trace.all_solid);
        for _ in 0..1200 {
            player.tick(&world, crate::movement::Controls::default());
            assert!(world.body_clear(player.feet));
            player.validate_save().unwrap();
        }
        assert!(player.grounded);
        assert!(player.velocity.length() < 0.1);
        assert_eq!(player.landings, 1);
        assert!((player.feet.z + 512.).abs() < 0.1);
    }
    #[test]
    fn rotated_corner_is_clear_and_has_walkable_top_contact() {
        let source = World::fixture(&[(vec3(-4., -40., -4.), vec3(52., 40., 480.))]);
        for degrees in [0.1875_f32, 22.5, 45., -45.] {
            let hull = Hull::transformed(
                &source.hulls[0].planes,
                vec3(100., 3000., 64.),
                Quat::from_rotation_y(degrees.to_radians()),
            )
            .unwrap();
            // Both the broad phase and the convex sweep must agree just above
            // the highest corner, including the first tiny rotation increment.
            let top = vec3(if degrees > 0. { -4. } else { 52. }, 0., 480.);
            let corner = vec3(100., 3000., 64.) + Quat::from_rotation_y(degrees.to_radians()) * top;
            let feet = corner + Vec3::Z * 0.1;
            let mut world = World::fixture(&[]);
            world.set_dynamic(vec![Collider::from_hulls(vec![hull])]);
            assert!(world.body_clear(feet), "{degrees}");
            let t = world.body_trace(feet, feet - Vec3::Z * 2.);
            assert!(
                !t.start_solid && !t.all_solid && t.fraction < 1.,
                "{degrees}: {t:?}"
            );
            assert_eq!(t.normal, Vec3::Z, "{degrees}");
            let mut p = crate::movement::Player::new(feet + Vec3::Z * 40.);
            for _ in 0..240 {
                p.tick(&world, Default::default());
            }
            assert!(p.grounded && world.body_clear(p.feet), "{degrees}: {p:?}");
            let before = p.feet;
            p.tick(
                &world,
                crate::movement::Controls {
                    jump: true,
                    ..Default::default()
                },
            );
            assert!(p.jumps == 1 && p.feet.z > before.z, "{degrees}");
        }
    }
    #[test]
    fn tiny_overlap_escape_is_bounded_and_does_not_cross_another_solid() {
        let floor = (vec3(-100., -100., -10.), vec3(100., 100., 0.));
        let w = World::fixture(&[floor]);
        let feet = vec3(0., 0., -0.001);
        let corrected = w.correct_footing(feet).unwrap();
        assert!(corrected.z > 0. && corrected.distance(feet) <= 1. && w.body_clear(corrected));
        assert!(w.correct_footing(vec3(0., 0., -2.)).is_none());
        assert!(w.correct_footing(Vec3::Z).is_none());
        let roof = (vec3(-100., -100., 56.02), vec3(100., 100., 56.04));
        let tight = World::fixture(&[floor, roof]);
        assert!(
            tight.correct_footing(feet).is_none(),
            "Must not pass through the thin roof to a clear endpoint"
        );
        let mut p = crate::movement::Player::new(feet);
        p.tick(
            &w,
            crate::movement::Controls {
                jump: true,
                ..Default::default()
            },
        );
        assert!(p.jumps == 1 && p.feet.z > 0. && w.body_clear(p.feet));
    }
    #[test]
    fn sweep_prevents_tunnelling_and_allows_parallel_motion() {
        let w = World::fixture(&[(vec3(0., -100., -100.), vec3(1., 100., 100.))]);
        let t = w.sweep(vec3(-100., 0., 0.), vec3(1000., 0., 0.), Vec3::splat(2.));
        assert!(!t.start_solid);
        assert!(t.fraction < 0.1);
        assert_eq!(t.normal, -Vec3::X);
        let stop = -100. + 1100. * t.fraction;
        assert!((stop + 2. + SKIN).abs() < 0.001);
        assert_eq!(
            w.sweep(vec3(-2., 0., 0.), vec3(-2., 10., 0.), Vec3::splat(2.))
                .fraction,
            1.
        );
        assert_eq!(
            w.sweep(vec3(-2., 0., 0.), vec3(-1.99999, 0., 0.), Vec3::splat(2.))
                .fraction,
            0.
        );
        assert_eq!(
            w.sweep(vec3(-2., 0., 0.), vec3(-10., 0., 0.), Vec3::splat(2.))
                .fraction,
            1.
        );
    }
    #[test]
    fn reports_embedded_body_and_can_exit() {
        let w = World::fixture(&[(Vec3::splat(-10.), Vec3::splat(10.))]);
        let t = w.sweep(Vec3::ZERO, Vec3::X, Vec3::ZERO);
        assert!(t.start_solid && t.all_solid);
        assert_eq!(t.fraction, 0.);
        let t = w.sweep(Vec3::ZERO, vec3(20., 0., 0.), Vec3::ZERO);
        assert!(t.start_solid && !t.all_solid);
        assert_eq!(t.fraction, 1.);
    }
    #[test]
    fn patch_triangle_has_both_sides_and_bounds() {
        let hull = triangle_hull(
            vec3(-100., -100., 0.),
            vec3(100., -100., 0.),
            vec3(0., 100., 0.),
        )
        .unwrap();
        for (start, end) in [
            (vec3(0., 0., 100.), vec3(0., 0., -100.)),
            (vec3(0., 0., -100.), vec3(0., 0., 100.)),
        ] {
            let mut t = Trace::default();
            clip_hull(&hull, start, end, Vec3::splat(1.), &mut t);
            assert!(t.fraction > 0.4 && t.fraction < 0.5);
        }
        let mut t = Trace::default();
        clip_hull(
            &hull,
            vec3(200., 0., 100.),
            vec3(200., 0., -100.),
            Vec3::splat(1.),
            &mut t,
        );
        assert_eq!(t.fraction, 1.);
    }
}

#[cfg(test)]
mod performance_tests;
