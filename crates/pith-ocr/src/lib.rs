//! Распознавание текста субтитров с кадра — локально, без сети.
//!
//! Движок — `ocrs` (открытый, MIT/Apache-2.0), две модели в файлах:
//! поиск текста и чтение строк. Модели лежат рядом с программой
//! (`models/`) либо в папке данных; если их нет, их можно скачать
//! (`download_models`). Сам OCR читает латиницу.

mod models;

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;

pub use models::{download_models, models_ready};

#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    #[error("модели распознавания не найдены: {0}")]
    NoModels(String),
    #[error("не удалось загрузить модель: {0}")]
    Model(String),
    #[error("картинку не открыть: {0}")]
    Image(String),
    #[error("распознавание не удалось: {0}")]
    Engine(String),
    #[error("не удалось скачать модели: {0}")]
    Download(String),
}

/// Движок, собранный один раз: загрузка моделей стоит около секунды.
static ENGINE: OnceLock<Mutex<Option<OcrEngine>>> = OnceLock::new();

/// Строки короче этого — шум детектора, а не текст.
const MIN_LINE_CHARS: usize = 2;

/// Картинки ниже этой высоты увеличиваются: мелкий шрифт читается хуже.
const MIN_HEIGHT: u32 = 80;

/// Читает текст с картинки. `models` — папка с файлами моделей.
///
/// Блокирующий вызов: от десятых долей секунды до нескольких секунд.
/// Вызывать только вне потока интерфейса.
pub fn recognize(models: &Path, image: &Path) -> Result<String, OcrError> {
    let slot = ENGINE.get_or_init(|| Mutex::new(None));
    let mut guard = slot
        .lock()
        .map_err(|_| OcrError::Engine("движок занят аварией другого потока".into()))?;

    if guard.is_none() {
        *guard = Some(load_engine(models)?);
    }
    let engine = guard
        .as_ref()
        .ok_or_else(|| OcrError::Engine("движок не собрался".into()))?;

    let mut rgb = image::open(image)
        .map_err(|e| OcrError::Image(e.to_string()))?
        .into_rgb8();

    if rgb.height() < MIN_HEIGHT {
        let scale = MIN_HEIGHT.div_ceil(rgb.height().max(1));
        rgb = image::imageops::resize(
            &rgb,
            rgb.width() * scale,
            rgb.height() * scale,
            image::imageops::FilterType::CatmullRom,
        );
    }

    let source = ImageSource::from_bytes(rgb.as_raw(), rgb.dimensions())
        .map_err(|e| OcrError::Engine(e.to_string()))?;
    let input = engine
        .prepare_input(source)
        .map_err(|e| OcrError::Engine(e.to_string()))?;

    let words = engine
        .detect_words(&input)
        .map_err(|e| OcrError::Engine(e.to_string()))?;
    let lines = engine.find_text_lines(&input, &words);
    let texts = engine
        .recognize_text(&input, &lines)
        .map_err(|e| OcrError::Engine(e.to_string()))?;

    let text = texts
        .iter()
        .flatten()
        .map(ToString::to_string)
        .filter(|line| line.trim().chars().count() >= MIN_LINE_CHARS)
        .collect::<Vec<_>>()
        .join(" ");

    Ok(clean(&text))
}

fn load_engine(models: &Path) -> Result<OcrEngine, OcrError> {
    if !models_ready(models) {
        return Err(OcrError::NoModels(models.display().to_string()));
    }

    let detection = Model::load_file(models.join(models::DETECTION_FILE))
        .map_err(|e| OcrError::Model(e.to_string()))?;
    let recognition = Model::load_file(models.join(models::RECOGNITION_FILE))
        .map_err(|e| OcrError::Model(e.to_string()))?;

    tracing::info!("модели распознавания загружены");

    OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection),
        recognition_model: Some(recognition),
        ..Default::default()
    })
    .map_err(|e| OcrError::Model(e.to_string()))
}

/// Лишние пробелы и перевод строк — одним пробелом.
fn clean(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::clean;

    #[test]
    fn пробелы_и_переводы_строк_схлопываются() {
        assert_eq!(clean("  Hello \n  world  "), "Hello world");
    }
}
