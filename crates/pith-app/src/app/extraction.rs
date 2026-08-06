//! Запуск нарезки отрезков и слежение за ходом работы.

use std::sync::mpsc::{Receiver, channel};

use pith_fragments::ExtractionOutcome;

use super::PithApp;
use super::extraction_queue::{ExtractionEvent, ExtractionRequest, ListPlan, spawn};

/// Ход нарезки.
#[derive(Debug, Clone, Copy)]
pub struct ExtractionProgress {
    pub done: usize,
    /// Ноль означает, что задачи ещё готовятся.
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

    /// Идёт ли ещё подготовка задач.
    pub fn is_preparing(&self) -> bool {
        self.total == 0
    }
}

/// Состояние нарезки.
#[derive(Default)]
pub struct ExtractionState {
    progress: Option<ExtractionProgress>,
    events: Option<Receiver<ExtractionEvent>>,
}

impl PithApp {
    pub fn extraction_progress(&self) -> Option<ExtractionProgress> {
        self.extraction.progress
    }

    /// Можно ли резать: нужен FFmpeg и хотя бы одна закладка.
    ///
    /// Первый вызов запускает `ffmpeg -version` и потому небесплатен;
    /// дальше ответ берётся из кэша. Замер покажет, если проверка успела
    /// попасть в кадр интерфейса.
    pub fn can_extract(&self) -> bool {
        crate::slow::probe(
            "проверка наличия FFmpeg",
            pith_fragments::is_ffmpeg_available,
        )
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

    /// Режет один отрезок — тот, что напротив нажатой закладки.
    ///
    /// Резать весь список ради одной метки долго: на длинном списке это
    /// десятки файлов и минуты ожидания, а нужен бывает ровно один.
    pub fn start_extraction_one(&mut self, time_ms: i64) {
        let plans = self.plan_single_bookmark(time_ms);
        self.start_jobs(plans);
    }

    /// Отдаёт работу фоновому потоку и сразу показывает сообщение.
    ///
    /// Подготовка задач зовёт `ffprobe` и занимает секунды на большом
    /// файле — в потоке интерфейса это выглядело зависанием.
    fn start_jobs(&mut self, plans: Vec<ListPlan>) {
        if self.extraction.progress.is_some() {
            return;
        }

        let Some(source) = self.current_path.clone() else {
            return;
        };

        let bookmarks: usize = plans.iter().map(|p| p.list.bookmarks.len()).sum();
        if bookmarks == 0 {
            self.show_notice(crate::tr!(
                "Нет закладок для нарезки",
                "No bookmarks to cut"
            ));
            return;
        }

        // Пауза на время нарезки. Несколько процессов FFmpeg занимают диск,
        // и картинка всё равно перестаёт поспевать за звуком — пусть лучше
        // воспроизведение честно стоит, чем идёт рывками.
        self.pause_for_extraction();

        // Ноль в `total` означает «идёт подготовка»: сообщение появляется
        // в том же кадре, в котором нажали кнопку.
        self.extraction.progress = Some(ExtractionProgress {
            done: 0,
            total: 0,
            failed: 0,
        });

        let (sender, receiver) = channel();
        self.extraction.events = Some(receiver);

        let workers = self.worker_count(bookmarks);
        tracing::info!(закладок = bookmarks, потоков = workers, "готовлю нарезку");

        spawn(
            ExtractionRequest {
                source,
                plans,
                reencode: self.settings.fragments.reencode,
                audio_aac: self.settings.fragments.audio_aac,
                audio_index: self.current_audio_index(),
                extension: self.container_for_current_file(),
                workers,
            },
            sender,
        );
    }

    /// Останавливает воспроизведение перед нарезкой.
    fn pause_for_extraction(&mut self) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        if engine.state().paused {
            return;
        }

        if let Err(e) = engine.set_paused(true) {
            tracing::warn!(error = %e, "не удалось поставить паузу перед нарезкой");
        }
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

    /// Тот же активный список, но с единственной закладкой.
    ///
    /// Настройки отрезка — длительность, отступ, папка — берутся у списка:
    /// одиночная нарезка ничем от общей не отличается, кроме числа меток.
    fn plan_single_bookmark(&self, time_ms: i64) -> Vec<ListPlan> {
        let Some(list) = self.current_bookmarks().and_then(|v| v.active()) else {
            return Vec::new();
        };
        let Some(dir) = self.output_dir_for(list) else {
            return Vec::new();
        };
        let Some(bookmark) = list.bookmarks.iter().find(|b| b.time_ms == time_ms) else {
            return Vec::new();
        };

        let mut single = list.clone();
        single.bookmarks = vec![bookmark.clone()];

        vec![ListPlan { list: single, dir }]
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

    /// Забирает вести от фонового потока.
    pub(super) fn poll_extraction(&mut self) {
        let Some(receiver) = self.extraction.events.as_ref() else {
            return;
        };

        let mut events = Vec::new();
        while let Ok(event) = receiver.try_recv() {
            events.push(event);
        }

        if events.is_empty() {
            return;
        }

        for event in events {
            self.apply_extraction_event(event);
        }

        self.finish_extraction_if_done();
    }

    fn apply_extraction_event(&mut self, event: ExtractionEvent) {
        let Some(progress) = self.extraction.progress.as_mut() else {
            return;
        };

        match event {
            ExtractionEvent::Prepared { total } => {
                progress.total = total;
                tracing::info!(отрезков = total, "нарезка началась");
            }
            ExtractionEvent::Finished(ExtractionOutcome::Done { output, bytes }) => {
                progress.done += 1;
                tracing::info!(?output, мегабайт = bytes / 1_048_576, "отрезок готов");
            }
            ExtractionEvent::Finished(ExtractionOutcome::Failed { output, reason }) => {
                progress.done += 1;
                progress.failed += 1;
                tracing::error!(?output, %reason, "отрезок не вырезан");
            }
            ExtractionEvent::DirectoryFailed => {
                self.extraction.progress = None;
                self.extraction.events = None;
                self.show_notice(crate::tr!(
                    "Не удалось создать папку для отрезков",
                    "Could not create the folder for fragments"
                ));
            }
        }
    }

    fn finish_extraction_if_done(&mut self) {
        let Some(progress) = self.extraction.progress else {
            return;
        };

        if progress.is_preparing() || progress.done < progress.total {
            return;
        }

        self.extraction.progress = None;
        self.extraction.events = None;
        self.show_notice(&summarize(progress));
    }
}

/// Сколько процессов FFmpeg держать одновременно, если в настройках ноль.
const DEFAULT_WORKERS: usize = 3;

fn summarize(progress: ExtractionProgress) -> String {
    if progress.failed == 0 {
        crate::tr!(
            format!("Готово: {} отрезков", progress.total),
            format!("Done: {} fragments", progress.total)
        )
    } else {
        crate::tr!(
            format!(
                "Готово: {} из {}, с ошибками: {}",
                progress.total - progress.failed,
                progress.total,
                progress.failed
            ),
            format!(
                "Done: {} of {}, failed: {}",
                progress.total - progress.failed,
                progress.total,
                progress.failed
            )
        )
    }
}
