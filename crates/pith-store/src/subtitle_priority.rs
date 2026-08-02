//! Правила автовыбора дорожек субтитров и аудио.
//!
//! Перенос `SubtitlePrioritySettings` из v4 (PLAN.md §6.3): дорожка
//! выбирается по тегам в названии и языке.

use serde::{Deserialize, Serialize};

/// Настройки автовыбора.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubtitlePriority {
    /// Теги для основных субтитров, по убыванию важности.
    pub main_tags: Vec<String>,
    /// Включать основные субтитры автоматически.
    pub main_enabled: bool,

    /// Теги для вторых субтитров.
    pub secondary_tags: Vec<String>,
    /// Включать вторые субтитры автоматически.
    pub secondary_enabled: bool,

    /// Теги, по которым дорожку выбирать не следует.
    pub blacklist_tags: Vec<String>,

    /// Не выбирать дорожку, если ни один тег не совпал.
    pub skip_unmatched: bool,
}

impl Default for SubtitlePriority {
    /// Значения по умолчанию совпадают с v4.
    fn default() -> Self {
        Self {
            main_tags: ["sdh", "english", "eng", "full", "complete"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            main_enabled: true,
            secondary_tags: ["russian", "rus", "ru", "full", "forced"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            secondary_enabled: false,
            blacklist_tags: Vec::new(),
            skip_unmatched: false,
        }
    }
}

/// Насколько дорожка подходит под набор тегов.
///
/// Чем раньше тег в списке, тем он важнее. Ноль означает «ничего не совпало».
pub fn score(search_text: &str, tags: &[String], blacklist: &[String]) -> i32 {
    if tags.is_empty() {
        return 0;
    }

    if blacklist.iter().any(|tag| contains_tag(search_text, tag)) {
        return 0;
    }

    let total = tags.len() as i32;

    tags.iter()
        .enumerate()
        .filter(|(_, tag)| contains_tag(search_text, tag))
        // Вес убывает с позицией: первый тег ценнее последнего.
        .map(|(position, _)| total - position as i32)
        .sum()
}

/// Есть ли тег в тексте дорожки.
///
/// Сравнение по подстроке, как в v4: названия дорожек пишут кто во что горазд
/// («English SDH», «eng-forced», «Русские полные»).
fn contains_tag(search_text: &str, tag: &str) -> bool {
    if tag.is_empty() {
        return false;
    }
    search_text.contains(&tag.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn теги(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn совпадение_тега_даёт_вес() {
        let tags = теги(&["sdh", "english"]);
        assert!(score("english sdh eng", &tags, &[]) > 0);
    }

    #[test]
    fn ранние_теги_весомее_поздних() {
        let tags = теги(&["sdh", "english"]);

        let первый = score("sdh", &tags, &[]);
        let второй = score("english", &tags, &[]);

        assert!(
            первый > второй,
            "первый тег списка обязан весить больше: {первый} против {второй}"
        );
    }

    #[test]
    fn несовпадение_даёт_ноль() {
        assert_eq!(score("немецкие ger", &теги(&["eng", "english"]), &[]), 0);
    }

    #[test]
    fn чёрный_список_обнуляет_дорожку() {
        let tags = теги(&["english", "eng"]);
        let blacklist = теги(&["commentary"]);

        assert!(score("english commentary eng", &tags, &blacklist) == 0);
        assert!(score("english eng", &tags, &blacklist) > 0);
    }

    #[test]
    fn пустой_список_тегов_даёт_ноль() {
        assert_eq!(score("english", &[], &[]), 0);
    }

    #[test]
    fn регистр_не_важен() {
        let tags = теги(&["ENGLISH"]);
        assert!(score("english sdh", &tags, &[]) > 0);
    }

    #[test]
    fn совпадение_нескольких_тегов_суммируется() {
        let tags = теги(&["sdh", "english", "full"]);

        let один = score("english", &tags, &[]);
        let три = score("english sdh full", &tags, &[]);

        assert!(три > один, "несколько совпадений обязаны весить больше");
    }

    #[test]
    fn значения_по_умолчанию_как_в_версии_4() {
        let settings = SubtitlePriority::default();

        assert_eq!(settings.main_tags[0], "sdh");
        assert!(settings.main_enabled);
        assert_eq!(settings.secondary_tags[0], "russian");
        assert!(!settings.secondary_enabled);
    }
}
