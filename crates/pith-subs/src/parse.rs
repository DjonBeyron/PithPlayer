//! Разбор субтитров формата SRT.
//!
//! Другие форматы отдельно не разбираются: встроенные дорожки и внешние
//! файлы приводятся к SRT через FFmpeg, который понимает ASS, VTT и прочее
//! (PLAN.md §6.2).

/// Одна реплика.
#[derive(Debug, Clone, PartialEq)]
pub struct Cue {
    /// Начало, секунды.
    pub start: f64,
    /// Конец, секунды.
    pub end: f64,
    /// Текст без разметки, переносы сохранены.
    pub text: String,
}

impl Cue {
    /// Текст одной строкой — для показа в списке результатов.
    pub fn single_line(&self) -> String {
        self.text.replace('\n', " ")
    }
}

/// Разбирает SRT.
///
/// Битые блоки пропускаются: субтитры часто собраны вручную, и одна
/// испорченная реплика не повод терять весь файл.
pub fn parse_srt(content: &str) -> Vec<Cue> {
    let content = content.replace("\r\n", "\n");

    content
        .split("\n\n")
        .filter_map(|block| parse_block(block.trim()))
        .collect()
}

fn parse_block(block: &str) -> Option<Cue> {
    if block.is_empty() {
        return None;
    }

    let mut lines = block.lines();

    // Первая строка — номер, но встречаются файлы без него.
    let first = lines.next()?;
    let timing_line = if first.contains("-->") {
        first
    } else {
        lines.next()?
    };

    let (start, end) = parse_timing(timing_line)?;
    let text = strip_markup(&lines.collect::<Vec<_>>().join("\n"));

    (!text.is_empty()).then_some(Cue { start, end, text })
}

/// Разбирает строку вида `00:00:01,000 --> 00:00:03,500`.
fn parse_timing(line: &str) -> Option<(f64, f64)> {
    let (from, to) = line.split_once("-->")?;
    Some((parse_timestamp(from.trim())?, parse_timestamp(to.trim())?))
}

/// Разбирает метку времени `ЧЧ:ММ:СС,мс`.
///
/// Точка вместо запятой тоже принимается — так пишут некоторые редакторы.
fn parse_timestamp(value: &str) -> Option<f64> {
    let value = value.split_whitespace().next()?.replace(',', ".");
    let mut parts = value.split(':');

    let hours: f64 = parts.next()?.parse().ok()?;
    let minutes: f64 = parts.next()?.parse().ok()?;
    let seconds: f64 = parts.next()?.parse().ok()?;

    Some(hours * 3600.0 + minutes * 60.0 + seconds)
}

/// Убирает разметку: теги вида `<i>` и фигурные скобки формата ASS.
fn strip_markup(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut skip_until = None;

    for ch in text.chars() {
        match (skip_until, ch) {
            (None, '<') => skip_until = Some('>'),
            (None, '{') => skip_until = Some('}'),
            (Some(closing), c) if c == closing => skip_until = None,
            (None, c) => result.push(c),
            _ => {}
        }
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ОБРАЗЕЦ: &str = "1
00:00:01,000 --> 00:00:03,500
Первая реплика

2
00:01:10,250 --> 00:01:12,000
Вторая реплика,
в две строки
";

    #[test]
    fn разбирает_простой_файл() {
        let cues = parse_srt(ОБРАЗЕЦ);

        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "Первая реплика");
        assert_eq!(cues[0].start, 1.0);
        assert_eq!(cues[0].end, 3.5);
    }

    #[test]
    fn сохраняет_переносы_внутри_реплики() {
        let cues = parse_srt(ОБРАЗЕЦ);
        assert_eq!(cues[1].text, "Вторая реплика,\nв две строки");
        assert_eq!(cues[1].single_line(), "Вторая реплика, в две строки");
    }

    #[test]
    fn считает_время_с_часами_и_минутами() {
        let cues = parse_srt(ОБРАЗЕЦ);
        assert_eq!(cues[1].start, 70.25);
    }

    #[test]
    fn принимает_переносы_строк_windows() {
        let cues = parse_srt("1\r\n00:00:01,000 --> 00:00:02,000\r\nТекст\r\n");
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text, "Текст");
    }

    #[test]
    fn принимает_точку_вместо_запятой() {
        let cues = parse_srt("1\n00:00:01.500 --> 00:00:02.000\nТекст\n");
        assert_eq!(cues[0].start, 1.5);
    }

    #[test]
    fn работает_без_номера_блока() {
        let cues = parse_srt("00:00:01,000 --> 00:00:02,000\nБез номера\n");
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text, "Без номера");
    }

    #[test]
    fn убирает_разметку() {
        let cues = parse_srt("1\n00:00:01,000 --> 00:00:02,000\n<i>Курсив</i> и {\\an8}позиция\n");
        assert_eq!(cues[0].text, "Курсив и позиция");
    }

    #[test]
    fn битые_блоки_пропускаются_без_потери_остальных() {
        let содержимое =
            "1\nчепуха вместо времени\nТекст\n\n2\n00:00:05,000 --> 00:00:06,000\nЦелая реплика\n";
        let cues = parse_srt(содержимое);

        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text, "Целая реплика");
    }

    #[test]
    fn пустой_файл_даёт_пустой_список() {
        assert!(parse_srt("").is_empty());
        assert!(parse_srt("\n\n\n").is_empty());
    }

    #[test]
    fn реплики_без_текста_отбрасываются() {
        assert!(parse_srt("1\n00:00:01,000 --> 00:00:02,000\n\n").is_empty());
    }
}
