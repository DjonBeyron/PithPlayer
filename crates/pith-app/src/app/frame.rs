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
        crate::slow::probe("разбор событий mpv", || {
            self.process_engine_events()
        });
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
        self.remember_window_geometry();
        self.release_instance_port();
        self.shutdown_engine();
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let probe = crate::slow::FrameProbe::start();
        self.frame_time = ui.ctx().input(|i| i.time);

        // Меню запоминаем до отрисовки: нажатие мимо него закрывает меню
        // ещё до того, как мы спросим о нём, и к моменту проверки открытым
        // оно уже не считается.
        self.menu_was_open = egui::Popup::is_any_open(ui.ctx());

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

        self.poll_preview(ui.ctx());
        self.expire_resume_offer();
        self.ensure_window_on_screen(ui.ctx());
        self.announce_maximized(frame);
        self.track_window_geometry(ui.ctx());
        self.track_manual_resize(ui.ctx());
        self.fit_window_to_video(ui.ctx());
        self.track_pointer_activity(ui.ctx());
        self.track_window_focus(ui.ctx());
        self.store_volume(ui.ctx());
        self.store_subtitle_style(ui.ctx());

        if let Some(message) = self.fatal_error.clone() {
            ui::show_fatal_error(ui, &message);
            return;
        }

        let frame_painted =
            crate::slow::probe("отрисовка видео", || self.paint_video(ui));
        crate::slow::probe("отрисовка интерфейса", || {
            self.paint_overlays(ui.ctx())
        });
        self.record_frame(ui.ctx(), frame_painted);

        self.refresh_window_title(ui.ctx());
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

        // Нажатие по кадру ставит и снимает паузу. Кроме трёх случаев: им
        // подняли окно из другого приложения, им закрывают открытую панель
        // отрезков или открытое меню. Всё это — не просьба о паузе, а способ
        // вернуться к плееру; она достанется следующему нажатию.
        let dismissing = self.bookmarks_panel_dismissible();
        let busy = self.just_regained_focus() || dismissing || self.menu_was_open;

        if video_area.clicked() && !busy {
            self.toggle_pause();
        }

        // Двойное нажатие разворачивает плеер и возвращает обратно.
        //
        // Пауза при этом остаётся как была: egui считает второе нажатие
        // и обычным тоже, поэтому она успевает переключиться дважды —
        // туда и сразу обратно.
        if video_area.double_clicked() && !dismissing {
            self.toggle_fullscreen(ui.ctx());
        }

        // Меню показывается штатным механизмом egui: только внутри него
        // подменю раскрываются по наведению и размещаются сбоку.
        video_area.context_menu(|ui| ui::show_menu_items(self, ui));

        frame_painted
    }

    /// Слои интерфейса поверх видео.
    fn paint_overlays(&mut self, ctx: &egui::Context) {
        ui::show_subtitles(self, ctx);
        ui::show_playback_badge(self, ctx);
        ui::show_seek_hud(self, ctx);

        // Панель отрезков идёт во всю высоту окна, и её нижний край
        // приходится на панель управления. Рисуем её раньше: полоса
        // перемотки и кнопки должны остаться сверху и в рабочем виде.
        ui::show_panel_handle(self, ctx);
        crate::slow::probe("отрисовка панели отрезков", || {
            ui::show_bookmarks_panel(self, ctx)
        });

        ui::show_controls(self, ctx);
        ui::show_list_dialog(self, ctx);
        ui::show_bookmark_rename(self, ctx);
        ui::show_clear_list_prompt(self, ctx);
        ui::show_fragment_settings(self, ctx);
        ui::show_file_types_prompt(self, ctx);
        ui::show_extraction_notice(self, ctx);
        ui::show_export(self, ctx);
        ui::show_search(self, ctx);
        ui::show_subtitle_style(self, ctx);
        ui::show_history(self, ctx);
        ui::show_notice(self, ctx);
        ui::show_migration_report(self, ctx);
        ui::show_resume_offer(self, ctx);

        // Окно актёров — своё окно, а не слой поверх кадра: его двигают
        // куда угодно, хоть на второй экран.
        ui::show_actors_window(self, ctx);
        ui::show_integrations(self, ctx);
    }

    /// Держит в заголовке окна название текущего видео.
    ///
    /// Заголовок — единственное место, где имя файла видно целиком: поверх
    /// видео его не покажешь, не закрыв кадр. Поэтому там больше ничего и
    /// нет — ни версии, ни названия программы: имена файлов длинные, и всё
    /// прочее отъедает у них место, а в панели задач и так видно, чей это
    /// значок. Без файла остаётся одно название программы.
    fn refresh_window_title(&mut self, ctx: &egui::Context) {
        let title = match self.current_video_name() {
            Some(name) => name,
            None => "Pith Player".to_string(),
        };

        if self.window_title.as_deref() == Some(title.as_str()) {
            return;
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
        self.window_title = Some(title);
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
