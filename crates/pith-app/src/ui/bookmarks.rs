//! Панель закладок и отрезков.
//!
//! Порт `BookmarksPanel` из v4 (PLAN.md §6.5).

use crate::app::PithApp;
use crate::theme;
use crate::tr;
use crate::ui::{bookmarks_actions, format_time, icons, panel_head};

/// Ширина панели.
const PANEL_WIDTH: f32 = 320.0;

/// Отступ содержимого от краёв панели.
const PANEL_PADDING: i8 = 12;

/// Сколько места оставляем снизу под панелью управления.
///
/// Панель отрезков идёт во всю высоту окна, и без этого запаса её
/// последние кнопки прятались бы под полосой перемотки.
const CONTROLS_SPACE: i8 = 52;

/// Наименьшая высота списка: ниже него он бесполезен.
const MIN_LIST_HEIGHT: f32 = 90.0;

/// Отступ между списком и кнопками нарезки.
const HEAD_GAP: f32 = 14.0;

/// Отступ вокруг строки кнопок.
pub(super) const BUTTON_GAP: f32 = 6.0;

/// Высота кнопок нарезки.
pub(super) const BUTTON_HEIGHT: f32 = 32.0;

/// Скругление кнопок нарезки.
pub(super) const BUTTON_RADIUS: u8 = 8;

/// Ширина колонки со временем: столько занимает «0:00:00».
const TIME_COLUMN: f32 = 52.0;

/// Что пользователь сделал в панели за кадр.
#[derive(Default)]
pub(super) struct PanelActions {
    pub(super) jump_to: Option<f64>,
    pub(super) remove: Option<i64>,
    /// Перенос закладки: время метки и имя списка-приёмника.
    pub(super) move_to: Option<(i64, String)>,
    /// Какую закладку переименовать.
    pub(super) rename: Option<i64>,
    /// Спросить подтверждение на очистку списка.
    pub(super) clear: bool,
    pub(super) extract_active: bool,
    pub(super) extract_all: bool,
    /// Вырезать один отрезок — метка, напротив которой нажали ножницы.
    pub(super) extract_one: Option<i64>,
}

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if !app.bookmarks_panel_open() {
        return;
    }

    let mut actions = PanelActions::default();

    // Своя область, а не `Window`: панель выдвигается от края, ей не нужны
    // ни заголовок, ни перетаскивание. Положение считаем сами: область
    // с якорем не знает своей ширины на первом кадре и уезжает за край.
    let screen = ctx.input(|i| i.viewport_rect());
    let position = egui::pos2(screen.max.x - PANEL_WIDTH, screen.min.y);

    // Первые кадры панель рисуется целиком, но невидимой: egui считает
    // размеры списка и полосы прокрутки, и при длинном списке иначе видно,
    // как панель достраивается на глазах.
    let opacity = app.bookmarks_panel_opacity();

    let area = egui::Area::new(egui::Id::new("bookmarks_panel"))
        // Слоем ниже панели управления: ящик идёт во всю высоту окна,
        // и его нижний край накрывал бы полосу перемотки и кнопки.
        // В одном слое побеждает та область, которую трогали последней,
        // — а трогают как раз ящик.
        .order(egui::Order::Middle)
        .fixed_pos(position)
        .show(ctx, |ui| {
            ui.set_opacity(opacity);
            ui.set_width(PANEL_WIDTH);

            egui::Frame::NONE
                // Непрозрачная: выдвижной ящик, а не полупрозрачная
                // карточка — сквозь неё не должно просвечивать видео.
                .fill(theme::PANEL_CARD)
                .inner_margin(egui::Margin {
                    left: PANEL_PADDING,
                    right: PANEL_PADDING,
                    top: PANEL_PADDING,
                    bottom: CONTROLS_SPACE,
                })
                .show(ui, |ui| {
                    ui.set_width(PANEL_WIDTH);
                    // Во всю высоту окна: панель — выдвижной ящик, а не
                    // висящая посреди экрана карточка.
                    ui.set_min_height(screen.height() - f32::from(PANEL_PADDING + CONTROLS_SPACE));
                    show_body(app, ui, &mut actions);
                });
        });

    if opacity < 1.0 {
        // Кадры прогрева должны пройти и на паузе, когда egui сам
        // перерисовывать не станет.
        app.finish_panel_warmup_frame();
        ctx.request_repaint();

        // Пока панель не показана, нажатия по ней не принимаем: щёлкнуть
        // в невидимое пользователь не мог.
        return;
    }

    // Нажатие мимо панели закрывает её. Диалоги, вызванные из панели,
    // считаются её продолжением — их отсекает сам `close_bookmarks_panel`.
    if area.response.clicked_elsewhere() {
        app.close_bookmarks_panel();
    }

    apply(app, actions);
}

