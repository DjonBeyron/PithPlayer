//! Логирование. Замена `SubtitleLogger` и `Debug.WriteLine` из v4.
//!
//! `println!` в коде запрещён — всё идёт через `tracing` (CLAUDE.md).

use std::fs::File;
use std::path::PathBuf;

use tracing_subscriber::EnvFilter;

/// Настраивает вывод логов.
///
/// В отладочной сборке — в консоль. В релизной консоли нет
/// (`windows_subsystem = "windows"`), поэтому пишем в файл рядом
/// с исполняемым файлом.
///
/// Уровень задаётся переменной окружения `PITH_LOG`
/// (`error`, `warn`, `info`, `debug`, `trace`). По умолчанию `info`.
pub fn init() {
    let filter = EnvFilter::try_from_env("PITH_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false);

    match log_file() {
        Some(file) => builder.with_writer(file).with_ansi(false).init(),
        None => builder.with_ansi(true).init(),
    }
}

/// Файл для записи логов. `None` — писать в консоль.
///
/// Отсутствие файла не должно мешать запуску: не смогли создать —
/// просто останемся без файлового лога.
fn log_file() -> Option<File> {
    if cfg!(debug_assertions) {
        return None;
    }

    let path = log_path()?;
    File::create(&path).ok()
}

fn log_path() -> Option<PathBuf> {
    let mut path = std::env::current_exe().ok()?;
    path.set_file_name("pith-player.log");
    Some(path)
}

/// Перехват паник: пишем в лог вместо тихого падения.
///
/// Плеер не должен исчезать без объяснений (CLAUDE.md, раздел «Надёжность»).
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("аварийное завершение: {info}");
        default_hook(info);
    }));
}
