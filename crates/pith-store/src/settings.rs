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
            audio_languages: ["eng", "en"].iter().map(|s| s.to_string()).collect(),
            subtitle_priority: SubtitlePriority::default(),
            subtitles_visible: true,
            main_subtitle: SubtitleLayout::main(),
            secondary_subtitle: SubtitleLayout::secondary(),
            volume: 80,
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
