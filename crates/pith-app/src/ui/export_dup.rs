//! Предупреждение окна выгрузки: такие отрезки в базе уже есть.

use crate::theme;
use crate::tr;

/// Что выбрал человек.
pub(super) enum Choice {
    Skip,
    Keep,
    Back,
}

/// Сколько повторов и несколько примеров; три кнопки на выбор.
pub(super) fn show_duplicates(
    ui: &mut egui::Ui,
    count: usize,
    samples: &[String],
) -> Option<Choice> {
    ui.label(
        egui::RichText::new(tr!(
            format!("В базе уже есть такие отрезки: {count}"),
            format!("The database already has these fragments: {count}")
        ))
        .color(theme::ERROR)
        .size(15.0)
        .strong(),
    );
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(tr!(
            "Тот же фильм и та же реплика. Например:",
            "Same film and same line. For example:"
        ))
        .color(theme::TEXT_SECONDARY)
        .size(12.0),
    );
    ui.add_space(4.0);

    for text in samples {
        ui.label(
            egui::RichText::new(format!("• {text}"))
                .color(theme::TEXT_PRIMARY)
                .size(12.0),
        );
    }

    ui.add_space(12.0);

    let mut choice = None;

    ui.horizontal_wrapped(|ui| {
        if ui
            .button(tr!("Выгрузить без повторов", "Export without duplicates"))
            .clicked()
        {
            choice = Some(Choice::Skip);
        }
        if ui
            .button(tr!("Выгрузить всё равно", "Export anyway"))
            .clicked()
        {
            choice = Some(Choice::Keep);
        }
        if ui.button(tr!("Назад", "Back")).clicked() {
            choice = Some(Choice::Back);
        }
    });

    choice
}
