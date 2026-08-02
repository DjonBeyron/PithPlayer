//! Тёмная тема. Палитра снята с v4 (PLAN.md §6.11).
//!
//! Все цвета интерфейса берутся отсюда — «магические» значения по коду
//! запрещены (CLAUDE.md).

// Палитра перенесена из v4 целиком и наполняется по мере переноса экранов:
// часть цветов ждёт своих панелей на этапах 3–5. Это справочник, а не
// мёртвый код — иначе пришлось бы каждый раз лезть в исходники v4.
#![allow(dead_code)]

use egui::Color32;

/// Фон панелей и области содержимого.
pub const PANEL_BG: Color32 = Color32::from_rgb(0x1C, 0x1C, 0x1C);
/// Фон окон и диалогов.
pub const WINDOW_BG: Color32 = Color32::from_rgb(45, 45, 45);
/// Поля ввода.
pub const INPUT_BG: Color32 = Color32::from_rgb(60, 60, 60);
/// Кнопки и разделители.
pub const CONTROL: Color32 = Color32::from_rgb(70, 70, 70);
/// Тёмные зоны под видео.
pub const DARK_BG: Color32 = Color32::from_rgb(30, 30, 30);

/// Основной текст.
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(255, 255, 255);
/// Вторичный текст и подписи.
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(200, 200, 200);
/// Неактивные элементы.
pub const TEXT_DISABLED: Color32 = Color32::from_rgb(100, 100, 100);

/// Акцент: выделение, активные элементы, пройденная часть полосы.
pub const ACCENT: Color32 = Color32::from_rgb(100, 200, 255);
/// Незаполненная часть полосы перемотки.
pub const TIMELINE_TRACK: Color32 = Color32::from_rgb(90, 90, 90);
/// Отрезок, который попадёт в сохранённый фрагмент.
pub const FRAGMENT: Color32 = Color32::from_rgb(255, 205, 60);
/// Метка закладки на полосе.
pub const BOOKMARK: Color32 = Color32::from_rgb(255, 235, 130);

/// Текст субтитров.
pub const SUBTITLE_TEXT: Color32 = Color32::from_rgb(255, 255, 255);
/// Подложка под субтитрами: полупрозрачная, чтобы текст читался на любом фоне.
pub const SUBTITLE_BG: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 150);
/// Успешное завершение.
pub const SUCCESS: Color32 = Color32::from_rgb(100, 200, 100);
/// Ошибка.
pub const ERROR: Color32 = Color32::from_rgb(220, 60, 60);

/// Применяет тему к контексту egui.
pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = PANEL_BG;
    visuals.window_fill = WINDOW_BG;
    visuals.extreme_bg_color = DARK_BG;
    visuals.override_text_color = Some(TEXT_PRIMARY);

    visuals.widgets.inactive.bg_fill = CONTROL;
    visuals.widgets.hovered.bg_fill = INPUT_BG;
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.6);

    ctx.set_visuals(visuals);
}
