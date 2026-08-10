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

/// Дорожка в том виде, в каком её отдаёт mpv.
///
/// Лишние поля пропускаются: их у mpv два десятка, а нужны семь.
#[derive(serde::Deserialize)]
struct RawTrack {
    id: i64,
    #[serde(rename = "type")]
    kind: String,
    title: Option<String>,
    lang: Option<String>,
    #[serde(default)]
    default: bool,
    #[serde(default)]
    forced: bool,
    #[serde(default)]
    selected: bool,
}

impl Engine {
    /// Весь список дорожек одной строкой, как его отдаёт mpv.
    pub fn track_list_json(&self) -> Option<String> {
        self.property_string("track-list").ok()
    }

    /// Список дорожек файла.
    ///
    /// Читается одним свойством: mpv отдаёт весь список строкой JSON.
    /// Прежде поля запрашивались поштучно — семь запросов на дорожку,
    /// полсотни на фильм, — и на занятом mpv это стоило 233 мс замершего
    /// окна при открытии файла (PLAN.md §6.14). Один запрос вместо
    /// полусотни укладывается в единицы миллисекунд.
    pub fn tracks(&self) -> Vec<Track> {
        let Some(json) = self.track_list_json() else {
            return Vec::new();
        };

        parse_tracks(&json)
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
    ///
    /// Берётся из состояния: реплики приходят подпиской (`observe.rs`),
    /// а не спрашиваются у mpv на каждом кадре.
    pub fn subtitle_text(&self) -> Option<String> {
        self.state().subtitle.clone()
    }

    /// Текущая реплика вторых субтитров.
    pub fn secondary_subtitle_text(&self) -> Option<String> {
        self.state().secondary_subtitle.clone()
    }
}

/// Разбирает список дорожек из строки JSON.
///
/// Битая строка означает пустой список: плеер продолжит играть, а дорожки
/// выберет сам mpv по `alang`/`slang`. Дорожки неизвестного вида
/// (картинки обложек, вложения) пропускаются.
fn parse_tracks(json: &str) -> Vec<Track> {
    let raw: Vec<RawTrack> = match serde_json::from_str(json) {
        Ok(list) => list,
        Err(e) => {
            tracing::warn!(error = %e, "не удалось разобрать список дорожек");
            return Vec::new();
        }
    };

    raw.into_iter()
        .filter_map(|t| {
            Some(Track {
                id: t.id,
                kind: TrackKind::from_mpv(&t.kind)?,
                title: t.title.filter(|s| !s.is_empty()),
                lang: t.lang.filter(|s| !s.is_empty()),
                default: t.default,
                forced: t.forced,
                selected: t.selected,
            })
        })
        .collect()
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

    /// Строка mpv, урезанная до нужных полей: их там два десятка.
    const СПИСОК: &str = r#"[
        {"id":1,"type":"video","default":true,"forced":false,"selected":true,"codec":"hevc"},
        {"id":2,"type":"audio","title":"Surround","lang":"eng","default":true,"forced":false,"selected":true},
        {"id":3,"type":"sub","title":"SDH","lang":"eng","default":false,"forced":false,"selected":false},
        {"id":4,"type":"привидение","default":false,"forced":false,"selected":false}
    ]"#;

    #[test]
    fn список_дорожек_разбирается_из_строки() {
        let tracks = parse_tracks(СПИСОК);

        // Дорожка неизвестного вида отброшена, остальные три на месте.
        assert_eq!(tracks.len(), 3);

        let sub = &tracks[2];
        assert_eq!(sub.id, 3);
        assert_eq!(sub.kind, TrackKind::Subtitle);
        assert_eq!(sub.title.as_deref(), Some("SDH"));
        assert_eq!(sub.lang.as_deref(), Some("eng"));
        assert!(!sub.default);

        let audio = &tracks[1];
        assert_eq!(audio.kind, TrackKind::Audio);
        assert!(audio.default);
        assert!(audio.selected);
    }

    #[test]
    fn дорожка_без_названия_и_языка_разбирается() {
        let tracks = parse_tracks(r#"[{"id":1,"type":"sub","title":"","lang":""}]"#);

        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].title, None);
        assert_eq!(tracks[0].lang, None);
        assert!(!tracks[0].forced);
    }

    #[test]
    fn битая_строка_даёт_пустой_список() {
        assert!(parse_tracks("не json").is_empty());
        assert!(parse_tracks("").is_empty());
    }
}
