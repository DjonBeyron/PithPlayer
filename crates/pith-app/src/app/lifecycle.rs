//! Жизненный цикл: открытие файлов, события движка, закрытие окна.

use pith_mpv::{Engine, EngineEvent, EngineOptions};

use super::PithApp;

impl PithApp {
    /// Открывает файл и запускает замер времени до первого кадра.
    ///
    /// Отсутствующий файл отсекается сразу: mpv на него отвечает лишь
    /// событием ошибки спустя время, и пользователь успевает решить,
    /// что плеер завис.
    pub fn open_file(&mut self, path: &str) {
        if is_missing_local_file(path) {
            tracing::warn!(path, "файла нет на диске");
            self.report_playback_error(crate::tr!("Файл не найден", "File not found"));
            return;
        }

        self.playback_error = None;
        self.set_current_path(path);
        self.remember_in_history(path);
        // Мозаика миниатюр и второй экземпляр mpv были про прошлый файл.
        self.reset_preview();

        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        self.metrics.mark_open_start();

        if let Err(e) = engine.load_file(path) {
            tracing::error!(error = %e, "не удалось открыть файл");
            self.report_playback_error(crate::tr!(
                "Не удалось открыть файл",
                "Could not open the file"
            ));
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
    pub(super) fn handle_close_request(&mut self, ctx: &egui::Context) -> bool {
        if !ctx.input(|i| i.viewport().close_requested()) {
            return false;
        }

        if self.engine.is_none() {
            // Движок уже освобождён — не мешаем закрытию.
            return true;
        }

        tracing::debug!("запрос на закрытие окна, освобождаю движок");
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        self.remember_window_geometry();
        self.shutdown_engine();
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        true
    }

    /// Останавливает воспроизведение и освобождает движок.
    ///
    /// Вызывается при закрытии окна. Повторный вызов безвреден.
    pub(super) fn shutdown_engine(&mut self) {
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
    pub(super) fn accept_files_from_other_instances(&mut self, ctx: &egui::Context) {
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
    pub(super) fn process_engine_events(&mut self) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        let mut file_loaded = false;
        let mut playback_failed = false;
        let mut seek_done = false;

        for event in engine.pump_events() {
            match event {
                EngineEvent::FileLoaded => {
                    engine.refresh_video_size();
                    let active_hwdec = engine.active_hwdec();
                    let audio = engine.audio_summary();
                    let state = engine.state();
                    tracing::info!(
                        width = state.display_width,
                        height = state.display_height,
                        hwdec_active = %active_hwdec,
                        // Звук в логе: разбор жалоб вида «стало играть в моно»
                        // без него сводится к гаданию.
                        звук = %audio,
                        "файл загружен"
                    );
                    file_loaded = true;
                }
                EngineEvent::EndFile => tracing::debug!("файл закончился"),
                EngineEvent::SeekDone => seek_done = true,
                EngineEvent::PlaybackError => playback_failed = true,
                EngineEvent::Shutdown => tracing::info!("mpv завершает работу"),
            }
        }

        // Пока идёт перетаскивание, свойства mpv не трогаем: он занят
        // перемоткой, и каждый запрос ждёт её окончания — двести
        // миллисекунд на кадр по замеру.
        if !self.scrubbing {
            engine.refresh_state();
        }

        if seek_done {
            self.scrub_finished();
        }

        self.settle_seek_target();

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
            // У нового видео свои поля — прежняя обрезка к нему не подходит.
            self.forget_crop();
        }

        self.poll_subtitle_extraction();
        self.poll_extraction();
        self.poll_crop();
        self.refresh_subtitle_text();
        self.store_position_periodically();

        // Перемотка считается завершённой, когда mpv отдал новую позицию.
        if self.seek_pending {
            self.seek_pending = false;
            self.metrics.mark_seek_done();
        }
    }

    /// Запускает движок и подключает его к контексту OpenGL окна.
    pub(super) fn start_engine(
        cc: &eframe::CreationContext<'_>,
        options: &EngineOptions,
    ) -> Result<Engine, String> {
        let loader = cc.get_proc_address.clone().ok_or_else(|| {
            crate::tr!(
                "контекст OpenGL недоступен: eframe не отдал загрузчик функций",
                "OpenGL context unavailable: eframe gave no function loader"
            )
            .to_string()
        })?;

        let mut engine = Engine::new(options).map_err(engine_error_text)?;

        // Пробуждаем интерфейс, когда mpv готов показать новый кадр.
        // Внутри обратного вызова обращаться к mpv нельзя.
        let egui_ctx = cc.egui_ctx.clone();
        engine
            .init_render_context(loader, move || egui_ctx.request_repaint())
            .map_err(engine_error_text)?;

        Ok(engine)
    }
}

/// Локальный путь, которого нет на диске.
///
/// URL и потоки не проверяем: их существование определяет сам mpv.
fn is_missing_local_file(path: &str) -> bool {
    if path.contains("://") {
        return false;
    }

    !std::path::Path::new(path).exists()
}

/// Текст ошибки запуска движка на языке интерфейса.
///
/// Подробность от libmpv остаётся как есть: она техническая и на любом
/// языке выглядит одинаково — её пересылают в отчёте о неполадке.
fn engine_error_text(error: pith_mpv::MpvError) -> String {
    match error {
        pith_mpv::MpvError::Init(detail) => crate::tr!(
            format!(
                "не удалось запустить движок mpv: {detail}. \
                 Проверьте, что рядом с программой лежит libmpv-2.dll"
            ),
            format!(
                "could not start the mpv engine: {detail}. \
                 Check that libmpv-2.dll sits next to the program"
            )
        ),
        pith_mpv::MpvError::Render(detail) => crate::tr!(
            format!("не удалось создать контекст отрисовки mpv: {detail}"),
            format!("could not create the mpv render context: {detail}")
        ),
        // Остальные ошибки движка до этого окна не доходят: они случаются
        // уже в работе и показываются уведомлением.
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::is_missing_local_file;

    #[test]
    fn сетевой_адрес_не_проверяется_на_диске() {
        assert!(!is_missing_local_file("https://example.com/видео.mp4"));
        assert!(!is_missing_local_file("rtsp://камера/поток"));
    }

    #[test]
    fn несуществующий_файл_распознаётся() {
        assert!(is_missing_local_file("C:\\нет-такого-файла-12345.mkv"));
    }
}
