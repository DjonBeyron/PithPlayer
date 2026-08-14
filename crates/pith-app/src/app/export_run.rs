//! Рабочая часть выгрузки: то, что выполняется в отдельном потоке.
//!
//! Отдельно от состояния окна (`app/export.rs`): здесь нет ни полей
//! диалога, ни `PithApp` — только запросы к Notion и разбор их итога.
//! Сеть в потоке интерфейса подвесила бы плеер на десятки секунд.

use pith_notion::{Notion, Prepared, Report, Row, Stable, discover, prepare_from};

use super::export::ExportStage;

/// Итог работы в том виде, в каком его показывает окно.
pub(super) fn finish(answer: Result<Report, String>) -> ExportStage {
    match answer {
        Ok(report) => {
            tracing::info!(
                создано = report.created,
                без_актёра = report.without_actor,
                не_принято = report.failed.len(),
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

/// Куда и с чем идти в Notion.
///
/// Отдельным набором, а не россыпью доводов: их накопилось столько, что
/// перепутать местами два соседних `&str` стало проще простого.
pub(super) struct Plan<'a> {
    /// Ответы Notion, добытые заранее, пока было открыто окно.
    pub(super) ready: Option<Prepared>,
    /// Постоянная часть, узнанная в прошлый раз: с ней подготовка дешевле.
    pub(super) known: Option<Stable>,
    pub(super) work: &'a str,
    pub(super) template: &'a str,
    pub(super) film_name: &'a str,
    pub(super) rows: &'a [Row],
}

/// Сама выгрузка.
///
/// Подготовки нет (не успела или не вышла) — спрашиваем здесь же.
pub(super) fn run(
    notion: &Notion,
    plan: Plan<'_>,
    // `Send`: строки пишутся в несколько потоков, и о готовой строке
    // сообщает тот поток, который её дописал.
    progress: impl FnMut(usize, usize) + Send,
) -> Result<Report, String> {
    let prepared = match plan.ready {
        Some(prepared) => prepared,
        None => prepare_now(notion, plan.known, plan.work, plan.template)?,
    };

    Ok(pith_notion::export(
        notion,
        &prepared,
        plan.film_name,
        plan.rows,
        progress,
    ))
}

/// Подготовка: номер базы, заготовка строки и последний занятый номер.
///
/// `known` — постоянная часть, узнанная в прошлый раз. С ней остаётся
/// один запрос вместо пяти: последний номер спрашивать приходится всегда,
/// а всё остальное меняется только вместе со ссылками на страницы.
pub(super) fn prepare_now(
    notion: &Notion,
    known: Option<Stable>,
    work: &str,
    template: &str,
) -> Result<Prepared, String> {
    if let Some(stable) = known {
        tracing::debug!("подготовка из памяти — спрошу только последний номер");

        return Ok(prepare_from(notion, stable));
    }

    let work = page_id(
        work,
        crate::tr!(
            "В ссылке на страницу с базой нет номера",
            "The database page link has no id"
        ),
    )?;
    let template = page_id(
        template,
        crate::tr!(
            "В ссылке на страницу образца нет номера",
            "The template page link has no id"
        ),
    )?;

    let stable = discover(notion, &work, &template).map_err(|e| e.to_string())?;

    Ok(prepare_from(notion, stable))
}

/// Номер страницы из ссылки.
fn page_id(link: &str, complaint: &str) -> Result<String, String> {
    pith_notion::parse_id(link).ok_or_else(|| complaint.to_string())
}
