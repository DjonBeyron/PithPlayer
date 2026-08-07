//! Движок воспроизведения — обёртка над libmpv.
//!
//! Интерфейс приложения обращается к mpv только через этот тип (PLAN.md §12.4).

use std::sync::Arc;

use libmpv2::Mpv;

use crate::error::{MpvError, Result};
use crate::options::EngineOptions;
use crate::render::RenderContext;

/// Границы скорости воспроизведения.
pub const MIN_SPEED: f64 = 0.25;
pub const MAX_SPEED: f64 = 4.0;

/// Состояние воспроизведения для отрисовки интерфейса.
///
/// Все поля — «последнее известное»: если mpv ещё не сообщил значение,
/// остаётся предыдущее. Это нормально и не является ошибкой.
#[derive(Debug, Clone, Default)]
pub struct PlaybackState {
    /// Текущая позиция, секунды.
    pub position: f64,
    /// Длительность, секунды. Ноль — ещё неизвестна.
    pub duration: f64,
    pub paused: bool,
    /// Файл загружен и готов к воспроизведению.
    pub file_loaded: bool,
    pub volume: i64,
    /// Скорость воспроизведения: 1.0 — обычная.
    pub speed: f64,
    /// Звук выключен.
    ///
    /// Отдельно от нулевой громкости: выключение звука должно возвращать
    /// ту громкость, что была, а не ноль.
    pub muted: bool,
    /// Файл повторяется по кругу.
    pub looping: bool,
    /// Файл доигран до конца и стоит на последнем кадре.
    ///
    /// mpv в этом состоянии остаётся с открытым файлом (`keep-open`),
    /// но играть ему уже нечего: обычное «продолжить» ничего не делает.
    pub finished: bool,
    /// Отображаемая ширина кадра с учётом поворота (`video-params/dw`).
    pub display_width: i64,
    /// Отображаемая высота кадра с учётом поворота (`video-params/dh`).
    pub display_height: i64,
}

impl PlaybackState {
    /// Соотношение сторон кадра. `None`, если размеры ещё неизвестны
    /// или это файл без видео.
    pub fn aspect_ratio(&self) -> Option<f32> {
        if self.display_width > 0 && self.display_height > 0 {
            Some(self.display_width as f32 / self.display_height as f32)
        } else {
            None
        }
    }
}

/// События движка, интересные приложению.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineEvent {
    /// Файл загружен: можно читать размеры кадра и дорожки.
    FileLoaded,
    /// Воспроизведение файла завершено.
    EndFile,
    /// Перемотка закончилась, картинка на новом месте.
    ///
    /// Ждать этого события надёжнее и дешевле, чем опрашивать позицию:
    /// пока mpv перематывает, любое обращение к его свойствам блокирует
    /// поток интерфейса.
    SeekDone,
    /// Файл не удалось прочитать или декодировать.
    ///
    /// Битый файл, отсутствующий кодек, файл удалили во время просмотра —
    /// mpv сообщает обо всём этом одинаково, и плеер обязан остаться живым.
    PlaybackError,
    /// mpv завершает работу.
    Shutdown,
}

pub struct Engine {
    /// ВАЖНО: объявлен первым — Rust уничтожает поля в порядке объявления,
    /// поэтому контекст отрисовки освободится раньше `mpv`, на который ссылается.
    render_ctx: Option<Arc<RenderContext>>,
    /// В куче: адрес должен быть стабилен, на него ссылается контекст отрисовки.
    ///
    /// Видно всему крейту: разбор событий живёт в соседнем модуле.
    pub(crate) mpv: Box<Mpv>,
    pub(crate) state: PlaybackState,
}

impl Engine {
    /// Создаёт движок. Ошибка здесь означает, что libmpv недоступна —
    /// приложение должно показать понятное сообщение, а не падать.
    pub fn new(options: &EngineOptions) -> Result<Self> {
        let engine = Self::with_options(options.to_mpv_options())?;
        tracing::info!(hwdec = options.hwdec.as_mpv_value(), "движок mpv запущен");

        Ok(Self {
            state: PlaybackState {
                volume: options.volume,
                muted: options.muted,
                looping: options.looping,
                speed: 1.0,
                ..Default::default()
            },
            ..engine
        })
    }

