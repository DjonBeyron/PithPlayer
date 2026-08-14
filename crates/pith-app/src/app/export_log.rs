//! Журнал окна выгрузки: строки, метки и цвет.
//!
//! Отдельно от состояния окна: у журнала своя забота — показать, что
//! произошло и **откуда взято значение**. Главный вопрос к нему один:
//! сходил плеер в сеть или взял из памяти, — и ответ должен читаться,
//! не вчитываясь.

/// Откуда взялось значение — по этому журнал и раскрашивается.
///
/// Цвет здесь не украшение: главный вопрос к журналу — «сходил ли плеер
/// в сеть или взял из памяти», и ответ должен читаться, не вчитываясь.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    /// Ход работы: что начали, чем кончили.
    Step,
    /// Взято из памяти — ничего не спрашивали.
    Memory,
    /// Нашлось в первом словаре.
    First,
    /// Нашлось во втором словаре: первый не знал.
    Second,
    /// Не нашлось нигде.
    Missing,
    /// Дело сделано.
    Done,
    /// Не вышло.
    Failed,
}

impl LogKind {
    /// Метка в начале строки — по ней журнал читается столбцом.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Step => "···",
            Self::Memory => crate::tr!("память", "memory"),
            Self::First => "wooordhunt",
            Self::Second => "cambridge",
            Self::Missing => crate::tr!("нет", "none"),
            Self::Done => crate::tr!("готово", "done"),
            Self::Failed => crate::tr!("отказ", "failed"),
        }
    }
}

/// Строка журнала.
#[derive(Debug, Clone)]
pub struct LogLine {
    pub kind: LogKind,
    pub text: String,
}

impl LogLine {
    pub fn new(kind: LogKind, text: String) -> Self {
        Self { kind, text }
    }

    /// Строка для буфера обмена: метка и текст.
    pub fn plain(&self) -> String {
        format!("{:<11} {}", self.kind.tag(), self.text)
    }
}
