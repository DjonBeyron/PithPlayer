//! Настройки режима разработчика.
//!
//! Отдельным файлом `dev.json`, а не полем в `Settings`: журнал нужен ещё
//! до загрузки настроек (логирование стартует первым), и эти значения
//! человек правит редко и руками.

use serde::{Deserialize, Serialize};

use crate::file::{read_json, write_json};
use crate::paths::DataPaths;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DevSettings {
    /// Папка для журналов (`pith-player.log` и `mpv.log`). Пусто — по умолчанию.
    pub log_dir: String,
}

impl DevSettings {
    /// Читает настройки. Нет файла или он битый — значения по умолчанию.
    pub fn load(paths: &DataPaths) -> Self {
        read_json(&paths.root().join("dev.json"))
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    /// Сохраняет настройки.
    pub fn save(&self, paths: &DataPaths) {
        if let Err(e) = write_json(&paths.root().join("dev.json"), self) {
            tracing::warn!(error = %e, "не удалось сохранить dev.json");
        }
    }

    /// Папка журналов, если задана.
    pub fn log_dir(&self) -> Option<std::path::PathBuf> {
        let dir = self.log_dir.trim();
        (!dir.is_empty()).then(|| std::path::PathBuf::from(dir))
    }
}
