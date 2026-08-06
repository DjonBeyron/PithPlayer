//! Громкость в панели управления.
//!
//! В широком окне полоса стоит в строке рядом с динамиком. В узком окне
//! места на неё нет — остаётся один динамик, а полоса открывается по
//! нажатию и стоит вертикально.

use crate::app::PithApp;
use crate::theme;
use crate::tr;
use crate::ui::controls::{BUTTON_SIZE, icon_button};
use crate::ui::{icons, timeline};

/// Предел громкости — тот же, что у движка.
const MAX_VOLUME: i64 = 150;

/// Ширина полосы в широком окне.
const WIDTH: f32 = 110.0;

/// Высота вертикальной полосы.
const POPUP_HEIGHT: f32 = 130.0;

/// Ширина всплывающего окна с полосой.
const POPUP_WIDTH: f32 = 46.0;

/// Полоса громкости и динамик.
///
/// `compact` — узкое окно: полоса прячется под динамик.
pub fn show(app: &mut PithApp, ui: &mut egui::Ui, compact: bool) {
    let Some(engine) = app.engine() else {
        return;
    };

    let volume = engine.state().volume;
    let muted = engine.state().muted;

    if compact {
        show_menu(app, ui, volume, muted);
        return;
    }

    // При выключенном звуке полоса пустая: заполненная рядом с
    // перечёркнутым динамиком противоречила бы сама себе.
    let shown = if muted { 0 } else { volume };

    // Выкладка идёт справа налево, поэтому полоса рисуется первой,
    // а динамик после неё оказывается слева.
    if let Some(chosen) = horizontal_bar(ui, shown, WIDTH) {
        apply(app, chosen, muted);
    }

    // Значок — кнопка, а не картинка: раньше по нему нажимали, ожидая
    // тишины, и ничего не происходило.
    let (icon, hint) = speaker(volume, muted);

    if icon_button(ui, icon, &hint) {
        app.toggle_muted();
    }
}

/// Динамик с выпадающей полосой — для узкого окна.
fn show_menu(app: &mut PithApp, ui: &mut egui::Ui, volume: i64, muted: bool) {
    let (icon, _) = speaker(volume, muted);

    let button = egui::Button::new(icon.text().color(theme::TEXT_PRIMARY))
        .frame(false)
        .min_size(egui::vec2(BUTTON_SIZE[0], BUTTON_SIZE[1]));

    let (response, _) = egui::containers::menu::MenuButton::from_button(button)
        .ui(ui, |ui| show_popup(app, ui, volume, muted));

    // Выключение звука одним нажатием никуда не делось: в широком окне
    // оно на самом динамике, здесь — на правом щелчке по нему.
    if response.secondary_clicked() {
        app.toggle_muted();
    }

    let hint = if muted {
        tr!(
            "Громкость. Правым щелчком — включить звук",
            "Volume. Right-click to unmute"
        )
    } else {
        tr!(
            "Громкость. Правым щелчком — выключить звук",
            "Volume. Right-click to mute"
        )
    };

    response.on_hover_text(hint);
}

/// Содержимое всплывающего окна: проценты и вертикальная полоса.
fn show_popup(app: &mut PithApp, ui: &mut egui::Ui, volume: i64, muted: bool) {
    let shown = if muted { 0 } else { volume };

    ui.set_min_width(POPUP_WIDTH);

    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new(format!("{shown}%"))
                .color(theme::TEXT_SECONDARY)
                .small(),
        );

        if let Some(chosen) = vertical_bar(ui, shown, POPUP_HEIGHT) {
            apply(app, chosen, muted);
        }
    });
}

/// Ставит выбранную громкость.
fn apply(app: &mut PithApp, volume: i64, muted: bool) {
    // Тронули полосу — звук возвращаем: громкость просят именно эту.
    if muted {
        app.toggle_muted();
    }

    app.set_volume(volume);
}

