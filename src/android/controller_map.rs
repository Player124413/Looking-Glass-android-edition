//! Controller setup screen for the Android launcher.
//!
//! Layout: action categories on the left, a list of actions (8 per page), and
//! grouped controller buttons on the right. Select an action, then either press
//! a controller button or tap a button on the right to assign it. Assigning a
//! button that another action already uses swaps the two, exactly like
//! `Preferences::bind` does in the in-game Controls page.
//!
//! Nothing is written to disk until "Save & Close". Back/B/Escape discards the
//! draft. Start is reserved for the menu and cannot be assigned.

use super::draw_text_fit;
use crate::input::PAD_BUTTONS;
use crate::preferences::{Preferences, BINDINGS, PAD_LAYOUT_VERSION};
use macroquad::prelude::*;

const ROWS_PER_PAGE: usize = 8;
const LIST_ROW_PITCH: f32 = 36.;

const PAD_START: u32 = 0x0010;
const PAD_A: u32 = 0x1000;
const PAD_B: u32 = 0x2000;
const PAD_LB: u32 = 0x0100;
const PAD_RB: u32 = 0x0200;
const PAD_D_UP: u32 = 0x0001;
const PAD_D_DOWN: u32 = 0x0002;

/// Pick the display language the same way the Java side does: the launcher
/// stores the chosen language in this environment variable.
pub fn is_ru() -> bool {
    super::i18n::is_ru()
}

/// Pick the English or Russian text.
pub fn tr(ru: bool, en: &'static str, russian: &'static str) -> &'static str {
    if ru {
        russian
    } else {
        en
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Move,
    Combat,
    Toys,
    Actions,
    Other,
}

impl Tab {
    const ALL: [Tab; 5] = [Tab::Move, Tab::Combat, Tab::Toys, Tab::Actions, Tab::Other];

    fn name(self, ru: bool) -> &'static str {
        match self {
            Tab::Move => tr(ru, "Move", "Движение"),
            Tab::Combat => tr(ru, "Combat", "Бой"),
            Tab::Toys => tr(ru, "Toys", "Игрушки"),
            Tab::Actions => tr(ru, "Actions", "Действия"),
            Tab::Other => tr(ru, "Other", "Прочее"),
        }
    }
}

/// Every entry of `preferences::BINDINGS`, keyed by its keyboard key, with its
/// category and Russian name. A test checks that every key appears exactly once.
const ACTIONS: &[(&str, Tab, &str)] = &[
    ("W", Tab::Move, "Вперёд"),
    ("S", Tab::Move, "Назад"),
    ("A", Tab::Move, "Шаг влево"),
    ("D", Tab::Move, "Шаг вправо"),
    ("Space", Tab::Move, "Прыжок / лазанье"),
    ("E", Tab::Move, "Взаимодействие"),
    ("Shift", Tab::Move, "Бег / ходьба"),
    ("Ctrl", Tab::Move, "Нырок / спуск"),
    ("Mouse 1", Tab::Combat, "Основная атака"),
    ("Mouse 2", Tab::Combat, "Вторичная атака"),
    ("Wheel Up", Tab::Toys, "Предыдущая игрушка"),
    ("Wheel Down", Tab::Toys, "Следующая игрушка"),
    ("1", Tab::Toys, "Клинок Ворпал"),
    ("2", Tab::Toys, "Карты"),
    ("3", Tab::Toys, "Молоток"),
    ("4", Tab::Toys, "Джек-бомба"),
    ("5", Tab::Toys, "Ледяная палочка"),
    ("6", Tab::Toys, "Джеки"),
    ("7", Tab::Toys, "Кубики демонов"),
    ("8", Tab::Toys, "Посох глаза"),
    ("9", Tab::Toys, "Мушкетон"),
    ("0", Tab::Toys, "Карманные часы"),
    ("C", Tab::Actions, "Подсказка Чеширского кота"),
    ("R", Tab::Actions, "Устойчивая опора"),
    ("Enter", Tab::Actions, "Повтор / пропуск сцены"),
    ("H", Tab::Actions, "Помощь"),
    ("P", Tab::Actions, "Пауза"),
    ("V", Tab::Actions, "Смена вида"),
    ("I", Tab::Actions, "Инвентарь"),
    ("Tab", Tab::Other, "Выбор уровня"),
    ("F5", Tab::Other, "Быстрое сохранение"),
    ("F9", Tab::Other, "Быстрая загрузка"),
    ("Home", Tab::Other, "Перезапуск уровня"),
    ("M", Tab::Other, "Выключить звук"),
    ("Left", Tab::Other, "Смотреть влево"),
    ("Right", Tab::Other, "Смотреть вправо"),
    ("Up", Tab::Other, "Смотреть вверх"),
    ("Down", Tab::Other, "Смотреть вниз"),
];

