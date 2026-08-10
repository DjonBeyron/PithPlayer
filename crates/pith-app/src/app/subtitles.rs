//! Субтитры: автовыбор дорожек и текущий текст.
//!
//! Текст берётся у mpv свойствами `sub-text` и `secondary-sub-text` —
//! в v4 ради этого извлекали дорожки в `.srt` через FFmpeg и держали пять
//! синхронизаторов (PLAN.md §5.1).

use pith_mpv::{Track, TrackKind};
use pith_store::tag_score;

use super::PithApp;

/// Сколько секунд после реплики она ещё годится в название закладки.
///
/// Восьми хватает на паузу между репликами; за более долгую тишину
/// действие уходит далеко, и старая фраза закладку только запутает.
const RECENT_SUBTITLE_SEC: f64 = 8.0;

/// Текущие реплики субтитров.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SubtitleText {
    pub main: Option<String>,
    pub secondary: Option<String>,
}

impl SubtitleText {
    /// Нет ни одной реплики.
    ///
    /// Пригодится меню и поиску на следующих этапах; сейчас проверяется
    /// только тестами.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.main.is_none() && self.secondary.is_none()
    }
}

/// Какая дорожка выбрана сейчас.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SelectedTracks {
    pub audio: Option<i64>,
    pub subtitle: Option<i64>,
    pub secondary_subtitle: Option<i64>,
}

impl PithApp {
    /// Дорожки текущего файла.
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn selected_tracks(&self) -> SelectedTracks {
        self.selected_tracks
    }

    /// Перечитывает список дорожек и текущий выбор.
    ///
    /// Список меняется только при смене файла, поэтому держим его в кэше:
    /// опрашивать mpv каждый кадр незачем.
    pub(super) fn refresh_tracks(&mut self) {
        let Some(engine) = self.engine.as_ref() else {
            return;
        };

        // Список дорожек читается из mpv по частям: у файла с десятком
        // дорожек это десятки обращений к свойствам.
        self.tracks = crate::slow::probe("чтение списка дорожек", || {
            engine.tracks()
        });
        self.refresh_selected_tracks();
    }

    /// Обновляет отметки о выбранных дорожках.
    pub(super) fn refresh_selected_tracks(&mut self) {
        let Some(engine) = self.engine.as_ref() else {
            return;
        };

        self.selected_tracks = SelectedTracks {
            audio: engine.current_audio_track(),
            subtitle: engine.current_subtitle_track(),
            secondary_subtitle: engine.current_secondary_subtitle_track(),
        };
    }

    /// Включает дорожку субтитров вручную.
    pub fn choose_subtitle_track(&mut self, id: Option<i64>) {
        if let Some(engine) = self.engine.as_mut() {
            engine.set_subtitle_track(id);
        }
        self.refresh_selected_tracks();
    }

    /// Включает вторую дорожку субтитров вручную.
    pub fn choose_secondary_subtitle_track(&mut self, id: Option<i64>) {
        if let Some(engine) = self.engine.as_mut() {
            engine.set_secondary_subtitle_track(id);
        }
        self.refresh_selected_tracks();
    }

    /// Включает аудиодорожку вручную.
    pub fn choose_audio_track(&mut self, id: Option<i64>) {
        if let Some(engine) = self.engine.as_mut() {
            engine.set_audio_track(id);
        }
        self.refresh_selected_tracks();
    }

    /// Выбирает дорожки по тегам после загрузки файла.
    ///
    /// mpv отдаёт список сразу и достоверно, поэтому никаких повторных
    /// попыток по таймеру, как было в v4, не нужно.
    pub(super) fn select_tracks_by_tags(&mut self) {
        let Some(engine) = self.engine.as_ref() else {
            return;
        };

        let tracks = crate::slow::probe("список дорожек у mpv", || engine.tracks());
        if tracks.is_empty() {
            return;
        }

        let rules = self.settings.subtitle_priority.clone();
        let subtitles: Vec<&Track> = tracks
            .iter()
            .filter(|t| t.kind == TrackKind::Subtitle)
            .collect();

        let main = rules
            .main_enabled
            .then(|| best_track(&subtitles, &rules.main_tags, &rules.blacklist_tags))
            .flatten();

        // Вторые субтитры не должны повторять основные.
        let secondary = rules
            .secondary_enabled
            .then(|| {
                let candidates: Vec<&Track> = subtitles
                    .iter()
                    .filter(|t| Some(t.id) != main)
                    .copied()
                    .collect();
                best_track(&candidates, &rules.secondary_tags, &rules.blacklist_tags)
            })
            .flatten();

        let audio: Vec<&Track> = tracks
            .iter()
            .filter(|t| t.kind == TrackKind::Audio)
            .collect();
        let audio_choice = best_track(&audio, &self.settings.audio_languages, &[]);

        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        crate::slow::probe("включение дорожек", || {
            if let Some(id) = main {
                engine.set_subtitle_track(Some(id));
            }
            if let Some(id) = secondary {
                engine.set_secondary_subtitle_track(Some(id));
            }
            if let Some(id) = audio_choice {
                engine.set_audio_track(Some(id));
            }
        });

        tracing::info!(
            ?main,
            ?secondary,
            аудио = ?audio_choice,
            дорожек = tracks.len(),
            "дорожки выбраны по тегам"
        );
    }

