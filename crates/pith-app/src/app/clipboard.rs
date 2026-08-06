//! Копирование реплик субтитров и настройки слоёв.

use pith_store::SubtitleLayout;

use super::PithApp;
use crate::ui::subtitles::Layer;

/// Сколько секунд показывать уведомление о копировании.
const NOTICE_SECONDS: f64 = 1.8;

/// Шаг изменения размера шрифта колесом мыши.
const FONT_STEP: f32 = 2.0;

impl PithApp {
    pub fn settings(&self) -> &pith_store::Settings {
        &self.settings
    }

    pub fn save_settings(&self) {
        self.settings.save(&self.data_paths);
    }

    pub fn subtitle_layout(&self, layer: Layer) -> SubtitleLayout {
        match layer {
            Layer::Main => self.settings.main_subtitle,
            Layer::Secondary => self.settings.secondary_subtitle,
        }
    }

    /// Сдвигает слой субтитров на долю окна.
    pub fn move_subtitle_layout(&mut self, layer: Layer, dx: f32, dy: f32) {
        let layout = match layer {
            Layer::Main => &mut self.settings.main_subtitle,
            Layer::Secondary => &mut self.settings.secondary_subtitle,
        };

        layout.x += dx;
        layout.y += dy;
        *layout = layout.clamped();
    }

    /// Меняет размер шрифта слоя субтитров.
    pub fn adjust_subtitle_font_size(&mut self, layer: Layer, direction: f32) {
        let layout = match layer {
            Layer::Main => &mut self.settings.main_subtitle,
            Layer::Secondary => &mut self.settings.secondary_subtitle,
        };

        layout.font_size += FONT_STEP * direction;
        *layout = layout.clamped();

        self.save_settings();
    }

    /// Показывать ли субтитры.
    pub fn toggle_subtitles(&mut self) {
        self.settings.subtitles_visible = !self.settings.subtitles_visible;
        self.save_settings();

        tracing::debug!(
            видимы = self.settings.subtitles_visible,
            "переключены субтитры"
        );
    }

    /// Копирует текущую реплику основных субтитров.
    ///
    /// Если основных нет, берём вторые: пользователь нажал «копировать»,
    /// значит хочет текст, который видит.
    pub fn copy_current_subtitle(&mut self) {
        let text = self
            .subtitle_text
            .main
            .clone()
            .or_else(|| self.subtitle_text.secondary.clone());

        match text {
            Some(text) => self.copy_text_to_clipboard(&text),
            None => self.show_notice(crate::tr!("Субтитров сейчас нет", "No subtitles right now")),
        }
    }

    /// Кладёт текст в буфер обмена и показывает уведомление.
    pub fn copy_text_to_clipboard(&mut self, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }

        match arboard::Clipboard::new().and_then(|mut c| c.set_text(text.to_string())) {
            Ok(()) => {
                tracing::debug!(длина = text.len(), "реплика скопирована");
                self.show_notice(crate::tr!("Скопировано", "Copied"));
            }
            Err(e) => {
                tracing::warn!(error = %e, "не удалось скопировать в буфер обмена");
                self.show_notice(crate::tr!("Не удалось скопировать", "Could not copy"));
            }
        }
    }

    /// Показывает короткое уведомление поверх видео.
    pub fn show_notice(&mut self, text: &str) {
        self.notice = Some(Notice {
            text: text.to_string(),
            until: self.frame_time + NOTICE_SECONDS,
        });
    }

    /// Текущее уведомление, если оно ещё не погасло.
    pub fn notice(&self) -> Option<&str> {
        self.notice
            .as_ref()
            .filter(|n| n.until > self.frame_time)
            .map(|n| n.text.as_str())
    }
}

/// Всплывающее уведомление.
pub struct Notice {
    pub text: String,
    /// Время кадра, после которого уведомление гаснет.
    pub until: f64,
}
