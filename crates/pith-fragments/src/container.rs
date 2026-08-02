//! Выбор контейнера для вырезанного отрезка.
//!
//! Перепаковка копирует потоки как есть, поэтому контейнер обязан принять
//! исходные кодеки: MP4 не возьмёт DTS, TrueHD или ProRes (PLAN.md §6.4).

/// Кодеки, которые MP4 принимает без перекодирования.
const MP4_VIDEO: &[&str] = &["h264", "hevc", "av1", "mpeg4", "vp9"];
const MP4_AUDIO: &[&str] = &["aac", "mp3", "ac3", "eac3", "alac", "opus"];

/// Подбирает расширение файла под кодеки.
///
/// MP4 предпочтителен: с ним дружат монтажные программы. Всё, что в него
/// не влезает, уходит в MKV — он принимает почти любые потоки.
pub fn choose_container(video_codec: Option<&str>, audio_codec: Option<&str>) -> &'static str {
    let video_ok = video_codec.is_none_or(|c| MP4_VIDEO.contains(&normalize(c).as_str()));
    let audio_ok = audio_codec.is_none_or(|c| MP4_AUDIO.contains(&normalize(c).as_str()));

    if video_ok && audio_ok { "mp4" } else { "mkv" }
}

fn normalize(codec: &str) -> String {
    codec.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn обычное_видео_уходит_в_mp4() {
        assert_eq!(choose_container(Some("h264"), Some("aac")), "mp4");
        assert_eq!(choose_container(Some("hevc"), Some("eac3")), "mp4");
    }

    #[test]
    fn несовместимый_звук_переводит_в_mkv() {
        assert_eq!(choose_container(Some("hevc"), Some("dts")), "mkv");
        assert_eq!(choose_container(Some("h264"), Some("truehd")), "mkv");
        assert_eq!(choose_container(Some("h264"), Some("pcm_s16le")), "mkv");
    }

    #[test]
    fn несовместимое_видео_переводит_в_mkv() {
        assert_eq!(choose_container(Some("prores"), Some("aac")), "mkv");
    }

    #[test]
    fn регистр_и_пробелы_не_мешают() {
        assert_eq!(choose_container(Some(" H264 "), Some("AAC")), "mp4");
    }

    #[test]
    fn неизвестные_кодеки_считаются_совместимыми() {
        // Данных нет — пробуем MP4, как делала v4.
        assert_eq!(choose_container(None, None), "mp4");
    }
}
