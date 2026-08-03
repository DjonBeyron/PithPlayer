//! Нижняя панель управления.
//!
//! Рисуется поверх видео отдельным слоем: mpv закрашивает всё, что
//! нарисовано до него.

use crate::app::PithApp;
use crate::theme;
use crate::ui::{format_time, metrics, timeline};

/// Ширина кнопок, времени и отступов — без громкости.
const BASE_CONTROLS_WIDTH: f32 = 300.0;
/// Ширина регулятора громкости вместе со значком.
const VOLUME_WIDTH: f32 = 130.0;
/// Минимальная ширина полосы перемотки.
const MIN_TIMELINE_WIDTH: f32 = 120.0;
/// Уже этой ширины громкость прячем: окно подогнано под вертикальное видео,
/// и места хватает только на перемотку.
const NARROW_WINDOW: f32 = 620.0;
/// Через сколько секунд без движения мыши прятать панель.
const HIDE_AFTER_SECONDS: f64 = 2.5;

/// Панель управления и панель замеров.
pub fn show_controls(app: &mut PithApp, ctx: &egui::Context) {
    metrics::show(app, ctx);

    if !is_visible(app, ctx) {
        return;
    }

    let screen = ctx.input(|i| i.viewport_rect());

    // Якорь к нижнему краю, а не расчёт положения по заданной высоте:
    // содержимое занимает не ровно столько, сколько мы предполагали,
    // и под панелью оставалась полоска видео.
    egui::Area::new(egui::Id::new("controls"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::LEFT_BOTTOM, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_width(screen.width());

            egui::Frame::NONE
                .fill(theme::PANEL_BG.gamma_multiply(0.92))
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    let inner_width = screen.width() - 20.0;
                    ui.set_width(inner_width);
                    show_row(app, ui, inner_width);
                });
        });
}

/// Показывать ли панель.
///
/// В оконном режиме панель видна всегда. В полноэкранном прячется, если
/// мышь не двигалась, — но не тогда, когда курсор над самой панелью.
fn is_visible(app: &PithApp, ctx: &egui::Context) -> bool {
    if !app.is_fullscreen() {
        return true;
    }

    let idle = ctx.input(|i| i.time - app.last_pointer_activity());
    idle < HIDE_AFTER_SECONDS
}

fn show_row(app: &mut PithApp, ui: &mut egui::Ui, inner_width: f32) {
    ui.horizontal(|ui| {
        if ui.button("Открыть").on_hover_text("Открыть файл").clicked() {
            app.open_file_dialog();
        }

        let paused = app.engine().map(|e| e.state().paused).unwrap_or(true);
        let play_label = if paused { "▶" } else { "❚❚" };

        if ui
            .button(play_label)
            .on_hover_text("Пауза / продолжить (пробел)")
            .clicked()
        {
            app.toggle_pause();
        }

        show_time_label(app, ui);

        // В узком окне (подогнанном под вертикальное видео) громкости места
        // не остаётся — уступаем его полосе перемотки.
        let show_volume_slider = inner_width >= NARROW_WINDOW;
        let reserved = if show_volume_slider {
            BASE_CONTROLS_WIDTH + VOLUME_WIDTH
        } else {
            BASE_CONTROLS_WIDTH
        };

        // Внутри плавающего слоя `available_width` не отражает ширину окна,
        // поэтому размер полосы считаем сами.
        let timeline_width = (inner_width - reserved).max(MIN_TIMELINE_WIDTH);
        show_timeline(app, ui, timeline_width);

        show_speed(app, ui);

        if show_volume_slider {
            show_volume(app, ui);
        }
    });
}

fn show_time_label(app: &PithApp, ui: &mut egui::Ui) {
    let (position, duration) = app
        .engine()
        .map(|e| (e.state().position, e.state().duration))
        .unwrap_or((0.0, 0.0));

    ui.label(
        egui::RichText::new(format!(
            "{} / {}",
            format_time(position),
            format_time(duration)
        ))
        .color(theme::TEXT_PRIMARY)
        .monospace(),
    );
}

fn show_timeline(app: &mut PithApp, ui: &mut egui::Ui, width: f32) {
    let (position, duration) = app
        .engine()
        .map(|e| (e.state().position, e.state().duration))
        .unwrap_or((0.0, 0.0));

    // Закладки появятся на этапе 4 — до тех пор жёлтых отрезков нет.
    let fragments = app.fragment_ranges();
    let response = timeline::show(ui, position, duration, width, &fragments);

    if let Some(target) = response.seek_to {
        app.seek_absolute(target);
    }
}

/// Скорость воспроизведения. Показывается, только когда отличается от обычной.
fn show_speed(app: &mut PithApp, ui: &mut egui::Ui) {
    let speed = app.engine().map(|e| e.state().speed).unwrap_or(1.0);

    if (speed - 1.0).abs() < f64::EPSILON {
        return;
    }

    let label = ui.label(
        egui::RichText::new(format!("{speed:.2}×"))
            .color(theme::ACCENT)
            .monospace(),
    );

    if label
        .on_hover_text("Сбросить скорость")
        .interact(egui::Sense::click())
        .clicked()
    {
        app.reset_speed();
    }
}

fn show_volume(app: &mut PithApp, ui: &mut egui::Ui) {
    let Some(engine) = app.engine() else {
        return;
    };

    let mut volume = engine.state().volume;
    let muted = volume == 0;

    ui.label(egui::RichText::new(if muted { "🔇" } else { "🔊" }).color(theme::TEXT_SECONDARY));

    ui.spacing_mut().slider_width = VOLUME_WIDTH;

    let changed = ui
        .add_sized(
            [VOLUME_WIDTH, 20.0],
            egui::Slider::new(&mut volume, 0..=150).show_value(false),
        )
        .on_hover_text(format!("Громкость: {volume}%"))
        .changed();

    if changed {
        app.set_volume(volume);
    }
}
