//! Дорожки аудио и субтитров.
//!
//! Читаются из `track-list` по частям: так не нужен разбор JSON, а mpv
//! отдаёт список сразу и достоверно — в отличие от VLC, где v4 ждала
//! таймерами и делала три попытки (PLAN.md §6.3).

use crate::engine::Engine;

/// Вид дорожки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackKind {
    Audio,
    Subtitle,
    Video,
}

impl TrackKind {
    fn from_mpv(value: &str) -> Option<Self> {
        match value {
            "audio" => Some(Self::Audio),
            "sub" => Some(Self::Subtitle),
            "video" => Some(Self::Video),
            _ => None,
        }
    }
}

/// Дорожка файла.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// Номер дорожки для mpv (`aid`, `sid`).
    pub id: i64,
    pub kind: TrackKind,
    /// Название из метаданных, если есть.
    pub title: Option<String>,
    /// Код языка: `eng`, `rus` и подобные.
    pub lang: Option<String>,
    /// Помечена в файле как основная.
    pub default: bool,
    /// Форсированная: обычно переводы надписей, а не вся речь.
    pub forced: bool,
    /// Выбрана сейчас.
    pub selected: bool,
}

impl Track {
    /// Строка для поиска по тегам: название и язык вместе, в нижнем регистре.
    pub fn search_text(&self) -> String {
        let title = self.title.as_deref().unwrap_or_default();
        let lang = self.lang.as_deref().unwrap_or_default();
        format!("{title} {lang}").to_lowercase()
    }

    /// Подпись для меню: название дорожки и язык, как их записал файл.
    ///
    /// Своих слов здесь нет: движок не знает языка интерфейса, и пометки
    /// вроде «форсированные» дописывает тот, кто рисует меню.
    pub fn label(&self) -> String {
        let mut parts = Vec::new();

        if let Some(title) = &self.title {
            parts.push(title.clone());
        }
        if let Some(lang) = &self.lang {
            parts.push(format!("[{lang}]"));
        }

        if parts.is_empty() {
            format!("#{}", self.id)
        } else {
            parts.join(" ")
        }
    }
}

impl Engine {
    /// Список дорожек файла.
    pub fn tracks(&self) -> Vec<Track> {
        let count = self.property_i64("track-list/count").unwrap_or(0);

        (0..count).filter_map(|i| self.read_track(i)).collect()
    }

    fn read_track(&self, index: i64) -> Option<Track> {
        let kind = TrackKind::from_mpv(&self.property_string_at(index, "type")?)?;
        let id = self.property_i64(&format!("track-list/{index}/id"))?;

        Some(Track {
            id,
            kind,
            title: self.property_string_at(index, "title"),
            lang: self.property_string_at(index, "lang"),
            default: self.property_bool_at(index, "default"),
            forced: self.property_bool_at(index, "forced"),
            selected: self.property_bool_at(index, "selected"),
        })
    }

    fn property_string_at(&self, index: i64, field: &str) -> Option<String> {
        self.property_string(&format!("track-list/{index}/{field}"))
            .ok()
            .filter(|s| !s.is_empty())
    }

    fn property_bool_at(&self, index: i64, field: &str) -> bool {
        self.property_bool(&format!("track-list/{index}/{field}"))
            .unwrap_or(false)
    }

    /// Включает дорожку субтитров. `None` — выключить субтитры.
    pub fn set_subtitle_track(&mut self, id: Option<i64>) {
        self.set_track("sid", id);
        self.hide_own_subtitles();
    }

    /// Включает вторую дорожку субтитров.
    ///
    /// mpv показывает её одновременно с основной — в v4 для этого была
    /// отдельная система из полутора тысяч строк.
    pub fn set_secondary_subtitle_track(&mut self, id: Option<i64>) {
        self.set_track("secondary-sid", id);
        self.hide_own_subtitles();
    }

