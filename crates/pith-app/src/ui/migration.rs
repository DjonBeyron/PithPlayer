//! Сообщение об итогах переноса данных из версии 4.
//!
//! Показывается один раз, после первого запуска.

use crate::app::PithApp;
use crate::theme;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    let Some(report) = app.migration_report() else {
        return;
    };

    let moved = report.positions_moved;
    let skipped = report.positions_skipped;
    let mut dismissed = false;

    egui::Modal::new(egui::Id::new("migration_report")).show(ctx, |ui| {
        ui.set_width(440.0);

        ui.heading(egui::RichText::new("Данные перенесены").color(theme::TEXT_PRIMARY));
        ui.add_space(10.0);

        ui.label(
            egui::RichText::new(format!("Позиций просмотра перенесено: {moved}"))
                .color(theme::SUCCESS),
        );

        if skipped > 0 {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!(
                    "Пропущено: {skipped} — этих файлов уже нет на диске"
                ))
                .color(theme::TEXT_SECONDARY),
            );
        }

        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(
                "Данные версии 4 остались нетронутыми — старый плеер работает как прежде.",
            )
            .color(theme::TEXT_SECONDARY)
            .small(),
        );

        ui.add_space(16.0);

        if ui.button("Понятно").clicked() {
            dismissed = true;
        }
    });

    if dismissed || ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
        app.dismiss_migration_report();
    }
}
