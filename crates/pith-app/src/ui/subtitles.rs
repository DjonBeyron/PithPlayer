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

    let response = egui::Area::new(egui::Id::new(layer.id()))
        .order(egui::Order::Middle)
        .current_pos(anchor)
        .pivot(egui::Align2::CENTER_CENTER)
        .movable(true)
        .show(ctx, |ui| {
            ui.set_max_width(screen.width() * MAX_WIDTH_FRACTION);

            egui::Frame::NONE
                .fill(theme::SUBTITLE_BG)
                .inner_margin(PADDING)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(line)
                            .size(layout.font_size)
                            .color(theme::SUBTITLE_TEXT)
                            .strong(),
                    );
                });
        })
        .response;

    handle_interaction(app, ctx, layer, line, &response, screen);
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
        let center = response.rect.center();
        app.set_subtitle_layout_position(
            layer,
            (center.x - screen.min.x) / screen.width().max(1.0),
            (center.y - screen.min.y) / screen.height().max(1.0),
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
