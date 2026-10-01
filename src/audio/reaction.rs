//! One non-overlapping voice for Alice's physical reactions. Gameplay damage
//! remains independent of presentation, including per-tick liquid damage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Pain,
    Choke,
    Gasp,
    Death,
}
pub fn kind(path: &str) -> Option<Kind> {
    let name = path.strip_prefix("sound/character/alice/")?;
    if name.starts_with("pain") {
        Some(Kind::Pain)
    } else if name.starts_with("choke") {
        Some(Kind::Choke)
    } else if name.starts_with("gasp") {
        Some(Kind::Gasp)
    } else if name.starts_with("death") {
        Some(Kind::Death)
    } else {
        None
    }
}
#[derive(Default)]
pub struct Gate {
    current: Option<Kind>,
    remaining: f32,
}
impl Gate {
    pub fn tick(&mut self, dt: f32) {
        self.remaining = (self.remaining - dt).max(0.);
    }
    pub fn accept(&mut self, kind: Kind, duration: f32) -> bool {
        if self.remaining > 0. && self.current.is_some_and(|current| kind <= current) {
            return false;
        }
        self.current = Some(kind);
        // Leave a breath between repeated hazard reactions and always let the
        // actual asset finish. Surfacing and death may interrupt lesser cries.
        self.remaining = (duration + 0.2).max(1.2);
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuous_hazards_do_not_restart_voices_at_any_frame_rate() {
        for hz in [30, 60, 144] {
            let mut gate = Gate::default();
            let mut starts = vec![];
            for frame in 0..hz * 10 {
                let now = frame as f32 / hz as f32;
                if gate.accept(Kind::Pain, 0.998) {
                    starts.push(now);
                }
                gate.tick(1. / hz as f32);
            }
            assert!((8..=9).contains(&starts.len()));
            assert!(starts.windows(2).all(|t| t[1] - t[0] >= 1.199));
        }
    }
    #[test]
    fn choking_finishes_and_surfacing_or_death_preempts_without_overlap() {
        let mut gate = Gate::default();
        assert!(gate.accept(Kind::Pain, 0.5));
        assert!(gate.accept(Kind::Choke, 0.775));
        gate.tick(0.5);
        assert!(!gate.accept(Kind::Choke, 0.775));
        assert!(!gate.accept(Kind::Pain, 0.998));
        gate.tick(0.); // pause
        assert!(!gate.accept(Kind::Choke, 0.775));
        assert!(gate.accept(Kind::Gasp, 1.295));
        assert!(!gate.accept(Kind::Choke, 0.775));
        assert!(gate.accept(Kind::Death, 2.));
        gate.tick(1.5);
        assert!(!gate.accept(Kind::Pain, 0.5));
        gate.tick(0.71);
        assert!(gate.accept(Kind::Pain, 0.5));
    }
}
