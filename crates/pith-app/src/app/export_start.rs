//! Запуск выгрузки: сборка строк, подготовка и рабочий поток.
//!
//! Отдельно от состояния окна (`app/export.rs`): здесь всё, что случается
//! по нажатию «Выгрузить», и подготовка, которая идёт заранее — пока
//! человек ещё отвечает на вопрос о названии.

use std::sync::mpsc::channel;

use pith_notion::{Notion, Prepared, Report, Row, Stable};
use pith_store::NotionKnown;

use super::PithApp;
use super::export::{ExportEvent, ExportStage};
use super::export_log::{LogKind, LogLine};
use super::export_run::{Plan, prepare_now, run};
use super::transcription;

/// Итог выгрузки строкой журнала.
pub(super) fn summary(answer: &Result<Report, String>) -> LogLine {
    match answer {
        Ok(report) => {
            let last = report.first_number + report.created.max(1) - 1;

            LogLine::new(
                LogKind::Done,
                crate::tr!(
                    format!(
                        "Готово: строк {}, номера {}–{last}, не принято {}",
                        report.created,
                        report.first_number,
                        report.failed.len()
                    ),
                    format!(
                        "Done: {} rows, numbers {}–{last}, rejected {}",
                        report.created,
                        report.first_number,
                        report.failed.len()
                    )
                ),
            )
        }
        Err(why) => LogLine::new(
            LogKind::Failed,
            crate::tr!(
                format!("Выгрузка не удалась: {why}"),
                format!("The export failed: {why}")
            ),
        ),
    }
}

impl PithApp {
    /// Спрашивает Notion всё нужное для выгрузки, пока открыт вопрос.
    ///
    /// Три запроса — это около трёх секунд, и провести их лучше сейчас:
    /// человек в это время выбирает название и вид картины. К нажатию
    /// «Выгрузить» ответ обычно уже здесь, и строки идут сразу.
    ///
    /// Не успел или не вышло — не беда: выгрузка спросит сама, в своём
    /// потоке. Отказ доступа при этом виден заранее, прямо в окне.
    pub fn prefetch_export(&mut self, ctx: &egui::Context) {
        // Один раз на открытие окна, и только на первом его кадре: раньше
        // взять `Context` дочернего окна негде.
        match self.export.as_mut() {
            Some(dialog) if dialog.prefetch_pending => dialog.prefetch_pending = false,
            _ => return,
        }

        let Some(notion) = Notion::new(&self.settings.notion.token) else {
            return;
        };

        let work = self.settings.notion.work_page.clone();
        let template = self.settings.notion.template_page.clone();
        let known = self.remembered_notion();

        let (sender, receiver) = channel();
        let ctx = ctx.clone();

        if let Some(dialog) = self.export.as_mut() {
            dialog.prepared = None;
            dialog.preparing = Some(receiver);
        }

        std::thread::spawn(move || {
            let _ = sender.send(prepare_now(&notion, known, &work, &template));
            ctx.request_repaint();
        });
    }

    /// Постоянная часть ответов Notion, если она уже узнана.
    ///
    /// Годится только для тех же страниц: сменили ссылку — спросим заново.
    fn remembered_notion(&self) -> Option<Stable> {
        let known = &self.settings.notion.known;

        if !known.fits(
            &self.settings.notion.work_page,
            &self.settings.notion.template_page,
        ) {
            return None;
        }

        let sample = serde_json::from_str(&known.sample)
            .inspect_err(|e| tracing::warn!(error = %e, "заготовку строки не разобрать"))
            .ok()?;

        Some(Stable {
            database: known.database.clone(),
            sample,
            numbered: known.numbered,
        })
    }

    /// Запоминает постоянную часть — чтобы в следующий раз не спрашивать.
    fn remember_notion(&mut self, prepared: &Prepared) {
        let sample = match serde_json::to_string(&prepared.sample) {
            Ok(sample) => sample,
            Err(e) => {
                tracing::warn!(error = %e, "заготовку строки не записать");
                return;
            }
        };

        let known = NotionKnown {
            pages: NotionKnown::key(
                &self.settings.notion.work_page,
                &self.settings.notion.template_page,
            ),
            database: prepared.database.clone(),
            sample,
            numbered: prepared.numbered,
        };

        if self.settings.notion.known == known {
            return;
        }

        self.settings.notion.known = known;
        self.save_settings();

        tracing::info!("постоянная часть ответов Notion запомнена");
    }

