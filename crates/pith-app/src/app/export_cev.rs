//! Выгрузка в базу CodeEditVideo (GitHub) вместо Notion.
//!
//! Тот же путь, что у Notion: строки списка, транскрипция, поток с отчётом
//! в окно выгрузки. Отличается только назначение — файл `db/records.json`
//! в репозитории проектов (крейт `pith-cev`).

use std::sync::mpsc::channel;

use pith_cev::{Cev, Entry};
use pith_notion::{Report, Row};

use super::PithApp;
use super::export::{ExportEvent, ExportStage};
use super::export_log::{LogKind, LogLine};
use super::export_start::summary;
use super::transcription;

impl PithApp {
    /// Подключено ли то место, куда сейчас идёт выгрузка.
    pub(super) fn export_target_ready(&self) -> bool {
        if self.settings.cev.enabled {
            self.settings.cev.is_ready()
        } else {
            self.settings.notion.is_ready()
        }
    }

    /// Фраза «сначала подключите…» для текущего места выгрузки.
    pub(super) fn export_target_missing(&self) -> &'static str {
        if self.settings.cev.enabled {
            crate::tr!("Сначала укажите токен GitHub", "Set the GitHub token first")
        } else {
            crate::tr!("Сначала подключите Notion", "Connect Notion first")
        }
    }

    /// Отправляет активный список в CodeEditVideo. Сеть — в отдельном потоке.
    pub(super) fn start_export_cev(&mut self, ctx: &egui::Context, mut rows: Vec<Row>) {
        let Some(cev) = Cev::new(&self.settings.cev.token) else {
            return;
        };
        let Some(dialog) = self.export.as_ref() else {
            return;
        };

        let film_name = dialog.film_name();
        let sounding = dialog.transcribe;
        let known = if sounding {
            self.sounds.snapshot()
        } else {
            Default::default()
        };

        tracing::info!(картина = %film_name, строк = rows.len(), "выгружаю в CodeEditVideo");

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
            if sounding {
                let sounds = sender.clone();
                let ring = ctx.clone();

                let result = transcription::transcribe(&mut rows, known, |done, total, line| {
                    let _ = sounds.send(ExportEvent::Log(line));
                    let _ = sounds.send(ExportEvent::Sound { done, total });
                    ring.request_repaint();
                });

                let _ = sender.send(ExportEvent::Words(result.fresh));
            }

            let _ = sender.send(ExportEvent::Log(LogLine::new(
                LogKind::Step,
                crate::tr!(
                    format!("Записываю в CodeEditVideo: {}", rows.len()),
                    format!("Writing to CodeEditVideo: {}", rows.len())
                ),
            )));

            let entries: Vec<Entry> = rows.iter().map(entry).collect();
            let total = entries.len();

            let answer = cev
                .push(&film_name, &entries)
                .map(|done| Report {
                    // Пусто — окно покажет ссылку на репозиторий, а не Notion.
                    database_id: String::new(),
                    created: done.created,
                    without_actor: done.without_actor,
                    failed: Vec::new(),
                    sample_taken: true,
                    first_number: done.first_number,
                })
                .map_err(|e| e.to_string());

            let _ = sender.send(ExportEvent::Step { done: total, total });
            let _ = sender.send(ExportEvent::Log(summary(&answer)));
            let _ = sender.send(ExportEvent::Finished(Box::new(answer)));
            ctx.request_repaint();
        });
    }
}

/// Строка списка в виде записи базы: реплика, актёр и транскрипция.
fn entry(row: &Row) -> Entry {
    Entry {
        text: row.text.clone(),
        actor: row.actor.clone(),
        ipa: row
            .sounds
            .iter()
            .map(|sound| sound.transcription.as_str())
            .collect::<Vec<_>>()
            .join(" "),
    }
}
