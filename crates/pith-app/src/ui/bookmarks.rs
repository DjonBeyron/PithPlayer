//! Панель закладок и отрезков.
//!
//! Порт `BookmarksPanel` из v4 (PLAN.md §6.5). Левый край панели — её
//! ширина и язычок закрытия — живёт отдельно, в `panel_resize.rs`.

use crate::app::PithApp;
use crate::theme;
use crate::tr;
use crate::ui::{bookmarks_actions, bookmarks_row, panel_head, panel_resize};

/// Отступ содержимого от краёв панели.
const PANEL_PADDING: i8 = 12;

/// Размер откреплённого окна при первом показе и его предел.
const DETACHED_SIZE: [f32; 2] = [420.0, 700.0];
const DETACHED_MIN_SIZE: [f32; 2] = [300.0, 320.0];

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
    /// Выгрузить активный список в Notion.
    pub(super) export: bool,
    /// Вырезать один отрезок — метка, напротив которой нажали ножницы.
    pub(super) extract_one: Option<i64>,
    /// Название закладки, которое просят положить в буфер обмена.
    pub(super) copy_name: Option<String>,
}

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    // Откреплённая панель живёт своим окном и от наведения не зависит.
    if app.bookmarks_panel_detached() {
        show_detached(app, ctx);
        return;
    }

    if !app.bookmarks_panel_open() {
        return;
    }

    let mut actions = PanelActions::default();

    // Своя область, а не `Window`: панель выдвигается от края, ей не нужны
    // ни заголовок, ни перетаскивание. Положение считаем сами: область
    // с якорем не знает своей ширины на первом кадре и уезжает за край.
    let screen = ctx.input(|i| i.viewport_rect());
    let width = app.bookmarks_panel_width(screen.width());
    let position = egui::pos2(screen.max.x - width, screen.min.y);

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
        // Область не двигают: место у неё своё, у правого края. Иначе она
        // забирает перетаскивание себе — и полоса, за которую тянут ширину,
        // не получает его вовсе.
        .movable(false)
        .show(ctx, |ui| {
            ui.set_opacity(opacity);
            ui.set_width(width);

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
                    // Содержимому достаётся ширина без полей рамки: рамка
                    // прибавляет их снаружи, и панель выходила шире
                    // заказанного. Тогда её левый край не совпадал с полосой,
                    // за которую тянут, — та оказывалась на два десятка точек
                    // правее видимого края.
                    ui.set_width(width - f32::from(PANEL_PADDING) * 2.0);
                    // Во всю высоту окна: панель — выдвижной ящик, а не
                    // висящая посреди экрана карточка.
                    ui.set_min_height(screen.height() - f32::from(PANEL_PADDING + CONTROLS_SPACE));
                    show_body(app, ui, &mut actions);
                });

            // Край, за который тянут, — последним: в egui позднейший
            // виджет забирает нажатие себе, и полоса не отдаст его списку
            // под ней. И он внутри области — иначе перетаскивание считалось
            // бы нажатием мимо панели, и она закрывалась бы под рукой.
            panel_resize::show(app, ui, screen, position.x);
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

/// Панель отдельным окном системы.
///
/// Место и размер запоминаются между запусками, как у окон актёров
/// и выгрузки. Ни язычка, ни края для растягивания здесь нет: размер
/// окна задают его собственной рамкой, а закрывают крестиком — он же
/// возвращает панель в окно плеера.
fn show_detached(app: &mut PithApp, ctx: &egui::Context) {
    let viewport = app.place_bookmarks_window(
        egui::ViewportBuilder::default()
            .with_title(tr!("Отрезки", "Fragments"))
            .with_min_inner_size(DETACHED_MIN_SIZE),
        DETACHED_SIZE,
    );

    let id = egui::ViewportId::from_hash_of("bookmarks");
    let mut actions = PanelActions::default();
    let mut attach = false;

    ctx.show_viewport_immediate(id, viewport, |ctx, _class| {
        app.track_bookmarks_window(ctx);

        // Подложка во всё окно, и явно: буфер этого окна общий с кадром
        // mpv, и незакрашенные места показывают видео (CLAUDE.md —
        // `vo=libmpv`, один контекст отрисовки на всё).
        let window = ctx.input(|i| i.viewport_rect());

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(theme::PANEL_CARD)
                    .inner_margin(egui::Margin::same(PANEL_PADDING)),
            )
            .show(ctx, |ui| {
                ui.painter().rect_filled(window, 0.0, theme::PANEL_CARD);

                show_body(app, ui, &mut actions);
            });

        // Крестик окна возвращает панель в плеер: отдельного окна больше
        // нет, а панель никуда не девается.
        if ctx.input(|i| i.viewport().close_requested()) {
            attach = true;
        }
    });

    apply(app, actions);

    if attach {
        app.toggle_bookmarks_panel_detached();
    }
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

/// Сколько высоты занимают кнопки под списком.
///
/// Строк бывает одна или две: нарезка активного списка вместе со значками
/// выгрузки и очистки, а под ней — нарезка всех списков. Вторая появляется
/// не всегда, и место нужно отмерить заранее: список забирает всё, что выше.
fn actions_height(app: &PithApp) -> f32 {
    let Some(video) = app.current_bookmarks() else {
        return BUTTON_HEIGHT + BUTTON_GAP;
    };

    let has_active = video
        .active()
        .is_some_and(|list| !list.bookmarks.is_empty());
    let rows = usize::from(has_active) + usize::from(video.lists.len() > 1);

    rows.max(1) as f32 * (BUTTON_HEIGHT + BUTTON_GAP)
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
                bookmarks_row::show(ui, bookmark, &others, can_extract, actions);
            }
        });
}

fn show_empty(ui: &mut egui::Ui) {
    panel_head::show_empty(ui);
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
    if actions.export {
        app.open_export();
    }
    if let Some(time_ms) = actions.extract_one {
        app.start_extraction_one(time_ms);
    }
    if let Some(name) = actions.copy_name {
        app.copy_text_to_clipboard(&name);
    }
}
