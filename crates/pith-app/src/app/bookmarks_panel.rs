//! Панель отрезков у правого края: показ, закрепление, прогрев разметки.

use pith_store::MIN_PANEL_WIDTH as MIN_WIDTH;

use super::PithApp;

/// Сколько кадра окна панель никогда не занимает.
///
/// Полоса видео у левого края нужна не для красоты: панель закрывают
/// нажатием мимо неё, и во весь экран её было бы нечем убрать — меню
/// осталось бы под ней. Полоса и есть это «мимо».
const KEEP_FREE: f32 = 96.0;

/// До какой ширины панель можно растянуть в таком окне.
fn max_width(window_width: f32) -> f32 {
    (window_width - KEEP_FREE).max(MIN_WIDTH)
}

/// Сколько кадров панель рисуется невидимой перед показом.
///
/// Двух достаточно: на первом egui считает размеры списка, на втором —
/// положение полосы прокрутки, которое от этих размеров зависит.
const PANEL_WARMUP_FRAMES: u8 = 2;

impl PithApp {
    /// Видна ли панель отрезков.
    pub fn bookmarks_panel_open(&self) -> bool {
        self.bookmarks_panel || self.bookmarks_panel_pinned
    }

    pub fn bookmarks_panel_pinned(&self) -> bool {
        self.bookmarks_panel_pinned
    }

    /// Закрепляет панель, чтобы она не пряталась при уходе курсора.
    pub fn toggle_bookmarks_panel(&mut self) {
        self.bookmarks_panel_pinned = !self.bookmarks_panel_pinned;
        self.bookmarks_panel = false;

        if self.bookmarks_panel_pinned {
            self.bookmarks_panel_warmup = PANEL_WARMUP_FRAMES;
        }
    }

    /// Закрепляет панель кнопкой в ней самой.
    ///
    /// От `toggle_bookmarks_panel` отличается тем, что панель в обоих
    /// случаях остаётся на экране: нажали булавку — она перестаёт
    /// закрываться нажатием мимо, нажали ещё раз — снова закрывается,
    /// но не исчезает из-под пальца в тот же миг.
    pub fn toggle_panel_pin(&mut self) {
        self.bookmarks_panel_pinned = !self.bookmarks_panel_pinned;
        self.bookmarks_panel = !self.bookmarks_panel_pinned;
    }

    /// Насколько панель видна сейчас.
    ///
    /// Ноль на кадрах прогрева: панель рисуется целиком, но не показывается,
    /// пока egui не посчитает размеры списка и полосы прокрутки.
    pub fn bookmarks_panel_opacity(&self) -> f32 {
        if self.bookmarks_panel_warmup > 0 {
            0.0
        } else {
            1.0
        }
    }

    /// Отмечает, что кадр прогрева пройден.
    ///
    /// Вызывается панелью после отрисовки: считать нужно только те кадры,
    /// в которых разметка действительно считалась.
    pub fn finish_panel_warmup_frame(&mut self) {
        self.bookmarks_panel_warmup = self.bookmarks_panel_warmup.saturating_sub(1);
    }

    /// Ширина панели при таком окне.
    ///
    /// Сохранённая ширина приводится к нынешнему окну: её могли задать
    /// на широком экране, а плеер открыть на узком.
    pub fn bookmarks_panel_width(&self, window_width: f32) -> f32 {
        self.settings
            .bookmarks_panel_width
            .clamp(MIN_WIDTH, max_width(window_width))
    }

    /// Запоминает ширину, до которой панель растянули.
    ///
    /// На диск не пишем: файл настроек не должен переписываться на каждое
    /// движение мышью. Запись случится, когда край отпустят.
    pub fn set_bookmarks_panel_width(&mut self, width: f32, window_width: f32) {
        self.settings.bookmarks_panel_width = width.clamp(MIN_WIDTH, max_width(window_width));
    }

    /// Записывает ширину — когда край отпустили.
    pub fn store_bookmarks_panel_width(&mut self) {
        self.save_settings();
        tracing::debug!(
            ширина = self.settings.bookmarks_panel_width,
            "ширина панели отрезков запомнена"
        );
    }

    /// Открыт ли диалог, вызванный из панели отрезков.
    ///
    /// Все они рисуются поверх неё, и на время их жизни панель остаётся
    /// на месте, даже если курсор ушёл из окна.
    fn panel_dialog_open(&self) -> bool {
        self.list_dialog.is_some() || self.bookmark_rename.is_some() || self.clear_list_pending
    }

