//! Диалог переименования закладки.

use crate::app::PithApp;
use crate::theme;
use crate::tr;

/// Ширина диалога.
const WIDTH: f32 = 420.0;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if app.bookmark_rename().is_none() {
        return;
    }

    let mut apply = false;
    let mut cancel = false;

    egui::Window::new(tr!("Название закладки", "Bookmark name"))
        .order(egui::Order::Foreground)
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
                    .hint_text(tr!("Реплика из фильма", "A line from the film")),
            );

            if fields.focus_pending {
                field.request_focus();
                fields.focus_pending = false;
            }

            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(tr!(
                    "Название становится именем вырезанного файла. \
                         Пустое поле вернёт подпись по времени.",
                    "The name becomes the file name of the cut fragment. \
                         Leave it empty to fall back to the timestamp.",
                ))
                .color(theme::TEXT_DISABLED)
                .small(),
            );

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                apply |= ui.button(tr!("Сохранить", "Save")).clicked();
                cancel |= ui.button(tr!("Отмена", "Cancel")).clicked();
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
