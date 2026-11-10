//! Распознавание текста с кадра для закладок без субтитров.
//!
//! Нет текста субтитров под рукой (нет дорожки или встроены в картинку) —
//! при добавлении закладки берётся кадр выбранной зоны и распознаётся
//! локальным движком. Прогресс виден в строке закладки; не получилось —
//! остаётся название по умолчанию.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

use pith_store::{OcrSettings, Zone};

use super::PithApp;

/// Какая закладка распознаётся: видео, список и время метки.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct JobKey {
    video: String,
    list: String,
    time_ms: i64,
}

/// Весть от рабочего потока.
enum Message {
    Progress(f32, &'static str),
    Done(Result<String, String>),
}

struct Job {
    progress: f32,
    stage: &'static str,
    receiver: Receiver<Message>,
}

/// Состояние моделей распознавания.
pub enum Models {
    /// Не проверяли.
    Unknown,
    Ready,
    Downloading(Receiver<Result<(), String>>),
    Failed(String),
}

pub struct OcrState {
    pub settings: OcrSettings,
    jobs: HashMap<JobKey, Job>,
    pub models: Models,
    /// Идёт выбор зоны: показан слой поверх кадра.
    pub selecting: bool,
    /// Зона, которую рисуют сейчас, пока не сохранили.
    pub draft: Option<Zone>,
    /// Закладки без названия, которым ещё не завели распознавание.
    pub queue: Vec<i64>,
}

impl OcrState {
    pub fn new(settings: OcrSettings) -> Self {
        Self {
            settings,
            jobs: HashMap::new(),
            models: Models::Unknown,
            selecting: false,
            draft: None,
            queue: Vec::new(),
        }
    }
}

impl PithApp {
    pub fn ocr(&mut self) -> &mut OcrState {
        &mut self.ocr
    }

    pub fn save_ocr_settings(&self) {
        self.ocr.settings.save(&self.data_paths);
    }

    /// Папка с моделями: рядом с программой, а если там пусто — в данных.
    pub fn ocr_models_dir(&self) -> PathBuf {
        let beside = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.join("models")));

