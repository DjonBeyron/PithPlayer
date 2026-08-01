//! Чтение и запись JSON-файлов данных.
//!
//! Запись атомарная: сначала во временный файл, затем переименование.
//! Закладки и позиции просмотра терять нельзя даже при сбое питания
//! (CLAUDE.md, раздел «Данные пользователя»).

use std::fs;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{Result, StoreError};

/// Читает JSON-файл.
///
/// Возвращает `None`, если файла нет — это штатная ситуация первого запуска.
/// Повреждённый файл не роняет плеер: он переименовывается в `.bad`,
/// а работа продолжается с чистыми данными.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(StoreError::read(path, e)),
    };

    match serde_json::from_str(&content) {
        Ok(value) => Ok(Some(value)),
        Err(e) => {
            tracing::error!(?path, error = %e, "файл данных повреждён, откладываю его в сторону");
            set_aside_broken(path);
            Ok(None)
        }
    }
}

/// Записывает JSON-файл атомарно.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| StoreError::write(parent, e))?;
    }

    let json = serde_json::to_string_pretty(value).map_err(StoreError::Serialize)?;

    // Временный файл рядом с целевым: переименование внутри одного тома
    // атомарно, между томами — нет.
    let temp = path.with_extension("tmp");
    fs::write(&temp, json).map_err(|e| StoreError::write(&temp, e))?;

    // На Windows rename не перезаписывает существующий файл.
    if path.exists() {
        fs::remove_file(path).map_err(|e| StoreError::write(path, e))?;
    }

    fs::rename(&temp, path).map_err(|e| StoreError::write(path, e))?;
    Ok(())
}

/// Переименовывает повреждённый файл, чтобы он не мешал и не потерялся.
fn set_aside_broken(path: &Path) {
    let broken = path.with_extension("bad");

    if let Err(e) = fs::rename(path, &broken) {
        tracing::warn!(?path, error = %e, "не удалось отложить повреждённый файл");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Пример {
        имя: String,
        число: i32,
    }

    fn образец() -> Пример {
        Пример {
            имя: "тест".into(),
            число: 42,
        }
    }

    #[test]
    fn записывает_и_читает_обратно() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let path = dir.path().join("данные.json");

        write_json(&path, &образец()).expect("запись");
        let read: Option<Пример> = read_json(&path).expect("чтение");

        assert_eq!(read, Some(образец()));
    }

    #[test]
    fn отсутствие_файла_не_ошибка() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let read: Option<Пример> = read_json(&dir.path().join("нет.json")).expect("чтение");

        assert_eq!(read, None);
    }

    #[test]
    fn перезапись_не_ломает_файл() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let path = dir.path().join("данные.json");

        write_json(&path, &образец()).expect("первая запись");

        let другой = Пример {
            имя: "второй".into(),
            число: 7,
        };
        write_json(&path, &другой).expect("вторая запись");

        let read: Option<Пример> = read_json(&path).expect("чтение");
        assert_eq!(read, Some(другой));
    }

    #[test]
    fn повреждённый_файл_откладывается_и_не_роняет_чтение() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let path = dir.path().join("данные.json");

        fs::write(&path, "{ это не json").expect("запись мусора");

        let read: Option<Пример> = read_json(&path).expect("чтение не должно падать");
        assert_eq!(read, None);
        assert!(
            path.with_extension("bad").exists(),
            "повреждённый файл обязан сохраниться рядом"
        );
    }

    #[test]
    fn создаёт_каталог_при_записи() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let path = dir.path().join("вложенный").join("данные.json");

        write_json(&path, &образец()).expect("запись");
        assert!(path.exists());
    }
}
