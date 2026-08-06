//! Подгонка окна под соотношение сторон видео (PLAN.md §6.12).
//!
//! Главное правило — безопасность: любое сомнение означает «окно не трогаем».
//! Тогда остаётся исходное поведение, при котором mpv сам вписывает кадр
//! с полями по краям. Это корректно, просто не идеально.

/// Какую долю рабочей области занимать максимум.
const MAX_SCREEN_FRACTION: f32 = 0.9;

/// Минимальная ширина окна: уже неё панель управления не помещается даже
/// в сокращённом виде.
///
/// Ограничение только по ширине — вертикальное видео заведомо ниже своей
/// высоты, и требовать от него «минимальной высоты» бессмысленно.
const MIN_WINDOW_WIDTH: f32 = 380.0;

/// Размеры кадра, признаваемые правдоподобными.
const MIN_REASONABLE_SIDE: i64 = 16;
const MAX_REASONABLE_SIDE: i64 = 16384;

/// Считает размер окна под кадр `width`×`height`.
///
/// Возвращает `None`, если подгонять не нужно или не из чего:
/// это штатный исход, а не ошибка.
pub fn fit_size(width: i64, height: i64, available: egui::Vec2) -> Option<egui::Vec2> {
    if !is_reasonable(width) || !is_reasonable(height) {
        return None;
    }

    if available.x <= 0.0 || available.y <= 0.0 {
        return None;
    }

    let frame = egui::vec2(width as f32, height as f32);
    let limit = available * MAX_SCREEN_FRACTION;

    // Только уменьшаем: увеличивать мелкое видео — значит растягивать мыло.
    let scale = (limit.x / frame.x).min(limit.y / frame.y).min(1.0);
    let fitted = frame * scale;

    if fitted.x < MIN_WINDOW_WIDTH {
        return None;
    }

    Some(fitted)
}

fn is_reasonable(side: i64) -> bool {
    (MIN_REASONABLE_SIDE..=MAX_REASONABLE_SIDE).contains(&side)
}

/// Насколько форма окна может расходиться с формой кадра, прежде чем
/// это станет заметно полями.
///
/// Два процента — это пара точек на окне в тысячу: столько даёт округление
/// оконной системы, и ради него окно двигать не стоит.
const ASPECT_TOLERANCE: f32 = 0.02;

