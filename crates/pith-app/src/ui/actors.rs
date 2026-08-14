//! Окно актёров — отдельное окно поверх всех.
//!
//! Отдельное, а не панель внутри плеера: его двигают куда угодно, хоть
//! на второй экран, и оно не закрывает кадр. Прячется клавишей `A`.

use pith_store::{CastMember, VideoCast};

use crate::app::{CastStatus, PhotoSize, PithApp};
use crate::theme;
use crate::tr;
use crate::ui::{actor_photo, panel_head};

/// Размер окна при первом показе.
const DEFAULT_SIZE: [f32; 2] = [340.0, 520.0];

/// Наименьший размер: уже него список актёров нечитаем.
const MIN_SIZE: [f32; 2] = [260.0, 200.0];

/// Отступ содержимого от краёв.
const PADDING: i8 = 12;

/// Какую долю ширины окна занимает фотография в строке.
///
/// Окно растягивают на весь экран, чтобы разглядеть лица, — и строки
/// должны расти вместе с ним, а не оставаться марками в углу.
const PHOTO_SHARE: f32 = 0.14;

/// Пределы высоты фотографии: от узкого окна до развёрнутого во весь экран.
const PHOTO_MIN: f32 = 36.0;
const PHOTO_MAX: f32 = 132.0;

/// Отступ фотографии от краёв строки.
const PHOTO_MARGIN: f32 = 4.0;

/// Где взять ключ доступа к базе.
const KEY_URL: &str = "https://www.themoviedb.org/settings/api";

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if !app.actors_open() {
        return;
    }

    let viewport = app.place_actors_window(
        egui::ViewportBuilder::default()
            .with_title(tr!("Актёры", "Cast"))
            .with_min_inner_size(MIN_SIZE)
            // Поверх всех окон: с ним работают, не отводя глаз от кадра.
            .with_always_on_top(),
        DEFAULT_SIZE,
    );

    let id = egui::ViewportId::from_hash_of("actors");
    let mut closed = false;

    ctx.show_viewport_immediate(id, viewport, |ctx, _class| {
        app.track_actors_window(ctx);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(theme::PANEL_CARD)
                    .inner_margin(egui::Margin::same(PADDING)),
            )
            .show(ctx, |ui| show_body(app, ui));

        // Раскрытая фотография — поверх всего окна, поэтому рисуется
        // после списка и уже вне его области.
        actor_photo::show_preview(app, ctx);

        // Крестик окна прячет его так же, как клавиша. При выходе из плеера
        // это не мешает: кадр закрытия обрывается до отрисовки окон,
        // и просьбу закрыться здесь никто не читает — открытость
        // в настройках остаётся той, какой её оставил пользователь.
        if ctx.input(|i| i.viewport().close_requested()) {
            closed = true;
        }
    });

    if closed {
        app.toggle_actors_window();
    }
}

fn show_body(app: &mut PithApp, ui: &mut egui::Ui) {
    panel_head::style_boxes(ui);

    if !app.has_tmdb_key() {
        show_key_request(app, ui);
        return;
    }

    show_title_row(app, ui);
    ui.add_space(8.0);
    show_status(app, ui);
    show_cast(app, ui);
    show_attribution(ui);
}

/// Просьба ключа — вместо всего остального, пока его нет.
fn show_key_request(app: &mut PithApp, ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new(tr!(
            "Нужен ключ доступа к базе фильмов",
            "A movie database key is needed"
        ))
        .color(theme::TEXT_PRIMARY)
        .size(15.0)
        .strong(),
    );

    ui.add_space(6.0);
    ui.add(
        egui::Label::new(
            egui::RichText::new(tr!(
                "Ключ бесплатный и выдаётся сразу после регистрации. \
                 Нужен короткий «API Key (v3 auth)», а не длинный токен.",
                "The key is free and issued right after signing up. \
                 You need the short «API Key (v3 auth)», not the long token."
            ))
            .color(theme::PANEL_MUTED)
            .size(12.0),
        )
        .wrap(),
    );

    ui.add_space(6.0);
    ui.hyperlink_to(egui::RichText::new(KEY_URL).size(12.0), KEY_URL);

    ui.add_space(10.0);

    let response = ui.add(
        egui::TextEdit::singleline(app.actors_key_input())
            .hint_text(tr!("Вставьте ключ сюда", "Paste the key here"))
            .desired_width(f32::INFINITY),
    );

    let entered = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

    ui.add_space(8.0);

    if ui.button(tr!("Сохранить ключ", "Save the key")).clicked() || entered {
        app.save_tmdb_key();
    }
}

/// Строка с найденной картиной и кнопкой запроса состава.
fn show_title_row(app: &mut PithApp, ui: &mut egui::Ui) {
    let known = app.actors_state().cast.as_ref().map(title_line);

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(known.unwrap_or_else(|| {
                tr!("Состав ещё не запрашивали", "Cast not requested yet").into()
            }))
            .color(theme::TEXT_PRIMARY)
            .size(13.0),
        );
    });

    ui.add_space(6.0);

    let working = app.actors_state().status == CastStatus::Working;
    let label = if app.actors_state().cast.is_some() {
        tr!("Обновить список", "Refresh the cast")
    } else {
        tr!("Сформировать список", "Build the cast")
    };

    let button = egui::Button::new(egui::RichText::new(label).color(theme::PANEL_CARD).strong())
        .fill(theme::PANEL_ACCENT)
        .min_size(egui::vec2(ui.available_width(), 30.0));

    if ui
        .add_enabled(!working && app.has_open_file(), button)
        .on_disabled_hover_text(tr!("Сначала откройте файл", "Open a file first"))
        .clicked()
    {
        app.request_cast(ui.ctx());
    }
}

