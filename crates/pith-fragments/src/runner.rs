//! Запуск FFmpeg и подготовка имён файлов.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::command::FragmentJob;

/// Итог одной задачи.
#[derive(Debug)]
pub enum ExtractionOutcome {
    Done { output: PathBuf, bytes: u64 },
    Failed { output: PathBuf, reason: String },
}

/// Выполняет нарезку одного отрезка.
pub fn run_job(job: &FragmentJob) -> ExtractionOutcome {
    let args = job.to_args();
    tracing::debug!(?args, "запускаю FFmpeg");

    let output = match Command::new("ffmpeg").args(&args).output() {
        Ok(output) => output,
        Err(e) => {
            return ExtractionOutcome::Failed {
                output: job.output.clone(),
                reason: format!("не удалось запустить FFmpeg: {e}"),
            };
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return ExtractionOutcome::Failed {
            output: job.output.clone(),
            reason: if stderr.is_empty() {
                "FFmpeg завершился с ошибкой".into()
            } else {
                stderr
            },
        };
    }

    // Файл нулевого размера означает, что FFmpeg отработал вхолостую:
    // в v4 это было частой поломкой, и молча её пропускать нельзя.
    match std::fs::metadata(&job.output) {
        Ok(meta) if meta.len() > 0 => ExtractionOutcome::Done {
            output: job.output.clone(),
            bytes: meta.len(),
        },
        Ok(_) => ExtractionOutcome::Failed {
            output: job.output.clone(),
            reason: "получился пустой файл".into(),
        },
        Err(e) => ExtractionOutcome::Failed {
            output: job.output.clone(),
            reason: format!("файл не создан: {e}"),
        },
    }
}

/// Путь, который ещё не занят.
///
/// При совпадении добавляется номер: `имя_01.mp4`, `имя_02.mp4` — как в v4.
pub fn unique_output_path(dir: &Path, name: &str, extension: &str) -> PathBuf {
    let safe = sanitize(name);
    let candidate = dir.join(format!("{safe}.{extension}"));

    if !candidate.exists() {
        return candidate;
    }

    for index in 1..1000 {
        let candidate = dir.join(format!("{safe}_{index:02}.{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    dir.join(format!("{safe}_{}.{extension}", std::process::id()))
}

/// Убирает из названия символы, недопустимые в именах файлов.
pub fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => ' ',
            c if (c as u32) < 32 => ' ',
            c => c,
        })
        .collect();

    let trimmed = cleaned.trim().trim_end_matches('.').trim();

    if trimmed.is_empty() {
        "фрагмент".into()
    } else {
        // Слишком длинные имена не принимает файловая система.
        trimmed.chars().take(120).collect()
    }
}

/// Прогревает проверку наличия FFmpeg в фоне.
///
/// Первый вызов `is_ffmpeg_available` запускает внешний процесс. Если он
/// случится при отрисовке панели отрезков, кадр встанет на всё время
/// запуска — вплоть до секунды, когда мешает антивирус. Вызывается при
/// старте приложения из отдельного потока.
pub fn warm_up() {
    std::thread::spawn(|| {
        let started = std::time::Instant::now();
        let found = is_ffmpeg_available();
        tracing::debug!(
            found,
            ms = started.elapsed().as_millis(),
            "проверка FFmpeg прогрета"
        );
    });
}

/// Доступен ли FFmpeg.
///
/// Результат запоминается: проверка запускает внешний процесс, а спрашивают
/// о ней при каждой отрисовке панели. Без кэша интерфейс встаёт колом.
pub fn is_ffmpeg_available() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

    *AVAILABLE.get_or_init(|| {
        let found = Command::new("ffmpeg")
            .arg("-version")
            .output()
            .is_ok_and(|out| out.status.success());

        tracing::info!(found, "проверка наличия FFmpeg");
        found
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn запрещённые_символы_заменяются() {
        assert_eq!(sanitize("Что: делать?"), "Что  делать");
        assert_eq!(sanitize("путь/к\\файлу"), "путь к файлу");
    }

    #[test]
    fn пустое_имя_заменяется_запасным() {
        assert_eq!(sanitize(""), "фрагмент");
        assert_eq!(sanitize("   "), "фрагмент");
        assert_eq!(sanitize("???"), "фрагмент");
    }

    #[test]
    fn точки_в_конце_убираются() {
        // Windows не принимает имена, оканчивающиеся точкой.
        assert_eq!(sanitize("Конец..."), "Конец");
    }

    #[test]
    fn длинное_имя_обрезается() {
        let длинное = "а".repeat(300);
        assert_eq!(sanitize(&длинное).chars().count(), 120);
    }

    #[test]
    fn кириллица_сохраняется() {
        assert_eq!(sanitize("Реплика из фильма"), "Реплика из фильма");
    }

    #[test]
    fn свободное_имя_остаётся_как_есть() {
        let dir = tempfile::tempdir().expect("каталог");
        let path = unique_output_path(dir.path(), "кусок", "mp4");

        assert_eq!(path.file_name().unwrap(), "кусок.mp4");
    }

    #[test]
    fn занятое_имя_получает_номер() {
        let dir = tempfile::tempdir().expect("каталог");
        std::fs::write(dir.path().join("кусок.mp4"), "").expect("занятый файл");

        let path = unique_output_path(dir.path(), "кусок", "mp4");
        assert_eq!(path.file_name().unwrap(), "кусок_01.mp4");
    }

    #[test]
    fn номера_идут_подряд() {
        let dir = tempfile::tempdir().expect("каталог");
        std::fs::write(dir.path().join("кусок.mp4"), "").expect("файл");
        std::fs::write(dir.path().join("кусок_01.mp4"), "").expect("файл");

        let path = unique_output_path(dir.path(), "кусок", "mp4");
        assert_eq!(path.file_name().unwrap(), "кусок_02.mp4");
    }
}
