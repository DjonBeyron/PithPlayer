//! Панель закладок и отрезков.
//!
//! Порт `BookmarksPanel` из v4 (PLAN.md §6.5).

use crate::app::PithApp;
use crate::theme;
use crate::ui::format_time;

/// Ширина панели.
const PANEL_WIDTH: f32 = 320.0;

/// Отступ сверху: под панелью замеров.
const PANEL_TOP: f32 = 240.0;

/// Отступ от правого края окна.
const PANEL_MARGIN: f32 = 36.0;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if !app.bookmarks_panel_open() {
        return;
    }

    let mut jump_to = None;
    let mut remove = None;
    let mut start_extraction = false;

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

                    ui.label(
                        egui::RichText::new("Отрезки")
                            .color(theme::TEXT_PRIMARY)
                            .strong()
                            .size(16.0),
                    );
                    ui.separator();

                    show_summary(app, ui);
                    ui.separator();

                    show_list(app, ui, &mut jump_to, &mut remove);

                    ui.separator();
                    start_extraction = show_actions(app, ui);
                });
        });

    if let Some(time) = jump_to {
        app.seek_absolute(time);
    }
    if let Some(time_ms) = remove {
        app.remove_bookmark_at(time_ms);
    }
    if start_extraction {
        app.start_extraction();
    }
}

/// Сводка: список, длительность отрезка, папка вывода.
fn show_summary(app: &PithApp, ui: &mut egui::Ui) {
    let Some(video) = app.current_bookmarks() else {
        ui.label(egui::RichText::new("Файл не открыт").color(theme::TEXT_SECONDARY));
        return;
    };

    let Some(list) = video.active() else {
        return;
    };

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(&list.name)
                .color(theme::TEXT_PRIMARY)
                .strong(),
        );
        ui.label(
            egui::RichText::new(format!(
                "{} с, отступ {} с",
                list.duration_sec, list.buffer_sec
            ))
            .color(theme::TEXT_SECONDARY)
            .small(),
        );
    });

    if let Some(dir) = app.fragments_output_dir() {
        ui.label(
            egui::RichText::new(dir.to_string_lossy())
                .color(theme::TEXT_DISABLED)
                .small(),
        )
        .on_hover_text("Куда сохраняются вырезанные отрезки");
    }
}

/// Список закладок активного списка.
fn show_list(
    app: &PithApp,
    ui: &mut egui::Ui,
    jump_to: &mut Option<f64>,
    remove: &mut Option<i64>,
) {
    let Some(list) = app.current_bookmarks().and_then(|v| v.active()) else {
        return;
    };

    if list.bookmarks.is_empty() {
        ui.label(
            egui::RichText::new("Пусто. Клавиша T ставит закладку на текущем месте.")
                .color(theme::TEXT_SECONDARY),
        );
        return;
    }

    egui::ScrollArea::vertical()
        .max_height(360.0)
        .show(ui, |ui| {
            for bookmark in &list.bookmarks {
                ui.horizontal(|ui| {
                    let label =
                        format!("{}  {}", format_time(bookmark.seconds()), bookmark.label());

                    if ui
                        .selectable_label(false, label)
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        *jump_to = Some(bookmark.seconds());
                    }

                    if ui.small_button("✕").on_hover_text("Убрать").clicked() {
                        *remove = Some(bookmark.time_ms);
                    }
                });
            }
        });
}

/// Кнопка запуска нарезки и ход выполнения.
fn show_actions(app: &PithApp, ui: &mut egui::Ui) -> bool {
    if let Some(progress) = app.extraction_progress() {
        ui.label(
            egui::RichText::new(format!("Нарезка: {} из {}", progress.done, progress.total))
                .color(theme::ACCENT),
        );
        ui.add(egui::ProgressBar::new(progress.fraction()).show_percentage());
        return false;
    }

    let count = app
        .current_bookmarks()
        .and_then(|v| v.active())
        .map(|l| l.bookmarks.len())
        .unwrap_or(0);

    if count == 0 {
        return false;
    }

    ui.add_enabled(
        app.can_extract(),
        egui::Button::new(format!("Вырезать отрезки ({count})")),
    )
    .on_disabled_hover_text("Нужен ffmpeg.exe рядом с плеером")
    .clicked()
}
