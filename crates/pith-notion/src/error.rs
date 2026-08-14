//! Почему выгрузка не удалась.
//!
//! Ни один случай не повод падать: плеер продолжает работать, а окно
//! показывает причину словами.

use std::fmt;

#[derive(Debug)]
pub enum NotionError {
    /// Не задан токен или страница с карточками.
    NotConnected,
    /// Ссылка есть, а номера в ней нет.
    BadLink { what: &'static str },
    /// На странице образца не нашлось базы.
    NoTemplate,
    /// Сеть недоступна или Notion не ответил.
    Network(String),
    /// Notion отказал: неверный токен, нет доступа к странице.
    Refused { status: u16, message: String },
    /// Ответ пришёл, но разобрать его не вышло.
    Malformed(String),
}

impl fmt::Display for NotionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected => write!(f, "Notion не подключён"),
            Self::BadLink { what } => write!(f, "в ссылке на {what} нет номера страницы"),
            Self::NoTemplate => write!(f, "на странице образца нет базы данных"),
            Self::Network(e) => write!(f, "Notion не ответил: {e}"),
            Self::Refused { status, message } => {
                write!(f, "Notion отказал ({status}): {message}")
            }
            Self::Malformed(e) => write!(f, "ответ Notion не разобрать: {e}"),
        }
    }
}

impl std::error::Error for NotionError {}

pub type Result<T> = std::result::Result<T, NotionError>;
