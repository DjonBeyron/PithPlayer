//! Скорость воспроизведения: кнопка панели и список готовых значений.

use crate::app::PithApp;
use crate::theme;
use crate::tr;
use crate::ui::controls::BUTTON_SIZE;
use crate::ui::icons;

/// Ширина кнопки: с запасом под надпись вида «0.75×».
const BUTTON_WIDTH: f32 = 44.0;

/// Ширина списка скоростей.
const MENU_WIDTH: f32 = 160.0;

/// Готовые значения скорости.
const PRESETS: [f64; 7] = [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0];

/// Кнопка со списком готовых значений.
///
/// Замедление и ускорение не меняют тональность — mpv растягивает звук
/// по времени, голоса не басят и не пищат.
pub fn show(app: &mut PithApp, ui: &mut egui::Ui) {
    let speed = app.engine().map(|e| e.state().speed).unwrap_or(1.0);
    let usual = (speed - 1.0).abs() < f64::EPSILON;

    // Обычная скорость — только значок; изменённая пишется числом,
    // чтобы её было видно, не наводя мышь.
    let label = if usual {
        icons::SPEED.text().color(theme::TEXT_PRIMARY)
    } else {
        egui::RichText::new(format!("{speed:.2}×"))
            .color(theme::PANEL_ACCENT)
            .size(12.0)
    };

    let button = egui::Button::new(label)
        .frame(false)
        .min_size(egui::vec2(BUTTON_WIDTH, BUTTON_SIZE[1]));

    let (response, _) =
        egui::containers::menu::MenuButton::from_button(button).ui(ui, |ui| show_menu(app, ui));

    response.on_hover_text(tr!(
        "Скорость воспроизведения. Тональность не меняется",
        "Playback speed. Pitch stays the same"
    ));
}

/// Список готовых скоростей.
fn show_menu(app: &mut PithApp, ui: &mut egui::Ui) {
    let current = app.engine().map(|e| e.state().speed).unwrap_or(1.0);
    let mut chosen = None;

    ui.set_min_width(MENU_WIDTH);

    for speed in PRESETS {
        let active = (current - speed).abs() < 0.01;
        let label = if (speed - 1.0).abs() < f64::EPSILON {
            tr!("1.00× — обычная", "1.00× — normal").to_string()
        } else {
            format!("{speed:.2}×")
        };

        if ui.radio(active, label).clicked() {
            chosen = Some(speed);
            ui.close();
        }
    }

    if let Some(speed) = chosen {
        app.set_speed(speed);
    }
}
