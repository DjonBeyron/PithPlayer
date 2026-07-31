//! Состояние приложения и главный цикл.
//!
//! Всё состояние живёт здесь и нигде больше (PLAN.md §12.4) — в v4 оно было
//! размазано по семи partial-файлам `MainForm`.

use pith_mpv::{Engine, EngineEvent, EngineOptions, HwDec};

use crate::bench::Metrics;
use crate::ui;
use crate::video;

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
}

impl PithApp {
    /// Создаёт приложение. Ошибка запуска движка не роняет программу —
    /// окно откроется и покажет причину.
    pub fn new(cc: &eframe::CreationContext<'_>, args: crate::cli::Args) -> Self {
        let hwdec = args.hwdec.unwrap_or_default();
        let options = EngineOptions {
            hwdec,
            ..Default::default()
        };

        let mut app = Self {
            engine: None,
            fatal_error: None,
            metrics: Metrics::default(),
            hwdec,
            show_metrics: !args.hide_metrics,
            seek_pending: false,
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

    pub fn engine_mut(&mut self) -> Option<&mut Engine> {
        self.engine.as_mut()
    }

    /// Открывает файл и запускает замер времени до первого кадра.
    pub fn open_file(&mut self, path: &str) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        self.metrics.mark_open_start();

        if let Err(e) = engine.load_file(path) {
            tracing::error!(error = %e, "не удалось открыть файл");
        }
    }

    /// Диалог выбора файла.
    pub fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Видео и аудио",
                &[
                    "mkv", "mp4", "avi", "mov", "webm", "ts", "m2ts", "m4v", "flv", "wmv", "mpg",
                    "mpeg", "vob", "ogv", "3gp", "mp3", "flac", "wav", "aac", "m4a", "opus",
                ],
            )
            .add_filter("Все файлы", &["*"])
            .pick_file()
        {
            self.open_file(&path.to_string_lossy());
        }
    }

    /// Перемотка относительно текущей позиции с замером длительности.
    pub fn seek_relative(&mut self, seconds: f64) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        self.metrics.mark_seek_start();
        self.seek_pending = true;

        if let Err(e) = engine.seek_relative(seconds) {
            tracing::warn!(error = %e, "перемотка не удалась");
            self.seek_pending = false;
        }
    }

    /// Перемотка на абсолютную позицию.
    pub fn seek_absolute(&mut self, seconds: f64) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        self.metrics.mark_seek_start();
        self.seek_pending = true;

        if let Err(e) = engine.seek_absolute(seconds) {
            tracing::warn!(error = %e, "перемотка не удалась");
            self.seek_pending = false;
        }
    }

    pub fn toggle_pause(&mut self) {
        if let Some(engine) = self.engine.as_mut()
            && let Err(e) = engine.toggle_pause()
        {
            tracing::warn!(error = %e, "не удалось переключить паузу");
        }
    }

    pub fn adjust_volume(&mut self, delta: i64) {
        if let Some(engine) = self.engine.as_mut() {
            let target = engine.state().volume + delta;
            if let Err(e) = engine.set_volume(target) {
                tracing::warn!(error = %e, "не удалось изменить громкость");
            }
        }
    }

    /// Разбор событий движка перед отрисовкой кадра.
    fn process_engine_events(&mut self) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };

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
                }
                EngineEvent::EndFile => tracing::debug!("файл закончился"),
                EngineEvent::Shutdown => tracing::info!("mpv завершает работу"),
            }
        }

        engine.refresh_state();

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
        self.process_engine_events();
        ui::handle_hotkeys(self, ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(message) = self.fatal_error.clone() {
            ui::show_fatal_error(ui, &message);
            return;
        }

        ui::show_controls(self, ui);

        // Видео рисуется в центральной области, под элементами управления.
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
