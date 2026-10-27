//! Запись базы CodeEditVideo и правка документа `db/records.json`.
//!
//! Чистая логика без сети — её можно проверить тестами. Поля записи
//! повторяют те, что пишет сам CodeEditVideo (см. `db/records.json`
//! в его репозитории); незнакомые поля документа не трогаются.

use serde_json::{Map, Value, json};

/// Отрезок, каким он уходит в базу.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Реплика (поле `eng`). Пусто — закладка без названия.
    pub text: String,
    /// Актёр «Имя (Роль)». Пусто — актёра не выбирали.
    pub actor: Option<String>,
    /// Транскрипция реплики: `|hiː| |ɪz|`. Пусто — не запрашивали.
    pub ipa: String,
}

/// Итог пакетной записи.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub created: usize,
    /// Номер первой созданной записи (сквозной по базе).
    pub first_number: usize,
    /// Сколько записей ушло без актёра.
    pub without_actor: usize,
}

/// Добавляет записи в документ. Возвращает отчёт.
///
/// Номер — `seq + 1`, но не меньше наибольшего занятого: так повтор
/// после гонки с самим CodeEditVideo не создаст два раза один номер.
pub fn append(
    doc: &mut Value,
    film: &str,
    entries: &[Entry],
    now_ms: u64,
) -> Result<Report, String> {
    let root = doc
        .as_object_mut()
        .ok_or("records.json: корень не объект")?;

    let records = root
        .entry("records")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or("records.json: records не список")?;

    let max_num = records
        .iter()
        .filter_map(|r| r.get("num").and_then(Value::as_u64))
        .max()
        .unwrap_or(0);

    let seq = root.get("seq").and_then(Value::as_u64).unwrap_or(0);
    let first = seq.max(max_num) + 1;

    let records = root
        .get_mut("records")
        .and_then(Value::as_array_mut)
        .ok_or("records.json: records не список")?;

    let mut without_actor = 0;

    for (index, entry) in entries.iter().enumerate() {
        let number = first + index as u64;
        let actor = entry.actor.as_deref().map(str::trim).unwrap_or_default();

        if actor.is_empty() {
            without_actor += 1;
        }

        records.push(record(entry, actor, film, number, now_ms + index as u64));
    }

    let last = first + entries.len().saturating_sub(1) as u64;
    root.insert("seq".into(), json!(last.max(seq)));
    root.insert("savedAt".into(), json!(now_ms));

    Ok(Report {
        created: entries.len(),
        first_number: first as usize,
        without_actor,
    })
}

/// Номера отрезков, которые в базе уже есть: тот же фильм и та же реплика.
///
/// Реплики сравниваются без учёта регистра и лишних пробелов: у записей
/// из CodeEditVideo в конце бывает перевод строки, а на пробелах плеер
/// и редактор расходятся чаще, чем в словах.
pub fn duplicate_indexes(doc: &Value, film: &str, entries: &[Entry]) -> Vec<usize> {
    let film = film.trim();

    let existing: std::collections::HashSet<String> = doc
        .get("records")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|r| r.get("film").and_then(Value::as_str).map(str::trim) == Some(film))
        .filter_map(|r| r.get("eng").and_then(Value::as_str))
        .map(normalize)
        .collect();

    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| !normalize(&e.text).is_empty() && existing.contains(&normalize(&e.text)))
        .map(|(index, _)| index)
        .collect()
}

fn normalize(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn record(entry: &Entry, actor: &str, film: &str, num: u64, at: u64) -> Value {
    let text = entry.text.trim();
    let mut map = Map::new();

    map.insert("id".into(), json!(new_id(at)));
    map.insert("num".into(), json!(num));
    map.insert("createdAt".into(), json!(at));
    map.insert("eng".into(), json!(text));
    for empty in ["rus", "description", "word", "translations", "chat"] {
        map.insert(empty.into(), json!(""));
    }
    map.insert("actor".into(), json!(actor));
    map.insert("film".into(), json!(film));
    map.insert("name".into(), json!(text));
    map.insert("updatedAt".into(), json!(at));
    map.insert("ipaSentence".into(), json!(entry.ipa));
    // Как у записей, пришедших из плеера: в Notion их досылать не нужно.
    map.insert("pushed".into(), json!(true));

    Value::Object(map)
}

/// Идентификатор в манере CodeEditVideo: `c` + время в base36 + 4 знака.
fn new_id(at: u64) -> String {
    use std::hash::{BuildHasher, Hasher};

    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u64(at);

    format!(
        "c{}{}",
        base36(at),
        &base36(hasher.finish() | 1 << 40)[1..5]
    )
}

fn base36(mut n: u64) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();

    while n > 0 {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse();

    String::from_utf8(out).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(text: &str, actor: Option<&str>) -> Entry {
        Entry {
            text: text.into(),
            actor: actor.map(Into::into),
            ipa: "|hiː|".into(),
        }
    }

    fn doc() -> Value {
        json!({
            "records": [{ "id": "a", "num": 45 }],
            "deleted": { "x": 1 },
            "prefs": { "k": 1 },
            "seq": 47,
            "savedAt": 1
        })
    }

    #[test]
    fn номера_идут_после_счётчика_и_чужое_не_трогается() {
        let mut d = doc();
        let report = append(
            &mut d,
            "Фильм: Тест",
            &[entry("Hi", Some("Леон (Leon)")), entry("Yo", None)],
            1000,
        )
        .expect("добавлено");

        assert_eq!(report.first_number, 48);
        assert_eq!(report.created, 2);
        assert_eq!(report.without_actor, 1);
        assert_eq!(d["seq"], 49);
        assert_eq!(d["deleted"]["x"], 1);
        assert_eq!(d["prefs"]["k"], 1);
        assert_eq!(d["records"].as_array().map(Vec::len), Some(3));
        assert_eq!(d["records"][1]["film"], "Фильм: Тест");
        assert_eq!(d["records"][1]["actor"], "Леон (Leon)");
        assert_eq!(d["records"][1]["eng"], "Hi");
    }

    #[test]
    fn номер_не_ниже_занятого_если_счётчик_отстал() {
        let mut d = json!({ "records": [{ "num": 90 }], "seq": 10 });
        let report = append(&mut d, "f", &[entry("a", None)], 5).expect("добавлено");

        assert_eq!(report.first_number, 91);
    }

    #[test]
    fn идентификатор_похож_на_родной() {
        let id = new_id(1_792_858_241_339);

        assert!(id.starts_with("cmvm"), "{id}");
        assert_eq!(id.len(), 13, "{id}");
    }

    #[test]
    fn повтор_ищется_по_фильму_и_реплике() {
        let d = json!({ "records": [
            { "film": "Фильм: А", "eng": "He is nuts \n" },
            { "film": "Фильм: Б", "eng": "Hello" }
        ]});
        let entries = [
            entry("he  is NUTS", None),
            entry("Hello", None),
            entry("New", None),
        ];

        assert_eq!(duplicate_indexes(&d, "Фильм: А", &entries), vec![0]);
        assert_eq!(duplicate_indexes(&d, "Фильм: Б", &entries), vec![1]);
        assert!(duplicate_indexes(&d, "Фильм: В", &entries).is_empty());
    }

    #[test]
    fn чужой_корень_отказ_а_не_паника() {
        assert!(append(&mut json!([1]), "f", &[], 0).is_err());
    }
}
