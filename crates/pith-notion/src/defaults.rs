//! Значения строки-образца — заготовка для новых строк.
//!
//! Копию страницы Notion через API сделать нельзя, поэтому «дублировать
//! первую строку» приходится иначе: у образца читаются значения его
//! единственной строки, и с них начинается каждая новая. Так в копию
//! попадает то, что в образце проставлено заранее, — `STATUS` со значением
//! «Созданы» (на него смотрит обратная синхронизация) и разделитель
//! в `TRANSCRIPTION SENTENCE`.
//!
//! Свои поля (заголовок, реплика, актёр, название картины) заготовка
//! не занимает: их заполняет `row::properties` поверх.
//!
//! Прочитанное значение и записываемое — не одно и то же: Notion отдаёт
//! варианты списков с номерами, а номера у копии свои, и текст приходит
//! с оформлением и `plain_text`, которого при записи быть не должно.
//! Поэтому значения не переносятся дословно, а пересобираются.

use serde_json::{Map, Value, json};

use crate::row;

/// Свойства, которые Notion ведёт сам: задать их нельзя.
const READ_ONLY: &[&str] = &[
    "formula",
    "rollup",
    "created_time",
    "created_by",
    "last_edited_time",
    "last_edited_by",
    "unique_id",
    "title",
];

/// Свойства, которые плеер заполняет сам.
const OURS: &[&str] = &[
    row::TITLE,
    row::ENG,
    row::ACTOR,
    row::FILM_NAME,
    row::NUMBER,
];

/// Переводит строку-образец в заготовку для новых строк.
pub fn from_row(source: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();

    for (name, property) in source {
        if OURS.contains(&name.as_str()) {
            continue;
        }

        let Some(kind) = property.get("type").and_then(Value::as_str) else {
            continue;
        };

        if READ_ONLY.contains(&kind) {
            continue;
        }

        if let Some(value) = writable(kind, property.get(kind)) {
            out.insert(name.clone(), json!({ kind: value }));
        }
    }

    out
}

/// Значение свойства в том виде, в каком Notion примет его обратно.
///
/// `None` означает «переносить нечего»: пустое поле или тип, который
/// в копии не на что нацелить.
fn writable(kind: &str, value: Option<&Value>) -> Option<Value> {
    let value = value?;

    match kind {
        // Оформление и `plain_text` при записи не нужны — остаётся текст.
        "rich_text" => {
            let text = plain_text(value);

            (!text.is_empty()).then(|| json!([ { "text": { "content": text } } ]))
        }
        // Вариант списка называем по имени: номера у копии свои.
        "select" | "status" => {
            let name = value.get("name")?.as_str()?;

            Some(json!({ "name": name }))
        }
        "multi_select" => {
            let names: Vec<Value> = value
                .as_array()?
                .iter()
                .filter_map(|option| option.get("name")?.as_str())
                .map(|name| json!({ "name": name }))
                .collect();

            (!names.is_empty()).then_some(json!(names))
        }
        // Связи и вложения нацелены на чужую базу и чужие файлы.
        "relation" | "rollup" | "files" | "people" => None,
        // Остальное — число, галочка, дата, ссылка — переносится как есть.
        _ => (!value.is_null()).then(|| value.clone()),
    }
}

/// Склеивает текст Notion — он приходит кусками с оформлением.
fn plain_text(value: &Value) -> String {
    let Some(parts) = value.as_array() else {
        return String::new();
    };

    parts
        .iter()
        .filter_map(|part| part.get("plain_text").and_then(Value::as_str))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::from_row;
    use serde_json::{Map, Value, json};

    fn строка(value: Value) -> Map<String, Value> {
        value.as_object().expect("объект").clone()
    }

    #[test]
    fn список_переносится_по_имени_без_номера() {
        let source = строка(json!({
            "STATUS": {
                "type": "select",
                "select": { "id": "018c1437", "name": "Созданы", "color": "gray" }
            }
        }));

        let defaults = from_row(&source);

        assert_eq!(defaults["STATUS"]["select"]["name"], "Созданы");
        assert!(
            defaults["STATUS"]["select"].get("id").is_none(),
            "номер варианта принадлежит образцу"
        );
    }

    #[test]
    fn текст_переносится_без_оформления() {
        let source = строка(json!({
            "TRANSCRIPTION SENTENCE": {
                "type": "rich_text",
                "rich_text": [ {
                    "type": "text",
                    "text": { "content": "////////////////" },
                    "annotations": { "bold": true },
                    "plain_text": "////////////////"
                } ]
            }
        }));

        let defaults = from_row(&source);
        let text = &defaults["TRANSCRIPTION SENTENCE"]["rich_text"][0];

        assert_eq!(text["text"]["content"], "////////////////");
        assert!(
            text.get("plain_text").is_none(),
            "при записи его быть не должно"
        );
    }

    #[test]
    fn пустые_поля_не_переносятся() {
        let source = строка(json!({
            "RUS": { "type": "rich_text", "rich_text": [] },
            "ACTOR2": { "type": "multi_select", "multi_select": [] }
        }));

        assert!(from_row(&source).is_empty(), "пустое поле слать незачем");
    }

    #[test]
    fn вычисляемое_и_служебное_пропускается() {
        let source = строка(json!({
            "Copy": { "type": "formula", "formula": { "type": "string", "string": "1. [ ]" } },
            "Last edited time": { "type": "last_edited_time", "last_edited_time": "2026-06-17" },
            "": { "type": "title", "title": [ { "plain_text": "CARD (1)" } ] }
        }));

        assert!(from_row(&source).is_empty(), "эти свойства ведёт Notion");
    }

    #[test]
    fn свои_поля_заготовка_не_занимает() {
        let source = строка(json!({
            "FILM NAME": {
                "type": "rich_text",
                "rich_text": [ { "plain_text": "Фильм:  Сериал:" } ]
            },
            " ENG": { "type": "rich_text", "rich_text": [ { "plain_text": "образец" } ] },
            "ACTOR": {
                "type": "multi_select",
                "multi_select": [ { "id": "1", "name": "Кто-то" } ]
            }
        }));

        assert!(
            from_row(&source).is_empty(),
            "название картины, реплику и актёра кладёт плеер"
        );
    }

    #[test]
    fn галочка_и_число_переносятся_как_есть() {
        let source = строка(json!({
            "Готово": { "type": "checkbox", "checkbox": true },
            "Вес": { "type": "number", "number": 7 }
        }));

        let defaults = from_row(&source);

        assert_eq!(defaults["Готово"]["checkbox"], true);
        assert_eq!(defaults["Вес"]["number"], 7);
    }
}
