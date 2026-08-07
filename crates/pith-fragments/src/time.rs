//! Время в том виде, в каком его понимает FFmpeg.

/// Время в формате `ЧЧ:ММ:СС.мс`, который понимает FFmpeg.
pub fn format_time(seconds: f64) -> String {
    let seconds = seconds.max(0.0);
    let total_ms = (seconds * 1000.0).round() as u64;

    let hours = total_ms / 3_600_000;
    let minutes = (total_ms % 3_600_000) / 60_000;
    let secs = (total_ms % 60_000) / 1000;
    let millis = total_ms % 1000;

    format!("{hours:02}:{minutes:02}:{secs:02}.{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::format_time;

    #[test]
    fn время_форматируется_с_миллисекундами() {
        assert_eq!(format_time(0.0), "00:00:00.000");
        assert_eq!(format_time(65.5), "00:01:05.500");
        assert_eq!(format_time(3661.25), "01:01:01.250");
    }
    #[test]
    fn отрицательное_время_обрезается_нулём() {
        assert_eq!(format_time(-10.0), "00:00:00.000");
    }
}
