//! Закладки и именованные списки отрезков (PLAN.md §6.5).
//!
//! Закладка — точка во времени. Отрезок для нарезки считается от неё:
//! `[метка − отступ, метка − отступ + длительность]`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::file::{read_json, write_json};
use crate::paths::DataPaths;

/// Версия формата. Первая — плоские списки v4, вторая — именованные.
const FORMAT_VERSION: u32 = 2;

/// Имя списка, создаваемого по умолчанию.
pub const DEFAULT_LIST: &str = "Основной";

/// Закладка: точка во времени.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeBookmark {
    pub time_ms: i64,
    /// Название, заданное пользователем.
    pub name: Option<String>,
}

impl TimeBookmark {
    pub fn seconds(&self) -> f64 {
        self.time_ms as f64 / 1000.0
    }

    /// Подпись: название либо время.
    pub fn label(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| format_time(self.time_ms))
    }
}

/// Именованный список отрезков внутри одного видео.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BookmarkList {
    pub name: String,
    /// Длительность вырезаемого фрагмента, секунды.
    pub duration_sec: u32,
    /// Отступ назад от метки, секунды.
    pub buffer_sec: u32,
    /// Своя папка вывода. `None` — общая из настроек.
    pub output_dir: Option<PathBuf>,
    pub bookmarks: Vec<TimeBookmark>,
}

impl BookmarkList {
    pub fn new(name: impl Into<String>, duration_sec: u32, buffer_sec: u32) -> Self {
        Self {
            name: name.into(),
            duration_sec,
            buffer_sec,
            output_dir: None,
            bookmarks: Vec::new(),
        }
    }

    /// Добавляет закладку, сохраняя порядок по времени.
    ///
    /// Возвращает `false`, если рядом уже есть метка: в v4 дубликаты
    /// в пределах секунды отсекались, и это правило сохраняется.
    pub fn add(&mut self, time_ms: i64, name: Option<String>) -> bool {
        if self.has_near(time_ms) {
            return false;
        }

        self.bookmarks.push(TimeBookmark { time_ms, name });
        self.bookmarks.sort_by_key(|b| b.time_ms);
        true
    }

    /// Есть ли закладка в пределах секунды от указанного времени.
    pub fn has_near(&self, time_ms: i64) -> bool {
        self.bookmarks
            .iter()
            .any(|b| (b.time_ms - time_ms).abs() < 1000)
    }

    pub fn remove_at(&mut self, time_ms: i64) -> bool {
        let before = self.bookmarks.len();
        self.bookmarks.retain(|b| b.time_ms != time_ms);
        self.bookmarks.len() != before
    }
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
}

#[derive(Debug, Serialize, Deserialize)]
struct BookmarksFile {
    version: u32,
    videos: Vec<VideoBookmarks>,
}

impl Default for BookmarksFile {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            videos: Vec::new(),
        }
    }
}

/// Хранилище закладок.
pub struct Bookmarks {
    paths: DataPaths,
    data: BookmarksFile,
}

impl Bookmarks {
    pub fn load(paths: DataPaths) -> Self {
        let data: BookmarksFile = read_json(&paths.bookmarks())
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "не удалось прочитать закладки");
                None
            })
            .unwrap_or_default();

        tracing::debug!(видео = data.videos.len(), "закладки загружены");
        Self { paths, data }
    }

    /// Закладки видео, если они есть.
    pub fn for_video(&self, video_file_name: &str) -> Option<&VideoBookmarks> {
        self.data
            .videos
            .iter()
            .find(|v| v.video_file_name == video_file_name)
    }

    /// Закладки видео, создавая запись при необходимости.
    pub fn for_video_mut(
        &mut self,
        video_file_name: &str,
        duration_sec: u32,
        buffer_sec: u32,
    ) -> &mut VideoBookmarks {
        if let Some(index) = self
            .data
            .videos
            .iter()
            .position(|v| v.video_file_name == video_file_name)
        {
            return &mut self.data.videos[index];
        }

        self.data.videos.push(VideoBookmarks::new(
            video_file_name,
            duration_sec,
            buffer_sec,
        ));
        self.data
            .videos
            .last_mut()
            .expect("запись только что добавлена")
    }

    /// Добавляет запись напрямую. Нужно миграции из v4.
    pub fn insert_raw(&mut self, video: VideoBookmarks) {
        self.data.videos.push(video);
    }

    pub fn videos_count(&self) -> usize {
        self.data.videos.len()
    }

    pub fn save(&self) {
        if let Err(e) = write_json(&self.paths.bookmarks(), &self.data) {
            tracing::error!(error = %e, "не удалось сохранить закладки");
        }
    }
}

