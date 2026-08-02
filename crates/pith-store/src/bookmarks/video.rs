//! Списки отрезков одного видео и операции над ними (PLAN.md §6.5).

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{BookmarkList, DEFAULT_LIST};

/// Почему операция над списком не удалась.
///
/// Тексты готовы к показу пользователю: интерфейс просто выводит их
/// уведомлением, не пересказывая своими словами.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ListError {
    #[error("У списка должно быть имя")]
    EmptyName,

    #[error("Список с таким именем уже есть")]
    DuplicateName,

    #[error("Такого списка нет")]
    NotFound,

    #[error("Последний список удалить нельзя")]
    LastList,

    #[error("В том списке уже есть метка рядом")]
    AlreadyThere,
}

/// Все списки одного видео.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoBookmarks {
    /// Имя файла без расширения — ключ, как в v4.
    pub video_file_name: String,
    /// Какой список выбран сейчас.
    pub active_list: String,
    pub lists: Vec<BookmarkList>,
}

impl VideoBookmarks {
    pub fn new(video_file_name: impl Into<String>, duration_sec: u32, buffer_sec: u32) -> Self {
        Self {
            video_file_name: video_file_name.into(),
            active_list: DEFAULT_LIST.to_string(),
            lists: vec![BookmarkList::new(DEFAULT_LIST, duration_sec, buffer_sec)],
        }
    }

    pub fn active(&self) -> Option<&BookmarkList> {
        self.lists.iter().find(|l| l.name == self.active_list)
    }

    pub fn active_mut(&mut self) -> Option<&mut BookmarkList> {
        let name = self.active_list.clone();
        self.lists.iter_mut().find(|l| l.name == name)
    }

    pub fn names(&self) -> Vec<String> {
        self.lists.iter().map(|l| l.name.clone()).collect()
    }

    /// Список по имени.
    pub fn find(&self, name: &str) -> Option<&BookmarkList> {
        self.lists.iter().find(|l| same_name(&l.name, name))
    }

    pub fn find_mut(&mut self, name: &str) -> Option<&mut BookmarkList> {
        self.lists.iter_mut().find(|l| same_name(&l.name, name))
    }

    /// Делает список активным.
    pub fn set_active(&mut self, name: &str) -> Result<(), ListError> {
        let found = self.find(name).ok_or(ListError::NotFound)?;
        self.active_list = found.name.clone();
        Ok(())
    }

    /// Создаёт список и делает его активным.
    ///
    /// Новый список сразу становится активным: его создают, чтобы тут же
    /// в него класть закладки.
    pub fn create_list(
        &mut self,
        name: &str,
        duration_sec: u32,
        buffer_sec: u32,
    ) -> Result<(), ListError> {
        let name = self.check_free_name(name)?;

        self.lists
            .push(BookmarkList::new(name.clone(), duration_sec, buffer_sec));
        self.active_list = name;
        Ok(())
    }

    /// Переименовывает список, сохраняя выбор активного.
    pub fn rename_list(&mut self, from: &str, to: &str) -> Result<(), ListError> {
        let to = clean(to)?;

        if !self.exists(from) {
            return Err(ListError::NotFound);
        }

        // Смена регистра в собственном имени — не конфликт.
        if self.lists.iter().any(|l| same_name(&l.name, &to)) && !same_name(from, &to) {
            return Err(ListError::DuplicateName);
        }

        let was_active = same_name(&self.active_list, from);
        let list = self.find_mut(from).ok_or(ListError::NotFound)?;
        list.name = to.clone();

        if was_active {
            self.active_list = to;
        }
        Ok(())
    }

    /// Удаляет список вместе с его закладками.
    ///
    /// Последний список не удаляется: видео без единого списка не имело бы
    /// куда класть закладку по клавише `T`.
    pub fn remove_list(&mut self, name: &str) -> Result<(), ListError> {
        if !self.exists(name) {
            return Err(ListError::NotFound);
        }
        if self.lists.len() <= 1 {
            return Err(ListError::LastList);
        }

        self.lists.retain(|l| !same_name(&l.name, name));

        if same_name(&self.active_list, name) {
            self.active_list = self
                .lists
                .first()
                .map(|l| l.name.clone())
                .unwrap_or_default();
        }
        Ok(())
    }

