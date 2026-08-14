//! Общий клиент и прокси.
//!
//! Устроено так же, как у прочих сетевых крейтов: один клиент на процесс
//! и прокси, названный приложением при запуске. У GitHub тот же случай,
//! что у базы фильмов, — без прокси у части провайдеров он недоступен.

use std::sync::{OnceLock, RwLock};
use std::time::Duration;

/// Адрес прокси, если приложение его назвало.
static PROXY: RwLock<Option<String>> = RwLock::new(None);

/// Клиент для вопроса «какая версия вышла».
static ASKING: OnceLock<ureq::Agent> = OnceLock::new();

/// Клиент для загрузки установщика.
///
/// Отдельный от первого, и вот почему: срок ожидания у `ureq` считается
/// на весь запрос вместе с чтением тела. Один клиент на оба дела означал
/// бы либо вопрос, висящий четверть часа, либо загрузку сорока мегабайт,
/// оборванную на десятой секунде.
static DOWNLOADING: OnceLock<ureq::Agent> = OnceLock::new();

/// Сколько ждать ответа на вопрос о последнем выпуске.
pub(crate) const ASK_TIMEOUT: Duration = Duration::from_secs(15);

/// Сколько отводится на загрузку установщика целиком.
pub(crate) const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// Называет прокси, через который ходить.
pub fn use_proxy(address: Option<String>) {
    if let Ok(mut proxy) = PROXY.write() {
        *proxy = address.filter(|value| !value.trim().is_empty());
    }
}

/// Клиент для коротких запросов.
pub(crate) fn asking() -> ureq::Agent {
    ASKING.get_or_init(|| build(ASK_TIMEOUT)).clone()
}

/// Клиент для загрузки файла.
pub(crate) fn downloading() -> ureq::Agent {
    DOWNLOADING.get_or_init(|| build(DOWNLOAD_TIMEOUT)).clone()
}

fn build(timeout: Duration) -> ureq::Agent {
    let mut config = ureq::Agent::config_builder().timeout_global(Some(timeout));

    if let Some(proxy) = configured() {
        config = config.proxy(Some(proxy));
    }

    ureq::Agent::new_with_config(config.build())
}

/// Разобранный адрес прокси.
fn configured() -> Option<ureq::Proxy> {
    let address = PROXY.read().ok()?.clone()?;

    ureq::Proxy::new(&address)
        .inspect_err(|e| tracing::warn!(%address, error = %e, "прокси не разобрать"))
        .ok()
}
