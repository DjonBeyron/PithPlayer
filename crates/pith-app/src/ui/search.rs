//! Окно поиска по субтитрам.
//!
//! Порт `SubtitleSearchForm` из v4 (PLAN.md §6.2).

use crate::app::PithApp;
use crate::theme;
use crate::tr;
use crate::ui::{format_time, icons};

/// Высота списка результатов.
const RESULTS_HEIGHT: f32 = 320.0;

/// Размер кнопки «поставить закладку» в строке результата.
const ADD_BUTTON: [f32; 2] = [24.0, 20.0];

/// Найденная реплика, на которую просят поставить закладку.
struct BookmarkRequest {
    seconds: f64,
    text: String,
}

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if !app.search_state().open {
        return;
    }

    let mut open = true;
    let mut query = app.search_state().query.clone();
    let mut jump_to = None;
    let mut bookmark = None;
    let mut query_changed = false;

    egui::Window::new(tr!("Поиск по субтитрам", "Subtitle search"))
        .order(egui::Order::Foreground)
        .open(&mut open)
        .default_width(560.0)
        .collapsible(false)
        .show(ctx, |ui| {
            let field = ui.add(
                egui::TextEdit::singleline(&mut query)
                    .hint_text(tr!("Введите фразу", "Type a phrase"))
                    .desired_width(f32::INFINITY),
            );

            // Курсор сразу в поле: окно открывают, чтобы печатать.
            if field.changed() {
                query_changed = true;
            }
            if !field.has_focus() && ui.memory(|m| m.focused().is_none()) {
                field.request_focus();
            }

            ui.add_space(6.0);

            if let Some(status) = &app.search_state().status {
                ui.label(egui::RichText::new(status).color(theme::TEXT_SECONDARY));
                return;
            }

            show_results(app, ui, &mut jump_to, &mut bookmark);
        });

    if query_changed {
        app.set_search_query(query);
    }

    if let Some(request) = bookmark {
        app.add_bookmark_at(request.seconds, Some(request.text));
    }

    if let Some(time) = jump_to {
        app.seek_absolute(time);
    }

    if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.close_search();
    }
}

fn show_results(
    app: &PithApp,
    ui: &mut egui::Ui,
    jump_to: &mut Option<f64>,
    bookmark: &mut Option<BookmarkRequest>,
) {
    let state = app.search_state();

    if state.query.trim().is_empty() {
        ui.label(
            egui::RichText::new(tr!(
                "Начните вводить фразу — найденные реплики появятся здесь",
                "Start typing — matching lines will show up here"
            ))
            .color(theme::TEXT_SECONDARY),
        );
        return;
    }

    if state.hits.is_empty() {
        ui.label(
            egui::RichText::new(tr!("Ничего не найдено", "Nothing found"))
                .color(theme::TEXT_SECONDARY),
        );
        return;
    }

    ui.label(
        egui::RichText::new(tr!(
            format!("Найдено реплик: {}", state.hits.len()),
            format!("Lines found: {}", state.hits.len())
        ))
        .color(theme::TEXT_SECONDARY)
        .small(),
    );
    ui.add_space(4.0);

    egui::ScrollArea::vertical()
        .max_height(RESULTS_HEIGHT)
        .show(ui, |ui| {
            for hit in &state.hits {
                ui.horizontal(|ui| {
                    // Закладка прямо из списка: чаще всего именно за этим
                    // реплику и ищут — чтобы потом вырезать отрезок.
                    let add = ui
                        .add_sized(
                            ADD_BUTTON,
                            egui::Button::new(icons::ADD.text().color(theme::TEXT_SECONDARY))
                                .frame(false),
                        )
                        .on_hover_text(tr!(
                            "Поставить закладку на этой реплике",
                            "Bookmark this line"
                        ));

                    // Фокус кнопке не оставляем: он нужен полю запроса,
                    // иначе после закладки в нём нельзя дописать ни буквы.
                    if crate::ui::clicked(&add) {
                        *bookmark = Some(BookmarkRequest {
                            seconds: hit.start,
                            text: hit.text.clone(),
                        });
                    }

                    // Остальная строка — один выбираемый пункт списка: он
                    // подсвечивается под курсором и кликается по всей ширине.
                    // Обычные надписи в egui выделяются мышью, и щелчок по ним
                    // уходил в выделение текста вместо перехода к реплике.
                    ui.style_mut().spacing.interact_size.x = ui.available_width();

                    let row = ui.selectable_label(
                        false,
                        (
                            egui::RichText::new(format!("{}  ", format_time(hit.start)))
                                .color(theme::ACCENT)
                                .monospace(),
                            egui::RichText::new(&hit.text).color(theme::TEXT_PRIMARY),
                        ),
                    );

                    if row
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        *jump_to = Some(hit.start);
                    }
                });
            }
        });
}
