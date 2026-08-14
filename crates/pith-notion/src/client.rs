//! Запросы к Notion.
//!
//! Все блокирующие — вызывать полагается в отдельном потоке.

use std::time::Duration;

use serde_json::{Map, Value, json};

use crate::error::{NotionError, Result};
use crate::id;

const HOST: &str = "https://api.notion.com/v1";

/// Версия API, на которую рассчитан разбор ответов.
const VERSION: &str = "2022-06-28";

/// Сколько ждать ответа.
const TIMEOUT: Duration = Duration::from_secs(20);

/// Сколько новейших строк смотреть, разыскивая наибольший номер.
///
/// Одной хватило бы: строки создаются по возрастанию. Берём несколько
/// на случай, если в базу что-то добавили руками не по порядку.
const NEWEST_ROWS: usize = 5;

/// Доступ к Notion.
pub struct Notion {
    token: String,
}

impl Notion {
    /// `None`, если токен не задан.
    pub fn new(token: &str) -> Option<Self> {
        let token = token.trim();

        (!token.is_empty()).then(|| Self {
            token: token.into(),
        })
    }

    /// Номер базы, лежащей внутри страницы.
    ///
    /// Так устроены карточки фильмов: страница с названием, а в ней база.
    pub fn database_in_page(&self, page_id: &str) -> Result<String> {
        let url = format!(
            "{HOST}/blocks/{}/children?page_size=100",
            id::dashed(page_id)
        );
        let answer: Value = self.get(&url)?;

        answer
            .get("results")
            .and_then(Value::as_array)
            .and_then(|blocks| {
                blocks.iter().find_map(|block| {
                    (block.get("type")?.as_str()? == "child_database")
                        .then(|| block.get("id")?.as_str().map(String::from))
                        .flatten()
                })
            })
            .ok_or(NotionError::NoTemplate)
    }

