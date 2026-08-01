//! Хранилище данных пользователя: настройки, позиции просмотра, закладки.
//!
//! Запись атомарная, форматы версионируются, повреждённый файл не роняет
//! плеер (CLAUDE.md, раздел «Данные пользователя»).

mod error;
mod file;
mod file_key;
mod paths;
mod watch_positions;

pub use error::{Result, StoreError};
pub use file_key::{FileKey, file_key, key_from_parts};
pub use paths::DataPaths;
pub use watch_positions::{WatchPosition, WatchPositions, is_worth_remembering};
