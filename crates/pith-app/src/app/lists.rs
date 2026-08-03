//! Именованные списки отрезков внутри видео (PLAN.md §6.5).
//!
//! Списки независимы: закладка по клавише `T` всегда попадает в активный,
//! и переключение списка не трогает воспроизведение — меняется только
//! содержимое панели и разметка полосы перемотки.

use std::path::PathBuf;

use pith_store::{ListError, VideoBookmarks};

use super::PithApp;

/// Что делает открытый диалог.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListDialogKind {
    /// Создание нового списка.
    Create,
    /// Правка существующего: имя, длительность, отступ, папка.
    Edit { original: String },
}

/// Диалог создания и настройки списка.
///
/// Поля правит интерфейс напрямую — отдельные команды на каждое нажатие
/// клавиши в поле ввода были бы лишним слоем.
#[derive(Debug, Clone)]
pub struct ListDialog {
    pub kind: ListDialogKind,
    pub name: String,
    pub duration_sec: u32,
    pub buffer_sec: u32,
    pub output_dir: Option<PathBuf>,
    /// Сообщение о неудачной попытке применить: пустое или занятое имя.
    pub error: Option<String>,
    /// Полю имени ещё нужно отдать фокус.
    ///
    /// Ровно один раз: запрос фокуса на каждом кадре не давал бы полю
    /// его терять, а вместе с этим ломал бы и обработку Enter.
    pub focus_pending: bool,
}

impl ListDialog {
    pub fn title(&self) -> &'static str {
        match self.kind {
            ListDialogKind::Create => "Новый список отрезков",
            ListDialogKind::Edit { .. } => "Настройки списка",
        }
    }
}

impl PithApp {
    /// Имена списков текущего видео.
    ///
    /// Пока у видео нет ни одной закладки, записи в хранилище тоже нет —
    /// но список «Основной» существует и без неё: он появится при первом
    /// нажатии `T`.
    pub fn list_names(&self) -> Vec<String> {
        match self.current_bookmarks() {
            Some(video) => video.names(),
            None if self.has_open_file() => vec![pith_store::DEFAULT_LIST.to_string()],
            None => Vec::new(),
        }
    }

    /// Имя активного списка.
    pub fn active_list_name(&self) -> Option<String> {
        match self.current_bookmarks() {
            Some(video) => Some(video.active_list.clone()),
            None if self.has_open_file() => Some(pith_store::DEFAULT_LIST.to_string()),
            None => None,
        }
    }

    /// Длительность фрагмента и отступ активного списка.
    ///
    /// Без записи в хранилище берутся общие значения из настроек — те же,
    /// с которыми список и будет создан.
    pub fn active_list_timing(&self) -> (u32, u32) {
        match self.current_bookmarks().and_then(VideoBookmarks::active) {
            Some(list) => (list.duration_sec, list.buffer_sec),
            None => (
                self.settings.fragments.duration_sec,
                self.settings.fragments.buffer_sec,
            ),
        }
    }

    /// Переключает активный список. Воспроизведение не трогается.
    pub fn switch_list(&mut self, name: &str) {
        let name = name.to_string();
        self.apply_to_video(|video| video.set_active(&name));
    }

    /// Копирует активный список вместе с закладками.
    pub fn duplicate_active_list(&mut self) {
        let Some(active) = self.active_list_name() else {
            return;
        };

        let mut created = None;
        self.apply_to_video(|video| {
            let name = video.duplicate_list(&active)?;
            created = Some(name);
            Ok(())
        });

        if let Some(name) = created {
            self.show_notice(&format!("Создан список «{name}»"));
        }
    }

    /// Ждёт ли подтверждения очистка списка.
    ///
    /// Возвращает имя списка и число закладок в нём — их видно в вопросе,
    /// чтобы пользователь понимал, что именно потеряет.
    pub fn clear_list_prompt(&self) -> Option<(String, usize)> {
        if !self.clear_list_pending {
            return None;
        }

        let list = self.current_bookmarks().and_then(VideoBookmarks::active)?;
        Some((list.name.clone(), list.bookmarks.len()))
    }

    /// Спрашивает подтверждение на очистку активного списка.
    pub fn ask_clear_list(&mut self) {
        self.clear_list_pending = true;
    }

    pub fn cancel_clear_list(&mut self) {
        self.clear_list_pending = false;
    }

    /// Убирает из активного списка все закладки, сам список оставляя.
    ///
    /// Действие необратимо, поэтому выполняется только по подтверждению.
    pub fn confirm_clear_list(&mut self) {
        self.clear_list_pending = false;

        let Some(list) = self
            .current_bookmarks_mut()
            .and_then(VideoBookmarks::active_mut)
        else {
            return;
        };

        let removed = list.bookmarks.len();
        list.bookmarks.clear();

        self.bookmarks.save();
        tracing::info!(закладок = removed, "список очищен");
        self.show_notice(&format!("Убрано закладок: {removed}"));
    }

    /// Удаляет активный список вместе с его закладками.
    pub fn delete_active_list(&mut self) {
        let Some(active) = self.active_list_name() else {
            return;
        };

        let mut removed = false;
        self.apply_to_video(|video| {
            video.remove_list(&active)?;
            removed = true;
            Ok(())
        });

        if removed {
            self.show_notice(&format!("Список «{active}» удалён"));
        }
    }

    /// Переносит закладку из активного списка в другой.
    pub fn move_bookmark_to_list(&mut self, time_ms: i64, target: &str) {
        let target = target.to_string();
        let mut moved = false;

        self.apply_to_video(|video| {
            video.move_bookmark(time_ms, &target)?;
            moved = true;
            Ok(())
        });

        if moved {
            self.show_notice(&format!("Перенесено в «{target}»"));
        }
    }

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
            self.show_notice("Файл не открыт");
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
            self.show_notice("Файл не открыт");
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
                    open.error = Some(e.to_string());
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
    fn apply_to_video(
        &mut self,
        action: impl FnOnce(&mut VideoBookmarks) -> Result<(), ListError>,
    ) {
        let Some(video) = self.current_bookmarks_mut() else {
            self.show_notice("Файл не открыт");
            return;
        };

        match action(video) {
            Ok(()) => self.bookmarks.save(),
            Err(e) => {
                tracing::debug!(error = %e, "операция над списком не выполнена");
                self.show_notice(&e.to_string());
            }
        }
    }
}
