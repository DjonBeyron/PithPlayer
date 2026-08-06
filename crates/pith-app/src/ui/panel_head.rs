//! Шапка панели отрезков: название, выбор списка и параметры нарезки.
//!
//! Вид собран по макету: плашки с обводкой, приглушённые подписи и крупные
//! числа акцентом. Прежняя шапка была плотной строкой из подписей и
//! разделителей — в ней приходилось вчитываться, чтобы понять, какой
//! список выбран и какой длины выйдет отрезок.

use crate::app::PithApp;
use crate::theme;
use crate::tr;
use crate::ui::{icons, lists};

/// Высота плашек в строке выбора списка.
const BOX_HEIGHT: f32 = 36.0;

/// Скругление плашек.
const BOX_RADIUS: u8 = 8;

/// Отступ между блоками шапки.
const GAP: f32 = 12.0;

/// Насколько видна надпись с версией: половина от обычной.
const VERSION_OPACITY: f32 = 0.5;

/// Ширина кнопки с шестерёнкой.
const GEAR_WIDTH: f32 = 40.0;

/// Отступ внутри подсказки о пустом списке.
const CARD_PADDING: i8 = 12;

/// Размер квадратных кнопок в строке с папкой.
const DIR_BUTTON: f32 = 26.0;

/// Промежуток между парами «подпись — число».
const NUMBERS_GAP: f32 = 28.0;

/// Отступ в начале названия списка — под кружок его цвета.
const DOT_SPACE: &str = "       ";

/// Радиус кружка и его отступ от левого края плашки.
const DOT_RADIUS: f32 = 5.0;
const DOT_OFFSET: f32 = 18.0;

/// Название панели, выбор списка и параметры нарезки.
pub fn show(app: &mut PithApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr!("Отрезки", "Fragments"))
                .color(theme::TEXT_PRIMARY)
                .size(21.0)
                .strong(),
        );

        show_version(ui);
    });

    if !app.has_open_file() {
        ui.add_space(GAP);
        ui.label(
            egui::RichText::new(tr!("Файл не открыт", "No file open")).color(theme::PANEL_MUTED),
        );
        return;
    }

    ui.add_space(GAP + 4.0);
    show_switcher(app, ui);

    ui.add_space(GAP + 4.0);
    show_details(app, ui);

    ui.add_space(GAP);
    dashed_divider(ui);
    ui.add_space(GAP);
}

/// Версия плеера — в правом углу шапки.
///
/// Наполовину прозрачная: знать её нужно раз в жизни — когда сообщают
/// о неполадке, — и мозолить глаза при каждом взгляде на панель она
/// не должна.
fn show_version(ui: &mut egui::Ui) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label(
            egui::RichText::new(format!("v{}", crate::VERSION))
                .color(theme::PANEL_MUTED.gamma_multiply(VERSION_OPACITY))
                .size(12.0),
        )
        .on_hover_text(tr!("Версия плеера", "Player version"));
    });
}

/// Подсказка вместо списка, когда закладок ещё нет.
pub fn show_empty(ui: &mut egui::Ui) {
    // Ширину запоминаем до рамки: длинная строка внутри неё растягивала
    // всю панель — область подстраивается под содержимое.
    let width = ui.available_width();

    egui::Frame::NONE
        .fill(theme::PANEL_SUNKEN)
        .stroke(egui::Stroke::new(1.0, theme::PANEL_BORDER))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(CARD_PADDING, CARD_PADDING))
        .show(ui, |ui| {
            ui.set_max_width(width - f32::from(CARD_PADDING) * 2.0);

            ui.horizontal_wrapped(|ui| {
                ui.label(icons::NOTE.text().color(theme::PANEL_MUTED));
                ui.add_space(4.0);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(tr!(
                            "Пусто. Клавиша T ставит закладку на текущем месте.",
                            "Empty. The T key adds a bookmark at the current spot."
                        ))
                        .color(theme::TEXT_PRIMARY),
                    )
                    .wrap(),
                );
            });
        });
}

