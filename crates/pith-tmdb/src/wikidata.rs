//! Русские имена актёров из Wikidata.
//!
//! База фильмов отдаёт имена по-русски только тем, кому их кто-то перевёл:
//! на «Меню» это 17 человек из 34, на свежей картине — 10 из 47. Остальные
//! приходят латиницей.
//!
//! Wikidata знает часть недостающих, и — что важно — ищется по **номеру
//! TMDB** (свойство `P4985`), а не по имени: однофамильцев не спутать
//! и угадывать нечего. Ключа не нужно, данные общественного достояния.
//!
//! Живёт в этом крейте, а не в своём: без номера TMDB запрос бессмыслен,
//! и связка с базой фильмов у него неразрывная.
//!
//! Замер на живых данных: одним запросом на весь состав, 1–2 с;
//! «Меню» — три имени сверх TMDB, свежая картина — ни одного. Отсюда
//! правило: не нашлось — не беда, имя останется латиницей, и его можно
//! поправить руками в окне актёров.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::Deserialize;

use crate::error::{Result, TmdbError};

/// Сколько ждать ответа.
///
/// Больше, чем у базы фильмов: первый за долгое время запрос там холодный
/// и однажды занял 26 с, тогда как повторные укладываются в секунду.
const TIMEOUT: Duration = Duration::from_secs(30);

/// Служба запросов Wikidata.
const ENDPOINT: &str = "https://query.wikidata.org/sparql";

/// Правила Викимедиа требуют представляться осмысленно.
const AGENT: &str = concat!(
    "PithPlayer/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/DjonBeyron/PithPlayer)"
);

/// Сколько номеров спрашиваем за раз.
///
/// Состав ограничен полусотней, так что на фильм выходит один запрос.
/// Предел оставлен на случай длинного сериала: очень длинный запрос
/// служба отклоняет.
const BATCH: usize = 50;

/// Русские имена по номерам TMDB.
///
/// Возвращает только найденное: кого Wikidata не знает, того в ответе нет.
/// Пустой список номеров запроса не делает.
pub fn russian_names(ids: &[i64]) -> Result<BTreeMap<i64, String>> {
    let mut found = BTreeMap::new();

    for chunk in ids.chunks(BATCH) {
        ask(chunk, &mut found)?;
    }

    Ok(found)
}

/// Один запрос на пачку номеров.
fn ask(ids: &[i64], found: &mut BTreeMap<i64, String>) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }

    let url = format!(
        "{ENDPOINT}?format=json&query={}",
        crate::client::encode(&query(ids))
    );

    let answer: Answer = crate::net::agent(TIMEOUT)
        .get(&url)
        .header("User-Agent", AGENT)
        .header("Accept", "application/sparql-results+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(status) => TmdbError::Refused { status },
            other => TmdbError::Network(other.to_string()),
        })?
        .into_body()
        .read_json()
        .map_err(|e| TmdbError::Malformed(e.to_string()))?;

    for row in answer.results.bindings {
        let Ok(id) = row.tmdb.value.parse::<i64>() else {
            continue;
        };

        found.entry(id).or_insert(row.ru.value);
    }

    Ok(())
}

/// Запрос: по номеру TMDB — русская подпись человека.
fn query(ids: &[i64]) -> String {
    let values = ids
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect::<Vec<_>>()
        .join(" ");

    format!(
        "SELECT ?tmdb ?ru WHERE {{ \
         VALUES ?tmdb {{ {values} }} \
         ?person wdt:P4985 ?tmdb . \
         ?person rdfs:label ?ru . FILTER(lang(?ru) = \"ru\") }}"
    )
}

/// Ответ службы запросов.
#[derive(Deserialize)]
struct Answer {
    results: Results,
}

#[derive(Deserialize)]
struct Results {
    bindings: Vec<Row>,
}

#[derive(Deserialize)]
struct Row {
    tmdb: Value,
    ru: Value,
}

#[derive(Deserialize)]
struct Value {
    value: String,
}

#[cfg(test)]
mod tests {
    use super::{query, russian_names};

    #[test]
    fn в_запрос_попадают_все_номера() {
        let text = query(&[1154221, 69483]);

        assert!(text.contains("\"1154221\""));
        assert!(text.contains("\"69483\""));
        assert!(text.contains("P4985"), "ищем именно по номеру TMDB");
        assert!(text.contains("\"ru\""), "берём русскую подпись");
    }

    #[test]
    fn без_номеров_в_сеть_не_ходим() {
        let found = russian_names(&[]).expect("пустой список — не ошибка");

        assert!(found.is_empty());
    }
}
