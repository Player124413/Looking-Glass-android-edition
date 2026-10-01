//! Small, persistent ground detours. Every edge uses the actor's real supported hull.
use super::*;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct Detour {
    path: Vec<Vec3>,
    goal: Option<Vec3>,
    retry: f32,
}
impl Detour {
    pub fn valid(&self) -> bool {
        self.path.len() <= 32
            && self
                .path
                .iter()
                .chain(self.goal.iter())
                .all(|p| p.is_finite() && p.abs().max_element() < 100_000.)
            && self.retry.is_finite()
            && (0. ..=0.5).contains(&self.retry)
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn walk(
        &mut self,
        world: &World,
        feet: Vec3,
        goal: Vec3,
        half: Vec3,
        distance: f32,
        dt: f32,
    ) -> Vec3 {
        if dt <= 0. || distance <= 0. {
            return feet;
        }
        self.retry = (self.retry - dt).max(0.);
        if self.goal.is_some_and(|old| old.distance(goal) > 96.) {
            self.path.clear();
        }
        self.goal = Some(goal);
        while self
            .path
            .first()
            .is_some_and(|p| p.truncate().distance(feet.truncate()) <= 2.)
        {
            self.path.remove(0);
        }
        let target = self.path.first().copied().unwrap_or(goal);
        let delta = (target - feet).with_z(0.).clamp_length_max(distance);
        if delta.length_squared() < 0.001 {
            return feet;
        }
        let next = walk_body(world, feet, delta, half);
        if next.truncate().distance_squared(feet.truncate()) > 0.0001 {
            return next;
        }
        self.path.clear();
        if self.retry == 0. {
            self.path = plan(world, feet, goal, half);
            self.retry = 0.5;
        }
        feet
    }
}

fn edge(world: &World, from: Vec3, goal: Vec3, half: Vec3) -> Option<Vec3> {
    let delta = (goal - from).with_z(0.);
    let steps = (delta.length() / 12.).ceil().max(1.) as usize;
    let step = delta / steps as f32;
    let mut p = from;
    for _ in 0..steps {
        let q = walk_body(world, p, step, half);
        if q.truncate().distance_squared((p + step).truncate()) > 0.1 {
            return None;
        }
        p = q;
    }
    Some(p)
}

fn plan(world: &World, feet: Vec3, goal: Vec3, half: Vec3) -> Vec<Vec3> {
    struct Node {
        p: Vec3,
        parent: usize,
        cost: f32,
        closed: bool,
        cell: (i32, i32),
    }
    let remaining = |p: Vec3| p.truncate().distance(goal.truncate());
    let mut nodes = vec![Node {
        p: feet,
        parent: 0,
        cost: 0.,
        closed: false,
        cell: (0, 0),
    }];
    let mut seen = std::collections::BTreeSet::from([(0, 0, (feet.z / 16.).round() as i32)]);
    let mut best = 0;
    for _ in 0..96 {
        let Some(i) = (0..nodes.len())
            .filter(|&i| !nodes[i].closed)
            .min_by(|&a, &b| {
                (nodes[a].cost + remaining(nodes[a].p))
                    .total_cmp(&(nodes[b].cost + remaining(nodes[b].p)))
            })
        else {
            break;
        };
        nodes[i].closed = true;
        let at = nodes[i].p;
        if remaining(at) < remaining(nodes[best].p) {
            best = i;
        }
        if remaining(at) < 72. && (at.z - goal.z).abs() < 80. {
            if let Some(p) = edge(world, at, goal, half) {
                nodes.push(Node {
                    p,
                    parent: i,
                    cost: 0.,
                    closed: true,
                    cell: (0, 0),
                });
                best = nodes.len() - 1;
                break;
            }
        }
        for (x, y) in [
            (1, 0),
            (0, 1),
            (-1, 0),
            (0, -1),
            (1, 1),
            (-1, 1),
            (-1, -1),
            (1, -1),
        ] {
            if nodes.len() >= 384 {
                break;
            }
            let cell = (nodes[i].cell.0 + x, nodes[i].cell.1 + y);
            let aim = vec3(
                feet.x + cell.0 as f32 * 48.,
                feet.y + cell.1 as f32 * 48.,
                at.z,
            );
            if seen.contains(&(cell.0, cell.1, (at.z / 16.).round() as i32)) {
                continue;
            }
            if let Some(p) = edge(world, at, aim, half) {
                if seen.insert((cell.0, cell.1, (p.z / 16.).round() as i32)) {
                    nodes.push(Node {
                        p,
                        parent: i,
                        cost: nodes[i].cost + p.distance(at),
                        closed: false,
                        cell,
                    });
                }
            }
        }
    }
    if remaining(nodes[best].p) + 12. >= remaining(feet) {
        return Vec::new();
    }
    let mut path = Vec::new();
    while best != 0 && path.len() < 32 {
        path.push(nodes[best].p);
        best = nodes[best].parent;
    }
    if best != 0 {
        return Vec::new();
    }
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detours_around_a_pillar_and_restores_midway() {
        let world = World::fixture(&[
            (vec3(-500., -500., -20.), vec3(500., 500., 0.)),
            (vec3(90., -65., 0.), vec3(150., 65., 120.)),
        ]);
        let half = vec3(36., 36., 36.);
        let goal = vec3(330., 0., 0.1);
        let mut nav = Detour::default();
        let mut at = Vec3::Z * 0.1;
        for _ in 0..120 {
            at = nav.walk(&world, at, goal, half, 1., 1. / 120.);
        }
        let mut restored: Detour =
            serde_json::from_value(serde_json::to_value(&nav).unwrap()).unwrap();
        let mut other = at;
        for _ in 0..650 {
            at = nav.walk(&world, at, goal, half, 1., 1. / 120.);
            other = restored.walk(&world, other, goal, half, 1., 1. / 120.);
            assert_eq!(at, other);
            assert!(
                !world
                    .sweep(at + Vec3::Z * 36., at + Vec3::Z * 36., half)
                    .start_solid
            );
        }
        assert!(at.distance(goal) < 3., "{at:?}");
        assert!(nav.valid());
    }
    #[test]
    fn no_route_across_a_gap_or_sealed_wall_and_pause_is_inert() {
        for world in [
            World::fixture(&[
                (vec3(-100., -150., -20.), vec3(60., 150., 0.)),
                (vec3(180., -150., -20.), vec3(500., 150., 0.)),
            ]),
            World::fixture(&[
                (vec3(-100., -150., -20.), vec3(500., 150., 0.)),
                (vec3(70., -150., 0.), vec3(80., 150., 200.)),
            ]),
        ] {
            let mut nav = Detour::default();
            let mut at = Vec3::Z * 0.1;
            for _ in 0..300 {
                at = nav.walk(
                    &world,
                    at,
                    vec3(300., 0., 0.1),
                    vec3(36., 36., 36.),
                    1.,
                    1. / 120.,
                );
            }
            assert!(at.x < 40.);
            let before = serde_json::to_value(&nav).unwrap();
            assert_eq!(
                at,
                nav.walk(&world, at, Vec3::X * 400., Vec3::splat(36.), 1., 0.)
            );
            assert_eq!(before, serde_json::to_value(nav).unwrap());
        }
    }
}