/// Название картины с годом — по нему видно, ту ли нашли.
fn title_line(cast: &VideoCast) -> String {
    match cast.year {
        Some(year) => format!("{} ({year})", cast.title),
        None => cast.title.clone(),
    }
}

/// Строка о том, что происходит: поиск или отказ.
///
/// О записанном актёре здесь не сообщаем: это делает уведомление поверх
/// кадра, как при добавлении закладки.
fn show_status(app: &PithApp, ui: &mut egui::Ui) {
    let (text, color) = match &app.actors_state().status {
        CastStatus::Idle => return,
        CastStatus::Working => (
            tr!("Спрашиваю базу…", "Asking the database…").to_string(),
            theme::PANEL_MUTED,
        ),
        CastStatus::Failed(why) => (why.clone(), theme::ERROR),
    };

    ui.add(egui::Label::new(egui::RichText::new(text).color(color).size(12.0)).wrap());
    ui.add_space(6.0);
}

/// Список актёров: нажатие записывает актёра ближайшей закладке.
///
/// Строки рисуются только видимые, и фотография запрашивается тоже только
/// у них: тянуть полсотни картинок ради десятка на экране незачем.
fn show_cast(app: &mut PithApp, ui: &mut egui::Ui) {
    let Some(cast) = app.actors_state().cast.clone() else {
        return;
    };

    let photo_height = photo_height(ui.available_width());
    let row_height = photo_height + PHOTO_MARGIN * 2.0;

    let mut chosen: Option<String> = None;

    // Отступ между строками `show_rows` добавляет сам — он и просит высоту
    // «без отступа». Свой `add_space` сверху ломал счёт: область прокрутки
    // считала высоту списка по одному отступу, а строки занимали два,
    // и при прокрутке список плыл под полосой.
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, cast.members.len(), |ui, range| {
            for member in &cast.members[range] {
                if show_row(app, ui, member, photo_height) {
                    chosen = Some(member.label());
                }
            }
        });

    if let Some(label) = chosen {
        app.assign_actor(&label);
    }
}

/// Высота фотографии при такой ширине окна.
fn photo_height(width: f32) -> f32 {
    (width * PHOTO_SHARE).clamp(PHOTO_MIN, PHOTO_MAX)
}

/// Одна строка: фотография и подпись.
///
/// Возвращает `true`, если нажали по строке мимо фотографии, — тогда актёра
/// записывают закладке. Нажатие по самой фотографии раскрывает её крупно.
fn show_row(app: &mut PithApp, ui: &mut egui::Ui, member: &CastMember, photo_height: f32) -> bool {
    let photo_path = member.photo.clone();
    let texture = photo_path
        .as_deref()
        .and_then(|path| app.actor_photo(ui.ctx(), path, PhotoSize::Row));

    let row_height = photo_height + PHOTO_MARGIN * 2.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), row_height),
        egui::Sense::click(),
    );

    if response.hovered() {
        ui.painter()
            .rect_filled(rect, 6.0, theme::PANEL_ELEMENT_HOVER);
    }

    // Кадр постоянной формы: фотография вписывается в него с обрезкой,
    // а не растягиванием.
    let photo_rect = egui::Rect::from_min_size(
        egui::pos2(
            rect.left() + PHOTO_MARGIN,
            rect.center().y - photo_height / 2.0,
        ),
        egui::vec2(photo_height * actor_photo::ASPECT, photo_height),
    );

    actor_photo::draw(ui, photo_rect, texture.as_ref());
    show_label(ui, rect, photo_rect, member);

    let Some(path) = photo_path else {
        return assign_response(response).clicked();
    };

    // Фотография — своя область поверх строки, а не ручная проверка
    // координат: попадание считает сам egui, и нажатие достаётся ровно
    // одному из двух — картинке или строке.
    let photo = ui.interact(
        photo_rect,
        ui.id().with(("actor_photo", member.id)),
        egui::Sense::click(),
    );

    // Нажатие по фотографии раскрывает её, по остальной строке — записывает
    // актёра: два действия в одной строке, и путать их нельзя.
    let mut assign = false;

    if photo.clicked() {
        let now = ui.input(|i| i.time);
        app.open_actor_photo(&path, &member.label(), now);
    } else if response.clicked() {
        assign = true;
    }

    photo.on_hover_cursor(egui::CursorIcon::ZoomIn);
    assign_response(response);

    assign
}

/// Подсказка о том, что делает нажатие по строке.
fn assign_response(response: egui::Response) -> egui::Response {
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tr!(
            "Записать ближайшей закладке",
            "Assign to the nearest bookmark"
        ))
}

/// Имя актёра с ролью — в одну строку, с многоточием на длинном.
fn show_label(ui: &egui::Ui, rect: egui::Rect, photo: egui::Rect, member: &CastMember) {
    let left = photo.right() + 8.0;

    let mut job = egui::text::LayoutJob::simple_singleline(
        member.label(),
        egui::FontId::proportional(13.0),
        theme::TEXT_PRIMARY,
    );
    job.wrap.max_width = rect.right() - left - 4.0;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;

    let galley = ui.painter().layout_job(job);
    let top = rect.center().y - galley.size().y / 2.0;

    ui.painter()
        .galley(egui::pos2(left, top), galley, theme::TEXT_PRIMARY);
}

/// Условие базы: указывать, чьи это данные.
fn show_attribution(ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.add(
        egui::Label::new(
            egui::RichText::new(
                "This product uses the TMDB API but is not endorsed or certified by TMDB.",
            )
            .color(theme::TEXT_DISABLED)
            .size(10.0),
        )
        .wrap(),
    );
}
