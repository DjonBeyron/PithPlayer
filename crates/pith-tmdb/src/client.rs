//! Запросы к базе фильмов.
//!
//! Запросов всего три: найти картину, взять её состав, узнать русское имя
//! актёра. Все блокирующие — вызывать их полагается в отдельном потоке,
//! а не в потоке интерфейса.

use std::io::Read;
use std::time::Duration;

use serde::Deserialize;

use crate::error::{Result, TmdbError};
use crate::model::{Actor, Title};
use crate::normalize::Query;

/// Сколько ждать ответа. Больше — пользователь решит, что плеер завис.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Сколько человек берём из состава.
///
/// Состав приходит целиком одним запросом, так что предел — только про
/// длину списка на экране. Полсотни закрывает и эпизодические роли.
const CAST_LIMIT: usize = 50;

const HOST: &str = "https://api.themoviedb.org/3";

/// Доступ к базе фильмов.
pub struct Tmdb {
    key: String,
}

impl Tmdb {
    /// `None`, если ключ не задан.
    pub fn new(key: &str) -> Option<Self> {
        let key = key.trim();

        (!key.is_empty()).then(|| Self { key: key.into() })
    }

    /// Ищет картину и возвращает первое совпадение.
    pub fn find(&self, query: &Query) -> Result<Title> {
        let mut url = format!(
            "{HOST}/search/multi?api_key={}&language=ru-RU&include_adult=false&query={}",
            self.key,
            encode(&query.title)
        );

        if let Some(year) = query.year {
            url.push_str(&format!("&year={year}"));
        }

        let found: SearchResponse = self.get(&url)?;

        found
            .results
            .into_iter()
            .find_map(Title::from_search)
            .ok_or_else(|| TmdbError::NotFound {
                query: query.title.clone(),
            })
    }

    /// Состав картины.
    pub fn cast(&self, title: &Title) -> Result<Vec<Actor>> {
        let url = if title.series {
            format!(
                "{HOST}/tv/{}/aggregate_credits?api_key={}&language=ru-RU",
                title.id, self.key
            )
        } else {
            format!(
                "{HOST}/movie/{}/credits?api_key={}&language=ru-RU",
                title.id, self.key
            )
        };

        let credits: CreditsResponse = self.get(&url)?;

        Ok(credits
            .cast
            .into_iter()
            .take(CAST_LIMIT)
            .map(RawActor::into_actor)
            .collect())
    }

    /// Русское имя актёра, если база его знает.
    ///
    /// Отдельным запросом и только для выбранного человека: в составе имена
    /// приходят латиницей, а перевод лежит в списке прочих имён. Тянуть его
    /// на полсотни человек ради одного выбранного незачем.
    pub fn russian_name(&self, actor_id: i64) -> Result<Option<String>> {
        let url = format!("{HOST}/person/{actor_id}?api_key={}", self.key);
        let person: PersonResponse = self.get(&url)?;

        Ok(person.also_known_as.into_iter().find(|name| cyrillic(name)))
    }

    fn get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let response = crate::net::agent(TIMEOUT)
            .get(url)
            .call()
            .map_err(|e| match e {
                ureq::Error::StatusCode(status) => TmdbError::Refused { status },
                other => TmdbError::Network(other.to_string()),
            })?;

        response
            .into_body()
            .read_json()
            .map_err(|e| TmdbError::Malformed(e.to_string()))
    }
}

/// Забирает фотографию по адресу.
///
/// Отдельно от `Tmdb`: картинки лежат на другом сервере и ключа не просят.
/// Вызывать полагается в отдельном потоке — как и всё остальное здесь.
pub fn fetch_photo(url: &str) -> Result<Vec<u8>> {
    let mut response = crate::net::agent(TIMEOUT)
        .get(url)
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(status) => TmdbError::Refused { status },
            other => TmdbError::Network(other.to_string()),
        })?;

    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| TmdbError::Network(e.to_string()))?;

    Ok(bytes)
}

/// Есть ли в строке кириллица.
fn cyrillic(text: &str) -> bool {
    text.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c))
}

/// Переводит строку в вид, пригодный для адреса.
///
/// Кодируем всё, кроме букв и цифр: своя реализация избавляет от ещё одной
/// зависимости ради десятка символов.
fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());

    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }

    out
}

#[derive(Deserialize)]
struct SearchResponse {
    results: Vec<SearchItem>,
}

#[derive(Deserialize)]
struct SearchItem {
    id: i64,
    media_type: Option<String>,
    /// У фильма название здесь.
    title: Option<String>,
    /// У сериала — здесь.
    name: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
}

impl Title {
    fn from_search(item: SearchItem) -> Option<Self> {
        let series = match item.media_type.as_deref() {
            Some("tv") => true,
            Some("movie") => false,
            // Люди и всё прочее в выдаче нас не интересуют.
            _ => return None,
        };

        let name = item.title.or(item.name)?;
        let date = item.release_date.or(item.first_air_date);

        Some(Self {
            id: item.id,
            name,
            year: date.as_deref().and_then(year_of),
            series,
        })
    }
}

/// Год из даты вида `2025-04-17`.
fn year_of(date: &str) -> Option<u32> {
    date.get(..4)?.parse().ok()
}

#[derive(Deserialize)]
struct CreditsResponse {
    cast: Vec<RawActor>,
}

#[derive(Deserialize)]
struct RawActor {
    id: i64,
    name: String,
    profile_path: Option<String>,
    /// У фильма роль здесь.
    character: Option<String>,
    /// У сериала ролей может быть несколько — берём первую.
    #[serde(default)]
    roles: Vec<RawRole>,
}

#[derive(Deserialize)]
struct RawRole {
    character: Option<String>,
}

impl RawActor {
    fn into_actor(self) -> Actor {
        let role = self
            .character
            .or_else(|| self.roles.into_iter().find_map(|r| r.character))
            .filter(|role| !role.is_empty());

        Actor {
            id: self.id,
            name: self.name,
            role,
            photo: self.profile_path,
        }
    }
}

#[derive(Deserialize)]
struct PersonResponse {
    #[serde(default)]
    also_known_as: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::{Tmdb, cyrillic, encode, year_of};

    #[test]
    fn без_ключа_доступа_нет() {
        assert!(Tmdb::new("").is_none());
        assert!(Tmdb::new("   ").is_none());
        assert!(Tmdb::new("ключ").is_some());
    }

    #[test]
    fn кириллица_отличается_от_латиницы() {
        assert!(cyrillic("Леонардо ДиКаприо"));
        assert!(!cyrillic("Leonardo DiCaprio"));
        assert!(!cyrillic(""));
    }

    #[test]
    fn адрес_кодируется() {
        assert_eq!(encode("Dune Part Two"), "Dune+Part+Two");
        assert_eq!(encode("Тайна"), "%D0%A2%D0%B0%D0%B9%D0%BD%D0%B0");
        assert_eq!(encode("a&b=c"), "a%26b%3Dc");
    }

    #[test]
    fn год_из_даты() {
        assert_eq!(year_of("2025-04-17"), Some(2025));
        assert_eq!(year_of(""), None);
        assert_eq!(year_of("не дата"), None);
    }
}
