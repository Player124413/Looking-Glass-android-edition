//! W22b: delivery is a transaction, separate from the scene's durable commitment.
use super::spec::ExitSpec;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ExitState {
    pub committed: bool,
    /// A newly loaded process must deliver a committed, still-current exit again.
    /// Successful departure replaces the owner; there is no durable acknowledgement
    /// before the destination has actually loaded.
    #[serde(skip)]
    in_flight: bool,
    pub retry_time: f32,
}
impl ExitState {
    pub fn advance(&mut self, dt: f32) {
        self.retry_time = (self.retry_time - dt).max(0.);
    }
    pub fn request(&mut self, spec: ExitSpec) -> Option<(String, Option<String>)> {
        if !self.committed || self.in_flight || self.retry_time > 0. {
            return None;
        }
        self.in_flight = true;
        Some(spec.destination())
    }
    pub fn failed(&mut self) {
        if self.in_flight {
            self.in_flight = false;
            self.retry_time = 1.;
        }
    }
}
