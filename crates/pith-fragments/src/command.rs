//! Сборка команд FFmpeg для нарезки отрезков.
//!
//! Логика перенесена из `VideoFragmentExtractor` версии 4 дословно —
//! это выстраданные исправления реальных багов (PLAN.md §6.4).

use std::path::Path;

/// Что и как вырезать.
#[derive(Debug, Clone)]
pub struct FragmentJob {
    /// Исходный файл.
    pub source: std::path::PathBuf,
    /// Куда сохранить.
    pub output: std::path::PathBuf,
    /// Начало отрезка, секунды.
    pub start: f64,
    /// Длительность, секунды.
    pub duration: f64,
    /// Порядковый номер аудиодорожки. `None` — все дорожки.
    pub audio_index: Option<i64>,
    /// Перекодировать вместо перепаковки.
    pub reencode: bool,
    /// Перекодировать звук в AAC, оставив видео копией.
    ///
    /// Premiere Pro и After Effects не читают EAC3, DTS и подобное:
    /// файл открывается, но звуковой дорожки в нём для монтажной
    /// программы нет. AAC понимают все. Видео при этом копируется
    /// как обычно, скорость почти не страдает.
    pub audio_aac: bool,
}

/// На сколько секунд отступать назад при перекодировании.
///
/// Значение из v4: некоторые видео имеют ключевые кадры раз в 6–10 секунд,
/// и меньшего отступа не хватало.
const SEEK_BACK_SEC: f64 = 10.0;

impl FragmentJob {
    /// Аргументы FFmpeg для этой задачи.
    pub fn to_args(&self) -> Vec<String> {
        let mut args = vec!["-v".into(), "error".into(), "-y".into()];

        if self.reencode {
            self.push_reencode_seek(&mut args);
        } else {
            // При перепаковке `-ss` до `-i` — быстрое позиционирование.
            args.push("-ss".into());
            args.push(format_time(self.start));
            args.push("-i".into());
            args.push(path_arg(&self.source));
        }

        args.push("-t".into());
        args.push(format_time(self.duration));

        // Только основной видеопоток: `-map 0:v` захватил бы и обложки,
        // а они ломают вывод. Исправление реального бага v4.
        args.push("-map".into());
        args.push("0:v:0".into());

        args.push("-map".into());
        match self.audio_index {
            Some(index) => args.push(format!("0:a:{index}")),
            None => args.push("0:a".into()),
        }

        if self.reencode {
            self.push_reencode_codecs(&mut args);
        } else if self.audio_aac {
            // Видео копируем, звук приводим к AAC ради монтажных программ.
            for arg in ["-c:v", "copy", "-c:a", "aac", "-ac", "2", "-b:a", "320k"] {
                args.push(arg.into());
            }
        } else {
            args.push("-c".into());
            args.push("copy".into());
        }

        // Исправляет отрицательные метки времени в начале отрезка.
        args.push("-avoid_negative_ts".into());
        args.push("make_zero".into());

        args.push(path_arg(&self.output));
        args
    }

    /// Двойная перемотка для точного старта при перекодировании.
    ///
    /// Первый `-ss` до `-i` быстро встаёт на ключевой кадр раньше нужного
    /// момента, второй после `-i` доводит точно. Эта пара и решала проблему
    /// чёрных кадров в начале фрагмента.
    fn push_reencode_seek(&self, args: &mut Vec<String>) {
        let rough = (self.start - SEEK_BACK_SEC).max(0.0);
        let precise = self.start - rough;

        args.push("-ss".into());
        args.push(format_time(rough));
        args.push("-i".into());
        args.push(path_arg(&self.source));

        if precise > 0.0 {
            args.push("-ss".into());
            args.push(format_time(precise));
        }
    }

