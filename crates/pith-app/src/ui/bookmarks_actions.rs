//! Нижние кнопки панели отрезков: нарезка, очистка и ход работы.
//!
//! Отдельно от списка закладок: у кнопок своя разметка в две-три строки
//! и свои проверки — есть ли FFmpeg, есть ли что резать.

use crate::app::PithApp;
use crate::theme;
use crate::tr;
use crate::ui::bookmarks::{BUTTON_GAP, BUTTON_HEIGHT, BUTTON_RADIUS, PanelActions};
use crate::ui::{icons, panel_head};

/// Ширина кнопки-значка: выгрузка и очистка.
///
/// Обе — про весь список, но рядом с нарезкой должны быть тише её:
/// значок вместо подписи оставляет главному действию всю ширину панели.
const ICON_BUTTON: f32 = 38.0;

pub fn show(app: &PithApp, ui: &mut egui::Ui, actions: &mut PanelActions) {
    // Выгрузка, спрятанная вместе с окном, идёт дальше — и её ход должен
    // быть виден там, где человек сейчас.
    if let Some((done, total, sounding)) = app.hidden_export_progress() {
        show_hidden_export(ui, done, total, sounding);
        return;
    }

    if let Some(progress) = app.extraction_progress() {
        ui.label(
            egui::RichText::new(tr!(
                format!("Нарезка: {} из {}", progress.done, progress.total),
                format!("Cutting: {} of {}", progress.done, progress.total)
            ))
            .color(theme::ACCENT),
        );
        ui.add(egui::ProgressBar::new(progress.fraction()).show_percentage());
        return;
    }

    let Some(video) = app.current_bookmarks() else {
        return;
    };

    let active_count = video.active().map(|l| l.bookmarks.len()).unwrap_or(0);
    let total: usize = video.lists.iter().map(|l| l.bookmarks.len()).sum();

    if total == 0 {
        return;
    }

    let can_extract = app.can_extract();

    panel_head::style_boxes(ui);

    if active_count > 0 {
        show_main_row(ui, active_count, can_extract, actions);
    }

    // Все списки — только когда их больше одного: иначе кнопка повторяет
    // соседнюю и лишь путает.
    if video.lists.len() > 1 {
        ui.add_space(BUTTON_GAP);

        let all = egui::Button::new(
            egui::RichText::new(tr!(
                format!("Вырезать все списки ({total})"),
                format!("Cut all lists ({total})")
            ))
            .color(theme::TEXT_PRIMARY),
        )
        .min_size(egui::vec2(0.0, BUTTON_HEIGHT));

        actions.extract_all |= ui
            .add_enabled(can_extract, all)
            .on_hover_text(tr!(
                "Каждый список — в свою подпапку",
                "Each list into its own subfolder"
            ))
            .on_disabled_hover_text(tr!(
                "Нужен ffmpeg.exe рядом с плеером",
                "ffmpeg.exe must sit next to the player"
            ))
            .clicked();
    }
}

/// Ход выгрузки, у которой спрятали окно.
fn show_hidden_export(ui: &mut egui::Ui, done: usize, total: usize, sounding: bool) {
    let title = if sounding {
        tr!(
            format!("Транскрипция: {done} из {total} слов"),
            format!("Transcription: {done} of {total} words")
        )
    } else {
        tr!(
            format!("Выгрузка в Notion: {done} из {total}"),
            format!("Export to Notion: {done} of {total}")
        )
    };

    ui.label(egui::RichText::new(title).color(theme::ACCENT));

    let fraction = if total == 0 {
        0.0
    } else {
        done as f32 / total as f32
    };

    ui.add(egui::ProgressBar::new(fraction).show_percentage());
}

/// Главная строка: нарезка активного списка, выгрузка и очистка.
///
/// Порядок задан пользователем: сначала нарезка, за ней Notion, за ним
/// очистка. Два последних — значками: подписи рядом с главной кнопкой
/// съедали ширину и спорили с ней за внимание.
fn show_main_row(
    ui: &mut egui::Ui,
    active_count: usize,
    can_extract: bool,
    actions: &mut PanelActions,
) {
    ui.horizontal(|ui| {
        let icons_width = 2.0 * (ICON_BUTTON + ui.spacing().item_spacing.x);

        // Главное действие панели — единственная заметная кнопка:
        // остальное рядом с ней должно выглядеть тише.
        let extract = egui::Button::new(
            egui::RichText::new(tr!(
                format!("Вырезать отрезки ({active_count})"),
                format!("Cut fragments ({active_count})")
            ))
            .color(theme::PANEL_CARD)
            .strong(),
        )
        .fill(theme::PANEL_ACCENT)
        .corner_radius(BUTTON_RADIUS)
        .min_size(egui::vec2(
            (ui.available_width() - icons_width).max(0.0),
            BUTTON_HEIGHT,
        ));

        actions.extract_active |= ui
            .add_enabled(can_extract, extract)
            .on_disabled_hover_text(tr!(
                "Нужен ffmpeg.exe рядом с плеером",
                "ffmpeg.exe must sit next to the player"
            ))
            .clicked();

        // Notion не подключён — кнопка не прячется: нажатие откроет окно
        // интеграций, и станет видно, чего не хватает.
        actions.export |= icon_button(ui, icons::UPLOAD, theme::TEXT_PRIMARY)
            .on_hover_text(tr!(
                format!("Выгрузить в Notion ({active_count})"),
                format!("Export to Notion ({active_count})")
            ))
            .clicked();

        actions.clear |= icon_button(ui, icons::CLEAR, theme::PANEL_MUTED)
            .on_hover_text(tr!(
                "Убрать все закладки этого списка",
                "Remove every bookmark of this list"
            ))
            .clicked();
    });
}

/// Кнопка-значок постоянной ширины.
fn icon_button(ui: &mut egui::Ui, icon: icons::Icon, color: egui::Color32) -> egui::Response {
    ui.add(
        egui::Button::new(icon.text().color(color))
            .corner_radius(BUTTON_RADIUS)
            .min_size(egui::vec2(ICON_BUTTON, BUTTON_HEIGHT)),
    )
}
