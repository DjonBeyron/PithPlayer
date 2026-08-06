//! Цвет и начертание субтитров: окно настройки и хранение выбора.

use super::PithApp;
use crate::ui::subtitles::Layer;

impl PithApp {
    /// Открыто ли окно настройки вида субтитров.
    ///
    /// По нему же показывается пример: пока окно открыто, оба слоя видны
    /// на своих местах, даже когда реплик сейчас нет.
    pub fn subtitle_style_open(&self) -> bool {
        self.subtitle_style_open
    }

    pub fn open_subtitle_style(&mut self) {
        self.subtitle_style_open = true;
    }

    pub fn close_subtitle_style(&mut self) {
        self.subtitle_style_open = false;
    }

    /// Задаёт цвет и начертание слоя.
    pub fn set_subtitle_style(&mut self, layer: Layer, color: [u8; 3], bold: bool) {
        let layout = self.subtitle_layout_mut(layer);

        if layout.color == color && layout.bold == bold {
            return;
        }

        layout.color = color;
        layout.bold = bold;

        // Не сохраняем прямо здесь: цвет тянут мышью по кругу выбора,
        // и файл настроек переписывался бы на каждом шаге.
        self.subtitle_style_dirty = true;
    }

    /// Возвращает слою исходные цвет и начертание.
    pub fn reset_subtitle_style(&mut self, layer: Layer) {
        self.subtitle_layout_mut(layer).reset_style();
        self.subtitle_style_dirty = true;
    }

    /// Записывает выбранный вид, когда мышь отпустили.
    pub(super) fn store_subtitle_style(&mut self, ctx: &egui::Context) {
        if !self.subtitle_style_dirty {
            return;
        }

        // Пока кнопка нажата, цвет ещё выбирают.
        if ctx.input(|i| i.pointer.any_down()) {
            return;
        }

        self.subtitle_style_dirty = false;
        self.save_settings();

        tracing::debug!("вид субтитров запомнен");
    }

    fn subtitle_layout_mut(&mut self, layer: Layer) -> &mut pith_store::SubtitleLayout {
        match layer {
            Layer::Main => &mut self.settings.main_subtitle,
            Layer::Secondary => &mut self.settings.secondary_subtitle,
        }
    }
}
