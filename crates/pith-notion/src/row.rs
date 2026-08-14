//! Строка базы: отрезок плеера в свойствах Notion.
//!
//! Имена свойств взяты из образца и в коде не выдумываются: заголовок
//! назван пустой строкой, у английской реплики в начале имени пробел.
//! Ошибиться здесь легко, поэтому имена собраны в одном месте.

use serde_json::{Map, Value, json};

/// Заголовок строки — свойство с пустым именем.
pub const TITLE: &str = "";

/// Реплика отрезка.
pub const ENG: &str = " ENG";

/// Кто в кадре. В образце это список, а не текст.
pub const ACTOR: &str = "ACTOR";

/// Название картины: «Фильм: Титаник» либо «Сериал: Во все тяжкие».
pub const FILM_NAME: &str = "FILM NAME";

/// Транскрипция реплики — по слову на запись, каждая ссылкой на словарь.
pub const TRANSCRIPTION: &str = "TRANSCRIPTION SENTENCE";

/// Номер строки числом — по нему вид базы и сортируется.
///
/// Заголовок тоже номер, но он текстовый: по тексту «10» встаёт перед «2»,
/// и сортировать по нему нельзя. Числовое поле снимает и вторую беду —
/// порядок строк перестаёт зависеть от порядка создания, а значит строки
/// можно создавать в несколько потоков.
pub const NUMBER: &str = "NUM";

/// Транскрипция слова реплики.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sound {
    /// Транскрипция вместе с чертами: `|wer|`.
    pub transcription: String,
    /// Страница слова в словаре — на неё ведёт ссылка.
    pub url: String,
}

/// Отрезок, каким он уходит в Notion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Номер по порядку — он же заголовок строки.
    pub number: usize,
    /// Реплика. Пусто — закладка без названия.
    pub text: String,
    /// Актёр «Имя (Роль)». Пусто — актёра не выбирали.
    pub actor: Option<String>,
    /// Транскрипция реплики: по одной записи на слово, в порядке слов.
    ///
    /// Пусто — транскрипцию не запрашивали или не нашли: поле останется
    /// незаполненным, и это не ошибка.
    pub sounds: Vec<Sound>,
}

/// Собирает свойства строки для запроса на создание.
///
/// `number` — сквозной номер в базе, а не порядок в списке плеера: база
/// одна на все картины, и счёт в ней продолжается. Идёт и в заголовок,
/// и в числовое поле `NUM`, если оно в базе есть (`numbered`) — по нему
/// сортируется вид.
///
/// Отсутствующий актёр не ошибка: поле просто не заполняется.
pub fn properties(row: &Row, film_name: &str, number: usize, numbered: bool) -> Map<String, Value> {
    let mut props = Map::new();

    props.insert(
        TITLE.into(),
        json!({ "title": text_value(&number.to_string()) }),
    );

    // Свойства, которого в базе нет, Notion не принимает — строка была бы
    // отвергнута целиком. Поэтому только когда оно там точно есть.
    if numbered {
        props.insert(NUMBER.into(), json!({ "number": number }));
    }

    props.insert(ENG.into(), json!({ "rich_text": text_value(&row.text) }));
    props.insert(
        FILM_NAME.into(),
        json!({ "rich_text": text_value(film_name) }),
    );

    if let Some(actor) = row.actor.as_deref().filter(|a| !a.trim().is_empty()) {
        props.insert(
            ACTOR.into(),
            json!({ "multi_select": [ { "name": clean_option(actor) } ] }),
        );
    }

    if !row.sounds.is_empty() {
        props.insert(
            TRANSCRIPTION.into(),
            json!({ "rich_text": sounds_value(&row.sounds) }),
        );
    }

    props
}

/// Транскрипция реплики в виде кусков текста со ссылками.
///
/// Кусок на слово, между словами пробел — и каждый ведёт на страницу слова
/// в словаре, чтобы из Notion можно было послушать произношение. Так же
/// устроено в готовой системе пользователя.
///
/// Notion не принимает больше сотни кусков в одном поле, поэтому слишком
/// длинная реплика теряет хвост: строка важнее полноты транскрипции.
fn sounds_value(sounds: &[Sound]) -> Value {
    const LIMIT: usize = 100;

    let last = sounds.len().min(LIMIT) - 1;

    let parts: Vec<Value> = sounds
        .iter()
        .take(LIMIT)
        .enumerate()
        .map(|(at, sound)| {
            let text = if at < last {
                format!("{} ", sound.transcription)
            } else {
                sound.transcription.clone()
            };

            json!({ "text": { "content": text, "link": { "url": sound.url } } })
        })
        .collect();

    json!(parts)
}

/// Значение текстового поля Notion.
///
/// Notion не принимает текст длиннее двух тысяч знаков в одном куске —
/// длинную реплику обрезаем, иначе запрос отвергается целиком.
fn text_value(text: &str) -> Value {
    const LIMIT: usize = 2000;

    let mut cut = text.to_string();
    if cut.chars().count() > LIMIT {
        cut = cut.chars().take(LIMIT).collect();
    }

    json!([ { "text": { "content": cut } } ])
}

