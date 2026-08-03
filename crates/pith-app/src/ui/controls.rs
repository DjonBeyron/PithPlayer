//! Нижняя панель управления.
//!
//! Рисуется поверх видео отдельным слоем: mpv закрашивает всё, что
//! нарисовано до него.

use crate::app::PithApp;
use crate::theme;
use crate::ui::{format_time, metrics, timeline};

/// Размер кнопок панели: одинаковый, чтобы строка не прыгала.
const BUTTON_SIZE: [f32; 2] = [30.0, 24.0];
/// Сторона иконки на кнопке открытия файла.
const ICON_SIZE: f32 = 22.0;
/// Ширина полосы громкости.
const VOLUME_WIDTH: f32 = 110.0;
/// Минимальная ширина полосы перемотки.
const MIN_TIMELINE_WIDTH: f32 = 120.0;
/// Уже этой ширины громкость прячем: окно подогнано под вертикальное видео,
/// и места хватает только на перемотку.
const NARROW_WINDOW: f32 = 620.0;
/// Отступ содержимого панели от краёв окна — одинаковый слева и справа.
const SIDE_MARGIN: f32 = 12.0;
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
                // Отступы слева и справа задаёт только эта рамка: сужать
                // содержимое дополнительно нельзя, иначе правый край
                // отходит дальше левого.
                .inner_margin(egui::Margin::symmetric(SIDE_MARGIN as i8, 8))
                .show(ui, |ui| {
                    show_row(app, ui, screen.width() - SIDE_MARGIN * 2.0);
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
        show_open_button(app, ui);
        show_play_button(app, ui);
        show_time_label(app, ui);

        // В узком окне (подогнанном под вертикальное видео) громкости места
        // не остаётся — уступаем его полосе перемотки.
        let with_volume = inner_width >= NARROW_WINDOW;

        // Правый край выкладываем справа налево: тогда последний элемент
        // упирается ровно в отступ панели, и края выходят одинаковыми.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if with_volume {
                show_volume(app, ui);
            }

            show_fullscreen_button(app, ui);
            show_speed(app, ui);

            // Полоса перемотки забирает всё, что осталось между краями.
            let width =
                (ui.available_width() - ui.spacing().item_spacing.x).max(MIN_TIMELINE_WIDTH);
            show_timeline(app, ui, width);
        });
    });
}

/// Кнопка открытия файла — иконкой приложения.
fn show_open_button(app: &mut PithApp, ui: &mut egui::Ui) {
    let clicked = match crate::ui::logo_texture(ui.ctx()) {
        Some(texture) => ui
            .add(
                egui::Button::image(
                    egui::Image::new(&texture).fit_to_exact_size(egui::vec2(ICON_SIZE, ICON_SIZE)),
                )
                .frame(false),
            )
            .on_hover_text("Открыть файл")
            .clicked(),
        // Иконка не загрузилась — остаётся подпись, кнопка обязана работать.
        None => ui
            .add_sized(BUTTON_SIZE, egui::Button::new("Открыть"))
            .on_hover_text("Открыть файл")
            .clicked(),
    };

    if clicked {
        app.open_file_dialog();
    }
}

/// Пауза и продолжение — одной кнопкой постоянного размера.
///
/// Размер задан явно: у «▶» и «❚❚» разная ширина, и кнопка прыгала
/// при каждой паузе.
fn show_play_button(app: &mut PithApp, ui: &mut egui::Ui) {
    let paused = app.engine().map(|e| e.state().paused).unwrap_or(true);
    let label = if paused { "▶" } else { "❚❚" };

    if ui
        .add_sized(BUTTON_SIZE, egui::Button::new(label))
        .on_hover_text("Пауза / продолжить (пробел)")
        .clicked()
    {
        app.toggle_pause();
    }
}

/// Кнопка полноэкранного режима.
///
/// Значок рисуется сам: подходящего знака в шрифтах egui нет, вместо
/// иконки выходил пустой квадрат.
fn show_fullscreen_button(app: &mut PithApp, ui: &mut egui::Ui) {
    let hint = if app.is_fullscreen() {
        "Выйти из полного экрана (F)"
    } else {
        "Во весь экран (F)"
    };

    let (rect, response) = ui.allocate_exact_size(BUTTON_SIZE.into(), egui::Sense::click());

    let color = if response.hovered() {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_SECONDARY
    };

    paint_fullscreen_icon(ui.painter(), rect, app.is_fullscreen(), color);

    if response.on_hover_text(hint).clicked() {
        app.toggle_fullscreen(ui.ctx());
    }
}

/// Рисует значок «во весь экран»: рамку с уголками.
///
/// В полноэкранном режиме уголки смотрят внутрь — это привычная подсказка
/// «свернуть обратно».
fn paint_fullscreen_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    fullscreen: bool,
    color: egui::Color32,
) {
    let side = ICON_SIZE * 0.6;
    let frame = egui::Rect::from_center_size(rect.center(), egui::vec2(side, side * 0.75));
    let stroke = egui::Stroke::new(1.4, color);
    let arm = side * 0.28;

    painter.rect_stroke(frame, 1.0, stroke, egui::StrokeKind::Middle);

    // Уголки: наружу — «развернуть», внутрь — «свернуть».
    let corners = [
        (frame.left_top(), egui::vec2(1.0, 1.0)),
        (frame.right_top(), egui::vec2(-1.0, 1.0)),
        (frame.left_bottom(), egui::vec2(1.0, -1.0)),
        (frame.right_bottom(), egui::vec2(-1.0, -1.0)),
    ];

    for (corner, inward) in corners {
        let direction = if fullscreen { inward } else { -inward };
        painter.line_segment(
            [corner, corner + egui::vec2(direction.x * arm, 0.0)],
            stroke,
        );
        painter.line_segment(
            [corner, corner + egui::vec2(0.0, direction.y * arm)],
            stroke,
        );
    }
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

/// Громкость — такой же полосой, как перемотка.
///
/// Выкладывается справа налево, поэтому полоса идёт первой, а значок
/// после неё оказывается слева.
fn show_volume(app: &mut PithApp, ui: &mut egui::Ui) {
    let Some(engine) = app.engine() else {
        return;
    };

    let volume = engine.state().volume;

    if let Some(chosen) = timeline::volume_bar(ui, volume, VOLUME_WIDTH, MAX_VOLUME) {
        app.set_volume(chosen);
    }

    ui.label(
        egui::RichText::new(if volume == 0 { "🔇" } else { "🔊" }).color(theme::TEXT_SECONDARY),
    )
    .on_hover_text(format!("Громкость: {volume}%"));
}

/// Предел громкости — тот же, что у движка.
const MAX_VOLUME: i64 = 150;
