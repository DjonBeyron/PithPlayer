//! Закладки и именованные списки отрезков (PLAN.md §6.5).
//!
//! Закладка — точка во времени. Отрезок для нарезки считается от неё:
//! `[метка − отступ, метка − отступ + длительность]`.
//!
//! Списки — независимые наборы закладок **внутри одного видео**, а не
//! плейлисты файлов: для фильма отдельно ведутся «Диалоги», «На вырезку»,
//! «Музыка», каждый со своей длительностью фрагмента и папкой вывода.

mod store;
mod video;

pub use store::Bookmarks;
pub use video::{ListError, VideoBookmarks};

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Имя списка, создаваемого по умолчанию.
pub const DEFAULT_LIST: &str = "Основной";

/// Закладка: точка во времени.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeBookmark {
    pub time_ms: i64,
    /// Название, заданное пользователем.
    pub name: Option<String>,
    /// Кто в кадре: «Имя (Роль)», выбранное в окне актёров.
    ///
    /// В списке отрезков не показывается — виден и правится в диалоге
    /// переименования. В названии закладки ему не место: оно идёт в имя
    /// вырезанного файла. Старые закладки поля не имеют, и это не ошибка.
    #[serde(default)]
    pub actor: Option<String>,
}

impl TimeBookmark {
    pub fn seconds(&self) -> f64 {
        self.time_ms as f64 / 1000.0
    }

    /// Подпись: название либо время.
    pub fn label(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| format_time(self.time_ms))
    }
}

/// Именованный список отрезков внутри одного видео.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BookmarkList {
    pub name: String,
    /// Длительность вырезаемого фрагмента, секунды.
    pub duration_sec: u32,
    /// Отступ назад от метки, секунды.
    pub buffer_sec: u32,
    /// Своя папка вывода. `None` — общая из настроек.
    pub output_dir: Option<PathBuf>,
    pub bookmarks: Vec<TimeBookmark>,
}

impl BookmarkList {
    pub fn new(name: impl Into<String>, duration_sec: u32, buffer_sec: u32) -> Self {
        Self {
            name: name.into(),
            duration_sec,
            buffer_sec,
            output_dir: None,
            bookmarks: Vec::new(),
        }
    }

    /// Добавляет закладку, сохраняя порядок по времени.
    ///
    /// Возвращает `false`, если рядом уже есть метка: в v4 дубликаты
    /// в пределах секунды отсекались, и это правило сохраняется.
    pub fn add(&mut self, time_ms: i64, name: Option<String>) -> bool {
        if self.has_near(time_ms) {
            return false;
        }

        self.bookmarks.push(TimeBookmark {
            time_ms,
            name,
            actor: None,
        });
        self.bookmarks.sort_by_key(|b| b.time_ms);
        true
    }

    /// Записывает актёра закладке. Возвращает `false`, если метки нет.
    pub fn set_actor(&mut self, time_ms: i64, actor: Option<String>) -> bool {
        let Some(bookmark) = self.bookmarks.iter_mut().find(|b| b.time_ms == time_ms) else {
            return false;
        };

        bookmark.actor = actor;
        true
    }

    /// Ближайшая по времени закладка. Нужна окну актёров: выбранный актёр
    /// достаётся отрезку под курсором, а если его там нет — соседнему.
    pub fn nearest(&self, time_ms: i64) -> Option<&TimeBookmark> {
        self.bookmarks
            .iter()
            .min_by_key(|b| (b.time_ms - time_ms).abs())
    }

    /// Есть ли закладка в пределах секунды от указанного времени.
    pub fn has_near(&self, time_ms: i64) -> bool {
        self.bookmarks
            .iter()
            .any(|b| (b.time_ms - time_ms).abs() < NEAR_MS)
    }

    pub fn remove_at(&mut self, time_ms: i64) -> bool {
        let before = self.bookmarks.len();
        self.bookmarks.retain(|b| b.time_ms != time_ms);
        self.bookmarks.len() != before
    }

    /// Забирает закладку из списка целиком — нужно переносу между списками.
    pub fn take_at(&mut self, time_ms: i64) -> Option<TimeBookmark> {
        let index = self.bookmarks.iter().position(|b| b.time_ms == time_ms)?;
        Some(self.bookmarks.remove(index))
    }

    /// Кладёт готовую закладку, сохраняя порядок по времени.
    pub fn insert(&mut self, bookmark: TimeBookmark) {
        self.bookmarks.push(bookmark);
        self.bookmarks.sort_by_key(|b| b.time_ms);
    }
}

/// Насколько близко стоящие метки считаются одной и той же.
const NEAR_MS: i64 = 1000;

