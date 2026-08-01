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

/// Отрезок, который попадёт в сохранённый фрагмент.
///
/// Считается от закладки: `[метка − отступ, метка − отступ + длительность]`.
/// На полосе показывается жёлтым, чтобы было видно, что именно вырежется.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FragmentRange {
    pub start: f64,
    pub end: f64,
}

impl FragmentRange {
    /// Диапазон фрагмента для закладки.
    ///
    /// `bookmark` — время метки, `buffer` — отступ назад от неё,
    /// `duration` — длительность фрагмента. Всё в секундах.
    ///
    /// Вызывающая сторона появится на этапе 4 вместе с закладками;
    /// сам расчёт готов и покрыт тестами.
    #[allow(dead_code)]
    pub fn from_bookmark(bookmark: f64, buffer: f64, duration: f64) -> Self {
        let start = (bookmark - buffer).max(0.0);
        Self {
            start,
            end: start + duration.max(0.0),
        }
    }
}

/// Что пользователь сделал с полосой.
#[derive(Default)]
pub struct TimelineResponse {
    /// Куда перемотать, секунды. Заполняется по отпусканию мыши.
    pub seek_to: Option<f64>,
    /// Позиция под курсором — для всплывающей подсказки со временем.
    pub hovered_time: Option<f64>,
}

/// Рисует полосу перемотки и возвращает действия пользователя.
///
/// `fragments` — отрезки будущих фрагментов, подсвечиваются жёлтым.
pub fn show(
    ui: &mut egui::Ui,
    position: f64,
    duration: f64,
    width: f32,
    fragments: &[FragmentRange],
) -> TimelineResponse {
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

    // Отрезки будущих фрагментов — под индикатором воспроизведения,
    // чтобы текущая позиция оставалась читаемой.
    paint_fragments(painter, track, duration, fragments);

    // Пройденная часть. Полупрозрачная, чтобы жёлтые отрезки просвечивали.
    let progress = (position / duration).clamp(0.0, 1.0) as f32;
    if progress > 0.0 {
        let played = egui::Rect::from_min_size(
            track.min,
            egui::vec2(track.width() * progress, track_height),
        );
        painter.rect_filled(played, radius, theme::ACCENT.gamma_multiply(0.75));
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

/// Рисует жёлтые отрезки будущих фрагментов.
fn paint_fragments(
    painter: &egui::Painter,
    track: egui::Rect,
    duration: f64,
    fragments: &[FragmentRange],
) {
    for fragment in fragments {
        let from = (fragment.start / duration).clamp(0.0, 1.0) as f32;
        let to = (fragment.end / duration).clamp(0.0, 1.0) as f32;

        if to <= from {
            continue;
        }

        let left = track.min.x + track.width() * from;
        let right = track.min.x + track.width() * to;

        // Совсем узкий отрезок всё равно должен быть заметен.
        let rect = egui::Rect::from_min_max(
            egui::pos2(left, track.min.y),
            egui::pos2(right.max(left + 2.0), track.max.y),
        );

        painter.rect_filled(rect, 0.0, theme::FRAGMENT);
    }
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
    use super::{FragmentRange, time_at};

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

    #[test]
    fn закладка_без_отступа_даёт_отрезок_вперёд() {
        // Метка на 15-й секунде, длительность 10 с — жёлтым будет 15…25.
        let range = FragmentRange::from_bookmark(15.0, 0.0, 10.0);
        assert_eq!(range.start, 15.0);
        assert_eq!(range.end, 25.0);
    }

    #[test]
    fn отступ_сдвигает_отрезок_назад() {
        // Метка на 60-й, отступ 10 с, длительность 30 с — отрезок 50…80.
        let range = FragmentRange::from_bookmark(60.0, 10.0, 30.0);
        assert_eq!(range.start, 50.0);
        assert_eq!(range.end, 80.0);
    }

    #[test]
    fn отрезок_не_уходит_за_начало_файла() {
        // Метка на 3-й секунде с отступом 10 с — начало обрезается нулём.
        let range = FragmentRange::from_bookmark(3.0, 10.0, 20.0);
        assert_eq!(range.start, 0.0);
        assert_eq!(range.end, 20.0);
    }

    #[test]
    fn нулевая_длительность_даёт_пустой_отрезок() {
        let range = FragmentRange::from_bookmark(15.0, 0.0, 0.0);
        assert_eq!(range.start, range.end);
    }
}
