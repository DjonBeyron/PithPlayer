//! Кадр из файла через FFmpeg: вырезанный участок картинки для распознавания
//! текста. Идёт отдельным процессом и не мешает воспроизведению.

use std::path::Path;

/// Прямоугольник в долях кадра: от 0 до 1 по каждой стороне.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Region {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Region {
    /// Выражение фильтра `crop`. Размеры округляются до чётных: так
    /// FFmpeg не спорит о форматах с пониженной цветностью.
    fn crop_filter(self) -> String {
        format!(
            "crop=trunc(iw*{w:.4}/2)*2:trunc(ih*{h:.4}/2)*2:trunc(iw*{x:.4}/2)*2:trunc(ih*{y:.4}/2)*2",
            x = self.x,
            y = self.y,
            w = self.w,
            h = self.h
        )
    }
}

/// Сохраняет участок кадра на указанной секунде в PNG.
pub fn capture_region(
    video: &Path,
    seconds: f64,
    region: Region,
    out: &Path,
) -> Result<(), String> {
    let output = crate::quiet::background_command("ffmpeg")
        .args([
            "-v",
            "error",
            "-ss",
            &format!("{:.3}", seconds.max(0.0)),
            "-i",
        ])
        .arg(video)
        .args(["-frames:v", "1", "-vf", &region.crop_filter(), "-y"])
        .arg(out)
        .output()
        .map_err(|e| format!("ffmpeg не запустился: {e}"))?;

    if !output.status.success() || !out.exists() {
        return Err(format!(
            "ffmpeg не взял кадр: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Region;

    #[test]
    fn фильтр_обрезки_задаёт_долями_и_чётными_числами() {
        let filter = Region {
            x: 0.1,
            y: 0.8,
            w: 0.8,
            h: 0.15,
        }
        .crop_filter();

        assert!(filter.starts_with("crop=trunc(iw*0.8000/2)*2:trunc(ih*0.1500/2)*2:"));
        assert!(filter.contains("trunc(ih*0.8000/2)*2"));
    }
}
