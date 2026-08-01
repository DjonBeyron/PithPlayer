//! Запоминание позиции просмотра и предложение продолжить.
//!
//! Правила отбора живут в `pith-store`; здесь — когда сохранять
//! и что показывать пользователю.

use std::path::{Path, PathBuf};

use super::PithApp;

/// Как часто сохранять позицию во время просмотра, секунды.
///
/// Плеер может закрыться аварийно, и терять больше этого времени обидно.
const SAVE_INTERVAL_SEC: f64 = 5.0;

/// Предложение продолжить просмотр.
pub struct ResumeOffer {
    pub file_name: String,
    pub position: f64,
}

impl PithApp {
    /// Запоминает путь к открываемому файлу.
    pub(super) fn set_current_path(&mut self, path: &str) {
        // Перед сменой файла сохраняем позицию предыдущего.
        self.store_position();
        self.current_path = Some(PathBuf::from(path));
        self.resume_offer = None;
        self.last_position_save = 0.0;
    }

    /// Готовит предложение продолжить, если для файла есть позиция.
    pub(super) fn prepare_resume_offer(&mut self) {
        let Some(path) = self.current_path.clone() else {
            return;
        };

        let Some(entry) = self.watch_positions.resume_position(&path) else {
            return;
        };

        tracing::info!(
            позиция = entry.position,
            файл = %entry.file_name,
            "есть сохранённая позиция просмотра"
        );

        self.resume_offer = Some(ResumeOffer {
            file_name: entry.file_name.clone(),
            position: entry.position,
        });
    }

    pub fn resume_offer(&self) -> Option<&ResumeOffer> {
        self.resume_offer.as_ref()
    }

    /// Продолжить с сохранённой позиции.
    pub fn accept_resume(&mut self) {
        let Some(offer) = self.resume_offer.take() else {
            return;
        };

        tracing::info!(позиция = offer.position, "продолжаю просмотр");
        self.seek_absolute(offer.position);
    }

    /// Смотреть с начала. Сохранённая позиция больше не нужна.
    pub fn decline_resume(&mut self) {
        self.resume_offer = None;

        if let Some(path) = self.current_path.clone() {
            self.watch_positions.forget(&path);
        }
    }

    /// Периодически сохраняет позицию во время воспроизведения.
    pub(super) fn store_position_periodically(&mut self) {
        let Some(engine) = self.engine.as_ref() else {
            return;
        };

        let state = engine.state();
        if state.paused || !state.file_loaded {
            return;
        }

        if (state.position - self.last_position_save).abs() < SAVE_INTERVAL_SEC {
            return;
        }

        self.store_position();
    }

    /// Сохраняет текущую позицию просмотра.
    ///
    /// Пока пользователь не ответил на предложение продолжить, позицию
    /// не трогаем: иначе воспроизведение с начала затрёт её.
    pub(super) fn store_position(&mut self) {
        if self.resume_offer.is_some() {
            return;
        }

        let Some(path) = self.current_path.clone() else {
            return;
        };

        let Some(engine) = self.engine.as_ref() else {
            return;
        };

        let state = engine.state();
        if !state.file_loaded {
            return;
        }

        let position = state.position;
        let duration = state.duration;
        self.last_position_save = position;

        let saved = self
            .watch_positions
            .remember(Path::new(&path), position, duration);
        tracing::debug!(position, duration, saved, "сохранение позиции просмотра");
    }
}
