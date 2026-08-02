//! Поиск по репликам субтитров.

use crate::parse::Cue;

/// Найденная реплика.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    /// Куда перематывать, секунды.
    pub start: f64,
    /// Текст одной строкой.
    pub text: String,
}

/// Ищет реплики, содержащие запрос.
///
/// Сравнение без учёта регистра. Пустой запрос ничего не находит —
/// показывать весь файл списком бессмысленно.
pub fn search(cues: &[Cue], query: &str, limit: usize) -> Vec<SearchHit> {
    let query = query.trim().to_lowercase();

    if query.is_empty() {
        return Vec::new();
    }

    cues.iter()
        .filter(|cue| cue.text.to_lowercase().contains(&query))
        .take(limit)
        .map(|cue| SearchHit {
            start: cue.start,
            text: cue.single_line(),
        })
        .collect()
}

/// Реплика, звучащая в заданный момент.
pub fn cue_at(cues: &[Cue], time: f64) -> Option<&Cue> {
    cues.iter().find(|cue| time >= cue.start && time <= cue.end)
}

/// Ближайшая реплика после заданного момента.
///
/// Нужна переходу к следующей реплике, когда сейчас тишина.
pub fn next_cue(cues: &[Cue], time: f64) -> Option<&Cue> {
    cues.iter().find(|cue| cue.start > time)
}

/// Ближайшая реплика перед заданным моментом.
pub fn previous_cue(cues: &[Cue], time: f64) -> Option<&Cue> {
    cues.iter().rev().find(|cue| cue.start < time)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn реплики() -> Vec<Cue> {
        vec![
            Cue {
                start: 1.0,
                end: 3.0,
                text: "Привет, мир".into(),
            },
            Cue {
                start: 5.0,
                end: 7.0,
                text: "Как дела?".into(),
            },
            Cue {
                start: 10.0,
                end: 12.0,
                text: "Привет ещё раз\nи снова".into(),
            },
        ]
    }

    #[test]
    fn находит_по_подстроке() {
        let hits = search(&реплики(), "привет", 50);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].start, 1.0);
    }

    #[test]
    fn регистр_не_важен() {
        assert_eq!(search(&реплики(), "ПРИВЕТ", 50).len(), 2);
    }

    #[test]
    fn многострочная_реплика_показывается_одной_строкой() {
        let hits = search(&реплики(), "ещё", 50);
        assert_eq!(hits[0].text, "Привет ещё раз и снова");
    }

    #[test]
    fn пустой_запрос_ничего_не_находит() {
        assert!(search(&реплики(), "", 50).is_empty());
        assert!(search(&реплики(), "   ", 50).is_empty());
    }

    #[test]
    fn ограничение_числа_результатов_работает() {
        assert_eq!(search(&реплики(), "привет", 1).len(), 1);
    }

    #[test]
    fn ничего_не_найдено_даёт_пустой_список() {
        assert!(search(&реплики(), "отсутствует", 50).is_empty());
    }

    #[test]
    fn находит_реплику_текущего_момента() {
        let cues = реплики();
        assert_eq!(cue_at(&cues, 2.0).map(|c| c.start), Some(1.0));
        assert_eq!(cue_at(&cues, 4.0), None, "между репликами тишина");
    }

    #[test]
    fn переходит_к_следующей_реплике() {
        let cues = реплики();
        assert_eq!(next_cue(&cues, 2.0).map(|c| c.start), Some(5.0));
        assert_eq!(next_cue(&cues, 99.0), None);
    }

    #[test]
    fn переходит_к_предыдущей_реплике() {
        let cues = реплики();
        assert_eq!(previous_cue(&cues, 6.0).map(|c| c.start), Some(5.0));
        assert_eq!(previous_cue(&cues, 0.5), None);
    }
}
