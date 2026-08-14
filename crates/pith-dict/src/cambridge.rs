//! Второй словарь: dictionary.cambridge.org.
//!
//! Спрашивается, когда первый словарь слова не знает. Берём **английский**
//! словарь, а не англо-русский: на англо-русской странице транскрипции нет
//! вовсе — проверено запросом, слова `where` там не найти ни в одном виде.
//!
//! Разметка американского произношения (со страницы `where`):
//!
//! ```html
//! <span class="us dpron-i "><span class="region dreg">us</span>
//!   <span class="daud">…</span>
//!   <span class="pron dpron">/<span class="ipa dipa lpr-2 lpl-1">wer</span>/</span>
//! </span>
//! ```
//!
//! Важные мелочи: у класса блока **пробел в конце** (`"us dpron-i "`),
//! а сама запись собирается из нескольких кусков — в `weər` часть `r`
//! лежит отдельным `span class="sp dsp"`. Поэтому берём всё между косыми
//! и снимаем разметку.

/// Адрес страницы слова.
pub fn url(key: &str) -> String {
    format!("https://dictionary.cambridge.org/dictionary/english/{key}")
}

/// Американская транскрипция со страницы, в том же виде `|…|`, что у первого
/// словаря: поле в Notion одно, и записи в нём должны выглядеть одинаково.
///
/// Американского произношения нет — берём первое, какое есть: у части слов
/// Cambridge даёт только британское, и это лучше пустого поля.
pub fn parse(html: &str) -> Option<String> {
    let sound = american(html).or_else(|| first_pron(html))?;

    (!sound.is_empty()).then(|| format!("|{sound}|"))
}

/// Транскрипция из американского блока.
fn american(html: &str) -> Option<String> {
    // Класс ищем без закрывающей кавычки: у сайта он с пробелом в конце.
    let at = html.find("class=\"us dpron-i")?;

    first_pron(&html[at..])
}

/// Первая запись `/…/` в куске разметки.
///
/// Запись обрамлена косыми — `/<span…>wer</span>/` — но косая есть
/// и в каждом закрывающем теге. Поэтому разметку приходится читать
/// с оглядкой: искать закрывающую косую **вне** тегов. Без этого
/// `ˈhæv.<span…>ə</span>nt` обрывалось на `ˈhæv.ə`: разбор натыкался
/// на косую внутри `</span>`.
fn first_pron(html: &str) -> Option<String> {
    const OPEN: &str = "class=\"pron dpron\">";

    let at = html.find(OPEN)? + OPEN.len();
    let mut text = String::new();
    let mut inside_tag = false;
    let mut started = false;

    for symbol in html[at..].chars() {
        match symbol {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if inside_tag => {}
            // Косая вне тега: открывающая — начало записи, вторая — конец.
            '/' if !started => started = true,
            '/' => return Some(trim_variant(&text)),
            _ if started => text.push(symbol),
            _ => {}
        }
    }

    None
}

/// Приводит найденное к одному варианту.
///
/// У части слов Cambridge даёт два произношения через запятую — берём
/// первое, как и первый словарь берёт сильную форму.
fn trim_variant(text: &str) -> String {
    text.split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{parse, url};

    /// Куски разметки `where` — сняты с живой страницы.
    const WHERE: &str = r#"<span class="uk dpron-i "><span class="region dreg">uk</span>\
<span class="daud"><amp-audio></amp-audio></span>\
<span class="pron dpron">/<span class="ipa dipa lpr-2 lpl-1">weə<span class="sp dsp">r</span></span>/</span></span> \
<span class="us dpron-i "><span class="region dreg">us</span><span class="daud"></span>\
<span class="pron dpron">/<span class="ipa dipa lpr-2 lpl-1">wer</span>/</span></span>"#;

    /// Слово с двумя американскими вариантами через запятую.
    const TWO: &str = r#"<span class="us dpron-i "><span class="pron dpron">\
/<span class="ipa dipa lpr-2 lpl-1">hweər</span>, <span class="ipa dipa lpr-2 lpl-1">weər</span>/</span></span>"#;

    #[test]
    fn адрес_ведёт_в_английский_словарь() {
        assert_eq!(
            url("where"),
            "https://dictionary.cambridge.org/dictionary/english/where"
        );
    }

    #[test]
    fn берётся_американская_а_не_британская() {
        // Британская `weər` стоит на странице первой — брать нужно `wer`.
        assert_eq!(parse(WHERE).as_deref(), Some("|wer|"));
    }

    #[test]
    fn хвост_после_вложенного_тега_не_теряется() {
        // Разметка `haven't`: в закрывающем теге тоже есть косая, и разбор
        // обрывался на ней — выходило `ˈhæv.ə` вместо `ˈhæv.ənt`.
        let html = r#"<span class="us dpron-i "><span class="pron dpron">\
/<span class="ipa dipa lpr-2 lpl-1">ˈhæv.<span class="sp dsp">ə</span>nt</span>/</span></span>"#;

        assert_eq!(parse(html).as_deref(), Some("|ˈhæv.ənt|"));
    }

    #[test]
    fn запись_склеивается_из_кусков() {
        let html = r#"<span class="us dpron-i "><span class="pron dpron">\
/<span class="ipa dipa">weə<span class="sp dsp">r</span></span>/</span></span>"#;

        assert_eq!(
            parse(html).as_deref(),
            Some("|weər|"),
            "часть записи лежит отдельным span — пробела между ними быть не должно"
        );
    }

    #[test]
    fn из_двух_вариантов_берётся_первый() {
        assert_eq!(parse(TWO).as_deref(), Some("|hweər|"));
    }

    #[test]
    fn без_американского_берём_какое_есть() {
        let html = r#"<span class="uk dpron-i "><span class="pron dpron">\
/<span class="ipa dipa">ˈbɒt.l̩</span>/</span></span>"#;

        assert_eq!(parse(html).as_deref(), Some("|ˈbɒt.l̩|"));
    }

    #[test]
    fn на_чужой_странице_ничего_не_находим() {
        assert!(parse("<html><body>слова нет</body></html>").is_none());
    }
}
