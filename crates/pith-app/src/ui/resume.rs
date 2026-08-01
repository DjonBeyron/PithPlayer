//! Предложение продолжить просмотр с сохранённой позиции.
//!
//! Порт `ContinueWatchingDialog` из v4 (PLAN.md §6.6).

use crate::app::PithApp;
use crate::theme;
use crate::ui::format_time;

/// Показывает предложение продолжить, если оно есть.
pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    let Some(offer) = app.resume_offer() else {
        return;
    };

    let file_name = offer.file_name.clone();
    let position = offer.position;

    let mut accepted = false;
    let mut declined = false;

    egui::Modal::new(egui::Id::new("resume_watching")).show(ctx, |ui| {
        ui.set_width(420.0);

        ui.heading(egui::RichText::new("Продолжить просмотр?").color(theme::TEXT_PRIMARY));
        ui.add_space(10.0);

        ui.label(egui::RichText::new(&file_name).color(theme::TEXT_SECONDARY));
        ui.add_space(6.0);

        ui.label(
            egui::RichText::new(format!("Вы остановились на {}", format_time(position)))
                .color(theme::ACCENT)
                .monospace(),
        );

        ui.add_space(18.0);

        ui.horizontal(|ui| {
            if ui
                .button(format!("Продолжить с {}", format_time(position)))
                .clicked()
            {
                accepted = true;
            }

            if ui.button("Сначала").clicked() {
                declined = true;
            }
        });
    });

    // Enter продолжает, Esc начинает сначала — привычное поведение диалогов.
    ctx.input(|i| {
        if i.key_pressed(egui::Key::Enter) {
            accepted = true;
        }
        if i.key_pressed(egui::Key::Escape) {
            declined = true;
        }
    });

    if accepted {
        app.accept_resume();
    } else if declined {
        app.decline_resume();
    }
}