/// Diagram groups: English title, Russian title, and buttons in display order.
/// "Start" is shown but reserved for the menu.
const SECTIONS: &[(&str, &str, &[&str])] = &[
    ("Triggers & bumpers", "Курки и бамперы", &["LT", "LB", "RB", "RT"]),
    ("Face buttons", "Кнопки действий", &["Y", "X", "B", "A"]),
    ("D-pad", "Крестовина", &["D Up", "D Left", "D Right", "D Down"]),
    ("Sticks & menu", "Стики и меню", &["L Stick", "R Stick", "Back", "Start"]),
];

fn row_of(key: &str) -> Option<usize> {
    BINDINGS.iter().position(|(_, k)| *k == key)
}

/// Binding rows (indexes into `BINDINGS`) that belong to a tab, in list order.
fn tab_rows(tab: Tab) -> Vec<usize> {
    ACTIONS
        .iter()
        .filter(|(_, t, _)| *t == tab)
        .filter_map(|(key, _, _)| row_of(key))
        .collect()
}

fn tab_of(row: usize) -> Tab {
    let key = BINDINGS[row].1;
    ACTIONS
        .iter()
        .find(|(k, _, _)| *k == key)
        .map_or(Tab::Other, |(_, tab, _)| *tab)
}

fn action_label(row: usize, ru: bool) -> &'static str {
    let (label, key) = BINDINGS[row];
    if !ru {
        return label;
    }
    ACTIONS
        .iter()
        .find(|(k, _, _)| *k == key)
        .map_or(label, |(_, _, name)| *name)
}

/// Swap semantics identical to `Preferences::bind`: the old holder of `name`
/// receives the button previously held by `row`. "None" never swaps.
pub fn assign_pad(draft: &mut [String], row: usize, name: &str) {
    if row >= draft.len() {
        return;
    }
    if name != "None" {
        if let Some(other) = draft.iter().position(|k| k.as_str() == name) {
            let previous = draft[row].clone();
            draft[other] = previous;
        }
    }
    draft[row] = name.to_owned();
}

/// True when every entry is "None" or a unique, known pad button name.
pub fn valid_pad_draft(draft: &[String]) -> bool {
    if draft.len() != BINDINGS.len() {
        return false;
    }
    let mut seen: Vec<&str> = Vec::new();
    for name in draft {
        let name = name.as_str();
        if name == "None" {
            continue;
        }
        if !PAD_BUTTONS.iter().any(|(known, _)| *known == name) || seen.contains(&name) {
            return false;
        }
        seen.push(name);
    }
    true
}

fn pad_bit(name: &str) -> u32 {
    PAD_BUTTONS
        .iter()
        .find(|(known, _)| *known == name)
        .map_or(0, |(_, bit)| *bit)
}

/// Current pad state as bits: the 16 buttons, plus LT/RT as 0x10000/0x20000.
fn pad_mask() -> u32 {
    match super::read_gamepad() {
        Some((buttons, lt, rt, _, _, _, _)) => {
            let mut bits = u32::from(buttons);
            if lt > 30 {
                bits |= 0x10000;
            }
            if rt > 30 {
                bits |= 0x20000;
            }
            bits
        }
        None => 0,
    }
}

/// First button in `bits` that exists in `PAD_BUTTONS`.
fn first_pressed_pad(bits: u32) -> Option<&'static str> {
    PAD_BUTTONS
        .iter()
        .find(|(_, bit)| bits & *bit != 0)
        .map(|(name, _)| *name)
}

fn draw_box(rect: Rect, s: f32, label: &str, fill: u32, border: u32, active: bool) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::from_hex(fill));
    let thickness = if active { 3. * s } else { 1.5 * s };
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, thickness, Color::from_hex(border));
    draw_text_fit(
        label,
        rect.x + 8. * s,
        rect.y + rect.h * 0.5 + 6. * s,
        17. * s,
        rect.w - 16. * s,
        WHITE,
    );
}

pub enum Outcome {
    Continue,
    Saved(String),
    Cancelled,
}

pub struct ControllerEditor {
    draft: Vec<String>,
    tab: Tab,
    page: usize,
    selected: Option<usize>,
    listening: bool,
    prev_mask: u32,
    status: String,
}

