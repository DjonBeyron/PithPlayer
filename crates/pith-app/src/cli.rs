//! Разбор аргументов командной строки.
//!
//! Аргументов немного, поэтому обходимся без внешней библиотеки
//! (CLAUDE.md: новая зависимость требует обоснования).

use pith_mpv::HwDec;

/// Что передали при запуске.
#[derive(Debug, Default)]
pub struct Args {
    /// Файл для открытия. Приходит при запуске по двойному клику.
    pub file: Option<String>,
    /// Режим аппаратного декодирования. `None` — взять значение по умолчанию.
    pub hwdec: Option<HwDec>,
    /// Скрыть панель замеров.
    pub hide_metrics: bool,
    /// Не подгонять окно под форму видео.
    pub no_fit_window: bool,
}

/// Текст справки.
pub const HELP: &str = "\
Pith Player v5

Использование:
  pith-player [файл] [параметры]

Параметры:
  --hwdec=<режим>   Режим декодирования: zero-copy | copy | software
  --no-metrics      Скрыть панель замеров
  --no-fit-window   Не подгонять окно под форму видео
  --help            Показать эту справку

Управление:
  Пробел            Пауза / продолжить
  ← →               Перемотка на 5 с (Shift — 1 с, Ctrl — 1 мин)
  ↑ ↓               Громкость
  Esc, F            Полноэкранный режим
  [ ]               Скорость воспроизведения
  Backspace         Обычная скорость

Переменные окружения:
  PITH_LOG          Уровень логов: error | warn | info | debug | trace
";

impl Args {
    /// Разбирает аргументы. Неизвестные молча игнорируются: плеер должен
    /// запускаться, даже если система передала что-то лишнее.
    pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Self {
        let mut parsed = Self::default();

        for arg in args {
            if let Some(value) = arg.strip_prefix("--hwdec=") {
                parsed.hwdec = parse_hwdec(value);
                if parsed.hwdec.is_none() {
                    tracing::warn!(value, "неизвестный режим декодирования, беру обычный");
                }
            } else if arg == "--no-metrics" {
                parsed.hide_metrics = true;
            } else if arg == "--no-fit-window" {
                parsed.no_fit_window = true;
            } else if !arg.starts_with("--") && parsed.file.is_none() {
                parsed.file = Some(arg);
            }
        }

        parsed
    }
}

fn parse_hwdec(value: &str) -> Option<HwDec> {
    match value {
        "zero-copy" | "d3d11va" => Some(HwDec::ZeroCopy),
        "copy" | "auto-copy" => Some(HwDec::Copy),
        "software" | "no" => Some(HwDec::Software),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Args {
        Args::parse(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn разбирает_путь_к_файлу() {
        let args = parse(&["C:\\видео.mkv"]);
        assert_eq!(args.file.as_deref(), Some("C:\\видео.mkv"));
    }

    #[test]
    fn разбирает_режим_декодирования() {
        assert_eq!(parse(&["--hwdec=zero-copy"]).hwdec, Some(HwDec::ZeroCopy));
        assert_eq!(parse(&["--hwdec=copy"]).hwdec, Some(HwDec::Copy));
        assert_eq!(parse(&["--hwdec=software"]).hwdec, Some(HwDec::Software));
    }

    #[test]
    fn неизвестный_режим_не_ломает_запуск() {
        let args = parse(&["--hwdec=чепуха"]);
        assert_eq!(args.hwdec, None);
    }

    #[test]
    fn неизвестные_параметры_игнорируются() {
        let args = parse(&["--что-то-новое", "видео.mp4"]);
        assert_eq!(args.file.as_deref(), Some("видео.mp4"));
    }

    #[test]
    fn файл_и_параметры_вместе() {
        let args = parse(&["видео.mkv", "--hwdec=software", "--no-metrics"]);
        assert_eq!(args.file.as_deref(), Some("видео.mkv"));
        assert_eq!(args.hwdec, Some(HwDec::Software));
        assert!(args.hide_metrics);
    }
}
