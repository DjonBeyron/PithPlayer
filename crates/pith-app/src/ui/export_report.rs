//! Отчёт и отказ окна выгрузки.
//!
//! Отдельно от самого окна: у отчёта своя разметка и свои оговорки —
//! номера созданных строк, слова без транскрипции, строки, которых Notion
//! не принял. Окно и вопрос — в `ui/export.rs` и `ui/export_form.rs`.

use pith_notion::Report;

use crate::theme;
use crate::tr;

/// Адрес созданной базы — по нему её открывают в браузере.
const NOTION_PAGE: &str = "https://www.notion.so/";

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

    // Строка начинается со значений образца — `STATUS` в том числе,
    // а на него смотрит синхронизатор. Не прочиталось — строки на месте,
    // но эти поля у них пусты.
    if !report.sample_taken {
        ui.add_space(4.0);
        ui.add(
            egui::Label::new(
                egui::RichText::new(tr!(
                    "Заготовку строки взять у образца не удалось — STATUS пуст",
                    "The row sample could not be read from the template — STATUS is empty"
                ))
                .color(theme::ERROR)
                .size(12.0),
            )
            .wrap(),
        );
    }

    show_failures(ui, report);

    ui.add_space(10.0);
    let url = format!("{NOTION_PAGE}{}", report.database_id.replace('-', ""));
    ui.hyperlink_to(
        egui::RichText::new(tr!("Открыть в Notion", "Open in Notion")).size(12.0),
        url,
    );

    ui.add_space(12.0);
    ui.button(tr!("Закрыть", "Close")).clicked()
}

/// Строки, которые Notion не принял, — с причинами.
fn show_failures(ui: &mut egui::Ui, report: &Report) {
    if report.failed.is_empty() {
        return;
    }

    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(tr!(
            format!("Не принято: {}", report.failed.len()),
            format!("Rejected: {}", report.failed.len())
        ))
        .color(theme::ERROR)
        .size(13.0),
    );

    egui::ScrollArea::vertical()
        .max_height(120.0)
        .show(ui, |ui| {
            for (number, why) in &report.failed {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(format!("{number}: {why}"))
                            .color(theme::TEXT_SECONDARY)
                            .size(11.0),
                    )
                    .wrap(),
                );
            }
        });
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
