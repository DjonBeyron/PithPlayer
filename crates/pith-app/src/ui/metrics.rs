//! Панель замеров производительности.
//!
//! Нужна для сравнения режимов декодирования (PLAN.md §3). Скрывается
//! ключом `--no-metrics`.

use crate::app::PithApp;
use crate::theme;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if !app.show_metrics {
        return;
    }

    egui::Window::new("Замеры")
        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
        .resizable(false)
        .collapsible(true)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(app.metrics.report(app.hwdec.label()))
                    .color(theme::TEXT_PRIMARY)
                    .monospace(),
            );

            if let Some(engine) = app.engine() {
                let state = engine.state();
                ui.separator();
                ui.label(
                    egui::RichText::new(format!(
                        "Кадр: {}×{}\nСкорость: {:.2}×",
                        state.display_width, state.display_height, state.speed
                    ))
                    .color(theme::TEXT_SECONDARY)
                    .monospace(),
                );
            }
        });
}
