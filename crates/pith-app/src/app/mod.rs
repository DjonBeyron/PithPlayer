//! Состояние приложения и главный цикл.
//!
//! Всё состояние живёт здесь и нигде больше (PLAN.md §12.4) — в v4 оно было
//! размазано по семи partial-файлам `MainForm`.

mod audio;
mod bookmark_rename;
mod bookmarks;
mod clipboard;
mod crop;
mod extraction;
mod extraction_queue;
mod file_types;
mod fragment_settings;
mod frame;
mod import_v4;
mod lifecycle;
mod lists;
mod playback;
mod search;
mod subtitles;
mod viewport;
mod watching;
mod window_state;

use std::path::PathBuf;

use pith_mpv::{Engine, EngineOptions, HwDec};
use pith_store::{DataPaths, Settings, WatchPositions};

use crate::bench::Metrics;

pub use bookmark_rename::BookmarkRename;
use clipboard::Notice;
pub use file_types::FileTypesPrompt;
pub use fragment_settings::FragmentSettingsDialog;
pub use lists::ListDialog;
pub use subtitles::SubtitleText;
pub use watching::ResumeOffer;

pub struct PithApp {
    engine: Option<Engine>,
    /// Сообщение об ошибке запуска движка. Показывается вместо интерфейса.
    fatal_error: Option<String>,
    pub metrics: Metrics,
    pub hwdec: HwDec,
    /// Показывать ли панель замеров. Выбор запоминается между запусками.
    show_metrics: bool,
    /// Что сейчас написано в заголовке окна — чтобы не слать команду зря.
    window_title: Option<String>,
    /// Последнее известное положение окна — запоминается при закрытии.
    window_geometry: Option<pith_store::WindowGeometry>,
    /// Нужно проверить, что восстановленное окно видно на экране.
    window_position_pending: bool,
    /// Окно восстановлено из настроек, и первый файл его не переставляет.
    restored_geometry_pending: bool,
    /// Перемотка ожидает подтверждения — нужна для замера её длительности.
    seek_pending: bool,
    /// Полноэкранный режим.
    fullscreen: bool,
    /// Когда мышь двигалась последний раз — по этому прячется панель.
    last_pointer_activity: f64,
    /// Подгонять ли окно под форму видео.
    fit_window_enabled: bool,
    /// Пользователь менял размер окна вручную — больше не навязываем подгонку.
    window_resized_by_user: bool,
    /// Размер окна, который мы задали сами: помогает отличить свой ресайз
    /// от пользовательского.
    expected_window_size: Option<egui::Vec2>,
    /// Файл только что загружен — нужно подогнать окно в ближайшем кадре.
    fit_window_pending: bool,
    /// Позиции просмотра.
    watch_positions: WatchPositions,
    /// Путь к текущему файлу — ключ для позиции просмотра.
    current_path: Option<PathBuf>,
    /// Предложение продолжить просмотр, пока пользователь не ответил.
    resume_offer: Option<ResumeOffer>,
    /// Позиция, записанная в хранилище последней.
    last_position_save: f64,
    /// Приём файлов от других запусков плеера.
    instance: crate::single_instance::InstanceServer,
    /// Итог переноса данных из версии 4 — показывается один раз.
    migration: Option<pith_store::MigrationReport>,
    /// Настройки плеера.
    settings: Settings,
    /// Каталог данных — нужен для сохранения настроек.
    data_paths: DataPaths,
    /// Текущие реплики субтитров.
    subtitle_text: SubtitleText,
    /// Всплывающее уведомление.
    notice: Option<Notice>,
    /// Почему не играет текущий файл.
    ///
    /// В отличие от уведомления не гаснет: иначе пользователь остаётся
    /// перед чёрным окном без единого объяснения.
    playback_error: Option<String>,
    /// Время текущего кадра — по нему гаснут уведомления.
    frame_time: f64,
    /// Дорожки текущего файла. Список меняется только при смене файла.
    tracks: Vec<pith_mpv::Track>,
    /// Какие дорожки выбраны сейчас.
    selected_tracks: subtitles::SelectedTracks,
    /// Поиск по субтитрам.
    search: search::SearchState,
    /// Закладки и списки отрезков.
    bookmarks: pith_store::Bookmarks,
    /// Панель отрезков показана наведением на правый край.
    bookmarks_panel: bool,
    /// Панель закреплена через меню и не прячется сама.
    bookmarks_panel_pinned: bool,
    /// Сколько кадров панель ещё рисуется невидимой.
    ///
    /// Первый кадр egui считает разметку списка закладок и полосу прокрутки,
    /// и при длинном списке видно, как панель достраивается. Показываем её,
    /// когда размеры уже посчитаны.
    bookmarks_panel_warmup: u8,
    /// Открытый диалог работы со списком отрезков.
    list_dialog: Option<ListDialog>,
    /// Открытое переименование закладки.
    bookmark_rename: Option<BookmarkRename>,
    /// Ждёт подтверждения очистка списка закладок.
    clear_list_pending: bool,
    /// Открытый диалог общих настроек нарезки.
    fragment_settings: Option<FragmentSettingsDialog>,
    /// Спрошенное подтверждение на смену файловых ассоциаций.
    file_types_prompt: Option<FileTypesPrompt>,
    /// Связаны ли файлы с плеером. `None` — ещё не спрашивали реестр.
    file_types_registered: Option<bool>,
    /// Ход нарезки.
    extraction: extraction::ExtractionState,
    /// Обрезка чёрных полей.
    crop: crop::CropState,
}

