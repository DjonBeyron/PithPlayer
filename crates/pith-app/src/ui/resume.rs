//! Предложение продолжить просмотр с сохранённой позиции.
//!
//! Порт `ContinueWatchingDialog` из v4 (PLAN.md §6.6), но не диалогом:
//! модальное окно посреди экрана закрывало кадр и не давало ни нажать
//! паузу, ни перемотать, пока на него не ответят. Здесь — узкая плашка
//! сверху, поверх которой плеер работает как обычно. Не ответили —
//! она исчезает сама, и фильм просто идёт с начала.

use crate::app::PithApp;
use crate::theme;
use crate::tr;
use crate::ui::format_time;

/// Отступ от верхнего края окна.
const TOP_MARGIN: f32 = 24.0;

/// Толщина полоски обратного отсчёта.
const COUNTDOWN_HEIGHT: f32 = 2.0;

/// Показывает предложение продолжить, если оно есть.
pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    let Some(offer) = app.resume_offer() else {
        return;
    };

    let position = offer.position;
    let remaining = app.resume_remaining();

    let mut accepted = false;
    let mut declined = false;
    let mut hovered = false;

    let area = egui::Area::new(egui::Id::new("resume_watching"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_TOP, [0.0, TOP_MARGIN])
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(theme::PANEL_BG.gamma_multiply(0.95))
                .inner_margin(egui::Margin::symmetric(14, 8))
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(tr!("Вы остановились на", "You stopped at"))
                                .color(theme::TEXT_SECONDARY),
                        );
                        ui.label(
                            egui::RichText::new(format_time(position))
                                .color(theme::ACCENT)
                                .monospace(),
                        );

                        ui.add_space(8.0);

                        // Enter повторён в подсказке: плашка живёт недолго,
                        // и до мыши можно не успеть.
                        if ui
                            .button(tr!("Продолжить", "Resume"))
                            .on_hover_text(tr!(
                                "Продолжить с этого места (Enter)",
                                "Continue from here (Enter)"
                            ))
                            .clicked()
                        {
                            accepted = true;
                        }

                        if ui
                            .button(tr!("Сначала", "From start"))
                            .on_hover_text(tr!(
                                "Смотреть с начала и забыть сохранённое место",
                                "Watch from the beginning and forget the saved spot"
                            ))
                            .clicked()
                        {
                            declined = true;
                        }
                    });

                    show_countdown(ui, remaining);
                });
        });

    // Под курсором плашка не исчезает: обидно потерять её ровно в тот
    // миг, когда до неё дотянулись.
    if area.response.hovered() {
        hovered = true;
    }

    // Enter продолжает — привычно и для диалога, и для плашки. Escape
    // не занимаем: он выходит из полноэкранного режима.
    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
        accepted = true;
    }

    if accepted {
        app.accept_resume();
    } else if declined {
        app.decline_resume();
    } else if hovered {
        app.postpone_resume();
    }

    // Отсчёт должен идти и на паузе, когда кадры сами не обновляются.
    ctx.request_repaint_after(std::time::Duration::from_millis(100));
}

/// Полоска, показывающая, сколько плашке осталось.
fn show_countdown(ui: &mut egui::Ui, remaining: f32) {
    let width = ui.min_rect().width();
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(width, COUNTDOWN_HEIGHT), egui::Sense::hover());

    let left = egui::Rect::from_min_size(
        rect.min,
        egui::vec2(rect.width() * remaining.clamp(0.0, 1.0), rect.height()),
    );

    ui.painter()
        .rect_filled(left, 1.0, theme::ACCENT.gamma_multiply(0.5));
}