    /// Движок со своим набором опций — для служебных экземпляров без окна.
    pub(crate) fn with_options(options: Vec<(&'static str, String)>) -> Result<Self> {
        let mpv = Mpv::with_initializer(|init| {
            for (name, value) in &options {
                init.set_property(name, value.as_str())?;
            }
            Ok(())
        })
        .map_err(|e| MpvError::Init(e.to_string()))?;

        Ok(Self {
            render_ctx: None,
            mpv: Box::new(mpv),
            state: PlaybackState::default(),
        })
    }

    /// Ждёт любого события mpv, но не дольше срока.
    ///
    /// Нужен служебным экземплярам: они ничего не рисуют и просто ждут,
    /// когда движок разберётся с файлом. Опроса по кругу здесь нет —
    /// будит сама очередь событий.
    pub(crate) fn wait_any_event(&mut self, seconds: f64) {
        let _ = self.mpv.wait_event(seconds);
    }

    pub(crate) fn mpv_ref(&self) -> &Mpv {
        &self.mpv
    }

    pub(crate) fn set_render_context(&mut self, context: Arc<RenderContext>) {
        self.render_ctx = Some(context);
    }

    pub(crate) fn render_context(&self) -> Option<&Arc<RenderContext>> {
        self.render_ctx.as_ref()
    }

    pub fn state(&self) -> &PlaybackState {
        &self.state
    }

    /// Открывает файл. mpv понимает и локальные пути, и URL.
    ///
    /// Путь передаётся без кавычек: libmpv экранирует аргументы сам,
    /// а лишние кавычки становятся частью имени файла.
    pub fn load_file(&mut self, path: &str) -> Result<()> {
        self.mpv
            .command("loadfile", &[path])
            .map_err(|e| MpvError::command("loadfile", e))?;

        // Размеры кадра станут известны только после загрузки — сбрасываем,
        // чтобы не подогнать окно под предыдущий файл.
        self.state.file_loaded = false;
        self.state.display_width = 0;
        self.state.display_height = 0;
        self.state.duration = 0.0;
        self.state.position = 0.0;

        tracing::info!(path, "открываю файл");
        Ok(())
    }

    /// Пауза / продолжение.
    pub fn toggle_pause(&mut self) -> Result<()> {
        self.set_paused(!self.state.paused)
    }

    pub fn set_paused(&mut self, paused: bool) -> Result<()> {
        self.mpv
            .set_property("pause", paused)
            .map_err(|e| MpvError::command("pause", e))?;
        self.state.paused = paused;
        tracing::debug!(paused, "пауза переключена");
        Ok(())
    }

    /// Перемотка на абсолютную позицию в секундах.
    pub fn seek_absolute(&mut self, seconds: f64) -> Result<()> {
        let target = seconds.max(0.0);
        tracing::debug!(target, "перемотка на позицию");
        self.mpv
            .command("seek", &[&target.to_string(), "absolute"])
            .map_err(|e| MpvError::command("seek", e))
    }

    /// Быстрая перемотка по ближайшему опорному кадру.
    ///
    /// Нужна, пока пользователь тянет ползунок: точная перемотка
    /// декодирует всё от опорного кадра до нужной миллисекунды, и на 4К
    /// картинка отстаёт от мыши. По опорным кадрам она поспевает,
    /// а точное место доводится по отпусканию.
    pub fn seek_keyframe(&mut self, seconds: f64) -> Result<()> {
        let target = seconds.max(0.0);
        tracing::debug!(target, "быстрая перемотка");

        self.mpv
            .command("seek", &[&target.to_string(), "absolute+keyframes"])
            .map_err(|e| MpvError::command("seek", e))
    }

    /// Перемотка относительно текущей позиции.
    pub fn seek_relative(&mut self, delta_seconds: f64) -> Result<()> {
        tracing::debug!(delta_seconds, "перемотка относительно текущей");
        self.mpv
            .command("seek", &[&delta_seconds.to_string(), "relative"])
            .map_err(|e| MpvError::command("seek", e))
    }

    /// Громкость 0–150.
    pub fn set_volume(&mut self, volume: i64) -> Result<()> {
        let clamped = volume.clamp(0, 150);
        self.mpv
            .set_property("volume", clamped)
            .map_err(|e| MpvError::command("volume", e))?;
        self.state.volume = clamped;
        Ok(())
    }

    /// Зацикливает файл или выключает цикл.
    pub fn set_looping(&mut self, looping: bool) -> Result<()> {
        let value = if looping { "inf" } else { "no" };

        self.mpv
            .set_property("loop-file", value)
            .map_err(|e| MpvError::command("loop-file", e))?;

        self.state.looping = looping;
        tracing::debug!(looping, "повтор файла переключён");
        Ok(())
    }

    /// Начинает файл сначала.
    ///
    /// Нужна, когда фильм доигран: mpv держит последний кадр, и обычное
    /// «продолжить» ему уже нечего играть.
    pub fn restart(&mut self) -> Result<()> {
        self.state.finished = false;
        self.seek_absolute(0.0)?;
        self.set_paused(false)
    }

    /// Выключает и включает звук, не трогая громкость.
    pub fn set_muted(&mut self, muted: bool) -> Result<()> {
        self.mpv
            .set_property("mute", muted)
            .map_err(|e| MpvError::command("mute", e))?;

        self.state.muted = muted;
        tracing::debug!(muted, "звук переключён");
        Ok(())
    }

    /// Скорость воспроизведения.
    ///
    /// Ограничена разумными пределами: звук за их границами превращается
    /// в кашу, а mpv начинает ронять кадры.
    pub fn set_speed(&mut self, speed: f64) -> Result<()> {
        let clamped = speed.clamp(MIN_SPEED, MAX_SPEED);

        self.mpv
            .set_property("speed", clamped)
            .map_err(|e| MpvError::command("speed", e))?;

        self.state.speed = clamped;
        tracing::debug!(speed = clamped, "скорость изменена");
        Ok(())
    }

    /// Обновляет состояние из свойств mpv.
    ///
    /// Недоступное свойство — не ошибка: mpv ещё не готов его отдать,
    /// оставляем прежнее значение.
    pub fn refresh_state(&mut self) {
        if let Ok(v) = self.mpv.get_property::<f64>("time-pos") {
            self.state.position = v;
        }
        if let Ok(v) = self.mpv.get_property::<f64>("duration") {
            self.state.duration = v;
        }
        if let Ok(v) = self.mpv.get_property::<bool>("pause") {
            self.state.paused = v;
        }
        if let Ok(v) = self.mpv.get_property::<i64>("volume") {
            self.state.volume = v;
        }
        if let Ok(v) = self.mpv.get_property::<bool>("mute") {
            self.state.muted = v;
        }
        if let Ok(v) = self.mpv.get_property::<f64>("speed") {
            self.state.speed = v;
        }
    }

    /// Запоминает размеры кадра. Заполняется разбором свойств в `video.rs`.
    pub(crate) fn set_display_size(&mut self, width: i64, height: i64) {
        self.state.display_width = width;
        self.state.display_height = height;
    }

    /// Останавливает воспроизведение и выгружает файл.
    ///
    /// Обязательно вызывать перед завершением работы: пока mpv декодирует
    /// и ждёт отрисовки очередного кадра, освобождение контекста отрисовки
    /// блокируется и приложение зависает при закрытии.
    pub fn stop(&mut self) {
        if let Err(e) = self.mpv.command("stop", &[]) {
            tracing::warn!(error = %e, "не удалось остановить воспроизведение");
        }
        self.state.file_loaded = false;
    }

    /// Сколько ссылок на контекст отрисовки живо.
    ///
    /// Больше одной означает, что копия задержалась в обратном вызове egui
    /// и контекст не освободится вовремя.
    pub fn render_context_refs(&self) -> usize {
        self.render_ctx.as_ref().map_or(0, Arc::strong_count)
    }

    /// Читает произвольное свойство. Нужно для замеров и диагностики.
    pub fn property_string(&self, name: &str) -> Result<String> {
        self.mpv
            .get_property::<String>(name)
            .map_err(|e| MpvError::property(name, e))
    }

    pub(crate) fn property_i64(&self, name: &str) -> Option<i64> {
        self.mpv.get_property::<i64>(name).ok()
    }

    pub(crate) fn property_bool(&self, name: &str) -> Option<bool> {
        self.mpv.get_property::<bool>(name).ok()
    }

    /// Записывает строковое свойство.
    pub(crate) fn set_property_string(&mut self, name: &str, value: &str) -> Result<()> {
        self.mpv
            .set_property(name, value)
            .map_err(|e| MpvError::command(name, e))
    }
}
