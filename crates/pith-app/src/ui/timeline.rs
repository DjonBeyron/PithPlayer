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
/// На полосе показывается цветом своего списка, чтобы было видно, что
/// именно вырежется и к какому списку относится.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FragmentRange {
    pub start: f64,
    pub end: f64,
    pub color: egui::Color32,
}

impl FragmentRange {
    /// Диапазон фрагмента для закладки.
    ///
    /// `bookmark` — время метки, `buffer` — отступ назад от неё,
    /// `duration` — длительность фрагмента. Всё в секундах.
    pub fn from_bookmark(bookmark: f64, buffer: f64, duration: f64, color: egui::Color32) -> Self {
        let start = (bookmark - buffer).max(0.0);
        Self {
            start,
            end: start + duration.max(0.0),
            color,
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
/// `fragments` — отрезки будущих фрагментов, каждый цветом своего списка.
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

/// Рисует цветные отрезки будущих фрагментов.
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

        painter.rect_filled(rect, 0.0, fragment.color);
    }
}

/// Полоса громкости — той же высоты и в том же стиле, что перемотка.
///
/// Стандартный ползунок egui выглядит рядом с ней чужеродно: другая
/// высота, свой кружок, свои отступы.
///
/// Возвращает новое значение, когда пользователь его меняет.
pub fn volume_bar(ui: &mut egui::Ui, volume: i64, width: f32, max: i64) -> Option<i64> {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, HIT_HEIGHT), egui::Sense::click_and_drag());

    let hovered = response.hovered() || response.dragged();

    // Высота постоянная, в отличие от полосы перемотки: громкость стоит
    // рядом с ней в одну линию, и утолщение под курсором ломало ряд.
    let track_height = TRACK_HEIGHT;

    let track = egui::Rect::from_center_size(rect.center(), egui::vec2(width, track_height));
    let painter = ui.painter();
    let radius = track_height / 2.0;

    painter.rect_filled(track, radius, theme::TIMELINE_TRACK);

    let filled = (volume as f32 / max as f32).clamp(0.0, 1.0);
    if filled > 0.0 {
        let played =
            egui::Rect::from_min_size(track.min, egui::vec2(track.width() * filled, track_height));
        painter.rect_filled(played, radius, theme::ACCENT.gamma_multiply(0.75));
    }

    if hovered {
        let knob_x = track.min.x + track.width() * filled;
        painter.circle_filled(
            egui::pos2(knob_x, track.center().y),
            KNOB_RADIUS,
            theme::TEXT_PRIMARY,
        );
    }

    if !(response.dragged() || response.clicked()) {
        return None;
    }

    let pointer = response
        .interact_pointer_pos()
        .or_else(|| response.hover_pos())?;

    if track.width() <= 0.0 {
        return None;
    }

    let ratio = ((pointer.x - track.min.x) / track.width()).clamp(0.0, 1.0);
    Some((ratio * max as f32).round() as i64)
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
    use crate::theme;

    fn track() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(200.0, 5.0))
    }

    /// Отрезок с цветом первого списка — цвет в расчёте не участвует.
    fn range(bookmark: f64, buffer: f64, duration: f64) -> FragmentRange {
        FragmentRange::from_bookmark(bookmark, buffer, duration, theme::list_color(0))
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
        // Метка на 15-й секунде, длительность 10 с — цветом будет 15…25.
        let range = range(15.0, 0.0, 10.0);
        assert_eq!(range.start, 15.0);
        assert_eq!(range.end, 25.0);
    }

    #[test]
    fn отступ_сдвигает_отрезок_назад() {
        // Метка на 60-й, отступ 10 с, длительность 30 с — отрезок 50…80.
        let range = range(60.0, 10.0, 30.0);
        assert_eq!(range.start, 50.0);
        assert_eq!(range.end, 80.0);
    }

    #[test]
    fn отрезок_не_уходит_за_начало_файла() {
        // Метка на 3-й секунде с отступом 10 с — начало обрезается нулём.
        let range = range(3.0, 10.0, 20.0);
        assert_eq!(range.start, 0.0);
        assert_eq!(range.end, 20.0);
    }

    #[test]
    fn нулевая_длительность_даёт_пустой_отрезок() {
        let range = range(15.0, 0.0, 0.0);
        assert_eq!(range.start, range.end);
    }

    #[test]
    fn у_каждого_списка_свой_цвет() {
        // Полоса красит отрезок цветом из самого `FragmentRange`, поэтому
        // разным спискам достаточно разных цветов в палитре.
        assert_ne!(theme::list_color(0), theme::list_color(1));
        assert_eq!(
            theme::list_color(0),
            theme::list_color(6),
            "после шести списков цвета идут по кругу"
        );
    }
}