impl Default for ControllerEditor {
    fn default() -> Self {
        Self::new(&Preferences::default())
    }
}

impl ControllerEditor {
    fn new(prefs: &Preferences) -> Self {
        let draft = if valid_pad_draft(&prefs.pad_bindings) {
            prefs.pad_bindings.clone()
        } else {
            Preferences::default().pad_bindings
        };
        Self {
            draft,
            tab: Tab::Combat,
            page: 0,
            selected: row_of("Mouse 1"),
            listening: false,
            // Buttons held when the screen opens must not assign anything.
            prev_mask: pad_mask(),
            status: String::new(),
        }
    }

    /// Reset the draft from the saved preferences. Call when the screen opens.
    pub fn start(&mut self, prefs: &Preferences) {
        *self = Self::new(prefs);
    }

    fn assign(&mut self, row: usize, name: &'static str, ru: bool) {
        assign_pad(&mut self.draft, row, name);
        let target = if name == "None" {
            tr(ru, "unbound", "не назначено")
        } else {
            name
        };
        self.status = format!(
            "{} → {} ({})",
            action_label(row, ru),
            target,
            tr(ru, "not saved yet", "ещё не сохранено")
        );
    }

    fn move_selection(&mut self, delta: i32) {
        let rows = tab_rows(self.tab);
        if rows.is_empty() {
            return;
        }
        let current = self
            .selected
            .and_then(|row| rows.iter().position(|&r| r == row))
            .unwrap_or(0) as i32;
        let next = (current + delta).clamp(0, rows.len() as i32 - 1) as usize;
        self.selected = Some(rows[next]);
        self.page = next / ROWS_PER_PAGE;
        self.listening = false;
    }

    fn switch_tab(&mut self, delta: i32) {
        let tabs = Tab::ALL;
        let index = tabs.iter().position(|t| *t == self.tab).unwrap_or(0) as i32;
        let next = (index + delta).rem_euclid(tabs.len() as i32) as usize;
        self.tab = tabs[next];
        self.page = 0;
        self.listening = false;
    }

    fn save(&mut self, prefs: &mut Preferences, ru: bool) -> Outcome {
        let before = prefs.pad_bindings.clone();
        let before_version = prefs.pad_layout_version;
        prefs.pad_bindings = self.draft.clone();
        prefs.pad_layout_version = PAD_LAYOUT_VERSION;
        match prefs.save() {
            Ok(()) => Outcome::Saved(
                tr(ru, "Controller mapping saved.", "Назначения геймпада сохранены.").into(),
            ),
            Err(err) => {
                prefs.pad_bindings = before;
                prefs.pad_layout_version = before_version;
                self.status = format!("{}: {err}", tr(ru, "Could not save", "Не удалось сохранить"));
                Outcome::Continue
            }
        }
    }

    /// Draw one frame and handle taps, keys and controller input.
    pub fn update_and_draw(&mut self, prefs: &mut Preferences) -> Outcome {
        let ru = is_ru();
        let (w, h) = (screen_width().max(320.), screen_height().max(240.));
        // Same scale rule as the launcher, but with a larger bottom margin so the
        // bottom bar always fits on screen.
        let s = (h / 560.)
            .clamp(0.65, 2.4)
            .min(w / 1000.)
            .min((h - 40.).max(200.) / 560.);
        let panel = Rect::new(
            (w - 960. * s).max(20.) * 0.5,
            (h - 560. * s).max(16.) * 0.5,
            (w - 40.).min(960. * s),
            (h - 32.).min(560. * s),
        );
        // Design units (960x560) to screen pixels inside the panel.
        let r = |x: f32, y: f32, rw: f32, rh: f32| {
            Rect::new(panel.x + x * s, panel.y + y * s, rw * s, rh * s)
        };

        clear_background(Color::from_hex(0x141118));
        draw_rectangle(panel.x, panel.y, panel.w, panel.h, Color::from_hex(0x221b29));
        draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 2. * s, Color::from_hex(0x8c6d46));

