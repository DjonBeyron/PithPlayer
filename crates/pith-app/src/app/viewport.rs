//! Управление окном: подгонка под форму видео, полный экран, активность мыши.
//!
//! Вынесено из основного файла приложения, чтобы тот не разрастался
//! (CLAUDE.md: не более 400 строк на файл).

use super::PithApp;
use crate::window;

impl PithApp {
    pub fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    pub fn last_pointer_activity(&self) -> f64 {
        self.last_pointer_activity
    }

    pub fn toggle_fullscreen(&mut self, ctx: &egui::Context) {
        self.fullscreen = !self.fullscreen;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));

        // Подгонка размера в полноэкранном режиме бессмысленна, а при
        // возврате в окно пользователь ожидает прежний размер.
        self.expected_window_size = None;

        tracing::debug!(fullscreen = self.fullscreen, "полноэкранный режим");
    }

    /// Подгоняет окно под форму видео.
    ///
    /// Любое сомнение — окно не трогаем: mpv сам вписывает кадр по
    /// пропорциям, и это корректное поведение (PLAN.md §6.12).
    pub(super) fn fit_window_to_video(&mut self, ctx: &egui::Context) {
        if !self.fit_window_pending {
            return;
        }
        self.fit_window_pending = false;

        if !self.fit_window_enabled || self.fullscreen || self.window_resized_by_user {
            return;
        }

        let Some(engine) = self.engine.as_ref() else {
            return;
        };

        let state = engine.state();
        let monitor = ctx.input(|i| i.viewport().monitor_size);
        let available = monitor.unwrap_or_else(|| ctx.input(|i| i.viewport_rect().size()));

        let Some(size) = window::fit_size(state.display_width, state.display_height, available)
        else {
            tracing::debug!("подгонка окна пропущена — оставляем как есть");
            return;
        };

        tracing::debug!(?size, "подгоняю окно под форму видео");
        self.expected_window_size = Some(size);
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));

        // Центрируем на мониторе, если знаем его размер.
        if let Some(monitor) = monitor {
            let position = ((monitor - size) / 2.0).max(egui::Vec2::ZERO);
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position.to_pos2()));
        }
    }

    /// Запоминает момент последнего движения мыши.
    ///
    /// По нему в полноэкранном режиме прячется панель управления.
    pub(super) fn track_pointer_activity(&mut self, ctx: &egui::Context) {
        let moved = ctx.input(|i| {
            i.pointer.velocity().length() > 0.0
                || i.pointer.any_click()
                || i.smooth_scroll_delta.length() > 0.0
        });

        if moved {
            self.last_pointer_activity = ctx.input(|i| i.time);
        }
    }

    /// Замечает ручное изменение размера окна, чтобы больше не навязывать
    /// подгонку до смены файла.
    pub(super) fn track_manual_resize(&mut self, ctx: &egui::Context) {
        let Some(expected) = self.expected_window_size else {
            return;
        };

        let current = ctx.input(|i| i.viewport_rect().size());

        // Небольшое расхождение — округления оконной системы, а не действие
        // пользователя.
        if (current - expected).abs().max_elem() > 4.0 {
            self.window_resized_by_user = true;
            self.expected_window_size = None;
            tracing::debug!("размер окна изменён вручную — подгонку отключаю");
        }
    }
}
