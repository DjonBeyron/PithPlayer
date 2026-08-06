//! Сообщение об итогах переноса данных из версии 4.
//!
//! Показывается один раз, после первого запуска.

use crate::app::PithApp;
use crate::theme;
use crate::tr;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    let Some(report) = app.migration_report() else {
        return;
    };

    let moved = report.positions_moved;
    let skipped = report.positions_skipped;
    let mut dismissed = false;

    egui::Modal::new(egui::Id::new("migration_report")).show(ctx, |ui| {
        ui.set_width(440.0);

        ui.heading(
            egui::RichText::new(tr!("Данные перенесены", "Data imported"))
                .color(theme::TEXT_PRIMARY),
        );
        ui.add_space(10.0);

        ui.label(
            egui::RichText::new(tr!(
                format!("Позиций просмотра перенесено: {moved}"),
                format!("Watch positions imported: {moved}")
            ))
            .color(theme::SUCCESS),
        );

        if skipped > 0 {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(tr!(
                    format!("Пропущено: {skipped} — этих файлов уже нет на диске"),
                    format!("Skipped: {skipped} — those files are gone from disk")
                ))
                .color(theme::TEXT_SECONDARY),
            );
        }

        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(tr!(
                "Данные версии 4 остались нетронутыми — старый плеер работает как прежде.",
                "Version 4 data is untouched — the old player still works as before."
            ))
            .color(theme::TEXT_SECONDARY)
            .small(),
        );

        ui.add_space(16.0);

        if ui.button(tr!("Понятно", "Got it")).clicked() {
            dismissed = true;
        }
    });

    if dismissed || ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
        app.dismiss_migration_report();
    }
}
