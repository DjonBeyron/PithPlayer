//! Левый край панели отрезков: за него её растягивают, там же язычок,
//! которым её убирают.
//!
//! Отдельно от самой панели: у края своя забота — ширина и её память,
//! и держать это вперемешку со списком закладок незачем.

use crate::app::PithApp;
use crate::theme;
use crate::tr;

/// Толщина линии, которую видно при перетаскивании.
///
/// В одну точку: это подсказка «край здесь», а не украшение. Широкая
/// полоса читалась бы как часть разметки и делила бы окно надвое.
const GRIP_WIDTH: f32 = 1.0;

/// На сколько область захвата шире видимой линии с каждой стороны.
///
/// Хватать нужно шире, чем видно: линия в точку — цель, по которой мышью
/// не попасть. Зона захвата заходит и на видео слева, и на поле панели.
const GRAB_MARGIN: f32 = 7.0;

/// На каком расстоянии от края появляются полоса и язычок.
///
/// Тянуться к шестипиксельной линии неудобно, поэтому зона шире её самой.
const EDGE_ZONE: f32 = 28.0;

/// Размер язычка, которым панель убирают.
const TAB_WIDTH: f32 = 18.0;
const TAB_HEIGHT: f32 = 56.0;

/// Половина ширины и высоты стрелки на язычке.
const ARROW_HALF_WIDTH: f32 = 3.5;
const ARROW_HALF_HEIGHT: f32 = 6.0;

/// Левый край панели: за него её растягивают, и там же язычок закрытия.
///
/// Ширина запоминается между запусками, но на диск ложится не на каждое
/// движение мышью, а когда край отпустят: файл настроек — не журнал.
pub(super) fn show(app: &mut PithApp, ui: &mut egui::Ui, screen: egui::Rect, left: f32) {
    let rect = egui::Rect::from_min_size(
        egui::pos2(left, screen.min.y),
        egui::vec2(GRIP_WIDTH, screen.height()),
    );

    // Хватать можно шире, чем видно: линия в точку — цель, по которой
    // мышью не попасть. Но зона захвата целиком **внутри** панели: выйди
    // она левее, на видео, — и нажатие там считалось бы «мимо панели»,
    // а панель закрывалась бы прямо под рукой.
    let grab = egui::Rect::from_min_size(
        egui::pos2(left, screen.min.y),
        egui::vec2(GRIP_WIDTH + GRAB_MARGIN * 2.0, screen.height()),
    );

    let response = ui.interact(
        grab,
        egui::Id::new("bookmarks_panel_grip"),
        egui::Sense::drag(),
    );

    if response.dragged()
        && let Some(pointer) = ui.ctx().input(|i| i.pointer.interact_pos())
    {
        app.set_bookmarks_panel_width(screen.max.x - pointer.x, screen.width());
    }

    if response.drag_stopped() {
        app.store_bookmarks_panel_width();
    }

    let near = pointer_near_edge(ui.ctx(), left);

    // Линия показывается только во время перетаскивания и цветом не кричит:
    // о том, что край здесь, уже сказали курсор и язычок. Цветная полоса
    // на всю высоту окна спорила бы с самим списком.
    if response.dragged() {
        ui.painter().rect_filled(rect, 0.0, theme::PANEL_BORDER);
    }

    response.on_hover_cursor(egui::CursorIcon::ResizeHorizontal);

    if near {
        show_close_tab(app, ui, screen, left);
    }
}

/// Курсор у левого края панели.
///
/// По этому признаку появляются и полоса, и язычок: тянуться к тонкой
/// линии, которой не видно, неудобно, а показывать её всегда — значит
/// рисовать посреди окна черту без смысла.
fn pointer_near_edge(ctx: &egui::Context, left: f32) -> bool {
    let Some(pointer) = ctx.input(|i| i.pointer.hover_pos()) else {
        return false;
    };

    (pointer.x - left).abs() <= EDGE_ZONE
}

/// Язычок закрытия на краю панели.
///
/// Панель убирают нажатием мимо неё — но растянутой во всю ширину окна
/// «мимо» почти не остаётся, а меню, которым её тоже можно убрать, уходит
/// под неё. Язычок и есть выход: он всегда на краю самой панели.
fn show_close_tab(app: &mut PithApp, ui: &mut egui::Ui, screen: egui::Rect, left: f32) {
    let rect = egui::Rect::from_min_size(
        egui::pos2(left - TAB_WIDTH, screen.center().y - TAB_HEIGHT / 2.0),
        egui::vec2(TAB_WIDTH, TAB_HEIGHT),
    );

    let response = ui.interact(
        rect,
        egui::Id::new("bookmarks_panel_close"),
        egui::Sense::click(),
    );

    let hovered = response.hovered();
    let strength = if hovered { 1.0 } else { 0.75 };

    ui.painter().rect_filled(
        rect,
        // Скругляем только левые углы: правые упираются в саму панель.
        egui::CornerRadius {
            nw: 6,
            sw: 6,
            ne: 0,
            se: 0,
        },
        theme::PANEL_ELEMENT.gamma_multiply(strength),
    );

    // Стрелка вправо — «убрать за край». Рисуем кистью: знака в шрифте
    // может не оказаться, и вместо него выходит пустой квадрат.
    let center = rect.center();
    let arrow = vec![
        egui::pos2(center.x + ARROW_HALF_WIDTH, center.y),
        egui::pos2(center.x - ARROW_HALF_WIDTH, center.y - ARROW_HALF_HEIGHT),
        egui::pos2(center.x - ARROW_HALF_WIDTH, center.y + ARROW_HALF_HEIGHT),
    ];

    let color = if hovered {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_SECONDARY
    };

    ui.painter().add(egui::Shape::convex_polygon(
        arrow,
        color,
        egui::Stroke::NONE,
    ));

    if response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tr!("Убрать панель", "Hide the panel"))
        .clicked()
    {
        app.hide_bookmarks_panel();
    }
}
