//! Надпись со временем в панели управления.
//!
//! Рисуется по знакам, а не обычной надписью, ради двух вещей сразу.
//! Место под цифры остаётся постоянным весь фильм: полоса перемотки
//! занимает остаток строки, и растущая надпись дёргала бы её на каждом
//! переходе разряда. А двоеточию моноширинный шрифт отводит столько же
//! места, сколько цифре, — время из-за этого выглядит разреженным.

use crate::theme;

/// Какую долю обычного места занимает двоеточие.
const COLON_WIDTH: f32 = 0.5;

/// Рисует время. Ширина зависит только от вида надписи, не от цифр в ней.
pub fn show(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let font = egui::TextStyle::Monospace.resolve(ui.style());

    let glyphs: Vec<(char, f32)> = text
        .chars()
        .map(|glyph| (glyph, advance(ui, &font, glyph)))
        .collect();

    let width = glyphs.iter().map(|(_, advance)| advance).sum();

    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(width, ui.spacing().interact_size.y),
        egui::Sense::hover(),
    );

    let painter = ui.painter();
    let mut x = rect.left();

    for (glyph, advance) in glyphs {
        painter.text(
            egui::pos2(x + advance / 2.0, rect.center().y),
            egui::Align2::CENTER_CENTER,
            glyph,
            font.clone(),
            theme::TEXT_PRIMARY,
        );
        x += advance;
    }

    response
}

/// Сколько места отводится знаку.
fn advance(ui: &egui::Ui, font: &egui::FontId, glyph: char) -> f32 {
    // Цифры меряются по нулю: у разных цифр ширина может отличаться,
    // а надпись должна стоять на одном месте всё время просмотра.
    let measured = if glyph.is_ascii_digit() { '0' } else { glyph };

    let width = ui
        .painter()
        .layout_no_wrap(measured.to_string(), font.clone(), theme::TEXT_PRIMARY)
        .size()
        .x;

    if glyph == ':' {
        width * COLON_WIDTH
    } else {
        width
    }
}
