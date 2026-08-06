//! Подсказка о времени при перемотке с клавиатуры.
//!
//! Стрелками перематывают вслепую: панель управления в полноэкранном
//! режиме спрятана, а в оконном взгляд всё равно на кадре. Подсказка
//! в правом верхнем углу показывает, куда попали, и гаснет сама, когда
//! перематывать перестали.

use crate::app::PithApp;
use crate::theme;
use crate::ui::{format_time, format_time_padded};

/// Отступ от краёв окна.
const MARGIN: f32 = 18.0;

/// Отступ содержимого внутри плашки.
const PADDING: i8 = 10;

/// Насколько опустить подсказку, когда над ней панель замеров.
const METRICS_HEIGHT: f32 = 240.0;

/// Показывает подсказку, пока идёт перемотка клавишами.
pub fn show(app: &PithApp, ctx: &egui::Context) {
    let Some(hud) = app.seek_hud() else {
        return;
    };

    let (position, duration) = hud;

    // Панель замеров занимает тот же угол — при ней подсказка встаёт ниже.
    let top = if app.show_metrics() {
        METRICS_HEIGHT
    } else {
        MARGIN
    };

    egui::Area::new(egui::Id::new("seek_hud"))
        // Слоем ниже окон: подсказка о перемотке относится к кадру и не
        // должна ложиться на открытый диалог.
        .order(egui::Order::Middle)
        .anchor(egui::Align2::RIGHT_TOP, [-MARGIN, top])
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(theme::PANEL_CARD.gamma_multiply(0.92))
                .stroke(egui::Stroke::new(1.0, theme::PANEL_BORDER))
                .inner_margin(egui::Margin::symmetric(PADDING + 4, PADDING))
                .corner_radius(8.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;

                        ui.label(
                            egui::RichText::new(format_time_padded(position, duration))
                                .color(theme::PANEL_ACCENT)
                                .monospace()
                                .size(22.0)
                                .strong(),
                        );

                        ui.label(
                            egui::RichText::new(format!("/ {}", format_time(duration)))
                                .color(theme::PANEL_MUTED)
                                .monospace()
                                .size(13.0),
                        );
                    });
                });
        });

    // Подсказка гаснет по времени, а на паузе кадры сами не идут.
    ctx.request_repaint_after(std::time::Duration::from_millis(100));
}
