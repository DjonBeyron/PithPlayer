//! Пункты меню про распознавание текста с кадра.

use crate::app::{OcrModels, PithApp};
use crate::theme;
use crate::tr;

pub(super) fn show_ocr_items(app: &mut PithApp, ui: &mut egui::Ui) {
    ui.separator();
    ui.label(
        egui::RichText::new(tr!("Текст с кадра (OCR)", "Text from the frame (OCR)"))
            .color(theme::TEXT_SECONDARY)
            .small(),
    );

    let mut on = app.ocr().settings.enabled;

    if ui
        .checkbox(
            &mut on,
            tr!(
                "Распознавать, если у закладки нет субтитров",
                "Recognize when a bookmark has no subtitles"
            ),
        )
        .on_hover_text(tr!(
            "Закладка без реплики берёт кадр выбранной зоны и читает с него текст",
            "A bookmark with no line takes the chosen zone of the frame and reads text from it"
        ))
        .changed()
    {
        app.set_ocr_enabled(ui.ctx(), on);

        // Включили без зоны — сразу предлагаем её выбрать.
        if on && app.ocr().settings.zone.is_none() {
            app.start_zone_selection();
            ui.close();
        }
    }

    if ui
        .button(tr!("Выбрать зону субтитров…", "Choose the subtitle zone…"))
        .on_hover_text(tr!(
            "Обведите рамкой место, где стоят субтитры; зона сохранится для всех видео",
            "Frame the place where subtitles appear; the zone is kept for all videos"
        ))
        .clicked()
    {
        app.start_zone_selection();
        ui.close();
    }

    let zone_set = app.ocr().settings.zone.is_some();
    let status = match &app.ocr().models {
        OcrModels::Downloading(_) => tr!("Скачиваю модели…", "Downloading the models…").to_string(),
        OcrModels::Failed(why) => {
            format!("{} {why}", tr!("Модели не скачались:", "Models failed:"))
        }
        _ if !zone_set => tr!("Зона не выбрана", "No zone chosen").to_string(),
        _ => tr!("Зона выбрана", "Zone chosen").to_string(),
    };

    ui.label(
        egui::RichText::new(status)
            .color(theme::PANEL_MUTED)
            .small(),
    );

    if matches!(app.ocr().models, OcrModels::Failed(_))
        && ui
            .button(tr!("Скачать модели ещё раз", "Download the models again"))
            .clicked()
    {
        app.ocr().models = OcrModels::Unknown;
        app.ensure_ocr_models(ui.ctx());
    }
}
