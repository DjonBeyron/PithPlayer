//! Поиск ближайшего ключевого кадра.
//!
//! Перепаковка режет только по ключевым кадрам. Если начать с середины
//! между ними, первый кадр окажется неполным — отсюда чёрный экран
//! в начале отрезка, ради которого в v4 включали перекодирование.
//! Мы вместо этого встаём точно на ключевой кадр (PLAN.md §6.4).

use std::path::Path;

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
    align_to_keyframes(video, &[time]).into_iter().next()?
}

/// То же для пакета меток — **одним** вызовом `ffprobe`.
///
/// В v4 индекс опорных кадров запрашивался на каждую закладку заново:
/// запуск процесса и чтение файла умножались на число меток (PLAN.md §6.4).
/// Здесь окна вокруг всех меток передаются одним списком интервалов.
///
/// Порядок ответа совпадает с порядком запроса; `None` в элементе означает
/// «выровнять не удалось, режем с запрошенного места».
pub fn align_to_keyframes(video: &Path, times: &[f64]) -> Vec<Option<f64>> {
    if times.is_empty() {
        return Vec::new();
    }

    let intervals = read_intervals(times);

    let output = crate::quiet::command("ffprobe")
        .args(["-v", "error"])
        // Только опорные кадры: остальные пропускаются при разборе.
        .args(["-skip_frame", "nokey"])
        .args(["-select_streams", "v:0"])
        // Именно `pts_time`: поле `pkt_pts_time` убрано в FFmpeg 7,
        // и запрос молча возвращал пустые значения — выравнивание
        // не работало, а отрезки выходили длиннее заданного.
        .args(["-show_entries", "frame=pts_time"])
        .args(["-read_intervals", &intervals])
        .args(["-of", "csv=p=0"])
        .arg(video)
        .output();

    let Ok(output) = output else {
        return vec![None; times.len()];
    };

    if !output.status.success() {
        return vec![None; times.len()];
    }

    let text = String::from_utf8_lossy(&output.stdout);
    times
        .iter()
        .map(|time| pick_nearest(&text, *time))
        .collect()
}

/// Список окон для `-read_intervals`: по окну на каждую метку.
///
/// Перекрытие окон безвредно — в вывод просто попадут повторы, а выбор
/// ближайшего кадра от этого не меняется.
fn read_intervals(times: &[f64]) -> String {
    times
        .iter()
        .map(|time| {
            let from = (time - LOOKBACK_SEC).max(0.0);
            format!("{from}%{time}")
        })
        .collect::<Vec<_>>()
        .join(",")
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

    #[test]
    fn окна_запрашиваются_одним_списком() {
        // Один вызов ffprobe на весь пакет вместо вызова на каждую метку.
        let intervals = read_intervals(&[20.0, 100.0]);
        assert_eq!(intervals, "5%20,85%100");
    }

    #[test]
    fn окно_не_уходит_за_начало_файла() {
        assert_eq!(read_intervals(&[3.0]), "0%3");
    }

    #[test]
    fn пустой_запрос_не_зовёт_ffprobe() {
        // Пустой список интервалов ffprobe принял бы за ошибку.
        assert!(align_to_keyframes(std::path::Path::new("нет.mp4"), &[]).is_empty());
    }

    #[test]
    fn один_вывод_обслуживает_все_метки() {
        let вывод = "10.000000\n20.000000\n90.000000\n100.000000\n";
        let результат: Vec<_> = [25.0, 95.0]
            .iter()
            .map(|time| pick_nearest(вывод, *time))
            .collect();

        assert_eq!(результат, vec![Some(20.0), Some(90.0)]);
    }
}
