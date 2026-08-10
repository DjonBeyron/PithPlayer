//! Короткое уведомление поверх видео.
//!
//! Заменяет `FloatingCopyNotification` и `SubtitleCopyOverlay` из v4.

use crate::app::PithApp;
use crate::theme;

/// Отступ от верхнего края окна.
const TOP_MARGIN: f32 = 24.0;

/// Размер текста уведомления.
const FONT_SIZE: f32 = 15.0;

/// Какую долю ширины окна уведомление занимает, прежде чем перенестись.
///
/// Ширину переноса задаём сами: свободного места у области с якорем
/// на первом кадре ещё нет, и текст рвался по слову на строку — длинное
/// название закладки превращалось в столбик.
const MAX_WIDTH_FRACTION: f32 = 0.6;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    show_playback_error(app, ctx);

    let Some(text) = app.notice() else {
        return;
    };
    let text = text.to_string();
    let wrap_width = ctx.input(|i| i.viewport_rect()).width() * MAX_WIDTH_FRACTION;

    egui::Area::new(egui::Id::new("notice"))
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_TOP, [0.0, TOP_MARGIN])
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(theme::PANEL_BG.gamma_multiply(0.95))
                .inner_margin(egui::Margin::symmetric(16, 10))
                .corner_radius(6.0)
                .show(ui, |ui| {
                    show_text(ui, text, wrap_width);
                });
        });

    // Уведомление должно погаснуть само, даже если видео на паузе.
    ctx.request_repaint_after(std::time::Duration::from_millis(200));
}

/// Строка уведомления: раскладка своя, место под неё берётся по ней же.
///
/// Обычная надпись переносится по свободному месту области, а у области
/// с якорем его ровно столько, сколько она занимала прошлым кадром.
/// Выходил замкнутый круг: узкая область — узкий текст, и длинное
/// название закладки рассыпалось в столбик по слову на строку.
fn show_text(ui: &mut egui::Ui, text: String, wrap_width: f32) {
    let job = egui::text::LayoutJob::simple(
        text,
        egui::FontId::proportional(FONT_SIZE),
        theme::TEXT_PRIMARY,
        wrap_width,
    );

    let galley = ui.painter().layout_job(job);
    let (rect, _) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());

    ui.painter().galley(rect.min, galley, theme::TEXT_PRIMARY);
}

/// Надпись посреди окна, когда файл не играет.
///
/// Всплывашка гаснет через пару секунд, а пустое чёрное окно остаётся —
/// и выглядит поломкой плеера, хотя дело в файле.
fn show_playback_error(app: &PithApp, ctx: &egui::Context) {
    let Some(text) = app.playback_error() else {
        return;
    };

    egui::Area::new(egui::Id::new("playback_error"))
        .order(egui::Order::Middle)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .interactable(false)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(text).color(theme::ERROR).size(20.0));
                ui.add_space(8.0);
                ui.label(
                    // Без стрелок и прочих знаков: в шрифтах egui их нет,
                    // на месте символа выходит пустой квадрат.
                    egui::RichText::new(crate::tr!(
                        "Правый щелчок по видео, затем «Открыть файл…»",
                        "Right-click the video, then «Open file…»"
                    ))
                    .color(theme::TEXT_SECONDARY),
                );
            });
        });
}
