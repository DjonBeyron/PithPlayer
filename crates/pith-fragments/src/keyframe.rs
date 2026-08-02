//! Поиск ближайшего ключевого кадра.
//!
//! Перепаковка режет только по ключевым кадрам. Если начать с середины
//! между ними, первый кадр окажется неполным — отсюда чёрный экран
//! в начале отрезка, ради которого в v4 включали перекодирование.
//! Мы вместо этого встаём точно на ключевой кадр (PLAN.md §6.4).

use std::path::Path;
use std::process::Command;

/// Насколько далеко назад искать ключевой кадр, секунды.
///
/// Больше десяти не нужно: даже у видео с редкими опорными кадрами
/// интервал не превышает этого значения.
const LOOKBACK_SEC: f64 = 15.0;

/// Ближайший ключевой кадр не позже указанного времени.
///
/// Возвращает `None`, если `ffprobe` недоступен или ничего не нашёл —
/// тогда режем с запрошенного места, как делала v4.
pub fn align_to_keyframe(video: &Path, time: f64) -> Option<f64> {
    let from = (time - LOOKBACK_SEC).max(0.0);

    let output = Command::new("ffprobe")
        .args(["-v", "error"])
        // Только опорные кадры: остальные пропускаются при разборе.
        .args(["-skip_frame", "nokey"])
        .args(["-select_streams", "v:0"])
        // Именно `pts_time`: поле `pkt_pts_time` убрано в FFmpeg 7,
        // и запрос молча возвращал пустые значения — выравнивание
        // не работало, а отрезки выходили длиннее заданного.
        .args(["-show_entries", "frame=pts_time"])
        .args(["-read_intervals", &format!("{from}%{time}")])
        .args(["-of", "csv=p=0"])
        .arg(video)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    pick_nearest(&text, time)
}

/// Выбирает последний ключевой кадр не позже нужного времени.
fn pick_nearest(ffprobe_output: &str, time: f64) -> Option<f64> {
    ffprobe_output
        .lines()
        // В строке может быть несколько полей через запятую; время идёт первым.
        .filter_map(|line| line.split(',').next())
        .filter_map(|value| value.trim().parse::<f64>().ok())
        .filter(|value| *value <= time + 0.001)
        .fold(None, |best: Option<f64>, value| match best {
            Some(current) if current >= value => Some(current),
            _ => Some(value),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn берётся_последний_кадр_до_нужного_момента() {
        let вывод = "10.000000\n20.000000\n30.000000\n";
        assert_eq!(pick_nearest(вывод, 25.0), Some(20.0));
    }

    #[test]
    fn кадр_ровно_в_нужный_момент_подходит() {
        assert_eq!(pick_nearest("10.000000\n20.000000\n", 20.0), Some(20.0));
    }

    #[test]
    fn кадры_после_момента_игнорируются() {
        assert_eq!(pick_nearest("30.000000\n40.000000\n", 25.0), None);
    }

    #[test]
    fn пустой_вывод_даёт_ничего() {
        assert_eq!(pick_nearest("", 25.0), None);
        assert_eq!(pick_nearest("\n\n", 25.0), None);
    }

    /// ffprobe дополняет строку пустыми полями через запятую.
    #[test]
    fn лишние_поля_в_строке_не_мешают() {
        assert_eq!(pick_nearest("10.000000,\n20.000000,\n", 25.0), Some(20.0));
        assert_eq!(
            pick_nearest("10.000000,,,,\n20.000000,,,,\n", 25.0),
            Some(20.0),
            "так выглядит вывод FFmpeg 8"
        );
    }

    #[test]
    fn мусорные_строки_пропускаются() {
        let вывод = "side_data\n10.000000\nN/A\n20.000000\n";
        assert_eq!(pick_nearest(вывод, 25.0), Some(20.0));
    }
}
