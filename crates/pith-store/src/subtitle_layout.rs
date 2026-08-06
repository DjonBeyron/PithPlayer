//! Раскладка и вид слоя субтитров.
//!
//! Отдельно от прочих настроек: у слоя своё место на экране, размер,
//! цвет и начертание — и свои пределы, которыми всё это обрезается.

use serde::{Deserialize, Serialize};

/// Куда прижат слой субтитров по горизонтали и вертикали.
///
/// Хранится долей от размера окна, а не пикселями: иначе после смены
/// разрешения субтитры уезжали бы за край.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SubtitleLayout {
    /// 0 — левый край, 1 — правый.
    pub x: f32,
    /// 0 — верх, 1 — низ.
    pub y: f32,
    /// Размер текста в точках.
    pub font_size: f32,
    /// Цвет текста, RGB.
    ///
    /// В настройках, записанных прежними версиями, поля нет — тогда берётся
    /// белый, каким субтитры и были.
    #[serde(default = "default_subtitle_color")]
    pub color: [u8; 3],
    /// Более жирное начертание.
    #[serde(default)]
    pub bold: bool,
}

/// Цвет субтитров по умолчанию — белый.
fn default_subtitle_color() -> [u8; 3] {
    SubtitleLayout::DEFAULT_COLOR
}

impl SubtitleLayout {
    /// Цвет, с которого начинают оба слоя.
    pub const DEFAULT_COLOR: [u8; 3] = [255, 255, 255];

    /// Раскладка основных субтитров: внизу по центру.
    pub fn main() -> Self {
        Self {
            x: 0.5,
            y: 0.88,
            font_size: 30.0,
            color: Self::DEFAULT_COLOR,
            bold: false,
        }
    }

    /// Раскладка вторых субтитров: над основными.
    pub fn secondary() -> Self {
        Self {
            x: 0.5,
            y: 0.76,
            font_size: 26.0,
            color: Self::DEFAULT_COLOR,
            bold: false,
        }
    }

    /// Возвращает цвет и начертание к исходным.
    ///
    /// Место на экране и размер шрифта не трогаются: их подбирают под
    /// свой экран один раз и сбрасывать вместе с цветом не просят.
    pub fn reset_style(&mut self) {
        self.color = Self::DEFAULT_COLOR;
        self.bold = false;
    }

    /// Ограничивает раскладку разумными пределами.
    pub fn clamped(self) -> Self {
        Self {
            x: self.x.clamp(0.0, 1.0),
            y: self.y.clamp(0.0, 1.0),
            font_size: self.font_size.clamp(10.0, 96.0),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SubtitleLayout;
    use crate::paths::DataPaths;
    use crate::settings::Settings;

    #[test]
    fn раскладка_обрезается_по_пределам() {
        let layout = SubtitleLayout {
            x: 5.0,
            y: -3.0,
            font_size: 500.0,
            ..SubtitleLayout::main()
        }
        .clamped();

        assert_eq!(layout.x, 1.0);
        assert_eq!(layout.y, 0.0);
        assert_eq!(layout.font_size, 96.0);
    }

    #[test]
    fn цвет_и_начертание_переживают_обрезку() {
        let layout = SubtitleLayout {
            color: [10, 20, 30],
            bold: true,
            ..SubtitleLayout::main()
        }
        .clamped();

        assert_eq!(layout.color, [10, 20, 30]);
        assert!(layout.bold);
    }

    #[test]
    fn сброс_возвращает_белый_цвет_не_трогая_место() {
        let mut layout = SubtitleLayout {
            color: [255, 0, 0],
            bold: true,
            ..SubtitleLayout::secondary()
        };
        layout.reset_style();

        assert_eq!(layout.color, SubtitleLayout::DEFAULT_COLOR);
        assert!(!layout.bold);
        assert_eq!(layout.y, SubtitleLayout::secondary().y, "место осталось");
    }

    #[test]
    fn настройки_прежних_версий_читаются_без_цвета() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        std::fs::create_dir_all(dir.path()).expect("каталог");
        std::fs::write(
            paths.settings(),
            r#"{"main_subtitle": {"x": 0.5, "y": 0.9, "font_size": 34.0}}"#,
        )
        .expect("настройки без цвета");

        let settings = Settings::load(&paths);
        assert_eq!(settings.main_subtitle.font_size, 34.0, "своё сохранилось");
        assert_eq!(settings.main_subtitle.color, SubtitleLayout::DEFAULT_COLOR);
        assert!(!settings.main_subtitle.bold);
    }

    #[test]
    fn вторые_субтитры_выше_основных() {
        assert!(
            SubtitleLayout::secondary().y < SubtitleLayout::main().y,
            "второй слой обязан быть над основным"
        );
    }
}
