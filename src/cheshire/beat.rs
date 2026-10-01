//! Reusable CatBeat presentation clock. Dialogue and quest commits belong to their owners.
use anyhow::{ensure, Result};

const FADE_IN: f32 = 2.;
const SPEECH: f32 = 2.5;
// Story already retains each legacy hint for 0.35 s after its recording.
const HOLD: f32 = 0.15;
const FADE_OUT: f32 = 2.;
const TAIL: f32 = 1.;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct CatBeat {
    // Omitted in old format-12 appearances. Finish those on their existing timeline.
    #[serde(default, skip_serializing_if = "legacy")]
    version: u8,
    pub elapsed: f32,
    pub leaving: Option<f32>,
}
fn legacy(version: &u8) -> bool {
    *version == 0
}
impl CatBeat {
    pub fn new() -> Self {
        Self {
            version: 1,
            elapsed: 0.,
            leaving: None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version <= 1
                && self.elapsed.is_finite()
                && (0. ..=1e6).contains(&self.elapsed)
                && self.leaving.is_none_or(|t| t.is_finite()
                    && (if self.version == 0 { 0. } else { -HOLD }..=FADE_OUT + TAIL).contains(&t)),
            "Invalid saved CatBeat"
        );
        Ok(())
    }
    pub fn speech_ready(&self, visible: bool) -> bool {
        self.version == 0 || !visible || self.elapsed >= SPEECH
    }
    pub fn opacity(&self) -> f32 {
        let (fade_in, fade_out) = if self.version == 0 {
            (0.35, 0.4)
        } else {
            (FADE_IN, FADE_OUT)
        };
        (self.elapsed / fade_in).clamp(0., 1.)
            * self
                .leaving
                .map_or(1., |t| (1. - t / fade_out).clamp(0., 1.))
    }
    /// (departure cue, finished). The saved departure clock makes the cue one-shot.
    pub fn tick(&mut self, dt: f32, speaking: bool, visible: bool) -> (bool, bool) {
        if dt <= 0. || !dt.is_finite() {
            return (false, false);
        }
        self.elapsed += dt;
        let first = !speaking && self.leaving.is_none();
        if first {
            self.leaving = Some(if self.version == 0 || !visible {
                0.
            } else {
                -HOLD
            });
        }
        let Some(t) = &mut self.leaving else {
            return (false, false);
        };
        let before = *t;
        *t += dt;
        if self.version == 0 {
            return (first, *t >= 0.4);
        }
        (
            visible && before < 0. && *t >= 0.,
            !visible || *t >= FADE_OUT + TAIL,
        )
    }
    pub fn cooldown(&self, visible: bool) -> f32 {
        if self.version == 0 {
            2.
        } else if visible {
            9.
        } else {
            14.65
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_appearance_keeps_timing_and_pause_rejects_invalid_deltas() {
        let old = serde_json::json!({"elapsed":0.2_f32,"leaving":null});
        let mut b: CatBeat = serde_json::from_value(old.clone()).unwrap();
        b.validate().unwrap();
        assert!(b.speech_ready(true));
        for dt in [0., -1., f32::NAN, f32::INFINITY] {
            assert_eq!(b.tick(dt, false, true), (false, false));
            assert_eq!(serde_json::to_value(&b).unwrap(), old);
        }
        assert_eq!(b.tick(0.4, false, true), (true, true));
        assert_eq!(b.cooldown(true), 2.);
    }
    #[test]
    fn saved_fade_clock_emits_departure_once_at_each_frame_rate() {
        for hz in [30, 60, 144] {
            let mut b = CatBeat::new();
            let dt = 1. / hz as f32;
            for _ in 0..hz {
                b.tick(dt, true, true);
            }
            assert!((b.opacity() - 0.5).abs() < 0.001);
            assert!(!b.speech_ready(true));
            for _ in 0..hz * 2 {
                b.tick(dt, true, true);
            }
            assert!(b.speech_ready(true));
            let mut cues = 0;
            for _ in 0..hz * 4 {
                b = serde_json::from_value(serde_json::to_value(&b).unwrap()).unwrap();
                b.validate().unwrap();
                let (cue, done) = b.tick(dt, false, true);
                cues += usize::from(cue);
                if done {
                    break;
                }
            }
            assert_eq!(cues, 1);
            assert_eq!(b.opacity(), 0.);
        }
    }
}
