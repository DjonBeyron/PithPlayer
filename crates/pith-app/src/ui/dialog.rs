//! Детали окон настроек: карточки, переключатели, кнопки.
//!
//! Собраны в одном месте, чтобы окна настроек выглядели одинаково и не
//! обрастали каждое своими отступами и оттенками.

use crate::theme;

/// Скругление карточек и полей.
pub const CARD_RADIUS: u8 = 8;
const FIELD_RADIUS: u8 = 6;

/// Отступ внутри карточки.
const CARD_PADDING: i8 = 16;

/// Размеры переключателя.
const SWITCH_SIZE: [f32; 2] = [40.0, 22.0];
/// Насколько кружок меньше самого переключателя.
const KNOB_INSET: f32 = 3.0;

/// Высота кнопок в подвале окна.
const BUTTON_HEIGHT: f32 = 32.0;
/// Наименьшая ширина этих кнопок: у «Отмены» и «Сохранить» она общая.
const BUTTON_WIDTH: f32 = 104.0;

/// Заголовок раздела над карточкой.
pub fn section(ui: &mut egui::Ui, title: &str) {
    ui.label(
        egui::RichText::new(title)
            .color(theme::DIALOG_TEXT)
            .size(15.0),
    );
    ui.add_space(8.0);
}

/// Карточка с группой настроек — во всю ширину окна.
pub fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    // Ширину запоминаем до рамки: содержимое карточки не должно её сужать,
    // иначе соседние карточки выходят разной ширины.
    let width = ui.available_width() - f32::from(CARD_PADDING) * 2.0;

    egui::Frame::NONE
        .fill(theme::DIALOG_CARD)
        .stroke(egui::Stroke::new(1.0, theme::DIALOG_BORDER))
        .corner_radius(CARD_RADIUS)
        .inner_margin(CARD_PADDING)
        .show(ui, |ui| {
            ui.set_width(width);
            add(ui)
        })
        .inner
}

/// Тонкая черта во всю ширину.
pub fn divider(ui: &mut egui::Ui) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), egui::Sense::hover());

    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        egui::Stroke::new(1.0, theme::DIALOG_BORDER),
    );
}

/// Подпись поля.
pub fn label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).color(theme::DIALOG_LABEL));
}

/// Пояснение мелким шрифтом.
pub fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .color(theme::DIALOG_MUTED)
            .size(12.0),
    );
}

/// Поле с числом.
pub fn number(ui: &mut egui::Ui, value: &mut u32, range: std::ops::RangeInclusive<u32>) {
    let widgets = &mut ui.visuals_mut().widgets;

    for visuals in [
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
    ] {
        visuals.weak_bg_fill = theme::DIALOG_FIELD;
        visuals.bg_fill = theme::DIALOG_FIELD;
        visuals.bg_stroke = egui::Stroke::new(1.0, theme::DIALOG_BORDER);
        visuals.corner_radius = egui::CornerRadius::same(FIELD_RADIUS);
    }

    ui.add(
        egui::DragValue::new(value)
            .range(range)
            .speed(0.2)
            .update_while_editing(false),
    )
    .on_hover_text(crate::tr!(
        "Тяните или введите число",
        "Drag or type a number"
    ));
}

/// Строка с путём: моноширинная, в собственной рамке.
pub fn path_box(ui: &mut egui::Ui, text: &str) {
    let width = ui.available_width();

    egui::Frame::NONE
        .fill(theme::DIALOG_BG)
        .stroke(egui::Stroke::new(1.0, theme::DIALOG_BORDER))
        .corner_radius(FIELD_RADIUS)
        .inner_margin(8)
        .show(ui, |ui| {
            ui.set_width(width - 16.0);
            ui.add(
                egui::Label::new(
                    egui::RichText::new(text)
                        .color(theme::DIALOG_LABEL)
                        .monospace()
                        .size(12.0),
                )
                .wrap(),
            );
        });
}

/// Переключатель-тумблер.
///
/// Своя отрисовка: галочка egui рядом с карточками макета выглядит
/// заплаткой, а тумблер читается с одного взгляда — видно и состояние,
/// и то, что по нему нажимают.
pub fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(
        egui::vec2(SWITCH_SIZE[0], SWITCH_SIZE[1]),
        egui::Sense::click(),
    );

    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    // Плавный переход: мгновенный скачок кружка читается как мигание.
    let progress = ui.ctx().animate_bool_responsive(response.id, *on);

    let fill = if *on {
        theme::DIALOG_ACCENT_FILL
    } else {
        theme::DIALOG_FIELD
    };

    let radius = rect.height() / 2.0;

    ui.painter().rect(
        rect,
        radius,
        fill,
        egui::Stroke::new(1.0, theme::DIALOG_OUTLINE),
        egui::StrokeKind::Inside,
    );

    let knob_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), progress);
    let knob_color = if *on {
        theme::DIALOG_ON_ACCENT
    } else {
        theme::DIALOG_KNOB
    };

    ui.painter().circle_filled(
        egui::pos2(knob_x, rect.center().y),
        radius - KNOB_INSET,
        knob_color,
    );

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Строка настройки: тумблер, название и пояснение под ним.
pub fn toggle_row(ui: &mut egui::Ui, on: &mut bool, title: &str, description: &str) {
    // Ширину текста считаем заранее: внутри вложенной колонки её уже
    // не от чего взять, и длинное пояснение уходило бы в одну строку.
    let text_width = ui.available_width() - SWITCH_SIZE[0] - TOGGLE_GAP;

    ui.horizontal_top(|ui| {
        toggle(ui, on);
        ui.add_space(TOGGLE_GAP - ui.spacing().item_spacing.x);

        ui.vertical(|ui| {
            ui.set_max_width(text_width);
            ui.label(egui::RichText::new(title).color(theme::DIALOG_TEXT));
            ui.add_space(4.0);
            hint(ui, description);
        });
    });
}

/// Промежуток между тумблером и текстом.
const TOGGLE_GAP: f32 = 14.0;

/// Главная кнопка окна — заливкой.
pub fn accent_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let button = egui::Button::new(egui::RichText::new(text).color(theme::DIALOG_ON_ACCENT))
        .fill(theme::DIALOG_ACCENT_FILL)
        .corner_radius(FIELD_RADIUS)
        .min_size(egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT));

    ui.add(button)
}

/// Вторая кнопка окна — только обводкой.
pub fn outline_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let button = egui::Button::new(egui::RichText::new(text).color(theme::DIALOG_ACCENT))
        .fill(egui::Color32::TRANSPARENT)
        .stroke(egui::Stroke::new(1.0, theme::DIALOG_OUTLINE))
        .corner_radius(FIELD_RADIUS)
        .min_size(egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT));

    ui.add(button)
}

/// Обычная кнопка внутри карточки.
pub fn card_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let button = egui::Button::new(egui::RichText::new(text).color(theme::DIALOG_TEXT))
        .fill(theme::DIALOG_FIELD)
        .stroke(egui::Stroke::new(1.0, theme::DIALOG_BORDER))
        .corner_radius(FIELD_RADIUS)
        .min_size(egui::vec2(0.0, BUTTON_HEIGHT - 4.0));

    ui.add(button)
}