    /// Свойства первой строки базы.
    ///
    /// В образце строка одна, и в ней проставлено то, что должно быть
    /// у каждой новой: `STATUS`, разделитель в расшифровке. Копию строки
    /// API не делает, поэтому её значения читаются и кладутся в заготовку.
    ///
    /// `None`, если строк в базе нет.
    pub fn first_row(&self, database_id: &str) -> Result<Option<Map<String, Value>>> {
        let url = format!("{HOST}/databases/{}/query", id::dashed(database_id));
        let answer: Value = self.post(&url, &json!({ "page_size": 1 }))?;

        Ok(answer
            .get("results")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("properties"))
            .and_then(Value::as_object)
            .cloned())
    }

    /// Наибольший номер среди заголовков строк базы.
    ///
    /// База одна на все картины, и нумерация в ней сквозная: новые строки
    /// продолжают счёт, а не начинают его заново. Иначе в базе оказывается
    /// по нескольку строк с заголовком «1», и найти отрезок по номеру
    /// нельзя. Одинаковые реплики при этом в порядке вещей — совпадать
    /// не должны только номера.
    ///
    /// Спрашиваем **одним** запросом: самые новые сверху, и смотрим
    /// на горстку. Строки всегда создаются по возрастанию номера, значит
    /// наибольший — у одной из последних созданных. Обход всей базы стоил
    /// бы запрос на сотню строк: на девятистах — девять запросов и семь
    /// секунд ожидания перед каждой выгрузкой.
    ///
    /// Заголовки, которые числом не читаются (у образца это `CARD (1)`),
    /// пропускаются. Пустая база даёт ноль — счёт начнётся с единицы.
    pub fn max_number(&self, database_id: &str) -> Result<usize> {
        let url = format!("{HOST}/databases/{}/query", id::dashed(database_id));

        let body = json!({
            "page_size": NEWEST_ROWS,
            "sorts": [ { "timestamp": "created_time", "direction": "descending" } ]
        });

        let answer: Value = self.post(&url, &body)?;

        let largest = answer
            .get("results")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| {
                        row.get("properties")
                            .and_then(Value::as_object)
                            .and_then(row_number)
                    })
                    .max()
                    .unwrap_or(0)
            })
            .unwrap_or(0);

        Ok(largest)
    }

    /// Следит, чтобы в базе было числовое поле номера.
    ///
    /// По нему сортируется вид, и без него порядок строк держится только
    /// на порядке создания — а значит строки нельзя писать в несколько
    /// потоков. Свойство добавляется один раз: дальше запрос лишь убеждается,
    /// что оно на месте.
    ///
    /// Возвращает `true`, если поле есть и в него можно писать. `false` —
    /// не вышло: строки уйдут без номера-числа, но уйдут.
    pub fn ensure_number_property(&self, database_id: &str, name: &str) -> Result<bool> {
        let url = format!("{HOST}/databases/{}", id::dashed(database_id));
        let database: Value = self.get(&url)?;

        let existing = database
            .get("properties")
            .and_then(Value::as_object)
            .and_then(|props| props.get(name));

        if let Some(property) = existing {
            let numeric = property.get("type").and_then(Value::as_str) == Some("number");

            if !numeric {
                tracing::warn!(поле = name, "в базе занято не числом — номер не пишем");
            }

            return Ok(numeric);
        }

        tracing::info!(поле = name, "добавляю в базу числовое поле номера");

        let body = json!({ "properties": { name: { "number": {} } } });
        let _: Value = self.patch(&url, &body)?;

        Ok(true)
    }

    /// Заводит строку в базе.
    pub fn create_row(&self, database_id: &str, properties: Map<String, Value>) -> Result<()> {
        let body = json!({
            "parent": { "database_id": id::dashed(database_id) },
            "properties": properties
        });

        let _: Value = self.post(&format!("{HOST}/pages"), &body)?;
        Ok(())
    }

    /// Проверяет, что токен рабочий и страница видна интеграции.
    pub fn check_access(&self, page_id: &str) -> Result<()> {
        let url = format!("{HOST}/pages/{}", id::dashed(page_id));
        let _: Value = self.get(&url)?;

        Ok(())
    }

    fn agent(&self) -> ureq::Agent {
        crate::net::agent(TIMEOUT)
    }

    fn get(&self, url: &str) -> Result<Value> {
        let response = self
            .agent()
            .get(url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Notion-Version", VERSION)
            .call()
            .map_err(convert)?;

        read_answer(response)
    }

    fn patch(&self, url: &str, body: &Value) -> Result<Value> {
        let response = self
            .agent()
            .patch(url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Notion-Version", VERSION)
            .header("Content-Type", "application/json")
            .send_json(body)
            .map_err(convert)?;

        read_answer(response)
    }

    fn post(&self, url: &str, body: &Value) -> Result<Value> {
        let response = self
            .agent()
            .post(url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Notion-Version", VERSION)
            .header("Content-Type", "application/json")
            .send_json(body)
            .map_err(convert)?;

        read_answer(response)
    }
}

/// Разбирает ответ: тело или причина отказа.
///
/// Тело читается строкой, а не сразу деревом: отказ Notion объясняет
/// по-человечески («page not shared with integration»), и потерять это
/// объяснение из-за неожиданного вида ответа нельзя — без него
/// пользователь видит один код.
fn read_answer(response: ureq::http::Response<ureq::Body>) -> Result<Value> {
    let status = response.status();

    let text = response
        .into_body()
        .read_to_string()
        .map_err(|e| NotionError::Network(e.to_string()))?;

    if !status.is_success() {
        return Err(NotionError::Refused {
            status: status.as_u16(),
            message: explain(&text),
        });
    }

    serde_json::from_str(&text).map_err(|e| NotionError::Malformed(e.to_string()))
}

/// Достаёт объяснение отказа. Не JSON — берём само тело, обрезав длинное.
fn explain(text: &str) -> String {
    const LIMIT: usize = 300;

    let message = serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|body| {
            body.get("message")
                .and_then(Value::as_str)
                .map(String::from)
        })
        .unwrap_or_else(|| text.trim().to_string());

    message.chars().take(LIMIT).collect()
}

/// Номер строки из её заголовка.
///
/// Заголовок ищем по типу, а не по имени: в образце он назван пустой
/// строкой, и полагаться на это имя лишний раз незачем.
fn row_number(properties: &Map<String, Value>) -> Option<usize> {
    let title = properties
        .values()
        .find(|value| value.get("type").and_then(Value::as_str) == Some("title"))?
        .get("title")?;

    number_of(&plain_text(title))
}

/// Число из заголовка. `None`, если это не число.
fn number_of(title: &str) -> Option<usize> {
    title.trim().parse().ok()
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

/// Переводит отказ клиента в понятную причину.
///
/// Тело отказа Notion объясняет по-человечески («page not shared with
/// integration»), и терять это объяснение нельзя — без него пользователь
/// видит один код.
fn convert(error: ureq::Error) -> NotionError {
    match error {
        ureq::Error::StatusCode(status) => NotionError::Refused {
            status,
            message: String::new(),
        },
        other => NotionError::Network(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{number_of, row_number};
    use serde_json::{Map, Value, json};

    fn строка(value: Value) -> Map<String, Value> {
        value.as_object().expect("объект").clone()
    }

    #[test]
    fn номер_читается_из_заголовка() {
        assert_eq!(number_of("7"), Some(7));
        assert_eq!(number_of(" 42 "), Some(42));
    }

    #[test]
    fn заголовок_образца_числом_не_читается() {
        assert!(number_of("CARD (1)").is_none(), "строка образца не в счёт");
        assert!(number_of("").is_none());
        assert!(number_of("1.5").is_none());
    }

    #[test]
    fn номер_ищется_по_типу_свойства() {
        // У образца заголовок назван пустой строкой — имени не доверяем.
        let row = строка(json!({
            "": { "type": "title", "title": [ { "plain_text": "12" } ] },
            " ENG": { "type": "rich_text", "rich_text": [ { "plain_text": "34" } ] }
        }));

        assert_eq!(row_number(&row), Some(12));
    }

    #[test]
    fn без_заголовка_номера_нет() {
        let row = строка(json!({
            " ENG": { "type": "rich_text", "rich_text": [ { "plain_text": "5" } ] }
        }));

        assert!(row_number(&row).is_none());
    }
}
