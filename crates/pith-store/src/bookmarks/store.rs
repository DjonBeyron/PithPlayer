//! Файл закладок: чтение, запись, доступ по видео.

use serde::{Deserialize, Serialize};

use crate::file::{read_json, write_json};
use crate::paths::DataPaths;

use super::VideoBookmarks;

/// Версия формата. Первая — плоские списки v4, вторая — именованные.
const FORMAT_VERSION: u32 = 2;

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
            // разрешено: запись добавлена строкой выше, пустым список не бывает
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bookmarks::DEFAULT_LIST;

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

    #[test]
    fn несколько_списков_переживают_перезапуск() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        let mut store = Bookmarks::load(paths.clone());
        let video = store.for_video_mut("фильм", 18, 5);
        video.create_list("Диалоги", 30, 10).expect("создан");
        video.active_mut().expect("список").add(7000, None);
        store.save();

        let loaded = Bookmarks::load(paths);
        let video = loaded.for_video("фильм").expect("видео найдено");

        assert_eq!(video.names(), vec![DEFAULT_LIST, "Диалоги"]);
        assert_eq!(video.active_list, "Диалоги");
        assert_eq!(video.active().map(|l| l.bookmarks.len()), Some(1));
        assert!(
            video
                .find(DEFAULT_LIST)
                .is_some_and(|l| l.bookmarks.is_empty()),
            "закладка легла только в активный список"
        );
    }
}
