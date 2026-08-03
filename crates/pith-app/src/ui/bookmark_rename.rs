//! Диалог переименования закладки.

use crate::app::PithApp;
use crate::theme;

/// Ширина диалога.
const WIDTH: f32 = 420.0;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if app.bookmark_rename().is_none() {
        return;
    }

    let mut apply = false;
    let mut cancel = false;

    egui::Window::new("Название закладки")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_width(WIDTH);

            let Some(fields) = app.bookmark_rename_mut() else {
                return;
            };

            let field = ui.add(
                egui::TextEdit::singleline(&mut fields.name)
                    .desired_width(f32::INFINITY)
                    .hint_text("Реплика из фильма"),
            );

            if fields.focus_pending {
                field.request_focus();
                fields.focus_pending = false;
            }

            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(
                    "Название становится именем вырезанного файла. \
                     Пустое поле вернёт подпись по времени.",
                )
                .color(theme::TEXT_DISABLED)
                .small(),
            );

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                apply |= ui.button("Сохранить").clicked();
                cancel |= ui.button("Отмена").clicked();
            });
        });

    apply |= ctx.input(|i| i.key_pressed(egui::Key::Enter));
    cancel |= ctx.input(|i| i.key_pressed(egui::Key::Escape));

    if apply {
        app.apply_bookmark_rename();
    } else if cancel {
        app.close_bookmark_rename();
    }
}
