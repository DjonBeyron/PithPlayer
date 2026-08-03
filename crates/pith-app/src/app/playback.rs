//! Управление воспроизведением: перемотка, пауза, громкость, скорость.
//!
//! Вынесено из основного файла приложения ради предела в 400 строк
//! (CLAUDE.md).

use super::PithApp;

/// Насколько близко к цели считается, что перемотка завершилась.
const SEEK_SETTLED: f64 = 0.6;

impl PithApp {
    /// Разбирает ошибку воспроизведения от mpv.
    ///
    /// Битый файл, отсутствующий кодек, файл удалили во время просмотра —
    /// плеер обязан остаться живым и сказать, что случилось (PLAN.md,
    /// чек-лист этапа 6). Причину mpv не сообщает, поэтому различаем сами:
    /// пропавший с диска файл — самый частый случай.
    pub(super) fn handle_playback_error(&mut self) {
        let missing = self
            .current_path
            .as_ref()
            .is_some_and(|path| !path.exists());

        let message = if missing {
            "Файл пропал с диска"
        } else {
            "Не удалось воспроизвести файл"
        };

        tracing::error!(файл = ?self.current_path, missing, "{message}");

        // Позицию просмотра не трогаем: файл может вернуться (сетевой диск,
        // переподключённая флешка), и досмотреть его нужно с того же места.
        self.report_playback_error(message);
    }

    /// Сообщает о неудаче: всплывашкой и надписью посреди окна.
    pub(super) fn report_playback_error(&mut self, message: &str) {
        self.playback_error = Some(message.to_string());
        self.show_notice(message);
    }

    /// Почему не играет текущий файл.
    pub fn playback_error(&self) -> Option<&str> {
        self.playback_error.as_deref()
    }

    /// Перемотка относительно текущей позиции с замером длительности.
    pub fn seek_relative(&mut self, seconds: f64) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        self.metrics.mark_seek_start();
        self.seek_pending = true;

