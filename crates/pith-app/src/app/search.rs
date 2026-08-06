//! Поиск по субтитрам: извлечение дорожки и состояние окна поиска.

use std::path::Path;
use std::sync::mpsc::{Receiver, channel};

use pith_subs::{Cue, SearchHit};

use super::PithApp;

/// Сколько результатов показываем.
const MAX_RESULTS: usize = 200;

/// Состояние окна поиска.
#[derive(Default)]
pub struct SearchState {
    pub open: bool,
    pub query: String,
    pub hits: Vec<SearchHit>,
    /// Реплики текущей дорожки. Пусто, пока идёт извлечение.
    cues: Vec<Cue>,
    /// Извлечение идёт в фоне: на длинном фильме оно занимает секунды.
    pending: Option<Receiver<Vec<Cue>>>,
    /// Что показать вместо списка: ход извлечения или причина неудачи.
    pub status: Option<String>,
}

impl SearchState {
    pub fn is_ready(&self) -> bool {
        !self.cues.is_empty()
    }
}

impl PithApp {
    pub fn search_state(&self) -> &SearchState {
        &self.search
    }

    /// Открывает окно поиска и запускает извлечение дорожки.
    pub fn open_search(&mut self) {
        self.search.open = true;

        if self.search.is_ready() || self.search.pending.is_some() {
            return;
        }

        self.start_subtitle_extraction();
    }

    pub fn close_search(&mut self) {
        self.search.open = false;
    }

    /// Обновляет строку запроса и пересчитывает результаты.
    pub fn set_search_query(&mut self, query: String) {
        self.search.query = query;
        self.search.hits = pith_subs::search(&self.search.cues, &self.search.query, MAX_RESULTS);
    }

    /// Запускает извлечение текущей дорожки субтитров в фоне.
    fn start_subtitle_extraction(&mut self) {
        let Some(path) = self.current_path.clone() else {
            self.search.status = Some(crate::tr!("Файл не открыт", "No file open").into());
            return;
        };

        let Some(index) = self.current_subtitle_index() else {
            self.search.status =
                Some(crate::tr!("Субтитры не выбраны", "No subtitle track selected").into());
            return;
        };

        if !pith_subs::is_ffmpeg_available() {
            self.search.status = Some(
                crate::tr!(
                    "Для поиска нужен FFmpeg — положите ffmpeg.exe рядом с плеером",
                    "Search needs FFmpeg — put ffmpeg.exe next to the player"
                )
                .into(),
            );
            return;
        }

        self.search.status = Some(crate::tr!("Читаю субтитры…", "Reading subtitles…").into());

        let (sender, receiver) = channel();
        self.search.pending = Some(receiver);

        std::thread::spawn(move || {
            let cues = extract(&path, index);
            let _ = sender.send(cues);
        });
    }

    /// Порядковый номер выбранной дорожки среди субтитров.
    ///
    /// FFmpeg считает дорожки по порядку внутри своего вида, а mpv выдаёт
    /// сквозные номера — их нельзя передавать напрямую.
    fn current_subtitle_index(&self) -> Option<i64> {
        let selected = self.selected_tracks.subtitle?;

        self.tracks
            .iter()
            .filter(|t| t.kind == pith_mpv::TrackKind::Subtitle)
            .position(|t| t.id == selected)
            .map(|index| index as i64)
    }

    /// Забирает результат извлечения, если он готов.
    pub(super) fn poll_subtitle_extraction(&mut self) {
        let Some(receiver) = self.search.pending.as_ref() else {
            return;
        };

        let Ok(cues) = receiver.try_recv() else {
            return;
        };

        self.search.pending = None;

        if cues.is_empty() {
            self.search.status = Some(
                crate::tr!(
                    "Не удалось прочитать субтитры",
                    "Could not read the subtitles"
                )
                .into(),
            );
            return;
        }

        tracing::info!(реплик = cues.len(), "субтитры готовы к поиску");
        self.search.status = None;
        self.search.cues = cues;

        // Запрос мог быть введён, пока шло извлечение.
        let query = self.search.query.clone();
        self.set_search_query(query);
    }

    /// Сбрасывает разобранные субтитры при смене файла.
    pub(super) fn reset_search(&mut self) {
        self.search = SearchState::default();
    }
}

fn extract(path: &Path, index: i64) -> Vec<Cue> {
    match pith_subs::extract_track(path, index) {
        Ok(cues) => cues,
        Err(e) => {
            tracing::warn!(error = %e, "не удалось извлечь субтитры для поиска");
            Vec::new()
        }
    }
}
