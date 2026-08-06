//! История открытых файлов и путей.
//!
//! Плеер обычно ходит по одним и тем же папкам: исходники лежат в двух-трёх
//! местах, и добираться до них через диалог каждый раз долго. Здесь
//! запоминаются последние файлы и последние папки, из которых их открывали.
//!
//! Список короткий: пять записей на каждый вид. Новая запись становится
//! первой, самая старая уходит.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::file::{read_json, write_json};
use crate::paths::DataPaths;

/// Версия формата файла.
const FORMAT_VERSION: u32 = 1;

/// Сколько записей храним в каждом списке.
pub const CAPACITY: usize = 5;

#[derive(Debug, Serialize, Deserialize)]
struct HistoryFile {
    version: u32,
    /// Последние открытые файлы, свежие впереди.
    files: Vec<PathBuf>,
    /// Последние папки, из которых открывали файлы.
    dirs: Vec<PathBuf>,
}

impl Default for HistoryFile {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            files: Vec::new(),
            dirs: Vec::new(),
        }
    }
}

/// История открытых файлов и папок.
pub struct History {
    paths: DataPaths,
    data: HistoryFile,
}

impl History {
    /// Загружает историю. Отсутствие файла — не ошибка.
    pub fn load(paths: DataPaths) -> Self {
        let data: HistoryFile = read_json(&paths.history())
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "не удалось прочитать историю");
                None
            })
            .unwrap_or_default();

        tracing::debug!(
            файлов = data.files.len(),
            папок = data.dirs.len(),
            "история загружена"
        );

        Self { paths, data }
    }

    /// Последние открытые файлы, свежие впереди.
    pub fn files(&self) -> &[PathBuf] {
        &self.data.files
    }

    /// Последние папки, свежие впереди.
    pub fn dirs(&self) -> &[PathBuf] {
        &self.data.dirs
    }

    /// Запоминает открытый файл и папку, в которой он лежит.
    ///
    /// Возвращает `true`, если история изменилась и была записана на диск.
    pub fn remember(&mut self, path: &Path) -> bool {
        // Потоки и ссылки на сеть в историю не идут: вернуться по ним
        // через диалог всё равно нельзя.
        if !path.is_absolute() {
            return false;
        }

        let mut changed = push_front(&mut self.data.files, path.to_path_buf());

        if let Some(dir) = path.parent() {
            changed |= push_front(&mut self.data.dirs, dir.to_path_buf());
        }

        if changed {
            self.save();
        }

        changed
    }

    /// Убирает из истории то, чего больше нет на диске.
    ///
    /// Возвращает `true`, если что-то убрали.
    pub fn forget_missing(&mut self) -> bool {
        let before = self.data.files.len() + self.data.dirs.len();

        self.data.files.retain(|path| path.exists());
        self.data.dirs.retain(|path| path.is_dir());

        let changed = before != self.data.files.len() + self.data.dirs.len();
        if changed {
            self.save();
        }

        changed
    }

    fn save(&self) {
        if let Err(e) = write_json(&self.paths.history(), &self.data) {
            tracing::warn!(error = %e, "не удалось сохранить историю");
        }
    }
}

/// Ставит запись первой, убирая её прежнее место и лишний хвост.
fn push_front(list: &mut Vec<PathBuf>, value: PathBuf) -> bool {
    if list.first() == Some(&value) {
        return false;
    }

    // Сравниваем без учёта регистра: в Windows это один и тот же путь,
    // а в истории он иначе задваивается.
    let key = value.to_string_lossy().to_lowercase();
    list.retain(|item| item.to_string_lossy().to_lowercase() != key);

    list.insert(0, value);
    list.truncate(CAPACITY);

    true
}

#[cfg(test)]
mod tests {
    use super::{CAPACITY, push_front};
    use std::path::PathBuf;

    fn list(paths: &[&str]) -> Vec<PathBuf> {
        paths.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn новая_запись_становится_первой() {
        let mut items = list(&["C:\\один.mkv"]);

        assert!(push_front(&mut items, PathBuf::from("C:\\два.mkv")));
        assert_eq!(items, list(&["C:\\два.mkv", "C:\\один.mkv"]));
    }

    #[test]
    fn повторное_открытие_поднимает_запись_наверх() {
        let mut items = list(&["C:\\один.mkv", "C:\\два.mkv", "C:\\три.mkv"]);

        assert!(push_front(&mut items, PathBuf::from("C:\\три.mkv")));
        assert_eq!(items, list(&["C:\\три.mkv", "C:\\один.mkv", "C:\\два.mkv"]));
    }

    #[test]
    fn тот_же_файл_подряд_историю_не_меняет() {
        let mut items = list(&["C:\\один.mkv"]);

        assert!(!push_front(&mut items, PathBuf::from("C:\\один.mkv")));
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn регистр_пути_не_создаёт_двойников() {
        let mut items = list(&["C:\\Кино\\один.mkv"]);

        push_front(&mut items, PathBuf::from("c:\\кино\\ОДИН.mkv"));
        assert_eq!(items.len(), 1, "тот же путь другим регистром: {items:?}");
    }

    #[test]
    fn самая_старая_запись_вытесняется() {
        let mut items = Vec::new();

        for index in 0..CAPACITY + 3 {
            push_front(&mut items, PathBuf::from(format!("C:\\файл{index}.mkv")));
        }

        assert_eq!(items.len(), CAPACITY);
        assert_eq!(items[0], PathBuf::from("C:\\файл7.mkv"), "свежая впереди");
        assert!(
            !items.contains(&PathBuf::from("C:\\файл0.mkv")),
            "самая старая ушла: {items:?}"
        );
    }
}
