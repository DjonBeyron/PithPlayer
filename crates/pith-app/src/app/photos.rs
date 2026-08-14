//! Фотографии актёров: загрузка, кэш на диске и текстуры для окна.
//!
//! Фотографий полсотни, а видно за раз десяток. Поэтому картинка
//! запрашивается только тогда, когда строка попала на экран, качается
//! в отдельном потоке и кладётся в кэш на диск — после перезапуска
//! список рисуется без единого обращения к сети.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

use super::PithApp;

/// Что известно о фотографии.
enum Photo {
    /// Качается прямо сейчас.
    Loading(Receiver<Option<Vec<u8>>>),
    /// Готова к показу.
    Ready(egui::TextureHandle),
    /// Не вышло — больше не пробуем, пока не сменится файл.
    Failed,
}

/// Какого размера нужна фотография.
///
/// Мелкая — для строки списка, крупная — для увеличения по нажатию.
/// Растянуть мелкую на пол-окна нельзя: у неё сторона 185 точек, и вблизи
/// это каша. Оба размера кэшируются каждый сам по себе.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoSize {
    Row,
    Large,
}

impl PhotoSize {
    /// Имя размера в адресах базы — оно же различает записи кэша.
    fn folder(self) -> &'static str {
        match self {
            Self::Row => "w185",
            Self::Large => "w500",
        }
    }
}

/// Кэш фотографий по размеру и пути внутри базы.
#[derive(Default)]
pub struct PhotoCache {
    photos: HashMap<String, Photo>,
}

impl PithApp {
    /// Текстура фотографии. `None` — ещё не готова или её нет вовсе.
    ///
    /// Первое обращение заводит загрузку: сначала смотрим на диск, и только
    /// если там пусто — идём в сеть.
    pub fn actor_photo(
        &mut self,
        ctx: &egui::Context,
        path: &str,
        size: PhotoSize,
    ) -> Option<egui::TextureHandle> {
        let key = cache_key(path, size);

        match self.photos.photos.get(&key) {
            Some(Photo::Ready(texture)) => return Some(texture.clone()),
            Some(Photo::Failed) => return None,
            Some(Photo::Loading(_)) => return self.take_loaded(ctx, &key),
            None => {}
        }

        self.start_loading(ctx, &key, path, size);
        None
    }

    /// Забирает скачанное и превращает в текстуру.
    fn take_loaded(&mut self, ctx: &egui::Context, key: &str) -> Option<egui::TextureHandle> {
        let Some(Photo::Loading(receiver)) = self.photos.photos.get(key) else {
            return None;
        };

        let bytes = receiver.try_recv().ok()?;

        let photo = match bytes.as_deref().and_then(decode) {
            Some(image) => {
                let texture = ctx.load_texture(key, image, egui::TextureOptions::LINEAR);
                Photo::Ready(texture)
            }
            None => Photo::Failed,
        };

        let ready = match &photo {
            Photo::Ready(texture) => Some(texture.clone()),
            _ => None,
        };

        self.photos.photos.insert(key.to_string(), photo);
        ready
    }

    /// Заводит загрузку фотографии в отдельном потоке.
    ///
    /// Скачанное будит интерфейс: без этого картинка появлялась бы только
    /// при следующей отрисовке, то есть после движения мышью.
    fn start_loading(&mut self, ctx: &egui::Context, key: &str, path: &str, size: PhotoSize) {
        let Some(url) = photo_url(path, size) else {
            self.photos.photos.insert(key.to_string(), Photo::Failed);
            return;
        };

        let file = self.photo_file(path, size);
        let (sender, receiver) = channel();

        self.photos
            .photos
            .insert(key.to_string(), Photo::Loading(receiver));

        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let _ = sender.send(load_or_fetch(&file, &url));
            ctx.request_repaint();
        });
    }

    /// Где лежит фотография в кэше.
    ///
    /// Имя пути внутри базы — `/abc123.jpg`; косые черты в имени файла
    /// недопустимы, поэтому берём только последнюю часть. Размер идёт
    /// в имя: иначе крупная затёрла бы мелкую.
    fn photo_file(&self, path: &str, size: PhotoSize) -> PathBuf {
        let name = path.rsplit('/').next().unwrap_or(path);

        self.data_paths
            .photos()
            .join(format!("{}_{name}", size.folder()))
    }
}

/// Ключ записи кэша: у каждого размера своя.
fn cache_key(path: &str, size: PhotoSize) -> String {
    format!("{}{path}", size.folder())
}

/// Адрес фотографии нужного размера.
fn photo_url(path: &str, size: PhotoSize) -> Option<String> {
    (!path.is_empty()).then(|| format!("https://image.tmdb.org/t/p/{}{path}", size.folder()))
}

/// Берёт картинку из кэша, а если её там нет — из сети.
///
/// Выполняется в отдельном потоке: и чтение с диска, и сеть блокирующие.
fn load_or_fetch(file: &PathBuf, url: &str) -> Option<Vec<u8>> {
    if let Ok(bytes) = std::fs::read(file) {
        return Some(bytes);
    }

    let bytes = pith_tmdb::fetch_photo(url)
        .inspect_err(|e| tracing::debug!(url, error = %e, "фотография не скачалась"))
        .ok()?;

    // Неудачная запись в кэш — не беда: картинка уже есть, просто в
    // следующий раз скачается заново.
    if let Some(dir) = file.parent()
        && std::fs::create_dir_all(dir).is_ok()
    {
        let _ = std::fs::write(file, &bytes);
    }

    Some(bytes)
}

/// Превращает файл картинки в изображение для egui.
fn decode(bytes: &[u8]) -> Option<egui::ColorImage> {
    let image = image::load_from_memory(bytes)
        .inspect_err(|e| tracing::debug!(error = %e, "фотографию не разобрать"))
        .ok()?;

    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];

    Some(egui::ColorImage::from_rgba_unmultiplied(
        size,
        rgba.as_raw(),
    ))
}

#[cfg(test)]
mod tests {
    use super::{PhotoSize, cache_key, photo_url};

    #[test]
    fn адрес_собирается_из_пути() {
        assert_eq!(
            photo_url("/abc.jpg", PhotoSize::Row).as_deref(),
            Some("https://image.tmdb.org/t/p/w185/abc.jpg")
        );
    }

    #[test]
    fn крупная_фотография_берётся_другим_размером() {
        assert_eq!(
            photo_url("/abc.jpg", PhotoSize::Large).as_deref(),
            Some("https://image.tmdb.org/t/p/w500/abc.jpg")
        );
    }

    #[test]
    fn пустой_путь_адреса_не_даёт() {
        assert!(photo_url("", PhotoSize::Row).is_none());
    }

    #[test]
    fn у_размеров_разные_записи_кэша() {
        assert_ne!(
            cache_key("/abc.jpg", PhotoSize::Row),
            cache_key("/abc.jpg", PhotoSize::Large),
            "иначе крупная затрёт мелкую"
        );
    }
}
