//! Общий клиент и прокси — устроено так же, как у прочих сетевых крейтов.

use std::sync::{OnceLock, RwLock};
use std::time::Duration;

static PROXY: RwLock<Option<String>> = RwLock::new(None);
static AGENT: OnceLock<ureq::Agent> = OnceLock::new();

/// Сколько ждать ответа GitHub на один запрос.
const TIMEOUT: Duration = Duration::from_secs(30);

/// Называет прокси, через который ходить. Вызывается при запуске.
pub fn use_proxy(address: Option<String>) {
    if let Ok(mut proxy) = PROXY.write() {
        *proxy = address.filter(|value| !value.trim().is_empty());
    }
}

pub(crate) fn agent() -> ureq::Agent {
    AGENT.get_or_init(build).clone()
}

fn build() -> ureq::Agent {
    // Статусы 4xx/5xx читаем сами: по ним решаем, что делать (409 — повтор).
    let mut config = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .http_status_as_error(false);

    if let Some(proxy) = configured() {
        config = config.proxy(Some(proxy));
    }

    ureq::Agent::new_with_config(config.build())
}

fn configured() -> Option<ureq::Proxy> {
    let address = PROXY.read().ok()?.clone()?;

    ureq::Proxy::new(&address)
        .inspect_err(|e| tracing::warn!(%address, error = %e, "прокси не разобрать"))
        .ok()
}
