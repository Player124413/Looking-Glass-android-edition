//! Source collapse maps, constrained by a conservative all-frame displacement bound.
use macroquad::prelude::*;
#[derive(Clone)]
pub struct Level {
    pub indices: Vec<u16>,
    pub error: f32,
}
pub fn levels(
    indices: &[u16],
    frames: &[Vec<Vec3>],
    collapse: &[u16],
    minimum: usize,
) -> Vec<Level> {
    let Some(first) = frames.first() else {
        return vec![];
    };
    let n = first.len();
    if collapse.len() != n
        || n < 16
        || collapse
            .iter()
            .enumerate()
            .any(|(i, &p)| i > 0 && p as usize >= i)
    {
        return vec![];
    }
    let mut out = Vec::new();
    for fraction in [0.98, 0.95, 0.9, 0.8, 0.6, 0.4] {
        let keep = ((n as f32 * fraction) as usize).max(minimum).max(3);
        if keep >= n {
            continue;
        }
        let mut map: Vec<u16> = (0..n as u16).collect();
        for i in keep..n {
            map[i] = map[collapse[i] as usize];
        }
        let reduced: Vec<u16> = indices
            .chunks_exact(3)
            .filter_map(|t| {
                let t = [map[t[0] as usize], map[t[1] as usize], map[t[2] as usize]];
                (t[0] != t[1] && t[1] != t[2] && t[0] != t[2]).then_some(t)
            })
            .flatten()
            .collect();
        if reduced.is_empty() || reduced.len() >= indices.len() {
            continue;
        }
        let error = frames
            .iter()
            .flat_map(|f| {
                map.iter()
                    .enumerate()
                    .map(move |(i, &j)| f[i].distance(f[j as usize]))
            })
            .fold(0_f32, f32::max);
        out.push(Level {
            indices: reduced,
            error,
        });
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cycles_disable_detail_and_animated_error_is_bounded() {
        let f = vec![(0..20).map(|i| Vec3::X * i as f32).collect::<Vec<_>>()];
        let triangles = vec![0, 18, 19, 0, 1, 2];
        assert!(levels(&triangles, &f, &(0..20).collect::<Vec<_>>(), 0).is_empty());
        let c = (0..20_u16).map(|i| i.saturating_sub(1)).collect::<Vec<_>>();
        let l = levels(&triangles, &f, &c, 0);
        assert!(!l.is_empty());
        assert!(l
            .iter()
            .all(|l| l.indices.len() < triangles.len() && l.error >= 1.));
        assert!(levels(&triangles, &f, &c, 20).is_empty());
        let mut animated = f.clone();
        animated.push(f[0].iter().map(|p| *p * 3.).collect());
        let moving = levels(&triangles, &animated, &c, 0);
        assert!(l
            .iter()
            .zip(moving)
            .all(|(a, b)| (b.error - a.error * 3.).abs() < 0.001));
    }
}
