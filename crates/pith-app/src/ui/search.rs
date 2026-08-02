//! Окно поиска по субтитрам.
//!
//! Порт `SubtitleSearchForm` из v4 (PLAN.md §6.2).

use crate::app::PithApp;
use crate::theme;
use crate::ui::format_time;

/// Высота списка результатов.
const RESULTS_HEIGHT: f32 = 320.0;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if !app.search_state().open {
        return;
    }

    let mut open = true;
    let mut query = app.search_state().query.clone();
    let mut jump_to = None;
    let mut query_changed = false;

    egui::Window::new("Поиск по субтитрам")
        .open(&mut open)
        .default_width(560.0)
        .collapsible(false)
        .show(ctx, |ui| {
            let field = ui.add(
                egui::TextEdit::singleline(&mut query)
                    .hint_text("Введите фразу")
                    .desired_width(f32::INFINITY),
            );

            // Курсор сразу в поле: окно открывают, чтобы печатать.
            if field.changed() {
                query_changed = true;
            }
            if !field.has_focus() && ui.memory(|m| m.focused().is_none()) {
                field.request_focus();
            }

            ui.add_space(6.0);

            if let Some(status) = &app.search_state().status {
                ui.label(egui::RichText::new(status).color(theme::TEXT_SECONDARY));
                return;
            }

            show_results(app, ui, &mut jump_to);
        });

    if query_changed {
        app.set_search_query(query);
    }

    if let Some(time) = jump_to {
        app.seek_absolute(time);
    }

    if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.close_search();
    }
}

fn show_results(app: &PithApp, ui: &mut egui::Ui, jump_to: &mut Option<f64>) {
    let state = app.search_state();

    if state.query.trim().is_empty() {
        ui.label(
            egui::RichText::new("Начните вводить фразу — найденные реплики появятся здесь")
                .color(theme::TEXT_SECONDARY),
        );
        return;
    }

    if state.hits.is_empty() {
        ui.label(egui::RichText::new("Ничего не найдено").color(theme::TEXT_SECONDARY));
        return;
    }

    ui.label(
        egui::RichText::new(format!("Найдено реплик: {}", state.hits.len()))
            .color(theme::TEXT_SECONDARY)
            .small(),
    );
    ui.add_space(4.0);

    egui::ScrollArea::vertical()
        .max_height(RESULTS_HEIGHT)
        .show(ui, |ui| {
            for hit in &state.hits {
                let row = ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format_time(hit.start))
                            .color(theme::ACCENT)
                            .monospace(),
                    );
                    ui.label(egui::RichText::new(&hit.text).color(theme::TEXT_PRIMARY));
                });

                // Щелчок в любом месте строки перематывает к реплике.
                if row
                    .response
                    .interact(egui::Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    *jump_to = Some(hit.start);
                }

                ui.separator();
            }
        });
}
