//! Фотографии актёров: кадр в строке и раскрытая крупно.
//!
//! Все снимки базы приходят разной высоты, а в списке должны стоять
//! ровным столбцом. Поэтому кадр постоянный, а картинка в него вписывается
//! **с обрезкой**, а не растягиванием: растянутая фотография сплющивает
//! лица, и это первое, что бросается в глаза.

use crate::app::{PhotoPreview, PhotoSize, PithApp};
use crate::theme;
use crate::tr;
use crate::ui::icons;

/// Отношение сторон кадра — как у снимков базы.
///
/// TMDB отдаёт портреты 2:3. Кадр той же формы обрезает у большинства
/// снимков ровно ничего, а лишнее забирает поровну с боков.
pub const ASPECT: f32 = 2.0 / 3.0;

/// Скругление кадра.
const RADIUS: f32 = 4.0;

/// Какую долю окна занимает раскрытая фотография.
const PREVIEW_SHARE: f32 = 0.8;

/// Сколько раскрытая картинка не принимает нажатий, секунды.
///
/// Ровно чтобы пережить тот кадр, в котором её и раскрыли.
const SETTLE: f64 = 0.15;

/// Рисует фотографию в кадре, обрезая лишнее по краям.
///
/// Нет текстуры — рисуется углубление того же размера: без него подписи
/// прыгали бы, когда картинки подгружаются одна за другой.
pub fn draw(ui: &egui::Ui, rect: egui::Rect, texture: Option<&egui::TextureHandle>) {
    let Some(texture) = texture else {
        ui.painter().rect_filled(rect, RADIUS, theme::PANEL_SUNKEN);
        return;
    };

    ui.painter().image(
        texture.id(),
        rect,
        cover_uv(texture.size_vec2(), rect.size()),
        egui::Color32::WHITE,
    );
}

/// Какую часть картинки показать, чтобы она заполнила кадр без искажений.
///
/// Лишнее срезается поровну с двух сторон: лицо на портрете стоит по
/// середине, и обрезка от середины его не задевает.
fn cover_uv(source: egui::Vec2, target: egui::Vec2) -> egui::Rect {
    let full = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

    if source.x <= 0.0 || source.y <= 0.0 || target.x <= 0.0 || target.y <= 0.0 {
        return full;
    }

    let source_ratio = source.x / source.y;
    let target_ratio = target.x / target.y;

    if (source_ratio - target_ratio).abs() < f32::EPSILON {
        return full;
    }

    if source_ratio > target_ratio {
        // Картинка шире кадра — срезаем бока.
        let keep = target_ratio / source_ratio;
        let margin = (1.0 - keep) / 2.0;

        egui::Rect::from_min_max(egui::pos2(margin, 0.0), egui::pos2(1.0 - margin, 1.0))
    } else {
        // Картинка выше кадра — срезаем верх и низ.
        let keep = source_ratio / target_ratio;
        let margin = (1.0 - keep) / 2.0;

        egui::Rect::from_min_max(egui::pos2(0.0, margin), egui::pos2(1.0, 1.0 - margin))
    }
}

/// Раскрытая фотография поверх списка.
///
/// Рисуется последней и во весь размер окна: список под ней приглушён,
/// и промахнуться мимо картинки — значит закрыть её.
pub fn show_preview(app: &mut PithApp, ctx: &egui::Context) {
    let Some(preview) = app.actors_state().preview.clone() else {
        return;
    };

    // Крупная едет отдельной загрузкой, и приехать может не сразу — или
    // не приехать вовсе, если сети нет. Мелкая к этому времени уже лежит
    // в кэше: растянутая, она хуже, но это лучше пустого прямоугольника,
    // из которого не понять, сломалось что-то или просто не загрузилось.
    let large = app.actor_photo(ctx, &preview.path, PhotoSize::Large);
    let texture = large
        .clone()
        .or_else(|| app.actor_photo(ctx, &preview.path, PhotoSize::Row));

    // Надпись нужна только когда показывать нечего вовсе. Поверх уже
    // видимой картинки она лишь мешает: человек и так видит лицо, а «идёт
    // загрузка» относится к замене на более крупную — дело служебное.
    let waiting = texture.is_none();
    let screen = ctx.input(|i| i.viewport_rect());

    let mut close = false;

    egui::Area::new(egui::Id::new("actor_photo_preview"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.set_min_size(screen.size());

            let painter = ui.painter();
            painter.rect_filled(screen, 0.0, theme::PANEL_CARD.gamma_multiply(0.92));

            close = show_photo(ui, screen, &preview, texture.as_ref(), waiting);
        });

    if close {
        app.close_actor_photo();
    }
}