        if let Err(e) = engine.seek_relative(seconds) {
            tracing::warn!(error = %e, "перемотка не удалась");
            self.seek_pending = false;
        }
    }

    /// Перемотка во время перетаскивания ползунка.
    ///
    /// Команда уходит не по таймеру, а только когда mpv отработал прошлую.
    /// По таймеру запросы копились в очереди: картинка показывала то, что
    /// пользователь проехал секунду назад, и перемотка выглядела рваной.
    /// Промежуточные положения мыши просто отбрасываются — важно последнее.
    pub fn scrub_to(&mut self, seconds: f64) {
        // Показываем желаемое место сразу: пока mpv догоняет, ползунок
        // должен стоять под пальцем, а не прыгать назад.
        self.seek_target = Some(seconds);
        self.scrub_wanted = Some(seconds);
        self.scrubbing = true;

        // На время перетаскивания останавливаем воспроизведение: иначе mpv
        // между перемотками успевает проиграть кусок, и кадр дёргается
        // сам по себе.
        self.pause_for_scrub();
        self.send_pending_scrub();
    }

    /// Отправляет отложенную перемотку, если движок освободился.
    pub(super) fn send_pending_scrub(&mut self) {
        if self.scrub_in_flight {
            return;
        }

        let Some(target) = self.scrub_wanted.take() else {
            return;
        };

        // Мышь стоит на месте — движку там уже нечего делать.
        if self.scrub_sent == Some(target) {
            return;
        }

        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        self.scrub_in_flight = true;
        self.scrub_sent = Some(target);

        if let Err(e) = engine.seek_keyframe(target) {
            tracing::warn!(error = %e, "быстрая перемотка не удалась");
            self.scrub_in_flight = false;
        }
    }

    /// Отмечает, что mpv закончил перемотку и готов к следующей.
    pub(super) fn scrub_finished(&mut self) {
        self.scrub_in_flight = false;
        self.send_pending_scrub();
    }

    /// Ставит воспроизведение на паузу на время перетаскивания.
    fn pause_for_scrub(&mut self) {
        if self.paused_by_scrub {
            return;
        }

        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        if engine.state().paused {
            return;
        }

        if engine.set_paused(true).is_ok() {
            self.paused_by_scrub = true;
        }
    }

    /// Возвращает воспроизведение после перетаскивания.
    pub fn resume_after_scrub(&mut self) {
        self.scrub_wanted = None;
        self.scrub_sent = None;
        self.scrubbing = false;

        if !self.paused_by_scrub {
            return;
        }
        self.paused_by_scrub = false;

        if let Some(engine) = self.engine.as_mut()
            && let Err(e) = engine.set_paused(false)
        {
            tracing::warn!(error = %e, "не удалось продолжить после перемотки");
        }
    }

    /// Позиция, которую показывает интерфейс.
    ///
    /// Пока идёт перемотка, это желаемое место: mpv отвечает не сразу,
    /// и без этого ползунок дёргался — прыгал назад и снова вперёд.
    pub fn display_position(&self) -> f64 {
        if let Some(target) = self.seek_target {
            return target;
        }

        self.engine
            .as_ref()
            .map(|e| e.state().position)
            .unwrap_or_default()
    }

    /// Забывает желаемое место, когда mpv до него добрался.
    ///
    /// Во время перетаскивания позицию не спрашиваем — там всё решает
    /// событие о завершении перемотки.
    pub(super) fn settle_seek_target(&mut self) {
        if self.scrubbing {
            return;
        }

        let Some(target) = self.seek_target else {
            return;
        };

        let Some(engine) = self.engine.as_ref() else {
            self.seek_target = None;
            self.scrub_finished();
            return;
        };

        if (engine.state().position - target).abs() < SEEK_SETTLED {
            self.seek_target = None;
            // Движок дошёл до места — можно отправлять следующее.
            self.scrub_finished();
        }
    }

    /// Перемотка на абсолютную позицию.
    pub fn seek_absolute(&mut self, seconds: f64) {
        self.seek_target = Some(seconds);

        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        self.metrics.mark_seek_start();
        self.seek_pending = true;

        if let Err(e) = engine.seek_absolute(seconds) {
            tracing::warn!(error = %e, "перемотка не удалась");
            self.seek_pending = false;
        }
    }

    pub fn toggle_pause(&mut self) {
        if let Some(engine) = self.engine.as_mut()
            && let Err(e) = engine.toggle_pause()
        {
            tracing::warn!(error = %e, "не удалось переключить паузу");
        }
    }

    pub fn adjust_volume(&mut self, delta: i64) {
        if let Some(engine) = self.engine.as_mut() {
            let target = engine.state().volume + delta;
            if let Err(e) = engine.set_volume(target) {
                tracing::warn!(error = %e, "не удалось изменить громкость");
            }
        }
    }

    pub fn set_volume(&mut self, volume: i64) {
        if let Some(engine) = self.engine.as_mut()
            && let Err(e) = engine.set_volume(volume)
        {
            tracing::warn!(error = %e, "не удалось изменить громкость");
        }
    }

    /// Меняет скорость на `delta` относительно текущей.
    pub fn adjust_speed(&mut self, delta: f64) {
        if let Some(engine) = self.engine.as_mut() {
            let target = engine.state().speed + delta;
            if let Err(e) = engine.set_speed(target) {
                tracing::warn!(error = %e, "не удалось изменить скорость");
            }
        }
    }

    /// Задаёт скорость воспроизведения.
    pub fn set_speed(&mut self, speed: f64) {
        if let Some(engine) = self.engine.as_mut()
            && let Err(e) = engine.set_speed(speed)
        {
            tracing::warn!(error = %e, "не удалось задать скорость");
        }
    }

    /// Возвращает обычную скорость воспроизведения.
    pub fn reset_speed(&mut self) {
        if let Some(engine) = self.engine.as_mut()
            && let Err(e) = engine.set_speed(1.0)
        {
            tracing::warn!(error = %e, "не удалось сбросить скорость");
        }
    }

    /// Диалог выбора файла.
    pub fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Видео и аудио",
                &[
                    "mkv", "mp4", "avi", "mov", "webm", "ts", "m2ts", "m4v", "flv", "wmv", "mpg",
                    "mpeg", "vob", "ogv", "3gp", "mp3", "flac", "wav", "aac", "m4a", "opus",
                ],
            )
            .add_filter("Все файлы", &["*"])
            .pick_file()
        {
            self.open_file(&path.to_string_lossy());
        }
    }
}
