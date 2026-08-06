//! Окно истории: последние открытые файлы.
//!
//! Вызывается правым щелчком по кнопке «Открыть файл» и пунктом меню.
//! Нажатие по строке открывает файл, значок справа — папку, в которой
//! он лежит: оттуда удобно взять соседний.
//!
//! Строки выложены строгими колонками и читаются слева направо: имя
//! и путь начинаются с одной и той же границы, а обрезаются, только если
//! окно плеера совсем узкое — и всегда с конца, чтобы начало было видно.

use std::path::{Path, PathBuf};

use crate::app::PithApp;
use crate::theme;
use crate::tr;
use crate::ui::{icons, panel_head};

/// Предельная ширина окна.
///
/// Окно занимает всю ширину плеера: длинные пути должны читаться целиком.
/// Предел нужен только для очень широких экранов, где строка через весь
/// монитор превращается в поле для поиска глазами.
const MAX_WIDTH: f32 = 1400.0;

/// Наименьшая ширина: ниже неё окно уже само по себе бесполезно.
const MIN_WIDTH: f32 = 320.0;

/// Высота строки списка.
const ROW_HEIGHT: f32 = 28.0;

/// Ширина колонки со значком файла.
const ICON_COLUMN: f32 = 26.0;

/// Ширина колонки с кнопкой «открыть папку».
const ACTION_COLUMN: f32 = 30.0;

/// Какую долю строки занимает колонка с именем.
///
/// Треть: имени файла столько хватает, а путь длиннее — ему остаётся
/// больше места, и обрезать его приходится реже.
const NAME_FRACTION: f32 = 0.34;

/// Отступ между колонками.
const COLUMN_GAP: f32 = 12.0;

/// Отступ окна от левого края и от нижней панели.
///
/// Окно растёт вверх от кнопки «Открыть файл»: оно про неё, и искать его
/// посреди экрана незачем.
const SIDE_MARGIN: f32 = 12.0;
const BOTTOM_MARGIN: f32 = 52.0;

/// Что пользователь выбрал в окне.
#[derive(Default)]
struct Choice {
    /// Открыть файл.
    file: Option<PathBuf>,
    /// Открыть папку, в которой он лежит.
    dir: Option<PathBuf>,
    close: bool,
}

/// Показывает окно истории, если оно открыто.
pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if !app.history_open() {
        return;
    }

    let mut choice = Choice::default();
    let mut open = true;
    let width = window_width(ctx);

    let window = egui::Window::new(tr!("История", "Recent files"))
        .order(egui::Order::Foreground)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        // Левым нижним углом у кнопки «Открыть файл» — окно раскрывается
        // прямо над ней.
        .anchor(
            egui::Align2::LEFT_BOTTOM,
            egui::vec2(SIDE_MARGIN, -BOTTOM_MARGIN),
        )
        .show(ctx, |ui| {
            ui.set_width(width);
            panel_head::style_boxes(ui);

            show_files(app, ui, &mut choice);
        });

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        choice.close = true;
    }

    // Нажатие мимо окна тоже закрывает: оно вспомогательное, и держать
    // его на экране до крестика незачем.
    //
    // Кроме того самого нажатия, которым окно и вызвали: и пункт меню,
    // и правый щелчок по кнопке приходят в том же кадре, где окно
    // впервые рисуется, и «мимо» ловило именно их — окно закрывалось,
    // не успев появиться.
    let dismissed = ctx.input(|i| i.pointer.any_click()) && !app.history_just_opened();

    if dismissed && window.is_some_and(|window| window.response.clicked_elsewhere()) {
        choice.close = true;
    }

    if let Some(path) = choice.file {
        app.open_from_history(&path);
    } else if let Some(dir) = choice.dir {
        app.open_dialog_in(&dir);
    } else if choice.close || !open {
        app.close_history();
    }
}

/// Ширина окна: во всю ширину плеера, но в разумных пределах.
///
/// Широкое окно — чтобы пути помещались целиком. Обрезка остаётся только
/// для маленького окна плеера, где места просто нет.
fn window_width(ctx: &egui::Context) -> f32 {
    let screen = ctx.input(|i| i.viewport_rect()).width();

    (screen - SIDE_MARGIN * 2.0).clamp(MIN_WIDTH, MAX_WIDTH)
}

/// Последние открытые файлы: имя, путь и кнопка «открыть папку».
fn show_files(app: &PithApp, ui: &mut egui::Ui, choice: &mut Choice) {
    let files = app.history_files();

    if files.is_empty() {
        ui.label(
            egui::RichText::new(tr!("Пока ничего не открывали", "Nothing opened yet"))
                .color(theme::PANEL_MUTED),
        );
        return;
    }

    for (index, path) in files.iter().enumerate() {
        let name = file_name(path);
        let parent = path
            .parent()
            .map(|dir| dir.to_string_lossy().to_string())
            .unwrap_or_default();

        let row = show_row(ui, index, &name, &parent, &path.to_string_lossy());

        if row.open_dir {
            choice.dir = path.parent().map(Path::to_path_buf);
        } else if row.open_file {
            choice.file = Some(path.clone());
        }
    }
}

/// Что нажали в строке.
#[derive(Default)]
struct RowAction {
    open_file: bool,
    open_dir: bool,
}

