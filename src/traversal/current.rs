//! Map-owned gravity paths. Only the nearest eligible chain contributes.
use super::*;
#[derive(Clone, Debug)]
pub struct Node {
    pub position: Vec3,
    pub speed: f32,
    pub radius: f32,
    pub maxspeed: f32,
    pub active: bool,
}
#[derive(Clone, Debug)]
pub struct Path {
    pub nodes: Vec<Node>,
    pub oppose: bool,
    pub water: bool,
}
impl Path {
    pub fn load(map: &Bsp, world: &World) -> Result<Vec<Self>> {
        let mut out = vec![];
        for (id, e) in map.entities.iter().enumerate().filter(|(_, e)| {
            e.get("classname")
                .is_some_and(|s| s == "info_grav_pathnode")
                && number(e, "spawnflags", 0.) as u32 & 1 != 0
        }) {
            let mut next = Some(id);
            let mut seen = std::collections::BTreeSet::new();
            let mut nodes = vec![];
            while let Some(i) = next {
                anyhow::ensure!(seen.insert(i), "Cyclic gravity path");
                let e = &map.entities[i];
                let position = e
                    .get("origin")
                    .and_then(|s| vector(s))
                    .context("Missing current origin")?;
                nodes.push(Node {
                    position,
                    speed: number(e, "speed", 100.),
                    radius: number(e, "radius", 256.),
                    maxspeed: number(e, "maxspeed", 200.),
                    active: true,
                });
                next = e.get("target").and_then(|n| {
                    map.entities
                        .iter()
                        .position(|e| e.get("targetname") == Some(n))
                });
            }
            anyhow::ensure!(nodes.len() > 1, "Gravity path needs a segment");
            out.push(Self {
                oppose: number(e, "spawnflags", 0.) as u32 & 2 != 0,
                water: world.liquid_at(nodes[0].position) & 0x20 != 0,
                nodes,
            });
        }
        Ok(out)
    }
    fn sample(&self, at: Vec3) -> Option<(f32, Vec3, f32)> {
        if !self.nodes[0].active {
            return None;
        }
        let mut closest = None;
        let mut distance = 0.;
        for pair in self.nodes.windows(2) {
            let [a, b] = pair else { unreachable!() };
            let delta = b.position - a.position;
            let len = delta.length();
            if len <= 0.001 {
                continue;
            }
            let f = ((at - a.position).dot(delta) / (len * len)).clamp(0., 1.);
            let gap = at.distance(a.position + delta * f);
            let radius = a.radius + (b.radius - a.radius) * f;
            if gap <= radius && closest.is_none_or(|(d, _, _)| gap < d) {
                let speed = if a.active { a.speed } else { 0. } * (1. - f)
                    + if b.active { b.speed } else { 0. } * f;
                closest = Some((gap, distance + len * f, speed));
            }
            distance += len;
        }
        let (gap, along, speed) = closest?;
        let ahead = along + speed;
        let mut start = 0.;
        for p in self.nodes.windows(2) {
            let delta = p[1].position - p[0].position;
            let len = delta.length();
            if ahead < start + len {
                // Native look-ahead offsets the input position by this segment's
                // direction; it does not attract the player toward the centerline.
                return Some((gap, delta.normalize_or_zero() * speed, p[1].maxspeed));
            }
            start += len;
        }
        let last = self.nodes.last()?;
        Some((
            gap,
            (last.position - at).normalize_or_zero() * speed,
            last.maxspeed,
        ))
    }
}
pub fn apply(paths: &[Path], world: &World, feet: Vec3, velocity: &mut Vec3) {
    let Some((path, (_, pull, cap))) = paths
        .iter()
        .filter(|p| !p.water || world.liquid_at(feet) & 0x20 != 0)
        .filter_map(|p| p.sample(feet).map(|s| (p, s)))
        .min_by(|a, b| a.1 .0.total_cmp(&b.1 .0))
    else {
        return;
    };
    if pull.length_squared() < 0.001 {
        return;
    }
    // Resample the native server-step contribution at the port's fixed 120Hz.
    let step = crate::movement::FIXED_DT * 20.;
    if path.oppose {
        let dot = velocity.normalize_or_zero().dot(pull.normalize_or_zero());
        if dot < 0. {
            *velocity *= (1. - (1. - dot) * step).max(0.);
        }
    }
    let old = velocity.length();
    let candidate = *velocity + pull * step;
    if candidate.length() < old {
        *velocity = candidate;
    } else if old >= cap {
        velocity.z += pull.z * step;
    } else {
        *velocity = candidate.clamp_length_max(cap);
    }
}