fn show_body(app: &mut PithApp, ui: &mut egui::Ui, actions: &mut PanelActions) {
    panel_head::show(app, ui);

    if !app.has_open_file() {
        return;
    }

    // Кнопки нарезки всегда у нижнего края панели: они главное действие,
    // и искать их в конце списка на полсотни закладок неудобно. Место под
    // них отмеряется от низа, а список забирает всё, что выше.
    let panel = ui.max_rect();
    let reserved = actions_height(app);
    let buttons_top = panel.bottom() - reserved;

    let list_height = (buttons_top - ui.cursor().top() - HEAD_GAP).max(MIN_LIST_HEIGHT);
    show_list(app, ui, list_height, actions);

    let buttons = egui::Rect::from_min_max(egui::pos2(panel.left(), buttons_top), panel.max);
    let mut bottom = ui.new_child(egui::UiBuilder::new().max_rect(buttons));
    bookmarks_actions::show(app, &mut bottom, actions);
}

/// Сколько высоты занимают кнопки нарезки.
///
/// Вторая строка появляется, только когда списков больше одного, —
/// и место под неё нужно отмерить заранее.
fn actions_height(app: &PithApp) -> f32 {
    let many_lists = app
        .current_bookmarks()
        .is_some_and(|video| video.lists.len() > 1);

    if many_lists {
        BUTTON_HEIGHT * 2.0 + BUTTON_GAP * 2.0
    } else {
        BUTTON_HEIGHT + BUTTON_GAP
    }
}

/// Закладки активного списка.
fn show_list(app: &PithApp, ui: &mut egui::Ui, height: f32, actions: &mut PanelActions) {
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

    let can_extract = app.can_extract();

    egui::ScrollArea::vertical()
        .max_height(height)
        .show(ui, |ui| {
            for bookmark in &list.bookmarks {
                show_row(ui, bookmark, &others, can_extract, actions);
            }
        });
}

/// Одна строка списка: время, название, кнопки.
///
/// Кнопки выкладываются справа налево первыми, поэтому стоят ровным
/// столбцом независимо от длины реплики. Время — в колонке постоянной
/// ширины, иначе названия начинались бы вразнобой.
fn show_row(
    ui: &mut egui::Ui,
    bookmark: &pith_store::TimeBookmark,
    others: &[String],
    can_extract: bool,
    actions: &mut PanelActions,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new(icons::DELETE.text()).frame(false))
                .on_hover_text(tr!("Убрать", "Remove"))
                .clicked()
            {
                actions.remove = Some(bookmark.time_ms);
            }

            if ui
                .add(egui::Button::new(icons::EDIT.text()).frame(false))
                .on_hover_text(tr!("Переименовать", "Rename"))
                .clicked()
            {
                actions.rename = Some(bookmark.time_ms);
            }

            // Вырезать только этот отрезок: резать весь список ради одной
            // метки — это лишние файлы и минуты ожидания.
            if ui
                .add_enabled(
                    can_extract,
                    egui::Button::new(icons::CUT.text().color(theme::PANEL_ACCENT)).frame(false),
                )
                .on_hover_text(tr!("Вырезать этот отрезок", "Cut this fragment"))
                .on_disabled_hover_text(tr!(
                    "Нужен ffmpeg.exe рядом с плеером",
                    "ffmpeg.exe must sit next to the player"
                ))
                .clicked()
            {
                actions.extract_one = Some(bookmark.time_ms);
            }

            // Остаток строки — под время и название.
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(TIME_COLUMN, ui.available_height()),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.label(
                            egui::RichText::new(format_time(bookmark.seconds()))
                                .color(theme::PANEL_MUTED)
                                .monospace(),
                        );
                    },
                );

                let title = bookmark.name.clone().unwrap_or_default();
                let row = ui
                    .add(
                        egui::Label::new(egui::RichText::new(title).color(theme::TEXT_PRIMARY))
                            .truncate()
                            .sense(egui::Sense::click()),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                if row.clicked() {
                    actions.jump_to = Some(bookmark.seconds());
                }

                row.context_menu(|ui| {
                    show_move_menu(ui, bookmark.time_ms, others, actions);
                });
            });
        });
    });
}

fn show_empty(ui: &mut egui::Ui) {
    panel_head::show_empty(ui);
}

/// Меню переноса закладки в другой список.
fn show_move_menu(ui: &mut egui::Ui, time_ms: i64, others: &[String], actions: &mut PanelActions) {
    if others.is_empty() {
        ui.label(
            egui::RichText::new(tr!("Других списков нет", "No other lists"))
                .color(theme::TEXT_SECONDARY),
        );
        return;
    }

    ui.label(
        egui::RichText::new(tr!("Перенести в список", "Move to list")).color(theme::TEXT_SECONDARY),
    );

    for name in others {
        if ui.button(crate::i18n::list_name(name)).clicked() {
            actions.move_to = Some((time_ms, name.clone()));
            ui.close();
        }
    }
}

/// Кнопки запуска нарезки и ход выполнения.
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
    if let Some(time_ms) = actions.rename {
        app.open_bookmark_rename(time_ms);
    }
    if actions.clear {
        app.ask_clear_list();
    }
    if actions.extract_active {
        app.start_extraction();
    }
    if actions.extract_all {
        app.start_extraction_all_lists();
    }
    if let Some(time_ms) = actions.extract_one {
        app.start_extraction_one(time_ms);
    }
}
