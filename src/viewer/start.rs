//! Front end before a world exists: browsing and quitting cannot create a save.
use super::*;
use crate::menu::{Action, Menu, Page};

pub enum Start {
    Chapter,
    NewGame,
    Load(Box<save::Loaded>),
    Quit,
}

pub async fn choose(
    options: &mut Options,
    menu: &mut Menu,
    preferences: &mut crate::preferences::Preferences,
    input: &mut crate::input::Input,
    audio: &mut Audio,
    store: &Store,
    maps: &[String],
) -> Start {
    menu.frontend = true;
    menu.open(Page::Main, preferences, audio.settings);
    loop {
        match menu
            .run(preferences, audio, store, Vec3::ZERO, 0., input)
            .await
        {
            Action::Chapters => {
                let current = maps.iter().position(|m| m == "gvillage").unwrap_or(0);
                match menu
                    .choose_chapter(
                        maps,
                        current,
                        None,
                        &mut options.difficulty,
                        preferences,
                        audio,
                        input,
                    )
                    .await
                {
                    crate::menu::ChapterChoice::Begin(choice) => {
                        options.map = maps[choice.map].clone();
                        options.entry = choice.entry.map(str::to_owned);
                        options.new_game = true;
                        return Start::Chapter;
                    }
                    crate::menu::ChapterChoice::Quit => return Start::Quit,
                    crate::menu::ChapterChoice::Back => (),
                }
            }
            Action::NewGame(difficulty) => {
                options.map = "gvillage".into();
                options.entry = None;
                options.difficulty = difficulty;
                options.new_game = true;
                return Start::NewGame;
            }
            Action::Load(slot) => match store.read(slot) {
                Ok(loaded) => return Start::Load(Box::new(loaded)),
                Err(e) => menu.message = format!("Load failed: {e:#}"),
            },
            Action::Quit => return Start::Quit,
            Action::Save(_) | Action::Resume => unreachable!("No game exists at the front end"),
        }
    }
}
