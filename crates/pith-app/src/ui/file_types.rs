//! Подтверждение смены файловых ассоциаций.
//!
//! Отдельное окно, а не молчаливое действие: запись в реестр меняет
//! поведение системы, и пользователь должен видеть, на что соглашается.

use crate::app::PithApp;
use crate::theme;
use crate::tr;

/// Ширина окна подтверждения.
const DIALOG_WIDTH: f32 = 480.0;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    let Some(prompt) = app.file_types_prompt().copied() else {
        return;
    };

    let mut confirm = false;
    let mut cancel = false;

    egui::Window::new(prompt.title())
        .order(egui::Order::Foreground)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_width(DIALOG_WIDTH);

            ui.label(egui::RichText::new(prompt.explanation()).color(theme::TEXT_SECONDARY));

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                confirm |= ui.button(prompt.confirm_label()).clicked();
                cancel |= ui.button(tr!("Отмена", "Cancel")).clicked();
            });
        });

    cancel |= ctx.input(|i| i.key_pressed(egui::Key::Escape));

    // Enter здесь не принимаем: согласие на правку реестра должно быть
    // осознанным нажатием, а не случайной клавишей.
    if confirm {
        app.confirm_file_types();
    } else if cancel {
        app.cancel_file_types();
    }
}
