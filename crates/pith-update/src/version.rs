//! Сравнение номеров версий.
//!
//! Номер у плеера простой — три числа через точку, — и разбирать его
//! библиотекой семантического версионирования незачем. Зато нужно быть
//! стойким к тому, что придёт с GitHub: метка выпуска пишется руками
//! и приходит то `v5.1.41`, то `5.1.41`, а то и с лишним пробелом.

/// Разобранный номер версии.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl Version {
    /// Разбирает `5.1.41` или `v5.1.41`.
    ///
    /// `None` — номер не разобрать: тогда обновление просто не предлагается.
    /// Молча промолчать здесь лучше, чем звать на установку неизвестно чего.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().trim_start_matches(['v', 'V']);

        let mut parts = text.split('.');
        let major = parts.next()?.trim().parse().ok()?;
        let minor = parts.next()?.trim().parse().ok()?;

        // Хвост после третьего числа отбрасываем: у метки может оказаться
        // приписка вроде `5.1.41-beta`, и она сравнению не помеха.
        let patch = parts
            .next()
            .unwrap_or("0")
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap_or("0")
            .parse()
            .ok()?;

        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

/// Новее ли выпуск, чем то, что запущено сейчас.
///
/// Строго новее: та же версия обновлением не считается, иначе плеер
/// предлагал бы поставить сам себя.
pub fn is_newer(current: &str, released: &str) -> bool {
    match (Version::parse(current), Version::parse(released)) {
        (Some(current), Some(released)) => released > current,
        _ => {
            tracing::warn!(current, released, "номер версии не разобрать");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Version, is_newer};

    #[test]
    fn метка_с_буквой_и_без_разбирается_одинаково() {
        assert_eq!(Version::parse("v5.1.41"), Version::parse("5.1.41"));
        assert_eq!(Version::parse(" 5.1.41 "), Version::parse("5.1.41"));
    }

    #[test]
    fn сравнение_идёт_по_числам_а_не_по_буквам() {
        // Строкой «5.1.9» больше «5.1.41» — ровно та ошибка, ради которой
        // номер и разбирается на числа.
        assert!(is_newer("5.1.9", "5.1.41"));
        assert!(is_newer("5.1.41", "5.2.0"));
        assert!(is_newer("5.9.9", "6.0.0"));
    }

    #[test]
    fn та_же_и_более_старая_версия_обновлением_не_считаются() {
        assert!(!is_newer("5.1.41", "5.1.41"));
        assert!(!is_newer("5.1.41", "5.1.40"));
        assert!(!is_newer("5.1.41", "v5.1.41"));
    }

    #[test]
    fn приписка_к_номеру_не_мешает() {
        assert!(is_newer("5.1.41", "v5.1.42-beta"));
    }

    #[test]
    fn неразборчивый_номер_обновлением_не_считается() {
        assert!(!is_newer("5.1.41", "последняя"));
        assert!(!is_newer("5.1.41", ""));
        assert!(!is_newer("", "5.1.42"));
    }
}