        match beside {
            Some(dir) if pith_ocr::models_ready(&dir) => dir,
            _ => self.data_paths.root().join("ocr-models"),
        }
    }

    /// Проверяет модели; нет — запускает скачивание в фоне.
    pub fn ensure_ocr_models(&mut self, ctx: &egui::Context) {
        if matches!(self.ocr.models, Models::Downloading(_)) {
            return;
        }

        let dir = self.ocr_models_dir();

        if pith_ocr::models_ready(&dir) {
            self.ocr.models = Models::Ready;
            return;
        }

        let (sender, receiver) = channel();
        self.ocr.models = Models::Downloading(receiver);
        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let _ = sender.send(pith_ocr::download_models(&dir, |_| {}).map_err(|e| e.to_string()));
            ctx.request_repaint();
        });
    }

    /// Включает и выключает распознавание. Включённое готовит модели.
    pub fn set_ocr_enabled(&mut self, ctx: &egui::Context, on: bool) {
        self.ocr.settings.enabled = on;
        self.save_ocr_settings();

        if on {
            self.ensure_ocr_models(ctx);
        }
    }

    /// Начинает выбор зоны. Стартует с сохранённой или нижней полосы.
    pub fn start_zone_selection(&mut self) {
        self.ocr.draft = Some(self.ocr.settings.zone.unwrap_or(Zone::DEFAULT));
        self.ocr.selecting = true;
    }

    /// Сохраняет нарисованную зону и включает распознавание.
    pub fn save_zone(&mut self, ctx: &egui::Context) {
        if let Some(zone) = self.ocr.draft.filter(Zone::is_sane) {
            self.ocr.settings.zone = Some(zone);
            self.ocr.settings.enabled = true;
            self.save_ocr_settings();
            self.ensure_ocr_models(ctx);
        }

        self.ocr.selecting = false;
    }

    /// Убирает зону: распознавание перестаёт работать, пока не выберут новую.
    pub fn reset_zone(&mut self) {
        self.ocr.settings.zone = None;
        self.ocr.draft = Some(Zone::DEFAULT);
        self.save_ocr_settings();
    }

    /// Прогресс распознавания закладки активного списка: доля и стадия.
    pub fn ocr_progress(&self, time_ms: i64) -> Option<(f32, &'static str)> {
        let video = self.current_video_name()?;
        let list = self.current_bookmarks()?.active_list.clone();

        self.ocr
            .jobs
            .get(&JobKey {
                video,
                list,
                time_ms,
            })
            .map(|job| (job.progress, job.stage))
    }

    /// Запускает распознавание для только что добавленной закладки.
    ///
    /// Условия: включено, зона задана, ffmpeg на месте. Иначе — молча
    /// остаётся название по умолчанию.
    pub(super) fn start_ocr(&mut self, ctx: &egui::Context, time_ms: i64) {
        let (Some(zone), Some(path), Some(video)) = (
            self.ocr
                .settings
                .zone
                .filter(|_| self.ocr.settings.is_ready()),
            self.current_path.clone(),
            self.current_video_name(),
        ) else {
            return;
        };
        let Some(list) = self.current_bookmarks().map(|v| v.active_list.clone()) else {
            return;
        };

        if !self.can_extract() {
            self.show_notice(crate::tr!(
                "Для распознавания нужен ffmpeg.exe рядом с плеером",
                "Recognition needs ffmpeg.exe next to the player"
            ));
            return;
        }

        self.ensure_ocr_models(ctx);
        let models = self.ocr_models_dir();
        let key = JobKey {
            video,
            list,
            time_ms,
        };

        let (sender, receiver) = channel();
        self.ocr.jobs.insert(
            key,
            Job {
                progress: 0.05,
                stage: crate::tr!("Кадр…", "Frame…"),
                receiver,
            },
        );

        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let wake = ctx.clone();
            let result = run_job(&path, time_ms, zone, &models, |p, stage| {
                let _ = sender.send(Message::Progress(p, stage));
                wake.request_repaint();
            });
            let _ = sender.send(Message::Done(result));
            ctx.request_repaint();
        });
    }

    /// Забирает вести заданий и скачивания моделей. Каждый кадр.
    pub(super) fn poll_ocr(&mut self, ctx: &egui::Context) {
        // Закладки, поставленные без реплики, — в работу (нужен `ctx`).
        for time_ms in std::mem::take(&mut self.ocr.queue) {
            self.start_ocr(ctx, time_ms);
        }

        if let Models::Downloading(receiver) = &self.ocr.models
            && let Ok(answer) = receiver.try_recv()
        {
            self.ocr.models = match answer {
                Ok(()) => Models::Ready,
                Err(why) => {
                    tracing::warn!(причина = %why, "модели распознавания не скачались");
                    Models::Failed(why)
                }
            };
        }

        let mut finished = Vec::new();

        for (key, job) in &mut self.ocr.jobs {
            while let Ok(message) = job.receiver.try_recv() {
                match message {
                    Message::Progress(p, stage) => {
                        job.progress = p;
                        job.stage = stage;
                    }
                    Message::Done(result) => finished.push((key.clone(), result)),
                }
            }
        }

        for (key, result) in finished {
            self.ocr.jobs.remove(&key);
            self.apply_ocr_result(&key, result);
        }
    }

    /// Кладёт распознанное в название закладки, если его не успели задать.
    fn apply_ocr_result(&mut self, key: &JobKey, result: Result<String, String>) {
        let text = match result {
            Ok(text) if !text.trim().is_empty() => text,
            Ok(_) => {
                self.show_notice(crate::tr!(
                    "Текст на кадре не найден",
                    "No text found in the frame"
                ));
                return;
            }
            Err(why) => {
                tracing::warn!(причина = %why, "распознавание не удалось");
                self.show_notice(crate::tr!("Распознать не вышло", "Recognition failed"));
                return;
            }
        };

        let duration = self.settings.fragments.duration_sec;
        let buffer = self.settings.fragments.buffer_sec;
        let video = self.bookmarks.for_video_mut(&key.video, duration, buffer);

        let Some(bookmark) = video
            .find_mut(&key.list)
            .and_then(|list| list.bookmarks.iter_mut().find(|b| b.time_ms == key.time_ms))
        else {
            return;
        };

        // Название уже дали руками, пока шло распознавание, — не трогаем.
        if bookmark
            .name
            .as_deref()
            .is_some_and(|n| !n.trim().is_empty())
        {
            return;
        }

        bookmark.name = Some(text.clone());
        self.bookmarks.save();
        self.warm_words(&text);
        self.show_notice(&crate::tr!(
            format!("Распознано: {text}"),
            format!("Recognized: {text}")
        ));
    }
}

/// Кадр зоны и его распознавание. Выполняется в отдельном потоке.
fn run_job(
    video: &std::path::Path,
    time_ms: i64,
    zone: Zone,
    models: &std::path::Path,
    progress: impl Fn(f32, &'static str),
) -> Result<String, String> {
    let png = std::env::temp_dir().join(format!("pith_ocr_{}_{time_ms}.png", std::process::id()));

    progress(0.15, crate::tr!("Кадр…", "Frame…"));

    let region = pith_fragments::Region {
        x: zone.x,
        y: zone.y,
        w: zone.w,
        h: zone.h,
    };
    pith_fragments::capture_region(video, time_ms as f64 / 1000.0, region, &png)?;

    progress(0.55, crate::tr!("Распознаю…", "Recognizing…"));

    let text = pith_ocr::recognize(models, &png).map_err(|e| e.to_string());
    let _ = std::fs::remove_file(&png);

    text
}