/// Время в формате «ЧЧ:ММ:СС».
fn format_time(time_ms: i64) -> String {
    let total = (time_ms / 1000).max(0);
    format!(
        "{:02}:{:02}:{:02}",
        total / 3600,
        (total % 3600) / 60,
        total % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn закладки_хранятся_по_возрастанию_времени() {
        let mut list = BookmarkList::new("Основной", 18, 5);

        list.add(30_000, None);
        list.add(10_000, None);
        list.add(20_000, None);

        let времена: Vec<_> = list.bookmarks.iter().map(|b| b.time_ms).collect();
        assert_eq!(времена, vec![10_000, 20_000, 30_000]);
    }

    #[test]
    fn близкие_закладки_не_дублируются() {
        let mut list = BookmarkList::new("Основной", 18, 5);

        assert!(list.add(10_000, None));
        assert!(
            !list.add(10_500, None),
            "метка в пределах секунды — это та же метка"
        );
        assert_eq!(list.bookmarks.len(), 1);
    }

    #[test]
    fn закладка_дальше_секунды_добавляется() {
        let mut list = BookmarkList::new("Основной", 18, 5);

        list.add(10_000, None);
        assert!(list.add(11_500, None));
        assert_eq!(list.bookmarks.len(), 2);
    }

    #[test]
    fn закладка_удаляется_по_времени() {
        let mut list = BookmarkList::new("Основной", 18, 5);
        list.add(10_000, None);

        assert!(list.remove_at(10_000));
        assert!(
            !list.remove_at(10_000),
            "повторное удаление ничего не меняет"
        );
        assert!(list.bookmarks.is_empty());
    }

    #[test]
    fn закладка_забирается_вместе_с_названием() {
        let mut list = BookmarkList::new("Основной", 18, 5);
        list.add(10_000, Some("Реплика".into()));

        let taken = list.take_at(10_000).expect("закладка найдена");
        assert_eq!(taken.name.as_deref(), Some("Реплика"));
        assert!(list.bookmarks.is_empty());
        assert!(list.take_at(10_000).is_none());
    }

    #[test]
    fn вставленная_закладка_встаёт_по_времени() {
        let mut list = BookmarkList::new("Основной", 18, 5);
        list.add(30_000, None);
        list.add(10_000, None);

        list.insert(TimeBookmark {
            time_ms: 20_000,
            name: None,
            actor: None,
        });

        let времена: Vec<_> = list.bookmarks.iter().map(|b| b.time_ms).collect();
        assert_eq!(времена, vec![10_000, 20_000, 30_000]);
    }

    #[test]
    fn подпись_берётся_из_названия_или_времени() {
        let named = TimeBookmark {
            time_ms: 372_398,
            name: Some("Реплика".into()),
            actor: None,
        };
        let plain = TimeBookmark {
            time_ms: 372_398,
            name: None,
            actor: None,
        };

        assert_eq!(named.label(), "Реплика");
        assert_eq!(plain.label(), "00:06:12");
    }

    #[test]
    fn актёр_записывается_и_заменяется() {
        let mut list = BookmarkList::new("Основной", 18, 5);
        list.add(10_000, None);

        assert!(list.set_actor(10_000, Some("Леонардо ДиКаприо (Jack)".into())));
        assert_eq!(
            list.bookmarks[0].actor.as_deref(),
            Some("Леонардо ДиКаприо (Jack)")
        );

        // Новый выбор заменяет прежнего: актёр у закладки один.
        assert!(list.set_actor(10_000, Some("Кейт Уинслет (Rose)".into())));
        assert_eq!(
            list.bookmarks[0].actor.as_deref(),
            Some("Кейт Уинслет (Rose)")
        );

        // Метки нет — и это не ошибка, просто ничего не записали.
        assert!(!list.set_actor(999, Some("Кто-то".into())));
    }

    #[test]
    fn ближайшая_закладка_находится_по_времени() {
        let mut list = BookmarkList::new("Основной", 18, 5);
        list.add(10_000, None);
        list.add(60_000, None);

        assert_eq!(list.nearest(12_000).map(|b| b.time_ms), Some(10_000));
        assert_eq!(list.nearest(50_000).map(|b| b.time_ms), Some(60_000));
        // Ровно посередине — берётся первая по порядку.
        assert_eq!(list.nearest(35_000).map(|b| b.time_ms), Some(10_000));
    }

    #[test]
    fn пустой_список_ближайшей_не_имеет() {
        assert!(BookmarkList::new("Основной", 18, 5).nearest(0).is_none());
    }
}
