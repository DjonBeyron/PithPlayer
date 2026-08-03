//! Положение и размер окна между запусками.
//!
//! Плеер должен открываться там же, где его закрыли: на втором мониторе,
//! той же ширины и высоты. Без этого каждое видео на втором экране
//! приходится перетаскивать заново.

use pith_store::WindowGeometry;

use super::PithApp;

impl PithApp {
    /// Запоминает текущее положение окна в настройках.
    ///
    /// Вызывается при закрытии: писать настройки на каждое движение мышью
    /// незачем, а размер к моменту выхода уже устоялся.
    pub(super) fn remember_window_geometry(&mut self) {
        let Some(geometry) = self.window_geometry else {
            return;
        };

        if self.settings.window == Some(geometry) {
            return;
        }

        self.settings.window = Some(geometry);
        self.settings.save(&self.data_paths);
        tracing::info!(?geometry, "положение окна запомнено");
    }

    /// Следит за фактическим положением окна.
    ///
    /// Полноэкранный режим пропускаем: возвращаться из него нужно
    /// в прежнее окно, а не в размер экрана.
    pub(super) fn track_window_geometry(&mut self, ctx: &egui::Context) {
        if self.fullscreen {
            return;
        }

        // Положение берём внешнее, размер — внутренний: именно так их
        // и задают при создании окна. Смешаешь — окно будет расти на
        // высоту заголовка при каждом запуске.
        let (outer, inner) = ctx.input(|i| (i.viewport().outer_rect, i.viewport().inner_rect));

        let (Some(outer), Some(inner)) = (outer, inner) else {
            return;
        };

        // Свёрнутое окно Windows отдаёт с бессмысленными координатами.
        if inner.width() < 1.0 || inner.height() < 1.0 {
            return;
        }

        self.window_geometry = Some(WindowGeometry {
            x: outer.min.x,
            y: outer.min.y,
            width: inner.width(),
            height: inner.height(),
        });
    }

    /// Возвращает окно на экран, если сохранённое место исчезло.
    ///
    /// Второй монитор могли отключить, и окно оказалось бы за пределами
    /// рабочего стола — доступным только через клавиатуру.
    pub(super) fn ensure_window_on_screen(&mut self, ctx: &egui::Context) {
        if !self.window_position_pending {
            return;
        }
        self.window_position_pending = false;

        let (outer, monitor) = ctx.input(|i| (i.viewport().outer_rect, i.viewport().monitor_size));

        let (Some(rect), Some(monitor)) = (outer, monitor) else {
            return;
        };

        // Монитор, на котором окно сейчас, начинается не в нуле, если он
        // не основной. Судим по перекрытию: полностью пустое пересечение
        // означает, что окна не видно.
        let visible = rect.max.x > 0.0
            && rect.max.y > 0.0
            && rect.min.x < monitor.x * MONITOR_SPAN
            && rect.min.y < monitor.y * MONITOR_SPAN;

        if visible {
            return;
        }

        tracing::warn!(?rect, ?monitor, "окно вне экрана, возвращаю в центр");
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
            (monitor.x - rect.width()) / 2.0,
            (monitor.y - rect.height()) / 2.0,
        )));
    }
}

/// Сколько экранов вправо и вниз считаем допустимым расположением.
///
/// Мониторы выстраиваются в ряд, и координаты соседнего кратно больше
/// размеров основного. Пятикратный запас покрывает разумные сборки.
const MONITOR_SPAN: f32 = 5.0;
