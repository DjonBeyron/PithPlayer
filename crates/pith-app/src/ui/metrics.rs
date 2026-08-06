//! Панель замеров производительности.
//!
//! Нужна для сравнения режимов декодирования (PLAN.md §3). Скрывается
//! ключом `--no-metrics`.

use crate::app::PithApp;
use crate::theme;
use crate::tr;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if !app.show_metrics() {
        return;
    }

    let mut hide = false;

    egui::Window::new(tr!("Замеры", "Metrics"))
        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
        .resizable(false)
        .collapsible(true)
        .show(ctx, |ui| {
            // Вернуть панель можно из меню по правому щелчку.
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    hide = ui
                        .small_button(tr!("Скрыть", "Hide"))
                        .on_hover_text(tr!(
                            "Вернуть можно через меню по правому щелчку",
                            "Bring it back from the right-click menu"
                        ))
                        .clicked();
                });
            });

            ui.label(
                egui::RichText::new(app.metrics.report(hwdec_label(app.hwdec)))
                    .color(theme::TEXT_PRIMARY)
                    .monospace(),
            );

            if let Some(engine) = app.engine() {
                let state = engine.state();
                ui.separator();
                ui.label(
                    egui::RichText::new(tr!(
                        format!(
                            "Кадр: {}×{}\nСкорость: {:.2}×",
                            state.display_width, state.display_height, state.speed
                        ),
                        format!(
                            "Frame: {}×{}\nSpeed: {:.2}×",
                            state.display_width, state.display_height, state.speed
                        )
                    ))
                    .color(theme::TEXT_SECONDARY)
                    .monospace(),
                );
            }
        });

    if hide {
        app.toggle_metrics();
    }
}

/// Название режима декодирования.
///
/// Слова живут здесь, а не в `pith-mpv`: движок ничего не знает о языке
/// интерфейса, и знать не должен.
fn hwdec_label(hwdec: pith_mpv::HwDec) -> &'static str {
    match hwdec {
        pith_mpv::HwDec::ZeroCopy => tr!(
            "Аппаратное, без копирования (d3d11va с откатом)",
            "Hardware, zero-copy (d3d11va with fallback)"
        ),
        pith_mpv::HwDec::Copy => tr!(
            "Аппаратное с копированием (auto-copy)",
            "Hardware with copy (auto-copy)"
        ),
        pith_mpv::HwDec::Software => {
            tr!("Программное (без ускорения)", "Software (no acceleration)")
        }
    }
}

// Версии поверх кадра больше нет: она лежала на картинке и мешала.
// Какая сборка запущена, видно в заголовке окна и в первой строке лога.
