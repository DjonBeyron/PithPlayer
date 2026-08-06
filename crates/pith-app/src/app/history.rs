//! История открытых файлов и папок.
//!
//! Хранилище лежит в `pith-store`; здесь — когда пополнять историю
//! и что показывает её окно.

use std::path::PathBuf;

use super::PithApp;

impl PithApp {
    /// Открыто ли окно истории.
    pub fn history_open(&self) -> bool {
        self.history_open
    }

    /// Показывает историю. Пропавшие записи убираются сразу: предлагать
    /// открыть то, чего больше нет, незачем.
    pub fn open_history(&mut self) {
        self.history.forget_missing();
        self.history_open = true;
        self.history_opened_at = Some(self.frame_time);
    }

    /// Окно истории открыто прямо сейчас, этим же кадром.
    ///
    /// Нажатие, которым его вызвали — пункт меню или правый щелчок по
    /// кнопке, — приходит в том же кадре и считается «мимо окна». Без
    /// этой проверки окно закрывалось тем же щелчком, что и открывалось.
    pub fn history_just_opened(&self) -> bool {
        self.history_opened_at == Some(self.frame_time)
    }

    pub fn close_history(&mut self) {
        self.history_open = false;
    }

    /// Последние открытые файлы, свежие впереди.
    pub fn history_files(&self) -> Vec<PathBuf> {
        self.history.files().to_vec()
    }

    /// Последние папки, свежие впереди.
    pub fn history_dirs(&self) -> Vec<PathBuf> {
        self.history.dirs().to_vec()
    }

    /// Открывает файл из истории.
    pub fn open_from_history(&mut self, path: &std::path::Path) {
        self.history_open = false;
        self.open_file(&path.to_string_lossy());
    }

    /// Открывает диалог выбора файла, начиная с папки из истории.
    pub fn open_dialog_in(&mut self, dir: &std::path::Path) {
        self.history_open = false;

        if let Some(path) = self.file_dialog().set_directory(dir).pick_file() {
            self.open_file(&path.to_string_lossy());
        }
    }

    /// Пополняет историю открытым файлом.
    pub(super) fn remember_in_history(&mut self, path: &str) {
        let path = PathBuf::from(path);

        // Сетевые адреса и потоки в историю не идут: вернуться по ним
        // через диалог всё равно нельзя.
        if !path.is_absolute() {
            return;
        }

        self.history.remember(&path);
    }
}
