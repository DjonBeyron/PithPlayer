//! Второй экземпляр mpv — источник кадров для предпросмотра.
//!
//! Раньше каждый кадр подсказки доставал отдельный запуск `ffmpeg`. Дорого
//! в нём не декодирование, а всё вокруг: запуск процесса, открытие файла,
//! разбор заголовков и построение индекса — и так на каждое движение мыши.
//!
//! Здесь файл открыт один раз и остаётся открытым: на запрос уходит только
//! перемотка по опорным кадрам и снимок. Так же устроен thumbfast у mpv.
//! Экземпляр отдельный, потому что основной занят фильмом: перематывать
//! его ради подсказки нельзя.

use std::path::{Path, PathBuf};
use std::time::Instant;

use libmpv2::Mpv;
use libmpv2::events::Event;

use crate::error::{MpvError, Result};

/// Ширина кадра предпросмотра, точки. Высота — по пропорциям кадра.
///
/// Кадр уменьшает сам mpv фильтром: снимать 4К ради окошка в 240 точек
/// значит гонять по памяти в сотню раз больше данных, чем нужно.
const WIDTH: u32 = 240;

/// Сколько ждать кадр после перемотки, секунды.
///
/// Обычная перемотка по опорным кадрам укладывается в десятки
/// миллисекунд; предел нужен на случай, когда файл читается с сетевого
/// диска и ответа можно не дождаться вовсе.
const SEEK_TIMEOUT: f64 = 3.0;

/// Сколько ждать загрузку файла, секунды.
const LOAD_TIMEOUT: f64 = 10.0;

/// Отдельный mpv, который умеет только отдавать кадры.
pub struct PreviewEngine {
    mpv: Mpv,
    /// Куда mpv кладёт снимок. Файл один и тот же, перезаписывается.
    shot: PathBuf,
}

impl PreviewEngine {
    /// Запускает экземпляр. `shot` — файл для снимков, он будет
    /// перезаписываться на каждый кадр.
    pub fn new(shot: PathBuf) -> Result<Self> {
        let mpv = Mpv::with_initializer(|init| {
            for (name, value) in options() {
                init.set_property(name, value.as_str())?;
            }
            Ok(())
        })
        .map_err(|e| MpvError::Init(e.to_string()))?;

        tracing::debug!(?shot, "запущен экземпляр mpv для предпросмотра");

        Ok(Self { mpv, shot })
    }

    /// Открывает файл и ждёт, пока он будет готов отдавать кадры.
    pub fn load_file(&mut self, path: &Path) -> Result<()> {
        let path = path.to_string_lossy().to_string();

        self.mpv
            .command("loadfile", &[&path])
            .map_err(|e| MpvError::command("loadfile", e))?;

        self.wait_for(LOAD_TIMEOUT, |event| matches!(event, Event::FileLoaded))?;

        tracing::debug!(path, "предпросмотр: файл открыт");
        Ok(())
    }

    /// Достаёт кадр указанной секунды. Возвращает JPEG в памяти.
    pub fn grab(&mut self, time: f64) -> Result<Vec<u8>> {
        // Хвосты прошлых команд не должны сойти за ответ на эту.
        self.drain_events();

        let target = time.max(0.0).to_string();
        self.mpv
            .command("seek", &[&target, "absolute+keyframes"])
            .map_err(|e| MpvError::command("seek", e))?;

        // Снимок до окончания перемотки вернул бы прежнее место.
        self.wait_for(SEEK_TIMEOUT, |event| {
            matches!(event, Event::PlaybackRestart)
        })?;

        let shot = self.shot.to_string_lossy().to_string();
        self.mpv
            .command("screenshot-to-file", &[&shot, "video"])
            .map_err(|e| MpvError::command("screenshot-to-file", e))?;

        let data = std::fs::read(&self.shot)
            .map_err(|e| MpvError::Init(format!("кадр предпросмотра не прочитался: {e}")))?;

        // Файл больше не нужен: следующий снимок пишется заново, а этот
        // иначе останется в каталоге временных файлов после закрытия.
        let _ = std::fs::remove_file(&self.shot);

        Ok(data)
    }

