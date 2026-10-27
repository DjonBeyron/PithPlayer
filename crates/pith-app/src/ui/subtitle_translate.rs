//! Перевод реплики через DeepL: значок над субтитром и панель с переводом.
//!
//! Значок появляется, когда мышь над репликой; по щелчку под ней показывается
//! перевод на выбранные языки. Он исчезает, когда реплика сменится или по
//! повторному щелчку на значок.

use crate::app::PithApp;
use crate::theme;
use crate::tr;

/// Насколько расширена область, где значок остаётся видимым: к нему
/// нужно успеть дойти мышью от реплики.
const HOVER_MARGIN: f32 = 28.0;

/// Подпись значка: латиница и кириллица есть в любом шрифте.
const ICON_TEXT: &str = "A·Я";

/// Значок рядом с репликой и перевод под ней.
///
/// `rect` — где нарисована реплика. `family` — шрифт слоя.
pub(super) fn show(
    app: &mut PithApp,
    ctx: &egui::Context,
    id: &'static str,
    line: &str,
    rect: egui::Rect,
    font: egui::FontId,
) {
    let translated_here = app
        .deepl()
        .shown
        .as_ref()
        .is_some_and(|s| s.text == line.trim());

    let near = ctx
        .pointer_hover_pos()
        .is_some_and(|p| rect.expand(HOVER_MARGIN).contains(p));

    if near || translated_here {
        show_icon(app, ctx, id, line, rect, translated_here);
    }

    if translated_here {
        show_panel(app, ctx, id, rect, font);
    }
}

fn show_icon(
    app: &mut PithApp,
    ctx: &egui::Context,
    id: &'static str,
    line: &str,
    rect: egui::Rect,
    active: bool,
) {
    let area = egui::Area::new(egui::Id::new((id, "translate_icon")))
        .order(egui::Order::Foreground)
        .fixed_pos(rect.right_top())
        .pivot(egui::Align2::RIGHT_BOTTOM);

    let clicked = area
        .show(ctx, |ui| {
            let mut button = egui::Button::new(
                egui::RichText::new(ICON_TEXT)
                    .size(12.0)
                    .color(theme::TEXT_PRIMARY),
            )
            .corner_radius(4.0);

            if active {
                button = button.fill(theme::PANEL_ACCENT);
            }

            ui.add(button)
                .on_hover_text(tr!("Перевести (DeepL)", "Translate (DeepL)"))
                .clicked()
        })
        .inner;

    if !clicked {
        return;
    }

    if active {
        app.hide_translation();
    } else {
        app.translate_line(ctx, line);
    }
}

fn show_panel(
    app: &mut PithApp,
    ctx: &egui::Context,
    id: &'static str,
    rect: egui::Rect,
    font: egui::FontId,
) {
    let Some(shown) = app.deepl().shown.as_ref() else {
        return;
    };

    let (lines, failed): (Vec<String>, bool) = match shown.result.as_ref() {
        None => (vec![tr!("Перевожу…", "Translating…").to_string()], false),
        Some(Ok(parts)) => (
            parts
                .iter()
                .map(|(lang, text)| {
                    if parts.len() > 1 {
                        format!("{lang}: {text}")
                    } else {
                        text.clone()
                    }
                })
                .collect(),
            false,
        ),
        Some(Err(why)) => (vec![why.clone()], true),
    };

    egui::Area::new(egui::Id::new((id, "translation")))
        .order(egui::Order::Middle)
        .fixed_pos(rect.center_bottom() + egui::vec2(0.0, 4.0))
        .pivot(egui::Align2::CENTER_TOP)
        .show(ctx, |ui| {
            ui.set_max_width(rect.width().max(320.0));

            egui::Frame::NONE
                .fill(theme::SUBTITLE_BG)
                .inner_margin(8.0)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    let color = if failed {
                        theme::ERROR
                    } else {
                        theme::TEXT_PRIMARY
                    };

                    for line in &lines {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(line).font(font.clone()).color(color),
                            )
                            .wrap(),
                        );
                    }
                });
        });
}