    /// Отправляет активный список в Notion.
    ///
    /// Запрос за запросом на строку, всё в отдельном потоке: сеть в потоке
    /// интерфейса подвесила бы плеер на минуты.
    pub fn start_export(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.export.as_ref() else {
            return;
        };
        if dialog.is_working() {
            return;
        }

        let mut rows = self.export_rows();
        if rows.is_empty() {
            self.show_notice(crate::tr!(
                "В списке нет отрезков",
                "The list has no fragments"
            ));
            return;
        }

        let Some(notion) = Notion::new(&self.settings.notion.token) else {
            return;
        };

        let film_name = dialog.film_name();
        let ready = dialog.prepared.clone();
        let sounding = dialog.transcribe;
        let work = self.settings.notion.work_page.clone();
        let template = self.settings.notion.template_page.clone();

        // Снимок известных слов: поток к хранилищу не ходит, а найденное
        // возвращает списком — файл пишет приложение.
        let remembered = self.remembered_notion();
        let known = if sounding {
            self.sounds.snapshot()
        } else {
            Default::default()
        };

        tracing::info!(
            картина = %film_name,
            строк = rows.len(),
            подготовлено = ready.is_some(),
            транскрипция = sounding,
            "выгружаю в Notion"
        );

        // Ответы окна помнятся между запусками — записываем их прямо здесь,
        // не дожидаясь закрытия.
        self.remember_export_answers();
        self.save_settings();

        let (sender, receiver) = channel();
        let ctx = ctx.clone();

        if let Some(dialog) = self.export.as_mut() {
            dialog.stage = ExportStage::Working {
                done: 0,
                total: rows.len(),
            };
            dialog.events = Some(receiver);
            dialog.preparing = None;
        }

        std::thread::spawn(move || {
            let steps = sender.clone();
            let wake = ctx.clone();

            // Первым делом — подготовка, и только потом словари. Оба шага
            // ходят в сеть, но подготовка стоит одного запроса и без сети
            // отказывает за секунду. Словари же спрашивают каждое незнакомое
            // слово, и отказ каждого стоит четырёх секунд: спросив их
            // раньше, мы заставляли человека ждать минуты, чтобы услышать
            // «сети нет». Строки при этом всё равно не создались бы.
            let prepared = match ready {
                Some(prepared) => Ok(prepared),
                None => prepare_now(&notion, remembered, &work, &template),
            };

            let prepared = match prepared {
                Ok(prepared) => prepared,
                Err(why) => {
                    let answer = Err(why);
                    let _ = sender.send(ExportEvent::Log(summary(&answer)));
                    let _ = sender.send(ExportEvent::Finished(Box::new(answer)));
                    ctx.request_repaint();
                    return;
                }
            };

            // Транскрипция до создания строк: без неё строке нечего положить
            // в поле, а переписывать уже созданные строки — лишние запросы.
            if sounding {
                let sounds = sender.clone();
                let ring = ctx.clone();

                let result = transcription::transcribe(&mut rows, known, |done, total, line| {
                    let _ = sounds.send(ExportEvent::Log(line));
                    let _ = sounds.send(ExportEvent::Sound { done, total });
                    ring.request_repaint();
                });

                if result.missing > 0 {
                    tracing::warn!(слов = result.missing, "транскрипции не нашлось");
                }

                let _ = sender.send(ExportEvent::Words(result.fresh));
            }

            let _ = sender.send(ExportEvent::Log(LogLine::new(
                LogKind::Step,
                crate::tr!(
                    format!("Создаю строки: {}", rows.len()),
                    format!("Creating rows: {}", rows.len())
                ),
            )));

            let plan = Plan {
                ready: Some(prepared),
                // Подготовка уже в руках — спрашивать её второй раз незачем.
                known: None,
                work: &work,
                template: &template,
                film_name: &film_name,
                rows: &rows,
            };

            let answer = run(&notion, plan, |done, total| {
                let _ = steps.send(ExportEvent::Step { done, total });
                wake.request_repaint();
            });

            let _ = sender.send(ExportEvent::Log(summary(&answer)));
            let _ = sender.send(ExportEvent::Finished(Box::new(answer)));
            ctx.request_repaint();
        });
    }

    /// Забирает готовую подготовку, если она подоспела.
    pub(super) fn poll_export_prepare(&mut self) {
        let Some(dialog) = self.export.as_mut() else {
            return;
        };
        let Some(receiver) = dialog.preparing.as_ref() else {
            return;
        };
        let Ok(answer) = receiver.try_recv() else {
            return;
        };

        dialog.preparing = None;

        let mut remember = None;

        match answer {
            Ok(prepared) => {
                tracing::debug!(
                    последний_номер = prepared.base,
                    "подготовка к выгрузке готова"
                );

                let sample = prepared.has_sample();
                remember = Some(prepared.clone());

                dialog.log.push(LogLine::new(
                    LogKind::Step,
                    crate::tr!(
                        format!(
                            "База найдена · последний номер {} · заготовка строки {}",
                            prepared.base,
                            if sample {
                                "взята"
                            } else {
                                "не взята"
                            }
                        ),
                        format!(
                            "Database found · last number {} · row sample {}",
                            prepared.base,
                            if sample { "taken" } else { "missing" }
                        )
                    ),
                ));

                dialog.prepared = Some(prepared);
            }
            Err(why) => {
                tracing::warn!(причина = %why, "подготовка к выгрузке не удалась");
                dialog.prepare_failed = Some(why);
            }
        }

        // Заём окна отпущен — можно писать настройки.
        if let Some(prepared) = remember {
            self.remember_notion(&prepared);
        }
    }

    /// Отрезки активного списка в том виде, в каком их ждёт Notion.
    fn export_rows(&self) -> Vec<Row> {
        let Some(list) = self.current_bookmarks().and_then(|video| video.active()) else {
            return Vec::new();
        };

        list.bookmarks
            .iter()
            .enumerate()
            .map(|(index, bookmark)| Row {
                number: index + 1,
                text: bookmark.name.clone().unwrap_or_default(),
                actor: bookmark.actor.clone(),
                // Транскрипцию добывает поток выгрузки: сеть, и медленная.
                sounds: Vec::new(),
            })
            .collect()
    }
}
