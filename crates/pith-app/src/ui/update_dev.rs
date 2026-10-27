//! Раздел режима разработчика в окне обновления.

use crate::app::PithApp;
use crate::theme;
use crate::tr;

/// Поле папки журналов, выбор папки и запись.
pub(super) fn show_dev(app: &mut PithApp, ui: &mut egui::Ui) {
    let mut pick = false;
    let mut save = false;

    ui.add_space(10.0);
    ui.label(
        egui::RichText::new(tr!("Режим разработчика", "Developer mode"))
            .color(theme::TEXT_PRIMARY)
            .size(15.0)
            .strong(),
    );
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(tr!(
            "Папка для журналов (pith-player.log и mpv.log). Применится после перезапуска.",
            "Folder for logs (pith-player.log and mpv.log). Applies after a restart."
        ))
        .color(theme::PANEL_MUTED)
        .size(12.0),
    );
    ui.add_space(4.0);

    let dev = app.dev_panel();
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut dev.log_dir).desired_width(280.0));
        pick = ui.button(tr!("Выбрать…", "Browse…")).clicked();
        save = ui.button(tr!("Сохранить", "Save")).clicked();
    });

    if dev.saved {
        ui.label(
            egui::RichText::new(tr!(
                "Сохранено. Перезапустите плеер.",
                "Saved. Restart the player."
            ))
            .color(theme::PANEL_ACCENT)
            .size(12.0),
        );
    }

    if pick {
        app.dev_pick_folder();
    }
    if save {
        app.dev_save();
    }
}
