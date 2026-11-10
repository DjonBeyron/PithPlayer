//! Ручной состав: люди и фотографии вписываются без базы фильмов.
//!
//! Всё лежит в `cast.json` рядом с составами из базы (человек с номером
//! меньше нуля — вписанный руками), а фотографии — файлами PNG в папке
//! `manual-photos`: переживают и перезапуск плеера, и перезагрузку компьютера.

use std::path::{Path, PathBuf};

use pith_store::LOCAL_PHOTO;

use super::PithApp;

/// Самая длинная сторона сохраняемой фотографии: лицу хватает, а файл невелик.
const PHOTO_MAX_SIDE: u32 = 512;

/// Поля формы «добавить человека».
#[derive(Default)]
pub struct ManualForm {
    pub name: String,
    pub role: String,
    /// Фотография для нового человека: путь с приставкой `file:`.
    pub photo: Option<String>,
    /// Название картины, как его набирают в поле.
    pub title: String,
    /// Для какого видео поле названия уже заполнено.
    title_for: Option<String>,
}

impl PithApp {
    /// Ручной режим окна актёров включён.
    pub fn manual_mode(&self) -> bool {
        self.cast_store.manual_mode()
    }

    pub fn set_manual_mode(&mut self, on: bool) {
        self.cast_store.set_manual_mode(on);
    }

    pub fn manual_form(&mut self) -> &mut ManualForm {
        &mut self.actors.manual
    }

    /// Подтягивает название картины в поле, когда сменился файл.
    pub fn sync_manual_title(&mut self) {
        let Some(video) = self.current_video_name() else {
            return;
        };

        if self.actors.manual.title_for.as_deref() == Some(video.as_str()) {
            return;
        }

        self.actors.manual.title = self
            .cast_store
            .for_video(&video)
            .map(|c| c.title.clone())
            .unwrap_or_default();
        self.actors.manual.title_for = Some(video);
    }

    /// Записывает набранное название картины.
    pub fn save_manual_title(&mut self) {
        let Some(video) = self.current_video_name() else {
            return;
        };

        let title = self.actors.manual.title.clone();
        self.cast_store.set_title(&video, &title);
        self.reload_cast();
    }

    /// Добавляет человека из формы.
    pub fn manual_add_member(&mut self) {
        let Some(video) = self.current_video_name() else {
            self.show_notice(crate::tr!("Сначала откройте файл", "Open a file first"));
            return;
        };

        let form = &mut self.actors.manual;
        if form.name.trim().is_empty() {
            return;
        }

        let name = std::mem::take(&mut form.name);
        let role = Some(std::mem::take(&mut form.role));
        let photo = form.photo.take();

        self.cast_store
            .add_manual_member(&video, &name, role, photo);
        self.reload_cast();
    }

    /// Убирает человека.
    pub fn manual_remove_member(&mut self, id: i64) {
        if let Some(video) = self.current_video_name()
            && self.cast_store.remove_member(&video, id)
        {
            self.reload_cast();
        }
    }

    /// Ставит человеку фотографию (`None` — убрать).
    pub fn manual_set_photo(&mut self, id: i64, photo: Option<String>) {
        if let Some(video) = self.current_video_name()
            && self.cast_store.set_member_photo(&video, id, photo)
        {
            self.reload_cast();
        }
    }

    /// Сохраняет картинку из буфера обмена. `None` — там нет картинки.
    pub fn photo_from_clipboard(&mut self) -> Option<String> {
        let saved = clipboard_image().and_then(|image| self.save_photo(image));

        if saved.is_none() {
            self.show_notice(crate::tr!(
                "В буфере обмена нет картинки",
                "There is no picture in the clipboard"
            ));
        }

        saved
    }

    /// Выбор файла с фотографией и его сохранение. `None` — отмена или не картинка.
    pub fn photo_from_file(&mut self) -> Option<String> {
        let path = rfd::FileDialog::new()
            .add_filter(
                crate::tr!("Картинки", "Pictures"),
                &["png", "jpg", "jpeg", "webp", "bmp"],
            )
            .pick_file()?;

        let saved = image::open(&path)
            .map_err(|e| tracing::warn!(error = %e, "фотография не открылась"))
            .ok()
            .and_then(|image| self.save_photo(image));

        if saved.is_none() {
            self.show_notice(crate::tr!(
                "Этот файл не удалось открыть как картинку",
                "This file could not be opened as a picture"
            ));
        }

        saved
    }

    /// Уменьшает и кладёт картинку в папку ручных фотографий.
    fn save_photo(&self, image: image::DynamicImage) -> Option<String> {
        let dir = self.manual_photos_dir();
        std::fs::create_dir_all(&dir).ok()?;

        let image = if image.width().max(image.height()) > PHOTO_MAX_SIDE {
            image.thumbnail(PHOTO_MAX_SIDE, PHOTO_MAX_SIDE)
        } else {
            image
        };

        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        let path = dir.join(format!("{stamp}.png"));

        image
            .save(&path)
            .map_err(|e| tracing::warn!(error = %e, "фотографию не сохранить"))
            .ok()?;

        Some(local_path(&path))
    }

    fn manual_photos_dir(&self) -> PathBuf {
        self.data_paths.root().join("manual-photos")
    }

    /// Обновляет состав на экране из хранилища.
    fn reload_cast(&mut self) {
        self.actors.cast = self
            .current_video_name()
            .and_then(|name| self.cast_store.for_video(&name).cloned());
    }
}

/// Путь фотографии в составе: с приставкой, чтобы отличить от базы фильмов.
fn local_path(path: &Path) -> String {
    format!("{LOCAL_PHOTO}{}", path.display())
}

/// Картинка из буфера обмена.
fn clipboard_image() -> Option<image::DynamicImage> {
    let data = arboard::Clipboard::new().ok()?.get_image().ok()?;
    let buffer = image::RgbaImage::from_raw(
        u32::try_from(data.width).ok()?,
        u32::try_from(data.height).ok()?,
        data.bytes.into_owned(),
    )?;

    Some(image::DynamicImage::ImageRgba8(buffer))
}
