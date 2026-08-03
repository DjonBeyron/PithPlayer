//! Предпросмотр кадра при перемотке.
//!
//! Пока ползунок под пальцем, полезно видеть, что происходит в том месте
//! фильма. Кадр достаёт FFmpeg отдельным процессом: сам плеер при этом
//! продолжает показывать своё, и картинка не мигает.

use std::sync::mpsc::{Receiver, channel};
use std::time::Instant;

use super::PithApp;

/// Насколько должно измениться время, чтобы просить новый кадр, секунды.
///
/// Пока пользователь ведёт мышь, позиция меняется непрерывно. Каждый
/// кадр — отдельный запуск FFmpeg, и просить их на каждое движение
/// бессмысленно: они не успеют.
const STEP: f64 = 1.5;

/// Как часто вообще можно просить кадр.
///
/// Согласовано с замером: на 4К HEVC кадр достаётся примерно за 220 мс,
/// и просить чаще — значит копить процессы, которые уже не нужны.
const INTERVAL_MS: u128 = 150;

/// Сколько мышь должна простоять на месте, прежде чем просить кадр.
///
/// Пока рука ведёт ползунок, чтение файла отдельным процессом мешает
/// движку, и перемотка идёт рывками. Кадр нужен там, где пользователь
/// остановился, — его и достаём.
const SETTLE_MS: u128 = 180;

/// Готовый кадр предпросмотра.
pub struct PreviewFrame {
    /// Время, которому кадр соответствует.
    pub time: f64,
    /// Изображение в памяти egui.
    pub texture: egui::TextureHandle,
}

/// Состояние предпросмотра.
#[derive(Default)]
pub struct PreviewState {
    /// Показанный кадр.
    frame: Option<PreviewFrame>,
    /// Время, для которого кадр уже заказан.
    requested: Option<f64>,
    /// Когда заказывали в последний раз.
    last_request: Option<Instant>,
    /// Место, для которого кадр нужен, но запрос ещё не ушёл.
    wanted: Option<f64>,
    /// Когда это место было выбрано — от него отсчитывается пауза.
    wanted_since: Option<Instant>,
    /// Ответ фонового потока: время и картинка в PNG.
    result: Option<Receiver<(f64, Vec<u8>)>>,
}

impl PithApp {
    /// Кадр предпросмотра, если он готов.
    pub fn preview_frame(&self) -> Option<&PreviewFrame> {
        self.preview.frame.as_ref()
    }

    /// Отмечает место, для которого нужен кадр.
    ///
    /// Сам FFmpeg не запускается: пока мышь в движении, кадр всё равно
    /// устареет раньше, чем будет готов, а чтение файла мешает mpv —
    /// именно от этого перемотка «залипала» первые секунды. Запрос уходит,
    /// когда рука остановилась (см. `poll_preview`).
    pub fn request_preview(&mut self, time: f64) {
        if self.preview.wanted != Some(time) {
            self.preview.wanted = Some(time);
            self.preview.wanted_since = Some(Instant::now());
        }
    }

    /// Запускает отложенный запрос, если мышь замерла.
    fn send_pending_preview(&mut self) {
        let (Some(time), Some(since)) = (self.preview.wanted, self.preview.wanted_since) else {
            return;
        };

        if since.elapsed().as_millis() < SETTLE_MS {
            return;
        }

        let Some(path) = self.current_path.clone() else {
            return;
        };

        if !self.worth_requesting(time) {
            return;
        }

        // Пока движок сам перематывает, диск занят им: свой процесс
        // подождёт, иначе оба будут мешать друг другу.
        if self.scrub_in_flight {
            return;
        }

        self.preview.requested = Some(time);
        self.preview.last_request = Some(Instant::now());

        let (sender, receiver) = channel();
        self.preview.result = Some(receiver);

        std::thread::spawn(move || {
            if let Some(png) = pith_fragments::grab_frame(&path, time) {
                let _ = sender.send((time, png));
            }
        });
    }

    /// Убирает предпросмотр: перемотка закончилась.
    pub fn clear_preview(&mut self) {
        self.preview.frame = None;
        self.preview.requested = None;
        self.preview.result = None;
        self.preview.wanted = None;
        self.preview.wanted_since = None;
    }

    /// Забирает готовый кадр и делает из него текстуру.
    pub(super) fn poll_preview(&mut self, ctx: &egui::Context) {
        self.send_pending_preview();

        let Some(receiver) = self.preview.result.as_ref() else {
            return;
        };

        let Ok((time, png)) = receiver.try_recv() else {
            return;
        };

        self.preview.result = None;

        let Ok(image) = image::load_from_memory(&png) else {
            tracing::warn!("кадр предпросмотра не разобрался");
            return;
        };

        let rgba = image.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let pixels = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());

        self.preview.frame = Some(PreviewFrame {
            time,
            texture: ctx.load_texture("preview", pixels, egui::TextureOptions::LINEAR),
        });
    }

    /// Стоит ли просить кадр для этого времени.
    fn worth_requesting(&self, time: f64) -> bool {
        // Прошлый запрос ещё в работе — дождёмся его.
        if self.preview.result.is_some() {
            return false;
        }

        if let Some(last) = self.preview.last_request
            && last.elapsed().as_millis() < INTERVAL_MS
        {
            return false;
        }

        // Уже показываем кадр примерно того же места.
        let shown = self.preview.frame.as_ref().map(|f| f.time);
        let requested = self.preview.requested;

        !matches!(shown.or(requested), Some(known) if (known - time).abs() < STEP)
    }
}
