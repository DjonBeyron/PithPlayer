//! Окошко предпросмотра над полосой перемотки.
//!
//! Показывает кадр того места, где стоит курсор. Откуда кадр берётся —
//! из мозаики миниатюр или от второго экземпляра mpv — решает
//! `app::preview`; здесь только показ.

use crate::app::PithApp;
use crate::theme;
use crate::ui::{format_time, format_time_padded, timeline};

/// Высота места под кадр. Ширина считается по форме самого видео.
///
/// Место занимается всегда, даже пока кадра нет: иначе окно меняет
/// размер под каждый новый кадр и мигает под курсором.
const HEIGHT: f32 = 90.0;

/// Наименьшая ширина окошка: столько занимает подпись со временем.
///
/// Уже неё окно всё равно не станет, а кадр в нём поедет — лучше
/// не сужать.
const MIN_WIDTH: f32 = 120.0;

/// Наибольшая ширина: очень широкий фильм не должен растягивать
/// подсказку на пол-экрана.
const MAX_WIDTH: f32 = 260.0;

/// Наибольшая высота — предел для вертикального видео.
const MAX_HEIGHT: f32 = 220.0;

/// Отступ содержимого внутри окошка.
const PADDING: i8 = 4;

/// Насколько окошко поднято над полосой.
const GAP: f32 = 10.0;

/// Рисует окошко предпросмотра для места под курсором.
pub fn show(
    app: &PithApp,
    ctx: &egui::Context,
    response: &timeline::TimelineResponse,
    duration: f64,
) {
    let (Some(x), Some(time)) = (response.pointer_x, response.hovered_time) else {
        return;
    };

    let position = egui::pos2(x, response.track_top - GAP);

    egui::Area::new(egui::Id::new("timeline_preview"))
        .order(egui::Order::Tooltip)
        .fixed_pos(position)
        .pivot(egui::Align2::CENTER_BOTTOM)
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(theme::WINDOW_BG.gamma_multiply(0.96))
                .inner_margin(PADDING)
                .corner_radius(4.0)
                .show(ui, |ui| show_contents(app, ui, time, duration));
        });
}

/// Кадр и подпись со временем.
fn show_contents(app: &PithApp, ui: &mut egui::Ui, time: f64, duration: f64) {
    let size = frame_size(app);

    // Размер не пересчитывается под каждый новый кадр: форма видео за
    // фильм не меняется, а иначе подложка мигала бы под курсором.
    ui.set_width(size.x);
    ui.spacing_mut().item_spacing.y = 2.0;

    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());

    // Пока не готов ни один источник, место под кадр уже занято: окно
    // не прыгает, когда картинка приходит.
    if let Some(image) = app.preview_image(time) {
        egui::Image::new(image.texture)
            // Клетка мозаики — часть общей картинки, поэтому берём
            // не всю текстуру, а её кусок.
            .uv(image.uv)
            .corner_radius(3.0)
            // Вписываем по пропорциям: у широкого кадра они не 16:9,
            // и растянутая картинка врала бы о том, что в этом месте фильма.
            .paint_at(ui, fit_inside(rect, image.size));
    }

    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "{} / {}",
                format_time_padded(time, duration),
                format_time(duration)
            ))
            .color(theme::TEXT_PRIMARY)
            .monospace()
            .size(12.0),
        );
    });
}

/// Место под кадр — той же формы, что и само видео.
///
/// Раньше оно было ровно 160×90, и всё, что не 16:9, показывалось
/// с чёрными полями: окошко было шире или выше самого кадра. Форма
/// известна сразу после открытия файла и до конца не меняется, поэтому
/// окно от этого не дёргается.
///
/// Обычный кадр задаёт ширину при постоянной высоте. Узкий (снятый
/// телефоном) упёрся бы в ширину подписи со временем — такому тянем
/// высоту, иначе поля вернулись бы, только по бокам.
fn frame_size(app: &PithApp) -> egui::Vec2 {
    let aspect = app
        .engine()
        .and_then(|engine| engine.state().aspect_ratio())
        .unwrap_or(16.0 / 9.0);

    if aspect <= 0.0 {
        return egui::vec2(MIN_WIDTH, HEIGHT);
    }

    let width = HEIGHT * aspect;

    if width < MIN_WIDTH {
        egui::vec2(MIN_WIDTH, (MIN_WIDTH / aspect).min(MAX_HEIGHT))
    } else if width > MAX_WIDTH {
        egui::vec2(MAX_WIDTH, MAX_WIDTH / aspect)
    } else {
        egui::vec2(width, HEIGHT)
    }
}