impl PithApp {
    /// Создаёт приложение. Ошибка запуска движка не роняет программу —
    /// окно откроется и покажет причину.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        args: crate::cli::Args,
        instance: crate::single_instance::InstanceServer,
    ) -> Self {
        let data_paths = DataPaths::discover();
        let mut settings = Settings::load(&data_paths);
        let mut bookmarks = pith_store::Bookmarks::load(data_paths.clone());

        let hwdec = args.hwdec.unwrap_or_default();
        let options = EngineOptions {
            hwdec,
            volume: settings.volume,
            audio_languages: settings.audio_languages.clone(),
            subtitle_languages: settings.subtitle_priority.main_tags.clone(),
            audio_device: settings.audio_device.clone(),
        };

        let mut watch_positions = WatchPositions::load(data_paths.clone());

        // Первый запуск: переносим данные версии 4.
        let migration = import_v4::run_once(
            &data_paths,
            &mut watch_positions,
            &mut settings,
            &mut bookmarks,
            args.import_from.as_deref(),
        );

        let mut app = Self {
            engine: None,
            fatal_error: None,
            metrics: Metrics::default(),
            hwdec,
            // Ключ командной строки перекрывает настройку: он нужен
            // для замеров, где панель мешает снимкам.
            show_metrics: settings.show_metrics && !args.hide_metrics,
            window_title: None,
            window_geometry: settings.window,
            // Окно уже создано с сохранёнными координатами: проверим
            // в первом же кадре, что оно попало на существующий экран.
            window_position_pending: settings.window.is_some(),
            restored_geometry_pending: settings.window.is_some(),
            seek_pending: false,
            fullscreen: false,
            last_pointer_activity: 0.0,
            fit_window_enabled: !args.no_fit_window,
            window_resized_by_user: false,
            expected_window_size: None,
            fit_window_pending: false,
            watch_positions,
            current_path: None,
            resume_offer: None,
            last_position_save: 0.0,
            instance,
            migration,
            settings,
            data_paths,
            subtitle_text: SubtitleText::default(),
            notice: None,
            playback_error: None,
            frame_time: 0.0,
            tracks: Vec::new(),
            selected_tracks: subtitles::SelectedTracks::default(),
            search: search::SearchState::default(),
            bookmarks,
            bookmarks_panel: false,
            bookmarks_panel_pinned: false,
            bookmarks_panel_warmup: 0,
            list_dialog: None,
            bookmark_rename: None,
            clear_list_pending: false,
            fragment_settings: None,
            file_types_prompt: None,
            file_types_registered: None,
            extraction: extraction::ExtractionState::default(),
            crop: crop::CropState::default(),
        };

        // Проверка наличия FFmpeg запускает внешний процесс. Делаем это
        // в фоне при старте, чтобы она не досталась кадру интерфейса.
        pith_fragments::warm_up();

        match Self::start_engine(cc, &options) {
            Ok(engine) => app.engine = Some(engine),
            Err(message) => {
                tracing::error!("{message}");
                app.fatal_error = Some(message);
            }
        }

        if let Some(path) = args.file {
            app.open_file(&path);
        }

        app
    }

    /// Запускает движок и подключает его к контексту OpenGL окна.
    fn start_engine(
        cc: &eframe::CreationContext<'_>,
        options: &EngineOptions,
    ) -> Result<Engine, String> {
        let loader = cc.get_proc_address.clone().ok_or_else(|| {
            "контекст OpenGL недоступен: eframe не отдал загрузчик функций".to_string()
        })?;

        let mut engine = Engine::new(options).map_err(|e| e.to_string())?;

        // Пробуждаем интерфейс, когда mpv готов показать новый кадр.
        // Внутри обратного вызова обращаться к mpv нельзя.
        let egui_ctx = cc.egui_ctx.clone();
        engine
            .init_render_context(loader, move || egui_ctx.request_repaint())
            .map_err(|e| e.to_string())?;

        Ok(engine)
    }

    pub fn engine(&self) -> Option<&Engine> {
        self.engine.as_ref()
    }

    /// Видна ли панель замеров.
    pub fn show_metrics(&self) -> bool {
        self.show_metrics
    }

    /// Прячет или возвращает панель замеров. Выбор запоминается.
    pub fn toggle_metrics(&mut self) {
        self.show_metrics = !self.show_metrics;
        self.settings.show_metrics = self.show_metrics;
        self.settings.save(&self.data_paths);
    }
}