    /// Открывает панель — по нажатию на язычок у правого края.
    ///
    /// Раньше она выезжала от одного наведения и тем самым появлялась
    /// незваной: курсор шёл к кнопкам полного экрана или просто пересекал
    /// край экрана. Теперь её вызывают нажатием.
    pub fn open_bookmarks_panel(&mut self) {
        if self.bookmarks_panel {
            return;
        }

        // Панель только что вызвана — дадим ей кадр на разметку.
        self.bookmarks_panel_warmup = PANEL_WARMUP_FRAMES;
        self.bookmarks_panel = true;
        tracing::debug!("панель отрезков открыта");
    }

    /// Закроется ли панель ближайшим нажатием мимо неё.
    ///
    /// По этому признаку кадр пропускает такое нажатие: щелчок, которым
    /// панель убирают, не должен заодно ставить паузу. Пауза — за
    /// следующим щелчком.
    pub fn bookmarks_panel_dismissible(&self) -> bool {
        self.bookmarks_panel && !self.panel_dialog_open()
    }

    /// Откреплена ли панель в своё окно.
    pub fn bookmarks_panel_detached(&self) -> bool {
        self.settings.bookmarks_panel_detached
    }

    /// Откручивает панель в отдельное окно и обратно.
    ///
    /// Откреплённая панель — окно системы: её уносят на второй экран
    /// и оставляют рядом с кадром, а не поверх него. Место и размер такого
    /// окна помнятся между запусками, как у окон актёров и выгрузки.
    pub fn toggle_bookmarks_panel_detached(&mut self) {
        let detached = !self.settings.bookmarks_panel_detached;

        self.settings.bookmarks_panel_detached = detached;
        self.bookmarks_window_placed = false;

        // Прикреплённая обратно панель должна остаться на виду: её только
        // что видели окном, и исчезнуть она не должна.
        if !detached {
            self.bookmarks_panel = true;
            self.bookmarks_panel_warmup = PANEL_WARMUP_FRAMES;
        }

        self.save_settings();
        tracing::debug!(откреплена = detached, "панель отрезков");
    }

    /// Ставит окно панели туда, где оно стояло в прошлый раз.
    pub fn place_bookmarks_window(
        &mut self,
        builder: egui::ViewportBuilder,
        default_size: [f32; 2],
    ) -> egui::ViewportBuilder {
        let saved = self.settings.bookmarks_window;

        super::child_window::place(
            builder,
            saved,
            default_size,
            &mut self.bookmarks_window_placed,
        )
    }

    /// Запоминает, куда окно панели перетащили.
    pub fn track_bookmarks_window(&mut self, ctx: &egui::Context) {
        if let Some(geometry) = super::child_window::geometry(ctx) {
            self.settings.bookmarks_window = Some(geometry);
        }
    }

    /// Убирает панель язычком на её краю.
    ///
    /// От «нажали мимо» отличается тем, что снимает и закрепление: иначе
    /// закреплённую панель этим язычком было бы не убрать, а на всю ширину
    /// окна нажимать мимо неё уже некуда — ради этого он и заведён.
    pub fn hide_bookmarks_panel(&mut self) {
        self.bookmarks_panel = false;
        self.bookmarks_panel_pinned = false;

        tracing::debug!("панель отрезков убрана язычком");
    }

    /// Закрывает панель — по нажатию мимо неё.
    pub fn close_bookmarks_panel(&mut self) {
        if !self.bookmarks_panel {
            return;
        }

        // Пока открыт вызванный из панели диалог, она остаётся на месте:
        // нажатие в диалоге — это работа с панелью, а не мимо неё.
        if self.panel_dialog_open() {
            return;
        }

        self.bookmarks_panel = false;
        tracing::debug!("панель отрезков убрана: нажали мимо неё");
    }

    /// Следит за фокусом окна.
    ///
    /// Отсюда два следствия. Панель отрезков закрывается, когда работают
    /// уже не с плеером: нажатие в другом окне до него не доходит, и
    /// «нажали мимо» такого не ловит. И запоминается миг возвращения
    /// фокуса — нажатием, которым окно подняли, паузу переключать нельзя.
    pub(super) fn track_window_focus(&mut self, ctx: &egui::Context) {
        let regained = ctx.input(|i| {
            i.events
                .iter()
                .any(|event| matches!(event, egui::Event::WindowFocused(true)))
        });

        if regained {
            self.focus_regained_at = Some(self.frame_time);
        }

        if !self.bookmarks_panel || ctx.input(|i| i.focused) {
            return;
        }

        // Диалоги панели иногда уводят фокус сами (выбор папки системным
        // окном) — из-за такого ухода панель закрывать нельзя.
        if self.panel_dialog_open() {
            return;
        }

        self.bookmarks_panel = false;
        tracing::debug!("панель отрезков убрана: плеер потерял фокус");
    }
}