/// Плашка выбранного списка и кнопка действий над ним.
fn show_switcher(app: &mut PithApp, ui: &mut egui::Ui) {
    let Some(active) = app.active_list_name() else {
        return;
    };

    ui.horizontal(|ui| {
        style_boxes(ui);

        let width = ui.available_width() - GEAR_WIDTH - ui.spacing().item_spacing.x;

        // Место под кружок цвета: сам он рисуется кистью — знака ●
        // в шрифте может не оказаться, и вместо него выходит пустая рамка.
        let list = egui::Button::new(
            egui::RichText::new(format!("{DOT_SPACE}{active}"))
                .color(theme::TEXT_PRIMARY)
                .size(15.0),
        )
        .right_text(icons::CHEVRON.text().color(theme::PANEL_MUTED))
        .min_size(egui::vec2(width, BOX_HEIGHT));

        let color = app.active_list_color();
        let (response, _) = egui::containers::menu::MenuButton::from_button(list)
            .ui(ui, |ui| lists::show_choices(app, ui));

        paint_dot(ui, response.rect, color);
        response.on_hover_text(tr!("Выбрать список отрезков", "Choose fragment list"));

        let gear = egui::Button::new(icons::SETTINGS.text().color(theme::TEXT_PRIMARY))
            .min_size(egui::vec2(GEAR_WIDTH, BOX_HEIGHT));

        let (response, _) = egui::containers::menu::MenuButton::from_button(gear)
            .ui(ui, |ui| lists::show_actions(app, ui));
        response.on_hover_text(tr!("Действия со списком", "List actions"));
    });
}

/// Кружок цвета списка: тем же цветом его отрезки отмечены на полосе.
fn paint_dot(ui: &egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let center = egui::pos2(rect.left() + DOT_OFFSET, rect.center().y);
    ui.painter().circle_filled(center, DOT_RADIUS, color);
}

/// Длительность отрезка, отступ назад и папка вывода.
///
/// Всё правится прямо здесь: числа — колесом или вводом, папка — кнопкой.
/// Раньше ради каждой правки приходилось открывать диалог настроек списка.
fn show_details(app: &mut PithApp, ui: &mut egui::Ui) {
    let (mut duration_sec, mut buffer_sec) = app.active_list_timing();
    let (was_duration, was_buffer) = (duration_sec, buffer_sec);

    // Одной строкой: подпись, поле, единица — и промежуток между парами.
    // Двухэтажная раскладка занимала вдвое больше высоты ради тех же
    // двух чисел.
    ui.horizontal(|ui| {
        style_boxes(ui);

        show_number(
            ui,
            tr!("Длительность", "Duration"),
            &mut duration_sec,
            1..=PithApp::MAX_DURATION_SEC,
        );

        ui.add_space(NUMBERS_GAP);

        show_number(
            ui,
            tr!("Отступ", "Lead-in"),
            &mut buffer_sec,
            0..=PithApp::MAX_BUFFER_SEC,
        );
    });

    if duration_sec != was_duration || buffer_sec != was_buffer {
        app.set_active_list_timing(duration_sec, buffer_sec);
    }

    ui.add_space(GAP);
    show_output_dir(app, ui);
}