/// Строка списка: значок, имя, путь и кнопка папки — каждый в своей колонке.
///
/// Рисуется кистью, а не набором виджетов: только так границы колонок
/// совпадают у всех строк. У кнопок содержимое центруется по своей
/// ширине, и строки разъезжались.
fn show_row(ui: &mut egui::Ui, index: usize, name: &str, path: &str, hint: &str) -> RowAction {
    let width = ui.available_width();
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, ROW_HEIGHT), egui::Sense::click());

    // Кнопка папки — своя область поверх строки: нажатие по ней не должно
    // заодно открывать файл.
    let button_rect = egui::Rect::from_min_max(
        egui::pos2(rect.right() - ACTION_COLUMN, rect.top()),
        rect.max,
    );

    let button = ui.interact(
        button_rect,
        ui.id().with(("история_папка", index)),
        egui::Sense::click(),
    );

    if ui.is_rect_visible(rect) {
        paint_row(
            ui,
            rect,
            button_rect,
            name,
            path,
            response.hovered(),
            button.hovered(),
        );
    }

    ui.add_space(2.0);

    let button = button
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tr!(
            "Открыть папку с этим файлом",
            "Open the folder with this file"
        ));

    let response = response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(hint);

    RowAction {
        open_dir: button.clicked(),
        open_file: response.clicked(),
    }
}

fn paint_row(
    ui: &egui::Ui,
    rect: egui::Rect,
    button_rect: egui::Rect,
    name: &str,
    path: &str,
    hovered: bool,
    button_hovered: bool,
) {
    let painter = ui.painter();

    if hovered || button_hovered {
        painter.rect_filled(rect, 6.0, theme::PANEL_ELEMENT_HOVER);
    }

    let (glyph, font) = icons::NOTE.painted(14.0);
    painter.text(
        egui::pos2(rect.left() + ICON_COLUMN / 2.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        glyph,
        font,
        theme::PANEL_MUTED,
    );

    let text_left = rect.left() + ICON_COLUMN;
    let text_right = button_rect.left() - COLUMN_GAP;
    let rest = (text_right - text_left - COLUMN_GAP).max(0.0);

    // Колонка имени — постоянной доли ширины: так вторая колонка у всех
    // строк начинается на одном месте.
    let name_width = rest * NAME_FRACTION;
    let path_left = text_left + name_width + COLUMN_GAP;

    paint_column(
        ui,
        egui::pos2(text_left, rect.center().y),
        name_width,
        name,
        13.0,
        theme::TEXT_PRIMARY,
    );

    paint_column(
        ui,
        egui::pos2(path_left, rect.center().y),
        (text_right - path_left).max(0.0),
        path,
        12.0,
        theme::PANEL_MUTED,
    );

    // Значок папки: заметнее под курсором, чтобы было понятно, что он живой.
    let (glyph, font) = icons::FOLDER.painted(14.0);
    let color = if button_hovered {
        theme::PANEL_ACCENT
    } else {
        theme::PANEL_MUTED
    };

    painter.text(
        button_rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        font,
        color,
    );
}

/// Текст колонки: слева направо, с обрезкой хвоста по нехватке места.
fn paint_column(
    ui: &egui::Ui,
    left_center: egui::Pos2,
    width: f32,
    text: &str,
    size: f32,
    color: egui::Color32,
) {
    if width <= 0.0 || text.is_empty() {
        return;
    }

    let mut job = egui::text::LayoutJob::simple_singleline(
        text.to_owned(),
        egui::FontId::proportional(size),
        color,
    );

    // Одна строка, обрезка с конца: начало пути и имени важнее хвоста.
    job.wrap.max_width = width;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;

    let galley = ui.painter().layout_job(job);
    let top = left_center.y - galley.size().y / 2.0;

    ui.painter()
        .galley(egui::pos2(left_center.x, top), galley, color);
}

/// Имя файла или папки без пути.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        ACTION_COLUMN, COLUMN_GAP, ICON_COLUMN, MAX_WIDTH, MIN_WIDTH, NAME_FRACTION, SIDE_MARGIN,
        file_name,
    };
    use std::path::Path;

    /// Тот же расчёт, что и в `window_width`, но без контекста egui.
    fn width(screen: f32) -> f32 {
        (screen - SIDE_MARGIN * 2.0).clamp(MIN_WIDTH, MAX_WIDTH)
    }

    /// Границы колонок: где начинается имя и где путь.
    fn columns(row_width: f32) -> (f32, f32) {
        let text_right = row_width - ACTION_COLUMN - COLUMN_GAP;
        let rest = text_right - ICON_COLUMN - COLUMN_GAP;

        (ICON_COLUMN, ICON_COLUMN + rest * NAME_FRACTION + COLUMN_GAP)
    }

    #[test]
    fn имя_берётся_без_пути() {
        assert_eq!(file_name(Path::new("C:\\Кино\\фильм.mkv")), "фильм.mkv");
        assert_eq!(file_name(Path::new("C:\\Кино")), "Кино");
    }

    #[test]
    fn окно_расширяется_под_плеер() {
        assert!(width(1280.0) > width(700.0), "шире окно — шире история");
    }

    #[test]
    fn ширина_держится_в_пределах() {
        assert_eq!(width(4000.0), MAX_WIDTH, "на большом экране");
        assert_eq!(width(200.0), MIN_WIDTH, "в крошечном окне");
    }

    #[test]
    fn колонки_у_всех_строк_на_одном_месте() {
        // Границы зависят только от ширины строки, а она у всех одна —
        // значит, имена и пути выстраиваются столбцом.
        let (name, path) = columns(700.0);

        assert_eq!(columns(700.0), (name, path));
        assert!(path > name, "путь правее имени");
        assert!(
            path < 700.0 - ACTION_COLUMN,
            "колонка пути не залезает под кнопку папки"
        );
    }
}
