//! Замеры производительности для этапа 0.
//!
//! Цель — сравнить режимы `hwdec` на реальных 4К-файлах и зафиксировать
//! выбор в PLAN.md §3. Без цифр решение не принимается (CLAUDE.md).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Сколько последних кадров усредняем.
const FRAME_WINDOW: usize = 120;

#[derive(Default)]
pub struct Metrics {
    open_started: Option<Instant>,
    /// Время от команды открытия до первого показанного кадра.
    pub time_to_first_frame: Option<Duration>,

    seek_started: Option<Instant>,
    /// Длительность последней перемотки.
    pub last_seek: Option<Duration>,

    last_frame_at: Option<Instant>,
    frame_times: VecDeque<Duration>,
    /// Всего отрисовано кадров с момента запуска.
    pub frames_rendered: u64,
}

impl Metrics {
    /// Начало открытия файла.
    pub fn mark_open_start(&mut self) {
        self.open_started = Some(Instant::now());
        self.time_to_first_frame = None;
        self.last_frame_at = None;
        self.frame_times.clear();
    }

    /// Первый кадр показан. Повторные вызовы игнорируются.
    pub fn mark_first_frame(&mut self) {
        if self.time_to_first_frame.is_some() {
            return;
        }
        if let Some(started) = self.open_started {
            let elapsed = started.elapsed();
            self.time_to_first_frame = Some(elapsed);
            tracing::info!(ms = elapsed.as_millis(), "время до первого кадра");
        }
    }

    pub fn mark_seek_start(&mut self) {
        self.seek_started = Some(Instant::now());
    }

    /// Перемотка завершена — позиция изменилась и кадр перерисован.
    pub fn mark_seek_done(&mut self) {
        if let Some(started) = self.seek_started.take() {
            let elapsed = started.elapsed();
            self.last_seek = Some(elapsed);
            tracing::info!(ms = elapsed.as_millis(), "перемотка завершена");
        }
    }

    /// Отмечает отрисовку кадра.
    pub fn record_frame(&mut self) {
        self.frames_rendered += 1;

        let now = Instant::now();
        if let Some(previous) = self.last_frame_at {
            self.frame_times.push_back(now - previous);
            if self.frame_times.len() > FRAME_WINDOW {
                self.frame_times.pop_front();
            }
        }
        self.last_frame_at = Some(now);
    }

    /// Среднее время кадра в миллисекундах.
    pub fn average_frame_ms(&self) -> Option<f64> {
        if self.frame_times.is_empty() {
            return None;
        }
        let total: Duration = self.frame_times.iter().sum();
        Some(total.as_secs_f64() * 1000.0 / self.frame_times.len() as f64)
    }

    /// Кадров в секунду по усреднённому времени кадра.
    pub fn fps(&self) -> Option<f64> {
        self.average_frame_ms()
            .filter(|ms| *ms > 0.0)
            .map(|ms| 1000.0 / ms)
    }

    /// Строка для отчёта о замерах — её вписываем в PLAN.md.
    pub fn report(&self, hwdec_label: &str) -> String {
        let first_frame = self
            .time_to_first_frame
            .map(|d| format!("{} мс", d.as_millis()))
            .unwrap_or_else(|| "—".into());

        let seek = self
            .last_seek
            .map(|d| format!("{} мс", d.as_millis()))
            .unwrap_or_else(|| "—".into());

        let frame = self
            .average_frame_ms()
            .map(|ms| format!("{ms:.1} мс"))
            .unwrap_or_else(|| "—".into());

        let fps = self
            .fps()
            .map(|f| format!("{f:.0}"))
            .unwrap_or_else(|| "—".into());

        format!(
            "Режим: {hwdec_label}\n\
             До первого кадра: {first_frame}\n\
             Перемотка: {seek}\n\
             Время кадра: {frame} (~{fps} к/с)\n\
             Кадров отрисовано: {}",
            self.frames_rendered
        )
    }
}
