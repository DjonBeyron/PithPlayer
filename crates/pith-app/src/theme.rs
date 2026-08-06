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
pub const PANEL_BG: Color32 = Color32::from_rgb(0x13, 0x13, 0x13);
/// Фон окон и диалогов.
pub const WINDOW_BG: Color32 = Color32::from_rgb(30, 30, 30);
/// Поля ввода.
pub const INPUT_BG: Color32 = Color32::from_rgb(42, 42, 42);
/// Кнопки и разделители.
pub const CONTROL: Color32 = Color32::from_rgb(50, 50, 50);
/// Подсветка строки под курсором.
pub const HIGHLIGHT: Color32 = Color32::from_rgb(42, 54, 68);
/// Тёмные зоны под видео.
pub const DARK_BG: Color32 = Color32::from_rgb(20, 20, 20);

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

// ===== Панель отрезков =====
//
// У неё своя, более глубокая гамма: панель занимает всю высоту окна и
// стоит вплотную к кадру, а серый цвет остальных панелей рядом с видео
// выглядит грязным пятном.

/// Подложка панели.
pub const PANEL_CARD: Color32 = Color32::from_rgb(0x10, 0x12, 0x17);
/// Плашки внутри панели: переключатель списка, кнопки.
pub const PANEL_ELEMENT: Color32 = Color32::from_rgb(0x1A, 0x1D, 0x23);
/// Плашка под курсором.
pub const PANEL_ELEMENT_HOVER: Color32 = Color32::from_rgb(0x22, 0x26, 0x2E);
/// Обводка плашек и разделители.
pub const PANEL_BORDER: Color32 = Color32::from_rgb(0x26, 0x2A, 0x32);
/// Углубление: подсказка о пустом списке.
pub const PANEL_SUNKEN: Color32 = Color32::from_rgb(0x0C, 0x0E, 0x12);
/// Приглушённые подписи панели.
pub const PANEL_MUTED: Color32 = Color32::from_rgb(0xA1, 0xA5, 0xB0);
/// Числа и выделения в панели.
pub const PANEL_ACCENT: Color32 = Color32::from_rgb(0xFA, 0xCC, 0x15);

/// Цвета списков отрезков.
///
/// Каждому списку — свой цвет, чтобы на полосе было видно принадлежность
/// метки (PLAN.md §6.5). Первый — привычный жёлтый из v4: у большинства
/// видео список ровно один, и цвет полосы не должен измениться.
const LIST_COLORS: [Color32; 6] = [
    FRAGMENT,
    Color32::from_rgb(90, 215, 200),
    Color32::from_rgb(185, 150, 255),
    Color32::from_rgb(255, 140, 90),
    Color32::from_rgb(140, 220, 120),
    Color32::from_rgb(255, 130, 180),
];

/// Цвет списка по его порядковому номеру. Дальше цвета повторяются.
pub fn list_color(index: usize) -> Color32 {
    LIST_COLORS[index % LIST_COLORS.len()]
}

// ===== Окна настроек =====
//
// Своя гамма: тёмная подложка, карточки чуть светлее её, синий акцент —
// тот же, каким налита пройденная часть полосы перемотки. Серые кнопки
// остального интерфейса в окне настроек сливались в кашу: глазу не за что
// зацепиться, а решения там принимают осознанно.

/// Подложка окна настроек.
pub const DIALOG_BG: Color32 = Color32::from_rgb(0x0F, 0x0F, 0x10);
/// Карточка с группой настроек.
pub const DIALOG_CARD: Color32 = Color32::from_rgb(0x18, 0x19, 0x1B);
/// Поля ввода и неактивные переключатели.
pub const DIALOG_FIELD: Color32 = Color32::from_rgb(0x27, 0x28, 0x2B);
/// Обводка карточек, полей и разделители.
pub const DIALOG_BORDER: Color32 = Color32::from_rgb(0x27, 0x28, 0x2B);
/// Обводка кнопок и переключателей.
pub const DIALOG_OUTLINE: Color32 = Color32::from_rgb(0x3A, 0x4A, 0x58);
/// Заголовки и основной текст окна.
pub const DIALOG_TEXT: Color32 = Color32::from_rgb(0xE5, 0xE2, 0xE1);
/// Подписи полей.
pub const DIALOG_LABEL: Color32 = Color32::from_rgb(0xBC, 0xC8, 0xD2);
/// Пояснения под настройками.
pub const DIALOG_MUTED: Color32 = Color32::from_rgb(0x8C, 0x92, 0x99);
/// Акцент: текст кнопки с обводкой.
pub const DIALOG_ACCENT: Color32 = ACCENT;
/// Заливка главной кнопки и включённого переключателя.
pub const DIALOG_ACCENT_FILL: Color32 = ACCENT;
/// Текст на акцентной заливке.
pub const DIALOG_ON_ACCENT: Color32 = Color32::from_rgb(0x0A, 0x20, 0x2E);
/// Кружок переключателя.
pub const DIALOG_KNOB: Color32 = Color32::from_rgb(0xC8, 0xC6, 0xC5);

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

    // Кнопки берут фон из `weak_bg_fill`, а не из `bg_fill`. Без этого
    // строки списков и кнопки без рамки никак не откликались на наведение.
    visuals.widgets.inactive.weak_bg_fill = CONTROL;
    visuals.widgets.hovered.weak_bg_fill = HIGHLIGHT;
    visuals.widgets.active.weak_bg_fill = ACCENT.gamma_multiply(0.35);

    ctx.set_visuals(visuals);
}
