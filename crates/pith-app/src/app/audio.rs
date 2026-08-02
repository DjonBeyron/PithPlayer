//! Выбор устройства вывода звука (PLAN.md, этап 6).

use pith_mpv::{AUTO_DEVICE, AudioDevice};

use super::PithApp;

impl PithApp {
    /// Устройства вывода, доступные сейчас.
    ///
    /// Список запрашивается у mpv при каждом открытии меню: наушники
    /// втыкают и вынимают по ходу просмотра, кэш быстро устарел бы.
    pub fn audio_devices(&self) -> Vec<AudioDevice> {
        self.engine
            .as_ref()
            .map(pith_mpv::Engine::audio_devices)
            .unwrap_or_default()
    }

    /// Имя выбранного устройства.
    pub fn current_audio_device(&self) -> String {
        self.engine
            .as_ref()
            .map(pith_mpv::Engine::audio_device)
            .unwrap_or_else(|| AUTO_DEVICE.to_string())
    }

    /// Переключает вывод звука и запоминает выбор.
    ///
    /// Перезапуск не нужен — mpv пересоздаёт звуковой выход на лету.
    pub fn choose_audio_device(&mut self, name: &str) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        if let Err(e) = engine.set_audio_device(name) {
            tracing::warn!(error = %e, устройство = name, "не удалось переключить вывод звука");
            self.show_notice("Не удалось переключить вывод звука");
            return;
        }

        // «auto» не храним: пусть на новой машине mpv решает сам.
        self.settings.audio_device = if name == AUTO_DEVICE {
            None
        } else {
            Some(name.to_string())
        };
        self.settings.save(&self.data_paths);

        self.show_notice("Вывод звука переключён");
    }
}
