//! Слои субтитров поверх видео.
//!
//! Рисуются самостоятельно, а не средствами mpv: только так остаются
//! перетаскивание, изменение размера и копирование реплики (PLAN.md §6.2).

use crate::app::PithApp;
use crate::theme;

/// Отступ текста внутри подложки.
const PADDING: f32 = 8.0;
/// Доля ширины окна, которую занимает строка субтитров максимум.
const MAX_WIDTH_FRACTION: f32 = 0.9;

/// Какой слой субтитров рисуем.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Main,
    Secondary,
}

impl Layer {
    fn id(self) -> &'static str {
        match self {
            Self::Main => "subtitle_main",
            Self::Secondary => "subtitle_secondary",
        }
    }
}

/// Рисует оба слоя субтитров.
pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if !app.settings().subtitles_visible {
        return;
    }

    let text = app.subtitle_text().clone();

    if let Some(line) = text.main {
        show_layer(app, ctx, Layer::Main, &line);
    }
    if let Some(line) = text.secondary {
        show_layer(app, ctx, Layer::Secondary, &line);
    }
}

fn show_layer(app: &mut PithApp, ctx: &egui::Context, layer: Layer, line: &str) {
    let screen = ctx.input(|i| i.viewport_rect());
    let layout = app.subtitle_layout(layer);

    // Положение хранится долей окна, чтобы не уезжать при смене размера.
    let anchor = egui::pos2(
        screen.min.x + screen.width() * layout.x,
        screen.min.y + screen.height() * layout.y,
    );

    // Область не двигаем средствами egui: положение задаётся настройками
    // и переписывалось бы на каждом кадре. Перетаскивание ведём сами.
    let response = egui::Area::new(egui::Id::new(layer.id()))
        .order(egui::Order::Middle)
        .fixed_pos(anchor)
        .pivot(egui::Align2::CENTER_CENTER)
        .movable(false)
        .show(ctx, |ui| {
            ui.set_max_width(screen.width() * MAX_WIDTH_FRACTION);

            egui::Frame::NONE
                .fill(theme::SUBTITLE_BG)
                .inner_margin(PADDING)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    // Вторые субтитры — другим шрифтом: их читают поверх
                    // основных, и различать слои по начертанию проще, чем
                    // по одному лишь положению.
                    let mut text = egui::RichText::new(line)
                        .size(layout.font_size)
                        .color(theme::SUBTITLE_TEXT)
                        .strong();

                    if layer == Layer::Secondary {
                        text = text.family(crate::fonts::secondary_subtitle_family());
                    }

                    ui.add(
                        egui::Label::new(text)
                            // Иначе при попытке потащить субтитры выделяется текст.
                            .selectable(false)
                            .sense(egui::Sense::click_and_drag()),
                    )
                })
                .inner
        });

    let label = response.inner;

    if label.hovered() || label.dragged() {
        ctx.set_cursor_icon(egui::CursorIcon::Grab);
    }

    handle_interaction(app, ctx, layer, line, &label, screen);
}

/// Перетаскивание, изменение размера колесом и копирование по щелчку.
fn handle_interaction(
    app: &mut PithApp,
    ctx: &egui::Context,
    layer: Layer,
    line: &str,
    response: &egui::Response,
    screen: egui::Rect,
) {
    if response.dragged() {
        // Двигаем на смещение мыши, а не пересчитываем из центра области:
        // размер подложки меняется вместе с длиной реплики, и центр
        // прыгал бы на каждой новой строке.
        let delta = response.drag_delta();
        app.move_subtitle_layout(
            layer,
            delta.x / screen.width().max(1.0),
            delta.y / screen.height().max(1.0),
        );
    }

    if response.drag_stopped() {
        app.save_settings();
    }

    // Колесо над субтитрами меняет размер шрифта.
    if response.hovered() {
        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            app.adjust_subtitle_font_size(layer, scroll.signum());
        }
    }

    // Щелчок копирует реплику — как клавиша C, но мышью.
    if response.clicked() {
        app.copy_text_to_clipboard(line);
    }
}
