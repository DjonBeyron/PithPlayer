//! Имя файла — в запрос к базе фильмов.
//!
//! Файлы приходят с раздач: `Obsession.2025.2160p.MA.WEB-DL.DDP5.1.DV.HDR.H.265.mkv`.
//! Из такого имени нужно достать название и год, а всё, что описывает саму
//! раздачу — разрешение, кодек, источник, группу, — выбросить.

/// Что удалось понять из имени файла.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// Название для поиска.
    pub title: String,
    /// Год выпуска, если он был в имени.
    pub year: Option<u32>,
}

/// Годы, которые считаем правдоподобными.
///
/// Ниже — уже не год, а часть названия («Ford v Ferrari 1966»), выше —
/// опечатка. Кино снимают с конца XIX века.
const FIRST_YEAR: u32 = 1888;
const LAST_YEAR: u32 = 2999;

/// Слова, которыми подписывают раздачу, а не фильм.
///
/// Всё, что начинается с такого слова, к названию отношения не имеет:
/// дальше идут только технические подробности.
const RELEASE_WORDS: &[&str] = &[
    "2160p", "1080p", "1080i", "720p", "480p", "4k", "uhd", "hdr", "hdr10", "dv", "sdr", "webrip",
    "web", "webdl", "bluray", "bdrip", "brrip", "dvdrip", "hdtv", "hdrip", "camrip", "remux",
    "x264", "x265", "h264", "h265", "hevc", "avc", "xvid", "divx", "aac", "ac3", "dts", "ddp5",
    "ddp", "dd5", "flac", "opus", "atmos", "truehd", "mvo", "dvo", "avo", "dub", "sub", "rus",
    "eng", "multi", "proper", "repack", "extended", "unrated", "imax", "ma", "amzn", "nf", "hmax",
    "dsnp", "atvp", "complete",
];

/// Разбирает имя файла.
///
/// `None` — от имени ничего не осталось: одни технические слова или пусто.
pub fn parse(file_name: &str) -> Option<Query> {
    let stem = strip_extension(file_name);
    let words = split_words(stem);

    let mut title: Vec<&str> = Vec::new();
    let mut year = None;

    for word in words {
        // Номер серии обрывает название так же, как технические слова:
        // дальше идёт подпись раздачи, а не имя картины.
        if is_episode(word) {
            break;
        }

        // Год до названия — это само название: «1917», «2012».
        if !title.is_empty()
            && let Some(found) = as_year(word)
        {
            // Год берём первый: дальше могут идти числа из подписи раздачи.
            year.get_or_insert(found);
            break;
        }

        if is_release_word(word) {
            break;
        }

        title.push(word);
    }

    let title = title.join(" ").trim().to_string();

    (!title.is_empty()).then_some(Query { title, year })
}

/// Убирает расширение, если оно похоже на расширение.
///
/// Похоже — это короткий хвост из одних букв: `mkv`, `mp4`. Длинный хвост
/// или хвост с цифрами расширением не считаем, иначе `Interstellar.2014`
/// лишится года, а `Mr. Robot` — половины названия.
fn strip_extension(name: &str) -> &str {
    match name.rsplit_once('.') {
        Some((stem, ext))
            if (1..=4).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphabetic()) =>
        {
            stem
        }
        _ => name,
    }
}

/// Делит имя на слова: точки, подчёркивания и скобки в раздачах вместо пробелов.
fn split_words(stem: &str) -> impl Iterator<Item = &str> {
    stem.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
}

fn as_year(word: &str) -> Option<u32> {
    if word.len() != 4 {
        return None;
    }

    word.parse()
        .ok()
        .filter(|y| (FIRST_YEAR..=LAST_YEAR).contains(y))
}

fn is_release_word(word: &str) -> bool {
    let lower = word.to_ascii_lowercase();
    RELEASE_WORDS.contains(&lower.as_str())
}

/// Похоже ли слово на номер серии: `s01e05`, `s01`, `e05`.
fn is_episode(word: &str) -> bool {
    let lower = word.to_ascii_lowercase();
    let mut chars = lower.chars();

    match chars.next() {
        Some('s') | Some('e') => {}
        _ => return false,
    }

    let rest: String = chars.collect();
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == 'e')
}

#[cfg(test)]
mod tests {
    use super::{Query, parse};

    fn запрос(title: &str, year: Option<u32>) -> Option<Query> {
        Some(Query {
            title: title.into(),
            year,
        })
    }

    #[test]
    fn название_и_год_из_имени_раздачи() {
        assert_eq!(
            parse("Obsession.2025.2160p.MA.WEB-DL.DDP5.1.DV.HDR.H.265.mkv"),
            запрос("Obsession", Some(2025))
        );
    }

    #[test]
    fn название_из_нескольких_слов() {
        assert_eq!(
            parse("The.Shawshank.Redemption.1994.1080p.BluRay.x264.mkv"),
            запрос("The Shawshank Redemption", Some(1994))
        );
    }

    #[test]
    fn номер_серии_обрывает_название() {
        assert_eq!(
            parse("Breaking.Bad.S01E05.1080p.WEB-DL.mkv"),
            запрос("Breaking Bad", None)
        );
    }

    #[test]
    fn имя_без_подписи_раздачи() {
        assert_eq!(
            parse("Тайна третьей планеты.mkv"),
            запрос("Тайна третьей планеты", None)
        );
    }

    #[test]
    fn пробелы_и_скобки_вместо_точек() {
        assert_eq!(
            parse("Dune Part Two (2024) [2160p].mkv"),
            запрос("Dune Part Two", Some(2024))
        );
    }

    #[test]
    fn число_в_названии_не_год() {
        // 1917 — год, и он же название. Первое число всё равно год выпуска:
        // отличить их в имени файла нельзя, а поиск по названию с годом
        // находит картину и так.
        assert_eq!(parse("1917.2019.1080p.mkv"), запрос("1917", Some(2019)));
    }

    #[test]
    fn имя_без_расширения_разбирается() {
        assert_eq!(
            parse("Interstellar.2014"),
            запрос("Interstellar", Some(2014))
        );
    }

    #[test]
    fn из_одних_технических_слов_ничего_не_выходит() {
        assert_eq!(parse("1080p.x264.mkv"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn длинный_хвост_не_считается_расширением() {
        // Точка в названии — не расширение, и обрезать по ней нельзя.
        assert_eq!(parse("Mr. Robot"), запрос("Mr Robot", None));
    }
}
