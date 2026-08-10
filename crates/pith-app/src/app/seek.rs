//! Перемотка стрелками: очередь из одного заказа и грубый промежуточный шаг.
//!
//! Раньше каждое нажатие уходило в mpv точной перемоткой. На 4К такая
//! перемотка стоит около 340 мс — она декодирует всё от опорного кадра
//! до нужной миллисекунды, — и десяток быстрых нажатий выстраивался
//! в очередь на три с половиной секунды: пользователь давно отпустил
//! клавишу, а картинка всё догоняла (замер в PLAN.md §6.13).
//!
//! Теперь так. Пока mpv занят, новые нажатия не отправляются — копится
//! только последнее место. А если нажатия идут подряд, промежуточные
//! места берутся по опорному кадру: это десятки миллисекунд вместо сотен,
//! и картинка успевает за клавишей. Точное место доводится одной
//! перемоткой, когда нажатия кончились.

use super::PithApp;

impl PithApp {
    /// Заказывает перемотку на указанное место.
    pub(super) fn request_seek(&mut self, target: f64) {
        // Показываем желаемое место сразу: mpv отвечает не мгновенно,
        // а время под курсором прыгать назад не должно.
        self.seek_target = Some(target);

        // Нажали, пока mpv не ответил на прошлое, — значит жмут подряд,
        // и это место промежуточное.
        if self.key_seek_in_flight {
            self.key_seek_rough = true;
        }

        self.key_seek_wanted = Some(target);
        self.send_pending_seek();
    }

    /// Отправляет накопленный заказ, если движок освободился.
    pub(super) fn send_pending_seek(&mut self) {
        if self.key_seek_in_flight {
            return;
        }

        match self.key_seek_wanted.take() {
            Some(target) => {
                let rough = self.key_seek_rough;
                self.key_seek_rough = false;
                self.key_seek_needs_exact = rough;
                self.send_seek(target, rough);
            }
            // Заказов больше нет. Если последняя перемотка была грубой,
            // доводим до заказанного места точно — иначе плеер остался бы
            // на опорном кадре, а время показывал бы заказанное.
            None => {
                if !self.key_seek_needs_exact {
                    return;
                }
                self.key_seek_needs_exact = false;

                if let Some(target) = self.seek_target {
                    self.send_seek(target, false);
                }
            }
        }
    }

    /// Отдаёт команду движку и заводит замер.
    fn send_seek(&mut self, target: f64, rough: bool) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        self.metrics.mark_seek_start();
        self.seek_pending = true;
        self.key_seek_in_flight = true;

        let sent = if rough {
            engine.seek_keyframe(target)
        } else {
            engine.seek_absolute(target)
        };

        if let Err(e) = sent {
            tracing::warn!(error = %e, "перемотка не удалась");
            self.seek_pending = false;
            self.key_seek_in_flight = false;
        }
    }

    /// mpv готов играть с нового места — можно слать следующий заказ.
    pub(super) fn seek_finished(&mut self) {
        self.key_seek_in_flight = false;
        self.send_pending_seek();
    }

    /// Забывает незаконченную перемотку при смене файла.
    pub(super) fn forget_seek(&mut self) {
        self.key_seek_in_flight = false;
        self.key_seek_wanted = None;
        self.key_seek_rough = false;
        self.key_seek_needs_exact = false;
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
}
