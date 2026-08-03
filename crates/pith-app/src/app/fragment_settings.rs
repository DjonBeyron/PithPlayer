//! Общие настройки нарезки (порт `RecordingSettingsForm` из v4, §5.2).
//!
//! Это значения по умолчанию: у каждого списка отрезков могут быть свои
//! длительность, отступ и папка (PLAN.md §6.5).

use std::path::PathBuf;

use super::PithApp;

/// Поля диалога настроек нарезки.
///
/// Правятся интерфейсом напрямую и применяются целиком по «Сохранить»:
/// половинчатое состояние в настройках не задерживается.
#[derive(Debug, Clone)]
pub struct FragmentSettingsDialog {
    pub output_dir: Option<PathBuf>,
    pub duration_sec: u32,
    pub buffer_sec: u32,
    pub reencode: bool,
    pub audio_aac: bool,
}

impl PithApp {
    pub fn fragment_settings_dialog(&self) -> Option<&FragmentSettingsDialog> {
        self.fragment_settings.as_ref()
    }

    /// Поля диалога для правки интерфейсом.
    pub fn fragment_settings_dialog_mut(&mut self) -> Option<&mut FragmentSettingsDialog> {
        self.fragment_settings.as_mut()
    }

    pub fn open_fragment_settings(&mut self) {
        tracing::debug!("открываю настройки нарезки");
        let fragments = &self.settings.fragments;

        self.fragment_settings = Some(FragmentSettingsDialog {
            output_dir: fragments.output_dir.clone(),
            duration_sec: fragments.duration_sec,
            buffer_sec: fragments.buffer_sec,
            reencode: fragments.reencode,
            audio_aac: fragments.audio_aac,
        });
    }

    pub fn close_fragment_settings(&mut self) {
        self.fragment_settings = None;
    }

    /// Сохраняет настройки нарезки.
    pub fn apply_fragment_settings(&mut self) {
        let Some(dialog) = self.fragment_settings.take() else {
            return;
        };

        let duration_sec = dialog.duration_sec.max(1);

        let fragments = &mut self.settings.fragments;
        fragments.output_dir = dialog.output_dir;
        fragments.duration_sec = duration_sec;
        fragments.buffer_sec = dialog.buffer_sec;
        fragments.reencode = dialog.reencode;
        fragments.audio_aac = dialog.audio_aac;

        self.settings.save(&self.data_paths);

        tracing::info!(
            длительность = duration_sec,
            отступ = dialog.buffer_sec,
            перекодирование = dialog.reencode,
            звук_в_aac = dialog.audio_aac,
            "настройки нарезки сохранены"
        );
        self.show_notice("Настройки нарезки сохранены");
    }

    /// Доступна ли нарезка: без FFmpeg настройки бесполезны.
    pub fn fragment_settings_hint(&self) -> Option<&'static str> {
        if self.can_extract() {
            None
        } else {
            Some("Рядом с плеером нет ffmpeg.exe — нарезка недоступна")
        }
    }
}
