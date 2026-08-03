//! Сообщение поверх видео на время нарезки.
//!
//! Нарезка занимает диск целиком, воспроизведение на это время ставится
//! на паузу. Без объяснения это выглядит зависанием, и первое желание —
//! закрыть плеер, оборвав работу на середине.

use crate::app::PithApp;
use crate::theme;

/// Ширина окна сообщения.
const WIDTH: f32 = 420.0;

pub fn show(app: &PithApp, ctx: &egui::Context) {
    let Some(progress) = app.extraction_progress() else {
        return;
    };

    egui::Area::new(egui::Id::new("extraction_notice"))
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(theme::WINDOW_BG.gamma_multiply(0.97))
                .inner_margin(egui::Margin::symmetric(24, 20))
                .corner_radius(8.0)
                .show(ui, |ui| {
                    ui.set_width(WIDTH);

                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("Идёт вырезание отрезков")
                                .color(theme::TEXT_PRIMARY)
                                .strong()
                                .size(20.0),
                        );
                        ui.add_space(10.0);

                        // Пока задачи готовятся, число отрезков ещё неизвестно:
                        // их считает `ffprobe`, и на большом файле это секунды.
                        let stage = if progress.is_preparing() {
                            "Подготовка…".to_string()
                        } else {
                            format!("Готово {} из {}", progress.done, progress.total)
                        };

                        ui.label(
                            egui::RichText::new(stage)
                                .color(theme::TEXT_SECONDARY)
                                .size(15.0),
                        );
                        ui.add_space(10.0);

                        let bar = egui::ProgressBar::new(progress.fraction()).desired_width(WIDTH);
                        ui.add(if progress.is_preparing() {
                            bar.animate(true)
                        } else {
                            bar.show_percentage()
                        });
                        ui.add_space(12.0);

                        ui.label(
                            egui::RichText::new(
                                "Не закрывайте приложение. Воспроизведение \
                                 остановлено, пока идёт работа.",
                            )
                            .color(theme::TEXT_SECONDARY),
                        );
                    });
                });
        });

    // Пока идёт нарезка, воспроизведение стоит: без явного запроса
    // отрисовки прогресс замер бы на месте.
    ctx.request_repaint_after(std::time::Duration::from_millis(200));
}
