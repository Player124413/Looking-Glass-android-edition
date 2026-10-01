//! Local in-game command console. Commands never invoke a shell or original scripts.
pub mod gameplay;
use macroquad::prelude::*;
use std::collections::VecDeque;

const COMMANDS: &[&str] = &[
    "help",
    "cmdlist",
    "clear",
    "version",
    "status",
    "maplist",
    "map",
    "restart",
    "save",
    "load",
    "noclip",
    "god",
    "notarget",
    "wuss",
    "health",
    "cg_cameradist",
    "give",
    "itemlist",
    "cheshire",
    "fullbright",
    "quit",
];
#[derive(Debug, PartialEq)]
pub enum Command {
    Help,
    Clear,
    Version,
    Status,
    Maps,
    Map(String, Option<String>),
    Restart,
    Save,
    Load(crate::save::Slot),
    Noclip,
    God,
    Notarget,
    Wuss,
    Health(f32),
    CameraDistance(Option<f32>),
    Give(String),
    Items,
    Cheshire,
    Fullbright,
    Quit,
}
pub fn parse(line: &str) -> Result<Command, String> {
    let text = line
        .trim()
        .trim_start_matches(['/', '\\'])
        .to_ascii_lowercase();
    let words = text.split_whitespace().collect::<Vec<_>>();
    match words.as_slice() {
        ["help" | "cmdlist"] => Ok(Command::Help),
        ["clear"] => Ok(Command::Clear),
        ["version"] => Ok(Command::Version),
        ["status" | "getpos"] => Ok(Command::Status),
        ["maplist"] => Ok(Command::Maps),
        ["map", name] => crate::interaction::destination(name)
            .map(|(map, entry)| Command::Map(map, entry))
            .ok_or_else(|| "Use: map <level[$entrance]>".into()),
        ["restart"] => Ok(Command::Restart),
        ["save"] | ["save", "quick"] => Ok(Command::Save),
        ["load"] | ["load", "quick"] => Ok(Command::Load(crate::save::Slot::Quick)),
        ["load", "auto"] => Ok(Command::Load(crate::save::Slot::Auto)),
        ["noclip"] => Ok(Command::Noclip),
        ["god"] => Ok(Command::God),
        ["notarget"] => Ok(Command::Notarget),
        ["wuss"] => Ok(Command::Wuss),
        ["health", value] => {
            number(value, 0., crate::inventory::MAX_CHEAT_HEALTH).map(Command::Health)
        }
        ["cg_cameradist"] => Ok(Command::CameraDistance(None)),
        ["cg_cameradist", value] => {
            number(value, -1024., 1024.).map(|v| Command::CameraDistance(Some(v)))
        }
        ["give", item] => give_name(item).map(Command::Give),
        ["itemlist"] => Ok(Command::Items),
        ["cheshire"] => Ok(Command::Cheshire),
        ["fullbright"] => Ok(Command::Fullbright),
        ["quit"] => Ok(Command::Quit),
        _ => Err("Unknown command or arguments. Type help for supported commands.".into()),
    }
}
fn number(s: &str, min: f32, max: f32) -> Result<f32, String> {
    s.parse::<f32>()
        .ok()
        .filter(|n| (min..=max).contains(n))
        .ok_or_else(|| format!("Enter a finite number between {min} and {max}."))
}
fn give_name(s: &str) -> Result<String, String> {
    let s = s
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(s);
    if matches!(s, "all" | "weapons" | "health" | "will") {
        return Ok(s.into());
    }
    let s = s.replace('\\', "/");
    let s = s.strip_prefix("models/").unwrap_or(&s);
    if s.strip_suffix(".tik").is_some_and(|stem| {
        !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    }) {
        Ok(s.into())
    } else {
        Err(
            "Use: give all|weapons|health|will or give <item.tik>. Type itemlist for filenames."
                .into(),
        )
    }
}
#[derive(Default)]
pub struct Console {
    pub open: bool,
    input: String,
    cursor: usize,
    lines: VecDeque<String>,
    history: VecDeque<String>,
    history_at: Option<usize>,
    draft: String,
    scroll: usize,
}
impl Console {
    pub fn new() -> Self {
        let mut s = Self::default();
        s.print(format!(
            "Looking Glass {} / developer console",
            env!("CARGO_PKG_VERSION")
        ));
        s.print("Type help. Enter runs; Tab completes; Up/Down history; PgUp/PgDn scroll.");
        s
    }
    pub fn print(&mut self, line: impl Into<String>) {
        for row in line.into().lines() {
            println!("Console output: {row}");
            self.lines.push_back(row.chars().take(512).collect());
        }
        while self.lines.len() > 128 {
            self.lines.pop_front();
        }
        self.scroll = 0;
    }
    pub fn clear(&mut self) {
        self.lines.clear();
        self.scroll = 0;
    }
    fn insert(&mut self, c: char) {
        if (c.is_ascii_graphic() || c == ' ') && self.input.len() < 256 && c != '`' && c != '~' {
            self.input.insert(self.cursor, c);
            self.cursor += 1;
        }
    }
    fn history(&mut self, up: bool) {
        if self.history.is_empty() {
            return;
        }
        let next = match (self.history_at, up) {
            (None, true) => {
                self.draft = self.input.clone();
                Some(self.history.len() - 1)
            }
            (Some(i), true) => Some(i.saturating_sub(1)),
            (Some(i), false) if i + 1 < self.history.len() => Some(i + 1),
            _ => None,
        };
        self.input = next.map_or_else(|| self.draft.clone(), |i| self.history[i].clone());
        self.cursor = self.input.len();
        self.history_at = next;
    }
    fn submit(&mut self) -> Option<Result<Command, String>> {
        let line = std::mem::take(&mut self.input);
        self.cursor = 0;
        self.history_at = None;
        self.draft.clear();
        if line.trim().is_empty() {
            return None;
        }
        self.print(format!("> {line}"));
        println!("Console: {line}");
        if self.history.back() != Some(&line) {
            self.history.push_back(line.clone());
        }
        while self.history.len() > 48 {
            self.history.pop_front();
        }
        Some(parse(&line))
    }
    pub fn input(&mut self, chars: &[char]) -> Option<Result<Command, String>> {
        for &c in chars {
            self.insert(c);
        }
        if is_key_pressed(KeyCode::Left) {
            self.cursor = self.cursor.saturating_sub(1);
        }
        if is_key_pressed(KeyCode::Right) {
            self.cursor = (self.cursor + 1).min(self.input.len());
        }
        if is_key_pressed(KeyCode::Home) {
            self.cursor = 0;
        }
        if is_key_pressed(KeyCode::End) {
            self.cursor = self.input.len();
        }
        if is_key_pressed(KeyCode::Backspace) && self.cursor > 0 {
            self.cursor -= 1;
            self.input.remove(self.cursor);
        }
        if is_key_pressed(KeyCode::Delete) && self.cursor < self.input.len() {
            self.input.remove(self.cursor);
        }
        if is_key_pressed(KeyCode::U)
            && (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
        {
            self.input.clear();
            self.cursor = 0;
        }
        if is_key_pressed(KeyCode::Up) {
            self.history(true);
        }
        if is_key_pressed(KeyCode::Down) {
            self.history(false);
        }
        if is_key_pressed(KeyCode::PageUp) {
            self.scroll = (self.scroll + 8).min(self.lines.len().saturating_sub(1));
        }
        if is_key_pressed(KeyCode::PageDown) {
            self.scroll = self.scroll.saturating_sub(8);
        }
        if is_key_pressed(KeyCode::Tab) {
            let prefix = self.input.to_ascii_lowercase();
            let matches = COMMANDS
                .iter()
                .filter(|s| s.starts_with(&prefix))
                .copied()
                .collect::<Vec<_>>();
            if matches.len() == 1 {
                self.input = format!("{} ", matches[0]);
                self.cursor = self.input.len();
            } else {
                self.print(matches.join("  "));
            }
        }
        if is_key_pressed(KeyCode::Enter) {
            self.submit()
        } else {
            None
        }
    }
    pub fn draw(&self, ui: &crate::ui::Ui) {
        if !self.open {
            return;
        }
        let width = screen_width();
        let height = (screen_height() * 0.56).max(180.);
        let font = 18;
        let ink = Color::from_hex(0xe8ddc8);
        ui.tile(Rect::new(0., 0., width, height));
        ui.courier.left(
            "Console   ~ / Esc to close",
            18.,
            26.,
            18.,
            Color::from_hex(0xc6ab79),
        );
        let columns = ((width - 40.) / ui.courier.width("W", font as f32).max(1.)).max(8.) as usize;
        let rows = self
            .lines
            .iter()
            .flat_map(|s| {
                let chars = s.chars().collect::<Vec<_>>();
                chars
                    .chunks(columns)
                    .map(|c| c.iter().collect::<String>())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let count = ((height - 86.) / 22.).max(1.) as usize;
        let end = rows.len().saturating_sub(self.scroll);
        let start = end.saturating_sub(count);
        for (i, row) in rows[start..end].iter().enumerate() {
            ui.courier
                .left(row, 18., 52. + i as f32 * 22., font as f32, ink);
        }
        // Scroll long input horizontally to keep its insertion point visible.
        let start = self.cursor.saturating_sub(columns.saturating_sub(3));
        let visible = &self.input[start..self.input.len().min(start + columns.saturating_sub(2))];
        ui.courier
            .left(&format!("> {visible}"), 18., height - 18., font as f32, ink);
        let x = 18.
            + ui.courier.width(
                &format!("> {}", &self.input[start..self.cursor]),
                font as f32,
            );
        draw_line(x, height - 34., x, height - 15., 1.5, ink);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gameplay_commands_accept_original_spelling_and_validate_arguments() {
        for (text, expected) in [
            ("god", Command::God),
            ("noclip", Command::Noclip),
            ("notarget", Command::Notarget),
            ("wuss", Command::Wuss),
            ("health 100", Command::Health(100.)),
            ("health 250", Command::Health(250.)),
            ("cg_cameradist -45", Command::CameraDistance(Some(-45.))),
            ("cg_cameradist 128", Command::CameraDistance(Some(128.))),
            ("give all", Command::Give("all".into())),
            (
                "give \"MODELS\\W_KNIFE.TIK\"",
                Command::Give("w_knife.tik".into()),
            ),
            ("map pandemonium", Command::Map("pandemonium".into(), None)),
        ] {
            assert_eq!(parse(text), Ok(expected), "{text}");
        }
        for bad in [
            "health NaN",
            "health inf",
            "health -1",
            "health 1000001",
            "health",
            "cg_cameradist NaN",
            "cg_cameradist 2048",
            "give ../w_knife.tik",
            "give w_knife.tik;quit",
            "notarget 1",
            "wuss extra",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn save_commands_accept_only_the_fixed_slots() {
        assert_eq!(parse("save"), Ok(Command::Save));
        assert_eq!(parse("save quick"), Ok(Command::Save));
        assert_eq!(parse("load"), Ok(Command::Load(crate::save::Slot::Quick)));
        assert_eq!(
            parse("load auto"),
            Ok(Command::Load(crate::save::Slot::Auto))
        );
        for bad in [
            "save auto",
            "save ../file",
            "load ../../file",
            "load quick; quit",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn commands_are_explicit_and_never_execute_chained_input() {
        assert_eq!(parse("\\GOD"), Ok(Command::God));
        assert_eq!(
            parse("map skool1$skool1_start2"),
            Ok(Command::Map("skool1".into(), Some("skool1_start2".into())))
        );
        for bad in [
            "god; quit",
            "exec config.cfg",
            "map ../secret",
            "give quest",
            "god extra",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn history_restores_draft_and_text_and_logs_are_bounded() {
        let mut s = Console::default();
        for c in "status".chars() {
            s.insert(c);
        }
        assert_eq!(s.submit(), Some(Ok(Command::Status)));
        s.insert('m');
        s.history(true);
        assert_eq!(s.input, "status");
        s.history(false);
        assert_eq!(s.input, "m");
        for _ in 0..500 {
            s.insert('x');
            s.print("row");
        }
        assert_eq!(s.input.len(), 256);
        assert_eq!(s.lines.len(), 128);
        s.insert('é');
        assert_eq!(s.input.len(), 256);
    }
}
