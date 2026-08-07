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
mod history;
mod import_v4;
mod lifecycle;
mod list_dialog;
mod lists;
mod playback;
mod preview;
mod preview_source;
mod search;
mod subtitle_style;
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
    /// Куда перематываем. Показывается вместо позиции mpv, пока он догоняет.
    seek_target: Option<f64>,
    /// Куда нужно перемотать, когда движок освободится.
    scrub_wanted: Option<f64>,
    /// Перемотка отправлена и ещё не завершилась.
    scrub_in_flight: bool,
    /// Куда уже отправляли: не просим движок о том же месте дважды.
    scrub_sent: Option<f64>,
    /// Паузу поставили мы сами на время перетаскивания.
    paused_by_scrub: bool,
    /// Пользователь тянет ползунок прямо сейчас.
    scrubbing: bool,
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
    /// Открыто ли окно настройки вида субтитров.
    subtitle_style_open: bool,
    /// Цвет или начертание меняли — их нужно записать в настройки.
    subtitle_style_dirty: bool,
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
    /// Предпросмотр кадра при перемотке.
    preview: preview::PreviewState,
    /// Состояние, которое показывает значок в центре кадра.
    badge_paused: bool,
    /// Когда паузу переключали последний раз. `None` — ещё не трогали.
    badge_started: Option<f64>,
    /// Когда окно последний раз вернуло себе фокус.
    focus_regained_at: Option<f64>,
    /// Громкость меняли — её нужно запомнить в настройках.
    volume_changed: bool,
    /// До какого времени висит подсказка о перемотке клавишами.
    seek_hud_until: Option<f64>,
    /// История открытых файлов и папок.
    history: pith_store::History,
    /// Открыто ли окно истории.
    history_open: bool,
    /// Время кадра, в котором историю открыли.
    history_opened_at: Option<f64>,
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

        // Язык поднимаем до первого кадра: иначе интерфейс успел бы
        // мелькнуть по-русски у того, кто выбрал английский.
        Self::choose_language(&mut settings, &data_paths, args.language);

        let hwdec = args.hwdec.unwrap_or_default();
        let options = EngineOptions {
            hwdec,
            volume: settings.volume,
            muted: settings.muted,
            looping: settings.looping,
            audio_languages: settings.audio_languages.clone(),
            subtitle_languages: settings.subtitle_priority.main_tags.clone(),
            audio_device: settings.audio_device.clone(),
            // Подробный журнал mpv включается переменной окружения
            // PITH_MPV_LOG=<путь>: он нужен для разбора того, на что
            // уходит время открытия файла, и в обычной работе не пишется.
            log_file: std::env::var("PITH_MPV_LOG").ok(),
        };

        let mut watch_positions = WatchPositions::load(data_paths.clone());
        let history = pith_store::History::load(data_paths.clone());

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
            // Размер первого окна выбран до его создания — по форме кадра,
            // узнанной у демуксера (main::restore_geometry). Менять его
            // после загрузки файла нельзя: окно уже на экране, и правка
            // видна скачком. Остаётся только поправить форму, если размеры
            // узнать не удалось.
            restored_geometry_pending: true,
            seek_pending: false,
            seek_target: None,
            scrub_wanted: None,
            scrub_in_flight: false,
            scrub_sent: None,
            paused_by_scrub: false,
            scrubbing: false,
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
            subtitle_style_open: false,
            subtitle_style_dirty: false,
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
            preview: preview::PreviewState::default(),
            badge_paused: false,
            badge_started: None,
            focus_regained_at: None,
            volume_changed: false,
            seek_hud_until: None,
            history,
            history_open: false,
            history_opened_at: None,
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

    pub fn engine(&self) -> Option<&Engine> {
        self.engine.as_ref()
    }

    /// Открыто ли окно, для которого Escape означает «закрыть».
    ///
    /// Пока такое окно на экране, Escape принадлежит ему. Иначе одно
    /// нажатие делает два дела сразу: закрывает окно и заодно бросает
    /// плеер в полноэкранный режим.
    pub fn escape_belongs_to_window(&self) -> bool {
        self.search.open
            || self.list_dialog.is_some()
            || self.bookmark_rename.is_some()
            || self.clear_list_pending
            || self.fragment_settings.is_some()
            || self.file_types_prompt.is_some()
            || self.subtitle_style_open
    }

    /// Открыто ли поверх кадра окно, которому принадлежат клавиши.
    ///
    /// Такие окна сами разбирают Escape и Enter, и горячие клавиши плеера
    /// на это время замолкают: иначе Escape закрывал окно и одновременно
    /// переключал полный экран.
    pub fn dialog_open(&self) -> bool {
        self.history_open
            || self.search.open
            || self.list_dialog.is_some()
            || self.bookmark_rename.is_some()
            || self.clear_list_pending
            || self.fragment_settings.is_some()
            || self.file_types_prompt.is_some()
            || self.migration.is_some()
            || self.subtitle_style_open
    }

    /// Выбирает язык при запуске.
    ///
    /// Порядок такой: выбор из установщика, потом сохранённый в настройках,
    /// потом язык системы. Установщик спрашивает язык в мастере, и второй
    /// раз его спрашивать незачем; на чистой машине настроек ещё нет —
    /// берём тот, на котором говорит Windows.
    fn choose_language(
        settings: &mut Settings,
        paths: &DataPaths,
        chosen: Option<pith_store::Language>,
    ) {
        let first_run = !paths.settings().exists();

        let language = match (chosen, first_run) {
            (Some(language), _) => language,
            (None, true) => crate::system_language::detect(),
            (None, false) => settings.language,
        };

        if settings.language != language {
            settings.language = language;
            settings.save(paths);
        }

        crate::i18n::set(language);
        tracing::info!(?language, первый_запуск = first_run, "язык интерфейса");
    }

    /// Язык интерфейса.
    pub fn language(&self) -> pith_store::Language {
        self.settings.language
    }

    /// Переключает язык интерфейса. Выбор запоминается.
    pub fn set_language(&mut self, language: pith_store::Language) {
        if self.settings.language == language {
            return;
        }

        self.settings.language = language;
        self.settings.save(&self.data_paths);
        crate::i18n::set(language);

        tracing::info!(?language, "язык интерфейса переключён");
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