/// Папка вывода: показывается и меняется на месте.
fn show_output_dir(app: &mut PithApp, ui: &mut egui::Ui) {
    let Some(dir) = app.fragments_output_dir() else {
        return;
    };

    // Своя папка списка или общая из настроек — видно по подсказке.
    let own = app
        .current_bookmarks()
        .and_then(|video| video.active())
        .is_some_and(|list| list.output_dir.is_some());

    let mut choose = false;
    let mut reset = false;

    ui.horizontal(|ui| {
        style_boxes(ui);
        ui.spacing_mut().item_spacing.x = 4.0;

        // Кнопка сброса нужна, только когда у списка своя папка.
        let reserved = if own { 2.0 * DIR_BUTTON } else { DIR_BUTTON };
        let width = (ui.available_width() - reserved - 12.0).max(60.0);

        // Путь рисуется кистью с обрезкой хвоста: длинная строка в кнопке
        // раздвигала всю панель — область подстраивается под содержимое.
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, DIR_BUTTON), egui::Sense::click());

        paint_path(ui, rect, &dir.to_string_lossy(), response.hovered());

        choose |= response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(tr!(
                format!(
                    "{}\nНажмите, чтобы выбрать другую папку",
                    dir.to_string_lossy()
                ),
                format!("{}\nClick to choose another folder", dir.to_string_lossy())
            ))
            .clicked();

        choose |= ui
            .add_sized(
                egui::vec2(DIR_BUTTON, DIR_BUTTON),
                egui::Button::new(icons::FOLDER.text().color(theme::TEXT_PRIMARY)),
            )
            .on_hover_text(tr!(
                "Выбрать папку для отрезков",
                "Choose the folder for fragments"
            ))
            .clicked();

        if own {
            reset |= ui
                .add_sized(
                    egui::vec2(DIR_BUTTON, DIR_BUTTON),
                    egui::Button::new(icons::DELETE.text().color(theme::PANEL_MUTED)),
                )
                .on_hover_text(tr!(
                    "Вернуть общую папку из настроек",
                    "Back to the shared folder from settings"
                ))
                .clicked();
        }
    });

    if choose {
        app.choose_active_list_output_dir();
    } else if reset {
        app.set_active_list_output_dir(None);
    }
}

/// Путь к папке: одной строкой, лишнее с конца отсекается.
fn paint_path(ui: &egui::Ui, rect: egui::Rect, path: &str, hovered: bool) {
    let painter = ui.painter();

    if hovered {
        painter.rect_filled(rect, 6.0, theme::PANEL_ELEMENT_HOVER);
    }

    let mut job = egui::text::LayoutJob::simple_singleline(
        path.to_owned(),
        egui::FontId::proportional(12.0),
        theme::PANEL_MUTED,
    );

    job.wrap.max_width = rect.width() - 8.0;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;

    let galley = painter.layout_job(job);
    let top = rect.center().y - galley.size().y / 2.0;

    painter.galley(
        egui::pos2(rect.left() + 4.0, top),
        galley,
        theme::PANEL_MUTED,
    );
}

/// Подпись, поле с числом и единица — всё в одну строку.
fn show_number(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u32,
    range: std::ops::RangeInclusive<u32>,
) {
    ui.spacing_mut().item_spacing.x = 5.0;

    ui.label(
        egui::RichText::new(label)
            .color(theme::PANEL_MUTED)
            .size(12.0),
    );

    ui.add(
        egui::DragValue::new(value)
            .range(range)
            .speed(0.2)
            .update_while_editing(false),
    )
    .on_hover_text(tr!("Тяните или введите число", "Drag or type a number"));

    ui.label(
        egui::RichText::new(tr!("с", "s"))
            .color(theme::PANEL_MUTED)
            .size(12.0),
    );
}

/// Пунктир, отделяющий шапку от списка закладок.
fn dashed_divider(ui: &mut egui::Ui) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), egui::Sense::hover());

    ui.painter().extend(egui::Shape::dashed_line(
        &[rect.left_center(), rect.right_center()],
        egui::Stroke::new(1.0, theme::PANEL_BORDER),
        4.0,
        4.0,
    ));
}

/// Общий вид плашек панели: тёмная заливка, тонкая обводка, скругление.
pub fn style_boxes(ui: &mut egui::Ui) {
    let widgets = &mut ui.visuals_mut().widgets;

    for (visuals, fill) in [
        (&mut widgets.inactive, theme::PANEL_ELEMENT),
        (&mut widgets.hovered, theme::PANEL_ELEMENT_HOVER),
        (&mut widgets.active, theme::PANEL_ELEMENT_HOVER),
    ] {
        visuals.weak_bg_fill = fill;
        visuals.bg_fill = fill;
        visuals.bg_stroke = egui::Stroke::new(1.0, theme::PANEL_BORDER);
        visuals.corner_radius = egui::CornerRadius::same(BOX_RADIUS);
    }
}
