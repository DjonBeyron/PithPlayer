//! Окно настройки вида субтитров: цвет и начертание каждого слоя.
//!
//! Пока окно открыто, оба слоя показываются на своих местах поверх кадра —
//! цвет виден там же, где его потом читать, а не в квадратике настроек.
//! Пример рисует `ui::subtitles`, здесь только органы управления.

use crate::app::PithApp;
use crate::theme;
use crate::tr;
use crate::ui::subtitles::Layer;

/// Ширина окна.
const WIDTH: f32 = 340.0;

/// Отступ окна от верхнего края.
///
/// Прижато к верху, а не к центру: посередине и внизу стоят сами субтитры,
/// и окно закрыло бы то, ради чего его открыли.
const TOP_MARGIN: f32 = 48.0;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if !app.subtitle_style_open() {
        return;
    }

    let mut open = true;

    egui::Window::new(tr!("Вид субтитров", "Subtitle look"))
        .order(egui::Order::Foreground)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, TOP_MARGIN))
        .show(ctx, |ui| {
            ui.set_width(WIDTH);

            ui.label(
                egui::RichText::new(tr!(
                    "Пример показан на своём месте поверх кадра. Субтитры \
                         двигаются мышью, размер меняется колесом над ними.",
                    "The sample sits where the subtitles really are. Drag them \
                         with the mouse, resize with the wheel over them.",
                ))
                .color(theme::TEXT_SECONDARY)
                .small(),
            );

            ui.add_space(10.0);
            show_layer(app, ui, Layer::Main, tr!("Основные", "Main"));

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            show_layer(app, ui, Layer::Secondary, tr!("Вторые", "Second"));
        });

    if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.close_subtitle_style();
    }
}

/// Строка одного слоя: цвет, начертание и сброс.
fn show_layer(app: &mut PithApp, ui: &mut egui::Ui, layer: Layer, title: &str) {
    let layout = app.subtitle_layout(layer);
    let mut color = layout.color;
    let mut bold = layout.bold;
    let mut reset = false;

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).color(theme::TEXT_PRIMARY));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            reset |= ui
                .add_enabled(
                    color != pith_store::SubtitleLayout::DEFAULT_COLOR || bold,
                    egui::Button::new(tr!("Сбросить", "Reset")),
                )
                .on_hover_text(tr!(
                    "Вернуть белый цвет и обычное начертание",
                    "Back to white and normal weight"
                ))
                .on_disabled_hover_text(tr!(
                    "Цвет и начертание и так исходные",
                    "Colour and weight are already the default"
                ))
                .clicked();

            ui.checkbox(&mut bold, tr!("Жирнее", "Bolder"));

            ui.color_edit_button_srgb(&mut color)
                .on_hover_text(tr!("Цвет текста", "Text colour"));
        });
    });

    if reset {
        app.reset_subtitle_style(layer);
        return;
    }

    app.set_subtitle_style(layer, color, bold);
}
