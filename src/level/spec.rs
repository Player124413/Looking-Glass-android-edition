//! Minimal C1 declarations used as a library by legacy scene owners.
use crate::skeletal::Transform;

#[derive(Clone, Copy)]
pub struct ExitSpec {
    pub map: &'static str,
    pub entrance: &'static str,
}
impl ExitSpec {
    pub fn destination(self) -> (String, Option<String>) {
        (self.map.into(), Some(self.entrance.into()))
    }
    pub fn matches(self, exit: &(String, Option<String>)) -> bool {
        exit.0 == self.map && exit.1.as_deref() == Some(self.entrance)
    }
}

#[derive(Clone, Copy)]
pub struct EndSpec {
    /// None returns to the captured pose, unless an exit retains player ownership.
    pub landing: Option<Transform>,
    pub exit: Option<ExitSpec>,
}

pub struct ShotSpec {
    pub start: f32,
    pub track: &'static str,
    pub offset: f32,
    /// Clamp playback at this local time, independently of the scene clock.
    pub hold: f32,
}

pub struct SceneSpec {
    pub id: &'static str,
    pub version: u8,
    pub duration: f32,
    pub shots: &'static [ShotSpec],
    pub cues: &'static [f32],
    pub end: EndSpec,
}
