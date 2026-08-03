//! Диалог общих настроек нарезки (порт `RecordingSettingsForm` из v4).

use crate::app::PithApp;
use crate::theme;

/// Ширина диалога.
const DIALOG_WIDTH: f32 = 460.0;

/// Пределы длительности и отступа, секунды.
const MAX_DURATION: u32 = 600;
const MAX_BUFFER: u32 = 120;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if app.fragment_settings_dialog().is_none() {
        return;
    }

    let mut apply = false;
    let mut cancel = false;
    let mut pick_dir = false;
    let mut clear_dir = false;

    egui::Window::new("Настройки нарезки")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_width(DIALOG_WIDTH);

            if let Some(hint) = app.fragment_settings_hint() {
                ui.label(egui::RichText::new(hint).color(theme::ERROR));
                ui.add_space(8.0);
            }

            let Some(fields) = app.fragment_settings_dialog_mut() else {
                return;
            };

            ui.label(
                egui::RichText::new("Значения по умолчанию для новых списков отрезков")
                    .color(theme::TEXT_SECONDARY)
                    .small(),
            );
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Длительность, с").color(theme::TEXT_SECONDARY));
                ui.add(egui::DragValue::new(&mut fields.duration_sec).range(1..=MAX_DURATION));

                ui.add_space(16.0);

                ui.label(egui::RichText::new("Отступ назад, с").color(theme::TEXT_SECONDARY));
                ui.add(egui::DragValue::new(&mut fields.buffer_sec).range(0..=MAX_BUFFER));
            });

            ui.label(
                egui::RichText::new(
                    "Закладка на 00:10 при длительности 5 с и отступе 3 с даёт отрезок 00:07—00:12",
                )
                .color(theme::TEXT_DISABLED)
                .small(),
            );

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            ui.label(egui::RichText::new("Папка вывода").color(theme::TEXT_SECONDARY));
            ui.horizontal(|ui| {
                pick_dir |= ui.button("Выбрать…").clicked();

                if fields.output_dir.is_some() {
                    clear_dir |= ui
                        .button("Рядом с видео")
                        .on_hover_text("Складывать отрезки в папку исходного файла")
                        .clicked();
                }
            });

            let dir = match &fields.output_dir {
                Some(dir) => dir.to_string_lossy().to_string(),
                None => "Рядом с исходным файлом".to_string(),
            };
            ui.label(egui::RichText::new(dir).color(theme::TEXT_DISABLED).small());

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            ui.checkbox(
                &mut fields.audio_aac,
                "Звук в AAC — для Premiere Pro и After Effects",
            );
            ui.label(
                egui::RichText::new(
                    "Монтажные программы Adobe не читают EAC3, DTS и подобные дорожки: \
                     отрезок открывается, но звука в нём для программы нет. AAC понимают \
                     все. Видео при этом копируется без перекодирования.",
                )
                .color(theme::TEXT_DISABLED)
                .small(),
            );

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            ui.checkbox(&mut fields.reencode, "Перекодировать вместо перепаковки");
            ui.label(
                egui::RichText::new(
                    "Перепаковка быстрее в десятки раз и не теряет качества, но начинает \
                     отрезок с ближайшего опорного кадра. Перекодирование ставит старт \
                     точно по метке и выручает, когда монтажная программа не принимает \
                     исходный кодек.",
                )
                .color(theme::TEXT_DISABLED)
                .small(),
            );

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                apply |= ui.button("Сохранить").clicked();
                cancel |= ui.button("Отмена").clicked();
            });
        });

    apply |= ctx.input(|i| i.key_pressed(egui::Key::Enter));
    cancel |= ctx.input(|i| i.key_pressed(egui::Key::Escape));

    if pick_dir
        && let Some(dir) = rfd::FileDialog::new().pick_folder()
        && let Some(fields) = app.fragment_settings_dialog_mut()
    {
        fields.output_dir = Some(dir);
    }

    if clear_dir && let Some(fields) = app.fragment_settings_dialog_mut() {
        fields.output_dir = None;
    }

    if apply {
        app.apply_fragment_settings();
    } else if cancel {
        app.close_fragment_settings();
    }
}
