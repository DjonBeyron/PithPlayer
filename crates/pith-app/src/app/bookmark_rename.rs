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

        self.bookmark_rename = Some(BookmarkRename {
            time_ms,
            name: current
                .as_ref()
                .and_then(|b| b.name.clone())
                .unwrap_or_default(),
            actor: current
                .as_ref()
                .and_then(|b| b.actor.clone())
                .unwrap_or_default(),
            focus_pending: true,
        });
    }

    pub fn close_bookmark_rename(&mut self) {
        self.bookmark_rename = None;
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
        self.show_notice(crate::tr!("Закладка переименована", "Bookmark renamed"));
    }
}
