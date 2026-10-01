//! Bounded, saved emitter poses: particle birth positions survive a reload.
use crate::skeletal::Transform;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct Trail(Vec<(f32, Transform)>);
impl Trail {
    pub fn record(&mut self, age: f32, pose: Transform) {
        if self.0.last().is_some_and(|p| age <= p.0) {
            return;
        }
        self.0.push((age, pose));
        let drop = self
            .0
            .partition_point(|p| p.0 < age - 5.1)
            .saturating_sub(1);
        if drop > 0 {
            self.0.drain(..drop);
        }
        if self.0.len() > 800 {
            self.0.remove(0);
        }
    }
    pub fn sample(&self, age: f32, fallback: Transform) -> Transform {
        let i = self.0.partition_point(|p| p.0 <= age);
        if i == 0 {
            return self.0.first().map_or(fallback, |p| p.1);
        }
        let a = self.0[i - 1];
        let Some(&b) = self.0.get(i) else {
            return a.1;
        };
        a.1.blend(b.1, ((age - a.0) / (b.0 - a.0)).clamp(0., 1.))
    }
    pub fn valid(&self, age: f64) -> bool {
        self.0.len() <= 800
            && self.0.iter().all(|(t, p)| {
                t.is_finite()
                    && *t >= 0.
                    && *t as f64 <= age + 0.00001 + age.abs() * f32::EPSILON as f64
                    && p.translation.is_finite()
                    && p.rotation.is_finite()
                    && (p.rotation.length_squared() - 1.).abs() < 0.01
            })
            && self.0.windows(2).all(|s| s[0].0 < s[1].0)
    }
}
