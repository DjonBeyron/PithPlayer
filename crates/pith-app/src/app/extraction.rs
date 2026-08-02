//! Запуск нарезки отрезков и слежение за ходом работы.

use std::sync::mpsc::{Receiver, channel};

use pith_fragments::{ExtractionOutcome, FragmentJob};

use super::PithApp;

/// Ход нарезки.
#[derive(Debug, Clone, Copy)]
pub struct ExtractionProgress {
    pub done: usize,
    pub total: usize,
    pub failed: usize,
}

impl ExtractionProgress {
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        self.done as f32 / self.total as f32
    }
}

/// Состояние нарезки.
#[derive(Default)]
pub struct ExtractionState {
    progress: Option<ExtractionProgress>,
    events: Option<Receiver<ExtractionOutcome>>,
}

impl PithApp {
    pub fn extraction_progress(&self) -> Option<ExtractionProgress> {
        self.extraction.progress
    }

    /// Можно ли резать: нужен FFmpeg и хотя бы одна закладка.
    pub fn can_extract(&self) -> bool {
        pith_fragments::is_ffmpeg_available()
    }

    /// Запускает нарезку всех закладок активного списка.
    pub fn start_extraction(&mut self) {
        if self.extraction.progress.is_some() {
            return;
        }

        let Some(jobs) = self.build_jobs() else {
            return;
        };

        if jobs.is_empty() {
            self.show_notice("Нет закладок для нарезки");
            return;
        }

        let total = jobs.len();
        self.extraction.progress = Some(ExtractionProgress {
            done: 0,
            total,
            failed: 0,
        });

        let (sender, receiver) = channel();
        self.extraction.events = Some(receiver);

        tracing::info!(отрезков = total, "начинаю нарезку");

        std::thread::spawn(move || {
            for job in jobs {
                let outcome = pith_fragments::run_job(&job);
                if sender.send(outcome).is_err() {
                    // Приложение закрылось — продолжать незачем.
                    break;
                }
            }
        });
    }

    /// Готовит задачи для всех закладок активного списка.
    fn build_jobs(&mut self) -> Option<Vec<FragmentJob>> {
        let source = self.current_path.clone()?;
        let output_dir = self.fragments_output_dir()?;

        if let Err(e) = std::fs::create_dir_all(&output_dir) {
            tracing::error!(error = %e, ?output_dir, "не удалось создать папку вывода");
            self.show_notice("Не удалось создать папку для отрезков");
            return None;
        }

        let video = self.current_bookmarks()?;
        let list = video.active()?;

        let reencode = self.settings.fragments.reencode;
        let audio_index = self.current_audio_index();
        let extension = self.container_for_current_file();

        let jobs = list
            .bookmarks
            .iter()
            .map(|bookmark| {
                let requested = (bookmark.seconds() - f64::from(list.buffer_sec)).max(0.0);

                // Перепаковка режет по опорным кадрам: встаём точно на них,
                // иначе отрезок начинается с чёрного экрана.
                let start = if reencode {
                    requested
                } else {
                    pith_fragments::align_to_keyframe(&source, requested).unwrap_or(requested)
                };

                let output =
                    pith_fragments::unique_output_path(&output_dir, &bookmark.label(), extension);

                FragmentJob {
                    source: source.clone(),
                    output,
                    start,
                    duration: f64::from(list.duration_sec),
                    audio_index,
                    reencode,
                }
            })
            .collect();

        Some(jobs)
    }

    /// Порядковый номер выбранной аудиодорожки среди аудио.
    ///
    /// FFmpeg считает дорожки внутри своего вида, а mpv выдаёт сквозные
    /// номера — передавать их напрямую нельзя.
    fn current_audio_index(&self) -> Option<i64> {
        let selected = self.selected_tracks.audio?;

        self.tracks
            .iter()
            .filter(|t| t.kind == pith_mpv::TrackKind::Audio)
            .position(|t| t.id == selected)
            .map(|index| index as i64)
    }

    /// Контейнер под кодеки текущего файла.
    fn container_for_current_file(&self) -> &'static str {
        let Some(engine) = self.engine.as_ref() else {
            return "mp4";
        };

        let video = engine.property_string("video-format").ok();
        let audio = engine.property_string("audio-codec-name").ok();

        pith_fragments::choose_container(video.as_deref(), audio.as_deref())
    }

    /// Забирает готовые результаты нарезки.
    pub(super) fn poll_extraction(&mut self) {
        let Some(receiver) = self.extraction.events.as_ref() else {
            return;
        };

        let mut finished = Vec::new();
        while let Ok(outcome) = receiver.try_recv() {
            finished.push(outcome);
        }

        if finished.is_empty() {
            return;
        }

        let Some(progress) = self.extraction.progress.as_mut() else {
            return;
        };

        for outcome in finished {
            match outcome {
                ExtractionOutcome::Done { output, bytes } => {
                    progress.done += 1;
                    tracing::info!(?output, мегабайт = bytes / 1_048_576, "отрезок готов");
                }
                ExtractionOutcome::Failed { output, reason } => {
                    progress.done += 1;
                    progress.failed += 1;
                    tracing::error!(?output, %reason, "отрезок не вырезан");
                }
            }
        }

        if progress.done >= progress.total {
            let summary = summarize(*progress);
            self.extraction.progress = None;
            self.extraction.events = None;
            self.show_notice(&summary);
        }
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
    }

    /// Показывает панель при наведении на правый край окна.
    ///
    /// Так она вызывалась в v4: подвести курсор к правой стороне —
    /// панель выезжает, увести — прячется.
    pub(super) fn update_bookmarks_panel_hover(&mut self, ctx: &egui::Context) {
        if self.bookmarks_panel_pinned {
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

/// До какой границы слева панель считается «под курсором».
///
/// Чуть шире самой панели: иначе она мигала бы на её краю.
const PANEL_KEEP_WIDTH: f32 = 380.0;

fn summarize(progress: ExtractionProgress) -> String {
    if progress.failed == 0 {
        format!("Готово: {} отрезков", progress.total)
    } else {
        format!(
            "Готово: {} из {}, с ошибками: {}",
            progress.total - progress.failed,
            progress.total,
            progress.failed
        )
    }
}