/// Значок динамика и подсказка к нему.
fn speaker(volume: i64, muted: bool) -> (icons::Icon, String) {
    if muted || volume == 0 {
        (icons::MUTE, tr!("Включить звук", "Unmute").to_string())
    } else {
        (
            icons::VOLUME,
            tr!(
                format!("Выключить звук ({volume}%)"),
                format!("Mute ({volume}%)")
            ),
        )
    }
}

/// Полоса громкости — той же высоты и в том же стиле, что перемотка.
///
/// Стандартный ползунок egui выглядит рядом с ней чужеродно: другая
/// высота, свой кружок, свои отступы.
///
/// Возвращает новое значение, когда пользователь его меняет.
fn horizontal_bar(ui: &mut egui::Ui, volume: i64, width: f32) -> Option<i64> {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(width, timeline::HIT_HEIGHT),
        egui::Sense::click_and_drag(),
    );

    // Высота постоянная, в отличие от полосы перемотки: громкость стоит
    // рядом с ней в одну линию, и утолщение под курсором ломало ряд.
    let track =
        egui::Rect::from_center_size(rect.center(), egui::vec2(width, timeline::TRACK_HEIGHT));

    let filled = paint_track(ui, track, volume);

    if response.hovered() || response.dragged() {
        let knob_x = track.min.x + track.width() * filled;
        paint_knob(ui, egui::pos2(knob_x, track.center().y));
    }

    let pointer = pointer_of(&response)?;

    if track.width() <= 0.0 {
        return None;
    }

    let ratio = ((pointer.x - track.min.x) / track.width()).clamp(0.0, 1.0);
    Some(chosen_volume(ratio))
}

/// Та же полоса, поставленная стоймя: верх — громче.
fn vertical_bar(ui: &mut egui::Ui, volume: i64, height: f32) -> Option<i64> {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(timeline::HIT_HEIGHT, height),
        egui::Sense::click_and_drag(),
    );

    let track =
        egui::Rect::from_center_size(rect.center(), egui::vec2(timeline::TRACK_HEIGHT, height));

    let filled = paint_track(ui, track, volume);

    if response.hovered() || response.dragged() {
        let knob_y = track.max.y - track.height() * filled;
        paint_knob(ui, egui::pos2(track.center().x, knob_y));
    }

    let pointer = pointer_of(&response)?;

    if track.height() <= 0.0 {
        return None;
    }

    let ratio = ((track.max.y - pointer.y) / track.height()).clamp(0.0, 1.0);
    Some(chosen_volume(ratio))
}

/// Рисует полосу и её заполненную часть. Возвращает долю заполнения.
fn paint_track(ui: &egui::Ui, track: egui::Rect, volume: i64) -> f32 {
    let painter = ui.painter();
    let vertical = track.height() > track.width();
    let radius = if vertical {
        track.width() / 2.0
    } else {
        track.height() / 2.0
    };

    painter.rect_filled(track, radius, theme::TIMELINE_TRACK);

    let filled = (volume as f32 / MAX_VOLUME as f32).clamp(0.0, 1.0);

    if filled > 0.0 {
        let played = if vertical {
            egui::Rect::from_min_max(
                egui::pos2(track.min.x, track.max.y - track.height() * filled),
                track.max,
            )
        } else {
            egui::Rect::from_min_max(
                track.min,
                egui::pos2(track.min.x + track.width() * filled, track.max.y),
            )
        };

        painter.rect_filled(played, radius, theme::ACCENT.gamma_multiply(0.75));
    }

    filled
}

/// Кружок на текущем значении.
fn paint_knob(ui: &egui::Ui, center: egui::Pos2) {
    ui.painter()
        .circle_filled(center, timeline::KNOB_RADIUS, theme::TEXT_PRIMARY);
}

/// Место курсора, если полосу прямо сейчас двигают.
fn pointer_of(response: &egui::Response) -> Option<egui::Pos2> {
    if !(response.dragged() || response.clicked()) {
        return None;
    }

    response
        .interact_pointer_pos()
        .or_else(|| response.hover_pos())
}

/// Громкость, соответствующая доле полосы.
fn chosen_volume(ratio: f32) -> i64 {
    (ratio * MAX_VOLUME as f32).round() as i64
}
