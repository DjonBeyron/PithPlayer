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

/// Чем заменить значок, если шрифта набора нет.
#[derive(Debug, Clone, Copy)]
enum Fallback {
    /// Знак, понятный без языка.
    Sign(&'static str),
    /// Слово: по-русски и по-английски.
    Word(&'static str, &'static str),
}

impl Fallback {
    fn text(self) -> &'static str {
        match self {
            Self::Sign(sign) => sign,
            Self::Word(ru, en) => crate::tr!(ru, en),
        }
    }
}

/// Значок кнопки: код в наборе и запасной символ.
#[derive(Debug, Clone, Copy)]
pub struct Icon {
    glyph: &'static str,
    fallback: Fallback,
}

impl Icon {
    /// Готовый текст для кнопки.
    pub fn text(self) -> egui::RichText {
        if fonts::icons_available() {
            egui::RichText::new(self.glyph)
                .family(fonts::icon_family())
                .size(SIZE)
        } else {
            egui::RichText::new(self.fallback.text())
        }
    }

    /// Знак и шрифт заданного размера — для рисования значка кистью.
    ///
    /// Кнопкам хватает `text`, но значок в центре кадра меняет размер
    /// на ходу, и его приходится рисовать самим.
    pub fn painted(self, size: f32) -> (&'static str, egui::FontId) {
        if fonts::icons_available() {
            (self.glyph, egui::FontId::new(size, fonts::icon_family()))
        } else {
            (self.fallback.text(), egui::FontId::proportional(size))
        }
    }
}

/// Размер значка в кнопках панели.
const SIZE: f32 = 15.0;

/// Открыть файл.
pub const OPEN: Icon = Icon {
    glyph: "\u{E8E5}",
    fallback: Fallback::Word("Открыть", "Open"),
};

/// Воспроизведение.
pub const PLAY: Icon = Icon {
    glyph: "\u{E768}",
    fallback: Fallback::Sign("▶"),
};

/// Пауза.
pub const PAUSE: Icon = Icon {
    glyph: "\u{E769}",
    fallback: Fallback::Sign("❚❚"),
};

/// Развернуть на весь экран.
pub const FULLSCREEN: Icon = Icon {
    glyph: "\u{E740}",
    fallback: Fallback::Sign("[ ]"),
};

/// Вернуться из полного экрана в окно.
pub const RESTORE: Icon = Icon {
    glyph: "\u{E73F}",
    fallback: Fallback::Sign("] ["),
};

/// Звук включён.
pub const VOLUME: Icon = Icon {
    glyph: "\u{E767}",
    fallback: Fallback::Sign("🔊"),
};

/// Звук выключен.
pub const MUTE: Icon = Icon {
    glyph: "\u{E74F}",
    fallback: Fallback::Sign("🔇"),
};

/// Поставить закладку.
pub const ADD: Icon = Icon {
    glyph: "\u{E710}",
    fallback: Fallback::Sign("+"),
};

/// Поиск по субтитрам — лупа.
pub const SEARCH: Icon = Icon {
    glyph: "\u{E721}",
    fallback: Fallback::Word("Поиск", "Search"),
};

/// Удалить.
pub const DELETE: Icon = Icon {
    glyph: "\u{E74D}",
    fallback: Fallback::Sign("🗑"),
};

/// Переименовать.
pub const EDIT: Icon = Icon {
    glyph: "\u{E70F}",
    fallback: Fallback::Sign("✏"),
};

/// Повтор файла по кругу.
pub const LOOP: Icon = Icon {
    glyph: "\u{E8EE}",
    fallback: Fallback::Sign("↻"),
};

/// Скорость воспроизведения.
pub const SPEED: Icon = Icon {
    glyph: "\u{EC4A}",
    fallback: Fallback::Sign("×"),
};

/// Вырезать отрезок — ножницы.
pub const CUT: Icon = Icon {
    glyph: "\u{E8C6}",
    fallback: Fallback::Word("Вырезать", "Cut"),
};

/// Скопировать название закладки — два листа.
pub const COPY: Icon = Icon {
    glyph: "\u{E8C8}",
    fallback: Fallback::Word("Копировать", "Copy"),
};

/// Закрепить панель — булавка.
pub const PIN: Icon = Icon {
    glyph: "\u{E718}",
    fallback: Fallback::Sign("📌"),
};

/// Раскрыть список — уголок вниз.
pub const CHEVRON: Icon = Icon {
    glyph: "\u{E70D}",
    fallback: Fallback::Sign("v"),
};

/// Папка вывода.
pub const FOLDER: Icon = Icon {
    glyph: "\u{E8B7}",
    fallback: Fallback::Word("Папка", "Folder"),
};

/// Лист бумаги — для подсказки о пустом списке.
pub const NOTE: Icon = Icon {
    glyph: "\u{E7C3}",
    fallback: Fallback::Sign("•"),
};

/// Настройки.
pub const SETTINGS: Icon = Icon {
    glyph: "\u{E713}",
    fallback: Fallback::Sign("⚙"),
};

/// Растянуть картинку, убрав чёрные поля.
pub const FIT_SCREEN: Icon = Icon {
    glyph: "\u{E799}",
    fallback: Fallback::Sign("[↔]"),
};

/// Вернуть поля на место.
pub const FIT_ORIGINAL: Icon = Icon {
    glyph: "\u{E7A0}",
    fallback: Fallback::Sign("[=]"),
};