/// Готовит имя варианта списка.
///
/// Запятая в имени варианта Notion не разрешена — она делит варианты.
fn clean_option(name: &str) -> String {
    name.replace(',', " ").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{ACTOR, ENG, FILM_NAME, NUMBER, Row, Sound, TITLE, TRANSCRIPTION, properties};

    fn отрезок(number: usize, text: &str, actor: Option<&str>) -> Row {
        Row {
            number,
            text: text.into(),
            actor: actor.map(String::from),
            sounds: Vec::new(),
        }
    }

    /// Отрезок с транскрипцией реплики.
    fn озвученный(sounds: &[(&str, &str)]) -> Row {
        Row {
            number: 1,
            text: "Where are you".into(),
            actor: None,
            sounds: sounds
                .iter()
                .map(|(transcription, url)| Sound {
                    transcription: (*transcription).into(),
                    url: (*url).into(),
                })
                .collect(),
        }
    }

    #[test]
    fn транскрипция_ложится_ссылками_по_словам() {
        let row = озвученный(&[
            ("|wer|", "https://wooordhunt.ru/word/where"),
            ("|ɑːr|", "https://wooordhunt.ru/word/are"),
            ("|juː|", "https://wooordhunt.ru/word/you"),
        ]);
        let props = properties(&row, "Ф", 1, false);
        let parts = &props[TRANSCRIPTION]["rich_text"];

        assert_eq!(
            parts[0]["text"]["content"], "|wer| ",
            "между словами пробел"
        );
        assert_eq!(
            parts[0]["text"]["link"]["url"], "https://wooordhunt.ru/word/where",
            "каждая запись — ссылка на своё слово"
        );
        assert_eq!(
            parts[2]["text"]["content"], "|juː|",
            "у последнего слова пробела нет"
        );
    }

    #[test]
    fn без_транскрипции_поля_нет() {
        let props = properties(&отрезок(1, "Реплика", None), "Ф", 1, false);

        assert!(
            !props.contains_key(TRANSCRIPTION),
            "не спрашивали или не нашли — поле остаётся пустым"
        );
    }

    #[test]
    fn слишком_длинная_транскрипция_обрезается() {
        let many: Vec<(&str, &str)> = (0..150).map(|_| ("|a|", "https://x/a")).collect();
        let props = properties(&озвученный(&many), "Ф", 1, false);

        let parts = props[TRANSCRIPTION]["rich_text"]
            .as_array()
            .expect("массив");

        assert_eq!(parts.len(), 100, "Notion больше сотни кусков не принимает");
    }

    #[test]
    fn заголовок_это_переданный_номер() {
        let props = properties(&отрезок(3, "Реплика", None), "Фильм: Титаник", 3, false);

        assert_eq!(props[TITLE]["title"][0]["text"]["content"], "3");
    }

    #[test]
    fn заголовок_продолжает_счёт_базы() {
        // Третий отрезок выгрузки, а в базе уже сорок строк.
        let props = properties(&отрезок(3, "Реплика", None), "Фильм: Титаник", 43, false);

        assert_eq!(
            props[TITLE]["title"][0]["text"]["content"], "43",
            "в заголовок идёт сквозной номер, а не порядок в списке"
        );
    }

    #[test]
    fn номер_идёт_и_числом_когда_поле_есть() {
        let props = properties(&отрезок(3, "Реплика", None), "Ф", 43, true);

        assert_eq!(props[NUMBER]["number"], 43, "по нему сортируется вид");
        assert_eq!(
            props[TITLE]["title"][0]["text"]["content"], "43",
            "заголовок тот же номер"
        );
    }

    #[test]
    fn без_поля_номера_его_не_отправляем() {
        let props = properties(&отрезок(3, "Реплика", None), "Ф", 43, false);

        assert!(
            !props.contains_key(NUMBER),
            "свойства, которого в базе нет, Notion не примет"
        );
    }

    #[test]
    fn реплика_и_название_картины_на_местах() {
        let props = properties(
            &отрезок(1, "Hello there", None),
            "Сериал: Во все тяжкие",
            1,
            false,
        );

        assert_eq!(props[ENG]["rich_text"][0]["text"]["content"], "Hello there");
        assert_eq!(
            props[FILM_NAME]["rich_text"][0]["text"]["content"],
            "Сериал: Во все тяжкие"
        );
    }

    #[test]
    fn актёр_ложится_списком() {
        let props = properties(
            &отрезок(1, "Реплика", Some("Леонардо ДиКаприо (Jack)")),
            "Фильм: Титаник",
            1,
            false,
        );

        assert_eq!(
            props[ACTOR]["multi_select"][0]["name"],
            "Леонардо ДиКаприо (Jack)"
        );
    }

    #[test]
    fn без_актёра_поля_нет() {
        let props = properties(&отрезок(1, "Реплика", None), "Фильм: Титаник", 1, false);
        assert!(!props.contains_key(ACTOR), "пустое поле не отправляем");

        let props = properties(
            &отрезок(1, "Реплика", Some("   ")),
            "Фильм: Титаник",
            1,
            false,
        );
        assert!(!props.contains_key(ACTOR), "пробелы — тот же пустой актёр");
    }

    #[test]
    fn запятая_в_имени_актёра_не_ломает_список() {
        let props = properties(
            &отрезок(1, "Р", Some("Ли, Кристофер (Saruman)")),
            "Ф",
            1,
            false,
        );

        assert_eq!(
            props[ACTOR]["multi_select"][0]["name"],
            "Ли  Кристофер (Saruman)"
        );
    }

    #[test]
    fn слишком_длинная_реплика_обрезается() {
        let long = "я".repeat(2500);
        let props = properties(&отрезок(1, &long, None), "Ф", 1, false);

        let content = props[ENG]["rich_text"][0]["text"]["content"]
            .as_str()
            .expect("строка");

        assert_eq!(content.chars().count(), 2000);
    }
}
