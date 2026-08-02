//! Состояние приложения и главный цикл.
//!
//! Всё состояние живёт здесь и нигде больше (PLAN.md §12.4) — в v4 оно было
//! размазано по семи partial-файлам `MainForm`.

mod audio;
mod bookmarks;
mod clipboard;
mod extraction;
mod frame;
mod import_v4;
mod lists;
mod playback;
mod search;
mod subtitles;
mod viewport;
mod watching;

use std::path::PathBuf;

use pith_mpv::{Engine, EngineEvent, EngineOptions, HwDec};
use pith_store::{DataPaths, Settings, WatchPositions};

use crate::bench::Metrics;

use clipboard::Notice;
pub use lists::ListDialog;

/// Локальный путь, которого нет на диске.
///
/// URL и потоки не проверяем: их существование определяет сам mpv.
fn is_missing_local_file(path: &str) -> bool {
    if path.contains("://") {
        return false;
    }

    !std::path::Path::new(path).exists()
}

pub use subtitles::SubtitleText;
pub use watching::ResumeOffer;

pub struct PithApp {
    engine: Option<Engine>,
    /// Сообщение об ошибке запуска движка. Показывается вместо интерфейса.
    fatal_error: Option<String>,
    pub metrics: Metrics,
    pub hwdec: HwDec,
    /// Показывать ли панель замеров (этап 0).
    pub show_metrics: bool,
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
    /// Открытый диалог работы со списком отрезков.
    list_dialog: Option<ListDialog>,
    /// Ход нарезки.
    extraction: extraction::ExtractionState,
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
            show_metrics: !args.hide_metrics,
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
            list_dialog: None,
            extraction: extraction::ExtractionState::default(),
        };

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

    /// Открывает файл и запускает замер времени до первого кадра.
    ///
    /// Отсутствующий файл отсекается сразу: mpv на него отвечает лишь
    /// событием ошибки спустя время, и пользователь успевает решить,
    /// что плеер завис.
    pub fn open_file(&mut self, path: &str) {
        if is_missing_local_file(path) {
            tracing::warn!(path, "файла нет на диске");
            self.report_playback_error("Файл не найден");
            return;
        }

        self.playback_error = None;
        self.set_current_path(path);

        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        self.metrics.mark_open_start();

        if let Err(e) = engine.load_file(path) {
            tracing::error!(error = %e, "не удалось открыть файл");
            self.report_playback_error("Не удалось открыть файл");
        }
    }

    /// Перехватывает закрытие окна и освобождает движок до уничтожения окна.
    ///
    /// Возвращает `true`, если закрытие обработано и остальную работу
    /// кадра делать не нужно.
    ///
    /// Пока mpv держит загруженный файл, уничтожение окна зависает.
    /// Поэтому первое закрытие отменяем, освобождаем движок и просим
    /// закрыться заново — на следующем кадре освобождать уже нечего
    /// и окно закрывается штатно.
    fn handle_close_request(&mut self, ctx: &egui::Context) -> bool {
        if !ctx.input(|i| i.viewport().close_requested()) {
            return false;
        }

        if self.engine.is_none() {
            // Движок уже освобождён — не мешаем закрытию.
            return true;
        }

        tracing::debug!("запрос на закрытие окна, освобождаю движок");
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        self.shutdown_engine();
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        true
    }

    /// Останавливает воспроизведение и освобождает движок.
    ///
    /// Вызывается при закрытии окна. Повторный вызов безвреден.
    fn shutdown_engine(&mut self) {
        // Позицию сохраняем до остановки: после неё mpv уже не отдаст время.
        self.store_position();

        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        // Сначала останавливаем декодирование: иначе mpv ждёт отрисовки
        // очередного кадра, а освобождение контекста ждёт mpv.
        engine.stop();
        tracing::info!(
            ссылок_на_контекст = engine.render_context_refs(),
            "воспроизведение остановлено"
        );

        // Drop разбирает поля по порядку: сначала контекст отрисовки, затем mpv.
        self.engine = None;
        tracing::info!("движок освобождён");
    }

    /// Открывает файлы, присланные другими запусками плеера.
    ///
    /// Берём последний: если пользователь быстро открыл несколько файлов,
    /// он ждёт именно тот, что кликнул последним.
    fn accept_files_from_other_instances(&mut self, ctx: &egui::Context) {
        let Some(path) = self
            .instance
            .pending_files()
            .into_iter()
            .rfind(|p| crate::single_instance::is_openable(p))
        else {
            return;
        };

        self.open_file(&path.to_string_lossy());

        // Поднимаем окно: пользователь только что кликнул по файлу.
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    }

    /// Разбор событий движка перед отрисовкой кадра.
    fn process_engine_events(&mut self) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        let mut file_loaded = false;
        let mut playback_failed = false;

        for event in engine.pump_events() {
            match event {
                EngineEvent::FileLoaded => {
                    engine.refresh_video_size();
                    let active_hwdec = engine.active_hwdec();
                    let state = engine.state();
                    tracing::info!(
                        width = state.display_width,
                        height = state.display_height,
                        hwdec_active = %active_hwdec,
                        "файл загружен"
                    );
                    file_loaded = true;
                }
                EngineEvent::EndFile => tracing::debug!("файл закончился"),
                EngineEvent::PlaybackError => playback_failed = true,
                EngineEvent::Shutdown => tracing::info!("mpv завершает работу"),
            }
        }

        engine.refresh_state();

        if playback_failed {
            self.handle_playback_error();
        }

        if file_loaded {
            // Файл пошёл — прошлая жалоба больше не про него.
            self.playback_error = None;
            // Подгонять окно будем в кадре: там доступны размеры экрана.
            self.fit_window_pending = true;
            self.window_resized_by_user = false;
            self.prepare_resume_offer();
            self.select_tracks_by_tags();
            self.refresh_tracks();
            // Субтитры прошлого файла к новому отношения не имеют.
            self.reset_search();
        }

        self.poll_subtitle_extraction();
        self.poll_extraction();
        self.refresh_subtitle_text();
        self.store_position_periodically();

        // Перемотка считается завершённой, когда mpv отдал новую позицию.
        if self.seek_pending {
            self.seek_pending = false;
            self.metrics.mark_seek_done();
        }
    }
}
