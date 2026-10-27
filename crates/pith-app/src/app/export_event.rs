//! Вести от рабочего потока выгрузки к окну.

use pith_notion::Report;
use pith_store::Sound as StoredSound;

use super::export_log::LogLine;

/// Вести от рабочего потока.
pub(super) enum ExportEvent {
    /// Строка журнала: что произошло и откуда взято значение.
    Log(LogLine),
    /// Спрошено слов из скольких — идёт транскрипция.
    Sound {
        done: usize,
        total: usize,
    },
    /// Новые слова: приложение положит их в хранилище.
    Words(Vec<(String, StoredSound)>),
    Step {
        done: usize,
        total: usize,
    },
    /// Нашлись повторы в базе: сколько и несколько примеров.
    Duplicates {
        count: usize,
        samples: Vec<String>,
    },
    Finished(Box<Result<Report, String>>),
}
