//! Какие окна открыты поверх кадра.
//!
//! Оба списка перечисляют все окна разом, поэтому новое окно правится
//! в одном месте, а не разыскивается по обработчикам клавиш.

use super::PithApp;

impl PithApp {
    /// Открыто ли окно, для которого Escape означает «закрыть».
    ///
    /// Пока такое окно на экране, Escape принадлежит ему. Иначе одно
    /// нажатие делает два дела сразу: закрывает окно и заодно бросает
    /// плеер в полноэкранный режим.
    pub fn escape_belongs_to_window(&self) -> bool {
        self.search.open
            || self.list_dialog.is_some()
            || self.bookmark_rename.is_some()
            || self.clear_list_pending
            || self.fragment_settings.is_some()
            || self.file_types_prompt.is_some()
            || self.subtitle_style_open
            || self.export.is_some()
    }

    /// Открыто ли поверх кадра окно, которому принадлежат клавиши.
    ///
    /// Такие окна сами разбирают Escape и Enter, и горячие клавиши плеера
    /// на это время замолкают: иначе Escape закрывал окно и одновременно
    /// переключал полный экран.
    pub fn dialog_open(&self) -> bool {
        self.history_open
            || self.search.open
            || self.list_dialog.is_some()
            || self.bookmark_rename.is_some()
            || self.clear_list_pending
            || self.fragment_settings.is_some()
            || self.file_types_prompt.is_some()
            || self.migration.is_some()
            || self.subtitle_style_open
            || self.export.is_some()
    }
}
