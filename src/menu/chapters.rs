use super::*;
use crate::{
    campaign,
    chapters::{Chapters, Hit},
};

pub enum ChapterChoice {
    Begin(campaign::LevelChoice),
    Back,
    Quit,
}

impl Menu {
    /// The same chapter navigation as the in-game overlay, over the original menu art.
    pub async fn choose_chapter(
        &mut self,
        maps: &[String],
        current: usize,
        entry: Option<&str>,
        difficulty: &mut Difficulty,
        prefs: &Preferences,
        audio: &mut audio::Audio,
        input: &mut crate::input::Input,
    ) -> ChapterChoice {
        let mut choices = campaign::level_choices(maps);
        let mut chapters = Chapters::default();
        chapters.open(
            campaign::choice_position(&choices, current, entry),
            choices.len(),
        );
        show_mouse(false);
        loop {
            let focused = crate::look::window_focused();
            input.update(prefs, focused);
            audio.update(0., Vec3::ZERO, 0., true, true);
            let hit = chapters.update(input, choices.len(), difficulty);
            let result = if is_quit_requested() {
                Some(ChapterChoice::Quit)
            } else if input.ui(KeyCode::Escape) || input.ui(KeyCode::Tab) || hit == Some(Hit::Back)
            {
                Some(ChapterChoice::Back)
            } else if !choices.is_empty() && (input.ui(KeyCode::Enter) || hit == Some(Hit::Begin)) {
                Some(ChapterChoice::Begin(choices.remove(chapters.selected)))
            } else {
                None
            };
            if let Some(result) = result {
                input.suppress();
                show_mouse(true);
                next_frame().await;
                return result;
            }
            self.backdrop();
            chapters.draw(
                &self.ui,
                &choices,
                maps,
                *difficulty,
                focused,
                input.using_pad,
            );
            if focused {
                self.ui.cursor();
            }
            next_frame().await;
        }
    }
}
