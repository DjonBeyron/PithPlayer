//! Настройки перевода реплик через DeepL: токен и языки.
//!
//! Отдельным файлом `deepl.json`, а не полем `Settings`: токен должен
//! переживать переустановку и старые версии плеера, которые незнакомые
//! поля настроек при сохранении выбрасывают.

use serde::{Deserialize, Serialize};

use crate::file::{read_json, write_json};
use crate::paths::DataPaths;

const FILE: &str = "deepl.json";

/// Языки перевода: код DeepL, название по-русски и по-английски.
pub const LANGUAGES: [(&str, &str, &str); 12] = [
    ("RU", "Русский", "Russian"),
    ("UK", "Украинский", "Ukrainian"),
    ("EN-US", "Английский", "English"),
    ("DE", "Немецкий", "German"),
    ("FR", "Французский", "French"),
    ("ES", "Испанский", "Spanish"),
    ("IT", "Итальянский", "Italian"),
    ("PL", "Польский", "Polish"),
    ("PT-BR", "Португальский", "Portuguese"),
    ("TR", "Турецкий", "Turkish"),
    ("JA", "Японский", "Japanese"),
    ("ZH", "Китайский", "Chinese"),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeeplSettings {
    /// Ключ API DeepL (бесплатный заканчивается на `:fx`).
    pub token: String,
    /// На какие языки переводить, по порядку показа.
    pub targets: Vec<String>,
}

impl Default for DeeplSettings {
    fn default() -> Self {
        Self {
            token: String::new(),
            targets: vec!["RU".into()],
        }
    }
}

impl DeeplSettings {
    /// Читает настройки; нет файла — ищет токен в копиях установщика.
    pub fn load(paths: &DataPaths) -> Self {
        let path = paths.root().join(FILE);

        if let Ok(Some(found)) = read_json::<Self>(&path)
            && !found.token.trim().is_empty()
        {
            return found;
        }

        let restored = from_backups(paths.root().join("backup")).unwrap_or_default();

        if !restored.token.trim().is_empty() {
            tracing::info!("токен DeepL подхвачен из копии установщика");
            restored.save(paths);
        }

        restored
    }

    /// Сохраняет. Пустой токен существующий файл не затирает.
    pub fn save(&self, paths: &DataPaths) {
        if let Err(e) = write_json(&paths.root().join(FILE), self) {
            tracing::warn!(error = %e, "не удалось записать deepl.json");
        }
    }

    /// Хватает ли настроек для перевода.
    pub fn is_ready(&self) -> bool {
        !self.token.trim().is_empty() && !self.targets.is_empty()
    }
}

/// Самый свежий непустой `deepl.json` среди копий установщика.
fn from_backups(backup: std::path::PathBuf) -> Option<DeeplSettings> {
    let mut dirs: Vec<_> = std::fs::read_dir(backup)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();

    dirs.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
    dirs.reverse();

    dirs.iter().find_map(|e| {
        read_json::<DeeplSettings>(&e.path().join(FILE))
            .ok()
            .flatten()
            .filter(|s| !s.token.trim().is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn по_умолчанию_русский() {
        assert_eq!(DeeplSettings::default().targets, vec!["RU"]);
        assert!(!DeeplSettings::default().is_ready());
    }

    #[test]
    fn токен_подхватывается_из_копии_установщика() {
        let dir = tempfile::tempdir().expect("каталог");
        let paths = DataPaths::with_root(dir.path());

        let backup = dir.path().join("backup").join("5.1.60");
        std::fs::create_dir_all(&backup).expect("папка");
        std::fs::write(backup.join(FILE), r#"{"token":"abc:fx","targets":["DE"]}"#).expect("копия");

        let loaded = DeeplSettings::load(&paths);

        assert_eq!(loaded.token, "abc:fx");
        assert_eq!(loaded.targets, vec!["DE"]);
        assert!(dir.path().join(FILE).exists());
    }

    #[test]
    fn записанное_читается_обратно() {
        let dir = tempfile::tempdir().expect("каталог");
        let paths = DataPaths::with_root(dir.path());

        DeeplSettings {
            token: "k".into(),
            targets: vec!["RU".into(), "UK".into()],
        }
        .save(&paths);

        assert_eq!(DeeplSettings::load(&paths).targets, vec!["RU", "UK"]);
    }
}