        let (pointer, pressed, _) = crate::touch::pointer_state();
        let mask = pad_mask();
        let rising = mask & !self.prev_mask;
        self.prev_mask = mask;
        let escape = is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Back);
        let mut outcome = Outcome::Continue;
        let mut save_now = false;

        // Controller input. While listening, every button press is captured.
        if self.listening {
            if escape || rising & PAD_START != 0 {
                self.listening = false;
                self.status = tr(
                    ru,
                    "Stopped. Pick an action to remap it.",
                    "Остановлено. Выберите действие, чтобы назначить кнопку.",
                )
                .into();
            } else if let (Some(row), Some(name)) = (self.selected, first_pressed_pad(rising)) {
                self.assign(row, name, ru);
                self.listening = false;
            }
        } else if escape || rising & PAD_B != 0 {
            outcome = Outcome::Cancelled;
        } else {
            if rising & PAD_D_UP != 0 {
                self.move_selection(-1);
            }
            if rising & PAD_D_DOWN != 0 {
                self.move_selection(1);
            }
            if rising & PAD_LB != 0 {
                self.switch_tab(-1);
            }
            if rising & PAD_RB != 0 {
                self.switch_tab(1);
            }
            if rising & PAD_A != 0 && self.selected.is_some() {
                self.listening = true;
            }
        }

        // Header
        draw_text_fit(
            tr(ru, "Controller Setup", "Настройка геймпада"),
            panel.x + 26. * s,
            panel.y + 50. * s,
            28. * s,
            panel.w - 52. * s,
            Color::from_hex(0xf3e5c8),
        );
        draw_text_fit(
            tr(
                ru,
                "Select an action, then press a button on your controller or tap one on the right.",
                "Выберите действие, затем нажмите кнопку геймпада или нажмите кнопку справа.",
            ),
            panel.x + 26. * s,
            panel.y + 72. * s,
            16. * s,
            panel.w - 52. * s,
            Color::from_hex(0xd8cbb8),
        );

        // Category tabs
        for (i, tab) in Tab::ALL.into_iter().enumerate() {
            let rect = r(26. + i as f32 * 90., 88., 84., 32.);
            let active = self.tab == tab;
            let fill = if active { 0x5c3d6e } else { 0x2a2233 };
            draw_box(rect, s, tab.name(ru), fill, 0x8c6d46, active);
            if pressed && rect.contains(pointer) && !active {
                self.tab = tab;
                self.page = 0;
                self.listening = false;
            }
        }

        // Action list for the current tab, one page at a time
        let rows = tab_rows(self.tab);
        let pages = rows.len().div_ceil(ROWS_PER_PAGE).max(1);
        self.page = self.page.min(pages - 1);
        let page_rows: Vec<usize> = rows
            .iter()
            .copied()
            .skip(self.page * ROWS_PER_PAGE)
            .take(ROWS_PER_PAGE)
            .collect();
        for (j, row) in page_rows.into_iter().enumerate() {
            let rect = r(26., 128. + j as f32 * LIST_ROW_PITCH, 444., 32.);
            let is_selected = self.selected == Some(row);
            let fill = if is_selected { 0x3b2f4a } else { 0x2a2233 };
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::from_hex(fill));
            if is_selected {
                draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2. * s, Color::from_hex(0x5dade2));
            }
            draw_text_fit(
                action_label(row, ru),
                rect.x + 10. * s,
                rect.y + 22. * s,
                18. * s,
                rect.w - 160. * s,
                WHITE,
            );
            let bound = self.draft[row].clone();
            let bound_text = if bound == "None" { "—" } else { bound.as_str() };
            draw_text_fit(
                bound_text,
                rect.x + rect.w - 140. * s,
                rect.y + 22. * s,
                18. * s,
                130. * s,
                Color::from_hex(0xf3e5c8),
            );
            if pressed && rect.contains(pointer) {
                self.selected = Some(row);
                self.listening = true;
            }
        }

        // Paging
        let prev_btn = r(26., 422., 120., 30.);
        let next_btn = r(350., 422., 120., 30.);
        draw_box(prev_btn, s, tr(ru, "< Prev", "< Назад"), 0x2f2638, 0x7c6750, false);
        draw_box(next_btn, s, tr(ru, "Next >", "Далее >"), 0x2f2638, 0x7c6750, false);
        draw_text_fit(
            &format!("{} / {}", self.page + 1, pages),
            panel.x + 160. * s,
            panel.y + 443. * s,
            17. * s,
            180. * s,
            Color::from_hex(0xd8cbb8),
        );
        if pressed && prev_btn.contains(pointer) && self.page > 0 {
            self.page -= 1;
        }
        if pressed && next_btn.contains(pointer) && self.page + 1 < pages {
            self.page += 1;
        }

        // Controller diagram, grouped by button type
        for (si, &(en, ru_title, buttons)) in SECTIONS.iter().enumerate() {
            let y0 = 88. + si as f32 * 72.;
            draw_text_fit(
                tr(ru, en, ru_title),
                panel.x + 490. * s,
                panel.y + (y0 + 12.) * s,
                15. * s,
                440. * s,
                Color::from_hex(0xd8cbb8),
            );
            for (ci, &name) in buttons.iter().enumerate() {
                let rect = r(490. + ci as f32 * 112.5, y0 + 20., 106.5, 40.);
                let owner = self.draft.iter().position(|k| k.as_str() == name);
                let is_reserved = name == "Start";
                let is_target = owner.is_some() && owner == self.selected;
                let held = pad_bit(name) & mask != 0;
                let fill = if is_reserved {
                    0x1a1620
                } else if held {
                    0x2e5a3a
                } else {
                    0x2f2638
                };
                let border = if is_target { 0x5dade2 } else { 0x7c6750 };
                draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::from_hex(fill));
                draw_rectangle_lines(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    if is_target { 3. * s } else { 1.5 * s },
                    Color::from_hex(border),
                );
                draw_text_fit(name, rect.x + 6. * s, rect.y + 17. * s, 16. * s, rect.w - 12. * s, WHITE);
                let (caption, caption_color) = if is_reserved {
                    (tr(ru, "menu", "меню"), 0x8a7d70)
                } else {
                    match owner {
                        Some(row) => (action_label(row, ru), 0xf3e5c8),
                        None => (tr(ru, "unbound", "не назначено"), 0x8a7d70),
                    }
                };
                draw_text_fit(
                    caption,
                    rect.x + 6. * s,
                    rect.y + 34. * s,
                    13. * s,
                    rect.w - 12. * s,
                    Color::from_hex(caption_color),
                );

                if pressed && rect.contains(pointer) {
                    if is_reserved {
                        self.status = tr(
                            ru,
                            "Start always opens the menu and cannot be remapped.",
                            "Start всегда открывает меню и не переназначается.",
                        )
                        .into();
                    } else if let Some(sel) = self.selected {
                        self.assign(sel, name, ru);
                        self.listening = false;
                    } else if let Some(row) = owner {
                        self.selected = Some(row);
                        self.tab = tab_of(row);
                        let index = tab_rows(self.tab)
                            .iter()
                            .position(|&r| r == row)
                            .unwrap_or(0);
                        self.page = index / ROWS_PER_PAGE;
                        self.listening = false;
                    }
                }
            }
        }

        // Selected action and unbind
        let selected_text: String = match self.selected {
            Some(row) => format!("{}: {}", tr(ru, "Selected", "Выбрано"), action_label(row, ru)),
            None => tr(ru, "No action selected", "Действие не выбрано").into(),
        };
        draw_text_fit(
            &selected_text,
            panel.x + 490. * s,
            panel.y + 392. * s,
            17. * s,
            440. * s,
            WHITE,
        );
        let unbind_btn = r(490., 400., 150., 30.);
        draw_box(
            unbind_btn,
            s,
            tr(ru, "Unbind selected", "Снять кнопку"),
            0x4a2c2c,
            0xc07a6a,
            false,
        );
        if pressed && unbind_btn.contains(pointer) {
            if let Some(row) = self.selected {
                self.assign(row, "None", ru);
                self.listening = false;
            }
        }
        if self.listening {
            draw_text_fit(
                tr(
                    ru,
                    "Press a controller button now (Start to stop)…",
                    "Нажмите кнопку геймпада (Start — отмена)…",
                ),
                panel.x + 650. * s,
                panel.y + 420. * s,
                17. * s,
                284. * s,
                Color::from_hex(0x85c1e9),
            );
        }

        // Status line
        let status_text = if self.status.is_empty() {
            tr(
                ru,
                "Start is reserved for the menu. Back/B closes without saving.",
                "Start зарезервирован под меню. Назад/B закрывает без сохранения.",
            )
            .to_owned()
        } else {
            self.status.clone()
        };
        draw_text_fit(
            &status_text,
            panel.x + 26. * s,
            panel.y + 470. * s,
            17. * s,
            panel.w - 52. * s,
            Color::from_hex(0xd8cbb8),
        );

        // Presets and final actions
        let cancel_btn = r(26., 496., 120., 40.);
        let default_btn = r(370., 496., 120., 40.);
        let attack_x_btn = r(498., 496., 130., 40.);
        let attack_rb_btn = r(636., 496., 130., 40.);
        let save_btn = r(774., 496., 160., 40.);
        draw_box(cancel_btn, s, tr(ru, "Cancel", "Отмена"), 0x2f2638, 0x7c6750, false);
        draw_box(default_btn, s, tr(ru, "Default", "Стандарт"), 0x2f2638, 0x7c6750, false);
        draw_box(attack_x_btn, s, tr(ru, "Attack: X", "Атака: X"), 0x2f2638, 0x7c6750, false);
        draw_box(attack_rb_btn, s, tr(ru, "Attack: RB", "Атака: RB"), 0x2f2638, 0x7c6750, false);
        draw_box(save_btn, s, tr(ru, "Save & Close", "Сохранить"), 0x1f618d, 0x85c1e9, false);

        if pressed && cancel_btn.contains(pointer) {
            outcome = Outcome::Cancelled;
        }
        if pressed && default_btn.contains(pointer) {
            self.draft = Preferences::default().pad_bindings;
            self.listening = false;
            self.status = tr(
                ru,
                "Default layout restored (not saved yet).",
                "Стандартная раскладка восстановлена (ещё не сохранено).",
            )
            .into();
        }
        if pressed && attack_x_btn.contains(pointer) {
            if let Some(row) = row_of("Mouse 1") {
                self.assign(row, "X", ru);
            }
            self.listening = false;
        }
        if pressed && attack_rb_btn.contains(pointer) {
            if let Some(row) = row_of("Mouse 1") {
                self.assign(row, "RB", ru);
            }
            self.listening = false;
        }
        if pressed && save_btn.contains(pointer) {
            save_now = true;
        }
        if save_now && matches!(outcome, Outcome::Continue) {
            outcome = self.save(prefs, ru);
        }

        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_draft() -> Vec<String> {
        Preferences::default().pad_bindings
    }

    #[test]
    fn actions_cover_every_binding_once() {
        assert_eq!(ACTIONS.len(), BINDINGS.len());
        for (_, key) in BINDINGS {
            let count = ACTIONS.iter().filter(|(k, _, _)| k == key).count();
            assert_eq!(count, 1, "binding {key} must appear exactly once");
        }
        let total: usize = Tab::ALL.iter().map(|t| tab_rows(*t).len()).sum();
        assert_eq!(total, BINDINGS.len());
    }

    #[test]
    fn section_buttons_are_known_pad_buttons_or_reserved_start() {
        for (_, _, buttons) in SECTIONS {
            for name in buttons.iter() {
                let known = PAD_BUTTONS.iter().any(|(k, _)| k == name);
                assert!(known || *name == "Start", "unknown button {name}");
            }
        }
    }

    #[test]
    fn default_draft_is_valid() {
        assert!(valid_pad_draft(&default_draft()));
    }

    #[test]
    fn assigning_a_used_button_swaps_like_bind() {
        let mut draft = default_draft();
        let attack = row_of("Mouse 1").unwrap();
        let interact = row_of("E").unwrap();
        let attack_before = draft[attack].clone();
        let interact_before = draft[interact].clone();
        assign_pad(&mut draft, attack, &interact_before);
        assert_eq!(draft[attack], interact_before);
        assert_eq!(draft[interact], attack_before);
        assert!(valid_pad_draft(&draft));
    }

    #[test]
    fn unbinding_frees_a_button_without_swapping() {
        let mut draft = default_draft();
        let attack = row_of("Mouse 1").unwrap();
        let old = draft[attack].clone();
        assign_pad(&mut draft, attack, "None");
        assert_eq!(draft[attack], "None");
        assert_eq!(draft.iter().filter(|k| **k == old).count(), 0);
        assert!(valid_pad_draft(&draft));
    }

    #[test]
    fn reassigning_same_button_is_a_no_op() {
        let mut draft = default_draft();
        let before = draft.clone();
        let attack = row_of("Mouse 1").unwrap();
        let current = draft[attack].clone();
        assign_pad(&mut draft, attack, &current);
        assert_eq!(draft, before);
    }

    #[test]
    fn presets_stay_valid() {
        let mut draft = default_draft();
        assign_pad(&mut draft, row_of("Mouse 1").unwrap(), "X");
        assert!(valid_pad_draft(&draft));
        let mut draft = default_draft();
        assign_pad(&mut draft, row_of("Mouse 1").unwrap(), "RB");
        assert!(valid_pad_draft(&draft));
    }

    #[test]
    fn start_cannot_be_assigned_by_validation() {
        let mut draft = default_draft();
        draft[row_of("Mouse 1").unwrap()] = "Start".to_owned();
        assert!(!valid_pad_draft(&draft));
    }
}
