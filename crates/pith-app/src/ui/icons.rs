//! Значки интерфейса.
//!
//! Берутся из системного набора Windows (Segoe Fluent Icons, а на
//! Windows 10 — Segoe MDL2 Assets). Он уже установлен, скачивать и
//! распространять ничего не нужно, а вид у кнопок такой же, как
//! у остальных программ системы.
//!
//! Коды знаков у обоих наборов общие. Если шрифта нет, показывается
//! запасной символ из обычного шрифта — кнопка остаётся понятной.

use crate::fonts;

/// Значок кнопки: код в наборе и запасной символ.
#[derive(Debug, Clone, Copy)]
pub struct Icon {
    glyph: &'static str,
    fallback: &'static str,
}

impl Icon {
    /// Готовый текст для кнопки.
    pub fn text(self) -> egui::RichText {
        if fonts::icons_available() {
            egui::RichText::new(self.glyph)
                .family(fonts::icon_family())
                .size(SIZE)
        } else {
            egui::RichText::new(self.fallback)
        }
    }
}

/// Размер значка в кнопках панели.
const SIZE: f32 = 15.0;

/// Открыть файл.
pub const OPEN: Icon = Icon {
    glyph: "\u{E8E5}",
    fallback: "Открыть",
};

/// Воспроизведение.
pub const PLAY: Icon = Icon {
    glyph: "\u{E768}",
    fallback: "▶",
};

/// Пауза.
pub const PAUSE: Icon = Icon {
    glyph: "\u{E769}",
    fallback: "❚❚",
};

/// Развернуть на весь экран.
pub const FULLSCREEN: Icon = Icon {
    glyph: "\u{E740}",
    fallback: "[ ]",
};

/// Вернуться из полного экрана в окно.
pub const RESTORE: Icon = Icon {
    glyph: "\u{E73F}",
    fallback: "] [",
};

/// Звук включён.
pub const VOLUME: Icon = Icon {
    glyph: "\u{E767}",
    fallback: "🔊",
};

/// Звук выключен.
pub const MUTE: Icon = Icon {
    glyph: "\u{E74F}",
    fallback: "🔇",
};

/// Удалить.
pub const DELETE: Icon = Icon {
    glyph: "\u{E74D}",
    fallback: "🗑",
};

/// Переименовать.
pub const EDIT: Icon = Icon {
    glyph: "\u{E70F}",
    fallback: "✏",
};

/// Настройки.
pub const SETTINGS: Icon = Icon {
    glyph: "\u{E713}",
    fallback: "⚙",
};

/// Растянуть картинку, убрав чёрные поля.
pub const FIT_SCREEN: Icon = Icon {
    glyph: "\u{E799}",
    fallback: "[↔]",
};

/// Вернуть поля на место.
pub const FIT_ORIGINAL: Icon = Icon {
    glyph: "\u{E7A0}",
    fallback: "[=]",
};
