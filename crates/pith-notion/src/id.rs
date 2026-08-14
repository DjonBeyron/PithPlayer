//! Номера страниц и баз Notion.
//!
//! Пользователь приносит их ссылкой из браузера, а не голым номером:
//! `https://app.notion.com/p/DIFF-330b5e5392878039ab95ef453be3db03`.
//! Номер — последние 32 знака шестнадцатеричного вида, с чёрточками
//! или без них.

/// Сколько знаков в номере без чёрточек.
const LEN: usize = 32;

/// Достаёт номер из ссылки или строки с номером.
///
/// `None` — номера в строке нет. Это не ошибка разбора, а обычный случай:
/// пользователь мог вставить не то.
pub fn parse(text: &str) -> Option<String> {
    let mut found = None;

    // Кусок ссылки — всё, что состоит из шестнадцатеричных знаков и чёрточек.
    for token in text.split(|c: char| !(c.is_ascii_hexdigit() || c == '-')) {
        let digits: String = token
            .chars()
            .filter(char::is_ascii_hexdigit)
            .map(|c| c.to_ascii_lowercase())
            .collect();

        if digits.len() < LEN {
            continue;
        }

        // Берём хвост: в ссылке номер стоит в конце, а перед ним бывает
        // название — и в нём попадаются те же буквы. «DIFF-330b…» без
        // этого давало номер, сдвинутый на «ff».
        found = Some(digits[digits.len() - LEN..].to_string());
    }

    found
}

/// Возвращает номер в виде с чёрточками, как его любит Notion.
pub fn dashed(id: &str) -> String {
    if id.len() != LEN {
        return id.to_string();
    }

    format!(
        "{}-{}-{}-{}-{}",
        &id[0..8],
        &id[8..12],
        &id[12..16],
        &id[16..20],
        &id[20..32]
    )
}

#[cfg(test)]
mod tests {
    use super::{dashed, parse};

    #[test]
    fn номер_берётся_из_ссылки() {
        assert_eq!(
            parse("https://app.notion.com/p/DIFF-330b5e5392878039ab95ef453be3db03").as_deref(),
            Some("330b5e5392878039ab95ef453be3db03")
        );
    }

    #[test]
    fn номер_берётся_из_ссылки_с_запросом() {
        assert_eq!(
            parse("https://www.notion.so/Cards-285b5e53928780b7aad4d589f7d78cdf?pvs=4").as_deref(),
            Some("285b5e53928780b7aad4d589f7d78cdf")
        );
    }

    #[test]
    fn голый_номер_принимается() {
        assert_eq!(
            parse("285b5e53928780b7aad4d589f7d78cdf").as_deref(),
            Some("285b5e53928780b7aad4d589f7d78cdf")
        );
    }

    #[test]
    fn номер_с_чёрточками_принимается() {
        assert_eq!(
            parse("330b5e53-9287-8159-8951-c802c4a494f0").as_deref(),
            Some("330b5e53928781598951c802c4a494f0")
        );
    }

    #[test]
    fn без_номера_ничего_не_выходит() {
        assert!(parse("https://app.notion.com/p/DIFF").is_none());
        assert!(parse("").is_none());
        assert!(parse("совсем не ссылка").is_none());
    }

    #[test]
    fn короткий_шестнадцатеричный_хвост_не_номер() {
        assert!(parse("abc123").is_none());
    }

    #[test]
    fn чёрточки_расставляются_как_в_notion() {
        assert_eq!(
            dashed("330b5e53928781598951c802c4a494f0"),
            "330b5e53-9287-8159-8951-c802c4a494f0"
        );
    }

    #[test]
    fn чужая_длина_остаётся_как_есть() {
        assert_eq!(dashed("коротко"), "коротко");
    }
}
