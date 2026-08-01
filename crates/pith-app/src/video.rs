//! Композит видео-кадра mpv в кадр egui.
//!
//! mpv рисует в тот же буфер, что и интерфейс, но раньше него: обратный вызов
//! размещается в фоновом слое, поэтому элементы управления оказываются сверху.

use egui::PaintCallbackInfo;
use pith_mpv::{FrameSize, SharedRenderContext};

/// Добавляет отрисовку видео в переданную область.
///
/// Возвращает `true`, если кадр был запрошен к отрисовке.
pub fn paint(ui: &mut egui::Ui, rect: egui::Rect, context: SharedRenderContext) -> bool {
    if rect.width() < 1.0 || rect.height() < 1.0 {
        return false;
    }

    let callback = egui::PaintCallback {
        rect,
        callback: std::sync::Arc::new(eframe::egui_glow::CallbackFn::new(
            move |info: PaintCallbackInfo, _painter| {
                // Контекст мог быть освобождён при закрытии окна — тогда
                // просто пропускаем кадр.
                let Some(context) = context.upgrade() else {
                    return;
                };

                // Размер именно всего буфера окна, а не области обратного
                // вызова: mpv рисует в нулевой framebuffer целиком и сам
                // вписывает кадр по пропорциям. Если передать меньший
                // размер, картинка займёт лишь часть окна.
                let size =
                    FrameSize::new(info.screen_size_px[0] as i32, info.screen_size_px[1] as i32);

                // Ноль — буфер окна: mpv рисует прямо в текущий кадр.
                if let Err(e) = context.render(0, size) {
                    // Ошибка отрисовки одного кадра не повод останавливать плеер.
                    tracing::warn!(error = %e, "не удалось отрисовать кадр");
                }
            },
        )),
    };

    ui.painter().add(callback);
    true
}
