//! Форма кадра, узнанная до создания окна.
//!
//! Окно подстраивается под соотношение сторон видео, но `video-params/dw`
//! появляется только когда mpv настроил вывод — а это полсекунды спустя:
//! сначала заводится декодер и звуковое устройство. Окно к тому времени
//! уже показано в прежнем размере, и подгонка видна скачком.
//!
//! Демуксер знает размеры сразу: замер показал 17 мс от создания движка
//! до готового ответа. За это время окна ещё нет, и открыть его можно
//! сразу нужным.
//!
//! Экземпляр здесь служебный: без вывода, без звука и без декодера —
//! он только открывает контейнер и читает заголовок.

use std::time::{Duration, Instant};

use crate::Engine;

/// Сколько ждать ответа, прежде чем открыть окно как получится.
///
/// Локальный файл отвечает за десятки миллисекунд. Срок нужен для сетевых
/// адресов и битых файлов: задерживать окно ради формы кадра дольше —
/// значит менять один изъян на другой, куда более заметный.
pub const PROBE_TIMEOUT: Duration = Duration::from_millis(250);

/// Форма кадра `(ширина, высота)` с учётом поворота из метаданных.
///
/// `None` — узнать не удалось: файла нет, он не видео или не ответил в срок.
/// Это штатный исход, окно тогда открывается как раньше.
pub fn probe(path: &str, timeout: Duration) -> Option<(i64, i64)> {
    let started = Instant::now();

    let mut engine = Engine::with_options(options()).ok()?;
    engine.load_file(path).ok()?;

    loop {
        if let Some(size) = engine.demux_video_size() {
            tracing::debug!(
                ms = started.elapsed().as_millis(),
                ширина = size.0,
                высота = size.1,
                "форма кадра узнана до окна"
            );
            return Some(size);
        }

        let left = timeout.checked_sub(started.elapsed())?;

        // Просыпаемся на каждом событии движка; верхняя граница ожидания
        // нужна лишь на случай, если событий не будет вовсе.
        engine.wait_any_event(left.min(Duration::from_millis(20)).as_secs_f64());
    }
}

/// Опции служебного экземпляра: ничего лишнего, только чтение заголовка.
fn options() -> Vec<(&'static str, String)> {
    vec![
        ("vo", "null".into()),
        ("ao", "null".into()),
        ("audio-display", "no".into()),
        ("aid", "no".into()),
        ("sid", "no".into()),
        ("sub-auto", "no".into()),
        ("audio-file-auto", "no".into()),
        ("hwdec", "no".into()),
        ("pause", "yes".into()),
        ("cache", "no".into()),
        // Читаем только заголовок: дальше файла заглядывать незачем.
        ("demuxer-max-bytes", "2MiB".into()),
        ("config", "no".into()),
        ("load-scripts", "no".into()),
        ("osc", "no".into()),
        ("terminal", "no".into()),
        ("input-default-bindings", "no".into()),
        ("input-vo-keyboard", "no".into()),
    ]
}

#[cfg(test)]
mod tests {
    use super::options;

    #[test]
    fn служебный_экземпляр_молчит_и_не_рисует() {
        let options = options();
        let get = |name: &str| {
            options
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.as_str())
        };

        assert_eq!(get("vo"), Some("null"), "окна у него быть не должно");
        assert_eq!(get("ao"), Some("null"), "и звука тоже");
        assert_eq!(get("hwdec"), Some("no"), "декодер не заводим вовсе");
    }
}
