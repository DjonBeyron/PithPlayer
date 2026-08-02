//! Запуск нарезки отрезков и слежение за ходом работы.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use pith_fragments::{ExtractionOutcome, FragmentJob};
use pith_store::BookmarkList;

use super::PithApp;

/// Что и куда режем: один список закладок и его папка вывода.
struct ListPlan {
    list: BookmarkList,
    dir: PathBuf,
}

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
        let plans = self.plan_active_list();
        self.start_jobs(plans);
    }

    /// Режет все списки видео: каждый — в свою подпапку с именем списка.
    pub fn start_extraction_all_lists(&mut self) {
        let plans = self.plan_all_lists();
        self.start_jobs(plans);
    }

    fn start_jobs(&mut self, plans: Vec<ListPlan>) {
        if self.extraction.progress.is_some() {
            return;
        }

        let Some(jobs) = self.build_jobs(plans) else {
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

        let workers = self.worker_count(total);
        tracing::info!(отрезков = total, потоков = workers, "начинаю нарезку");

        run_queue(jobs, workers, sender);
    }

    /// Сколько отрезков резать одновременно.
    ///
    /// При перепаковке узкое место — диск. На SSD три процесса дают
    /// ускорение в два-три раза, на HDD параллельность вредит из-за
    /// перемещений головки (PLAN.md §6.4). Тип носителя не определяем:
    /// значение берётся из настроек, ноль означает «как обычно».
    fn worker_count(&self, total: usize) -> usize {
        let configured = self.settings.fragments.parallel_jobs;

        let workers = if configured == 0 {
            DEFAULT_WORKERS
        } else {
            configured
        };

        workers.clamp(1, total.max(1))
    }

    /// Активный список и папка, куда лягут его отрезки.
    fn plan_active_list(&self) -> Vec<ListPlan> {
        let Some(list) = self.current_bookmarks().and_then(|v| v.active()) else {
            return Vec::new();
        };
        let Some(dir) = self.output_dir_for(list) else {
            return Vec::new();
        };

        vec![ListPlan {
            list: list.clone(),
            dir,
        }]
    }

    /// Все списки видео, каждый — в подпапку со своим именем.
    ///
    /// Подпапка нужна, чтобы отрезки разных списков не перемешивались:
    /// одинаковые реплики в разных списках дали бы одно имя файла.
    fn plan_all_lists(&self) -> Vec<ListPlan> {
        let Some(video) = self.current_bookmarks() else {
            return Vec::new();
        };

        video
            .lists
            .iter()
            .filter(|list| !list.bookmarks.is_empty())
            .filter_map(|list| {
                let dir = self
                    .output_dir_for(list)?
                    .join(pith_fragments::sanitize(&list.name));

                Some(ListPlan {
                    list: list.clone(),
                    dir,
                })
            })
            .collect()
    }

    /// Готовит задачи FFmpeg по списку закладок каждого набора.
    fn build_jobs(&mut self, plans: Vec<ListPlan>) -> Option<Vec<FragmentJob>> {
        let source = self.current_path.clone()?;

        let reencode = self.settings.fragments.reencode;
        let audio_index = self.current_audio_index();
        let extension = self.container_for_current_file();

        let mut jobs = Vec::new();

        for plan in plans {
            if let Err(e) = std::fs::create_dir_all(&plan.dir) {
                tracing::error!(error = %e, папка = ?plan.dir, "не удалось создать папку вывода");
                self.show_notice("Не удалось создать папку для отрезков");
                return None;
            }

            let requested: Vec<f64> = plan
                .list
                .bookmarks
                .iter()
                .map(|b| (b.seconds() - f64::from(plan.list.buffer_sec)).max(0.0))
                .collect();

            // Перепаковка режет по опорным кадрам: встаём точно на них,
            // иначе отрезок начинается с чёрного экрана. Весь список
            // выравнивается одним вызовом ffprobe (PLAN.md §6.4).
            let starts: Vec<f64> = if reencode {
                requested.clone()
            } else {
                pith_fragments::align_to_keyframes(&source, &requested)
                    .into_iter()
                    .zip(&requested)
                    .map(|(aligned, requested)| aligned.unwrap_or(*requested))
                    .collect()
            };

            for (bookmark, start) in plan.list.bookmarks.iter().zip(starts) {
                jobs.push(FragmentJob {
                    source: source.clone(),
                    output: pith_fragments::unique_output_path(
                        &plan.dir,
                        &bookmark.label(),
                        extension,
                    ),
                    start,
                    duration: f64::from(plan.list.duration_sec),
                    audio_index,
                    reencode,
                });
            }
        }

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
}

/// Сколько процессов FFmpeg держать одновременно, если в настройках ноль.
const DEFAULT_WORKERS: usize = 3;

/// Раздаёт очередь задач нескольким потокам.
///
/// Очередь общая: кто освободился, тот берёт следующую. Так медленный
/// отрезок не задерживает остальные, в отличие от деления пополам.
fn run_queue(jobs: Vec<FragmentJob>, workers: usize, sender: Sender<ExtractionOutcome>) {
    let queue = Arc::new(Mutex::new(jobs.into_iter()));

    for _ in 0..workers {
        let queue = Arc::clone(&queue);
        let sender = sender.clone();

        std::thread::spawn(move || {
            // Замок держим только на время выдачи задачи, а не на нарезку.
            while let Some(job) = queue.lock().ok().and_then(|mut q| q.next()) {
                if sender.send(pith_fragments::run_job(&job)).is_err() {
                    // Приложение закрылось — продолжать незачем.
                    break;
                }
            }
        });
    }
}

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
