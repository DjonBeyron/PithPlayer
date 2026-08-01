//! Полоса перемотки в стиле v4.
//!
//! Своя отрисовка вместо стандартного ползунка: на этапе 4 сюда лягут
//! метки закладок, а стандартный виджет этого не позволяет.

use crate::theme;

/// Высота полосы в покое.
const TRACK_HEIGHT: f32 = 5.0;
/// Высота полосы под курсором.
const TRACK_HEIGHT_HOVER: f32 = 8.0;
/// Радиус кружка на текущей позиции.
const KNOB_RADIUS: f32 = 7.0;
/// Высота области захвата — по ней ловится клик мимо тонкой полосы.
const HIT_HEIGHT: f32 = 22.0;

/// Что пользователь сделал с полосой.
#[derive(Default)]
pub struct TimelineResponse {
    /// Куда перемотать, секунды. Заполняется по отпусканию мыши.
    pub seek_to: Option<f64>,
    /// Позиция под курсором — для всплывающей подсказки со временем.
    pub hovered_time: Option<f64>,
}

/// Рисует полосу перемотки и возвращает действия пользователя.
pub fn show(ui: &mut egui::Ui, position: f64, duration: f64, width: f32) -> TimelineResponse {
    let mut result = TimelineResponse::default();

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, HIT_HEIGHT), egui::Sense::click_and_drag());

    if duration <= 0.0 {
        return result;
    }

    let hovered = response.hovered() || response.dragged();
    let track_height = if hovered {
        TRACK_HEIGHT_HOVER
    } else {
        TRACK_HEIGHT
    };

    let track = egui::Rect::from_center_size(rect.center(), egui::vec2(width, track_height));
    let painter = ui.painter();
    let radius = track_height / 2.0;

    // Фон полосы.
    painter.rect_filled(track, radius, theme::TIMELINE_TRACK);

    // Пройденная часть.
    let progress = (position / duration).clamp(0.0, 1.0) as f32;
    if progress > 0.0 {
        let played = egui::Rect::from_min_size(
            track.min,
            egui::vec2(track.width() * progress, track_height),
        );
        painter.rect_filled(played, radius, theme::ACCENT);
    }

    // Кружок текущей позиции — только когда полоса под курсором.
    if hovered {
        let knob_x = track.min.x + track.width() * progress;
        painter.circle_filled(
            egui::pos2(knob_x, track.center().y),
            KNOB_RADIUS,
            theme::TEXT_PRIMARY,
        );
    }

    // Время под курсором.
    if let Some(pointer) = response.hover_pos() {
        result.hovered_time = Some(time_at(pointer.x, track, duration));
    }

    // Перемотка по отпусканию: на 4К перемотка за каждый пиксель движения
    // сделала бы интерфейс неотзывчивым.
    if response.drag_stopped() || response.clicked() {
        let pointer = response
            .interact_pointer_pos()
            .or_else(|| response.hover_pos());

        if let Some(pointer) = pointer {
            result.seek_to = Some(time_at(pointer.x, track, duration));
        }
    }

    result
}

/// Время, соответствующее координате X на полосе.
fn time_at(x: f32, track: egui::Rect, duration: f64) -> f64 {
    if track.width() <= 0.0 {
        return 0.0;
    }

    let ratio = ((x - track.min.x) / track.width()).clamp(0.0, 1.0);
    (ratio as f64 * duration).clamp(0.0, duration)
}

#[cfg(test)]
mod tests {
    use super::time_at;

    fn track() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(200.0, 5.0))
    }

    #[test]
    fn начало_полосы_даёт_ноль() {
        assert_eq!(time_at(100.0, track(), 60.0), 0.0);
    }

    #[test]
    fn середина_полосы_даёт_половину() {
        assert_eq!(time_at(200.0, track(), 60.0), 30.0);
    }

    #[test]
    fn клик_за_границами_обрезается() {
        assert_eq!(time_at(0.0, track(), 60.0), 0.0);
        assert_eq!(time_at(9999.0, track(), 60.0), 60.0);
    }

    #[test]
    fn нулевая_ширина_не_делит_на_ноль() {
        let empty = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(0.0, 5.0));
        assert_eq!(time_at(50.0, empty, 60.0), 0.0);
    }
}
