//! Хранилище данных пользователя: настройки, позиции просмотра, закладки.
//!
//! Запись атомарная, форматы версионируются, повреждённый файл не роняет
//! плеер (CLAUDE.md, раздел «Данные пользователя»).

mod bookmarks;
mod error;
mod file;
mod file_key;
mod history;
mod language;
mod migration;
mod migration_v4_settings;
mod paths;
mod settings;
mod subtitle_layout;
mod subtitle_priority;
mod watch_positions;

pub use bookmarks::{
    BookmarkList, Bookmarks, DEFAULT_LIST, ListError, TimeBookmark, VideoBookmarks,
};
pub use error::{Result, StoreError};
pub use file_key::{FileKey, file_key, key_from_parts};
pub use history::{CAPACITY as HISTORY_CAPACITY, History};
pub use language::Language;
pub use migration::{MigrationReport, migrate_watch_positions};
pub use migration_v4_settings::{migrate_bookmarks, migrate_settings, migrate_subtitle_priority};
pub use paths::DataPaths;
pub use settings::{FragmentSettings, Settings, WindowGeometry};
pub use subtitle_layout::SubtitleLayout;
pub use subtitle_priority::{SubtitlePriority, score as tag_score};
pub use watch_positions::{WatchPosition, WatchPositions, is_worth_remembering};
