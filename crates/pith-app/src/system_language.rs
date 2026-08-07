//! Язык системы — чтобы при первом запуске интерфейс говорил на нём.
//!
//! Спрашивается один раз: дальше выбор живёт в настройках и меняется
//! только из меню. Читаем реестр, а не WinAPI, — крейт `winreg` уже
//! в зависимостях, и обходимся без `unsafe`.

use pith_store::Language;

/// Где Windows хранит язык интерфейса пользователя, например `ru-RU`.
const INTERNATIONAL: &str = r"Control Panel\International";
const LOCALE_NAME: &str = "LocaleName";

/// Язык системы. Незнакомый или нечитаемый — английский.
///
/// Русский выбирается только по явному совпадению: на любом другом языке
/// понятнее английский, чем незнакомая кириллица.
pub fn detect() -> Language {
    match locale_name() {
        Some(locale) if locale.to_lowercase().starts_with("ru") => Language::Ru,
        Some(locale) => {
            tracing::debug!(locale, "язык системы не русский — беру английский");
            Language::En
        }
        None => Language::En,
    }
}

#[cfg(windows)]
fn locale_name() -> Option<String> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(INTERNATIONAL)
        .ok()?;

    key.get_value::<String, _>(LOCALE_NAME).ok()
}

#[cfg(not(windows))]
fn locale_name() -> Option<String> {
    std::env::var("LANG").ok()
}
