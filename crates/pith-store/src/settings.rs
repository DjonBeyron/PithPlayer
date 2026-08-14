//! Настройки плеера.

use serde::{Deserialize, Serialize};

use crate::file::{read_json, write_json};
use crate::language::Language;
use crate::notion::NotionSettings;
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

/// Ширина панели отрезков, пока её не растягивали.
pub const DEFAULT_PANEL_WIDTH: f32 = 320.0;

/// Уже этого панель бесполезна: кнопки нарезки перестают помещаться.
pub const MIN_PANEL_WIDTH: f32 = 260.0;

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

    /// Окно было развёрнуто кнопкой заголовка при прошлом закрытии.
    ///
    /// Отдельно от размеров: развёрнутое окно нужно и открывать
    /// развёрнутым, а не окном размером с экран — у второго кнопка
    /// заголовка показывает не то состояние, и «свернуть обратно»
    /// возвращать некуда. Размеры при этом хранятся прежние, до разворота:
    /// именно в них окно и вернётся по нажатию кнопки.
    pub window_maximized: bool,

    /// Устройство вывода звука (`audio-device` в mpv).
    ///
    /// `None` — выбирает mpv. Имя устройства привязано к системе, поэтому
    /// на другой машине оно просто не найдётся, и звук пойдёт как обычно.
    pub audio_device: Option<String>,

    /// Настройки нарезки отрезков.
    pub fragments: FragmentSettings,

    /// Ключ доступа к базе фильмов TMDB.
    ///
    /// Пусто — состав актёров не запрашивается, окно просит ключ.
    /// Бесплатный, выдаётся сразу после регистрации на themoviedb.org.
    /// В поставку не входит: ключ у каждого свой, и предел запросов тоже.
    pub tmdb_key: String,

    /// Где стояло окно актёров и открытым ли его закрыли.
    pub actors_window: Option<WindowGeometry>,
    pub actors_window_open: bool,

    /// Где стояло окно выгрузки в Notion.
    ///
    /// Открытым его не запоминаем: выгрузку затевают заново каждый раз,
    /// а вот искать окно по экранам всякий раз незачем.
    pub export_window: Option<WindowGeometry>,

    /// Панель отрезков откреплена в своё окно.
    ///
    /// Откреплённая живёт отдельным окном системы: её уносят на второй
    /// экран и оставляют там рядом с кадром, а не поверх него.
    pub bookmarks_panel_detached: bool,
    /// Где стояло открепленное окно панели.
    pub bookmarks_window: Option<WindowGeometry>,

    /// Ширина панели отрезков в точках.
    ///
    /// Панель тянут за левый край: на длинных репликах узкая полоса
    /// нечитаема, а кому-то она нужна почти во весь экран. Ширина
    /// запоминается между запусками и приводится к размеру окна при показе.
    pub bookmarks_panel_width: f32,

    /// Выгружается сериал, а не фильм.
    ///
    /// Ответы окна выгрузки помнятся между запусками: человек режет
    /// один и тот же сериал неделями, и выбирать одно и то же каждый раз —
    /// работа на пустом месте.
    pub export_series: bool,
    /// Заполнять транскрипцию реплик.
    pub export_transcribe: bool,
    /// Вырезать отрезки сразу после выгрузки.
    pub export_cut_after: bool,

    /// Доступ к Notion: токен интеграции и страницы.
    ///
    /// Пусто — выгрузка не подключена, и кнопка отрезков открывает окно
    /// интеграций вместо выгрузки.
    pub notion: NotionSettings,
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
            window_maximized: false,
            audio_device: None,
            fragments: FragmentSettings::default(),
            tmdb_key: String::new(),
            actors_window: None,
            actors_window_open: false,
            export_window: None,
            bookmarks_panel_detached: false,
            bookmarks_window: None,
            bookmarks_panel_width: DEFAULT_PANEL_WIDTH,
            // Сериал: коротких отрезков из сериалов режется больше.
            export_series: true,
            export_transcribe: true,
            export_cut_after: false,
            notion: NotionSettings::default(),
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

        // Верхнего предела здесь нет: он зависит от окна и накладывается
        // при показе панели. А вот бессмысленно узкую ширину — из правки
        // руками или из другого экрана — поправим сразу.
        if !settings.bookmarks_panel_width.is_finite()
            || settings.bookmarks_panel_width < MIN_PANEL_WIDTH
        {
            settings.bookmarks_panel_width = DEFAULT_PANEL_WIDTH;
        }

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
    fn ширина_панели_переживает_перезапуск() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        Settings {
            bookmarks_panel_width: 900.0,
            ..Default::default()
        }
        .save(&paths);

        assert_eq!(Settings::load(&paths).bookmarks_panel_width, 900.0);
    }

    #[test]
    fn бессмысленная_ширина_панели_поправляется() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        std::fs::create_dir_all(dir.path()).expect("каталог");
        std::fs::write(paths.settings(), r#"{"bookmarks_panel_width": 12}"#).expect("настройки");

        assert_eq!(
            Settings::load(&paths).bookmarks_panel_width,
            super::DEFAULT_PANEL_WIDTH,
            "уже предела панель бесполезна"
        );
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
