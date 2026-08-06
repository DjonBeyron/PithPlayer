//! Растягивание картинки на весь экран без чёрных полей.
//!
//! Широкий фильм в контейнере 16:9 несёт полосы сверху и снизу, и на
//! большом экране им достаётся заметная часть площади. Границы полезной
//! части ищет FFmpeg, а обрезает mpv — перекодировать ничего не нужно.

use std::sync::mpsc::{Receiver, channel};

use pith_fragments::Crop;

use super::PithApp;

/// Состояние обрезки.
#[derive(Default)]
pub struct CropState {
    /// Применённая обрезка, если она есть.
    applied: Option<Crop>,
    /// Идёт поиск границ.
    detecting: bool,
    /// Ответ фонового поиска.
    result: Option<Receiver<Option<Crop>>>,
}

impl PithApp {
    /// Растянута ли картинка сейчас.
    pub fn is_cropped(&self) -> bool {
        self.crop.applied.is_some()
    }

    /// Идёт ли поиск полей.
    pub fn is_detecting_crop(&self) -> bool {
        self.crop.detecting
    }

    /// Растягивает картинку или возвращает её как было.
    pub fn toggle_crop(&mut self) {
        if self.crop.applied.is_some() {
            self.reset_crop();
            return;
        }

        self.start_crop_detection();
    }

    /// Снимает обрезку.
    pub fn reset_crop(&mut self) {
        self.crop.applied = None;

        if let Some(engine) = self.engine.as_mut()
            && let Err(e) = engine.set_video_crop(None)
        {
            tracing::warn!(error = %e, "не удалось снять обрезку");
        }

        self.show_notice(crate::tr!("Поля возвращены", "Black bars restored"));
    }

    /// Запускает поиск полей в фоне.
    ///
    /// FFmpeg просматривает несколько секунд с текущего места — это
    /// заметное время, и в потоке интерфейса ему не место.
    fn start_crop_detection(&mut self) {
        if self.crop.detecting {
            return;
        }

        let Some(path) = self.current_path.clone() else {
            return;
        };

        if !self.can_extract() {
            self.show_notice(crate::tr!(
                "Нужен ffmpeg.exe рядом с плеером",
                "ffmpeg.exe must sit next to the player"
            ));
            return;
        }

        let position = self
            .engine
            .as_ref()
            .map(|e| e.state().position)
            .unwrap_or_default();

        // Размеры исходника нужны поиску: по ним видно, вернулся кадр
        // целиком или в нём действительно нашлись поля.
        let Some(source) = self.engine.as_ref().and_then(pith_mpv::Engine::source_size) else {
            self.show_notice(crate::tr!(
                "Размер кадра ещё неизвестен",
                "Frame size is not known yet"
            ));
            return;
        };

        let (sender, receiver) = channel();
        self.crop.result = Some(receiver);
        self.crop.detecting = true;

        self.show_notice(crate::tr!("Ищу чёрные поля…", "Looking for black bars…"));

        std::thread::spawn(move || {
            let found = pith_fragments::detect_crop(&path, position, source);
            let _ = sender.send(found);
        });
    }

    /// Забирает итог поиска и применяет его.
    pub(super) fn poll_crop(&mut self) {
        let Some(receiver) = self.crop.result.as_ref() else {
            return;
        };

        let Ok(found) = receiver.try_recv() else {
            return;
        };

        self.crop.result = None;
        self.crop.detecting = false;

        let Some(crop) = found else {
            self.show_notice(crate::tr!("Чёрных полей не нашлось", "No black bars found"));
            return;
        };

        self.apply_crop(crop);
    }

    fn apply_crop(&mut self, crop: Crop) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };

        match engine.set_video_crop(Some(&crop.to_filter())) {
            Ok(()) => {
                self.crop.applied = Some(crop);
                self.show_notice(crate::tr!(
                    "Картинка растянута на весь экран",
                    "The picture now fills the screen"
                ));
            }
            Err(e) => {
                tracing::warn!(error = %e, "не удалось применить обрезку");
                self.show_notice(crate::tr!(
                    "Не удалось растянуть картинку",
                    "Could not fill the screen"
                ));
            }
        }
    }

    /// Сбрасывает обрезку при смене файла: у нового видео свои поля.
    pub(super) fn forget_crop(&mut self) {
        self.crop.applied = None;
        self.crop.detecting = false;
        self.crop.result = None;
    }
}
