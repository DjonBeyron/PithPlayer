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

/// На сколько точек расходятся оттиски текста при жирном начертании.
///
/// Настоящего жирного начертания у вложенных шрифтов нет, поэтому строка
/// печатается несколько раз со сдвигом в доли точки — так утолщаются сами
/// штрихи, а форма букв остаётся прежней.
const BOLD_SPREAD: f32 = 0.7;

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

    /// Строка примера — её показывает окно настройки вида.
    fn sample(self) -> &'static str {
        match self {
            Self::Main => crate::tr!(
                "Так выглядят основные субтитры",
                "This is how the main subtitles look"
            ),
            Self::Secondary => crate::tr!("А так — вторые", "And these are the second ones"),
        }
    }
}

/// Рисует оба слоя субтитров.
pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    // Пока настраивают вид, слои видны всегда: иначе цвет пришлось бы
    // подбирать вслепую — на паузе или в тишине реплик просто нет.
    let sample = app.subtitle_style_open();

    if !app.settings().subtitles_visible && !sample {
        return;
    }

    let text = app.subtitle_text().clone();

    if let Some(line) = line_of(text.main, Layer::Main, sample) {
        show_layer(app, ctx, Layer::Main, &line);
    }
    if let Some(line) = line_of(text.secondary, Layer::Secondary, sample) {
        show_layer(app, ctx, Layer::Secondary, &line);
    }
}

/// Что показать в слое: текущую реплику, а при настройке — хотя бы пример.
fn line_of(current: Option<String>, layer: Layer, sample: bool) -> Option<String> {
    current.or_else(|| sample.then(|| layer.sample().to_string()))
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
                .show(ui, |ui| paint_line(ui, line, layer, layout))
                .inner
        });

    let label = response.inner;

    if label.hovered() || label.dragged() {
        ctx.set_cursor_icon(egui::CursorIcon::Grab);
    }

    handle_interaction(app, ctx, layer, line, &label, screen);
}

/// Рисует строку выбранным цветом и начертанием.
///
/// Кистью, а не обычной надписью: жирное начертание получается несколькими
/// оттисками со сдвигом, а `Label` печатает строку один раз.
fn paint_line(
    ui: &mut egui::Ui,
    line: &str,
    layer: Layer,
    layout: pith_store::SubtitleLayout,
) -> egui::Response {
    // У слоёв разные шрифты: их читают одновременно, и начертание различает
    // слои надёжнее, чем одно лишь положение на экране. Оба шрифта вложены
    // в программу, так что вид не зависит от машины.
    let family = match layer {
        Layer::Main => crate::fonts::main_subtitle_family(),
        Layer::Secondary => crate::fonts::secondary_subtitle_family(),
    };

    let [red, green, blue] = layout.color;
    let color = egui::Color32::from_rgb(red, green, blue);

    let galley = ui.painter().layout(
        line.to_owned(),
        egui::FontId::new(layout.font_size, family),
        color,
        ui.available_width(),
    );

    // Тащат и щёлкают по всей строке, поэтому отклик берёт её целиком.
    let (rect, response) = ui.allocate_exact_size(galley.size(), egui::Sense::click_and_drag());

    let painter = ui.painter();

    if layout.bold {
        for offset in BOLD_OFFSETS {
            painter.galley(rect.min + offset * BOLD_SPREAD, galley.clone(), color);
        }
    }

    painter.galley(rect.min, galley, color);

    response
}

/// Куда сдвигаются оттиски жирной строки.
const BOLD_OFFSETS: [egui::Vec2; 4] = [
    egui::vec2(1.0, 0.0),
    egui::vec2(-1.0, 0.0),
    egui::vec2(0.0, 1.0),
    egui::vec2(0.0, -1.0),
];

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
