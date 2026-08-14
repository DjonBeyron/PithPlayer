//! Прокси системы: через что плееру ходить в сеть.
//!
//! Зачем это нужно. Программы, дающие доступ в сеть (VPN-клиенты и им
//! подобные), прописывают прокси **в настройки Windows** — те самые,
//! которыми пользуются браузеры. Переменных окружения при этом обычно
//! не появляется, а плеер, запущенный из проводника, только их и мог бы
//! увидеть: `ureq` берёт прокси из `HTTP_PROXY` и `HTTPS_PROXY` сам.
//!
//! Без прокси запрос уходит напрямую, и у части провайдеров имя
//! `api.themoviedb.org` разрешается в `127.0.0.1`. Слушать там некому,
//! и пользователь видит «connection refused» — при работающей сети
//! и рабочем ключе. Notion при этом открывается напрямую, и выходит совсем
//! непонятно: одна интеграция работает, другая нет.
//!
//! Поэтому: переменные окружения главнее (их задают осознанно), а если
//! их нет — берём прокси системы и называем его сетевым крейтам.

use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

/// Где Windows держит настройки соединения.
const SETTINGS_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

/// Переменные, которые задают прокси сами.
const ENV_NAMES: [&str; 4] = ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"];

/// Находит прокси и называет его сетевым крейтам.
///
/// Вызывается один раз при запуске, до первого запроса в сеть.
pub fn announce_proxy() {
    let proxy = detect();

    pith_tmdb::use_proxy(proxy.clone());
    pith_notion::use_proxy(proxy.clone());
    pith_dict::use_proxy(proxy);
}

/// Адрес прокси, через который ходить. `None` — идём напрямую.
fn detect() -> Option<String> {
    // Заданное в окружении не перебиваем: `ureq` разберётся сам, а вместе
    // с переменными обычно задан и `NO_PROXY`, который мы бы потеряли.
    if ENV_NAMES
        .iter()
        .any(|name| std::env::var(name).is_ok_and(|value| !value.trim().is_empty()))
    {
        tracing::debug!("прокси задан в окружении — берём его");
        return None;
    }

    let uri = system_proxy()?;

    tracing::info!(прокси = %uri, "прокси взят из настроек системы");
    Some(uri)
}

/// Прокси из настроек Windows, если он там включён.
fn system_proxy() -> Option<String> {
    let settings = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(SETTINGS_KEY)
        .inspect_err(|e| tracing::debug!(error = %e, "настройки соединения не открыть"))
        .ok()?;

    // Выключенный прокси — не прокси: адрес в настройках остаётся и после
    // того, как его перестали использовать.
    let enabled: u32 = settings.get_value("ProxyEnable").unwrap_or(0);
    if enabled == 0 {
        return None;
    }

    let server: String = settings.get_value("ProxyServer").ok()?;

    normalize(&server)
}

/// Приводит запись Windows к адресу для клиента.
///
/// Записывают её двумя способами: один адрес на все протоколы
/// (`127.0.0.1:10809`) либо по протоколам через точку с запятой
/// (`http=узел:8080;https=узел:8443`). Во втором виде берём `https`,
/// а если его нет — `http`: наши запросы все шифрованные.
fn normalize(server: &str) -> Option<String> {
    let server = server.trim();

    if server.is_empty() {
        return None;
    }

    if !server.contains('=') {
        return Some(with_scheme(server));
    }

    let parts: Vec<(&str, &str)> = server
        .split(';')
        .filter_map(|part| part.split_once('='))
        .map(|(name, value)| (name.trim(), value.trim()))
        .collect();

    for wanted in ["https", "http"] {
        if let Some((_, value)) = parts.iter().find(|(name, _)| *name == wanted)
            && !value.is_empty()
        {
            return Some(with_scheme(value));
        }
    }

    None
}

/// Дописывает схему, если её нет: Windows хранит адрес без неё.
///
/// Схема именно `http`, а не `https`: к самому прокси подключаются
/// открыто, шифрование идёт внутри туннеля.
fn with_scheme(address: &str) -> String {
    if address.contains("://") {
        address.to_string()
    } else {
        format!("http://{address}")
    }
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn один_адрес_на_все_протоколы() {
        assert_eq!(
            normalize("127.0.0.1:10809").as_deref(),
            Some("http://127.0.0.1:10809")
        );
    }

    #[test]
    fn схему_не_дописываем_дважды() {
        assert_eq!(
            normalize("http://узел:8080").as_deref(),
            Some("http://узел:8080")
        );
    }

    #[test]
    fn из_записи_по_протоколам_берётся_https() {
        assert_eq!(
            normalize("http=узел:8080;https=узел:8443").as_deref(),
            Some("http://узел:8443")
        );
    }

    #[test]
    fn без_https_годится_http() {
        assert_eq!(
            normalize("ftp=узел:21;http=узел:8080").as_deref(),
            Some("http://узел:8080")
        );
    }

    #[test]
    fn пустая_запись_адреса_не_даёт() {
        assert!(normalize("").is_none());
        assert!(normalize("   ").is_none());
        assert!(
            normalize("socks=узел:1080").is_none(),
            "нужен http или https"
        );
    }
}
