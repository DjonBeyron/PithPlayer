//! Отчёт и отказ окна выгрузки.
//!
//! Отдельно от самого окна: у отчёта своя разметка и свои оговорки —
//! номера созданных записей и записи без актёра. Окно и вопрос — в `ui/export.rs` и `ui/export_form.rs`.

use pith_cev::Report;

use crate::theme;
use crate::tr;

/// Файл базы CodeEditVideo на GitHub.
const CEV_PAGE: &str =
    "https://github.com/DjonBeyron/codeeditvideo-projects/blob/main/db/records.json";

/// Итог: сколько создано, сколько без актёра, что не принято.
pub(super) fn show_report(ui: &mut egui::Ui, report: &Report) -> bool {
    ui.label(
        egui::RichText::new(tr!(
            format!("Создано строк: {}", report.created),
            format!("Rows created: {}", report.created)
        ))
        .color(theme::ACCENT)
        .size(15.0),
    );

    // Номера — сквозные по всей базе: она одна на все картины, и по номеру
    // строки видно, куда именно легли отрезки.
    if report.created > 0 {
        let last = report.first_number + report.created - 1;

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(tr!(
                format!("Номера: {} — {last}", report.first_number),
                format!("Numbers: {} — {last}", report.first_number)
            ))
            .color(theme::TEXT_SECONDARY)
            .size(12.0),
        );
    }

    if report.without_actor > 0 {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(tr!(
                format!("Без актёра: {}", report.without_actor),
                format!("Without an actor: {}", report.without_actor)
            ))
            .color(theme::TEXT_SECONDARY)
            .size(12.0),
        );
    }

    ui.add_space(10.0);
    ui.hyperlink_to(
        egui::RichText::new(tr!("Открыть базу на GitHub", "Open the database on GitHub"))
            .size(12.0),
        CEV_PAGE,
    );

    ui.add_space(12.0);
    ui.button(tr!("Закрыть", "Close")).clicked()
}

/// Отказ до первой строки: обычно нет доступа или образец не тот.
pub(super) fn show_failure(ui: &mut egui::Ui, why: &str) -> bool {
    ui.add(
        egui::Label::new(
            egui::RichText::new(tr!("Выгрузка не удалась", "The export failed"))
                .color(theme::ERROR)
                .size(15.0),
        )
        .wrap(),
    );

    ui.add_space(6.0);
    ui.add(
        egui::Label::new(
            egui::RichText::new(why)
                .color(theme::TEXT_SECONDARY)
                .size(12.0),
        )
        .wrap(),
    );

    ui.add_space(12.0);
    ui.button(tr!("Закрыть", "Close")).clicked()
}
