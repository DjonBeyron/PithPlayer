//! Элементы управления этапа 0.
//!
//! Полноценный интерфейс в стиле v4 делается на этапе 1 (PLAN.md §6.11).
//! Здесь — минимум, нужный чтобы открыть файл и снять замеры.

use crate::app::PithApp;
use crate::theme;

/// Шаги перемотки, секунды. Схема из v4 (PLAN.md §6.8).
const SEEK_STEP: f64 = 5.0;
const SEEK_STEP_SMALL: f64 = 1.0;
const SEEK_STEP_LARGE: f64 = 60.0;
const VOLUME_STEP: i64 = 5;

/// Сообщение о невозможности запуска движка.
pub fn show_fatal_error(ui: &mut egui::Ui, message: &str) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(theme::WINDOW_BG))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.heading(egui::RichText::new("Плеер не смог запуститься").color(theme::ERROR));
                ui.add_space(16.0);
                ui.label(egui::RichText::new(message).color(theme::TEXT_PRIMARY));
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new(
                        "Проверьте, что рядом с программой лежит файл libmpv-2.dll",
                    )
                    .color(theme::TEXT_SECONDARY),
                );
            });
        });
}

/// Нижняя панель управления и панель замеров.
pub fn show_controls(app: &mut PithApp, ui: &mut egui::Ui) {
    show_metrics_panel(app, ui.ctx());

    egui::Panel::bottom(egui::Id::new("controls"))
        .exact_size(64.0)
        .frame(egui::Frame::NONE.fill(theme::PANEL_BG).inner_margin(8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Открыть").clicked() {
                    app.open_file_dialog();
                }

                let paused = app.engine().map(|e| e.state().paused).unwrap_or(true);

                if ui.button(if paused { "▶" } else { "❚❚" }).clicked() {
                    app.toggle_pause();
                }

                show_timeline(app, ui);
                show_volume(app, ui);
            });
        });
}

/// Полоса перемотки и текущее время.
fn show_timeline(app: &mut PithApp, ui: &mut egui::Ui) {
    let Some(engine) = app.engine() else {
        return;
    };

    let state = engine.state();
    let duration = state.duration;
    let mut position = state.position;

    ui.label(
        egui::RichText::new(format!(
            "{} / {}",
            format_time(position),
            format_time(duration)
        ))
        .color(theme::TEXT_PRIMARY)
        .monospace(),
    );

    if duration <= 0.0 {
        return;
    }

    let slider = ui.add_sized(
        [ui.available_width() - 160.0, 20.0],
        egui::Slider::new(&mut position, 0.0..=duration)
            .show_value(false)
            .trailing_fill(true),
    );

    // Перематываем только когда пользователь отпустил ползунок:
    // иначе на 4К каждый пиксель движения вызывал бы перемотку.
    if slider.drag_stopped() || (slider.changed() && !slider.dragged()) {
        app.seek_absolute(position);
    }
}

fn show_volume(app: &mut PithApp, ui: &mut egui::Ui) {
    let Some(engine) = app.engine() else {
        return;
    };

    let mut volume = engine.state().volume;

    ui.label(egui::RichText::new("🔊").color(theme::TEXT_SECONDARY));

    if ui
        .add_sized(
            [100.0, 20.0],
            egui::Slider::new(&mut volume, 0..=150).show_value(false),
        )
        .changed()
        && let Some(engine) = app.engine_mut()
        && let Err(e) = engine.set_volume(volume)
    {
        tracing::warn!(error = %e, "не удалось изменить громкость");
    }
}

/// Панель замеров этапа 0.
fn show_metrics_panel(app: &mut PithApp, ctx: &egui::Context) {
    if !app.show_metrics {
        return;
    }

    egui::Window::new("Замеры")
        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
        .resizable(false)
        .collapsible(true)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(app.metrics.report(app.hwdec.label()))
                    .color(theme::TEXT_PRIMARY)
                    .monospace(),
            );

            ui.separator();
            ui.label(
                egui::RichText::new(
                    "Режим декодирования задаётся при запуске:\n\
                     pith-player --hwdec=zero-copy | copy | software",
                )
                .color(theme::TEXT_SECONDARY)
                .small(),
            );
        });
}

/// Горячие клавиши. Схема сохраняется из v4 (PLAN.md §6.8).
pub fn handle_hotkeys(app: &mut PithApp, ctx: &egui::Context) {
    // Пока фокус в текстовом поле, клавиши принадлежат ему.
    if ctx.egui_wants_keyboard_input() {
        return;
    }

    let (seek, volume, toggle_pause) = ctx.input(|i| {
        let step = if i.modifiers.ctrl {
            SEEK_STEP_LARGE
        } else if i.modifiers.shift {
            SEEK_STEP_SMALL
        } else {
            SEEK_STEP
        };

        let mut seek = 0.0;
        if i.key_pressed(egui::Key::ArrowRight) {
            seek += step;
        }
        if i.key_pressed(egui::Key::ArrowLeft) {
            seek -= step;
        }

        let mut volume = 0;
        if i.key_pressed(egui::Key::ArrowUp) {
            volume += VOLUME_STEP;
        }
        if i.key_pressed(egui::Key::ArrowDown) {
            volume -= VOLUME_STEP;
        }

        (seek, volume, i.key_pressed(egui::Key::Space))
    });

    if toggle_pause {
        app.toggle_pause();
    }
    if seek != 0.0 {
        app.seek_relative(seek);
    }
    if volume != 0 {
        app.adjust_volume(volume);
    }
}

/// Время в формате «Ч:ММ:СС» либо «М:СС» для коротких файлов.
fn format_time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "0:00".into();
    }

    let total = seconds as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes}:{secs:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::format_time;

    #[test]
    fn форматирует_короткое_время() {
        assert_eq!(format_time(0.0), "0:00");
        assert_eq!(format_time(65.0), "1:05");
        assert_eq!(format_time(599.0), "9:59");
    }

    #[test]
    fn форматирует_часы() {
        assert_eq!(format_time(3600.0), "1:00:00");
        assert_eq!(format_time(7325.0), "2:02:05");
    }

    #[test]
    fn защищается_от_некорректных_значений() {
        assert_eq!(format_time(-5.0), "0:00");
        assert_eq!(format_time(f64::NAN), "0:00");
        assert_eq!(format_time(f64::INFINITY), "0:00");
    }
}
