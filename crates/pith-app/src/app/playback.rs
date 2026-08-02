//! Управление воспроизведением: перемотка, пауза, громкость, скорость.
//!
//! Вынесено из основного файла приложения ради предела в 400 строк
//! (CLAUDE.md).

use super::PithApp;

impl PithApp {
    /// Разбирает ошибку воспроизведения от mpv.
    ///
    /// Битый файл, отсутствующий кодек, файл удалили во время просмотра —
    /// плеер обязан остаться живым и сказать, что случилось (PLAN.md,
    /// чек-лист этапа 6). Причину mpv не сообщает, поэтому различаем сами:
    /// пропавший с диска файл — самый частый случай.
    pub(super) fn handle_playback_error(&mut self) {
        let missing = self
            .current_path
            .as_ref()
            .is_some_and(|path| !path.exists());

        let message = if missing {
            "Файл пропал с диска"
        } else {
            "Не удалось воспроизвести файл"
        };

        tracing::error!(файл = ?self.current_path, missing, "{message}");

        // Позицию просмотра не трогаем: файл может вернуться (сетевой диск,
        // переподключённая флешка), и досмотреть его нужно с того же места.
        self.report_playback_error(message);
    }

    /// Сообщает о неудаче: всплывашкой и надписью посреди окна.
    pub(super) fn report_playback_error(&mut self, message: &str) {
        self.playback_error = Some(message.to_string());
        self.show_notice(message);
    }

    /// Почему не играет текущий файл.
    pub fn playback_error(&self) -> Option<&str> {
        self.playback_error.as_deref()
    }

    /// Перемотка относительно текущей позиции с замером длительности.
    pub fn seek_relative(&mut self, seconds: f64) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        self.metrics.mark_seek_start();
        self.seek_pending = true;

        if let Err(e) = engine.seek_relative(seconds) {
            tracing::warn!(error = %e, "перемотка не удалась");
            self.seek_pending = false;
        }
    }

    /// Перемотка на абсолютную позицию.
    pub fn seek_absolute(&mut self, seconds: f64) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        self.metrics.mark_seek_start();
        self.seek_pending = true;

        if let Err(e) = engine.seek_absolute(seconds) {
            tracing::warn!(error = %e, "перемотка не удалась");
            self.seek_pending = false;
        }
    }

    pub fn toggle_pause(&mut self) {
        if let Some(engine) = self.engine.as_mut()
            && let Err(e) = engine.toggle_pause()
        {
            tracing::warn!(error = %e, "не удалось переключить паузу");
        }
    }

    pub fn adjust_volume(&mut self, delta: i64) {
        if let Some(engine) = self.engine.as_mut() {
            let target = engine.state().volume + delta;
            if let Err(e) = engine.set_volume(target) {
                tracing::warn!(error = %e, "не удалось изменить громкость");
            }
        }
    }

    pub fn set_volume(&mut self, volume: i64) {
        if let Some(engine) = self.engine.as_mut()
            && let Err(e) = engine.set_volume(volume)
        {
            tracing::warn!(error = %e, "не удалось изменить громкость");
        }
    }

    /// Меняет скорость на `delta` относительно текущей.
    pub fn adjust_speed(&mut self, delta: f64) {
        if let Some(engine) = self.engine.as_mut() {
            let target = engine.state().speed + delta;
            if let Err(e) = engine.set_speed(target) {
                tracing::warn!(error = %e, "не удалось изменить скорость");
            }
        }
    }

    /// Задаёт скорость воспроизведения.
    pub fn set_speed(&mut self, speed: f64) {
        if let Some(engine) = self.engine.as_mut()
            && let Err(e) = engine.set_speed(speed)
        {
            tracing::warn!(error = %e, "не удалось задать скорость");
        }
    }

    /// Возвращает обычную скорость воспроизведения.
    pub fn reset_speed(&mut self) {
        if let Some(engine) = self.engine.as_mut()
            && let Err(e) = engine.set_speed(1.0)
        {
            tracing::warn!(error = %e, "не удалось сбросить скорость");
        }
    }

    /// Диалог выбора файла.
    pub fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Видео и аудио",
                &[
                    "mkv", "mp4", "avi", "mov", "webm", "ts", "m2ts", "m4v", "flv", "wmv", "mpg",
                    "mpeg", "vob", "ogv", "3gp", "mp3", "flac", "wav", "aac", "m4a", "opus",
                ],
            )
            .add_filter("Все файлы", &["*"])
            .pick_file()
        {
            self.open_file(&path.to_string_lossy());
        }
    }
}
