//! Ключ, по которому узнаётся файл.
//!
//! Считается от размера и даты изменения — **без пути и имени**. Благодаря
//! этому переименование или перенос файла не теряет позицию просмотра
//! (в v4 путь входил в ключ, и позиция терялась).

use std::path::Path;
use std::time::UNIX_EPOCH;

/// Ключ файла в хранилище.
pub type FileKey = String;

/// Считает ключ по метаданным файла.
///
/// Возвращает `None`, если файла нет или метаданные недоступны.
pub fn file_key(path: &Path) -> Option<FileKey> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let seconds = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();

    Some(key_from_parts(metadata.len(), seconds))
}

/// Ключ из размера и времени изменения.
///
/// Вынесено отдельно, чтобы алгоритм можно было проверить без файлов.
pub fn key_from_parts(size: u64, modified_secs: u64) -> FileKey {
    let source = format!("{size}|{modified_secs}");
    format!("{:016x}", fnv1a64(source.as_bytes()))
}

/// FNV-1a, 64 бита.
///
/// Своя реализация вместо зависимости: алгоритм в пять строк, а главное —
/// он неизменен. Штатный `DefaultHasher` не обещает одинаковый результат
/// между версиями Rust, и однажды все сохранённые позиции перестали бы
/// находиться.
fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    bytes.iter().fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn одинаковые_данные_дают_одинаковый_ключ() {
        assert_eq!(
            key_from_parts(1024, 1700000000),
            key_from_parts(1024, 1700000000)
        );
    }

    #[test]
    fn разный_размер_даёт_разные_ключи() {
        assert_ne!(
            key_from_parts(1024, 1700000000),
            key_from_parts(2048, 1700000000)
        );
    }

    #[test]
    fn разное_время_даёт_разные_ключи() {
        assert_ne!(
            key_from_parts(1024, 1700000000),
            key_from_parts(1024, 1700000001)
        );
    }

    #[test]
    fn ключ_фиксированной_длины() {
        assert_eq!(key_from_parts(0, 0).len(), 16);
        assert_eq!(key_from_parts(u64::MAX, u64::MAX).len(), 16);
    }

    /// Алгоритм не должен «поплыть»: иначе все сохранённые позиции
    /// перестанут находиться после обновления плеера.
    #[test]
    fn алгоритм_закреплён_эталонным_значением() {
        assert_eq!(key_from_parts(1024, 1700000000), "ea87b02a5845f1f2");
    }

    #[test]
    fn ключ_несуществующего_файла_не_считается() {
        assert_eq!(file_key(Path::new("не-существует-точно.mkv")), None);
    }
}
