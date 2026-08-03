//! Подготовка и выполнение очереди нарезки в фоне.
//!
//! Готовить задачи в потоке интерфейса нельзя: выравнивание по опорным
//! кадрам зовёт `ffprobe`, и на большом файле это секунды. Пользователь
//! в это время видел неподвижное окно и решал, что плеер завис.

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use pith_fragments::{ExtractionOutcome, FragmentJob};
use pith_store::BookmarkList;

/// Что и куда режем: один список закладок и его папка вывода.
pub struct ListPlan {
    pub list: BookmarkList,
    pub dir: PathBuf,
}

/// Всё, что нужно фоновому потоку для подготовки задач.
pub struct ExtractionRequest {
    pub source: PathBuf,
    pub plans: Vec<ListPlan>,
    pub reencode: bool,
    pub audio_aac: bool,
    pub audio_index: Option<i64>,
    pub extension: &'static str,
    pub workers: usize,
}

/// Что приходит от фонового потока.
pub enum ExtractionEvent {
    /// Задачи готовы, известно их число.
    Prepared { total: usize },
    /// Один отрезок закончен.
    Finished(ExtractionOutcome),
    /// Папку вывода создать не удалось.
    DirectoryFailed,
}

/// Готовит задачи и выполняет их, ни на миг не занимая поток интерфейса.
pub fn spawn(request: ExtractionRequest, sender: Sender<ExtractionEvent>) {
    std::thread::spawn(move || {
        let Some(jobs) = build_jobs(&request, &sender) else {
            return;
        };

        // Число задач становится известно только после подготовки: одна
        // закладка может дать несколько файлов при нарезке всех списков.
        if sender
            .send(ExtractionEvent::Prepared { total: jobs.len() })
            .is_err()
        {
            return;
        }

        run_queue(jobs, request.workers, sender);
    });
}

/// Собирает задачи FFmpeg по каждому списку.
fn build_jobs(
    request: &ExtractionRequest,
    sender: &Sender<ExtractionEvent>,
) -> Option<Vec<FragmentJob>> {
    let mut jobs = Vec::new();

    for plan in &request.plans {
        if let Err(e) = std::fs::create_dir_all(&plan.dir) {
            tracing::error!(error = %e, папка = ?plan.dir, "не удалось создать папку вывода");
            let _ = sender.send(ExtractionEvent::DirectoryFailed);
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
        let starts: Vec<f64> = if request.reencode {
            requested.clone()
        } else {
            pith_fragments::align_to_keyframes(&request.source, &requested)
                .into_iter()
                .zip(&requested)
                .map(|(aligned, requested)| aligned.unwrap_or(*requested))
                .collect()
        };

        for (bookmark, start) in plan.list.bookmarks.iter().zip(starts) {
            jobs.push(FragmentJob {
                source: request.source.clone(),
                output: pith_fragments::unique_output_path(
                    &plan.dir,
                    &bookmark.label(),
                    request.extension,
                ),
                start,
                duration: f64::from(plan.list.duration_sec),
                audio_index: request.audio_index,
                reencode: request.reencode,
                audio_aac: request.audio_aac,
            });
        }
    }

    Some(jobs)
}

/// Раздаёт очередь задач нескольким потокам.
///
/// Очередь общая: кто освободился, тот берёт следующую. Так медленный
/// отрезок не задерживает остальные, в отличие от деления пополам.
fn run_queue(jobs: Vec<FragmentJob>, workers: usize, sender: Sender<ExtractionEvent>) {
    let queue = Arc::new(Mutex::new(jobs.into_iter()));

    for _ in 0..workers {
        let queue = Arc::clone(&queue);
        let sender = sender.clone();

        std::thread::spawn(move || {
            // Замок держим только на время выдачи задачи, а не на нарезку.
            while let Some(job) = queue.lock().ok().and_then(|mut q| q.next()) {
                let outcome = pith_fragments::run_job(&job);
                if sender.send(ExtractionEvent::Finished(outcome)).is_err() {
                    // Приложение закрылось — продолжать незачем.
                    break;
                }
            }
        });
    }
}