/// Вписывает картинку в отведённое место, сохраняя пропорции.
fn fit_inside(area: egui::Rect, image: egui::Vec2) -> egui::Rect {
    if image.x <= 0.0 || image.y <= 0.0 {
        return area;
    }

    let scale = (area.width() / image.x).min(area.height() / image.y);
    egui::Rect::from_center_size(area.center(), image * scale)
}

#[cfg(test)]
mod tests {
    use super::{HEIGHT, MAX_HEIGHT, MAX_WIDTH, MIN_WIDTH, fit_inside};

    /// Размер места под кадр для видео такой формы.
    ///
    /// Повторяет `frame_size` без обращения к движку: сам движок в тестах
    /// не поднять, а считаемое им — только соотношение сторон.
    fn size(aspect: f32) -> egui::Vec2 {
        let width = HEIGHT * aspect;

        if width < MIN_WIDTH {
            egui::vec2(MIN_WIDTH, (MIN_WIDTH / aspect).min(MAX_HEIGHT))
        } else if width > MAX_WIDTH {
            egui::vec2(MAX_WIDTH, MAX_WIDTH / aspect)
        } else {
            egui::vec2(width, HEIGHT)
        }
    }

    /// Полей не осталось, если кадр вписался в место точь-в-точь.
    fn без_полей(aspect: f32) -> bool {
        let area = egui::Rect::from_min_size(egui::Pos2::ZERO, size(aspect));
        let fitted = fit_inside(area, egui::vec2(aspect, 1.0));

        (fitted.width() - area.width()).abs() < 0.5 && (fitted.height() - area.height()).abs() < 0.5
    }

    #[test]
    fn обычный_кадр_заполняет_окошко_целиком() {
        // 16:9, широкий экран 2.39:1, старое телевидение 4:3 и квадрат.
        for aspect in [16.0 / 9.0, 2.39, 4.0 / 3.0, 1.0] {
            assert!(без_полей(aspect), "поля при соотношении {aspect}");
        }
    }

    #[test]
    fn вертикальный_кадр_тянет_окошко_вверх() {
        // Съёмка телефоном: место становится высоким, а не широким.
        let size = size(9.0 / 16.0);
        assert_eq!(size.x, MIN_WIDTH);
        assert!(size.y > HEIGHT, "высота обязана вырасти: {}", size.y);
        assert!(без_полей(9.0 / 16.0));
    }

    #[test]
    fn очень_широкий_кадр_не_растёт_без_предела() {
        let size = size(4.0);
        assert_eq!(size.x, MAX_WIDTH);
        assert!(size.y < HEIGHT, "высота подстраивается под ширину");
    }

    fn area() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(160.0, 90.0))
    }

    #[test]
    fn широкий_кадр_упирается_в_края_по_ширине() {
        let fitted = fit_inside(area(), egui::vec2(320.0, 180.0));
        assert_eq!(fitted.width(), 160.0);
        assert_eq!(fitted.height(), 90.0);
    }

    #[test]
    fn вертикальный_кадр_не_растягивается() {
        // Кадр с телефона: по высоте он и упрётся, по бокам останется место.
        let fitted = fit_inside(area(), egui::vec2(90.0, 160.0));
        assert_eq!(fitted.height(), 90.0);
        assert!(fitted.width() < 160.0, "растянутый кадр врал бы о фильме");
    }

    #[test]
    fn пустой_размер_не_делит_на_ноль() {
        assert_eq!(fit_inside(area(), egui::vec2(0.0, 0.0)), area());
    }
}
