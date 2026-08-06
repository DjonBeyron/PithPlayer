//! Язык интерфейса.
//!
//! Живёт в хранилище, а не в приложении: выбор запоминается вместе
//! с остальными настройками и должен читаться тем же кодом.

use serde::{Deserialize, Serialize};

/// Язык интерфейса.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Русский — язык, на котором плеер написан.
    #[default]
    Ru,
    /// English.
    En,
}

impl Language {
    /// Все языки по порядку — для меню выбора.
    pub const ALL: [Self; 2] = [Self::Ru, Self::En];

    /// Название языка на нём самом: в списке ищут родное слово.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ru => "Русский",
            Self::En => "English",
        }
    }

    /// Номер языка — чтобы хранить выбор в одной ячейке памяти.
    pub fn code(self) -> u8 {
        match self {
            Self::Ru => 0,
            Self::En => 1,
        }
    }

    /// Язык по его номеру. Незнакомый номер — русский.
    pub fn from_code(code: u8) -> Self {
        match code {
            1 => Self::En,
            _ => Self::Ru,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Language;

    #[test]
    fn номер_языка_переживает_обратное_превращение() {
        for language in Language::ALL {
            assert_eq!(Language::from_code(language.code()), language);
        }
    }

    #[test]
    fn незнакомый_номер_даёт_русский() {
        assert_eq!(Language::from_code(200), Language::Ru);
    }
}
