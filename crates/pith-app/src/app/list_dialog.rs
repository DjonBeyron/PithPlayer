//! Диалог создания и настройки списка отрезков.
//!
//! Отдельно от самих списков (`app::lists`): там правила, по которым
//! списки живут, а здесь — поля открытого окна и их проверка.

use pith_store::{ListError, VideoBookmarks};

use super::PithApp;
use super::lists::{ListDialog, ListDialogKind};

impl PithApp {
    pub fn list_dialog(&self) -> Option<&ListDialog> {
        self.list_dialog.as_ref()
    }

    /// Поля диалога для правки интерфейсом.
    pub fn list_dialog_mut(&mut self) -> Option<&mut ListDialog> {
        self.list_dialog.as_mut()
    }

    pub fn close_list_dialog(&mut self) {
        self.list_dialog = None;
    }

    /// Открывает диалог создания списка.
    ///
    /// Длительность и отступ берутся из активного списка: новый список чаще
    /// всего ведут с теми же настройками, что и текущий.
    pub fn open_new_list_dialog(&mut self) {
        if !self.has_open_file() {
            self.show_notice(crate::tr!("Файл не открыт", "No file open"));
            return;
        }

        let (duration_sec, buffer_sec) = self.active_list_timing();

        self.list_dialog = Some(ListDialog {
            kind: ListDialogKind::Create,
            name: String::new(),
            duration_sec,
            buffer_sec,
            output_dir: None,
            error: None,
            focus_pending: true,
        });
    }

    /// Открывает диалог настроек активного списка.
    ///
    /// Список материализуется в хранилище: настраивать можно и то, куда
    /// ещё не положили ни одной закладки.
    pub fn open_list_settings_dialog(&mut self) {
        let Some(list) = self
            .current_bookmarks_mut()
            .and_then(VideoBookmarks::active_mut)
        else {
            self.show_notice(crate::tr!("Файл не открыт", "No file open"));
            return;
        };

        self.list_dialog = Some(ListDialog {
            kind: ListDialogKind::Edit {
                original: list.name.clone(),
            },
            name: list.name.clone(),
            duration_sec: list.duration_sec,
            buffer_sec: list.buffer_sec,
            output_dir: list.output_dir.clone(),
            error: None,
            focus_pending: true,
        });
    }

    /// Применяет диалог: создаёт список либо правит существующий.
    ///
    /// Диалог остаётся открытым, если имя не подошло, — иначе введённое
    /// пропало бы вместе с окном.
    pub fn apply_list_dialog(&mut self) {
        let Some(dialog) = self.list_dialog.clone() else {
            return;
        };

        let result = match &dialog.kind {
            ListDialogKind::Create => self.create_list_from(&dialog),
            ListDialogKind::Edit { original } => self.update_list_from(&dialog, original),
        };

        match result {
            Ok(()) => {
                self.bookmarks.save();
                self.list_dialog = None;
            }
            Err(e) => {
                if let Some(open) = self.list_dialog.as_mut() {
                    open.error = Some(list_error_text(e).to_string());
                }
            }
        }
    }

    fn create_list_from(&mut self, dialog: &ListDialog) -> Result<(), ListError> {
        let video = self.current_bookmarks_mut().ok_or(ListError::NotFound)?;

        video.create_list(&dialog.name, dialog.duration_sec, dialog.buffer_sec)?;

        let created = video.active_list.clone();
        if let Some(list) = video.find_mut(&created) {
            list.output_dir = dialog.output_dir.clone();
        }

        tracing::info!(список = %created, "создан список отрезков");
        Ok(())
    }

    fn update_list_from(&mut self, dialog: &ListDialog, original: &str) -> Result<(), ListError> {
        let video = self.current_bookmarks_mut().ok_or(ListError::NotFound)?;

        video.rename_list(original, &dialog.name)?;

        let list = video.find_mut(&dialog.name).ok_or(ListError::NotFound)?;
        list.duration_sec = dialog.duration_sec;
        list.buffer_sec = dialog.buffer_sec;
        list.output_dir = dialog.output_dir.clone();

        tracing::info!(список = %dialog.name, "настройки списка изменены");
        Ok(())
    }

    /// Выполняет операцию над списками текущего видео и сохраняет результат.
    ///
    /// Ошибка показывается пользователю готовым текстом из `ListError`.
    pub(super) fn apply_to_video(
        &mut self,
        action: impl FnOnce(&mut VideoBookmarks) -> Result<(), ListError>,
    ) {
        let Some(video) = self.current_bookmarks_mut() else {
            self.show_notice(crate::tr!("Файл не открыт", "No file open"));
            return;
        };

        match action(video) {
            Ok(()) => self.bookmarks.save(),
            Err(e) => {
                tracing::debug!(error = %e, "операция над списком не выполнена");
                self.show_notice(list_error_text(e));
            }
        }
    }
}

/// Текст ошибки работы со списком.
///
/// Слова живут здесь: хранилище о языке интерфейса не знает.
fn list_error_text(error: ListError) -> &'static str {
    match error {
        ListError::EmptyName => crate::tr!("У списка должно быть имя", "The list needs a name"),
        ListError::DuplicateName => crate::tr!(
            "Список с таким именем уже есть",
            "A list with this name already exists"
        ),
        ListError::NotFound => crate::tr!("Такого списка нет", "No such list"),
        ListError::LastList => crate::tr!(
            "Последний список удалить нельзя",
            "The last list cannot be deleted"
        ),
        ListError::AlreadyThere => crate::tr!(
            "В том списке уже есть метка рядом",
            "That list already has a mark nearby"
        ),
    }
}
