//! Панель замеров производительности.
//!
//! Нужна для сравнения режимов декодирования (PLAN.md §3). Скрывается
//! ключом `--no-metrics`.

use crate::app::PithApp;
use crate::theme;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    show_version(app, ctx);

    if !app.show_metrics() {
        return;
    }

    let mut hide = false;

    egui::Window::new("Замеры")
        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
        .resizable(false)
        .collapsible(true)
        .show(ctx, |ui| {
            // Вернуть панель можно из меню по правому щелчку.
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    hide = ui
                        .small_button("Скрыть")
                        .on_hover_text("Вернуть можно через меню по правому щелчку")
                        .clicked();
                });
            });

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

    if hide {
        app.toggle_metrics();
    }
}

/// Версия программы в правом верхнем углу.
///
/// Нужна, чтобы при разборе жалоб сразу было видно, какая сборка запущена.
/// Когда панель замеров открыта, версия уходит ей под низ.
fn show_version(app: &PithApp, ctx: &egui::Context) {
    let offset = if app.show_metrics() {
        METRICS_HEIGHT
    } else {
        VERSION_MARGIN
    };

    egui::Area::new(egui::Id::new("version"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::RIGHT_TOP, [-VERSION_MARGIN, offset])
        .interactable(false)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(format!("Pith Player {}", crate::VERSION))
                    .color(theme::TEXT_DISABLED)
                    .small(),
            );
        });
}

/// Отступ версии от края окна.
const VERSION_MARGIN: f32 = 14.0;

/// Насколько опустить версию, когда над ней панель замеров.
const METRICS_HEIGHT: f32 = 230.0;
