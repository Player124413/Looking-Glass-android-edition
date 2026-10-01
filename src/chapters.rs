//! Chapter chooser: stable mouse rows, bounded wheel scrolling and keyboard selection.
use crate::{campaign::LevelChoice, powerups::Difficulty, ui};
use macroquad::prelude::*;

const VISIBLE: usize = 11;
const LIST: Rect = Rect::new(24., 98., 580., 297.);
const DIFFICULTY: Rect = Rect::new(36., 62., 568., 24.);
const BEGIN: Rect = Rect::new(62., 429., 224., 28.);
const BACK: Rect = Rect::new(354., 429., 224., 28.);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    Visit(usize),
    Begin,
    Back,
    Difficulty,
}

#[derive(Default)]
pub struct Chapters {
    pub selected: usize,
    top: usize,
    wheel: f32,
    last_click: Option<(usize, f64, Vec2)>,
}

impl Chapters {
    /// Shared keyboard, controller and pointer navigation for both menu contexts.
    pub fn update(
        &mut self,
        input: &crate::input::Input,
        count: usize,
        difficulty: &mut Difficulty,
    ) -> Option<Hit> {
        if count == 0 || !crate::look::window_focused() { return None; }
        if input.ui(KeyCode::Down) { self.select((self.selected + 1) % count, count); }
        if input.ui(KeyCode::Up) { self.select((self.selected + count - 1) % count, count); }
        if input.ui(KeyCode::Home) { self.select(0, count); }
        if input.ui(KeyCode::End) { self.select(count - 1, count); }
        // Miniquad exposes raw Windows WHEEL_DELTA units, not notches.
        let wheel_unit = if cfg!(target_os = "windows") { 120. } else { 1. };
        self.scroll(mouse_wheel().1 / wheel_unit, count);
        let hit = if is_mouse_button_pressed(MouseButton::Left) {
            let canvas = ui::Canvas::new(screen_width(), screen_height());
            self.click(canvas.pointer(Vec2::from(mouse_position())), get_time(), count)
        } else { None };
        if is_key_pressed(KeyCode::D) || input.pad_pressed("Y") || hit == Some(Hit::Difficulty) {
            *difficulty = Difficulty::ALL[(*difficulty as usize + 1) % 4];
        }
        hit
    }

    pub fn open(&mut self, selected: usize, count: usize) {
        self.selected = selected.min(count.saturating_sub(1));
        self.top = self
            .selected
            .saturating_sub(VISIBLE / 2)
            .min(count.saturating_sub(VISIBLE));
        self.wheel = 0.;
        self.last_click = None;
    }

    pub fn select(&mut self, selected: usize, count: usize) {
        self.selected = selected.min(count.saturating_sub(1));
        // Reveal keyboard selection without shifting every mouse-selected row.
        self.top = self.top.min(self.selected);
        if self.selected >= self.top + VISIBLE {
            self.top = self.selected + 1 - VISIBLE;
        }
        self.top = self.top.min(count.saturating_sub(VISIBLE));
        self.last_click = None;
        self.wheel = 0.;
    }

    pub fn scroll(&mut self, delta: f32, count: usize) {
        if delta == 0. || !delta.is_finite() || count == 0 {
            return;
        }
        self.last_click = None;
        self.wheel -= delta * 3.;
        let rows = self.wheel.trunc();
        self.wheel -= rows;
        self.top =
            (self.top as f32 + rows).clamp(0., count.saturating_sub(VISIBLE) as f32) as usize;
        // The Begin button must always refer to a visibly selected chapter.
        self.selected = self
            .selected
            .clamp(self.top, (self.top + VISIBLE - 1).min(count - 1));
    }

    pub fn hit(&self, pointer: Vec2, count: usize) -> Option<Hit> {
        if LIST.contains(pointer) {
            let index = self.top + ((pointer.y - LIST.y) / 27.) as usize;
            return (index < count && index < self.top + VISIBLE).then_some(Hit::Visit(index));
        }
        [
            (DIFFICULTY, Hit::Difficulty),
            (BEGIN, Hit::Begin),
            (BACK, Hit::Back),
        ]
        .into_iter()
        .find_map(|(rect, hit)| rect.contains(pointer).then_some(hit))
    }

    pub fn click(&mut self, pointer: Vec2, now: f64, count: usize) -> Option<Hit> {
        let hit = self.hit(pointer, count);
        let previous = self.last_click.take();
        if let Some(Hit::Visit(index)) = hit {
            self.selected = index;
            if previous.is_some_and(|(last, time, position)| {
                index == last
                    && (0. ..=0.4).contains(&(now - time))
                    && pointer.distance_squared(position) <= 36.
            }) {
                return Some(Hit::Begin);
            }
            self.last_click = Some((index, now, pointer));
            None
        } else {
            hit
        }
    }

