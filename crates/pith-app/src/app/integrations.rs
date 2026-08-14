//! Окно интеграций: доступ к Notion и ключ базы фильмов.
//!
//! Поля правятся в черновике, а в настройки ложатся по кнопке «Сохранить»:
//! наполовину введённый токен не должен вытеснить рабочий. Само окно —
//! в `ui/integrations.rs`.

use std::sync::mpsc::{Receiver, channel};

use pith_notion::Notion;
use pith_store::NotionSettings;

use super::PithApp;

/// Где синхронизатор пользователя держит свои настройки.
///
/// Оттуда берётся токен, когда плеер о нём ещё не знает: интеграция
/// у пользователя уже заведена, и вводить руками то, что лежит рядом
/// на диске, незачем. Нет файла — просто пустое поле.
const SYNC_CONFIG: &str = r"C:\PITH\Development\NOTION_PITH\NotionSync\config.json";

/// Страница-образец по умолчанию — база `DIFF` рабочего пространства.
const DEFAULT_TEMPLATE: &str = "https://app.notion.com/p/DIFF-330b5e5392878039ab95ef453be3db03";

/// Рабочая страница по умолчанию — копия образца, сделанная руками.
///
/// В неё складываются отрезки всех картин: база одна, картины различаются
/// полем `FILM NAME`.
const DEFAULT_WORK: &str = "https://app.notion.com/p/Cards-prod-3bab5e539287804d83b7f934db040493";

/// Что окно говорит о доступе.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessStatus {
    /// Ничего не проверяли и не сохраняли.
    Idle,
    /// Проверка идёт.
    Working,
    /// Обе страницы видны интеграции.
    Ok,
    /// Причина отказа словами.
    Failed(String),
    /// Настройки записаны.
    Saved,
}

/// Состояние окна интеграций.
pub struct IntegrationsState {
    pub open: bool,
    /// Черновик полей Notion — то, что набрано в окне.
    pub notion: NotionSettings,
    /// Черновик ключа базы фильмов.
    pub tmdb_key: String,
    pub status: AccessStatus,
    /// Ответ проверки, которого ждём: сеть идёт в отдельном потоке.
    pending: Option<Receiver<Result<(), String>>>,
}

impl Default for IntegrationsState {
    fn default() -> Self {
        Self {
            open: false,
            notion: NotionSettings::default(),
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
        self.integrations.notion = self.settings.notion.clone();
        self.integrations.tmdb_key = self.settings.tmdb_key.clone();
        self.integrations.status = AccessStatus::Idle;

        fill_defaults(&mut self.integrations.notion);

        self.integrations.open = true;
    }

    /// Закрывает окно. Несохранённый черновик пропадает — он и был черновиком.
    pub fn close_integrations(&mut self) {
        self.integrations.open = false;
    }

    /// Записывает черновик в настройки.
    pub fn save_integrations(&mut self) {
        let draft = &self.integrations;

        self.settings.notion = NotionSettings {
            token: draft.notion.token.trim().to_string(),
            work_page: draft.notion.work_page.trim().to_string(),
            template_page: draft.notion.template_page.trim().to_string(),
            // Узнанное у Notion переносим как есть: годность оно проверяет
            // само, по ссылкам, — сменили страницу, и запись отпадёт.
            known: self.settings.notion.known.clone(),
        };
        self.settings.tmdb_key = draft.tmdb_key.trim().to_string();

        self.save_settings();
        self.integrations.status = AccessStatus::Saved;

        tracing::info!(
            notion = self.settings.notion.is_ready(),
            tmdb = !self.settings.tmdb_key.is_empty(),
            "настройки интеграций сохранены"
        );
    }

    /// Спрашивает Notion, видны ли ему обе страницы.
    ///
    /// Запрос уходит в отдельный поток: сеть в потоке интерфейса подвесила
    /// бы окно на всё время ожидания. Готовый ответ будит интерфейс сам —
    /// без этого он лежал бы в канале до ближайшего движения мышью.
    pub fn check_notion_access(&mut self, ctx: &egui::Context) {
        let draft = self.integrations.notion.clone();

        self.integrations.status = AccessStatus::Working;

        let (sender, receiver) = channel();
        self.integrations.pending = Some(receiver);

        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let _ = sender.send(check(&draft));
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
            Ok(()) => {
                tracing::info!("доступ к Notion есть");
                AccessStatus::Ok
            }
            Err(why) => {
                tracing::warn!(причина = %why, "доступа к Notion нет");
                AccessStatus::Failed(why)
            }
        };
    }
}

/// Подставляет то, что известно и без пользователя.
///
/// Страницы — известные, токен берём у синхронизатора: у него он уже
/// прописан. Уже введённое не трогаем.
fn fill_defaults(draft: &mut NotionSettings) {
    for (field, default) in [
        (&mut draft.template_page, DEFAULT_TEMPLATE),
        (&mut draft.work_page, DEFAULT_WORK),
    ] {
        if field.trim().is_empty() {
            *field = default.to_string();
        }
    }

    if !draft.token.trim().is_empty() {
        return;
    }

    if let Some(token) = read_sync_config()
        .as_ref()
        .and_then(|config| config.get("NOTION_TOKEN"))
        .and_then(serde_json::Value::as_str)
    {
        draft.token = token.to_string();
    }
}

/// Настройки синхронизатора. Нет файла или он битый — просто `None`.
fn read_sync_config() -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(SYNC_CONFIG).ok()?;

    serde_json::from_str(&text)
        .inspect_err(|e| tracing::debug!(error = %e, "настройки синхронизатора не разобрать"))
        .ok()
}

/// Проверяет доступ. Выполняется в отдельном потоке.
///
/// Базы ищутся в обеих страницах: в рабочей — та, куда лягут строки,
/// в образце — та, из чьей строки берётся заготовка. Нет базы — выгружать
/// некуда, и узнать об этом лучше сейчас, а не на середине работы.
fn check(settings: &NotionSettings) -> Result<(), String> {
    let notion = Notion::new(&settings.token).ok_or_else(|| {
        crate::tr!(
            "Не задан токен интеграции",
            "The integration token is missing"
        )
        .to_string()
    })?;

    let work = page_id(&settings.work_page, "с базой", "database")?;
    let template = page_id(&settings.template_page, "образца", "template")?;

    for page in [&work, &template] {
        notion.check_access(page).map_err(|e| e.to_string())?;
        notion.database_in_page(page).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Номер страницы из ссылки. Пустая или чужая строка — понятный отказ.
fn page_id(link: &str, what_ru: &str, what_en: &str) -> Result<String, String> {
    pith_notion::parse_id(link).ok_or_else(|| {
        crate::tr!(
            format!("В ссылке на страницу {what_ru} нет номера"),
            format!("The {what_en} page link has no id")
        )
    })
}
