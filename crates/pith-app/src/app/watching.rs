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

/// Сколько секунд предложение продолжить висит без ответа.
///
/// Оно ничего не загораживает и ничем не грозит: не ответили — фильм
/// просто идёт с начала. Поэтому висеть ему долго незачем, а восьми
/// секунд хватает, чтобы заметить и попасть мышью.
const OFFER_SECONDS: f64 = 8.0;

/// Предложение продолжить просмотр.
pub struct ResumeOffer {
    pub position: f64,
    /// Время кадра, после которого предложение исчезает само.
    until: f64,
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
            position: entry.position,
            until: self.frame_time + OFFER_SECONDS,
        });
    }

    pub fn resume_offer(&self) -> Option<&ResumeOffer> {
        self.resume_offer.as_ref()
    }

    /// Убирает предложение, на которое не ответили.
    ///
    /// Сохранённую позицию при этом не забываем — её просто перепишет
    /// текущий просмотр, как и всякий другой. Ответ «Сначала» отличается
    /// именно тем, что стирает её сразу.
    pub(super) fn expire_resume_offer(&mut self) {
        let expired = self
            .resume_offer
            .as_ref()
            .is_some_and(|offer| offer.until <= self.frame_time);

        if expired {
            tracing::debug!("предложение продолжить убрано без ответа");
            self.resume_offer = None;
        }
    }

    /// Продлевает предложение: под курсором оно исчезать не должно.
    pub fn postpone_resume(&mut self) {
        let until = self.frame_time + OFFER_SECONDS;

        if let Some(offer) = self.resume_offer.as_mut() {
            offer.until = until;
        }
    }

    /// Какая доля отведённого времени ещё осталась: 1.0 — только показали.
    ///
    /// По ней рисуется полоска обратного отсчёта, чтобы исчезновение
    /// не выглядело внезапным.
    pub fn resume_remaining(&self) -> f32 {
        let Some(offer) = self.resume_offer.as_ref() else {
            return 0.0;
        };

        (((offer.until - self.frame_time) / OFFER_SECONDS) as f32).clamp(0.0, 1.0)
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

        // Считает хэш файла и пишет json — самая тяжёлая операция кадра
        // из тех, что делаются по ходу воспроизведения.
        let saved = crate::slow::probe(
            "сохранение позиции просмотра",
            || {
                self.watch_positions
                    .remember(Path::new(&path), position, duration)
            },
        );
        tracing::debug!(position, duration, saved, "сохранение позиции просмотра");
    }
}
