//! Выбор устройства вывода звука (PLAN.md, этап 6).

use pith_mpv::{AUTO_DEVICE, AudioDevice};

use super::PithApp;
use crate::tr;

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

    /// Выключен ли звук.
    pub fn is_muted(&self) -> bool {
        self.engine
            .as_ref()
            .is_some_and(|engine| engine.state().muted)
    }

    /// Выключает и включает звук.
    ///
    /// Громкость при этом не трогается: вернув звук, пользователь ждёт
    /// прежнюю громкость, а не ноль и не сто. Выбор запоминается —
    /// выключенный звук переживает перезапуск.
    pub fn toggle_muted(&mut self) {
        let muted = !self.is_muted();

        if let Some(engine) = self.engine.as_mut()
            && let Err(e) = engine.set_muted(muted)
        {
            tracing::warn!(error = %e, "не удалось переключить звук");
            return;
        }

        self.settings.muted = muted;
        self.settings.save(&self.data_paths);
    }

    /// Запоминает громкость в настройках, когда её перестали крутить.
    ///
    /// Сохранять на каждом шаге ползунка нельзя: за одно движение мыши
    /// файл настроек переписался бы полсотни раз.
    pub(super) fn store_volume(&mut self, ctx: &egui::Context) {
        if !self.volume_changed {
            return;
        }

        // Пока кнопка мыши нажата, ползунок ещё ведут.
        if ctx.input(|i| i.pointer.any_down()) {
            return;
        }

        let Some(engine) = self.engine.as_ref() else {
            return;
        };

        let volume = engine.state().volume;
        self.volume_changed = false;

        if self.settings.volume == volume {
            return;
        }

        self.settings.volume = volume;
        self.settings.save(&self.data_paths);
        tracing::debug!(volume, "громкость запомнена");
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
            self.show_notice(tr!(
                "Не удалось переключить вывод звука",
                "Could not switch the audio output"
            ));
            return;
        }

        // «auto» не храним: пусть на новой машине mpv решает сам.
        self.settings.audio_device = if name == AUTO_DEVICE {
            None
        } else {
            Some(name.to_string())
        };
        self.settings.save(&self.data_paths);

        self.show_notice(tr!("Вывод звука переключён", "Audio output switched"));
    }
}
