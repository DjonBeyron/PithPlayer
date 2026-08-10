//! Папка вывода в шапке панели отрезков.
//!
//! Отдельно от остальной шапки: у строки с папкой своя разметка с
//! обрезкой длинного пути и две кнопки со своими условиями.

use crate::app::PithApp;
use crate::theme;
use crate::tr;
use crate::ui::{icons, panel_head};

/// Размер квадратных кнопок в строке с папкой.
const DIR_BUTTON: f32 = 26.0;

/// Папка вывода: показывается и меняется на месте.
pub fn show(app: &mut PithApp, ui: &mut egui::Ui) {
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
        panel_head::style_boxes(ui);
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
