//! Транскрипции слов, найденные в словарях.
//!
//! Словари медленные: первый ограничивает частоту запросов и требует паузы
//! между словами, так что слово стоит около секунды. Реплики же повторяются
//! из фильма в фильм, и однажды найденное слово спрашивать заново незачем —
//! в этом весь смысл файла. Готовая система пользователя устроена так же
//! (`NOTION_PITH/TRANSCRIPTION_LOGIC.md`).
//!
//! Ключ — слово в нижнем регистре без апострофов: `It's` и `its` для словаря
//! одно и то же. Ключи делает `pith-dict`, здесь только хранение.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::file::{read_json, write_json};
use crate::paths::DataPaths;

/// Версия формата файла.
const FORMAT_VERSION: u32 = 1;

/// Найденная транскрипция и страница, с которой она взята.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sound {
    /// Транскрипция вместе с обрамляющими чертами: `|wer|`.
    pub transcription: String,
    /// Адрес страницы слова — на него ведёт ссылка в Notion.
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SoundsFile {
    version: u32,
    /// Слова по алфавиту: файл смотрят глазами, и порядок в нём нужен.
    words: BTreeMap<String, Sound>,
}

impl Default for SoundsFile {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            words: BTreeMap::new(),
        }
    }
}

/// Хранилище транскрипций.
pub struct SoundStore {
    paths: DataPaths,
    data: SoundsFile,
}

impl SoundStore {
    /// Читает файл. Отсутствие — не ошибка: словарь наполнится сам.
    pub fn load(paths: DataPaths) -> Self {
        let data: SoundsFile = read_json(&paths.transcriptions())
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "не удалось прочитать транскрипции");
                None
            })
            .unwrap_or_default();

        tracing::debug!(слов = data.words.len(), "транскрипции загружены");
        Self { paths, data }
    }

    /// Сколько слов уже известно.
    pub fn len(&self) -> usize {
        self.data.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.words.is_empty()
    }

    /// Известное слово.
    pub fn get(&self, key: &str) -> Option<&Sound> {
        self.data.words.get(key)
    }

    /// Снимок для рабочего потока.
    ///
    /// Поток выгрузки не может держать хранилище: оно живёт в приложении
    /// и правится в потоке интерфейса. Поэтому поток получает копию, а всё
    /// новое возвращает списком — его и принимает `remember`.
    pub fn snapshot(&self) -> BTreeMap<String, Sound> {
        self.data.words.clone()
    }

    /// Запоминает найденные слова и записывает файл.
    ///
    /// Пустой список файла не трогает: незачем переписывать его, когда
    /// все слова взялись из него же.
    pub fn remember(&mut self, found: impl IntoIterator<Item = (String, Sound)>) {
        let before = self.data.words.len();

        for (key, sound) in found {
            self.data.words.insert(key, sound);
        }

        if self.data.words.len() == before {
            return;
        }

        tracing::info!(
            новых = self.data.words.len() - before,
            всего = self.data.words.len(),
            "транскрипции пополнились"
        );

        if let Err(e) = write_json(&self.paths.transcriptions(), &self.data) {
            tracing::error!(error = %e, "не удалось сохранить транскрипции");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Sound, SoundStore};
    use crate::paths::DataPaths;

    fn звук(text: &str) -> Sound {
        Sound {
            transcription: text.into(),
            url: format!("https://wooordhunt.ru/word/{text}"),
        }
    }

    #[test]
    fn слово_переживает_перезапуск() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        let mut store = SoundStore::load(paths.clone());
        store.remember([("where".to_string(), звук("|wer|"))]);

        let reopened = SoundStore::load(paths);
        let sound = reopened.get("where").expect("слово на месте");

        assert_eq!(sound.transcription, "|wer|");
    }

    #[test]
    fn чужое_слово_не_находится() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let store = SoundStore::load(DataPaths::with_root(dir.path()));

        assert!(store.get("where").is_none());
        assert!(store.is_empty());
    }

    #[test]
    fn повторная_запись_заменяет_прежнюю() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let mut store = SoundStore::load(DataPaths::with_root(dir.path()));

        store.remember([("to".to_string(), звук("|tə|"))]);
        store.remember([("to".to_string(), звук("|tuː|"))]);

        assert_eq!(store.get("to").expect("слово").transcription, "|tuː|");
        assert_eq!(store.len(), 1, "слово одно, а не два");
    }

    #[test]
    fn снимок_отдаёт_все_слова() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let mut store = SoundStore::load(DataPaths::with_root(dir.path()));

        store.remember([
            ("where".to_string(), звук("|wer|")),
            ("you".to_string(), звук("|juː|")),
        ]);

        let snapshot = store.snapshot();

        assert_eq!(snapshot.len(), 2);
        assert!(snapshot.contains_key("you"));
    }
}
