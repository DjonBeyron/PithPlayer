//! Подписка на свойства mpv вместо их опроса.
//!
//! Раньше состояние обновлялось чтением шести свойств на каждом кадре.
//! Чтение свойства ждёт ответа mpv прямо в потоке интерфейса, а занятый
//! mpv отвечает не сразу: при открытии 4К-файла окно замирало на 488 мс
//! до загрузки и ещё на 200 мс сразу после неё (PLAN.md §6.14).
//!
//! Теперь mpv сам присылает новые значения событием `PropertyChange`.
//! Приложение их только разбирает — ждать нечего, и опроса по кругу
//! больше нет, как и требует правило проекта.

use libmpv2::Format;
use libmpv2::events::PropertyData;

use crate::engine::{Engine, PlaybackState};

/// Свойства, за которыми следим, и их вид.
///
/// Номер подписки нам не нужен — свойство узнаётся по имени в событии.
const WATCHED: &[(&str, Format)] = &[
    ("time-pos", Format::Double),
    ("duration", Format::Double),
    ("pause", Format::Flag),
    ("volume", Format::Int64),
    ("mute", Format::Flag),
    ("speed", Format::Double),
    // Реплики субтитров тоже приходят сами. Прежде их спрашивали дважды
    // на каждом кадре, и на занятом mpv это стоило 197 мс замершего окна.
    ("sub-text", Format::String),
    ("secondary-sub-text", Format::String),
    // Размеры кадра с учётом поворота — по ним окно принимает форму видео.
    // Спрошенные на загрузке, они стоили 198 мс замершего окна, а приходят
    // всё равно позже: mpv называет их, когда настроит вывод.
    ("dwidth", Format::Int64),
    ("dheight", Format::Int64),
];

/// Один общий номер на все подписки: снимать их по отдельности не нужно.
const SUBSCRIPTION: u64 = 1;

impl Engine {
    /// Подписывается на свойства воспроизведения.
    ///
    /// Неудача не смертельна: без подписки останутся прежние значения,
    /// плеер продолжит играть, а не упадёт.
    pub(crate) fn observe_playback_properties(&self) {
        for (name, format) in WATCHED {
            if let Err(e) = self.mpv_ref().observe_property(name, *format, SUBSCRIPTION) {
                tracing::warn!(свойство = name, error = %e, "не удалось подписаться на свойство");
            }
        }
    }
}

/// Принимает новое значение свойства из события mpv.
///
/// Берёт само состояние, а не движок целиком: очередь событий читается
/// из `self.mpv`, и одолжить весь движок посреди разбора нельзя.
pub(crate) fn apply(state: &mut PlaybackState, name: &str, change: &PropertyData) {
    match (name, change) {
        ("time-pos", PropertyData::Double(v)) => state.position = *v,
        ("duration", PropertyData::Double(v)) => state.duration = *v,
        ("pause", PropertyData::Flag(v)) => state.paused = *v,
        ("volume", PropertyData::Int64(v)) => state.volume = *v,
        ("mute", PropertyData::Flag(v)) => state.muted = *v,
        ("speed", PropertyData::Double(v)) => state.speed = *v,
        ("sub-text", PropertyData::Str(v)) => state.subtitle = line(v),
        ("secondary-sub-text", PropertyData::Str(v)) => state.secondary_subtitle = line(v),
        // Нулевые размеры приходят при смене файла — это «пока неизвестно»,
        // и окно по ним трогать нельзя.
        ("dwidth", PropertyData::Int64(v)) => state.display_width = *v,
        ("dheight", PropertyData::Int64(v)) => state.display_height = *v,
        _ => {}
    }
}

/// Пустая реплика означает «сейчас субтитров нет».
fn line(text: &str) -> Option<String> {
    (!text.trim().is_empty()).then(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::{PlaybackState, PropertyData, apply};

    #[test]
    fn значения_свойств_ложатся_в_состояние() {
        let mut state = PlaybackState::default();

        apply(&mut state, "time-pos", &PropertyData::Double(12.5));
        apply(&mut state, "duration", &PropertyData::Double(300.0));
        apply(&mut state, "pause", &PropertyData::Flag(true));
        apply(&mut state, "volume", &PropertyData::Int64(70));
        apply(&mut state, "mute", &PropertyData::Flag(true));
        apply(&mut state, "speed", &PropertyData::Double(1.5));

        assert_eq!(state.position, 12.5);
        assert_eq!(state.duration, 300.0);
        assert!(state.paused);
        assert_eq!(state.volume, 70);
        assert!(state.muted);
        assert_eq!(state.speed, 1.5);
    }

    #[test]
    fn чужое_свойство_состояние_не_трогает() {
        let mut state = PlaybackState {
            position: 5.0,
            ..Default::default()
        };

        // Вид значения не тот, какой ждали, — оставляем прежнее.
        apply(&mut state, "time-pos", &PropertyData::Int64(99));
        apply(&mut state, "чужое-свойство", &PropertyData::Double(99.0));

        assert_eq!(state.position, 5.0);
    }
}
