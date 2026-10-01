use super::*;
use crate::audio::{world::Clock, LoopCue};
impl Keep {
    pub(super) fn collect_sound(&self, loops: &mut Vec<LoopCue>, clocks: &mut Vec<Clock>) {
        if self.saved.elapsed < 5. {
            loops.push(LoopCue {
                id: BASE + 5,
                path: "sound/ambience/special/lift_loop.wav",
                origin: vec3(512., 288., -80. + 192. * self.saved.elapsed / 5.),
                clock: Some(self.saved.elapsed),
            });
        }
        clocks.push(Clock {
            key: "keep.lift",
            time: self.saved.elapsed,
            period: None,
            origin: vec3(512., 288., 112.),
            cues: &[(5., "sound/ambience/special/lift_end.wav")],
        });
        let Some(s) = &self.saved.scene else {
            return;
        };
        if s.skip.is_some() {
            return;
        }
        if s.kind == Kind::Mirror {
            clocks.push(Clock {
                key: "keep.mirror",
                time: s.time,
                period: None,
                origin: vec3(512., 2456., -16.),
                cues: &[
                    (0., "sound/world/machine/lever1.wav"),
                    (1.5, "sound/ambience/special/stone grind.wav"),
                    (6.5, "sound/ambience/special/door_slam2.wav"),
                ],
            });
        }
        if let Some((at, _, _, _, _)) = self.cat() {
            let (key, time, delay) = match s.kind {
                Kind::Arrival => ("keep.cat.arrival", s.time, 3.),
                Kind::Hint => ("keep.cat.hint", s.time, 0.5),
                Kind::Death => match s.line {
                    0 => ("keep.cat.first", s.time, 1.),
                    1 => ("keep.cat.second", s.time - s.starts[1], 0.),
                    _ => ("keep.cat.third", s.time - s.starts[2], 0.),
                },
                _ => return,
            };
            let cues = if delay == 3. {
                &[(3., "sound/character/cheshire_cat/appear.wav")][..]
            } else if delay == 1. {
                &[(1., "sound/character/cheshire_cat/appear.wav")][..]
            } else if delay == 0.5 {
                &[(0.5, "sound/character/cheshire_cat/appear.wav")][..]
            } else {
                &[(0., "sound/character/cheshire_cat/appear.wav")][..]
            };
            clocks.push(Clock {
                key,
                time,
                period: None,
                origin: at.translation,
                cues,
            });
            if let Some(time) = s.ending {
                clocks.push(Clock {
                    key: "keep.cat.out",
                    time,
                    period: None,
                    origin: at.translation,
                    cues: &[(0., "sound/character/cheshire_cat/disappear.wav")],
                });
            }
        }
        if let Some(time) = self.strike_time().filter(|t| *t >= 0.) {
            clocks.push(Clock {
                key: "keep.strike",
                time,
                period: None,
                origin: self.data.at("cat_pace2").translation,
                cues: &[
                    (0., "sound/character/queen/popup_explode.wav"),
                    (0.05, "sound/character/cheshire_cat/decap.wav"),
                ],
            });
        }
    }
}
