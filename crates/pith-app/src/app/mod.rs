//! Состояние приложения и главный цикл.
//!
//! Всё состояние живёт здесь и нигде больше (PLAN.md §12.4) — в v4 оно было
//! размазано по семи partial-файлам `MainForm`.

mod clipboard;
mod import_v4;
mod playback;
mod subtitles;
mod viewport;
mod watching;

use std::path::PathBuf;

use pith_mpv::{Engine, EngineEvent, EngineOptions, HwDec};
use pith_store::{DataPaths, Settings, WatchPositions};

use crate::bench::Metrics;
use crate::ui;
use crate::video;

use clipboard::Notice;
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
    /// Время текущего кадра — по нему гаснут уведомления.
    frame_time: f64,
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
        let settings = Settings::load(&data_paths);

        let hwdec = args.hwdec.unwrap_or_default();
        let options = EngineOptions {
            hwdec,
            volume: settings.volume,
            audio_languages: settings.audio_languages.clone(),
            subtitle_languages: settings.subtitle_priority.main_tags.clone(),
        };

        let mut watch_positions = WatchPositions::load(data_paths.clone());

        // Первый запуск: переносим данные версии 4.
        let migration = import_v4::run_once(
            &data_paths,
            &mut watch_positions,
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
            frame_time: 0.0,
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

    /// Отрезки, которые попадут в сохранённые фрагменты.
    ///
    /// Показываются на полосе перемотки жёлтым. Закладки появятся на
    /// этапе 4 — до тех пор список пуст.
    pub fn fragment_ranges(&self) -> Vec<crate::ui::FragmentRange> {
        Vec::new()
    }

    /// Открывает файл и запускает замер времени до первого кадра.
    pub fn open_file(&mut self, path: &str) {
        self.set_current_path(path);

        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        self.metrics.mark_open_start();

        if let Err(e) = engine.load_file(path) {
            tracing::error!(error = %e, "не удалось открыть файл");
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
                EngineEvent::Shutdown => tracing::info!("mpv завершает работу"),
            }
        }

        engine.refresh_state();

        if file_loaded {
            // Подгонять окно будем в кадре: там доступны размеры экрана.
            self.fit_window_pending = true;
            self.window_resized_by_user = false;
            self.prepare_resume_offer();
            self.select_tracks_by_tags();
        }

        self.refresh_subtitle_text();
        self.store_position_periodically();

        // Перемотка считается завершённой, когда mpv отдал новую позицию.
        if self.seek_pending {
            self.seek_pending = false;
            self.metrics.mark_seek_done();
        }
    }
}

impl eframe::App for PithApp {
    /// Логика кадра: разбор событий движка и горячие клавиши.
    /// Рисовать здесь нельзя (требование eframe).
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.handle_close_request(ctx) {
            return;
        }

        self.accept_files_from_other_instances(ctx);
        self.process_engine_events();
    }

    /// Освобождение движка до закрытия окна.
    ///
    /// Контекст отрисовки mpv держит ресурсы OpenGL. Если не уничтожить его
    /// здесь, пока контекст GL ещё жив, приложение зависает при выходе.
    /// Фон окна — непрозрачный чёрный.
    ///
    /// Задаётся явно: mpv не заполняет буфер там, где нет кадра — поля по
    /// краям при несовпадении пропорций и момент до первого кадра.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }

    /// Страховка: если закрытие пришло мимо кадра egui, освобождаем здесь.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.shutdown_engine();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame_time = ui.ctx().input(|i| i.time);

        // Пользователь закрывает окно — освобождаем движок прямо здесь,
        // пока контекст OpenGL и окно ещё живы.
        //
        // В `on_exit` делать это поздно: eframe до него не доходит.
        // Контекст отрисовки mpv держит ресурсы OpenGL, и уничтожение окна
        // блокируется, пока mpv их не отпустит, — приложение зависает.
        if self.handle_close_request(ui.ctx()) {
            return;
        }

        // Клавиши разбираются здесь, а не в `logic`: там кадр egui ещё
        // не начат и `input()` не отдаёт нажатия.
        ui::handle_hotkeys(self, ui.ctx());

        self.track_manual_resize(ui.ctx());
        self.fit_window_to_video(ui.ctx());
        self.track_pointer_activity(ui.ctx());

        if let Some(message) = self.fatal_error.clone() {
            ui::show_fatal_error(ui, &message);
            return;
        }

        // Видео рисуется первым, на всю область окна. Элементы управления
        // накладываются поверх отдельным слоем — иначе mpv их закрасит.
        let render_context = self.engine.as_ref().and_then(|e| e.shared_render_context());
        let mut frame_painted = false;

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ui, |ui| {
                let rect = ui.available_rect_before_wrap();

                if let Some(context) = render_context
                    && video::paint(ui, rect, context)
                {
                    frame_painted = true;
                }
            });

        ui::show_subtitles(self, ui.ctx());
        ui::show_controls(self, ui.ctx());
        ui::show_notice(self, ui.ctx());
        ui::show_migration_report(self, ui.ctx());
        ui::show_resume_offer(self, ui.ctx());

        if frame_painted {
            self.metrics.record_frame();

            // Первым кадром считается только настоящий кадр видео: до загрузки
            // файла egui рисует пустую чёрную область, и засчитывать её нельзя.
            if self.engine.as_ref().is_some_and(|e| e.state().file_loaded) {
                self.metrics.mark_first_frame();
            }
        }

        // Пока идёт воспроизведение, интерфейс обновляет позицию и замеры.
        if self
            .engine
            .as_ref()
            .is_some_and(|e| !e.state().paused && e.state().file_loaded)
        {
            ui.ctx().request_repaint();
        }
    }
}