    /// Кодеки перекодирования. Значения из v4.
    fn push_reencode_codecs(&self, args: &mut Vec<String>) {
        for arg in [
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            // Качество, неотличимое от исходного.
            "-crf",
            "18",
            "-c:a",
            "aac",
            // Даунмикс в стерео: многоканальный звук ломал монтажные программы.
            "-ac",
            "2",
            "-b:a",
            "320k",
        ] {
            args.push(arg.into());
        }
    }
}

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

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn задача(reencode: bool) -> FragmentJob {
        FragmentJob {
            source: PathBuf::from("C:\\видео\\фильм.mkv"),
            output: PathBuf::from("C:\\выход\\кусок.mp4"),
            start: 65.5,
            duration: 18.0,
            audio_index: Some(1),
            reencode,
            audio_aac: false,
        }
    }

    fn позиция(args: &[String], value: &str) -> Option<usize> {
        args.iter().position(|a| a == value)
    }

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

    #[test]
    fn перепаковка_копирует_потоки() {
        let args = задача(false).to_args();

        assert!(args.contains(&"copy".to_string()));
        assert!(
            !args.contains(&"libx264".to_string()),
            "перепаковка не перекодирует"
        );
    }

    #[test]
    fn перепаковка_ставит_перемотку_до_входного_файла() {
        let args = задача(false).to_args();

        let ss = позиция(&args, "-ss").expect("перемотка есть");
        let input = позиция(&args, "-i").expect("вход есть");

        assert!(
            ss < input,
            "`-ss` обязан идти до `-i`: так перемотка быстрая"
        );
    }

    /// Второй `-ss` при копировании потоков даёт отрицательные метки
    /// времени и чёрные кадры — этого быть не должно.
    #[test]
    fn перепаковка_не_содержит_второй_перемотки() {
        let args = задача(false).to_args();
        let count = args.iter().filter(|a| *a == "-ss").count();

        assert_eq!(count, 1, "при копировании перемотка только одна");
    }

    #[test]
    fn перекодирование_использует_двойную_перемотку() {
        let args = задача(true).to_args();
        let count = args.iter().filter(|a| *a == "-ss").count();

        assert_eq!(count, 2, "грубая перемотка и точная");
    }

    #[test]
    fn грубая_перемотка_отступает_на_десять_секунд() {
        let args = задача(true).to_args();
        let ss = позиция(&args, "-ss").expect("перемотка есть");

        // 65.5 − 10 = 55.5
        assert_eq!(args[ss + 1], "00:00:55.500");
    }

    #[test]
    fn отступ_не_уходит_за_начало_файла() {
        let mut job = задача(true);
        job.start = 3.0;

        let args = job.to_args();
        let ss = позиция(&args, "-ss").expect("перемотка есть");

        assert_eq!(args[ss + 1], "00:00:00.000");
        assert_eq!(
            args.iter().filter(|a| *a == "-ss").count(),
            2,
            "точная перемотка всё равно нужна"
        );
    }

    #[test]
    fn копируется_только_основной_видеопоток() {
        let args = задача(false).to_args();

        assert!(
            args.contains(&"0:v:0".to_string()),
            "обложки не должны попадать в отрезок"
        );
        assert!(!args.contains(&"0:v".to_string()));
    }

    #[test]
    fn выбранная_аудиодорожка_попадает_в_отрезок() {
        let args = задача(false).to_args();
        assert!(args.contains(&"0:a:1".to_string()));
    }

    #[test]
    fn без_указания_дорожки_копируются_все() {
        let mut job = задача(false);
        job.audio_index = None;

        assert!(job.to_args().contains(&"0:a".to_string()));
    }

    #[test]
    fn метки_времени_приводятся_к_нулю() {
        let args = задача(false).to_args();
        assert!(args.contains(&"make_zero".to_string()));
    }

    #[test]
    fn звук_в_aac_оставляет_видео_копией() {
        // Premiere и After Effects не видят дорожку EAC3 или DTS: файл
        // открывается, а звука для монтажной программы в нём нет.
        let mut job = задача(false);
        job.audio_aac = true;
        let args = job.to_args();

        assert!(args.contains(&"-c:v".to_string()));
        assert!(args.contains(&"aac".to_string()), "звук приводится к AAC");
        assert!(
            !args.contains(&"libx264".to_string()),
            "видео перекодировать незачем"
        );

        let c = позиция(&args, "-c");
        assert!(c.is_none(), "общего `-c copy` быть не должно");
    }

    #[test]
    fn перекодирование_даунмиксит_звук_в_стерео() {
        let args = задача(true).to_args();

        assert!(args.contains(&"aac".to_string()));
        assert!(args.contains(&"2".to_string()), "два канала");
    }
}
