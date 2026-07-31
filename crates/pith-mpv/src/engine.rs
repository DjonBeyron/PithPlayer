//! Движок воспроизведения — обёртка над libmpv.
//!
//! Интерфейс приложения обращается к mpv только через этот тип (PLAN.md §12.4).

use std::sync::Arc;

use libmpv2::Mpv;
use libmpv2::events::Event;

use crate::error::{MpvError, Result};
use crate::options::EngineOptions;
use crate::render::RenderContext;

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
    /// mpv завершает работу.
    Shutdown,
}

pub struct Engine {
    /// ВАЖНО: объявлен первым — Rust уничтожает поля в порядке объявления,
    /// поэтому контекст отрисовки освободится раньше `mpv`, на который ссылается.
    render_ctx: Option<Arc<RenderContext>>,
    /// В куче: адрес должен быть стабилен, на него ссылается контекст отрисовки.
    mpv: Box<Mpv>,
    state: PlaybackState,
}

impl Engine {
    /// Создаёт движок. Ошибка здесь означает, что libmpv недоступна —
    /// приложение должно показать понятное сообщение, а не падать.
    pub fn new(options: &EngineOptions) -> Result<Self> {
        let mpv = Mpv::with_initializer(|init| {
            for (name, value) in options.to_mpv_options() {
                init.set_property(name, value.as_str())?;
            }
            Ok(())
        })
        .map_err(|e| MpvError::Init(e.to_string()))?;

        tracing::info!(hwdec = options.hwdec.as_mpv_value(), "движок mpv запущен");

        Ok(Self {
            render_ctx: None,
            mpv: Box::new(mpv),
            state: PlaybackState {
                volume: options.volume,
                ..Default::default()
            },
        })
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
        Ok(())
    }

    /// Перемотка на абсолютную позицию в секундах.
    pub fn seek_absolute(&mut self, seconds: f64) -> Result<()> {
        let target = seconds.max(0.0);
        self.mpv
            .command("seek", &[&target.to_string(), "absolute"])
            .map_err(|e| MpvError::command("seek", e))
    }

    /// Перемотка относительно текущей позиции.
    pub fn seek_relative(&mut self, delta_seconds: f64) -> Result<()> {
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
                    events.push(EngineEvent::FileLoaded);
                }
                Ok(Event::EndFile(reason)) => {
                    tracing::debug!(?reason, "воспроизведение файла завершено");
                    events.push(EngineEvent::EndFile);
                }
                Ok(Event::Shutdown) => events.push(EngineEvent::Shutdown),
                Ok(_) => {}
                Err(e) => {
                    // Ошибка разбора одного события не должна останавливать цикл.
                    tracing::warn!(error = %e, "ошибка при разборе события mpv");
                }
            }
        }

        events
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
    }

    /// Читает размеры кадра после загрузки файла.
    ///
    /// Берём `dw`/`dh`, а не `w`/`h`: они уже учитывают поворот из метаданных.
    /// Иначе вертикальное видео с телефона откроется лёжа (PLAN.md §6.12).
    pub fn refresh_video_size(&mut self) {
        let width = self.mpv.get_property::<i64>("video-params/dw").ok();
        let height = self.mpv.get_property::<i64>("video-params/dh").ok();

        match (width, height) {
            (Some(w), Some(h)) if w > 0 && h > 0 => {
                self.state.display_width = w;
                self.state.display_height = h;
                tracing::debug!(w, h, "размеры кадра получены");
            }
            _ => {
                // Штатная ситуация: аудиофайл либо кадр ещё не готов.
                tracing::debug!("размеры кадра недоступны — окно не трогаем");
            }
        }
    }

    /// Читает произвольное свойство. Нужно для замеров и диагностики.
    pub fn property_string(&self, name: &str) -> Result<String> {
        self.mpv
            .get_property::<String>(name)
            .map_err(|e| MpvError::property(name, e))
    }

    /// Фактически применённый режим аппаратного декодирования.
    ///
    /// Отличается от запрошенного, если mpv не смог включить нужный режим
    /// и молча откатился. Без этой проверки замеры этапа 0 недостоверны.
    pub fn active_hwdec(&self) -> String {
        self.property_string("hwdec-current")
            .unwrap_or_else(|_| "неизвестно".into())
    }
}
