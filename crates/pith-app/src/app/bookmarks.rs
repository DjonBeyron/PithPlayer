//! Закладки: добавление, удаление, отрезки для полосы перемотки.

use std::path::Path;

use pith_store::VideoBookmarks;

use super::PithApp;
use crate::ui::FragmentRange;

impl PithApp {
    /// Ключ текущего видео в хранилище закладок — имя файла без расширения.
    fn video_key(&self) -> Option<String> {
        self.current_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_string())
    }

    /// Закладки текущего видео.
    pub fn current_bookmarks(&self) -> Option<&VideoBookmarks> {
        let key = self.video_key()?;
        self.bookmarks.for_video(&key)
    }

    /// Ставит закладку на текущей позиции.
    ///
    /// Название по умолчанию — реплика субтитров: в v4 закладки чаще всего
    /// подписывались именно фразой из фильма.
    pub fn add_bookmark_here(&mut self) {
        let Some(key) = self.video_key() else {
            self.show_notice("Файл не открыт");
            return;
        };

        let Some(engine) = self.engine.as_ref() else {
            return;
        };

        let time_ms = (engine.state().position * 1000.0) as i64;
        let name = self.subtitle_text.main.clone().map(|line| {
            // Реплика может быть многострочной, а имя файла — нет.
            line.replace('\n', " ").trim().to_string()
        });

        let duration = self.settings.fragments.duration_sec;
        let buffer = self.settings.fragments.buffer_sec;

        let video = self.bookmarks.for_video_mut(&key, duration, buffer);
        let Some(list) = video.active_mut() else {
            return;
        };

        if list.add(time_ms, name.clone()) {
            self.bookmarks.save();
            let label = name.unwrap_or_else(|| "закладка".into());
            self.show_notice(&format!("Добавлено: {label}"));
            tracing::info!(time_ms, "закладка добавлена");
        } else {
            self.show_notice("Здесь уже есть закладка");
        }
    }

    /// Убирает закладку, ближайшую к текущей позиции.
    pub fn remove_bookmark_here(&mut self) {
        let Some(key) = self.video_key() else {
            return;
        };

        let Some(engine) = self.engine.as_ref() else {
            return;
        };

        let time_ms = (engine.state().position * 1000.0) as i64;
        let duration = self.settings.fragments.duration_sec;
        let buffer = self.settings.fragments.buffer_sec;

        let video = self.bookmarks.for_video_mut(&key, duration, buffer);
        let Some(list) = video.active_mut() else {
            return;
        };

        // Ближайшая метка в пределах длительности фрагмента.
        let nearest = list
            .bookmarks
            .iter()
            .min_by_key(|b| (b.time_ms - time_ms).abs())
            .filter(|b| (b.time_ms - time_ms).abs() < i64::from(duration) * 1000)
            .map(|b| b.time_ms);

        match nearest {
            Some(found) if list.remove_at(found) => {
                self.bookmarks.save();
                self.show_notice("Закладка убрана");
            }
            _ => self.show_notice("Рядом нет закладок"),
        }
    }

    /// Удаляет закладку по точному времени.
    pub fn remove_bookmark_at(&mut self, time_ms: i64) {
        let Some(key) = self.video_key() else {
            return;
        };

        let duration = self.settings.fragments.duration_sec;
        let buffer = self.settings.fragments.buffer_sec;

        let video = self.bookmarks.for_video_mut(&key, duration, buffer);
        if let Some(list) = video.active_mut()
            && list.remove_at(time_ms)
        {
            self.bookmarks.save();
        }
    }

    /// Отрезки активного списка — те самые жёлтые области на полосе.
    pub fn fragment_ranges(&self) -> Vec<FragmentRange> {
        let Some(video) = self.current_bookmarks() else {
            return Vec::new();
        };
        let Some(list) = video.active() else {
            return Vec::new();
        };

        list.bookmarks
            .iter()
            .map(|b| {
                FragmentRange::from_bookmark(
                    b.seconds(),
                    f64::from(list.buffer_sec),
                    f64::from(list.duration_sec),
                )
            })
            .collect()
    }

    /// Куда складывать вырезанные фрагменты.
    ///
    /// У списка может быть своя папка; иначе берётся общая из настроек,
    /// а если и её нет — папка рядом с исходным файлом.
    pub fn fragments_output_dir(&self) -> Option<std::path::PathBuf> {
        let list_dir = self
            .current_bookmarks()
            .and_then(|v| v.active())
            .and_then(|l| l.output_dir.clone());

        list_dir
            .or_else(|| self.settings.fragments.output_dir.clone())
            .or_else(|| {
                self.current_path
                    .as_ref()
                    .and_then(|p| p.parent())
                    .map(Path::to_path_buf)
            })
    }
}
