//! Словарь транскрипций, вложенный в поставку.
//!
//! Слова копятся у каждого свои — из тех фильмов, что он смотрит. Но
//! половина реплик состоит из слов, которые есть в любом фильме, и
//! спрашивать о них словари на каждой новой машине незачем: собранное
//! однажды едет вместе с плеером.
//!
//! Наполняется файл `scripts\collect_dictionary.ps1` перед выпуском.
//!
//! Лежит внутри самого exe, а не файлом рядом: так он одинаково достаётся
//! и установленному плееру, и распакованному из архива, и потерять его
//! по дороге нельзя.

use std::collections::BTreeMap;

use pith_store::{Sound, SoundStore};

/// Словарь поставки, каким он был на сборке.
const PACKED: &str = include_str!("../../assets/dictionary.json");

/// Файл словаря.
#[derive(serde::Deserialize)]
struct Packed {
    words: BTreeMap<String, Sound>,
}

/// Добавляет слова поставки к словарю пользователя.
///
/// Свои слова главнее: подменять добытое на этой машине привезённой копией
/// незачем. Работа идёт один раз — на втором запуске добавлять нечего,
/// и файл не переписывается.
///
/// Битый словарь поставки не беда: плеер просто спросит слова у сайтов,
/// как делал раньше.
pub(super) fn seed(store: &mut SoundStore) {
    let packed: Packed = match serde_json::from_str(PACKED) {
        Ok(packed) => packed,
        Err(e) => {
            tracing::warn!(error = %e, "словарь поставки не разобрать");
            return;
        }
    };

    let added = store.seed(packed.words);

    tracing::debug!(прибавилось = added, всего = store.len(), "словарь поставки");
}

#[cfg(test)]
mod tests {
    use super::{PACKED, Packed};

    /// Словарь едет в каждой поставке, и разобрать его должно получаться
    /// всегда: собран он скриптом, а скрипты правят руками.
    #[test]
    fn словарь_поставки_разбирается() {
        let packed: Packed = serde_json::from_str(PACKED).expect("словарь поставки разобран");

        for (key, sound) in &packed.words {
            assert!(!key.is_empty(), "слово без ключа");
            assert!(
                sound.transcription.starts_with('|') && sound.transcription.ends_with('|'),
                "транскрипция без обрамляющих черт: {key} — {}",
                sound.transcription
            );
        }
    }
}
