//! Общий клиент и прокси.
//!
//! Клиент один на процесс: у него свой пул соединений, и заведённый заново
//! он каждый раз открывает TCP и пожимает руки по TLS. Слов в реплике
//! десяток, а через прокси это стоит больше секунды на запрос.

use std::sync::{OnceLock, RwLock};
use std::time::Duration;

/// Адрес прокси, если приложение его назвало.
static PROXY: RwLock<Option<String>> = RwLock::new(None);

/// Клиент, общий на все запросы.
static AGENT: OnceLock<ureq::Agent> = OnceLock::new();

/// Называет прокси, через который ходить.
///
/// Словарные сайты живут снаружи, и у пользователя доступ к ним может идти
/// через тот же прокси, что к Notion и базе фильмов. Вызывается один раз
/// при запуске — до первого запроса.
pub fn use_proxy(address: Option<String>) {
    if let Ok(mut proxy) = PROXY.write() {
        *proxy = address.filter(|value| !value.trim().is_empty());
    }
}

/// Клиент с общими настройками.
pub(crate) fn agent(timeout: Duration) -> ureq::Agent {
    AGENT
        .get_or_init(|| {
            // Код состояния разбираем сами: у первого словаря `404` означает
            // «слова не знаю», и это не то же, что «не отвечаю» — во втором
            // случае запрос надо повторить, а в первом сразу идти к другому
            // словарю. Клиент, превращающий код в ошибку, эту разницу теряет.
            let mut config = ureq::Agent::config_builder()
                .timeout_global(Some(timeout))
                .http_status_as_error(false);

            if let Some(proxy) = configured() {
                config = config.proxy(Some(proxy));
            }

            ureq::Agent::new_with_config(config.build())
        })
        .clone()
}

/// Разобранный адрес прокси.
fn configured() -> Option<ureq::Proxy> {
    let address = PROXY.read().ok()?.clone()?;

    ureq::Proxy::new(&address)
        .inspect_err(|e| tracing::warn!(%address, error = %e, "прокси не разобрать"))
        .ok()
}
