//! Окно интеграций: токен GitHub для базы CodeEditVideo и ключ базы фильмов.
//!
//! Поля правятся в черновике, а в настройки ложатся по кнопке «Сохранить»
//! (проверенный токен сохраняется сразу): наполовину введённый токен не
//! должен вытеснить рабочий. Само окно — в `ui/integrations.rs`.

use std::sync::mpsc::{Receiver, channel};

use pith_store::CevSettings;

use super::PithApp;

/// Что окно говорит о доступе.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessStatus {
    /// Ничего не проверяли и не сохраняли.
    Idle,
    /// Проверка идёт.
    Working,
    /// Доступ есть; пояснение словами (сколько записей в базе и т. п.).
    Verified(String),
    /// Причина отказа словами.
    Failed(String),
    /// Настройки записаны.
    Saved,
}

/// Состояние окна интеграций.
pub struct IntegrationsState {
    pub open: bool,
    /// Черновик выгрузки в CodeEditVideo.
    pub cev: CevSettings,
    /// Черновик ключа базы фильмов.
    pub tmdb_key: String,
    pub status: AccessStatus,
    /// Ответ проверки, которого ждём: сеть идёт в отдельном потоке.
    pending: Option<Receiver<Result<String, String>>>,
}

impl Default for IntegrationsState {
    fn default() -> Self {
        Self {
            open: false,
            cev: CevSettings::default(),
            tmdb_key: String::new(),
            status: AccessStatus::Idle,
            pending: None,
        }
    }
}

impl PithApp {
    pub fn integrations_open(&self) -> bool {
        self.integrations.open
    }

    /// Поля окна — правятся прямо в нём.
    pub fn integrations_state(&mut self) -> &mut IntegrationsState {
        &mut self.integrations
    }

    /// Открывает окно, наполняя черновик тем, что уже известно.
    pub fn open_integrations(&mut self) {
        self.integrations.cev = self.settings.cev.clone();
        self.integrations.tmdb_key = self.settings.tmdb_key.clone();
        self.integrations.status = AccessStatus::Idle;
        self.integrations.open = true;
    }

    /// Закрывает окно. Несохранённый черновик пропадает — он и был черновиком.
    pub fn close_integrations(&mut self) {
        self.integrations.open = false;
    }

    /// Записывает черновик в настройки.
    pub fn save_integrations(&mut self) {
        let draft = &self.integrations;

        self.settings.cev = CevSettings {
            token: draft.cev.token.trim().to_string(),
        };
        self.settings.tmdb_key = draft.tmdb_key.trim().to_string();

        self.save_settings();
        self.integrations.status = AccessStatus::Saved;

        tracing::info!(
            github = self.settings.cev.is_ready(),
            tmdb = !self.settings.tmdb_key.is_empty(),
            "настройки интеграций сохранены"
        );
    }

    /// Спрашивает GitHub, годится ли токен.
    ///
    /// Запрос уходит в отдельный поток: сеть в потоке интерфейса подвесила
    /// бы окно на всё время ожидания. Готовый ответ будит интерфейс сам.
    pub fn check_cev_access(&mut self, ctx: &egui::Context) {
        let cev = self.integrations.cev.clone();

        self.integrations.status = AccessStatus::Working;

        let (sender, receiver) = channel();
        self.integrations.pending = Some(receiver);

        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let _ = sender.send(check_cev(&cev));
            ctx.request_repaint();
        });
    }

    /// Забирает итог проверки, если он готов.
    pub(super) fn poll_integrations(&mut self) {
        let Some(receiver) = self.integrations.pending.as_ref() else {
            return;
        };

        let Ok(answer) = receiver.try_recv() else {
            return;
        };

        self.integrations.pending = None;
        self.integrations.status = match answer {
            Ok(info) => {
                tracing::info!(%info, "доступ к CodeEditVideo есть");
                AccessStatus::Verified(info)
            }
            Err(why) => {
                tracing::warn!(причина = %why, "доступа к CodeEditVideo нет");
                AccessStatus::Failed(why)
            }
        };

        // Проверенные настройки рабочие — запоминаем сразу, не дожидаясь
        // «Сохранить»: закрытое окно иначе уносило токен вместе с черновиком.
        if matches!(self.integrations.status, AccessStatus::Verified(_)) {
            let verified = self.integrations.status.clone();
            self.save_integrations();
            self.integrations.status = verified;
        }
    }
}

/// Проверка доступа к базе CodeEditVideo: только чтение, ничего не пишет.
fn check_cev(settings: &CevSettings) -> Result<String, String> {
    let cev = pith_cev::Cev::new(&settings.token).ok_or_else(|| {
        crate::tr!("Не задан токен GitHub", "The GitHub token is missing").to_string()
    })?;

    let access = cev.check().map_err(|e| e.to_string())?;

    Ok(crate::tr!(
        format!(
            "База доступна для записи: записей {}, следующий номер {}",
            access.records, access.next_number
        ),
        format!(
            "The database is writable: {} records, next number {}",
            access.records, access.next_number
        )
    ))
}
