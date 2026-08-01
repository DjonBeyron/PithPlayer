//! Позиции просмотра и предложение продолжить.
//!
//! Правила отбора перенесены из v4 без изменений (PLAN.md §6.6):
//! короткие файлы, самое начало и самый конец не запоминаются.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::file::{read_json, write_json};
use crate::file_key::{FileKey, file_key};
use crate::paths::DataPaths;

/// Версия формата файла. Меняется только при несовместимых правках.
const FORMAT_VERSION: u32 = 1;

/// Видео короче — позицию не запоминаем.
const MIN_VIDEO_DURATION_SEC: f64 = 300.0;
/// Первые и последние столько секунд не запоминаем.
const EDGE_MARGIN_SEC: f64 = 30.0;
/// Просмотрено больше — продолжать нечего.
const MAX_CONTINUE_FRACTION: f64 = 0.95;

/// Запись о просмотре одного файла.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchPosition {
    /// Имя файла — только для показа пользователю.
    pub file_name: String,
    /// Позиция, секунды.
    pub position: f64,
    /// Длительность, секунды.
    pub duration: f64,
    /// Когда смотрели последний раз, unix-время в секундах.
    pub last_watched: u64,
}

impl WatchPosition {
    /// Доля просмотренного, 0…1.
    pub fn fraction(&self) -> f64 {
        if self.duration <= 0.0 {
            return 0.0;
        }
        (self.position / self.duration).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PositionsFile {
    version: u32,
    positions: HashMap<FileKey, WatchPosition>,
}

impl Default for PositionsFile {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            positions: HashMap::new(),
        }
    }
}

/// Хранилище позиций просмотра.
pub struct WatchPositions {
    paths: DataPaths,
    data: PositionsFile,
}

impl WatchPositions {
    /// Загружает позиции. Отсутствие файла — не ошибка.
    pub fn load(paths: DataPaths) -> Self {
        let data: PositionsFile = read_json(&paths.watch_positions())
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "не удалось прочитать позиции просмотра");
                None
            })
            .unwrap_or_default();

        tracing::debug!(
            записей = data.positions.len(),
            "позиции просмотра загружены"
        );
        Self { paths, data }
    }

    /// Сохраняет позицию, если она достойна запоминания.
    ///
    /// Возвращает `true`, если запись изменилась и файл был перезаписан.
    pub fn remember(&mut self, path: &Path, position: f64, duration: f64) -> bool {
        let Some(key) = file_key(path) else {
            return false;
        };

        if !is_worth_remembering(position, duration) {
            // Раньше могли запомнить — теперь запись потеряла смысл.
            return self.forget_key(&key);
        }

        let entry = WatchPosition {
            file_name: file_name_of(path),
            position,
            duration,
            last_watched: now_unix(),
        };

        self.data.positions.insert(key, entry);
        self.save();
        true
    }

    /// Позиция для файла, если её стоит предлагать.
    pub fn resume_position(&self, path: &Path) -> Option<&WatchPosition> {
        let key = file_key(path)?;
        let entry = self.data.positions.get(&key)?;

        (entry.fraction() <= MAX_CONTINUE_FRACTION && entry.position > 0.0).then_some(entry)
    }

    /// Забывает позицию для файла.
    pub fn forget(&mut self, path: &Path) -> bool {
        let Some(key) = file_key(path) else {
            return false;
        };
        self.forget_key(&key)
    }

    fn forget_key(&mut self, key: &FileKey) -> bool {
        if self.data.positions.remove(key).is_none() {
            return false;
        }
        self.save();
        true
    }

    pub fn len(&self) -> usize {
        self.data.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.positions.is_empty()
    }

    /// Добавляет запись напрямую. Нужно миграции из v4.
    pub fn insert_raw(&mut self, key: FileKey, entry: WatchPosition) {
        self.data.positions.insert(key, entry);
    }

    /// Записывает файл. Ошибка записи не должна ломать воспроизведение.
    pub fn save(&self) {
        if let Err(e) = write_json(&self.paths.watch_positions(), &self.data) {
            tracing::error!(error = %e, "не удалось сохранить позиции просмотра");
        }
    }
}

/// Стоит ли запоминать позицию. Правила из v4.
pub fn is_worth_remembering(position: f64, duration: f64) -> bool {
    if !position.is_finite() || !duration.is_finite() {
        return false;
    }

    // Короткие ролики не про «продолжить просмотр».
    if duration < MIN_VIDEO_DURATION_SEC {
        return false;
    }

    // Самое начало: пользователь ничего толком не посмотрел.
    if position < EDGE_MARGIN_SEC {
        return false;
    }

    // Самый конец: продолжать нечего.
    if duration - position < EDGE_MARGIN_SEC {
        return false;
    }

    position / duration <= MAX_CONTINUE_FRACTION
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

pub(crate) fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Фильм полтора часа.
    const ФИЛЬМ: f64 = 5400.0;

    #[test]
    fn середину_фильма_запоминаем() {
        assert!(is_worth_remembering(2700.0, ФИЛЬМ));
    }

    #[test]
    fn короткое_видео_не_запоминаем() {
        // Ролик четыре минуты — короче пятиминутного порога.
        assert!(!is_worth_remembering(120.0, 240.0));
    }

    #[test]
    fn первые_тридцать_секунд_не_запоминаем() {
        assert!(!is_worth_remembering(29.0, ФИЛЬМ));
        assert!(is_worth_remembering(31.0, ФИЛЬМ));
    }

    #[test]
    fn последние_тридцать_секунд_не_запоминаем() {
        assert!(!is_worth_remembering(ФИЛЬМ - 29.0, ФИЛЬМ));
    }

    #[test]
    fn досмотренное_почти_до_конца_не_предлагаем() {
        // 96 % просмотра — продолжать нечего.
        assert!(!is_worth_remembering(ФИЛЬМ * 0.96, ФИЛЬМ));
    }

    #[test]
    fn некорректные_числа_отбрасываются() {
        assert!(!is_worth_remembering(f64::NAN, ФИЛЬМ));
        assert!(!is_worth_remembering(100.0, f64::INFINITY));
    }

    #[test]
    fn доля_просмотра_считается() {
        let entry = WatchPosition {
            file_name: "фильм.mkv".into(),
            position: 2700.0,
            duration: ФИЛЬМ,
            last_watched: 0,
        };
        assert!((entry.fraction() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn нулевая_длительность_не_делит_на_ноль() {
        let entry = WatchPosition {
            file_name: "битый.mkv".into(),
            position: 10.0,
            duration: 0.0,
            last_watched: 0,
        };
        assert_eq!(entry.fraction(), 0.0);
    }
}