    /// Гасит собственную отрисовку субтитров mpv.
    ///
    /// Повторяется при каждой смене дорожки, а не только при запуске:
    /// иначе выбранная вручную дорожка рисуется ещё и средствами mpv —
    /// поверх наших слоёв появляется текст, который нельзя ни сдвинуть,
    /// ни скопировать.
    fn hide_own_subtitles(&mut self) {
        for property in ["sub-visibility", "secondary-sub-visibility"] {
            if let Err(e) = self.set_property_string(property, "no") {
                tracing::warn!(property, error = %e, "не удалось погасить отрисовку субтитров mpv");
            }
        }
    }

    /// Включает аудиодорожку.
    pub fn set_audio_track(&mut self, id: Option<i64>) {
        self.set_track("aid", id);
    }

    fn set_track(&mut self, property: &str, id: Option<i64>) {
        let result = match id {
            Some(id) => self.set_property_string(property, &id.to_string()),
            None => self.set_property_string(property, "no"),
        };

        match result {
            Ok(()) => tracing::debug!(property, ?id, "дорожка выбрана"),
            Err(e) => tracing::warn!(property, ?id, error = %e, "не удалось выбрать дорожку"),
        }
    }

    /// Выбранная дорожка субтитров.
    pub fn current_subtitle_track(&self) -> Option<i64> {
        self.property_i64("sid")
    }

    /// Выбранная вторая дорожка субтитров.
    pub fn current_secondary_subtitle_track(&self) -> Option<i64> {
        self.property_i64("secondary-sid")
    }

    /// Выбранная аудиодорожка.
    pub fn current_audio_track(&self) -> Option<i64> {
        self.property_i64("aid")
    }

    /// Текущая реплика основных субтитров.
    pub fn subtitle_text(&self) -> Option<String> {
        self.non_empty_property("sub-text")
    }

    /// Текущая реплика вторых субтитров.
    pub fn secondary_subtitle_text(&self) -> Option<String> {
        self.non_empty_property("secondary-sub-text")
    }

    fn non_empty_property(&self, name: &str) -> Option<String> {
        self.property_string(name).ok().filter(|s| !s.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn дорожка(title: Option<&str>, lang: Option<&str>, forced: bool) -> Track {
        Track {
            id: 1,
            kind: TrackKind::Subtitle,
            title: title.map(String::from),
            lang: lang.map(String::from),
            default: false,
            forced,
            selected: false,
        }
    }

    #[test]
    fn текст_для_поиска_объединяет_название_и_язык() {
        let track = дорожка(Some("English SDH"), Some("eng"), false);
        assert_eq!(track.search_text(), "english sdh eng");
    }

    #[test]
    fn текст_для_поиска_работает_без_метаданных() {
        assert_eq!(дорожка(None, None, false).search_text().trim(), "");
    }

    #[test]
    fn подпись_собирается_из_названия_и_языка() {
        let track = дорожка(Some("Русские"), Some("rus"), false);
        assert_eq!(track.label(), "Русские [rus]");
    }

    #[test]
    fn подпись_не_содержит_своих_слов() {
        // Пометку о форсированной дорожке дописывает интерфейс: здесь
        // остаётся только то, что записано в самом файле.
        let track = дорожка(Some("Signs"), Some("eng"), true);
        assert_eq!(track.label(), "Signs [eng]");
    }

    #[test]
    fn без_метаданных_подпись_содержит_номер() {
        assert_eq!(дорожка(None, None, false).label(), "#1");
    }

    #[test]
    fn виды_дорожек_разбираются() {
        assert_eq!(TrackKind::from_mpv("audio"), Some(TrackKind::Audio));
        assert_eq!(TrackKind::from_mpv("sub"), Some(TrackKind::Subtitle));
        assert_eq!(TrackKind::from_mpv("video"), Some(TrackKind::Video));
        assert_eq!(TrackKind::from_mpv("что-то"), None);
    }
}
