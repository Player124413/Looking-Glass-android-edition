//! The two difficulty-selected vial contacts share one authored, non-quest appearance.
use super::*;
pub const EVENT: &str = "trigger_cat_rage";
const APPEAR: &str = "sound/character/cheshire_cat/appear.wav";
const DISAPPEAR: &str = "sound/character/cheshire_cat/disappear.wav";

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct State {
    // A queued line must not make the actor vanish before it begins speaking.
    pub(super) time: Option<f32>,
    cues: u8,
    pub(super) watch: crate::facial::Watch,
}
impl State {
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.time.is_none_or(|t| t.is_finite() && (0. ..=3.).contains(&t))
                && self.cues <= 3 && self.watch.valid()
                && (self.time.is_some() || self.cues == 0),
            "Invalid school vial hint"
        );
        Ok(())
    }
    pub fn opacity(&self) -> f32 {
        // The source reverses a two-second appearance after only one second.
        self.time.map_or(0., |t| if t < 1. { t / 2. } else { 0.5 * (3. - t) / 2. })
    }
    fn tick(&mut self, dt: f32, local_target: Vec3) -> Vec<&'static str> {
        if !dt.is_finite() || dt <= 0. { return Vec::new(); }
        let Some(t) = &mut self.time else { return Vec::new(); };
        let mut sounds = Vec::new();
        if self.cues & 1 == 0 { self.cues |= 1; sounds.push(APPEAR); }
        *t = (*t + dt.min(0.1)).min(3.);
        if *t >= 1. && self.cues & 2 == 0 { self.cues |= 2; sounds.push(DISAPPEAR); }
        self.watch.update(dt, Some(local_target));
        sounds
    }
}
impl School2 {
    pub(crate) fn rage_hint_unseen(&self) -> bool { self.rage_hint.is_none() }
    pub(super) fn start_rage_hint(&mut self) -> Events {
        let mut e = Events::default();
        if self.rage_hint.is_none() {
            self.rage_hint = Some(State::default());
            e.story.push(EVENT.into());
        }
        e
    }
    pub(super) fn sync_rage_hint(&mut self, story: &crate::story::Story) {
        if story.progress(EVENT).is_some() {
            if let Some(s) = &mut self.rage_hint { s.time.get_or_insert(0.); }
        }
    }
    pub(super) fn advance_rage_hint(&mut self, dt: f32, feet: Vec3, feedback: &mut Feedback) {
        if let Some(s) = &mut self.rage_hint {
            let local = self.rage_pose.rotation.inverse()
                * (feet + Vec3::Z * 48. - (self.rage_pose.translation + Vec3::Z * 24.));
            feedback.spatial_sounds.extend(s.tick(dt, local).into_iter()
                .map(|s| (s.into(), self.rage_pose.translation)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queued_paused_and_restored_vial_hint_keeps_one_sound_pair() {
        for hz in [30, 60, 144] {
            let mut s = State::default();
            assert!(s.tick(10., Vec3::X).is_empty());
            assert_eq!(s.opacity(), 0.);
            s.time = Some(0.);
            let mut sounds = Vec::new();
            for _ in 0..hz * 4 {
                let saved = serde_json::to_value(&s).unwrap();
                for dt in [0., -1., f32::NAN] { assert!(s.tick(dt, Vec3::X).is_empty()); }
                assert_eq!(saved, serde_json::to_value(&s).unwrap());
                s = serde_json::from_value(saved).unwrap();
                sounds.extend(s.tick(1. / hz as f32, Vec3::X));
                s.validate().unwrap();
            }
            assert_eq!(sounds, [APPEAR, DISAPPEAR]);
            assert_eq!(s.opacity(), 0.);
        }
    }
}
