//! Хранилище данных пользователя: настройки, позиции просмотра, закладки.
//!
//! Запись атомарная, форматы версионируются, повреждённый файл не роняет
//! плеер (CLAUDE.md, раздел «Данные пользователя»).

mod bookmarks;
mod error;
mod file;
mod file_key;
mod migration;
mod migration_v4_settings;
mod paths;
mod settings;
mod subtitle_priority;
mod watch_positions;

pub use bookmarks::{
    BookmarkList, Bookmarks, DEFAULT_LIST, ListError, TimeBookmark, VideoBookmarks,
};
pub use error::{Result, StoreError};
pub use file_key::{FileKey, file_key, key_from_parts};
pub use migration::{MigrationReport, migrate_watch_positions};
pub use migration_v4_settings::{migrate_bookmarks, migrate_settings, migrate_subtitle_priority};
pub use paths::DataPaths;
pub use settings::{FragmentSettings, Settings, SubtitleLayout};
pub use subtitle_priority::{SubtitlePriority, score as tag_score};
pub use watch_positions::{WatchPosition, WatchPositions, is_worth_remembering};
