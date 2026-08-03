//! Свойства картинки: размеры кадра и обрезка полей.

use crate::engine::Engine;
use crate::error::Result;

impl Engine {
    /// Читает размеры кадра после загрузки файла.
    ///
    /// Берём `dw`/`dh`, а не `w`/`h`: они уже учитывают поворот из метаданных.
    /// Иначе вертикальное видео с телефона откроется лёжа (PLAN.md §6.12).
    pub fn refresh_video_size(&mut self) {
        let width = self.property_i64("video-params/dw");
        let height = self.property_i64("video-params/dh");

        match (width, height) {
            (Some(w), Some(h)) if w > 0 && h > 0 => {
                self.set_display_size(w, h);
                tracing::debug!(w, h, "размеры кадра получены");
            }
            _ => {
                // Штатная ситуация: аудиофайл либо кадр ещё не готов.
                tracing::debug!("размеры кадра недоступны — окно не трогаем");
            }
        }
    }

    /// Размеры кадра исходника — по ним видно, что обрезать есть что.
    pub fn source_size(&self) -> Option<(i64, i64)> {
        let width = self.property_i64("video-params/w")?;
        let height = self.property_i64("video-params/h")?;

        (width > 0 && height > 0).then_some((width, height))
    }

    /// Обрезает кадр фильтром mpv и растягивает остаток на всё окно.
    ///
    /// `None` возвращает всё как было. Перекодирования не происходит:
    /// mpv показывает часть кадра и масштабирует её.
    ///
    /// Одной обрезки мало. Убрав поля, мы получаем кадр другой формы —
    /// у широкого фильма он оказывается уже окна, и поля возвращаются
    /// с других сторон. Поэтому вместе с обрезкой включается `panscan`:
    /// картинка увеличивается до полного заполнения окна.
    pub fn set_video_crop(&mut self, filter: Option<&str>) -> Result<()> {
        let value = filter.unwrap_or("");
        self.set_property_string("vf", value)?;

        let panscan = if filter.is_some() { "1.0" } else { "0.0" };
        self.set_property_string("panscan", panscan)?;

        tracing::info!(фильтр = value, panscan, "обрезка кадра изменена");
        Ok(())
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