    /// Копирует список вместе с закладками и делает копию активной.
    ///
    /// Возвращает имя копии.
    pub fn duplicate_list(&mut self, name: &str) -> Result<String, ListError> {
        let source = self.find(name).ok_or(ListError::NotFound)?;

        let mut copy = source.clone();
        copy.name = self.free_name(&format!("{} (копия)", source.name));

        let created = copy.name.clone();
        self.lists.push(copy);
        self.active_list = created.clone();
        Ok(created)
    }

    /// Переносит закладку из активного списка в другой.
    ///
    /// Из исходного списка метка убирается только после успешной вставки:
    /// потерять закладку из-за совпадения времени нельзя.
    pub fn move_bookmark(&mut self, time_ms: i64, to: &str) -> Result<(), ListError> {
        let target = self.find(to).ok_or(ListError::NotFound)?.name.clone();

        if same_name(&self.active_list, &target) {
            return Ok(());
        }

        if self
            .find(&target)
            .is_some_and(|list| list.has_near(time_ms))
        {
            return Err(ListError::AlreadyThere);
        }

        let bookmark = self
            .active_mut()
            .and_then(|list| list.take_at(time_ms))
            .ok_or(ListError::NotFound)?;

        self.find_mut(&target)
            .ok_or(ListError::NotFound)?
            .insert(bookmark);
        Ok(())
    }

    fn exists(&self, name: &str) -> bool {
        self.lists.iter().any(|l| same_name(&l.name, name))
    }

    /// Проверяет, что имя годится для нового списка.
    fn check_free_name(&self, name: &str) -> Result<String, ListError> {
        let name = clean(name)?;

        if self.exists(&name) {
            return Err(ListError::DuplicateName);
        }
        Ok(name)
    }

    /// Подбирает свободное имя, добавляя номер: «Диалоги (копия) 2».
    fn free_name(&self, base: &str) -> String {
        if !self.exists(base) {
            return base.to_string();
        }

        for index in 2..1000 {
            let candidate = format!("{base} {index}");
            if !self.exists(&candidate) {
                return candidate;
            }
        }

        format!("{base} {}", self.lists.len())
    }
}

/// Имена сравниваются без учёта регистра и краевых пробелов: «Диалоги»
/// и «диалоги » для пользователя — один и тот же список.
fn same_name(left: &str, right: &str) -> bool {
    left.trim().to_lowercase() == right.trim().to_lowercase()
}

