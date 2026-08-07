//! Поиск чёрных полей по краям кадра.
//!
//! Широкие фильмы в контейнере 16:9 несут чёрные полосы сверху и снизу,
//! а вертикальное видео — слева и справа. На весь экран смотреть такое
//! неудобно: полезной картинке достаётся половина площади.
//!
//! Границы ищет `ffmpeg` фильтром `cropdetect`, а обрезает уже mpv —
//! перекодировать ради этого ничего не нужно.

use std::path::Path;

/// Границы полезной части кадра.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crop {
    pub width: i64,
    pub height: i64,
    pub x: i64,
    pub y: i64,
}

impl Crop {
    /// Строка фильтра для mpv: `crop=w:h:x:y`.
    pub fn to_filter(self) -> String {
        format!("crop={}:{}:{}:{}", self.width, self.height, self.x, self.y)
    }

    /// Есть ли что обрезать.
    ///
    /// Полосой считаем только заметную: пара пикселей — это разница
    /// округления размеров, а не поля.
    pub fn is_meaningful(self, source_width: i64, source_height: i64) -> bool {
        let trimmed_x = source_width - self.width;
        let trimmed_y = source_height - self.height;

        self.width > 0 && self.height > 0 && (trimmed_x >= MIN_BAR || trimmed_y >= MIN_BAR)
    }
}

/// Насколько широкой должна быть полоса, чтобы её стоило убирать.
const MIN_BAR: i64 = 8;

/// Сколько секунд кадра просматривать.
///
/// Титры и тёмные сцены сбивают поиск, поэтому берём отрезок от текущей
/// позиции — там, где пользователь и смотрит.
const SAMPLE_SECONDS: u32 = 6;

/// Пороги яркости, ниже которой точка считается чёрной.
///
/// У обычного видео чёрный близок к нулю, и хватает 24. У HDR и Dolby
/// Vision чёрный заметно светлее — с порогом 24 поля просто не находятся.
/// Поэтому пробуем по возрастанию и берём первый порог, который увидел
/// настоящие поля.
const LIMITS: [u32; 2] = [24, 64];

/// Ищет границы картинки без чёрных полей.
///
/// `position` — откуда смотреть, секунды; `source` — размеры кадра,
/// по ним видно, нашлись поля или кадр вернулся целиком.
///
/// `None` означает, что полей нет или `ffmpeg` недоступен.
pub fn detect(video: &Path, position: f64, source: (i64, i64)) -> Option<Crop> {
    let (width, height) = source;

    for limit in LIMITS {
        let found = detect_with_limit(video, position, limit)?;

        if found.is_meaningful(width, height) {
            tracing::info!(limit, ?found, "чёрные поля найдены");
            return Some(found);
        }
    }

    None
}

/// Один прогон `cropdetect` с заданным порогом.
fn detect_with_limit(video: &Path, position: f64, limit: u32) -> Option<Crop> {
    let start = position.max(0.0);

    let output = crate::quiet::command("ffmpeg")
        .args(["-hide_banner", "-nostats"])
        .args(["-ss", &crate::time::format_time(start)])
        .arg("-i")
        .arg(video)
        .args(["-t", &SAMPLE_SECONDS.to_string()])
        // `cropdetect` пишет найденные границы в журнал на уровне info.
        .args(["-vf", &format!("cropdetect={limit}:2:0")])
        .args(["-f", "null", "-"])
        .output()
        .ok()?;

    // Вывод идёт в поток ошибок — таков `cropdetect`.
    let text = String::from_utf8_lossy(&output.stderr);
    parse_last_crop(&text)
}

/// Берёт последнюю подсказку `crop=w:h:x:y` из вывода FFmpeg.
///
/// Последняя точнее первых: фильтр уточняет границы по мере просмотра.
fn parse_last_crop(output: &str) -> Option<Crop> {
    output
        .lines()
        .filter_map(|line| line.rsplit_once("crop="))
        .filter_map(|(_, value)| parse_crop(value.trim()))
        .next_back()
}

/// Разбирает `1920:800:0:140`.
fn parse_crop(value: &str) -> Option<Crop> {
    let mut parts = value
        .split_whitespace()
        .next()?
        .split(':')
        .map(|p| p.parse::<i64>().ok());

    Some(Crop {
        width: parts.next()??,
        height: parts.next()??,
        x: parts.next()??,
        y: parts.next()??,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn разбирается_подсказка_ffmpeg() {
        let вывод = "[Parsed_cropdetect_0 @ 0000] x1:0 x2:1919 y1:140 y2:939 \
                     w:1920 h:800 x:0 y:140 pts:4 t:0.04 crop=1920:800:0:140";

        assert_eq!(
            parse_last_crop(вывод),
            Some(Crop {
                width: 1920,
                height: 800,
                x: 0,
                y: 140
            })
        );
    }

    #[test]
    fn берётся_последняя_подсказка() {
        // Фильтр уточняет границы по ходу просмотра.
        let вывод = "crop=1920:1080:0:0\ncrop=1920:816:0:132\n";

        assert_eq!(parse_last_crop(вывод).map(|c| c.height), Some(816));
    }

    #[test]
    fn мусор_не_разбирается() {
        assert_eq!(parse_last_crop(""), None);
        assert_eq!(parse_last_crop("crop=не число"), None);
        assert_eq!(parse_last_crop("ничего похожего"), None);
    }

    #[test]
    fn фильтр_для_mpv_собирается() {
        let crop = Crop {
            width: 1920,
            height: 800,
            x: 0,
            y: 140,
        };

        assert_eq!(crop.to_filter(), "crop=1920:800:0:140");
    }

    #[test]
    fn узкая_полоса_не_считается_полем() {
        // Пара пикселей — округление размеров, а не чёрное поле.
        let почти_весь = Crop {
            width: 1920,
            height: 1076,
            x: 0,
            y: 2,
        };

        assert!(!почти_весь.is_meaningful(1920, 1080));
    }

    #[test]
    fn настоящие_поля_распознаются() {
        let широкий = Crop {
            width: 1920,
            height: 800,
            x: 0,
            y: 140,
        };

        assert!(широкий.is_meaningful(1920, 1080));
    }

    #[test]
    fn боковые_поля_тоже_считаются() {
        // Такие поля даёт вертикальное видео и кадр 2.35:1 в контейнере 4К.
        let узкий = Crop {
            width: 3240,
            height: 2160,
            x: 300,
            y: 0,
        };

        assert!(узкий.is_meaningful(3840, 2160));
    }

    #[test]
    fn целый_кадр_полем_не_считается() {
        // Так `cropdetect` отвечает, когда полей нет вовсе.
        let целиком = Crop {
            width: 3840,
            height: 2160,
            x: 0,
            y: 0,
        };

        assert!(!целиком.is_meaningful(3840, 2160));
    }
}
