//! Панель закладок и отрезков.
//!
//! Порт `BookmarksPanel` из v4 (PLAN.md §6.5).

use crate::app::PithApp;
use crate::theme;
use crate::ui::{format_time, lists};

/// Ширина панели.
const PANEL_WIDTH: f32 = 320.0;

/// Отступ сверху: под панелью замеров.
const PANEL_TOP: f32 = 240.0;

/// Отступ от правого края окна.
const PANEL_MARGIN: f32 = 36.0;

/// Что пользователь сделал в панели за кадр.
#[derive(Default)]
struct PanelActions {
    jump_to: Option<f64>,
    remove: Option<i64>,
    /// Перенос закладки: время метки и имя списка-приёмника.
    move_to: Option<(i64, String)>,
    extract_active: bool,
    extract_all: bool,
}

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if !app.bookmarks_panel_open() {
        return;
    }

    let mut actions = PanelActions::default();

    // Своя область, а не `Window`: панель выезжает и прячется сама,
    // ей не нужны ни заголовок, ни перетаскивание.
    // Положение считаем сами: область с якорем не знает своей ширины
    // на первом кадре и уезжает за край окна.
    let screen = ctx.input(|i| i.viewport_rect());
    let position = egui::pos2(
        screen.max.x - PANEL_WIDTH - PANEL_MARGIN,
        screen.min.y + PANEL_TOP,
    );

    egui::Area::new(egui::Id::new("bookmarks_panel"))
        .order(egui::Order::Foreground)
        .fixed_pos(position)
        .show(ctx, |ui| {
            ui.set_width(PANEL_WIDTH);

            egui::Frame::NONE
                .fill(theme::WINDOW_BG.gamma_multiply(0.96))
                .inner_margin(12.0)
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.set_width(PANEL_WIDTH);
                    show_body(app, ui, &mut actions);
                });
        });

    apply(app, actions);
}

fn show_body(app: &mut PithApp, ui: &mut egui::Ui, actions: &mut PanelActions) {
    ui.label(
        egui::RichText::new("Отрезки")
            .color(theme::TEXT_PRIMARY)
            .strong()
            .size(16.0),
    );
    ui.separator();

    if !app.has_open_file() {
        ui.label(egui::RichText::new("Файл не открыт").color(theme::TEXT_SECONDARY));
        return;
    }

    lists::show_switcher(app, ui);
    show_summary(app, ui);
    ui.separator();

    show_list(app, ui, actions);

    ui.separator();
    show_actions(app, ui, actions);
}

/// Параметры активного списка: длительность отрезка и папка вывода.
fn show_summary(app: &PithApp, ui: &mut egui::Ui) {
    let (duration_sec, buffer_sec) = app.active_list_timing();

    ui.label(
        egui::RichText::new(format!("{duration_sec} с, отступ {buffer_sec} с"))
            .color(theme::TEXT_SECONDARY)
            .small(),
    );

    if let Some(dir) = app.fragments_output_dir() {
        ui.label(
            egui::RichText::new(dir.to_string_lossy())
                .color(theme::TEXT_DISABLED)
                .small(),
        )
        .on_hover_text("Куда сохраняются вырезанные отрезки");
    }
}

/// Закладки активного списка.
fn show_list(app: &PithApp, ui: &mut egui::Ui, actions: &mut PanelActions) {
    let list = app.current_bookmarks().and_then(|v| v.active());

    let Some((video, list)) = app.current_bookmarks().zip(list) else {
        show_empty(ui);
        return;
    };

    if list.bookmarks.is_empty() {
        show_empty(ui);
        return;
    }

    // Куда можно перенести метку — все списки, кроме текущего.
    let others: Vec<String> = video
        .names()
        .into_iter()
        .filter(|name| *name != video.active_list)
        .collect();

    egui::ScrollArea::vertical()
        .max_height(360.0)
        .show(ui, |ui| {
            for bookmark in &list.bookmarks {
                ui.horizontal(|ui| {
                    // Без названия подписью служит само время — второй раз
                    // его показывать незачем.
                    let label = match &bookmark.name {
                        Some(name) => format!("{}  {name}", format_time(bookmark.seconds())),
                        None => format_time(bookmark.seconds()),
                    };

                    let row = ui
                        .selectable_label(false, label)
                        .on_hover_cursor(egui::CursorIcon::PointingHand);

                    if row.clicked() {
                        actions.jump_to = Some(bookmark.seconds());
                    }

                    row.context_menu(|ui| {
                        show_move_menu(ui, bookmark.time_ms, &others, actions);
                    });

                    // Не «✕»: этого знака нет в шрифтах egui, кнопка выходила
                    // пустым квадратом.
                    if ui.small_button("🗑").on_hover_text("Убрать").clicked() {
                        actions.remove = Some(bookmark.time_ms);
                    }
                });
            }
        });
}

fn show_empty(ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new("Пусто. Клавиша T ставит закладку на текущем месте.")
            .color(theme::TEXT_SECONDARY),
    );
}

/// Меню переноса закладки в другой список.
fn show_move_menu(ui: &mut egui::Ui, time_ms: i64, others: &[String], actions: &mut PanelActions) {
    if others.is_empty() {
        ui.label(egui::RichText::new("Других списков нет").color(theme::TEXT_SECONDARY));
        return;
    }

    ui.label(egui::RichText::new("Перенести в список").color(theme::TEXT_SECONDARY));

    for name in others {
        if ui.button(name).clicked() {
            actions.move_to = Some((time_ms, name.clone()));
            ui.close();
        }
    }
}

/// Кнопки запуска нарезки и ход выполнения.
fn show_actions(app: &PithApp, ui: &mut egui::Ui, actions: &mut PanelActions) {
    if let Some(progress) = app.extraction_progress() {
        ui.label(
            egui::RichText::new(format!("Нарезка: {} из {}", progress.done, progress.total))
                .color(theme::ACCENT),
        );
        ui.add(egui::ProgressBar::new(progress.fraction()).show_percentage());
        return;
    }

    let Some(video) = app.current_bookmarks() else {
        return;
    };

    let active_count = video.active().map(|l| l.bookmarks.len()).unwrap_or(0);
    let total: usize = video.lists.iter().map(|l| l.bookmarks.len()).sum();

    if total == 0 {
        return;
    }

    let can_extract = app.can_extract();

    if active_count > 0 {
        actions.extract_active |= ui
            .add_enabled(
                can_extract,
                egui::Button::new(format!("Вырезать отрезки ({active_count})")),
            )
            .on_disabled_hover_text("Нужен ffmpeg.exe рядом с плеером")
            .clicked();
    }

    // Все списки — только когда их больше одного: иначе кнопка повторяет
    // соседнюю и лишь путает.
    if video.lists.len() > 1 {
        actions.extract_all |= ui
            .add_enabled(
                can_extract,
                egui::Button::new(format!("Вырезать все списки ({total})")),
            )
            .on_hover_text("Каждый список — в свою подпапку")
            .on_disabled_hover_text("Нужен ffmpeg.exe рядом с плеером")
            .clicked();
    }
}

fn apply(app: &mut PithApp, actions: PanelActions) {
    if let Some(time) = actions.jump_to {
        app.seek_absolute(time);
    }
    if let Some(time_ms) = actions.remove {
        app.remove_bookmark_at(time_ms);
    }
    if let Some((time_ms, target)) = actions.move_to {
        app.move_bookmark_to_list(time_ms, &target);
    }
    if actions.extract_active {
        app.start_extraction();
    }
    if actions.extract_all {
        app.start_extraction_all_lists();
    }
}
