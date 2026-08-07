//! Язык интерфейса: выбор при запуске и переключение из меню.

use pith_store::{DataPaths, Language, Settings};

use super::PithApp;

impl PithApp {
    /// Выбирает язык при запуске.
    ///
    /// Порядок такой: выбор из установщика, потом сохранённый в настройках,
    /// потом язык системы. Установщик спрашивает язык в мастере, и второй
    /// раз его спрашивать незачем; на чистой машине настроек ещё нет —
    /// берём тот, на котором говорит Windows.
    pub(super) fn choose_language(
        settings: &mut Settings,
        paths: &DataPaths,
        chosen: Option<Language>,
    ) {
        let first_run = !paths.settings().exists();

        let language = match (chosen, first_run) {
            (Some(language), _) => language,
            (None, true) => crate::system_language::detect(),
            (None, false) => settings.language,
        };

        if settings.language != language {
            settings.language = language;
            settings.save(paths);
        }

        crate::i18n::set(language);
        tracing::info!(?language, первый_запуск = first_run, "язык интерфейса");
    }

    /// Язык интерфейса.
    pub fn language(&self) -> Language {
        self.settings.language
    }

    /// Переключает язык интерфейса. Выбор запоминается.
    pub fn set_language(&mut self, language: Language) {
        if self.settings.language == language {
            return;
        }

        self.settings.language = language;
        self.settings.save(&self.data_paths);
        crate::i18n::set(language);

        tracing::info!(?language, "язык интерфейса переключён");
    }
}
