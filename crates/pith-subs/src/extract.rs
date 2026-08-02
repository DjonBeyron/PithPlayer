//! Извлечение дорожки субтитров через FFmpeg.
//!
//! Один вызов на дорожку: FFmpeg приводит любой формат к SRT и пишет его
//! в поток вывода. В v4 то же делалось батчами по 15 реплик с таймаутами
//! и приоритетами — при наличии `sub-text` от mpv всё это нужно только
//! для поиска по всему файлу (PLAN.md §6.2).

use std::path::Path;
use std::process::Command;

use crate::parse::{Cue, parse_srt};

/// Имя исполняемого файла FFmpeg.
const FFMPEG: &str = "ffmpeg";

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error(
        "не удалось запустить FFmpeg: {0}. \
         Поиск по субтитрам без него недоступен"
    )]
    Spawn(String),

    #[error("FFmpeg не смог извлечь дорожку {track}")]
    Failed { track: i64 },
}

/// Извлекает дорожку субтитров и разбирает её.
///
/// `track_index` — порядковый номер среди дорожек субтитров (0, 1, 2…),
/// а не идентификатор mpv.
pub fn extract_track(video: &Path, track_index: i64) -> Result<Vec<Cue>, ExtractError> {
    let output = Command::new(FFMPEG)
        .args(["-v", "error"])
        .arg("-i")
        .arg(video)
        // Только нужная дорожка субтитров, без видео и звука.
        .args(["-map", &format!("0:s:{track_index}")])
        .args(["-f", "srt", "-"])
        .output()
        .map_err(|e| ExtractError::Spawn(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(track = track_index, %stderr, "FFmpeg не извлёк дорожку");
        return Err(ExtractError::Failed { track: track_index });
    }

    let content = String::from_utf8_lossy(&output.stdout);
    let cues = parse_srt(&content);

    tracing::info!(
        track = track_index,
        реплик = cues.len(),
        "дорожка субтитров извлечена"
    );

    Ok(cues)
}

/// Читает внешний файл субтитров.
///
/// Файл может быть в любой кодировке; неверные байты заменяются, чтобы
/// одна испорченная реплика не лишила поиска весь файл.
pub fn read_external(path: &Path) -> Result<Vec<Cue>, ExtractError> {
    // Через FFmpeg: он приводит ASS и VTT к SRT и разбирается с кодировками.
    let output = Command::new(FFMPEG)
        .args(["-v", "error"])
        .arg("-i")
        .arg(path)
        .args(["-f", "srt", "-"])
        .output()
        .map_err(|e| ExtractError::Spawn(e.to_string()))?;

    if !output.status.success() {
        return Err(ExtractError::Failed { track: 0 });
    }

    Ok(parse_srt(&String::from_utf8_lossy(&output.stdout)))
}

/// Доступен ли FFmpeg.
///
/// Проверяется до показа поиска: без FFmpeg он не работает, и лучше
/// сказать об этом прямо, чем показывать пустой список.
pub fn is_ffmpeg_available() -> bool {
    Command::new(FFMPEG)
        .arg("-version")
        .output()
        .is_ok_and(|out| out.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn несуществующий_файл_даёт_ошибку() {
        let result = extract_track(Path::new("нет-такого-файла.mkv"), 0);
        assert!(result.is_err());
    }

    /// FFmpeg есть в системе разработчика; на другой машине тест
    /// просто не найдёт его и это тоже допустимый исход.
    #[test]
    fn проверка_доступности_не_падает() {
        let _ = is_ffmpeg_available();
    }
}
