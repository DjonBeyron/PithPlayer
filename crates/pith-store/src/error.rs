//! Ошибки хранилища.

use std::path::{Path, PathBuf};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("не удалось прочитать файл «{path}»: {source_msg}")]
    Read { path: PathBuf, source_msg: String },

    #[error("не удалось записать файл «{path}»: {source_msg}")]
    Write { path: PathBuf, source_msg: String },

    #[error("не удалось подготовить данные к записи: {0}")]
    Serialize(#[source] serde_json::Error),
}

impl StoreError {
    pub(crate) fn read(path: impl AsRef<Path>, source: std::io::Error) -> Self {
        Self::Read {
            path: path.as_ref().to_path_buf(),
            source_msg: source.to_string(),
        }
    }

    pub(crate) fn write(path: impl AsRef<Path>, source: std::io::Error) -> Self {
        Self::Write {
            path: path.as_ref().to_path_buf(),
            source_msg: source.to_string(),
        }
    }
}
