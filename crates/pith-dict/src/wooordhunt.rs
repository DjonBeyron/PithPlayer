//! Первый словарь: wooordhunt.ru.
//!
//! Разметка у него не одна: сайт показывает транскрипцию по-разному
//! в зависимости от слова. Три стратегии разбора перенесены из готовой
//! системы пользователя (`NOTION_PITH/TRANSCRIPTION_LOGIC.md`) и проверены
//! на живых страницах: `where` и `staying` разбираются первой стратегией,
//! `to` — второй.

/// Адрес страницы слова.
pub fn url(key: &str) -> String {
    format!("https://wooordhunt.ru/word/{key}")
}

/// Признак того, что страница настоящая, а не заглушка.
///
/// Сервер ограничивает частоту запросов и на четвёртый подряд отдаёт
/// страницу без транскрипций вовсе. Отличить её от слова, которого в словаре
/// нет, можно только так: у настоящей страницы этот класс есть всегда.
pub fn looks_real(html: &str) -> bool {
    html.contains("class=\"transcription\"")
}

/// Американская транскрипция со страницы. `None` — на странице её нет.
pub fn parse(html: &str) -> Option<String> {
    direct(html).or_else(|| american_section(html)).or_else(|| {
        // Запасная: первая транскрипция на странице. Обычно она же
        // американская — сайт начинает с неё.
        first_transcription(html)
    })
}

/// Стратегия 1: у нужного `span` прямо сказано, что транскрипция американская.
///
/// Порядок атрибутов у сайта плавает, поэтому проверяем оба.
fn direct(html: &str) -> Option<String> {
    const MARK: &str = "американская транскрипция";

    let mut from = 0;

    while let Some(start) = html[from..].find("<span").map(|at| from + at) {
        let end = html[start..].find('>').map(|at| start + at)?;
        let tag = &html[start..end];

        from = end + 1;

        if !tag.contains(MARK) || !tag.contains("class=\"transcription\"") {
            continue;
        }

        if let Some(found) = piped(&html[from..]) {
            return Some(found);
        }
    }

    None
}

/// Стратегия 2: раздел между «амер.» и «брит.».
///
/// Так устроены слова с сильной и слабой формой — `to`, `you`, `of`, `the`.
/// Первая транскрипция в разделе — сильная форма, её и берём.
fn american_section(html: &str) -> Option<String> {
    let start = html.find("амер.")? + "амер.".len();
    let rest = &html[start..];
    let end = rest.find("брит.").unwrap_or(rest.len());

    first_transcription(&rest[..end])
}

/// Первая транскрипция в куске разметки.
fn first_transcription(html: &str) -> Option<String> {
    let at = html.find("class=\"transcription\"")?;
    let rest = &html[at..];
    let after_tag = rest.find('>')? + 1;

    piped(&rest[after_tag..])
}

/// Читает `|транскрипцию|` с начала куска, пропустив пробелы.
///
/// Ограничиваемся одним элементом: дальше на странице идут другие формы
/// и другие слова, и до них дело не наше.
fn piped(text: &str) -> Option<String> {
    let text = text.trim_start();
    let mut symbols = text.char_indices();

    if symbols.next()?.1 != '|' {
        return None;
    }

    for (at, symbol) in symbols {
        match symbol {
            '|' => return Some(text[..=at].to_string()),
            // За разметку транскрипция не выходит — значит это не она.
            '<' | '\n' => return None,
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{looks_real, parse, url};

    /// Разметка `where` — как её отдаёт сайт (стратегия 1).
    const WHERE: &str = r#"<div class="word_pron"><span title="американская транскрипция слова where" \
class="transcription"> |wer|</span> <span class="tr_tail">амер.</span></div>"#;

    /// Разметка `to`: сильная и слабая формы, американская и британская
    /// (стратегия 2). Порядок и обрамление взяты со страницы.
    const TO: &str = r#"<div><i class="es_i">амер.</i></div> <div> <i>strong</i> \
<span class="transcription"> |tuː|</span> <audio id="audio_us_s"></audio> </div> \
<div> <i>weak</i> <span class="transcription"> |tə|</span> </div> \
<div><i class="es_i">брит.</i></div> <div> <span class="transcription"> |tuː|</span> </div>"#;

    #[test]
    fn адрес_собирается_из_ключа() {
        assert_eq!(url("where"), "https://wooordhunt.ru/word/where");
    }

    #[test]
    fn стратегия_прямого_указания() {
        assert_eq!(parse(WHERE).as_deref(), Some("|wer|"));
    }

    #[test]
    fn атрибуты_в_обратном_порядке_тоже_читаются() {
        let html = r#"<span class="transcription" title="американская транскрипция слова both"> |bəʊθ|</span>"#;

        assert_eq!(parse(html).as_deref(), Some("|bəʊθ|"));
    }

    #[test]
    fn стратегия_раздела_амер() {
        // Британская форма стоит на странице тоже — взять нужно американскую.
        assert_eq!(parse(TO).as_deref(), Some("|tuː|"));
    }

    #[test]
    fn слабая_форма_не_берётся() {
        // В разделе «амер.» первой идёт сильная форма — она и нужна.
        assert_ne!(parse(TO).as_deref(), Some("|tə|"));
    }

    #[test]
    fn запасная_стратегия_берёт_первую() {
        let html = r#"<span class="transcription"> |ˈsteɪɪŋ|</span>"#;

        assert_eq!(parse(html).as_deref(), Some("|ˈsteɪɪŋ|"));
    }

    #[test]
    fn без_транскрипций_страница_считается_заглушкой() {
        assert!(!looks_real("<html><body>ничего</body></html>"));
        assert!(looks_real(WHERE));
    }

    #[test]
    fn на_странице_без_транскрипции_ничего_не_находим() {
        assert!(parse("<html><body>слова нет</body></html>").is_none());
    }
}