/// Время в формате «ЧЧ:ММ:СС».
fn format_time(time_ms: i64) -> String {
    let total = (time_ms / 1000).max(0);
    format!(
        "{:02}:{:02}:{:02}",
        total / 3600,
        (total % 3600) / 60,
        total % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn закладки_хранятся_по_возрастанию_времени() {
        let mut list = BookmarkList::new("Основной", 18, 5);

        list.add(30_000, None);
        list.add(10_000, None);
        list.add(20_000, None);

        let времена: Vec<_> = list.bookmarks.iter().map(|b| b.time_ms).collect();
        assert_eq!(времена, vec![10_000, 20_000, 30_000]);
    }

    #[test]
    fn близкие_закладки_не_дублируются() {
        let mut list = BookmarkList::new("Основной", 18, 5);

        assert!(list.add(10_000, None));
        assert!(
            !list.add(10_500, None),
            "метка в пределах секунды — это та же метка"
        );
        assert_eq!(list.bookmarks.len(), 1);
    }

    #[test]
    fn закладка_дальше_секунды_добавляется() {
        let mut list = BookmarkList::new("Основной", 18, 5);

        list.add(10_000, None);
        assert!(list.add(11_500, None));
        assert_eq!(list.bookmarks.len(), 2);
    }

    #[test]
    fn закладка_удаляется_по_времени() {
        let mut list = BookmarkList::new("Основной", 18, 5);
        list.add(10_000, None);

        assert!(list.remove_at(10_000));
        assert!(
            !list.remove_at(10_000),
            "повторное удаление ничего не меняет"
        );
        assert!(list.bookmarks.is_empty());
    }

    #[test]
    fn подпись_берётся_из_названия_или_времени() {
        let named = TimeBookmark {
            time_ms: 372_398,
            name: Some("Реплика".into()),
        };
        let plain = TimeBookmark {
            time_ms: 372_398,
            name: None,
        };

        assert_eq!(named.label(), "Реплика");
        assert_eq!(plain.label(), "00:06:12");
    }

    #[test]
    fn новое_видео_получает_список_по_умолчанию() {
        let video = VideoBookmarks::new("фильм", 18, 5);

        assert_eq!(video.active_list, DEFAULT_LIST);
        assert_eq!(video.lists.len(), 1);
        assert_eq!(video.active().map(|l| l.duration_sec), Some(18));
    }

    #[test]
    fn активный_список_находится_по_имени() {
        let mut video = VideoBookmarks::new("фильм", 18, 5);
        video.lists.push(BookmarkList::new("Диалоги", 30, 10));
        video.active_list = "Диалоги".into();

        assert_eq!(video.active().map(|l| l.duration_sec), Some(30));
    }

    #[test]
    fn закладки_разных_списков_не_смешиваются() {
        let mut video = VideoBookmarks::new("фильм", 18, 5);
        video.lists.push(BookmarkList::new("Диалоги", 30, 10));

        video.active_mut().expect("основной список").add(1000, None);
        video.active_list = "Диалоги".into();
        video.active_mut().expect("второй список").add(2000, None);

        assert_eq!(video.lists[0].bookmarks.len(), 1);
        assert_eq!(video.lists[1].bookmarks.len(), 1);
        assert_eq!(video.lists[1].bookmarks[0].time_ms, 2000);
    }

    #[test]
    fn запись_и_чтение_сохраняют_списки() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        let mut store = Bookmarks::load(paths.clone());
        store
            .for_video_mut("фильм", 18, 5)
            .active_mut()
            .expect("список")
            .add(5000, Some("Метка".into()));
        store.save();

        let loaded = Bookmarks::load(paths);
        let video = loaded.for_video("фильм").expect("видео найдено");
        assert_eq!(video.lists[0].bookmarks[0].name.as_deref(), Some("Метка"));
    }
}
