//! Значок воспроизведения и паузы в центре кадра.
//!
//! Нажали паузу — в середине появляется знак воспроизведения и остаётся
//! висеть: по нему видно, что плеер стоит. Нажали «играть» — знак меняется
//! на паузу, расходится кругом и гаснет.
//!
//! Нужен во всех режимах, но особенно в полноэкранном: там панель
//! управления спрятана, и другого способа увидеть, что произошло, нет.

use crate::app::PithApp;
use crate::theme;
use crate::ui::icons;

/// Какую долю меньшей стороны окна занимает радиус кружка.
///
/// Значок растёт вместе с окном: на весь экран прежние 44 точки терялись
/// посреди кадра.
const RADIUS_FRACTION: f32 = 0.07;

/// Пределы радиуса: в крошечном окне значок не должен закрывать кадр,
/// а на большом экране — теряться.
const MIN_RADIUS: f32 = 34.0;
const MAX_RADIUS: f32 = 84.0;

/// Доля радиуса, которую занимает сам знак.
const ICON_FRACTION: f32 = 0.78;

/// Насколько значок разрастается, уходя.
const GROWTH: f32 = 0.45;

/// Насколько он «приезжает» при появлении.
const APPEAR_FROM: f32 = 0.75;

/// Плотность подложки под значком.
const BACKDROP: f32 = 0.55;

/// Рисует значок состояния, если ему есть что показать.
///
/// На паузе значок ещё и кнопка: по нажатию воспроизведение продолжается.
/// Рисовать знак воспроизведения и не отзываться на нажатие — обманывать
/// пользователя.
pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    let Some(badge) = app.playback_badge() else {
        return;
    };

    let progress = (badge.age / PithApp::badge_fade_seconds()).clamp(0.0, 1.0) as f32;

    // Замедление к концу: резкий скачок в начале и мягкая остановка
    // читаются как нажатие, а не как мигание.
    let eased = 1.0 - (1.0 - progress).powi(3);

    let (scale, alpha) = if badge.paused {
        // Появление: значок подрастает до своего размера и остаётся.
        (APPEAR_FROM + (1.0 - APPEAR_FROM) * eased, eased)
    } else {
        // Уход: расходится и гаснет.
        (1.0 + GROWTH * eased, 1.0 - eased)
    };

    let icon = if badge.paused {
        icons::PLAY
    } else {
        icons::PAUSE
    };

    let radius = badge_radius(ctx);

    // Нажимать имеет смысл только по стоящему значку: уходящий гаснет
    // за доли секунды, и попасть по нему нельзя, а перехватывать нажатия
    // по кадру он не должен.
    let clickable = badge.paused;

    let area = egui::Area::new(egui::Id::new("playback_badge"))
        // Слоем ниже окон: значок относится к кадру, а не к диалогу.
        // В общем слое он всплывал поверх открытых настроек — область
        // рисуется позже них и в своём слое оказывается сверху.
        .order(egui::Order::Middle)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .interactable(clickable)
        .show(ctx, |ui| {
            // Место берём с запасом на разрастание: иначе значок обрежется
            // на самом заметном кадре.
            let side = radius * 2.0 * (1.0 + GROWTH);
            let sense = if clickable {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            };
            let (rect, response) = ui.allocate_exact_size(egui::vec2(side, side), sense);

            let painter = ui.painter();
            painter.circle_filled(
                rect.center(),
                radius * scale,
                theme::PANEL_BG.gamma_multiply(BACKDROP * alpha),
            );

            let (glyph, font) = icon.painted(radius * ICON_FRACTION * scale);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                glyph,
                font,
                theme::TEXT_PRIMARY.gamma_multiply(alpha),
            );

            response.on_hover_cursor(egui::CursorIcon::PointingHand)
        });

    if crate::ui::clicked(&area.inner) {
        app.toggle_pause();
    }

    // Пока идёт всплеск, кадры нужны и на паузе — сама она их не даёт.
    if progress < 1.0 {
        ctx.request_repaint();
    }
}

/// Радиус значка для текущего окна.
fn badge_radius(ctx: &egui::Context) -> f32 {
    let screen = ctx.input(|i| i.viewport_rect().size());
    let side = screen.x.min(screen.y);

    (side * RADIUS_FRACTION).clamp(MIN_RADIUS, MAX_RADIUS)
}

#[cfg(test)]
mod tests {
    use super::{MAX_RADIUS, MIN_RADIUS, RADIUS_FRACTION};

    /// Тот же расчёт, что и в `badge_radius`, но без контекста egui.
    fn radius(width: f32, height: f32) -> f32 {
        (width.min(height) * RADIUS_FRACTION).clamp(MIN_RADIUS, MAX_RADIUS)
    }

    #[test]
    fn значок_растёт_вместе_с_окном() {
        let small = radius(640.0, 480.0);
        let big = radius(2560.0, 1440.0);

        assert!(big > small, "на большом экране значок обязан быть крупнее");
    }

    #[test]
    fn размер_не_выходит_за_пределы() {
        assert_eq!(radius(200.0, 150.0), MIN_RADIUS, "в крошечном окне");
        assert_eq!(radius(5120.0, 2880.0), MAX_RADIUS, "на огромном экране");
    }
}
