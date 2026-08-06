//! Где лежат данные пользователя.
//!
//! По умолчанию `%APPDATA%\PithPlayer` — не теряется при переустановке.
//! Если рядом с программой лежит `portable.txt`, данные хранятся там же
//! (PLAN.md §6.10).

use std::path::{Path, PathBuf};

/// Имя файла-признака переносного режима.
const PORTABLE_MARKER: &str = "portable.txt";

/// Каталог с данными приложения.
#[derive(Debug, Clone)]
pub struct DataPaths {
    root: PathBuf,
}

impl DataPaths {
    /// Определяет каталог данных: рядом с программой либо в профиле.
    pub fn discover() -> Self {
        if let Some(dir) = portable_dir() {
            tracing::info!(?dir, "переносной режим: данные рядом с программой");
            return Self { root: dir };
        }

        let root = appdata_dir().unwrap_or_else(|| {
            // Профиля нет — кладём рядом с программой. Плеер должен
            // работать, даже если система устроена необычно.
            tracing::warn!("каталог профиля недоступен, использую папку программы");
            exe_dir().unwrap_or_else(|| PathBuf::from("."))
        });

        Self { root }
    }

    /// Каталог данных для явно заданного корня. Нужен тестам.
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn settings(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    pub fn watch_positions(&self) -> PathBuf {
        self.root.join("watch_positions.json")
    }

    pub fn bookmarks(&self) -> PathBuf {
        self.root.join("bookmarks.json")
    }

    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// История открытых файлов и папок.
    pub fn history(&self) -> PathBuf {
        self.root.join("history.json")
    }

    /// Каталог мозаик миниатюр для предпросмотра на полосе перемотки.
    ///
    /// Содержимое восстановимо: его можно удалить в любой момент, плеер
    /// соберёт мозаику заново.
    pub fn thumbnails(&self) -> PathBuf {
        self.root.join("thumbs")
    }

    /// Создаёт каталог данных, если его ещё нет.
    pub fn ensure_exists(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.root)
    }
}

/// Каталог программы, если рядом лежит признак переносного режима.
fn portable_dir() -> Option<PathBuf> {
    let dir = exe_dir()?;
    dir.join(PORTABLE_MARKER).exists().then_some(dir)
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(|p| p.to_path_buf())
}

fn appdata_dir() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(appdata).join("PithPlayer"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn файлы_лежат_в_корне_данных() {
        let paths = DataPaths::with_root("C:\\данные");

        assert_eq!(paths.settings(), Path::new("C:\\данные\\settings.json"));
        assert_eq!(
            paths.watch_positions(),
            Path::new("C:\\данные\\watch_positions.json")
        );
        assert_eq!(paths.bookmarks(), Path::new("C:\\данные\\bookmarks.json"));
    }
}
