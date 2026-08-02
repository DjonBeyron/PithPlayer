//! Файловые ассоциации (PLAN.md §6.7).
//!
//! Пишем только в `HKCU\Software\Classes` — это настройки текущего
//! пользователя, права администратора не нужны и система целиком
//! не затрагивается.
//!
//! Вызывается **исключительно по явному действию пользователя**: изменение
//! ассоциаций меняет поведение системы, и делать это молча нельзя.

use std::path::Path;

use anyhow::{Context, Result};
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS};

/// Идентификатор типа файла в реестре.
const PROG_ID: &str = "PithPlayer.Video";

/// Расширения, которые предлагаем связать с плеером.
///
/// Тот же список, что и в диалоге открытия файла: связывать что-то,
/// чего плеер не открывает, бессмысленно.
pub const EXTENSIONS: [&str; 15] = [
    "mkv", "mp4", "avi", "mov", "webm", "ts", "m2ts", "m4v", "flv", "wmv", "mpg", "mpeg", "vob",
    "ogv", "3gp",
];

/// Связывает видеофайлы с плеером.
///
/// Возвращает число связанных расширений.
pub fn register(exe: &Path) -> Result<usize> {
    let classes = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags("Software\\Classes", KEY_ALL_ACCESS)
        .context("нет доступа к разделу реестра Software\\Classes")?;

    write_prog_id(&classes, exe)?;

    let mut linked = 0;
    for extension in EXTENSIONS {
        match link_extension(&classes, extension) {
            Ok(()) => linked += 1,
            Err(e) => tracing::warn!(extension, error = %e, "расширение не связано"),
        }
    }

    tracing::info!(linked, "видеофайлы связаны с плеером");
    Ok(linked)
}

/// Убирает ассоциации, возвращая систему к прежнему поведению.
///
/// Записи расширений удаляются только свои: чужой обработчик, лежащий
/// в том же ключе, не трогаем.
pub fn unregister() -> Result<()> {
    let classes = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags("Software\\Classes", KEY_ALL_ACCESS)
        .context("нет доступа к разделу реестра Software\\Classes")?;

    for extension in EXTENSIONS {
        let key = format!(".{extension}\\OpenWithProgids");
        if let Ok(progids) = classes.open_subkey_with_flags(&key, KEY_ALL_ACCESS) {
            let _ = progids.delete_value(PROG_ID);
        }
    }

    let _ = classes.delete_subkey_all(PROG_ID);
    tracing::info!("ассоциации сняты");
    Ok(())
}

/// Связаны ли файлы с плеером сейчас.
pub fn is_registered() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(format!("Software\\Classes\\{PROG_ID}"))
        .is_ok()
}

/// Описание типа файла: имя, иконка, команда запуска.
fn write_prog_id(classes: &RegKey, exe: &Path) -> Result<()> {
    let (prog_id, _) = classes
        .create_subkey(PROG_ID)
        .context("не удалось создать описание типа файла")?;
    prog_id.set_value("", &"Видео Pith Player")?;

    let exe = exe.display().to_string();

    let (icon, _) = prog_id.create_subkey("DefaultIcon")?;
    icon.set_value("", &format!("\"{exe}\",0"))?;

    // `%1` в кавычках: без них путь с пробелами приходит по частям.
    let (command, _) = prog_id.create_subkey("shell\\open\\command")?;
    command.set_value("", &format!("\"{exe}\" \"%1\""))?;

    Ok(())
}

/// Добавляет плеер в список программ для расширения.
///
/// Пишем в `OpenWithProgids`, а не в значение по умолчанию: так плеер
/// появляется в «Открыть с помощью», но не отбирает у пользователя
/// уже выбранную им программу без спроса.
fn link_extension(classes: &RegKey, extension: &str) -> Result<()> {
    let (key, _) = classes.create_subkey(format!(".{extension}"))?;
    let (progids, _) = key.create_subkey("OpenWithProgids")?;

    // Значение пустое — важен сам факт наличия имени.
    progids.set_value(PROG_ID, &"")?;
    Ok(())
}

/// Путь к запущенному exe — его и записываем в реестр.
pub fn current_exe() -> Result<std::path::PathBuf> {
    std::env::current_exe().context("не удалось определить путь к программе")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn список_расширений_без_точек_и_повторов() {
        for extension in EXTENSIONS {
            assert!(!extension.starts_with('.'), "лишняя точка: {extension}");
            assert!(!extension.is_empty());
        }

        let mut sorted = EXTENSIONS.to_vec();
        sorted.sort_unstable();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(before, sorted.len(), "в списке есть повторы");
    }
}
