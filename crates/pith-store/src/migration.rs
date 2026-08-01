//! Перенос данных из версии 4.
//!
//! Файлы v4 только читаются и никогда не изменяются: старый плеер должен
//! продолжать работать после миграции (PLAN.md §6.10).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::file_key::file_key;
use crate::watch_positions::{WatchPosition, WatchPositions, is_worth_remembering, now_unix};

/// Итог переноса — показывается пользователю.
#[derive(Debug, Default, PartialEq)]
pub struct MigrationReport {
    /// Сколько позиций просмотра перенесено.
    pub positions_moved: usize,
    /// Сколько пропущено: файлов уже нет на диске либо запись не нужна.
    pub positions_skipped: usize,
}

impl MigrationReport {
    pub fn is_empty(&self) -> bool {
        self.positions_moved == 0 && self.positions_skipped == 0
    }
}

/// Переносит позиции просмотра из `watch_positions.ini` версии 4.
///
/// Ключи пересчитываются: в v4 в них входил путь, поэтому переименование
/// файла теряло позицию. Записи, чьих файлов уже нет, пропускаем — они
/// не сработали бы и в v4.
pub fn migrate_watch_positions(v4_file: &Path, target: &mut WatchPositions) -> MigrationReport {
    let Ok(content) = std::fs::read_to_string(v4_file) else {
        return MigrationReport::default();
    };

    let mut report = MigrationReport::default();

    for entry in parse_ini_sections(&content) {
        let Some(record) = entry.to_record() else {
            report.positions_skipped += 1;
            continue;
        };

        // Ключ считаем по живому файлу: старый ключ несовместим с новым.
        let Some(key) = file_key(&record.path) else {
            tracing::debug!(?record.path, "файла нет на диске, позицию пропускаю");
            report.positions_skipped += 1;
            continue;
        };

        if !is_worth_remembering(record.position, record.duration) {
            report.positions_skipped += 1;
            continue;
        }

        target.insert_raw(
            key,
            WatchPosition {
                file_name: record.file_name,
                position: record.position,
                duration: record.duration,
                last_watched: now_unix(),
            },
        );
        report.positions_moved += 1;
    }

    if report.positions_moved > 0 {
        target.save();
    }

    tracing::info!(
        перенесено = report.positions_moved,
        пропущено = report.positions_skipped,
        "перенос позиций просмотра из версии 4"
    );

    report
}

/// Разобранная запись из файла v4.
struct V4Record {
    path: PathBuf,
    file_name: String,
    position: f64,
    duration: f64,
}

/// Секция INI-файла: набор пар «ключ — значение».
struct IniSection {
    values: HashMap<String, String>,
}

impl IniSection {
    fn to_record(&self) -> Option<V4Record> {
        let path = self.values.get("FilePath")?;

        // В v4 время хранится в миллисекундах.
        let position_ms: f64 = self.values.get("Position")?.parse().ok()?;
        let duration_ms: f64 = self.values.get("Duration")?.parse().ok()?;

        Some(V4Record {
            path: PathBuf::from(path),
            file_name: self
                .values
                .get("FileName")
                .cloned()
                .unwrap_or_else(|| file_name_of(path)),
            position: position_ms / 1000.0,
            duration: duration_ms / 1000.0,
        })
    }
}

