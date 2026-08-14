//! Сборка приложения при запуске: загрузка данных, настройка движка.
//!
//! Состояние объявлено в `mod.rs` и живёт только там (PLAN.md §12.4) —
//! здесь лишь его первичное заполнение.

use pith_mpv::{EngineOptions, HwDec};
use pith_store::{DataPaths, Settings, WatchPositions};

use crate::bench::Metrics;

use super::{
    ActorsState, IntegrationsState, PithApp, SubtitleText, crop, extraction, import_v4, photos,
    preview, search, subtitles,
};

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
        let options = engine_options(hwdec, &settings);

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
            window_maximized: settings.window_maximized,
            announce_maximized_pending: settings.window_maximized,
            playback_started_pending: false,
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
            key_seek_in_flight: false,
            key_seek_wanted: None,
            key_seek_rough: false,
            key_seek_needs_exact: false,
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
            cast_store: pith_store::CastStore::load(data_paths.clone()),
            sounds: pith_store::SoundStore::load(data_paths.clone()),
            warmup: Default::default(),
            actors: ActorsState::new(settings.actors_window_open),
            photos: photos::PhotoCache::default(),
            integrations: IntegrationsState::default(),
            update: Default::default(),
            update_checked: false,
            export: None,
            settings,
            data_paths,
            subtitle_text: SubtitleText::default(),
            last_subtitle: None,
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
            bookmarks_window_placed: false,
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
            menu_was_open: false,
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
}

/// Собирает настройки движка из сохранённых предпочтений.
fn engine_options(hwdec: HwDec, settings: &Settings) -> EngineOptions {
    EngineOptions {
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
    }
}
