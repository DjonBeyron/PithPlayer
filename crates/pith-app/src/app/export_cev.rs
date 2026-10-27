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

/// Что делать с отрезками, которые в базе уже есть.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DupPolicy {
    /// Спросить, если повторы найдены.
    Ask,
    /// Не выгружать повторы.
    Skip,
    /// Выгрузить всё, как есть.
    Keep,
}

/// Сколько примеров повторов показывать в предупреждении.
const SAMPLES: usize = 5;

impl PithApp {
    /// Вернуться от предупреждения о повторах к вопросу о названии.
    pub fn export_back_to_question(&mut self) {
        if let Some(dialog) = self.export.as_mut() {
            dialog.stage = ExportStage::Asking;
        }
    }

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
    pub(super) fn start_export_cev(
        &mut self,
        ctx: &egui::Context,
        mut rows: Vec<Row>,
        policy: DupPolicy,
    ) {
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
            if policy != DupPolicy::Keep {
                match check_duplicates(&cev, &film_name, &mut rows, policy) {
                    Ok(Some((count, samples))) => {
                        let _ = sender.send(ExportEvent::Duplicates { count, samples });
                        ctx.request_repaint();
                        return;
                    }
                    Ok(None) => {}
                    Err(why) => {
                        let answer = Err(why);
                        let _ = sender.send(ExportEvent::Log(summary(&answer)));
                        let _ = sender.send(ExportEvent::Finished(Box::new(answer)));
                        ctx.request_repaint();
                        return;
                    }
                }
            }

            if rows.is_empty() {
                let answer = Ok(empty_report());
                let _ = sender.send(ExportEvent::Log(summary(&answer)));
                let _ = sender.send(ExportEvent::Finished(Box::new(answer)));
                ctx.request_repaint();
                return;
            }

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

/// Проверка повторов. `Some` — нужно спросить человека; при `Skip` повторы
/// молча убираются из списка.
fn check_duplicates(
    cev: &Cev,
    film: &str,
    rows: &mut Vec<Row>,
    policy: DupPolicy,
) -> Result<Option<(usize, Vec<String>)>, String> {
    let entries: Vec<Entry> = rows.iter().map(entry).collect();
    let dups = cev.duplicates(film, &entries).map_err(|e| e.to_string())?;

    if dups.is_empty() {
        return Ok(None);
    }

    if policy == DupPolicy::Ask {
        let samples = dups
            .iter()
            .take(SAMPLES)
            .map(|&i| rows[i].text.trim().to_string())
            .collect();

        return Ok(Some((dups.len(), samples)));
    }

    let mut index = 0;
    rows.retain(|_| {
        let keep = !dups.contains(&index);
        index += 1;
        keep
    });

    Ok(None)
}

/// Отчёт, когда после пропуска повторов выгружать нечего.
fn empty_report() -> Report {
    Report {
        database_id: String::new(),
        created: 0,
        without_actor: 0,
        failed: Vec::new(),
        sample_taken: true,
        first_number: 0,
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
