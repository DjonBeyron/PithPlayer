//! Ошибки движка. Паники запрещены (см. CLAUDE.md), всё возвращается через `Result`.

use thiserror::Error;

/// Результат операций движка.
pub type Result<T> = std::result::Result<T, MpvError>;

#[derive(Debug, Error)]
pub enum MpvError {
    /// libmpv не удалось загрузить или создать экземпляр.
    /// Чаще всего — отсутствует `libmpv-2.dll` рядом с исполняемым файлом.
    #[error(
        "не удалось запустить движок mpv: {0}. \
         Проверьте, что рядом с программой лежит libmpv-2.dll"
    )]
    Init(String),

    /// Ошибка команды воспроизведения (открытие файла, перемотка и т. п.).
    #[error("команда mpv «{command}» завершилась ошибкой: {source_msg}")]
    Command { command: String, source_msg: String },

    /// Не удалось прочитать свойство mpv.
    /// Часто это штатная ситуация: свойство ещё не готово.
    #[error("не удалось прочитать свойство mpv «{property}»: {source_msg}")]
    Property {
        property: String,
        source_msg: String,
    },

    /// Ошибка создания контекста отрисовки.
    #[error("не удалось создать контекст отрисовки mpv: {0}")]
    Render(String),
}

impl MpvError {
    /// Ошибка команды с текстом от libmpv.
    pub(crate) fn command(command: impl Into<String>, source: libmpv2::Error) -> Self {
        Self::Command {
            command: command.into(),
            source_msg: source.to_string(),
        }
    }

    /// Ошибка чтения свойства с текстом от libmpv.
    pub(crate) fn property(property: impl Into<String>, source: libmpv2::Error) -> Self {
        Self::Property {
            property: property.into(),
            source_msg: source.to_string(),
        }
    }
}
