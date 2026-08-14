//! Что плеер получает от базы фильмов.

use serde::{Deserialize, Serialize};

/// Найденная картина.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Title {
    pub id: i64,
    /// Название так, как его знает база.
    pub name: String,
    /// Год выпуска. Пусто, если база его не знает.
    pub year: Option<u32>,
    /// Сериал или фильм: у них разные запросы за составом.
    pub series: bool,
}

/// Актёр из состава.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub id: i64,
    /// Имя так, как его отдала база.
    pub name: String,
    /// Кого играет. Пусто — база роли не знает.
    pub role: Option<String>,
    /// Путь к фотографии внутри базы, без адреса сервера.
    ///
    /// Хранится путём, а не готовой ссылкой: адрес сервера база может
    /// сменить, а путь останется прежним.
    pub photo: Option<String>,
}

impl Actor {
    /// Подпись для списка: «Имя (Роль)».
    pub fn label(&self) -> String {
        match &self.role {
            Some(role) if !role.is_empty() => format!("{} ({role})", self.name),
            _ => self.name.clone(),
        }
    }

    /// Адрес фотографии нужного размера.
    pub fn photo_url(&self, size: PhotoSize) -> Option<String> {
        self.photo
            .as_ref()
            .map(|path| format!("https://image.tmdb.org/t/p/{}{path}", size.as_str()))
    }
}

/// Размер фотографии, который просим у базы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoSize {
    /// Кружок в списке — больше не нужно.
    List,
}

impl PhotoSize {
    fn as_str(self) -> &'static str {
        match self {
            Self::List => "w185",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Actor, PhotoSize};

    fn актёр(role: Option<&str>, photo: Option<&str>) -> Actor {
        Actor {
            id: 1,
            name: "Леонардо ДиКаприо".into(),
            role: role.map(String::from),
            photo: photo.map(String::from),
        }
    }

    #[test]
    fn подпись_содержит_роль_в_скобках() {
        assert_eq!(
            актёр(Some("Jack Dawson"), None).label(),
            "Леонардо ДиКаприо (Jack Dawson)"
        );
    }

    #[test]
    fn без_роли_остаётся_одно_имя() {
        assert_eq!(актёр(None, None).label(), "Леонардо ДиКаприо");
        assert_eq!(актёр(Some(""), None).label(), "Леонардо ДиКаприо");
    }

    #[test]
    fn адрес_фотографии_собирается_из_пути() {
        assert_eq!(
            актёр(None, Some("/abc.jpg"))
                .photo_url(PhotoSize::List)
                .as_deref(),
            Some("https://image.tmdb.org/t/p/w185/abc.jpg")
        );
    }

    #[test]
    fn без_фотографии_адреса_нет() {
        assert!(актёр(None, None).photo_url(PhotoSize::List).is_none());
    }
}