fn file_name_of(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Разбирает INI на секции. Комментарии и пустые строки пропускаются.
fn parse_ini_sections(content: &str) -> Vec<IniSection> {
    let mut sections = Vec::new();
    let mut current: Option<IniSection> = None;

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            if let Some(section) = current.take() {
                sections.push(section);
            }
            current = Some(IniSection {
                values: HashMap::new(),
            });
            continue;
        }

        if let Some((key, value)) = line.split_once('=')
            && let Some(section) = current.as_mut()
        {
            section
                .values
                .insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    if let Some(section) = current {
        sections.push(section);
    }

    sections
}

#[cfg(test)]
mod tests {
    use super::*;

    const ОБРАЗЕЦ_V4: &str = "\
; Watch Positions for Media Player
; Generated: 2025-11-20 10:00:00

[abc123]
FilePath=C:\\кино\\фильм.mkv
FileName=фильм.mkv
Position=2700000
Duration=5400000
LastWatched=2025-11-20 10:00:00
FileSize=1234567
FileHash=abc123

[def456]
FilePath=C:\\кино\\второй.mkv
FileName=второй.mkv
Position=60000
Duration=7200000
LastWatched=2025-11-19 22:00:00
FileSize=7654321
FileHash=def456
";

    #[test]
    fn разбирает_секции_и_пропускает_комментарии() {
        let sections = parse_ini_sections(ОБРАЗЕЦ_V4);
        assert_eq!(sections.len(), 2);
        assert_eq!(
            sections[0].values.get("FileName"),
            Some(&"фильм.mkv".to_string())
        );
    }

    #[test]
    fn переводит_миллисекунды_в_секунды() {
        let sections = parse_ini_sections(ОБРАЗЕЦ_V4);
        let record = sections[0].to_record().expect("запись разбирается");

        assert_eq!(record.position, 2700.0);
        assert_eq!(record.duration, 5400.0);
    }

    #[test]
    fn запись_без_обязательных_полей_пропускается() {
        let sections = parse_ini_sections("[x]\nFileName=нет-пути.mkv\n");
        assert!(sections[0].to_record().is_none());
    }

    #[test]
    fn пустой_файл_не_ломает_разбор() {
        assert!(parse_ini_sections("").is_empty());
        assert!(parse_ini_sections("; только комментарий\n").is_empty());
    }

    #[test]
    fn отсутствующий_файл_версии_4_не_ошибка() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let mut target = WatchPositions::load(crate::DataPaths::with_root(dir.path()));

        let report = migrate_watch_positions(&dir.path().join("нет.ini"), &mut target);
        assert!(report.is_empty());
    }

    #[test]
    fn записи_несуществующих_файлов_пропускаются() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let v4 = dir.path().join("watch_positions.ini");
        std::fs::write(&v4, ОБРАЗЕЦ_V4).expect("запись образца");

        let mut target = WatchPositions::load(crate::DataPaths::with_root(dir.path()));
        let report = migrate_watch_positions(&v4, &mut target);

        // Файлов C:\кино\... на диске нет — переносить нечего.
        assert_eq!(report.positions_moved, 0);
        assert_eq!(report.positions_skipped, 2);
        assert!(target.is_empty());
    }

    #[test]
    fn позиция_существующего_файла_переносится() {
        let dir = tempfile::tempdir().expect("временный каталог");

        // Файл должен существовать: ключ считается по его метаданным.
        let video = dir.path().join("фильм.mkv");
        std::fs::write(&video, "не настоящее видео").expect("создание файла");

        let ini = format!(
            "[hash]\nFilePath={}\nFileName=фильм.mkv\nPosition=2700000\nDuration=5400000\n",
            video.display()
        );
        let v4 = dir.path().join("watch_positions.ini");
        std::fs::write(&v4, ini).expect("запись ini");

        let mut target = WatchPositions::load(crate::DataPaths::with_root(dir.path()));
        let report = migrate_watch_positions(&v4, &mut target);

        assert_eq!(report.positions_moved, 1);
        assert_eq!(target.len(), 1);

        // Файл узнаётся по новому ключу.
        let restored = target.resume_position(&video).expect("позиция найдена");
        assert_eq!(restored.position, 2700.0);
    }

    #[test]
    fn файл_версии_4_не_изменяется() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let v4 = dir.path().join("watch_positions.ini");
        std::fs::write(&v4, ОБРАЗЕЦ_V4).expect("запись образца");

        let mut target = WatchPositions::load(crate::DataPaths::with_root(dir.path()));
        migrate_watch_positions(&v4, &mut target);

        let after = std::fs::read_to_string(&v4).expect("чтение после миграции");
        assert_eq!(
            after, ОБРАЗЕЦ_V4,
            "файл версии 4 обязан остаться нетронутым"
        );
    }
}
