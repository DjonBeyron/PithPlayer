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

    /// Форма кадра по данным демуксера — до того, как заведён декодер.
    ///
    /// `video-params/dw` появляется только когда mpv настроил вывод, а это
    /// полсекунды спустя: декодер D3D11 и звуковое устройство поднимаются
    /// раньше. Демуксер знает размеры сразу после открытия файла, и окно
    /// можно открыть нужным с первого показа, без скачка.
    ///
    /// Поворот из метаданных учитывается: видео с телефона иначе откроется
    /// лёжа (CLAUDE.md, §6.12).
    pub fn demux_video_size(&self) -> Option<(i64, i64)> {
        let count = self.property_i64("track-list/count")?;

        for index in 0..count {
            let Ok(kind) = self.property_string(&format!("track-list/{index}/type")) else {
                continue;
            };

            if kind != "video" {
                continue;
            }

            let width = self.property_i64(&format!("track-list/{index}/demux-w"))?;
            let height = self.property_i64(&format!("track-list/{index}/demux-h"))?;

            if width <= 0 || height <= 0 {
                continue;
            }

            let rotate = self
                .property_i64(&format!("track-list/{index}/demux-rotation"))
                .unwrap_or(0);

            return Some(if rotate % 180 == 90 {
                (height, width)
            } else {
                (width, height)
            });
        }

        None
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
