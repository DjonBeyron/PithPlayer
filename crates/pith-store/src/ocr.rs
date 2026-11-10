//! Настройки распознавания текста с кадра: включено ли и какая зона.
//!
//! Отдельным файлом `ocr.json`: зона выбирается один раз и действует для
//! всех следующих видео, а файл настроек плеера и без того велик.

use serde::{Deserialize, Serialize};

use crate::file::{read_json, write_json};
use crate::paths::DataPaths;

const FILE: &str = "ocr.json";

/// Зона субтитров в долях кадра: от 0 до 1 по каждой стороне.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Zone {
    /// Нижняя полоса кадра, где обычно стоят субтитры.
    pub const DEFAULT: Zone = Zone {
        x: 0.05,
        y: 0.78,
        w: 0.90,
        h: 0.17,
    };

    /// Зона осмысленна: внутри кадра и не слишком мала.
    pub fn is_sane(&self) -> bool {
        self.w >= 0.02
            && self.h >= 0.02
            && self.x >= 0.0
            && self.y >= 0.0
            && self.x + self.w <= 1.001
            && self.y + self.h <= 1.001
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OcrSettings {
    /// Распознавать текст с кадра, когда у закладки нет субтитров.
    pub enabled: bool,
    /// Выбранная зона. `None` — не выбрана, распознавание не работает.
    pub zone: Option<Zone>,
}

impl OcrSettings {
    pub fn load(paths: &DataPaths) -> Self {
        read_json(&paths.root().join(FILE))
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    pub fn save(&self, paths: &DataPaths) {
        if let Err(e) = write_json(&paths.root().join(FILE), self) {
            tracing::warn!(error = %e, "не удалось сохранить ocr.json");
        }
    }

    /// Всё нужное для работы: включено и зона задана.
    pub fn is_ready(&self) -> bool {
        self.enabled && self.zone.is_some_and(|z| z.is_sane())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn зона_и_флаг_переживают_перезапуск() {
        let dir = tempfile::tempdir().expect("каталог");
        let paths = DataPaths::with_root(dir.path());

        OcrSettings {
            enabled: true,
            zone: Some(Zone::DEFAULT),
        }
        .save(&paths);
        let loaded = OcrSettings::load(&paths);

        assert!(loaded.is_ready());
        assert_eq!(loaded.zone, Some(Zone::DEFAULT));
    }

    #[test]
    fn без_зоны_или_выключенное_не_готово() {
        assert!(!OcrSettings::default().is_ready());
        assert!(
            !OcrSettings {
                enabled: true,
                zone: None
            }
            .is_ready()
        );
        assert!(
            !OcrSettings {
                enabled: false,
                zone: Some(Zone::DEFAULT)
            }
            .is_ready()
        );
    }

    #[test]
    fn нелепая_зона_отвергается() {
        assert!(
            !Zone {
                x: 0.9,
                y: 0.9,
                w: 0.5,
                h: 0.5
            }
            .is_sane()
        );
        assert!(
            !Zone {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.1
            }
            .is_sane()
        );
        assert!(Zone::DEFAULT.is_sane());
    }
}
