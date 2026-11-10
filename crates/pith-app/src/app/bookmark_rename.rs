//! Переименование закладки.
//!
//! Название закладки становится именем вырезанного файла, поэтому
//! поправить его нужно до нарезки, а не переименовывать потом отрезки.

use super::PithApp;

/// Открытый диалог переименования.
#[derive(Debug, Clone)]
pub struct BookmarkRename {
    /// Какую закладку правим — время метки.
    pub time_ms: i64,
    pub name: String,
    /// Кто в кадре. В списке отрезков не показывается — виден только здесь.
    pub actor: String,
    /// Полю ещё нужно отдать фокус.
    pub focus_pending: bool,
    /// Какому полю отдать фокус: пустое название — ему, заполненное —
    /// актёру (курсор в конце).
    pub focus_actor: bool,
}

impl PithApp {
    pub fn bookmark_rename(&self) -> Option<&BookmarkRename> {
        self.bookmark_rename.as_ref()
    }

    pub fn bookmark_rename_mut(&mut self) -> Option<&mut BookmarkRename> {
        self.bookmark_rename.as_mut()
    }

    /// Открывает переименование закладки активного списка.
    pub fn open_bookmark_rename(&mut self, time_ms: i64) {
        let current = self
            .current_bookmarks()
            .and_then(|v| v.active())
            .and_then(|list| list.bookmarks.iter().find(|b| b.time_ms == time_ms))
            .cloned();

        let mut name = current
            .as_ref()
            .and_then(|b| b.name.clone())
            .unwrap_or_default();

        // Пустое название: если включена автовставка, кладём текст из буфера.
        if name.trim().is_empty() && self.settings.rename_autopaste {
            name = clipboard_line().unwrap_or_default();
        }

        let focus_actor = !name.trim().is_empty();

        self.bookmark_rename = Some(BookmarkRename {
            time_ms,
            name,
            actor: current
                .as_ref()
                .and_then(|b| b.actor.clone())
                .unwrap_or_default(),
            focus_pending: true,
            focus_actor,
        });
    }

    /// Включает и выключает автовставку из буфера. Запоминается.
    pub fn set_rename_autopaste(&mut self, on: bool) {
        self.settings.rename_autopaste = on;
        self.save_settings();

        // Включили при пустом названии — вставляем сразу.
        if on
            && let Some(fields) = self.bookmark_rename.as_mut()
            && fields.name.trim().is_empty()
            && let Some(line) = clipboard_line()
        {
            fields.name = line;
            fields.focus_pending = true;
            fields.focus_actor = true;
        }
    }

    /// Где стоит окно переименования.
    pub fn place_rename_window(
        &mut self,
        builder: egui::ViewportBuilder,
        default_size: [f32; 2],
    ) -> egui::ViewportBuilder {
        let saved = self.settings.rename_window;

        super::child_window::place(builder, saved, default_size, &mut self.rename_placed)
    }

    /// Запоминает, куда окно перетащили (на диск — при закрытии).
    pub fn track_rename_window(&mut self, ctx: &egui::Context) {
        if let Some(geometry) = super::child_window::geometry(ctx) {
            self.settings.rename_window = Some(geometry);
        }
    }

    pub fn close_bookmark_rename(&mut self) {
        self.bookmark_rename = None;
        // Спрятанное окно система уничтожает: следующее ставим на место заново.
        self.rename_placed = false;
        self.save_settings();
    }

    /// Применяет новое название и актёра.
    ///
    /// Пустое имя убирает название вовсе — тогда подписью снова служит
    /// время, как у закладки, поставленной без субтитров. Пустой актёр
    /// точно так же стирает прежнего.
    pub fn apply_bookmark_rename(&mut self) {
        let Some(dialog) = self.bookmark_rename.take() else {
            return;
        };

        let name = dialog.name.trim().to_string();
        let actor = dialog.actor.trim().to_string();
        let Some(video) = self.current_bookmarks_mut() else {
            return;
        };

        let Some(list) = video.active_mut() else {
            return;
        };

        let Some(bookmark) = list
            .bookmarks
            .iter_mut()
            .find(|b| b.time_ms == dialog.time_ms)
        else {
            return;
        };

        bookmark.name = if name.is_empty() { None } else { Some(name) };
        bookmark.actor = if actor.is_empty() { None } else { Some(actor) };

        self.bookmarks.save();
        self.rename_placed = false;
        self.save_settings();
        self.show_notice(crate::tr!("Закладка переименована", "Bookmark renamed"));
    }
}

/// Текст из буфера обмена одной строкой: переводы строк — пробелами,
/// ведущие тире (как у реплик из субтитров) убираются.
fn clipboard_line() -> Option<String> {
    let text = arboard::Clipboard::new().ok()?.get_text().ok()?;
    let joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let cleaned = crate::text::strip_leading_dashes(&joined)
        .trim()
        .to_string();

    (!cleaned.is_empty()).then_some(cleaned)
}