    pub fn draw(
        &self,
        ui: &ui::Ui,
        choices: &[LevelChoice],
        maps: &[String],
        difficulty: Difficulty,
        interactive: bool,
        pad: bool,
    ) {
        let canvas = ui::Canvas::new(screen_width(), screen_height());
        let hover = interactive
            .then(|| self.hit(canvas.pointer(Vec2::from(mouse_position())), choices.len()))
            .flatten();
        ui.panel(canvas.rect(Rect::new(16., 16., 608., 448.)));
        ui.font.draw(
            canvas,
            "Chapters of Wonderland",
            Rect::new(36., 28., 568., 34.),
            30.,
            ui::INK,
        );
        ui.font.draw(
            canvas,
            &format!(
                "{} - {} visits   ({} to change)",
                difficulty.name(),
                choices.len(),
                if pad { "Y" } else { "D / click" }
            ),
            DIFFICULTY,
            20.,
            if hover == Some(Hit::Difficulty) {
                ui::SELECTED
            } else {
                ui::INK
            },
        );
        for (row, index) in (self.top..(self.top + VISIBLE).min(choices.len())).enumerate() {
            let y = LIST.y + row as f32 * 27.;
            let choice = &choices[index];
            if index == self.selected {
                ui.marker(canvas.rect(Rect::new(24., y + 5., 16., 16.)));
            }
            ui.font.draw_left(
                canvas,
                &format!("{:02}  {}", index + 1, choice.title),
                Rect::new(43., y + 1., 431., 25.),
                21.,
                if index == self.selected || hover == Some(Hit::Visit(index)) {
                    ui::SELECTED
                } else {
                    ui::INK
                },
            );
            ui.courier.draw_left(
                canvas,
                &maps[choice.map],
                Rect::new(488., y + 3., 109., 22.),
                14.,
                ui::INK,
            );
        }
        ui.font.draw(
            canvas,
            if pad {
                "D-pad / left stick: browse   A: begin   B: return"
            } else {
                "Click: select   Double-click: begin   Wheel / arrows: browse"
            },
            Rect::new(30., 402., 580., 21.),
            17.,
            ui::INK,
        );
        for (rect, hit, title) in [
            (
                BEGIN,
                Hit::Begin,
                if pad { "Begin / A" } else { "Begin / Enter" },
            ),
            (
                BACK,
                Hit::Back,
                if pad {
                    "Return / B"
                } else {
                    "Return / Tab / Esc"
                },
            ),
        ] {
            ui.font.draw(
                canvas,
                title,
                rect,
                22.,
                if hover == Some(hit) {
                    ui::SELECTED
                } else {
                    ui::INK
                },
            );
            if hover == Some(hit) {
                let text_left = rect.x + (rect.w - ui.font.width(title, 22.).min(rect.w)) * 0.5;
                ui.marker(canvas.rect(Rect::new(text_left - 20., rect.y + 6., 16., 16.)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicks_do_not_recentre_rows_and_double_click_requires_same_visit() {
        let mut menu = Chapters::default();
        menu.open(0, 39);
        let point = vec2(300., 300.); // school return, row 8
        assert_eq!(menu.click(point, 1., 39), None);
        assert_eq!(menu.selected, 7);
        assert_eq!(menu.hit(point, 39), Some(Hit::Visit(7)));
        assert_eq!(menu.click(point, 1.2, 39), Some(Hit::Begin));
        assert_eq!(menu.click(point, 2., 39), None);
        assert_eq!(menu.click(point - Vec2::Y * 27., 2.1, 39), None);
        menu.scroll(-1., 39);
        assert_eq!(menu.click(point, 2.2, 39), None);
    }

    #[test]
    fn wheel_keeps_selection_visible_and_clamps_at_both_ends() {
        let mut menu = Chapters::default();
        menu.scroll(-100., 39);
        assert_eq!((menu.top, menu.selected), (28, 28));
        menu.select(38, 39);
        menu.scroll(100., 39);
        assert_eq!((menu.top, menu.selected), (0, 10));
        menu.select(38, 39);
        assert_eq!(menu.top, 28);
        menu.select(0, 39);
        assert_eq!(menu.top, 0);
        menu.open(0, 3);
        menu.scroll(-100., 3);
        assert_eq!(menu.top, 0);
        assert_eq!(menu.hit(vec2(100., 240.), 3), None);
    }

    #[test]
    fn small_wheel_events_accumulate_without_forcing_whole_pages() {
        let mut menu = Chapters::default();
        menu.scroll(-0.125, 39);
        menu.scroll(-0.125, 39);
        assert_eq!(menu.top, 0);
        menu.scroll(-0.125, 39);
        assert_eq!(menu.top, 1);
    }

    #[test]
    fn scaled_mouse_hits_match_drawn_rows_and_ignore_letterbox() {
        let menu = Chapters::default();
        for (w, h) in [
            (800., 600.),
            (1280., 720.),
            (1920., 1080.),
            (2560., 1440.),
            (3840., 2160.),
        ] {
            let canvas = ui::Canvas::new(w, h);
            let row = canvas.rect(Rect::new(300., 300., 1., 1.));
            assert_eq!(
                menu.hit(canvas.pointer(vec2(row.x, row.y)), 39),
                Some(Hit::Visit(7))
            );
            let begin = canvas.rect(BEGIN);
            assert_eq!(
                menu.hit(canvas.pointer(begin.center()), 39),
                Some(Hit::Begin)
            );
            assert_eq!(menu.hit(canvas.pointer(vec2(0., h * 0.5)), 39), None);
        }
    }
}