/// Подгоняет форму уже открытого окна под кадр, сохраняя его величину.
///
/// Нужно, когда окно восстановлено с прошлого запуска: размер пользователь
/// выбрал сам и менять его незачем, а вот форма от нового файла своя —
/// иначе mpv вписывает кадр с чёрными полями по краям.
///
/// `None` — форма уже подходит либо подгонять нечего.
pub fn reshape(
    current: egui::Vec2,
    width: i64,
    height: i64,
    available: egui::Vec2,
) -> Option<egui::Vec2> {
    if !is_reasonable(width) || !is_reasonable(height) {
        return None;
    }

    if current.x <= 0.0 || current.y <= 0.0 || available.x <= 0.0 || available.y <= 0.0 {
        return None;
    }

    let aspect = width as f32 / height as f32;
    let current_aspect = current.x / current.y;

    if (current_aspect / aspect - 1.0).abs() < ASPECT_TOLERANCE {
        return None;
    }

    // Высоту оставляем прежней: по ней пользователь и выбирал размер.
    let mut fitted = egui::vec2(current.y * aspect, current.y);

    // Если по ширине окно не помещается на экране, уменьшаем целиком.
    let limit = available * MAX_SCREEN_FRACTION;
    let scale = (limit.x / fitted.x).min(limit.y / fitted.y).min(1.0);
    fitted *= scale;

    if fitted.x < MIN_WINDOW_WIDTH {
        return None;
    }

    Some(fitted)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Рабочая область обычного экрана 1920×1080.
    fn screen() -> egui::Vec2 {
        egui::vec2(1920.0, 1040.0)
    }

    #[test]
    fn вертикальное_видео_даёт_вертикальное_окно() {
        let size = fit_size(1080, 1920, screen()).expect("должно подогнать");
        assert!(
            size.y > size.x,
            "окно обязано быть выше своей ширины, получено {size:?}"
        );
    }

    #[test]
    fn сохраняет_пропорции_кадра() {
        let size = fit_size(1080, 1920, screen()).expect("должно подогнать");
        let expected = 1080.0 / 1920.0;
        let actual = size.x / size.y;
        assert!(
            (actual - expected).abs() < 0.01,
            "пропорции искажены: {actual} вместо {expected}"
        );
    }

    #[test]
    fn горизонтальное_видео_вписывается_в_экран() {
        let size = fit_size(3840, 2160, screen()).expect("должно подогнать");
        assert!(size.x <= screen().x, "окно шире экрана: {size:?}");
        assert!(size.y <= screen().y, "окно выше экрана: {size:?}");
    }

    #[test]
    fn форма_окна_правится_под_кадр() {
        // Окно осталось от широкого фильма, открыли вертикальное видео.
        let size = reshape(egui::vec2(1280.0, 720.0), 1080, 1920, screen())
            .expect("форму нужно поправить");

        let expected = 1080.0 / 1920.0;
        assert!(
            (size.x / size.y - expected).abs() < 0.01,
            "форма не совпала с кадром: {size:?}"
        );
        assert_eq!(size.y, 720.0, "высоту менять незачем");
    }

    #[test]
    fn подходящую_форму_не_трогаем() {
        // Окно уже 16:9 — открывать такое же видео и дёргать окно не нужно.
        assert!(reshape(egui::vec2(1280.0, 720.0), 1920, 1080, screen()).is_none());
    }

    #[test]
    fn форма_не_выводит_окно_за_экран() {
        // Сверхширокий кадр при высоком окне: ширина упёрлась бы в экран.
        let size =
            reshape(egui::vec2(600.0, 900.0), 3840, 1080, screen()).expect("форму нужно поправить");

        assert!(size.x <= screen().x, "окно шире экрана: {size:?}");
        assert!(size.y <= screen().y, "окно выше экрана: {size:?}");
    }

    #[test]
    fn слишком_узкую_форму_пропускаем() {
        // Вертикальное видео в низком окне дало бы окно уже панели управления.
        assert!(reshape(egui::vec2(900.0, 400.0), 1080, 1920, screen()).is_none());
    }

    #[test]
    fn мелкое_видео_не_растягивается() {
        let size = fit_size(640, 480, screen()).expect("должно подогнать");
        assert_eq!(size, egui::vec2(640.0, 480.0));
    }

    #[test]
    fn слишком_узкое_видео_пропускается() {
        // Окно вышло бы уже минимума — панель управления не поместится.
        assert!(fit_size(200, 240, screen()).is_none());
    }

    #[test]
    fn обычное_вертикальное_видео_с_телефона_подгоняется() {
        let size = fit_size(1080, 1920, screen()).expect("должно подогнать");
        assert!(size.y > size.x, "окно не вертикальное: {size:?}");
        assert!(size.y <= screen().y, "не влезло по высоте: {size:?}");
    }

    #[test]
    fn высокое_вертикальное_видео_подгоняется() {
        // Ограничение по ширине не должно отсекать высокое видео:
        // панель управления в узком окне показывается сокращённой.
        let size = fit_size(1080, 2300, screen()).expect("должно подогнать");
        assert!(size.y <= screen().y, "не влезло по высоте: {size:?}");
        assert!(size.x >= MIN_WINDOW_WIDTH, "слишком узкое окно: {size:?}");
        assert!(size.y > size.x, "окно не вертикальное: {size:?}");
    }

    #[test]
    fn нулевые_размеры_кадра_пропускаются() {
        assert!(fit_size(0, 0, screen()).is_none());
        assert!(fit_size(1920, 0, screen()).is_none());
    }

    #[test]
    fn абсурдные_размеры_пропускаются() {
        assert!(fit_size(-100, 200, screen()).is_none());
        assert!(fit_size(99999, 99999, screen()).is_none());
        assert!(fit_size(2, 2, screen()).is_none());
    }

    #[test]
    fn пустая_рабочая_область_пропускается() {
        assert!(fit_size(1920, 1080, egui::vec2(0.0, 0.0)).is_none());
    }

    #[test]
    fn слишком_узкий_результат_пропускается() {
        // Экран-полоска: подогнанное окно вышло бы меньше разумного минимума.
        let narrow = egui::vec2(200.0, 2000.0);
        assert!(fit_size(3840, 2160, narrow).is_none());
    }
}
