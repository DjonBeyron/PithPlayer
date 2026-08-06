//! Язычок у правого края, которым вызывается панель отрезков.
//!
//! Панель больше не выезжает от одного наведения: так она появлялась
//! незваной — курсор шёл к кнопкам полного экрана, пересекал край экрана
//! или просто проходил мимо. Теперь у края показывается маленький
//! треугольник, и панель открывается нажатием на него.

use crate::app::PithApp;
use crate::theme;

/// Какую долю ширины окна занимает зона, в которой виден язычок.
///
/// Пятая часть: треть окна язычок занимал слишком рано — он появлялся,
/// когда курсор шёл совсем в другое место.
const ZONE_FRACTION: f32 = 1.0 / 5.0;

/// Высота нижней полосы, где язычок не показывается.
///
/// Там панель управления, и её кнопки стоят у того же правого края:
/// язычок поверх них только мешал бы попадать по ним мышью.
const CONTROLS_ZONE: f32 = 90.0;

/// Размер язычка.
const WIDTH: f32 = 18.0;
const HEIGHT: f32 = 56.0;

/// Половина ширины и высоты самого треугольника.
const ARROW_HALF_WIDTH: f32 = 3.5;
const ARROW_HALF_HEIGHT: f32 = 6.0;

/// Показывает язычок, если курсор в правой трети окна.
pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    // Открытой панели язычок не нужен: закрывают её нажатием мимо.
    if app.bookmarks_panel_open() {
        return;
    }

    if !pointer_in_zone(ctx) {
        return;
    }

    let area = egui::Area::new(egui::Id::new("bookmarks_handle"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::RIGHT_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(WIDTH, HEIGHT), egui::Sense::click());

            paint(ui, rect, response.hovered());

            response.on_hover_text(crate::tr!("Отрезки и закладки", "Fragments and bookmarks"))
        });

    if crate::ui::clicked(&area.inner) {
        app.open_bookmarks_panel();
    }
}

/// Рисует плашку со стрелкой влево — «потяни отсюда».
fn paint(ui: &egui::Ui, rect: egui::Rect, hovered: bool) {
    let strength = if hovered { 0.95 } else { 0.6 };

    let painter = ui.painter();
    painter.rect_filled(
        rect,
        // Скругляем только левые углы: правые уходят за край окна.
        egui::CornerRadius {
            nw: 6,
            sw: 6,
            ne: 0,
            se: 0,
        },
        theme::PANEL_BG.gamma_multiply(strength),
    );

    let center = rect.center();
    let arrow = vec![
        egui::pos2(center.x - ARROW_HALF_WIDTH, center.y),
        egui::pos2(center.x + ARROW_HALF_WIDTH, center.y - ARROW_HALF_HEIGHT),
        egui::pos2(center.x + ARROW_HALF_WIDTH, center.y + ARROW_HALF_HEIGHT),
    ];

    let color = if hovered {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_SECONDARY
    };

    painter.add(egui::Shape::convex_polygon(
        arrow,
        color,
        egui::Stroke::NONE,
    ));
}

/// Курсор в правой трети окна и выше панели управления.
fn pointer_in_zone(ctx: &egui::Context) -> bool {
    let screen = ctx.input(|i| i.viewport_rect());

    let Some(pointer) = ctx.input(|i| i.pointer.hover_pos()) else {
        return false;
    };

    pointer.x >= screen.max.x - screen.width() * ZONE_FRACTION
        && pointer.y < screen.max.y - CONTROLS_ZONE
}
