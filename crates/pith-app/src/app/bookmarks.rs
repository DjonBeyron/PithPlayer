//! Закладки: добавление, удаление, отрезки для полосы перемотки.

use std::path::Path;

use pith_store::{BookmarkList, VideoBookmarks};

use super::PithApp;
use crate::theme;
use crate::ui::FragmentRange;

/// Насколько приглушаются отрезки неактивных списков.
const DIMMED: f32 = 0.4;

impl PithApp {
    /// Ключ текущего видео в хранилище закладок — имя файла без расширения.
    fn video_key(&self) -> Option<String> {
        self.current_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_string())
    }

    /// Открыт ли файл.
    ///
    /// Отдельная проверка от `current_bookmarks`: у только что открытого
    /// видео записи в хранилище ещё нет, но панель отрезков работать обязана.
    pub fn has_open_file(&self) -> bool {
        self.current_path.is_some()
    }

    /// Закладки текущего видео.
    pub fn current_bookmarks(&self) -> Option<&VideoBookmarks> {
        let key = self.video_key()?;
        self.bookmarks.for_video(&key)
    }

    /// Закладки текущего видео для правки; запись создаётся при первом обращении.
    pub(super) fn current_bookmarks_mut(&mut self) -> Option<&mut VideoBookmarks> {
        let key = self.video_key()?;
        let duration = self.settings.fragments.duration_sec;
        let buffer = self.settings.fragments.buffer_sec;

        Some(self.bookmarks.for_video_mut(&key, duration, buffer))
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
            crate::slow::probe("сохранение закладок", || {
                self.bookmarks.save()
            });
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

    /// Отрезки всех списков — те самые цветные области на полосе.
    ///
    /// Каждому списку — свой цвет, чтобы было видно принадлежность метки
    /// (PLAN.md §6.5). Неактивные приглушены, активный рисуется последним
    /// и потому остаётся поверх остальных.
    pub fn fragment_ranges(&self) -> Vec<FragmentRange> {
        let Some(video) = self.current_bookmarks() else {
            return Vec::new();
        };

        let mut ranges = Vec::new();

        for (index, list) in video.lists.iter().enumerate() {
            let active = list.name == video.active_list;
            if active {
                continue;
            }
            push_ranges(
                &mut ranges,
                list,
                theme::list_color(index).gamma_multiply(DIMMED),
            );
        }

        if let Some((index, list)) = video
            .lists
            .iter()
            .enumerate()
            .find(|(_, l)| l.name == video.active_list)
        {
            push_ranges(&mut ranges, list, theme::list_color(index));
        }

        ranges
    }

    /// Цвет активного списка — им же помечены строки в панели.
    pub fn active_list_color(&self) -> egui::Color32 {
        let index = self
            .current_bookmarks()
            .and_then(|v| v.lists.iter().position(|l| l.name == v.active_list))
            .unwrap_or(0);

        theme::list_color(index)
    }

    /// Куда складывать отрезки активного списка.
    pub fn fragments_output_dir(&self) -> Option<std::path::PathBuf> {
        let list_dir = self
            .current_bookmarks()
            .and_then(|v| v.active())
            .and_then(|l| l.output_dir.clone());

        self.output_dir_or_default(list_dir)
    }

    /// Куда складывать отрезки указанного списка.
    ///
    /// У списка может быть своя папка; иначе берётся общая из настроек,
    /// а если и её нет — папка рядом с исходным файлом.
    pub(super) fn output_dir_for(&self, list: &BookmarkList) -> Option<std::path::PathBuf> {
        self.output_dir_or_default(list.output_dir.clone())
    }

    fn output_dir_or_default(
        &self,
        list_dir: Option<std::path::PathBuf>,
    ) -> Option<std::path::PathBuf> {
        list_dir
            .or_else(|| self.settings.fragments.output_dir.clone())
            .or_else(|| {
                self.current_path
                    .as_ref()
                    .and_then(|p| p.parent())
                    .map(Path::to_path_buf)
            })
    }

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

    /// Показывает панель при наведении на правый край окна.
    ///
    /// Так она вызывалась в v4: подвести курсор к правой стороне —
    /// панель выезжает, увести — прячется.
    pub(super) fn update_bookmarks_panel_hover(&mut self, ctx: &egui::Context) {
        if self.bookmarks_panel_pinned {
            return;
        }

        // Пока открыт диалог списка, панель не должна уезжать из-под него.
        if self.list_dialog.is_some() {
            self.bookmarks_panel = true;
            return;
        }

        let screen = ctx.input(|i| i.viewport_rect());
        let Some(pointer) = ctx.input(|i| i.pointer.hover_pos()) else {
            // Курсор вне окна — прятать не спешим: он мог уйти на панель
            // соседнего монитора.
            return;
        };

        let was_open = self.bookmarks_panel;

        if pointer.x >= screen.max.x - HOVER_ZONE_WIDTH {
            if !self.bookmarks_panel {
                // Панель только что вызвана — дадим ей кадр на разметку.
                self.bookmarks_panel_warmup = PANEL_WARMUP_FRAMES;
            }
            self.bookmarks_panel = true;
        } else if self.bookmarks_panel && pointer.x < screen.max.x - PANEL_KEEP_WIDTH {
            // Пока курсор над самой панелью, она остаётся на месте.
            self.bookmarks_panel = false;
        }

        if was_open != self.bookmarks_panel {
            tracing::debug!(
                открыта = self.bookmarks_panel,
                курсор_x = pointer.x,
                "панель отрезков"
            );
        }
    }
}

/// Ширина полосы у правого края, вызывающей панель.
const HOVER_ZONE_WIDTH: f32 = 40.0;

/// Сколько кадров панель рисуется невидимой перед показом.
///
/// Двух достаточно: на первом egui считает размеры списка, на втором —
/// положение полосы прокрутки, которое от этих размеров зависит.
const PANEL_WARMUP_FRAMES: u8 = 2;

/// До какой границы слева панель считается «под курсором».
///
/// Чуть шире самой панели: иначе она мигала бы на её краю.
const PANEL_KEEP_WIDTH: f32 = 380.0;

/// Складывает отрезки списка в общий набор для полосы перемотки.
fn push_ranges(ranges: &mut Vec<FragmentRange>, list: &BookmarkList, color: egui::Color32) {
    ranges.extend(list.bookmarks.iter().map(|b| {
        FragmentRange::from_bookmark(
            b.seconds(),
            f64::from(list.buffer_sec),
            f64::from(list.duration_sec),
            color,
        )
    }));
}
