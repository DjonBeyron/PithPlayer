//! Разбор очереди событий mpv.
//!
//! Вынесено из `engine`: там управление воспроизведением, а здесь —
//! перевод событий libmpv в те, что понимает приложение, и разбор
//! кодов ошибок.

use libmpv2::events::Event;

use crate::engine::{Engine, EngineEvent};

/// Коды ошибок mpv, означающие «этот файл воспроизвести не удалось».
///
/// Значения из `mpv_error`: файл не открылся, играть нечего, формат
/// неизвестен, формат не поддерживается. Остальные коды — сбои отдельных
/// команд и свойств, из-за них плеер останавливать нельзя.
const LOADING_FAILED: i32 = -13;
const NOTHING_TO_PLAY: i32 = -16;
const UNKNOWN_FORMAT: i32 = -17;
const UNSUPPORTED: i32 = -18;

fn is_playback_failure(code: i32) -> bool {
    matches!(
        code,
        LOADING_FAILED | NOTHING_TO_PLAY | UNKNOWN_FORMAT | UNSUPPORTED
    )
}

impl Engine {
    /// Разбирает очередь событий mpv без блокировки.
    ///
    /// Вызывается каждый кадр интерфейса.
    pub fn pump_events(&mut self) -> Vec<EngineEvent> {
        let mut events = Vec::new();

        // Ноль означает «не ждать»: очередь опрашивается до опустошения.
        while let Some(result) = self.mpv.wait_event(0.0) {
            match result {
                Ok(Event::FileLoaded) => {
                    self.state.file_loaded = true;
                    self.state.finished = false;
                    events.push(EngineEvent::FileLoaded);
                }
                Ok(Event::EndFile(reason)) => {
                    tracing::debug!(?reason, "воспроизведение файла завершено");
                    // Файл остаётся открытым на последнем кадре: нажатие
                    // «играть» после этого должно начинать сначала.
                    self.state.finished = true;
                    events.push(EngineEvent::EndFile);
                }
                Ok(Event::PlaybackRestart) => events.push(EngineEvent::SeekDone),
                Ok(Event::Shutdown) => events.push(EngineEvent::Shutdown),
                Ok(_) => {}
                // Неудачу с файлом libmpv2 отдаёт не событием, а ошибкой:
                // `EndFile` с ненулевым кодом подменяется на `Err`.
                Err(libmpv2::Error::Raw(code)) if is_playback_failure(code) => {
                    tracing::error!(code, "файл не удалось воспроизвести");
                    self.state.file_loaded = false;
                    events.push(EngineEvent::PlaybackError);
                }
                Err(e) => {
                    // Ошибка разбора одного события не должна останавливать цикл.
                    tracing::warn!(error = %e, "ошибка при разборе события mpv");
                }
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn неудача_с_файлом_узнаётся_по_коду() {
        assert!(is_playback_failure(UNKNOWN_FORMAT), "битый файл");
        assert!(is_playback_failure(LOADING_FAILED), "файл не открылся");
        assert!(is_playback_failure(UNSUPPORTED), "нет кодека");
        assert!(is_playback_failure(NOTHING_TO_PLAY), "нечего играть");
    }

    #[test]
    fn сбой_свойства_не_считается_неудачей_с_файлом() {
        // Свойство недоступно (-10) и команда не выполнилась (-12) случаются
        // в обычной работе: останавливать из-за них воспроизведение нельзя.
        assert!(!is_playback_failure(-10));
        assert!(!is_playback_failure(-12));
        assert!(!is_playback_failure(0));
    }
}
