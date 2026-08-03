//! Кадр приложения: обработка ввода и отрисовка слоёв поверх видео.

use super::PithApp;
use crate::ui;
use crate::video;

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
        let probe = crate::slow::FrameProbe::start();
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
        self.update_bookmarks_panel_hover(ui.ctx());

        if let Some(message) = self.fatal_error.clone() {
            ui::show_fatal_error(ui, &message);
            return;
        }

        let frame_painted = self.paint_video(ui);
        self.paint_overlays(ui.ctx());
        self.record_frame(ui.ctx(), frame_painted);

        probe.finish(&self.frame_hint());
    }
}

impl PithApp {
    /// Рисует кадр видео и вешает контекстное меню на его область.
    ///
    /// Видео идёт первым, на всю область окна: элементы управления
    /// накладываются поверх отдельным слоем — иначе mpv их закрасит.
    fn paint_video(&mut self, ui: &mut egui::Ui) -> bool {
        let render_context = self.engine.as_ref().and_then(|e| e.shared_render_context());
        let mut frame_painted = false;

        let video_area = egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ui, |ui| {
                let rect = ui.available_rect_before_wrap();

                if let Some(context) = render_context
                    && video::paint(ui, rect, context)
                {
                    frame_painted = true;
                }

                // Область для правого щелчка: сюда вешается контекстное меню.
                ui.interact(
                    rect,
                    egui::Id::new("video_area"),
                    egui::Sense::click_and_drag(),
                )
            })
            .inner;

        // Меню показывается штатным механизмом egui: только внутри него
        // подменю раскрываются по наведению и размещаются сбоку.
        video_area.context_menu(|ui| ui::show_menu_items(self, ui));

        frame_painted
    }

    /// Слои интерфейса поверх видео.
    fn paint_overlays(&mut self, ctx: &egui::Context) {
        ui::show_subtitles(self, ctx);
        ui::show_controls(self, ctx);
        crate::slow::probe("отрисовка панели отрезков", || {
            ui::show_bookmarks_panel(self, ctx)
        });
        ui::show_list_dialog(self, ctx);
        ui::show_fragment_settings(self, ctx);
        ui::show_file_types_prompt(self, ctx);
        ui::show_extraction_notice(self, ctx);
        ui::show_search(self, ctx);
        ui::show_notice(self, ctx);
        ui::show_migration_report(self, ctx);
        ui::show_resume_offer(self, ctx);
    }

    /// Что было на экране в этом кадре — подсказка для разбора заминок.
    fn frame_hint(&self) -> String {
        let mut parts = Vec::new();

        if self.bookmarks_panel_open() {
            parts.push("панель отрезков");
        }
        if self.list_dialog.is_some() {
            parts.push("диалог списка");
        }
        if self.fragment_settings.is_some() {
            parts.push("настройки нарезки");
        }
        if self.extraction_progress().is_some() {
            parts.push("идёт нарезка");
        }

        if parts.is_empty() {
            "только видео".to_string()
        } else {
            parts.join(", ")
        }
    }

    /// Учёт кадра в замерах и запрос следующей отрисовки.
    fn record_frame(&mut self, ctx: &egui::Context, frame_painted: bool) {
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
            ctx.request_repaint();
        }
    }
}
