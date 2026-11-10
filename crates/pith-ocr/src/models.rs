//! Файлы моделей распознавания: проверка и загрузка.

use std::io::Read;
use std::path::Path;

use crate::OcrError;

pub(crate) const DETECTION_FILE: &str = "text-detection.onnx";
pub(crate) const RECOGNITION_FILE: &str = "text-recognition.onnx";

/// Адреса моделей автора движка. Долговечность адресов не гарантирована —
/// поэтому поставка тоже кладёт модели рядом с программой.
const BASE: &str = "https://ocrs-models.s3-accelerate.amazonaws.com";

/// Обе модели на месте.
pub fn models_ready(dir: &Path) -> bool {
    dir.join(DETECTION_FILE).is_file() && dir.join(RECOGNITION_FILE).is_file()
}

/// Скачивает недостающие модели в папку. `progress` — доля от 0 до 1.
pub fn download_models(dir: &Path, mut progress: impl FnMut(f32)) -> Result<(), OcrError> {
    std::fs::create_dir_all(dir).map_err(|e| OcrError::Download(e.to_string()))?;

    let files = [DETECTION_FILE, RECOGNITION_FILE];

    for (index, name) in files.iter().enumerate() {
        let target = dir.join(name);
        if target.is_file() {
            continue;
        }

        let mut response = ureq::get(&format!("{BASE}/{name}"))
            .call()
            .map_err(|e| OcrError::Download(e.to_string()))?;

        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| OcrError::Download(e.to_string()))?;

        // Через временный файл: оборванная загрузка не должна сойти за модель.
        let part = target.with_extension("part");
        std::fs::write(&part, &bytes).map_err(|e| OcrError::Download(e.to_string()))?;
        std::fs::rename(&part, &target).map_err(|e| OcrError::Download(e.to_string()))?;

        progress((index + 1) as f32 / files.len() as f32);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn без_файлов_модели_не_готовы() {
        let dir = std::env::temp_dir().join("pith_ocr_no_models");
        assert!(!models_ready(&dir));
    }
}
