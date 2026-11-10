//! Запуск выгрузки: сборка строк активного списка и передача рабочему потоку.
//!
//! Отдельно от состояния окна (`app/export.rs`). Сама работа с базой
//! CodeEditVideo — в `app/export_cev.rs`.

use pith_cev::{Report, Row};

use super::PithApp;
use super::export_cev::DupPolicy;
use super::export_log::{LogKind, LogLine};

/// Итог выгрузки строкой журнала.
pub(super) fn summary(answer: &Result<Report, String>) -> LogLine {
    match answer {
        Ok(report) => {
            let last = report.first_number + report.created.max(1) - 1;

            LogLine::new(
                LogKind::Done,
                crate::tr!(
                    format!(
                        "Готово: записей {}, номера {}–{last}",
                        report.created, report.first_number
                    ),
                    format!(
                        "Done: {} records, numbers {}–{last}",
                        report.created, report.first_number
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
    /// Отправляет активный список в базу CodeEditVideo.
    ///
    /// Всё в отдельном потоке: сеть в потоке интерфейса подвесила бы плеер.
    pub fn start_export(&mut self, ctx: &egui::Context) {
        self.start_export_with(ctx, DupPolicy::Ask);
    }

    /// То же с решением о повторах в базе.
    pub fn start_export_with(&mut self, ctx: &egui::Context, policy: DupPolicy) {
        let Some(dialog) = self.export.as_ref() else {
            return;
        };
        if dialog.is_working() {
            return;
        }

        let rows = self.export_rows();
        if rows.is_empty() {
            self.show_notice(crate::tr!(
                "В списке нет отрезков",
                "The list has no fragments"
            ));
            return;
        }

        self.start_export_cev(ctx, rows, policy);
    }

    /// Отрезки активного списка в том виде, в каком их ждёт база.
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
