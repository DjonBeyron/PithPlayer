//! Почему не удалось получить состав.
//!
//! Ни один из этих случаев не повод падать: плеер продолжает играть,
//! а окно актёров показывает причину словами.

use std::fmt;

#[derive(Debug)]
pub enum TmdbError {
    /// Ключ доступа не задан в настройках.
    NoKey,
    /// Из имени файла не вышло названия для поиска.
    NoTitle,
    /// База не нашла картины с таким названием.
    NotFound { query: String },
    /// Сеть недоступна или база не ответила.
    Network(String),
    /// База ответила отказом: неверный ключ, превышен предел запросов.
    Refused { status: u16 },
    /// Ответ пришёл, но разобрать его не вышло.
    Malformed(String),
}

impl fmt::Display for TmdbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoKey => write!(f, "не задан ключ доступа к базе фильмов"),
            Self::NoTitle => write!(f, "из имени файла не вышло названия для поиска"),
            Self::NotFound { query } => write!(f, "база не нашла картину «{query}»"),
            Self::Network(e) => write!(f, "база не ответила: {e}"),
            Self::Refused { status } => write!(f, "база отказала, код {status}"),
            Self::Malformed(e) => write!(f, "ответ базы не разобрать: {e}"),
        }
    }
}

impl std::error::Error for TmdbError {}

pub type Result<T> = std::result::Result<T, TmdbError>;
