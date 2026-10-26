//! Настройки выгрузки в базу CodeEditVideo (репозиторий на GitHub).

use serde::{Deserialize, Serialize};

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
