//! Sound facade over the shared animation-event timeline.
pub use crate::animation_events::Span;
use crate::{
    animation_events::{Command, Model as Events},
    assets::Assets,
};
use anyhow::Result;
#[derive(Clone, Debug)]
pub struct Cue {
    pub path: String,
    pub volume: f32,
}
#[derive(Default)]
pub struct Model {
    events: Events,
}
impl Model {
    pub fn load(assets: &mut Assets, path: &str) -> Result<Self> {
        Self::parse(&String::from_utf8_lossy(&assets.read(path)?))
    }
    fn parse(text: &str) -> Result<Self> {
        Ok(Self {
            events: Events::parse(text)?,
        })
    }
    pub fn retain(&mut self, clips: &[&str]) {
        self.events.clips.retain(|n, _| clips.contains(&n.as_str()));
    }
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.events
            .clips
            .values()
            .flatten()
            .filter_map(|e| match &e.command {
                Command::Sound { path, .. } => Some(path.as_str()),
                _ => None,
            })
    }
    pub fn between(&self, name: &str, span: Span) -> Vec<Cue> {
        self.events
            .between(name, span)
            .into_iter()
            .filter_map(|e| match e {
                Command::Sound { path, volume } => Some(Cue {
                    path: path.clone(),
                    volume: *volume,
                }),
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn model() -> Model {
        Model::parse("animations\n{\nrun run.ska\n{\nserver\n{\n0 sound sound/wrong.wav\n}\nclient\n{\n0 sound sound/start.wav 0 .6 384\n4 sound sound/step.wav 0 .5 384\n}\n}\n}").unwrap()
    }
    #[test]
    fn performances_sharing_frames_keep_separate_sound_commands() {
        let model = Model::parse("animations\n{\nrun run.ska\n{\nclient\n{\n4 sound sound/step.wav 0 .6\n}\n}\nrun_jump run.ska\n{\nclient\n{\n1 sound sound/jump.wav 1 .8\n}\n}\nsilent_run run.ska\n}").unwrap();
        let span = Span {
            start: 0.,
            end: 0.8,
            duration: 1.,
            frame_time: 0.1,
            looping: true,
            entered: true,
        };
        assert_eq!(
            model
                .between("run", span)
                .iter()
                .map(|c| c.path.as_str())
                .collect::<Vec<_>>(),
            ["sound/step.wav"]
        );
        assert_eq!(model.between("run_jump", span)[0].path, "sound/jump.wav");
        assert!(model.between("silent_run", span).is_empty());
    }
    #[test]
    fn frame_crossings_pause_resume_and_wrap_do_not_duplicate() {
        let model = model();
        let mut span = Span {
            start: 0.,
            end: 0.5,
            duration: 1.,
            frame_time: 0.1,
            looping: true,
            entered: true,
        };
        assert_eq!(model.between("run", span).len(), 2);
        span.entered = false;
        span.start = 0.5;
        span.end = 0.5;
        assert!(model.between("run", span).is_empty());
        span.end = 1.1;
        assert_eq!(model.between("run", span)[0].path, "sound/start.wav");
        span.start = 0.4;
        span.end = 0.5;
        assert!(
            model.between("run", span).is_empty(),
            "restored boundary must not replay"
        );
    }
    #[test]
    fn frame_events_are_independent_of_render_rate() {
        for hz in [30, 60, 144] {
            let model = model();
            let mut count = 0;
            for i in 0..hz * 2 {
                count += model
                    .between(
                        "run",
                        Span {
                            start: i as f32 / hz as f32,
                            end: (i + 1) as f32 / hz as f32,
                            duration: 1.,
                            frame_time: 0.1,
                            looping: true,
                            entered: i == 0,
                        },
                    )
                    .len();
            }
            assert_eq!(count, 5);
        }
    }
}
