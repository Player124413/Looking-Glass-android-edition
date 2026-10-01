//! Source-derived action rules, separate from projectile/AI availability.
//! Frame times come from Alice TIKI/SKA and the Blunderbuss TAN. Staff native
//! effects and funded pulses are implemented by the custom heavy-weapon state.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    #[default]
    Shot,
    Charge,
    Sustain,
    End,
}

#[derive(Clone, Copy, Debug)]
pub struct Rule {
    pub clip: usize,
    pub release: f32,
    pub debit: f32,
    pub sound: f32,
    pub cost: f32,
    pub cycle: f32,
}

pub fn canonical_mode(weapon: usize, alternate: bool) -> bool {
    alternate && !matches!(weapon, 6 | 8 | 9)
}

pub fn rule(weapon: usize, alternate: bool, variation: usize) -> Rule {
    let alternate = canonical_mode(weapon, alternate);
    let (clip, release, debit, sound, cost, cycle) = match (weapon, alternate) {
        (0, false) => (1 + variation % 3, 0.35, 0.35, 0.25, 0., 0.),
        (0, true) => (4, 0.40, 0.40, 0.30, 0., 3.5),
        (1, false) => (5, 0., 0., 0., 3., 0.),
        (1, true) => (6, 0.35, 0.35, 0.35, 20., 0.5),
        (2, false) => (7 + variation % 2, 0.35, 0.35, 0., 0., 0.),
        (2, true) => (9, 0.65, 0.65, 0.10, 8., 0.),
        (3, false) => (10, 0.35, 0.35, 0.35, 15., 3.),
        (3, true) => (10, 0.35, 0.35, 0.35, 20., 8.),
        (4, false) => (12, 0., 0., 0., 0.75, 0.),
        (4, true) => (13, 0.45, 0.45, 0.05, 10., 0.),
        (5, false) => (10, 0.35, 0.35, 0.20, 10., 6.5),
        (5, true) => (4, 0.40, 0.40, 0.20, 20., 2.5),
        (6, _) => (10, 0.35, 0.35, 0.35, 40., 6.),
        // Custom class starts a held effect once; native pulses own its debit.
        (7, false) => (14, 0., 0., 0., 0., 0.),
        (7, true) => (16, 0.60, 0.60, 0., 0., 0.),
        // Alice starts the weapon at frame 1; its TAN shoots 14 frames later.
        (8, _) => (19, 0.75, 0.05, 0.05, 99., 0.),
        (9, _) => (11, 1.10, 1.10, 1.10, 1., 360.),
        _ => unreachable!("invalid toy"),
    };
    Rule {
        clip,
        release,
        debit,
        sound,
        cost,
        cycle,
    }
}

/// A fresh release is required after a menu/focus/cinematic boundary. This is
/// device-neutral: callers OR the rebound mouse/keyboard/pad binding states.
#[derive(Default)]
pub struct Buttons {
    blocked: bool,
    previous: [bool; 2],
    pub pressed: [bool; 2],
    pub released: [bool; 2],
}
impl Buttons {
    pub fn block(&mut self) {
        self.blocked = true;
    }
    pub fn sample(&mut self, down: [bool; 2], enabled: bool) -> Option<bool> {
        self.pressed = std::array::from_fn(|i| down[i] && !self.previous[i]);
        self.released = std::array::from_fn(|i| !down[i] && self.previous[i]);
        self.previous = down;
        if !enabled {
            self.blocked = true;
            return None;
        }
        if self.blocked {
            self.blocked = down.iter().any(|b| *b);
            return None;
        }
        // STAND tests primary before alternate in the original state graph.
        if down[0] {
            Some(false)
        } else if down[1] {
            Some(true)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn held_buttons_edges_priority_and_resume_are_device_independent() {
        let mut b = Buttons::default();
        assert_eq!(b.sample([true, true], true), Some(false));
        assert_eq!(b.pressed, [true, true]);
        assert_eq!(b.sample([true, true], true), Some(false));
        assert_eq!(b.pressed, [false, false]);
        assert_eq!(b.sample([false, true], true), Some(true));
        assert_eq!(b.released, [true, false]);
        assert_eq!(b.sample([false, true], false), None);
        assert_eq!(b.sample([false, true], true), None);
        assert_eq!(b.sample([false, false], true), None);
        assert_eq!(b.sample([false, true], true), Some(true));
    }
    #[test]
    fn every_toy_has_two_valid_input_mappings() {
        for toy in 0..10 {
            for alt in [false, true] {
                let r = rule(toy, alt, 0);
                assert!(r.cost >= 0. && r.debit <= r.release && r.cycle >= 0.);
                if [6, 8, 9].contains(&toy) {
                    assert_eq!(r.clip, rule(toy, !alt, 0).clip);
                    assert_eq!(r.cost, rule(toy, !alt, 0).cost);
                }
            }
        }
    }
}
