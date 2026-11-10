//! Итог выгрузки в том виде, в каком его показывает окно.

use pith_cev::Report;

use super::export::ExportStage;

pub(super) fn finish(answer: Result<Report, String>) -> ExportStage {
    match answer {
        Ok(report) => {
            tracing::info!(
                создано = report.created,
                без_актёра = report.without_actor,
                "выгрузка окончена"
            );
            ExportStage::Done(report)
        }
        Err(why) => {
            tracing::warn!(причина = %why, "выгрузка не удалась");
            ExportStage::Failed(why)
        }
    }
}