    /// Обновляет текущие реплики субтитров.
    ///
    /// Реплики приходят подпиской на свойства mpv, спрашивать его не нужно:
    /// прежде их читали дважды на каждом кадре, и на занятом mpv это стоило
    /// сотен миллисекунд замершего окна — при открытии файла и первые
    /// секунды после перемотки (PLAN.md §6.14).
    pub(super) fn refresh_subtitle_text(&mut self) {
        let Some(engine) = self.engine.as_ref() else {
            return;
        };

        let fresh = SubtitleText {
            main: engine.subtitle_text(),
            secondary: engine.secondary_subtitle_text(),
        };

        // Смена реплики — редкое событие, логировать его дёшево и полезно:
        // сразу видно, доходят ли субтитры от mpv.
        if fresh != self.subtitle_text {
            tracing::debug!(
                основные = ?fresh.main,
                вторые = ?fresh.secondary,
                "реплика субтитров сменилась"
            );
        }

        // Последняя прозвучавшая реплика запоминается: закладку ставят
        // и в тишине между репликами, а название ей всё равно нужно.
        if let Some(line) = fresh.main.clone().or_else(|| fresh.secondary.clone()) {
            self.last_subtitle = Some(RecentLine {
                text: line,
                at: engine.state().position,
            });
        }

        self.subtitle_text = fresh;
    }

    pub fn subtitle_text(&self) -> &SubtitleText {
        &self.subtitle_text
    }

    /// Реплика, прозвучавшая только что, — на случай тишины.
    ///
    /// Ставя закладку в паузе между репликами, пользователь имеет в виду
    /// ту, что сейчас отзвучала, а не пустое название. Давняя реплика
    /// не годится: за полминуты тишины действие ушло далеко. Реплики
    /// «из будущего» не берём — их плеер ещё не видел.
    pub(super) fn recent_subtitle_line(&self) -> Option<String> {
        let recent = self.last_subtitle.as_ref()?;
        let now = self.engine.as_ref()?.state().position;
        let elapsed = now - recent.at;

        (0.0..=RECENT_SUBTITLE_SEC)
            .contains(&elapsed)
            .then(|| recent.text.clone())
    }
}

/// Реплика, которую плеер показывал последней.
pub struct RecentLine {
    pub text: String,
    /// Позиция воспроизведения, на которой её видели.
    pub at: f64,
}

/// Самая подходящая дорожка по тегам.
///
/// Если ни одна не набрала веса, берём помеченную в файле как основную —
/// лучше показать хоть что-то, чем ничего.
fn best_track(tracks: &[&Track], tags: &[String], blacklist: &[String]) -> Option<i64> {
    let best = tracks
        .iter()
        .map(|track| (track, tag_score(&track.search_text(), tags, blacklist)))
        .filter(|(_, score)| *score > 0)
        .max_by_key(|(_, score)| *score);

    if let Some((track, _)) = best {
        return Some(track.id);
    }

    tracks
        .iter()
        .find(|track| track.default)
        .map(|track| track.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn дорожка(id: i64, title: &str, lang: &str, default: bool) -> Track {
        Track {
            id,
            kind: TrackKind::Subtitle,
            title: Some(title.to_string()),
            lang: Some(lang.to_string()),
            default,
            forced: false,
            selected: false,
        }
    }

    fn теги(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn выбирается_дорожка_с_лучшим_совпадением() {
        let русские = дорожка(1, "Русские", "rus", false);
        let английские = дорожка(2, "English SDH", "eng", false);
        let tracks = vec![&русские, &английские];

        let выбор = best_track(&tracks, &теги(&["sdh", "english"]), &[]);
        assert_eq!(выбор, Some(2));
    }

    #[test]
    fn без_совпадений_берётся_дорожка_по_умолчанию() {
        let первая = дорожка(1, "Deutsch", "ger", false);
        let вторая = дорожка(2, "Français", "fra", true);
        let tracks = vec![&первая, &вторая];

        assert_eq!(best_track(&tracks, &теги(&["english"]), &[]), Some(2));
    }

    #[test]
    fn без_совпадений_и_без_умолчания_ничего_не_выбирается() {
        let единственная = дорожка(1, "Deutsch", "ger", false);
        let tracks = vec![&единственная];

        assert_eq!(best_track(&tracks, &теги(&["english"]), &[]), None);
    }

    #[test]
    fn чёрный_список_отсекает_дорожку() {
        let комментарии = дорожка(1, "English commentary", "eng", false);
        let обычные = дорожка(2, "English", "eng", false);
        let tracks = vec![&комментарии, &обычные];

        let выбор = best_track(&tracks, &теги(&["english"]), &теги(&["commentary"]));
        assert_eq!(выбор, Some(2));
    }

    #[test]
    fn пустой_список_дорожек_не_ломает_выбор() {
        assert_eq!(best_track(&[], &теги(&["english"]), &[]), None);
    }

    #[test]
    fn отсутствие_текста_считается_пустым() {
        assert!(SubtitleText::default().is_empty());
        assert!(
            !SubtitleText {
                main: Some("реплика".into()),
                secondary: None,
            }
            .is_empty()
        );
    }
}
