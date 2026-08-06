//! Нижние кнопки панели отрезков: нарезка, очистка и ход работы.
//!
//! Отдельно от списка закладок: у кнопок своя разметка в две-три строки
//! и свои проверки — есть ли FFmpeg, есть ли что резать.

use crate::app::PithApp;
use crate::theme;
use crate::tr;
use crate::ui::bookmarks::{BUTTON_HEIGHT, BUTTON_RADIUS, PanelActions};
use crate::ui::panel_head;

pub fn show(app: &PithApp, ui: &mut egui::Ui, actions: &mut PanelActions) {
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
        ui.horizontal(|ui| {
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
            .min_size(egui::vec2(0.0, BUTTON_HEIGHT));

            actions.extract_active |= ui
                .add_enabled(can_extract, extract)
                .on_disabled_hover_text(tr!(
                    "Нужен ffmpeg.exe рядом с плеером",
                    "ffmpeg.exe must sit next to the player"
                ))
                .clicked();

            let clear = egui::Button::new(
                egui::RichText::new(tr!("Очистить", "Clear")).color(theme::TEXT_PRIMARY),
            )
            .min_size(egui::vec2(0.0, BUTTON_HEIGHT));

            actions.clear |= ui
                .add(clear)
                .on_hover_text(tr!(
                    "Убрать все закладки этого списка",
                    "Remove every bookmark of this list"
                ))
                .clicked();
        });
    }

    // Все списки — только когда их больше одного: иначе кнопка повторяет
    // соседнюю и лишь путает.
    if video.lists.len() > 1 {
        ui.add_space(6.0);

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
