//! Перенос данных из версии 4 при первом запуске.
//!
//! Файлы v4 только читаются: старый плеер должен продолжать работать
//! после миграции (PLAN.md §6.10).

use std::path::PathBuf;

use pith_store::{DataPaths, MigrationReport, WatchPositions, migrate_watch_positions};

/// Метка о том, что перенос уже выполнялся.
const MARKER: &str = "migrated_from_v4";

/// Какая часть переноса реализована.
///
/// Номер записывается в метку. Когда на следующих этапах добавятся закладки
/// и настройки субтитров, номер вырастет — и перенос доделает недостающее
/// даже у тех, кто уже запускал плеер. Без этого метка закрыла бы перенос
/// навсегда после первой же стадии.
const MIGRATION_STAGE: u32 = 2;

/// Известные места, где лежат данные версии 4.
const KNOWN_V4_DIRS: &[&str] = &[
    r"C:\PITH\Source\Pith player\bin\x64\Debug\net6.0-windows",
    r"C:\PITH\Source\Pith player\bin\x64\Release\net6.0-windows",
];

/// Выполняет перенос, если он ещё не делался.
///
/// Возвращает отчёт, если что-то перенесено. Любая неудача не мешает
/// запуску: плеер просто начнёт с чистыми данными.
pub fn run_once(
    paths: &DataPaths,
    positions: &mut WatchPositions,
    settings: &mut pith_store::Settings,
    bookmarks: &mut pith_store::Bookmarks,
    explicit_dir: Option<&str>,
) -> Option<MigrationReport> {
    let marker = paths.root().join(MARKER);

    if completed_stage(&marker) >= MIGRATION_STAGE {
        return None;
    }

    let source = explicit_dir
        .map(PathBuf::from)
        .or_else(find_v4_dir)
        .filter(|dir| dir.exists())?;

    tracing::info!(?source, "найдены данные версии 4, переношу");

    // Каталог данных нужен и для метки, и для переноса. Создаём его первым,
    // чтобы отметка о выполнении не зависела от порядка действий.
    if let Err(e) = paths.ensure_exists() {
        tracing::error!(error = %e, "нет каталога данных, перенос отложен до следующего запуска");
        return None;
    }

    let mut report = migrate_watch_positions(&source.join("watch_positions.ini"), positions);

    // Настройки переносим до закладок: длительность и отступ из них
    // становятся значениями для перенесённых списков.
    let settings_moved = pith_store::migrate_settings(&source.join("settings.ini"), settings);
    let subtitles_moved =
        pith_store::migrate_subtitle_priority(&source.join("subtitle_settings.ini"), settings);

    if settings_moved || subtitles_moved {
        settings.save(paths);
        report.settings_moved = true;
    }

    report.bookmarks_moved = pith_store::migrate_bookmarks(
        &source.join("bookmarks.json"),
        bookmarks,
        settings.fragments.duration_sec,
        settings.fragments.buffer_sec,
    );

    // Метку ставим в любом случае: повторять перенос не нужно, даже если
    // переносить было нечего.
    mark_done(&marker);

    (!report.is_empty()).then_some(report)
}

/// Ищет каталог с данными версии 4 в известных местах.
fn find_v4_dir() -> Option<PathBuf> {
    KNOWN_V4_DIRS
        .iter()
        .map(PathBuf::from)
        .find(|dir| dir.join("watch_positions.ini").exists() || dir.join("bookmarks.json").exists())
}

/// Какая стадия переноса уже выполнена.
///
/// Метка без номера осталась от первых сборок — считаем её первой стадией.
///
/// Метку порядка байтов срезаем: файл могли переписать блокнотом или
/// скриптом, и невидимый символ в начале превращал номер стадии в мусор —
/// перенос запускался заново и перетирал данные.
fn completed_stage(marker: &std::path::Path) -> u32 {
    match std::fs::read_to_string(marker) {
        Ok(content) => content
            .trim_start_matches('\u{feff}')
            .trim()
            .parse()
            .unwrap_or(1),
        Err(_) => 0,
    }
}

/// Отмечает перенос выполненным.
///
/// Без метки перенос повторится при следующем запуске. Повтор не портит
/// данные, но лишний раз перетирает позиции просмотра.
fn mark_done(marker: &std::path::Path) {
    if let Err(e) = std::fs::write(marker, MIGRATION_STAGE.to_string()) {
        tracing::error!(error = %e, ?marker, "не удалось отметить перенос выполненным");
    }
}

impl crate::app::PithApp {
    /// Итог переноса данных, пока пользователь его не закрыл.
    pub fn migration_report(&self) -> Option<&MigrationReport> {
        self.migration.as_ref()
    }

    pub fn dismiss_migration_report(&mut self) {
        self.migration = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn повторный_перенос_не_выполняется() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());
        paths.ensure_exists().expect("каталог данных");

        std::fs::write(dir.path().join(MARKER), MIGRATION_STAGE.to_string()).expect("метка");

        let mut positions = WatchPositions::load(paths.clone());
        let mut settings = pith_store::Settings::default();
        let mut bookmarks = pith_store::Bookmarks::load(paths.clone());

        assert!(
            run_once(
                &paths,
                &mut positions,
                &mut settings,
                &mut bookmarks,
                Some("C:\\что-угодно")
            )
            .is_none()
        );
    }

    #[test]
    fn старая_метка_без_номера_считается_первой_стадией() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let marker = dir.path().join(MARKER);
        std::fs::write(&marker, "").expect("метка без номера");

        assert_eq!(completed_stage(&marker), 1);
    }

    #[test]
    fn отсутствие_метки_означает_нулевую_стадию() {
        let dir = tempfile::tempdir().expect("временный каталог");
        assert_eq!(completed_stage(&dir.path().join(MARKER)), 0);
    }

    #[test]
    fn метка_порядка_байтов_не_ломает_номер_стадии() {
        // Так метку записывает PowerShell 5.1 через `Set-Content -Encoding utf8`.
        let dir = tempfile::tempdir().expect("временный каталог");
        let marker = dir.path().join(MARKER);
        std::fs::write(&marker, "\u{feff}2").expect("метка с BOM");

        assert_eq!(completed_stage(&marker), 2, "перенос повторился бы заново");
    }

    /// Когда добавятся закладки, номер стадии вырастет — и перенос
    /// доделает недостающее у тех, кто уже запускал плеер.
    #[test]
    fn новая_стадия_запускает_перенос_повторно() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let marker = dir.path().join(MARKER);
        std::fs::write(&marker, "1").expect("метка первой стадии");

        assert!(
            completed_stage(&marker) < 2,
            "стадия 2 обязана считаться невыполненной"
        );
    }

    #[test]
    fn отсутствие_данных_версии_4_не_ошибка() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        let mut positions = WatchPositions::load(paths.clone());
        let mut settings = pith_store::Settings::default();
        let mut bookmarks = pith_store::Bookmarks::load(paths.clone());

        let report = run_once(
            &paths,
            &mut positions,
            &mut settings,
            &mut bookmarks,
            Some("C:\\нет-такой-папки"),
        );

        assert!(report.is_none());
    }
}
