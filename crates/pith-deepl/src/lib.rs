//! Перевод реплик через DeepL.
//!
//! Один запрос на реплику и язык. Ключ бесплатного тарифа заканчивается
//! на `:fx` и ходит на `api-free.deepl.com`, платного — на `api.deepl.com`.

mod net;

use serde_json::{Value, json};

pub use net::use_proxy;

#[derive(Debug, thiserror::Error)]
pub enum DeeplError {
    #[error("DeepL не отвечает: {0}")]
    Network(String),
    #[error("ключ DeepL не подходит (403): проверьте токен")]
    BadKey,
    #[error("лимит символов DeepL исчерпан (456)")]
    QuotaExceeded,
    #[error("слишком много запросов к DeepL (429), подождите немного")]
    TooMany,
    #[error("DeepL ответил отказом ({0}): {1}")]
    Refused(u16, String),
    #[error("ответ DeepL не разобрать: {0}")]
    Malformed(String),
}

type Result<T> = std::result::Result<T, DeeplError>;

/// Расход символов по тарифу.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    pub used: u64,
    pub limit: u64,
}

pub struct Deepl {
    key: String,
}

impl Deepl {
    /// `None` — ключ пуст.
    pub fn new(key: &str) -> Option<Self> {
        let key = key.trim();
        (!key.is_empty()).then(|| Self {
            key: key.to_string(),
        })
    }

    fn base(&self) -> &'static str {
        if self.key.ends_with(":fx") {
            "https://api-free.deepl.com"
        } else {
            "https://api.deepl.com"
        }
    }

    /// Переводит реплику на язык `target` (`RU`, `EN-US`, `DE`…).
    pub fn translate(&self, text: &str, target: &str) -> Result<String> {
        let body = json!({ "text": [text], "target_lang": target });
        let url = format!("{}/v2/translate", self.base());

        let mut response = net::agent()
            .post(&url)
            .header("Authorization", format!("DeepL-Auth-Key {}", self.key))
            .header("User-Agent", AGENT)
            .send_json(&body)
            .map_err(|e| DeeplError::Network(e.to_string()))?;

        parse_translation(&read(&mut response)?)
    }

    /// Сколько символов израсходовано. Заодно проверяет ключ: чтение, без списаний.
    pub fn usage(&self) -> Result<Usage> {
        let url = format!("{}/v2/usage", self.base());

        let mut response = net::agent()
            .get(&url)
            .header("Authorization", format!("DeepL-Auth-Key {}", self.key))
            .header("User-Agent", AGENT)
            .call()
            .map_err(|e| DeeplError::Network(e.to_string()))?;

        let answer = read(&mut response)?;

        Ok(Usage {
            used: answer["character_count"].as_u64().unwrap_or(0),
            limit: answer["character_limit"].as_u64().unwrap_or(0),
        })
    }
}

const AGENT: &str = concat!("PithPlayer/", env!("CARGO_PKG_VERSION"));

/// Тело ответа как JSON; статусы с ошибкой — в понятные причины.
fn read(response: &mut ureq::http::Response<ureq::Body>) -> Result<Value> {
    let code = response.status().as_u16();

    if (200..300).contains(&code) {
        return response
            .body_mut()
            .read_json()
            .map_err(|e| DeeplError::Malformed(e.to_string()));
    }

    Err(match code {
        403 => DeeplError::BadKey,
        429 => DeeplError::TooMany,
        456 => DeeplError::QuotaExceeded,
        _ => DeeplError::Refused(
            code,
            response.body_mut().read_to_string().unwrap_or_default(),
        ),
    })
}

fn parse_translation(answer: &Value) -> Result<String> {
    answer["translations"][0]["text"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| DeeplError::Malformed("нет translations[0].text".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn перевод_достаётся_из_ответа() {
        let answer =
            json!({ "translations": [{ "detected_source_language": "EN", "text": "Привет" }] });

        assert_eq!(parse_translation(&answer).expect("перевод"), "Привет");
    }

    #[test]
    fn пустой_ответ_это_ошибка() {
        assert!(parse_translation(&json!({})).is_err());
    }

    #[test]
    fn адрес_зависит_от_типа_ключа() {
        assert_eq!(
            Deepl::new("abc:fx").expect("ключ").base(),
            "https://api-free.deepl.com"
        );
        assert_eq!(
            Deepl::new("abc").expect("ключ").base(),
            "https://api.deepl.com"
        );
        assert!(Deepl::new("  ").is_none());
    }
}
