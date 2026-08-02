//! Настройки плеера.

use serde::{Deserialize, Serialize};

use crate::file::{read_json, write_json};
use crate::paths::DataPaths;
use crate::subtitle_priority::SubtitlePriority;

const FORMAT_VERSION: u32 = 1;

/// Куда прижат слой субтитров по горизонтали и вертикали.
///
/// Хранится долей от размера окна, а не пикселями: иначе после смены
/// разрешения субтитры уезжали бы за край.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SubtitleLayout {
    /// 0 — левый край, 1 — правый.
    pub x: f32,
    /// 0 — верх, 1 — низ.
    pub y: f32,
    /// Размер текста в точках.
    pub font_size: f32,
}

impl SubtitleLayout {
    /// Раскладка основных субтитров: внизу по центру.
    pub fn main() -> Self {
        Self {
            x: 0.5,
            y: 0.88,
            font_size: 30.0,
        }
    }

    /// Раскладка вторых субтитров: над основными.
    pub fn secondary() -> Self {
        Self {
            x: 0.5,
            y: 0.76,
            font_size: 26.0,
        }
    }

    /// Ограничивает раскладку разумными пределами.
    pub fn clamped(self) -> Self {
        Self {
            x: self.x.clamp(0.0, 1.0),
            y: self.y.clamp(0.0, 1.0),
            font_size: self.font_size.clamp(10.0, 96.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,

    /// Языки аудио по убыванию приоритета.
    pub audio_languages: Vec<String>,
    /// Правила автовыбора субтитров.
    pub subtitle_priority: SubtitlePriority,

    /// Показывать ли субтитры.
    pub subtitles_visible: bool,
    /// Раскладка основных субтитров.
    pub main_subtitle: SubtitleLayout,
    /// Раскладка вторых субтитров.
    pub secondary_subtitle: SubtitleLayout,

    /// Громкость, запоминается между запусками.
    pub volume: i64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            audio_languages: ["eng", "en"].iter().map(|s| s.to_string()).collect(),
            subtitle_priority: SubtitlePriority::default(),
            subtitles_visible: true,
            main_subtitle: SubtitleLayout::main(),
            secondary_subtitle: SubtitleLayout::secondary(),
            volume: 80,
        }
    }
}

impl Settings {
    /// Читает настройки. Отсутствие файла — не ошибка.
    pub fn load(paths: &DataPaths) -> Self {
        let mut settings: Settings = read_json(&paths.settings())
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "не удалось прочитать настройки");
                None
            })
            .unwrap_or_default();

        // Значения из файла могли прийти из ручной правки.
        settings.main_subtitle = settings.main_subtitle.clamped();
        settings.secondary_subtitle = settings.secondary_subtitle.clamped();
        settings.volume = settings.volume.clamp(0, 150);

        settings
    }

    /// Записывает настройки. Ошибка не должна ломать воспроизведение.
    pub fn save(&self, paths: &DataPaths) {
        if let Err(e) = write_json(&paths.settings(), self) {
            tracing::error!(error = %e, "не удалось сохранить настройки");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn раскладка_обрезается_по_пределам() {
        let layout = SubtitleLayout {
            x: 5.0,
            y: -3.0,
            font_size: 500.0,
        }
        .clamped();

        assert_eq!(layout.x, 1.0);
        assert_eq!(layout.y, 0.0);
        assert_eq!(layout.font_size, 96.0);
    }

    #[test]
    fn вторые_субтитры_выше_основных() {
        assert!(
            SubtitleLayout::secondary().y < SubtitleLayout::main().y,
            "второй слой обязан быть над основным"
        );
    }

    #[test]
    fn настройки_записываются_и_читаются() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        let priority = SubtitlePriority {
            secondary_enabled: true,
            ..Default::default()
        };

        let settings = Settings {
            volume: 55,
            subtitle_priority: priority,
            ..Default::default()
        };
        settings.save(&paths);

        let loaded = Settings::load(&paths);
        assert_eq!(loaded.volume, 55);
        assert!(loaded.subtitle_priority.secondary_enabled);
    }

    #[test]
    fn отсутствие_файла_даёт_значения_по_умолчанию() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let settings = Settings::load(&DataPaths::with_root(dir.path()));

        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn неполный_файл_дополняется_умолчаниями() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        std::fs::create_dir_all(dir.path()).expect("каталог");
        std::fs::write(paths.settings(), r#"{"volume": 42}"#).expect("частичные настройки");

        let settings = Settings::load(&paths);
        assert_eq!(settings.volume, 42);
        assert!(settings.subtitles_visible, "остальное берётся по умолчанию");
    }

    #[test]
    fn громкость_из_файла_обрезается() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        std::fs::create_dir_all(dir.path()).expect("каталог");
        std::fs::write(paths.settings(), r#"{"volume": 9000}"#).expect("настройки");

        assert_eq!(Settings::load(&paths).volume, 150);
    }
}
