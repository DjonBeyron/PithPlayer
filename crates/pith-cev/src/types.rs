//! Общие типы выгрузки: отрезок со словами, вид картины и её название.
//!
//! Раньше жили в крейте Notion; после перехода на базу CodeEditVideo
//! остались только они.

/// Транскрипция слова реплики.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sound {
    /// Транскрипция вместе с чертами: `|wer|`.
    pub transcription: String,
    /// Страница слова в словаре — на неё ведёт ссылка.
    pub url: String,
}

/// Отрезок списка: реплика, актёр и транскрипция.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Номер по порядку в списке плеера.
    pub number: usize,
    /// Реплика. Пусто — закладка без названия.
    pub text: String,
    /// Актёр «Имя (Роль)». Пусто — актёра не выбирали.
    pub actor: Option<String>,
    /// Транскрипция реплики: по одной записи на слово, в порядке слов.
    ///
    /// Пусто — транскрипцию не запрашивали или не нашли: это не ошибка.
    pub sounds: Vec<Sound>,
}

/// Фильм или сериал — выбор пользователя перед выгрузкой.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Series,
    Movie,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Series => "Сериал",
            Self::Movie => "Фильм",
        }
    }
}

/// Название картины в том виде, в каком оно уходит в базу: «Фильм: Титаник».
///
/// Пустое название заменяется на «Без названия»: строка идёт в поле
/// каждой записи, и пустой она быть не должна.
pub fn film_name(kind: Kind, title: &str) -> String {
    let title = title.trim();
    let title = if title.is_empty() {
        "Без названия"
    } else {
        title
    };

    format!("{}: {title}", kind.as_str())
}

#[cfg(test)]
mod tests {
    use super::{Kind, film_name};

    #[test]
    fn вид_и_название_склеиваются() {
        assert_eq!(film_name(Kind::Movie, "Титаник"), "Фильм: Титаник");
        assert_eq!(
            film_name(Kind::Series, "Во все тяжкие"),
            "Сериал: Во все тяжкие"
        );
    }

    #[test]
    fn пустое_название_заменяется() {
        assert_eq!(film_name(Kind::Movie, "  "), "Фильм: Без названия");
    }
}
