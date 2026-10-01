//! Bounded deterministic flight routing using swept body clearance.
use crate::collision::World;
use macroquad::prelude::*;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap},
};

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Navigator {
    pub path: Vec<Vec3>,
    goal: Vec3,
    retry: f32,
}
fn clear(world: &World, a: Vec3, b: Vec3, half: Vec3) -> bool {
    let trace = world.sweep(a, b, half);
    !trace.start_solid
        && trace.fraction >= 1.
        && (0..=4).all(|i| world.liquid_at(a.lerp(b, i as f32 / 4.) - Vec3::Z * half.z) == 0)
}
impl Navigator {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.path.len() <= 128
                && self.path.iter().all(|p| p.is_finite())
                && self.goal.is_finite()
                && (0.0..=1.1).contains(&self.retry),
            "Invalid saved flight route"
        );
        Ok(())
    }
    pub fn reset(&mut self) {
        self.path.clear();
        self.retry = 0.;
    }
    pub fn advance(
        &mut self,
        world: &World,
        start: Vec3,
        goal: Vec3,
        half: Vec3,
        speed: f32,
        dt: f32,
    ) -> Vec3 {
        if dt <= 0. {
            return start;
        }
        self.retry = (self.retry - dt).max(0.);
        let desired = if clear(world, start, goal, half) {
            self.path.clear();
            goal
        } else {
            if (self.path.is_empty() || self.goal.distance(goal) > 80.) && self.retry == 0. {
                self.path = route(world, start, goal, half);
                self.goal = goal;
                self.retry = 1.;
            }
            while self.path.first().is_some_and(|p| p.distance(start) < 4.) {
                self.path.remove(0);
            }
            // Shorten a checked route as soon as a later waypoint is directly reachable.
            if let Some(index) = self
                .path
                .iter()
                .rposition(|p| clear(world, start, *p, half))
            {
                self.path.drain(..index);
            }
            self.path.first().copied().unwrap_or(start)
        };
        let end = start + (desired - start).clamp_length_max(speed * dt);
        if clear(world, start, end, half) {
            end
        } else {
            self.path.clear();
            start
        }
    }
}
fn route(world: &World, start: Vec3, goal: Vec3, half: Vec3) -> Vec<Vec3> {
    const CELL: f32 = 80.;
    type Key = [i16; 3];
    let point = |k: Key| start + vec3(k[0] as f32, k[1] as f32, k[2] as f32) * CELL;
    let estimate = |k: Key| (point(k).distance(goal) * 10.) as u32;
    let root = [0; 3];
    let mut nodes = BTreeMap::from([(root, (0u32, root))]);
    let mut queue = BinaryHeap::from([Reverse((estimate(root), 0u32, root))]);
    let mut best = root;
    let mut reached = false;
    let mut sequence = 0;
    let mut expanded = 0;
    while let Some(Reverse((_, _, key))) = queue.pop() {
        expanded += 1;
        if expanded > 1536 {
            break;
        }
        let at = point(key);
        if estimate(key) < estimate(best) {
            best = key;
        }
        if clear(world, at, goal, half) {
            best = key;
            reached = true;
            break;
        }
        let cost = nodes[&key].0;
        for axis in 0..3 {
            for direction in [-1, 1] {
                let mut next = key;
                next[axis] += direction;
                if next[0].abs() > 8 || next[1].abs() > 8 || next[2].abs() > 6 {
                    continue;
                }
                let new_cost = cost + 800;
                if nodes.get(&next).is_some_and(|v| v.0 <= new_cost)
                    || !clear(world, at, point(next), half)
                {
                    continue;
                }
                nodes.insert(next, (new_cost, key));
                sequence += 1;
                queue.push(Reverse((new_cost + estimate(next), sequence, next)));
            }
        }
    }
    let mut result = Vec::new();
    if reached {
        result.push(goal);
    }
    let mut at = best;
    while at != root && result.len() < 128 {
        result.push(point(at));
        at = nodes[&at].1;
    }
    result.reverse();
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn route_goes_around_a_wall_under_a_ceiling_without_clipping() {
        let world = World::fixture(&[
            (vec3(130., -140., -40.), vec3(170., 140., 260.)),
            (vec3(-500., -500., 220.), vec3(700., 500., 250.)),
            (vec3(-500., -500., -80.), vec3(700., 500., 0.)),
        ]);
        for hz in [30, 60, 144] {
            let mut nav = Navigator::default();
            let mut at = vec3(0., 0., 90.);
            let goal = vec3(320., 0., 90.);
            let half = Vec3::splat(24.);
            let mut detoured = false;
            for _ in 0..hz * 8 {
                let next = nav.advance(&world, at, goal, half, 150., 1. / hz as f32);
                assert!(clear(&world, at, next, half));
                at = next;
                detoured |= at.y.abs() > 150.;
            }
            assert!(detoured && at.distance(goal) < 1., "{hz}: {at:?}");
        }
    }
    #[test]
    fn route_avoids_water_and_freezes_when_paused() {
        let mut world = World::fixture(&[]);
        world.add_liquid(vec3(100., -100., -100.), vec3(200., 100., 150.), 32, None);
        let mut nav = Navigator::default();
        let start = vec3(0., 0., 50.);
        assert_eq!(
            nav.advance(&world, start, Vec3::X * 300., Vec3::splat(24.), 240., 0.),
            start
        );
        let mut at = start;
        for _ in 0..600 {
            at = nav.advance(
                &world,
                at,
                vec3(300., 0., 50.),
                Vec3::splat(24.),
                240.,
                1. / 120.,
            );
            assert_eq!(world.liquid_at(at - Vec3::Z * 24.), 0);
        }
        assert!(at.distance(vec3(300., 0., 50.)) < 1.);
    }
}
