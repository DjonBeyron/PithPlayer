//! Выгрузка отрезков в базу CodeEditVideo.
//!
//! «База» — файл `db/records.json` в репозитории проектов CodeEditVideo
//! на GitHub. Плеер читает его, добавляет записи и коммитит обратно через
//! GitHub API. Нужен токен с правом записи в этот репозиторий
//! (fine-grained, Contents: Read and write).
//!
//! Файл правит и сам CodeEditVideo, поэтому запись оптимистичная: уходит
//! вместе с `sha` прочитанной версии, а если за это время файл изменился
//! (GitHub отвечает 409/422), чтение и добавление повторяются.

mod net;
mod record;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

pub use net::use_proxy;
pub use record::{Entry, Report, append};

const OWNER: &str = "DjonBeyron";
const REPO: &str = "codeeditvideo-projects";
const FILE: &str = "db/records.json";

/// Сколько раз пробуем записать при гонке с другим автором файла.
const ATTEMPTS: usize = 4;

#[derive(Debug, thiserror::Error)]
pub enum CevError {
    #[error("GitHub не отвечает: {0}")]
    Network(String),
    #[error("GitHub отказал ({0}): {1}")]
    Refused(u16, String),
    #[error("токен не подходит: нет права записи в {OWNER}/{REPO}")]
    NoWriteAccess,
    #[error("файл базы не разобрать: {0}")]
    Malformed(String),
    #[error("файл изменялся слишком часто, запись не удалась")]
    Contended,
}

type Result<T> = std::result::Result<T, CevError>;

/// Что показала проверка доступа.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Access {
    /// Записей в базе сейчас.
    pub records: usize,
    /// Следующий номер, который получит новая запись.
    pub next_number: usize,
}

/// Клиент GitHub с токеном.
pub struct Cev {
    token: String,
}

/// Версия файла: содержимое и `sha` для обновления.
struct Remote {
    doc: Value,
    sha: String,
}

impl Cev {
    /// `None` — токен пуст.
    pub fn new(token: &str) -> Option<Self> {
        let token = token.trim();
        (!token.is_empty()).then(|| Self {
            token: token.to_string(),
        })
    }

    /// Только чтение: токен годится, право записи есть, файл разбирается.
    ///
    /// Ничего не пишет — безопасно нажимать сколько угодно.
    pub fn check(&self) -> Result<Access> {
        let repo = self.get_json(&format!("https://api.github.com/repos/{OWNER}/{REPO}"))?;

        let can_push = repo["permissions"]["push"].as_bool().unwrap_or(false);
        if !can_push {
            return Err(CevError::NoWriteAccess);
        }

        let remote = self.read()?;
        let records = remote.doc["records"].as_array().map_or(0, Vec::len);
        let preview = append_preview(&remote.doc);

        Ok(Access {
            records,
            next_number: preview,
        })
    }

    /// Добавляет записи одним коммитом.
    pub fn push(&self, film: &str, entries: &[Entry]) -> Result<Report> {
        for attempt in 1..=ATTEMPTS {
            let mut remote = self.read()?;
            let now = now_ms();

            let report =
                append(&mut remote.doc, film, entries, now).map_err(CevError::Malformed)?;
            let text = serde_json::to_string_pretty(&remote.doc)
                .map_err(|e| CevError::Malformed(e.to_string()))?;

            let body = json!({
                "message": format!("Pith Player: {} отрезков, {film}", entries.len()),
                "content": STANDARD.encode(text),
                "sha": remote.sha,
            });

            let url = file_url();
            let mut response = self
                .with_headers(net::agent().put(&url))
                .send_json(&body)
                .map_err(|e| CevError::Network(e.to_string()))?;

            match response.status().as_u16() {
                200 | 201 => {
                    tracing::info!(
                        записей = report.created,
                        попытка = attempt,
                        "база обновлена"
                    );
                    return Ok(report);
                }
                409 | 422 => {
                    tracing::debug!(попытка = attempt, "файл изменился, читаю заново");
                }
                code => return Err(refused(code, &mut response)),
            }
        }

        Err(CevError::Contended)
    }

    fn read(&self) -> Result<Remote> {
        let meta = self.get_json(&file_url())?;

        let sha = meta["sha"]
            .as_str()
            .ok_or_else(|| CevError::Malformed("нет sha".into()))?
            .to_string();

        // Файлы больше мегабайта GitHub отдаёт без содержимого — тогда
        // забираем сырой текст отдельным запросом.
        let inline = meta["content"].as_str().unwrap_or_default();
        let bytes = if inline.is_empty() {
            self.get_raw()?
        } else {
            let clean: String = inline.split_whitespace().collect();
            STANDARD
                .decode(clean)
                .map_err(|e| CevError::Malformed(e.to_string()))?
        };

        let doc = serde_json::from_slice(&bytes).map_err(|e| CevError::Malformed(e.to_string()))?;

        Ok(Remote { doc, sha })
    }

    fn get_json(&self, url: &str) -> Result<Value> {
        let mut response = self
            .with_headers(net::agent().get(url))
            .call()
            .map_err(|e| CevError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(refused(response.status().as_u16(), &mut response));
        }

        response
            .body_mut()
            .read_json()
            .map_err(|e| CevError::Malformed(e.to_string()))
    }

    fn get_raw(&self) -> Result<Vec<u8>> {
        let mut response = self
            .with_headers(net::agent().get(&file_url()))
            .header("Accept", "application/vnd.github.raw+json")
            .call()
            .map_err(|e| CevError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(refused(response.status().as_u16(), &mut response));
        }

        response
            .body_mut()
            .with_config()
            .limit(256 * 1024 * 1024)
            .read_to_vec()
            .map_err(|e| CevError::Network(e.to_string()))
    }

    fn with_headers<B>(&self, request: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
        request
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header(
                "User-Agent",
                concat!("PithPlayer/", env!("CARGO_PKG_VERSION")),
            )
    }
}

fn file_url() -> String {
    format!("https://api.github.com/repos/{OWNER}/{REPO}/contents/{FILE}")
}

fn refused(code: u16, response: &mut ureq::http::Response<ureq::Body>) -> CevError {
    let text = response.body_mut().read_to_string().unwrap_or_default();
    let message = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| v["message"].as_str().map(str::to_string))
        .unwrap_or(text);

    CevError::Refused(code, message)
}

fn append_preview(doc: &Value) -> usize {
    let mut copy = doc.clone();
    append(&mut copy, "", &[], 0).map_or(0, |r| r.first_number)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}