fn clean(name: &str) -> Result<String, ListError> {
    let trimmed = name.trim();

    if trimmed.is_empty() {
        return Err(ListError::EmptyName);
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn видео() -> VideoBookmarks {
        VideoBookmarks::new("фильм", 18, 5)
    }

    #[test]
    fn новое_видео_получает_список_по_умолчанию() {
        let video = видео();

        assert_eq!(video.active_list, DEFAULT_LIST);
        assert_eq!(video.lists.len(), 1);
        assert_eq!(video.active().map(|l| l.duration_sec), Some(18));
    }

    #[test]
    fn активный_список_находится_по_имени() {
        let mut video = видео();
        video.lists.push(BookmarkList::new("Диалоги", 30, 10));
        video.active_list = "Диалоги".into();

        assert_eq!(video.active().map(|l| l.duration_sec), Some(30));
    }

    #[test]
    fn закладки_разных_списков_не_смешиваются() {
        let mut video = видео();
        video.create_list("Диалоги", 30, 10).expect("создан");

        video.set_active(DEFAULT_LIST).expect("переключён");
        video.active_mut().expect("основной список").add(1000, None);
        video.set_active("Диалоги").expect("переключён");
        video.active_mut().expect("второй список").add(2000, None);

        assert_eq!(video.lists[0].bookmarks.len(), 1);
        assert_eq!(video.lists[1].bookmarks.len(), 1);
        assert_eq!(video.lists[1].bookmarks[0].time_ms, 2000);
    }

    #[test]
    fn новый_список_становится_активным() {
        let mut video = видео();
        video.create_list("Музыка", 30, 0).expect("создан");

        assert_eq!(video.active_list, "Музыка");
        assert_eq!(video.names(), vec![DEFAULT_LIST, "Музыка"]);
    }

    #[test]
    fn имя_списка_не_повторяется() {
        let mut video = видео();

        assert_eq!(
            video.create_list(" основной ", 18, 5),
            Err(ListError::DuplicateName),
            "регистр и пробелы не делают имя новым"
        );
        assert_eq!(video.create_list("  ", 18, 5), Err(ListError::EmptyName));
        assert_eq!(video.lists.len(), 1);
    }

    #[test]
    fn переименование_сохраняет_активный_список() {
        let mut video = видео();
        video.rename_list(DEFAULT_LIST, "Реплики").expect("готово");

        assert_eq!(video.active_list, "Реплики");
        assert_eq!(video.lists[0].name, "Реплики");
    }

    #[test]
    fn переименование_в_занятое_имя_отклоняется() {
        let mut video = видео();
        video.create_list("Диалоги", 18, 5).expect("создан");

        assert_eq!(
            video.rename_list("Диалоги", DEFAULT_LIST),
            Err(ListError::DuplicateName)
        );
        assert!(video.rename_list("Диалоги", "ДИАЛОГИ").is_ok());
        assert_eq!(video.lists[1].name, "ДИАЛОГИ");
    }

    #[test]
    fn удаление_переключает_на_оставшийся_список() {
        let mut video = видео();
        video.create_list("Диалоги", 18, 5).expect("создан");

        video.remove_list("Диалоги").expect("удалён");

        assert_eq!(video.active_list, DEFAULT_LIST);
        assert_eq!(video.lists.len(), 1);
    }

    #[test]
    fn последний_список_не_удаляется() {
        let mut video = видео();

        assert_eq!(video.remove_list(DEFAULT_LIST), Err(ListError::LastList));
        assert_eq!(video.lists.len(), 1);
    }

    #[test]
    fn копия_списка_получает_свободное_имя_и_закладки() {
        let mut video = видео();
        video.active_mut().expect("список").add(1000, None);

        let first = video.duplicate_list(DEFAULT_LIST).expect("копия");
        assert_eq!(first, "Основной (копия)");
        assert_eq!(video.lists[1].bookmarks.len(), 1);

        let second = video.duplicate_list(DEFAULT_LIST).expect("вторая копия");
        assert_eq!(second, "Основной (копия) 2");
    }

    #[test]
    fn закладка_переносится_в_другой_список() {
        let mut video = видео();
        video
            .active_mut()
            .expect("список")
            .add(1000, Some("Метка".into()));
        video.create_list("Диалоги", 18, 5).expect("создан");
        video.set_active(DEFAULT_LIST).expect("переключён");

        video.move_bookmark(1000, "Диалоги").expect("перенесено");

        assert!(video.lists[0].bookmarks.is_empty());
        assert_eq!(video.lists[1].bookmarks[0].name.as_deref(), Some("Метка"));
    }

    #[test]
    fn перенос_на_занятое_время_не_теряет_закладку() {
        let mut video = видео();
        video.active_mut().expect("список").add(1000, None);
        video.create_list("Диалоги", 18, 5).expect("создан");
        video.active_mut().expect("список").add(1200, None);
        video.set_active(DEFAULT_LIST).expect("переключён");

        assert_eq!(
            video.move_bookmark(1000, "Диалоги"),
            Err(ListError::AlreadyThere)
        );
        assert_eq!(video.lists[0].bookmarks.len(), 1, "метка осталась на месте");
    }

    #[test]
    fn операции_с_несуществующим_списком_дают_ошибку() {
        let mut video = видео();

        assert_eq!(video.set_active("Нет такого"), Err(ListError::NotFound));
        assert_eq!(video.remove_list("Нет такого"), Err(ListError::NotFound));
        assert_eq!(
            video.rename_list("Нет такого", "Новое"),
            Err(ListError::NotFound)
        );
        assert_eq!(video.duplicate_list("Нет такого"), Err(ListError::NotFound));
    }
}
