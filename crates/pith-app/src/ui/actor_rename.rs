//! Поле правки имени актёра прямо в строке состава.
//!
//! Отдельно от списка (`ui/actors.rs`): у правки свои заботы — фокус,
//! Enter и Escape, — и держать их вперемешку с отрисовкой строк незачем.

use crate::app::PithApp;
use crate::theme;
use crate::tr;

/// Отступ поля от края строки.
const FIELD_MARGIN: f32 = 6.0;

/// Высота поля ввода.
const FIELD_HEIGHT: f32 = 28.0;

/// Поле правки имени вместо подписи.
///
/// Enter записывает, Escape бросает, уход фокуса — тоже записывает: щелчок
/// мимо здесь означает «закончил», а не «отменил».
pub(super) fn show_field(
    app: &mut PithApp,
    ui: &mut egui::Ui,
    row: egui::Rect,
    photo: egui::Rect,
    id: i64,
) {
    let rect = egui::Rect::from_min_max(
        egui::pos2(
            photo.right() + FIELD_MARGIN,
            row.center().y - FIELD_HEIGHT / 2.0,
        ),
        egui::pos2(
            row.right() - FIELD_MARGIN,
            row.center().y + FIELD_HEIGHT / 2.0,
        ),
    );

    let Some(rename) = app.actor_rename() else {
        return;
    };

    let field_id = egui::Id::new(("actor_rename", id));
    let mut builder = egui::UiBuilder::new().max_rect(rect);
    builder.layer_id = None;

    let response = ui.new_child(builder).add(
        egui::TextEdit::singleline(&mut rename.name)
            .id(field_id)
            .desired_width(rect.width())
            .text_color(theme::TEXT_PRIMARY)
            .hint_text(tr!("Имя актёра", "Actor name")),
    );

    // Фокус сразу: правку затевают, чтобы печатать, а не чтобы потом
    // ещё раз щёлкнуть по полю.
    if !response.has_focus() && !response.lost_focus() {
        ui.ctx().memory_mut(|m| m.request_focus(field_id));
    }

    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));

    if escape {
        app.cancel_actor_rename();
    } else if enter || response.lost_focus() {
        app.finish_actor_rename();
    }
}
