//! Настройки плеера.

use serde::{Deserialize, Serialize};

use crate::file::{read_json, write_json};
use crate::language::Language;
use crate::paths::DataPaths;
use crate::subtitle_layout::SubtitleLayout;
use crate::subtitle_priority::SubtitlePriority;

const FORMAT_VERSION: u32 = 1;

/// Положение и размер окна в точках экрана.
///
/// Координаты — от левого верхнего угла рабочего стола, поэтому окно
/// на втором мониторе имеет `x` больше ширины основного экрана. Именно
/// так плеер и запоминает, на каком экране его закрыли.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl WindowGeometry {
    /// Разумны ли сохранённые размеры.
    ///
    /// Файл могли поправить руками, а окно нулевого размера открыть нельзя.
    pub fn is_sane(&self) -> bool {
        let finite = self.x.is_finite() && self.y.is_finite();
        let sized = (MIN_SIDE..=MAX_SIDE).contains(&self.width)
            && (MIN_SIDE..=MAX_SIDE).contains(&self.height);

        finite && sized
    }
}

/// Пределы правдоподобных размеров окна, точки.
const MIN_SIDE: f32 = 200.0;
const MAX_SIDE: f32 = 16384.0;

/// Настройки нарезки отрезков (PLAN.md §6.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FragmentSettings {
    /// Куда складывать вырезанные фрагменты.
    pub output_dir: Option<std::path::PathBuf>,
    /// Длительность фрагмента, секунды.
    pub duration_sec: u32,
    /// Отступ назад от метки, секунды.
    pub buffer_sec: u32,
    /// Перекодировать вместо перепаковки.
    ///
    /// По умолчанию выключено: перепаковка в десятки раз быстрее и не
    /// теряет качества. Включается, когда нужен старт строго по метке
    /// или целевая программа не принимает исходный кодек.
    pub reencode: bool,
    /// Приводить звук к AAC, оставляя видео копией.
    ///
    /// Включено по умолчанию, как в v4: Premiere Pro и After Effects не
    /// читают EAC3, DTS и подобные дорожки — файл открывается, но звука
    /// в нём для монтажной программы нет. Видео при этом не перекодируется.
    pub audio_aac: bool,
    /// Сколько фрагментов резать одновременно. Ноль — определить самим.
    pub parallel_jobs: usize,
}

impl Default for FragmentSettings {
    fn default() -> Self {
        Self {
            output_dir: None,
            duration_sec: 18,
            buffer_sec: 5,
            reencode: false,
            audio_aac: true,
            parallel_jobs: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,

    /// Язык интерфейса.
    pub language: Language,

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

    /// Звук выключен.
    ///
    /// Отдельно от громкости: выключили звук — при следующем запуске он
    /// остаётся выключенным, а громкость ждёт прежняя.
    pub muted: bool,

    /// Повторять файл по кругу.
    pub looping: bool,

    /// Показывать панель замеров производительности.
    pub show_metrics: bool,

    /// Где и какого размера было окно при прошлом закрытии.
    ///
    /// `None` — плеер ещё не закрывали; тогда окно открывается по центру
    /// основного экрана с обычными размерами.
    pub window: Option<WindowGeometry>,

    /// Устройство вывода звука (`audio-device` в mpv).
    ///
    /// `None` — выбирает mpv. Имя устройства привязано к системе, поэтому
    /// на другой машине оно просто не найдётся, и звук пойдёт как обычно.
    pub audio_device: Option<String>,

    /// Настройки нарезки отрезков.
    pub fragments: FragmentSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            language: Language::default(),
            audio_languages: ["eng", "en"].iter().map(|s| s.to_string()).collect(),
            subtitle_priority: SubtitlePriority::default(),
            subtitles_visible: true,
            main_subtitle: SubtitleLayout::main(),
            secondary_subtitle: SubtitleLayout::secondary(),
            volume: 80,
            muted: false,
            looping: false,
            show_metrics: true,
            window: None,
            audio_device: None,
            fragments: FragmentSettings::default(),
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
    fn язык_запоминается() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        Settings {
            language: Language::En,
            ..Default::default()
        }
        .save(&paths);

        assert_eq!(Settings::load(&paths).language, Language::En);
    }

    #[test]
    fn без_записи_о_языке_остаётся_русский() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        std::fs::create_dir_all(dir.path()).expect("каталог");
        std::fs::write(paths.settings(), r#"{"volume": 42}"#).expect("настройки");

        assert_eq!(Settings::load(&paths).language, Language::Ru);
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
