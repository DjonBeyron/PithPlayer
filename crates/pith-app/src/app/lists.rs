//! Именованные списки отрезков внутри видео (PLAN.md §6.5).
//!
//! Списки независимы: закладка по клавише `T` всегда попадает в активный,
//! и переключение списка не трогает воспроизведение — меняется только
//! содержимое панели и разметка полосы перемотки.

use std::path::PathBuf;

use pith_store::VideoBookmarks;

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
            ListDialogKind::Create => crate::tr!("Новый список отрезков", "New fragment list"),
            ListDialogKind::Edit { .. } => crate::tr!("Настройки списка", "List settings"),
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

    /// Пределы длительности и отступа, секунды.
    ///
    /// Те же, что и в диалоге настройки списка: правка в панели не должна
    /// пускать значения, которых диалог не примет.
    pub const MAX_DURATION_SEC: u32 = 600;
    pub const MAX_BUFFER_SEC: u32 = 120;

    /// Меняет длительность и отступ активного списка.
    ///
    /// Правится прямо в панели: эти два числа подбираются по ходу работы,
    /// и ради каждой правки открывать диалог настроек утомительно.
    pub fn set_active_list_timing(&mut self, duration_sec: u32, buffer_sec: u32) {
        let (duration_sec, buffer_sec) = (
            duration_sec.clamp(1, Self::MAX_DURATION_SEC),
            buffer_sec.min(Self::MAX_BUFFER_SEC),
        );

        let changed = self.update_active_list(|list| {
            if list.duration_sec == duration_sec && list.buffer_sec == buffer_sec {
                return false;
            }

            list.duration_sec = duration_sec;
            list.buffer_sec = buffer_sec;
            true
        });

        if changed {
            tracing::debug!(duration_sec, buffer_sec, "параметры списка изменены");
        }
    }

    /// Задаёт папку вывода активного списка.
    ///
    /// `None` возвращает список к общей папке из настроек.
    pub fn set_active_list_output_dir(&mut self, dir: Option<std::path::PathBuf>) {
        let changed = self.update_active_list(|list| {
            if list.output_dir == dir {
                return false;
            }

            list.output_dir = dir.clone();
            true
        });

        if changed {
            tracing::debug!("папка вывода списка изменена");
        }
    }

    /// Спрашивает папку вывода и запоминает её за активным списком.
    pub fn choose_active_list_output_dir(&mut self) {
        let current = self.fragments_output_dir();

        let mut dialog = rfd::FileDialog::new();
        if let Some(dir) = current {
            dialog = dialog.set_directory(dir);
        }

        if let Some(dir) = dialog.pick_folder() {
            self.set_active_list_output_dir(Some(dir));
        }
    }

    /// Правит активный список и сохраняет закладки, если что-то изменилось.
    fn update_active_list(
        &mut self,
        edit: impl FnOnce(&mut pith_store::BookmarkList) -> bool,
    ) -> bool {
        let Some(video) = self.current_bookmarks_mut() else {
            return false;
        };

        let Some(list) = video.active_mut() else {
            return false;
        };

        if !edit(list) {
            return false;
        }

        self.bookmarks.save();
        true
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
            self.show_notice(&crate::tr!(
                format!("Создан список «{name}»"),
                format!("List «{name}» created")
            ));
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
        self.show_notice(&crate::tr!(
            format!("Убрано закладок: {removed}"),
            format!("Bookmarks removed: {removed}")
        ));
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
            let shown = crate::i18n::list_name(&active);
            self.show_notice(&crate::tr!(
                format!("Список «{shown}» удалён"),
                format!("List «{shown}» deleted")
            ));
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
            let shown = crate::i18n::list_name(&target);
            self.show_notice(&crate::tr!(
                format!("Перенесено в «{shown}»"),
                format!("Moved to «{shown}»")
            ));
        }
    }
}