/// Сама картинка с подписью. Возвращает `true`, если её просят закрыть.
fn show_photo(
    ui: &mut egui::Ui,
    screen: egui::Rect,
    preview: &PhotoPreview,
    texture: Option<&egui::TextureHandle>,
    waiting: bool,
) -> bool {
    let rect = frame_rect(screen);

    draw(ui, rect, texture);

    ui.painter().text(
        egui::pos2(rect.center().x, rect.bottom() + 10.0),
        egui::Align2::CENTER_TOP,
        &preview.label,
        egui::FontId::proportional(14.0),
        theme::TEXT_PRIMARY,
    );

    // Пока крупная едет — говорим об этом. Молчащий кадр читается как
    // «сломалось», особенно когда показана растянутая мелкая.
    if waiting {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            tr!("Загружаю…", "Loading…"),
            egui::FontId::proportional(13.0),
            theme::TEXT_SECONDARY,
        );
    }

    let cross = show_cross(ui, rect);
    let pointer = ui.input(|i| i.pointer.hover_pos());

    // Нажатие, которым картинку раскрыли, лежит во входных данных этого же
    // кадра. Без выдержки оно закрыло бы её сразу же — картинка не успевала
    // показаться ни на кадр.
    let settled = ui.input(|i| i.time) - preview.opened > SETTLE;

    let clicked_outside = settled
        && ui.input(|i| i.pointer.any_click())
        && pointer.is_none_or(|pos| !rect.contains(pos) || cross.contains(pos));

    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));

    clicked_outside || escape
}

/// Куда встанет картинка: доля окна, форма кадра сохраняется.
fn frame_rect(screen: egui::Rect) -> egui::Rect {
    /// Место под подпись снизу.
    const CAPTION: f32 = 28.0;

    let available = egui::vec2(
        screen.width() * PREVIEW_SHARE,
        screen.height() * PREVIEW_SHARE - CAPTION,
    );

    // Вписываем кадр 2:3 в отведённое место, не выходя за него ни одной
    // стороной, — иначе на узком окне картинка вылезет за край.
    let height = available.y.min(available.x / ASPECT).max(1.0);
    let size = egui::vec2(height * ASPECT, height);

    egui::Rect::from_center_size(
        egui::pos2(screen.center().x, screen.center().y - CAPTION / 2.0),
        size,
    )
}

/// Крестик в углу картинки. Возвращает свою область — по ней отличают
/// нажатие на него от нажатия мимо.
fn show_cross(ui: &mut egui::Ui, photo: egui::Rect) -> egui::Rect {
    /// Сторона крестика.
    const SIDE: f32 = 28.0;

    let rect = egui::Rect::from_min_size(
        egui::pos2(photo.right() - SIDE - 6.0, photo.top() + 6.0),
        egui::vec2(SIDE, SIDE),
    );

    let hovered = ui
        .input(|i| i.pointer.hover_pos())
        .is_some_and(|p| rect.contains(p));
    let fill = if hovered {
        theme::PANEL_ELEMENT_HOVER
    } else {
        theme::PANEL_ELEMENT
    };

    ui.painter().rect_filled(rect, RADIUS, fill);

    // Знак берём из системного набора: в шрифтах egui крестика нет,
    // и на его месте выходит пустой квадрат.
    let (glyph, font) = icons::CLOSE.painted(14.0);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        font,
        theme::TEXT_PRIMARY,
    );

    rect
}

#[cfg(test)]
mod tests {
    use super::cover_uv;

    /// Допуск сравнения долей: считаем в дробных числах.
    const EPS: f32 = 1e-4;

    #[test]
    fn такая_же_форма_берётся_целиком() {
        let uv = cover_uv(egui::vec2(200.0, 300.0), egui::vec2(40.0, 60.0));

        assert_eq!(uv.min, egui::pos2(0.0, 0.0));
        assert_eq!(uv.max, egui::pos2(1.0, 1.0));
    }

    #[test]
    fn широкая_картинка_режется_по_бокам() {
        // Квадрат в кадре 2:3: остаться должно две трети ширины.
        let uv = cover_uv(egui::vec2(300.0, 300.0), egui::vec2(40.0, 60.0));

        assert!(
            (uv.width() - 2.0 / 3.0).abs() < EPS,
            "ширина {}",
            uv.width()
        );
        assert_eq!(uv.height(), 1.0, "по высоте не режем");
        assert!(
            (uv.min.x - (1.0 - uv.width()) / 2.0).abs() < EPS,
            "срез поровну с двух сторон"
        );
    }

    #[test]
    fn высокая_картинка_режется_сверху_и_снизу() {
        // Полоса 1:3 в кадре 2:3: по высоте остаётся половина.
        let uv = cover_uv(egui::vec2(100.0, 300.0), egui::vec2(40.0, 60.0));

        assert_eq!(uv.width(), 1.0, "по ширине не режем");
        assert!((uv.height() - 0.5).abs() < EPS, "высота {}", uv.height());
        assert!((uv.min.y - 0.25).abs() < EPS, "срез поровну сверху и снизу");
    }

    #[test]
    fn пустой_размер_картинку_не_режет() {
        let uv = cover_uv(egui::vec2(0.0, 0.0), egui::vec2(40.0, 60.0));

        assert_eq!(uv.width(), 1.0);
        assert_eq!(uv.height(), 1.0);
    }
}
