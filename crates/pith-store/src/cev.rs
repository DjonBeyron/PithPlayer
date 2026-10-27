//! Настройки выгрузки в базу CodeEditVideo (репозиторий на GitHub).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::file::{read_json, write_json};
use crate::paths::DataPaths;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CevSettings {
    /// Выгружать в CodeEditVideo вместо Notion.
    pub enabled: bool,
    /// Токен GitHub с правом записи в репозиторий проектов
    /// (fine-grained: Contents — Read and write).
    pub token: String,
}

impl CevSettings {
    /// Хватает ли настроек для выгрузки: включено и токен задан.
    pub fn is_ready(&self) -> bool {
        self.enabled && !self.token.trim().is_empty()
    }
}

/// Отдельный файл с токеном: его не перезапишет ни старая версия плеера,
/// которая про выгрузку в CodeEditVideo не знает и при сохранении
/// настроек выбрасывает незнакомые поля, ни сброс `settings.json`.
const TOKEN_FILE: &str = "cev.json";

impl CevSettings {
    /// Подхватывает токен, если в настройках его нет.
    ///
    /// Ищет сначала в `cev.json`, потом в копиях настроек, которые делает
    /// установщик перед заменой версии (`backup/<версия>/settings.json`),
    /// начиная с самой свежей. Найденное сразу записывается в `cev.json`.
    pub fn restore(&mut self, paths: &DataPaths) {
        if self.token.trim().is_empty() {
            let found = read_json::<CevSettings>(&paths.root().join(TOKEN_FILE))
                .ok()
                .flatten()
                .filter(|c| !c.token.trim().is_empty())
                .or_else(|| from_backups(paths.root()));

            if let Some(found) = found {
                tracing::info!("токен GitHub подхвачен из прежних данных");
                *self = found;
            }
        }

        self.persist(paths);
    }

    /// Запоминает токен в `cev.json`. Пустой токен файл не затирает.
    pub fn persist(&self, paths: &DataPaths) {
        if self.token.trim().is_empty() {
            return;
        }

        if let Err(e) = write_json(&paths.root().join(TOKEN_FILE), self) {
            tracing::warn!(error = %e, "не удалось записать cev.json");
        }
    }
}

/// Самый свежий непустой токен среди копий настроек установщика.
fn from_backups(root: &Path) -> Option<CevSettings> {
    let mut dirs: Vec<_> = std::fs::read_dir(root.join("backup"))
        .ok()?
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .collect();

    dirs.sort_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok());
    dirs.reverse();

    dirs.iter().find_map(|entry| {
        let settings: serde_json::Value = read_json(&entry.path().join("settings.json"))
            .ok()
            .flatten()?;
        let cev: CevSettings = serde_json::from_value(settings.get("cev")?.clone()).ok()?;

        (!cev.token.trim().is_empty()).then_some(cev)
    })
}

#[cfg(test)]
mod restore_tests {
    use super::*;

    #[test]
    fn токен_берётся_из_cev_json_если_в_настройках_пусто() {
        let dir = tempfile::tempdir().expect("каталог");
        let paths = DataPaths::with_root(dir.path());

        CevSettings {
            enabled: true,
            token: "ghp_a".into(),
        }
        .persist(&paths);

        let mut now = CevSettings::default();
        now.restore(&paths);

        assert_eq!(now.token, "ghp_a");
        assert!(now.enabled);
    }

    #[test]
    fn токен_берётся_из_копии_установщика() {
        let dir = tempfile::tempdir().expect("каталог");
        let paths = DataPaths::with_root(dir.path());

        let backup = dir.path().join("backup").join("5.1.50");
        std::fs::create_dir_all(&backup).expect("папка копии");
        std::fs::write(
            backup.join("settings.json"),
            r#"{"language":"ru","cev":{"enabled":true,"token":"ghp_b"}}"#,
        )
        .expect("копия");

        let mut now = CevSettings::default();
        now.restore(&paths);

        assert_eq!(now.token, "ghp_b");
        assert!(
            dir.path().join("cev.json").exists(),
            "найденное запоминается"
        );
    }

    #[test]
    fn свой_токен_не_подменяется() {
        let dir = tempfile::tempdir().expect("каталог");
        let paths = DataPaths::with_root(dir.path());
        CevSettings {
            enabled: true,
            token: "old".into(),
        }
        .persist(&paths);

        let mut now = CevSettings {
            enabled: true,
            token: "new".into(),
        };
        now.restore(&paths);

        assert_eq!(now.token, "new");
    }
}

#[cfg(test)]
mod tests {
    use super::CevSettings;

    #[test]
    fn готово_только_включённое_и_с_токеном() {
        let mut cev = CevSettings::default();
        assert!(!cev.is_ready());

        cev.enabled = true;
        assert!(!cev.is_ready(), "без токена писать некуда");

        cev.token = " ghp_x ".into();
        assert!(cev.is_ready());

        cev.enabled = false;
        assert!(!cev.is_ready());
    }
}
