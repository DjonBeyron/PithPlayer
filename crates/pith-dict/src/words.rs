//! Фраза → слова, у которых спрашивают транскрипцию.
//!
//! Чистая логика, вся под тестами: ошибка здесь тиха и заметна только
//! в готовой карточке Notion.

/// Слова реплики в порядке появления.
///
/// Берутся только латинские буквы и апостроф: цифры, знаки препинания
/// и русские вставки словарю сказать нечего. Пустая реплика даёт пустой
/// список — и поле транскрипции останется незаполненным.
pub fn split(phrase: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();

    for symbol in phrase.chars() {
        if symbol.is_ascii_alphabetic() || is_apostrophe(symbol) {
            current.push(symbol);
            continue;
        }

        push_word(&mut words, &mut current);
    }

    push_word(&mut words, &mut current);
    words
}

/// Ключ слова: им же оно лежит в кэше и в адресе страницы.
///
/// Нижний регистр без апострофов — `It's` и `its` для словаря одно и то же
/// слово, и спрашивать его дважды незачем.
pub fn key(word: &str) -> String {
    word.chars()
        .filter(|symbol| !is_apostrophe(*symbol))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Слово для второго словаря: апостроф заменён дефисом.
///
/// Cambridge зовёт сокращения именно так: `haven't` живёт по адресу
/// `haven-t`. Со слитным `havent` он отвечает переадресацией в никуда,
/// и слово считалось ненайденным, хотя оно там есть.
pub fn dashed(word: &str) -> String {
    word.chars()
        .map(|symbol| if is_apostrophe(symbol) { '-' } else { symbol })
        .flat_map(char::to_lowercase)
        .collect()
}

/// Апостроф в любом из начертаний: прямой и типографский.
pub fn is_apostrophe(symbol: char) -> bool {
    matches!(symbol, '\'' | '\u{2019}' | '\u{02BC}')
}

/// Кладёт набранное слово, если оно не пустое.
fn push_word(words: &mut Vec<String>, current: &mut String) {
    if current.is_empty() {
        return;
    }

    // Слово из одних апострофов словом не считаем.
    if current.chars().any(|s| s.is_ascii_alphabetic()) {
        words.push(std::mem::take(current));
    } else {
        current.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{key, split};

    #[test]
    fn фраза_делится_на_слова() {
        assert_eq!(
            split("Where are you staying?"),
            ["Where", "are", "you", "staying"]
        );
    }

    #[test]
    fn апостроф_остаётся_внутри_слова() {
        assert_eq!(
            split("It's mine, isn't it?"),
            ["It's", "mine", "isn't", "it"]
        );
    }

    #[test]
    fn цифры_и_чужие_буквы_не_слова() {
        assert_eq!(split("Room 101, привет!"), ["Room"]);
    }

    #[test]
    fn пустая_фраза_даёт_пустой_список() {
        assert!(split("").is_empty());
        assert!(split("— 42 … ?!").is_empty());
    }

    #[test]
    fn тире_и_дефис_делят_слова() {
        assert_eq!(split("well-known one—two"), ["well", "known", "one", "two"]);
    }

    #[test]
    fn ключ_без_регистра_и_апострофов() {
        assert_eq!(key("It's"), "its");
        assert_eq!(key("WHERE"), "where");
        assert_eq!(key("don\u{2019}t"), "dont");
    }

    #[test]
    fn апостроф_становится_дефисом_для_второго_словаря() {
        // Cambridge зовёт сокращения через дефис: `haven't` → `haven-t`.
        assert_eq!(super::dashed("haven't"), "haven-t");
        assert_eq!(super::dashed("Shouldn\u{2019}t"), "shouldn-t");
        assert_eq!(
            super::dashed("menu"),
            "menu",
            "без апострофа ничего не меняем"
        );
    }

    #[test]
    fn одни_апострофы_словом_не_считаются() {
        assert!(split("'' ' ''").is_empty());
    }
}
