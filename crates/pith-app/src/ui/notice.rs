//! Короткое уведомление поверх видео.
//!
//! Заменяет `FloatingCopyNotification` и `SubtitleCopyOverlay` из v4.

use crate::app::PithApp;
use crate::theme;

/// Отступ от верхнего края окна.
const TOP_MARGIN: f32 = 24.0;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    let Some(text) = app.notice() else {
        return;
    };
    let text = text.to_string();

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
                    ui.label(
                        egui::RichText::new(text)
                            .color(theme::TEXT_PRIMARY)
                            .size(15.0),
                    );
                });
        });

    // Уведомление должно погаснуть само, даже если видео на паузе.
    ctx.request_repaint_after(std::time::Duration::from_millis(200));
}
