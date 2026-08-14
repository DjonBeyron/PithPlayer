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
            // Место дочерних окон записать всё равно нужно: они могли
            // переехать, пока главное окно стояло на месте.
            self.save_settings();
            return;
        };

        let maximized = self.window_maximized;

        if self.settings.window == Some(geometry) && self.settings.window_maximized == maximized {
            self.save_settings();
            return;
        }

        self.settings.window = Some(geometry);
        self.settings.window_maximized = maximized;
        self.settings.save(&self.data_paths);
        tracing::info!(
            ?geometry,
            развёрнуто = maximized,
            "положение окна запомнено"
        );
    }

    /// Сообщает системе, что окно открыто развёрнутым.
    ///
    /// Делается один раз и не на первом кадре, а когда окно уже показано:
    /// объявленное развёрнутым до показа, оно появлялось раньше срока
    /// и мигало (PLAN.md §6.12).
    pub(super) fn announce_maximized(&mut self, frame: &eframe::Frame) {
        if !self.announce_maximized_pending {
            return;
        }
        self.announce_maximized_pending = false;

        // Сворачивать окно кнопкой заголовка нужно в тот размер, каким его
        // закрыли до разворота, — он записан в настройках.
        let Some(restore) = self.settings.window.filter(|g| g.is_sane()) else {
            return;
        };

        crate::screen::mark_maximized(
            frame,
            egui::Rect::from_min_size(
                egui::pos2(restore.x, restore.y),
                egui::vec2(restore.width, restore.height),
            ),
        );
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

        // Окно во весь экран запоминается как развёрнутое, а размеры его
        // при этом не трогаются: запомнить нужно сам факт, а размеры — те,
        // что были до разворота. В них окно и вернётся.
        //
        // Считаем таким и то, что развернула кнопка заголовка, и то, что мы
        // открыли по рабочей области сами: разворачивать его средствами
        // системы нельзя — она проигрывает разворот с анимацией, и весь
        // интерфейс перестраивается на глазах (PLAN.md §6.12).
        let by_system = ctx.input(|i| i.viewport().maximized).unwrap_or(false);
        self.window_maximized = by_system || fills_work_area(outer);

        if self.window_maximized {
            return;
        }

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

/// Занимает ли окно всю рабочую область своего экрана.
///
/// Такое окно для пользователя ничем не отличается от развёрнутого: оно
/// закрывает экран целиком. Именно им плеер и открывается, когда его
/// закрыли развёрнутым.
fn fills_work_area(outer: egui::Rect) -> bool {
    /// Допуск на рамки окна и округление размеров.
    const TOLERANCE: f32 = 24.0;

    let Some((position, size)) = crate::screen::work_area(outer.center().x, outer.center().y)
    else {
        return false;
    };

    let area = egui::Rect::from_min_size(position, size);

    // Сравниваем по покрытию, а не по совпадению: развёрнутое системой окно
    // выходит за края экрана на толщину рамки, и точного равенства не будет.
    outer.min.x <= area.min.x + TOLERANCE
        && outer.min.y <= area.min.y + TOLERANCE
        && outer.max.x >= area.max.x - TOLERANCE
        && outer.max.y >= area.max.y - TOLERANCE
}

/// Сколько экранов вправо и вниз считаем допустимым расположением.
///
/// Мониторы выстраиваются в ряд, и координаты соседнего кратно больше
/// размеров основного. Пятикратный запас покрывает разумные сборки.
const MONITOR_SPAN: f32 = 5.0;
