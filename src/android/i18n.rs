//! Launcher translations (English is the source text; Russian is used when the
//! launcher language is Russian) and the Cyrillic-capable font for the launcher.
//!
//! The launcher language comes from `LOOKING_GLASS_ANDROID_LANG`, which the Java
//! side sets to `ru` or `en` (see `MainActivity.isRussianLocale`).
//!
//! The built-in macroquad font only has ASCII glyphs, so Russian text needs a
//! system font. Android ships `Roboto-Regular.ttf` (Cyrillic included) on every
//! release. If none of the candidates can be read, Russian text falls back to
//! the built-in font and may show boxes.

use macroquad::text::{load_ttf_font_from_bytes, Font};
use std::cell::{Cell, RefCell};

/// True when the launcher should show Russian text.
pub fn is_ru() -> bool {
    std::env::var("LOOKING_GLASS_ANDROID_LANG").is_ok_and(|lang| lang == "ru")
}

/// Whole-label translations, matched exactly.
const EXACT: &[(&str, &str)] = &[
    ("Looking Glass - Android Launcher", "Looking Glass — лаунчер для Android"),
    ("Port by Player1444 (TG)", "Telegram: Player1444"),
    ("Check Updates", "Обновления"),
    ("Credits", "Авторы"),
    ("Controller Setup", "Настройка геймпада"),
    (
        "Tap 'Choose Game Folder' or 'Select PK3 / ZIP' to import your game, then 'START GAME'.",
        "Нажмите «Выбрать папку игры» или «Выбрать PK3 / ZIP», чтобы импортировать игру, затем «ЗАПУСТИТЬ ИГРУ».",
    ),
    (
        "Needs: pak0, pak1_large, pak2, pak3, pak4_english, pak5_mod (.pk3)",
        "Нужны: pak0, pak1_large, pak2, pak3, pak4_english, pak5_mod (.pk3)",
    ),
    ("FPS Limit:", "Лимит FPS:"),
    ("Graphics:", "Графика:"),
    ("Controls:", "Управление:"),
    ("Unlimited", "Без лимита"),
    ("Auto", "Авто"),
    ("Quality", "Качество"),
    ("Balanced", "Баланс"),
    ("Performance", "Производительность"),
    ("Always On", "Всегда вкл."),
    ("Off", "Выкл."),
    ("Touch: Auto", "Касания: авто"),
    ("Touch: ON", "Касания: вкл."),
    ("Touch: OFF (Pad)", "Касания: выкл. (геймпад)"),
    ("Edit Touch HUD", "Кнопки на экране"),
    ("Choose Game Folder", "Выбрать папку игры"),
    ("Select PK3 / ZIP", "Выбрать PK3 / ZIP"),
    ("START GAME", "ЗАПУСТИТЬ ИГРУ"),
    (".. Parent", ".. Наверх"),
    ("Down >", "Вниз >"),
    ("Copy From This Folder", "Копировать из этой папки"),
    ("Back", "Назад"),
    ("Opening https://t.me/player1444ports ...", "Открываю https://t.me/player1444ports ..."),
    ("Checking for updates on GitHub...", "Проверяю обновления на GitHub..."),
    (
        "Opening Android system folder chooser...",
        "Открываю системный выбор папки...",
    ),
    (
        "Opening Android system file chooser (.pk3 / .zip)...",
        "Открываю системный выбор файлов (.pk3 / .zip)...",
    ),
    (
        "Touch controls DISABLED (Gamepad mode active).",
        "Сенсорное управление ОТКЛЮЧЕНО (активен режим геймпада).",
    ),
];

