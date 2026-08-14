//! Проверка обновлений плеера и загрузка нового установщика.
//!
//! Выпуски лежат на GitHub, и спрашивать о них можно без ключа: один
//! запрос к открытому описанию последнего выпуска. Оттуда берутся номер
//! версии, заметка к выпуску и ссылка на установщик.
//!
//! Крейт ничего не запускает и ничего не решает: он отвечает, что вышло
//! и где это лежит, а ставить обновление или нет — дело приложения
//! и человека за ним.

mod download;
mod net;
mod version;

use serde::Deserialize;

pub use download::download;
pub use net::use_proxy;
pub use version::is_newer;

/// Где живут выпуски.
///
/// Заведён постоянным: плеер и его выпуски — одно целое, и указывать
/// хранилище в настройках значило бы дать возможность подсунуть плееру
/// установщик со стороны.
const RELEASES: &str = "https://api.github.com/repos/DjonBeyron/PithPlayer/releases/latest";

/// GitHub требует представиться, иначе отвечает отказом.
const AGENT: &str = concat!("PithPlayer/", env!("CARGO_PKG_VERSION"));

/// Установщик среди вложений выпуска узнаётся по концу имени.
const INSTALLER_SUFFIX: &str = "-setup.exe";

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("GitHub не отвечает: {0}")]
    Network(String),
    #[error("GitHub ответил отказом: {0}")]
    Refused(u16),
    #[error("ответ GitHub не разобрать: {0}")]
    Malformed(String),
    #[error("у выпуска {0} нет установщика")]
    NoInstaller(String),
    #[error("не удалось записать файл: {0}")]
    Io(String),
    #[error("установщик пришёл не целиком: {got} из {expected} байт")]
    Incomplete { got: u64, expected: u64 },
}

/// Вышедший выпуск.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// Номер версии без буквы: `5.1.42`.
    pub version: String,
    /// Заметка к выпуску — её показываем перед установкой.
    pub notes: String,
    /// Страница выпуска: на неё ведёт ссылка «открыть в браузере».
    pub page: String,
    /// Установщик: имя, адрес и размер.
    pub installer: Installer,
}

/// Установщик нового выпуска.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installer {
    pub name: String,
    pub url: String,
    pub size: u64,
}

/// Есть ли выпуск новее запущенного.
///
/// `Ok(None)` — свежих выпусков нет, и это нормальный ответ, а не ошибка.
pub fn check(current: &str) -> Result<Option<Release>, UpdateError> {
    let release = latest()?;

    if !is_newer(current, &release.version) {
        tracing::debug!(
            запущено = current,
            выпущено = %release.version,
            "обновления нет"
        );
        return Ok(None);
    }

    tracing::info!(запущено = current, выпущено = %release.version, "вышло обновление");
    Ok(Some(release))
}

/// Последний выпуск, каким его показывает GitHub.
fn latest() -> Result<Release, UpdateError> {
    let answer: Answer = net::asking()
        .get(RELEASES)
        .header("User-Agent", AGENT)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(code) => UpdateError::Refused(code),
            other => UpdateError::Network(other.to_string()),
        })?
        .into_body()
        .read_json()
        .map_err(|e| UpdateError::Malformed(e.to_string()))?;

    answer.into_release()
}

/// Ответ GitHub в том виде, в каком он нужен.
#[derive(Deserialize)]
struct Answer {
    tag_name: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

impl Answer {
    fn into_release(self) -> Result<Release, UpdateError> {
        let version = self
            .tag_name
            .trim()
            .trim_start_matches(['v', 'V'])
            .to_string();

        let installer = self
            .assets
            .into_iter()
            .find(|asset| asset.name.ends_with(INSTALLER_SUFFIX))
            .map(|asset| Installer {
                name: asset.name,
                url: asset.browser_download_url,
                size: asset.size,
            })
            .ok_or_else(|| UpdateError::NoInstaller(version.clone()))?;

        Ok(Release {
            version,
            notes: self.body,
            page: self.html_url,
            installer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ответ(tag: &str, assets: &[&str]) -> Answer {
        Answer {
            tag_name: tag.to_string(),
            body: "заметка".to_string(),
            html_url: "https://example/release".to_string(),
            assets: assets
                .iter()
                .map(|name| Asset {
                    name: (*name).to_string(),
                    browser_download_url: format!("https://example/{name}"),
                    size: 42,
                })
                .collect(),
        }
    }

    #[test]
    fn буква_из_метки_выпуска_снимается() {
        let release = ответ("v5.1.42", &["PithPlayer-5.1.42-setup.exe"])
            .into_release()
            .expect("выпуск разобран");

        assert_eq!(release.version, "5.1.42");
    }

    #[test]
    fn установщик_выбирается_среди_прочих_вложений() {
        let release = ответ(
            "v5.1.42",
            &[
                "PithPlayer-5.1.42-portable.zip",
                "PithPlayer-5.1.42-setup.exe",
            ],
        )
        .into_release()
        .expect("выпуск разобран");

        assert_eq!(release.installer.name, "PithPlayer-5.1.42-setup.exe");
    }

    #[test]
    fn выпуск_без_установщика_обновлением_не_считается() {
        let answer = ответ("v5.1.42", &["PithPlayer-5.1.42-portable.zip"]);

        assert!(matches!(
            answer.into_release(),
            Err(UpdateError::NoInstaller(_))
        ));
    }
}
