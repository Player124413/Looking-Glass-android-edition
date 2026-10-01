//! Pond contact time and the non-skippable fish death, saved independently of frame rate.
use super::*;
#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct State {
    pub exposure: f64,
    pub gap: f64,
    pub attack: Option<f64>,
    pub at: Vec3,
    pub killed: bool,
}
impl State {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (0. ..=6.1).contains(&self.exposure) && (0. ..=0.6).contains(&self.gap),
            "Invalid fish contact clock"
        );
        ensure!(
            self.attack.is_none_or(|t| (0. ..=2.).contains(&t))
                && self.at.is_finite()
                && self.at.abs().max_element() < 100000.,
            "Invalid fish attack"
        );
        ensure!(
            !self.killed || self.attack == Some(2.),
            "Fish death before bite"
        );
        Ok(())
    }
    pub fn step(&mut self, dt: f32, inside: bool, p: &mut Player) {
        if dt <= 0. {
            return;
        }
        let dt = f64::from(dt.min(0.1));
        if let Some(t) = &mut self.attack {
            *t = (*t + dt).min(2.);
            p.velocity = Vec3::ZERO;
            p.script_motion = 1;
            p.cancel_climb();
            p.release_rope();
        } else {
            if inside {
                self.gap = 0.;
                self.exposure = (self.exposure + dt).min(6.1);
            } else {
                self.gap = (self.gap + dt).min(0.6);
                if self.gap > 0.5 {
                    self.exposure = 0.;
                } else if self.exposure > 0. {
                    self.exposure = (self.exposure + dt).min(6.1);
                }
            }
            if inside && self.exposure >= 6. {
                self.attack = Some(0.);
                self.at = p.feet;
            }
        }
    }
    pub fn camera(&self) -> Option<crate::cinematic::Camera> {
        self.attack.map(|_| crate::cinematic::Camera {
            eye: self.at + vec3(80., 80., 200.),
            target: self.at + Vec3::Z * 24.,
            up: Vec3::Z,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contact_gap_pause_and_bite_are_durable() {
        for hz in [30, 60, 120, 144] {
            let dt = 1. / hz as f32;
            let mut p = Player::new(Vec3::ZERO);
            let mut s = State::default();
            for _ in 0..hz * 5 {
                s.step(dt, true, &mut p);
            }
            let before = serde_json::to_value(&s).unwrap();
            s.step(0., true, &mut p);
            assert_eq!(before, serde_json::to_value(&s).unwrap());
            for _ in 0..hz {
                s.step(dt, false, &mut p);
            }
            assert_eq!(s.exposure, 0.);
            for _ in 0..hz * 5 {
                s.step(dt, true, &mut p);
            }
            let mut restored: State =
                serde_json::from_value(serde_json::to_value(&s).unwrap()).unwrap();
            for _ in 0..hz / 4 {
                s.step(dt, false, &mut p);
                restored.step(dt, false, &mut p);
            }
            for _ in 0..hz {
                s.step(dt, true, &mut p);
                restored.step(dt, true, &mut p);
            }
            assert!(s.attack.is_some());
            assert_eq!(
                serde_json::to_value(&s).unwrap(),
                serde_json::to_value(&restored).unwrap()
            );
            for _ in 0..hz * 3 {
                s.step(dt, true, &mut p);
            }
            assert_eq!(s.attack, Some(2.));
            assert!(s.validate().is_ok());
        }
    }
}
