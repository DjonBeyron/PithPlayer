//! Доступ к Notion: токен интеграции и страницы, с которыми она работает.
//!
//! Ссылки хранятся целиком, как их принёс пользователь из браузера. Номер
//! из них достаёт `pith-notion` в момент обращения: разбирать ссылку здесь
//! незачем, а видеть в настройках знакомый адрес понятнее номера.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotionSettings {
    /// Токен интеграции — строка вида `ntn_…`.
    pub token: String,
    /// Страница с рабочей базой: в неё складываются отрезки всех картин.
    ///
    /// База там одна на всё, картины различаются полем `FILM NAME`.
    /// Копию образца пользователь делает сам, кнопкой «Duplicate»: API
    /// Notion копировать не умеет, а ручная копия сохраняет виды, фильтры
    /// и оформление — то, чего сборка по свойствам дать не может.
    pub work_page: String,
    /// Страница-образец: из её строки берутся значения для новых.
    ///
    /// Читается и только: в образце ничего не меняется.
    pub template_page: String,
    /// Что уже узнано у Notion и не меняется.
    pub known: NotionKnown,
}

/// Постоянная часть ответов Notion — чтобы не спрашивать её каждый раз.
///
/// Номер базы, заготовка строки и наличие поля номера меняются только
/// вместе со ссылками на страницы. Узнать их стоит четырёх запросов,
/// около трёх секунд ожидания перед каждой выгрузкой, — и это ожидание
/// было заметно: плеер «думал» на пустом месте.
///
/// Хранится вместе со ссылками, при которых узнано: сменили страницу —
/// запись негодна, и всё спросится заново.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotionKnown {
    /// Ссылки, при которых это узнано: рабочая и образец, через перевод строки.
    pub pages: String,
    /// Номер рабочей базы.
    pub database: String,
    /// Заготовка строки, как её отдал Notion.
    pub sample: String,
    /// В базе есть числовое поле номера.
    pub numbered: bool,
}

impl NotionKnown {
    /// Годится ли запись для этих страниц.
    pub fn fits(&self, work_page: &str, template_page: &str) -> bool {
        !self.database.is_empty() && self.pages == Self::key(work_page, template_page)
    }

    /// Ключ записи — обе ссылки разом.
    pub fn key(work_page: &str, template_page: &str) -> String {
        format!("{}\n{}", work_page.trim(), template_page.trim())
    }
}

impl NotionSettings {
    /// Есть ли всё, что нужно для выгрузки.
    ///
    /// Только заполненность полей: рабочий ли токен и видны ли страницы
    /// интеграции — знает один Notion, и спрашивают об этом кнопкой
    /// «Проверить доступ».
    pub fn is_ready(&self) -> bool {
        [&self.token, &self.work_page, &self.template_page]
            .iter()
            .all(|field| !field.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::{NotionKnown, NotionSettings};

    #[test]
    fn пустые_настройки_не_готовы() {
        assert!(!NotionSettings::default().is_ready());
    }

    #[test]
    fn готовы_только_когда_заполнено_всё() {
        let mut settings = NotionSettings {
            token: "ntn_1".into(),
            work_page: "https://notion.so/cards-prod".into(),
            template_page: String::new(),
            known: NotionKnown::default(),
        };
        assert!(!settings.is_ready(), "без образца строки не с чего начать");

        settings.template_page = "https://notion.so/diff".into();
        assert!(settings.is_ready());
    }

    #[test]
    fn пробелы_за_заполненность_не_считаются() {
        let settings = NotionSettings {
            token: "   ".into(),
            work_page: "w".into(),
            template_page: "t".into(),
            known: NotionKnown::default(),
        };

        assert!(!settings.is_ready());
    }
}

#[cfg(test)]
mod known_tests {
    use super::NotionKnown;

    fn запись() -> NotionKnown {
        NotionKnown {
            pages: NotionKnown::key("работа", "образец"),
            database: "abc".into(),
            sample: "{}".into(),
            numbered: true,
        }
    }

    #[test]
    fn запись_годится_для_тех_же_страниц() {
        assert!(запись().fits("работа", "образец"));
    }

    #[test]
    fn пробелы_в_ссылках_роли_не_играют() {
        assert!(запись().fits("  работа ", " образец  "));
    }

    #[test]
    fn другая_страница_делает_запись_негодной() {
        assert!(!запись().fits("другая", "образец"));
        assert!(!запись().fits("работа", "другой"));
    }

    #[test]
    fn пустая_запись_не_годится_никогда() {
        assert!(!NotionKnown::default().fits("работа", "образец"));
    }
}
