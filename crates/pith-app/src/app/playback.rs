//! Управление воспроизведением: перемотка, пауза, громкость, скорость.
//!
//! Вынесено из основного файла приложения ради предела в 400 строк
//! (CLAUDE.md).

use super::PithApp;

impl PithApp {
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
