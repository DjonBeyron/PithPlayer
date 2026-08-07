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
    /// Откуда переносить данные версии 4.
    pub import_from: Option<String>,
    /// Язык интерфейса, выбранный при установке.
    ///
    /// Установщик передаёт его первому запуску: в мастере язык уже
    /// выбирали, спрашивать второй раз незачем.
    pub language: Option<pith_store::Language>,
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
  --import-from=<папка>
                    Откуда перенести данные версии 4
  --language=<код>  Язык интерфейса: ru | en
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
            } else if let Some(dir) = arg.strip_prefix("--import-from=") {
                parsed.import_from = Some(dir.to_string());
            } else if let Some(code) = arg.strip_prefix("--language=") {
                parsed.language = parse_language(code);
                if parsed.language.is_none() {
                    tracing::warn!(code, "неизвестный язык интерфейса, беру системный");
                }
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

/// Язык по коду из установщика или командной строки.
///
/// Принимаем и полные обозначения вроде `ru-RU`: установщик отдаёт имя
/// своего языкового файла, и оно может выглядеть по-разному.
fn parse_language(code: &str) -> Option<pith_store::Language> {
    let code = code.trim().to_lowercase();

    match code.split(['-', '_']).next() {
        Some("ru") => Some(pith_store::Language::Ru),
        Some("en") => Some(pith_store::Language::En),
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