/// Fragments inside longer messages (applied in order, so longer phrases come first).
const PARTS: &[(&str, &str)] = &[
    (
        "Use Choose Game Folder or Select PK3 / ZIP.",
        "Используйте «Выбрать папку игры» или «Выбрать PK3 / ZIP».",
    ),
    (
        "Select your game folder or PK3 archive to copy into ",
        "Выберите папку игры или PK3-архив для копирования в ",
    ),
    (
        "Game data ready in ",
        "Данные игры готовы: ",
    ),
    (
        ". Select FPS / Preset and tap START GAME.",
        ". Выберите FPS / графику и нажмите «ЗАПУСТИТЬ ИГРУ».",
    ),
    ("Import finished. ", "Импорт завершён. "),
    ("Missing game files: ", "Не найдены файлы игры: "),
    ("Cannot open game files in ", "Не удалось открыть файлы игры в "),
    ("Current Folder: ", "Текущая папка: "),
    ("Game files folder: ", "Папка файлов игры: "),
    ("[PK3 FOUND]", "[НАЙДЕНЫ PK3]"),
    ("[DIR] ", "[ПАПКА] "),
    ("Copied ", "Скопировано "),
    (" archive(s). ", " архив(ов). "),
    ("Unsupported file: ", "Неподдерживаемый файл: "),
    ("Invalid PK3 file name", "Недопустимое имя PK3-файла"),
    ("does not contain any .pk3 files", "не содержит PK3-файлов"),
    (
        "Expected a folder, .pk3 file, or .zip archive",
        "Ожидалась папка, файл .pk3 или архив .zip",
    ),
    ("Folder does not exist: ", "Папка не найдена: "),
    ("No .pk3 archives found in Download / Documents / LookingGlass folders", "PK3-архивы не найдены в папках Download / Documents / LookingGlass"),
    ("No .pk3 archives found in ", "В папке нет PK3-архивов: "),
];

/// Translate a launcher label. Returns the English text unchanged when the
/// launcher language is English or when no translation is known.
pub fn localize(en: &str) -> String {
    if !is_ru() {
        return en.to_owned();
    }
    if let Some((_, ru)) = EXACT.iter().find(|(source, _)| *source == en) {
        return (*ru).to_owned();
    }
    let mut out = en.to_owned();
    for (source, ru) in PARTS {
        if out.contains(*source) {
            out = out.replace(*source, *ru);
        }
    }
    out
}

/// Pick the English or Russian version of a formatted message.
pub fn pick(en: String, ru: String) -> String {
    if is_ru() {
        ru
    } else {
        en
    }
}

/// System font paths that have Cyrillic glyphs, most preferred first.
const FONT_CANDIDATES: &[&str] = &[
    "/system/fonts/Roboto-Regular.ttf",
    "/system/fonts/NotoSans-Regular.ttf",
    "/system/fonts/DroidSans.ttf",
];

thread_local! {
    static LAUNCHER_FONT: RefCell<Option<Font>> = const { RefCell::new(None) };
    static FONT_TRIED: Cell<bool> = const { Cell::new(false) };
}

/// Load the Cyrillic system font once. Call from the launcher, after the
/// macroquad context exists.
pub fn ensure_launcher_font() {
    if FONT_TRIED.with(|tried| tried.replace(true)) {
        return;
    }
    for path in FONT_CANDIDATES {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        if let Ok(font) = load_ttf_font_from_bytes(&bytes) {
            LAUNCHER_FONT.with(|slot| *slot.borrow_mut() = Some(font));
            return;
        }
    }
}

/// Font to use for launcher text: the Cyrillic system font in Russian mode,
/// otherwise `None` (the built-in font).
pub fn launcher_font() -> Option<Font> {
    if !is_ru() {
        return None;
    }
    LAUNCHER_FONT.with(|slot| slot.borrow().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_and_parts_keys_are_unique_and_non_empty() {
        for (source, ru) in EXACT.iter().chain(PARTS.iter()) {
            assert!(!source.is_empty());
            assert!(!ru.is_empty());
        }
        for (i, (source, _)) in EXACT.iter().enumerate() {
            assert!(
                EXACT.iter().skip(i + 1).all(|(other, _)| other != source),
                "duplicate exact key {source}"
            );
        }
    }
}