    /// Ждёт нужное событие, разбирая очередь.
    fn wait_for(&mut self, timeout: f64, wanted: impl Fn(&Event) -> bool) -> Result<()> {
        let started = Instant::now();

        while started.elapsed().as_secs_f64() < timeout {
            let Some(result) = self.mpv.wait_event(0.05) else {
                continue;
            };

            match result {
                Ok(event) if wanted(&event) => return Ok(()),
                Ok(Event::Shutdown) => {
                    return Err(MpvError::Init("предпросмотр: mpv завершился".into()));
                }
                Ok(_) => {}
                // Неудача с файлом приходит ошибкой, а не событием. Ждать
                // дальше нечего: кадра из этого файла не будет.
                Err(e) => {
                    return Err(MpvError::Init(format!("предпросмотр: {e}")));
                }
            }
        }

        Err(MpvError::Init("предпросмотр: кадр не дождался".into()))
    }

    /// Выбрасывает накопившиеся события.
    fn drain_events(&mut self) {
        while self.mpv.wait_event(0.0).is_some() {}
    }
}

/// Опции экземпляра: ничего лишнего, только декодирование кадров.
fn options() -> Vec<(&'static str, String)> {
    vec![
        // Ни окна, ни вывода: кадр забирается снимком.
        ("vo", "null".into()),
        ("ao", "null".into()),
        ("audio-display", "no".into()),
        // Звук, субтитры и прочие дорожки только замедлили бы открытие.
        ("aid", "no".into()),
        ("sid", "no".into()),
        ("sub-auto", "no".into()),
        ("audio-file-auto", "no".into()),
        // Аппаратное декодирование ради одного кадра настраивается дольше,
        // чем работает: на замере этапа 4 оно проигрывало программному.
        ("hwdec", "no".into()),
        // Экземпляр не играет, а стоит на нужном месте.
        ("pause", "yes".into()),
        ("keep-open", "always".into()),
        ("untimed", "yes".into()),
        // Ни своих настроек пользователя, ни скриптов, ни горячих клавиш:
        // это служебный экземпляр, он обязан вести себя одинаково у всех.
        ("config", "no".into()),
        ("load-scripts", "no".into()),
        ("osc", "no".into()),
        ("osd-level", "0".into()),
        ("input-default-bindings", "no".into()),
        ("input-vo-keyboard", "no".into()),
        ("terminal", "no".into()),
        // Уменьшаем кадр до открытия файла: снимок 4К ради окошка
        // в четверть тысячи точек — впустую прогнанная память.
        ("vf", format!("scale={WIDTH}:-2")),
        // Точность здесь не нужна: подсказка показывает, что примерно
        // происходит в этом месте, а опорный кадр находится сразу.
        ("hr-seek", "no".into()),
        // Пропускаем то, что на миниатюре всё равно не видно.
        ("vd-lavc-skiploopfilter", "all".into()),
        ("vd-lavc-fast", "yes".into()),
        // Небольшой запас чтения: экземпляр прыгает по файлу, и большой
        // кэш ему только мешает.
        ("cache", "no".into()),
        ("demuxer-max-bytes", "32MiB".into()),
        ("screenshot-format", "jpg".into()),
        ("screenshot-jpeg-quality", "80".into()),
    ]
}

#[cfg(test)]
mod tests {
    use super::{WIDTH, options};

    #[test]
    fn экземпляр_не_создаёт_окна_и_не_играет_звук() {
        let options = options();
        let get = |name: &str| {
            options
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.clone())
        };

        assert_eq!(
            get("vo").as_deref(),
            Some("null"),
            "своего окна быть не должно"
        );
        assert_eq!(get("ao").as_deref(), Some("null"), "звук не нужен");
        assert_eq!(get("aid").as_deref(), Some("no"));
        assert_eq!(get("pause").as_deref(), Some("yes"));
    }

    #[test]
    fn кадр_уменьшается_до_открытия_файла() {
        let options = options();
        let filter = options
            .iter()
            .find(|(key, _)| *key == "vf")
            .map(|(_, value)| value.clone())
            .expect("фильтр уменьшения обязателен");

        assert_eq!(filter, format!("scale={WIDTH}:-2"));
    }
}
